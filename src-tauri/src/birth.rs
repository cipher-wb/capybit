//! 诞生仪式 (PRD §2.3 + §4.1)。
//!
//! 流程（前端驱动，调三个 Rust 命令）：
//!   1. pick_birth_folder() —— 系统文件夹选择对话框
//!   2. scan_for_excerpt(folder) —— 扫描 .md/.txt，挑一段"诞生原文"
//!   3. confirm_birth({...}) —— 落盘 profile.json，关闭 birth 窗口，显示主窗
//!
//! 选段策略：从用户选的文件夹（深度 ≤ 3）里挑最近 30 个 .md/.txt，
//! 按修改时间倒序加权随机抽 1 个，从中取第一个 30-200 字的段落。

use std::path::{Path, PathBuf};

use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use walkdir::WalkDir;

use crate::profile::{Birth, Profile};

const EXCERPT_MIN_CHARS: usize = 30;
const EXCERPT_MAX_CHARS: usize = 200;
const SCAN_DEPTH: usize = 3;
const CANDIDATES_KEPT: usize = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcerptResult {
    pub source_file: String,
    pub excerpt: String,
}

#[derive(Debug, thiserror::Error)]
pub enum BirthError {
    #[error("no readable .md/.txt files found in folder")]
    NoCandidates,
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub fn scan(folder: &Path) -> Result<ExcerptResult, BirthError> {
    let mut candidates: Vec<(PathBuf, std::time::SystemTime)> = WalkDir::new(folder)
        .max_depth(SCAN_DEPTH)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            matches!(
                e.path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(str::to_ascii_lowercase)
                    .as_deref(),
                Some("md") | Some("txt") | Some("markdown")
            )
        })
        .filter_map(|e| {
            let mtime = e.metadata().ok().and_then(|m| m.modified().ok())?;
            Some((e.path().to_path_buf(), mtime))
        })
        .collect();

    if candidates.is_empty() {
        return Err(BirthError::NoCandidates);
    }

    // Sort newest-first, keep top N (recency bias).
    candidates.sort_by_key(|c| std::cmp::Reverse(c.1));
    candidates.truncate(CANDIDATES_KEPT);

    let mut rng = rand::thread_rng();
    candidates.shuffle(&mut rng);

    for (path, _) in candidates {
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Some(excerpt) = pick_paragraph(&content) else {
            continue;
        };
        return Ok(ExcerptResult {
            source_file: path.to_string_lossy().into_owned(),
            excerpt,
        });
    }

    Err(BirthError::NoCandidates)
}

/// Return the first paragraph (blank-line separated block) with EXCERPT_MIN_CHARS+
/// characters, truncated to EXCERPT_MAX_CHARS. Skips frontmatter / heading lines.
fn pick_paragraph(content: &str) -> Option<String> {
    // Strip YAML frontmatter if present.
    let body = if let Some(rest) = content.strip_prefix("---\n") {
        rest.find("\n---\n")
            .map(|i| &rest[i + 5..])
            .unwrap_or(content)
    } else {
        content
    };

    for raw in body.split("\n\n") {
        let cleaned: String = raw
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            // Skip markdown headings, code fences, list bullets.
            .filter(|l| !l.starts_with('#'))
            .filter(|l| !l.starts_with("```"))
            .filter(|l| !l.starts_with('|'))
            .collect::<Vec<_>>()
            .join(" ");

        let cleaned = cleaned.trim();
        let char_count = cleaned.chars().count();
        if char_count < EXCERPT_MIN_CHARS {
            continue;
        }
        if char_count <= EXCERPT_MAX_CHARS {
            return Some(cleaned.to_string());
        }
        // Truncate on char boundary.
        let truncated: String = cleaned.chars().take(EXCERPT_MAX_CHARS).collect();
        return Some(format!("{truncated}…"));
    }
    None
}

pub fn build_birth_record(
    capybara_name: String,
    source_file: String,
    source_excerpt: String,
) -> Birth {
    let now = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown".into());
    Birth {
        named_by_user: capybara_name,
        named_at: now,
        source_file,
        source_excerpt,
    }
}

pub fn is_uninitiated(profile: &Profile) -> bool {
    profile.birth.is_none()
}
