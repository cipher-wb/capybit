# ADR 0009 · 视觉形态切换：从 chibi 渲染到电子宠物点阵屏

- 状态：Accepted
- 日期：2026-05-23
- 范围：替换 ADR 0008 的代码绘制水豚视觉。技术决策（"不用 PNG，用代码绘制"）继承不变；美术方向重做。

## 决策

Capybit 的视觉形态从"一只 chibi 水豚浮在桌面上"改为**一台 90 年代 Tamagotchi 风的手持电子宠物设备**：

- 窗口 192×192 被画成一个**桃色塑料外壳的小机器**（圆角矩形外壳 + 高光 + 阴影 + LOGO + 喇叭洞 + 一颗螺丝点 + 一个底部按钮）
- 外壳中央嵌一块**暗色 bezel 框 + LCD 屏**
- LCD 屏是 **16×16 的像素网格**，每格 6×6 像素，cell 之间留 1px 缝隙模拟真 LCD 的栅格感
- 屏上的"水豚"用 **ASCII 字符艺术**直接写在源码里（`'#' = 亮起, '.' = 熄灭`）

10 个动作（idle / blink / sleep / walk×2 / happy / sad / surprised / curious / stretch / sneeze×2 / talk×2）每个都是手画的 16×16 帧；动作切换 + 帧轮换 + 时间相关的小动画（眨眼、Z 字飘浮、惊讶感叹号、喷嚏粒子、开心心形）由 `chooseFrame` + `drawOverlay` 程序化驱动。

## 为什么

- **ADR 0008 的代码绘制 chibi 水豚被用户判定"太丑"**——本质问题是渐变 + 椭圆叠加不可能做出"真水豚"的有机质感
- **真精灵图集（PRD §6）也不走**——已在 ADR 0008 否决，理由不变
- **点阵 LCD 风格的优势**：
  - "丑得自洽"——90 年代 LCD 本来就是低分辨率点阵，质感来自怀旧 + 设备感而不是写实
  - 容易识别——所有人都见过 Tamagotchi，认知门槛为 0
  - 容易扩展——加新动作只是再写一段 16×16 ASCII，不需要美术
  - 跟"长期陪伴"的产品定位（PRD §1.1）天然契合：电子宠物本就是一个"长伴"的玩具品类
- **CLAUDE.md §7 锁定的"Canvas 2D + 原生 JS"约束仍然成立**——只是画的内容换了

## 工程影响

### 1. 命中区域变成全窗口

ADR 0001 的"圆形 78px hit zone"模型废弃。原因：现在视觉**整个窗口**都是设备（圆角矩形外壳），用户应该在整张设备上都能拖动、悬停。

实现：

- Rust `PET_HITBOX_RADIUS_PX` 从 78 改成 9999——点击穿透轮询的 `inside` 判定永远为 true，等价于"窗口永远 solid，不穿透"
- JS `motion.js::insideHitbox` 改为 `insideCase`，用矩形 bounding box (4-188) 代替圆形距离判断
- JS `main.js` 的 hover tooltip 检测取消圆形限制

副作用：圆角矩形外壳的**四个角**是透明的，但点击那里仍会被本窗口接住（no-op）。这是已知小瑕疵，等真正需要时再做精确的 rounded-rect hit testing。

### 2. ADR 0001 的"水豚永远绘制在窗口正中半径 78px 圆内"约束作废

原约束服务于圆形点击穿透。现在没了那个 invariant，但**窗口尺寸 192×192 仍然不能动**——所有内部坐标都按 192×192 算了死。要改尺寸需要批量改 placeholder_capybara.js 里的常量。

### 3. `src/assets/sprites/` 进一步退化为纪念碑

ADR 0008 已说明这个目录不再读取。本 ADR 不变更那个结论。

### 4. PRD §2.1（形象 / 画风）作废

PRD 里的 "chibi 水豚 + 扁平卡通插画 + 柔和色彩"描述不再适用。形象层的产品描述以本 ADR 为准：电子宠物设备 + LCD 像素水豚剪影。
PRD 其他章节的产品语义（性格、记忆、感知）不受影响。

## 设计参数（位置 / 颜色 / 节奏）

集中在 `placeholder_capybara.js` 顶部常量。改这些数字就能快速调样：

- `CELL = 6`：LCD 每格大小
- `SCREEN_X / Y = 48 / 40`：LCD 在窗口里的位置
- `PALETTE.case*`：外壳配色（peach 桃色）
- `PALETTE.lcd*`：LCD 配色（GameBoy 风暖绿）

10 个动作 ASCII 数据每行严格 16 字符，靠 `parse()` 函数容错（多写少写都不会爆，只是显示截断）。
未来精修视觉时直接改这些 ASCII 表，所有改动一目了然。

## 后续可加（可选）

- **更精致的设备装饰**：天线小球、屏幕保护反光条纹、底部"防滑垫"线
- **多色外壳**：放进 `config.local.json`，让用户挑（桃 / 薄荷 / 粉 / 鼠灰）
- **真正的"按下按钮"**：底部那颗按钮点击响应。按一下 = open_bubble？暂未接线
- **状态图标**：LCD 角落显示 energy/mood 小条（像 Tamagotchi 的饥饿值），可选
- **设备 idle 一段时间显示日期时间**（像真 LCD 待机屏）

## 关联文件

- `src/renderer/placeholder_capybara.js`（核心实现，含 10 个动作的 ASCII 数据）
- `src/renderer/motion.js`（hitbox 改为矩形）
- `src/main.js`（hover 取消圆形限制）
- `src-tauri/src/window.rs`（`PET_HITBOX_RADIUS_PX` 改为 9999）
- ADR 0001（圆形点击穿透方案——本 ADR 之后**作废**那部分约束）
- ADR 0008（代码绘制方向——本 ADR 是它的美术方向 fork）
