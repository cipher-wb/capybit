# Capybit · 架构文档

> 本文档是**已知信息的凝固**——仅包含本轮对话中已经构建、修改、或验证过的内容；
> 不读新文件、不补新探索。需要更深入的部分用 `[推测]` 标注。
> 最后更新：2026-05-23（M5 + LLM function calling 完工时刻）

---

## 1. 这个项目是什么？

Capybit 是一只长在 Windows 桌面右下角的"电子宠物水豚"。它表现为一台 192×192 的微型透明窗口设备：
中间嵌一块 32×32 暖绿色 LCD 点阵屏，上面用代码绘制的像素水豚做出各种姿态和小动画。
它不是观赏摆件——它能感知用户在用什么应用、键鼠是否活跃，按 60 秒一拍的状态机更新自己的
精力/心情/好奇心/说话欲望四维数值；它能跟用户对话（双击它弹气泡或 `Ctrl+Shift+\` 全局快捷键），
对话走 OpenRouter 流式 LLM，调用 Claude Haiku / DeepSeek 系列模型，并通过 SQLite + sqlite-vec 长期记住
用户的事实和共同记忆；最关键的能力是**它会自己画屏**——LLM 通过 function calling 调用 `set_scene`
工具，决定屏上展示什么 sprite、画什么图标、写什么字、或干脆用自由像素现画一个小图案，
作为它对当下对话的视觉回应。它的产品定位不是"任务助手"也不是"chat 浮窗"，
而是一个有数字生命感、能感知能记忆能表达情绪的桌面陪伴体。

---

## 2. 入口路径（从命令行到第一行业务代码）

```
pnpm tauri dev
  └→ package.json::scripts.tauri "tauri"
       └→ Tauri CLI 调 cargo run --no-default-features --color always
            └→ src-tauri/src/main.rs::main()                  // [推测] 约 5 行内
                 └→ capybit_lib::run()
                      └→ src-tauri/src/lib.rs::run()           // ~30 行起
                           ├ tracing_subscriber::fmt() 初始化日志（默认 env: info,capybit=debug）
                           ├ tauri::Builder::default()
                           │    .plugin(tauri_plugin_dialog::init())
                           │    .plugin(tauri_plugin_global_shortcut::Builder::new().build())
                           │    .setup(|app| { ... })          // 业务装配在 setup 闭包里
                           │
                           │    setup 闭包内顺序：
                           │    1. state::load(&handle)        // 读 state.json
                           │    2. window::apply_initial_state // 还原窗口位置 + set_ignore_cursor_events(true)
                           │    3. app.manage(AppState)       // 注入到 Tauri state
                           │    4. app.manage(InnerStateStore) // 4 维状态机
                           │    5. app.manage(MonologueCache)  // 内心独白 5min 缓存
                           │    6. birth::is_uninitiated 判定 → 显示 birth 窗 / 主窗
                           │    7. memory::Db::open(&handle)   // 打开 conversations.sqlite + 注册 sqlite-vec
                           │    8. spawn_summary_catchup       // 启动后 3s 跑漏总结
                           │    9. spawn_daily_summary_ticker  // 23:59 每天总结
                           │    10. spawn_vector_backfill      // 启动后 60s 补 embedding
                           │    11. scheduler::tick::spawn     // 60s 状态机 tick
                           │    12. register_global_hotkey     // Ctrl+Shift+\
                           │    13. tray::install              // 系统托盘菜单
                           │    14. window::spawn_clickthrough_poller // 33ms 轮询点击穿透
                           │    15. window::watch_window_moves // 250ms 防抖位置持久化
                           │
                           │    .invoke_handler(generate_handler![...所有 commands])
                           │    .run(generate_context!())
```

**前端入口**：

```
tauri.conf.json::build.frontendDist = "../src"
  └→ src/index.html
       ├ <canvas id="stage" width="192" height="192">
       ├ <div id="tooltip">    // 鼠标悬停水豚时显示的"内心独白"
       └ <script type="module" src="./main.js">
            └→ src/main.js::boot()  // async IIFE
                 1. 读 window.__TAURI__ (全局对象, withGlobalTauri=true 提供)
                 2. tryPreloadSprites() → 因 PREFER_CODE_DRAWN=true 直接返回 null
                 3. startRenderLoop({ canvas, sprites:null, getAction })
                      └ requestAnimationFrame 循环 → drawPlaceholder(ctx, {t, action})
                 4. attachDragging       (整个 LCD 框可拖)
                    attachDoubleClickToTalk (双击 → invoke open_bubble)
                    wireStateEvents     (订阅 pet-state + lcd-scene 事件)
                    wireHoverTooltip    (订阅悬停 → invoke get_inner_monologue)
```

**其他两个窗口**：

- `src/birth.html` + `src/ui/birth.js` —— 仅首次启动显示，走"诞生仪式"流程后隐藏（不销毁）
- `src/bubble.html` + `src/ui/bubble.js` —— 对话气泡，默认隐藏，按需 `invoke('open_bubble')` 弹出

---

## 3. 真实的对话/绘屏流水线

> 项目里**最完整也最常被触发**的链路：用户敲一行字 → 水豚说话 + LCD 画屏。
> 各步标"L"= 调 LLM，"D" = 写 DB，"E" = emit 事件，"R" = 渲染。

```
[用户在气泡输入框敲字 + Enter]
         ↓
[1] src/ui/bubble.js::submit()
         · 生成 requestId = crypto.randomUUID()
         · invoke('send_message', { requestId, message })
         ↓
[2] src-tauri/src/commands.rs::send_message    (tauri::async_runtime::spawn)
         · state_machine::on_user_chat(state)
              D: mood+=5, urge=0, energy-=3, last_user_interaction=now
         · memory::write_message(db, "user", message, None)    [D: messages 表写入]
         · config::load(&app)                                   // 读 config.local.json
         · profile::load(&app)                                  // 读 profile.json
         · memory::recent_history(db, 10)                       // 拉 10 条历史
              · 滤掉刚写的那条 user 消息
              · role: "capybit" → "assistant"
         · render_recent_summaries(db)                          // 拼最近 5 天 diary
         · has_recall_trigger(&message)                         // "还记得"/"上次" 等 13 词
              · 若命中: embedder::embed(cfg, msg) [L] + memory::query_similar(db, emb, 3)
              · 拼成"被这次话题唤起的更早记忆"块
         · llm::prompts::build_chat_messages(profile, history, summaries, recalled, msg)
              · 读 docs/prompts/system.md 模板做占位符替换：
                {name},{birth_excerpt},{user_facts_block},{recent_summaries},
                {recalled_memories},{current_time},{day_of_week},{current_app},
                {window_title},{user_activity},{energy},{mood},{curiosity},
                {user_name_for_capybit}
         · spawn_persist_listener(app, request_id)              // 30s 内监听 llm-done 持久化
         · openrouter::stream_chat(app, request_id, cfg, messages).await   [L]
         ↓
[3] src-tauri/src/llm/openrouter.rs::stream_chat
         · 进入 for round in 0..MAX_TOOL_ROUNDS (=3) 循环
         · 每轮调 run_one_round(...)
              ↓
              [3a] HTTP POST → cfg.base_url/chat/completions
                   body = { model, messages, stream:true, max_tokens:1500,
                            temperature:0.85, tools: tools::all_tools(),
                            tool_choice: "auto" }
              [3b] SSE 流解析 (Reader 循环 + "\n\n" 分隔):
                   · delta.content        → 累积 full_content + emit "llm-chunk"  [E]
                   · delta.reasoning_content / reasoning → 累积 reasoning_content
                   · delta.tool_calls[]   → 按 index 累积到 AccumTool
                                          (id / name 只在首次出现取; arguments 是 JSON 分片拼接)
                   · usage / finish_reason 见即记
              [3c] 截断检测：finish_reason=="length" 且 tool_calls 中存在 JSON 解析失败 →
                   degrade 为 Final (放弃执行 tool, 把已有 text 当最终回复)
              [3d] 判定 RoundResult:
                   · 有 tool_calls + (finish_reason="tool_calls" 或 content 为空) → Tools
                   · 否则 → Final
         ·
         · 若是 Tools:
              · push 一条 assistant 消息 (role=assistant, content=text_before, tool_calls=[...])
              · 若有 reasoning_content → 同消息加 "reasoning_content" 字段（DeepSeek thinking 必须）
              · 遍历 tool_calls 调 tools::execute()
                   · "set_scene": 解析 args JSON → app.emit("lcd-scene", { scene, duration_ms }) [E]
                                  → 返回 "ok, scene shown for {N}ms"
              · 每个 tool 结果 push 一条 (role=tool, tool_call_id, content) 消息
              · 进入下一轮
         · 若是 Final:
              · app.emit("llm-done", { request_id, full_text, prompt_tokens, completion_tokens }) [E]
         ↓
[4] 前端事件接收:
         (4a) src/ui/bubble.js
              · listen("llm-chunk") → 累积到 typingTarget,
                由 pumpTyping 每 22ms/字符 写入 #reply
              · listen("llm-done")  → 等动画追上后展示 token 计数,
                重新启用输入框
              · listen("llm-error") → 显示错误文字, 启用输入
              · 触发的 spawn_persist_listener (在 Rust 端) 收到 llm-done →
                memory::write_message(db, "capybit", full_text, None) [D]
         (4b) src/main.js::wireStateEvents
              · listen("lcd-scene") → window.__capybit_pushScene(scene, duration_ms)
         ↓
[5] src/renderer/placeholder_capybara.js::pushExternalScene
         · _externalScene = scene
         · _externalUntil = performance.now() + duration_ms
         ↓
[6] 渲染循环（在 src/renderer/canvas.js::startRenderLoop 内 RAF 调用）
         · drawPlaceholder(ctx, { t, action })
              · resolveScene(frame):
                   - 若 _externalScene 在 TTL 内 → 返回它
                   - 否则 defaultSceneForAction(action, t)
              · composeScene(grid, scene, t):
                   - 选 SPRITES[scene.sprite] (16x16) blit 到 (8,10)
                   - 遍历 overlays 调 applyOverlay → icon / text / cells 三种
                   - 心跳点 (30,30) 按时间闪
              · paintBorder + paintLCD → 实际像素绘制
```

**这条流水线的极限路径**：用户消息 ≤ 1s 到达 → LLM 第一轮 1-3s（DeepSeek thinking 还要加 2-5s 思考）
→ 工具执行瞬时 emit lcd-scene → LCD 屏 1 帧（16ms）更新 → 第二轮 LLM 1-2s 生成文字 → 气泡逐字显示。
总感知延迟：**3-8 秒**（首字打到气泡），LCD 变化在工具调用瞬间发生（用户视觉上"先画后说"）。

---

## 4. LLM 调用点全表

| 位置 | 触发时机 | 模型 | 流式 | 工具 | Prompt 模板 | 备注 |
|---|---|---|---|---|---|---|
| `llm/openrouter.rs::stream_chat` | 每条用户消息 | cfg.model (用户当前 deepseek-v4-pro；默认 anthropic/claude-haiku-4-5) | ✓ | ✓ set_scene | `docs/prompts/system.md` | 最多 3 轮 tool-calling 循环；支持 reasoning_content；max_tokens=1500 |
| `llm/summarizer.rs::run_for_date` | 23:59 当日 + 启动 catchup（≤7 天） | cfg.model | ✗ | ✗ | `docs/prompts/daily_summary.md` | response_format=json_object；返回 {diary, new_facts}；高置信度 (≥0.8) 自动并入 profile.json；同时 embed diary 写入 vec_memories |
| `llm/monologue.rs::get_or_fetch` | 鼠标悬停水豚（命中 5min×action 缓存才真调） | cfg.model | ✗ | ✗ | `docs/prompts/inner_monologue.md` | max_tokens=60；空响应或 4xx/5xx → fallback 到 8 条硬编码文案 |
| `llm/opener.rs::generate` | scheduler tick 触发主动说话（urge>70 && !in_flow && 距上次 ≥30min） | cfg.model | ✗ | ✗ | `docs/prompts/proactive_opener.md` | max_tokens=80；产出的开场白会写入 conversations 表（role=capybit）+ emit "proactive-opener" 事件让气泡弹出 |
| `llm/embedder.rs::embed` | (1) summarizer 写完 diary，(2) 启动后 60s backfill，(3) chat 命中召唤词 | `openai/text-embedding-3-small` (硬编码) | n/a | n/a | n/a | 1536 维；存进 sqlite-vec 虚表 `vec_memories` + 元数据表 `vec_meta` |

**所有 LLM 调用共用一份 config.local.json**（`%APPDATA%\dev.cipher.capybit\config.local.json`）：
`api_key` / `model` / `base_url` (默认 `https://openrouter.ai/api/v1`) / `fallback_model` (字段已存在，**逻辑未实装**)。

---

## 5. [推测] 仍待深入的模块

以下模块在本对话中编写或修改过，但具体的算法细节、边界行为或性能特征**未在本对话中验证或回顾**，
后续开发者动它们之前应实际阅读源码：

- `[推测]` `window.rs::spawn_clickthrough_poller`：cursor 全局坐标的 DPI/多显示器行为，特别是高 DPI 屏 + 缩放下的 `cursor_position()` 单位
- `[推测]` `window.rs::watch_window_moves`：250ms 防抖期间应用关闭是否会丢失最后一次 move（应该会写盘但未验证）
- `[推测]` `birth.rs::scan`：random shuffle 后的"挑第一个可用段落"算法在大 vault（>1000 文件）上的扫描耗时
- `[推测]` `scheduler/state_machine.rs::step`：各项数值公式的稳定性——长期跑下来 mood/energy 会不会 stuck 在边界
- `[推测]` `memory/conversations.rs` 的 UTC→local 日期分桶：消息累积到几千条后的 `messages_for_date` 性能（已有 TODO 标记加冗余列）
- `[推测]` `memory/vectors.rs` 的 sqlite-vec 查询计划：上万条 embedding 时的检索延迟
- `[推测]` `llm/monologue.rs` 的缓存失效边界：action 在 5min 内反复切换是否会触发频繁 LLM 调用（按设计每次切就调）
- `[推测]` DeepSeek `reasoning_content` 的实际 token 占用——本次只看到 `reasoning_len=589`，多轮累积会膨胀多少
- `[推测]` 截断检测在 OpenAI 标准 + 非 thinking 模型（Haiku/Sonnet）上的行为是否一致
- `[推测]` `placeholder_capybara.js` 在分辨率 > 192 物理像素的屏（HiDPI / 缩放）上的清晰度
- `[推测]` 模型自由像素 (`cells` overlay) 的实际质量分布——本对话只验证了 1 次（DeepSeek 画的心形），样本太小

---

## 6. 后续开发建议

**短期清理（30 分钟级）**：

1. **降级调试日志等级**——`main.js`、`placeholder_capybara.js`、`openrouter.rs` 里几条 `console.info` / `tracing::debug!`（"lcd-scene received"、"external scene set"、"llm round: ..."）在功能验证后会刷屏；改成 trace 或加 dev-only 门控
2. **提交到 GitHub**：当前未推送的本地改动包含整套 LLM function calling + LCD scene 系统。下一次 commit 内容很大，建议拆 2-3 个：(a) scene 数据模型，(b) tools.rs + openrouter.rs 重写，(c) prompt 更新
3. **删除 `src/assets/sprites/surprised_0..png`** 那个双点错文件名的占位 PNG（git ls-files 看到的小瑕疵）

**中期扩展（半天到一天级）**：

- **A. 让 state machine 也能 push scene**——不只是 LLM 在画，让 mood 跌破 30 时自动给 LCD 推一个雨点 overlay；curiosity 突涨时推一个 question icon。这能让屏幕在用户**没在聊**的时候也有内在生命感。
- **B. 扩 icon 库 + 加几个 sprite**——模型当前画自由像素质量参差；如果库里有现成的"sunglasses"、"crown"、"sparkle_3x3"，模型会更愿意选 icon 而不是硬画
- **C. 中文像素字体**——现在 LCD 只能写 ASCII，写不了"早安"。8×8 字体能覆盖常用 200 个汉字（用户名 + 心情词），但 token 成本（写在 prompt 让模型知道有哪些字）需要权衡
- **D. fallback_model 真正生效**——config 字段已存在，逻辑未接；500 / 429 / 不支持 tools 的 400 都应自动 retry 到 GLM-4.6
- **E. M6 打包成 .msi**——需要先处理 `CARGO_MANIFEST_DIR` prompt 路径问题（TODO 已标在 prompts.rs / summarizer.rs / monologue.rs / opener.rs），换成 Tauri `BaseDirectory::Resource` 并在 tauri.conf.json 配 `bundle.resources`

**长期方向（一周以上）**：

- 工具列表扩展：`recall_memory`（手动召回，跳过召唤词）、`set_mood`（自我调节）、`mark_milestone`（用户说"记住这个"）、`request_screen_view`（vision 截图，PRD §3.3.2）
- 设置面板（M6 主体）：模型选择、工具开关、快捷键改键、感知黑名单编辑、低置信度事实人工审核 UI
- 自动启动 + 系统通知集成（PRD 未列但日常使用刚需）
- 多色外壳 / 主题切换（ADR 0010 提到的可选扩展）

---

## 7. 后续其他 AI 开发需要交代的内容

**进项目第一件事必读**（按优先级）：

1. `CLAUDE.md` —— 工程协作约束（**HOW**，不是 WHAT）。锁定了 Tauri / 原生 JS / Canvas / OpenRouter / SQLite 五项技术决策，挑战这些前必须先写 ADR
2. `docs/PRD.md` —— 产品需求（**WHAT/WHY**）。任何"为什么这样设计"的疑问先查这里
3. `docs/ADR/0001..0011-*.md` —— 11 份决策记录，每份对应一个非平凡的技术或形态变更：

   | ADR | 主题 | 状态 |
   |---|---|---|
   | 0001 | M0 点击穿透（圆形 hitbox） | **部分作废**（被 0010 覆盖） |
   | 0002 | M2 对话核心（独立 bubble 窗 + Rust 端 SSE 解析） | 现行 |
   | 0003 | M5 诞生仪式（独立 birth 窗 + 摘录抽取算法） | 现行 |
   | 0004 | M3 记忆系统（messages + daily_summaries + 启动 catchup + 23:59 ticker） | 现行 |
   | 0005 | M3 向量召回（sqlite-vec + 召唤词触发） | 现行 |
   | 0006 | M4 感知 + 状态机 + 内心独白 | 现行 |
   | 0007 | M5 主动说话 + 全局快捷键 | 现行 |
   | 0008 | 放弃精灵图集，走代码绘制（chibi 风格） | **被 0010 替代** |
   | 0009 | 电子宠物外壳设备形态 | **被 0010 替代** |
   | 0010 | LCD scene 合成系统（32×32 + 三层 overlay） | **现行（主美术决策）** |
   | 0011 | LLM function calling 接通 set_scene | **现行（最新）** |

**硬约束（不写 ADR 不许改）**：

- 窗口尺寸 192×192（改了之后 placeholder_capybara.js 全部坐标 + Rust hitbox 都要重算）
- LCD 网格 32×32，cell=5 px，screen 160×160 居中（`placeholder_capybara.js` 顶部常量）
- 命中区半边长 84（`window.rs::PET_HITBOX_RADIUS_PX`，命名是历史包袱实际是 square half-side）
- bubble 窗 / birth 窗 必须保持独立窗口（ADR 0002 / 0003 的几何 invariant 服务于点击穿透）
- API key 只在 Rust 持有，**绝不**通过 invoke 暴露给 webview（CLAUDE.md §8.1 + ADR 0002）
- prompt 模板**绝不**写死在 Rust 源码里，必须 `docs/prompts/*.md`（CLAUDE.md §9.3）

**已踩过且不许重复的坑**（CLAUDE.md §10 + 本对话）：

- Windows `tauri-build` 在 release/dev 都强制要 `src-tauri/icons/icon.ico`，即使 `bundle.active=false`。**不要**用 .NET 的 `Icon.Save()` 现造——ICONDIRENTRY 保留字段非 0 会被 Tauri 的 ico crate 拒绝（错误信息 `Invalid reserved field value in ICONDIRENTRY`）。用 `pnpm tauri icon path/to/source.png` 生成；本对话用了手写二进制方法绕过
- sqlite-vec 静态链接通过 `rusqlite::ffi::sqlite3_auto_extension` 注册在 `memory::db::Db::open` 里，**必须**在任何 Connection::open 之前完成（已用 `Once` 保护）
- `tracing::debug!` 默认看不到，需要 `RUST_LOG=info,capybit=debug`（lib.rs 已设默认 EnvFilter）
- `env!("CARGO_MANIFEST_DIR")` 用在 prompts 模板加载——**只在 dev 模式正确**；M6 打包前必须迁移到 `BaseDirectory::Resource`（已挂 TODO 在 4 个文件）
- DeepSeek / thinking-mode 模型要求 `reasoning_content` 在多轮 tool-calling 时原样回传（现在 openrouter.rs 已支持；非 thinking 模型字段缺失也兼容）
- Tool 参数 JSON 容易被 `max_tokens` 截断（现在 max_tokens=1500，截断后 fallback 到 text-only）
- Tauri 2 的 `event.listen` 是 async；首条事件可能在订阅生效前 (~500ms warmup) 就发出，被丢弃。当前所有事件路径都在用户交互之后才触发，不构成问题，但下次想加"启动立刻 emit"型事件要注意
- Windows 上 `git add -A` 会触发 LF→CRLF 警告（已正常），但 PowerShell 跑 git 命令时红色输出**不一定**是错误——push 成功消息也走 stderr

**禁区**（动了它们 = 强制重新 ADR）：

- `src-tauri/src/window.rs::spawn_clickthrough_poller`：换 hitbox 形状需要 ADR
- `src/renderer/placeholder_capybara.js` 顶部常量（`CELL`、`GRID_W/H`、`SCREEN_X/Y`）
- `src-tauri/tauri.conf.json` 的 windows 数组：label / url 跟 Rust commands 里 `get_webview_window("main"/"bubble"/"birth")` 强绑定，改名要全局搜
- `src-tauri/icons/icon.ico`：不要用 .NET 重生成，参见上文

**提交工作流**（CLAUDE.md §6 + §9.2）：

- 提交前必须 `cargo clippy --all-targets -- -D warnings` 干净 + `cargo fmt`
- 提交信息格式 `<type>(<scope>): <subject>`，type ∈ {feat, fix, refactor, docs, chore, test}
- 跨 ≥ 3 文件改动前先输出 plan 等用户确认
- 推 main 前 `cargo test --lib` （当前 0 测试，但保留 hook）
- 已有 GitHub remote: `https://github.com/cipher-wb/capybit.git`，分支 `main`

**当前 git 状态（截止本文档生成时）**：
- main 分支已推送一次 initial commit（`440389b chore: initial commit — Capybit desktop AI companion`）
- 本对话后续的 LLM function calling + LCD scene 系统**尚未提交**（包括 tools.rs / openrouter.rs 重写 / system.md 改写 / 调试日志 / monologue 兼容空响应 等）
- 用户表达过想用 DeepSeek V4 PRO 而非 Claude Haiku 控制成本；DeepSeek 已成功调通 set_scene 工具

---

*本文件随项目阶段性凝固。下一次重写时机：M6 打包前、或下一次大型架构变动（ADR ≥ 0012）之后。*
