# Prompts

Runtime-loaded prompt templates. **Do not hard-code prompt text in Rust source** — change templates here and just restart the app (or `pnpm tauri dev`).

## Files

- `system.md` — main chat system prompt; assembled by `src-tauri/src/llm/prompts.rs::render_system_prompt`. Placeholders use `{name}`-style braces and are replaced at runtime. Available placeholders:
  - `{name}` — capybara's current name
  - `{birth_excerpt}` — birth ritual source text (M5)
  - `{user_facts_block}` — bulleted facts about the user
  - `{recent_summaries}` — recent daily diaries (M3+)
  - `{current_time}`, `{day_of_week}` — clock
  - `{current_app}`, `{window_title}`, `{user_activity}` — perception (M4+)
  - `{energy}`, `{mood}`, `{curiosity}` — internal state (M4+)
  - `{user_name_for_capybit}` — what the capybara calls the user

> 占位符语义对应 PRD §7.1。改占位符名时记得同步 `prompts.rs`。

Templates added later (M3+): `daily_summary.md`, `fact_extraction.md`.
