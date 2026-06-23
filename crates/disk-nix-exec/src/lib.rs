use std::{collections::BTreeSet, process::Command};

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
    NotReady,
    Succeeded,
    Failed,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub execution_results: Vec<ExecutionCommandResult>,
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
pub enum ExecutionPhase {
    Command,
    Verification,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionCommandResult {
    pub phase: ExecutionPhase,
    pub action_id: String,
    pub argv: Vec<String>,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommandRunResult {
    success: bool,
    status_code: Option<i32>,
    stdout: String,
    stderr: String,
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
    prepare_execution_with_runner(plan, policy, mode, run_command)
}

fn prepare_execution_with_runner(
    plan: &Plan,
    policy: ApplyPolicy,
    mode: ExecutionMode,
    mut runner: impl FnMut(&[String]) -> CommandRunResult,
) -> ExecutionReport {
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
            execution_results: Vec::new(),
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
            execution_results: Vec::new(),
        },
        ExecutionMode::Execute => {
            if !command_summary.all_commands_ready() {
                return ExecutionReport {
                    apply,
                    status: ExecutionStatus::NotReady,
                    topology_comparison,
                    command_summary,
                    command_plan,
                    verification_summary,
                    verification_plan,
                    execution_results: Vec::new(),
                    messages: vec![
                        "execute refused: every planned command must be ready before mutating storage"
                            .to_string(),
                    ],
                };
            }

            let (status, execution_results) = execute_command_and_verification_plan(
                &command_plan,
                &verification_plan,
                &mut runner,
            );
            let messages = match status {
                ExecutionStatus::Succeeded => vec![format!(
                    "execute completed: ran {} planned command(s) and verification command(s)",
                    execution_results.len()
                )],
                ExecutionStatus::Failed => vec![format!(
                    "execute failed: stopped after {} command result(s)",
                    execution_results.len()
                )],
                _ => Vec::new(),
            };

            ExecutionReport {
                apply,
                status,
                topology_comparison,
                command_summary,
                command_plan,
                verification_summary,
                verification_plan,
                execution_results,
                messages,
            }
        }
    }
}

fn run_command(argv: &[String]) -> CommandRunResult {
    let Some((program, args)) = argv.split_first() else {
        return CommandRunResult {
            success: false,
            status_code: None,
            stdout: String::new(),
            stderr: "empty command argv".to_string(),
        };
    };

    match Command::new(program).args(args).output() {
        Ok(output) => CommandRunResult {
            success: output.status.success(),
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        },
        Err(error) => CommandRunResult {
            success: false,
            status_code: None,
            stdout: String::new(),
            stderr: error.to_string(),
        },
    }
}

fn execute_command_and_verification_plan(
    command_plan: &[ExecutionStep],
    verification_plan: &[VerificationStep],
    runner: &mut impl FnMut(&[String]) -> CommandRunResult,
) -> (ExecutionStatus, Vec<ExecutionCommandResult>) {
    let mut results = Vec::new();

    for step in command_plan {
        for command in &step.commands {
            let result = run_planned_command(
                ExecutionPhase::Command,
                &step.action_id,
                &command.argv,
                runner,
            );
            let success = result.success;
            results.push(result);
            if !success {
                return (ExecutionStatus::Failed, results);
            }
        }
    }

    for step in verification_plan {
        for command in &step.commands {
            let result = run_planned_command(
                ExecutionPhase::Verification,
                &step.action_id,
                &command.argv,
                runner,
            );
            let success = result.success;
            results.push(result);
            if !success {
                return (ExecutionStatus::Failed, results);
            }
        }
    }

    (ExecutionStatus::Succeeded, results)
}

fn run_planned_command(
    phase: ExecutionPhase,
    action_id: &str,
    argv: &[String],
    runner: &mut impl FnMut(&[String]) -> CommandRunResult,
) -> ExecutionCommandResult {
    let result = runner(argv);
    ExecutionCommandResult {
        phase,
        action_id: action_id.to_string(),
        argv: argv.to_vec(),
        success: result.success,
        status_code: result.status_code,
        stdout: result.stdout,
        stderr: result.stderr,
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
        Operation::Shrink
            if collection == Some("filesystems") || action.id.starts_with("filesystem:") =>
        {
            let mut commands = vec![command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify filesystem graph state after the reviewed shrink",
            )];
            if fs_type == Some("btrfs") {
                commands.push(command(
                    ["btrfs", "filesystem", "usage", "-b", target],
                    false,
                    "verify Btrfs allocation and free space after shrink",
                ));
            }
            (
                commands,
                vec![
                    desired_size
                        .map(|size| format!("filesystem size reports no more than {size}"))
                        .unwrap_or_else(|| "filesystem size matches the reviewed shrink target".to_string()),
                    "used data, metadata, and free-space counters remain internally consistent after re-probe"
                        .to_string(),
                    "mounts and dependent services are restored only after filesystem checks pass"
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
        Operation::Rescan if collection == Some("volumes") || action.id.starts_with("volumes:") => (
            vec![
                command(
                    ["lvs", "--reportformat", "json", target],
                    false,
                    "verify LVM logical volume attributes after status refresh",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled LV graph relationships after status refresh",
                ),
            ],
            vec![
                "logical volume size, attributes, and activation state are reviewed".to_string(),
                "dependent filesystems, mappings, or mounts still resolve the LV".to_string(),
            ],
        ),
        Operation::Create if collection == Some("volumes") => (
            vec![
                command(
                    ["lvs", "--reportformat", "json", target],
                    false,
                    "verify LVM logical volume exists after creation",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled LV graph relationships after creation",
                ),
            ],
            vec![
                "logical volume path exists by stable mapper or /dev/<vg>/<lv> name".to_string(),
                "LV size and VG free space match the desired allocation".to_string(),
            ],
        ),
        Operation::Activate | Operation::Deactivate
            if collection == Some("volumes")
                || collection == Some("thinPools")
                || collection == Some("lvmSnapshots") =>
        {
            (
                vec![
                    command(
                        ["lvs", "--reportformat", "json", target],
                        false,
                        "verify LVM logical volume activation state",
                    ),
                    command(
                        ["disk-nix", "inspect", target, "--json"],
                        false,
                        "verify modeled LV graph relationships after activation change",
                    ),
                ],
                vec![
                    "logical volume activation state matches the declared lifecycle operation"
                        .to_string(),
                    "dependent filesystems, mappings, mounts, and services are reviewed after activation state change"
                        .to_string(),
                ],
            )
        }
        Operation::Rescan if collection == Some("lvmSnapshots") => (
            vec![
                command(
                    ["lvs", "--reportformat", "json", target],
                    false,
                    "verify LVM snapshot origin, attributes, and COW usage after rescan",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled LVM snapshot graph relationships after rescan",
                ),
            ],
            vec![
                "snapshot origin, activation state, and COW usage match the refreshed topology"
                    .to_string(),
                "dependent filesystems or recovery mounts still resolve after snapshot status refresh"
                    .to_string(),
            ],
        ),
        Operation::Create if collection == Some("volumeGroups") => (
            vec![
                command(
                    ["vgs", "--reportformat", "json", target],
                    false,
                    "verify LVM volume group exists after creation",
                ),
                command(
                    ["pvs", "--reportformat", "json"],
                    false,
                    "verify physical volume membership after volume group creation",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled VG graph relationships after creation",
                ),
            ],
            vec![
                "volume group appears with the expected physical volume members".to_string(),
                "VG free extents and metadata state are reviewed before creating LVs".to_string(),
            ],
        ),
        Operation::Grow if collection == Some("volumeGroups") => (
            vec![
                command(
                    ["vgs", "--reportformat", "json", target],
                    false,
                    "verify LVM volume group size and free extents after extension",
                ),
                command(
                    ["pvs", "--reportformat", "json"],
                    false,
                    "verify physical volume membership after volume group growth",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled VG graph relationships after growth",
                ),
            ],
            vec![
                "volume group includes the expected new physical volume members".to_string(),
                "VG free extents reflect the added capacity before downstream LV growth"
                    .to_string(),
            ],
        ),
        Operation::Rescan if collection == Some("physicalVolumes") => (
            vec![
                command(
                    ["pvs", "--reportformat", "json"],
                    false,
                    "verify LVM physical volume inventory after metadata rescan",
                ),
                command(
                    ["vgs", "--reportformat", "json"],
                    false,
                    "verify volume group metadata after PV cache refresh",
                ),
                command(
                    ["disk-nix", "topology", "--json"],
                    false,
                    "re-probe topology after LVM physical volume rescan",
                ),
            ],
            vec![
                "PV metadata, size, and VG membership reflect current block-device state"
                    .to_string(),
                "dependent VGs no longer report stale or missing physical volumes".to_string(),
            ],
        ),
        Operation::Rescan if collection == Some("volumeGroups") => (
            vec![
                command(
                    ["vgs", "--reportformat", "json", target],
                    false,
                    "verify LVM volume group metadata after rescan",
                ),
                command(
                    ["pvs", "--reportformat", "json"],
                    false,
                    "verify physical volume membership after VG metadata refresh",
                ),
                command(
                    ["lvs", "--reportformat", "json", target],
                    false,
                    "verify contained logical volumes after VG metadata refresh",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled VG graph relationships after metadata rescan",
                ),
            ],
            vec![
                "volume group metadata and free extents match refreshed PV state".to_string(),
                "logical volumes remain active only where expected after refresh".to_string(),
            ],
        ),
        Operation::Activate | Operation::Deactivate if collection == Some("volumeGroups") => (
            vec![
                command(
                    ["vgs", "--reportformat", "json", target],
                    false,
                    "verify LVM volume group activation state",
                ),
                command(
                    ["lvs", "--reportformat", "json", target],
                    false,
                    "verify contained logical volume activation state",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled VG graph relationships after activation change",
                ),
            ],
            vec![
                "volume group activation state matches the declared lifecycle operation"
                    .to_string(),
                "contained logical volumes and dependent consumers are reviewed after activation state change"
                    .to_string(),
            ],
        ),
        Operation::Create if collection == Some("datasets") => (
            vec![
                command(
                    ["zfs", "list", "-H", "-p", "-t", "filesystem", target],
                    false,
                    "verify ZFS dataset exists after creation",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled dataset graph relationships after creation",
                ),
            ],
            vec![
                "dataset appears with expected inherited and explicit properties".to_string(),
                "mountpoint, quota, reservation, and encryption policy are reviewed".to_string(),
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
        Operation::Rescan if collection == Some("thinPools") => (
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
                    "verify thin pool data, metadata, and monitoring state after refresh",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify thin pool graph node and dependent thin volumes after refresh",
                ),
            ],
            vec![
                "thin pool data and metadata utilization are reviewed before further allocation"
                    .to_string(),
                "monitoring and autoextend state match the intended safety policy".to_string(),
            ],
        ),
        Operation::Create if collection == Some("thinPools") => (
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
                    "verify thin pool exists after creation",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify thin pool graph node and volume group relationship after creation",
                ),
            ],
            vec![
                "thin pool reports expected size and monitored state".to_string(),
                "data and metadata utilization are reviewed before thin volumes are created"
                    .to_string(),
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
        Operation::Rescan if collection == Some("swaps") => {
            let target = swap_target_path(action);
            (
                vec![
                    command(
                        ["swapon", "--show", "--bytes", "--raw"],
                        false,
                        "verify active swap inventory after refresh",
                    ),
                    swap_blkid_command(target, "verify swap signature label and UUID after refresh"),
                    swap_inspect_json_command(target, "verify swap graph node and backing storage after refresh"),
                ],
                vec![
                    "swap activation state, size, label, and UUID are reviewed".to_string(),
                    "resume, hibernation, and NixOS swapDevices references still match the refreshed identity"
                        .to_string(),
                ],
            )
        }
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
        Operation::Create if collection == Some("vdoVolumes") => (
            vec![
                command(
                    ["vdo", "status", "--name", target],
                    false,
                    "verify VDO volume configuration after creation",
                ),
                command(
                    ["vdostats", "--human-readable", target],
                    false,
                    "verify VDO runtime capacity and savings counters after creation",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify VDO graph node and backing relationships after creation",
                ),
            ],
            vec![
                "VDO device exists with the intended logical size and backing device".to_string(),
                "deduplication, compression, and write policy are reviewed before use".to_string(),
            ],
        ),
        Operation::Start if collection == Some("vdoVolumes") => (
            vec![
                command(
                    ["vdo", "status", "--name", target],
                    false,
                    "verify VDO volume configuration after start",
                ),
                command(
                    ["vdostats", "--human-readable", target],
                    false,
                    "verify VDO runtime counters after start",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify VDO graph node and active consumers after start",
                ),
            ],
            vec![
                "VDO volume is started and reports healthy runtime counters".to_string(),
                "dependent filesystems or mappings see the VDO device before use".to_string(),
            ],
        ),
        Operation::Rescan if collection == Some("vdoVolumes") => (
            vec![
                command(
                    ["vdo", "status", "--name", target],
                    false,
                    "verify VDO volume configuration after status refresh",
                ),
                command(
                    ["vdostats", "--human-readable", target],
                    false,
                    "verify VDO runtime counters after status refresh",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify VDO graph node and backing relationships after status refresh",
                ),
            ],
            vec![
                "VDO volume status and operating mode match expected state".to_string(),
                "utilization, available space, and space-saving counters are reviewed".to_string(),
                "dependent filesystems or mappings still resolve the VDO device".to_string(),
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
        Operation::Rescan if collection == Some("loopDevices") => (
            vec![
                command(
                    ["losetup", "--json", "--list", target],
                    false,
                    "verify loop device mapping inventory after rescan",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify loop graph node and dependent consumers after rescan",
                ),
            ],
            vec![
                "loop device backing file, offset, sizelimit, and autoclear state are reviewed"
                    .to_string(),
                "dependent mappings or filesystems still resolve the loop device".to_string(),
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
        Operation::Create if collection == Some("disks") => (
            vec![
                command(
                    ["parted", "-lm", target],
                    false,
                    "verify disk partition table label after initialization",
                ),
                command(
                    ["lsblk", "--json", "--bytes", "--output-all", target],
                    false,
                    "verify kernel disk and partition-table state after reread",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled disk graph node after initialization",
                ),
            ],
            vec![
                "disk reports the requested partition table label".to_string(),
                "no unexpected partitions or consumers remain after initialization".to_string(),
            ],
        ),
        Operation::Rescan if collection == Some("disks") || collection == Some("partitions") => {
            let disk = partition_rescan_disk(action);
            (
                vec![
                    disk_parted_machine_list_command(
                        disk,
                        "verify partition table after kernel reread",
                    ),
                    command(
                        ["disk-nix", "topology", "--json"],
                        false,
                        "verify disk and partition graph after partition-table rescan",
                    ),
                ],
                vec![
                    "kernel partition inventory matches the reviewed table".to_string(),
                    "dependent filesystems, mappings, and mounts still resolve stable paths"
                        .to_string(),
                ],
            )
        }
        Operation::Grow | Operation::Rescan
            if collection == Some("luns")
                || collection == Some("iscsiSessions")
                || action.id.starts_with("luns:")
                || action.id.starts_with("iscsiSessions:") =>
        {
            let is_rescan = action.operation == Operation::Rescan;
            let mut commands = vec![command(
                ["lsblk", "--json", "--bytes", "--output-all"],
                false,
                if is_rescan {
                    "verify kernel block-device inventory after host rescan"
                } else {
                    "verify kernel block-device capacity after host rescan"
                },
            )];
            for device in lun_rescan_devices(action) {
                commands.push(command_vec(
                    vec!["blockdev", "--getsize64", device.as_str()],
                    false,
                    if is_rescan {
                        "verify the reviewed LUN path is visible after rescan"
                    } else {
                        "verify the reviewed LUN path reports its post-rescan byte size"
                    },
                ));
            }
            commands.push(command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify LUN, iSCSI session, multipath, and consumers in the graph",
            ));
            (
                commands,
                vec![
                    if is_rescan {
                        "every expected path remains visible after rescan".to_string()
                    } else {
                        desired_size
                            .map(|size| format!("every expected path reports capacity {size}"))
                            .unwrap_or_else(|| {
                                "every expected path reports the new capacity".to_string()
                            })
                    },
                    if is_rescan {
                        "multipath maps and dependent volumes no longer report stale paths"
                            .to_string()
                    } else {
                        "multipath maps and dependent volumes no longer report stale sizes"
                            .to_string()
                    },
                    "no consumer remains on a missing or failed path".to_string(),
                ],
            )
        }
        Operation::Create | Operation::Attach
            if collection == Some("luns") || action.id.starts_with("luns:") =>
        {
            let mut commands = vec![command(
                ["lsblk", "--json", "--bytes", "--output-all"],
                false,
                "verify kernel block-device inventory after LUN attach",
            )];
            for device in lun_rescan_devices(action) {
                commands.push(command_vec(
                    vec!["blockdev", "--getsize64", device.as_str()],
                    false,
                    "verify the reviewed LUN path exists and reports capacity",
                ));
            }
            commands.push(command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify attached LUN, iSCSI session, multipath, and consumers in the graph",
            ));
            (
                commands,
                vec![
                    "every expected LUN path is visible by a stable device name".to_string(),
                    "multipath maps and consumers are created only after path identity is verified"
                        .to_string(),
                ],
            )
        }
        Operation::Destroy | Operation::Detach
            if collection == Some("luns") || action.id.starts_with("luns:") =>
        {
            let mut commands = vec![command(
                ["lsblk", "--json", "--bytes", "--output-all"],
                false,
                "verify kernel block-device inventory after LUN detach",
            )];
            for device in lun_rescan_devices(action) {
                commands.push(command_vec(
                    vec!["test", "!", "-e", device.as_str()],
                    false,
                    "verify the reviewed LUN path is no longer present",
                ));
            }
            commands.push(command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify detached LUN paths and remaining consumers in the graph",
            ));
            (
                commands,
                vec![
                    "detached LUN paths no longer appear in kernel block inventory".to_string(),
                    "remaining multipath maps, filesystems, and services have no stale dependencies"
                        .to_string(),
                ],
            )
        }
        Operation::Create | Operation::Destroy | Operation::Login | Operation::Logout
            if collection == Some("iscsiSessions") =>
        (
            vec![
                command(
                    ["iscsiadm", "--mode", "session"],
                    false,
                    "list active iSCSI sessions after login or logout",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify iSCSI session, LUN, multipath, and consumer graph state",
                ),
            ],
            vec![
                "session login state matches the declared lifecycle operation".to_string(),
                "dependent LUN paths and multipath maps are present only when expected".to_string(),
            ],
        ),
        Operation::Create | Operation::Mount if collection == Some("nfs.mounts") => {
            let mountpoint = nfs_mount_target_path(action);
            (
                vec![
                    nfs_mount_findmnt_command(mountpoint),
                    command(
                        ["disk-nix", "inspect", mountpoint.unwrap_or(target), "--json"],
                        false,
                        "verify NFS mount graph state after mount",
                    ),
                ],
                vec![
                    "findmnt reports the mountpoint with the reviewed NFS source and options"
                        .to_string(),
                    "local services see the expected mounted NFS source".to_string(),
                ],
            )
        }
        Operation::Remount if collection == Some("nfs.mounts") => {
            let mountpoint = nfs_mount_target_path(action);
            (
                vec![
                    nfs_mount_findmnt_command(mountpoint),
                    command(
                        ["disk-nix", "inspect", mountpoint.unwrap_or(target), "--json"],
                        false,
                        "verify NFS mount graph state after remount",
                    ),
                ],
                vec![
                    "findmnt reports the mountpoint with the reviewed NFS options".to_string(),
                    "local services continue to see the expected mount source and filesystem type"
                        .to_string(),
                ],
            )
        }
        Operation::Rescan if collection == Some("nfs.mounts") => {
            let mountpoint = nfs_mount_target_path(action);
            let inspect_target = mountpoint.unwrap_or("<mountpoint>");
            let inspect_command = match mountpoint {
                Some(mountpoint) => command(
                    ["disk-nix", "inspect", mountpoint, "--json"],
                    false,
                    "verify modeled NFS mount graph state after rescan",
                ),
                None => command_with_readiness(
                    ["disk-nix", "inspect", inspect_target, "--json"],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["mountpoint path"],
                    "verify modeled NFS mount graph state after selecting the mountpoint",
                ),
            };
            (
                vec![
                    nfs_mount_findmnt_command(mountpoint),
                    nfs_mount_stats_command(mountpoint),
                    inspect_command,
                ],
                vec![
                    "findmnt reports the reviewed NFS source and mount options".to_string(),
                    "NFS client statistics are reviewed before remount or unmount work"
                        .to_string(),
                ],
            )
        }
        Operation::Destroy | Operation::Unmount if collection == Some("nfs.mounts") => {
            let mountpoint = nfs_mount_target_path(action);
            (
                vec![
                    command(
                        ["findmnt", "--json", mountpoint.unwrap_or("<mountpoint>")],
                        false,
                        "verify NFS mountpoint is no longer mounted",
                    ),
                    command(
                        ["disk-nix", "topology", "--json"],
                        false,
                        "re-probe topology after NFS client unmount",
                    ),
                ],
                vec![
                    "findmnt no longer reports the NFS mountpoint as mounted".to_string(),
                    "local filesystems and services no longer depend on the unmounted path"
                        .to_string(),
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
        Operation::Create if collection == Some("pools") => (
            vec![
                command(
                    ["zpool", "status", "-P", target],
                    false,
                    "verify ZFS pool health and vdev topology after creation",
                ),
                command(
                    ["zpool", "list", "-H", "-p", target],
                    false,
                    "verify ZFS pool size, allocation, and free capacity after creation",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled pool graph relationships after creation",
                ),
            ],
            vec![
                "pool exists with expected vdev devices and health state".to_string(),
                "pool free space and allocation policy are reviewed before creating datasets"
                    .to_string(),
            ],
        ),
        Operation::AddDevice
        | Operation::ReplaceDevice
        | Operation::RemoveDevice
        | Operation::Rebalance
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
        | Operation::Create
        | Operation::Assemble
        | Operation::Stop
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
        Operation::Rescan if collection == Some("mdRaids") => {
            let target = md_array_target_path(action);
            let mut commands = Vec::new();
            if let Some(target) = target {
                commands.push(command(
                    ["mdadm", "--detail", target],
                    false,
                    "verify targeted MD RAID array state after metadata rescan",
                ));
            }
            commands.extend([
                command(
                    ["mdadm", "--detail", "--scan"],
                    false,
                    "verify assembled MD RAID array inventory after metadata rescan",
                ),
                command(
                    ["mdadm", "--examine", "--scan"],
                    false,
                    "verify member metadata inventory after MD RAID rescan",
                ),
                command(
                    ["cat", "/proc/mdstat"],
                    false,
                    "verify MD RAID kernel status after metadata rescan",
                ),
                command(
                    ["disk-nix", "topology", "--json"],
                    false,
                    "re-probe topology after MD RAID metadata rescan",
                ),
            ]);
            (
                commands,
                vec![
                    "array metadata inventory matches the reviewed member devices".to_string(),
                    "no unexpected arrays are assembled or missing after metadata refresh"
                        .to_string(),
                    "dependent filesystems and mappings still reference expected MD devices"
                        .to_string(),
                ],
            )
        }
        Operation::AddDevice
        | Operation::ReplaceDevice
        | Operation::RemoveDevice
        | Operation::Grow
        | Operation::Rescan
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
                    if action.operation == Operation::Rescan {
                        "map WWID and path state still match the reviewed topology".to_string()
                    } else {
                        "map size and WWID match desired state".to_string()
                    },
                    "dependent filesystems or mappings see the expected capacity".to_string(),
                ],
            )
        }
        Operation::Create | Operation::Grow | Operation::Destroy
            if collection == Some("nvmeNamespaces") =>
        {
            (
                vec![
                    command(
                        ["nvme", "list", "--output-format=json"],
                        false,
                        "verify NVMe controller and namespace inventory",
                    ),
                    command(
                        ["disk-nix", "inspect", target, "--json"],
                        false,
                        "verify NVMe namespace graph relationships after lifecycle change",
                    ),
                ],
                vec![
                    "namespace id, attachment state, and capacity match the desired lifecycle outcome"
                        .to_string(),
                    "dependent partitions, volumes, or filesystems see the expected namespace state"
                        .to_string(),
                ],
            )
        }
        Operation::Create | Operation::Grow | Operation::Destroy
            if collection == Some("physicalVolumes") =>
        {
            (
                vec![
                    command(
                        ["pvs", "--reportformat", "json"],
                        false,
                        "verify LVM physical volume inventory after lifecycle change",
                    ),
                    command(
                        ["disk-nix", "inspect", target, "--json"],
                        false,
                        "verify physical volume graph relationships after lifecycle change",
                    ),
                ],
                vec![
                    "PV metadata, size, and VG membership match the desired state".to_string(),
                    "dependent volume groups report expected free extents".to_string(),
                ],
            )
        }
        Operation::Create
        | Operation::AddKey
        | Operation::SetProperty
        | Operation::Destroy
        | Operation::RemoveKey
            if collection == Some("luksKeyslots") =>
        {
            let device = luks_keyslot_device(action);
            (
                vec![
                    luks_dump_command(device, "verify LUKS header and keyslot metadata"),
                    command(
                        ["disk-nix", "inspect", device.unwrap_or("<luks-device>"), "--json"],
                        false,
                        "verify modeled LUKS container relationships after keyslot change",
                    ),
                ],
                vec![
                    "at least one reviewed keyslot or token remains usable after the change"
                        .to_string(),
                    "LUKS header backup and keyslot inventory match the desired access policy"
                        .to_string(),
                ],
            )
        }
        Operation::Create
        | Operation::ImportToken
        | Operation::SetProperty
        | Operation::Destroy
        | Operation::RemoveToken
            if collection == Some("luksTokens") =>
        {
            let device = luks_token_device(action);
            (
                vec![
                    luks_dump_command(device, "verify LUKS header and token metadata"),
                    command(
                        ["disk-nix", "inspect", device.unwrap_or("<luks-device>"), "--json"],
                        false,
                        "verify modeled LUKS container relationships after token change",
                    ),
                ],
                vec![
                    "at least one reviewed keyslot or token remains usable after the change"
                        .to_string(),
                    "LUKS header backup and token inventory match the desired access policy"
                        .to_string(),
                ],
            )
        }
        Operation::Create
        | Operation::AddDevice
        | Operation::ReplaceDevice
        | Operation::RemoveDevice
        | Operation::SetProperty
        | Operation::Destroy
        | Operation::Rescan
            if collection == Some("lvmCaches") =>
        {
            let target = lvm_volume_target_path(Some(target));
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        Some("lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent"),
                        "verify LVM cache state after lifecycle change",
                    ),
                    command(
                        ["disk-nix", "inspect", target.unwrap_or("<lvm-cache>"), "--json"],
                        false,
                        "verify modeled LVM cache relationships after cache update",
                    ),
                ],
                vec![
                    "origin LV, cache pool, cache mode, and dirty data state match the desired cache lifecycle"
                        .to_string(),
                    "origin LV remains readable after cache attach, detach, or mode update".to_string(),
                ],
            )
        }
        Operation::AddDevice
        | Operation::ReplaceDevice
        | Operation::RemoveDevice
        | Operation::SetProperty
        | Operation::Rescan
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
        Operation::SetProperty if collection == Some("zvols") => (
            vec![
                command(
                    ["zfs", "get", "all", target],
                    false,
                    "verify zvol properties after update",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled zvol properties after re-probe",
                ),
            ],
            vec!["changed property equals the desired value".to_string()],
        ),
        Operation::Rescan if collection == Some("datasets") => (
            vec![
                command(
                    ["zfs", "list", "-H", "-p", "-t", "filesystem", target],
                    false,
                    "verify ZFS dataset inventory after rescan",
                ),
                command(
                    [
                        "zfs",
                        "get",
                        "-H",
                        "-p",
                        "-o",
                        "property,value,source",
                        "all",
                        target,
                    ],
                    false,
                    "verify ZFS dataset properties after rescan",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled ZFS dataset graph state after rescan",
                ),
            ],
            vec![
                "dataset properties, mountpoint, and inherited policy match refreshed inventory"
                    .to_string(),
                "snapshot, clone, mount, and export relationships are reviewed".to_string(),
            ],
        ),
        Operation::Rescan if collection == Some("zvols") => (
            vec![
                command(
                    ["zfs", "list", "-H", "-p", "-t", "volume", target],
                    false,
                    "verify zvol inventory after rescan",
                ),
                command(
                    [
                        "zfs",
                        "get",
                        "-H",
                        "-p",
                        "-o",
                        "property,value,source",
                        "all",
                        target,
                    ],
                    false,
                    "verify zvol properties after rescan",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled zvol block graph state after rescan",
                ),
            ],
            vec![
                "zvol volsize, reservation, and property state match refreshed inventory"
                    .to_string(),
                "dependent LUN, guest, partition, and filesystem consumers are reviewed"
                    .to_string(),
            ],
        ),
        Operation::SetProperty if collection == Some("vdoVolumes") => (
            vec![
                command(
                    ["vdo", "status", "--name", target],
                    false,
                    "verify VDO configuration after property update",
                ),
                command(
                    ["vdostats", "--verbose", target],
                    false,
                    "verify VDO runtime mode and policy after property update",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled VDO properties after re-probe",
                ),
            ],
            vec!["changed VDO property equals the desired value".to_string()],
        ),
        Operation::SetProperty if collection == Some("luks.devices") => {
            let device = action.context.device.as_deref();
            (
                vec![
                    luks_dump_command(device, "verify LUKS header metadata after property update"),
                    command(
                        ["disk-nix", "inspect", device.unwrap_or("<luks-device>"), "--json"],
                        false,
                        "verify modeled LUKS header properties after re-probe",
                    ),
                ],
                vec![
                    "changed LUKS header property equals the desired value".to_string(),
                    "initrd, crypttab, and dependent mappings still reference the intended encrypted container"
                        .to_string(),
                ],
            )
        }
        Operation::SetProperty if collection == Some("btrfsQgroups") => (
            vec![
                command(
                    ["btrfs", "qgroup", "show", "--raw", "-reF", target],
                    false,
                    "verify Btrfs qgroup limits and usage after update",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled Btrfs qgroup properties after re-probe",
                ),
            ],
            vec!["changed qgroup limit equals the desired value".to_string()],
        ),
        Operation::Create | Operation::Destroy if collection == Some("btrfsQgroups") => (
            vec![
                command(
                    ["btrfs", "qgroup", "show", "--raw", "-reF", target],
                    false,
                    "verify Btrfs qgroup inventory after lifecycle change",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled Btrfs qgroup topology after re-probe",
                ),
            ],
            vec!["Btrfs qgroup hierarchy and limits match desired state".to_string()],
        ),
        Operation::Rescan if collection == Some("btrfsQgroups") => (
            vec![
                command(
                    ["btrfs", "qgroup", "show", "--raw", "-reF", target],
                    false,
                    "verify Btrfs qgroup usage and hierarchy after rescan",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled Btrfs qgroup relationships after re-probe",
                ),
            ],
            vec![
                "Btrfs qgroup referenced and exclusive usage match refreshed topology"
                    .to_string(),
                "qgroup limits and hierarchy are reviewed before later enforcement changes"
                    .to_string(),
            ],
        ),
        Operation::Rescan if collection == Some("btrfsSubvolumes") => (
            vec![
                command(
                    ["btrfs", "subvolume", "show", target],
                    false,
                    "verify Btrfs subvolume metadata after rescan",
                ),
                command(
                    ["btrfs", "property", "get", "-ts", target, "ro"],
                    false,
                    "verify Btrfs subvolume read-only property after rescan",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled Btrfs subvolume relationships after re-probe",
                ),
            ],
            vec![
                "Btrfs subvolume metadata and read-only state match refreshed topology"
                    .to_string(),
                "snapshot and qgroup relationships are reviewed before later cleanup"
                    .to_string(),
            ],
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
        Operation::Rescan if collection == Some("exports") => {
            let target = export_target_path(action);
            let inspect_target = target.unwrap_or("<export-path>");
            let inspect_command = match target {
                Some(target) => command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled NFS export graph state after rescan",
                ),
                None => command_with_readiness(
                    ["disk-nix", "inspect", inspect_target, "--json"],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["NFS export path"],
                    "verify modeled NFS export graph state after selecting the export path",
                ),
            };
            (
                vec![
                    command(
                        ["exportfs", "-v"],
                        false,
                        "verify NFS export inventory after rescan",
                    ),
                    inspect_command,
                ],
                vec![
                    "exportfs reports the reviewed path and client options".to_string(),
                    "modeled NFS export relationships match the refreshed inventory".to_string(),
                ],
            )
        }
        Operation::SetProperty if collection == Some("snapshots") => {
            let snapshot = action.context.name.as_deref().unwrap_or(target);
            (
                vec![
                    snapshot_hold_list_command(snapshot),
                    command(
                        ["disk-nix", "inspect", snapshot, "--json"],
                        false,
                        "verify modeled snapshot properties after re-probe",
                    ),
                ],
                vec!["snapshot hold state matches the desired retention tag".to_string()],
            )
        }
        Operation::Create | Operation::Export | Operation::Destroy | Operation::Unexport
            if collection == Some("exports") =>
        (
            vec![
                command(
                    ["exportfs", "-v"],
                    false,
                    "verify exported NFS paths and client selectors",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled NFS export relationships after re-probe",
                ),
            ],
            vec![
                "export path, client selector, and options match desired state".to_string(),
                "remote clients are intentionally added, migrated, or drained".to_string(),
            ],
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
            if is_zfs_snapshot_name(snapshot) {
                commands.push(command(
                    ["zfs", "list", "-t", "snapshot", "-H", "-p", snapshot],
                    false,
                    "verify ZFS snapshot existence and metadata",
                ));
            } else if collection == Some("btrfsSubvolumes")
                || is_btrfs_snapshot_pair(target, snapshot)
            {
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
        Operation::Rescan if collection == Some("snapshots") => {
            let snapshot = action.context.name.as_deref().unwrap_or(target);
            let mut commands = vec![command(
                ["disk-nix", "inspect", snapshot, "--json"],
                false,
                "verify modeled snapshot graph relationships after metadata refresh",
            )];
            if is_zfs_snapshot_name(snapshot) {
                commands.push(zfs_snapshot_list_command(
                    snapshot,
                    "verify ZFS snapshot size and reference metadata after rescan",
                ));
                commands.push(command(
                    [
                        "zfs",
                        "get",
                        "-H",
                        "-p",
                        "-o",
                        "property,value",
                        "creation,used,referenced,userrefs,defer_destroy",
                        snapshot,
                    ],
                    false,
                    "verify ZFS snapshot properties and retention metadata after rescan",
                ));
                commands.push(snapshot_hold_list_command(snapshot));
            } else if snapshot.starts_with('/') {
                commands.push(command(
                    ["btrfs", "subvolume", "show", snapshot],
                    false,
                    "verify Btrfs snapshot subvolume metadata after rescan",
                ));
                commands.push(command(
                    ["btrfs", "property", "get", "-ts", snapshot, "ro"],
                    false,
                    "verify Btrfs snapshot read-only property after rescan",
                ));
            }
            (
                commands,
                vec![
                    "snapshot metadata, source relationship, and retention state match the refreshed topology"
                        .to_string(),
                ],
            )
        }
        Operation::Create | Operation::Open if collection == Some("luks.devices") => (
            vec![
                command(
                    ["cryptsetup", "status", target],
                    false,
                    "verify the LUKS mapper is open",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify opened mapper identity and graph relationships",
                ),
            ],
            vec![
                "mapper name and backing device match the desired declaration".to_string(),
                "dependent storage layers see the opened mapper path".to_string(),
            ],
        ),
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
        Operation::Destroy if collection == Some("datasets") => (
            vec![command(
                ["zfs", "list", "-H", "-p", "-t", "filesystem"],
                false,
                "verify ZFS dataset inventory after destruction",
            )],
            vec![
                "destroyed dataset no longer appears in ZFS filesystem listings".to_string(),
                "mounts, descendants, snapshots, and consumers were drained or updated".to_string(),
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
        Operation::Destroy if collection == Some("volumes") => (
            vec![
                command(
                    ["lvs", "--reportformat", "json"],
                    false,
                    "verify logical volume no longer appears in LVM inventory",
                ),
                command(
                    ["disk-nix", "topology", "--json"],
                    false,
                    "re-probe topology after logical volume removal",
                ),
            ],
            vec![
                "removed logical volume is absent from LVM reports".to_string(),
                "dependent filesystems, mappings, and mounts no longer reference the LV"
                    .to_string(),
            ],
        ),
        Operation::Destroy if collection == Some("thinPools") => (
            vec![
                command(
                    ["lvs", "--reportformat", "json"],
                    false,
                    "verify thin pool no longer appears in LVM inventory",
                ),
                command(
                    ["disk-nix", "topology", "--json"],
                    false,
                    "re-probe topology after thin pool removal",
                ),
            ],
            vec![
                "removed thin pool is absent from LVM reports".to_string(),
                "dependent thin volumes, filesystems, mappings, and mounts are removed or migrated"
                    .to_string(),
            ],
        ),
        Operation::Destroy if collection == Some("volumeGroups") => (
            vec![
                command(
                    ["vgs", "--reportformat", "json"],
                    false,
                    "verify LVM volume group inventory after removal",
                ),
                command(
                    ["pvs", "--reportformat", "json"],
                    false,
                    "verify physical volume state after volume group removal",
                ),
            ],
            vec![
                "removed volume group no longer appears in LVM reports".to_string(),
                "no logical volumes or device-mapper nodes still depend on the removed VG"
                    .to_string(),
            ],
        ),
        Operation::Import | Operation::Export if collection == Some("volumeGroups") => (
            vec![
                command(
                    ["vgs", "--reportformat", "json", target],
                    false,
                    "verify LVM volume group inventory after import or export",
                ),
                command(
                    ["pvs", "--reportformat", "json"],
                    false,
                    "verify physical volume membership after VG import or export",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled VG graph relationships after import or export",
                ),
            ],
            vec![
                "volume group import or export state matches the declared lifecycle operation"
                    .to_string(),
                "logical volumes, filesystems, mappings, mounts, and services are reviewed after the VG state change"
                    .to_string(),
            ],
        ),
        Operation::Destroy if collection == Some("vdoVolumes") => (
            vec![
                command(
                    ["vdo", "status"],
                    false,
                    "verify VDO volume inventory after removal",
                ),
                command(
                    ["disk-nix", "topology", "--json"],
                    false,
                    "re-probe topology after VDO volume removal",
                ),
            ],
            vec![
                "removed VDO volume no longer appears in VDO status output".to_string(),
                "dependent filesystems, mappings, and mounts no longer reference the VDO device"
                    .to_string(),
            ],
        ),
        Operation::Stop if collection == Some("vdoVolumes") => (
            vec![
                command(
                    ["vdo", "status"],
                    false,
                    "verify VDO volume inventory after stop",
                ),
                command(
                    ["disk-nix", "topology", "--json"],
                    false,
                    "re-probe topology after VDO volume stop",
                ),
            ],
            vec![
                "stopped VDO volume is no longer active in VDO status output".to_string(),
                "dependent filesystems, mappings, and mounts no longer reference the stopped VDO device"
                    .to_string(),
            ],
        ),
        Operation::Destroy if collection == Some("pools") => (
            vec![
                command(
                    ["zpool", "list", "-H", "-p"],
                    false,
                    "verify ZFS pool inventory after destruction",
                ),
                command(
                    ["disk-nix", "topology", "--json"],
                    false,
                    "re-probe topology after pool destruction",
                ),
            ],
            vec![
                "destroyed pool no longer appears in ZFS pool listings".to_string(),
                "datasets, zvols, exports, and mounts have been migrated or removed".to_string(),
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
        Operation::Rollback if collection == Some("snapshots") => {
            let snapshot = action.context.name.as_deref().unwrap_or(target);
            if !is_zfs_snapshot_name(snapshot) {
                return (Vec::new(), Vec::new());
            }
            let dataset = zfs_snapshot_dataset(snapshot).unwrap_or("<dataset>");
            (
                vec![
                    command(
                        ["zfs", "list", "-t", "snapshot", "-H", "-p", snapshot],
                        false,
                        "verify the ZFS snapshot still exists after rollback",
                    ),
                    command(
                        ["zfs", "list", "-H", "-p", dataset],
                        false,
                        "verify the rolled-back ZFS dataset after rollback",
                    ),
                    command(
                        ["disk-nix", "inspect", dataset, "--json"],
                        false,
                        "verify dataset graph state and consumers after rollback",
                    ),
                ],
                vec![
                    "dataset contents match the reviewed snapshot rollback point".to_string(),
                    "newer snapshots, clones, mounts, and dependent services were reviewed after rollback"
                        .to_string(),
                ],
            )
        }
        Operation::Clone if collection == Some("snapshots") => {
            let snapshot = action.context.name.as_deref().unwrap_or(target);
            let clone_target = action.context.target.as_deref().unwrap_or("<clone-dataset>");
            if !is_zfs_snapshot_name(snapshot) {
                return (Vec::new(), Vec::new());
            }
            (
                vec![
                    command(
                        ["zfs", "list", "-t", "snapshot", "-H", "-p", snapshot],
                        false,
                        "verify source ZFS snapshot exists before clone",
                    ),
                    command(
                        ["zfs", "list", "-H", "-p", clone_target],
                        false,
                        "verify cloned ZFS dataset after clone",
                    ),
                    command(
                        ["disk-nix", "inspect", clone_target, "--json"],
                        false,
                        "verify cloned dataset graph state after clone",
                    ),
                ],
                vec![
                    "clone dataset exists and is mounted or configured as expected".to_string(),
                    "clone origin points at the reviewed source snapshot".to_string(),
                ],
            )
        }
        Operation::Promote if collection == Some("datasets") || collection == Some("zvols") => {
            let target = action.context.target.as_deref().unwrap_or(target);
            (
                vec![
                    command(
                        ["zfs", "get", "-H", "-o", "value", "origin", target],
                        false,
                        "verify clone origin after promotion",
                    ),
                    command(
                        ["disk-nix", "inspect", target, "--json"],
                        false,
                        "verify promoted ZFS object graph state after promotion",
                    ),
                ],
                vec![
                    "promoted clone remains available at the reviewed dataset or zvol name"
                        .to_string(),
                    "origin dependency and dependent snapshots were reviewed after promotion"
                        .to_string(),
                ],
            )
        }
        Operation::Import | Operation::Export if collection == Some("pools") => (
            vec![
                command(
                    ["zpool", "status", "-P", target],
                    false,
                    "verify ZFS pool health after import or export",
                ),
                command(
                    ["zpool", "list", "-H", "-p"],
                    false,
                    "verify active ZFS pool inventory after import or export",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled pool graph relationships after import or export",
                ),
            ],
            vec![
                "pool import or export state matches the declared lifecycle operation".to_string(),
                "datasets, mountpoints, shares, LUN mappings, and services are reviewed after the pool state change"
                    .to_string(),
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
        Operation::Destroy | Operation::Close if collection == Some("luks.devices") => (
            vec![
                command(
                    ["cryptsetup", "status", target],
                    false,
                    "confirm LUKS mapper is closed or absent after close",
                ),
                command(
                    ["disk-nix", "topology", "--json"],
                    false,
                    "verify dependent graph no longer references the mapper",
                ),
            ],
            vec![
                "mapper is inactive or absent after close".to_string(),
                "backing LUKS device remains intact unless a separate format action was requested"
                    .to_string(),
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
        Operation::Destroy if collection == Some("snapshots") => {
            let snapshot = action.context.name.as_deref().unwrap_or(target);
            let source = action.context.target.as_deref().unwrap_or(target);
            (
                vec![
                    command(
                        ["disk-nix", "inspect", source, "--json"],
                        false,
                        "verify source target after snapshot deletion",
                    ),
                    command(
                        ["disk-nix", "topology", "--json"],
                        false,
                        "re-probe full topology after snapshot deletion",
                    ),
                ],
                vec![
                    format!("snapshot {snapshot} no longer appears in topology"),
                    "source target remains present with expected consumers and mount state"
                        .to_string(),
                ],
            )
        }
        Operation::Remount if action.context.collection.as_deref() == Some("filesystems") => {
            let mountpoint = filesystem_mountpoint(action);
            (
                vec![
                    filesystem_findmnt_command(mountpoint),
                    command(
                        ["disk-nix", "inspect", mountpoint.unwrap_or(target), "--json"],
                        false,
                        "verify filesystem graph state after remount",
                    ),
                ],
                vec![
                    "findmnt reports the mountpoint with the reviewed filesystem options"
                        .to_string(),
                    "local services continue to see the expected filesystem source and type"
                        .to_string(),
                ],
            )
        }
        Operation::Format
        | Operation::Shrink
        | Operation::Clone
        | Operation::Promote
        | Operation::Import
        | Operation::Export
        | Operation::Unexport
        | Operation::Attach
        | Operation::Detach
        | Operation::Activate
        | Operation::Deactivate
        | Operation::Assemble
        | Operation::Start
        | Operation::Stop
        | Operation::Login
        | Operation::Logout
        | Operation::Open
        | Operation::Close
        | Operation::Mount
        | Operation::Unmount
        | Operation::Remount
        | Operation::Rename
        | Operation::Rescan
        | Operation::AddKey
        | Operation::RemoveKey
        | Operation::ImportToken
        | Operation::RemoveToken
        | Operation::RemoveDevice
        | Operation::Repair
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
        Operation::Check => (
            vec![command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify target state after filesystem check",
            )],
            vec!["read-only check completed and no repair action was applied".to_string()],
        ),
        Operation::Scrub => (
            vec![command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify target state after scrub operation",
            )],
            vec!["scrub completed or is running with reviewed health status".to_string()],
        ),
        Operation::Trim => (
            vec![command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify target state after trim operation",
            )],
            vec!["filesystem remains mounted and reports consistent usage after trim".to_string()],
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
        Operation::Grow
            if collection == Some("filesystems") || action.id.starts_with("filesystem:") =>
        {
            let target = target.unwrap_or("<filesystem>");
            let fs_type = action
                .context
                .fs_type
                .as_deref()
                .unwrap_or("<filesystem-type>");
            let device = action.context.device.as_deref();
            let desired_size = action.context.desired_size.as_deref();
            let grow_command = filesystem_grow_command(fs_type, target, device, desired_size);
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
        Operation::Shrink
            if collection == Some("filesystems") || action.id.starts_with("filesystem:") =>
        {
            let target = target.unwrap_or("<filesystem>");
            let fs_type = action
                .context
                .fs_type
                .as_deref()
                .unwrap_or("<filesystem-type>");
            let device = action.context.device.as_deref();
            let desired_size = action.context.desired_size.as_deref();
            (
                filesystem_shrink_commands(fs_type, target, device, desired_size),
                vec![
                    "shrink only after backups or snapshots are verified".to_string(),
                    "prefer migrate-to-smaller-filesystem workflows when online shrink support is absent"
                        .to_string(),
                    "restore dependent mounts and services only after post-shrink checks pass"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Check
            if collection == Some("filesystems") || action.id.starts_with("filesystems:") =>
        {
            let target = target.unwrap_or("<filesystem>");
            let fs_type = action
                .context
                .fs_type
                .as_deref()
                .unwrap_or("<filesystem-type>");
            let device = action.context.device.as_deref();
            (
                filesystem_check_commands(fs_type, target, device),
                vec![
                    "run read-only consistency checks before any repair workflow".to_string(),
                    "quiesce or unmount the filesystem when the checker requires offline access"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Repair
            if collection == Some("filesystems") || action.id.starts_with("filesystems:") =>
        {
            let target = target.unwrap_or("<filesystem>");
            let fs_type = action
                .context
                .fs_type
                .as_deref()
                .unwrap_or("<filesystem-type>");
            let device = action.context.device.as_deref();
            (
                filesystem_repair_commands(fs_type, target, device),
                vec![
                    "repair only after a read-only check and backup review".to_string(),
                    "prefer repairing a cloned device before the production filesystem when practical"
                        .to_string(),
                    "restore mounts and services only after post-repair verification passes"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Remount
            if collection == Some("filesystems") || action.id.starts_with("filesystems:") =>
        {
            let mountpoint = filesystem_mountpoint(action);
            (
                vec![
                    filesystem_findmnt_command(mountpoint),
                    filesystem_remount_command(mountpoint, action.context.options.as_deref()),
                ],
                vec![
                    "review active services before changing filesystem mount options".to_string(),
                    "persist the final options through the NixOS fileSystems entry".to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("volumes") || action.id.starts_with("volumes:") => {
            let target = lvm_volume_target_path(target);
            let desired_size = action.context.desired_size.as_deref();
            let note = desired_size
                .map(|size| format!("desired size from spec: {size}"))
                .unwrap_or_else(|| {
                    "replace <size> after comparing desired state with probed capacity".to_string()
                });
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        None,
                        "inspect current LVM logical volume state",
                    ),
                    lvm_logical_volume_extend_command(target, desired_size),
                ],
                vec![note],
                true,
            )
        }
        Operation::Rescan if collection == Some("volumes") || action.id.starts_with("volumes:") => {
            let target = lvm_volume_target_path(target);
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        None,
                        "refresh LVM logical volume attributes and activation state",
                    ),
                    command(
                        ["disk-nix", "inspect", target.unwrap_or("<logical-volume>")],
                        false,
                        "inspect modeled LV graph relationships after status refresh",
                    ),
                ],
                vec![
                    "use grow when LV capacity must change".to_string(),
                    "use activate or deactivate when LV availability must change".to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            let device = action.context.device.as_deref();
            (
                vec![
                    command(
                        ["vgs", "--reportformat", "json", target],
                        false,
                        "inspect current volume group size and free extents before growth",
                    ),
                    volume_group_extend_command(target, device),
                ],
                vec![
                    "initialize or verify the physical volume before extending the VG".to_string(),
                    "grow dependent logical volumes only after VG free extents reflect added capacity"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("physicalVolumes") => {
            let target = lvm_physical_volume_target(action);
            (
                vec![
                    lvm_physical_volume_inspect_command(target),
                    lvm_physical_volume_resize_command(target),
                ],
                vec![
                    "grow the backing partition, LUN, or disk before pvresize".to_string(),
                    "verify volume group free extents before extending logical volumes".to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("physicalVolumes") => {
            let target = lvm_physical_volume_target(action);
            let inspect = match target {
                Some(target) => command(
                    ["pvs", "--reportformat", "json", target],
                    false,
                    "inspect physical volume metadata before cache refresh",
                ),
                None => command(
                    ["pvs", "--reportformat", "json"],
                    false,
                    "inspect current LVM physical volume inventory before cache refresh",
                ),
            };
            let mut commands = vec![inspect, lvm_physical_volume_rescan_command(target)];
            commands.push(command(
                ["pvs", "--reportformat", "json"],
                false,
                "inspect refreshed LVM physical volume inventory",
            ));
            (
                commands,
                vec![
                    "rescan backing block paths first when device visibility changed".to_string(),
                    "use grow when pvresize is needed after capacity changes".to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            (
                vec![
                    command(
                        ["vgs", "--reportformat", "json", target],
                        false,
                        "inspect volume group before metadata refresh",
                    ),
                    command(
                        ["pvscan", "--cache"],
                        true,
                        "refresh the LVM physical volume device cache",
                    ),
                    command(
                        ["vgscan"],
                        true,
                        "scan available LVM volume groups without creating metadata",
                    ),
                    command(
                        ["vgchange", "--refresh", target],
                        true,
                        "reactivate the reviewed volume group with refreshed metadata",
                    ),
                ],
                vec![
                    "run host path rescans before VG refresh when devices were added or resized"
                        .to_string(),
                    "verify LV activation state and VG free extents after refresh".to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("thinPools") => {
            let target = lvm_volume_target_path(target);
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        Some("lv_name,lv_size,data_percent,metadata_percent,seg_monitor"),
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
        Operation::Rescan if collection == Some("thinPools") => {
            let target = lvm_volume_target_path(target);
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        Some("lv_name,lv_size,data_percent,metadata_percent,seg_monitor"),
                        "refresh thin pool data, metadata, and monitoring state",
                    ),
                    command(
                        ["disk-nix", "inspect", target.unwrap_or("<thin-pool>")],
                        false,
                        "inspect modeled thin pool relationships after status refresh",
                    ),
                ],
                vec![
                    "use grow when data or metadata capacity must change".to_string(),
                    "review utilization before allocating more thin volumes".to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("thinPools") => {
            let target = target.unwrap_or("<thin-pool>");
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    command(
                        ["vgs", "--reportformat", "json"],
                        false,
                        "inspect volume group free space before creating the thin pool",
                    ),
                    thin_pool_create_command(target, desired_size),
                ],
                vec![
                    "verify the target volume group has enough data and metadata capacity"
                        .to_string(),
                    "choose overcommit, monitoring, and autoextend policy before using the thin pool"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("lvmCaches") => {
            let target = lvm_volume_target_path(target);
            let cache_pool = action
                .context
                .device
                .as_deref()
                .or_else(|| action.context.devices.first().map(String::as_str));
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        Some(
                            "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent",
                        ),
                        "inspect origin LV and cache state before attaching LVM cache",
                    ),
                    lvm_cache_attach_command(target, cache_pool),
                ],
                vec![
                    "verify the cache pool LV is clean and belongs to the same VG as the origin"
                        .to_string(),
                    "prefer writethrough cache mode until post-attach verification passes"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("lvmCaches") => {
            let target = lvm_volume_target_path(target);
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        Some(
                            "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent",
                        ),
                        "refresh LVM cache mode, policy, utilization, and metadata state",
                    ),
                    command(
                        ["disk-nix", "inspect", target.unwrap_or("<lvm-cache>")],
                        false,
                        "inspect modeled LVM cache relationships after status refresh",
                    ),
                ],
                vec![
                    "use property updates when cache mode or policy must change".to_string(),
                    "verify dirty data before detach, uncache, or cache-pool replacement"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("physicalVolumes") => {
            let target = lvm_physical_volume_target(action);
            (
                vec![
                    disk_nix_inspect_command(
                        target,
                        "<physical-volume>",
                        "physical volume device",
                        "inspect target device before creating LVM PV metadata",
                    ),
                    lvm_physical_volume_create_command(target),
                ],
                vec![
                    "verify the device contains no data that must be preserved before pvcreate"
                        .to_string(),
                    "extend or create a volume group only after pvs reports the PV".to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("loopDevices") => {
            let target = loop_device_target_path(action);
            (
                vec![
                    loop_device_list_command(
                        target,
                        "inspect loop device before refreshing backing size",
                    ),
                    loop_device_refresh_command(target),
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
        Operation::Rescan if collection == Some("loopDevices") => {
            let target = loop_device_target_path(action);
            (
                vec![
                    loop_device_list_command(target, "refresh loop device mapping inventory"),
                    loop_device_inspect_command(target),
                ],
                vec![
                    "loop rescan does not refresh size; use grow after backing size changes"
                        .to_string(),
                    "review dependent filesystems and mappings before detach".to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("luns") || action.id.starts_with("luns:") => {
            let target = target.unwrap_or("<lun>");
            let mut commands = vec![
                command(
                    ["iscsiadm", "--mode", "session", "--rescan"],
                    true,
                    "rescan iSCSI sessions to refresh existing LUN paths",
                ),
                command(
                    ["disk-nix", "inspect", target],
                    false,
                    "inspect current LUN paths before per-device rescans",
                ),
            ];
            let devices = lun_rescan_devices(action);
            if devices.is_empty() {
                commands.push(command_with_readiness(
                    ["<scsi-rescan-device>", "<lun-path>"],
                    true,
                    CommandReadiness::NeedsDomainImplementation,
                    ["stable LUN device path"],
                    "rescan the concrete SCSI path after declaring a stable by-path LUN device",
                ));
            }
            for device in devices {
                commands.push(scsi_device_rescan_command(&device));
            }
            commands.extend([
                command(
                    ["multipath", "-r"],
                    true,
                    "reload multipath maps after refreshed LUN paths",
                ),
                command(
                    ["disk-nix", "inspect", target],
                    false,
                    "verify that refreshed paths and consumers are visible",
                ),
            ]);
            (
                commands,
                vec![
                    "declare stable LUN path devices to render per-path SCSI rescans".to_string(),
                    "verify multipath maps before exposing dependent consumers".to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("luns") || action.id.starts_with("luns:") => {
            let target = target.unwrap_or("<lun>");
            let mut commands = vec![
                command(
                    ["iscsiadm", "--mode", "session", "--rescan"],
                    true,
                    "rescan iSCSI sessions after target-side LUN growth",
                ),
                command(
                    ["disk-nix", "inspect", target],
                    false,
                    "inspect current LUN paths before per-device rescans",
                ),
            ];
            let devices = lun_rescan_devices(action);
            if devices.is_empty() {
                commands.push(command_with_readiness(
                    ["<scsi-rescan-device>", "<lun-path>"],
                    true,
                    CommandReadiness::NeedsDomainImplementation,
                    ["stable LUN device path"],
                    "rescan the concrete SCSI path after declaring a stable by-path LUN device",
                ));
            }
            for device in devices {
                commands.push(scsi_device_rescan_command(&device));
            }
            commands.extend([
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
            ]);
            (
                commands,
                vec![
                    "coordinate the target-side LUN grow before host rescans".to_string(),
                    "declare stable LUN path devices to render per-path SCSI rescans".to_string(),
                ],
                true,
            )
        }
        Operation::Create | Operation::Attach
            if collection == Some("luns") || action.id.starts_with("luns:") =>
        {
            let target = target.unwrap_or("<lun>");
            let mut commands = vec![
                command(
                    ["iscsiadm", "--mode", "session", "--rescan"],
                    true,
                    "rescan iSCSI sessions after target-side LUN creation",
                ),
                command(
                    ["multipath", "-r"],
                    true,
                    "reload multipath maps after newly attached LUN paths appear",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "inspect the newly attached LUN and consumers",
                ),
            ];
            let devices = lun_rescan_devices(action);
            if devices.is_empty() {
                commands.push(command_vec_with_readiness(
                    vec!["blockdev", "--getsize64", "<lun-path>"],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["stable LUN device path"],
                    "verify the reviewed LUN path after declaring a stable by-path device",
                ));
            }
            for device in devices {
                commands.push(command_vec(
                    vec!["blockdev", "--getsize64", device.as_str()],
                    false,
                    "verify the reviewed LUN path is visible to the kernel",
                ));
            }
            (
                commands,
                vec![
                    "create or map the target-side LUN before host attach".to_string(),
                    "declare stable LUN path devices to verify every expected path".to_string(),
                    "enable filesystems, LVM, or multipath consumers only after verification"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Grow | Operation::Rescan
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
        Operation::Create | Operation::Login
            if collection == Some("iscsiSessions") || action.id.starts_with("iscsiSessions:") =>
        {
            let target = target.unwrap_or("<iscsi-target-iqn>");
            let portal = action.context.portal.as_deref();
            let discovery = match portal {
                Some(portal) => command_vec(
                    vec![
                        "iscsiadm",
                        "--mode",
                        "discovery",
                        "--type",
                        "sendtargets",
                        "--portal",
                        portal,
                    ],
                    true,
                    "discover iSCSI target records from the reviewed portal",
                ),
                None => command_vec_with_readiness(
                    vec![
                        "iscsiadm",
                        "--mode",
                        "discovery",
                        "--type",
                        "sendtargets",
                        "--portal",
                        "<portal>",
                    ],
                    true,
                    CommandReadiness::NeedsDomainImplementation,
                    ["iSCSI portal"],
                    "discover iSCSI target records after selecting the target portal",
                ),
            };
            let login = match portal {
                Some(portal) => command_vec(
                    vec![
                        "iscsiadm",
                        "--mode",
                        "node",
                        "--targetname",
                        target,
                        "--portal",
                        portal,
                        "--login",
                    ],
                    true,
                    "log in to the reviewed iSCSI target through the selected portal",
                ),
                None => command_vec_with_readiness(
                    vec![
                        "iscsiadm",
                        "--mode",
                        "node",
                        "--targetname",
                        target,
                        "--portal",
                        "<portal>",
                        "--login",
                    ],
                    true,
                    CommandReadiness::NeedsDomainImplementation,
                    ["iSCSI portal"],
                    "log in to the iSCSI target after selecting the target portal",
                ),
            };
            (
                vec![discovery, login],
                vec![
                    "verify the target IQN and portal before creating host sessions".to_string(),
                    "rescan and settle multipath paths before exposing dependent volumes"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy | Operation::Logout
            if collection == Some("iscsiSessions") || action.id.starts_with("iscsiSessions:") =>
        {
            let target = target.unwrap_or("<iscsi-target-iqn>");
            let portal = action.context.portal.as_deref();
            let logout = match portal {
                Some(portal) => command_vec(
                    vec![
                        "iscsiadm",
                        "--mode",
                        "node",
                        "--targetname",
                        target,
                        "--portal",
                        portal,
                        "--logout",
                    ],
                    true,
                    "log out from the reviewed iSCSI target and portal",
                ),
                None => command_vec(
                    vec![
                        "iscsiadm",
                        "--mode",
                        "node",
                        "--targetname",
                        target,
                        "--logout",
                    ],
                    true,
                    "log out from all node records for the reviewed iSCSI target",
                ),
            };
            (
                vec![logout],
                vec![
                    "unmount filesystems and deactivate mappings before logging out".to_string(),
                    "verify multipath, LVM, and filesystem consumers have migrated away"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("swaps") => {
            let target = swap_target_path(action);
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    command(
                        ["swapon", "--show", "--bytes", "--raw"],
                        false,
                        "inspect active swap state before resizing",
                    ),
                    swap_command(
                        "swapoff",
                        target,
                        "disable swap before changing backing storage or signature",
                    ),
                    swap_resize_command(target, desired_size),
                    swap_command(
                        "mkswap",
                        target,
                        "recreate the swap signature after backing storage resize",
                    ),
                    swap_command("swapon", target, "reactivate swap after verification"),
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
        Operation::Rescan if collection == Some("swaps") => {
            let target = swap_target_path(action);
            (
                vec![
                    command(
                        ["swapon", "--show", "--bytes", "--raw"],
                        false,
                        "refresh active swap inventory",
                    ),
                    swap_blkid_command(target, "refresh swap signature label and UUID"),
                    swap_inspect_command(
                        target,
                        "inspect modeled swap relationships after refresh",
                    ),
                ],
                vec![
                    "use grow when backing swap capacity changed".to_string(),
                    "use format only when replacing the swap signature is intended".to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("luks.devices") => {
            let mapper = target.unwrap_or("<mapper>");
            let device = action.context.device.as_deref();
            (
                vec![
                    luks_backing_inspect_command(
                        device,
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
        Operation::Start if collection == Some("vdoVolumes") => {
            let target = target.unwrap_or("<vdo-volume>");
            (
                vec![
                    command(
                        ["vdo", "status", "--name", target],
                        false,
                        "inspect VDO volume before start",
                    ),
                    command(
                        ["vdo", "start", "--name", target],
                        true,
                        "start the existing VDO volume after backing storage is present",
                    ),
                ],
                vec![
                    "verify the backing device is present and stable before starting VDO".to_string(),
                    "activate dependent filesystems, LVM layers, or mounts only after VDO status is healthy"
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
            let target = md_array_target_path(action);
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    md_raid_detail_command(
                        target,
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
        Operation::Rescan if collection == Some("mdRaids") => {
            let target = md_array_target_path(action);
            let mut commands = Vec::new();
            if let Some(target) = target {
                commands.push(command(
                    ["mdadm", "--detail", target],
                    false,
                    "inspect targeted MD RAID array before metadata rescan",
                ));
            }
            commands.extend([
                command(
                    ["mdadm", "--detail", "--scan"],
                    false,
                    "list assembled MD RAID arrays from current metadata",
                ),
                command(
                    ["mdadm", "--examine", "--scan"],
                    false,
                    "scan member devices for MD RAID metadata without assembling arrays",
                ),
                command(
                    ["cat", "/proc/mdstat"],
                    false,
                    "inspect kernel MD RAID status after metadata scan",
                ),
            ]);
            (
                commands,
                vec![
                    "use assemble when reviewed member metadata should activate an array"
                        .to_string(),
                    "verify member event counts before any replacement, grow, or assemble operation"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("multipathMaps") => {
            let target = multipath_map_target(action);
            (
                vec![
                    multipath_list_command(
                        target,
                        "inspect multipath map paths and size before growth",
                    ),
                    multipath_resize_command(target),
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
        Operation::Rescan if collection == Some("multipathMaps") => {
            let target = multipath_map_target(action);
            (
                vec![
                    multipath_list_command(target, "inspect multipath map paths before rescan"),
                    command(
                        ["multipath", "-r"],
                        true,
                        "reload multipath maps after refreshed backing paths",
                    ),
                    multipath_list_command(target, "verify multipath map paths after rescan"),
                ],
                vec![
                    "rescan backing SCSI or iSCSI paths before reloading the map".to_string(),
                    "verify the map WWID and every expected path before exposing consumers"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("nvmeNamespaces") => {
            let controller = nvme_controller_target(action);
            (
                vec![
                    nvme_list_namespaces_command(
                        controller,
                        "inspect NVMe namespaces before rescan",
                    ),
                    nvme_namespace_rescan_command(controller),
                    nvme_list_namespaces_command(controller, "verify NVMe namespaces after rescan"),
                ],
                vec![
                    "verify namespace inventory before exposing refreshed devices to consumers"
                        .to_string(),
                    "use grow when controller-side namespace capacity changed".to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("nvmeNamespaces") => {
            let controller = nvme_controller_target(action);
            (
                vec![
                    nvme_list_namespaces_command(controller, "inspect NVMe namespaces before rescan"),
                    nvme_namespace_rescan_command(controller),
                    nvme_list_namespaces_command(controller, "verify NVMe namespaces after rescan"),
                ],
                vec![
                    "perform controller-side namespace resize before host rescan".to_string(),
                    "grow dependent partitions, volumes, or filesystems only after the namespace reports the new size"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("partitions") => {
            let partition_target = partition_target_path(action);
            let disk = action.context.device.as_deref();
            let partition_number = action.context.partition_number.as_deref();
            let desired_end = action
                .context
                .end
                .as_deref()
                .or(action.context.desired_size.as_deref());
            (
                vec![
                    disk_nix_inspect_command(
                        partition_target,
                        "<partition>",
                        "partition path",
                        "inspect partition, consumers, and backing device before growth",
                    ),
                    partition_grow_command(disk, partition_number, desired_end),
                    command(
                        ["partprobe"],
                        true,
                        "ask the kernel to reread partition tables after the geometry change",
                    ),
                    partition_table_reread_command(disk),
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
        Operation::Rescan if collection == Some("disks") || collection == Some("partitions") => {
            let disk = partition_rescan_disk(action);
            (
                vec![
                    disk_nix_inspect_command(
                        disk,
                        "<disk>",
                        "disk path",
                        "inspect disk identity before partition-table rescan",
                    ),
                    partition_probe_command(disk),
                    partition_table_reread_command(disk),
                    disk_parted_machine_list_command(
                        disk,
                        "verify the disk partition table after reread",
                    ),
                ],
                vec![
                    "use grow or create when partition geometry changes are still required"
                        .to_string(),
                    "pause dependent consumers when an active kernel table cannot be reread"
                        .to_string(),
                    "verify stable by-id and by-partuuid paths before growing consumers"
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
                vec![
                    "select the grow command from the target storage domain and desired size"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::AddDevice if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            let device = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "add-device"));
            (
                vec![
                    command(
                        ["vgs", "--reportformat", "json", target],
                        false,
                        "inspect current volume group state before adding a physical volume",
                    ),
                    volume_group_extend_command(target, device),
                ],
                vec![
                    "initialize or verify the physical volume before extending the VG".to_string(),
                ],
                true,
            )
        }
        Operation::AddDevice if collection == Some("mdRaids") => {
            let target = md_array_target_path(action);
            let device = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "add-device"));
            (
                vec![
                    md_raid_detail_command(
                        target,
                        "inspect MD RAID redundancy before adding a member",
                    ),
                    md_raid_add_member_command(target, device),
                ],
                vec![
                    "add a member or spare only after confirming array health and intended role"
                        .to_string(),
                    "monitor /proc/mdstat until recovery or reshape completes".to_string(),
                ],
                true,
            )
        }
        Operation::AddDevice if collection == Some("multipathMaps") => {
            let target = multipath_map_target(action);
            let path = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "add-device"));
            (
                vec![
                    multipath_list_command(
                        target,
                        "inspect live multipath paths before adding a path",
                    ),
                    multipath_add_path_command(path),
                ],
                vec![
                    "verify the path belongs to the intended LUN before adding it to multipathd"
                        .to_string(),
                    "reload or resize maps only after every expected path is visible".to_string(),
                ],
                true,
            )
        }
        Operation::AddDevice => {
            let target = target.unwrap_or("<target>");
            let device = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "add-device"));
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect target health before adding a device",
                    ),
                    add_device_command(collection, target, device),
                ],
                vec![
                    "verify the new device identity and redundancy policy before attaching it"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::ReplaceDevice if collection == Some("mdRaids") => {
            let target = md_array_target_path(action);
            let from = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "replace-device"));
            let to = action.context.replacement.as_deref();
            (
                vec![
                    md_raid_detail_command(
                        target,
                        "inspect MD RAID redundancy before member replacement",
                    ),
                    md_raid_replace_member_command(target, from, to),
                ],
                vec![
                    "replace one member at a time while the array is healthy".to_string(),
                    "monitor /proc/mdstat until replacement sync completes".to_string(),
                ],
                true,
            )
        }
        Operation::ReplaceDevice if collection == Some("multipathMaps") => {
            let target = multipath_map_target(action);
            let from = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "replace-device"));
            let to = action.context.replacement.as_deref();
            (
                vec![
                    multipath_list_command(
                        target,
                        "inspect live multipath paths before replacement",
                    ),
                    multipath_add_path_command(to),
                    multipath_delete_path_command(from),
                ],
                vec![
                    "add and verify the replacement path before deleting the old path".to_string(),
                    "keep alternate paths active while replacing a single path".to_string(),
                ],
                true,
            )
        }
        Operation::ReplaceDevice if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            let from = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "replace-device"));
            let to = action.context.replacement.as_deref();
            (
                vec![
                    lvm_physical_volume_inspect_command(from),
                    lvm_physical_volume_inspect_command(to),
                    lvm_volume_group_extend_replacement_command(target, to),
                    lvm_physical_volume_move_to_command(from, to),
                    lvm_volume_group_reduce_command(target, from),
                ],
                vec![
                    "add the replacement physical volume before moving extents".to_string(),
                    "keep the old PV online until pvmove completes and no allocated extents remain"
                        .to_string(),
                    "verify logical volumes, thin pools, and filesystems before vgreduce"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::ReplaceDevice => {
            let target = target.unwrap_or("<target>");
            let from = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "replace-device"));
            let to = action.context.replacement.as_deref();
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect redundancy and source device health before replacement",
                    ),
                    replace_device_command(collection, target, from, to),
                ],
                vec![
                    "keep the old device available until post-apply verification passes"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rebalance => {
            let target = target.unwrap_or("<target>");
            let rebalance =
                rebalance_command(collection, target, &action.context.property_assignments);
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect pool or filesystem health before rebalance",
                    ),
                    rebalance,
                ],
                vec![
                    "monitor progress and health until the rebalance operation completes"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Scrub => {
            let target = target.unwrap_or("<target>");
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect pool or filesystem health before scrub",
                    ),
                    scrub_command(collection, target),
                ],
                vec!["monitor scrub progress and health until completion".to_string()],
                true,
            )
        }
        Operation::Trim => {
            let target = target.unwrap_or("<filesystem>");
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect filesystem and backing discard support before trim",
                    ),
                    filesystem_trim_command(collection, target),
                ],
                vec![
                    "verify discard is safe through LUKS, LVM, thin, VDO, and virtual layers"
                        .to_string(),
                    "prefer scheduled fstrim for routine maintenance".to_string(),
                ],
                true,
            )
        }
        Operation::SetProperty => {
            let target = target.unwrap_or("<target>");
            let Some(property) = action.context.property.as_deref() else {
                return (
                    vec![command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect declared storage object state",
                    )],
                    vec!["no property mutation was requested by this declaration".to_string()],
                    false,
                );
            };
            let property_assignment = property_assignment(action);
            let property_command = if collection == Some("exports") {
                nfs_export_property_command(
                    target,
                    action.context.client.as_deref(),
                    property,
                    action.context.property_value.as_deref(),
                    action.context.options.as_deref(),
                )
            } else if collection == Some("btrfsQgroups") {
                btrfs_qgroup_property_command(
                    target,
                    action.context.name.as_deref().unwrap_or("<qgroupid>"),
                    property,
                    &property_assignment,
                )
            } else if collection == Some("snapshots") {
                snapshot_property_command(
                    action.context.name.as_deref().unwrap_or(target),
                    property,
                    action.context.property_value.as_deref(),
                )
            } else if collection == Some("filesystems") {
                filesystem_property_command(
                    action.context.fs_type.as_deref(),
                    target,
                    action.context.device.as_deref(),
                    property,
                    &property_assignment,
                )
            } else if collection == Some("swaps") {
                swap_property_command(
                    swap_target_path(action),
                    property,
                    action.context.property_value.as_deref(),
                )
            } else if collection == Some("luks.devices") {
                luks_device_property_command(
                    action.context.device.as_deref(),
                    property,
                    action.context.property_value.as_deref(),
                )
            } else if collection == Some("luksKeyslots") {
                luks_change_key_command(
                    luks_keyslot_device(action),
                    luks_keyslot_id(action),
                    action.context.key_file.as_deref(),
                    action.context.property_value.as_deref(),
                )
            } else if collection == Some("luksTokens") {
                luks_token_import_command(
                    luks_token_device(action),
                    luks_token_id(action),
                    action
                        .context
                        .property_value
                        .as_deref()
                        .or(action.context.token_file.as_deref()),
                )
            } else {
                set_property_command(collection, target, property, &property_assignment)
            };
            let inspect_target = if collection == Some("snapshots") {
                action.context.name.as_deref().unwrap_or(target)
            } else {
                target
            };
            (
                vec![
                    command(
                        ["disk-nix", "inspect", inspect_target],
                        false,
                        "inspect current properties before applying changes",
                    ),
                    property_command,
                ],
                vec![
                    "property values must come from the desired spec and target domain".to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("snapshots") => {
            let snapshot = action.context.name.as_deref().unwrap_or("<snapshot>");
            let mut commands = vec![command(
                ["disk-nix", "inspect", snapshot],
                false,
                "inspect modeled snapshot graph relationships after metadata refresh",
            )];
            if is_zfs_snapshot_name(snapshot) {
                commands.push(zfs_snapshot_list_command(
                    snapshot,
                    "refresh ZFS snapshot size and reference metadata",
                ));
                commands.push(command(
                    [
                        "zfs",
                        "get",
                        "-H",
                        "-p",
                        "-o",
                        "property,value",
                        "creation,used,referenced,userrefs,defer_destroy",
                        snapshot,
                    ],
                    false,
                    "refresh ZFS snapshot properties and retention metadata",
                ));
                commands.push(snapshot_hold_list_command(snapshot));
            } else if snapshot.starts_with('/') {
                commands.push(command(
                    ["btrfs", "subvolume", "show", snapshot],
                    false,
                    "refresh Btrfs snapshot subvolume metadata",
                ));
                commands.push(command(
                    ["btrfs", "property", "get", "-ts", snapshot, "ro"],
                    false,
                    "refresh Btrfs snapshot read-only property",
                ));
            } else {
                commands.push(command_with_readiness(
                    ["<snapshot-rescan-tool>", snapshot],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["ZFS snapshot name or Btrfs snapshot path"],
                    "refresh snapshot metadata after selecting the target-specific tool",
                ));
            }
            (
                commands,
                vec![
                    "use hold or release operations for retention changes".to_string(),
                    "use clone or rollback only after reviewing refreshed snapshot metadata"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Snapshot => {
            let target = target.unwrap_or("<snapshot>");
            let snapshot = action.context.name.as_deref().unwrap_or(target);
            let snapshot_command = if collection == Some("lvmSnapshots") {
                lvm_snapshot_create_command(
                    target,
                    snapshot,
                    action.context.desired_size.as_deref(),
                )
            } else {
                snapshot_command(
                    collection,
                    target,
                    snapshot,
                    action.context.read_only.unwrap_or(false),
                )
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
        Operation::Rescan if collection == Some("btrfsSubvolumes") => {
            let target = target.unwrap_or("<btrfs-subvolume-path>");
            let ready = action.context.target.as_deref().is_some();
            let show_command = if ready {
                command(
                    ["btrfs", "subvolume", "show", target],
                    false,
                    "refresh Btrfs subvolume metadata",
                )
            } else {
                command_with_readiness(
                    ["btrfs", "subvolume", "show", target],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["Btrfs subvolume path"],
                    "refresh Btrfs subvolume metadata after selecting the subvolume path",
                )
            };
            let readonly_command = if ready {
                command(
                    ["btrfs", "property", "get", "-ts", target, "ro"],
                    false,
                    "refresh Btrfs subvolume read-only property",
                )
            } else {
                command_with_readiness(
                    ["btrfs", "property", "get", "-ts", target, "ro"],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["Btrfs subvolume path"],
                    "refresh Btrfs subvolume read-only property after selecting the subvolume path",
                )
            };
            let inspect_command = if ready {
                command(
                    ["disk-nix", "inspect", target],
                    false,
                    "inspect modeled Btrfs subvolume relationships after refresh",
                )
            } else {
                command_with_readiness(
                    ["disk-nix", "inspect", target],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["Btrfs subvolume path"],
                    "inspect modeled Btrfs subvolume relationships after selecting the subvolume path",
                )
            };
            (
                vec![show_command, readonly_command, inspect_command],
                vec![
                    "subvolume rescan does not change read-only enforcement or namespace layout"
                        .to_string(),
                    "review qgroup and snapshot relationships before later destructive cleanup"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("btrfsQgroups") => {
            let qgroup_id = action.context.name.as_deref().unwrap_or("<qgroupid>");
            let target_path = btrfs_qgroup_target_path(action.context.target.as_deref(), qgroup_id);
            let target = target_path.unwrap_or("<btrfs-filesystem-path>");
            let inspect_command = match target_path {
                Some(target) => command(
                    ["btrfs", "qgroup", "show", "--raw", "-reF", target],
                    false,
                    "inspect Btrfs qgroup inventory before creation",
                ),
                None => command_with_readiness(
                    ["btrfs", "qgroup", "show", "--raw", "-reF", target],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["mounted Btrfs filesystem path"],
                    "inspect Btrfs qgroups after selecting the mounted filesystem path",
                ),
            };
            let create_command = match target_path {
                Some(target) => command_vec(
                    vec![
                        "btrfs".to_string(),
                        "qgroup".to_string(),
                        "create".to_string(),
                        qgroup_id.to_string(),
                        target.to_string(),
                    ],
                    true,
                    "create the reviewed Btrfs qgroup",
                ),
                None => command_vec_with_readiness(
                    vec![
                        "btrfs".to_string(),
                        "qgroup".to_string(),
                        "create".to_string(),
                        qgroup_id.to_string(),
                        target.to_string(),
                    ],
                    true,
                    CommandReadiness::NeedsDomainImplementation,
                    ["mounted Btrfs filesystem path"],
                    "create the Btrfs qgroup after selecting the mounted filesystem path",
                ),
            };
            (
                vec![inspect_command, create_command],
                vec![
                    "verify qgroup quota accounting is enabled on the filesystem".to_string(),
                    "select the qgroup id intentionally to avoid hierarchy collisions".to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("btrfsQgroups") => {
            let qgroup_id = action.context.name.as_deref().unwrap_or("<qgroupid>");
            let target_path = btrfs_qgroup_target_path(action.context.target.as_deref(), qgroup_id);
            let target = target_path.unwrap_or("<btrfs-filesystem-path>");
            let show_command = match target_path {
                Some(target) => command(
                    ["btrfs", "qgroup", "show", "--raw", "-reF", target],
                    false,
                    "refresh Btrfs qgroup hierarchy, limits, and usage",
                ),
                None => command_with_readiness(
                    ["btrfs", "qgroup", "show", "--raw", "-reF", target],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["mounted Btrfs filesystem path"],
                    "refresh Btrfs qgroups after selecting the mounted filesystem path",
                ),
            };
            let inspect_command = match target_path {
                Some(target) => command(
                    ["disk-nix", "inspect", target],
                    false,
                    "inspect modeled Btrfs qgroup graph relationships after refresh",
                ),
                None => command_with_readiness(
                    ["disk-nix", "inspect", target],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["mounted Btrfs filesystem path"],
                    "inspect modeled Btrfs qgroup relationships after selecting the mounted filesystem path",
                ),
            };
            (
                vec![show_command, inspect_command],
                vec![
                    format!("review qgroup {qgroup_id} usage before limit or removal changes"),
                    "qgroup rescan does not change quota enforcement or delete policy".to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("nvmeNamespaces") => {
            let controller = nvme_controller_target(action);
            let desired_size = action.context.desired_size.as_deref();
            let namespace_id = action.context.namespace_id.as_deref();
            let controllers = action.context.controllers.as_deref();
            (
                vec![
                    nvme_list_namespaces_command(
                        controller,
                        "inspect NVMe namespace inventory before creation",
                    ),
                    nvme_create_namespace_command(controller, desired_size),
                    nvme_attach_namespace_command(controller, namespace_id, controllers),
                    nvme_namespace_rescan_command(controller),
                ],
                vec![
                    "nvme create-ns returns the namespace id; declare namespaceId before attach can be executable"
                        .to_string(),
                    "verify controller and namespace capacity before exposing consumers".to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("pools") => {
            let target = target.unwrap_or("<zfs-pool>");
            let device = action.context.device.as_deref();
            let devices = pool_create_devices(device, &action.context.devices);
            let mut commands = zfs_pool_preflight_commands(&devices);
            commands.push(zfs_pool_create_command(target, &devices));
            (
                commands,
                vec![
                    "verify every vdev device is empty or fully backed up before pool creation"
                        .to_string(),
                    "choose redundancy, ashift, feature, and autotrim policy before creating datasets"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("mdRaids") => {
            let target = md_array_target_path(action);
            (
                vec![
                    command(
                        ["cat", "/proc/mdstat"],
                        false,
                        "inspect existing MD RAID state before array creation",
                    ),
                    md_raid_create_command(
                        target,
                        action.context.level.as_deref(),
                        &action.context.devices,
                    ),
                ],
                vec![
                    "verify every member device is empty or fully backed up before array creation"
                        .to_string(),
                    "choose metadata, bitmap, and spare policy before creating production arrays"
                        .to_string(),
                    "monitor /proc/mdstat until initial sync completes".to_string(),
                ],
                true,
            )
        }
        Operation::Assemble if collection == Some("mdRaids") => {
            let target = md_array_target_path(action);
            (
                vec![
                    command(
                        ["cat", "/proc/mdstat"],
                        false,
                        "inspect existing MD RAID state before array assembly",
                    ),
                    md_raid_assemble_command(target, &action.context.devices),
                ],
                vec![
                    "verify member event counts and array UUID before assembly".to_string(),
                    "activate filesystems and mappings only after mdadm reports expected health"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Stop if collection == Some("mdRaids") => {
            let target = md_array_target_path(action);
            (
                vec![
                    md_raid_detail_command(target, "inspect MD RAID array before stopping"),
                    md_raid_stop_command(target),
                ],
                vec![
                    "unmount filesystems and deactivate mappings before stopping the array"
                        .to_string(),
                    "preserve member devices for later mdadm assemble".to_string(),
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
                    zvol_create_command(target, desired_size, &action.context.property_assignments),
                ],
                vec![
                    "decide sparse versus reserved allocation before creation".to_string(),
                    "expose the zvol to guests or LUN exports only after verification".to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("datasets") => {
            let target = target.unwrap_or("<zfs-dataset>");
            (
                vec![
                    command(
                        ["zpool", "list", "-H", "-p"],
                        false,
                        "inspect ZFS pool free space before creating the dataset",
                    ),
                    zfs_dataset_create_command(target, &action.context.property_assignments),
                ],
                vec![
                    "review inherited mountpoint, quota, reservation, and encryption properties"
                        .to_string(),
                    "set required properties before exposing the dataset to consumers".to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("datasets") => {
            let target = target.unwrap_or("<zfs-dataset>");
            (
                vec![
                    command(
                        ["zfs", "list", "-H", "-p", "-t", "filesystem", target],
                        false,
                        "refresh ZFS dataset inventory, mountpoint, and usage",
                    ),
                    command(
                        [
                            "zfs",
                            "get",
                            "-H",
                            "-p",
                            "-o",
                            "property,value,source",
                            "all",
                            target,
                        ],
                        false,
                        "refresh ZFS dataset property sources",
                    ),
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect modeled ZFS dataset relationships after refresh",
                    ),
                ],
                vec![
                    "dataset rescan does not change mountpoints, quotas, or reservations"
                        .to_string(),
                    "use property updates only after reviewing inherited policy".to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("zvols") => {
            let target = target.unwrap_or("<zvol>");
            (
                vec![
                    command(
                        ["zfs", "list", "-H", "-p", "-t", "volume", target],
                        false,
                        "refresh zvol inventory, volsize, and usage",
                    ),
                    command(
                        [
                            "zfs",
                            "get",
                            "-H",
                            "-p",
                            "-o",
                            "property,value,source",
                            "all",
                            target,
                        ],
                        false,
                        "refresh zvol property sources",
                    ),
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect modeled zvol block relationships after refresh",
                    ),
                ],
                vec![
                    "zvol rescan does not change volsize, reservations, or consumers".to_string(),
                    "use grow only after reviewing pool capacity and downstream consumers"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("vdoVolumes") => {
            let target = target.unwrap_or("<vdo-volume>");
            (
                vec![
                    command(
                        ["vdo", "status", "--name", target],
                        false,
                        "refresh VDO volume status and configuration",
                    ),
                    command(
                        ["vdostats", "--human-readable", target],
                        false,
                        "refresh VDO runtime capacity, utilization, and savings counters",
                    ),
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect VDO graph node and backing relationships after status refresh",
                    ),
                ],
                vec![
                    "use grow when logical or physical capacity must change".to_string(),
                    "use start or stop only when activation state must change".to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("volumes") => {
            let target = target.unwrap_or("<logical-volume>");
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    command(
                        ["vgs", "--reportformat", "json"],
                        false,
                        "inspect volume group free space before creating the logical volume",
                    ),
                    lvm_logical_volume_create_command(target, desired_size),
                ],
                vec![
                    "verify the target volume group has enough free extents".to_string(),
                    "create filesystems, LUKS mappings, or exports only after the LV appears"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("vdoVolumes") => {
            let target = target.unwrap_or("<vdo-volume>");
            let device = action.context.device.as_deref();
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    vdo_backing_inspect_command(device),
                    vdo_create_command(target, device, desired_size),
                ],
                vec![
                    "verify the backing device has no signatures or data that must be preserved"
                        .to_string(),
                    "select logical size, deduplication, and compression policy before exposing the VDO device"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            let device = action.context.device.as_deref();
            (
                vec![
                    command(
                        ["pvs", "--reportformat", "json"],
                        false,
                        "inspect physical volumes before creating the volume group",
                    ),
                    lvm_volume_group_create_command(target, device),
                ],
                vec![
                    "verify the physical volume path is stable and intentionally selected"
                        .to_string(),
                    "create logical volumes only after the VG appears and free extents are reviewed"
                        .to_string(),
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
        Operation::Create | Operation::Export if collection == Some("exports") => {
            let target = export_target_path(action);
            (
                vec![nfs_export_create_command(
                    target,
                    action.context.client.as_deref(),
                    action.context.options.as_deref(),
                )],
                vec![
                    "verify the local export path exists and has intended ownership".to_string(),
                    "prefer restrictive client selectors and read-only options before write access"
                        .to_string(),
                    "persist long-lived exports declaratively through NixOS configuration"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("exports") => {
            let target = export_target_path(action);
            let inspect_target = target.unwrap_or("<export-path>");
            let inspect_command = match target {
                Some(target) => command(
                    ["disk-nix", "inspect", target],
                    false,
                    "inspect modeled NFS export relationships after refresh",
                ),
                None => command_with_readiness(
                    ["disk-nix", "inspect", inspect_target],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["NFS export path"],
                    "inspect modeled NFS export relationships after selecting the export path",
                ),
            };
            (
                vec![
                    command(
                        ["exportfs", "-v"],
                        false,
                        "refresh NFS export inventory and client options",
                    ),
                    inspect_command,
                ],
                vec![
                    "export rescan does not reload exports or change client access".to_string(),
                    "use option updates only after reviewing active client visibility".to_string(),
                ],
                true,
            )
        }
        Operation::Create | Operation::Mount if collection == Some("nfs.mounts") => {
            let mountpoint = nfs_mount_target_path(action);
            (
                vec![nfs_mount_create_command(
                    action.context.device.as_deref(),
                    mountpoint,
                    action.context.fs_type.as_deref(),
                    action.context.options.as_deref(),
                )],
                vec![
                    "verify the NFS server, export permissions, and network path before mounting"
                        .to_string(),
                    "persist long-lived mounts through the NixOS fileSystems entry".to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("nfs.mounts") => {
            let mountpoint = nfs_mount_target_path(action);
            let inspect_target = mountpoint.unwrap_or("<mountpoint>");
            let inspect_command = match mountpoint {
                Some(mountpoint) => command(
                    ["disk-nix", "inspect", mountpoint],
                    false,
                    "inspect modeled NFS mount relationships after refresh",
                ),
                None => command_with_readiness(
                    ["disk-nix", "inspect", inspect_target],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["mountpoint path"],
                    "inspect modeled NFS mount relationships after selecting the mountpoint",
                ),
            };
            (
                vec![
                    nfs_mount_findmnt_command(mountpoint),
                    nfs_mount_stats_command(mountpoint),
                    inspect_command,
                ],
                vec![
                    "mount rescan does not remount, unmount, or change remote data".to_string(),
                    "use remount only after reviewing active services and desired options"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Remount if collection == Some("nfs.mounts") => {
            let mountpoint = nfs_mount_target_path(action);
            (
                vec![
                    nfs_mount_findmnt_command(mountpoint),
                    nfs_mount_remount_command(mountpoint, action.context.options.as_deref()),
                ],
                vec![
                    "review active services before changing NFS mount options".to_string(),
                    "persist the final options through the NixOS fileSystems entry".to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("disks") => {
            let disk = disk_target_path(action);
            let label = action.context.partition_type.as_deref().unwrap_or("gpt");
            (
                vec![
                    disk_nix_inspect_command(
                        disk,
                        "<disk>",
                        "disk path",
                        "inspect disk identity, signatures, and existing consumers before initialization",
                    ),
                    disk_create_label_command(disk, label),
                    partition_probe_command(disk),
                    disk_parted_machine_list_command(
                        disk,
                        "verify the disk reports the reviewed partition table label",
                    ),
                ],
                vec![
                    "creating a partition table can hide existing signatures and partitions"
                        .to_string(),
                    "prefer importing or preserving existing metadata when the disk is not empty"
                        .to_string(),
                    "create partitions only after the initialized disk is re-probed".to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("partitions") => {
            let partition_target = partition_target_path(action);
            let disk = action.context.device.as_deref();
            let start = action.context.start.as_deref();
            let end = action.context.end.as_deref();
            let partition_type = action.context.partition_type.as_deref();
            (
                vec![
                    disk_nix_inspect_command(
                        disk,
                        "<disk>",
                        "disk path",
                        "inspect disk identity and existing partition table before creation",
                    ),
                    partition_create_command(disk, partition_type, start, end),
                    partition_probe_command(disk),
                    partition_table_reread_command(disk),
                    disk_nix_inspect_command(
                        partition_target,
                        "<partition>",
                        "partition path",
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
            let target = swap_target_path(action);
            (
                vec![
                    disk_nix_inspect_command(
                        target,
                        "<swap>",
                        "swap target path",
                        "inspect target before creating a swap signature",
                    ),
                    swap_command(
                        "swapoff",
                        target,
                        "disable active swap before replacing its signature",
                    ),
                    swap_command("mkswap", target, "create a swap signature on the target"),
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
            let device = action.context.device.as_deref();
            (
                vec![
                    luks_backing_inspect_command(
                        device,
                        "inspect target before creating a LUKS container",
                    ),
                    luks_format_command(device),
                    luks_open_command(
                        device,
                        mapper,
                        "open the newly created LUKS container with the desired mapper name",
                    ),
                ],
                vec![
                    "verify header backups and key enrollment policy before formatting".to_string(),
                    "create filesystems or LVM layers only after the mapper is open".to_string(),
                ],
                true,
            )
        }
        Operation::Create | Operation::Open if collection == Some("luks.devices") => {
            let mapper = target.unwrap_or("<mapper>");
            let device = action.context.device.as_deref();
            (
                vec![
                    luks_backing_inspect_command(
                        device,
                        "inspect existing LUKS container before opening",
                    ),
                    luks_is_luks_command(device),
                    luks_open_command(
                        device,
                        mapper,
                        "open the existing LUKS container with the desired mapper name",
                    ),
                ],
                vec![
                    "verify the backing device identity before entering credentials".to_string(),
                    "keep formatting as a separate explicit action when data must be replaced"
                        .to_string(),
                    "create filesystems or LVM layers only after the mapper is open".to_string(),
                ],
                true,
            )
        }
        Operation::Create | Operation::AddKey if collection == Some("luksKeyslots") => {
            let device = luks_keyslot_device(action);
            let key_slot = luks_keyslot_id(action);
            let new_key_file = luks_new_key_file(action);
            (
                vec![
                    luks_dump_command(device, "inspect LUKS header before adding keyslot"),
                    luks_add_key_command(device, key_slot, new_key_file),
                ],
                vec![
                    "back up the LUKS header before enrolling new key material".to_string(),
                    "test the new keyslot before removing any old recovery key".to_string(),
                ],
                true,
            )
        }
        Operation::Create | Operation::ImportToken if collection == Some("luksTokens") => {
            let device = luks_token_device(action);
            let token_id = luks_token_id(action);
            let token_file = luks_token_file(action);
            (
                vec![
                    luks_dump_command(device, "inspect LUKS header before importing token"),
                    luks_token_import_command(device, token_id, token_file),
                ],
                vec![
                    "back up the LUKS header before importing token metadata".to_string(),
                    "test the token unlock path before removing any older token".to_string(),
                ],
                true,
            )
        }
        Operation::Destroy | Operation::Close if collection == Some("luks.devices") => {
            let mapper = target.unwrap_or("<mapper>");
            (
                vec![
                    command(
                        ["cryptsetup", "status", mapper],
                        false,
                        "inspect open LUKS mapper before close",
                    ),
                    command(
                        ["cryptsetup", "close", mapper],
                        true,
                        "close the reviewed LUKS mapper without erasing backing data",
                    ),
                ],
                vec![
                    "unmount filesystems and deactivate LVM volumes before closing the mapper"
                        .to_string(),
                    "verify no services still depend on the mapper path".to_string(),
                    "keep the backing LUKS header intact for later reopen".to_string(),
                ],
                true,
            )
        }
        Operation::Destroy | Operation::RemoveKey if collection == Some("luksKeyslots") => {
            let device = luks_keyslot_device(action);
            let key_slot = luks_keyslot_id(action);
            (
                vec![
                    luks_dump_command(device, "inspect LUKS keyslots before removal"),
                    luks_kill_slot_command(device, key_slot),
                ],
                vec![
                    "verify another key, token, or recovery passphrase unlocks the device first"
                        .to_string(),
                    "keep a LUKS header backup until post-removal unlock testing passes"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy | Operation::RemoveToken if collection == Some("luksTokens") => {
            let device = luks_token_device(action);
            let token_id = luks_token_id(action);
            (
                vec![
                    luks_dump_command(device, "inspect LUKS tokens before removal"),
                    luks_token_remove_command(device, token_id),
                ],
                vec![
                    "verify another keyslot, token, or recovery passphrase unlocks the device first"
                        .to_string(),
                    "keep a LUKS header backup until post-removal unlock testing passes".to_string(),
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
            vec![
                "creation commands require target-kind-specific arguments from the desired spec"
                    .to_string(),
            ],
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
        Operation::Rename if collection == Some("btrfsSubvolumes") => {
            let target = target.unwrap_or("<btrfs-subvolume-path>");
            let rename_to = action
                .context
                .rename_to
                .as_deref()
                .unwrap_or("<new-btrfs-subvolume-path>");
            (
                vec![
                    command(
                        ["btrfs", "subvolume", "show", target],
                        false,
                        "inspect Btrfs subvolume before rename",
                    ),
                    command(
                        ["mv", "--", target, rename_to],
                        true,
                        "rename the reviewed Btrfs subvolume path",
                    ),
                ],
                vec![
                    "update mounts, send/receive jobs, qgroups, and snapshots after rename"
                        .to_string(),
                    "validate consumers on the renamed subvolume before deleting the old path"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("btrfsQgroups") => {
            let qgroup_id = action.context.name.as_deref().unwrap_or("<qgroupid>");
            let target_path = btrfs_qgroup_target_path(action.context.target.as_deref(), qgroup_id);
            let target = target_path.unwrap_or("<btrfs-filesystem-path>");
            let inspect_command = match target_path {
                Some(target) => command(
                    ["btrfs", "qgroup", "show", "--raw", "-reF", target],
                    false,
                    "inspect Btrfs qgroup inventory before destruction",
                ),
                None => command_with_readiness(
                    ["btrfs", "qgroup", "show", "--raw", "-reF", target],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["mounted Btrfs filesystem path"],
                    "inspect Btrfs qgroups after selecting the mounted filesystem path",
                ),
            };
            let destroy_command = match target_path {
                Some(target) => command_vec(
                    vec![
                        "btrfs".to_string(),
                        "qgroup".to_string(),
                        "destroy".to_string(),
                        qgroup_id.to_string(),
                        target.to_string(),
                    ],
                    true,
                    "destroy the reviewed Btrfs qgroup",
                ),
                None => command_vec_with_readiness(
                    vec![
                        "btrfs".to_string(),
                        "qgroup".to_string(),
                        "destroy".to_string(),
                        qgroup_id.to_string(),
                        target.to_string(),
                    ],
                    true,
                    CommandReadiness::NeedsDomainImplementation,
                    ["mounted Btrfs filesystem path"],
                    "destroy the Btrfs qgroup after selecting the mounted filesystem path",
                ),
            };
            (
                vec![inspect_command, destroy_command],
                vec![
                    "verify no subvolume still depends on the qgroup limit".to_string(),
                    "preserve qgroup accounting policy elsewhere before deleting the qgroup"
                        .to_string(),
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
                    "detach LUN, VM, or filesystem consumers before destroying the zvol"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("pools") => {
            let target = target.unwrap_or("<zfs-pool>");
            (
                vec![
                    command(
                        ["zpool", "status", "-P", target],
                        false,
                        "inspect pool health and dependent vdevs before destruction",
                    ),
                    command(
                        ["zpool", "destroy", target],
                        true,
                        "destroy the reviewed ZFS pool after datasets and consumers are migrated",
                    ),
                ],
                vec![
                    "take recursive snapshots or verified backups before destroying the pool"
                        .to_string(),
                    "export the pool instead of destroying it when moving it to another host"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Import if collection == Some("pools") => {
            let target = target.unwrap_or("<zfs-pool>");
            (
                vec![
                    command(
                        ["zpool", "import"],
                        false,
                        "inspect importable ZFS pools before import",
                    ),
                    zfs_pool_import_command(target, action.context.read_only.unwrap_or(false)),
                ],
                vec![
                    "verify the pool identity, hostid, cachefile, mountpoints, and encryption keys before import"
                        .to_string(),
                    "use readOnly=true first when validating a moved or recovered pool".to_string(),
                ],
                true,
            )
        }
        Operation::Export if collection == Some("pools") => {
            let target = target.unwrap_or("<zfs-pool>");
            (
                vec![
                    command(
                        ["zpool", "status", "-P", target],
                        false,
                        "inspect pool health and active consumers before export",
                    ),
                    command(
                        ["zpool", "export", target],
                        true,
                        "export the reviewed ZFS pool without deleting data",
                    ),
                ],
                vec![
                    "stop mount, share, LUN, VM, and service consumers before export".to_string(),
                    "export instead of destroying a pool that will be moved or recovered elsewhere"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("datasets") => {
            let target = target.unwrap_or("<zfs-dataset>");
            (
                vec![
                    command(
                        ["zfs", "list", "-H", "-p", "-r", target],
                        false,
                        "inspect dataset descendants before destruction",
                    ),
                    command(
                        ["zfs", "destroy", target],
                        true,
                        "destroy the reviewed ZFS dataset after snapshots and consumers are handled",
                    ),
                ],
                vec![
                    "take a recursive snapshot or clone before destruction when rollback is required"
                        .to_string(),
                    "unmount dependents and review child datasets before destroying the dataset"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rename if collection == Some("datasets") || collection == Some("zvols") => {
            let target = target.unwrap_or("<zfs-dataset>");
            let rename_to = action
                .context
                .rename_to
                .as_deref()
                .unwrap_or("<new-zfs-name>");
            (
                vec![
                    command(
                        ["zfs", "list", "-H", "-p", target],
                        false,
                        "inspect ZFS object before rename",
                    ),
                    command(
                        ["zfs", "rename", target, rename_to],
                        true,
                        "rename the reviewed ZFS dataset or zvol",
                    ),
                ],
                vec![
                    "update mountpoints, shares, LUN mappings, and dependent services to the new name"
                        .to_string(),
                    "validate consumers on the renamed object before destroying any old path"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Promote if collection == Some("datasets") || collection == Some("zvols") => {
            let target = target.unwrap_or("<zfs-clone>");
            (
                vec![
                    command(
                        ["zfs", "get", "-H", "-o", "value", "origin", target],
                        false,
                        "inspect ZFS clone origin before promotion",
                    ),
                    command(
                        ["zfs", "promote", target],
                        true,
                        "promote the reviewed ZFS clone",
                    ),
                ],
                vec![
                    "promotion changes clone dependency ownership; review dependent snapshots first"
                        .to_string(),
                    "validate consumers on the promoted clone before destroying or renaming the origin"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("snapshots") => {
            let snapshot = action.context.name.as_deref().unwrap_or("<snapshot>");
            let source = action
                .context
                .target
                .as_deref()
                .unwrap_or("<snapshot-source>");
            if is_zfs_snapshot_name(snapshot) {
                (
                    vec![
                        command(
                            ["zfs", "list", "-t", "snapshot", "-H", "-p", snapshot],
                            false,
                            "inspect ZFS snapshot before destruction",
                        ),
                        command(
                            ["zfs", "destroy", snapshot],
                            true,
                            "destroy the reviewed ZFS snapshot recovery point",
                        ),
                    ],
                    vec![
                        "verify the snapshot is no longer needed as a recovery point".to_string(),
                        "hold, rename, clone, or replicate the snapshot before destruction when retention is uncertain"
                            .to_string(),
                    ],
                    true,
                )
            } else if is_btrfs_snapshot_pair(source, snapshot) {
                (
                    vec![
                        command(
                            ["btrfs", "subvolume", "show", snapshot],
                            false,
                            "inspect Btrfs snapshot subvolume before deletion",
                        ),
                        command(
                            ["btrfs", "subvolume", "delete", snapshot],
                            true,
                            "delete the reviewed Btrfs snapshot subvolume",
                        ),
                    ],
                    vec![
                        "verify the snapshot is no longer needed as a recovery point".to_string(),
                        "keep or clone the read-only snapshot before deletion when retention is uncertain"
                            .to_string(),
                    ],
                    true,
                )
            } else {
                (
                    vec![command_with_readiness(
                        ["<snapshot-destroy-tool>", source, snapshot],
                        true,
                        CommandReadiness::NeedsDomainImplementation,
                        ["snapshot destroy tool"],
                        "destroy the snapshot with zfs, btrfs, lvm, or the target-specific tool",
                    )],
                    vec![
                        "snapshot destruction command is only rendered for unambiguous ZFS names or Btrfs absolute paths"
                            .to_string(),
                    ],
                    true,
                )
            }
        }
        Operation::Rename if collection == Some("snapshots") => {
            let snapshot = action.context.name.as_deref().unwrap_or("<snapshot>");
            let rename_to = action
                .context
                .rename_to
                .as_deref()
                .unwrap_or("<new-snapshot-name>");
            if is_zfs_snapshot_name(snapshot) {
                (
                    vec![
                        command(
                            ["zfs", "list", "-t", "snapshot", "-H", "-p", snapshot],
                            false,
                            "inspect ZFS snapshot before rename",
                        ),
                        command(
                            ["zfs", "rename", snapshot, rename_to],
                            true,
                            "rename the reviewed ZFS snapshot recovery point",
                        ),
                    ],
                    vec![
                        "update retention, replication, and rollback references to the new snapshot name"
                            .to_string(),
                    ],
                    true,
                )
            } else if snapshot.starts_with('/') {
                (
                    vec![
                        command(
                            ["btrfs", "subvolume", "show", snapshot],
                            false,
                            "inspect Btrfs snapshot subvolume before rename",
                        ),
                        command(
                            ["mv", "--", snapshot, rename_to],
                            true,
                            "rename the reviewed Btrfs snapshot subvolume path",
                        ),
                    ],
                    vec![
                        "update retention and restore references to the renamed snapshot path"
                            .to_string(),
                    ],
                    true,
                )
            } else {
                (
                    Vec::new(),
                    vec![
                        "snapshot rename command is only rendered for unambiguous ZFS snapshot names or Btrfs absolute paths"
                            .to_string(),
                    ],
                    true,
                )
            }
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
        Operation::Destroy if collection == Some("volumes") => {
            let target = lvm_volume_target_path(target);
            (
                vec![
                    lvm_lvs_report_command(target, None, "inspect logical volume before removal"),
                    lvm_lvremove_command(
                        target,
                        "<logical-volume>",
                        "target in volume-group/logical-volume form",
                        "remove the reviewed logical volume after backups and consumers are verified",
                    ),
                ],
                vec![
                    "snapshot or migrate data before removing the logical volume".to_string(),
                    "unmount filesystems and deactivate dependent mappings before lvremove"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rename if collection == Some("volumes") || collection == Some("thinPools") => {
            let target = lvm_volume_target_path(target);
            let rename_to = action.context.rename_to.as_deref();
            (
                vec![
                    lvm_lvs_report_command(target, None, "inspect logical volume before rename"),
                    lvm_lvrename_command(
                        target,
                        rename_to,
                        "<logical-volume>",
                        "target in volume-group/logical-volume form",
                        "new logical volume name or path",
                        "rename the reviewed logical volume",
                    ),
                ],
                vec![
                    "update filesystems, crypttab, mounts, LUN exports, and services after rename"
                        .to_string(),
                    "keep the old declaration out of destructive mode until consumers are validated"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Activate | Operation::Deactivate
            if collection == Some("volumes")
                || collection == Some("thinPools")
                || collection == Some("lvmSnapshots") =>
        {
            let target = lvm_volume_target_path(target);
            let (flag, verb, placeholder, input) = match collection {
                Some("thinPools") => (
                    "y",
                    "activate",
                    "<thin-pool>",
                    "target in volume-group/thin-pool form",
                ),
                Some("lvmSnapshots") => (
                    "y",
                    "activate",
                    "<lvm-snapshot>",
                    "target in volume-group/snapshot form",
                ),
                _ => (
                    "y",
                    "activate",
                    "<logical-volume>",
                    "target in volume-group/logical-volume form",
                ),
            };
            let (flag, verb) = if action.operation == Operation::Deactivate {
                ("n", "deactivate")
            } else {
                (flag, verb)
            };
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        None,
                        "inspect logical volume before activation change",
                    ),
                    lvm_lvchange_activate_command(target, flag, placeholder, input),
                ],
                vec![
                    format!(
                        "{verb} only after filesystem, mapping, mount, and service consumers are reviewed"
                    ),
                    "activation state changes do not create or delete LV data".to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("lvmSnapshots") => {
            let target = lvm_volume_target_path(target);
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        Some("lv_name,origin,lv_attr,data_percent,metadata_percent,lv_size"),
                        "refresh LVM snapshot origin, attributes, and COW usage",
                    ),
                    command(
                        ["disk-nix", "inspect", target.unwrap_or("<lvm-snapshot>")],
                        false,
                        "inspect modeled LVM snapshot graph relationships after status refresh",
                    ),
                ],
                vec![
                    "use rollback only after reviewing origin and snapshot state".to_string(),
                    "activate the snapshot for recovery inspection before destructive removal"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("thinPools") => {
            let target = lvm_volume_target_path(target);
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        Some("lv_name,lv_attr,data_percent,metadata_percent"),
                        "inspect thin pool before removal",
                    ),
                    lvm_lvremove_command(
                        target,
                        "<thin-pool>",
                        "target in volume-group/thin-pool form",
                        "remove the reviewed thin pool after thin volumes and consumers are migrated",
                    ),
                ],
                vec![
                    "migrate or remove thin volumes before removing the thin pool".to_string(),
                    "unmount filesystems and deactivate mappings that depend on thin volumes"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            (
                vec![
                    command(
                        ["vgs", "--reportformat", "json", target],
                        false,
                        "inspect volume group before removal",
                    ),
                    command(
                        ["vgremove", "--yes", target],
                        true,
                        "remove the reviewed LVM volume group after all consumers are migrated",
                    ),
                ],
                vec![
                    "remove or migrate logical volumes before removing the volume group"
                        .to_string(),
                    "verify no filesystems, mappings, or services still reference the VG"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Import if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            (
                vec![
                    command(
                        ["pvs", "--reportformat", "json"],
                        false,
                        "inspect physical volumes and exported VG metadata before import",
                    ),
                    command(
                        ["vgimport", target],
                        true,
                        "import the reviewed LVM volume group without recreating it",
                    ),
                ],
                vec![
                    "verify PV identities, VG UUID, and metadata backups before vgimport"
                        .to_string(),
                    "activate logical volumes and mount consumers only after the VG is verified"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Export if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            (
                vec![
                    command(
                        ["vgs", "--reportformat", "json", target],
                        false,
                        "inspect volume group before export",
                    ),
                    command(
                        ["vgexport", target],
                        true,
                        "export the reviewed LVM volume group without deleting data",
                    ),
                ],
                vec![
                    "deactivate logical volumes and stop mount, mapping, and service consumers before vgexport"
                        .to_string(),
                    "export instead of removing a VG that will be moved to another host"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rename if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            let rename_to = action
                .context
                .rename_to
                .as_deref()
                .unwrap_or("<new-volume-group>");
            (
                vec![
                    command(
                        ["vgs", "--reportformat", "json", target],
                        false,
                        "inspect volume group before rename",
                    ),
                    command(
                        ["vgrename", target, rename_to],
                        true,
                        "rename the reviewed volume group",
                    ),
                ],
                vec![
                    "update every LV path, initrd reference, mount, and service before reboot"
                        .to_string(),
                    "validate boot and activation with the renamed volume group before cleanup"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Activate | Operation::Deactivate if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            let (flag, verb) = if action.operation == Operation::Deactivate {
                ("n", "deactivate")
            } else {
                ("y", "activate")
            };
            (
                vec![
                    command(
                        ["vgs", "--reportformat", "json", target],
                        false,
                        "inspect volume group before activation change",
                    ),
                    command(
                        ["vgchange", "--activate", flag, target],
                        true,
                        if flag == "y" {
                            "activate the reviewed LVM volume group"
                        } else {
                            "deactivate the reviewed LVM volume group without deleting data"
                        },
                    ),
                ],
                vec![
                    format!(
                        "{verb} the VG only after PV membership and dependent consumers are reviewed"
                    ),
                    "volume group activation changes do not create or remove VG metadata"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("physicalVolumes") => {
            let target = lvm_physical_volume_target(action);
            (
                vec![
                    lvm_physical_volume_inspect_command(target),
                    lvm_physical_volume_remove_command(target),
                ],
                vec![
                    "run pvmove and vgreduce before pvremove when the PV is in a VG".to_string(),
                    "keep the device available for recovery until backups are verified".to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("lvmCaches") => {
            let target = lvm_volume_target_path(target);
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        Some("lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent"),
                        "inspect LVM cache dirty state before removal",
                    ),
                    lvm_cache_uncache_command(target),
                ],
                vec![
                    "switch writeback caches to writethrough before removing cache state"
                        .to_string(),
                    "verify the origin LV after lvconvert --uncache before removing cache media"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("vdoVolumes") => {
            let target = target.unwrap_or("<vdo-volume>");
            (
                vec![
                    command(
                        ["vdo", "status", "--name", target],
                        false,
                        "inspect VDO volume before removal",
                    ),
                    command(
                        ["vdo", "remove", "--name", target],
                        true,
                        "remove the reviewed VDO volume after consumers are migrated",
                    ),
                ],
                vec![
                    "migrate data away from the VDO device before removal".to_string(),
                    "unmount filesystems and deactivate mappings that reference the VDO device"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Stop if collection == Some("vdoVolumes") => {
            let target = target.unwrap_or("<vdo-volume>");
            (
                vec![
                    command(
                        ["vdo", "status", "--name", target],
                        false,
                        "inspect VDO volume before stop",
                    ),
                    command(
                        ["vdo", "stop", "--name", target],
                        true,
                        "stop the existing VDO volume after consumers are inactive",
                    ),
                ],
                vec![
                    "unmount filesystems and deactivate mappings that reference the VDO device"
                        .to_string(),
                    "prefer stop over remove when preserving VDO metadata for later restart"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("nvmeNamespaces") => {
            let controller = nvme_controller_target(action);
            let namespace_id = action.context.namespace_id.as_deref();
            let controllers = action.context.controllers.as_deref();
            (
                vec![
                    nvme_list_namespaces_command(
                        controller,
                        "inspect NVMe namespace inventory before deletion",
                    ),
                    nvme_detach_namespace_command(controller, namespace_id, controllers),
                    nvme_delete_namespace_command(controller, namespace_id),
                    nvme_namespace_rescan_command(controller),
                ],
                vec![
                    "detach namespace consumers and migrate data before delete-ns".to_string(),
                    "prefer detach without delete when target-side namespace data must remain"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("loopDevices") => {
            let target = loop_device_target_path(action);
            (
                vec![
                    loop_device_list_command(
                        target,
                        "inspect loop device and backing file before detach",
                    ),
                    loop_device_detach_command(target),
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
        Operation::Rollback if collection == Some("snapshots") => {
            let target = target.unwrap_or("<snapshot>");
            let snapshot = action.context.name.as_deref().unwrap_or(target);
            if is_zfs_snapshot_name(snapshot) {
                (
                    vec![
                        command(
                            ["zfs", "list", "-t", "snapshot", "-H", "-p", snapshot],
                            false,
                            "inspect ZFS snapshot before rollback",
                        ),
                        zfs_snapshot_rollback_command(
                            snapshot,
                            action.context.recursive_rollback.unwrap_or(false),
                        ),
                    ],
                    vec![
                        "take a fresh snapshot of the current dataset before rollback".to_string(),
                        if action.context.recursive_rollback == Some(true) {
                            "recursive rollback destroys newer snapshots in the dataset lineage; review clones and dependent retention first"
                                .to_string()
                        } else {
                            "review newer snapshots and clones before considering zfs rollback -r or -R"
                                .to_string()
                        },
                    ],
                    true,
                )
            } else {
                (
                    Vec::new(),
                    vec![
                        "snapshot rollback command is only rendered for unambiguous ZFS snapshot names"
                            .to_string(),
                    ],
                    true,
                )
            }
        }
        Operation::Clone if collection == Some("snapshots") => {
            let target = target.unwrap_or("<clone-dataset>");
            let snapshot = action.context.name.as_deref().unwrap_or("<snapshot>");
            if is_zfs_snapshot_name(snapshot) {
                (
                    vec![
                        command(
                            ["zfs", "list", "-t", "snapshot", "-H", "-p", snapshot],
                            false,
                            "inspect ZFS snapshot before clone",
                        ),
                        command(
                            ["zfs", "clone", snapshot, target],
                            true,
                            "clone the reviewed ZFS snapshot to a writable dataset",
                        ),
                    ],
                    vec![
                        "use the clone for inspection, migration, or rollback rehearsal"
                            .to_string(),
                        "destroy temporary clones after validation to release snapshot dependencies"
                            .to_string(),
                    ],
                    true,
                )
            } else {
                (
                    Vec::new(),
                    vec![
                        "snapshot clone command is only rendered for unambiguous ZFS snapshot names"
                            .to_string(),
                    ],
                    true,
                )
            }
        }
        Operation::RemoveDevice if collection == Some("pools") => {
            let target = target.unwrap_or("<zfs-pool>");
            let device = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "remove-device"));
            (
                vec![
                    command(
                        ["zpool", "status", "-P", target],
                        false,
                        "inspect ZFS pool layout and health before device removal",
                    ),
                    zpool_remove_device_command(target, device),
                ],
                vec![
                    "verify the pool supports device removal for the selected vdev class"
                        .to_string(),
                    "monitor evacuation and keep replacement capacity available until verification passes"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::RemoveDevice if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            let device = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "remove-device"));
            (
                vec![
                    lvm_physical_volume_inspect_command(device),
                    lvm_physical_volume_move_command(device),
                    lvm_volume_group_reduce_command(target, device),
                ],
                vec![
                    "run pvmove or add replacement capacity before reducing a PV with allocated extents"
                        .to_string(),
                    "verify logical volumes and thin pools still have the intended redundancy and free space"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::RemoveDevice if collection == Some("mdRaids") => {
            let target = md_array_target_path(action);
            let device = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "remove-device"));
            (
                vec![
                    md_raid_detail_command(
                        target,
                        "inspect MD RAID redundancy before member removal",
                    ),
                    md_raid_fail_member_command(target, device),
                    md_raid_remove_member_command(target, device),
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
            let target = multipath_map_target(action);
            let path = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "remove-device"));
            (
                vec![
                    multipath_list_command(target, "inspect live multipath paths before deletion"),
                    multipath_delete_path_command(path),
                ],
                vec![
                    "remove a path only when alternate paths remain active".to_string(),
                    "verify the path belongs to the intended map WWID before deletion".to_string(),
                ],
                true,
            )
        }
        Operation::RemoveDevice if collection == Some("lvmCaches") => {
            let target = lvm_volume_target_path(target);
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        Some("lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent"),
                        "inspect LVM cache dirty state before detach",
                    ),
                    lvm_cache_uncache_command(target),
                ],
                vec![
                    "switch writeback caches to writethrough before detach".to_string(),
                    "wait for dirty data to drain before lvconvert --uncache".to_string(),
                ],
                true,
            )
        }
        Operation::RemoveDevice if collection == Some("filesystems") => {
            let target = target.unwrap_or("<btrfs-filesystem>");
            let device = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "remove-device"));
            (
                vec![
                    command(
                        ["btrfs", "filesystem", "usage", "-b", target],
                        false,
                        "inspect Btrfs allocation and free space before device removal",
                    ),
                    btrfs_remove_device_command(target, device),
                ],
                vec![
                    "remove a Btrfs device only when remaining data and metadata space are sufficient"
                        .to_string(),
                    "run or review balance progress until device evacuation completes".to_string(),
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
                    bcache_detach_command(target),
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
        Operation::Rescan if collection == Some("caches") => {
            let target = target.unwrap_or("<cache-device>");
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect modeled cache layer relationships after status refresh",
                    ),
                    bcache_sysfs_read_command(target, "state", "refresh bcache state"),
                    bcache_sysfs_read_command(target, "cache_mode", "refresh bcache cache mode"),
                    bcache_sysfs_read_command(target, "dirty_data", "refresh bcache dirty data"),
                ],
                vec![
                    "use add-device or remove-device when cache-set attachment must change"
                        .to_string(),
                    "verify dirty data before detach, replacement, or cache-mode changes"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy | Operation::Unexport if collection == Some("exports") => {
            let target = export_target_path(action);
            (
                vec![nfs_export_destroy_command(
                    target,
                    action.context.client.as_deref(),
                )],
                vec![
                    "drain or migrate clients before unexporting the path".to_string(),
                    "verify no active mounts still depend on the export after reload".to_string(),
                ],
                true,
            )
        }
        Operation::Destroy | Operation::Unmount if collection == Some("nfs.mounts") => {
            let mountpoint = nfs_mount_target_path(action);
            (
                vec![
                    nfs_mount_findmnt_command(mountpoint),
                    nfs_mount_destroy_command(mountpoint),
                ],
                vec![
                    "stop services and automount units that depend on the NFS mount before unmounting"
                        .to_string(),
                    "verify no open files, bind mounts, or user sessions still reference the mountpoint"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy | Operation::Detach
            if collection == Some("luns") || action.id.starts_with("luns:") =>
        {
            let target = target.unwrap_or("<lun>");
            let devices = lun_rescan_devices(action);
            let mut commands = vec![command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "inspect LUN consumers before detaching reviewed SCSI paths",
            )];
            if devices.is_empty() {
                commands.push(command_with_readiness(
                    ["<scsi-delete-device>", "<lun-path>"],
                    true,
                    CommandReadiness::NeedsDomainImplementation,
                    ["stable LUN device path"],
                    "detach a LUN path after declaring a stable by-path device",
                ));
            } else {
                for device in devices {
                    commands.push(scsi_device_delete_command(&device));
                }
            }
            commands.push(command(
                ["multipath", "-r"],
                true,
                "reload multipath maps after LUN path detach",
            ));
            commands.push(command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify detached LUN paths and remaining consumers",
            ));
            (
                commands,
                vec![
                    "unmount filesystems and deactivate dm, LVM, or multipath consumers before detach"
                        .to_string(),
                    "detach only reviewed stable paths; target-side LUN deletion remains an external storage-array action"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Format
        | Operation::Shrink
        | Operation::Check
        | Operation::Repair
        | Operation::Clone
        | Operation::Promote
        | Operation::Import
        | Operation::Export
        | Operation::Unexport
        | Operation::Attach
        | Operation::Detach
        | Operation::Activate
        | Operation::Deactivate
        | Operation::Assemble
        | Operation::Start
        | Operation::Stop
        | Operation::Login
        | Operation::Logout
        | Operation::Open
        | Operation::Close
        | Operation::Mount
        | Operation::Unmount
        | Operation::Remount
        | Operation::Rename
        | Operation::Rescan
        | Operation::AddKey
        | Operation::RemoveKey
        | Operation::ImportToken
        | Operation::RemoveToken
        | Operation::RemoveDevice
        | Operation::Rollback
        | Operation::Destroy => (
            Vec::new(),
            vec!["no domain-specific command plan is generated for this action yet".to_string()],
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
    device: Option<&str>,
    desired_size: Option<&str>,
) -> ExecutionCommand {
    match fs_type {
        "xfs" => command(
            ["xfs_growfs", target],
            true,
            "grow an already-mounted XFS filesystem",
        ),
        "ext2" | "ext3" | "ext4" => ext_filesystem_grow_command(target, device, desired_size),
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

fn filesystem_shrink_commands(
    fs_type: &str,
    target: &str,
    device: Option<&str>,
    desired_size: Option<&str>,
) -> Vec<ExecutionCommand> {
    let mut commands = vec![command(
        ["disk-nix", "inspect", target],
        false,
        "inspect filesystem usage, mount state, and consumers before shrinking",
    )];
    match fs_type {
        "btrfs" => {
            commands.push(command(
                ["btrfs", "filesystem", "usage", "-b", target],
                false,
                "inspect Btrfs allocation slack before shrinking",
            ));
            commands.push(btrfs_filesystem_shrink_command(target, desired_size));
        }
        "ext2" | "ext3" | "ext4" => {
            commands.push(command(
                [
                    "findmnt",
                    "--noheadings",
                    "--output",
                    "SOURCE,FSTYPE,SIZE,USED,AVAIL",
                    "--target",
                    target,
                ],
                false,
                "resolve the ext filesystem source device and capacity before offline shrink",
            ));
            commands.push(command(
                ["umount", target],
                true,
                "unmount the ext filesystem before fsck and shrink",
            ));
            commands.push(ext_filesystem_check_command(target, device));
            commands.push(ext_filesystem_shrink_command(target, device, desired_size));
        }
        "xfs" => {
            commands.push(command_with_readiness(
                ["<migrate-to-smaller-filesystem>", target],
                true,
                CommandReadiness::ManualOnly,
                ["replacement filesystem", "migration plan"],
                "XFS cannot shrink in place; create a smaller filesystem and migrate data",
            ));
        }
        _ => commands.push(command_with_readiness(
            ["<filesystem-shrink-tool>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem shrink tool", "filesystem source device"],
            "shrink with the filesystem-specific offline or migration workflow",
        )),
    }
    commands
}

fn btrfs_filesystem_shrink_command(target: &str, desired_size: Option<&str>) -> ExecutionCommand {
    match desired_size {
        Some(size) => command_vec(
            vec!["btrfs", "filesystem", "resize", size, target],
            true,
            "shrink the Btrfs filesystem to the reviewed size",
        ),
        None => command_with_readiness(
            ["btrfs", "filesystem", "resize", "<size>", target],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired filesystem size"],
            "shrink the Btrfs filesystem after selecting the target size",
        ),
    }
}

fn ext_filesystem_device<'a>(target: &'a str, device: Option<&'a str>) -> Option<&'a str> {
    device.or_else(|| target.starts_with("/dev/").then_some(target))
}

fn filesystem_source_device<'a>(target: &'a str, device: Option<&'a str>) -> Option<&'a str> {
    device.or_else(|| target.starts_with("/dev/").then_some(target))
}

fn filesystem_check_commands(
    fs_type: &str,
    target: &str,
    device: Option<&str>,
) -> Vec<ExecutionCommand> {
    vec![
        command(
            ["disk-nix", "inspect", target],
            false,
            "inspect filesystem identity, mount state, and consumers before check",
        ),
        filesystem_check_command(fs_type, target, device),
    ]
}

fn filesystem_repair_commands(
    fs_type: &str,
    target: &str,
    device: Option<&str>,
) -> Vec<ExecutionCommand> {
    vec![
        command(
            ["disk-nix", "inspect", target],
            false,
            "inspect filesystem identity, mount state, and consumers before repair",
        ),
        command(
            ["findmnt", "--json", "--target", target],
            false,
            "confirm mount state before offline repair",
        ),
        filesystem_repair_command(fs_type, target, device),
    ]
}

fn filesystem_check_command(fs_type: &str, target: &str, device: Option<&str>) -> ExecutionCommand {
    let source = filesystem_source_device(target, device);
    match (fs_type, source) {
        ("ext2" | "ext3" | "ext4", Some(source)) => command(
            ["e2fsck", "-n", source],
            false,
            "run a read-only ext filesystem consistency check",
        ),
        ("ext2" | "ext3" | "ext4", None) => command_with_readiness(
            ["e2fsck", "-n", "<filesystem-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "run ext filesystem check after resolving the source device",
        ),
        ("xfs", Some(source)) => command(
            ["xfs_repair", "-n", source],
            false,
            "run a no-modify XFS metadata check",
        ),
        ("xfs", None) => command_with_readiness(
            ["xfs_repair", "-n", "<filesystem-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "run XFS check after resolving the source device",
        ),
        ("btrfs", Some(source)) => command(
            ["btrfs", "check", "--readonly", source],
            false,
            "run a read-only Btrfs metadata check",
        ),
        ("btrfs", None) => command_with_readiness(
            ["btrfs", "check", "--readonly", "<filesystem-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "run Btrfs check after resolving the source device",
        ),
        ("fat" | "fat12" | "fat16" | "fat32" | "msdos" | "vfat", Some(source)) => command(
            ["fsck.fat", "-n", source],
            false,
            "run a no-write FAT filesystem consistency check",
        ),
        ("fat" | "fat12" | "fat16" | "fat32" | "msdos" | "vfat", None) => command_with_readiness(
            ["fsck.fat", "-n", "<filesystem-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "run FAT filesystem check after resolving the source device",
        ),
        ("exfat", Some(source)) => command(
            ["fsck.exfat", "-n", source],
            false,
            "run a no-write exFAT filesystem consistency check",
        ),
        ("exfat", None) => command_with_readiness(
            ["fsck.exfat", "-n", "<filesystem-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "run exFAT filesystem check after resolving the source device",
        ),
        ("ntfs" | "ntfs3", Some(source)) => command(
            ["ntfsfix", "--no-action", source],
            false,
            "run a no-action NTFS consistency probe",
        ),
        ("ntfs" | "ntfs3", None) => command_with_readiness(
            ["ntfsfix", "--no-action", "<filesystem-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "run NTFS consistency probe after resolving the source device",
        ),
        ("f2fs", Some(source)) => command(
            ["fsck.f2fs", "--dry-run", source],
            false,
            "run a dry-run F2FS filesystem consistency check",
        ),
        ("f2fs", None) => command_with_readiness(
            ["fsck.f2fs", "--dry-run", "<filesystem-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "run F2FS filesystem check after resolving the source device",
        ),
        ("bcachefs", Some(source)) => command(
            ["bcachefs", "fsck", "-n", source],
            false,
            "run a no-repair bcachefs filesystem consistency check",
        ),
        ("bcachefs", None) => command_with_readiness(
            ["bcachefs", "fsck", "-n", "<filesystem-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "run bcachefs filesystem check after resolving the source device",
        ),
        (_, Some(source)) => command_vec_with_readiness(
            vec!["<filesystem-check-tool>", source],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem check tool"],
            "run the filesystem-specific read-only check command",
        ),
        (_, None) => command_with_readiness(
            ["<filesystem-check-tool>", "<filesystem-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem check tool", "filesystem source device"],
            "run the filesystem-specific read-only check command",
        ),
    }
}

fn filesystem_repair_command(
    fs_type: &str,
    target: &str,
    device: Option<&str>,
) -> ExecutionCommand {
    let source = filesystem_source_device(target, device);
    match (fs_type, source) {
        ("ext2" | "ext3" | "ext4", Some(source)) => command(
            ["e2fsck", "-f", "-y", source],
            true,
            "repair ext filesystem metadata after offline review",
        ),
        ("ext2" | "ext3" | "ext4", None) => command_with_readiness(
            ["e2fsck", "-f", "-y", "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "repair ext filesystem after resolving the source device",
        ),
        ("xfs", Some(source)) => command(
            ["xfs_repair", source],
            true,
            "repair XFS metadata after offline review",
        ),
        ("xfs", None) => command_with_readiness(
            ["xfs_repair", "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "repair XFS after resolving the source device",
        ),
        ("btrfs", Some(source)) => command(
            ["btrfs", "check", "--repair", source],
            true,
            "repair Btrfs metadata only after explicit offline review",
        ),
        ("btrfs", None) => command_with_readiness(
            ["btrfs", "check", "--repair", "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "repair Btrfs after resolving the source device",
        ),
        ("fat" | "fat12" | "fat16" | "fat32" | "msdos" | "vfat", Some(source)) => command(
            ["fsck.fat", "-a", source],
            true,
            "repair FAT filesystem metadata after offline review",
        ),
        ("fat" | "fat12" | "fat16" | "fat32" | "msdos" | "vfat", None) => command_with_readiness(
            ["fsck.fat", "-a", "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "repair FAT filesystem after resolving the source device",
        ),
        ("exfat", Some(source)) => command(
            ["fsck.exfat", "-p", source],
            true,
            "repair exFAT filesystem metadata after offline review",
        ),
        ("exfat", None) => command_with_readiness(
            ["fsck.exfat", "-p", "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "repair exFAT filesystem after resolving the source device",
        ),
        ("ntfs" | "ntfs3", Some(source)) => command(
            ["ntfsfix", source],
            true,
            "apply limited NTFS fixes and schedule Windows consistency check after offline review",
        ),
        ("ntfs" | "ntfs3", None) => command_with_readiness(
            ["ntfsfix", "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "run limited NTFS repair after resolving the source device",
        ),
        ("f2fs", Some(source)) => command(
            ["fsck.f2fs", "-f", "-y", source],
            true,
            "repair F2FS filesystem metadata after offline review",
        ),
        ("f2fs", None) => command_with_readiness(
            ["fsck.f2fs", "-f", "-y", "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "repair F2FS filesystem after resolving the source device",
        ),
        ("bcachefs", Some(source)) => command(
            ["bcachefs", "fsck", "-y", source],
            true,
            "repair bcachefs metadata after offline review",
        ),
        ("bcachefs", None) => command_with_readiness(
            ["bcachefs", "fsck", "-y", "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "repair bcachefs after resolving the source device",
        ),
        (_, Some(source)) => command_vec_with_readiness(
            vec!["<filesystem-repair-tool>", source],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem repair tool"],
            "run the filesystem-specific repair command",
        ),
        (_, None) => command_with_readiness(
            ["<filesystem-repair-tool>", "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem repair tool", "filesystem source device"],
            "run the filesystem-specific repair command after resolving the source device",
        ),
    }
}

fn ext_filesystem_grow_command(
    target: &str,
    device: Option<&str>,
    desired_size: Option<&str>,
) -> ExecutionCommand {
    match (ext_filesystem_device(target, device), desired_size) {
        (Some(device), Some(size)) => command_vec(
            vec!["resize2fs", device, size],
            true,
            "grow an ext filesystem to the desired size after the backing block device has grown",
        ),
        (Some(device), None) => command(
            ["resize2fs", device],
            true,
            "grow an ext filesystem after the backing block device has grown",
        ),
        (None, Some(size)) => command_vec_with_readiness(
            vec!["resize2fs", "<filesystem-device>", size],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "grow the ext filesystem after resolving the source block device",
        ),
        (None, None) => command_with_readiness(
            ["resize2fs", "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "grow the ext filesystem after resolving the source block device",
        ),
    }
}

fn ext_filesystem_check_command(target: &str, device: Option<&str>) -> ExecutionCommand {
    if let Some(device) = ext_filesystem_device(target, device) {
        command(
            ["e2fsck", "-f", device],
            true,
            "run a forced ext filesystem check before shrinking",
        )
    } else {
        command_with_readiness(
            ["e2fsck", "-f", "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "run a forced ext filesystem check after resolving the source device",
        )
    }
}

fn ext_filesystem_shrink_command(
    target: &str,
    device: Option<&str>,
    desired_size: Option<&str>,
) -> ExecutionCommand {
    match (ext_filesystem_device(target, device), desired_size) {
        (Some(device), Some(size)) => command(
            ["resize2fs", device, size],
            true,
            "shrink the ext filesystem to the reviewed size",
        ),
        (Some(device), None) => command_with_readiness(
            ["resize2fs", device, "<size>"],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired filesystem size"],
            "shrink the ext filesystem after selecting the target size",
        ),
        (None, Some(size)) => command_vec_with_readiness(
            vec!["resize2fs", "<filesystem-device>", size],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "shrink the ext filesystem after resolving the source device",
        ),
        (None, None) => command_with_readiness(
            ["resize2fs", "<filesystem-device>", "<size>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device", "desired filesystem size"],
            "shrink the ext filesystem after resolving source device and target size",
        ),
    }
}

fn action_id_suffix<'a>(action_id: &'a str, operation: &str) -> Option<&'a str> {
    let marker = format!(":{operation}:");
    let (_, suffix) = action_id.split_once(&marker)?;
    (!suffix.is_empty()).then_some(suffix)
}

fn add_device_command(
    collection: Option<&str>,
    target: &str,
    device: Option<&str>,
) -> ExecutionCommand {
    let Some(device) = device else {
        return command_with_readiness(
            ["<add-device-tool>", target, "<device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["device to add"],
            "attach the new device after selecting the reviewed device path or cache-set UUID",
        );
    };
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
        Some("lvmCaches") => {
            lvm_cache_attach_command(lvm_volume_target_path(Some(target)), Some(device))
        }
        Some("caches") => bcache_attach_command(target, device),
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
    from: Option<&str>,
    to: Option<&str>,
) -> ExecutionCommand {
    let from_arg = from.unwrap_or("<old-device>");
    let to_arg = to.unwrap_or("<new-device>");
    let missing = missing_replacement_inputs(from, to);
    if !missing.is_empty() {
        return command_vec_with_readiness(
            vec!["<replace-device-tool>", target, from_arg, to_arg],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "start the storage-domain replacement operation after selecting both devices",
        );
    }
    let from = from.expect("missing replacement source is handled above");
    let to = to.expect("missing replacement target is handled above");
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
        Some("lvmCaches") => {
            lvm_cache_replace_command(lvm_volume_target_path(Some(target)), Some(from), Some(to))
        }
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

fn missing_replacement_inputs(from: Option<&str>, to: Option<&str>) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if from.is_none() {
        missing.push("device to replace");
    }
    if to.is_none() {
        missing.push("replacement device");
    }
    missing
}

fn zpool_remove_device_command(target: &str, device: Option<&str>) -> ExecutionCommand {
    match device {
        Some(device) => command(
            ["zpool", "remove", target, device],
            true,
            "remove the reviewed device from the ZFS pool when the layout supports evacuation",
        ),
        None => command_with_readiness(
            ["zpool", "remove", target, "<device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["device to remove"],
            "remove a ZFS pool device after selecting the reviewed vdev or device",
        ),
    }
}

fn lvm_physical_volume_inspect_command(device: Option<&str>) -> ExecutionCommand {
    match device {
        Some(device) => command(
            ["pvs", "--reportformat", "json", device],
            false,
            "inspect physical volume allocation before vgreduce",
        ),
        None => command_with_readiness(
            ["pvs", "--reportformat", "json", "<physical-volume>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["physical volume to remove"],
            "inspect physical volume allocation after selecting the reviewed PV",
        ),
    }
}

fn lvm_physical_volume_move_command(device: Option<&str>) -> ExecutionCommand {
    match device {
        Some(device) => command(
            ["pvmove", device],
            true,
            "evacuate allocated extents from the reviewed physical volume before vgreduce",
        ),
        None => command_with_readiness(
            ["pvmove", "<physical-volume>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["physical volume to remove"],
            "evacuate allocated extents after selecting the reviewed physical volume",
        ),
    }
}

fn lvm_volume_group_reduce_command(target: &str, device: Option<&str>) -> ExecutionCommand {
    match device {
        Some(device) => command(
            ["vgreduce", target, device],
            true,
            "remove the reviewed physical volume from the LVM volume group after extents are evacuated",
        ),
        None => command_with_readiness(
            ["vgreduce", target, "<physical-volume>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["physical volume to remove"],
            "remove the physical volume from the volume group after selecting it",
        ),
    }
}

fn md_array_target_path(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .target
        .as_deref()
        .filter(|target| target.starts_with("/dev/md"))
        .or_else(|| {
            action
                .context
                .name
                .as_deref()
                .filter(|name| name.starts_with("/dev/md"))
        })
}

fn md_raid_detail_command(target: Option<&str>, note: &'static str) -> ExecutionCommand {
    match target {
        Some(target) => command(["mdadm", "--detail", target], false, note),
        None => command_with_readiness(
            ["mdadm", "--detail", "<md-array>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["MD array path"],
            note,
        ),
    }
}

fn md_raid_add_member_command(target: Option<&str>, device: Option<&str>) -> ExecutionCommand {
    let target_arg = target.unwrap_or("<md-array>");
    match (target, device) {
        (Some(_), Some(device)) => command(
            ["mdadm", target_arg, "--add", device],
            true,
            "add the reviewed member or spare to the MD RAID array",
        ),
        _ => command_vec_with_readiness(
            vec!["mdadm", target_arg, "--add", device.unwrap_or("<device>")],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_md_member_operation_inputs(target, device),
            "add the MD RAID member after selecting the array and member",
        ),
    }
}

fn md_raid_fail_member_command(target: Option<&str>, device: Option<&str>) -> ExecutionCommand {
    let target_arg = target.unwrap_or("<md-array>");
    match (target, device) {
        (Some(_), Some(device)) => command(
            ["mdadm", target_arg, "--fail", device],
            true,
            "mark the MD RAID member failed before removal",
        ),
        _ => command_vec_with_readiness(
            vec!["mdadm", target_arg, "--fail", device.unwrap_or("<device>")],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_md_member_operation_inputs(target, device),
            "mark the MD RAID member failed after selecting the array and member",
        ),
    }
}

fn md_raid_remove_member_command(target: Option<&str>, device: Option<&str>) -> ExecutionCommand {
    let target_arg = target.unwrap_or("<md-array>");
    match (target, device) {
        (Some(_), Some(device)) => command(
            ["mdadm", target_arg, "--remove", device],
            true,
            "remove the reviewed MD RAID member",
        ),
        _ => command_vec_with_readiness(
            vec![
                "mdadm",
                target_arg,
                "--remove",
                device.unwrap_or("<device>"),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_md_member_operation_inputs(target, device),
            "remove the MD RAID member after selecting the array and member",
        ),
    }
}

fn md_raid_replace_member_command(
    target: Option<&str>,
    source: Option<&str>,
    replacement: Option<&str>,
) -> ExecutionCommand {
    let target_arg = target.unwrap_or("<md-array>");
    let source_arg = source.unwrap_or("<old-device>");
    let replacement_arg = replacement.unwrap_or("<new-device>");
    let mut missing = Vec::new();
    if target.is_none() {
        missing.push("MD array path");
    }
    if source.is_none() {
        missing.push("device to replace");
    }
    if replacement.is_none() {
        missing.push("replacement device");
    }

    if missing.is_empty() {
        command(
            [
                "mdadm",
                target_arg,
                "--replace",
                source_arg,
                "--with",
                replacement_arg,
            ],
            true,
            "replace an MD RAID member while preserving array redundancy",
        )
    } else {
        command_vec_with_readiness(
            vec![
                "mdadm",
                target_arg,
                "--replace",
                source_arg,
                "--with",
                replacement_arg,
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "replace the MD RAID member after selecting the array, old member, and replacement",
        )
    }
}

fn missing_md_member_operation_inputs(
    target: Option<&str>,
    device: Option<&str>,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if target.is_none() {
        missing.push("MD array path");
    }
    if device.is_none() {
        missing.push("member device to remove");
    }
    missing
}

fn multipath_add_path_command(path: Option<&str>) -> ExecutionCommand {
    match path {
        Some(path) => command(
            ["multipathd", "add", "path", path],
            true,
            "add or re-add the reviewed path to multipathd",
        ),
        None => command_with_readiness(
            ["multipathd", "add", "path", "<path>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["multipath path to add"],
            "add the multipath path after selecting the reviewed path",
        ),
    }
}

fn multipath_delete_path_command(path: Option<&str>) -> ExecutionCommand {
    match path {
        Some(path) => command(
            ["multipathd", "del", "path", path],
            true,
            "delete the reviewed path from multipathd",
        ),
        None => command_with_readiness(
            ["multipathd", "del", "path", "<path>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["multipath path to remove"],
            "delete the multipath path after selecting the reviewed path",
        ),
    }
}

fn multipath_map_target(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .target
        .as_deref()
        .filter(|target| is_multipath_map_target(target))
        .or_else(|| {
            action
                .context
                .name
                .as_deref()
                .filter(|name| is_multipath_map_target(name))
        })
}

fn is_multipath_map_target(target: &str) -> bool {
    target.starts_with("mpath") || target.starts_with("/dev/mapper/")
}

fn multipath_list_command(target: Option<&str>, note: &'static str) -> ExecutionCommand {
    match target {
        Some(target) => command(["multipath", "-ll", target], false, note),
        None => command_with_readiness(
            ["multipath", "-ll", "<multipath-map>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["multipath map target"],
            note,
        ),
    }
}

fn multipath_resize_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["multipathd", "resize", "map", target],
            true,
            "resize the multipath map after every backing path sees the new LUN size",
        ),
        None => command_with_readiness(
            ["multipathd", "resize", "map", "<multipath-map>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["multipath map target"],
            "resize the multipath map after every backing path sees the new LUN size",
        ),
    }
}

fn btrfs_remove_device_command(target: &str, device: Option<&str>) -> ExecutionCommand {
    match device {
        Some(device) => command(
            ["btrfs", "device", "remove", device, target],
            true,
            "remove the reviewed device from the Btrfs filesystem after data evacuation checks",
        ),
        None => command_with_readiness(
            ["btrfs", "device", "remove", "<device>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["device to remove"],
            "remove the Btrfs device after selecting the reviewed device",
        ),
    }
}

fn rebalance_command(
    collection: Option<&str>,
    target: &str,
    property_assignments: &[String],
) -> ExecutionCommand {
    match collection {
        Some("pools") => command(
            ["zpool", "scrub", target],
            true,
            "scrub the pool after topology changes; ZFS has no generic rebalance command",
        ),
        Some("filesystems") => {
            let mut argv = vec![
                "btrfs".to_string(),
                "balance".to_string(),
                "start".to_string(),
            ];
            argv.extend(btrfs_balance_filters(property_assignments));
            argv.push(target.to_string());
            command_vec(
                argv,
                true,
                "rebalance Btrfs chunks across available devices",
            )
        }
        _ => command_with_readiness(
            ["<rebalance-tool>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["rebalance tool"],
            "run the storage-domain rebalance command",
        ),
    }
}

fn scrub_command(collection: Option<&str>, target: &str) -> ExecutionCommand {
    match collection {
        Some("pools") => command(
            ["zpool", "scrub", target],
            true,
            "start the reviewed ZFS pool scrub",
        ),
        Some("filesystems") => command(
            ["btrfs", "scrub", "start", "-B", target],
            true,
            "run the reviewed Btrfs scrub and wait for completion",
        ),
        _ => command_with_readiness(
            ["<scrub-tool>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["scrub tool"],
            "run the storage-domain scrub command",
        ),
    }
}

fn filesystem_trim_command(collection: Option<&str>, target: &str) -> ExecutionCommand {
    match collection {
        Some("filesystems") => command(
            ["fstrim", "-v", target],
            true,
            "trim unused blocks from the mounted filesystem",
        ),
        _ => command_with_readiness(
            ["<trim-tool>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["trim tool"],
            "run the storage-domain trim or discard command",
        ),
    }
}

fn btrfs_balance_filters(property_assignments: &[String]) -> Vec<String> {
    property_assignments
        .iter()
        .filter_map(|assignment| {
            let (property, value) = assignment.split_once('=')?;
            let property = property
                .strip_prefix("btrfs.balance.")
                .or_else(|| property.strip_prefix("balance."))
                .or_else(|| property.strip_prefix("btrfs."))
                .unwrap_or(property);
            match property {
                "data" | "d" => Some(format!("-d{value}")),
                "metadata" | "meta" | "m" => Some(format!("-m{value}")),
                "system" | "s" => Some(format!("-s{value}")),
                _ => None,
            }
        })
        .collect()
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
        Some("zvols") => command(
            ["zfs", "set", assignment, target],
            true,
            "set a zvol property",
        ),
        Some("btrfsSubvolumes") => btrfs_subvolume_property_command(target, property, assignment),
        Some("exports") => command(
            ["exportfs", "-ra"],
            true,
            "reload NFS exports after export property changes",
        ),
        Some("lvmCaches") => {
            lvm_cache_property_command(lvm_volume_target_path(Some(target)), property, assignment)
        }
        Some("caches") => bcache_property_command(target, property, assignment),
        Some("vdoVolumes") => vdo_property_command(target, property, assignment),
        _ => command_with_readiness(
            ["<set-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["property update tool"],
            "apply the storage-domain property update",
        ),
    }
}

fn vdo_property_command(target: &str, property: &str, assignment: &str) -> ExecutionCommand {
    let value = assignment
        .split_once('=')
        .map(|(_, value)| value)
        .unwrap_or(assignment);
    match normalize_property_name(property).as_str() {
        "writepolicy" | "write-policy" | "vdo-write-policy" => {
            vdo_write_policy_command(target, value)
        }
        "compression" | "vdo-compression" => vdo_boolean_toggle_command(
            target,
            value,
            "enableCompression",
            "disableCompression",
            "compression",
        ),
        "deduplication" | "dedupe" | "vdo-deduplication" | "vdo-dedupe" => {
            vdo_boolean_toggle_command(
                target,
                value,
                "enableDeduplication",
                "disableDeduplication",
                "deduplication",
            )
        }
        _ => command_with_readiness(
            ["<vdo-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported VDO property"],
            "apply a VDO property update after selecting the domain-specific command",
        ),
    }
}

fn vdo_write_policy_command(target: &str, value: &str) -> ExecutionCommand {
    let policy = normalize_property_name(value);
    match policy.as_str() {
        "auto" | "sync" | "async" => command_vec(
            [
                "vdo",
                "changeWritePolicy",
                "--name",
                target,
                "--writePolicy",
                policy.as_str(),
            ],
            true,
            "change VDO write policy",
        ),
        _ => command_with_readiness(
            ["<vdo-property-tool>", target, "writePolicy"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["VDO write policy value"],
            "apply a VDO write policy after choosing auto, sync, or async",
        ),
    }
}

fn vdo_boolean_toggle_command(
    target: &str,
    value: &str,
    enable_command: &'static str,
    disable_command: &'static str,
    label: &'static str,
) -> ExecutionCommand {
    match normalize_property_name(value).as_str() {
        "enabled" | "enable" | "true" | "yes" | "on" => command(
            ["vdo", enable_command, "--name", target],
            true,
            &format!("enable VDO {label}"),
        ),
        "disabled" | "disable" | "false" | "no" | "off" => command(
            ["vdo", disable_command, "--name", target],
            true,
            &format!("disable VDO {label}"),
        ),
        _ => command_with_readiness(
            ["<vdo-property-tool>", target, label],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["boolean VDO property value"],
            "apply a VDO boolean property after choosing enabled or disabled",
        ),
    }
}

fn normalize_property_name(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("vdo.")
        .chars()
        .map(|character| match character {
            'A'..='Z' => character.to_ascii_lowercase(),
            'a'..='z' | '0'..='9' => character,
            _ => '-',
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn filesystem_property_command(
    fs_type: Option<&str>,
    target: &str,
    device: Option<&str>,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    match fs_type {
        Some("btrfs") => btrfs_filesystem_property_command(target, device, property, assignment),
        Some("ext2" | "ext3" | "ext4") => {
            ext_filesystem_property_command(device, target, property, assignment)
        }
        Some("fat" | "fat12" | "fat16" | "fat32" | "msdos" | "vfat") => {
            fat_filesystem_property_command(device, target, property, assignment)
        }
        Some("ntfs" | "ntfs3") => {
            ntfs_filesystem_property_command(device, target, property, assignment)
        }
        Some("exfat") => exfat_filesystem_property_command(device, target, property, assignment),
        Some("xfs") => xfs_filesystem_property_command(device, target, property, assignment),
        Some("zfs") => command(
            ["zfs", "set", assignment, target],
            true,
            "set a ZFS filesystem property",
        ),
        _ => command_with_readiness(
            ["<filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem type", "supported filesystem property"],
            "set a filesystem property after selecting the filesystem-specific command",
        ),
    }
}

fn swap_property_command(
    target: Option<&str>,
    property: &str,
    value: Option<&str>,
) -> ExecutionCommand {
    match (property, target, value) {
        ("label" | "swap.label", Some(target), Some(value)) => command(
            ["swaplabel", "--label", value, target],
            true,
            "set the swap signature label on the reviewed inactive swap target",
        ),
        ("label" | "swap.label", None, Some(value)) => command_with_readiness(
            ["swaplabel", "--label", value, "<swap>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["swap target path"],
            "set the swap label after resolving the swap target",
        ),
        ("label" | "swap.label", Some(target), None) => command_with_readiness(
            ["swaplabel", "--label", "<label>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["swap label"],
            "set the swap label after resolving the desired label",
        ),
        ("label" | "swap.label", None, None) => command_with_readiness(
            ["swaplabel", "--label", "<label>", "<swap>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["swap target path", "swap label"],
            "set the swap label after resolving target and label",
        ),
        ("uuid" | "swap.uuid", Some(target), Some(value)) => command(
            ["swaplabel", "--uuid", value, target],
            true,
            "set the swap signature UUID on the reviewed inactive swap target",
        ),
        ("uuid" | "swap.uuid", None, Some(value)) => command_with_readiness(
            ["swaplabel", "--uuid", value, "<swap>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["swap target path"],
            "set the swap UUID after resolving the swap target",
        ),
        ("uuid" | "swap.uuid", Some(target), None) => command_with_readiness(
            ["swaplabel", "--uuid", "<uuid>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["swap UUID"],
            "set the swap UUID after resolving the desired UUID",
        ),
        ("uuid" | "swap.uuid", None, None) => command_with_readiness(
            ["swaplabel", "--uuid", "<uuid>", "<swap>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["swap target path", "swap UUID"],
            "set the swap UUID after resolving target and UUID",
        ),
        _ => command_with_readiness(
            ["<swap-property-tool>", target.unwrap_or("<swap>"), property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported swap property"],
            "set a swap property after selecting a supported property mapping",
        ),
    }
}

fn fat_filesystem_property_command(
    device: Option<&str>,
    target: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<fat-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["FAT filesystem property value"],
            "set a FAT filesystem property after resolving the desired value",
        );
    };
    match (property, device) {
        ("label" | "fat.label" | "vfat.label" | "filesystem.label", Some(device)) => command(
            ["fatlabel", device, value],
            true,
            "set the FAT filesystem label on the reviewed backing device",
        ),
        ("label" | "fat.label" | "vfat.label" | "filesystem.label", None) => {
            command_with_readiness(
                ["fatlabel", "<filesystem-device>", value],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["filesystem source device"],
                "set the FAT filesystem label after resolving the backing device",
            )
        }
        (
            "uuid" | "fat.uuid" | "vfat.uuid" | "filesystem.uuid" | "volumeId" | "volume-id"
            | "fat.volume-id" | "vfat.volume-id",
            Some(device),
        ) => match fat_volume_id(value) {
            Some(volume_id) => command_vec(
                ["fatlabel", "-i", device, volume_id.as_str()],
                true,
                "set the FAT filesystem volume ID on the reviewed unmounted backing device",
            ),
            None => command_with_readiness(
                ["<fat-filesystem-property-tool>", target, property],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["8-hex-digit FAT volume ID"],
                "set a FAT filesystem volume ID after resolving a valid value",
            ),
        },
        (
            "uuid" | "fat.uuid" | "vfat.uuid" | "filesystem.uuid" | "volumeId" | "volume-id"
            | "fat.volume-id" | "vfat.volume-id",
            None,
        ) => match fat_volume_id(value) {
            Some(volume_id) => command_vec_with_readiness(
                ["fatlabel", "-i", "<filesystem-device>", volume_id.as_str()],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["filesystem source device"],
                "set the FAT filesystem volume ID after resolving the backing device",
            ),
            None => command_with_readiness(
                ["<fat-filesystem-property-tool>", target, property],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["filesystem source device", "8-hex-digit FAT volume ID"],
                "set a FAT filesystem volume ID after resolving device and value",
            ),
        },
        _ => command_with_readiness(
            ["<fat-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported FAT filesystem property"],
            "set a FAT filesystem property after selecting a supported property mapping",
        ),
    }
}

fn fat_volume_id(value: &str) -> Option<String> {
    let normalized: String = value
        .trim()
        .chars()
        .filter(|character| *character != '-')
        .collect();
    if normalized.len() == 8
        && normalized
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        Some(normalized.to_ascii_uppercase())
    } else {
        None
    }
}

fn ntfs_filesystem_property_command(
    device: Option<&str>,
    target: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<ntfs-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["NTFS filesystem property value"],
            "set an NTFS filesystem property after resolving the desired value",
        );
    };
    match (property, device) {
        ("label" | "ntfs.label" | "filesystem.label", Some(device)) => command(
            ["ntfslabel", device, value],
            true,
            "set the NTFS filesystem label on the reviewed backing device",
        ),
        ("label" | "ntfs.label" | "filesystem.label", None) => command_with_readiness(
            ["ntfslabel", "<filesystem-device>", value],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "set the NTFS filesystem label after resolving the backing device",
        ),
        (
            "uuid" | "ntfs.uuid" | "filesystem.uuid" | "serial" | "volumeSerial" | "volume-serial"
            | "ntfs.serial" | "ntfs.volume-serial",
            Some(device),
        ) => match ntfs_volume_serial(value) {
            Some(serial) => command_vec(
                vec![
                    "ntfslabel".to_string(),
                    format!("--new-serial={serial}"),
                    device.to_string(),
                ],
                true,
                "set the NTFS filesystem volume serial on the reviewed unmounted backing device",
            ),
            None => command_with_readiness(
                ["<ntfs-filesystem-property-tool>", target, property],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["16-hex-digit NTFS volume serial"],
                "set an NTFS filesystem volume serial after resolving a valid value",
            ),
        },
        (
            "uuid" | "ntfs.uuid" | "filesystem.uuid" | "serial" | "volumeSerial" | "volume-serial"
            | "ntfs.serial" | "ntfs.volume-serial",
            None,
        ) => match ntfs_volume_serial(value) {
            Some(serial) => command_vec_with_readiness(
                vec![
                    "ntfslabel".to_string(),
                    format!("--new-serial={serial}"),
                    "<filesystem-device>".to_string(),
                ],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["filesystem source device"],
                "set the NTFS filesystem volume serial after resolving the backing device",
            ),
            None => command_with_readiness(
                ["<ntfs-filesystem-property-tool>", target, property],
                true,
                CommandReadiness::NeedsDomainImplementation,
                [
                    "filesystem source device",
                    "16-hex-digit NTFS volume serial",
                ],
                "set an NTFS filesystem volume serial after resolving device and value",
            ),
        },
        _ => command_with_readiness(
            ["<ntfs-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported NTFS filesystem property"],
            "set an NTFS filesystem property after selecting a supported property mapping",
        ),
    }
}

fn ntfs_volume_serial(value: &str) -> Option<String> {
    let normalized: String = value
        .trim()
        .chars()
        .filter(|character| *character != '-')
        .collect();
    if normalized.len() == 16
        && normalized
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        Some(normalized.to_ascii_uppercase())
    } else {
        None
    }
}

fn exfat_filesystem_property_command(
    device: Option<&str>,
    target: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<exfat-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["exFAT filesystem property value"],
            "set an exFAT filesystem property after resolving the desired value",
        );
    };
    match (property, device) {
        ("label" | "exfat.label" | "filesystem.label", Some(device)) => command(
            ["exfatlabel", device, value],
            true,
            "set the exFAT filesystem label on the reviewed backing device",
        ),
        ("label" | "exfat.label" | "filesystem.label", None) => command_with_readiness(
            ["exfatlabel", "<filesystem-device>", value],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "set the exFAT filesystem label after resolving the backing device",
        ),
        (
            "uuid"
            | "exfat.uuid"
            | "filesystem.uuid"
            | "serial"
            | "volumeSerial"
            | "volume-serial"
            | "exfat.serial"
            | "exfat.volume-serial",
            Some(device),
        ) => match exfat_volume_serial(value) {
            Some(serial) => command_vec(
                ["exfatlabel", "-i", device, serial.as_str()],
                true,
                "set the exFAT filesystem volume serial on the reviewed unmounted backing device",
            ),
            None => command_with_readiness(
                ["<exfat-filesystem-property-tool>", target, property],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["8-hex-digit exFAT volume serial"],
                "set an exFAT filesystem volume serial after resolving a valid value",
            ),
        },
        (
            "uuid"
            | "exfat.uuid"
            | "filesystem.uuid"
            | "serial"
            | "volumeSerial"
            | "volume-serial"
            | "exfat.serial"
            | "exfat.volume-serial",
            None,
        ) => match exfat_volume_serial(value) {
            Some(serial) => command_vec_with_readiness(
                ["exfatlabel", "-i", "<filesystem-device>", serial.as_str()],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["filesystem source device"],
                "set the exFAT filesystem volume serial after resolving the backing device",
            ),
            None => command_with_readiness(
                ["<exfat-filesystem-property-tool>", target, property],
                true,
                CommandReadiness::NeedsDomainImplementation,
                [
                    "filesystem source device",
                    "8-hex-digit exFAT volume serial",
                ],
                "set an exFAT filesystem volume serial after resolving device and value",
            ),
        },
        _ => command_with_readiness(
            ["<exfat-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported exFAT filesystem property"],
            "set an exFAT filesystem property after selecting a supported property mapping",
        ),
    }
}

fn exfat_volume_serial(value: &str) -> Option<String> {
    let normalized: String = value
        .trim()
        .chars()
        .filter(|character| *character != '-')
        .collect();
    if normalized.len() == 8
        && normalized
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        Some(normalized.to_ascii_uppercase())
    } else {
        None
    }
}

fn xfs_filesystem_property_command(
    device: Option<&str>,
    target: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<xfs-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["XFS filesystem property value"],
            "set an XFS filesystem property after resolving the desired value",
        );
    };
    match (property, device) {
        ("label" | "xfs.label" | "filesystem.label", Some(device)) => command(
            ["xfs_admin", "-L", value, device],
            true,
            "set the XFS filesystem label on the reviewed backing device",
        ),
        ("label" | "xfs.label" | "filesystem.label", None) => command_with_readiness(
            ["xfs_admin", "-L", value, "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "set the XFS filesystem label after resolving the backing device",
        ),
        ("uuid" | "xfs.uuid" | "filesystem.uuid", Some(device)) => command(
            ["xfs_admin", "-U", value, device],
            true,
            "set the XFS filesystem UUID on the reviewed unmounted backing device",
        ),
        ("uuid" | "xfs.uuid" | "filesystem.uuid", None) => command_with_readiness(
            ["xfs_admin", "-U", value, "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "set the XFS filesystem UUID after resolving the backing device",
        ),
        _ => command_with_readiness(
            ["<xfs-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported XFS filesystem property"],
            "set an XFS filesystem property after selecting a supported property mapping",
        ),
    }
}

fn ext_filesystem_property_command(
    device: Option<&str>,
    target: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<ext-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["Ext filesystem property value"],
            "set an Ext filesystem property after resolving the desired value",
        );
    };
    match (property, device) {
        ("label" | "ext.label" | "filesystem.label", Some(device)) => command(
            ["e2label", device, value],
            true,
            "set the Ext filesystem label on the reviewed backing device",
        ),
        ("label" | "ext.label" | "filesystem.label", None) => command_with_readiness(
            ["e2label", "<filesystem-device>", value],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "set the Ext filesystem label after resolving the backing device",
        ),
        ("uuid" | "ext.uuid" | "filesystem.uuid", Some(device)) => command(
            ["tune2fs", "-U", value, device],
            true,
            "set the Ext filesystem UUID on the reviewed unmounted backing device",
        ),
        ("uuid" | "ext.uuid" | "filesystem.uuid", None) => command_with_readiness(
            ["tune2fs", "-U", value, "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "set the Ext filesystem UUID after resolving the backing device",
        ),
        _ => command_with_readiness(
            ["<ext-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported Ext filesystem property"],
            "set an Ext filesystem property after selecting a supported property mapping",
        ),
    }
}

fn btrfs_filesystem_property_command(
    target: &str,
    device: Option<&str>,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<btrfs-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["Btrfs filesystem property value"],
            "set a Btrfs filesystem property after resolving the desired value",
        );
    };
    match property {
        "label" | "btrfs.label" | "filesystem.label" => command(
            ["btrfs", "filesystem", "label", target, value],
            true,
            "set the Btrfs filesystem label",
        ),
        "uuid" | "btrfs.uuid" | "filesystem.uuid" => match device {
            Some(device) => command(
                ["btrfstune", "-U", value, device],
                true,
                "set the Btrfs filesystem UUID on the reviewed unmounted backing device",
            ),
            None => command_with_readiness(
                ["btrfstune", "-U", value, "<filesystem-device>"],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["filesystem source device"],
                "set the Btrfs filesystem UUID after resolving the backing device",
            ),
        },
        _ => command_with_readiness(
            ["<btrfs-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported Btrfs filesystem property"],
            "set a Btrfs filesystem property after selecting a supported property mapping",
        ),
    }
}

fn btrfs_subvolume_property_command(
    target: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<btrfs-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["Btrfs property value"],
            "set a Btrfs subvolume property after resolving the desired value",
        );
    };
    let property_name = match property {
        "ro" | "readonly" | "readOnly" | "btrfs.readonly" | "btrfs.ro" => "ro",
        _ => {
            return command_with_readiness(
                ["<btrfs-property-tool>", target, property],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["supported Btrfs subvolume property"],
                "set a Btrfs subvolume property after selecting a supported property mapping",
            );
        }
    };
    command_vec(
        vec![
            "btrfs".to_string(),
            "property".to_string(),
            "set".to_string(),
            "-ts".to_string(),
            target.to_string(),
            property_name.to_string(),
            normalize_boolish_btrfs_property_value(value),
        ],
        true,
        "set a Btrfs subvolume property",
    )
}

fn btrfs_qgroup_property_command(
    target: &str,
    qgroup_id: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<btrfs-qgroup-tool>", target, qgroup_id],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["Btrfs qgroup limit value"],
            "set a Btrfs qgroup limit after resolving the desired value",
        );
    };
    if target == qgroup_id || target.starts_with("0/") {
        return command_with_readiness(
            ["btrfs", "qgroup", "limit", value, qgroup_id, "<path>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["mounted Btrfs filesystem path"],
            "set a Btrfs qgroup limit after selecting the mounted filesystem path",
        );
    }
    let limit_value = normalize_btrfs_qgroup_limit(value);
    match property {
        "limit" | "maxReferenced" | "max-referenced" | "referenced" | "btrfs.max-referenced" => {
            command_vec(
                vec![
                    "btrfs".to_string(),
                    "qgroup".to_string(),
                    "limit".to_string(),
                    limit_value,
                    qgroup_id.to_string(),
                    target.to_string(),
                ],
                true,
                "set a Btrfs qgroup referenced-byte limit",
            )
        }
        "maxExclusive" | "max-exclusive" | "exclusive" | "btrfs.max-exclusive" => command_vec(
            vec![
                "btrfs".to_string(),
                "qgroup".to_string(),
                "limit".to_string(),
                "-e".to_string(),
                limit_value,
                qgroup_id.to_string(),
                target.to_string(),
            ],
            true,
            "set a Btrfs qgroup exclusive-byte limit",
        ),
        _ => command_with_readiness(
            ["<btrfs-qgroup-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported Btrfs qgroup property"],
            "set a Btrfs qgroup property after selecting a supported property mapping",
        ),
    }
}

fn btrfs_qgroup_target_path<'a>(target: Option<&'a str>, qgroup_id: &str) -> Option<&'a str> {
    let target = target?;
    if target == qgroup_id || target.starts_with("0/") {
        None
    } else {
        Some(target)
    }
}

fn normalize_btrfs_qgroup_limit(value: &str) -> String {
    match value {
        "null" | "none" | "None" | "NONE" | "unlimited" => "none".to_string(),
        other => other.to_string(),
    }
}

fn normalize_boolish_btrfs_property_value(value: &str) -> String {
    match value {
        "1" | "yes" | "on" | "true" => "true".to_string(),
        "0" | "no" | "off" | "false" => "false".to_string(),
        other => other.to_string(),
    }
}

fn is_bcache_target(target: &str) -> bool {
    target.starts_with("/dev/bcache")
}

fn lvm_cache_attach_command(target: Option<&str>, cache_pool: Option<&str>) -> ExecutionCommand {
    match (target, cache_pool) {
        (Some(target), Some(cache_pool)) => command(
            [
                "lvconvert",
                "--type",
                "cache",
                "--cachepool",
                cache_pool,
                target,
            ],
            true,
            "attach the reviewed LVM cache pool to the origin logical volume",
        ),
        (target, cache_pool) => command_vec_with_readiness(
            vec![
                "lvconvert".to_string(),
                "--type".to_string(),
                "cache".to_string(),
                "--cachepool".to_string(),
                cache_pool.unwrap_or("<cache-pool>").to_string(),
                target.unwrap_or("<origin-logical-volume>").to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_lvm_cache_inputs(target, cache_pool),
            "attach LVM cache after selecting an origin LV and cache-pool LV",
        ),
    }
}

fn lvm_cache_replace_command(
    target: Option<&str>,
    old_cache_pool: Option<&str>,
    new_cache_pool: Option<&str>,
) -> ExecutionCommand {
    match (target, new_cache_pool) {
        (Some(target), Some(new_cache_pool)) => command_vec(
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "lvconvert --uncache \"$1\" && lvconvert --type cache --cachepool \"$2\" \"$1\""
                    .to_string(),
                "disk-nix-lvm-cache-replace".to_string(),
                target.to_string(),
                new_cache_pool.to_string(),
            ],
            true,
            "detach the old LVM cache and attach the reviewed replacement cache pool",
        ),
        (target, new_cache_pool) => {
            let mut missing = missing_lvm_cache_inputs(target, new_cache_pool);
            if old_cache_pool.is_none() {
                missing.push("cache pool to replace");
            }
            command_vec_with_readiness(
                vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    "lvconvert --uncache \"$1\" && lvconvert --type cache --cachepool \"$2\" \"$1\""
                        .to_string(),
                    "disk-nix-lvm-cache-replace".to_string(),
                    target.unwrap_or("<origin-logical-volume>").to_string(),
                    new_cache_pool.unwrap_or("<replacement-cache-pool>").to_string(),
                ],
                true,
                CommandReadiness::NeedsDomainImplementation,
                missing,
                "replace LVM cache after selecting origin and replacement cache pool",
            )
        }
    }
}

fn lvm_cache_uncache_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["lvconvert", "--uncache", target],
            true,
            "detach LVM cache from the origin logical volume after dirty data is flushed",
        ),
        None => command_with_readiness(
            ["lvconvert", "--uncache", "<origin-logical-volume>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["target in volume-group/logical-volume form"],
            "detach LVM cache after selecting the origin logical volume",
        ),
    }
}

fn lvm_cache_property_command(
    target: Option<&str>,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            [
                "lvchange",
                "<cache-property>",
                "<value>",
                "<origin-logical-volume>",
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["cache property value"],
            "set LVM cache property after resolving the desired value",
        );
    };
    let Some(flag) = lvm_cache_property_flag(property) else {
        return command_with_readiness(
            [
                "lvchange",
                "<cache-property>",
                value,
                target.unwrap_or("<origin-logical-volume>"),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported LVM cache property"],
            "set LVM cache property after mapping it to an lvchange flag",
        );
    };
    match target {
        Some(target) => command(
            ["lvchange", flag, value, target],
            true,
            "set LVM cache mode or policy on the reviewed origin logical volume",
        ),
        None => command_vec_with_readiness(
            vec![
                "lvchange".to_string(),
                flag.to_string(),
                value.to_string(),
                "<origin-logical-volume>".to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["target in volume-group/logical-volume form"],
            "set LVM cache property after selecting the origin logical volume",
        ),
    }
}

fn missing_lvm_cache_inputs(target: Option<&str>, cache_pool: Option<&str>) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if target.is_none() {
        missing.push("target in volume-group/logical-volume form");
    }
    if cache_pool.is_none() {
        missing.push("cache-pool logical volume");
    }
    missing
}

fn lvm_cache_property_flag(property: &str) -> Option<&'static str> {
    match property {
        "cache-mode" | "cacheMode" | "lvm.cache-mode" | "lvm.cacheMode" => Some("--cachemode"),
        "cache-policy" | "cachePolicy" | "lvm.cache-policy" | "lvm.cachePolicy" => {
            Some("--cachepolicy")
        }
        _ => None,
    }
}

fn bcache_attach_command(target: &str, cache_set: &str) -> ExecutionCommand {
    if !is_bcache_target(target) {
        return command_vec_with_readiness(
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/attach\"".to_string(),
                "disk-nix-bcache-attach".to_string(),
                "<cache-device>".to_string(),
                cache_set.to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["bcache device path"],
            "attach an existing bcache cache-set UUID after selecting the backing bcache device",
        );
    }

    command_vec(
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/attach\"".to_string(),
            "disk-nix-bcache-attach".to_string(),
            target.to_string(),
            cache_set.to_string(),
        ],
        true,
        "attach an existing bcache cache-set UUID to the backing bcache device",
    )
}

fn bcache_detach_command(target: &str) -> ExecutionCommand {
    if !is_bcache_target(target) {
        return command_vec_with_readiness(
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "printf '1\\n' > \"/sys/block/${1#/dev/}/bcache/detach\"".to_string(),
                "disk-nix-bcache-detach".to_string(),
                "<cache-device>".to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["bcache device path"],
            "detach the bcache cache set after selecting the backing bcache device",
        );
    }

    command_vec(
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "printf '1\\n' > \"/sys/block/${1#/dev/}/bcache/detach\"".to_string(),
            "disk-nix-bcache-detach".to_string(),
            target.to_string(),
        ],
        true,
        "detach the bcache cache set from the backing device after dirty data is flushed",
    )
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
    if !is_bcache_target(target) {
        return command_vec_with_readiness(
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/$3\"".to_string(),
                "disk-nix-bcache-property".to_string(),
                "<cache-device>".to_string(),
                value.to_string(),
                bcache_sysfs_key(property),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["bcache device path"],
            "set a bcache sysfs property after selecting the backing bcache device",
        );
    }
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
    if !is_bcache_target(target) {
        return command_vec_with_readiness(
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "cat \"/sys/block/${1#/dev/}/bcache/$2\"".to_string(),
                "disk-nix-bcache-read".to_string(),
                "<cache-device>".to_string(),
                key.to_string(),
            ],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["bcache device path"],
            note,
        );
    }

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

fn lun_rescan_devices(action: &PlannedAction) -> Vec<String> {
    let mut devices = BTreeSet::new();
    if let Some(device) = action.context.device.as_deref() {
        devices.insert(device.to_string());
    }
    devices.extend(action.context.devices.iter().cloned());
    devices.into_iter().collect()
}

fn scsi_device_rescan_command(device: &str) -> ExecutionCommand {
    command_vec(
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "block=$(basename \"$(readlink -f \"$1\")\"); printf '1\\n' > \"/sys/class/block/${block}/device/rescan\""
                .to_string(),
            "disk-nix-scsi-rescan".to_string(),
            device.to_string(),
        ],
        true,
        "rescan the reviewed SCSI block path after target-side changes",
    )
}

fn scsi_device_delete_command(device: &str) -> ExecutionCommand {
    command_vec(
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "block=$(basename \"$(readlink -f \"$1\")\"); printf '1\\n' > \"/sys/class/block/${block}/device/delete\""
                .to_string(),
            "disk-nix-scsi-delete".to_string(),
            device.to_string(),
        ],
        true,
        "detach the reviewed SCSI block path from the host",
    )
}

fn nfs_export_create_command(
    target: Option<&str>,
    client: Option<&str>,
    options: Option<&str>,
) -> ExecutionCommand {
    let target_arg = target.unwrap_or("<export-path>");
    let mut missing = Vec::new();
    if target.is_none() {
        missing.push("NFS export path");
    }
    if client.is_none() {
        missing.push("NFS client selector");
    }
    if options.is_none() {
        missing.push("NFS export options");
    }

    match (target, client, options) {
        (Some(_), Some(client), Some(options)) => command_vec(
            vec![
                "exportfs".to_string(),
                "-i".to_string(),
                "-o".to_string(),
                options.to_string(),
                format!("{client}:{target_arg}"),
            ],
            true,
            "export an existing path to the selected NFS client set with reviewed options",
        ),
        _ => {
            let client_arg = client.unwrap_or("<client>");
            let options_arg = options.unwrap_or("<options>");
            command_vec_with_readiness(
                vec![
                    "exportfs".to_string(),
                    "-i".to_string(),
                    "-o".to_string(),
                    options_arg.to_string(),
                    format!("{client_arg}:{target_arg}"),
                ],
                true,
                CommandReadiness::NeedsDomainImplementation,
                missing,
                "export the path after selecting clients, options, and a local export path",
            )
        }
    }
}

fn nfs_export_property_command(
    target: &str,
    client: Option<&str>,
    property: &str,
    property_value: Option<&str>,
    existing_options: Option<&str>,
) -> ExecutionCommand {
    match property {
        "options" | "nfs.options" | "exportOptions" | "export-options" => {
            nfs_export_create_command(
                path_like_target(target),
                client,
                property_value.or(existing_options),
            )
        }
        _ => command_with_readiness(
            ["exportfs", "-ra"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported NFS export property"],
            "reload NFS exports after selecting a supported export property mapping",
        ),
    }
}

fn luks_device_property_command(
    device: Option<&str>,
    property: &str,
    value: Option<&str>,
) -> ExecutionCommand {
    let device_arg = device.unwrap_or("<luks-device>");
    let mut missing = Vec::new();
    if device.is_none() {
        missing.push("LUKS backing device");
    }
    if value.is_none() {
        missing.push("LUKS property value");
    }

    let Some(value) = value else {
        return command_vec_with_readiness(
            luks_device_property_argv(device_arg, property, "<value>"),
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "update LUKS header identity after selecting a property value",
        );
    };

    let argv = luks_device_property_argv(device_arg, property, value);
    if !missing.is_empty() {
        return command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "update LUKS header identity after selecting the backing device",
        );
    }

    if luks_device_property_argv_is_supported(property) {
        command_vec(argv, true, "update LUKS header identity metadata")
    } else {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            vec!["supported LUKS header property"],
            "update LUKS header identity after selecting a supported property mapping",
        )
    }
}

fn luks_device_property_argv(device: &str, property: &str, value: &str) -> Vec<String> {
    match property {
        "label" | "luks.label" | "cryptsetup.label" => vec![
            "cryptsetup".to_string(),
            "config".to_string(),
            device.to_string(),
            "--label".to_string(),
            value.to_string(),
        ],
        "subsystem" | "luks.subsystem" | "cryptsetup.subsystem" => vec![
            "cryptsetup".to_string(),
            "config".to_string(),
            device.to_string(),
            "--subsystem".to_string(),
            value.to_string(),
        ],
        "uuid" | "luks.uuid" | "cryptsetup.uuid" => vec![
            "cryptsetup".to_string(),
            "luksUUID".to_string(),
            device.to_string(),
            "--uuid".to_string(),
            value.to_string(),
        ],
        _ => vec![
            "<luks-property-tool>".to_string(),
            device.to_string(),
            property.to_string(),
            value.to_string(),
        ],
    }
}

fn luks_device_property_argv_is_supported(property: &str) -> bool {
    matches!(
        property,
        "label"
            | "luks.label"
            | "cryptsetup.label"
            | "subsystem"
            | "luks.subsystem"
            | "cryptsetup.subsystem"
            | "uuid"
            | "luks.uuid"
            | "cryptsetup.uuid"
    )
}

fn export_target_path(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .target
        .as_deref()
        .and_then(path_like_target)
        .or_else(|| action.context.name.as_deref().and_then(path_like_target))
}

fn path_like_target(target: &str) -> Option<&str> {
    target.starts_with('/').then_some(target)
}

fn nfs_mount_target_path(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .mountpoint
        .as_deref()
        .and_then(path_like_target)
        .or_else(|| action.context.target.as_deref().and_then(path_like_target))
        .or_else(|| action.context.name.as_deref().and_then(path_like_target))
}

fn filesystem_mountpoint(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .mountpoint
        .as_deref()
        .and_then(path_like_target)
        .or_else(|| action.context.target.as_deref().and_then(path_like_target))
        .or_else(|| action.context.name.as_deref().and_then(path_like_target))
}

fn filesystem_findmnt_command(mountpoint: Option<&str>) -> ExecutionCommand {
    match mountpoint {
        Some(mountpoint) => command(
            ["findmnt", "--json", mountpoint],
            false,
            "inspect the filesystem mount after selecting the mountpoint",
        ),
        None => command_with_readiness(
            ["findmnt", "--json", "<mountpoint>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["mountpoint path"],
            "inspect the filesystem mount after selecting the mountpoint",
        ),
    }
}

fn filesystem_remount_command(mountpoint: Option<&str>, options: Option<&str>) -> ExecutionCommand {
    let mountpoint_arg = mountpoint.unwrap_or("<mountpoint>");
    let remount_options = options
        .filter(|options| !options.is_empty())
        .map(|options| format!("remount,{options}"))
        .unwrap_or_else(|| "remount".to_string());

    match mountpoint {
        Some(_) => command_vec(
            vec![
                "mount".to_string(),
                "-o".to_string(),
                remount_options,
                mountpoint_arg.to_string(),
            ],
            true,
            "remount the filesystem path with the reviewed options",
        ),
        None => command_vec_with_readiness(
            vec![
                "mount".to_string(),
                "-o".to_string(),
                remount_options,
                mountpoint_arg.to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["mountpoint path"],
            "remount the filesystem path after selecting the mountpoint",
        ),
    }
}

fn nfs_mount_create_command(
    source: Option<&str>,
    mountpoint: Option<&str>,
    fs_type: Option<&str>,
    options: Option<&str>,
) -> ExecutionCommand {
    let source_arg = source.unwrap_or("<nfs-source>");
    let mountpoint_arg = mountpoint.unwrap_or("<mountpoint>");
    let fs_type_arg = fs_type.unwrap_or("nfs4");
    let mut missing = Vec::new();
    if source.is_none() {
        missing.push("NFS source");
    }
    if mountpoint.is_none() {
        missing.push("mountpoint path");
    }

    if source.is_some() && mountpoint.is_some() {
        let mut argv = vec![
            "mount".to_string(),
            "-t".to_string(),
            fs_type_arg.to_string(),
        ];
        if let Some(options) = options {
            argv.push("-o".to_string());
            argv.push(options.to_string());
        }
        argv.push(source_arg.to_string());
        argv.push(mountpoint_arg.to_string());
        command_vec(
            argv,
            true,
            "mount the reviewed NFS source at the selected mountpoint",
        )
    } else {
        let mut argv = vec![
            "mount".to_string(),
            "-t".to_string(),
            fs_type_arg.to_string(),
        ];
        if let Some(options) = options {
            argv.push("-o".to_string());
            argv.push(options.to_string());
        }
        argv.push(source_arg.to_string());
        argv.push(mountpoint_arg.to_string());
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "mount the NFS source after selecting a source and mountpoint",
        )
    }
}

fn nfs_mount_findmnt_command(mountpoint: Option<&str>) -> ExecutionCommand {
    match mountpoint {
        Some(mountpoint) => command(
            ["findmnt", "--json", mountpoint],
            false,
            "inspect the NFS mount before unmounting",
        ),
        None => command_with_readiness(
            ["findmnt", "--json", "<mountpoint>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["mountpoint path"],
            "inspect the NFS mount after selecting the mountpoint",
        ),
    }
}

fn nfs_mount_stats_command(mountpoint: Option<&str>) -> ExecutionCommand {
    match mountpoint {
        Some(mountpoint) => command(
            ["nfsstat", "-m", mountpoint],
            false,
            "inspect NFS client mount statistics and negotiated options",
        ),
        None => command_with_readiness(
            ["nfsstat", "-m", "<mountpoint>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["mountpoint path"],
            "inspect NFS client mount statistics after selecting the mountpoint",
        ),
    }
}

fn nfs_mount_destroy_command(mountpoint: Option<&str>) -> ExecutionCommand {
    match mountpoint {
        Some(mountpoint) => command(
            ["umount", mountpoint],
            true,
            "unmount the reviewed NFS client mount without touching remote data",
        ),
        None => command_with_readiness(
            ["umount", "<mountpoint>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["mountpoint path"],
            "unmount the NFS client mount after selecting the mountpoint",
        ),
    }
}

fn nfs_mount_remount_command(mountpoint: Option<&str>, options: Option<&str>) -> ExecutionCommand {
    let mountpoint_arg = mountpoint.unwrap_or("<mountpoint>");
    let remount_options = options
        .filter(|options| !options.is_empty())
        .map(|options| format!("remount,{options}"))
        .unwrap_or_else(|| "remount".to_string());

    match mountpoint {
        Some(_) => command_vec(
            vec![
                "mount".to_string(),
                "-o".to_string(),
                remount_options,
                mountpoint_arg.to_string(),
            ],
            true,
            "remount the NFS client path with the reviewed options",
        ),
        None => command_vec_with_readiness(
            vec![
                "mount".to_string(),
                "-o".to_string(),
                remount_options,
                mountpoint_arg.to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["mountpoint path"],
            "remount the NFS client path after selecting the mountpoint",
        ),
    }
}

fn nfs_export_destroy_command(target: Option<&str>, client: Option<&str>) -> ExecutionCommand {
    let target_arg = target.unwrap_or("<export-path>");
    let mut missing = Vec::new();
    if target.is_none() {
        missing.push("NFS export path");
    }
    if client.is_none() {
        missing.push("NFS client selector");
    }

    match (target, client) {
        (Some(_), Some(client)) => command_vec(
            vec![
                "exportfs".to_string(),
                "-u".to_string(),
                format!("{client}:{target_arg}"),
            ],
            true,
            "unexport the reviewed NFS path for the selected client set",
        ),
        _ => {
            let client_arg = client.unwrap_or("<client>");
            command_vec_with_readiness(
                vec![
                    "exportfs".to_string(),
                    "-u".to_string(),
                    format!("{client_arg}:{target_arg}"),
                ],
                true,
                CommandReadiness::NeedsDomainImplementation,
                missing,
                "unexport the path after selecting the client and local export path",
            )
        }
    }
}

fn snapshot_property_command(
    snapshot: &str,
    property: &str,
    tag: Option<&str>,
) -> ExecutionCommand {
    let Some(tag) = tag else {
        return command_with_readiness(
            ["zfs", "hold", "<tag>", snapshot],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["ZFS hold tag"],
            "update a ZFS snapshot hold after selecting the hold tag",
        );
    };
    if !is_zfs_snapshot_name(snapshot) {
        return command_with_readiness(
            ["<snapshot-property-tool>", snapshot, tag],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["ZFS snapshot name"],
            "update snapshot retention with the target-specific snapshot property tool",
        );
    }
    match property {
        "zfs.hold" | "hold" | "holdTag" => command(
            ["zfs", "hold", tag, snapshot],
            true,
            "add a ZFS snapshot hold with the reviewed retention tag",
        ),
        "zfs.releaseHold" | "releaseHold" | "release-hold" => command(
            ["zfs", "release", tag, snapshot],
            true,
            "release a ZFS snapshot hold with the reviewed retention tag",
        ),
        _ => command_with_readiness(
            ["<snapshot-property-tool>", snapshot, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported snapshot property"],
            "update a snapshot property after selecting a supported domain mapping",
        ),
    }
}

fn snapshot_hold_list_command(snapshot: &str) -> ExecutionCommand {
    if is_zfs_snapshot_name(snapshot) {
        command(
            ["zfs", "holds", snapshot],
            false,
            "verify ZFS snapshot hold tags",
        )
    } else {
        command_with_readiness(
            ["<snapshot-hold-list-tool>", snapshot],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["ZFS snapshot name"],
            "verify snapshot hold state with the target-specific tool",
        )
    }
}

fn zfs_snapshot_list_command(snapshot: &str, note: &str) -> ExecutionCommand {
    command(
        ["zfs", "list", "-t", "snapshot", "-H", "-p", snapshot],
        false,
        note,
    )
}

fn zfs_snapshot_rollback_command(snapshot: &str, recursive: bool) -> ExecutionCommand {
    if recursive {
        command(
            ["zfs", "rollback", "-r", snapshot],
            true,
            "recursively roll back the ZFS dataset after explicit review of newer snapshots",
        )
    } else {
        command(
            ["zfs", "rollback", snapshot],
            true,
            "roll back the ZFS dataset to the reviewed snapshot",
        )
    }
}

fn snapshot_command(
    collection: Option<&str>,
    target: &str,
    snapshot: &str,
    read_only: bool,
) -> ExecutionCommand {
    if is_zfs_snapshot_name(snapshot) {
        command(["zfs", "snapshot", snapshot], true, "create a ZFS snapshot")
    } else if collection == Some("btrfsSubvolumes") || is_btrfs_snapshot_pair(target, snapshot) {
        if read_only {
            command(
                ["btrfs", "subvolume", "snapshot", "-r", target, snapshot],
                true,
                "create a read-only Btrfs subvolume snapshot",
            )
        } else {
            command(
                ["btrfs", "subvolume", "snapshot", target, snapshot],
                true,
                "create a Btrfs subvolume snapshot",
            )
        }
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

fn is_zfs_snapshot_name(snapshot: &str) -> bool {
    let Some((dataset, name)) = snapshot.split_once('@') else {
        return false;
    };
    !dataset.is_empty() && !name.is_empty() && !dataset.starts_with('/')
}

fn zfs_snapshot_dataset(snapshot: &str) -> Option<&str> {
    snapshot.split_once('@').map(|(dataset, _)| dataset)
}

fn is_btrfs_snapshot_pair(target: &str, snapshot: &str) -> bool {
    target.starts_with('/') && snapshot.starts_with('/')
}

fn disk_create_label_command(target: Option<&str>, label: &str) -> ExecutionCommand {
    match target {
        Some(target) => command_vec(
            vec!["parted", "-s", target, "mklabel", label],
            true,
            "create the reviewed disk partition table label",
        ),
        None => command_vec_with_readiness(
            vec!["parted", "-s", "<disk>", "mklabel", label],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["disk path"],
            "create the reviewed disk partition table label after selecting the disk",
        ),
    }
}

fn disk_nix_inspect_command(
    target: Option<&str>,
    placeholder: &'static str,
    missing_input: &'static str,
    description: &'static str,
) -> ExecutionCommand {
    match target {
        Some(target) => command(["disk-nix", "inspect", target], false, description),
        None => command_with_readiness(
            ["disk-nix", "inspect", placeholder],
            false,
            CommandReadiness::NeedsDomainImplementation,
            [missing_input],
            description,
        ),
    }
}

fn partition_target_path(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .target
        .as_deref()
        .filter(|target| target.starts_with('/'))
        .or_else(|| {
            action
                .context
                .name
                .as_deref()
                .filter(|name| name.starts_with('/'))
        })
}

fn disk_target_path(action: &PlannedAction) -> Option<&str> {
    partition_target_path(action)
}

fn partition_rescan_disk(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .device
        .as_deref()
        .or_else(|| disk_target_path(action))
}

fn partition_create_command(
    disk: Option<&str>,
    partition_type: Option<&str>,
    start: Option<&str>,
    end: Option<&str>,
) -> ExecutionCommand {
    let argv = vec![
        "parted",
        "-s",
        disk.unwrap_or("<disk>"),
        "mkpart",
        partition_type.unwrap_or("<partition-type>"),
        start.unwrap_or("<start>"),
        end.unwrap_or("<end>"),
    ];
    let missing = missing_partition_create_inputs(disk, partition_type, start, end);
    if missing.is_empty() {
        command_vec(argv, true, "create a partition in the reviewed free region")
    } else {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "create a partition after resolving the disk, type, and offsets",
        )
    }
}

fn pool_create_devices(device: Option<&str>, devices: &[String]) -> Vec<String> {
    if devices.is_empty() {
        device.into_iter().map(ToString::to_string).collect()
    } else {
        devices.to_vec()
    }
}

fn zfs_pool_create_command(target: &str, devices: &[String]) -> ExecutionCommand {
    if devices.is_empty() {
        command_with_readiness(
            ["zpool", "create", target, "<vdev-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["vdev device or topology"],
            "create a ZFS pool after selecting the vdev topology",
        )
    } else {
        let mut argv = vec![
            "zpool".to_string(),
            "create".to_string(),
            target.to_string(),
        ];
        argv.extend(devices.iter().cloned());
        command_vec(
            argv,
            true,
            "create a ZFS pool on the reviewed vdev device set",
        )
    }
}

fn zfs_pool_import_command(target: &str, read_only: bool) -> ExecutionCommand {
    let mut argv = vec!["zpool".to_string(), "import".to_string()];
    if read_only {
        argv.extend(["-o".to_string(), "readonly=on".to_string()]);
    }
    argv.push(target.to_string());
    command_vec(
        argv,
        true,
        "import the reviewed ZFS pool without recreating it",
    )
}

fn zfs_pool_preflight_commands(devices: &[String]) -> Vec<ExecutionCommand> {
    let inspect_targets: Vec<&str> = devices
        .iter()
        .map(String::as_str)
        .filter(|device| device.starts_with('/'))
        .collect();
    if inspect_targets.is_empty() {
        vec![command_with_readiness(
            ["disk-nix", "inspect", "<vdev-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["vdev device or topology"],
            "inspect vdev device identity before creating the ZFS pool",
        )]
    } else {
        inspect_targets
            .into_iter()
            .map(|device| {
                command_vec(
                    vec!["disk-nix", "inspect", device],
                    false,
                    "inspect vdev device identity before creating the ZFS pool",
                )
            })
            .collect()
    }
}

fn partition_grow_command(
    disk: Option<&str>,
    partition_number: Option<&str>,
    desired_end: Option<&str>,
) -> ExecutionCommand {
    match (disk, partition_number, desired_end) {
        (Some(disk), Some(number), Some(end)) => command_vec(
            vec!["parted", "-s", disk, "resizepart", number, end],
            true,
            "grow the partition to the reviewed end offset after backing capacity is visible",
        ),
        (Some(disk), Some(number), None) => command_vec(
            vec!["growpart", disk, number],
            true,
            "grow the partition to the maximum visible backing capacity",
        ),
        (disk, partition_number, Some(end)) => command_vec_with_readiness(
            vec![
                "parted",
                "-s",
                disk.unwrap_or("<disk>"),
                "resizepart",
                partition_number.unwrap_or("<partition-number>"),
                end,
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_partition_resize_inputs(disk, partition_number),
            "grow a partition to the desired end offset or size after backing capacity is visible",
        ),
        (disk, partition_number, None) => command_vec_with_readiness(
            vec![
                "growpart",
                disk.unwrap_or("<disk>"),
                partition_number.unwrap_or("<partition-number>"),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_partition_resize_inputs(disk, partition_number),
            "grow a partition after backing capacity is visible",
        ),
    }
}

fn missing_partition_resize_inputs(
    disk: Option<&str>,
    partition_number: Option<&str>,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if disk.is_none() {
        missing.push("disk path");
    }
    if partition_number.is_none() {
        missing.push("partition number");
    }
    missing
}

fn missing_partition_create_inputs(
    disk: Option<&str>,
    partition_type: Option<&str>,
    start: Option<&str>,
    end: Option<&str>,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if disk.is_none() {
        missing.push("disk path");
    }
    if partition_type.is_none() {
        missing.push("partition type");
    }
    if start.is_none() {
        missing.push("partition start offset");
    }
    if end.is_none() {
        missing.push("partition end offset");
    }
    missing
}

fn partition_probe_command(disk: Option<&str>) -> ExecutionCommand {
    match disk {
        Some(disk) => command(
            ["partprobe", disk],
            true,
            "ask the kernel to reread the changed partition table",
        ),
        None => command_with_readiness(
            ["partprobe", "<disk>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["disk path"],
            "ask the kernel to reread the changed partition table after selecting the disk",
        ),
    }
}

fn partition_table_reread_command(disk: Option<&str>) -> ExecutionCommand {
    match disk {
        Some(disk) => command(
            ["blockdev", "--rereadpt", disk],
            true,
            "force a partition table reread for the reviewed backing disk",
        ),
        None => command_with_readiness(
            ["blockdev", "--rereadpt", "<disk>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["disk path"],
            "force a partition table reread when supported by the block device",
        ),
    }
}

fn disk_parted_machine_list_command(
    disk: Option<&str>,
    description: &'static str,
) -> ExecutionCommand {
    match disk {
        Some(disk) => command(["parted", "-lm", disk], false, description),
        None => command_with_readiness(
            ["parted", "-lm", "<disk>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["disk path"],
            description,
        ),
    }
}

fn swap_target_path(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .target
        .as_deref()
        .filter(|target| target.starts_with('/'))
        .or_else(|| {
            action
                .context
                .device
                .as_deref()
                .filter(|device| device.starts_with('/'))
        })
}

fn swap_command(
    command_name: &'static str,
    target: Option<&str>,
    note: &'static str,
) -> ExecutionCommand {
    match target {
        Some(target) => command([command_name, target], true, note),
        None => command_with_readiness(
            [command_name, "<swap>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["swap target path"],
            note,
        ),
    }
}

fn swap_blkid_command(target: Option<&str>, note: &'static str) -> ExecutionCommand {
    match target {
        Some(target) => command(["blkid", target], false, note),
        None => command_with_readiness(
            ["blkid", "<swap>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["swap target path"],
            note,
        ),
    }
}

fn swap_inspect_command(target: Option<&str>, note: &'static str) -> ExecutionCommand {
    disk_nix_inspect_command(target, "<swap>", "swap target path", note)
}

fn swap_inspect_json_command(target: Option<&str>, note: &'static str) -> ExecutionCommand {
    match target {
        Some(target) => command(["disk-nix", "inspect", target, "--json"], false, note),
        None => command_with_readiness(
            ["disk-nix", "inspect", "<swap>", "--json"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["swap target path"],
            note,
        ),
    }
}

fn swap_resize_command(target: Option<&str>, desired_size: Option<&str>) -> ExecutionCommand {
    let Some(target) = target else {
        return command_vec_with_readiness(
            vec![
                "<resize-swap-backing-storage>".to_string(),
                "<swap>".to_string(),
                desired_size.unwrap_or("<size>").to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_swap_resize_inputs(desired_size),
            "resize the swap backing device or file after selecting the target",
        );
    };

    if !target.starts_with("/dev/") {
        return match desired_size {
            Some(size) => command(
                ["fallocate", "--length", size, target],
                true,
                "resize the swap file to the desired length before recreating the signature",
            ),
            None => command_with_readiness(
                ["fallocate", "--length", "<size>", target],
                true,
                CommandReadiness::NeedsDesiredSize,
                ["desired swap file size"],
                "resize the swap file after selecting the desired size",
            ),
        };
    }

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

fn missing_swap_resize_inputs(desired_size: Option<&str>) -> Vec<&'static str> {
    let mut missing = vec!["swap target path"];
    if desired_size.is_none() {
        missing.push("desired swap size");
    }
    missing.push("backing storage domain");
    missing
}

fn luks_backing_inspect_command(device: Option<&str>, note: &str) -> ExecutionCommand {
    match device {
        Some(device) => command(["disk-nix", "inspect", device], false, note),
        None => command_with_readiness(
            ["disk-nix", "inspect", "<device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["LUKS backing device"],
            note,
        ),
    }
}

fn luks_is_luks_command(device: Option<&str>) -> ExecutionCommand {
    match device {
        Some(device) => command(
            ["cryptsetup", "isLuks", device],
            false,
            "verify the backing device has a LUKS header",
        ),
        None => command_with_readiness(
            ["cryptsetup", "isLuks", "<device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["LUKS backing device"],
            "verify the backing device has a LUKS header after selecting it",
        ),
    }
}

fn luks_format_command(device: Option<&str>) -> ExecutionCommand {
    match device {
        Some(device) => command(
            ["cryptsetup", "luksFormat", device],
            true,
            "create a LUKS container on the target device",
        ),
        None => command_with_readiness(
            ["cryptsetup", "luksFormat", "<device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["LUKS backing device"],
            "create a LUKS container after selecting the backing device",
        ),
    }
}

fn luks_open_command(device: Option<&str>, mapper: &str, note: &str) -> ExecutionCommand {
    match device {
        Some(device) => command_vec(vec!["cryptsetup", "open", device, mapper], true, note),
        None => command_vec_with_readiness(
            vec!["cryptsetup", "open", "<device>", mapper],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["LUKS backing device"],
            note,
        ),
    }
}

fn luks_keyslot_device(action: &PlannedAction) -> Option<&str> {
    action.context.device.as_deref().or(action
        .context
        .target
        .as_deref()
        .filter(|target| target.starts_with('/')))
}

fn luks_keyslot_id(action: &PlannedAction) -> Option<&str> {
    action.context.key_slot.as_deref().or_else(|| {
        action
            .context
            .name
            .as_deref()
            .and_then(|name| name.rsplit_once(':').map(|(_, slot)| slot).or(Some(name)))
            .filter(|slot| slot.chars().all(|character| character.is_ascii_digit()))
    })
}

fn luks_new_key_file(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .new_key_file
        .as_deref()
        .or(action.context.key_file.as_deref())
}

fn luks_token_device(action: &PlannedAction) -> Option<&str> {
    action.context.device.as_deref().or(action
        .context
        .target
        .as_deref()
        .filter(|target| target.starts_with('/')))
}

fn luks_token_id(action: &PlannedAction) -> Option<&str> {
    action.context.token_id.as_deref().or_else(|| {
        action
            .context
            .name
            .as_deref()
            .and_then(|name| name.rsplit_once(':').map(|(_, token)| token).or(Some(name)))
            .filter(|token| token.chars().all(|character| character.is_ascii_digit()))
    })
}

fn luks_token_file(action: &PlannedAction) -> Option<&str> {
    action.context.token_file.as_deref()
}

fn luks_dump_command(device: Option<&str>, note: &'static str) -> ExecutionCommand {
    match device {
        Some(device) => command(["cryptsetup", "luksDump", device], false, note),
        None => command_with_readiness(
            ["cryptsetup", "luksDump", "<luks-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["LUKS backing device"],
            note,
        ),
    }
}

fn luks_add_key_command(
    device: Option<&str>,
    key_slot: Option<&str>,
    new_key_file: Option<&str>,
) -> ExecutionCommand {
    let mut argv = vec!["cryptsetup".to_string(), "luksAddKey".to_string()];
    if let Some(key_slot) = key_slot {
        argv.extend(["--key-slot".to_string(), key_slot.to_string()]);
    }
    argv.push(device.unwrap_or("<luks-device>").to_string());
    argv.push(new_key_file.unwrap_or("<new-key-file>").to_string());

    let missing = missing_luks_add_key_inputs(device, new_key_file);
    if missing.is_empty() {
        command_vec(argv, true, "add reviewed key material to the LUKS header")
    } else {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "add LUKS key material after selecting the device and new key file",
        )
    }
}

fn luks_kill_slot_command(device: Option<&str>, key_slot: Option<&str>) -> ExecutionCommand {
    match (device, key_slot) {
        (Some(device), Some(key_slot)) => command(
            ["cryptsetup", "luksKillSlot", device, key_slot],
            true,
            "remove the reviewed LUKS keyslot after alternate unlock paths are verified",
        ),
        (device, key_slot) => command_vec_with_readiness(
            vec![
                "cryptsetup".to_string(),
                "luksKillSlot".to_string(),
                device.unwrap_or("<luks-device>").to_string(),
                key_slot.unwrap_or("<key-slot>").to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_luks_keyslot_inputs(device, key_slot),
            "remove LUKS keyslot after selecting the device and slot number",
        ),
    }
}

fn luks_change_key_command(
    device: Option<&str>,
    key_slot: Option<&str>,
    old_key_file: Option<&str>,
    new_key_file: Option<&str>,
) -> ExecutionCommand {
    let mut argv = vec!["cryptsetup".to_string(), "luksChangeKey".to_string()];
    if let Some(key_slot) = key_slot {
        argv.extend(["--key-slot".to_string(), key_slot.to_string()]);
    }
    if let Some(old_key_file) = old_key_file {
        argv.extend(["--key-file".to_string(), old_key_file.to_string()]);
    }
    argv.push(device.unwrap_or("<luks-device>").to_string());
    argv.push(new_key_file.unwrap_or("<new-key-file>").to_string());

    let missing = missing_luks_add_key_inputs(device, new_key_file);
    if missing.is_empty() {
        command_vec(
            argv,
            true,
            "change LUKS key material for the reviewed keyslot",
        )
    } else {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "change LUKS key material after selecting the device and replacement key file",
        )
    }
}

fn luks_token_import_command(
    device: Option<&str>,
    token_id: Option<&str>,
    token_file: Option<&str>,
) -> ExecutionCommand {
    let mut argv = vec![
        "cryptsetup".to_string(),
        "token".to_string(),
        "import".to_string(),
    ];
    if let Some(token_id) = token_id {
        argv.extend(["--token-id".to_string(), token_id.to_string()]);
    }
    argv.extend([
        "--json-file".to_string(),
        token_file.unwrap_or("<token-json-file>").to_string(),
        device.unwrap_or("<luks-device>").to_string(),
    ]);

    let missing = missing_luks_token_import_inputs(device, token_file);
    if missing.is_empty() {
        command_vec(argv, true, "import reviewed LUKS token metadata")
    } else {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "import LUKS token after selecting the device and token JSON file",
        )
    }
}

fn luks_token_remove_command(device: Option<&str>, token_id: Option<&str>) -> ExecutionCommand {
    match (device, token_id) {
        (Some(device), Some(token_id)) => command(
            [
                "cryptsetup",
                "token",
                "remove",
                "--token-id",
                token_id,
                device,
            ],
            true,
            "remove the reviewed LUKS token after alternate unlock paths are verified",
        ),
        (device, token_id) => command_vec_with_readiness(
            vec![
                "cryptsetup".to_string(),
                "token".to_string(),
                "remove".to_string(),
                "--token-id".to_string(),
                token_id.unwrap_or("<token-id>").to_string(),
                device.unwrap_or("<luks-device>").to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_luks_token_remove_inputs(device, token_id),
            "remove LUKS token after selecting the device and token id",
        ),
    }
}

fn missing_luks_add_key_inputs(
    device: Option<&str>,
    new_key_file: Option<&str>,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if device.is_none() {
        missing.push("LUKS backing device");
    }
    if new_key_file.is_none() {
        missing.push("new key file");
    }
    missing
}

fn missing_luks_keyslot_inputs(device: Option<&str>, key_slot: Option<&str>) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if device.is_none() {
        missing.push("LUKS backing device");
    }
    if key_slot.is_none() {
        missing.push("LUKS keyslot number");
    }
    missing
}

fn missing_luks_token_import_inputs(
    device: Option<&str>,
    token_file: Option<&str>,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if device.is_none() {
        missing.push("LUKS backing device");
    }
    if token_file.is_none() {
        missing.push("token JSON file");
    }
    missing
}

fn missing_luks_token_remove_inputs(
    device: Option<&str>,
    token_id: Option<&str>,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if device.is_none() {
        missing.push("LUKS backing device");
    }
    if token_id.is_none() {
        missing.push("LUKS token id");
    }
    missing
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

fn vdo_create_command(
    target: &str,
    device: Option<&str>,
    desired_size: Option<&str>,
) -> ExecutionCommand {
    match (device, desired_size) {
        (Some(device), Some(size)) => command_vec(
            vec![
                "vdo",
                "create",
                "--name",
                target,
                "--device",
                device,
                "--vdoLogicalSize",
                size,
            ],
            true,
            "create a VDO volume on the reviewed backing device",
        ),
        (Some(device), None) => command_vec_with_readiness(
            vec![
                "vdo",
                "create",
                "--name",
                target,
                "--device",
                device,
                "--vdoLogicalSize",
                "<size>",
            ],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired VDO logical size"],
            "create a VDO volume after selecting the logical size",
        ),
        (None, Some(size)) => command_vec_with_readiness(
            vec![
                "vdo",
                "create",
                "--name",
                target,
                "--device",
                "<backing-device>",
                "--vdoLogicalSize",
                size,
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["backing device"],
            "create a VDO volume after selecting the backing device",
        ),
        (None, None) => command_with_readiness(
            [
                "vdo",
                "create",
                "--name",
                target,
                "--device",
                "<backing-device>",
                "--vdoLogicalSize",
                "<size>",
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["backing device", "desired VDO logical size"],
            "create a VDO volume after selecting backing device and logical size",
        ),
    }
}

fn vdo_backing_inspect_command(device: Option<&str>) -> ExecutionCommand {
    match device {
        Some(device) => command(
            ["disk-nix", "inspect", device],
            false,
            "inspect backing device before creating the VDO volume",
        ),
        None => command_with_readiness(
            ["disk-nix", "inspect", "<backing-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["backing device"],
            "inspect backing device before creating the VDO volume",
        ),
    }
}

fn thin_pool_create_command(target: &str, desired_size: Option<&str>) -> ExecutionCommand {
    let Some((volume_group, thin_pool)) = target.split_once('/') else {
        return command_vec_with_readiness(
            vec![
                "lvcreate".to_string(),
                "--type".to_string(),
                "thin-pool".to_string(),
                "--size".to_string(),
                desired_size.unwrap_or("<size>").to_string(),
                "--name".to_string(),
                "<thin-pool>".to_string(),
                "<volume-group>".to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_lvm_create_inputs(
                "target in volume-group/thin-pool form",
                "desired thin pool size",
                desired_size,
            ),
            "create an LVM thin pool after resolving volume group and pool name",
        );
    };

    match desired_size {
        Some(size) => command_vec(
            vec![
                "lvcreate".to_string(),
                "--type".to_string(),
                "thin-pool".to_string(),
                "--size".to_string(),
                size.to_string(),
                "--name".to_string(),
                thin_pool.to_string(),
                volume_group.to_string(),
            ],
            true,
            "create an LVM thin pool with the desired size",
        ),
        None => command_vec_with_readiness(
            vec![
                "lvcreate".to_string(),
                "--type".to_string(),
                "thin-pool".to_string(),
                "--size".to_string(),
                "<size>".to_string(),
                "--name".to_string(),
                thin_pool.to_string(),
                volume_group.to_string(),
            ],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired thin pool size"],
            "create an LVM thin pool after selecting the desired size",
        ),
    }
}

fn lvm_volume_target_path(target: Option<&str>) -> Option<&str> {
    target.filter(|target| {
        let Some((volume_group, volume)) = target.split_once('/') else {
            return false;
        };
        !volume_group.is_empty() && !volume.is_empty()
    })
}

fn lvm_lvs_report_command(
    target: Option<&str>,
    columns: Option<&str>,
    description: &'static str,
) -> ExecutionCommand {
    match (target, columns) {
        (Some(target), Some(columns)) => command(
            ["lvs", "--reportformat", "json", "-o", columns, target],
            false,
            description,
        ),
        (Some(target), None) => command(
            ["lvs", "--reportformat", "json", target],
            false,
            description,
        ),
        (None, Some(columns)) => command_with_readiness(
            [
                "lvs",
                "--reportformat",
                "json",
                "-o",
                columns,
                "<logical-volume>",
            ],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["target in volume-group/logical-volume form"],
            description,
        ),
        (None, None) => command_with_readiness(
            ["lvs", "--reportformat", "json", "<logical-volume>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["target in volume-group/logical-volume form"],
            description,
        ),
    }
}

fn lvm_logical_volume_extend_command(
    target: Option<&str>,
    desired_size: Option<&str>,
) -> ExecutionCommand {
    match (target, desired_size) {
        (Some(target), Some(size)) => command_vec(
            vec!["lvextend", "--resizefs", "--size", size, target],
            true,
            "grow the logical volume and filesystem to the desired size",
        ),
        (Some(target), None) => command_with_readiness(
            ["lvextend", "--resizefs", "--size", "+<size>", target],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired size delta"],
            "grow the logical volume and filesystem together",
        ),
        (None, desired_size) => command_vec_with_readiness(
            vec![
                "lvextend".to_string(),
                "--resizefs".to_string(),
                "--size".to_string(),
                desired_size.unwrap_or("+<size>").to_string(),
                "<logical-volume>".to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_lvm_resize_inputs(
                "target in volume-group/logical-volume form",
                "desired size delta",
                desired_size,
            ),
            "grow the logical volume and filesystem after resolving the target",
        ),
    }
}

fn thin_pool_extend_command(target: Option<&str>, desired_size: Option<&str>) -> ExecutionCommand {
    match (target, desired_size) {
        (Some(target), Some(size)) => command_vec(
            vec!["lvextend", "--size", size, target],
            true,
            "extend the LVM thin pool data volume to the desired size",
        ),
        (Some(target), None) => command_with_readiness(
            ["lvextend", "--size", "+<size>", target],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired thin pool size or size delta"],
            "extend the LVM thin pool after selecting the desired size",
        ),
        (None, desired_size) => command_vec_with_readiness(
            vec![
                "lvextend".to_string(),
                "--size".to_string(),
                desired_size.unwrap_or("+<size>").to_string(),
                "<thin-pool>".to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_lvm_resize_inputs(
                "target in volume-group/thin-pool form",
                "desired thin pool size or size delta",
                desired_size,
            ),
            "extend the LVM thin pool after resolving the target",
        ),
    }
}

fn missing_lvm_resize_inputs(
    target_input: &'static str,
    size_input: &'static str,
    desired_size: Option<&str>,
) -> Vec<&'static str> {
    let mut missing = vec![target_input];
    if desired_size.is_none() {
        missing.push(size_input);
    }
    missing
}

fn lvm_lvremove_command(
    target: Option<&str>,
    placeholder: &'static str,
    target_input: &'static str,
    description: &'static str,
) -> ExecutionCommand {
    match target {
        Some(target) => command(["lvremove", "--yes", target], true, description),
        None => command_with_readiness(
            ["lvremove", "--yes", placeholder],
            true,
            CommandReadiness::NeedsDomainImplementation,
            [target_input],
            description,
        ),
    }
}

fn lvm_lvrename_command(
    target: Option<&str>,
    rename_to: Option<&str>,
    placeholder: &'static str,
    target_input: &'static str,
    rename_input: &'static str,
    description: &'static str,
) -> ExecutionCommand {
    match (target, rename_to) {
        (Some(target), Some(rename_to)) => {
            command(["lvrename", target, rename_to], true, description)
        }
        (target, rename_to) => command_vec_with_readiness(
            vec![
                "lvrename".to_string(),
                target.unwrap_or(placeholder).to_string(),
                rename_to.unwrap_or("<new-logical-volume>").to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_rename_inputs(target_input, rename_input, target, rename_to),
            description,
        ),
    }
}

fn lvm_lvchange_activate_command(
    target: Option<&str>,
    flag: &'static str,
    placeholder: &'static str,
    target_input: &'static str,
) -> ExecutionCommand {
    let description = if flag == "y" {
        "activate the reviewed LVM logical volume"
    } else {
        "deactivate the reviewed LVM logical volume without deleting data"
    };
    match target {
        Some(target) => command(["lvchange", "--activate", flag, target], true, description),
        None => command_vec_with_readiness(
            vec![
                "lvchange".to_string(),
                "--activate".to_string(),
                flag.to_string(),
                placeholder.to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            [target_input],
            if flag == "y" {
                "activate the LVM logical volume after selecting the target"
            } else {
                "deactivate the LVM logical volume after selecting the target"
            },
        ),
    }
}

fn missing_rename_inputs(
    target_input: &'static str,
    rename_input: &'static str,
    target: Option<&str>,
    rename_to: Option<&str>,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if target.is_none() {
        missing.push(target_input);
    }
    if rename_to.is_none() {
        missing.push(rename_input);
    }
    missing
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

fn lvm_logical_volume_create_command(target: &str, desired_size: Option<&str>) -> ExecutionCommand {
    let Some((volume_group, logical_volume)) = target.split_once('/') else {
        return command_vec_with_readiness(
            vec![
                "lvcreate".to_string(),
                "--size".to_string(),
                desired_size.unwrap_or("<size>").to_string(),
                "--name".to_string(),
                "<logical-volume>".to_string(),
                "<volume-group>".to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_lvm_create_inputs(
                "target in volume-group/logical-volume form",
                "desired logical volume size",
                desired_size,
            ),
            "create an LVM logical volume after resolving volume group and LV name",
        );
    };

    match desired_size {
        Some(size) => command_vec(
            vec![
                "lvcreate".to_string(),
                "--size".to_string(),
                size.to_string(),
                "--name".to_string(),
                logical_volume.to_string(),
                volume_group.to_string(),
            ],
            true,
            "create an LVM logical volume with the desired size",
        ),
        None => command_vec_with_readiness(
            vec![
                "lvcreate".to_string(),
                "--size".to_string(),
                "<size>".to_string(),
                "--name".to_string(),
                logical_volume.to_string(),
                volume_group.to_string(),
            ],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired logical volume size"],
            "create an LVM logical volume after selecting the desired size",
        ),
    }
}

fn missing_lvm_create_inputs(
    target_input: &'static str,
    size_input: &'static str,
    desired_size: Option<&str>,
) -> Vec<&'static str> {
    let mut missing = vec![target_input];
    if desired_size.is_none() {
        missing.push(size_input);
    }
    missing
}

fn lvm_volume_group_create_command(target: &str, device: Option<&str>) -> ExecutionCommand {
    match device {
        Some(device) => command(
            ["vgcreate", target, device],
            true,
            "create an LVM volume group on the reviewed physical volume",
        ),
        None => command_with_readiness(
            ["vgcreate", target, "<physical-volume>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["physical volume device"],
            "create an LVM volume group after selecting the physical volume",
        ),
    }
}

fn lvm_physical_volume_target(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .device
        .as_deref()
        .or(action.context.target.as_deref())
        .or(action.context.name.as_deref())
        .filter(|target| is_path_like(target))
}

fn is_path_like(target: &str) -> bool {
    target.starts_with('/')
}

fn lvm_physical_volume_create_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["pvcreate", target],
            true,
            "create LVM physical volume metadata on the reviewed device",
        ),
        None => command_with_readiness(
            ["pvcreate", "<physical-volume>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["physical volume device"],
            "create LVM physical volume metadata after selecting the device",
        ),
    }
}

fn lvm_physical_volume_resize_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["pvresize", target],
            true,
            "resize the LVM physical volume after backing storage growth",
        ),
        None => command_with_readiness(
            ["pvresize", "<physical-volume>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["physical volume device"],
            "resize the LVM physical volume after selecting the device",
        ),
    }
}

fn lvm_physical_volume_rescan_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["pvscan", "--cache", target],
            true,
            "refresh LVM physical volume metadata cache for the reviewed device",
        ),
        None => command(
            ["pvscan", "--cache"],
            true,
            "refresh the LVM physical volume metadata cache",
        ),
    }
}

fn lvm_physical_volume_remove_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["pvremove", "--yes", target],
            true,
            "remove LVM physical volume metadata from the reviewed device",
        ),
        None => command_with_readiness(
            ["pvremove", "--yes", "<physical-volume>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["physical volume device"],
            "remove LVM physical volume metadata after selecting the device",
        ),
    }
}

fn volume_group_extend_command(target: &str, device: Option<&str>) -> ExecutionCommand {
    match device {
        Some(device) => command(
            ["vgextend", target, device],
            true,
            "extend the LVM volume group with the reviewed physical volume",
        ),
        None => command_with_readiness(
            ["vgextend", target, "<physical-volume>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["physical volume device"],
            "extend the LVM volume group after selecting the physical volume",
        ),
    }
}

fn lvm_volume_group_extend_replacement_command(
    target: &str,
    replacement: Option<&str>,
) -> ExecutionCommand {
    match replacement {
        Some(replacement) => command(
            ["vgextend", target, replacement],
            true,
            "extend the LVM volume group with the replacement physical volume",
        ),
        None => command_with_readiness(
            ["vgextend", target, "<replacement-physical-volume>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["replacement physical volume"],
            "extend the LVM volume group after selecting the replacement physical volume",
        ),
    }
}

fn lvm_physical_volume_move_to_command(
    source: Option<&str>,
    destination: Option<&str>,
) -> ExecutionCommand {
    let source_arg = source.unwrap_or("<physical-volume>");
    let destination_arg = destination.unwrap_or("<replacement-physical-volume>");
    let mut missing = Vec::new();
    if source.is_none() {
        missing.push("physical volume to replace");
    }
    if destination.is_none() {
        missing.push("replacement physical volume");
    }

    if missing.is_empty() {
        command(
            ["pvmove", source_arg, destination_arg],
            true,
            "move allocated extents from the old PV to the replacement PV",
        )
    } else {
        command_vec_with_readiness(
            vec![
                "pvmove".to_string(),
                source_arg.to_string(),
                destination_arg.to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "move extents after selecting the old and replacement physical volumes",
        )
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

fn loop_device_target_path(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .target
        .as_deref()
        .filter(|target| target.starts_with("/dev/loop"))
        .or_else(|| {
            action
                .context
                .name
                .as_deref()
                .filter(|name| name.starts_with("/dev/loop"))
        })
}

fn loop_device_list_command(target: Option<&str>, description: &'static str) -> ExecutionCommand {
    match target {
        Some(target) => command(["losetup", "--json", "--list", target], false, description),
        None => command_with_readiness(
            ["losetup", "--json", "--list", "<loop-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["loop device path"],
            description,
        ),
    }
}

fn loop_device_inspect_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["disk-nix", "inspect", target],
            false,
            "inspect modeled loop device relationships after refresh",
        ),
        None => command_with_readiness(
            ["disk-nix", "inspect", "<loop-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["loop device path"],
            "inspect modeled loop device relationships after selecting the loop path",
        ),
    }
}

fn loop_device_refresh_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["losetup", "-c", target],
            true,
            "refresh the loop device size after backing storage growth",
        ),
        None => command_with_readiness(
            ["losetup", "-c", "<loop-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["loop device path"],
            "refresh the loop device size after backing storage growth",
        ),
    }
}

fn loop_device_detach_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["losetup", "--detach", target],
            true,
            "detach the loop device without deleting the backing file",
        ),
        None => command_with_readiness(
            ["losetup", "--detach", "<loop-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["loop device path"],
            "detach the loop device without deleting the backing file",
        ),
    }
}

fn zvol_create_command(
    target: &str,
    desired_size: Option<&str>,
    property_assignments: &[String],
) -> ExecutionCommand {
    match desired_size {
        Some(size) => {
            let mut argv = zfs_create_argv(property_assignments);
            argv.push("-V".to_string());
            argv.push(size.to_string());
            argv.push(target.to_string());
            command_vec(argv, true, "create a zvol with the desired volume size")
        }
        None => {
            let mut argv = zfs_create_argv(property_assignments);
            argv.push("-V".to_string());
            argv.push("<size>".to_string());
            argv.push(target.to_string());
            command_vec_with_readiness(
                argv,
                true,
                CommandReadiness::NeedsDesiredSize,
                ["desired zvol size"],
                "create a zvol after selecting the desired volume size",
            )
        }
    }
}

fn zfs_dataset_create_command(target: &str, property_assignments: &[String]) -> ExecutionCommand {
    let mut argv = zfs_create_argv(property_assignments);
    argv.push(target.to_string());
    command_vec(argv, true, "create the reviewed ZFS filesystem dataset")
}

fn zfs_create_argv(property_assignments: &[String]) -> Vec<String> {
    let mut argv = vec!["zfs".to_string(), "create".to_string()];
    for assignment in property_assignments {
        argv.push("-o".to_string());
        argv.push(assignment.clone());
    }
    argv
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

fn nvme_controller_target(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .device
        .as_deref()
        .or(action.context.target.as_deref())
        .or(action.context.name.as_deref())
        .filter(|target| is_nvme_controller_path(target))
}

fn is_nvme_controller_path(target: &str) -> bool {
    target
        .strip_prefix("/dev/nvme")
        .is_some_and(|suffix| !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit()))
}

fn nvme_list_namespaces_command(
    controller: Option<&str>,
    description: &'static str,
) -> ExecutionCommand {
    match controller {
        Some(controller) => command(
            [
                "nvme",
                "list-ns",
                controller,
                "--all",
                "--output-format=json",
            ],
            false,
            description,
        ),
        None => command_with_readiness(
            [
                "nvme",
                "list-ns",
                "<nvme-controller>",
                "--all",
                "--output-format=json",
            ],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["NVMe controller path such as /dev/nvme0"],
            description,
        ),
    }
}

fn nvme_create_namespace_command(
    controller: Option<&str>,
    desired_size: Option<&str>,
) -> ExecutionCommand {
    let controller_arg = controller.unwrap_or("<nvme-controller>");
    let size_arg = desired_size.unwrap_or("<size>");
    let argv = vec![
        "nvme",
        "create-ns",
        controller_arg,
        "--nsze-si",
        size_arg,
        "--ncap-si",
        size_arg,
    ];
    match (controller, desired_size) {
        (Some(_), Some(_)) => command_vec(
            argv,
            true,
            "create an NVMe namespace with the reviewed size and capacity",
        ),
        (Some(_), None) => command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired namespace size"],
            "create an NVMe namespace after selecting size and capacity",
        ),
        (None, desired_size) => command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_nvme_namespace_inputs(true, false, false, desired_size.is_none()),
            "create an NVMe namespace after selecting the controller and size",
        ),
    }
}

fn nvme_attach_namespace_command(
    controller: Option<&str>,
    namespace_id: Option<&str>,
    controllers: Option<&str>,
) -> ExecutionCommand {
    let controller_arg = controller.unwrap_or("<nvme-controller>");
    let namespace_arg = namespace_id.unwrap_or("<namespace-id>");
    let controllers_arg = controllers.unwrap_or("<controller-id-list>");
    let argv = vec![
        "nvme",
        "attach-ns",
        controller_arg,
        "--namespace-id",
        namespace_arg,
        "--controllers",
        controllers_arg,
    ];
    if controller.is_some() && namespace_id.is_some() && controllers.is_some() {
        command_vec(
            argv,
            true,
            "attach the reviewed NVMe namespace to controllers",
        )
    } else {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_nvme_namespace_inputs(
                controller.is_none(),
                namespace_id.is_none(),
                controllers.is_none(),
                false,
            ),
            "attach the NVMe namespace after selecting namespace id and controllers",
        )
    }
}

fn nvme_detach_namespace_command(
    controller: Option<&str>,
    namespace_id: Option<&str>,
    controllers: Option<&str>,
) -> ExecutionCommand {
    let controller_arg = controller.unwrap_or("<nvme-controller>");
    let namespace_arg = namespace_id.unwrap_or("<namespace-id>");
    let controllers_arg = controllers.unwrap_or("<controller-id-list>");
    let argv = vec![
        "nvme",
        "detach-ns",
        controller_arg,
        "--namespace-id",
        namespace_arg,
        "--controllers",
        controllers_arg,
    ];
    if controller.is_some() && namespace_id.is_some() && controllers.is_some() {
        command_vec(
            argv,
            true,
            "detach the reviewed NVMe namespace from controllers before deletion",
        )
    } else {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_nvme_namespace_inputs(
                controller.is_none(),
                namespace_id.is_none(),
                controllers.is_none(),
                false,
            ),
            "detach the NVMe namespace after selecting namespace id and controllers",
        )
    }
}

fn nvme_delete_namespace_command(
    controller: Option<&str>,
    namespace_id: Option<&str>,
) -> ExecutionCommand {
    let controller_arg = controller.unwrap_or("<nvme-controller>");
    let namespace_arg = namespace_id.unwrap_or("<namespace-id>");
    let argv = vec![
        "nvme",
        "delete-ns",
        controller_arg,
        "--namespace-id",
        namespace_arg,
    ];
    if controller.is_some() && namespace_id.is_some() {
        command_vec(argv, true, "delete the reviewed NVMe namespace")
    } else {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_nvme_namespace_inputs(
                controller.is_none(),
                namespace_id.is_none(),
                false,
                false,
            ),
            "delete the NVMe namespace after selecting namespace id",
        )
    }
}

fn nvme_namespace_rescan_command(controller: Option<&str>) -> ExecutionCommand {
    match controller {
        Some(controller) => command(
            ["nvme", "ns-rescan", controller],
            true,
            "rescan NVMe namespaces after controller-side changes",
        ),
        None => command_with_readiness(
            ["nvme", "ns-rescan", "<nvme-controller>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["NVMe controller path such as /dev/nvme0"],
            "rescan NVMe namespaces after selecting the controller",
        ),
    }
}

fn missing_nvme_namespace_inputs(
    missing_controller: bool,
    missing_namespace: bool,
    missing_controllers: bool,
    missing_size: bool,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if missing_controller {
        missing.push("NVMe controller path such as /dev/nvme0");
    }
    if missing_namespace {
        missing.push("namespace id");
    }
    if missing_controllers {
        missing.push("controller id list");
    }
    if missing_size {
        missing.push("desired namespace size");
    }
    missing
}

fn md_raid_create_command(
    target: Option<&str>,
    level: Option<&str>,
    devices: &[String],
) -> ExecutionCommand {
    let missing_target = target.is_none();
    let missing_level = level.is_none();
    let missing_devices = devices.is_empty();
    let target = target.unwrap_or("<md-array>");
    let level = level.unwrap_or("<level>");
    let raid_devices = if missing_devices {
        "<member-count>".to_string()
    } else {
        devices.len().to_string()
    };
    let mut argv = vec![
        "mdadm".to_string(),
        "--create".to_string(),
        target.to_string(),
        "--level".to_string(),
        level.to_string(),
        "--raid-devices".to_string(),
        raid_devices,
    ];
    if missing_devices {
        argv.push("<member-device>".to_string());
    } else {
        argv.extend(devices.iter().cloned());
    }

    if missing_target || missing_level || missing_devices {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_md_raid_create_inputs(missing_target, missing_level, missing_devices),
            "create the MD RAID array after selecting level and reviewed member devices",
        )
    } else {
        command_vec(
            argv,
            true,
            "create the reviewed MD RAID array from explicit member devices",
        )
    }
}

fn md_raid_assemble_command(target: Option<&str>, devices: &[String]) -> ExecutionCommand {
    let missing_target = target.is_none();
    let missing_devices = devices.is_empty();
    let target_arg = target.unwrap_or("<md-array>");
    let mut argv = vec![
        "mdadm".to_string(),
        "--assemble".to_string(),
        target_arg.to_string(),
    ];
    if missing_devices {
        argv.push("<member-device>".to_string());
    } else {
        argv.extend(devices.iter().cloned());
    }

    if missing_target || missing_devices {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_md_raid_assemble_inputs(missing_target, missing_devices),
            "assemble the MD RAID array after selecting the array and reviewed member devices",
        )
    } else {
        command_vec(
            argv,
            true,
            "assemble the reviewed MD RAID array from existing member metadata",
        )
    }
}

fn missing_md_raid_assemble_inputs(
    missing_target: bool,
    missing_devices: bool,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if missing_target {
        missing.push("MD array path");
    }
    if missing_devices {
        missing.push("member devices");
    }
    missing
}

fn md_raid_stop_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["mdadm", "--stop", target],
            true,
            "stop the reviewed MD RAID array without removing member metadata",
        ),
        None => command_with_readiness(
            ["mdadm", "--stop", "<md-array>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["MD array path"],
            "stop the MD RAID array after selecting the array path",
        ),
    }
}

fn missing_md_raid_create_inputs(
    missing_target: bool,
    missing_level: bool,
    missing_devices: bool,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if missing_target {
        missing.push("MD array path");
    }
    if missing_level {
        missing.push("RAID level");
    }
    if missing_devices {
        missing.push("member devices");
    }
    missing
}

fn md_raid_grow_command(target: Option<&str>, desired_size: Option<&str>) -> ExecutionCommand {
    let target_arg = target.unwrap_or("<md-array>");
    match (target, desired_size) {
        (Some(_), Some(size)) => command_vec(
            vec!["mdadm", "--grow", target_arg, "--size", size],
            true,
            "grow or reshape the MD RAID array to the desired component size",
        ),
        (Some(_), None) => command_with_readiness(
            ["mdadm", "--grow", target_arg, "--size", "<size-or-max>"],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired MD RAID component size or max"],
            "grow or reshape the MD RAID array after selecting the desired size",
        ),
        (None, desired_size) => command_vec_with_readiness(
            vec![
                "mdadm",
                "--grow",
                target_arg,
                "--size",
                desired_size.unwrap_or("<size-or-max>"),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_md_raid_grow_inputs(target, desired_size),
            "grow or reshape the MD RAID array after selecting the array and desired size",
        ),
    }
}

fn missing_md_raid_grow_inputs(
    target: Option<&str>,
    desired_size: Option<&str>,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if target.is_none() {
        missing.push("MD array path");
    }
    if desired_size.is_none() {
        missing.push("desired MD RAID component size or max");
    }
    missing
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
    use disk_nix_plan::{ActionContext, plan_and_policy_from_json_bytes};

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
    fn filesystem_check_and_repair_render_domain_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "filesystems": {
                  "home": {
                    "mountpoint": "/home",
                    "device": "/dev/disk/by-label/home",
                    "fsType": "ext4",
                    "operation": "check"
                  },
                  "data": {
                    "mountpoint": "/data",
                    "device": "/dev/disk/by-label/data",
                    "fsType": "btrfs",
                    "operation": "repair"
                  },
                  "scratch": {
                    "mountpoint": "/scratch",
                    "device": "/dev/disk/by-label/scratch",
                    "fsType": "xfs",
                    "operation": "check"
                  },
                  "efi": {
                    "mountpoint": "/boot",
                    "device": "/dev/disk/by-partlabel/EFI",
                    "fsType": "vfat",
                    "operation": "check"
                  },
                  "shared": {
                    "mountpoint": "/mnt/shared",
                    "device": "/dev/disk/by-label/Shared",
                    "fsType": "exfat",
                    "operation": "repair"
                  },
                  "windows": {
                    "mountpoint": "/mnt/windows",
                    "device": "/dev/disk/by-label/Windows",
                    "fsType": "ntfs",
                    "operation": "repair"
                  },
                  "mobile": {
                    "mountpoint": "/mnt/mobile",
                    "device": "/dev/disk/by-label/Mobile",
                    "fsType": "f2fs",
                    "operation": "check"
                  },
                  "bulk": {
                    "mountpoint": "/bulk",
                    "device": "/dev/disk/by-label/Bulk",
                    "fsType": "bcachefs",
                    "operation": "repair"
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
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:home:check"
                && step.commands.iter().any(|command| {
                    command.argv == ["e2fsck", "-n", "/dev/disk/by-label/home"]
                        && !command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:data:repair"
                && step.commands.iter().any(|command| {
                    command.argv == ["btrfs", "check", "--repair", "/dev/disk/by-label/data"]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:scratch:check"
                && step.commands.iter().any(|command| {
                    command.argv == ["xfs_repair", "-n", "/dev/disk/by-label/scratch"]
                        && !command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:efi:check"
                && step.commands.iter().any(|command| {
                    command.argv == ["fsck.fat", "-n", "/dev/disk/by-partlabel/EFI"]
                        && !command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:shared:repair"
                && step.commands.iter().any(|command| {
                    command.argv == ["fsck.exfat", "-p", "/dev/disk/by-label/Shared"]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:windows:repair"
                && step.commands.iter().any(|command| {
                    command.argv == ["ntfsfix", "/dev/disk/by-label/Windows"]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:mobile:check"
                && step.commands.iter().any(|command| {
                    command.argv == ["fsck.f2fs", "--dry-run", "/dev/disk/by-label/Mobile"]
                        && !command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:bulk:repair"
                && step.commands.iter().any(|command| {
                    command.argv == ["bcachefs", "fsck", "-y", "/dev/disk/by-label/Bulk"]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "filesystems:home:check"
                && step
                    .checks
                    .iter()
                    .any(|check| check.contains("read-only check completed"))
        }));
    }

    #[test]
    fn filesystem_check_and_repair_require_source_device_for_readiness() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "filesystems": {
                  "home": {
                    "mountpoint": "/home",
                    "fsType": "ext4",
                    "operation": "check"
                  },
                  "data": {
                    "mountpoint": "/data",
                    "fsType": "btrfs",
                    "operation": "repair"
                  },
                  "shared": {
                    "mountpoint": "/mnt/shared",
                    "fsType": "exfat",
                    "operation": "check"
                  },
                  "mobile": {
                    "mountpoint": "/mnt/mobile",
                    "fsType": "f2fs",
                    "operation": "check"
                  },
                  "bulk": {
                    "mountpoint": "/bulk",
                    "fsType": "bcachefs",
                    "operation": "repair"
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
        assert!(!report.command_summary.all_commands_ready());
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:home:check"
                && step.commands.iter().any(|command| {
                    command.argv == ["e2fsck", "-n", "<filesystem-device>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["filesystem source device"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:data:repair"
                && step.commands.iter().any(|command| {
                    command.argv == ["btrfs", "check", "--repair", "<filesystem-device>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["filesystem source device"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:shared:check"
                && step.commands.iter().any(|command| {
                    command.argv == ["fsck.exfat", "-n", "<filesystem-device>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["filesystem source device"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:mobile:check"
                && step.commands.iter().any(|command| {
                    command.argv == ["fsck.f2fs", "--dry-run", "<filesystem-device>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["filesystem source device"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:bulk:repair"
                && step.commands.iter().any(|command| {
                    command.argv == ["bcachefs", "fsck", "-y", "<filesystem-device>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["filesystem source device"]
                })
        }));
    }

    #[test]
    fn desired_sizes_and_devices_drive_resize_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "filesystems": {
                  "home": {
                    "mountpoint": "/home",
                    "fsType": "btrfs",
                    "resizePolicy": "grow-only",
                    "desiredSize": "750GiB"
                  },
                  "srv": {
                    "mountpoint": "/srv",
                    "device": "/dev/disk/by-label/srv",
                    "fsType": "ext4",
                    "resizePolicy": "grow-only",
                    "desiredSize": "100G"
                  },
                  "var": {
                    "mountpoint": "/var",
                    "fsType": "ext4",
                    "resizePolicy": "grow-only",
                    "desiredSize": "50G"
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
        assert_eq!(report.command_summary.needs_domain_implementation_count, 1);
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv == ["btrfs", "filesystem", "resize", "750GiB", "/home"]
                    && command.readiness == CommandReadiness::Ready
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystem:srv:grow"
                && step.commands.iter().any(|command| {
                    command.argv == ["resize2fs", "/dev/disk/by-label/srv", "100G"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystem:var:grow"
                && step.commands.iter().any(|command| {
                    command.argv == ["resize2fs", "<filesystem-device>", "50G"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["filesystem source device"]
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
    fn filesystem_shrink_renderer_uses_domain_commands() {
        let btrfs_action = PlannedAction {
            id: "filesystem:data:shrink".to_string(),
            description: "shrink btrfs data".to_string(),
            operation: Operation::Shrink,
            risk: RiskClass::PotentialDataLoss,
            destructive: false,
            context: ActionContext {
                collection: Some("filesystems".to_string()),
                name: Some("data".to_string()),
                target: Some("/data".to_string()),
                fs_type: Some("btrfs".to_string()),
                desired_size: Some("750GiB".to_string()),
                ..ActionContext::default()
            },
            advice: None,
        };
        let ext_action = PlannedAction {
            id: "filesystem:home:shrink".to_string(),
            description: "shrink ext home".to_string(),
            operation: Operation::Shrink,
            risk: RiskClass::PotentialDataLoss,
            destructive: false,
            context: ActionContext {
                collection: Some("filesystems".to_string()),
                name: Some("home".to_string()),
                target: Some("/home".to_string()),
                device: Some("/dev/disk/by-label/home".to_string()),
                fs_type: Some("ext4".to_string()),
                desired_size: Some("100G".to_string()),
                ..ActionContext::default()
            },
            advice: None,
        };
        let ext_mountpoint_action = PlannedAction {
            id: "filesystem:srv:shrink".to_string(),
            description: "shrink ext srv".to_string(),
            operation: Operation::Shrink,
            risk: RiskClass::PotentialDataLoss,
            destructive: false,
            context: ActionContext {
                collection: Some("filesystems".to_string()),
                name: Some("srv".to_string()),
                target: Some("/srv".to_string()),
                fs_type: Some("ext4".to_string()),
                desired_size: Some("50G".to_string()),
                ..ActionContext::default()
            },
            advice: None,
        };
        let xfs_action = PlannedAction {
            id: "filesystem:scratch:shrink".to_string(),
            description: "shrink xfs scratch".to_string(),
            operation: Operation::Shrink,
            risk: RiskClass::Unsupported,
            destructive: false,
            context: ActionContext {
                collection: Some("filesystems".to_string()),
                name: Some("scratch".to_string()),
                target: Some("/scratch".to_string()),
                fs_type: Some("xfs".to_string()),
                desired_size: Some("500G".to_string()),
                ..ActionContext::default()
            },
            advice: None,
        };

        let (btrfs_commands, btrfs_notes, btrfs_manual_review) = commands_for_action(&btrfs_action);
        let (ext_commands, ext_notes, ext_manual_review) = commands_for_action(&ext_action);
        let (ext_mountpoint_commands, _, _) = commands_for_action(&ext_mountpoint_action);
        let (xfs_commands, _, xfs_manual_review) = commands_for_action(&xfs_action);

        assert!(btrfs_manual_review);
        assert!(btrfs_commands.iter().any(|command| {
            command.argv == ["btrfs", "filesystem", "usage", "-b", "/data"] && !command.mutates
        }));
        assert!(btrfs_commands.iter().any(|command| {
            command.argv == ["btrfs", "filesystem", "resize", "750GiB", "/data"]
                && command.mutates
                && command.readiness == CommandReadiness::Ready
        }));
        assert!(
            btrfs_notes
                .iter()
                .any(|note| note.contains("backups or snapshots"))
        );

        assert!(ext_manual_review);
        assert!(ext_commands.iter().any(|command| {
            command.argv
                == [
                    "findmnt",
                    "--noheadings",
                    "--output",
                    "SOURCE,FSTYPE,SIZE,USED,AVAIL",
                    "--target",
                    "/home",
                ]
                && !command.mutates
        }));
        assert!(ext_commands.iter().any(|command| {
            command.argv == ["umount", "/home"]
                && command.mutates
                && command.readiness == CommandReadiness::Ready
        }));
        assert!(ext_commands.iter().any(|command| {
            command.argv == ["e2fsck", "-f", "/dev/disk/by-label/home"]
                && command.mutates
                && command.readiness == CommandReadiness::Ready
        }));
        assert!(ext_commands.iter().any(|command| {
            command.argv == ["resize2fs", "/dev/disk/by-label/home", "100G"]
                && command.mutates
                && command.readiness == CommandReadiness::Ready
        }));
        assert!(ext_mountpoint_commands.iter().any(|command| {
            command.argv == ["resize2fs", "<filesystem-device>", "50G"]
                && command.mutates
                && command.readiness == CommandReadiness::NeedsDomainImplementation
                && command.unresolved_inputs == ["filesystem source device"]
        }));
        assert!(
            ext_notes
                .iter()
                .any(|note| note.contains("migrate-to-smaller-filesystem"))
        );

        assert!(xfs_manual_review);
        assert!(xfs_commands.iter().any(|command| {
            command.argv == ["<migrate-to-smaller-filesystem>", "/scratch"]
                && command.readiness == CommandReadiness::ManualOnly
                && command.unresolved_inputs == ["replacement filesystem", "migration plan"]
        }));
    }

    #[test]
    fn btrfs_filesystem_device_removal_stays_blocked_by_apply_policy() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "filesystems": {
                  "data": {
                    "mountpoint": "/data",
                    "fsType": "btrfs",
                    "removeDevices": ["/dev/disk/by-id/old-btrfs-device"]
                  }
                }
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::Blocked);
        assert_eq!(report.command_summary.step_count, 1);
        assert!(
            !report.command_plan.iter().any(|step| {
                step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "btrfs",
                            "device",
                            "remove",
                            "/dev/disk/by-id/old-btrfs-device",
                            "/data",
                        ]
                })
            }),
            "potential-data-loss Btrfs device removal remains blocked by apply policy"
        );
        assert!(report.verification_plan.iter().all(|step| {
            step.action_id != "filesystems:data:remove-device:/dev/disk/by-id/old-btrfs-device"
        }));
    }

    #[test]
    fn btrfs_filesystem_device_removal_renderer_uses_btrfs_commands() {
        let action = PlannedAction {
            id: "filesystems:data:remove-device:/dev/disk/by-id/old-btrfs-device".to_string(),
            description: "remove old Btrfs device".to_string(),
            operation: Operation::RemoveDevice,
            risk: RiskClass::PotentialDataLoss,
            destructive: false,
            context: ActionContext {
                collection: Some("filesystems".to_string()),
                name: Some("data".to_string()),
                target: Some("/data".to_string()),
                device: Some("/dev/disk/by-id/old-btrfs-device".to_string()),
                fs_type: Some("btrfs".to_string()),
                ..ActionContext::default()
            },
            advice: None,
        };

        let (commands, notes, requires_manual_review) = commands_for_action(&action);
        let (verification_commands, verification_checks) = verification_for_action(&action);

        assert!(requires_manual_review);
        assert!(
            notes
                .iter()
                .any(|note| note.contains("remaining data and metadata space are sufficient"))
        );
        assert!(commands.iter().any(|command| {
            command.argv == ["btrfs", "filesystem", "usage", "-b", "/data"] && !command.mutates
        }));
        assert!(commands.iter().any(|command| {
            command.argv
                == [
                    "btrfs",
                    "device",
                    "remove",
                    "/dev/disk/by-id/old-btrfs-device",
                    "/data",
                ]
                && command.mutates
                && command.readiness == CommandReadiness::Ready
        }));
        assert!(verification_commands.iter().any(|command| {
            command.argv == ["btrfs", "filesystem", "usage", "-b", "/data"] && !command.mutates
        }));
        assert!(
            verification_checks
                .iter()
                .any(|check| check.contains("Btrfs device list matches desired topology"))
        );
    }

    #[test]
    fn btrfs_filesystem_label_property_is_ready() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "filesystems": {
                  "data": {
                  "mountpoint": "/data",
                  "fsType": "btrfs",
                  "properties": {
                    "label": "bulk-data"
                  }
                }
              }
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:data:set-property:label"
                && step.commands.iter().any(|command| {
                    command.argv == ["btrfs", "filesystem", "label", "/data", "bulk-data"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_summary.ready_count >= 3);
        assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
    }

    #[test]
    fn ext_filesystem_label_uses_declared_device() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "filesystems": {
                  "home": {
                    "mountpoint": "/home",
                    "device": "/dev/disk/by-label/home-old",
                    "fsType": "ext4",
                    "properties": {
                      "label": "home-new"
                    }
                  },
                  "missing-device": {
                    "mountpoint": "/srv",
                    "fsType": "ext4",
                    "properties": {
                      "label": "srv-new"
                    }
                  }
                }
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:home:set-property:label"
                && step.commands.iter().any(|command| {
                    command.argv == ["e2label", "/dev/disk/by-label/home-old", "home-new"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:missing-device:set-property:label"
                && step.commands.iter().any(|command| {
                    command.argv == ["e2label", "<filesystem-device>", "srv-new"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["filesystem source device"]
                })
        }));
    }

    #[test]
    fn xfs_filesystem_label_uses_declared_device() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "filesystems": {
                  "scratch": {
                    "mountpoint": "/scratch",
                    "device": "/dev/disk/by-label/scratch-old",
                    "fsType": "xfs",
                    "properties": {
                      "label": "scratch-new"
                    }
                  },
                  "missing-device": {
                    "mountpoint": "/archive",
                    "fsType": "xfs",
                    "properties": {
                      "xfs.label": "archive-new"
                    }
                  }
                }
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:scratch:set-property:label"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "xfs_admin",
                            "-L",
                            "scratch-new",
                            "/dev/disk/by-label/scratch-old",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:missing-device:set-property:xfs.label"
                && step.commands.iter().any(|command| {
                    command.argv == ["xfs_admin", "-L", "archive-new", "<filesystem-device>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["filesystem source device"]
                })
        }));
    }

    #[test]
    fn fat_filesystem_properties_use_fatlabel() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "filesystems": {
                  "efi": {
                    "mountpoint": "/boot",
                    "device": "/dev/disk/by-partlabel/EFI",
                    "fsType": "vfat",
                    "properties": {
                      "label": "NIXBOOT",
                      "vfat.uuid": "a1b2-c3d4"
                    }
                  },
                  "missing-device": {
                    "mountpoint": "/firmware",
                    "fsType": "vfat",
                    "properties": {
                      "volume-id": "deadbeef"
                    }
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
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:efi:set-property:label"
                && step.commands.iter().any(|command| {
                    command.argv == ["fatlabel", "/dev/disk/by-partlabel/EFI", "NIXBOOT"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:efi:set-property:vfat.uuid"
                && step.commands.iter().any(|command| {
                    command.argv == ["fatlabel", "-i", "/dev/disk/by-partlabel/EFI", "A1B2C3D4"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:missing-device:set-property:volume-id"
                && step.commands.iter().any(|command| {
                    command.argv == ["fatlabel", "-i", "<filesystem-device>", "DEADBEEF"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["filesystem source device"]
                })
        }));
    }

    #[test]
    fn ntfs_filesystem_properties_use_ntfslabel() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "filesystems": {
                  "windows": {
                    "mountpoint": "/mnt/windows",
                    "device": "/dev/disk/by-label/Windows",
                    "fsType": "ntfs",
                    "properties": {
                      "label": "Windows",
                      "ntfs.uuid": "01234567-89abcdef"
                    }
                  },
                  "missing-device": {
                    "mountpoint": "/mnt/media",
                    "fsType": "ntfs",
                    "properties": {
                      "volume-serial": "fedcba98-76543210"
                    }
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
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:windows:set-property:label"
                && step.commands.iter().any(|command| {
                    command.argv == ["ntfslabel", "/dev/disk/by-label/Windows", "Windows"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:windows:set-property:ntfs.uuid"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "ntfslabel",
                            "--new-serial=0123456789ABCDEF",
                            "/dev/disk/by-label/Windows",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:missing-device:set-property:volume-serial"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "ntfslabel",
                            "--new-serial=FEDCBA9876543210",
                            "<filesystem-device>",
                        ]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["filesystem source device"]
                })
        }));
    }

    #[test]
    fn exfat_filesystem_properties_use_exfatlabel() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "filesystems": {
                  "shared": {
                    "mountpoint": "/mnt/shared",
                    "device": "/dev/disk/by-label/Shared",
                    "fsType": "exfat",
                    "properties": {
                      "label": "Shared",
                      "exfat.uuid": "a1b2-c3d4"
                    }
                  },
                  "missing-device": {
                    "mountpoint": "/mnt/camera",
                    "fsType": "exfat",
                    "properties": {
                      "volume-serial": "deadbeef"
                    }
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
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:shared:set-property:label"
                && step.commands.iter().any(|command| {
                    command.argv == ["exfatlabel", "/dev/disk/by-label/Shared", "Shared"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:shared:set-property:exfat.uuid"
                && step.commands.iter().any(|command| {
                    command.argv == ["exfatlabel", "-i", "/dev/disk/by-label/Shared", "A1B2C3D4"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:missing-device:set-property:volume-serial"
                && step.commands.iter().any(|command| {
                    command.argv == ["exfatlabel", "-i", "<filesystem-device>", "DEADBEEF"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["filesystem source device"]
                })
        }));
    }

    #[test]
    fn filesystem_uuid_updates_render_domain_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "filesystems": {
                  "home": {
                    "mountpoint": "/home",
                    "device": "/dev/disk/by-label/home",
                    "fsType": "ext4",
                    "properties": {
                      "ext.uuid": "11111111-2222-3333-4444-555555555555"
                    }
                  },
                  "scratch": {
                    "mountpoint": "/scratch",
                    "device": "/dev/disk/by-label/scratch",
                    "fsType": "xfs",
                    "properties": {
                      "filesystem.uuid": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
                    }
                  },
                  "data": {
                    "mountpoint": "/data",
                    "device": "/dev/disk/by-label/data",
                    "fsType": "btrfs",
                    "properties": {
                      "btrfs.uuid": "bbbbbbbb-1111-2222-3333-cccccccccccc"
                    }
                  },
                  "missing-device": {
                    "mountpoint": "/archive",
                    "fsType": "xfs",
                    "properties": {
                      "uuid": "ffffffff-1111-2222-3333-444444444444"
                    }
                  },
                  "missing-btrfs": {
                    "mountpoint": "/missing-btrfs",
                    "fsType": "btrfs",
                    "properties": {
                      "uuid": "cccccccc-1111-2222-3333-dddddddddddd"
                    }
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
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:home:set-property:ext.uuid"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "tune2fs",
                            "-U",
                            "11111111-2222-3333-4444-555555555555",
                            "/dev/disk/by-label/home",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:scratch:set-property:filesystem.uuid"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "xfs_admin",
                            "-U",
                            "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                            "/dev/disk/by-label/scratch",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:data:set-property:btrfs.uuid"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "btrfstune",
                            "-U",
                            "bbbbbbbb-1111-2222-3333-cccccccccccc",
                            "/dev/disk/by-label/data",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:missing-device:set-property:uuid"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "xfs_admin",
                            "-U",
                            "ffffffff-1111-2222-3333-444444444444",
                            "<filesystem-device>",
                        ]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["filesystem source device"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:missing-btrfs:set-property:uuid"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "btrfstune",
                            "-U",
                            "cccccccc-1111-2222-3333-dddddddddddd",
                            "<filesystem-device>",
                        ]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["filesystem source device"]
                })
        }));
    }

    #[test]
    fn btrfs_filesystem_rebalance_uses_declared_filters() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "filesystems": {
                "data": {
                  "mountpoint": "/data",
                  "fsType": "btrfs",
                  "operation": "rebalance",
                  "properties": {
                    "balance.data": "usage=50",
                    "balance.metadata": "usage=75"
                  }
                }
              },
              "apply": {
                "allowRebalance": true
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:data:rebalance"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "btrfs",
                            "balance",
                            "start",
                            "-dusage=50",
                            "-musage=75",
                            "/data",
                        ]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "filesystems:data:rebalance"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["btrfs", "filesystem", "usage", "-b", "/data"])
        }));
    }

    #[test]
    fn scrub_lifecycle_reports_btrfs_and_zpool_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "filesystems": {
                  "data": {
                    "mountpoint": "/data",
                    "fsType": "btrfs",
                    "operation": "scrub"
                  }
                },
                "pools": {
                  "tank": {
                    "operation": "scrub"
                  }
                }
              },
              "apply": {}
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:data:scrub"
                && step.commands.iter().any(|command| {
                    command.argv == ["btrfs", "scrub", "start", "-B", "/data"]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "pools:tank:scrub"
                && step.commands.iter().any(|command| {
                    command.argv == ["zpool", "scrub", "tank"]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "pools:tank:scrub"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["disk-nix", "inspect", "tank", "--json"])
        }));
    }

    #[test]
    fn filesystem_trim_lifecycle_reports_fstrim_command() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "filesystems": {
                  "scratch": {
                    "mountpoint": "/scratch",
                    "fsType": "xfs",
                    "operation": "trim"
                  }
                }
              },
              "apply": {}
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:scratch:trim"
                && step.commands.iter().any(|command| {
                    command.argv == ["fstrim", "-v", "/scratch"]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "filesystems:scratch:trim"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["disk-nix", "inspect", "/scratch", "--json"])
        }));
    }

    #[test]
    fn filesystem_remount_lifecycle_reports_mount_command() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "filesystems": {
                  "scratch": {
                    "mountpoint": "/scratch",
                    "fsType": "xfs",
                    "operation": "remount",
                    "options": ["rw", "noatime", "discard=async"]
                  }
                }
              },
              "apply": {}
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert!(report.command_summary.all_commands_ready());
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:scratch:remount"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "mount",
                            "-o",
                            "remount,rw,noatime,discard=async",
                            "/scratch",
                        ]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "filesystems:scratch:remount"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["findmnt", "--json", "/scratch"])
        }));
    }

    #[test]
    fn filesystem_remount_requires_mountpoint_for_execute_readiness() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "filesystems": {
                  "scratch": {
                    "fsType": "xfs",
                    "operation": "remount",
                    "options": ["ro"]
                  }
                }
              },
              "apply": {}
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert!(!report.command_summary.all_commands_ready());
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "filesystems:scratch:remount"
                && step.commands.iter().any(|command| {
                    command.argv == ["mount", "-o", "remount,ro", "<mountpoint>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["mountpoint path"]
                })
        }));
    }

    #[test]
    fn remove_device_renderer_uses_pool_and_lvm_commands() {
        let pool_action = PlannedAction {
            id: "pools:tank:remove-device:/dev/disk/by-id/old-vdev".to_string(),
            description: "remove old pool device".to_string(),
            operation: Operation::RemoveDevice,
            risk: RiskClass::PotentialDataLoss,
            destructive: false,
            context: ActionContext {
                collection: Some("pools".to_string()),
                name: Some("tank".to_string()),
                target: Some("tank".to_string()),
                device: Some("/dev/disk/by-id/old-vdev".to_string()),
                ..ActionContext::default()
            },
            advice: None,
        };
        let vg_action = PlannedAction {
            id: "volumeGroups:vg0:remove-device:/dev/disk/by-id/old-pv".to_string(),
            description: "remove old physical volume".to_string(),
            operation: Operation::RemoveDevice,
            risk: RiskClass::PotentialDataLoss,
            destructive: false,
            context: ActionContext {
                collection: Some("volumeGroups".to_string()),
                name: Some("vg0".to_string()),
                target: Some("vg0".to_string()),
                device: Some("/dev/disk/by-id/old-pv".to_string()),
                ..ActionContext::default()
            },
            advice: None,
        };
        let missing_pool_action = PlannedAction {
            id: "pools:tank:removedevice".to_string(),
            description: "remove unspecified pool device".to_string(),
            operation: Operation::RemoveDevice,
            risk: RiskClass::PotentialDataLoss,
            destructive: false,
            context: ActionContext {
                collection: Some("pools".to_string()),
                name: Some("tank".to_string()),
                target: Some("tank".to_string()),
                ..ActionContext::default()
            },
            advice: None,
        };
        let missing_vg_action = PlannedAction {
            id: "volumeGroups:vg0:removedevice".to_string(),
            description: "remove unspecified physical volume".to_string(),
            operation: Operation::RemoveDevice,
            risk: RiskClass::PotentialDataLoss,
            destructive: false,
            context: ActionContext {
                collection: Some("volumeGroups".to_string()),
                name: Some("vg0".to_string()),
                target: Some("vg0".to_string()),
                ..ActionContext::default()
            },
            advice: None,
        };

        let (pool_commands, pool_notes, pool_manual_review) = commands_for_action(&pool_action);
        let (vg_commands, vg_notes, vg_manual_review) = commands_for_action(&vg_action);
        let (missing_pool_commands, _, _) = commands_for_action(&missing_pool_action);
        let (missing_vg_commands, _, _) = commands_for_action(&missing_vg_action);

        assert!(pool_manual_review);
        assert!(pool_commands.iter().any(|command| {
            command.argv == ["zpool", "status", "-P", "tank"] && !command.mutates
        }));
        assert!(pool_commands.iter().any(|command| {
            command.argv == ["zpool", "remove", "tank", "/dev/disk/by-id/old-vdev"]
                && command.mutates
                && command.readiness == CommandReadiness::Ready
        }));
        assert!(
            pool_notes
                .iter()
                .any(|note| note.contains("supports device removal"))
        );

        assert!(vg_manual_review);
        assert!(vg_commands.iter().any(|command| {
            command.argv == ["pvs", "--reportformat", "json", "/dev/disk/by-id/old-pv"]
                && !command.mutates
        }));
        assert!(vg_commands.iter().any(|command| {
            command.argv == ["pvmove", "/dev/disk/by-id/old-pv"]
                && command.mutates
                && command.readiness == CommandReadiness::Ready
        }));
        assert!(vg_commands.iter().any(|command| {
            command.argv == ["vgreduce", "vg0", "/dev/disk/by-id/old-pv"]
                && command.mutates
                && command.readiness == CommandReadiness::Ready
        }));
        assert!(
            vg_notes
                .iter()
                .any(|note| note.contains("pvmove or add replacement capacity"))
        );
        assert!(missing_pool_commands.iter().any(|command| {
            command.argv == ["zpool", "remove", "tank", "<device>"]
                && command.mutates
                && command.readiness == CommandReadiness::NeedsDomainImplementation
                && command.unresolved_inputs == ["device to remove"]
        }));
        assert!(missing_vg_commands.iter().any(|command| {
            command.argv == ["pvmove", "<physical-volume>"]
                && command.mutates
                && command.readiness == CommandReadiness::NeedsDomainImplementation
                && command.unresolved_inputs == ["physical volume to remove"]
        }));
        assert!(missing_vg_commands.iter().any(|command| {
            command.argv == ["vgreduce", "vg0", "<physical-volume>"]
                && command.mutates
                && command.readiness == CommandReadiness::NeedsDomainImplementation
                && command.unresolved_inputs == ["physical volume to remove"]
        }));
    }

    #[test]
    fn volume_group_replacement_renders_lvm_migration_commands() {
        let action = PlannedAction {
            id: "volumeGroups:vg0:replace-device:/dev/disk/by-id/old-pv".to_string(),
            description: "replace old physical volume".to_string(),
            operation: Operation::ReplaceDevice,
            risk: RiskClass::OfflineRequired,
            destructive: false,
            context: ActionContext {
                collection: Some("volumeGroups".to_string()),
                name: Some("vg0".to_string()),
                target: Some("vg0".to_string()),
                device: Some("/dev/disk/by-id/old-pv".to_string()),
                replacement: Some("/dev/disk/by-id/new-pv".to_string()),
                ..ActionContext::default()
            },
            advice: None,
        };
        let missing_action = PlannedAction {
            id: "volumeGroups:vg0:replacedevice".to_string(),
            description: "replace unspecified physical volume".to_string(),
            operation: Operation::ReplaceDevice,
            risk: RiskClass::OfflineRequired,
            destructive: false,
            context: ActionContext {
                collection: Some("volumeGroups".to_string()),
                name: Some("vg0".to_string()),
                target: Some("vg0".to_string()),
                ..ActionContext::default()
            },
            advice: None,
        };

        let (commands, notes, manual_review) = commands_for_action(&action);
        let (missing_commands, _, _) = commands_for_action(&missing_action);

        assert!(manual_review);
        assert!(commands.iter().any(|command| {
            command.argv == ["pvs", "--reportformat", "json", "/dev/disk/by-id/old-pv"]
                && !command.mutates
        }));
        assert!(commands.iter().any(|command| {
            command.argv == ["pvs", "--reportformat", "json", "/dev/disk/by-id/new-pv"]
                && !command.mutates
        }));
        assert!(commands.iter().any(|command| {
            command.argv == ["vgextend", "vg0", "/dev/disk/by-id/new-pv"]
                && command.mutates
                && command.readiness == CommandReadiness::Ready
        }));
        assert!(commands.iter().any(|command| {
            command.argv == ["pvmove", "/dev/disk/by-id/old-pv", "/dev/disk/by-id/new-pv"]
                && command.mutates
                && command.readiness == CommandReadiness::Ready
        }));
        assert!(commands.iter().any(|command| {
            command.argv == ["vgreduce", "vg0", "/dev/disk/by-id/old-pv"]
                && command.mutates
                && command.readiness == CommandReadiness::Ready
        }));
        assert!(
            notes
                .iter()
                .any(|note| note.contains("replacement physical volume"))
        );

        assert!(missing_commands.iter().any(|command| {
            command.argv == ["vgextend", "vg0", "<replacement-physical-volume>"]
                && command.readiness == CommandReadiness::NeedsDomainImplementation
                && command.unresolved_inputs == ["replacement physical volume"]
        }));
        assert!(missing_commands.iter().any(|command| {
            command.argv
                == [
                    "pvmove",
                    "<physical-volume>",
                    "<replacement-physical-volume>",
                ]
                && command.readiness == CommandReadiness::NeedsDomainImplementation
                && command.unresolved_inputs
                    == ["physical volume to replace", "replacement physical volume"]
        }));
        assert!(missing_commands.iter().any(|command| {
            command.argv == ["vgreduce", "vg0", "<physical-volume>"]
                && command.readiness == CommandReadiness::NeedsDomainImplementation
                && command.unresolved_inputs == ["physical volume to remove"]
        }));
    }

    #[test]
    fn zfs_snapshot_rollback_renderer_reports_reviewable_commands() {
        let action = PlannedAction {
            id: "snapshot:tank/home@before:rollback".to_string(),
            description: "roll back tank/home to snapshot tank/home@before".to_string(),
            operation: Operation::Rollback,
            risk: RiskClass::PotentialDataLoss,
            destructive: false,
            context: ActionContext {
                collection: Some("snapshots".to_string()),
                name: Some("tank/home@before".to_string()),
                target: Some("tank/home@before".to_string()),
                recursive_rollback: Some(true),
                ..ActionContext::default()
            },
            advice: None,
        };

        let (commands, notes, requires_manual_review) = commands_for_action(&action);
        let (verification_commands, verification_checks) = verification_for_action(&action);

        assert!(requires_manual_review);
        assert!(notes.iter().any(|note| note.contains("fresh snapshot")));
        assert!(notes.iter().any(|note| note.contains("recursive rollback")));
        assert!(commands.iter().any(|command| {
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
                && !command.mutates
        }));
        assert!(commands.iter().any(|command| {
            command.argv == ["zfs", "rollback", "-r", "tank/home@before"]
                && command.mutates
                && command.readiness == CommandReadiness::Ready
        }));
        assert!(verification_commands.iter().any(|command| {
            command.argv == ["zfs", "list", "-H", "-p", "tank/home"] && !command.mutates
        }));
        assert!(
            verification_checks
                .iter()
                .any(|check| check.contains("rollback point"))
        );
    }

    #[test]
    fn zfs_snapshot_rollback_stays_blocked_by_apply_policy() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "snapshots": {
                "tank/home@before": {
                  "target": "tank/home",
                  "rollback": true
                }
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::Blocked);
        assert!(report.command_plan.is_empty());
        assert_eq!(report.command_summary.step_count, 0);

        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "snapshots": {
                "tank/home@before": {
                  "target": "tank/home",
                  "rollback": true
                }
              },
              "apply": {
                "allowPotentialDataLoss": true
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert!(report.command_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["zfs", "rollback", "tank/home@before"])
        }));
    }

    #[test]
    fn zfs_snapshot_holds_render_safe_property_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "snapshots": {
                "tank/home@before": {
                  "target": "tank/home",
                  "hold": "disk-nix-retain"
                },
                "tank/home@old": {
                  "target": "tank/home",
                  "releaseHold": "old-retention"
                }
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert_eq!(report.command_plan.len(), 2);
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "snapshot:tank/home@before:hold:disk-nix-retain"
                && step.commands.iter().any(|command| {
                    command.argv == ["zfs", "hold", "disk-nix-retain", "tank/home@before"]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "snapshot:tank/home@old:release-hold:old-retention"
                && step.commands.iter().any(|command| {
                    command.argv == ["zfs", "release", "old-retention", "tank/home@old"]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "snapshot:tank/home@before:hold:disk-nix-retain"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["zfs", "holds", "tank/home@before"])
        }));
        assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
        assert!(report.command_summary.all_commands_ready());
    }

    #[test]
    fn snapshot_rescan_reports_read_only_metadata_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "snapshots": {
                "tank/home@before": {
                  "operation": "rescan",
                  "target": "tank/home"
                },
                "/mnt/persist/@home-before": {
                  "operation": "rescan",
                  "target": "/mnt/persist/@home",
                  "readOnly": true
                }
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert_eq!(report.command_plan.len(), 2);
        let zfs_step = report
            .command_plan
            .iter()
            .find(|step| step.action_id == "snapshot:tank/home@before:rescan")
            .expect("ZFS snapshot rescan step exists");
        assert!(
            zfs_step
                .commands
                .iter()
                .all(|command| command.readiness == CommandReadiness::Ready && !command.mutates)
        );
        assert!(zfs_step.commands.iter().any(|command| {
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
        }));
        assert!(
            zfs_step
                .commands
                .iter()
                .any(|command| command.argv == ["zfs", "holds", "tank/home@before"])
        );

        let btrfs_step = report
            .command_plan
            .iter()
            .find(|step| step.action_id == "snapshot:/mnt/persist/@home-before:rescan")
            .expect("Btrfs snapshot rescan step exists");
        assert!(
            btrfs_step
                .commands
                .iter()
                .all(|command| command.readiness == CommandReadiness::Ready && !command.mutates)
        );
        assert!(btrfs_step.commands.iter().any(|command| {
            command.argv == ["btrfs", "subvolume", "show", "/mnt/persist/@home-before"]
        }));
        assert!(btrfs_step.commands.iter().any(|command| {
            command.argv
                == [
                    "btrfs",
                    "property",
                    "get",
                    "-ts",
                    "/mnt/persist/@home-before",
                    "ro",
                ]
        }));

        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "snapshot:tank/home@before:rescan"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["zfs", "holds", "tank/home@before"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "snapshot:/mnt/persist/@home-before:rescan"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "btrfs",
                            "property",
                            "get",
                            "-ts",
                            "/mnt/persist/@home-before",
                            "ro",
                        ]
                })
        }));
        assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
        assert!(report.command_summary.all_commands_ready());
    }

    #[test]
    fn snapshot_destroy_reports_domain_specific_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "snapshots": {
                "tank/home@old": {
                  "target": "tank/home",
                  "destroy": true
                },
                "/mnt/persist/@home-old": {
                  "target": "/mnt/persist/@home",
                  "destroy": true
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
        assert_eq!(report.apply.blocked.len(), 0);
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "snapshot:tank/home@old:destroy"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["zfs", "destroy", "tank/home@old"])
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "snapshot:/mnt/persist/@home-old:destroy"
                && step.commands.iter().any(|command| {
                    command.argv == ["btrfs", "subvolume", "delete", "/mnt/persist/@home-old"]
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "snapshot:tank/home@old:destroy"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["disk-nix", "inspect", "tank/home", "--json"])
        }));
        assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
        assert!(report.command_summary.all_commands_ready());
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
    fn disk_initialization_requires_destructive_policy_and_renders_mklabel() {
        let (blocked_plan, blocked_policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "disks": {
                  "/dev/disk/by-id/nvme-root": {
                    "operation": "create",
                    "partitionType": "gpt"
                  }
                }
              }
            }"#,
        )
        .expect("document parses");

        let blocked = prepare_execution(&blocked_plan, blocked_policy, ExecutionMode::DryRun);

        assert_eq!(blocked.status, ExecutionStatus::Blocked);
        assert!(blocked.command_plan.is_empty());

        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "disks": {
                  "/dev/disk/by-id/nvme-root": {
                    "operation": "create",
                    "partitionType": "gpt"
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
        assert_eq!(report.command_plan.len(), 1);
        assert!(report.command_plan[0].requires_manual_review);
        assert!(report.command_plan[0].commands.iter().any(|command| {
            command.argv
                == [
                    "parted",
                    "-s",
                    "/dev/disk/by-id/nvme-root",
                    "mklabel",
                    "gpt",
                ]
                && command.mutates
                && command.readiness == CommandReadiness::Ready
        }));
        assert!(report.command_plan[0].commands.iter().any(|command| {
            command.argv == ["partprobe", "/dev/disk/by-id/nvme-root"] && command.mutates
        }));
        assert!(report.verification_plan[0].commands.iter().any(|command| {
            command.argv == ["parted", "-lm", "/dev/disk/by-id/nvme-root"] && !command.mutates
        }));
    }

    #[test]
    fn disk_initialization_requires_stable_disk_path_for_execute_readiness() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "disks": {
                  "root": {
                    "operation": "create",
                    "partitionType": "gpt"
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
        assert!(!report.command_summary.all_commands_ready());
        assert!(report.command_plan[0].commands.iter().any(|command| {
            command.argv == ["disk-nix", "inspect", "<disk>"]
                && command.readiness == CommandReadiness::NeedsDomainImplementation
                && command.unresolved_inputs == ["disk path"]
        }));
        assert!(report.command_plan[0].commands.iter().any(|command| {
            command.argv == ["parted", "-s", "<disk>", "mklabel", "gpt"]
                && command.readiness == CommandReadiness::NeedsDomainImplementation
                && command.unresolved_inputs == ["disk path"]
        }));
        assert!(report.command_plan[0].commands.iter().any(|command| {
            command.argv == ["partprobe", "<disk>"]
                && command.readiness == CommandReadiness::NeedsDomainImplementation
                && command.unresolved_inputs == ["disk path"]
        }));
        assert!(report.command_plan[0].commands.iter().any(|command| {
            command.argv == ["parted", "-lm", "<disk>"]
                && command.readiness == CommandReadiness::NeedsDomainImplementation
                && command.unresolved_inputs == ["disk path"]
        }));
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
                && command.readiness == CommandReadiness::Ready
        }));
        assert!(report.command_plan[0].commands.iter().any(|command| {
            command.argv == ["blockdev", "--rereadpt", "/dev/disk/by-id/nvme-root"]
                && command.readiness == CommandReadiness::Ready
        }));
        assert!(
            report.verification_plan[0]
                .commands
                .iter()
                .any(|command| command.argv == ["parted", "-lm"])
        );
    }

    #[test]
    fn partition_creation_requires_disk_and_stable_partition_path_for_execute_readiness() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "partitions": {
                  "root": {
                    "operation": "create",
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
        assert!(!report.command_summary.all_commands_ready());
        assert!(report.command_plan[0].commands.iter().any(|command| {
            command.argv == ["disk-nix", "inspect", "<disk>"]
                && command.readiness == CommandReadiness::NeedsDomainImplementation
                && command.unresolved_inputs == ["disk path"]
        }));
        assert!(report.command_plan[0].commands.iter().any(|command| {
            command.argv == ["parted", "-s", "<disk>", "mkpart", "linux", "1MiB", "100%"]
                && command.readiness == CommandReadiness::NeedsDomainImplementation
                && command.unresolved_inputs == ["disk path"]
        }));
        assert!(report.command_plan[0].commands.iter().any(|command| {
            command.argv == ["disk-nix", "inspect", "<partition>"]
                && command.readiness == CommandReadiness::NeedsDomainImplementation
                && command.unresolved_inputs == ["partition path"]
        }));
    }

    #[test]
    fn partition_growth_uses_partition_number_for_resizepart() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "partitions": {
                  "root": {
                    "operation": "grow",
                    "device": "/dev/disk/by-id/nvme-root",
                    "partitionNumber": 2,
                    "end": "100%"
                  }
                }
              },
              "apply": {
                "allowOffline": true,
                "allowGrow": true
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
                    "resizepart",
                    "2",
                    "100%",
                ]
                && command.mutates
                && command.readiness == CommandReadiness::Ready
        }));
        assert!(report.command_plan[0].commands.iter().any(|command| {
            command.argv == ["blockdev", "--rereadpt", "/dev/disk/by-id/nvme-root"]
                && command.mutates
                && command.readiness == CommandReadiness::Ready
        }));
    }

    #[test]
    fn partition_table_rescan_reports_partprobe_and_rereadpt_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "disks": {
                  "/dev/disk/by-id/nvme-data": {
                    "operation": "rescan"
                  }
                },
                "partitions": {
                  "data-table": {
                    "operation": "rescan",
                    "device": "/dev/disk/by-id/nvme-data"
                  }
                }
              },
              "apply": {}
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert_eq!(report.command_plan.len(), 2);
        assert!(report.command_summary.all_commands_ready());
        for action_id in [
            "disks:/dev/disk/by-id/nvme-data:rescan",
            "partitions:data-table:rescan",
        ] {
            assert!(report.command_plan.iter().any(|step| {
                step.action_id == action_id
                    && step.commands.iter().any(|command| {
                        command.argv == ["partprobe", "/dev/disk/by-id/nvme-data"]
                            && command.mutates
                            && command.readiness == CommandReadiness::Ready
                    })
                    && step.commands.iter().any(|command| {
                        command.argv == ["blockdev", "--rereadpt", "/dev/disk/by-id/nvme-data"]
                            && command.mutates
                            && command.readiness == CommandReadiness::Ready
                    })
            }));
        }
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "partitions:data-table:rescan"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["parted", "-lm", "/dev/disk/by-id/nvme-data"])
        }));
    }

    #[test]
    fn partition_table_rescan_requires_disk_path_for_execute_readiness() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "partitions": {
                  "data-table": {
                    "operation": "rescan"
                  }
                }
              },
              "apply": {}
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert!(!report.command_summary.all_commands_ready());
        assert!(report.command_plan[0].commands.iter().any(|command| {
            command.argv == ["partprobe", "<disk>"]
                && command.readiness == CommandReadiness::NeedsDomainImplementation
                && command.unresolved_inputs == ["disk path"]
        }));
        assert!(report.command_plan[0].commands.iter().any(|command| {
            command.argv == ["blockdev", "--rereadpt", "<disk>"]
                && command.readiness == CommandReadiness::NeedsDomainImplementation
                && command.unresolved_inputs == ["disk path"]
        }));
    }

    #[test]
    fn luks_keyslot_lifecycle_reports_cryptsetup_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "luksKeyslots": {
                  "cryptroot:1": {
                    "operation": "add-key",
                    "device": "/dev/disk/by-id/root-luks",
                    "metadata": {
                      "keySlot": "1",
                      "newKeyFile": "/run/keys/root-new"
                    }
                  },
                  "cryptroot:3": {
                    "properties": {
                      "keyFile": "/run/keys/root-rotated"
                    },
                    "device": "/dev/disk/by-id/root-luks",
                    "metadata": {
                      "keySlot": "3",
                      "keyFile": "/run/keys/root-old"
                    }
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
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "lukskeyslots:cryptroot:1:add-key"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "cryptsetup",
                            "luksAddKey",
                            "--key-slot",
                            "1",
                            "/dev/disk/by-id/root-luks",
                            "/run/keys/root-new",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luksKeyslots:cryptroot:3:set-property:keyFile"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "cryptsetup",
                            "luksChangeKey",
                            "--key-slot",
                            "3",
                            "--key-file",
                            "/run/keys/root-old",
                            "/dev/disk/by-id/root-luks",
                            "/run/keys/root-rotated",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "lukskeyslots:cryptroot:1:add-key"
                && step.commands.iter().any(|command| {
                    command.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/root-luks"]
                })
        }));
    }

    #[test]
    fn luks_keyslot_lifecycle_reports_missing_inputs_for_execute_readiness() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "luksKeyslots": {
                  "root-add": {
                    "operation": "add-key"
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
        assert!(!report.command_summary.all_commands_ready());
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "lukskeyslots:root-add:add-key"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "cryptsetup",
                            "luksAddKey",
                            "<luks-device>",
                            "<new-key-file>",
                        ]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["LUKS backing device", "new key file"]
                })
        }));
    }

    #[test]
    fn luks_keyslot_destroy_renderer_uses_cryptsetup_kill_slot() {
        let action = PlannedAction {
            id: "lukskeyslots:cryptroot:2:destroy".to_string(),
            description: "remove LUKS keyslot".to_string(),
            operation: Operation::Destroy,
            risk: RiskClass::PotentialDataLoss,
            destructive: false,
            context: ActionContext {
                collection: Some("luksKeyslots".to_string()),
                name: Some("cryptroot:2".to_string()),
                device: Some("/dev/disk/by-id/root-luks".to_string()),
                key_slot: Some("2".to_string()),
                ..ActionContext::default()
            },
            advice: None,
        };

        let (commands, _, requires_manual_review) = commands_for_action(&action);

        assert!(requires_manual_review);
        assert!(commands.iter().any(|command| {
            command.argv
                == [
                    "cryptsetup",
                    "luksKillSlot",
                    "/dev/disk/by-id/root-luks",
                    "2",
                ]
                && command.readiness == CommandReadiness::Ready
        }));
    }

    #[test]
    fn luks_token_lifecycle_reports_cryptsetup_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "luksTokens": {
                  "cryptroot:0": {
                    "operation": "import-token",
                    "device": "/dev/disk/by-id/root-luks",
                    "metadata": {
                      "tokenId": "0",
                      "tokenFile": "/run/keys/root-token.json"
                    }
                  },
                  "cryptroot:2": {
                    "properties": {
                      "tokenFile": "/run/keys/root-token-new.json"
                    },
                    "device": "/dev/disk/by-id/root-luks",
                    "metadata": {
                      "tokenId": "2"
                    }
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
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "lukstokens:cryptroot:0:import-token"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "cryptsetup",
                            "token",
                            "import",
                            "--token-id",
                            "0",
                            "--json-file",
                            "/run/keys/root-token.json",
                            "/dev/disk/by-id/root-luks",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luksTokens:cryptroot:2:set-property:tokenFile"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "cryptsetup",
                            "token",
                            "import",
                            "--token-id",
                            "2",
                            "--json-file",
                            "/run/keys/root-token-new.json",
                            "/dev/disk/by-id/root-luks",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "lukstokens:cryptroot:0:import-token"
                && step.commands.iter().any(|command| {
                    command.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/root-luks"]
                })
        }));
    }

    #[test]
    fn luks_token_lifecycle_reports_missing_inputs_for_execute_readiness() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "luksTokens": {
                  "root-token": {
                    "operation": "import-token"
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
        assert!(!report.command_summary.all_commands_ready());
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "lukstokens:root-token:import-token"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "cryptsetup",
                            "token",
                            "import",
                            "--json-file",
                            "<token-json-file>",
                            "<luks-device>",
                        ]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["LUKS backing device", "token JSON file"]
                })
        }));
    }

    #[test]
    fn luks_token_destroy_renderer_uses_cryptsetup_token_remove() {
        let action = PlannedAction {
            id: "lukstokens:cryptroot:1:destroy".to_string(),
            description: "remove LUKS token".to_string(),
            operation: Operation::Destroy,
            risk: RiskClass::PotentialDataLoss,
            destructive: false,
            context: ActionContext {
                collection: Some("luksTokens".to_string()),
                name: Some("cryptroot:1".to_string()),
                device: Some("/dev/disk/by-id/root-luks".to_string()),
                token_id: Some("1".to_string()),
                ..ActionContext::default()
            },
            advice: None,
        };

        let (commands, _, requires_manual_review) = commands_for_action(&action);

        assert!(requires_manual_review);
        assert!(commands.iter().any(|command| {
            command.argv
                == [
                    "cryptsetup",
                    "token",
                    "remove",
                    "--token-id",
                    "1",
                    "/dev/disk/by-id/root-luks",
                ]
                && command.readiness == CommandReadiness::Ready
        }));
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
                  },
                  "scratch": {
                    "device": "/swapfile",
                    "operation": "grow",
                    "desiredSize": "16GiB"
                  },
                  "inventory": {
                    "device": "/dev/disk/by-label/swap-inventory",
                    "operation": "rescan"
                  }
                },
                "luks": {
                  "devices": {
                    "cryptroot": {
                      "name": "cryptroot",
                      "device": "/dev/disk/by-partuuid/root",
                      "operation": "grow"
                    },
                    "cryptdata": {
                      "name": "cryptdata",
                      "device": "/dev/disk/by-id/data-luks",
                      "operation": "create"
                    },
                    "cryptarchive": {
                      "name": "cryptarchive",
                      "device": "/dev/disk/by-id/archive-luks",
                      "operation": "open"
                    },
                    "cryptmissing": {
                      "name": "cryptmissing",
                      "operation": "create"
                    },
                    "cryptold": {
                      "name": "cryptold",
                      "device": "/dev/disk/by-id/old-luks",
                      "operation": "destroy"
                    },
                    "cryptclosed": {
                      "name": "cryptclosed",
                      "device": "/dev/disk/by-id/closed-luks",
                      "operation": "close"
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
        assert_eq!(report.command_plan.len(), 9);
        assert!(report.command_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["mkswap", "/dev/disk/by-label/swap"])
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv == ["fallocate", "--length", "16GiB", "/swapfile"]
                    && command.readiness == CommandReadiness::Ready
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "swaps:inventory:rescan"
                && step.commands.iter().any(|command| {
                    command.argv == ["blkid", "/dev/disk/by-label/swap-inventory"]
                        && !command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
                && step.commands.iter().any(|command| {
                    command.argv == ["disk-nix", "inspect", "/dev/disk/by-label/swap-inventory"]
                        && !command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["cryptsetup", "resize", "cryptroot"])
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luks.devices:cryptdata:create"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "cryptsetup",
                            "open",
                            "/dev/disk/by-id/data-luks",
                            "cryptdata",
                        ]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luks.devices:cryptarchive:open"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "cryptsetup",
                            "open",
                            "/dev/disk/by-id/archive-luks",
                            "cryptarchive",
                        ]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luks.devices:cryptmissing:create"
                && step.commands.iter().any(|command| {
                    command.argv == ["cryptsetup", "isLuks", "<device>"]
                        && !command.mutates
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["LUKS backing device"]
                })
                && step.commands.iter().any(|command| {
                    command.argv == ["cryptsetup", "open", "<device>", "cryptmissing"]
                        && command.mutates
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["LUKS backing device"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luks.devices:cryptold:destroy"
                && step.commands.iter().any(|command| {
                    command.argv == ["cryptsetup", "close", "cryptold"]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luks.devices:cryptclosed:close"
                && step.commands.iter().any(|command| {
                    command.argv == ["cryptsetup", "close", "cryptclosed"]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "swaps:scratch:grow"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["swapon", "--show", "--bytes", "--raw"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "swaps:inventory:rescan"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "disk-nix",
                            "inspect",
                            "/dev/disk/by-label/swap-inventory",
                            "--json",
                        ]
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "luks.devices:cryptold:destroy"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["disk-nix", "topology", "--json"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "luks.devices:cryptdata:create"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["cryptsetup", "status", "cryptdata"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "luks.devices:cryptarchive:open"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["cryptsetup", "status", "cryptarchive"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "luks.devices:cryptclosed:close"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["disk-nix", "topology", "--json"])
        }));
    }

    #[test]
    fn swap_lifecycle_requires_target_path_for_execute_readiness() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "swaps": {
                  "scratch": {
                    "operation": "grow",
                    "desiredSize": "16GiB"
                  },
                  "inventory": {
                    "operation": "rescan"
                  },
                  "primary": {
                    "preserveData": false
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
        assert!(!report.command_summary.all_commands_ready());
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "swaps:scratch:grow"
                && step.commands.iter().any(|command| {
                    command.argv == ["swapoff", "<swap>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["swap target path"]
                })
                && step.commands.iter().any(|command| {
                    command.argv == ["<resize-swap-backing-storage>", "<swap>", "16GiB"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs
                            == ["swap target path", "backing storage domain"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "swaps:inventory:rescan"
                && step.commands.iter().any(|command| {
                    command.argv == ["blkid", "<swap>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["swap target path"]
                })
                && step.commands.iter().any(|command| {
                    command.argv == ["disk-nix", "inspect", "<swap>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["swap target path"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "swaps:primary:format"
                && step.commands.iter().any(|command| {
                    command.argv == ["mkswap", "<swap>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["swap target path"]
                })
        }));
    }

    #[test]
    fn luks_header_properties_use_cryptsetup_identity_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "luks": {
                  "devices": {
                    "cryptroot": {
                      "name": "cryptroot",
                      "device": "/dev/disk/by-id/root-luks",
                      "properties": {
                        "label": "root",
                        "luks.subsystem": "nixos",
                        "luks.uuid": "01234567-89ab-cdef-0123-456789abcdef"
                      }
                    },
                    "logical": {
                      "properties": {
                        "luks.label": "logical-root"
                      }
                    }
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
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luks.devices:cryptroot:set-property:label"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "cryptsetup",
                            "config",
                            "/dev/disk/by-id/root-luks",
                            "--label",
                            "root",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luks.devices:cryptroot:set-property:luks.subsystem"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "cryptsetup",
                            "config",
                            "/dev/disk/by-id/root-luks",
                            "--subsystem",
                            "nixos",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luks.devices:cryptroot:set-property:luks.uuid"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "cryptsetup",
                            "luksUUID",
                            "/dev/disk/by-id/root-luks",
                            "--uuid",
                            "01234567-89ab-cdef-0123-456789abcdef",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luks.devices:logical:set-property:luks.label"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "cryptsetup",
                            "config",
                            "<luks-device>",
                            "--label",
                            "logical-root",
                        ]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["LUKS backing device"]
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "luks.devices:cryptroot:set-property:label"
                && step.commands.iter().any(|command| {
                    command.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/root-luks"]
                })
        }));
    }

    #[test]
    fn swap_properties_use_swaplabel() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "swaps": {
                  "primary": {
                    "device": "/dev/disk/by-label/swap-old",
                    "properties": {
                      "label": "swap-new",
                      "swap.uuid": "01234567-89ab-cdef-0123-456789abcdef"
                    }
                  },
                  "logical": {
                    "properties": {
                      "swap.label": "logical-swap"
                    }
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
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "swaps:primary:set-property:label"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "swaplabel",
                            "--label",
                            "swap-new",
                            "/dev/disk/by-label/swap-old",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "swaps:primary:set-property:swap.uuid"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "swaplabel",
                            "--uuid",
                            "01234567-89ab-cdef-0123-456789abcdef",
                            "/dev/disk/by-label/swap-old",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "swaps:logical:set-property:swap.label"
                && step.commands.iter().any(|command| {
                    command.argv == ["swaplabel", "--label", "logical-swap", "<swap>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["swap target path"]
                })
        }));
    }

    #[test]
    fn vdo_lifecycle_reports_vdo_commands_and_verification() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "vdoVolumes": {
                  "new-cache": {
                    "operation": "create",
                    "device": "/dev/disk/by-id/vdo-backing",
                    "desiredSize": "2TiB"
                  },
                  "archive": {
                    "operation": "grow",
                    "desiredSize": "4TiB",
                    "properties": {
                      "writePolicy": "sync",
                      "compression": "enabled",
                      "deduplication": "disabled"
                    }
                  },
                  "warmArchive": {
                    "operation": "start"
                  },
                  "coldArchive": {
                    "operation": "stop"
                  },
                  "refreshArchive": {
                    "operation": "rescan"
                  },
                  "missing-backing": {
                    "operation": "create",
                    "desiredSize": "1TiB"
                  },
                  "old-cache": {
                    "destroy": true
                  }
                }
              },
              "apply": {
                "allowGrow": true,
                "allowOffline": true,
                "allowDestructive": true
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
                        "create",
                        "--name",
                        "new-cache",
                        "--device",
                        "/dev/disk/by-id/vdo-backing",
                        "--vdoLogicalSize",
                        "2TiB",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "vdovolumes:missing-backing:create"
                && step.commands.iter().any(|command| {
                    command.argv == ["disk-nix", "inspect", "<backing-device>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["backing device"]
                })
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "vdo",
                            "create",
                            "--name",
                            "missing-backing",
                            "--device",
                            "<backing-device>",
                            "--vdoLogicalSize",
                            "1TiB",
                        ]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["backing device"]
                })
        }));
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
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "vdoVolumes:archive:set-property:writePolicy"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "vdo",
                            "changeWritePolicy",
                            "--name",
                            "archive",
                            "--writePolicy",
                            "sync",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "vdoVolumes:archive:set-property:compression"
                && step.commands.iter().any(|command| {
                    command.argv == ["vdo", "enableCompression", "--name", "archive"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "vdoVolumes:archive:set-property:deduplication"
                && step.commands.iter().any(|command| {
                    command.argv == ["vdo", "disableDeduplication", "--name", "archive"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv == ["vdo", "start", "--name", "warmArchive"]
                    && command.readiness == CommandReadiness::Ready
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv == ["vdo", "stop", "--name", "coldArchive"]
                    && command.readiness == CommandReadiness::Ready
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "vdovolumes:refresharchive:rescan"
                && step
                    .commands
                    .iter()
                    .all(|command| command.readiness == CommandReadiness::Ready && !command.mutates)
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["vdo", "status", "--name", "refreshArchive"])
                && step.commands.iter().any(|command| {
                    command.argv == ["vdostats", "--human-readable", "refreshArchive"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv == ["vdo", "remove", "--name", "old-cache"]
                    && command.readiness == CommandReadiness::Ready
            })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "vdovolumes:new-cache:create"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["vdo", "status", "--name", "new-cache"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["vdostats", "--human-readable", "archive"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "vdovolumes:warmarchive:start"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["vdo", "status", "--name", "warmArchive"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "vdovolumes:coldarchive:stop"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["vdo", "status"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "vdovolumes:refresharchive:rescan"
                && step.commands.iter().any(|command| {
                    command.argv == ["disk-nix", "inspect", "refreshArchive", "--json"]
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "vdoVolumes:archive:set-property:writePolicy"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["vdostats", "--verbose", "archive"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "vdovolumes:old-cache:destroy"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["vdo", "status"])
        }));
    }

    #[test]
    fn vdo_property_lifecycle_blocks_unsupported_properties_and_values() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "vdoVolumes": {
                  "archive": {
                    "properties": {
                      "writePolicy": "eventual",
                      "compression": "maybe",
                      "indexMemory": "0.5"
                    }
                  }
                }
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::Blocked);
        assert_eq!(report.apply.blocked_summary.unsupported_count, 3);
        assert!(report.command_plan.is_empty());
        assert!(report.apply.blocked.iter().any(|blocked| {
            blocked.id == "vdoVolumes:archive:set-property:writePolicy"
                && blocked.risk == RiskClass::Unsupported
        }));
        assert!(report.apply.blocked.iter().any(|blocked| {
            blocked.id == "vdoVolumes:archive:set-property:compression"
                && blocked.risk == RiskClass::Unsupported
        }));
        assert!(report.apply.blocked.iter().any(|blocked| {
            blocked.id == "vdoVolumes:archive:set-property:indexMemory"
                && blocked.risk == RiskClass::Unsupported
        }));
    }

    #[test]
    fn zfs_snapshot_clone_renderer_reports_reviewable_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "snapshots": {
                "tank/home@before": {
                  "target": "tank/home",
                  "cloneTo": "tank/home-review"
                }
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "snapshot:tank/home@before:clone:tank/home-review"
                && step.commands.iter().any(|command| {
                    command.argv == ["zfs", "clone", "tank/home@before", "tank/home-review"]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "snapshot:tank/home@before:clone:tank/home-review"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["zfs", "list", "-H", "-p", "tank/home-review"])
        }));
    }

    #[test]
    fn rename_lifecycle_reports_domain_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "datasets": {
                  "tank/home": {
                    "operation": "rename",
                    "renameTo": "tank/home-staged"
                  }
                },
                "volumes": {
                  "vg0/old": {
                    "operation": "rename",
                    "renameTo": "vg0/new"
                  }
                },
                "btrfsSubvolumes": {
                  "/mnt/persist/@old": {
                    "operation": "rename",
                    "renameTo": "/mnt/persist/@new"
                  }
                },
                "snapshots": {
                  "tank/home@before-prune": {
                    "target": "tank/home",
                    "renameTo": "tank/home@retained"
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
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "datasets:tank/home:rename"
                && step.commands.iter().any(|command| {
                    command.argv == ["zfs", "rename", "tank/home", "tank/home-staged"]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "volumes:vg0/old:rename"
                && step.commands.iter().any(|command| {
                    command.argv == ["lvrename", "vg0/old", "vg0/new"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "btrfssubvolumes:/mnt/persist/@old:rename"
                && step.commands.iter().any(|command| {
                    command.argv == ["mv", "--", "/mnt/persist/@old", "/mnt/persist/@new"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "snapshot:tank/home@before-prune:rename:tank/home@retained"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "zfs",
                            "rename",
                            "tank/home@before-prune",
                            "tank/home@retained",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
    }

    #[test]
    fn zfs_clone_promotion_reports_reviewable_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "datasets": {
                  "tank/home-review": {
                    "operation": "promote"
                  }
                },
                "zvols": {
                  "tank/vm/root-review": {
                    "operation": "promote"
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
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "datasets:tank/home-review:promote"
                && step.commands.iter().any(|command| {
                    command.argv == ["zfs", "promote", "tank/home-review"]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "zvols:tank/vm/root-review:promote"
                && step.commands.iter().any(|command| {
                    command.argv == ["zfs", "promote", "tank/vm/root-review"]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "datasets:tank/home-review:promote"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "zfs",
                            "get",
                            "-H",
                            "-o",
                            "value",
                            "origin",
                            "tank/home-review",
                        ]
                })
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
                    "path": "/mnt/persist/@home",
                    "properties": {
                      "readonly": true
                    }
                  },
                  "/mnt/persist/@inventory": {
                    "operation": "rescan",
                    "path": "/mnt/persist/@inventory"
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
            step.action_id == "btrfsSubvolumes:/mnt/persist/@home:set-property:readonly"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "btrfs",
                            "property",
                            "set",
                            "-ts",
                            "/mnt/persist/@home",
                            "ro",
                            "true",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "btrfssubvolumes:/mnt/persist/@inventory:rescan"
                && step.commands.iter().any(|command| {
                    command.argv == ["btrfs", "subvolume", "show", "/mnt/persist/@inventory"]
                        && !command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "btrfs",
                            "property",
                            "get",
                            "-ts",
                            "/mnt/persist/@inventory",
                            "ro",
                        ]
                        && !command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "btrfssubvolumes:/mnt/persist/@inventory:rescan"
                && step.commands.iter().any(|command| {
                    command.argv == ["disk-nix", "inspect", "/mnt/persist/@inventory", "--json"]
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
    fn btrfs_qgroup_lifecycle_reports_limit_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "btrfsQgroups": {
                  "0/258": {
                    "target": "/mnt/persist",
                    "operation": "create"
                  },
                  "0/257": {
                    "target": "/mnt/persist",
                    "properties": {
                      "limit": "25GiB",
                      "maxExclusive": "10GiB"
                    }
                  },
                  "0/263": {
                    "target": "/mnt/persist",
                    "operation": "rescan"
                  },
                  "0/259": {
                    "target": "/mnt/persist",
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

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "btrfsqgroups:0/258:create"
                && step.commands.iter().any(|command| {
                    command.argv == ["btrfs", "qgroup", "create", "0/258", "/mnt/persist"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "btrfsQgroups:0/257:set-property:limit"
                && step.commands.iter().any(|command| {
                    command.argv == ["btrfs", "qgroup", "limit", "25GiB", "0/257", "/mnt/persist"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "btrfsQgroups:0/257:set-property:maxExclusive"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "btrfs",
                            "qgroup",
                            "limit",
                            "-e",
                            "10GiB",
                            "0/257",
                            "/mnt/persist",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "btrfsqgroups:0/259:destroy"
                && step.commands.iter().any(|command| {
                    command.argv == ["btrfs", "qgroup", "destroy", "0/259", "/mnt/persist"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "btrfsqgroups:0/263:rescan"
                && step
                    .commands
                    .iter()
                    .all(|command| command.readiness == CommandReadiness::Ready && !command.mutates)
                && step.commands.iter().any(|command| {
                    command.argv == ["btrfs", "qgroup", "show", "--raw", "-reF", "/mnt/persist"]
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "btrfsQgroups:0/257:set-property:limit"
                && step.commands.iter().any(|command| {
                    command.argv == ["btrfs", "qgroup", "show", "--raw", "-reF", "/mnt/persist"]
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "btrfsqgroups:0/263:rescan"
                && step.commands.iter().any(|command| {
                    command.argv == ["btrfs", "qgroup", "show", "--raw", "-reF", "/mnt/persist"]
                })
        }));
        assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
        assert!(report.command_summary.all_commands_ready());
    }

    #[test]
    fn btrfs_qgroup_lifecycle_without_target_reports_unresolved_path() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "btrfsQgroups": {
                  "0/260": {
                    "operation": "create"
                  },
                  "0/261": {
                    "properties": {
                      "limit": "5GiB"
                    }
                  },
                  "0/263": {
                    "operation": "rescan"
                  },
                  "0/262": {
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

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "btrfsqgroups:0/260:create"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "btrfs",
                            "qgroup",
                            "create",
                            "0/260",
                            "<btrfs-filesystem-path>",
                        ]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["mounted Btrfs filesystem path"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "btrfsQgroups:0/261:set-property:limit"
                && step.commands.iter().any(|command| {
                    command.argv == ["btrfs", "qgroup", "limit", "5GiB", "0/261", "<path>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["mounted Btrfs filesystem path"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "btrfsqgroups:0/263:rescan"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "btrfs",
                            "qgroup",
                            "show",
                            "--raw",
                            "-reF",
                            "<btrfs-filesystem-path>",
                        ]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["mounted Btrfs filesystem path"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "btrfsqgroups:0/262:destroy"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "btrfs",
                            "qgroup",
                            "destroy",
                            "0/262",
                            "<btrfs-filesystem-path>",
                        ]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["mounted Btrfs filesystem path"]
                })
        }));
        assert_eq!(report.command_summary.needs_domain_implementation_count, 7);
        assert!(!report.command_summary.all_commands_ready());
    }

    #[test]
    fn zvol_lifecycle_reports_zfs_volume_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "zvols": {
                  "tank/vm/root": {
                    "operation": "grow",
                    "desiredSize": "80GiB",
                    "properties": {
                      "compression": "zstd"
                    }
                  },
                  "tank/vm/tmp": {
                    "operation": "create",
                    "desiredSize": "20GiB",
                    "properties": {
                      "compression": "zstd",
                      "volblocksize": "16K"
                    }
                  },
                  "tank/vm/inventory": {
                    "operation": "rescan"
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
            step.action_id == "zvols:tank/vm/root:set-property:compression"
                && step.commands.iter().any(|command| {
                    command.argv == ["zfs", "set", "compression=zstd", "tank/vm/root"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv
                    == [
                        "zfs",
                        "create",
                        "-o",
                        "compression=zstd",
                        "-o",
                        "volblocksize=16K",
                        "-V",
                        "20GiB",
                        "tank/vm/tmp",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["zfs", "destroy", "tank/vm/old"])
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "zvols:tank/vm/inventory:rescan"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "zfs",
                            "list",
                            "-H",
                            "-p",
                            "-t",
                            "volume",
                            "tank/vm/inventory",
                        ]
                        && !command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
                && step.commands.iter().any(|command| {
                    command.argv == ["disk-nix", "inspect", "tank/vm/inventory"]
                        && !command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv == ["zfs", "list", "-H", "-p", "-t", "volume", "tank/vm/root"]
            })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "zvols:tank/vm/inventory:rescan"
                && step.commands.iter().any(|command| {
                    command.argv == ["disk-nix", "inspect", "tank/vm/inventory", "--json"]
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "zvols:tank/vm/root:set-property:compression"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["zfs", "get", "all", "tank/vm/root"])
        }));
    }

    #[test]
    fn zfs_dataset_lifecycle_reports_zfs_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "datasets": {
                  "tank/home": {
                    "operation": "create",
                    "properties": {
                      "compression": "zstd",
                      "mountpoint": "/home"
                    }
                  },
                  "tank/inventory": {
                    "operation": "rescan"
                  },
                  "tank/archive": {
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

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv
                    == [
                        "zfs",
                        "create",
                        "-o",
                        "compression=zstd",
                        "-o",
                        "mountpoint=/home",
                        "tank/home",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["zfs", "destroy", "tank/archive"])
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "datasets:tank/inventory:rescan"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "zfs",
                            "list",
                            "-H",
                            "-p",
                            "-t",
                            "filesystem",
                            "tank/inventory",
                        ]
                        && !command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
                && step.commands.iter().any(|command| {
                    command.argv == ["disk-nix", "inspect", "tank/inventory"]
                        && !command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "datasets:tank/home:create"
                && step.commands.iter().any(|command| {
                    command.argv == ["zfs", "list", "-H", "-p", "-t", "filesystem", "tank/home"]
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "datasets:tank/inventory:rescan"
                && step.commands.iter().any(|command| {
                    command.argv == ["disk-nix", "inspect", "tank/inventory", "--json"]
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "datasets:tank/archive:destroy"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["zfs", "list", "-H", "-p", "-t", "filesystem"])
        }));
    }

    #[test]
    fn md_raid_lifecycle_reports_mdadm_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "mdRaids": {
                  "existing": {
                    "target": "/dev/md/existing",
                    "operation": "assemble",
                    "devices": [
                      "/dev/disk/by-id/existing-a",
                      "/dev/disk/by-id/existing-b"
                    ]
                  },
                  "oldroot": {
                    "target": "/dev/md/oldroot",
                    "operation": "stop"
                  },
                  "inventory": {
                    "target": "/dev/md/root",
                    "operation": "rescan"
                  },
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
            step.action_id == "mdraids:existing:assemble"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "mdadm",
                            "--assemble",
                            "/dev/md/existing",
                            "/dev/disk/by-id/existing-a",
                            "/dev/disk/by-id/existing-b",
                        ]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "mdraids:oldroot:stop"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["mdadm", "--stop", "/dev/md/oldroot"])
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "mdraids:inventory:rescan"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["mdadm", "--detail", "/dev/md/root"])
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["mdadm", "--examine", "--scan"])
        }));
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
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "mdraids:inventory:rescan"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["disk-nix", "topology", "--json"])
        }));
    }

    #[test]
    fn md_raid_create_requires_destructive_policy_and_renders_mdadm_create() {
        let (blocked_plan, blocked_policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "mdRaids": {
                  "newroot": {
                    "target": "/dev/md/newroot",
                    "operation": "create",
                    "level": "1",
                    "devices": [
                      "/dev/disk/by-id/nvme-a",
                      "/dev/disk/by-id/nvme-b"
                    ]
                  }
                }
              }
            }"#,
        )
        .expect("document parses");

        let blocked = prepare_execution(&blocked_plan, blocked_policy, ExecutionMode::DryRun);

        assert_eq!(blocked.status, ExecutionStatus::Blocked);
        assert!(blocked.command_plan.is_empty());

        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "mdRaids": {
                  "newroot": {
                    "target": "/dev/md/newroot",
                    "operation": "create",
                    "level": "1",
                    "devices": [
                      "/dev/disk/by-id/nvme-a",
                      "/dev/disk/by-id/nvme-b"
                    ]
                  },
                  "missing-level": {
                    "target": "/dev/md/missing-level",
                    "operation": "create",
                    "devices": [
                      "/dev/disk/by-id/nvme-c",
                      "/dev/disk/by-id/nvme-d"
                    ]
                  },
                  "missing-members": {
                    "target": "/dev/md/missing-members",
                    "operation": "create",
                    "level": "10"
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
                command.argv
                    == [
                        "mdadm",
                        "--create",
                        "/dev/md/newroot",
                        "--level",
                        "1",
                        "--raid-devices",
                        "2",
                        "/dev/disk/by-id/nvme-a",
                        "/dev/disk/by-id/nvme-b",
                    ]
                    && command.readiness == CommandReadiness::Ready
                    && command.mutates
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "mdraids:missing-level:create"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "mdadm",
                            "--create",
                            "/dev/md/missing-level",
                            "--level",
                            "<level>",
                            "--raid-devices",
                            "2",
                            "/dev/disk/by-id/nvme-c",
                            "/dev/disk/by-id/nvme-d",
                        ]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["RAID level"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "mdraids:missing-members:create"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "mdadm",
                            "--create",
                            "/dev/md/missing-members",
                            "--level",
                            "10",
                            "--raid-devices",
                            "<member-count>",
                            "<member-device>",
                        ]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["member devices"]
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "mdraids:newroot:create"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["mdadm", "--detail", "/dev/md/newroot"])
        }));
    }

    #[test]
    fn md_raid_lifecycle_requires_array_path_for_execute_readiness() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "mdRaids": {
                  "newroot": {
                    "operation": "create",
                    "level": "1",
                    "devices": [
                      "/dev/disk/by-id/nvme-a",
                      "/dev/disk/by-id/nvme-b"
                    ]
                  },
                  "root": {
                    "operation": "grow",
                    "desiredSize": "max"
                  },
                  "existing": {
                    "operation": "assemble"
                  },
                  "oldroot": {
                    "operation": "stop"
                  },
                  "inventory": {
                    "operation": "rescan"
                  }
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowGrow": true,
                "allowOffline": true,
                "allowDeviceReplacement": true
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert!(!report.command_summary.all_commands_ready());
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "mdraids:newroot:create"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "mdadm",
                            "--create",
                            "<md-array>",
                            "--level",
                            "1",
                            "--raid-devices",
                            "2",
                            "/dev/disk/by-id/nvme-a",
                            "/dev/disk/by-id/nvme-b",
                        ]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["MD array path"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "mdraids:root:grow"
                && step.commands.iter().any(|command| {
                    command.argv == ["mdadm", "--grow", "<md-array>", "--size", "max"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["MD array path"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "mdraids:existing:assemble"
                && step.commands.iter().any(|command| {
                    command.argv == ["mdadm", "--assemble", "<md-array>", "<member-device>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["MD array path", "member devices"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "mdraids:oldroot:stop"
                && step.commands.iter().any(|command| {
                    command.argv == ["mdadm", "--stop", "<md-array>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["MD array path"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "mdraids:inventory:rescan"
                && step
                    .commands
                    .iter()
                    .all(|command| command.readiness == CommandReadiness::Ready)
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["mdadm", "--detail", "--scan"])
        }));

        let remove_action = PlannedAction {
            id: "mdRaids:root:remove-device:/dev/disk/by-id/failed-md-member".to_string(),
            description: "remove failed MD member".to_string(),
            operation: Operation::RemoveDevice,
            risk: RiskClass::PotentialDataLoss,
            destructive: false,
            context: ActionContext {
                collection: Some("mdRaids".to_string()),
                name: Some("root".to_string()),
                target: Some("root".to_string()),
                device: Some("/dev/disk/by-id/failed-md-member".to_string()),
                ..ActionContext::default()
            },
            advice: None,
        };
        let (commands, _, _) = commands_for_action(&remove_action);
        assert!(commands.iter().any(|command| {
            command.argv
                == [
                    "mdadm",
                    "<md-array>",
                    "--remove",
                    "/dev/disk/by-id/failed-md-member",
                ]
                && command.readiness == CommandReadiness::NeedsDomainImplementation
                && command.unresolved_inputs == ["MD array path"]
        }));

        let add_action = PlannedAction {
            id: "mdRaids:root:add-device:/dev/disk/by-id/nvme-spare".to_string(),
            description: "add MD member".to_string(),
            operation: Operation::AddDevice,
            risk: RiskClass::Online,
            destructive: false,
            context: ActionContext {
                collection: Some("mdRaids".to_string()),
                name: Some("root".to_string()),
                target: Some("root".to_string()),
                device: Some("/dev/disk/by-id/nvme-spare".to_string()),
                ..ActionContext::default()
            },
            advice: None,
        };
        let replace_action = PlannedAction {
            id: "mdRaids:root:replace-device:/dev/disk/by-id/old-md-member".to_string(),
            description: "replace MD member".to_string(),
            operation: Operation::ReplaceDevice,
            risk: RiskClass::OfflineRequired,
            destructive: false,
            context: ActionContext {
                collection: Some("mdRaids".to_string()),
                name: Some("root".to_string()),
                target: Some("root".to_string()),
                device: Some("/dev/disk/by-id/old-md-member".to_string()),
                replacement: Some("/dev/disk/by-id/new-md-member".to_string()),
                ..ActionContext::default()
            },
            advice: None,
        };
        let (add_commands, _, _) = commands_for_action(&add_action);
        let (replace_commands, _, _) = commands_for_action(&replace_action);
        assert!(add_commands.iter().any(|command| {
            command.argv == ["mdadm", "<md-array>", "--add", "/dev/disk/by-id/nvme-spare"]
                && command.readiness == CommandReadiness::NeedsDomainImplementation
                && command.unresolved_inputs == ["MD array path"]
        }));
        assert!(replace_commands.iter().any(|command| {
            command.argv
                == [
                    "mdadm",
                    "<md-array>",
                    "--replace",
                    "/dev/disk/by-id/old-md-member",
                    "--with",
                    "/dev/disk/by-id/new-md-member",
                ]
                && command.readiness == CommandReadiness::NeedsDomainImplementation
                && command.unresolved_inputs == ["MD array path"]
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
                  },
                  "mpathb": {
                    "target": "mpathb",
                    "operation": "rescan"
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
            step.action_id == "multipathmaps:mpathb:rescan"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["multipath", "-r"] && command.mutates)
                && step
                    .commands
                    .iter()
                    .filter(|command| command.argv == ["multipath", "-ll", "mpathb"])
                    .count()
                    == 2
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["multipathd", "add", "path", "/dev/sdd"])
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["multipathd", "del", "path", "/dev/sdc"])
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
    fn multipath_map_lifecycle_requires_explicit_map_target_for_execute_readiness() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "multipathMaps": {
                  "root-map": {
                    "operation": "grow"
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
        assert!(!report.command_summary.all_commands_ready());
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "multipathmaps:root-map:grow"
                && step.commands.iter().any(|command| {
                    command.argv == ["multipathd", "resize", "map", "<multipath-map>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["multipath map target"]
                })
        }));

        let remove_action = PlannedAction {
            id: "multipathMaps:root-map:remove-device:/dev/sde".to_string(),
            description: "remove stale multipath path".to_string(),
            operation: Operation::RemoveDevice,
            risk: RiskClass::PotentialDataLoss,
            destructive: false,
            context: ActionContext {
                collection: Some("multipathMaps".to_string()),
                name: Some("root-map".to_string()),
                target: Some("root-map".to_string()),
                device: Some("/dev/sde".to_string()),
                ..ActionContext::default()
            },
            advice: None,
        };
        let (commands, _, _) = commands_for_action(&remove_action);
        assert!(commands.iter().any(|command| {
            command.argv == ["multipath", "-ll", "<multipath-map>"]
                && command.readiness == CommandReadiness::NeedsDomainImplementation
                && command.unresolved_inputs == ["multipath map target"]
        }));
    }

    #[test]
    fn thin_pool_lifecycle_reports_lvm_pool_commands_and_verification() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "thinPools": {
                  "vg0/newpool": {
                    "operation": "create",
                    "desiredSize": "100GiB"
                  },
                  "vg0/pool": {
                    "operation": "grow",
                    "desiredSize": "500GiB"
                  },
                  "vg0/reporting": {
                    "operation": "rescan"
                  },
                  "badthin": {
                    "operation": "create"
                  },
                  "vg0/oldpool": {
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
                command.argv
                    == [
                        "lvcreate",
                        "--type",
                        "thin-pool",
                        "--size",
                        "100GiB",
                        "--name",
                        "newpool",
                        "vg0",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv == ["lvextend", "--size", "500GiB", "vg0/pool"]
                    && command.readiness == CommandReadiness::Ready
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "thinpools:vg0/reporting:rescan"
                && step
                    .commands
                    .iter()
                    .all(|command| command.readiness == CommandReadiness::Ready && !command.mutates)
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "lvs",
                            "--reportformat",
                            "json",
                            "-o",
                            "lv_name,lv_size,data_percent,metadata_percent,seg_monitor",
                            "vg0/reporting",
                        ]
                })
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["disk-nix", "inspect", "vg0/reporting"])
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "thinpools:badthin:create"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "lvcreate",
                            "--type",
                            "thin-pool",
                            "--size",
                            "<size>",
                            "--name",
                            "<thin-pool>",
                            "<volume-group>",
                        ]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs
                            == [
                                "target in volume-group/thin-pool form",
                                "desired thin pool size",
                            ]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["lvremove", "--yes", "vg0/oldpool"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "thinpools:vg0/newpool:create"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "lvs",
                            "--reportformat",
                            "json",
                            "-o",
                            "lv_name,lv_size,data_percent,metadata_percent,seg_monitor",
                            "vg0/newpool",
                        ]
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
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "thinpools:vg0/reporting:rescan"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "lvs",
                            "--reportformat",
                            "json",
                            "-o",
                            "lv_name,lv_size,data_percent,metadata_percent,seg_monitor",
                            "vg0/reporting",
                        ]
                })
                && step.commands.iter().any(|command| {
                    command.argv == ["disk-nix", "inspect", "vg0/reporting", "--json"]
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "thinpools:vg0/oldpool:destroy"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["disk-nix", "topology", "--json"])
        }));
    }

    #[test]
    fn lvm_logical_volume_lifecycle_reports_lvm_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "volumes": {
                  "vg0/scratch": {
                    "operation": "create",
                    "desiredSize": "10GiB"
                  },
                  "scratch": {
                    "operation": "create"
                  },
                  "vg0/home": {
                    "operation": "activate"
                  },
                  "vg0/archive": {
                    "operation": "deactivate"
                  },
                  "vg0/reporting": {
                    "operation": "rescan"
                  },
                  "vg0/old": {
                    "destroy": true
                  }
                }
              },
              "apply": {
                "allowDestructive": false,
                "allowOffline": true
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::Blocked);
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv == ["lvcreate", "--size", "10GiB", "--name", "scratch", "vg0"]
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "volumes:scratch:create"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "lvcreate",
                            "--size",
                            "<size>",
                            "--name",
                            "<logical-volume>",
                            "<volume-group>",
                        ]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs
                            == [
                                "target in volume-group/logical-volume form",
                                "desired logical volume size",
                            ]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "volumes:vg0/home:activate"
                && step.commands.iter().any(|command| {
                    command.argv == ["lvchange", "--activate", "y", "vg0/home"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "volumes:vg0/archive:deactivate"
                && step.commands.iter().any(|command| {
                    command.argv == ["lvchange", "--activate", "n", "vg0/archive"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "volumes:vg0/reporting:rescan"
                && step
                    .commands
                    .iter()
                    .all(|command| command.readiness == CommandReadiness::Ready && !command.mutates)
                && step.commands.iter().any(|command| {
                    command.argv == ["lvs", "--reportformat", "json", "vg0/reporting"]
                })
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["disk-nix", "inspect", "vg0/reporting"])
        }));
        assert!(
            !report.command_plan.iter().any(|step| {
                step.commands
                    .iter()
                    .any(|command| command.argv == ["lvremove", "--yes", "vg0/old"])
            }),
            "destructive LV removal remains blocked by default policy"
        );
        assert!(report.verification_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["lvs", "--reportformat", "json", "vg0/scratch"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "volumes:vg0/home:activate"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["lvs", "--reportformat", "json", "vg0/home"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "volumes:vg0/reporting:rescan"
                && step.commands.iter().any(|command| {
                    command.argv == ["lvs", "--reportformat", "json", "vg0/reporting"]
                })
                && step.commands.iter().any(|command| {
                    command.argv == ["disk-nix", "inspect", "vg0/reporting", "--json"]
                })
        }));
    }

    #[test]
    fn lvm_volume_update_and_remove_require_canonical_targets_for_execute_readiness() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "volumes": {
                  "scratch": {
                    "operation": "grow",
                    "desiredSize": "20GiB"
                  },
                  "old": {
                    "destroy": true
                  }
                },
                "thinPools": {
                  "pool": {
                    "operation": "grow",
                    "desiredSize": "200GiB"
                  },
                  "oldpool": {
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
        assert!(!report.command_summary.all_commands_ready());
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "volumes:scratch:grow"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "lvextend",
                            "--resizefs",
                            "--size",
                            "20GiB",
                            "<logical-volume>",
                        ]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs
                            == ["target in volume-group/logical-volume form"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "volumes:old:destroy"
                && step.commands.iter().any(|command| {
                    command.argv == ["lvremove", "--yes", "<logical-volume>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs
                            == ["target in volume-group/logical-volume form"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "thinpools:pool:grow"
                && step.commands.iter().any(|command| {
                    command.argv == ["lvextend", "--size", "200GiB", "<thin-pool>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["target in volume-group/thin-pool form"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "thinpools:oldpool:destroy"
                && step.commands.iter().any(|command| {
                    command.argv == ["lvremove", "--yes", "<thin-pool>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["target in volume-group/thin-pool form"]
                })
        }));
    }

    #[test]
    fn lvm_volume_group_lifecycle_reports_lvm_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "volumeGroups": {
                  "vg0": {
                    "operation": "create",
                    "device": "/dev/disk/by-id/nvme-vg0"
                  },
                  "vgdata": {
                    "operation": "grow",
                    "device": "/dev/disk/by-id/nvme-data-pv"
                  },
                  "vgrefresh": {
                    "operation": "rescan"
                  },
                  "vgmissing": {
                    "operation": "grow"
                  },
                  "vgadd": {
                    "operation": "add-device"
                  },
                  "vgreplace": {
                    "operation": "replace-device",
                    "device": "/dev/disk/by-id/old-pv"
                  },
                  "importvg": {
                    "operation": "import"
                  },
                  "exportvg": {
                    "operation": "export"
                  },
                  "activevg": {
                    "operation": "activate"
                  },
                  "coldvg": {
                    "operation": "deactivate"
                  },
                  "oldvg": {
                    "destroy": true
                  }
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowOffline": true
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv == ["vgcreate", "vg0", "/dev/disk/by-id/nvme-vg0"]
                    && command.readiness == CommandReadiness::Ready
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "volumegroups:vgdata:grow"
                && step.commands.iter().any(|command| {
                    command.argv == ["vgextend", "vgdata", "/dev/disk/by-id/nvme-data-pv"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "volumegroups:vgrefresh:rescan"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["pvscan", "--cache"])
                && step.commands.iter().any(|command| {
                    command.argv == ["vgchange", "--refresh", "vgrefresh"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "volumegroups:vgmissing:grow"
                && step.commands.iter().any(|command| {
                    command.argv == ["vgextend", "vgmissing", "<physical-volume>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["physical volume device"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "volumegroups:vgadd:adddevice"
                && step.commands.iter().any(|command| {
                    command.argv == ["vgextend", "vgadd", "<physical-volume>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["physical volume device"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "volumegroups:vgreplace:replacedevice"
                && step.commands.iter().any(|command| {
                    command.argv == ["vgextend", "vgreplace", "<replacement-physical-volume>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["replacement physical volume"]
                })
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "pvmove",
                            "/dev/disk/by-id/old-pv",
                            "<replacement-physical-volume>",
                        ]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["replacement physical volume"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["vgremove", "--yes", "oldvg"])
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "volumegroups:importvg:import"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["pvs", "--reportformat", "json"])
                && step.commands.iter().any(|command| {
                    command.argv == ["vgimport", "importvg"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "volumegroups:exportvg:export"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["vgs", "--reportformat", "json", "exportvg"])
                && step.commands.iter().any(|command| {
                    command.argv == ["vgexport", "exportvg"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "volumegroups:activevg:activate"
                && step.commands.iter().any(|command| {
                    command.argv == ["vgchange", "--activate", "y", "activevg"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "volumegroups:coldvg:deactivate"
                && step.commands.iter().any(|command| {
                    command.argv == ["vgchange", "--activate", "n", "coldvg"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "volumegroups:vg0:create"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["vgs", "--reportformat", "json", "vg0"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "volumegroups:vgdata:grow"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["pvs", "--reportformat", "json"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "volumegroups:vgrefresh:rescan"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["lvs", "--reportformat", "json", "vgrefresh"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "volumegroups:oldvg:destroy"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["pvs", "--reportformat", "json"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "volumegroups:importvg:import"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["vgs", "--reportformat", "json", "importvg"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "volumegroups:exportvg:export"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["disk-nix", "inspect", "exportvg", "--json"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "volumegroups:activevg:activate"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["lvs", "--reportformat", "json", "activevg"])
        }));
    }

    #[test]
    fn lvm_physical_volume_lifecycle_reports_lvm_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "physicalVolumes": {
                  "/dev/disk/by-id/nvme-pv-new": {
                    "operation": "create"
                  },
                  "/dev/disk/by-id/nvme-pv-grow": {
                    "operation": "grow"
                  },
                  "/dev/disk/by-id/nvme-pv-refresh": {
                    "operation": "rescan"
                  },
                  "/dev/disk/by-id/nvme-pv-old": {
                    "destroy": true
                  }
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowGrow": true
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "physicalvolumes:/dev/disk/by-id/nvme-pv-new:create"
                && step.commands.iter().any(|command| {
                    command.argv == ["pvcreate", "/dev/disk/by-id/nvme-pv-new"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "physicalvolumes:/dev/disk/by-id/nvme-pv-grow:grow"
                && step.commands.iter().any(|command| {
                    command.argv == ["pvresize", "/dev/disk/by-id/nvme-pv-grow"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "physicalvolumes:/dev/disk/by-id/nvme-pv-refresh:rescan"
                && step.commands.iter().any(|command| {
                    command.argv == ["pvscan", "--cache", "/dev/disk/by-id/nvme-pv-refresh"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "physicalvolumes:/dev/disk/by-id/nvme-pv-old:destroy"
                && step.commands.iter().any(|command| {
                    command.argv == ["pvremove", "--yes", "/dev/disk/by-id/nvme-pv-old"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "physicalvolumes:/dev/disk/by-id/nvme-pv-grow:grow"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["pvs", "--reportformat", "json"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "physicalvolumes:/dev/disk/by-id/nvme-pv-refresh:rescan"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["disk-nix", "topology", "--json"])
        }));
    }

    #[test]
    fn lvm_physical_volume_lifecycle_requires_device_path_for_execute_readiness() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "physicalVolumes": {
                  "logical-pv": {
                    "operation": "create"
                  },
                  "refresh-all": {
                    "operation": "rescan"
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
            step.action_id == "physicalvolumes:logical-pv:create"
                && step.commands.iter().any(|command| {
                    command.argv == ["pvcreate", "<physical-volume>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["physical volume device"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "physicalvolumes:refresh-all:rescan"
                && step
                    .commands
                    .iter()
                    .all(|command| command.readiness == CommandReadiness::Ready)
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["pvscan", "--cache"])
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
                  "vg0/root-inspect": {
                    "operation": "rescan"
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
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "lvmsnapshots:vg0/root-inspect:rescan"
                && step
                    .commands
                    .iter()
                    .all(|command| command.readiness == CommandReadiness::Ready && !command.mutates)
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "lvs",
                            "--reportformat",
                            "json",
                            "-o",
                            "lv_name,origin,lv_attr,data_percent,metadata_percent,lv_size",
                            "vg0/root-inspect",
                        ]
                })
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
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "lvmsnapshots:vg0/root-inspect:rescan"
                && step.commands.iter().any(|command| {
                    command.argv == ["lvs", "--reportformat", "json", "vg0/root-inspect"]
                })
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
                  "/dev/loop10": {
                    "operation": "rescan"
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
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "loopdevices:/dev/loop10:rescan"
                && step.commands.iter().any(|command| {
                    command.argv == ["losetup", "--json", "--list", "/dev/loop10"]
                        && !command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
                && step.commands.iter().any(|command| {
                    command.argv == ["disk-nix", "inspect", "/dev/loop10"]
                        && !command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
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
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "loopdevices:/dev/loop10:rescan"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["disk-nix", "inspect", "/dev/loop10", "--json"])
        }));
    }

    #[test]
    fn loop_device_update_and_detach_require_stable_loop_path_for_execute_readiness() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "loopDevices": {
                  "root-image": {
                    "operation": "grow"
                  },
                  "inventory-image": {
                    "operation": "rescan"
                  },
                  "old-image": {
                    "operation": "destroy"
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

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert!(!report.command_summary.all_commands_ready());
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "loopdevices:root-image:grow"
                && step.commands.iter().any(|command| {
                    command.argv == ["losetup", "-c", "<loop-device>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["loop device path"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "loopdevices:inventory-image:rescan"
                && step.commands.iter().any(|command| {
                    command.argv == ["disk-nix", "inspect", "<loop-device>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["loop device path"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "loopdevices:old-image:destroy"
                && step.commands.iter().any(|command| {
                    command.argv == ["losetup", "--detach", "<loop-device>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["loop device path"]
                })
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
    fn execute_refuses_non_ready_command_plans() {
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

        assert_eq!(report.status, ExecutionStatus::NotReady);
        assert!(!report.can_apply());
        assert_eq!(report.command_plan.len(), 1);
        assert!(report.execution_results.is_empty());
        assert!(
            report
                .messages
                .iter()
                .any(|message| message.contains("every planned command must be ready"))
        );
    }

    #[test]
    fn execute_runs_ready_commands_and_verification_with_runner() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "exports": {
                "/srv/share": {
                  "operation": "create",
                  "client": "192.0.2.0/24",
                  "options": "ro,sync"
                }
              }
            }"#,
        )
        .expect("document parses");

        let mut seen = Vec::new();
        let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
            seen.push(argv.to_vec());
            CommandRunResult {
                success: true,
                status_code: Some(0),
                stdout: "ok\n".to_string(),
                stderr: String::new(),
            }
        });

        assert_eq!(report.status, ExecutionStatus::Succeeded);
        assert_eq!(report.execution_results.len(), seen.len());
        assert!(report.execution_results.iter().all(|result| result.success));
        assert!(seen.iter().any(|argv| {
            argv == &[
                "exportfs".to_string(),
                "-i".to_string(),
                "-o".to_string(),
                "ro,sync".to_string(),
                "192.0.2.0/24:/srv/share".to_string(),
            ]
        }));
        assert!(report.execution_results.iter().any(|result| {
            result.phase == ExecutionPhase::Verification && result.argv == ["exportfs", "-v"]
        }));
    }

    #[test]
    fn execute_stops_after_first_failed_command() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "exports": {
                "/srv/share": {
                  "operation": "create",
                  "client": "192.0.2.0/24",
                  "options": "ro,sync"
                }
              }
            }"#,
        )
        .expect("document parses");

        let report =
            prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |_argv| {
                CommandRunResult {
                    success: false,
                    status_code: Some(32),
                    stdout: String::new(),
                    stderr: "export failed".to_string(),
                }
            });

        assert_eq!(report.status, ExecutionStatus::Failed);
        assert_eq!(report.execution_results.len(), 1);
        assert_eq!(report.execution_results[0].status_code, Some(32));
        assert_eq!(report.execution_results[0].stderr, "export failed");
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
                  "operation": "grow",
                  "device": "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0",
                  "devices": [
                    "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-0"
                  ]
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
        assert!(report.command_plan[0].commands.iter().any(|command| {
            command.argv
                == [
                    "sh",
                    "-c",
                    "block=$(basename \"$(readlink -f \"$1\")\"); printf '1\\n' > \"/sys/class/block/${block}/device/rescan\"",
                    "disk-nix-scsi-rescan",
                    "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0",
                ]
                && command.mutates
                && command.readiness == CommandReadiness::Ready
        }));
        assert!(report.command_plan[0].commands.iter().any(|command| {
            command.argv
                == [
                    "sh",
                    "-c",
                    "block=$(basename \"$(readlink -f \"$1\")\"); printf '1\\n' > \"/sys/class/block/${block}/device/rescan\"",
                    "disk-nix-scsi-rescan",
                    "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-0",
                ]
                && command.mutates
                && command.readiness == CommandReadiness::Ready
        }));
        assert!(report.verification_plan[0].commands.iter().any(|command| {
            command.argv
                == [
                    "blockdev",
                    "--getsize64",
                    "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0",
                ]
                && !command.mutates
        }));
    }

    #[test]
    fn host_storage_rescan_reports_online_refresh_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "luns": {
                  "iqn.2026-06.example:storage/root:0": {
                    "operation": "rescan",
                    "devices": [
                      "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0"
                    ]
                  }
                },
                "iscsiSessions": {
                  "iqn.2026-06.example:storage.root": {
                    "operation": "rescan"
                  }
                },
                "nvmeNamespaces": {
                  "/dev/nvme2": {
                    "operation": "rescan"
                  }
                }
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert!(report.apply.allowed_count >= 3);
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luns:iqn.2026-06.example:storage/root:0:rescan"
                && step.commands.iter().any(|command| {
                    command.argv == ["iscsiadm", "--mode", "session", "--rescan"]
                        && command.mutates
                })
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "sh",
                            "-c",
                            "block=$(basename \"$(readlink -f \"$1\")\"); printf '1\\n' > \"/sys/class/block/${block}/device/rescan\"",
                            "disk-nix-scsi-rescan",
                            "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["multipath", "-r"])
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "iscsisessions:iqn.2026-06.example:storage.root:rescan"
                && step.commands.iter().any(|command| {
                    command.argv == ["iscsiadm", "--mode", "session", "--rescan"] && command.mutates
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "nvmenamespaces:/dev/nvme2:rescan"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["nvme", "ns-rescan", "/dev/nvme2"])
        }));
        assert!(report.command_summary.all_commands_ready());
    }

    #[test]
    fn lun_attach_and_detach_reports_host_path_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "luns": {
                "iqn.2026-06.example:storage/root:0": {
                  "operation": "attach",
                  "device": "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0"
                },
                "iqn.2026-06.example:storage/old:1": {
                  "operation": "detach",
                  "devices": [
                    "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-1"
                  ]
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
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luns:iqn.2026-06.example:storage/root:0:attach"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["iscsiadm", "--mode", "session", "--rescan"])
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luns:iqn.2026-06.example:storage/root:0:attach"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "blockdev",
                            "--getsize64",
                            "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0",
                        ]
                        && !command.mutates
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luns:iqn.2026-06.example:storage/old:1:detach"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "sh",
                            "-c",
                            "block=$(basename \"$(readlink -f \"$1\")\"); printf '1\\n' > \"/sys/class/block/${block}/device/delete\"",
                            "disk-nix-scsi-delete",
                            "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-1",
                        ]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "luns:iqn.2026-06.example:storage/old:1:detach"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "test",
                            "!",
                            "-e",
                            "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-1",
                        ]
                        && !command.mutates
                })
        }));
    }

    #[test]
    fn nvme_namespace_lifecycle_reports_nvme_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "nvmeNamespaces": {
                "/dev/nvme0": {
                  "operation": "create",
                  "desiredSize": "100G",
                  "namespaceId": "4",
                  "controllers": "0x1"
                },
                "/dev/nvme1": {
                  "operation": "grow"
                },
                "/dev/nvme2": {
                  "destroy": true,
                  "namespaceId": "7",
                  "controllers": "0x2"
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowGrow": true,
                "allowOffline": true
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "nvmenamespaces:/dev/nvme0:create"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "nvme",
                            "create-ns",
                            "/dev/nvme0",
                            "--nsze-si",
                            "100G",
                            "--ncap-si",
                            "100G",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "nvme",
                            "attach-ns",
                            "/dev/nvme0",
                            "--namespace-id",
                            "4",
                            "--controllers",
                            "0x1",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "nvmenamespaces:/dev/nvme1:grow"
                && step.commands.iter().any(|command| {
                    command.argv == ["nvme", "ns-rescan", "/dev/nvme1"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "nvmenamespaces:/dev/nvme2:destroy"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "nvme",
                            "detach-ns",
                            "/dev/nvme2",
                            "--namespace-id",
                            "7",
                            "--controllers",
                            "0x2",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
                && step.commands.iter().any(|command| {
                    command.argv == ["nvme", "delete-ns", "/dev/nvme2", "--namespace-id", "7"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "nvmenamespaces:/dev/nvme0:create"
                && step.commands.iter().any(|command| {
                    command.argv == ["nvme", "list", "--output-format=json"] && !command.mutates
                })
        }));
    }

    #[test]
    fn nvme_namespace_lifecycle_requires_explicit_namespace_inputs() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "nvmeNamespaces": {
                "logical-ns": {
                  "operation": "create"
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
        let step = report
            .command_plan
            .iter()
            .find(|step| step.action_id == "nvmenamespaces:logical-ns:create")
            .expect("NVMe namespace create command plan exists");
        assert!(step.commands.iter().any(|command| {
            command.argv
                == [
                    "nvme",
                    "create-ns",
                    "<nvme-controller>",
                    "--nsze-si",
                    "<size>",
                    "--ncap-si",
                    "<size>",
                ]
                && command.readiness == CommandReadiness::NeedsDomainImplementation
                && command
                    .unresolved_inputs
                    .contains(&"NVMe controller path such as /dev/nvme0".to_string())
                && command
                    .unresolved_inputs
                    .contains(&"desired namespace size".to_string())
        }));
        assert!(step.commands.iter().any(|command| {
            command.argv
                == [
                    "nvme",
                    "attach-ns",
                    "<nvme-controller>",
                    "--namespace-id",
                    "<namespace-id>",
                    "--controllers",
                    "<controller-id-list>",
                ]
                && command.readiness == CommandReadiness::NeedsDomainImplementation
        }));
    }

    #[test]
    fn lun_attach_and_grow_without_stable_path_reports_unresolved_input() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "luns": {
                "iqn.2026-06.example:storage/new:0": {
                  "operation": "create"
                },
                "iqn.2026-06.example:storage/grow:1": {
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
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luns:iqn.2026-06.example:storage/new:0:create"
                && step.commands.iter().any(|command| {
                    command.argv == ["blockdev", "--getsize64", "<lun-path>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["stable LUN device path"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luns:iqn.2026-06.example:storage/grow:1:grow"
                && step.commands.iter().any(|command| {
                    command.argv == ["<scsi-rescan-device>", "<lun-path>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["stable LUN device path"]
                })
        }));
        assert_eq!(report.command_summary.needs_domain_implementation_count, 2);
        assert!(!report.command_summary.all_commands_ready());
    }

    #[test]
    fn lun_detach_without_stable_path_reports_unresolved_input() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "luns": {
                "iqn.2026-06.example:storage/old:1": {
                  "destroy": true
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
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luns:iqn.2026-06.example:storage/old:1:destroy"
                && step.commands.iter().any(|command| {
                    command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["stable LUN device path"]
                })
        }));
        assert_eq!(report.command_summary.needs_domain_implementation_count, 1);
    }

    #[test]
    fn iscsi_session_lifecycle_reports_login_and_logout_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "iscsiSessions": {
                "iqn.2026-06.example:storage.root": {
                  "operation": "login",
                  "portal": "192.0.2.10:3260"
                },
                "iqn.2026-06.example:storage.old": {
                  "operation": "logout",
                  "metadata": {
                    "portal": "192.0.2.11:3260"
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
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv
                    == [
                        "iscsiadm",
                        "--mode",
                        "discovery",
                        "--type",
                        "sendtargets",
                        "--portal",
                        "192.0.2.10:3260",
                    ]
                    && command.mutates
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv
                    == [
                        "iscsiadm",
                        "--mode",
                        "node",
                        "--targetname",
                        "iqn.2026-06.example:storage.root",
                        "--portal",
                        "192.0.2.10:3260",
                        "--login",
                    ]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv
                    == [
                        "iscsiadm",
                        "--mode",
                        "node",
                        "--targetname",
                        "iqn.2026-06.example:storage.old",
                        "--portal",
                        "192.0.2.11:3260",
                        "--logout",
                    ]
                    && command.mutates
            })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "iscsisessions:iqn.2026-06.example:storage.root:login"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["iscsiadm", "--mode", "session"])
        }));
    }

    #[test]
    fn iscsi_session_login_without_portal_reports_unresolved_input() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "iscsiSessions": {
                "iqn.2026-06.example:storage.root": {
                  "operation": "create"
                }
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
                        "iscsiadm",
                        "--mode",
                        "node",
                        "--targetname",
                        "iqn.2026-06.example:storage.root",
                        "--portal",
                        "<portal>",
                        "--login",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["iSCSI portal"]
            })
        }));
        assert!(!report.command_summary.all_commands_ready());
    }

    #[test]
    fn pool_actions_report_domain_specific_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "pools": {
                "newtank": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/new-pool-vdev"
                },
                "mirrorpool": {
                  "operation": "create",
                  "devices": [
                    "mirror",
                    "/dev/disk/by-id/mirror-a",
                    "/dev/disk/by-id/mirror-b"
                  ]
                },
                "tank": {
                  "operation": "rebalance",
                  "addDevices": ["/dev/disk/by-id/new"],
                  "properties": {
                    "autotrim": "on"
                  }
                },
                "vault": {
                  "operation": "import",
                  "readOnly": true
                },
                "moveme": {
                  "operation": "export"
                },
                "oldtank": {
                  "destroy": true
                }
              },
              "snapshots": {
                "tank/home@before": {
                  "target": "tank/home"
                },
                "/mnt/persist/@home-before": {
                  "target": "/mnt/persist/@home",
                  "readOnly": true
                }
              },
              "apply": {
                "allowGrow": true,
                "allowDestructive": true,
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
                        "zpool",
                        "create",
                        "newtank",
                        "/dev/disk/by-id/new-pool-vdev",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv
                    == [
                        "zpool",
                        "create",
                        "mirrorpool",
                        "mirror",
                        "/dev/disk/by-id/mirror-a",
                        "/dev/disk/by-id/mirror-b",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "pools:mirrorpool:create"
                && step.commands.iter().any(|command| {
                    command.argv == ["disk-nix", "inspect", "/dev/disk/by-id/mirror-a"]
                })
                && step.commands.iter().any(|command| {
                    command.argv == ["disk-nix", "inspect", "/dev/disk/by-id/mirror-b"]
                })
                && !step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["disk-nix", "inspect", "mirror"])
        }));
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
            step.action_id == "pools:vault:import"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["zpool", "import"])
                && step.commands.iter().any(|command| {
                    command.argv == ["zpool", "import", "-o", "readonly=on", "vault"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "pools:moveme:export"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["zpool", "status", "-P", "moveme"])
                && step.commands.iter().any(|command| {
                    command.argv == ["zpool", "export", "moveme"]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["zfs", "snapshot", "tank/home@before"])
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv
                    == [
                        "btrfs",
                        "subvolume",
                        "snapshot",
                        "-r",
                        "/mnt/persist/@home",
                        "/mnt/persist/@home-before",
                    ]
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["zpool", "destroy", "oldtank"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "pools:newtank:create"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["zpool", "list", "-H", "-p", "newtank"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["zpool", "status", "-P", "tank"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "pools:vault:import"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["zpool", "status", "-P", "vault"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "pools:moveme:export"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["disk-nix", "inspect", "moveme", "--json"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "pools:oldtank:destroy"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["disk-nix", "topology", "--json"])
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
        assert!(report.verification_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv == ["btrfs", "subvolume", "show", "/mnt/persist/@home-before"]
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
                    "operation": "rescan",
                    "addDevices": ["cache-set-uuid"],
                    "removeDevices": ["cache-set-uuid"],
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
            step.action_id == "caches:/dev/bcache0:rescan"
                && step
                    .commands
                    .iter()
                    .all(|command| command.readiness == CommandReadiness::Ready && !command.mutates)
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["disk-nix", "inspect", "/dev/bcache0"])
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "sh",
                            "-c",
                            "cat \"/sys/block/${1#/dev/}/bcache/$2\"",
                            "disk-nix-bcache-read",
                            "/dev/bcache0",
                            "state",
                        ]
                })
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "sh",
                            "-c",
                            "cat \"/sys/block/${1#/dev/}/bcache/$2\"",
                            "disk-nix-bcache-read",
                            "/dev/bcache0",
                            "cache_mode",
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
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "caches:/dev/bcache0:remove-device:cache-set-uuid"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "sh",
                            "-c",
                            "printf '1\\n' > \"/sys/block/${1#/dev/}/bcache/detach\"",
                            "disk-nix-bcache-detach",
                            "/dev/bcache0",
                        ]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
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
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "caches:/dev/bcache0:rescan"
                && step.commands.iter().any(|command| {
                    command.argv == ["disk-nix", "inspect", "/dev/bcache0", "--json"]
                })
                && step.commands.iter().any(|command| {
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

    #[test]
    fn lvm_cache_lifecycle_reports_lvm_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "lvmCaches": {
                  "vg0/root": {
                    "operation": "create",
                    "device": "vg0/root-cache",
                    "addDevices": ["vg0/root-cache"],
                    "removeDevices": ["vg0/root-cache"],
                    "replaceDevices": {
                      "vg0/root-cache": "vg0/root-cache-new"
                    },
                    "properties": {
                      "lvm.cache-mode": "writethrough"
                    },
                    "destroy": true
                  },
                  "vg0/archive": {
                    "operation": "rescan"
                  }
                }
              },
              "apply": {
                "allowOffline": true,
                "allowDeviceReplacement": true,
                "confirmation": true
              }
            }"#,
        )
        .expect("document parses");

        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "lvmcaches:vg0/root:create"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "lvconvert",
                            "--type",
                            "cache",
                            "--cachepool",
                            "vg0/root-cache",
                            "vg0/root",
                        ]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "lvmCaches:vg0/root:add-device:vg0/root-cache"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "lvconvert",
                            "--type",
                            "cache",
                            "--cachepool",
                            "vg0/root-cache",
                            "vg0/root",
                        ]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "lvmCaches:vg0/root:set-property:lvm.cache-mode"
                && step.commands.iter().any(|command| {
                    command.argv == ["lvchange", "--cachemode", "writethrough", "vg0/root"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "lvmCaches:vg0/root:remove-device:vg0/root-cache"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["lvconvert", "--uncache", "vg0/root"])
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "lvmcaches:vg0/root:destroy"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["lvconvert", "--uncache", "vg0/root"])
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "lvmcaches:vg0/archive:rescan"
                && step
                    .commands
                    .iter()
                    .all(|command| command.readiness == CommandReadiness::Ready && !command.mutates)
                && step.commands.iter().any(|command| {
                    command.argv.len() >= 6
                        && command.argv[0] == "lvs"
                        && command.argv[1] == "--reportformat"
                        && command.argv[2] == "json"
                        && command.argv[3] == "-o"
                        && command.argv[5] == "vg0/archive"
                })
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["disk-nix", "inspect", "vg0/archive"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "lvmCaches:vg0/root:set-property:lvm.cache-mode"
                && step.commands.iter().any(|command| {
                    command.argv.len() >= 4
                        && command.argv[0] == "lvs"
                        && command.argv[1] == "--reportformat"
                        && command.argv[2] == "json"
                        && command.argv[3] == "-o"
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "lvmcaches:vg0/archive:rescan"
                && step.commands.iter().any(|command| {
                    command.argv.len() >= 6
                        && command.argv[0] == "lvs"
                        && command.argv[1] == "--reportformat"
                        && command.argv[2] == "json"
                        && command.argv[3] == "-o"
                        && command.argv[5] == "vg0/archive"
                })
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["disk-nix", "inspect", "vg0/archive", "--json"])
        }));
    }

    #[test]
    fn lvm_cache_lifecycle_requires_origin_and_cache_pool_for_execute_readiness() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "lvmCaches": {
                  "root-cache": {
                    "operation": "create",
                    "properties": {
                      "lvm.cache-policy": "smq"
                    }
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
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "lvmcaches:root-cache:create"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "lvconvert",
                            "--type",
                            "cache",
                            "--cachepool",
                            "<cache-pool>",
                            "<origin-logical-volume>",
                        ]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs
                            == [
                                "target in volume-group/logical-volume form",
                                "cache-pool logical volume",
                            ]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "lvmCaches:root-cache:set-property:lvm.cache-policy"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "lvchange",
                            "--cachepolicy",
                            "smq",
                            "<origin-logical-volume>",
                        ]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs
                            == ["target in volume-group/logical-volume form"]
                })
        }));
    }

    #[test]
    fn cache_lifecycle_requires_bcache_device_path_for_execute_readiness() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "caches": {
                  "cache0": {
                    "addDevices": ["cache-set-uuid"],
                    "removeDevices": ["cache-set-uuid"],
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
        assert!(!report.command_summary.all_commands_ready());
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "caches:cache0:add-device:cache-set-uuid"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "sh",
                            "-c",
                            "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/attach\"",
                            "disk-nix-bcache-attach",
                            "<cache-device>",
                            "cache-set-uuid",
                        ]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["bcache device path"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "caches:cache0:set-property:bcache.cache-mode"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "sh",
                            "-c",
                            "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/$3\"",
                            "disk-nix-bcache-property",
                            "<cache-device>",
                            "writethrough",
                            "cache_mode",
                        ]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["bcache device path"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "caches:cache0:remove-device:cache-set-uuid"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "sh",
                            "-c",
                            "printf '1\\n' > \"/sys/block/${1#/dev/}/bcache/detach\"",
                            "disk-nix-bcache-detach",
                            "<cache-device>",
                        ]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["bcache device path"]
                })
        }));
    }

    #[test]
    fn nfs_export_lifecycle_reports_exportfs_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "exports": {
                  "/srv/share": {
                    "operation": "export",
                    "client": "192.0.2.0/24",
                    "options": "rw,sync,no_subtree_check"
                  },
                  "/srv/changed": {
                    "client": "192.0.2.0/24",
                    "properties": {
                      "options": "ro,sync,no_subtree_check"
                    }
                  },
                  "/srv/inventory": {
                    "operation": "rescan"
                  },
                  "/srv/unresolved": {
                    "properties": {
                      "options": "rw,sync"
                    }
                  },
                  "/srv/old": {
                    "operation": "unexport",
                    "client": "192.0.2.55"
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
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv
                    == [
                        "exportfs",
                        "-i",
                        "-o",
                        "rw,sync,no_subtree_check",
                        "192.0.2.0/24:/srv/share",
                    ]
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "exports:/srv/inventory:rescan"
                && step.commands.iter().any(|command| {
                    command.argv == ["exportfs", "-v"]
                        && !command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
                && step.commands.iter().any(|command| {
                    command.argv == ["disk-nix", "inspect", "/srv/inventory"]
                        && !command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "exports:/srv/inventory:rescan"
                && step.commands.iter().any(|command| {
                    command.argv == ["disk-nix", "inspect", "/srv/inventory", "--json"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv
                    == [
                        "exportfs",
                        "-i",
                        "-o",
                        "ro,sync,no_subtree_check",
                        "192.0.2.0/24:/srv/changed",
                    ]
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv
                    == [
                        "exportfs",
                        "-i",
                        "-o",
                        "rw,sync",
                        "<client>:/srv/unresolved",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["NFS client selector"]
            })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["exportfs", "-u", "192.0.2.55:/srv/old"])
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["exportfs", "-v"])
        }));
    }

    #[test]
    fn nfs_export_lifecycle_requires_path_for_execute_readiness() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "exports": {
                  "share": {
                    "operation": "create",
                    "client": "192.0.2.0/24",
                    "options": "rw,sync,no_subtree_check"
                  },
                  "inventory": {
                    "operation": "rescan"
                  },
                  "oldshare": {
                    "destroy": true,
                    "client": "192.0.2.55"
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
        assert!(!report.command_summary.all_commands_ready());
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "exports:share:create"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "exportfs",
                            "-i",
                            "-o",
                            "rw,sync,no_subtree_check",
                            "192.0.2.0/24:<export-path>",
                        ]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["NFS export path"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "exports:inventory:rescan"
                && step.commands.iter().any(|command| {
                    command.argv == ["disk-nix", "inspect", "<export-path>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["NFS export path"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "exports:oldshare:destroy"
                && step.commands.iter().any(|command| {
                    command.argv == ["exportfs", "-u", "192.0.2.55:<export-path>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["NFS export path"]
                })
        }));
    }

    #[test]
    fn nfs_mount_lifecycle_reports_mount_commands() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "nfs": {
                  "mounts": {
                    "/srv/shared": {
                      "operation": "mount",
                      "source": "nas.example.com:/srv/shared",
                      "fsType": "nfs4",
                      "options": ["_netdev", "vers=4.2"]
                    },
                    "/srv/tuned": {
                      "operation": "remount",
                      "source": "nas.example.com:/srv/tuned",
                      "options": ["_netdev", "ro", "vers=4.2"]
                    },
                    "/srv/inventory": {
                      "operation": "rescan",
                      "source": "nas.example.com:/srv/inventory"
                    },
                    "/srv/old": {
                      "operation": "unmount",
                      "source": "nas.example.com:/srv/old"
                    }
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
        assert!(report.command_summary.all_commands_ready());
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "nfs.mounts:/srv/shared:mount"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "mount",
                            "-t",
                            "nfs4",
                            "-o",
                            "_netdev,vers=4.2",
                            "nas.example.com:/srv/shared",
                            "/srv/shared",
                        ]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "nfs.mounts:/srv/tuned:remount"
                && step.commands.iter().any(|command| {
                    command.argv == ["mount", "-o", "remount,_netdev,ro,vers=4.2", "/srv/tuned"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "nfs.mounts:/srv/inventory:rescan"
                && step.commands.iter().any(|command| {
                    command.argv == ["findmnt", "--json", "/srv/inventory"]
                        && !command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
                && step.commands.iter().any(|command| {
                    command.argv == ["nfsstat", "-m", "/srv/inventory"]
                        && !command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
        assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "nfs.mounts:/srv/inventory:rescan"
                && step.commands.iter().any(|command| {
                    command.argv == ["disk-nix", "inspect", "/srv/inventory", "--json"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "nfs.mounts:/srv/old:unmount"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["umount", "/srv/old"])
        }));
    }

    #[test]
    fn nfs_mount_lifecycle_requires_mountpoint_for_execute_readiness() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "nfs": {
                  "mounts": {
                    "shared": {
                      "operation": "mount",
                      "source": "nas.example.com:/srv/shared"
                    },
                    "tuned": {
                      "operation": "remount",
                      "source": "nas.example.com:/srv/tuned",
                      "options": ["ro"]
                    },
                    "inventory": {
                      "operation": "rescan",
                      "source": "nas.example.com:/srv/inventory"
                    },
                    "old": {
                      "operation": "unmount",
                      "source": "nas.example.com:/srv/old"
                    }
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
        assert!(!report.command_summary.all_commands_ready());
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "nfs.mounts:shared:mount"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "mount",
                            "-t",
                            "nfs4",
                            "nas.example.com:/srv/shared",
                            "<mountpoint>",
                        ]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["mountpoint path"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "nfs.mounts:tuned:remount"
                && step.commands.iter().any(|command| {
                    command.argv == ["mount", "-o", "remount,ro", "<mountpoint>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["mountpoint path"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "nfs.mounts:inventory:rescan"
                && step.commands.iter().any(|command| {
                    command.argv == ["nfsstat", "-m", "<mountpoint>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["mountpoint path"]
                })
        }));
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == "nfs.mounts:old:unmount"
                && step.commands.iter().any(|command| {
                    command.argv == ["umount", "<mountpoint>"]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["mountpoint path"]
                })
        }));
    }
}
