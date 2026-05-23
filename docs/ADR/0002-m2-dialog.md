# ADR 0002 · M2 对话核心架构

- 状态：Accepted
- 日期：2026-05-22
- 范围：M2（OpenRouter 集成、气泡 UI、流式响应、profile/system prompt 装配）

## 关键选择

### 1. 气泡：独立 Tauri 窗口，而不是主窗内扩展

主窗是 192×192 透明圆形热区，状态机依赖"窗口几何中心 = 水豚中心"的不变量（见 ADR 0001）。把气泡画在主窗内意味着：
- 主窗要扩大 → 中心坐标变了 → 点击穿透轮询的常数失效
- 输入框是文本可选区域，要求 `set_ignore_cursor_events(false)`，与水豚区域语义打架

所以气泡是一个**独立的、非穿透的**窗口（`bubble.html`），主窗双击时通过 Rust 命令 `open_bubble` 计算位置并显示。两窗独立解决问题。

### 2. 流式：Rust 端 SSE 解析 + Tauri events，不走 SSE-to-frontend 直通

前端不直接连 OpenRouter，而是 Rust 用 reqwest 拿流，按 SSE 边界切块后用 `app.emit("llm-chunk", ...)` 推到前端。原因：

- API key 不进 WebView（前端连不到第三方域，更安全）
- Token 计数 / 重试 / fallback model（PRD §5.2）逻辑放后端，前端只管渲染
- 同一份 SSE 解析以后服务多窗口（设置、对话历史、调试视图等）

事件协议详见 `src-tauri/src/llm/openrouter.rs` 顶部注释。

### 3. Prompt 模板：运行时从 `docs/prompts/*.md` 读

强约束（CLAUDE.md §9.3）。当前实现走 `env!("CARGO_MANIFEST_DIR")`，**只在 dev 模式正确**。M6 打包前必须切到 Tauri 2 `BaseDirectory::Resource` + `bundle.resources`。这条已挂为 `TODO(cipher) [due: M6]` 在 `prompts.rs`。

### 4. 配置：`config.local.json` 放在 `app_data_dir`，不进项目目录

- 路径：`%APPDATA%\dev.cipher.capybit\config.local.json`（Mac: `~/Library/Application Support/...`）
- 启动时若文件不存在，写一份模板（空 api_key），用户手动编辑后下次启动生效
- 字段：`api_key`, `model`, `base_url`, `fallback_model`
- **fallback_model 当前未实装**，框架预留。下个迭代加（主模型 5xx 时自动降到 GLM-4.6 重试一次）

### 5. 打字动画

每 22 ms 显示一个字符（CSS `text-content` 替换，不是 token by token 立刻显示）。原因：OpenRouter Haiku 类模型平均吐字速度比"温柔语速"快很多，直接显示会有"啪嗒一下满屏字"的硬感。流结束时若打字还没追上，`pumpTyping` 让动画自然走完。

## 未做（明确推迟）

- **对话历史**：M2 不持久化历史，每次气泡独立的一轮（连同一窗口连发两条也不带上一条上下文）。M3 落地 conversations.sqlite 后接上。
- **流式重试**：网络断了直接 emit error。M5 之前补 retry-with-backoff。
- **Token 成本日志落盘**：当前只走 `tracing::info!`，没写专门的 cost.csv。等单日调用上百再做。
- **vision / 截图**：M5 才做。
- **诞生仪式**：默认名 "Capy"。首次启动诞生流程是 M5 的内容。

## 影响

- 新加 deps：reqwest（rustls）+ futures-util + time。首次 `cargo build` 会再花几分钟。
- 配置文件 schema 变动需迁移逻辑：当前用 serde `#[serde(default)]` 容忍缺字段，加字段不破坏旧配置。删字段或重命名时要写迁移脚本（M3 之前不会发生）。
