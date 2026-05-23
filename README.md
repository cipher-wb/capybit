# Capybit

> 一只长在你桌面上的电子宠物水豚。

Capybit 是一台微型 32×32 像素 LCD 屏，里面养着一只水豚。
他会感知你在用什么应用、什么时候发呆、什么时候心流；
他有自己的精力、心情、好奇心、说话欲望，按 60 秒一拍演化；
你跟他说话时，他会**用自己的屏画东西回应**——
画一颗心、写个温度数字、用自由像素涂鸦一个小图案——
全部由 LLM 实时决定，不是预设动画。
他会记住你说过的事，每天 23:59 写日记总结一天；半年后你说"还记得吗"，
他真的能想起来。

桌面陪伴不是观赏摆件、不是 To-Do 助手、不是 ChatGPT 浮窗——
他只是**在那里**，有自己的内在生活，并且认识你。

详细产品定义见 [`docs/PRD.md`](docs/PRD.md)。

---

## 当前状态

按 [`CLAUDE.md`](CLAUDE.md) 的里程碑：

| 里程碑 | 内容 | 状态 |
|---|---|---|
| **M0** · 脚手架 | Tauri 2 透明置顶窗 + 状态持久化 + 系统托盘 | ✓ Windows |
| **M2** · 对话核心 | OpenRouter 流式对话 + 气泡 UI + system prompt 装配 | ✓ |
| **M3** · 记忆系统 | conversations.sqlite + 每日总结 + 事实抽取 + sqlite-vec 召回 | ✓ |
| **M4** · 感知 + 状态机 | 活动窗口/键鼠空闲感知 + 4 维 tick + 鼠标悬停内心独白 | ✓ |
| **M5** · 主动 + 诞生仪式 | urge>70 主动开场白 + 全局快捷键 + 首次启动诞生仪式 | ✓ |
| **+** · LCD scene 系统 | 32×32 点阵屏 + sprite/icon/text/free-cells 四层合成 | ✓ |
| **+** · LLM function calling | `set_scene` 工具让水豚自主控制 LCD 显示什么 | ✓ |
| **M6** · 设置面板 + 打包 | 设置 UI、低置信度事实人工审核、`.msi` 安装包 | 计划中 |

Mac 端的窗口/感知代码尚未验证。

---

## 快速开始

### 1. 工具链准备（首次，约 30 分钟）

```powershell
# Rust（约 200 MB）
winget install --id Rustlang.Rustup
rustup default stable

# Microsoft C++ Build Tools（Tauri 的链接依赖，约 3 GB）
winget install --id Microsoft.VisualStudio.2022.BuildTools `
  --override "--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"

# WebView2 Runtime（Win11 自带，Win10 需要）
# 通常 Edge 已带，否则去：
# https://developer.microsoft.com/en-us/microsoft-edge/webview2/

# pnpm
npm install -g pnpm
```

### 2. 拉代码 + 装依赖

```powershell
git clone https://github.com/cipher-wb/capybit.git
cd capybit
pnpm install
```

### 3. 跑起来

```powershell
pnpm tauri dev
```

首次启动会 `cargo fetch` 下载 ~80 个 crate，5-15 分钟（视网络）。
代理用户可设：

```powershell
$env:HTTPS_PROXY = "http://127.0.0.1:7890"
$env:HTTP_PROXY  = "http://127.0.0.1:7890"
pnpm tauri dev
```

### 4. 第一次启动会触发诞生仪式

应用启动后中央会弹出一个仪式窗：
1. 选一个**你常常写字的文件夹**（Documents / Obsidian Vault / 任何含 `.md` `.txt` 的目录）
2. 应用从里面随机抽一段你写过的字作为"诞生原文"
3. 你给水豚起个名字 + 告诉他怎么称呼你
4. 仪式完成 → 桌面右下出现一只活着的水豚

那段诞生原文会**永久**进入水豚的 system prompt，是他记忆的根。

---

## 配置 OpenRouter API Key

诞生仪式完成后（或第一次启动时），应用会写一份模板到：

```
%APPDATA%\dev.cipher.capybit\config.local.json
```

PowerShell 一键打开：

```powershell
notepad "$env:APPDATA\dev.cipher.capybit\config.local.json"
```

填入你的 key + 选模型：

```json
{
  "api_key": "sk-or-v1-你的key",
  "model": "anthropic/claude-haiku-4-5",
  "base_url": "https://openrouter.ai/api/v1",
  "fallback_model": "z-ai/glm-4.6"
}
```

**模型选择建议**：

| 模型 | 月成本估算 | 工具调用质量 | 备注 |
|---|---|---|---|
| `anthropic/claude-haiku-4-5` | ¥20-50 | 优秀 | **推荐**。会积极调用 `set_scene` 画屏 |
| `deepseek/deepseek-v4-pro` | ¥10-25 | 良好 | thinking model，延迟多 2-5s，但思考质量更深 |
| `z-ai/glm-4.6` | ¥15-35 | 一般 | 中文友好，工具调用偶发 |

OpenRouter 申请 key：<https://openrouter.ai/keys>（免费注册，需充几块钱 credit）。

**改完 key 要重启应用**（config 是启动时读的）。

---

## 怎么用

| 操作 | 触发 |
|---|---|
| **双击水豚** | 弹出对话气泡 |
| **`Ctrl+Shift+\`**（全局快捷键） | 任何时候、任何应用里都能呼出气泡 |
| **拖动水豚** | 鼠标按住身体拖动，位置自动保存 |
| **鼠标悬停水豚** | 显示一句他的"内心独白"（5 分钟缓存，不会狂调 API） |
| **托盘菜单右键** | 暂时隐身 / 显示 / 打开 DevTools / 退出 |
| **气泡里发消息** | Enter 发送，Shift+Enter 换行，Esc 关闭气泡 |

**主动说话**：当水豚的 `urge` 涨过 70、且你没在心流（用户连续在一个应用 20+ 分钟键鼠活跃）时，
他会自动弹气泡跟你说点话。你可以"回应"或"等会儿"（"等会儿"压制 30 分钟）。

**他会画屏**：每条消息他都可能调用 `set_scene` 工具改变 LCD 屏上的内容——
切到对应心情的姿势、叠一个心形/星星图标、写一行字、或自由像素现画一个小图案。
持续 4-8 秒后回到默认。

---

## 调试

### 查看 Rust 后端日志

启动 `pnpm tauri dev` 的那个 PowerShell 窗口本身就是日志输出。所有
`tracing::info!` / `tracing::debug!` 都打在那里。
默认日志等级是 `info,capybit=debug`，看不到的话设：

```powershell
$env:RUST_LOG = "debug,capybit=trace"
pnpm tauri dev
```

### 查看前端日志（DevTools）

**右键托盘水豚图标 → 打开调试控制台**（仅 dev 模式可用）。
console 会显示 `[capybit] lcd-scene received {...}` 之类的事件流。

### 数据文件位置

| 文件 | 用途 |
|---|---|
| `%APPDATA%\dev.cipher.capybit\config.local.json` | API key + 模型配置（**gitignored**） |
| `%APPDATA%\dev.cipher.capybit\profile.json` | 长期事实库 + 诞生记录 |
| `%APPDATA%\dev.cipher.capybit\state.json` | 窗口位置 + 4 维内在状态 |
| `%APPDATA%\dev.cipher.capybit\conversations.sqlite` | 所有对话 + 每日总结 + 向量索引 |

**重新走诞生仪式**：删 `profile.json` 即可。**清空记忆**：删 `conversations.sqlite`。

---

## 文档导航

| 文件 | 内容 |
|---|---|
| [`docs/PRD.md`](docs/PRD.md) | 产品需求文档（**WHAT/WHY**） |
| [`CLAUDE.md`](CLAUDE.md) | 工程协作约束（**HOW**） |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | 当前架构快照 + LLM 调用全表 + 入口路径 |
| [`docs/ADR/`](docs/ADR/) | 11 份架构决策记录，每份对应一个非平凡的设计选择 |
| [`docs/prompts/`](docs/prompts/) | 所有 LLM prompt 模板（运行时加载） |

---

## 技术栈

- **运行容器**：Tauri 2.x（Rust 后端 + WebView 前端）
- **前端**：原生 JS + Canvas 2D（不上框架，零依赖）
- **AI 大脑**：OpenRouter 聚合（Claude Haiku / DeepSeek / GLM 任选）
- **嵌入**：OpenAI `text-embedding-3-small`（1536 维）
- **本地存储**：SQLite + sqlite-vec（静态链接，单文件）
- **跨平台感知**：`active-win-pos-rs`（Mac/Win/Linux），Windows 端键鼠空闲走 `GetLastInputInfo`

二进制大小：6.6 MB（release，PRD 8 MB 预算内）。
内存占用：常驻 < 150 MB。
CPU 占用：idle 时 < 1%（60s tick + 33ms 鼠标轮询）。

---

## License

暂未决定。计划开源，许可证待项目稳定后选定（候选：MIT / Apache 2.0）。
当前不接受外部贡献——项目处于早期主架构调整期，作者本人为第一用户。

---

*"他只是在那里，认识你。"*
