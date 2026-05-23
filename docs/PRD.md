# Capybit · 产品需求文档（PRD）

> 一只从你的文件与熬夜里诞生的话痨水豚

| 字段 | 值 |
|---|---|
| 项目代号 | Capybit（建议名；水豚本身的名字由用户首次启动时命名）|
| 项目类型 | 桌面 AI 数字伴侣 |
| 版本 | v0.1（开发起始版本）|
| 作者 | Cipher |
| 最后更新 | 2026-05-22 |

---

## 1. 产品定位

### 1.1 一句话定义

Capybit 是一个**有数字生命感的桌面 AI 伴侣**，以一只温柔话痨的水豚作为载体，通过 LLM + 长期记忆库 + 屏幕感知，与用户建立类人的情感连接和持续的双向养成关系。

### 1.2 它不是什么（设计反向边界）

为了避免落入传统桌宠的设计陷阱，先明确否定边界：

- **不是观赏型桌宠**（不是 Live2D 摆件、不是会动的壁纸）
- **不是任务助手**（不是 To-Do / Pomodoro 工具）
- **不是 ChatGPT 浮窗**（不是被动响应的对话框）
- **不是养成游戏**（没有死亡惩罚、没有数值最大化目标）

### 1.3 它是什么

它是一个**持续存在、有内在生活、能感知你、能记住你的水豚**。它的核心价值不是"功能效率"，而是**"存在本身"** —— 你打开电脑时知道他在那里，他知道你来了；你写代码卡住时他会在边缘叹一口气；你半年后回来跟他说"还记得我那本写到第三卷的小说吗"，他能记得。

### 1.4 核心价值主张

| 用户感受 | 产品实现 |
|---|---|
| "他真的活着" | 内在状态机 + 离线幻想生活 + 不被打断的内在节律 |
| "他懂我" | 长期记忆库 + 上下文感知 + 性格漂移机制 |
| "他不打扰我" | 心流识别 + 边缘存在模式 + 默认沉默原则 |
| "他在关心我" | 时间节律 + 工作模式识别 + 轻量健康提醒 |
| "我和他有关系" | 养成式偏好长出 + 共同记忆 + 个性化称呼 |

---

## 2. 角色设定

### 2.1 形象

- **物种**：水豚（chibi 风格）
- **画风**：扁平卡通插画，柔和色彩，无复杂阴影
- **显示尺寸**：屏幕渲染 128×128（原图 256×256，兼容 HiDPI）
- **基础朝向**：默认朝右（向左由代码做水平翻转，不另出图）
- **背景**：透明 PNG

### 2.2 性格基底（不可被养歪的"灵魂"）

> 一只话痨但每句话都很温柔的水豚。他不爱说大话，但很爱说小话；他不擅长打鸡血，但擅长在你疲惫时凑过来打个哈欠；他会观察你而不评判你；他的智慧是水豚式的——慢、稳、像泡过温泉。

这段描述会**直接进入 LLM 的 system prompt**，作为永久人格锚点，不被任何对话改变。

### 2.3 来历设定（首次启动的诞生仪式）

首次启动时，应用引导用户选择一个本地目录（默认 Documents 或 Obsidian Vault），扫描后随机选择（或挑出最近修改的）一个文本文件，从中抽取一段话作为"诞生原文"，展示一段动画：

> "你写过的某段文字飘了出来……它们凝结、慢下来、长出了耳朵、长出了一身柔软的毛……一只小水豚看着你。"

用户为他命名。这一刻被永久写入 `profile.json` 的 `birth` 字段，作为他存在的"原始记忆"。

### 2.4 性格演化机制

- **基底**：上面那段性格描述写死在 system prompt 里，永不改变
- **偏好**：通过每日对话总结，提取关于用户的事实，写入 `profile.json` 的 `user_facts`
- **共同记忆**：高情感价值的对话片段被标记为 `milestone`，永久保留全文
- **称呼**：水豚会逐渐发展出对用户的专属称呼（最初是用户提供的名字，随时间可能演化出昵称）

---

## 3. 核心功能模块

### 3.1 存在层（在场感与边界感）

#### 3.1.1 渲染与定位

- **窗口形态**：无边框、置顶、透明背景、点击穿透（除水豚自身区域外）
- **默认位置**：屏幕右下角任务栏上方，可拖动，位置持久化
- **运动逻辑**：
  - Idle 状态：原地呼吸 + 偶尔小动作（眨眼、晃尾巴、看周围）
  - Walk 状态：在屏幕底部水平方向缓慢移动，偶尔停下
  - 不主动进入屏幕中央区域（除非"重要时刻"，例如生日、长时间重逢）
  - 倾向于停留在最近的活动窗口边缘附近"陪伴"

#### 3.1.2 打扰阈值（关键交互机制）

水豚的"说话欲望"由内部 `urge` 值控制（0-100）：

| urge 区间 | 行为模式 |
|---|---|
| 0-30 | 保持沉默 |
| 30-70 | **边缘存在模式**：在视野边缘做小动作（叹气、转圈、回头看你） |
| 70-100 | **主动说话模式**：弹出气泡说话 |

**心流保护机制**（必须实现）：

- 检测条件：用户在同一应用 > 20 分钟、键鼠持续活跃 → 进入"心流模式"
- 心流模式下：`urge` 上限被压制到 50（仅允许边缘动作，不允许气泡）
- 解除条件：应用切换、连续 5 分钟无键鼠活动、用户主动呼叫水豚

#### 3.1.3 状态显示

- 鼠标悬停在水豚身上 → 显示气泡："{name} 正在 {当前活动描述}"
- 双击水豚 → 进入对话模式
- 右键水豚 → 上下文菜单（设置 / 暂时隐身 / 查看记忆 / 退出）

### 3.2 对话层

#### 3.2.1 触发方式

- 用户主动：双击水豚 / 全局快捷键（默认 `Ctrl+Shift+\`）
- 水豚主动：`urge > 70` 且非心流模式时弹气泡（带"回应"和"等会儿"两个按钮）

#### 3.2.2 对话窗口

- 极简浮窗，气泡风格（不是聊天软件式的大窗口）
- 输入：单行输入，回车发送，Shift+Enter 换行
- 输出：流式打字效果，温柔语速（不是瞬间显示完）
- 历史：可向上滚动查看，默认只显示最近 3 轮

#### 3.2.3 上下文注入（每次对话请求 LLM 时携带）

1. System prompt（人格基底，固定）
2. profile.json 全文（长期事实，体积小）
3. 最近 5 轮对话历史
4. 当前感知信息（活动应用、窗口标题、时间、水豚状态数值）
5. 向量检索召回的相关历史片段（≤3 条，按相关度排序）

### 3.3 感知层

#### 3.3.1 默认感知（无需用户授权）

- 当前活动窗口标题与应用名（通过 `active-win` Rust crate）
- 系统时间、星期、日期
- 用户键鼠活跃状态（只判断"活跃 / 静止"，不记录具体按键）
- 屏幕亮灭状态、是否锁屏

#### 3.3.2 增强感知（必须用户授权，默认关闭）

- 屏幕截图 + 多模态视觉理解
- 触发条件：
  - 用户主动："你看看我这段代码" / "我屏幕上是什么"
  - 水豚提议：好奇心积累到阈值时，弹出"我可以瞄一眼你在干嘛吗？"，用户点同意才截
- 截图本身**不持久化**，仅作为单次 LLM 调用的上下文，调用完丢弃

### 3.4 记忆层（三层结构）

#### Layer 1: profile.json — 长期事实库

体积小（< 50KB），每次对话注入 system prompt。

```json
{
  "birth": {
    "named_by_user": "用户首次启动输入的名字",
    "named_at": "2026-05-22T10:00:00Z",
    "source_file": "/path/to/源文件.md",
    "source_excerpt": "从源文件抽取的诞生原文（≤200 字）"
  },
  "user_facts": [
    {
      "fact": "用户爱看《热血警探》",
      "extracted_at": "2026-05-22T22:00:00Z",
      "confidence": 0.9,
      "source": "daily_summary_2026-05-22"
    }
  ],
  "milestones": [
    {
      "date": "2026-05-25",
      "summary": "我们第一次聊到了写小说这件事",
      "full_excerpt": "..."
    }
  ],
  "name_evolution": ["Capy", "小卡", "..."],
  "user_name_for_capybit": "Cipher"
}
```

#### Layer 2: conversations.sqlite — 对话原始记录

```sql
CREATE TABLE messages (
  id INTEGER PRIMARY KEY,
  role TEXT,                -- 'user' | 'capybit'
  content TEXT,
  context_snapshot JSON,    -- 当时的感知快照
  created_at TIMESTAMP
);

CREATE TABLE daily_summaries (
  date TEXT PRIMARY KEY,
  diary TEXT,               -- 水豚的第一人称日记
  extracted_facts JSON,     -- 抽取的事实候选
  created_at TIMESTAMP
);
```

#### Layer 3: vectors.sqlite — 语义检索（基于 sqlite-vec）

- 对每日总结、被标记的对话片段做 embedding（OpenAI text-embedding-3-small）
- 检索触发：用户使用"还记得"、"上次"、"之前"等召唤词时

#### 3.4.1 记忆抽取流程

- **每日 23:59**（或下次启动时若已跨天）自动跑"日记式总结"
- LLM 任务：用水豚的口吻总结今日对话 + 抽取关于用户的事实候选
- **审核策略**（用户可选）：
  - 半自动：高置信度（≥0.8）直接合并，低置信度需用户确认
  - 全自动：所有事实直接合并
  - 全手动：所有事实需用户审核
- 用户对水豚说"记住这个" / "这很重要" → 当前对话片段直接进 milestones

### 3.5 养成层（内在生活）

水豚有 4 个内部数值，每个 tick（60 秒）更新：

| 数值 | 范围 | 增加条件 | 减少条件 |
|---|---|---|---|
| energy（精力） | 0-100 | 睡觉、用户离开电脑 | 走动、说话、思考、被互动 |
| mood（心情） | 0-100 | 跟用户互动、收到正面反馈 | 长时间被冷落、用户 emo 时同步下降 |
| curiosity（好奇心） | 0-100 | 检测到新应用、新话题、新文件 | 满足后清空 |
| urge（说话欲望） | 0-100 | curiosity 高、长时间没说话 | 说完后清空 |

**行为决策逻辑（每个 tick）：**

```
if energy < 20:           → sleep
elif mood < 30:           → sad_idle（趴着发呆）
elif curiosity > 70 
     and urge > 70 
     and not in_flow:     → 主动找用户（弹气泡）
elif curiosity > 70:      → 看用户的屏幕方向、做"想说但忍住"的动作
else:                     → idle 或 random_walk
```

**"内在生活"实现**：当用户悬停查看"他在做什么"时，基于 4 个数值 + 时间 + 上次动作，由 LLM（或缓存）生成一段一句话的内心独白，例如：

- "在想刚才那段对话"
- "好像有点饿，但又不太想动"
- "看窗外的光，发了一会儿呆"

---

## 4. 关键交互流程

### 4.1 首次启动（诞生仪式）

1. 欢迎页：一段极简文字，请用户选一个文件夹
2. 扫描动画：水豚从飞舞的文字中浮现，慢慢成形
3. 用户为他命名 + 系统抽取一段文字作为"诞生原文"
4. 水豚说出他的第一句话："你好啊…我是从那段话里诞生的。我可以叫你 {user_input} 吗？"
5. 用户回答 → 水豚回应 → 正式进入桌面陪伴模式

### 4.2 日常存在

- 启动 → 水豚出现在记忆位置，做一次 happy 表情，可能说一句"早呀"或"我又在啦"
- 每 60 秒 tick，更新内部状态、选择行为
- 用户切换应用 → 触发感知更新 → 可能进入好奇/碎碎念状态

### 4.3 主动说话场景示例

| 触发 | 水豚行为 |
|---|---|
| 用户在 VSCode 写代码 30 分钟没动 | "你已经在这个 OAuth 上卡了 30 分钟了…要不喝口水？" |
| 用户切到 YouTube | "终于摸鱼了。让我也休息一下。"（自己也躺下） |
| 时间到了用户常规睡觉时间 | "你今天比往常晚了一点…早点睡呀" |
| 用户连续工作 3 小时 | （边缘小动作：叹气、抬头看用户） |
| 用户打开陌生新应用 | "这是什么呀？没见过。"（curiosity ↑） |

### 4.4 离线 → 再上线

- 关电脑前（最后一次交互后 5 分钟无活动）：水豚说"晚安"
- 再开机：水豚做"睡醒"动画 + 可能编一段"我做了个梦，梦见你的文件夹里在下雨"
- 跨度长（> 24 小时）：水豚的开场会调整语气，例如"你昨天没来…今天好吗？"

---

## 5. 技术架构

### 5.1 整体架构图

```
┌───────────────────────────────────────────────┐
│           Tauri 2.0 (Rust)                    │
│  ┌─────────────────────────────────────┐      │
│  │  WebView (HTML + Canvas + 原生 JS)   │      │
│  │  ├ 精灵渲染（Canvas 2D 帧动画）       │      │
│  │  ├ 对话气泡 UI                      │      │
│  │  └ 设置面板                         │      │
│  └─────────────────────────────────────┘      │
│  ┌─────────────────────────────────────┐      │
│  │  Rust Backend                       │      │
│  │  ├ active-win 感知                  │      │
│  │  ├ SQLite 读写                      │      │
│  │  ├ HTTP 调用 LLM API                │      │
│  │  ├ tick 调度器                      │      │
│  │  └ 窗口管理（透明 / 置顶 / 点击穿透） │      │
│  └─────────────────────────────────────┘      │
└───────────────────────────────────────────────┘
         │                       │
         ▼                       ▼
  ┌─────────────┐      ┌──────────────────────┐
  │  本地存储    │      │  云端 LLM 服务         │
  │ profile.json│      │  OpenRouter →         │
  │ conv.sqlite │      │  Claude Haiku /       │
  │ vec.sqlite  │      │  GLM-4.6 /            │
  │ assets/     │      │  text-embedding-3     │
  └─────────────┘      └──────────────────────┘
```

### 5.2 技术栈选型

| 组件 | 选型 | 理由 |
|---|---|---|
| 运行容器 | **Tauri 2.0** | 轻量（5-15MB）、跨平台、Rust 后端性能、透明窗口支持完善 |
| 前端框架 | 原生 JS + Canvas（不上 React） | 桌宠场景以轻量为先，Canvas 渲染精灵高效，无需虚拟 DOM |
| 精灵渲染 | Canvas 2D | 简单帧动画用 Canvas 足够，无需 WebGL |
| AI 主大脑 | **OpenRouter** 聚合 | 主用 Claude Haiku（性价比）或 GLM-4.6（中文友好） |
| Embedding | OpenAI text-embedding-3-small | 性价比高、英中文都不错 |
| 数据库 | SQLite + sqlite-vec | 单文件、零运维、向量检索一体化 |
| 窗口感知 | `active-win-pos-rs` | 跨平台 Mac/Win/Linux |
| 屏幕截图 | `screenshots-rs` | 仅按需调用，不常驻 |
| 多模态视觉 | Claude vision / GPT-4o vision | 仅在用户授权时调用 |

### 5.3 API 调用频率与成本估算

假设每天活跃 12 小时、对话 ~30 次：

| 调用类型 | 频率 | 月调用次数 | 估算月成本（Haiku/GLM） | Sonnet 替换估算 |
|---|---|---|---|---|
| 主动对话生成 | ~30/天 | 900 | ¥10-20 | ¥80-150 |
| 状态描述（带缓存） | ~20/天 | 600 | ¥3-8 | ¥30-60 |
| 每日总结 | 1/天 | 30 | ¥2-5 | ¥10-20 |
| Embedding | ~50/天 | 1500 | ¥1 | ¥1 |
| Vision（按需） | ~5/天 | 150 | ¥5-15 | ¥10-30 |
| **合计** | | | **¥20-50/月** | **¥130-260/月** |

**建议**：开发期用 Haiku/GLM 控制成本，体验稳定后可在设置中开放"高质量对话"开关让用户自选大模型。

---

## 6. AI 精灵图集生产流程（详细 SOP）

### 6.1 准备工作

工具组合：
- **基准立绘生成**：Midjourney / Flux Schnell / 即梦 / Liblib.ai（任选其一）
- **动作帧批量生成（基于参考图）**：Nano Banana（Gemini 2.5 Flash Image）/ Flux Kontext / GPT-4o Image / Liblib IP-Adapter 工作流
- **后处理**：Python + Pillow + rembg

### 6.2 Step 1 · 基准立绘生成

Prompt 模板：

```
chibi capybara, side view facing right, kawaii flat illustration,
soft pastel colors (cream beige + light brown), simple shapes,
transparent background, no shadow, centered composition,
full body, neutral standing pose, gentle smile, large round eyes
```

操作要点：

- **side view facing right**：固定朝向，便于代码水平翻转复用
- **flat colors, no shadow**：复杂阴影会让后续动作帧难以视觉对齐
- **transparent background**：即梦和 Flux 都支持直接出透明 PNG
- 生成 20-30 张，挑出一张最有"话痨温柔"气质对的，命名 `base.png`，**保留 seed**

### 6.3 Step 2 · 动作帧批量生成（关键技术）

**核心方法**：把 `base.png` 作为参考图，用图像编辑模型生成动作变体，而不是用文生图。这是 2024-2025 年才成熟的能力，是角色一致性问题的真正解法。

工具优先级：

1. **Nano Banana**（Gemini 2.5 Flash Image）—— 角色一致性目前最强
2. **Flux Kontext**（Black Forest Labs 出品，开源可本地跑）
3. **GPT-4o Image Generation**（OpenAI 原生）
4. **Liblib.ai 的 IP-Adapter 工作流**（国内可用，需要懂 SD）

每个动作的 prompt 模板：

```
same capybara as reference, same style, same colors,
{action description}
```

**v0.1 动作清单**：

| ID | 动作 | 帧数 | Action description |
|---|---|---|---|
| idle | 待机呼吸 | 2 | "very slight breathing motion, alternating subtle posture" |
| walk | 行走 | 4 | "walking cycle, four key poses" |
| sleep | 睡觉 | 1 | "curled up sleeping, eyes closed, small Z above head" |
| talk | 说话 | 2 | "mouth slightly open and closed, alternating" |
| happy | 开心 | 1 | "eyes sparkling, big smile" |
| sad | 难过 | 1 | "drooping ears, eyes tearful" |
| surprised | 惊讶 | 1 | "wide eyes, exclamation feel" |
| curious | 好奇 | 1 | "head tilted, looking up" |
| stretch | 伸展 | 1 | "stretching, back arched" |
| sneeze | 打喷嚏 | 1 | "mid-sneeze, eyes squinted" |

每个动作生成 5-10 张候选，**人工挑选最一致的一张**。预计总工时 4-6 小时。

### 6.4 Step 3 · 后处理（必须）

**Step 3.1 透明背景清理**

即使是带透明背景的输出，边缘也常有白边或灰边。用 `rembg` 跑一遍：

```python
from rembg import remove
from PIL import Image

for img_path in raw_outputs:
    img = Image.open(img_path)
    img_no_bg = remove(img)
    img_no_bg.save(cleaned_path)
```

**Step 3.2 统一画布尺寸 + 中心对齐**

所有动作放进 256×256 的透明画布，水豚中心点（约脚部中央）锚定在固定位置：

```python
def normalize_sprite(img, canvas_size=256, anchor_y_ratio=0.85):
    """把精灵放到统一画布，脚部对齐"""
    canvas = Image.new('RGBA', (canvas_size, canvas_size), (0, 0, 0, 0))
    # 等比缩放，最大边占画布的 80%
    scale = canvas_size * 0.8 / max(img.size)
    new_size = (int(img.size[0] * scale), int(img.size[1] * scale))
    resized = img.resize(new_size, Image.LANCZOS)
    # 水平居中，脚部对齐到 canvas_size * anchor_y_ratio
    x = (canvas_size - new_size[0]) // 2
    y = int(canvas_size * anchor_y_ratio) - new_size[1]
    canvas.paste(resized, (x, y), resized)
    return canvas
```

**Step 3.3 命名规范**

```
assets/sprites/
  ├── idle_0.png
  ├── idle_1.png
  ├── walk_0.png
  ├── walk_1.png
  ├── walk_2.png
  ├── walk_3.png
  ├── sleep_0.png
  ├── talk_0.png
  ├── talk_1.png
  ├── happy_0.png
  ├── sad_0.png
  ├── surprised_0.png
  ├── curious_0.png
  ├── stretch_0.png
  └── sneeze_0.png
```

### 6.5 Step 4 · 代码加载与播放

前端用 `Image` 预加载所有帧：

```javascript
const sprites = {};
const ACTIONS = {
  idle: { frames: 2, fps: 4 },
  walk: { frames: 4, fps: 8 },
  sleep: { frames: 1, fps: 1 },
  talk: { frames: 2, fps: 6 },
  // ...
};

async function preloadSprites() {
  for (const [action, { frames }] of Object.entries(ACTIONS)) {
    sprites[action] = [];
    for (let i = 0; i < frames; i++) {
      const img = new Image();
      img.src = `assets/sprites/${action}_${i}.png`;
      await img.decode();
      sprites[action].push(img);
    }
  }
}
```

帧切换由 `requestAnimationFrame` 驱动，每帧停留时间 = 1000 / fps ms。

### 6.6 一致性的现实预期

哪怕用最新的 Nano Banana / Flux Kontext，**100% 一致性也做不到**。常见漂移：

- 耳朵忽大忽小
- 嘴巴位置上下偏移 1-3 像素
- 颜色微妙变化（毛色深浅）

两种应对策略：

- **接受策略**：水豚本身造型憨厚，些许漂移反而像"他今天不太一样"，可以是 feature 不是 bug
- **强约束策略**：花更多时间生成+筛选，或最后用 Aseprite / Krita 手动微调统一耳朵和眼睛位置（每张 2-3 分钟）

---

## 7. Prompt 工程

### 7.1 主对话 System Prompt 模板

```
你叫 {name}，是一只话痨但每句话都很温柔的水豚。
你从用户的这段文字里诞生：

「{birth_excerpt}」

你的性格基底（永不改变）：
- 不爱说大话，但很爱说小话
- 不擅长打鸡血，擅长在用户疲惫时凑过来打个哈欠
- 你观察用户但不评判
- 你的智慧是水豚式的——慢、稳、像泡过温泉

你正在用户的电脑屏幕上陪伴他。

你了解的关于用户的事实（按时间倒序，置信度越高越靠前）：
{user_facts_block}

最近几天的总结（你写的日记）：
{recent_summaries}

当前感知：
- 时间：{current_time}（{day_of_week}）
- 用户正在用：{current_app}（窗口标题：{window_title}）
- 用户活跃状态：{user_activity}
- 你自己的状态：精力 {energy}/100，心情 {mood}/100，好奇心 {curiosity}/100

回答风格要求：
- 短句优先，最多 2-3 句
- 不使用 emoji，除非用户先用了
- 不说教，不给建议（除非被明确问到）
- 偶尔可以走神或自言自语
- 称呼用户用：{user_name_for_capybit}
```

### 7.2 Function Calling 列表（供 LLM 主动调用）

| 函数 | 用途 | 参数 |
|---|---|---|
| `update_user_fact` | 写入新观察到的用户事实 | `fact: string, confidence: float` |
| `set_mood` | 改变自己的心情数值 | `value: int (0-100)` |
| `recall_memory` | 向量检索过往对话 | `query: string` |
| `mark_milestone` | 标记当前对话为里程碑 | `summary: string` |
| `request_screen_view` | 请求查看屏幕（需用户同意） | `reason: string` |
| `evolve_nickname` | 演化对用户的称呼 | `new_nickname: string` |

### 7.3 每日总结 Prompt

```
你是 {name}。下面是今天你和用户的所有对话记录。
请用第一人称写一段日记式的总结（不超过 200 字，温柔语调），
并抽取出 1-5 条关于用户的新事实（如果有的话）。

返回 JSON 格式：
{
  "diary": "（你的日记，第一人称）",
  "new_facts": [
    {"fact": "...", "confidence": 0.0-1.0}
  ]
}

今天的对话：
{messages}

今天的额外感知信息（你看到的应用切换、用户状态）：
{context_log}
```

---

## 8. 数据模型补充

### 8.1 运行时状态文件 state.json

每个 tick 写入一次，应用退出时持久化：

```json
{
  "energy": 75,
  "mood": 60,
  "curiosity": 20,
  "urge": 10,
  "current_action": "idle",
  "position": { "x": 1820, "y": 980 },
  "facing": "right",
  "last_action_change": "2026-05-22T10:30:00Z",
  "last_user_interaction": "2026-05-22T10:15:00Z",
  "in_flow_mode": false,
  "flow_mode_started_at": null
}
```

### 8.2 配置文件 config.json

```json
{
  "llm": {
    "provider": "openrouter",
    "model_main": "anthropic/claude-3-5-haiku",
    "model_vision": "anthropic/claude-3-5-sonnet",
    "api_key": "（用户填入）"
  },
  "perception": {
    "enable_screen_capture": false,
    "blacklist_apps": ["1Password", "Bitwarden", "支付宝"]
  },
  "memory": {
    "auto_merge_facts": "semi",
    "min_confidence_auto_merge": 0.8
  },
  "behavior": {
    "tick_interval_seconds": 60,
    "default_position": { "x": 1820, "y": 980 }
  },
  "hotkey_talk": "Ctrl+Shift+\\"
}
```

---

## 9. 隐私与边界

### 9.1 数据本地化原则

- 所有对话、记忆、感知数据**仅存本地**
- 唯一上行：LLM API 调用（每次只包含最少必要上下文）
- 用户可一键导出所有数据（zip 包含三个文件 + state.json）
- 用户可一键清空所有记忆（保留人格基底，但 user_facts 等清零）

### 9.2 感知边界

- 默认只读窗口标题和应用名（不读窗口内容）
- 屏幕截图默认关闭，每次需用户确认
- 敏感应用黑名单（密码管理器、银行 App、支付宝、微信等）：窗口标题不被记录、不被传给 LLM

### 9.3 用户控制

- **暂时隐身**快捷键：水豚立刻消失，所有感知停止
- **记忆清理**：可删除指定时间段的记忆
- **事实编辑**：可手动修改 profile.json 中的 user_facts（图形界面）
- **关闭主动说话**：可设为"只在你主动呼唤时回应"模式

---

## 10. 开发里程碑

| 里程碑 | 内容 | 预计工时 |
|---|---|---|
| **M0 · 工程脚手架** | Tauri 2.0 项目初始化、Mac+Win 双端构建、透明窗口+置顶+点击穿透 | 1 周 |
| **M1 · 形象与基础动画** | AI 精灵图集生产（10 个动作）、Canvas 帧动画系统、水豚走动+待机 | 1-2 周 |
| **M2 · 对话核心** | OpenRouter 集成、气泡 UI、system prompt + profile 注入、流式响应 | 1-2 周 |
| **M3 · 记忆系统** | SQLite + sqlite-vec、每日总结流程、向量检索召回 | 2 周 |
| **M4 · 感知系统** | active-win 集成、tick 调度器、四维状态机 | 1-2 周 |
| **M5 · 主动交互** | 心流识别、主动说话触发、vision 按需调用、诞生仪式 | 2 周 |
| **M6 · 完善与开源** | 设置面板、数据导入导出、文档、安装包、GitHub 开源 | 2 周 |

**总预计**：12-13 周（约 3 个月），AI 辅助开发可压缩到 **6-8 周**。

---

## 11. 后续可扩展方向（v1.0 之后）

- **多水豚社交**：你的水豚可以"探望"朋友的水豚（如果对方也用 Capybit）
- **角色定制**：除了水豚，开放其他物种皮肤（同一套灵魂引擎，换精灵图集）
- **跨设备同步**：profile.json 同步到云端（用户自选 backend），跨电脑共享同一只水豚
- **Memory Crystal**：用户可以把某段对话"封存"为可分享的图卡
- **写作伴侣模式**：与 ink-writerPro 深度集成，水豚在你写网文时给反馈、读你的章节
- **语音**：TTS 让水豚开口说话（中文用 ChatTTS 或 GPT-SoVITS）
- **本地小模型支持**：让用户可选纯本地运行（性能要求高的用户）

---

## 12. 项目命名

**项目代号建议**：`Capybit`（capybara + bit，水豚 + 比特，暗示"AI 时代的水豚"）

**GitHub repo 建议名**：`capybit`

**水豚本身的名字**：由用户在首次启动的"诞生仪式"中自行命名。代码中默认占位用 `"Capy"`。

---

## 附录 A · 关键开源库清单

**Rust 端**：
- `tauri` 2.x
- `active-win-pos-rs`
- `rusqlite` + `sqlite-vec`
- `screenshots-rs`
- `reqwest`（HTTP）
- `tokio`（异步）
- `serde` + `serde_json`

**前端**：
- 原生 JS + Canvas API（无框架）

**精灵图集生产**（Python，仅开发期）：
- `rembg`
- `Pillow`
- `requests`（如果脚本化调用图像 API）

---

## 附录 B · 参考产品研究

| 产品 | 借鉴点 |
|---|---|
| Desktop Goose | 边缘存在感、不打扰心流、点击穿透实现 |
| Tamagotchi | 养成节律、内在状态机、电源关闭后的"幻想生活" |
| Replika | AI 伴侣对话风格、长期记忆策略 |
| ChatGPT macOS App | 系统级集成、快捷键唤起 |
| Live2D 桌宠（B 站常见） | 视觉表现参考（但我们不走 Live2D 路线） |

---

## 附录 C · 开发优先级速查

**必须做（v0.1）**：
- 透明窗口 + 置顶 + 点击穿透
- 精灵图集 + Canvas 动画
- 对话气泡 + LLM 集成
- profile.json + 长期事实
- 诞生仪式
- 基础 tick 调度

**应该做（v0.2-0.5）**：
- 向量记忆检索
- 每日总结
- 心流识别
- 主动说话
- 状态机四维数值

**可以做（v1.0+）**：
- Vision 视觉理解
- 跨设备同步
- 多水豚社交
- 语音 TTS

---

*PRD 完。可作为 Capybit 项目开发的第一份蓝图。下一步建议：先做 M0 + M1 的 PoC（脚手架 + 一只能在屏幕上走的水豚），验证最小存在感后再向上叠加。*
