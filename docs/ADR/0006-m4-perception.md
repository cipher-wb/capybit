# ADR 0006 · M4 感知 + tick 状态机 + 内心独白

- 状态：Accepted
- 日期：2026-05-23
- 范围：PRD §3.3 默认感知 + §3.5 4 维状态机 + 鼠标悬停内心独白。**不**含主动说话气泡（M5）和屏幕截图/vision（M5）。

## 决策

### 1. 感知与 tick 分离的双频率

- **感知采样**：随每次 tick 触发（60s），加上鼠标悬停 / 用户发消息时的按需采样
- **tick 调度**：60s 严格周期，更新状态机、写持久化、发 `pet-state` 事件

不做 5s 高频感知轮询。`active-win` + `GetLastInputInfo` 一次调用 < 1ms，但 60s 已经足够分辨"用户切了应用"，再快也只是浪费 CPU，违背 PRD §8.2 idle < 1% CPU 的硬约束。

### 2. `idle_seconds`：Windows 用 `GetLastInputInfo`，其他平台先打 0

跨平台 idle 检测有几个候选：
- `user-idle` crate —— 多平台封装，但维护状态不稳，依赖链长
- `device_query` —— 持续轮询键鼠状态，CPU 不友好
- 自己直调 OS API —— 最便宜

选第三条。Windows 只需 `GetLastInputInfo` + `GetTickCount`，两次 syscall，几微秒。Mac/Linux 平台先返回 0（永远视为活跃）—— 状态机里"用户在场"的判定会一直成立，能量不再回血，但其他行为正常。等 Mac/Linux 用户出现再各自实现一次（`IOHIDIdleTime` / `XScreenSaverQueryInfo`）。

### 3. 隐私：黑名单应用窗口标题不入 prompt 上下文

CLAUDE.md §8.1 列了密码管理器、支付宝、微信、网银。`perception::is_blacklisted_app` 命中时，`window_title` 被替换为"（敏感，已隐藏）"再返回。应用名本身保留（"用户在 Bitwarden"是有用的状态信号；具体在看哪个 vault 不该泄露）。

### 4. 状态机：4 维 + 行为决策表（PRD §3.5）

四个数值都是 f32，0-100。`step()` 每 tick 调一次：

| 规则 | 来源 |
|---|---|
| 用户离开 ≥ 5min → 视为不在场，能量缓慢恢复 | PRD §3.5 |
| 用户连续 5min 无键鼠 → 退出心流 | PRD §3.1.2 |
| 同一应用持续 20min + 在活跃 → 进入心流 | PRD §3.1.2 |
| 心流期间 urge 上限被压制到 50 | PRD §3.1.2 |
| 切到新应用 → curiosity +20 | PRD §3.5 |
| 用户聊天 → mood +5, urge 清零, energy -3 | PRD §3.5 |

行为决策也直接照 §3.5 的决策表实现（`choose_action`）。当前 `curious + urge > 70 + !in_flow` 这一档**只切到 `curious` 动画**，**不**自动弹气泡 —— 那是 M5 主动说话的内容，下一里程碑接。

### 5. InnerState 跟 WindowState 合并到同一份 state.json

`PersistedState` 新增 `inner: Option<InnerState>`。这样：

- 一份 state.json 一次写盘搞定，避免多文件竞态
- 老 state.json（M0 时只有 position+hidden）`#[serde(default)]` 兼容，inner 为 None 时用 `InnerState::default()` 重建
- 每 5 tick = 5min 写一次（不是每 tick），减少 IO

### 6. Inner monologue：cache-aside，TTL 5 分钟 + action 变更失效

PRD §3.5："由 LLM（或缓存）生成"。我们做最朴素的 LRU-of-one：

- 缓存一段独白 + 生成时的 action + 时间戳
- hover 时优先返回缓存；只有 (action 变了) **或** (TTL 5 分钟过了) 才发新 LLM 调用
- 没 api_key / 网络挂 → 8 条 fallback 文案按 action 选一条

成本估算：典型用户每天 hover 几十次，但 action 切换 < 10 次／天，平均 LLM 调用 ~8 次/天 ≈ ¥1-2/月。可接受。

### 7. `pet-state` 事件，前端 sprite action 跟着切

tick 每次发 `pet-state` 事件给前端，里面有 `current_action`。`main.js::wireStateEvents` 接住，更新 `state.action`，render loop 下一帧就切到对应精灵图 frames。

整条链：感知 → state machine → action 决定 → emit → 前端切图，60 秒一轮，**水豚的"内在生活"自动外显**。

## 未做

- **Lock screen / 屏幕灭** —— Windows 上要监听 `WTS_SESSION_*`；Mac 用 `CGSessionCopyCurrentDictionary`。M4 不做，状态机里假设屏幕亮着
- **跨多显示器、多桌面** —— `active-win` 在虚拟桌面切换时的行为未测
- **悬停 tooltip 跟随鼠标** —— 当前固定在窗口顶部居中，没做"跟着鼠标走"的浮窗。设计取舍：固定位置不会跟着鼠标颤抖，读起来更稳；想要"跟随"行为以后再加
- **主动说话气泡（urge > 70 自动弹气泡）** —— M5 的"主动交互"

## 影响

- 二进制：+200 KB (active-win-pos-rs)
- CPU：每 60s 一次 `GetLastInputInfo` + `get_active_window`，远低于 1% idle 预算
- API：~8 monologue 调用 / 天，每次 60 max_tokens，月成本 ¥1-2
- 隐私：M4 默认感知严格按 PRD §3.3.1，没引入 §3.3.2 增强感知（截图/vision），授权边界未变
