# ADR 0011 · LLM function calling — 让水豚自主画屏

- 状态：Accepted
- 日期：2026-05-23
- 范围：把 ADR 0010 的 LCD scene 系统跟 LLM 大脑接通。

## 决策

通过 OpenRouter / OpenAI 兼容的 function calling 协议，让 Claude Haiku（或 GLM-4.6 fallback）在对话流里**主动调用** `set_scene` 工具，控制水豚自己的 LCD 屏。

实现机制：

1. **工具定义**（`src-tauri/src/llm/tools.rs::all_tools()`）—— 输出符合 OpenAI tools 规范的 JSON Schema，描述 `set_scene` 的完整参数表（sprite 名称枚举、icon 名称枚举、字体字符集、坐标范围）。这份 schema 在每次 chat 请求里随 `tools: [...]` 字段发给模型
2. **流式协议解析**（`openrouter.rs::run_one_round`）—— SSE 增量解析新增 `delta.tool_calls[]` 路径，按 `index` 累积每个 tool_call 的 `id` / `name` / `arguments`（arguments 是分片到达的 JSON 字符串）
3. **多轮 tool-calling 循环**（`openrouter.rs::stream_chat`）—— 单次用户消息最多 3 轮：每轮跑一次 stream；若 `finish_reason == "tool_calls"` 或 stream 结束时只有 tool_calls 无 content，执行所有工具 → 把"assistant tool_calls 消息"+"tool 结果消息"加进 stack → 继续下一轮；遇到正常 finish 就 emit `llm-done`
4. **执行**（`tools.rs::execute_set_scene`）—— 直接 `app.emit("lcd-scene", payload)`，前端 `main.js::wireStateEvents` 已经监听，调 `window.__capybit_pushScene` 把场景推到 LCD 渲染器
5. **教学**（`docs/prompts/system.md`）—— 给水豚的 system prompt 末尾增加一段 ~150 字的说明，告诉它：屏幕是 32×32 点阵、它有 `set_scene` 这个工具、什么时候用、举三个具体例子

## 关键设计选择

### 1. 工具结果是短状态字符串

`execute_set_scene` 返回 `"ok, scene shown for 6000ms"`，不返回 scene 数据。理由：模型不需要"读"自己刚画的东西，只需要知道**调用成功了**好继续说话。把 tool result 做小，可以省 token。

### 2. 强制要求 set_scene 后必须说话

prompt 里加了硬约束："调用 set_scene 之后仍然要回复一段简短的文字（哪怕只是「嗯。」或「看。」）"。
原因：纯 tool call 无 content 时，气泡 UI 看上去是空的（只显示了 token 数），用户体验差。
模型本来可能想"只画一张图就够了"，但聊天框架还是聊天框架，需要文本闭环。

如果模型违反这条（偶尔会），气泡里只显示 `0+N tok` 不显示文字。这是已知容忍的小瑕疵；如果实测频率高再加 Rust 端兜底（emit 一句默认 "..." 替补）。

### 3. tool round 预算 3 次

最常见路径是 1 次（模型直接 set_scene 然后说话；OpenAI 模式下 tool_calls 完整在一轮）。Anthropic 通过 OpenRouter 时偶尔需要 2 次（先回点话 → 调工具 → 再说收尾）。3 次封顶防止失控自循环。

### 4. tool_call ID 缺失时合成

某些 OpenRouter 后端不发 `id`，只发 `index`。`run_one_round` 为缺失 id 的 tool_call 合成 `call_local_{idx}`，并在后续 assistant message + tool message 里**同名引用**——只要两边匹配，模型就接受。

### 5. 历史不持久化 tool_calls

`commands.rs::spawn_persist_listener` 仍然只把 `full_text`（assistant 最终文本）写进 conversations.sqlite。tool calls 是**副作用**不是对话记录。下一轮上下文重建时不会带 tool_call 历史进 prompt——所以模型每次都是"新鲜"调工具，不会因为看到自己之前调过就抑制。

这是有意识的取舍：保持 prompt 干净，**接受**模型可能短时间内重复画同一种东西。

### 6. 不强制 `tool_choice`

用 `tool_choice: "auto"` 让模型自己决定要不要画。试图强制 `"required"` 会让水豚每轮都画，反而很烦人。结合 prompt 里的"节制使用"指导，model 自然会判断。

## 不做的事

- **本地工具列表 / 配置 UI**：用户暂时不能开关哪些工具可用。M6 设置面板做
- **更多工具**：除了 `set_scene` 没有其他工具。后续可加 `recall_memory`（手动触发向量召回，跳过召唤词检测）、`set_mood`（自我调节）、`mark_milestone` 等（PRD §7.2 列表）
- **fallback model 切换**：当前 model 不支持 tools 时（小模型）会返回 400。当前 `cfg.fallback_model` 字段尚未实装；这个 ADR 不解决，单独一个迭代加 retry-with-fallback
- **vision / 多模态 tool**：M5 后期 + 用户授权才考虑

## 影响 / 成本

- **二进制**：~0（纯逻辑改动，无新 deps）
- **API token**：每次 chat 多带一份工具 schema（~500 input tokens / 请求）。月增量 ≈ ¥3-8（按 PRD §5.3 估算每天 ~30 chat）
- **延迟**：tools auto 模式下，模型选 set_scene 的时候要多一轮 round-trip。从用户视角是"先显示 scene 变化，再开始打字"，2-3 秒额外延迟（一次再请求 OpenRouter 的网络往返）。是可接受的代价
- **画面质量**：自由像素 (`type: "cells"`) 的质量完全取决于模型。Haiku 画小图标（心、雪花、笑脸）大致能对齐；复杂图像会歪扭——这是预期，自由像素本来就有"潦草手稿"味道

## 关联文件

- `src-tauri/src/llm/tools.rs`（新增）
- `src-tauri/src/llm/openrouter.rs`（重写支持 tool-calling 循环）
- `src-tauri/src/llm/mod.rs`（注册 tools 模块）
- `docs/prompts/system.md`（新增 LCD 画屏指导段落）
- ADR 0010（scene 数据模型——本 ADR 把它跟 LLM 真正接通）
