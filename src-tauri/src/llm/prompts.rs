//! Prompt assembly. Templates live in docs/prompts/*.md and are loaded at
//! runtime, NOT compiled in (per CLAUDE.md §9.3).
//!
//! TODO(cipher): use Tauri 2 resource bundling for prod builds.
//!   Right now we resolve via CARGO_MANIFEST_DIR which is dev-only correct.
//!   When M6 packaging lands, add `bundle.resources` and switch to
//!   `app.path().resolve("prompts/system.md", BaseDirectory::Resource)`.
//!   [due: M6]

use std::path::PathBuf;
use time::OffsetDateTime;

use super::ChatMessage;
use crate::profile::Profile;

fn prompts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("docs")
        .join("prompts")
}

fn read_template(name: &str) -> String {
    let p = prompts_dir().join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|err| {
        tracing::error!(?err, ?p, "missing prompt template");
        String::new()
    })
}

/// Build the full message stack for a user turn:
///   [ system, ...recent_history, { role: user, content: message } ]
///
/// `recent_history` should already be in chronological order (oldest first).
/// For M2 we don't yet plumb history; pass empty vec.
pub fn build_chat_messages(
    profile: &Profile,
    recent_history: Vec<ChatMessage>,
    recent_summaries: &str,
    recalled_memories: &str,
    user_message: &str,
) -> Vec<ChatMessage> {
    let mut out = Vec::with_capacity(recent_history.len() + 2);
    out.push(ChatMessage {
        role: "system".into(),
        content: render_system_prompt(profile, recent_summaries, recalled_memories),
    });
    out.extend(recent_history);
    out.push(ChatMessage {
        role: "user".into(),
        content: user_message.into(),
    });
    out
}

fn render_system_prompt(
    profile: &Profile,
    recent_summaries: &str,
    recalled_memories: &str,
) -> String {
    let template = read_template("system.md");

    let birth_excerpt = profile
        .birth
        .as_ref()
        .map(|b| b.source_excerpt.clone())
        .unwrap_or_else(|| "（你从用户的一段尚未被你看见的文字里诞生。）".into());

    let facts = if profile.user_facts.is_empty() {
        "（你还没有任何关于用户的固化事实——这是你们刚认识的阶段。）".into()
    } else {
        profile
            .user_facts
            .iter()
            .take(20)
            .map(|f| format!("- {} (置信度 {:.2})", f.fact, f.confidence))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let user_name = profile
        .user_name_for_capybit
        .clone()
        .unwrap_or_else(|| "你".into());

    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    let time_str = format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        now.year(),
        now.month() as u8,
        now.day(),
        now.hour(),
        now.minute()
    );
    let dow = match now.weekday() {
        time::Weekday::Monday => "周一",
        time::Weekday::Tuesday => "周二",
        time::Weekday::Wednesday => "周三",
        time::Weekday::Thursday => "周四",
        time::Weekday::Friday => "周五",
        time::Weekday::Saturday => "周六",
        time::Weekday::Sunday => "周日",
    };

    template
        .replace("{name}", &profile.name)
        .replace("{birth_excerpt}", &birth_excerpt)
        .replace("{user_facts_block}", &facts)
        .replace("{recent_summaries}", recent_summaries)
        .replace("{recalled_memories}", recalled_memories)
        .replace("{current_time}", &time_str)
        .replace("{day_of_week}", dow)
        .replace("{current_app}", "（M4 接入感知后填充）")
        .replace("{window_title}", "（M4 接入感知后填充）")
        .replace("{user_activity}", "（M4 接入感知后填充）")
        .replace("{energy}", "75")
        .replace("{mood}", "60")
        .replace("{curiosity}", "30")
        .replace("{user_name_for_capybit}", &user_name)
}
