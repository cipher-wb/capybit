# Capybit

一只温柔话痨的桌面水豚伴侣。详见 [`docs/PRD.md`](docs/PRD.md)。

当前阶段：**M2 · 对话核心**（OpenRouter 流式对话 + 气泡 UI + system prompt 装配）。

## 配置 OpenRouter API Key

第一次启动后，应用会在以下位置写入一份模板：

```
%APPDATA%\dev.cipher.capybit\config.local.json
```

打开它，填入你的 OpenRouter key：

```json
{
  "api_key": "sk-or-v1-你的key",
  "model": "anthropic/claude-haiku-4-5",
  "base_url": "https://openrouter.ai/api/v1",
  "fallback_model": "z-ai/glm-4.6"
}
```

没有 key 的话先去 <https://openrouter.ai/keys> 申请（免费注册，需要充几块钱 credit）。

**用法**：双击水豚 → 弹出气泡 → 输入消息回车 → 流式回复。Esc 关闭气泡。

---

## 首次环境准备（Windows）

只需做一次：

### 1. 安装 Rust 工具链

```powershell
# 下载并运行 rustup-init（约 200KB）
winget install --id Rustlang.Rustup
# 或手动：https://rustup.rs/

# 安装完成后，新开一个终端：
rustup default stable
rustc --version   # 应输出 1.78+
```

### 2. 安装 Microsoft C++ Build Tools（Tauri 的链接依赖）

```powershell
winget install --id Microsoft.VisualStudio.2022.BuildTools `
  --override "--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

或手动下载 [Build Tools for Visual Studio](https://aka.ms/vs/17/release/vs_BuildTools.exe)，
勾选 **"使用 C++ 的桌面开发"** 工作负载。

### 3. 安装 WebView2 Runtime（Win10 需要，Win11 已自带）

通常 Edge 已经带了，没有再手动装：
<https://developer.microsoft.com/en-us/microsoft-edge/webview2/>

### 4. 安装 pnpm

```powershell
npm install -g pnpm
```

### 5. 安装本项目依赖

```powershell
pnpm install
```

---

## 开发

```powershell
pnpm tauri dev
```

第一次启动会 `cargo fetch` 下载 ~80 个 crate，预计 5-15 分钟（受网络影响大）。
如果卡住，把 Clash Verge 的系统代理打开，或者：

```powershell
$env:HTTPS_PROXY = "http://127.0.0.1:7890"
$env:HTTP_PROXY  = "http://127.0.0.1:7890"
pnpm tauri dev
```

启动成功后你应该看到：
- 屏幕右下区域出现一只 **纯代码画的 chibi 水豚**（呼吸 + 偶尔眨眼）
- **拖动**它（鼠标按住身体）可以移动；位置自动保存到 `%APPDATA%\dev.cipher.capybit\state.json`
- **鼠标移开水豚区域**后，水豚下方/周围的窗口可正常点击（点击穿透生效）
- **系统托盘**有"暂时隐身/显示"、"退出"两项

## 仅预览前端（不装 Rust 也能看）

直接用任意静态服务器开 `src/index.html` 就能看到占位水豚动画（拖动和持久化不可用，仅视觉预览）：

```powershell
npx serve src
# 然后浏览器开 http://localhost:3000
```

---

## 真精灵图集放哪？

参见 [`src/assets/sprites/README.md`](src/assets/sprites/README.md)。
**TL;DR**：256×256 透明 PNG，按动作命名（`idle_0.png` 等）扔进 `src/assets/sprites/` 即可，
代码会在启动时自动探测并替换占位图，不用改任何代码。

如果你想在 M1 之前临时塞一张真图测试，按那份 README 第二段操作。

---

## 目录速览

```
.
├── CLAUDE.md                  # 工程协作上下文（HOW）
├── docs/
│   ├── PRD.md                 # 产品需求（WHAT/WHY）
│   └── ADR/                   # 架构决策记录
├── src/                       # 前端：原生 JS + Canvas
│   ├── index.html
│   ├── main.js
│   ├── renderer/              # 渲染层：sprite / canvas / motion / placeholder
│   ├── ui/styles.css
│   └── assets/sprites/        # 真精灵图集放这里
└── src-tauri/                 # Rust 后端
    ├── Cargo.toml
    ├── tauri.conf.json
    └── src/
        ├── main.rs / lib.rs
        ├── window.rs          # 透明窗 + 点击穿透轮询 + 移动持久化
        ├── tray.rs            # 系统托盘菜单
        ├── state.rs           # state.json 读写
        └── commands.rs        # 暴露给前端的命令
```

---

## M0 完成检查清单

对照 CLAUDE.md §2：

- [ ] Tauri 2.x 项目可在 Mac 和 Windows 双端构建
- [x] 主窗口：无边框、置顶、透明、点击穿透（除水豚区域）
- [x] WebView 渲染占位水豚
- [x] 拖动 + 位置持久化（state.json）
- [x] 系统托盘 + 退出菜单
- [ ] `cargo build --release` 在 Mac + Win 双端通过

未勾选项需要装好工具链后实跑验证。
