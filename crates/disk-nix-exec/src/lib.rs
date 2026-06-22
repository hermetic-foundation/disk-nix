use disk_nix_plan::{ApplyPolicy, ApplyReport, Plan, evaluate_apply_policy};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionMode {
    DryRun,
    Execute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionStatus {
    DryRun,
    Blocked,
    ExecutorUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionReport {
    pub apply: ApplyReport,
    pub status: ExecutionStatus,
    pub messages: Vec<String>,
}

impl ExecutionReport {
    #[must_use]
    pub fn can_apply(&self) -> bool {
        self.status == ExecutionStatus::DryRun && self.apply.can_execute()
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[must_use]
pub fn prepare_execution(plan: &Plan, policy: ApplyPolicy, mode: ExecutionMode) -> ExecutionReport {
    let apply = evaluate_apply_policy(plan, policy);
    if !apply.can_execute() {
        let blocked_count = apply.blocked_count;
        return ExecutionReport {
            apply,
            status: ExecutionStatus::Blocked,
            messages: vec![format!("apply policy blocked {blocked_count} action(s)")],
        };
    }

    match mode {
        ExecutionMode::DryRun => ExecutionReport {
            apply,
            status: ExecutionStatus::DryRun,
            messages: vec!["dry run only: no storage commands were run".to_string()],
        },
        ExecutionMode::Execute => ExecutionReport {
            apply,
            status: ExecutionStatus::ExecutorUnavailable,
            messages: vec![
                "executor is not implemented yet; policy validation passed but no storage commands were run"
                    .to_string(),
            ],
        },
    }
}

#[cfg(test)]
mod tests {
    use disk_nix_plan::plan_and_policy_from_json_bytes;

    use super::*;

    #[test]
    fn dry_run_reports_no_mutation_when_policy_allows_plan() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "volumes": {
                  "vg/root": { "operation": "grow" }
                }
              },
              "apply": {
                "allowGrow": true
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert!(report.can_apply());
    }

    #[test]
    fn execute_reports_executor_unavailable_after_policy_passes() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "volumes": {
                  "vg/root": { "operation": "grow" }
                }
              },
              "apply": {
                "allowGrow": true
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::Execute);

        assert_eq!(report.status, ExecutionStatus::ExecutorUnavailable);
        assert!(!report.can_apply());
    }

    #[test]
    fn blocked_policy_reports_blocked_status() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "datasets": {
                  "tank/old": { "destroy": true }
                }
              },
              "apply": {
                "allowDestructive": false
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::Blocked);
        assert_eq!(report.apply.blocked_count, 1);
    }
}
