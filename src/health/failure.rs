use chrono::Utc;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FailureSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl FailureSeverity {
    fn rank(self) -> u8 {
        match self {
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 3,
            Self::Critical => 4,
        }
    }
}

impl FromStr for FailureSeverity {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "low" => Ok(Self::Low),
            "medium" | "warn" | "warning" => Ok(Self::Medium),
            "high" | "error" => Ok(Self::High),
            "critical" => Ok(Self::Critical),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    Provider,
    Compact,
    Session,
    Channel,
    Memory,
    Config,
    Tool,
    Resource,
    Unknown,
}

impl FromStr for FailureKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "provider" => Ok(Self::Provider),
            "compact" => Ok(Self::Compact),
            "session" => Ok(Self::Session),
            "channel" => Ok(Self::Channel),
            "memory" => Ok(Self::Memory),
            "config" => Ok(Self::Config),
            "tool" | "tools" => Ok(Self::Tool),
            "resource" | "resources" => Ok(Self::Resource),
            "unknown" => Ok(Self::Unknown),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureRecord {
    pub id: String,
    pub kind: FailureKind,
    pub severity: FailureSeverity,
    pub message: String,
    pub context: BTreeMap<String, String>,
    pub first_seen: String,
    pub last_seen: String,
    pub occurrences: u64,
    pub resolved: bool,
    pub resolution: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct FailureFilter {
    pub kind: Option<FailureKind>,
    pub min_severity: Option<FailureSeverity>,
    pub unresolved_only: bool,
    pub limit: usize,
}

struct FailureRegistry {
    records: Mutex<BTreeMap<String, FailureRecord>>,
}

static REGISTRY: OnceLock<FailureRegistry> = OnceLock::new();

fn registry() -> &'static FailureRegistry {
    REGISTRY.get_or_init(|| FailureRegistry {
        records: Mutex::new(BTreeMap::new()),
    })
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

fn fingerprint(kind: FailureKind, message: &str) -> String {
    format!("{:?}::{}", kind, message.trim())
}

pub fn record_failure(
    kind: FailureKind,
    severity: FailureSeverity,
    message: impl Into<String>,
    context: BTreeMap<String, String>,
) -> String {
    let message = message.into();
    let key = fingerprint(kind, &message);
    let now = now_rfc3339();

    let mut map = registry().records.lock();

    if let Some(existing) = map.values_mut().find(|r| {
        !r.resolved && r.kind == kind && r.message == message
    }) {
        existing.occurrences = existing.occurrences.saturating_add(1);
        existing.last_seen = now;
        if severity.rank() > existing.severity.rank() {
            existing.severity = severity;
        }
        for (k, v) in context {
            existing.context.insert(k, v);
        }
        return existing.id.clone();
    }

    let id = format!("fail_{}", uuid::Uuid::new_v4());
    let mut ctx = BTreeMap::new();
    ctx.insert("fingerprint".into(), key);
    for (k, v) in context {
        ctx.insert(k, v);
    }

    map.insert(
        id.clone(),
        FailureRecord {
            id: id.clone(),
            kind,
            severity,
            message,
            context: ctx,
            first_seen: now.clone(),
            last_seen: now,
            occurrences: 1,
            resolved: false,
            resolution: None,
        },
    );

    id
}

pub fn resolve_failure(id: &str, resolution: Option<String>) -> bool {
    let mut map = registry().records.lock();
    if let Some(record) = map.get_mut(id) {
        record.resolved = true;
        record.resolution = resolution;
        record.last_seen = now_rfc3339();
        return true;
    }
    false
}

pub fn get_failure(id: &str) -> Option<FailureRecord> {
    let map = registry().records.lock();
    map.get(id).cloned()
}

pub fn list_failures(filter: &FailureFilter) -> Vec<FailureRecord> {
    let map = registry().records.lock();
    let mut records: Vec<FailureRecord> = map.values().cloned().collect();

    records.retain(|r| {
        if filter.unresolved_only && r.resolved {
            return false;
        }
        if let Some(kind) = filter.kind {
            if r.kind != kind {
                return false;
            }
        }
        if let Some(min) = filter.min_severity {
            if r.severity.rank() < min.rank() {
                return false;
            }
        }
        true
    });

    records.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));

    let limit = if filter.limit == 0 { 20 } else { filter.limit };
    records.truncate(limit);
    records
}

pub fn classify_category(category: &str) -> FailureKind {
    match category {
        "provider" => FailureKind::Provider,
        "compact" | "context" => FailureKind::Compact,
        "session" => FailureKind::Session,
        "channel" | "channels" => FailureKind::Channel,
        "memory" => FailureKind::Memory,
        "config" => FailureKind::Config,
        "tool" | "tools" | "cli-tools" => FailureKind::Tool,
        "resource" | "resources" => FailureKind::Resource,
        _ => FailureKind::Unknown,
    }
}

pub fn recovery_hint(kind: FailureKind) -> &'static str {
    match kind {
        FailureKind::Provider => "检查 API key/额度/网络；可执行 `zeroclaw doctor models` 验证 provider 连通性。",
        FailureKind::Compact => "降低上下文负载，检查压缩配置；必要时暂时关闭 context compact。",
        FailureKind::Session => "检查会话存储文件权限与损坏情况，必要时重建会话。",
        FailureKind::Channel => "检查渠道 token 与回调地址，确认网络可达后重连。",
        FailureKind::Memory => "检查 memory backend 路径/权限；必要时 vacuum 或切换后端。",
        FailureKind::Config => "校验配置字段与冲突项，修复后重新加载。",
        FailureKind::Tool => "检查工具依赖是否在 PATH，确认权限与参数。",
        FailureKind::Resource => "释放磁盘/内存压力，降低并发与上下文大小。",
        FailureKind::Unknown => "查看日志上下文进一步定位。",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_list_failure() {
        let id = record_failure(
            FailureKind::Config,
            FailureSeverity::High,
            "invalid config",
            BTreeMap::new(),
        );
        assert!(id.starts_with("fail_"));

        let list = list_failures(&FailureFilter {
            kind: Some(FailureKind::Config),
            min_severity: Some(FailureSeverity::Low),
            unresolved_only: true,
            limit: 10,
        });
        assert!(!list.is_empty());
        assert_eq!(list[0].kind, FailureKind::Config);
    }

    #[test]
    fn resolve_marks_record() {
        let id = record_failure(
            FailureKind::Tool,
            FailureSeverity::Medium,
            "tool missing",
            BTreeMap::new(),
        );
        assert!(resolve_failure(&id, Some("installed dependency".to_string())));

        let list = list_failures(&FailureFilter {
            unresolved_only: true,
            limit: 100,
            ..FailureFilter::default()
        });
        assert!(list.into_iter().all(|r| r.id != id));
    }
}
