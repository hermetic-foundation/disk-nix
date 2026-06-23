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
    pub command_summary: CommandPlanSummary,
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
    pub readiness: CommandReadiness,
    pub unresolved_inputs: Vec<String>,
    pub note: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CommandReadiness {
    Ready,
    NeedsDesiredSize,
    NeedsDomainImplementation,
    ManualOnly,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandPlanSummary {
    pub step_count: usize,
    pub command_count: usize,
    pub mutating_count: usize,
    pub manual_review_count: usize,
    pub ready_count: usize,
    pub needs_desired_size_count: usize,
    pub needs_domain_implementation_count: usize,
    pub manual_only_count: usize,
}

impl CommandPlanSummary {
    #[must_use]
    pub fn all_commands_ready(&self) -> bool {
        self.needs_desired_size_count == 0
            && self.needs_domain_implementation_count == 0
            && self.manual_only_count == 0
    }
}

#[must_use]
pub fn prepare_execution(plan: &Plan, policy: ApplyPolicy, mode: ExecutionMode) -> ExecutionReport {
    let apply = evaluate_apply_policy(plan, policy);
    let command_plan = command_plan(plan, &apply);
    let command_summary = summarize_command_plan(&command_plan);
    if !apply.can_execute() {
        let blocked_count = apply.blocked_count;
        return ExecutionReport {
            apply,
            status: ExecutionStatus::Blocked,
            command_summary,
            command_plan,
            messages: vec![format!("apply policy blocked {blocked_count} action(s)")],
        };
    }

    match mode {
        ExecutionMode::DryRun => ExecutionReport {
            apply,
            status: ExecutionStatus::DryRun,
            command_summary,
            messages: vec![format!(
                "dry run only: generated {} command plan step(s), no storage commands were run",
                command_plan.len()
            )],
            command_plan,
        },
        ExecutionMode::Execute => ExecutionReport {
            apply,
            status: ExecutionStatus::ExecutorUnavailable,
            command_summary,
            command_plan,
            messages: vec![
                "executor is not implemented yet; policy validation passed but no storage commands were run"
                    .to_string(),
            ],
        },
    }
}

fn summarize_command_plan(command_plan: &[ExecutionStep]) -> CommandPlanSummary {
    let mut summary = CommandPlanSummary {
        step_count: command_plan.len(),
        manual_review_count: command_plan
            .iter()
            .filter(|step| step.requires_manual_review)
            .count(),
        ..CommandPlanSummary::default()
    };

    for command in command_plan.iter().flat_map(|step| &step.commands) {
        summary.command_count += 1;
        if command.mutates {
            summary.mutating_count += 1;
        }
        match command.readiness {
            CommandReadiness::Ready => summary.ready_count += 1,
            CommandReadiness::NeedsDesiredSize => summary.needs_desired_size_count += 1,
            CommandReadiness::NeedsDomainImplementation => {
                summary.needs_domain_implementation_count += 1;
            }
            CommandReadiness::ManualOnly => summary.manual_only_count += 1,
        }
    }

    summary
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
            let grow_command = filesystem_grow_command(fs_type, target);
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "re-read graph state for the filesystem before resizing",
                    ),
                    grow_command,
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
                    command_with_readiness(
                        ["lvextend", "--resizefs", "--size", "+<size>", target],
                        true,
                        CommandReadiness::NeedsDesiredSize,
                        ["desired size delta"],
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
                    command_with_readiness(
                        ["<grow-storage-object-tool>", target],
                        true,
                        CommandReadiness::NeedsDomainImplementation,
                        ["grow tool", "desired size"],
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
                    add_device_command(collection, target, device),
                ],
                vec!["verify the new device identity and redundancy policy before attaching it".to_string()],
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
                    replace_device_command(collection, target, from, to),
                ],
                vec!["keep the old device available until post-apply verification passes".to_string()],
                true,
            )
        }
        Operation::Rebalance => {
            let target = target.unwrap_or("<target>");
            let rebalance = rebalance_command(collection, target);
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect pool or filesystem health before rebalance",
                    ),
                    rebalance,
                ],
                vec!["monitor progress and health until the rebalance operation completes".to_string()],
                true,
            )
        }
        Operation::SetProperty => {
            let target = target.unwrap_or("<target>");
            let property = action
                .context
                .property
                .as_deref()
                .unwrap_or("<key>");
            let property_assignment = property_assignment(action);
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect current properties before applying changes",
                    ),
                    set_property_command(collection, target, property, &property_assignment),
                ],
                vec!["property values must come from the desired spec and target domain".to_string()],
                true,
            )
        }
        Operation::Snapshot => {
            let target = target.unwrap_or("<snapshot>");
            let snapshot = action.context.name.as_deref().unwrap_or(target);
            let snapshot_command = snapshot_command(target, snapshot);
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect snapshot target before creation",
                    ),
                    snapshot_command,
                ],
                Vec::new(),
                true,
            )
        }
        Operation::Create => (
            vec![command_with_readiness(
                ["<create-storage-object-tool>", "<target>"],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["create tool", "target"],
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
    command_with_readiness(argv, mutates, CommandReadiness::Ready, [], note)
}

fn command_with_readiness<const N: usize, const M: usize>(
    argv: [&str; N],
    mutates: bool,
    readiness: CommandReadiness,
    unresolved_inputs: [&str; M],
    note: &str,
) -> ExecutionCommand {
    ExecutionCommand {
        argv: argv.iter().map(|value| (*value).to_string()).collect(),
        mutates,
        readiness,
        unresolved_inputs: unresolved_inputs
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        note: note.to_string(),
    }
}

fn filesystem_grow_command(fs_type: &str, target: &str) -> ExecutionCommand {
    match fs_type {
        "xfs" => command(
            ["xfs_growfs", target],
            true,
            "grow an already-mounted XFS filesystem",
        ),
        "ext2" | "ext3" | "ext4" => command(
            ["resize2fs", target],
            true,
            "grow an ext filesystem after the backing block device has grown",
        ),
        "btrfs" => command(
            ["btrfs", "filesystem", "resize", "max", target],
            true,
            "grow a Btrfs filesystem to the maximum visible device size",
        ),
        "zfs" => command_with_readiness(
            ["zfs", "set", "volsize=<size>", target],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired zvol size"],
            "set the ZFS volume size after selecting the desired size",
        ),
        _ => command_with_readiness(
            ["<filesystem-grow-tool>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem grow tool"],
            "run the filesystem-specific online grow command after device growth is visible",
        ),
    }
}

fn add_device_command(collection: Option<&str>, target: &str, device: &str) -> ExecutionCommand {
    match collection {
        Some("pools") => command(
            ["zpool", "add", target, device],
            true,
            "attach a vdev or device to a ZFS pool when the pool layout supports it",
        ),
        Some("volumeGroups") => command(
            ["vgextend", target, device],
            true,
            "add a physical volume to an LVM volume group",
        ),
        Some("filesystems") => command(
            ["btrfs", "device", "add", device, target],
            true,
            "add a device to a mounted Btrfs filesystem",
        ),
        _ => command_with_readiness(
            ["<add-device-tool>", target, device],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["add-device tool"],
            "attach the new device with the storage-domain-specific tool",
        ),
    }
}

fn replace_device_command(
    collection: Option<&str>,
    target: &str,
    from: &str,
    to: &str,
) -> ExecutionCommand {
    match collection {
        Some("pools") => command(
            ["zpool", "replace", target, from, to],
            true,
            "replace a ZFS pool device and resilver before detaching the old device",
        ),
        Some("filesystems") => command(
            ["btrfs", "replace", "start", from, to, target],
            true,
            "replace a Btrfs filesystem device",
        ),
        Some("caches") => command_with_readiness(
            ["<cache-replace-tool>", target, from, to],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["cache replacement tool", "cache flush or detach workflow"],
            "flush or detach dirty cache state before replacing the cache device",
        ),
        _ => command_with_readiness(
            ["<replace-device-tool>", target, from, to],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["replace-device tool"],
            "start the storage-domain replacement operation",
        ),
    }
}

fn rebalance_command(collection: Option<&str>, target: &str) -> ExecutionCommand {
    match collection {
        Some("pools") => command(
            ["zpool", "scrub", target],
            true,
            "scrub the pool after topology changes; ZFS has no generic rebalance command",
        ),
        Some("filesystems") => command(
            ["btrfs", "balance", "start", target],
            true,
            "rebalance Btrfs chunks across available devices",
        ),
        _ => command_with_readiness(
            ["<rebalance-tool>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["rebalance tool"],
            "run the storage-domain rebalance command",
        ),
    }
}

fn set_property_command(
    collection: Option<&str>,
    target: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    match collection {
        Some("pools") => command(
            ["zpool", "set", assignment, target],
            true,
            "set a ZFS pool property",
        ),
        Some("datasets") => command(
            ["zfs", "set", assignment, target],
            true,
            "set a ZFS dataset property",
        ),
        Some("exports") => command(
            ["exportfs", "-ra"],
            true,
            "reload NFS exports after export property changes",
        ),
        _ => command_with_readiness(
            ["<set-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["property update tool"],
            "apply the storage-domain property update",
        ),
    }
}

fn snapshot_command(target: &str, snapshot: &str) -> ExecutionCommand {
    if snapshot.contains('@') {
        command(["zfs", "snapshot", snapshot], true, "create a ZFS snapshot")
    } else {
        command_with_readiness(
            ["<snapshot-tool>", target, snapshot],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["snapshot tool"],
            "create the snapshot with zfs, btrfs, lvm, or the target-specific tool",
        )
    }
}

fn property_assignment(action: &PlannedAction) -> String {
    let key = action.context.property.as_deref().unwrap_or("<key>");
    let value = action
        .context
        .property_value
        .as_deref()
        .unwrap_or("<value>");
    format!("{key}={value}")
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
        assert_eq!(report.command_summary.step_count, 1);
        assert_eq!(report.command_summary.command_count, 2);
        assert_eq!(report.command_summary.ready_count, 1);
        assert_eq!(report.command_summary.needs_desired_size_count, 1);
        assert!(!report.command_summary.all_commands_ready());
        assert!(report.command_plan[0].requires_manual_review);
        assert!(report.command_plan[0].commands.iter().any(|command| {
            command
                .argv
                .first()
                .is_some_and(|program| program == "lvextend")
                && command.argv.contains(&"vg/root".to_string())
                && command.readiness == CommandReadiness::NeedsDesiredSize
                && command.unresolved_inputs == ["desired size delta"]
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
        assert_eq!(report.command_summary.command_count, 0);
        assert!(report.command_summary.all_commands_ready());
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

    #[test]
    fn pool_actions_report_domain_specific_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "pools": {
                "tank": {
                  "operation": "rebalance",
                  "addDevices": ["/dev/disk/by-id/new"],
                  "properties": {
                    "autotrim": "on"
                  }
                }
              },
              "snapshots": {
                "tank/home@before": {
                  "target": "tank/home"
                }
              },
              "apply": {
                "allowGrow": true,
                "allowPropertyChanges": true
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv == ["zpool", "add", "tank", "/dev/disk/by-id/new"]
                    && command.readiness == CommandReadiness::Ready
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["zpool", "set", "autotrim=on", "tank"])
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["zfs", "snapshot", "tank/home@before"])
        }));
        assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
        assert!(report.command_summary.all_commands_ready());
    }
}
