# ADR 0007 · M5 主动说话 + 全局快捷键

- 状态：Accepted
- 日期：2026-05-23
- 范围：PRD §3.1.2 + §3.2.1 + §4.3 的主动交互那一档。

## 决策

### 1. 触发条件：state machine 决定，scheduler 执行

`tick.rs::should_fire_proactive` 在每个 60s tick 评估三条：

- `urge > 70`
- `!in_flow`（PRD §3.1.2 心流保护硬约束）
- 距上次 firing ≥ 30 分钟（冷却）

三条全满足 → 立即把 `last_proactive_at` 时间戳写到 `InnerState`（**仍在 lock 范围内**），然后 spawn 异步任务跑 opener 生成。

**为什么 timestamp 要在 lock 内写**：opener LLM 调用 ~2-5s，期间下一个 tick 可能到了。如果时间戳写在 spawn 后，会出现"两个并发 firing"。把 mark 放在 tick 同步阶段最干净。

### 2. opener 文本走完整 LLM 调用 + 落盘

`llm::opener::generate` 是非流式的短调用（max_tokens=80, temperature=0.9），输入 = 当前 4 维状态 + 当前感知 + 用户对水豚的称呼。返回一句 ≤ 30 字的话。

落盘到 `messages` 表，role = "capybit"。这样**用户点"回应"输入回复后**，`send_message` 拉历史时，最近一条 capybit turn 就是这个 opener —— LLM 看到的上下文里有"我刚才说了 X"，回应自然连贯。

避免了"opener 像一段孤立的对白、用户回复时模型不知所云"的尴尬。

### 3. 三种 dismiss 路径

| 用户行为 | 副作用 |
|---|---|
| 点 "回应" | UI 切到输入框；urge / cooldown 不动（等用户真的发消息后，`on_user_chat` 会清 urge） |
| 点 "等会儿" | `dismiss_proactive`: urge → 30, last_proactive_at = now（强制 30 min 冷却）, 关气泡 |
| 点 "关闭" / Esc | 跟"等会儿"等效——不发请求，气泡关掉（urge 不动，下次 cooldown 到了照样会再来）|

为啥"等会儿"不把 urge 清 0：保留点"还想说话"的余温，让水豚有内在连续性。30 是阈值之下 (`> 70` 才触发) 的安全值。

### 4. 全局快捷键 `Ctrl+Shift+\`

走 `tauri-plugin-global-shortcut`。直接调 `commands::open_bubble`。**不**自动生成 opener —— 这是用户主动呼出，应该是空气泡等用户输入。和被动 opener 路径区分开。

失败处理：注册失败时 `tracing::warn!` 但**不阻塞启动**（PRD 没把它列为 must-have；某些机器可能被其他软件占用 `Ctrl+Shift+\`）。

### 5. 心流保护是硬约束，不是软提示

代码里 `should_fire_proactive` 第一条就 short-circuit。即使 urge=100，只要 `in_flow=true`，绝对不弹气泡。PRD §8.4 "不打扰心流"是不可妥协项，对应 CLAUDE.md §8.4。

这意味着用户专注 coding 时永远不会被打扰。代价：用户**真的**需要被打扰的场景（连续 4 小时不动、累垮了）当前判定不出来——那是更高阶的心流模型（detection of "stuck" flow vs "productive" flow），M5 不做。

## 未做

- **"用户连续工作 3 小时" 主动叹气** —— PRD §4.3 表里的边缘小动作场景。当前 `curious` 动画就当兜底；正式的"边缘存在动作"队列（叹气、抬头、转圈）跟 M1 的精灵图后处理一起做
- **主动开场区分场景** —— PRD §4.3 举的例子（"在 VSCode 卡 30 分钟"、"切到 YouTube"）当前**都走同一个** opener prompt。模型自己根据 perception 区分。如果质量差，再拆成 `proactive_*.md` 多模板，按 perception 规则路由
- **"暂时关闭主动说话"开关** —— config.local.json 还没字段。临时方案：托盘"暂时隐身"会停感知（窗口隐藏不影响 tick，但用户看不到气泡也算"关了"）；正式开关 M6 设置面板做
- **全局快捷键自定义** —— CLAUDE.md 的 config 模板里 `hotkey_talk` 字段还没生效。当前写死 `Ctrl+Shift+\`，按 PRD §8.2 默认值

## 影响

- 二进制：+~150 KB (global-shortcut 插件 + global-hotkey crate)
- API：每天最多 ~30 次 opener 调用（24 小时 / 30 min cooldown），实际远低（睡觉时间、心流时间都不触发）；按 PRD §5.3 估算月成本 +¥3-5
- 隐私：opener 把当前 app 名 + 窗口标题给了 OpenRouter。黑名单已经在 `perception::capture` 阶段把敏感标题替换了，符合 CLAUDE.md §8.1 边界
