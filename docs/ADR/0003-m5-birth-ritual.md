# ADR 0003 · 诞生仪式（PRD §2.3 + §4.1）

- 状态：Accepted
- 日期：2026-05-23
- 范围：M5 中"诞生仪式"这一子项（独立于 M5 其余内容：心流识别、主动说话、vision）

## 决策

### 1. 首次启动判定 = `profile.birth.is_none()`

不依赖 `profile.json` 文件是否存在（profile 文件可能因为别的原因被预创建），用业务字段判定更稳。判定在 `lib.rs::setup` 里，先于 `main` 窗口 `show`。

- 未初始化 → 隐藏 main，显示 `birth` 窗口
- 已初始化 → 跳过仪式，main 正常显示

副作用：手工删 `profile.json` 即可重新触发仪式（开发时方便）。

### 2. 仪式独立窗口而不是改主窗

跟 ADR 0002 关于气泡的选择同理：主窗的 192×192 中心几何约束服务于点击穿透，挪不动。`birth` 窗口是独立 520×420 居中窗口，自带阴影、半透明米色卡片背景，**不参与**点击穿透轮询。

仪式完成后窗口隐藏（不销毁），如果以后做"重新走一次诞生仪式"功能，可以从托盘菜单再 `show()` 回来。

### 3. 摘录选段算法（`birth::scan`）

- 扫描深度 ≤ 3（避免 Obsidian Vault 这种深嵌套吃掉性能）
- 只看 `.md` / `.txt` / `.markdown`
- 按修改时间倒序取最近 30 个候选
- **加权随机** 抽 1 个（recency bias 但有惊喜，符合 PRD §2.3"随机选择或挑出最近修改的"原文）
- 从内容里跳过 YAML frontmatter、markdown 标题、代码块、表格、列表项
- 取**第一个**满足 30-200 字的 blank-line 分段
- 超出 200 字截断到 200 + "…"

未达标的候选自动跳过，最终一个都没有就报 `NoCandidates` 错误，前端引导用户重选文件夹。

### 4. 双字段命名（capybara_name + user_name_for_capybit）

PRD §4.1 的对白模板"我可以叫你 {user_input} 吗"原文有歧义——这里务实地拆为两步：

- **必填**：用户给水豚起名（`profile.name`，替换默认 "Capy"）
- **可选**：用户告诉水豚怎么称呼自己（`profile.user_name_for_capybit`，留空也行，水豚以后自然演化称呼）

两个字段在同一页面填，避免来回切窗。

### 5. 摘录写入 `profile.birth.source_excerpt`

进入 system prompt 的 `{birth_excerpt}` 替换（已在 `llm/prompts.rs::render_system_prompt`），首次启动后**所有**对话都带这段原文上下文，符合 PRD §2.3"作为永久人格锚点"。

### 6. dialog 插件

文件夹选择走 `tauri-plugin-dialog` 的 Rust 端 API（`app.dialog().file().pick_folder()`），不暴露给前端。前端只调我们包了一层的 `pick_birth_folder` 命令。

这样的好处：

- capabilities 里不必给前端开 `dialog` 全套权限
- 未来如果想加"只允许在某些根目录下选"的安全约束，统一在 Rust 一处加

## 未做

- **诞生动画**：PRD §2.3 描述了"飘出、凝结、长出耳朵"的动画过程。当前实现只是文字渐入 + CSS spinner。视觉上的纯 Canvas 颗粒动画留到后续视觉打磨（不影响功能）。
- **首次对话自动开场**：PRD §4.1 步骤 4 的"水豚说出第一句话"目前未自动触发。仪式完成后只是显示主窗，等用户主动双击说话。下一迭代加：仪式完成后 emit `first-contact` 事件 → main.js 自动 `open_bubble` 并展示一段不需要 LLM 的硬编码温柔开场白。
- **重走仪式入口**：托盘菜单未加"重新诞生"项。开发时手删 `profile.json` 即可。
