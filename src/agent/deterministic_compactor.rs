use std::collections::HashSet;

use crate::memory::traits::MemoryEntry;

/// Deterministic budget for memory context compaction.
#[derive(Debug, Clone, Copy)]
pub struct CompressionBudget {
    pub max_chars: usize,
    pub max_entries: usize,
    pub max_entry_chars: usize,
}

impl Default for CompressionBudget {
    fn default() -> Self {
        Self {
            max_chars: 4_000,
            max_entries: 4,
            max_entry_chars: 800,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompactLine {
    pub key: String,
    pub content: String,
}

#[derive(Debug, Clone, Default)]
pub struct CompressionStats {
    pub original_entries: usize,
    pub kept_entries: usize,
    pub removed_duplicates: usize,
    pub truncated_entries: usize,
    pub omitted_entries: usize,
    pub original_chars: usize,
    pub compressed_chars: usize,
}

#[derive(Debug, Clone, Default)]
pub struct CompressionResult {
    pub lines: Vec<CompactLine>,
    pub stats: CompressionStats,
}

fn normalize_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_with_ellipsis(input: &str, max_chars: usize) -> (String, bool) {
    let mut chars = input.chars();
    let preview: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        (format!("{preview}…"), true)
    } else {
        (preview, false)
    }
}

/// Deterministically compacts memory entries by:
/// 1) line normalization
/// 2) de-duplication
/// 3) per-entry truncation
/// 4) total budget enforcement
pub fn compact_memory_entries(entries: &[MemoryEntry], budget: CompressionBudget) -> CompressionResult {
    let mut stats = CompressionStats {
        original_entries: entries.len(),
        ..CompressionStats::default()
    };

    let mut dedupe = HashSet::new();
    let mut lines: Vec<CompactLine> = Vec::new();
    let mut used_chars = 0usize;

    for entry in entries {
        stats.original_chars += entry.content.chars().count();

        if lines.len() >= budget.max_entries {
            stats.omitted_entries += 1;
            continue;
        }

        let normalized = normalize_whitespace(&entry.content);
        if normalized.is_empty() {
            stats.omitted_entries += 1;
            continue;
        }

        let (truncated, was_truncated) = if normalized.chars().count() > budget.max_entry_chars {
            truncate_with_ellipsis(&normalized, budget.max_entry_chars)
        } else {
            (normalized, false)
        };

        if was_truncated {
            stats.truncated_entries += 1;
        }

        let dedupe_key = format!("{}::{}", entry.key, truncated);
        if !dedupe.insert(dedupe_key) {
            stats.removed_duplicates += 1;
            continue;
        }

        let line_chars = entry.key.chars().count() + truncated.chars().count() + 4;
        if used_chars + line_chars > budget.max_chars {
            stats.omitted_entries += 1;
            continue;
        }

        used_chars += line_chars;
        lines.push(CompactLine {
            key: entry.key.clone(),
            content: truncated,
        });
    }

    stats.kept_entries = lines.len();
    stats.compressed_chars = used_chars;

    CompressionResult { lines, stats }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::traits::MemoryCategory;

    fn entry(key: &str, content: &str) -> MemoryEntry {
        MemoryEntry {
            id: key.to_string(),
            key: key.to_string(),
            content: content.to_string(),
            category: MemoryCategory::Conversation,
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            session_id: None,
            score: Some(1.0),
            namespace: "default".to_string(),
            importance: None,
            superseded_by: None,
        }
    }

    #[test]
    fn compactor_deduplicates_and_respects_budget() {
        let entries = vec![
            entry("a", "same content"),
            entry("a", "same content"),
            entry("b", "another content"),
        ];

        let result = compact_memory_entries(
            &entries,
            CompressionBudget {
                max_chars: 60,
                max_entries: 2,
                max_entry_chars: 20,
            },
        );

        assert_eq!(result.lines.len(), 2);
        assert_eq!(result.stats.removed_duplicates, 1);
    }

    #[test]
    fn compactor_truncates_long_entries() {
        let entries = vec![entry("a", "01234567890123456789")];
        let result = compact_memory_entries(
            &entries,
            CompressionBudget {
                max_chars: 100,
                max_entries: 4,
                max_entry_chars: 10,
            },
        );

        assert_eq!(result.lines.len(), 1);
        assert!(result.lines[0].content.ends_with('…'));
        assert_eq!(result.stats.truncated_entries, 1);
    }
}
