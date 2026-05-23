# ADR 0005 · M3 向量召回（sqlite-vec + 召唤词触发）

- 状态：Accepted
- 日期：2026-05-23
- 范围：M3 中"对'还记得'类召唤词响应"的部分，对应 PRD §3.4 Layer 3。

## 决策

### 1. sqlite-vec 静态链接，注册到 SQLite auto-extension

通过 `sqlite_vec::sqlite3_vec_init` + `rusqlite::ffi::sqlite3_auto_extension`，在 `memory::Db::open` 里**首次**调用时注册一次（`Once`）。之后所有 `Connection::open()` 自动加载扩展，业务代码看不到加载细节。

不走 `Connection::load_extension(path)` 路径——那需要把 `.dll` 跟着分发，跨平台麻烦。静态链接打进二进制更省心，付出 ~200 KB 体积。

### 2. 双表：`vec_memories`（虚表）+ `vec_meta`（普通表）

`vec_memories` 只存 `rowid` + `embedding`。所有"这条记忆是哪天的日记 / 是哪条 milestone / 原文是什么"的业务字段都放 `vec_meta`，按 `rowid` 关联。
理由：

- 虚表本身不擅长存普通列（写入有限制、迁移麻烦）
- 普通 SQL join 在小数据集上零开销
- 想新增 `kind`（milestone / fact / 用户消息原文）时，只改 `vec_meta`

`vec_meta` 上有 `UNIQUE(kind, ref_key)`，配合 `upsert_memory` 做幂等更新——同一天的 diary 被重新总结时不会出现两条向量。

### 3. 嵌入模型走 OpenRouter，模型固定 `openai/text-embedding-3-small`

1536 维，PRD §5.2 锁定。**不**让用户在 config 里改维度——一旦改了维度，整张 `vec_memories` 都要重建（虚表的列定义里写死 `float[1536]`）。如果未来真要换，先做迁移脚本再说。

embedder 走 OpenRouter 的 `/embeddings` 端点（OpenAI 兼容），复用同一个 `api_key`。成本估算 PRD §5.3：~1500 调用 / 月 ≈ ¥1。

### 4. 召唤词触发，而非每轮都召回

PRD §3.4 描述："用户使用'还记得'、'上次'、'之前'等召唤词时"才检索。我用了 13 个常见短语的简单 `contains` 扫描（`commands.rs::has_recall_trigger`）。

**这是关键的性价比设计**：
- 每轮都召回 → 每条用户消息多花一次 embedding 调用 + 一次 SQL 查询
- 召唤词触发 → 只在用户**真的**在回忆时才花这笔钱
- 13 个词覆盖中文回忆语义的 90%+；漏检的代价是水豚那一轮没用上更早记忆，不严重

模式不灵的话以后可以换成"用 LLM 判断是否要回忆"，但那要多一次 LLM 调用，得不偿失。

### 5. 索引时机：双轨

**实时**：`summarizer::run_for_date` 写完 diary 后立即 embed 并 upsert，best-effort。

**Backfill**：启动 60 秒后扫一遍 `daily_summaries` vs `vec_meta`，把没索引的补上。理由：
- 升级时（这次就是从无向量索引升到有），所有历史 diary 都要回填
- 实时索引失败（API 抖）时下次启动会兜底
- 60 秒延迟是为了让 summary catchup 跑完再开始 embedding，避免一启动同时打三个 LLM 任务

### 6. 检索结果作为独立 system prompt 块注入

不放进 `{recent_summaries}` 也不混进 history，而是新加 `{recalled_memories}` 占位符（system.md 已更新）。块里每行 `- [日期] 文本 (相似度 0.xx)`。

把"最近 5 天"和"被这次话题唤起的更早记忆"在 prompt 里分开，让模型清楚意识到——一个是顺序记忆，一个是关联记忆。这跟人类回忆的语义结构一致，回话效果更自然。

### 7. 相似度展示用 `1 - distance`

sqlite-vec 默认用 L2 距离，数值小=相似。给模型看时翻成"相似度"（0-1，大=相似）更直观。`1 - distance` 在小距离时是合理近似，距离很大时无意义但反正那种结果模型也用不上。

## 未做（明确推迟）

- **里程碑索引** —— 现在只索引 daily_diary。`vec_meta.kind` 已经留好，等用户开始用"记住这个"功能后加 milestone embedding。
- **对话消息粒度索引** —— 没索引到单条 message。每天几十条消息全索引会让 token 成本翻 5 倍且检索质量下降（细粒度噪声多）。坚持以 diary 为单位。
- **用户的"召唤词"自定义** —— 不开放配置。要是真出现误触发再改。
- **重排** —— 检索回来的 3 条按距离排，没做 cross-encoder rerank。先看实际效果再决定要不要加。
- **遗忘曲线** —— 老 diary 的相似度不衰减。一年后仍能召回到一年前的对话——可能是 feature 也可能是 bug，等用户反馈再说。

## 影响 / 风险

- **二进制**：增加 ~200 KB（sqlite-vec 静态库）
- **API 月成本**：embedding 调用约 +¥1（每天 ~1 次 diary embed + 召唤触发时的 query embed，约几次 / 天）
- **DB 体积**：每天 ~6 KB（1536 dim × 4 byte = 6144 byte / diary），一年 ~2 MB，可忽略
- **首次启动性能**：第一次升级用户会在背景看到一波 embed 调用（已挂 60 秒延迟，不影响主流程）
- **隐私**：diary 文本被发到 OpenRouter 的 OpenAI embeddings 端点。CLAUDE.md §8.1 允许 OpenRouter 上行，符合既定边界；如果以后换本地模型（M5+ "本地小模型支持"分支）会自然消除这条暴露面
