use serde::{Deserialize, Serialize};

use super::policy::CommandRiskLevel;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDecision {
    Allow,
    Deny,
    RequireApproval,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyAction {
    Allow,
    Deny,
    RequireApproval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub enabled: bool,
    pub priority: i32,
    pub condition: Condition,
    pub action: PolicyAction,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub command_contains: Option<String>,
    pub command_prefix: Option<String>,
    pub min_risk: Option<String>,
    pub tool_name: Option<String>,
    pub sender_id: Option<String>,
    pub channel: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvaluationContext {
    pub tool_name: Option<String>,
    pub sender_id: Option<String>,
    pub channel: Option<String>,
}

impl Condition {
    fn matches(&self, command: &str, risk: CommandRiskLevel, ctx: &EvaluationContext) -> bool {
        if let Some(prefix) = &self.command_prefix {
            if !command.trim_start().starts_with(prefix) {
                return false;
            }
        }
        if let Some(contains) = &self.command_contains {
            if !command.contains(contains) {
                return false;
            }
        }
        if let Some(min_risk) = &self.min_risk {
            let needed = match min_risk.as_str() {
                "low" => 1,
                "medium" => 2,
                "high" => 3,
                _ => 99,
            };
            let actual = match risk {
                CommandRiskLevel::Low => 1,
                CommandRiskLevel::Medium => 2,
                CommandRiskLevel::High => 3,
            };
            if actual < needed {
                return false;
            }
        }

        if let Some(tool_name) = &self.tool_name {
            if ctx.tool_name.as_deref() != Some(tool_name.as_str()) {
                return false;
            }
        }

        if let Some(sender_id) = &self.sender_id {
            if ctx.sender_id.as_deref() != Some(sender_id.as_str()) {
                return false;
            }
        }

        if let Some(channel) = &self.channel {
            if ctx.channel.as_deref() != Some(channel.as_str()) {
                return false;
            }
        }

        true
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicyEngine {
    pub rules: Vec<Rule>,
}

impl PolicyEngine {
    pub fn evaluate(&self, command: &str, risk: CommandRiskLevel) -> Option<(PolicyDecision, String)> {
        self.evaluate_with_context(command, risk, &EvaluationContext::default())
    }

    pub fn evaluate_with_context(
        &self,
        command: &str,
        risk: CommandRiskLevel,
        ctx: &EvaluationContext,
    ) -> Option<(PolicyDecision, String)> {
        let mut rules = self
            .rules
            .iter()
            .filter(|r| r.enabled)
            .collect::<Vec<_>>();
        rules.sort_by_key(|r| -r.priority);

        for rule in rules {
            if rule.condition.matches(command, risk, ctx) {
                let decision = match rule.action {
                    PolicyAction::Allow => PolicyDecision::Allow,
                    PolicyAction::Deny => PolicyDecision::Deny,
                    PolicyAction::RequireApproval => PolicyDecision::RequireApproval,
                };
                let reason = rule
                    .reason
                    .clone()
                    .unwrap_or_else(|| format!("matched rule {}", rule.id));
                return Some((decision, reason));
            }
        }
        None
    }

    pub fn with_default_rules() -> Self {
        Self {
            rules: vec![
                Rule {
                    id: "deny_rm_rf_root".to_string(),
                    enabled: true,
                    priority: 100,
                    condition: Condition {
                        command_contains: Some("rm -rf /".to_string()),
                        command_prefix: None,
                        min_risk: None,
                        tool_name: None,
                        sender_id: None,
                        channel: None,
                    },
                    action: PolicyAction::Deny,
                    reason: Some("dangerous root deletion pattern".to_string()),
                },
                Rule {
                    id: "approval_for_high_risk".to_string(),
                    enabled: true,
                    priority: 50,
                    condition: Condition {
                        command_contains: None,
                        command_prefix: None,
                        min_risk: Some("high".to_string()),
                        tool_name: None,
                        sender_id: None,
                        channel: None,
                    },
                    action: PolicyAction::RequireApproval,
                    reason: Some("high-risk command requires explicit approval".to_string()),
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_engine_denies_dangerous_pattern() {
        let engine = PolicyEngine::with_default_rules();
        let out = engine.evaluate("echo x && rm -rf /", CommandRiskLevel::High);
        assert!(out.is_some());
        let (decision, _) = out.unwrap();
        assert_eq!(decision, PolicyDecision::Deny);
    }
}
