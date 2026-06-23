use std::collections::BTreeSet;

use disk_nix_plan::{
    ApplyPolicy, ApplyReport, Operation, Plan, PlannedAction, RiskClass, TopologyComparison,
    evaluate_apply_policy,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topology_comparison: Option<TopologyComparison>,
    pub command_summary: CommandPlanSummary,
    pub command_plan: Vec<ExecutionStep>,
    pub verification_summary: VerificationPlanSummary,
    pub verification_plan: Vec<VerificationStep>,
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

    #[must_use]
    pub fn to_shell_script(&self) -> Option<String> {
        self.apply.can_execute().then(|| render_shell_script(self))
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationStep {
    pub action_id: String,
    pub operation: Operation,
    pub risk: RiskClass,
    pub commands: Vec<ExecutionCommand>,
    pub checks: Vec<String>,
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationPlanSummary {
    pub step_count: usize,
    pub command_count: usize,
    pub check_count: usize,
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
    let topology_comparison = plan.topology_comparison.clone();
    let command_plan = command_plan(plan, &apply);
    let command_summary = summarize_command_plan(&command_plan);
    let verification_plan = verification_plan(plan, &apply);
    let verification_summary = summarize_verification_plan(&verification_plan);
    if !apply.can_execute() {
        let blocked_count = apply.blocked_count;
        return ExecutionReport {
            apply,
            status: ExecutionStatus::Blocked,
            topology_comparison,
            command_summary,
            command_plan,
            verification_summary,
            verification_plan,
            messages: vec![format!("apply policy blocked {blocked_count} action(s)")],
        };
    }

    match mode {
        ExecutionMode::DryRun => ExecutionReport {
            apply,
            status: ExecutionStatus::DryRun,
            topology_comparison,
            command_summary,
            verification_summary,
            messages: vec![format!(
                "dry run only: generated {} command plan step(s) and {} verification step(s), no storage commands were run",
                command_plan.len(),
                verification_plan.len()
            )],
            command_plan,
            verification_plan,
        },
        ExecutionMode::Execute => ExecutionReport {
            apply,
            status: ExecutionStatus::ExecutorUnavailable,
            topology_comparison,
            command_summary,
            command_plan,
            verification_summary,
            verification_plan,
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

fn summarize_verification_plan(verification_plan: &[VerificationStep]) -> VerificationPlanSummary {
    VerificationPlanSummary {
        step_count: verification_plan.len(),
        command_count: verification_plan
            .iter()
            .map(|step| step.commands.len())
            .sum(),
        check_count: verification_plan.iter().map(|step| step.checks.len()).sum(),
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

fn verification_plan(plan: &Plan, apply: &ApplyReport) -> Vec<VerificationStep> {
    let blocked: BTreeSet<&str> = apply
        .blocked
        .iter()
        .map(|blocked| blocked.id.as_str())
        .collect();

    plan.actions
        .iter()
        .filter(|action| !blocked.contains(action.id.as_str()))
        .map(verification_step)
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

fn verification_step(action: &PlannedAction) -> VerificationStep {
    let (commands, checks) = verification_for_action(action);
    VerificationStep {
        action_id: action.id.clone(),
        operation: action.operation,
        risk: action.risk,
        commands,
        checks,
    }
}

fn render_shell_script(report: &ExecutionReport) -> String {
    let mut script = String::from(
        "#!/usr/bin/env bash\nset -euo pipefail\n\n# Generated by disk-nix.\n# Review every command before running this script on a storage host.\n\n",
    );

    if let Some(comparison) = &report.topology_comparison {
        script.push_str(&format!(
            "# Topology comparison: {} matched, {} missing, {} size diagnostics, {} type conflicts, {} already satisfied.\n\n",
            comparison.summary.matched_count,
            comparison.summary.missing_count,
            comparison.summary.size_diagnostic_count,
            comparison.summary.type_conflict_count,
            comparison.summary.already_satisfied_count
        ));
    }

    script.push_str("# Planned storage commands\n");
    for step in &report.command_plan {
        script.push_str(&format!(
            "\n# {:?} {:?} {}\n",
            step.risk, step.operation, step.action_id
        ));
        if step.requires_manual_review {
            script.push_str("# Manual review required before running this step.\n");
        }
        for note in &step.notes {
            script.push_str("# ");
            script.push_str(note);
            script.push('\n');
        }
        for command in &step.commands {
            render_script_command(&mut script, command);
        }
    }

    if !report.verification_plan.is_empty() {
        script.push_str("\n# Post-apply verification commands\n");
        for step in &report.verification_plan {
            script.push_str(&format!(
                "\n# Verify {:?} {:?} {}\n",
                step.risk, step.operation, step.action_id
            ));
            for check in &step.checks {
                script.push_str("# Check: ");
                script.push_str(check);
                script.push('\n');
            }
            for command in &step.commands {
                render_script_command(&mut script, command);
            }
        }
    }

    script
}

fn render_script_command(script: &mut String, command: &ExecutionCommand) {
    script.push_str("# ");
    script.push_str(&command.note);
    script.push('\n');
    if !command.unresolved_inputs.is_empty() {
        script.push_str("# Unresolved inputs: ");
        script.push_str(&command.unresolved_inputs.join(", "));
        script.push('\n');
    }
    if command.readiness == CommandReadiness::Ready {
        script.push_str(&shell_command(&command.argv));
    } else {
        script.push_str("# NOT READY: ");
        script.push_str(&shell_command(&command.argv));
    }
    script.push('\n');
}

fn shell_command(argv: &[String]) -> String {
    argv.iter()
        .map(|argument| shell_quote(argument))
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_quote(argument: &str) -> String {
    if argument.chars().all(|c| {
        c.is_ascii_alphanumeric()
            || matches!(c, '/' | '.' | '_' | '-' | ':' | '=' | '+' | '@' | '%')
    }) {
        argument.to_string()
    } else {
        format!("'{}'", argument.replace('\'', "'\"'\"'"))
    }
}

fn verification_for_action(action: &PlannedAction) -> (Vec<ExecutionCommand>, Vec<String>) {
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
        .or_else(|| parts.get(1).copied())
        .unwrap_or("<target>");
    let mountpoint = action.context.mountpoint.as_deref();
    let fs_type = action.context.fs_type.as_deref();
    let desired_size = action.context.desired_size.as_deref();

    match action.operation {
        Operation::Grow
            if collection == Some("filesystems") || action.id.starts_with("filesystem:") =>
        {
            let mut commands = vec![command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify the post-apply filesystem graph node",
            )];
            if let Some(mountpoint) = mountpoint {
                commands.push(command(
                    ["findmnt", "--json", "--bytes", mountpoint],
                    false,
                    "confirm the mounted filesystem reports the expected capacity",
                ));
            }
            match fs_type {
                Some("btrfs") => commands.push(command(
                    ["btrfs", "filesystem", "usage", "-b", target],
                    false,
                    "inspect Btrfs allocation and free space after resize",
                )),
                Some("zfs") => commands.push(command(
                    ["zfs", "list", "-H", "-p", target],
                    false,
                    "inspect ZFS dataset or zvol size after resize",
                )),
                _ => {}
            }
            (
                commands,
                vec![
                    desired_size
                        .map(|size| format!("filesystem size is at least {size}"))
                        .unwrap_or_else(|| {
                            "filesystem size is at least the desired size".to_string()
                        }),
                    "mountpoint remains present and writable when it was mounted before apply"
                        .to_string(),
                    "free and used byte counters are internally consistent after re-probe"
                        .to_string(),
                ],
            )
        }
        Operation::Grow if collection == Some("volumes") || action.id.starts_with("volumes:") => (
            vec![
                command(
                    ["lvs", "--reportformat", "json", target],
                    false,
                    "verify LVM logical volume size and attributes",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify dependent filesystem and mapping graph state",
                ),
            ],
            vec![
                desired_size
                    .map(|size| format!("logical volume reports size {size}"))
                    .unwrap_or_else(|| "logical volume reports the desired size".to_string()),
                "dependent filesystem capacity reflects the grown backing volume".to_string(),
            ],
        ),
        Operation::Grow if collection == Some("thinPools") => (
            vec![
                command(
                    [
                        "lvs",
                        "--reportformat",
                        "json",
                        "-o",
                        "lv_name,lv_size,data_percent,metadata_percent,seg_monitor",
                        target,
                    ],
                    false,
                    "verify thin pool size, data usage, metadata usage, and monitoring state",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify thin pool graph node and dependent thin volumes",
                ),
            ],
            vec![
                desired_size
                    .map(|size| format!("thin pool reports size {size}"))
                    .unwrap_or_else(|| "thin pool reports the desired size".to_string()),
                "data and metadata percentages remain below operational thresholds".to_string(),
                "dependent thin volumes remain active and monitored".to_string(),
            ],
        ),
        Operation::Grow if collection == Some("swaps") => (
            vec![
                command(
                    ["swapon", "--show", "--bytes", "--raw"],
                    false,
                    "verify active swap devices after resize workflow",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify swap graph node and backing storage",
                ),
            ],
            vec![
                "swap target reports the intended capacity".to_string(),
                "swap is active only after backing resize and signature recreation are complete"
                    .to_string(),
            ],
        ),
        Operation::Grow if collection == Some("luks.devices") => (
            vec![
                command(
                    ["cryptsetup", "status", target],
                    false,
                    "verify LUKS mapper state after resize",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify LUKS mapping and dependent graph layers",
                ),
            ],
            vec![
                "LUKS mapper sector count reflects the grown backing device".to_string(),
                "dependent LVM, filesystem, and mount layers see the new mapper capacity"
                    .to_string(),
            ],
        ),
        Operation::Grow if collection == Some("vdoVolumes") => (
            vec![
                command(
                    ["vdo", "status", "--name", target],
                    false,
                    "verify VDO volume configuration after growth",
                ),
                command(
                    ["vdostats", "--human-readable", target],
                    false,
                    "verify VDO runtime capacity, utilization, and savings after growth",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify VDO graph node and backing relationships",
                ),
            ],
            vec![
                "VDO logical or physical size matches desired state".to_string(),
                "used, available, and space-saving counters are reviewed after growth".to_string(),
                "dependent filesystems or mappings see the intended capacity".to_string(),
            ],
        ),
        Operation::Grow if collection == Some("zvols") => (
            vec![
                command(
                    ["zfs", "list", "-H", "-p", "-t", "volume", target],
                    false,
                    "verify zvol volsize after growth",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify zvol graph node and dependent block consumers",
                ),
            ],
            vec![
                desired_size
                    .map(|size| format!("zvol volsize reports {size}"))
                    .unwrap_or_else(|| "zvol volsize reports the desired capacity".to_string()),
                "dependent LUNs, guests, partitions, or filesystems see the intended capacity"
                    .to_string(),
            ],
        ),
        Operation::Grow if collection == Some("loopDevices") => (
            vec![
                command(
                    ["losetup", "--json", "--list", target],
                    false,
                    "verify loop device size and backing file after refresh",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify loop graph node and dependent consumers",
                ),
            ],
            vec![
                "loop device reports the refreshed backing size".to_string(),
                "dependent mappings or filesystems see the intended capacity".to_string(),
            ],
        ),
        Operation::Create | Operation::Grow if collection == Some("partitions") => (
            vec![
                command(
                    ["lsblk", "--json", "--bytes", "--output-all"],
                    false,
                    "verify kernel partition and consumer topology",
                ),
                command(
                    ["parted", "-lm"],
                    false,
                    "verify partition table geometry after the change",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify partition graph node and dependent mappings",
                ),
            ],
            vec![
                "partition start, end, size, type, and flags match desired state".to_string(),
                "kernel reread succeeded or a reboot is scheduled before resizing consumers"
                    .to_string(),
                "dependent LUKS, LVM, filesystem, and mount layers still resolve correctly"
                    .to_string(),
            ],
        ),
        Operation::Grow
            if collection == Some("luns")
                || collection == Some("iscsiSessions")
                || action.id.starts_with("luns:")
                || action.id.starts_with("iscsiSessions:") =>
        {
            (
                vec![
                    command(
                        ["lsblk", "--json", "--bytes", "--output-all"],
                        false,
                        "verify kernel block-device capacity after host rescan",
                    ),
                    command(
                        ["disk-nix", "inspect", target, "--json"],
                        false,
                        "verify LUN, iSCSI session, multipath, and consumers in the graph",
                    ),
                ],
                vec![
                    desired_size
                        .map(|size| format!("every expected path reports capacity {size}"))
                        .unwrap_or_else(|| {
                            "every expected path reports the new capacity".to_string()
                        }),
                    "multipath maps and dependent volumes no longer report stale sizes".to_string(),
                    "no consumer remains on a missing or failed path".to_string(),
                ],
            )
        }
        Operation::AddDevice | Operation::ReplaceDevice | Operation::Rebalance
            if collection == Some("pools") =>
        {
            (
                vec![
                    command(
                        ["zpool", "status", "-P", target],
                        false,
                        "verify ZFS pool health and device topology",
                    ),
                    command(
                        ["disk-nix", "inspect", target, "--json"],
                        false,
                        "verify pool graph relationships after topology change",
                    ),
                ],
                vec![
                    "pool is online or degraded only in an explicitly accepted state".to_string(),
                    "new, replaced, or rebalanced devices match desired topology".to_string(),
                    "scrub, resilver, or rebalance status is reviewed to completion".to_string(),
                ],
            )
        }
        Operation::AddDevice | Operation::ReplaceDevice | Operation::Rebalance
            if collection == Some("filesystems") =>
        {
            (
                vec![
                    command(
                        ["btrfs", "filesystem", "usage", "-b", target],
                        false,
                        "verify Btrfs device allocation after topology change",
                    ),
                    command(
                        ["disk-nix", "inspect", target, "--json"],
                        false,
                        "verify filesystem graph relationships after topology change",
                    ),
                ],
                vec![
                    "Btrfs device list matches desired topology".to_string(),
                    "allocation profiles remain intentional after rebalance".to_string(),
                ],
            )
        }
        Operation::AddDevice
        | Operation::ReplaceDevice
        | Operation::RemoveDevice
        | Operation::Grow
            if collection == Some("mdRaids") =>
        {
            (
                vec![
                    command(
                        ["mdadm", "--detail", target],
                        false,
                        "verify MD RAID array health and member topology",
                    ),
                    command(
                        ["cat", "/proc/mdstat"],
                        false,
                        "verify MD RAID sync, recovery, or reshape state",
                    ),
                    command(
                        ["disk-nix", "inspect", target, "--json"],
                        false,
                        "verify MD RAID graph relationships after topology change",
                    ),
                ],
                vec![
                    "array is clean or intentionally syncing, recovering, or reshaping".to_string(),
                    "member list and redundancy match the desired topology".to_string(),
                    "dependent filesystems or mappings see the expected capacity".to_string(),
                ],
            )
        }
        Operation::AddDevice
        | Operation::ReplaceDevice
        | Operation::RemoveDevice
        | Operation::Grow
            if collection == Some("multipathMaps") =>
        {
            (
                vec![
                    command(
                        ["multipath", "-ll", target],
                        false,
                        "verify multipath map paths, policy, and size",
                    ),
                    command(
                        ["disk-nix", "inspect", target, "--json"],
                        false,
                        "verify multipath graph relationships after path or map change",
                    ),
                ],
                vec![
                    "all expected paths are active or intentionally failed".to_string(),
                    "map size and WWID match desired state".to_string(),
                    "dependent filesystems or mappings see the expected capacity".to_string(),
                ],
            )
        }
        Operation::AddDevice
        | Operation::ReplaceDevice
        | Operation::RemoveDevice
        | Operation::SetProperty
            if collection == Some("caches") =>
        {
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target, "--json"],
                        false,
                        "verify modeled cache layer relationships after cache update",
                    ),
                    bcache_sysfs_read_command(target, "state", "verify bcache state after update"),
                    bcache_sysfs_read_command(
                        target,
                        "dirty_data",
                        "verify dirty data after cache update",
                    ),
                ],
                vec![
                    "cache backing device and cache-set relationships match desired topology"
                        .to_string(),
                    "dirty writeback data is flushed before detach or replacement".to_string(),
                    "cache mode matches the desired safety posture after the operation".to_string(),
                ],
            )
        }
        Operation::AddDevice | Operation::ReplaceDevice | Operation::Rebalance => (
            vec![command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify storage topology after device-level operation",
            )],
            vec!["target topology and health match the desired state".to_string()],
        ),
        Operation::SetProperty if collection == Some("pools") => (
            vec![
                command(
                    ["zpool", "get", "all", target],
                    false,
                    "verify ZFS pool properties after update",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled pool properties after re-probe",
                ),
            ],
            vec!["changed property equals the desired value".to_string()],
        ),
        Operation::SetProperty if collection == Some("datasets") => (
            vec![
                command(
                    ["zfs", "get", "all", target],
                    false,
                    "verify ZFS dataset properties after update",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled dataset properties after re-probe",
                ),
            ],
            vec!["changed property equals the desired value".to_string()],
        ),
        Operation::SetProperty if collection == Some("exports") => (
            vec![
                command(
                    ["exportfs", "-v"],
                    false,
                    "verify exported NFS paths and options",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled NFS export properties after re-probe",
                ),
            ],
            vec!["exported path and options match the desired value".to_string()],
        ),
        Operation::SetProperty => (
            vec![command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify modeled storage properties after re-probe",
            )],
            vec!["changed property equals the desired value".to_string()],
        ),
        Operation::Snapshot => {
            let snapshot = action.context.name.as_deref().unwrap_or(target);
            let mut commands = vec![command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify snapshot target and relationships after creation",
            )];
            if snapshot.contains('@') {
                commands.push(command(
                    ["zfs", "list", "-t", "snapshot", "-H", "-p", snapshot],
                    false,
                    "verify ZFS snapshot existence and metadata",
                ));
            } else if collection == Some("btrfsSubvolumes") {
                commands.push(command(
                    ["btrfs", "subvolume", "show", snapshot],
                    false,
                    "verify Btrfs snapshot subvolume existence and metadata",
                ));
            } else if collection == Some("lvmSnapshots") {
                commands.push(command(
                    ["lvs", "--reportformat", "json", snapshot],
                    false,
                    "verify LVM snapshot existence and attributes",
                ));
            }
            (
                commands,
                vec![
                    "snapshot exists with the expected name".to_string(),
                    "snapshot source still resolves to the intended dataset or volume".to_string(),
                ],
            )
        }
        Operation::Create => (
            vec![command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify the created storage object is present in the graph",
            )],
            vec![
                "created object identity, size, and relationships match desired state".to_string(),
            ],
        ),
        Operation::Destroy if collection == Some("zvols") => (
            vec![command(
                ["zfs", "list", "-H", "-p", "-t", "volume"],
                false,
                "verify zvol inventory after destruction",
            )],
            vec![
                "destroyed zvol no longer appears in ZFS volume listings".to_string(),
                "downstream LUN, guest, or filesystem consumers are detached or updated"
                    .to_string(),
            ],
        ),
        Operation::Destroy if collection == Some("lvmSnapshots") => (
            vec![command(
                ["lvs", "--reportformat", "json"],
                false,
                "verify LVM snapshot inventory after removal",
            )],
            vec![
                "removed snapshot no longer appears in LVM reports".to_string(),
                "origin logical volume remains active and healthy".to_string(),
            ],
        ),
        Operation::Destroy if collection == Some("loopDevices") => (
            vec![command(
                ["losetup", "--json", "--list"],
                false,
                "verify loop device is detached while backing file remains",
            )],
            vec![
                "loop device no longer appears in losetup inventory".to_string(),
                "backing file or block device remains intact".to_string(),
            ],
        ),
        Operation::Rollback if collection == Some("lvmSnapshots") => (
            vec![
                command(
                    ["lvs", "--reportformat", "json", target],
                    false,
                    "verify LVM snapshot merge state",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify origin and snapshot graph state after rollback",
                ),
            ],
            vec![
                "snapshot merge is complete or queued for next activation".to_string(),
                "origin logical volume contents and consumers are verified after merge".to_string(),
            ],
        ),
        Operation::Format if collection == Some("swaps") => (
            vec![
                command(
                    ["blkid", target],
                    false,
                    "verify swap signature identity after mkswap",
                ),
                command(
                    ["swapon", "--show", "--bytes", "--raw"],
                    false,
                    "verify swap activation state after signature creation",
                ),
            ],
            vec![
                "target has a swap signature and no unexpected filesystem signature".to_string(),
                "swap activation follows the desired NixOS swapDevices configuration".to_string(),
            ],
        ),
        Operation::Format if collection == Some("luks.devices") => (
            vec![
                command(
                    ["cryptsetup", "isLuks", target],
                    false,
                    "verify the target is a LUKS container",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify encrypted container identity and graph relationships",
                ),
            ],
            vec![
                "LUKS header exists and recovery header backup has been captured".to_string(),
                "mapper name and backing device match the desired declaration".to_string(),
            ],
        ),
        Operation::Destroy if collection == Some("btrfsSubvolumes") => (
            vec![command(
                ["disk-nix", "topology", "--json"],
                false,
                "re-probe full topology after Btrfs subvolume deletion",
            )],
            vec![
                "deleted Btrfs subvolume path no longer appears in subvolume listings".to_string(),
                "snapshots, qgroups, and mount consumers are reviewed after deletion".to_string(),
            ],
        ),
        Operation::Format
        | Operation::Shrink
        | Operation::RemoveDevice
        | Operation::Rollback
        | Operation::Destroy => (
            vec![command(
                ["disk-nix", "topology", "--json"],
                false,
                "re-probe full topology after high-risk operation",
            )],
            vec!["operator performs explicit high-risk post-change validation".to_string()],
        ),
        Operation::Grow => (
            vec![command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify target state after grow operation",
            )],
            vec!["target capacity and consumers match desired state".to_string()],
        ),
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
            let desired_size = action.context.desired_size.as_deref();
            let grow_command = filesystem_grow_command(fs_type, target, desired_size);
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
            let desired_size = action.context.desired_size.as_deref();
            let grow_command = match desired_size {
                Some(size) => command_vec(
                    vec!["lvextend", "--resizefs", "--size", size, target],
                    true,
                    "grow the logical volume and filesystem to the desired size",
                ),
                None => command_with_readiness(
                    ["lvextend", "--resizefs", "--size", "+<size>", target],
                    true,
                    CommandReadiness::NeedsDesiredSize,
                    ["desired size delta"],
                    "grow the logical volume and filesystem together",
                ),
            };
            let note = desired_size
                .map(|size| format!("desired size from spec: {size}"))
                .unwrap_or_else(|| "replace <size> after comparing desired state with probed capacity".to_string());
            (
                vec![
                    command(
                        ["lvs", "--reportformat", "json", target],
                        false,
                        "inspect current LVM logical volume state",
                    ),
                    grow_command,
                ],
                vec![note],
                true,
            )
        }
        Operation::Grow if collection == Some("thinPools") => {
            let target = target.unwrap_or("<thin-pool>");
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    command(
                        [
                            "lvs",
                            "--reportformat",
                            "json",
                            "-o",
                            "lv_name,lv_size,data_percent,metadata_percent,seg_monitor",
                            target,
                        ],
                        false,
                        "inspect current thin pool data and metadata utilization",
                    ),
                    thin_pool_extend_command(target, desired_size),
                ],
                vec![
                    "extend metadata before it approaches exhaustion".to_string(),
                    "verify thin pool autoextend policy and monitoring before growth".to_string(),
                    "review thin volume overcommit before adding virtual capacity".to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("loopDevices") => {
            let target = target.unwrap_or("<loop-device>");
            (
                vec![
                    command(
                        ["losetup", "--json", "--list", target],
                        false,
                        "inspect loop device before refreshing backing size",
                    ),
                    command(
                        ["losetup", "-c", target],
                        true,
                        "refresh the loop device size after backing storage growth",
                    ),
                ],
                vec![
                    "grow the backing file or block device before refreshing the loop mapping"
                        .to_string(),
                    "resize dependent filesystems only after losetup reports the new size"
                        .to_string(),
                ],
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
        Operation::Grow if collection == Some("swaps") => {
            let target = target.unwrap_or("<swap>");
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    command(
                        ["swapon", "--show", "--bytes", "--raw"],
                        false,
                        "inspect active swap state before resizing",
                    ),
                    command(
                        ["swapoff", target],
                        true,
                        "disable swap before changing backing storage or signature",
                    ),
                    swap_resize_command(target, desired_size),
                    command(
                        ["mkswap", target],
                        true,
                        "recreate the swap signature after backing storage resize",
                    ),
                    command(
                        ["swapon", target],
                        true,
                        "reactivate swap after verification",
                    ),
                ],
                vec![
                    "verify memory pressure and hibernation dependencies before swapoff"
                        .to_string(),
                    "prefer adding replacement swap capacity before resizing active swap"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("luks.devices") => {
            let mapper = target.unwrap_or("<mapper>");
            let device = action.context.device.as_deref().unwrap_or("<device>");
            (
                vec![
                    command(
                        ["disk-nix", "inspect", device],
                        false,
                        "inspect backing device before resizing the LUKS mapper",
                    ),
                    command(
                        ["cryptsetup", "status", mapper],
                        false,
                        "inspect open LUKS mapper before resize",
                    ),
                    command(
                        ["cryptsetup", "resize", mapper],
                        true,
                        "resize the open LUKS mapping after backing capacity changes",
                    ),
                ],
                vec![
                    "grow the backing partition, LUN, or volume before resizing the mapper"
                        .to_string(),
                    "coordinate dependent LVM and filesystem resizing after cryptsetup resize"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("vdoVolumes") => {
            let target = target.unwrap_or("<vdo-volume>");
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    command(
                        ["vdo", "status", "--name", target],
                        false,
                        "inspect VDO logical and physical size before growth",
                    ),
                    vdo_grow_logical_command(target, desired_size),
                    command(
                        ["vdo", "growPhysical", "--name", target],
                        true,
                        "grow VDO physical capacity after backing storage has grown",
                    ),
                ],
                vec![
                    "choose logical and physical growth intentionally; they are separate VDO operations"
                        .to_string(),
                    "confirm backing storage capacity before physical VDO growth".to_string(),
                    "review deduplication, compression, and slab utilization before increasing logical size"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("zvols") => {
            let target = target.unwrap_or("<zvol>");
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    command(
                        ["zfs", "get", "-H", "-p", "volsize", target],
                        false,
                        "inspect current zvol size before growth",
                    ),
                    zvol_set_volsize_command(target, desired_size),
                ],
                vec![
                    "verify pool free space and reservation policy before increasing volsize"
                        .to_string(),
                    "rescan dependent block consumers after zvol growth".to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("mdRaids") => {
            let target = target.unwrap_or("<md-array>");
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    command(
                        ["mdadm", "--detail", target],
                        false,
                        "inspect MD RAID array health before grow or reshape",
                    ),
                    md_raid_grow_command(target, desired_size),
                    command(
                        ["cat", "/proc/mdstat"],
                        false,
                        "monitor MD RAID reshape, recovery, or resync state",
                    ),
                ],
                vec![
                    "verify backups and redundancy before reshape".to_string(),
                    "do not grow dependent filesystems until mdadm reports the new array size"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("multipathMaps") => {
            let target = target.unwrap_or("<multipath-map>");
            (
                vec![
                    command(
                        ["multipath", "-ll", target],
                        false,
                        "inspect multipath map paths and size before growth",
                    ),
                    command(
                        ["multipathd", "resize", "map", target],
                        true,
                        "resize the multipath map after every backing path sees the new LUN size",
                    ),
                    command(
                        ["multipath", "-r"],
                        true,
                        "reload multipath maps after path rescans",
                    ),
                ],
                vec![
                    "rescan each SCSI path before resizing the multipath map".to_string(),
                    "grow dependent volumes or filesystems only after the map reports the new size"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("partitions") => {
            let target = target.unwrap_or("<partition>");
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect partition, consumers, and backing device before growth",
                    ),
                    partition_grow_command(target, desired_size),
                    command(
                        ["partprobe"],
                        true,
                        "ask the kernel to reread partition tables after the geometry change",
                    ),
                    command_with_readiness(
                        ["blockdev", "--rereadpt", "<disk>"],
                        true,
                        CommandReadiness::NeedsDomainImplementation,
                        ["disk path"],
                        "force a partition table reread when supported by the block device",
                    ),
                ],
                vec![
                    "confirm the backing disk or LUN has already grown".to_string(),
                    "pause dependent consumers when the kernel cannot reread an active table"
                        .to_string(),
                    "resize LUKS, LVM, and filesystems only after the partition reports the new size"
                        .to_string(),
                ],
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
            let snapshot_command = if collection == Some("lvmSnapshots") {
                lvm_snapshot_create_command(target, snapshot, action.context.desired_size.as_deref())
            } else {
                snapshot_command(collection, target, snapshot)
            };
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
        Operation::Create if collection == Some("btrfsSubvolumes") => {
            let target = target.unwrap_or("<btrfs-subvolume-path>");
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect target path and parent Btrfs mount before subvolume creation",
                    ),
                    command(
                        ["btrfs", "subvolume", "create", target],
                        true,
                        "create the Btrfs subvolume at the reviewed path",
                    ),
                ],
                vec![
                    "verify the parent path is on the intended Btrfs filesystem".to_string(),
                    "confirm the target path does not already contain data".to_string(),
                    "review qgroup and mount policy before using the new subvolume".to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("zvols") => {
            let target = target.unwrap_or("<zvol>");
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    command(
                        ["zpool", "list", "-H", "-p"],
                        false,
                        "inspect ZFS pool free space before creating the zvol",
                    ),
                    zvol_create_command(target, desired_size),
                ],
                vec![
                    "decide sparse versus reserved allocation before creation".to_string(),
                    "expose the zvol to guests or LUN exports only after verification".to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("loopDevices") => {
            let target = target.unwrap_or("<loop-device>");
            let backing = action.context.device.as_deref();
            (
                vec![loop_device_create_command(target, backing)],
                vec![
                    "verify the backing file or block device is the intended source".to_string(),
                    "persist the mapping declaratively when it must survive reboot".to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("partitions") => {
            let target = target.unwrap_or("<partition>");
            let disk = action.context.device.as_deref().unwrap_or("<disk>");
            let start = action.context.start.as_deref().unwrap_or("<start>");
            let end = action.context.end.as_deref().unwrap_or("<end>");
            let partition_type = action
                .context
                .partition_type
                .as_deref()
                .unwrap_or("<partition-type>");
            (
                vec![
                    command(
                        ["disk-nix", "inspect", disk],
                        false,
                        "inspect disk identity and existing partition table before creation",
                    ),
                    partition_create_command(disk, partition_type, start, end),
                    command(
                        ["partprobe", disk],
                        true,
                        "ask the kernel to reread the changed partition table",
                    ),
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "verify the new partition node before creating higher layers",
                    ),
                ],
                vec![
                    "verify the selected disk path is stable and matches the intended hardware"
                        .to_string(),
                    "verify the start and end offsets are inside known-free space".to_string(),
                    "format or map the new partition only after it appears by stable identity"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Format if collection == Some("swaps") => {
            let target = target.unwrap_or("<swap>");
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect target before creating a swap signature",
                    ),
                    command(
                        ["swapoff", target],
                        true,
                        "disable active swap before replacing its signature",
                    ),
                    command(
                        ["mkswap", target],
                        true,
                        "create a swap signature on the target",
                    ),
                ],
                vec![
                    "verify the target does not contain data that must be preserved".to_string(),
                    "confirm NixOS swapDevices points at a stable device identity".to_string(),
                ],
                true,
            )
        }
        Operation::Format if collection == Some("luks.devices") => {
            let mapper = target.unwrap_or("<mapper>");
            let device = action.context.device.as_deref().unwrap_or("<device>");
            (
                vec![
                    command(
                        ["disk-nix", "inspect", device],
                        false,
                        "inspect target before creating a LUKS container",
                    ),
                    command(
                        ["cryptsetup", "luksFormat", device],
                        true,
                        "create a LUKS container on the target device",
                    ),
                    command_vec(
                        vec!["cryptsetup", "open", device, mapper],
                        true,
                        "open the newly created LUKS container with the desired mapper name",
                    ),
                ],
                vec![
                    "verify header backups and key enrollment policy before formatting"
                        .to_string(),
                    "create filesystems or LVM layers only after the mapper is open".to_string(),
                ],
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
        Operation::Destroy if collection == Some("btrfsSubvolumes") => {
            let target = target.unwrap_or("<btrfs-subvolume-path>");
            (
                vec![
                    command(
                        ["btrfs", "subvolume", "show", target],
                        false,
                        "inspect Btrfs subvolume metadata before deletion",
                    ),
                    command(
                        ["btrfs", "subvolume", "delete", target],
                        true,
                        "delete the reviewed Btrfs subvolume",
                    ),
                ],
                vec![
                    "take a read-only snapshot before deletion when data may be needed".to_string(),
                    "unmount or redirect consumers before deleting the subvolume".to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("zvols") => {
            let target = target.unwrap_or("<zvol>");
            (
                vec![
                    command(
                        ["zfs", "list", "-H", "-p", "-t", "volume", target],
                        false,
                        "inspect zvol metadata before destruction",
                    ),
                    command(
                        ["zfs", "destroy", target],
                        true,
                        "destroy the reviewed zvol after consumers are detached",
                    ),
                ],
                vec![
                    "take a snapshot or clone before destruction when rollback is required"
                        .to_string(),
                    "detach LUN, VM, or filesystem consumers before destroying the zvol".to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("lvmSnapshots") => {
            let target = target.unwrap_or("<lvm-snapshot>");
            (
                vec![
                    command(
                        ["lvs", "--reportformat", "json", target],
                        false,
                        "inspect LVM snapshot before removal",
                    ),
                    command(
                        ["lvremove", "--yes", target],
                        true,
                        "remove the reviewed LVM snapshot",
                    ),
                ],
                vec![
                    "verify the snapshot is no longer needed as a recovery point".to_string(),
                    "prefer a fresh snapshot or backup before deleting old snapshots".to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("loopDevices") => {
            let target = target.unwrap_or("<loop-device>");
            (
                vec![
                    command(
                        ["losetup", "--json", "--list", target],
                        false,
                        "inspect loop device and backing file before detach",
                    ),
                    command(
                        ["losetup", "--detach", target],
                        true,
                        "detach the loop device without deleting the backing file",
                    ),
                ],
                vec![
                    "unmount filesystems and deactivate mappings before detach".to_string(),
                    "verify the backing file remains available after detach".to_string(),
                ],
                true,
            )
        }
        Operation::Rollback if collection == Some("lvmSnapshots") => {
            let target = target.unwrap_or("<lvm-snapshot>");
            (
                vec![
                    command(
                        ["lvs", "--reportformat", "json", target],
                        false,
                        "inspect LVM snapshot before merge rollback",
                    ),
                    command(
                        ["lvconvert", "--merge", target],
                        true,
                        "merge the LVM snapshot back into its origin",
                    ),
                ],
                vec![
                    "take a fresh snapshot of the origin before merging".to_string(),
                    "schedule downtime when the origin must be deactivated for merge".to_string(),
                ],
                true,
            )
        }
        Operation::RemoveDevice if collection == Some("mdRaids") => {
            let target = target.unwrap_or("<md-array>");
            let device = action
                .context
                .device
                .as_deref()
                .or_else(|| parts.last().copied())
                .unwrap_or("<device>");
            (
                vec![
                    command(
                        ["mdadm", "--detail", target],
                        false,
                        "inspect MD RAID redundancy before member removal",
                    ),
                    command(
                        ["mdadm", target, "--fail", device],
                        true,
                        "mark the MD RAID member failed before removal",
                    ),
                    command(
                        ["mdadm", target, "--remove", device],
                        true,
                        "remove the reviewed MD RAID member",
                    ),
                ],
                vec![
                    "remove a member only when redundancy and free capacity remain sufficient"
                        .to_string(),
                    "monitor /proc/mdstat until recovery or reshape completes".to_string(),
                ],
                true,
            )
        }
        Operation::RemoveDevice if collection == Some("multipathMaps") => {
            let target = target.unwrap_or("<multipath-map>");
            let path = action
                .context
                .device
                .as_deref()
                .or_else(|| parts.last().copied())
                .unwrap_or("<path>");
            (
                vec![
                    command(
                        ["multipath", "-ll", target],
                        false,
                        "inspect live multipath paths before deletion",
                    ),
                    command(
                        ["multipathd", "del", "path", path],
                        true,
                        "delete the reviewed path from multipathd",
                    ),
                ],
                vec![
                    "remove a path only when alternate paths remain active".to_string(),
                    "verify the path belongs to the intended map WWID before deletion".to_string(),
                ],
                true,
            )
        }
        Operation::RemoveDevice if collection == Some("caches") => {
            let target = target.unwrap_or("<cache-device>");
            (
                vec![
                    bcache_sysfs_read_command(
                        target,
                        "dirty_data",
                        "inspect dirty data before bcache detach",
                    ),
                    command_vec(
                        vec![
                            "sh".to_string(),
                            "-c".to_string(),
                            "printf '1\\n' > \"/sys/block/${1#/dev/}/bcache/detach\""
                                .to_string(),
                            "disk-nix-bcache-detach".to_string(),
                            target.to_string(),
                        ],
                        true,
                        "detach the bcache cache set from the backing device after dirty data is flushed",
                    ),
                ],
                vec![
                    "switch writeback caches to writethrough and wait for dirty data to drain before detach"
                        .to_string(),
                    "keep backing storage online and verify it remains readable after detach"
                        .to_string(),
                ],
                true,
            )
        }
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

fn command_vec<I, S>(argv: I, mutates: bool, note: &str) -> ExecutionCommand
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    command_vec_with_readiness(
        argv,
        mutates,
        CommandReadiness::Ready,
        Vec::<&str>::new(),
        note,
    )
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

fn command_vec_with_readiness<I, S, U, T>(
    argv: I,
    mutates: bool,
    readiness: CommandReadiness,
    unresolved_inputs: U,
    note: &str,
) -> ExecutionCommand
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
    U: IntoIterator<Item = T>,
    T: Into<String>,
{
    ExecutionCommand {
        argv: argv.into_iter().map(Into::into).collect(),
        mutates,
        readiness,
        unresolved_inputs: unresolved_inputs.into_iter().map(Into::into).collect(),
        note: note.to_string(),
    }
}

fn filesystem_grow_command(
    fs_type: &str,
    target: &str,
    desired_size: Option<&str>,
) -> ExecutionCommand {
    match fs_type {
        "xfs" => command(
            ["xfs_growfs", target],
            true,
            "grow an already-mounted XFS filesystem",
        ),
        "ext2" | "ext3" | "ext4" => match desired_size {
            Some(size) => command_vec(
                vec!["resize2fs", target, size],
                true,
                "grow an ext filesystem to the desired size after the backing block device has grown",
            ),
            None => command(
                ["resize2fs", target],
                true,
                "grow an ext filesystem after the backing block device has grown",
            ),
        },
        "btrfs" => command_vec(
            vec![
                "btrfs",
                "filesystem",
                "resize",
                desired_size.unwrap_or("max"),
                target,
            ],
            true,
            "grow a Btrfs filesystem to the requested or maximum visible device size",
        ),
        "zfs" => match desired_size {
            Some(size) => command_vec(
                vec![
                    "zfs".to_string(),
                    "set".to_string(),
                    format!("volsize={size}"),
                    target.to_string(),
                ],
                true,
                "set the ZFS volume size to the desired size",
            ),
            None => command_with_readiness(
                ["zfs", "set", "volsize=<size>", target],
                true,
                CommandReadiness::NeedsDesiredSize,
                ["desired zvol size"],
                "set the ZFS volume size after selecting the desired size",
            ),
        },
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
        Some("mdRaids") => command(
            ["mdadm", target, "--add", device],
            true,
            "add a member or spare to an MD RAID array",
        ),
        Some("multipathMaps") => command(
            ["multipathd", "add", "path", device],
            true,
            "add or re-add a path to multipathd",
        ),
        Some("caches") => command_vec(
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/attach\"".to_string(),
                "disk-nix-bcache-attach".to_string(),
                target.to_string(),
                device.to_string(),
            ],
            true,
            "attach an existing bcache cache-set UUID to the backing bcache device",
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
        Some("mdRaids") => command(
            ["mdadm", target, "--replace", from, "--with", to],
            true,
            "replace an MD RAID member while preserving array redundancy",
        ),
        Some("multipathMaps") => command_vec(
            vec![
                "sh".to_string(),
                "-c".to_string(),
                format!(
                    "multipathd add path {} && multipathd del path {}",
                    shell_quote(to),
                    shell_quote(from)
                ),
            ],
            true,
            "add the replacement multipath path before deleting the old path",
        ),
        Some("caches") => command_vec_with_readiness(
            vec![
                "make-bcache".to_string(),
                "-C".to_string(),
                to.to_string(),
                "--writeback".to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            [
                "confirmed empty replacement cache device",
                "new cache-set UUID",
            ],
            &format!(
                "initialize replacement cache device after flushing and detaching {from} from {target}"
            ),
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
        Some("caches") => bcache_property_command(target, property, assignment),
        _ => command_with_readiness(
            ["<set-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["property update tool"],
            "apply the storage-domain property update",
        ),
    }
}

fn bcache_property_command(target: &str, property: &str, assignment: &str) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<cache-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["cache property value"],
            "set a cache property after resolving the desired value",
        );
    };
    command_vec(
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/$3\"".to_string(),
            "disk-nix-bcache-property".to_string(),
            target.to_string(),
            value.to_string(),
            bcache_sysfs_key(property),
        ],
        true,
        "set a bcache sysfs property on the target cache device",
    )
}

fn bcache_sysfs_read_command(target: &str, key: &str, note: &str) -> ExecutionCommand {
    command_vec(
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "cat \"/sys/block/${1#/dev/}/bcache/$2\"".to_string(),
            "disk-nix-bcache-read".to_string(),
            target.to_string(),
            key.to_string(),
        ],
        false,
        note,
    )
}

fn bcache_sysfs_key(property: &str) -> String {
    property
        .strip_prefix("bcache.")
        .unwrap_or(property)
        .replace('-', "_")
}

fn snapshot_command(collection: Option<&str>, target: &str, snapshot: &str) -> ExecutionCommand {
    if snapshot.contains('@') {
        command(["zfs", "snapshot", snapshot], true, "create a ZFS snapshot")
    } else if collection == Some("btrfsSubvolumes") {
        command(
            ["btrfs", "subvolume", "snapshot", target, snapshot],
            true,
            "create a Btrfs subvolume snapshot",
        )
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

fn partition_create_command(
    disk: &str,
    partition_type: &str,
    start: &str,
    end: &str,
) -> ExecutionCommand {
    command_vec_with_readiness(
        vec!["parted", "-s", disk, "mkpart", partition_type, start, end],
        true,
        CommandReadiness::NeedsDomainImplementation,
        ["verified free region", "partition number and flags"],
        "create a partition in the reviewed free region",
    )
}

fn partition_grow_command(target: &str, desired_size: Option<&str>) -> ExecutionCommand {
    match desired_size {
        Some(size) => command_vec_with_readiness(
            vec!["parted", "-s", "<disk>", "resizepart", target, size],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["disk path", "partition number"],
            "grow a partition to the desired end offset or size after backing capacity is visible",
        ),
        None => command_vec_with_readiness(
            vec!["growpart", "<disk>", "<partition-number>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["disk path", "partition number", "desired end offset"],
            "grow a partition after backing capacity is visible",
        ),
    }
}

fn swap_resize_command(target: &str, desired_size: Option<&str>) -> ExecutionCommand {
    match desired_size {
        Some(size) => command_vec_with_readiness(
            vec!["<resize-swap-backing-storage>", target, size],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["backing storage domain"],
            "resize the swap backing device or file before recreating the swap signature",
        ),
        None => command_vec_with_readiness(
            vec!["<resize-swap-backing-storage>", target, "<size>"],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired swap size", "backing storage domain"],
            "resize the swap backing device or file before recreating the swap signature",
        ),
    }
}

fn vdo_grow_logical_command(target: &str, desired_size: Option<&str>) -> ExecutionCommand {
    match desired_size {
        Some(size) => command_vec(
            vec![
                "vdo",
                "growLogical",
                "--name",
                target,
                "--vdoLogicalSize",
                size,
            ],
            true,
            "grow VDO logical size to the desired value",
        ),
        None => command_with_readiness(
            [
                "vdo",
                "growLogical",
                "--name",
                target,
                "--vdoLogicalSize",
                "<size>",
            ],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired VDO logical size"],
            "grow VDO logical size after selecting the desired size",
        ),
    }
}

fn thin_pool_extend_command(target: &str, desired_size: Option<&str>) -> ExecutionCommand {
    match desired_size {
        Some(size) => command_vec(
            vec!["lvextend", "--size", size, target],
            true,
            "extend the LVM thin pool data volume to the desired size",
        ),
        None => command_with_readiness(
            ["lvextend", "--size", "+<size>", target],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired thin pool size or size delta"],
            "extend the LVM thin pool after selecting the desired size",
        ),
    }
}

fn lvm_snapshot_create_command(
    origin: &str,
    snapshot: &str,
    desired_size: Option<&str>,
) -> ExecutionCommand {
    match desired_size {
        Some(size) => command_vec(
            vec![
                "lvcreate",
                "--snapshot",
                "--size",
                size,
                "--name",
                snapshot,
                origin,
            ],
            true,
            "create an LVM snapshot of the origin logical volume",
        ),
        None => command_with_readiness(
            [
                "lvcreate",
                "--snapshot",
                "--size",
                "<size>",
                "--name",
                snapshot,
                origin,
            ],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired LVM snapshot size"],
            "create an LVM snapshot after selecting the snapshot size",
        ),
    }
}

fn loop_device_create_command(target: &str, backing: Option<&str>) -> ExecutionCommand {
    match backing {
        Some(backing) if target.starts_with("/dev/loop") => command(
            ["losetup", target, backing],
            true,
            "create the requested loop-device mapping",
        ),
        Some(backing) => command(
            ["losetup", "--find", "--show", backing],
            true,
            "create a loop-device mapping with the next available loop device",
        ),
        None => command_with_readiness(
            ["losetup", "--find", "--show", "<backing-file>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["backing file or block device"],
            "create a loop-device mapping after selecting the backing path",
        ),
    }
}

fn zvol_create_command(target: &str, desired_size: Option<&str>) -> ExecutionCommand {
    match desired_size {
        Some(size) => command_vec(
            vec!["zfs", "create", "-V", size, target],
            true,
            "create a zvol with the desired volume size",
        ),
        None => command_with_readiness(
            ["zfs", "create", "-V", "<size>", target],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired zvol size"],
            "create a zvol after selecting the desired volume size",
        ),
    }
}

fn zvol_set_volsize_command(target: &str, desired_size: Option<&str>) -> ExecutionCommand {
    match desired_size {
        Some(size) => command_vec(
            vec!["zfs", "set", &format!("volsize={size}"), target],
            true,
            "grow the zvol by setting volsize",
        ),
        None => command_with_readiness(
            ["zfs", "set", "volsize=<size>", target],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired zvol size"],
            "grow the zvol after selecting the desired volume size",
        ),
    }
}

fn md_raid_grow_command(target: &str, desired_size: Option<&str>) -> ExecutionCommand {
    match desired_size {
        Some(size) => command_vec(
            vec!["mdadm", "--grow", target, "--size", size],
            true,
            "grow or reshape the MD RAID array to the desired component size",
        ),
        None => command_with_readiness(
            ["mdadm", "--grow", target, "--size", "<size-or-max>"],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired MD RAID component size or max"],
            "grow or reshape the MD RAID array after selecting the desired size",
        ),
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
        assert_eq!(report.verification_summary.step_count, 1);
        assert!(report.verification_summary.command_count >= 1);
        assert_eq!(report.verification_plan.len(), 1);
        assert!(
            report.verification_plan[0].commands.iter().all(|command| {
                !command.mutates && command.readiness == CommandReadiness::Ready
            })
        );
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
    fn desired_sizes_make_resize_commands_concrete() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "filesystems": {
                  "home": {
                    "mountpoint": "/home",
                    "fsType": "btrfs",
                    "resizePolicy": "grow-only",
                    "desiredSize": "750GiB"
                  }
                },
                "volumes": {
                  "vg/home": {
                    "operation": "grow",
                    "desiredSize": "800GiB"
                  }
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
        assert_eq!(report.command_summary.needs_desired_size_count, 0);
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv == ["btrfs", "filesystem", "resize", "750GiB", "/home"]
                    && command.readiness == CommandReadiness::Ready
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv == ["lvextend", "--resizefs", "--size", "800GiB", "vg/home"]
                    && command.readiness == CommandReadiness::Ready
                    && command.unresolved_inputs.is_empty()
            })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.checks
                .iter()
                .any(|check| check.contains("750GiB") || check.contains("800GiB"))
        }));
    }

    #[test]
    fn shell_script_includes_commands_and_verification() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "filesystems": {
                  "home": {
                    "mountpoint": "/home",
                    "fsType": "btrfs",
                    "resizePolicy": "grow-only",
                    "desiredSize": "750GiB"
                  }
                }
              },
              "apply": {
                "allowGrow": true
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);
        let script = report.to_shell_script().expect("script can render");

        assert!(script.starts_with("#!/usr/bin/env bash"));
        assert!(script.contains("btrfs filesystem resize 750GiB /home"));
        assert!(script.contains("# Post-apply verification commands"));
        assert!(script.contains("disk-nix inspect /home --json"));
    }

    #[test]
    fn shell_script_comments_non_ready_commands() {
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
        let script = report.to_shell_script().expect("script can render");

        assert!(script.contains("# NOT READY: lvextend --resizefs --size '+<size>' vg/root"));
        assert!(script.contains("# Unresolved inputs: desired size delta"));
    }

    #[test]
    fn partition_creation_reports_reviewable_commands_when_offline_allowed() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "partitions": {
                  "root": {
                    "operation": "create",
                    "device": "/dev/disk/by-id/nvme-root",
                    "start": "1MiB",
                    "end": "100%",
                    "partitionType": "linux"
                  }
                }
              },
              "apply": {
                "allowOffline": true
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert_eq!(report.command_plan.len(), 1);
        assert!(report.command_plan[0].commands.iter().any(|command| {
            command.argv
                == [
                    "parted",
                    "-s",
                    "/dev/disk/by-id/nvme-root",
                    "mkpart",
                    "linux",
                    "1MiB",
                    "100%",
                ]
                && command.readiness == CommandReadiness::NeedsDomainImplementation
        }));
        assert!(
            report.verification_plan[0]
                .commands
                .iter()
                .any(|command| command.argv == ["parted", "-lm"])
        );
    }

    #[test]
    fn swap_and_luks_commands_follow_policy_gates() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "swaps": {
                  "primary": {
                    "device": "/dev/disk/by-label/swap",
                    "preserveData": false
                  }
                },
                "luks": {
                  "devices": {
                    "cryptroot": {
                      "name": "cryptroot",
                      "device": "/dev/disk/by-partuuid/root",
                      "operation": "grow"
                    }
                  }
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowFormat": true,
                "allowOffline": true,
                "allowGrow": true
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert_eq!(report.command_plan.len(), 2);
        assert!(report.command_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["mkswap", "/dev/disk/by-label/swap"])
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["cryptsetup", "resize", "cryptroot"])
        }));
    }

    #[test]
    fn vdo_growth_reports_vdo_commands_and_verification() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "vdoVolumes": {
                  "archive": {
                    "operation": "grow",
                    "desiredSize": "4TiB"
                  }
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
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv
                    == [
                        "vdo",
                        "growLogical",
                        "--name",
                        "archive",
                        "--vdoLogicalSize",
                        "4TiB",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["vdostats", "--human-readable", "archive"])
        }));
    }

    #[test]
    fn btrfs_subvolume_lifecycle_reports_domain_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "btrfsSubvolumes": {
                  "/mnt/persist/@home": {
                    "operation": "create",
                    "path": "/mnt/persist/@home"
                  },
                  "/mnt/persist/@old": {
                    "destroy": true,
                    "preserveData": false
                  }
                }
              },
              "apply": {
                "allowDestructive": true
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv == ["btrfs", "subvolume", "create", "/mnt/persist/@home"]
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv == ["btrfs", "subvolume", "delete", "/mnt/persist/@old"]
            })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "btrfssubvolumes:/mnt/persist/@old:destroy"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["disk-nix", "topology", "--json"])
        }));
    }

    #[test]
    fn zvol_lifecycle_reports_zfs_volume_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "zvols": {
                  "tank/vm/root": {
                    "operation": "grow",
                    "desiredSize": "80GiB"
                  },
                  "tank/vm/tmp": {
                    "operation": "create",
                    "desiredSize": "20GiB"
                  },
                  "tank/vm/old": {
                    "destroy": true
                  }
                }
              },
              "apply": {
                "allowGrow": true,
                "allowDestructive": true
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv == ["zfs", "set", "volsize=80GiB", "tank/vm/root"]
                    && command.readiness == CommandReadiness::Ready
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv == ["zfs", "create", "-V", "20GiB", "tank/vm/tmp"]
                    && command.readiness == CommandReadiness::Ready
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["zfs", "destroy", "tank/vm/old"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv == ["zfs", "list", "-H", "-p", "-t", "volume", "tank/vm/root"]
            })
        }));
    }

    #[test]
    fn md_raid_lifecycle_reports_mdadm_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "mdRaids": {
                  "root": {
                    "target": "/dev/md/root",
                    "operation": "grow",
                    "desiredSize": "max",
                    "addDevices": ["/dev/disk/by-id/nvme-spare"],
                    "replaceDevices": {
                      "/dev/disk/by-id/old-md-member": "/dev/disk/by-id/new-md-member"
                    },
                    "removeDevices": ["/dev/disk/by-id/failed-md-member"]
                  }
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

        assert_eq!(report.status, ExecutionStatus::Blocked);
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv
                    == [
                        "mdadm",
                        "/dev/md/root",
                        "--add",
                        "/dev/disk/by-id/nvme-spare",
                    ]
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["mdadm", "--grow", "/dev/md/root", "--size", "max"])
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv
                    == [
                        "mdadm",
                        "/dev/md/root",
                        "--replace",
                        "/dev/disk/by-id/old-md-member",
                        "--with",
                        "/dev/disk/by-id/new-md-member",
                    ]
            })
        }));
        assert!(
            !report.command_plan.iter().any(|step| {
                step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "mdadm",
                            "/dev/md/root",
                            "--remove",
                            "/dev/disk/by-id/failed-md-member",
                        ]
                })
            }),
            "potential-data-loss remove action remains blocked by apply policy"
        );
        assert!(report.verification_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["cat", "/proc/mdstat"])
        }));
    }

    #[test]
    fn multipath_map_lifecycle_reports_multipath_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "multipathMaps": {
                  "mpatha": {
                    "target": "mpatha",
                    "operation": "grow",
                    "addDevices": ["/dev/sdb"],
                    "replaceDevices": {
                      "/dev/sdc": "/dev/sdd"
                    },
                    "removeDevices": ["/dev/sde"]
                  }
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

        assert_eq!(report.status, ExecutionStatus::Blocked);
        assert!(report.command_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["multipathd", "resize", "map", "mpatha"])
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["multipathd", "add", "path", "/dev/sdb"])
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv
                    == [
                        "sh",
                        "-c",
                        "multipathd add path /dev/sdd && multipathd del path /dev/sdc",
                    ]
            })
        }));
        assert!(
            !report.command_plan.iter().any(|step| {
                step.commands
                    .iter()
                    .any(|command| command.argv == ["multipathd", "del", "path", "/dev/sde"])
            }),
            "potential-data-loss path removal remains blocked by apply policy"
        );
        assert!(report.verification_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["multipath", "-ll", "mpatha"])
        }));
    }

    #[test]
    fn thin_pool_growth_reports_lvm_pool_commands_and_verification() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "thinPools": {
                  "vg0/pool": {
                    "operation": "grow",
                    "desiredSize": "500GiB"
                  }
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
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv == ["lvextend", "--size", "500GiB", "vg0/pool"]
                    && command.readiness == CommandReadiness::Ready
            })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv
                    == [
                        "lvs",
                        "--reportformat",
                        "json",
                        "-o",
                        "lv_name,lv_size,data_percent,metadata_percent,seg_monitor",
                        "vg0/pool",
                    ]
            })
        }));
    }

    #[test]
    fn lvm_snapshot_lifecycle_reports_lvm_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "lvmSnapshots": {
                  "vg0/root-snap": {
                    "operation": "snapshot",
                    "target": "vg0/root",
                    "desiredSize": "20GiB"
                  },
                  "vg0/root-rollback": {
                    "operation": "rollback"
                  },
                  "vg0/old-snap": {
                    "destroy": true
                  }
                }
              },
              "apply": {
                "allowDestructive": true
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::Blocked);
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv
                    == [
                        "lvcreate",
                        "--snapshot",
                        "--size",
                        "20GiB",
                        "--name",
                        "vg0/root-snap",
                        "vg0/root",
                    ]
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["lvremove", "--yes", "vg0/old-snap"])
        }));
        assert!(
            !report.command_plan.iter().any(|step| {
                step.commands
                    .iter()
                    .any(|command| command.argv == ["lvconvert", "--merge", "vg0/root-rollback"])
            }),
            "potential-data-loss rollback remains blocked by apply policy"
        );
        assert!(report.verification_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["lvs", "--reportformat", "json", "vg0/root-snap"])
        }));
    }

    #[test]
    fn loop_device_lifecycle_reports_losetup_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "loopDevices": {
                  "/dev/loop7": {
                    "operation": "create",
                    "device": "/var/lib/images/root.img"
                  },
                  "/dev/loop8": {
                    "operation": "grow"
                  },
                  "/dev/loop9": {
                    "operation": "destroy"
                  }
                }
              },
              "apply": {
                "allowGrow": true
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::Blocked);
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv == ["losetup", "/dev/loop7", "/var/lib/images/root.img"]
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["losetup", "-c", "/dev/loop8"])
        }));
        assert!(
            !report.command_plan.iter().any(|step| {
                step.commands
                    .iter()
                    .any(|command| command.argv == ["losetup", "--detach", "/dev/loop9"])
            }),
            "offline detach remains blocked by default policy"
        );
        assert!(report.verification_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["losetup", "--json", "--list", "/dev/loop8"])
        }));
    }

    #[test]
    fn blocked_reports_do_not_render_scripts() {
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

        assert!(report.to_shell_script().is_none());
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
        assert_eq!(report.verification_summary.step_count, 0);
        assert!(report.verification_plan.is_empty());
    }

    #[test]
    fn filesystem_growth_reports_read_only_verification_steps() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "filesystems": {
                "root": {
                  "mountpoint": "/",
                  "fsType": "xfs",
                  "resizePolicy": "grow-only"
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
        assert_eq!(report.verification_plan.len(), 1);
        assert!(report.verification_plan[0].commands.iter().any(|command| {
            command.argv == ["findmnt", "--json", "--bytes", "/"] && !command.mutates
        }));
        assert!(
            report.verification_plan[0]
                .checks
                .iter()
                .any(|check| check.contains("filesystem size"))
        );
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
        assert!(report.verification_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["zpool", "status", "-P", "tank"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv
                    == [
                        "zfs",
                        "list",
                        "-t",
                        "snapshot",
                        "-H",
                        "-p",
                        "tank/home@before",
                    ]
            })
        }));
        assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
        assert!(report.command_summary.all_commands_ready());
    }

    #[test]
    fn cache_lifecycle_reports_bcache_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "caches": {
                  "/dev/bcache0": {
                    "addDevices": ["cache-set-uuid"],
                    "replaceDevices": {
                      "/dev/disk/by-id/old-cache": "/dev/disk/by-id/new-cache"
                    },
                    "properties": {
                      "bcache.cache-mode": "writethrough"
                    }
                  }
                }
              },
              "apply": {
                "allowDeviceReplacement": true,
                "allowOffline": true,
                "allowPropertyChanges": true
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv
                    == [
                        "sh",
                        "-c",
                        "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/attach\"",
                        "disk-nix-bcache-attach",
                        "/dev/bcache0",
                        "cache-set-uuid",
                    ]
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv
                    == [
                        "sh",
                        "-c",
                        "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/$3\"",
                        "disk-nix-bcache-property",
                        "/dev/bcache0",
                        "writethrough",
                        "cache_mode",
                    ]
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv
                    == [
                        "make-bcache",
                        "-C",
                        "/dev/disk/by-id/new-cache",
                        "--writeback",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
            })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv
                    == [
                        "sh",
                        "-c",
                        "cat \"/sys/block/${1#/dev/}/bcache/$2\"",
                        "disk-nix-bcache-read",
                        "/dev/bcache0",
                        "dirty_data",
                    ]
            })
        }));
    }
}
