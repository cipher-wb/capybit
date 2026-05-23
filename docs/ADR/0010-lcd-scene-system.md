# ADR 0010 · 升级到点阵 scene 合成系统

- 状态：Accepted
- 日期：2026-05-23
- 范围：替换 ADR 0009 的外壳装饰，重新定义渲染层的数据模型与扩展接口。

## 决策

Capybit 的视觉形态进一步收缩到**纯 LCD 点阵屏 + 细边框**：

- 窗口 192×192，中央 168×168 = 4px 边框 + 160×160 LCD 屏
- LCD 屏：**32×32 点阵**，每格 5px
- 窗口四角 12px 边距完全透明、点击穿透
- 外壳装饰（peach 塑料 / logo / 喇叭洞 / 按钮）全部移除

但更关键的改动在数据模型层：屏幕内容从"action → sprite"的固定映射变为 **scene 合成系统**。任何代码都能向 LCD 推送一个 Scene 描述符，渲染层把它合成进点阵网格。

## Scene 数据模型

```js
Scene = {
  sprite:    'idle' | 'happy' | 'sleep' | ... | null,   // 16×16 基础位图
  spriteAt:  { x, y },                                  // 网格坐标，默认 (8,10)
  overlays:  Overlay[],
  bg:        'plain' | 'sparkle' | 'stars',             // 可选背景图案
}

Overlay =
  | { type:'icon',  name:string, x, y, blink?, animate? }
  | { type:'text',  text:string, x, y, blink? }
  | { type:'cells', cells:string[]|number[][], x, y }    // 自由像素
```

三层结构组合起来能表达：

- **本体**：从 14 个预设 sprite 中选一个（idle/happy/sad/sleep/walk×2/talk×2/curious/surprised/stretch/sneeze×2/idle_blink）
- **图标**：从 ICONS 库取一个小位图浮在 LCD 上（heart / star / spark / Z / ! / ? / music / sun / moon / cloud / arrow_up / arrow_down / capy_face / dots3 / heart_small）
- **文字**：用内置 5×7 像素字体显示一串 ASCII（数字 + 大写字母 + `: . - ? ! / 空格`），够写温度、时间、单词
- **自由像素**：直接用 0/1 字符数组绘任意位图。**这是给 AI 用的**：让 LLM 输出 ASCII 网格作为 `cells` overlay，前端原样画出来

## 三条 scene 来源（优先级递减）

1. **外部 scene 推送**（`window.__capybit_pushScene(scene, duration_ms)`）—— 最高优先级，TTL 期内屏蔽默认行为
2. **Rust → 前端事件**（`lcd-scene` 事件，由 `commands::set_scene` 命令或 Rust 内部逻辑发出）—— 实际上落到 #1
3. **默认 scene**（`defaultSceneForAction`）—— 当前没有 override 时按 state machine 的 action 计算

机制保证：

- LLM 在对话时（function calling）调 `set_scene` → 屏幕立刻切换到"对应心情的 scene"，过几秒回到默认
- 一段时间没有 LLM 干预 → 屏幕跟着 state machine 自动演化
- 想做天气：未来加 weather perception 模块，每小时 push 一次包含温度数字 + 太阳/云图标的 scene

## "屏幕永远动态"的实现

不依赖 LLM 持续在线。`composeScene` 始终结合 `t`（毫秒级时间）计算：

- **眨眼**：idle/walk/stretch 状态下，每 ~3.8s 双连快眨
- **呼吸**：稍后可加 sprite 偏移 1 cell
- **图标动画**：`animate: 'bob'` 让图标小幅上下漂；`blink: true` 让图标按 1Hz 闪
- **粒子**：sleep 的 Z 字、sneeze 的爆发粒子、happy 的 heart pulse
- **心跳点**：右下角一个永远在闪的 1×1 cell，2Hz，给"设备开着"的存在感

即使整张 scene 是静态 sprite，心跳点 + 偶尔眨眼也保证屏幕不死。

## AI 自由像素的契约（为后续）

LLM 通过 OpenRouter function calling 可以调用：

```
set_scene({
  sprite: 'idle',
  overlays: [
    { type: 'cells', x: 18, y: 2, cells: ["..#..","..#..","#####",".###.","##.##"] },
    { type: 'text', x: 2, y: 26, text: '23C' }
  ]
}, duration_ms: 8000)
```

这把"AI 实时画屏"的能力推到 LLM 本身——它知道屏幕是 32×32 网格，可以自己写 ASCII 拼出心情、表情、天气图案等。本 ADR 不约束 LLM **何时**这样做（那是 prompt 工程的事），只保证**它能这样做**。

工程预期：

- 一开始很可能歪歪扭扭、对不齐——这是预期，自由像素就是 quirky 的味道
- 高质量 AI 像素艺术不在期望内；图标库 + sprite 库才是稳定输出的主力
- token 成本：1 次 32×32 自由像素 ≈ 1024 字符 ≈ ~500 token output，按 Haiku 估约 ¥0.02 / 次，可控

## 命中区域

ADR 0009 的"窗口全 solid"作废。LCD 框尺寸是 168×168 正方形，因此：

- Rust `PET_HITBOX_RADIUS_PX` 改为半边长 84（命名暂未重构）
- 命中判定从圆形距离改为方形 `abs(dx) < 84 && abs(dy) < 84`
- 窗口角落 12px 边距真正透明 + 点击穿透

motion.js 的 `insideCase` 同步改成矩形 `[12, 180]`。

## 影响 / 未做

- **窗口尺寸保持 192×192**：暂不缩到 168×168，留个外缘呼吸空间和未来加状态信息（如气泡、悬停提示）的位置
- **LLM function calling 还没接**：set_scene 命令准备好了，但没在 system prompt 里告诉模型有这个工具。下一步加，需要写 OpenRouter tools 调用的代码（属于 §3.2.3 function calling 列表的扩展）
- **字体还很穷**：5×7 fontk 只覆盖大写字母 + 数字 + 几个符号，不含小写、中文、emoji。够写"23C"、"AM"、"HI"，写不了中文。中文要么用 8×8 字体（每字符占大半屏），要么用拼音/英文转写
- **没接对话情绪联动**：用户对话 happy → 屏幕显示心形 这种联动还需要 LLM 工具调用接好。当前可手动通过 DevTools `window.__capybit_pushScene` 测试

## 关联文件

- `src/renderer/placeholder_capybara.js`（整体重写为 scene 合成器）
- `src/main.js`（监听 `lcd-scene` 事件）
- `src/renderer/motion.js`（hitbox 矩形改为贴 LCD 框）
- `src-tauri/src/window.rs`（hitbox 半边长 84）
- `src-tauri/src/commands.rs`（新增 `set_scene` 命令）
- `src-tauri/src/lib.rs`（注册 `set_scene`）
- ADR 0009（外壳装饰方案 ——本 ADR 替换其美术决策；scene 数据模型部分是新内容）
