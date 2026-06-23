use std::collections::BTreeSet;

use disk_nix_plan::{
    ApplyPolicy, ApplyReport, Operation, Plan, PlannedAction, RiskClass, evaluate_apply_policy,
};
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
    pub command_plan: Vec<ExecutionStep>,
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionStep {
    pub action_id: String,
    pub operation: Operation,
    pub risk: RiskClass,
    pub requires_manual_review: bool,
    pub commands: Vec<ExecutionCommand>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionCommand {
    pub argv: Vec<String>,
    pub mutates: bool,
    pub note: String,
}

#[must_use]
pub fn prepare_execution(plan: &Plan, policy: ApplyPolicy, mode: ExecutionMode) -> ExecutionReport {
    let apply = evaluate_apply_policy(plan, policy);
    let command_plan = command_plan(plan, &apply);
    if !apply.can_execute() {
        let blocked_count = apply.blocked_count;
        return ExecutionReport {
            apply,
            status: ExecutionStatus::Blocked,
            command_plan,
            messages: vec![format!("apply policy blocked {blocked_count} action(s)")],
        };
    }

    match mode {
        ExecutionMode::DryRun => ExecutionReport {
            apply,
            status: ExecutionStatus::DryRun,
            messages: vec![format!(
                "dry run only: generated {} command plan step(s), no storage commands were run",
                command_plan.len()
            )],
            command_plan,
        },
        ExecutionMode::Execute => ExecutionReport {
            apply,
            status: ExecutionStatus::ExecutorUnavailable,
            command_plan,
            messages: vec![
                "executor is not implemented yet; policy validation passed but no storage commands were run"
                    .to_string(),
            ],
        },
    }
}

fn command_plan(plan: &Plan, apply: &ApplyReport) -> Vec<ExecutionStep> {
    let blocked: BTreeSet<&str> = apply
        .blocked
        .iter()
        .map(|blocked| blocked.id.as_str())
        .collect();

    plan.actions
        .iter()
        .filter(|action| !blocked.contains(action.id.as_str()))
        .map(execution_step)
        .collect()
}

fn execution_step(action: &PlannedAction) -> ExecutionStep {
    let (commands, mut notes, requires_manual_review) = commands_for_action(action);
    if let Some(advice) = &action.advice {
        notes.push(format!("advice: {}", advice.summary));
        notes.extend(
            advice
                .alternatives
                .iter()
                .map(|alternative| format!("alternative: {alternative}")),
        );
    }

    ExecutionStep {
        action_id: action.id.clone(),
        operation: action.operation,
        risk: action.risk,
        requires_manual_review,
        commands,
        notes,
    }
}

fn commands_for_action(action: &PlannedAction) -> (Vec<ExecutionCommand>, Vec<String>, bool) {
    let parts: Vec<&str> = action.id.split(':').collect();
    let collection = action
        .context
        .collection
        .as_deref()
        .or_else(|| parts.first().copied());
    let target = action
        .context
        .target
        .as_deref()
        .or(action.context.name.as_deref())
        .or_else(|| parts.get(1).copied());
    match action.operation {
        Operation::Grow if collection == Some("filesystems") || action.id.starts_with("filesystem:") => {
            let target = target.unwrap_or("<filesystem>");
            let fs_type = action.context.fs_type.as_deref().unwrap_or("<filesystem-type>");
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "re-read graph state for the filesystem before resizing",
                    ),
                    command(
                        ["<filesystem-grow-tool>", target],
                        true,
                        "run the filesystem-specific online grow command after device growth is visible",
                    ),
                ],
                vec![
                    format!(
                        "select the {fs_type} grow command: xfs_growfs, resize2fs, btrfs filesystem resize, zfs set volsize, or equivalent"
                    ),
                    "verify available backing capacity before running the grow command".to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("volumes") || action.id.starts_with("volumes:") => {
            let target = target.unwrap_or("<volume>");
            (
                vec![
                    command(
                        ["lvs", "--reportformat", "json", target],
                        false,
                        "inspect current LVM logical volume state",
                    ),
                    command(
                        ["lvextend", "--resizefs", "--size", "+<size>", target],
                        true,
                        "grow the logical volume and filesystem together",
                    ),
                ],
                vec!["replace <size> after comparing desired state with probed capacity".to_string()],
                true,
            )
        }
        Operation::Grow if collection == Some("luns") || action.id.starts_with("luns:") => {
            let target = target.unwrap_or("<lun>");
            (
                vec![
                    command(
                        ["iscsiadm", "--mode", "session", "--rescan"],
                        true,
                        "rescan iSCSI sessions after target-side LUN growth",
                    ),
                    command(
                        ["multipath", "-r"],
                        true,
                        "reload multipath maps when the LUN is multipathed",
                    ),
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "verify that consumers see the new capacity",
                    ),
                ],
                vec!["coordinate the target-side LUN grow before host rescans".to_string()],
                true,
            )
        }
        Operation::Grow
            if collection == Some("iscsiSessions") || action.id.starts_with("iscsiSessions:") =>
        {
            let target = target.unwrap_or("<iscsi-session>");
            (
            vec![
                command(
                    ["iscsiadm", "--mode", "session", "--rescan"],
                    true,
                    "rescan iSCSI sessions after target-side changes",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify updated iSCSI, LUN, and consumer topology",
                ),
            ],
            vec!["coordinate session rescans with every dependent LUN consumer".to_string()],
            true,
            )
        }
        Operation::Grow => {
            let target = target.unwrap_or("<target>");
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect current target state before growth",
                    ),
                    command(
                        ["<grow-storage-object-tool>", target],
                        true,
                        "grow the storage object with the target-domain-specific command",
                    ),
                ],
                vec!["select the grow command from the target storage domain and desired size".to_string()],
                true,
            )
        }
        Operation::AddDevice => {
            let target = target.unwrap_or("<target>");
            let device = action
                .context
                .device
                .as_deref()
                .or_else(|| parts.last().copied())
                .unwrap_or("<device>");
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect target health before adding a device",
                    ),
                    command(
                        ["<pool-or-volume-add-device-tool>", target, device],
                        true,
                        "attach the new device with the storage-domain-specific tool",
                    ),
                ],
                vec!["choose zpool add, btrfs device add, vgextend, or an equivalent domain command from the target kind".to_string()],
                true,
            )
        }
        Operation::ReplaceDevice => {
            let target = target.unwrap_or("<target>");
            let from = action
                .context
                .device
                .as_deref()
                .unwrap_or("<old-device>");
            let to = action
                .context
                .replacement
                .as_deref()
                .unwrap_or("<new-device>");
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect redundancy and source device health before replacement",
                    ),
                    command(
                        ["<replace-device-tool>", target, from, to],
                        true,
                        "start the storage-domain replacement operation",
                    ),
                ],
                vec!["keep the old device available until post-apply verification passes".to_string()],
                true,
            )
        }
        Operation::Rebalance => {
            let target = target.unwrap_or("<target>");
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect pool or filesystem health before rebalance",
                    ),
                    command(
                        ["<rebalance-tool>", target],
                        true,
                        "run the storage-domain rebalance command",
                    ),
                ],
                vec!["choose btrfs balance or the relevant pool-specific balancing operation".to_string()],
                true,
            )
        }
        Operation::SetProperty => {
            let target = target.unwrap_or("<target>");
            let property = action
                .context
                .property
                .as_deref()
                .unwrap_or("<key>=<value>");
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect current properties before applying changes",
                    ),
                    command(
                        ["<set-property-tool>", target, property],
                        true,
                        "apply the storage-domain property update",
                    ),
                ],
                vec!["property values must come from the desired spec and target domain".to_string()],
                true,
            )
        }
        Operation::Snapshot => {
            let target = target.unwrap_or("<snapshot>");
            let snapshot = action.context.name.as_deref().unwrap_or(target);
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect snapshot target before creation",
                    ),
                    command(
                        ["<snapshot-tool>", target, snapshot],
                        true,
                        "create the snapshot with zfs, btrfs, lvm, or the target-specific tool",
                    ),
                ],
                Vec::new(),
                true,
            )
        }
        Operation::Create => (
            vec![command(
                ["<create-storage-object-tool>", "<target>"],
                true,
                "create the requested storage object",
            )],
            vec!["creation commands require target-kind-specific arguments from the desired spec".to_string()],
            true,
        ),
        Operation::Format | Operation::Shrink | Operation::RemoveDevice | Operation::Rollback | Operation::Destroy => (
            Vec::new(),
            vec!["no command plan is generated for this risk class unless future explicit policy and executor support are added".to_string()],
            true,
        ),
    }
}

fn command<const N: usize>(argv: [&str; N], mutates: bool, note: &str) -> ExecutionCommand {
    ExecutionCommand {
        argv: argv.iter().map(|value| (*value).to_string()).collect(),
        mutates,
        note: note.to_string(),
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
        assert_eq!(report.command_plan.len(), 1);
        assert!(report.command_plan[0].requires_manual_review);
        assert!(report.command_plan[0].commands.iter().any(|command| {
            command
                .argv
                .first()
                .is_some_and(|program| program == "lvextend")
                && command.argv.contains(&"vg/root".to_string())
        }));
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
        assert_eq!(report.command_plan.len(), 1);
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
        assert!(report.command_plan.is_empty());
    }

    #[test]
    fn allowed_lun_growth_reports_rescan_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "luns": {
                "iqn.2026-06.example:storage/root:0": {
                  "operation": "grow"
                }
              },
              "apply": {
                "allowGrow": true,
                "allowOffline": true
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert_eq!(report.command_plan.len(), 1);
        assert!(report.command_plan[0].commands.iter().any(|command| {
            command.argv == ["iscsiadm", "--mode", "session", "--rescan"] && command.mutates
        }));
    }
}
