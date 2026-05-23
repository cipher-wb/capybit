# CLAUDE.md

> 这份文件是 **Capybit** 项目的工程协作上下文。Claude Code 每次进入这个 repo，必须先读完此文件再开始任何工作。
>
> 产品决策（WHAT / WHY）见 `docs/PRD.md`。本文件只关心 **HOW**：代码结构、约定、工作方式、边界。

---

## 1. Project Identity（30 秒理解）

**Capybit** 是一个桌面 AI 伴侣应用，以一只温柔话痨的水豚作为载体，通过 LLM + 长期记忆库 + 屏幕感知，与用户建立持续陪伴关系。

- **类型**：跨平台桌面应用（macOS + Windows，必要时支持 Linux）
- **运行容器**：Tauri 2.x（Rust 后端 + WebView 前端）
- **AI 大脑**：纯云端，OpenRouter 聚合，主用 Claude Haiku / GLM-4.6
- **数据存储**：纯本地（profile.json + SQLite + sqlite-vec）
- **目标**：长期开源项目，开发者本人为第一用户

完整产品定义见 `docs/PRD.md`。

---

## 2. Current Phase

| 字段 | 值 |
|---|---|
| 当前里程碑 | **M2 · 对话核心**（已跑通；下一步进 M3 记忆或 M5 诞生仪式由 Cipher 决定） |
| 下一里程碑 | M3 · 记忆系统 / M5 · 诞生仪式（待定） |
| 当前主分支 | `main` |
| 工作分支模式 | `feat/m{N}-{topic}`、`fix/{topic}` |

**M0 完成的判定标准**（Claude Code 每次工作前对照检查）：

- [~] Tauri 2.x 项目可在 macOS 和 Windows 双端构建（Win ✓，Mac 未验证）
- [x] 主窗口实现：无边框、置顶、透明背景、点击穿透（水豚区域除外）
- [x] WebView 内渲染一个占位水豚（纯 Canvas 2D chibi 水豚）
- [x] 主窗口位置可拖动并持久化到 `state.json`
- [x] 系统托盘存在，含"退出"菜单项
- [~] `cargo build --release` 在 Mac 和 Win 上都过（Win ✓ = 6.5MB，Mac 未验证）

---

## 3. Codebase Map（目标结构）

当前是空 repo。Claude Code 应朝以下结构演进：

```
capybit/
├── CLAUDE.md                    # 本文件
├── README.md                    # 用户向（项目简介 + 安装 + 截图）
├── docs/
│   ├── PRD.md                   # 完整产品需求文档
│   ├── ADR/                     # 架构决策记录（每个重要决策一个文件）
│   └── prompts/                 # 所有 LLM prompt 模板（独立维护）
│       ├── system.md
│       ├── daily_summary.md
│       └── ...
├── src-tauri/                   # Rust 后端
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs              # 入口
│       ├── window.rs            # 窗口管理（透明/置顶/点击穿透/拖动）
│       ├── perception/          # 感知层
│       │   ├── mod.rs
│       │   ├── active_window.rs
│       │   └── screenshot.rs
│       ├── memory/              # 记忆层
│       │   ├── mod.rs
│       │   ├── profile.rs       # profile.json 读写
│       │   ├── conversations.rs # SQLite 对话
│       │   └── vectors.rs       # sqlite-vec 向量
│       ├── llm/                 # LLM 调用
│       │   ├── mod.rs
│       │   ├── openrouter.rs
│       │   └── prompts.rs       # 拼装 prompt（不写死内容，从 docs/prompts/ 读）
│       ├── scheduler/           # tick + event
│       │   ├── mod.rs
│       │   ├── tick.rs
│       │   └── state_machine.rs # energy/mood/curiosity/urge
│       └── commands.rs          # tauri::command 暴露给前端的 API
├── src/                         # 前端（原生 JS + Canvas，不上框架）
│   ├── index.html
│   ├── main.js                  # 入口、tauri API 调用
│   ├── renderer/
│   │   ├── sprite.js            # 精灵帧动画
│   │   ├── motion.js            # 移动/拖动/吸附逻辑
│   │   └── canvas.js            # Canvas 渲染循环
│   ├── ui/
│   │   ├── bubble.js            # 对话气泡
│   │   ├── settings.js          # 设置面板
│   │   └── styles.css
│   └── assets/
│       └── sprites/             # 精灵图集（M1 才有）
├── scripts/                     # 开发用辅助脚本
│   ├── gen_sprites.py           # AI 精灵图集后处理（rembg + Pillow）
│   └── normalize_sprite.py
└── .claude/                     # Claude Code 配置
    ├── skills/                  # 自定义 skills
    └── hooks/                   # 提交前格式化等
```

**规则**：新增模块前先在 `docs/ADR/` 写一份决策记录（哪怕只有 5 行），避免后续 Claude Code 实例不知道为什么这样设计。

---

## 4. Tech Stack & Versions（锁定）

| 组件 | 版本 | 备注 |
|---|---|---|
| Rust | 1.78+ | `rustup default stable` |
| Tauri | 2.x | 不用 1.x |
| Node | 20 LTS | 仅 dev 时用 |
| Python | 3.11+ | 仅 sprite 生产脚本用 |
| SQLite | 3.45+ | rusqlite 自带 |
| sqlite-vec | latest | `cargo add sqlite-vec` |

**关键 crates**：

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.31", features = ["bundled"] }
sqlite-vec = "0.1"
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
active-win-pos-rs = "0.9"
anyhow = "1"
thiserror = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
```

前端零依赖，原生 JS + Canvas。**不引入 React/Vue/Svelte**。

---

## 5. Commands Cheatsheet

**开发**：

```bash
# 首次拉取后
pnpm install                     # 安装前端 dev 依赖（仅 tauri-cli）
cd src-tauri && cargo fetch      # 预下载 Rust 依赖

# 开发模式（带 hot reload）
pnpm tauri dev

# 生产构建
pnpm tauri build

# 仅前端调试（开浏览器看 UI，但拿不到 tauri API）
# 不推荐，桌宠强依赖 tauri 能力
```

**测试**：

```bash
cd src-tauri && cargo test       # Rust 单测
cargo clippy -- -D warnings      # lint，警告即错误
cargo fmt                        # 格式化
```

**精灵图集生产**（M1 才用）：

```bash
python scripts/gen_sprites.py \
  --input raw_outputs/ \
  --output src/assets/sprites/
```

**代理环境提示**（开发者用 Clash Verge）：

- `cargo` 默认走系统代理，但偶尔需要显式设置：
  ```bash
  export HTTPS_PROXY=http://127.0.0.1:7890
  export HTTP_PROXY=http://127.0.0.1:7890
  ```
- `pnpm` / `npm` 设置：
  ```bash
  pnpm config set registry https://registry.npmmirror.com
  ```
- 如果 `cargo fetch` 卡住，先试 `~/.cargo/config.toml` 配 `[source.crates-io] replace-with = "tuna"`

---

## 6. Coding Conventions

### Rust

- **错误处理**：库代码用 `thiserror` 定义错误类型；应用层用 `anyhow::Result<T>` 串起来
- **日志**：用 `tracing`，禁止 `println!`、`dbg!` 进入提交
- **异步**：业务全用 `tokio`，绝不在 tauri command 里阻塞
- **命名**：模块 snake_case，类型 PascalCase，函数/变量 snake_case
- **文件大小**：单个 .rs 文件超过 400 行就要考虑拆分
- **公共 API**：所有 pub 函数必须有 doc comment

### JavaScript

- **风格**：ES2022+，原生 module（`<script type="module">`），不用 bundler
- **导入**：相对路径 + `.js` 后缀（浏览器原生 import 要求）
- **状态**：避免全局变量，用模块作用域；状态变更走单一函数
- **DOM**：直接操作 DOM，不引入虚拟 DOM 库
- **命名**：camelCase，常量 SCREAMING_SNAKE

### 通用

- **提交信息**：
  - 格式 `<type>(<scope>): <subject>`
  - type: `feat / fix / refactor / docs / chore / test`
  - scope: `window / perception / memory / llm / scheduler / ui / sprite`
  - 例：`feat(window): implement transparent click-through on macOS`
- **注释**：写"为什么"，不写"是什么"；代码本身要能说明"是什么"
- **TODO**：必须带格式 `// TODO(cipher): <reason> [due: M{N}]`，禁止裸 TODO

---

## 7. Architectural Decisions Locked

以下决策**已锁定**，Claude Code 不要在没有明确指令的情况下挑战或修改这些选择。如有充分理由提出修改，先写一份 `docs/ADR/NNNN-{title}.md` 提案，等用户确认。

| 决策 | 选择 | 理由（简） |
|---|---|---|
| 运行容器 | Tauri 2.x | 体积小、Rust 性能、透明窗口支持完善 |
| 前端框架 | 原生 JS + Canvas | 桌宠场景轻量为先，避免框架开销 |
| 精灵渲染 | Canvas 2D | 帧动画够用，不需 WebGL |
| LLM 调用 | 纯云端（OpenRouter） | 24/7 常驻不抢本地显存 |
| Embedding | OpenAI text-embedding-3-small | 性价比、中英文均可 |
| 数据库 | SQLite + sqlite-vec | 单文件、零运维、向量一体化 |
| 形象方案 | AI 生成精灵图集 | 无美术资源约束下的最优解 |
| 数据存储 | 纯本地，仅本机 | 隐私优先 |

---

## 8. Critical Constraints（不可妥协）

### 8.1 隐私

- **绝对不允许**：把用户的对话内容上传到 OpenRouter 之外的服务
- **绝对不允许**：默认开启屏幕截图。截图必须每次显式授权
- **绝对不允许**：把窗口标题/内容记录到任何远程日志服务（包括 crash report）
- **黑名单应用**：1Password, Bitwarden, 支付宝, 微信, 网银 → 窗口标题不进 LLM 上下文

### 8.2 性能

- 内存占用：常驻状态 **< 150MB**（不含 WebView 本身）
- CPU 占用：idle 状态 **< 1%**（在 M1 Mac mini 上测）
- tick 间隔：**60 秒**（不要因为"想多动"改成更短）
- 帧率：60fps，但 idle 时动画 4-8fps 即可，省电

### 8.3 跨平台

- 所有窗口/感知代码必须 Mac + Win 双端验证
- 路径处理一律用 `std::path::PathBuf`，禁止字符串拼接路径
- 文件名禁用 `:`, `?`, `*`, `<`, `>`, `|`（Windows 不允许）

### 8.4 体验

- **不打扰心流**：心流模式检测必须先实现再做主动说话功能
- **默认沉默**：任何疑问场景下，水豚默认选择不说话
- **可控**：用户必须随时能"暂时隐身"，快捷键全局生效

---

## 9. Working Style for Claude Code

### 9.1 每次任务开始前

1. 读本文件第 2 节确认当前里程碑
2. 读相关模块的 `docs/ADR/`（如果存在）
3. 用 `git status` 和 `git log -5` 看看上次工作到哪
4. **跨多文件改动 ≥ 3 个时**，先输出一份 plan，等用户确认再动手

### 9.2 实现节奏

- **优先做能跑的**：M0 阶段可以用占位水豚 emoji，不必等 M1 的真精灵
- **垂直切片**：宁可把一个功能从 Rust 后端 + JS 前端 + UI 都打通，也不要先把所有模块的 Rust 部分写完
- **每个里程碑结束**：跑一遍 `cargo clippy + cargo fmt + cargo test`，更新本文件第 2 节的勾选状态

### 9.3 LLM 相关代码的特殊约定

- **Prompt 不写死在代码里**：所有 prompt 模板存 `docs/prompts/*.md`，运行时读取
- **API 调用必须有 retry + timeout**：网络抖动是常态
- **API 调用必须可 mock**：单测时不要真打 OpenRouter
- **成本监控**：每次调用记录 token 数到本地日志，便于估算月成本

### 9.4 Skills & Hooks 建议

可以考虑在 `.claude/skills/` 里加：

- `sprite-pipeline/SKILL.md`：精灵图集生产流程的复用 skill
- `tauri-window/SKILL.md`：透明窗口/点击穿透的平台差异处理

可以考虑在 `.claude/hooks/` 里加：

- pre-commit：跑 `cargo fmt --check` 和 `cargo clippy`
- pre-tool-use（针对 bash）：拦截 `rm -rf` 类危险命令

### 9.5 Multi-agent 工作流提示

这个项目天然可拆三类工作：

- **Backend agent**：Rust 侧（窗口、感知、记忆、调度）
- **Frontend agent**：JS + Canvas（渲染、UI、交互）
- **Content agent**：Prompt 工程、精灵生产、文案

复杂任务可考虑用 Task tool 派生子 agent 并行处理。

---

## 10. Known Pitfalls（踩过/会踩的坑）

| 坑 | 应对 |
|---|---|
| Tauri 2.x 透明窗口在 Windows 上的 DPI 缩放问题 | 用 `tauri::WebviewWindowBuilder` 的 `inner_size` + 物理像素，不用逻辑像素 |
| macOS 点击穿透需要 `ignoreCursorEvents` 切换 | 鼠标进入水豚 hitbox 时关闭穿透，离开时打开 |
| `active-win` 在 macOS 首次调用需要 Accessibility 权限 | 启动时显式引导用户去系统设置授权 |
| sqlite-vec 在 musl 构建上可能有问题 | 用 glibc 构建，或者 vendored sqlite |
| Tauri WebView 在 Linux 上是 webkit2gtk，行为可能与 Mac/Win 不一致 | M0-M5 阶段先不管 Linux |
| OpenRouter 偶尔返回不规范 JSON | 每个 LLM 调用都 try-catch + 重试 |
| 长对话上下文爆 token | 上下文超过 8K token 时触发记忆压缩 |
| Windows 不允许文件名含 `:` | 时间戳格式用 `2026-05-22T10-30-00` 不用 `:` |
| Clash Verge 代理偶尔劫持 localhost | OpenRouter URL 走 https，不会被劫持，但内部 IPC 注意 |
| Windows 上 `tauri-build` 强制要求 `src-tauri/icons/icon.ico`，即使 `bundle.active = false` | 用 `pnpm tauri icon path/to/source.png` 生成；**不要**用 .NET 的 `Icon.Save()` 现造——它写出来的 ICONDIRENTRY 保留字段非 0，会被 Tauri 的 ico crate 拒绝（错误信息 `Invalid reserved field value in ICONDIRENTRY`）。手写 ICO 时 byte 3（reserved）必须为 0 |

---

## 11. Out of Scope（明确不做）

以下事项**不要在没有明确指令时做**，即使你觉得"顺手就能加"：

- ✗ 移动端（iOS / Android）
- ✗ Web 端（浏览器内运行）
- ✗ 用户系统、登录、注册
- ✗ 云端数据同步（v0 阶段）
- ✗ 多用户共享同一只水豚
- ✗ 内置 TTS / 语音（后续可能加，v0 不做）
- ✗ 任何打扰用户的主动通知（除了水豚气泡）
- ✗ 自动更新（用户手动下载新版本）
- ✗ 数据分析、埋点、telemetry（隐私第一）
- ✗ 集成任何广告 / 付费墙

---

## 12. Reference Files

- `docs/PRD.md` — 完整产品需求文档（凡涉及"为什么这样设计"的疑问先查这里）
- `docs/ADR/` — 架构决策记录
- `docs/prompts/` — LLM prompt 模板
- `scripts/gen_sprites.py` — 精灵生产辅助脚本
- `.claude/skills/` — 项目专属 skills
- `.claude/hooks/` — 项目专属 hooks

---

## 13. Quick Start for Claude Code（第一次进项目时）

如果你是第一次在这个 repo 工作：

1. 读 `docs/PRD.md` 第 1-5 节，理解产品形态
2. 读本文件全文
3. `git log --oneline | head -20` 看最近做了什么
4. 看 `docs/ADR/` 里最近 3 个决策
5. 问用户："当前要推进 M{N} 的哪一项？"

不要假设、不要自由发挥，**先对齐再动手**。

---

*本文件版本随项目演进。每完成一个里程碑，至少更新第 2 节状态、第 10 节坑的清单、必要时第 7 节决策。*
