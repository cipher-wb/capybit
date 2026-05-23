# ADR 0004 · M3 记忆系统（对话持久化 + 每日总结 + 事实抽取）

- 状态：Accepted
- 日期：2026-05-23
- 范围：M3 中"对话持久化"+"每日总结"+"高置信度事实自动并入 profile"。**不**含向量召回（sqlite-vec），那部分单独下次迭代。

## 决策

### 1. 单连接 + Mutex，不上连接池

`conversations.sqlite` 的并发模型：
- 写：用户发消息 / 助手回完一条消息 / 每日总结写一行
- 读：每次 send_message 装配 prompt 时拉 10 条历史 + 5 条 diary

写入频率极低（用户级 QPS，至多 1/s），读延迟 < 1 ms。一把 `Mutex<Connection>` 完全够。Pool 留给下次大流量场景出现时。

### 2. UTC 写入，本地时区读出做天分桶

`messages.created_at` 存 RFC3339 UTC。读取时按 `current_local_offset()` 转本地日期分桶。理由：

- 服务器/夏令时切换/换时区都不破坏数据
- 本地日期分桶发生在 Rust 端，filter 在 50-200 条消息量级零感知
- 真要变热（每日上千条）再加一个冗余列 `created_at_local_date`。`TODO(cipher) [due: M6]` 已挂

### 3. 每日总结的两个触发路径

**catchup（启动时）** —— 检查 `messages` 表里有但 `daily_summaries` 没有的本地日期，按时间顺序最多跑 7 天。理由：
- 第一次启动可能积压了很多过去几天的对话（如果用户从 sqlite 导入旧数据，那种 corner case）
- 7 天封顶避免一启动就把月度 token 烧光

**ticker（23:59）** —— tokio 任务，每天 23:59 醒来跑当天总结。

跨天捡漏：如果 23:59 跑失败（网络断），第二天启动 catchup 会补上。两个机制冗余但廉价。

### 4. 事实合并：阈值自动并入，低置信度丢弃 + warn

- ≥ 0.8 → 自动写进 `profile.user_facts`
- < 0.8 → 当前丢弃但 `tracing::info!` 数量

PRD §3.4.1 描述的"半自动 / 全自动 / 全手动"三档 audit policy 框架先不做。**当前等价于"全自动"模式**。等做了设置面板（M6）+ 用户审核 UI 再分档。**这是有意识的折中**：宁可少记几条，也不要 UI 半成品堵在主路径上。

### 5. 助手回复持久化用事件监听而不是改流式接口

`send_message` 调 `openrouter::stream_chat` 后，主动 `app.listen("llm-done", ...)`：

- 匹配本次 request_id
- 拿到 `full_text` 写入 `messages` 表，role = "capybit"
- 30 秒后自动 unlisten，避免 listener 堆积

**为什么不直接在 `stream_chat` 里写盘？**
- 想保持 LLM 模块对 DB 无依赖（PRD §6 锁的"LLM 应可 mock"，不能让它强依赖 DB）
- 事件总线已经在那儿了（前端也用着），不引入新的耦合

### 6. system prompt 注入新增 `{recent_summaries}` 实际内容

之前是占位字符串"M3 里程碑后开始累积"。现在替换为最近 5 天 diary（按日期倒序拼），格式：

```
[2026-05-21] 今天用户提到他在写《断戎书》第三卷……
[2026-05-22] ……
```

进入 system prompt，水豚回话会自然带上跨日记忆。

### 7. 历史轮数：拉 10 条，过滤刚写的用户消息

`recent_history(db, 10)` —— 拉最近 10 条（5 轮左右），过滤掉刚才那条 user 消息（它会被 `build_chat_messages` 单独 append），剩余作为 chat history 送给 LLM。PRD §3.2.3 是"最近 5 轮"，10 条 = 5 轮的稳妥估计。

## 未做（明确推迟）

- **向量召回** —— sqlite-vec 集成（依赖 OpenAI embeddings API、扩展加载等）较重，单开一次推。届时新增 `vectors.sqlite` + 召唤词检测（"还记得"/"上次"/"之前"）+ 检索注入。
- **审核 UI** —— "查看记忆"右键菜单项、低置信度事实手动确认。M6 设置面板做。
- **milestones 写入** —— PRD §3.4 描述的"用户说'记住这个'"流程未做。Function calling 暂时没启用。
- **数据导出/清空** —— PRD §9 用户控制项目，M6 做。
- **跨日 ticker 鲁棒性** —— 当前 ticker 在系统休眠时可能错过 23:59。靠次日 catchup 兜底，但不够稳。未来用 RTC wakeup 或者更频繁的"是否过了 23:59"轮询。

## 影响

- 二进制大小：增加 ~1.5 MB（rusqlite bundled SQLite）。仍在 PRD 8MB 预算内。
- API 成本：每天多一次 daily_summary 调用，按 PRD §5.3 估算 ¥2-5/月，不显著。
- profile.json 增长：每天 0-5 条事实。一年大约 1000 条。装进 system prompt 时已 `.take(20)`，长期不会爆 token。
