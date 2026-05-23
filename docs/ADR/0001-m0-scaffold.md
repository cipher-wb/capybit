# ADR 0001 · M0 工程脚手架的关键决策

- 状态：Accepted
- 日期：2026-05-22
- 范围：M0 里程碑（脚手架 + 透明窗口 + 占位水豚）

## 背景

CLAUDE.md §2 定义的 M0 完成标准里有一条不那么显然的要求：**"点击穿透（水豚区域除外）"**。
透明置顶窗口本身用 Tauri 2 的 `transparent: true / decorations: false / alwaysOnTop: true` 就能搞定，
但"圆形水豚是热区、其余完全穿透"在桌宠领域是个老问题，决定怎么实现是 M0 的最大一块技术债。

## 选项

1. **CSS pointer-events + 整窗接收事件**：透明区域看不见但仍吃事件，会盖住下层窗口的点击。**违背 M0 要求，否决。**
2. **像素级 alpha 嗅探**：每次 mousemove 读 canvas 该点 alpha，决定要不要 `setIgnoreCursorEvents`。问题：穿透打开时 mousemove 根本不会触发，无法在外部检测光标"靠近"水豚。
3. **Rust 后台轮询全局光标位置**：30Hz 轮询 `app.cursor_position()`，与水豚中心比较，跨入/跨出半径时切换 `set_ignore_cursor_events`。前端只要保证水豚始终绘制在窗口正中即可。

## 决策

**采用方案 3**（见 `src-tauri/src/window.rs::spawn_clickthrough_poller`）。

支撑这个方案能成立的几个约束，**全部需要保持**：

- 窗口尺寸固定 `192 × 192`，水豚**永远**绘制在窗口正中（半径 ≈ 78 px）。
  - 这样"光标命中水豚"⇔"光标在窗口几何中心 78 px 圆内"，常数判定，无需把精灵 bbox 从前端回传给 Rust。
  - "水豚走动"靠**移动整个窗口**实现，而不是改窗口内 canvas 中的绘制坐标。
- 占位水豚和未来真精灵都在 `placeholder_capybara.js` / `sprites/*.png` 里**严格控制视觉半径 ≤ 78 px**。
  - `src/assets/sprites/README.md` 的"主体外接圆直径 ≤ 156 px"约束就是为此。

## 影响 / 后果

- **优点**：JS 端零特殊处理；点击穿透对 LLM 气泡、设置面板等 UI 不构成耦合；CPU 30Hz 单 atomic 比较，开销可忽略。
- **缺点**：
  - 未来要支持非圆形 hit-test（例如水豚带气泡、双开），必须升级为"前端回传 hitmask / bbox 到 Rust"的更复杂模型。
  - 多显示器拖动时 `cursor_position()` 返回的是物理像素全局坐标，目前 PoC 没区分多显示器 DPI 差异。Win10 单屏 100%/150% 缩放下实测应该没问题，**第一次接触多屏配置时必须复测**。

## 同时锁定的次要决策

- **状态持久化路径**：`%APPDATA%/dev.cipher.capybit/state.json`（Mac 上为 `~/Library/Application Support/dev.cipher.capybit/state.json`），由 `tauri::path::app_data_dir()` 解析，不写到项目目录。
- **窗口移动持久化**：监听 `WindowEvent::Moved`，250 ms debounce 后写盘。避免每次拖动产生数百次磁盘写。
- **前端零依赖**：通过 `app.withGlobalTauri = true` 暴露 `window.__TAURI__`，避免引入 bundler。
- **托盘菜单**：M0 只放两项 —— "暂时隐身/显示" 和 "退出"。设置/查看记忆/对话窗等晚再加。
- **bundle.active = false**：M0 不出安装包，避免被缺图标卡住。要打包时 `pnpm tauri icon ./your-logo.png` 一次生成全套图标后改回 `true`。

## 后续

- M1 一旦补齐 `sprites/*.png`，无代码改动即生效（自动探测）。
- M5 前要把 PET_HITBOX_RADIUS_PX 从常量改为可配置（不同精灵尺寸需要不同热区）。
