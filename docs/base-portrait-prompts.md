# Capybit · 基准立绘 Prompt 工程文档 v1

> 这是 PRD 第 6.2 节 (Step 1) 的具体落地文档。目标：产出一张 `base.png` 作为后续所有动作帧的参考图基准。
>
> 路径建议：`docs/prompts/sprites/base-portrait-prompts.md`

---

## 0. 写在最前 · 水豚化 chibi 的核心陷阱

水豚的可爱 ≠ 萌系动物的可爱。所有 prompt 必须严格遵守以下"水豚特征锚点"，否则生成出来的就是一只"长得有点像水豚的米色小动物"，灵魂不对：

| 必须保留 | 必须避免 |
|---|---|
| **小眼睛**（占脸部 5-10%） | 大眼睛、闪光眼、anime 眼 |
| **钝鼻吻部**（圆润，不尖） | 尖嘴、长鼻、狐狸/狗嘴 |
| **桶状/梨形身体**（圆胖） | 瘦长身体、人形比例 |
| **短粗腿**（stub legs） | 长腿、拉伸的人形腿 |
| **小半圆耳朵**（贴头） | 大耳朵、长耳朵、立耳 |
| **永远淡定的表情**（deadpan calm） | 夸张表情、大笑、皱眉 |
| **慵懒钝感** | 灵动、机灵、警觉 |

记住：**水豚是看起来在思考但其实只是在发呆的动物**。这股劲儿要在静态立绘里就传达出来。

---

## 1. 三个候选画风方向

### 方向 A · 现代绘本插画 ⭐ 推荐

**调性**：克制、诗意、有手作感、像被画在一本独立绘本里。  
**适合场景**：长期桌面常驻不审美疲劳；跟"从你的文字里诞生"的设定意境契合；跟你写网文的身份呼应。  
**视觉灵感**（仅供 prompt 描述参照，**不要在 prompt 里直接 cite 艺术家姓名**）：当代独立绘本的克制美学、限定色板、纸质感。

---

### 方向 B · 治愈系软萌

**调性**：温暖、cozy、像一杯热茶、想伸手摸。  
**适合场景**：希望水豚更讨喜、更易传播、开源后用户接受度高的情况。  
**视觉灵感**：独立游戏的角色设计美学（咖啡馆故事、农场模拟）的温润质感。

---

### 方向 C · 极简符号化

**调性**：冷静、几何、像一个被精心设计的图标。  
**适合场景**：你的桌面是开发者极简风格，水豚要"融入"而不是"打扰"。  
**视觉灵感**：现代科技公司的官方插画语言、扁平几何。

---

## 2. 方向 A 完整 Prompt（推荐版）

### 2.1 主 Prompt（适用于 Midjourney v6 / Flux / 即梦）

```
A chibi capybara character, single subject centered, side view facing right, full body visible from head to toe, standing in a neutral relaxed pose with a tiny perpetual quiet smile. Small eyes, blunt rounded snout, pear-shaped chubby body, short stubby legs, small half-circle ears tucked close to head. Calm deadpan expression — the iconic capybara serenity.

Style: modern indie picture book illustration. Hand-drawn feel with subtle textured brush strokes, like ink and gouache on warm cream paper. Limited color palette: 4-5 muted earthy colors. Subtle paper grain texture overlay across the entire illustration. No outline, or very thin organic outline same hue as body but slightly darker.

Color palette (strictly stick to these):
- Body: warm oat milk beige (#E8D4B5)
- Belly accent (lighter zone on lower body): soft cream (#F4E6CC)  
- Eyes: dark soft brown dots, very small (#3D2914)
- Nose: tiny dot same as eyes (#3D2914)
- Optional: barely-visible dusty pink cheek (#E8B5B5), 2% opacity

Composition:
- Character occupies about 55-60% of frame, fully visible with breathing room
- Centered horizontally and vertically
- Square frame, 1:1 aspect ratio
- Pure transparent background, no shadow under feet, no ground line, no decorative elements

Mood: quiet, gentle, intelligent in a slow water-soaked way. A creature with all the time in the world. Like the visual equivalent of a long deep breath.

Technical: high quality digital illustration, transparent PNG output, 1024x1024, character full body within frame with margin.
```

### 2.2 Negative Prompt（SD / Flux / Liblib 用）

```
3d render, photorealistic, anime, manga, big eyes, shiny eyes, sparkle eyes, kawaii sparkles, hearts, stars, decorative ornaments, harsh shadow, drop shadow, ground shadow, gradient background, white background, multiple characters, two capybaras, text, watermark, signature, glossy plastic shading, dramatic lighting, rim light, lens flare, smooth airbrush, vector clean style, anime hair shine, sharp edges, vector outline, character outline thick black, ink outline heavy
```

### 2.3 推荐色板（你可以直接喂给图像工具，或者后期校色）

```
#E8D4B5  ████████  oat milk body (主体)
#F4E6CC  ████████  cream belly (浅腹)
#3D2914  ████████  dark eye dots (五官)
#5A3D24  ████████  nose (鼻子, 可选)
#E8B5B5  ████████  subtle blush (脸颊, 2% 不透明度)
```

---

## 3. 方向 B 完整 Prompt（治愈软萌版）

### 3.1 主 Prompt

```
A chibi capybara character, single subject centered, side view facing right, full body visible, standing in a relaxed slightly sleepy pose with a soft warm smile. Small half-lidded sleepy eyes, blunt rounded snout, very round barrel-shaped chubby body like a warm bread loaf, short stubby legs, small soft ears. Cozy calm capybara essence, not anime cute, not big-eyed cute.

Style: cozy soft cute illustration in the warm tradition of indie game character art. Smooth digital painting with soft edges, gentle warm interior lighting (no harsh light source), slightly painterly brushwork. Slight rim of warmth along the top of the body.

Color palette:
- Body: warm honey beige (#D9B380)
- Belly: soft cream (#F0DBB8)
- Inner ears: dusty rose (#C58F8F)
- Eyes: deep warm brown (#4A3326), small half-lidded
- Cheek blush: subtle warm pink (#E8B59E)

Details:
- Eyes are small dark ovals, half-closed sleepy look, with one tiny white highlight per eye (just 1 pixel feel)
- Mouth is a small gentle "u" curve, very slight, suggesting quiet contentment
- Soft body outline same color family as body but darker, not black
- Subtle warm gradient on the underside (very gentle, no hard shadow)
- The capybara looks soft to touch

Background: pure transparent PNG, no shadow under character, no ground, no border, no decoration.

Mood: the feeling of holding a warm cup of tea on a rainy afternoon. Invites you to want to pet it.

Technical: high quality digital illustration, transparent PNG, 1024x1024, character occupies 55-60% of frame, centered.
```

### 3.2 Negative Prompt

```
3d render, photorealistic, anime, manga, big anime eyes, shiny sparkle eyes, kawaii sparkles, hearts, stars, harsh shadow, drop shadow, gradient background, white background, multiple characters, text, watermark, glossy plastic, lens flare, overly saturated colors, neon colors, vector flat, hard outline, thick black outline, sharp geometric edges
```

### 3.3 色板

```
#D9B380  ████████  honey beige body
#F0DBB8  ████████  cream belly
#C58F8F  ████████  inner ears
#4A3326  ████████  eye color
#E8B59E  ████████  cheek blush
```

---

## 4. 方向 C 完整 Prompt（极简符号化版）

### 4.1 主 Prompt

```
A minimal geometric capybara character, single subject centered, side view facing right, full body visible, standing in a neutral pose with a barely-there smile. Small dot eyes, blunt rounded snout, clean oval body, simple rounded-rectangle legs, small semicircle ears. Strict geometric simplification — the capybara built from circles, ovals, and rounded rectangles.

Style: modern flat geometric illustration, the visual language of contemporary tech-brand illustrations. Pure flat colors, absolutely no gradients, no textures, no brush strokes. Clean vector-style shapes but rendered with very slightly soft edges (not razor-sharp). Highly designed feel.

Color palette (strictly 3 + 1 accent):
- Body: muted warm taupe (#B8A088)
- Belly zone: lighter taupe (#D4C4AC)
- Eye + nose dots: deep charcoal (#2C2823)
- Optional accent (very small cheek area): muted coral (#D49284)

Details:
- Eye is ONE small solid dark dot, 3% of body width
- Nose is ONE tiny dark dot
- Mouth is a single thin line, very subtly curved upward, almost imperceptible
- No outline, OR if outline used: same color family as body, very thin
- ZERO shadows on the character
- Optionally one very subtle lighter spot on top of head as highlight (but no required)

Background: pure transparent PNG, no shadow, no ground, no decoration, no border.

Mood: calm, modern, intelligent. Like a thoughtfully designed app icon that happens to be alive. The kind of capybara that lives on a developer's desk.

Technical: high quality vector-style flat illustration, transparent PNG, 1024x1024, character occupies 50-55% of frame, centered. Clean and minimal.
```

### 4.2 Negative Prompt

```
3d render, photorealistic, anime, manga, big eyes, sparkle, kawaii, decorations, hearts, stars, texture, paper texture, brush strokes, sketch lines, pencil lines, gradient on character, shadow on body, painterly, watercolor, drop shadow, ground shadow, complex background, multiple characters, text, watermark, glossy, plastic look, soft airbrush, fluffy fur texture, hair detail, whiskers
```

### 4.3 色板

```
#B8A088  ████████  taupe body
#D4C4AC  ████████  light belly
#2C2823  ████████  charcoal accent
#D49284  ████████  coral cheek (可选)
```

---

## 5. 不同工具的 Prompt 适配建议

### 5.1 Midjourney v6 / v7

- 主 prompt 复制粘贴可直接用
- 末尾追加参数：`--ar 1:1 --no background --style raw --s 100`
- `--no background` 可帮助产生更干净的透明背景
- 风格描述放前面（"Style: ..."），主体描述放最前面
- 一次生成 4 张，挑最优的 upscale
- 透明背景能力一般，可能需要后期 rembg

### 5.2 Flux Schnell / Flux Dev / Flux Kontext

- Flux 喜欢自然语言长描述，可以保留所有形容词
- Flux Dev 比 Schnell 更尊重 prompt 细节
- Flux Kontext 是图像编辑模型，**不用于生成 base.png**，用于后续动作帧
- 透明背景：直接在 prompt 末尾强调 `transparent PNG output, no background`

### 5.3 Nano Banana (Gemini 2.5 Flash Image)

- 这是图像编辑模型，**Step 1 阶段不用它**
- Step 2 动作帧生成时它是首选
- Step 1 期间它可以用于"微调已有立绘"（输入一张近似立绘 + 修改指令）

### 5.4 即梦（字节）

- 中文 prompt 也支持，可以双语混合
- 透明背景支持较好（"背景：透明"）
- 关键词版本：把上面英文 prompt 翻译成中文短句更友好
- 提供参考图功能，可以先用 MJ 生成 1 张再喂给即梦做风格统一

### 5.5 Liblib.ai / Stable Diffusion (SDXL / Flux 本地)

- 喜欢 booru-style 逗号分隔的关键词，不喜欢长自然语言
- 转换：把主 prompt 拆成关键词列表
- 例（方向 A）：
  ```
  chibi capybara, side view, facing right, full body, small eyes, 
  blunt snout, pear body, short stubby legs, oat milk beige color, 
  picture book illustration, hand drawn, paper texture, flat colors, 
  limited palette, soft brush, deadpan expression, gentle smile, 
  transparent background, no shadow, centered composition,
  high quality, masterpiece
  ```
- 配合 LoRA：去 Liblib / Civitai 搜 "capybara" / "picture book style" / "flat illustration"
- 必须配 Negative Prompt
- 推荐采样：DPM++ 2M Karras，Steps 28-32

---

## 6. Prompt 工程通用技巧

### 6.1 同 seed 复用

第一次生成出来一张满意的立绘后，**记下它的 seed 值**。后续所有动作帧生成（Step 2 用图像编辑模型）时，同 seed + 同参考图 → 一致性提升 30-50%。

### 6.2 风格关键词固定模板（提取通用部分）

后续所有动作帧 prompt 都应该包含这一段"风格锚"（以方向 A 为例）：

```
[STYLE_ANCHOR]
chibi capybara, side view facing right, small eyes, blunt rounded snout, 
pear body, short stubby legs, deadpan calm expression, modern picture book 
illustration, paper texture, limited muted palette (oat milk beige, cream, 
dark brown dots), flat colors, soft brush, transparent background, no shadow.
```

Step 2 时每个动作 prompt = `[STYLE_ANCHOR]` + `[ACTION_DESCRIPTION]`。

### 6.3 接受"漂移"的预算

不要追求 100% 一致。规则：

- **必须一致**：物种、朝向、色板、整体比例
- **可以微浮**：耳朵位置 ±2px、嘴巴弧度、毛色深浅小范围
- **应该一致**：眼睛大小、身体形状

如果某一帧偏差太大，**重新生成而不是手动改 prompt**。重生成成本远低于手动修。

### 6.4 关于"诞生于文字"的视觉暗示（可选 easter egg）

如果你想给水豚加一点"从你的文字里诞生"的诗意暗示，可以在 prompt 里 **极轻微**地加：

```
faint hint of ink or letter shapes subtly visible within the body texture, 
extremely subtle, almost imperceptible
```

但谨慎用：很容易让形象变得过度装饰、失去克制感。建议 v0.1 不加，v1.0 再考虑。

---

## 7. Step 1 完整执行流程

```
[Day 1 上午]
1. 选定一个画风方向（A / B / C），把对应主 prompt 准备好
2. 选定 1-2 个图像工具（推荐 Midjourney + 即梦，或 Flux Dev + 即梦）

[Day 1 下午]
3. 在选定工具上跑 30-50 张候选
4. 第一轮筛选：保留 5-10 张"水豚特征对、风格统一"的
5. 第二轮筛选：从这 5-10 张里挑 3 张作为 finalist
6. 把 3 张并排打开，对比哪张最有"话痨温柔"的气质

[Day 2 上午]
7. 选定 1 张作为 base.png 候选
8. 如果还差一点，用 Nano Banana / Flux Kontext / 即梦的"参考图微调"功能
   做局部修正（例：耳朵小一点、嘴巴再上扬一点）
9. 用 rembg 处理透明背景（即使原图已是透明，再过一遍清理边缘）
10. 用 Pillow 脚本统一到 256×256 画布，水豚脚部锚定到固定 Y 位置
11. 命名 base.png，保存到 docs/sprites_dev/

[Day 2 下午]
12. 记录：base.png 用的 prompt、seed、工具、参数，写入 docs/sprites_dev/base.meta.md
13. 这张 base.png 就是后续所有动作帧的"圣经"
```

---

## 8. 失败案例预警

容易踩的几个坑：

| 现象 | 可能原因 | 调整 |
|---|---|---|
| 生成出来是大眼睛萌系动物 | "chibi" 关键词触发了 anime 模式 | 删除 "chibi"，改为 "small rounded character"；加强 negative |
| 水豚像猪/像河狸/像仓鼠 | 没强调"blunt rounded snout, pear body" | 这两个特征必须保留 |
| 出现了人形比例（站立、长腿） | 模型默认了拟人化 | 加 "quadruped, four short stubby legs, NO bipedal pose" |
| 颜色偏离色板 | 模型没严格执行色板 | 在 prompt 里重复色板，每个颜色描述两遍 |
| 透明背景里有杂边/灰边 | 工具原生透明不彻底 | 必须过 rembg 后处理 |
| 三视图、多角色 | 模型生成了"参考图"风格 | 加 "single subject, one capybara only" 强调 |
| 阴影粘在脚下 | 模型默认加了 ground shadow | negative prompt 里强调 no ground shadow |

---

## 9. 我的最终推荐与理由

**推荐方向 A（现代绘本插画）**，理由：

1. **跟产品灵魂同频**：水豚是"从你的文字里诞生"的，绘本式的纸质感和限定色板，是这个设定最贴的视觉语言
2. **桌面常驻友好**：限定色板和柔和的笔触不会在长期可见的位置造成视觉疲劳
3. **与用户身份呼应**：作者是网文写作者，"书的世界"感觉是一种内嵌的彩蛋
4. **AI 一致性更高**：限定色板和扁平笔触比软萌的渐变更容易让 AI 跨多个动作保持一致

如果你的优先级是"开源后获得更多人喜欢"，可以考虑方向 B。
如果你的优先级是"极致克制不打扰"，可以考虑方向 C。

---

## 10. 选定后的下一步

确定方向后，输入这条指令给 Claude Code（或者直接告诉我）：

> 我选定方向 X 作为基准画风。请基于 docs/prompts/sprites/base-portrait-prompts.md 的方向 X，帮我：
> 1. 生成 Step 2 阶段 10 个动作的完整 prompt 包
> 2. 写一份 scripts/gen_sprites.py 后处理脚本（rembg + Pillow normalize）
> 3. 更新 docs/sprites_dev/base.meta.md 模板

---

*本文件版本 v1。每次生成结果反馈后可能迭代。变更记录写在文件底部。*

## 变更记录

- v1 (2026-05-22): 初版，三方向 + 工具适配 + 执行流程
