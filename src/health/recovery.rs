use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;

use crate::config::Config;

use super::failure::{get_failure, resolve_failure, FailureKind};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryAction {
    EnsureMemoryDir,
    EnsureSessionsDir,
    EnsureStateDir,
    SuggestProviderCheck,
    SuggestCompactFallback,
    SuggestChannelReconnect,
    SuggestToolDependencyCheck,
    SuggestResourceRelief,
    Noop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStepResult {
    pub action: RecoveryAction,
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryReport {
    pub failure_id: String,
    pub kind: FailureKind,
    pub success: bool,
    pub steps: Vec<RecoveryStepResult>,
}

fn recovery_plan(kind: FailureKind) -> Vec<RecoveryAction> {
    match kind {
        FailureKind::Memory => vec![
            RecoveryAction::EnsureMemoryDir,
            RecoveryAction::EnsureStateDir,
        ],
        FailureKind::Session => vec![RecoveryAction::EnsureSessionsDir],
        FailureKind::Provider => vec![RecoveryAction::SuggestProviderCheck],
        FailureKind::Compact => vec![RecoveryAction::SuggestCompactFallback],
        FailureKind::Channel => vec![RecoveryAction::SuggestChannelReconnect],
        FailureKind::Tool => vec![RecoveryAction::SuggestToolDependencyCheck],
        FailureKind::Resource => vec![RecoveryAction::SuggestResourceRelief],
        FailureKind::Config | FailureKind::Unknown => vec![RecoveryAction::Noop],
    }
}

fn run_action(action: &RecoveryAction, config: &Config, dry_run: bool) -> RecoveryStepResult {
    match action {
        RecoveryAction::EnsureMemoryDir => {
            let path = config.workspace_dir.join("memory");
            if dry_run {
                return RecoveryStepResult {
                    action: action.clone(),
                    ok: true,
                    message: format!("[dry-run] would ensure {}", path.display()),
                };
            }
            match fs::create_dir_all(&path) {
                Ok(_) => RecoveryStepResult {
                    action: action.clone(),
                    ok: true,
                    message: format!("ensured {}", path.display()),
                },
                Err(e) => RecoveryStepResult {
                    action: action.clone(),
                    ok: false,
                    message: format!("failed to ensure {}: {e}", path.display()),
                },
            }
        }
        RecoveryAction::EnsureSessionsDir => {
            let path = config.workspace_dir.join("sessions");
            if dry_run {
                return RecoveryStepResult {
                    action: action.clone(),
                    ok: true,
                    message: format!("[dry-run] would ensure {}", path.display()),
                };
            }
            match fs::create_dir_all(&path) {
                Ok(_) => RecoveryStepResult {
                    action: action.clone(),
                    ok: true,
                    message: format!("ensured {}", path.display()),
                },
                Err(e) => RecoveryStepResult {
                    action: action.clone(),
                    ok: false,
                    message: format!("failed to ensure {}: {e}", path.display()),
                },
            }
        }
        RecoveryAction::EnsureStateDir => {
            let path = config.workspace_dir.join("state");
            if dry_run {
                return RecoveryStepResult {
                    action: action.clone(),
                    ok: true,
                    message: format!("[dry-run] would ensure {}", path.display()),
                };
            }
            match fs::create_dir_all(&path) {
                Ok(_) => RecoveryStepResult {
                    action: action.clone(),
                    ok: true,
                    message: format!("ensured {}", path.display()),
                },
                Err(e) => RecoveryStepResult {
                    action: action.clone(),
                    ok: false,
                    message: format!("failed to ensure {}: {e}", path.display()),
                },
            }
        }
        RecoveryAction::SuggestProviderCheck => RecoveryStepResult {
            action: action.clone(),
            ok: true,
            message: "建议：检查 API key / 余额 / 网络，执行 `zeroclaw doctor models`".to_string(),
        },
        RecoveryAction::SuggestCompactFallback => RecoveryStepResult {
            action: action.clone(),
            ok: true,
            message: "建议：降低 context 压缩预算或临时关闭 compact 相关配置".to_string(),
        },
        RecoveryAction::SuggestChannelReconnect => RecoveryStepResult {
            action: action.clone(),
            ok: true,
            message: "建议：检查渠道 token 与回调地址并重连".to_string(),
        },
        RecoveryAction::SuggestToolDependencyCheck => RecoveryStepResult {
            action: action.clone(),
            ok: true,
            message: "建议：检查工具依赖是否在 PATH 与执行权限".to_string(),
        },
        RecoveryAction::SuggestResourceRelief => RecoveryStepResult {
            action: action.clone(),
            ok: true,
            message: "建议：释放磁盘/内存并降低并发".to_string(),
        },
        RecoveryAction::Noop => RecoveryStepResult {
            action: action.clone(),
            ok: true,
            message: "暂无自动恢复动作，请根据日志手动处理".to_string(),
        },
    }
}

pub fn recover_failure_by_id(config: &Config, failure_id: &str, dry_run: bool) -> Result<RecoveryReport> {
    let failure = get_failure(failure_id)
        .ok_or_else(|| anyhow::anyhow!("failure not found: {failure_id}"))?;

    let plan = recovery_plan(failure.kind);
    let mut steps = Vec::with_capacity(plan.len());

    for action in &plan {
        steps.push(run_action(action, config, dry_run));
    }

    let success = steps.iter().all(|s| s.ok);

    if success && !dry_run {
        let _ = resolve_failure(
            failure_id,
            Some("auto recovery completed".to_string()),
        );
    }

    Ok(RecoveryReport {
        failure_id: failure_id.to_string(),
        kind: failure.kind,
        success,
        steps,
    })
}
