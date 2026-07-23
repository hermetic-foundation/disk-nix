fn recovery_actions_for_report(report: &ExecutionReport) -> Vec<RecoveryAction> {
    let mut actions = match report.status {
        ExecutionStatus::Blocked => blocked_recovery_actions(report),
        ExecutionStatus::NotReady => not_ready_recovery_actions(report),
        ExecutionStatus::Failed => failed_recovery_actions(report),
        ExecutionStatus::DryRun | ExecutionStatus::Succeeded => Vec::new(),
    };

    if report.status == ExecutionStatus::Failed && report_has_mutating_or_risky_steps(report) {
        actions.push(RecoveryAction {
            kind: RecoveryActionKind::PreserveRecoveryPoints,
            summary: "Preserve backups, snapshots, and captured metadata until recovery is complete"
                .to_string(),
            commands: Vec::new(),
            notes: vec![
                "do not prune snapshots, LUKS headers, partition tables, or prior apply reports while investigating a partial apply".to_string(),
                "prefer clone, snapshot, import read-only, or mount read-only workflows before rollback or destroy operations".to_string(),
            ],
        });
    }

    actions
}

fn blocked_recovery_actions(report: &ExecutionReport) -> Vec<RecoveryAction> {
    vec![
        RecoveryAction {
            kind: RecoveryActionKind::ReviewPolicy,
            summary: "Review blocked actions and choose a safer update path before execution"
                .to_string(),
            commands: Vec::new(),
            notes: vec![
                format!(
                    "policy blocked {} action(s): {} destructive, {} potential data loss, {} offline required, {} unsupported",
                    report.apply.blocked_count,
                    report.apply.blocked_summary.destructive_count,
                    report.apply.blocked_summary.potential_data_loss_count,
                    report.apply.blocked_summary.offline_required_count,
                    report.apply.blocked_summary.unsupported_count
                ),
                "prefer non-destructive alternatives from action advice before enabling broader policy gates".to_string(),
            ],
        },
        RecoveryAction {
            kind: RecoveryActionKind::InspectCurrentState,
            summary: "Refresh current storage state before changing policy or spec".to_string(),
            commands: state_inspection_commands(),
            notes: vec![
                "rerun planning with current topology after editing policy, desired state, or safety gates"
                    .to_string(),
            ],
        },
    ]
}

fn not_ready_recovery_actions(report: &ExecutionReport) -> Vec<RecoveryAction> {
    let missing_tool_count = report
        .tool_requirements
        .iter()
        .filter(|requirement| requirement.availability == ToolAvailability::Missing)
        .count();
    let graph_dependency_conflict_count =
        graph_dependency_conflict_count(report.topology_comparison.as_ref());
    let partially_suppressed_group_count =
        partially_suppressed_reconciliation_group_count(report.topology_comparison.as_ref());
    vec![
        RecoveryAction {
            kind: RecoveryActionKind::ResolveInputs,
            summary: "Resolve unresolved command inputs before requesting execution".to_string(),
            commands: Vec::new(),
            notes: vec![format!(
                "{} command(s) need desired size, {} need domain command implementation, {} are manual-only, {} required tool(s) are missing, {} graph dependency conflict(s) need plan splitting or ordering review, {} partially suppressed reconciliation group(s) need fresh-topology review",
                report.command_summary.needs_desired_size_count,
                report.command_summary.needs_domain_implementation_count,
                report.command_summary.manual_only_count,
                missing_tool_count,
                graph_dependency_conflict_count,
                partially_suppressed_group_count
            )],
        },
        RecoveryAction {
            kind: RecoveryActionKind::InspectCurrentState,
            summary: "Compare the spec with fresh topology after filling missing inputs".to_string(),
            commands: state_inspection_commands(),
            notes: vec![
                "non-ready command plans do not mutate storage; fix declarations or renderer support first"
                    .to_string(),
            ],
        },
    ]
}

fn failed_recovery_actions(report: &ExecutionReport) -> Vec<RecoveryAction> {
    let failed = report
        .execution_results
        .iter()
        .find(|result| !result.success);
    let mut actions = vec![
        RecoveryAction {
            kind: RecoveryActionKind::ReviewExecutionFailure,
            summary: "Review the first failed command before running additional mutations".to_string(),
            commands: Vec::new(),
            notes: failed
                .map(failed_result_notes)
                .unwrap_or_else(|| vec!["execution failed before a command result was recorded".to_string()]),
        },
        RecoveryAction {
            kind: RecoveryActionKind::InspectCurrentState,
            summary: "Capture current topology and probe diagnostics after the stopped apply".to_string(),
            commands: state_inspection_commands(),
            notes: vec![
                "compare current topology with the saved apply report before deciding whether to resume, roll forward, or roll back".to_string(),
            ],
        },
        RecoveryAction {
            kind: RecoveryActionKind::ResumeAfterFix,
            summary: "Resume only after validation shows the remaining plan is ready".to_string(),
            commands: Vec::new(),
            notes: vec![
                "rerun validate and dry-run apply against the current host before using --execute again"
                    .to_string(),
                "do not rerun destructive, rollback, or format commands blindly; inspect whether the prior command already changed state".to_string(),
            ],
        },
    ];

    if failed.is_some_and(|result| result.phase == ExecutionPhase::Verification) {
        actions.push(RecoveryAction {
            kind: RecoveryActionKind::RunVerification,
            summary: "Repeat read-only verification after repairing the reported condition".to_string(),
            commands: verification_commands_for_report(report),
            notes: vec![
                "a verification failure means planned commands ran first; confirm actual state before any rollback attempt".to_string(),
            ],
        });
    }
    actions.extend(domain_recovery_actions_for_failure(report, failed));

    actions
}

fn domain_recovery_actions_for_failure(
    report: &ExecutionReport,
    failed: Option<&ExecutionCommandResult>,
) -> Vec<RecoveryAction> {
    let Some(failed) = failed else {
        return Vec::new();
    };
    let Some(step) = report
        .command_plan
        .iter()
        .find(|step| step.action_id == failed.action_id)
    else {
        return Vec::new();
    };
    if !requires_domain_recovery(step) {
        return Vec::new();
    }

    let completed_mutating_commands = report
        .execution_results
        .iter()
        .take_while(|result| result.action_id != failed.action_id || result.argv != failed.argv)
        .filter(|result| {
            result.success
                && result.phase == ExecutionPhase::Command
                && command_plan_command(report, result).is_some_and(|command| command.mutates)
        })
        .count();
    let mut actions = vec![RecoveryAction {
        kind: RecoveryActionKind::DomainRecovery,
        summary: domain_recovery_summary(step),
        commands: domain_recovery_commands(step),
        notes: domain_recovery_notes(step, failed, completed_mutating_commands),
    }];
    actions.extend(roll_forward_recovery_actions(report, step, failed));
    actions.extend(rollback_recovery_actions(step, failed));
    actions
}

fn requires_domain_recovery(step: &ExecutionStep) -> bool {
    if matches!(
        step.risk,
        RiskClass::Destructive | RiskClass::PotentialDataLoss | RiskClass::Irreversible
    ) {
        return true;
    }

    if matches!(
        (step.operation, command_step_collection(step)),
        (
            Operation::Create | Operation::Destroy | Operation::Login | Operation::Logout,
            Some("iscsiSessions")
        ) | (
            Operation::Create
                | Operation::Destroy
                | Operation::Grow
                | Operation::SetProperty
                | Operation::Start
                | Operation::Stop,
            Some("vdoVolumes")
        ) | (
            Operation::AddDevice
                | Operation::Destroy
                | Operation::Grow
                | Operation::RemoveDevice
                | Operation::ReplaceDevice
                | Operation::Rescan,
            Some("multipathMaps")
        ) | (
            Operation::Close
                | Operation::Create
                | Operation::Destroy
                | Operation::Format
                | Operation::Grow
                | Operation::Open
                | Operation::SetProperty,
            Some("luks.devices")
        ) | (
            Operation::AddKey
                | Operation::Create
                | Operation::Destroy
                | Operation::RemoveKey
                | Operation::SetProperty,
            Some("luksKeyslots")
        ) | (
            Operation::Create
                | Operation::Destroy
                | Operation::ImportToken
                | Operation::RemoveToken
                | Operation::SetProperty,
            Some("luksTokens")
        ) | (
            Operation::Create | Operation::Grow | Operation::Rescan,
            Some("partitions")
        ) | (Operation::Create | Operation::Rescan, Some("disks"))
            | (
                Operation::Create
                    | Operation::Destroy
                    | Operation::Export
                    | Operation::Rescan
                    | Operation::SetProperty
                    | Operation::Unexport,
                Some("exports")
            )
            | (
                Operation::Create
                    | Operation::Destroy
                    | Operation::Mount
                    | Operation::Remount
                    | Operation::Rescan
                    | Operation::Unmount,
                Some("nfs.mounts")
            )
            | (
                Operation::Create | Operation::Grow | Operation::Rescan,
                Some("backingFiles")
            )
            | (
                Operation::Create | Operation::Destroy | Operation::Grow | Operation::Rescan,
                Some("loopDevices")
            )
            | (
                Operation::Destroy | Operation::Rename | Operation::Rescan,
                Some("dmMaps")
            )
            | (
                Operation::AddDevice
                    | Operation::Check
                    | Operation::Format
                    | Operation::Grow
                    | Operation::Mount
                    | Operation::Rebalance
                    | Operation::RemoveDevice
                    | Operation::Remount
                    | Operation::Repair
                    | Operation::Rescan
                    | Operation::Scrub
                    | Operation::SetProperty
                    | Operation::Shrink
                    | Operation::Trim
                    | Operation::Unmount,
                Some("filesystems")
            )
            | (
                Operation::Activate
                    | Operation::Create
                    | Operation::Deactivate
                    | Operation::Destroy
                    | Operation::Grow
                    | Operation::Rename
                    | Operation::Rescan
                    | Operation::Shrink,
                Some("volumes" | "thinPools" | "physicalVolumes")
            )
            | (
                Operation::Activate
                    | Operation::AddDevice
                    | Operation::Create
                    | Operation::Deactivate
                    | Operation::Destroy
                    | Operation::Export
                    | Operation::Grow
                    | Operation::Import
                    | Operation::RemoveDevice
                    | Operation::Rename
                    | Operation::ReplaceDevice
                    | Operation::Rescan,
                Some("volumeGroups")
            )
            | (
                Operation::AddDevice
                    | Operation::Create
                    | Operation::Destroy
                    | Operation::Export
                    | Operation::Grow
                    | Operation::Import
                    | Operation::Promote
                    | Operation::Rebalance
                    | Operation::RemoveDevice
                    | Operation::Rename
                    | Operation::ReplaceDevice
                    | Operation::Rescan
                    | Operation::Scrub
                    | Operation::SetProperty,
                Some("pools" | "datasets" | "zvols")
            )
            | (
                Operation::Clone
                    | Operation::Create
                    | Operation::Destroy
                    | Operation::Rename
                    | Operation::Rescan
                    | Operation::Rollback
                    | Operation::SetProperty,
                Some("snapshots")
            )
            | (
                Operation::Create
                    | Operation::Destroy
                    | Operation::Rename
                    | Operation::Rescan
                    | Operation::SetProperty,
                Some("btrfsSubvolumes")
            )
            | (
                Operation::Create | Operation::Destroy | Operation::Rescan | Operation::SetProperty,
                Some("btrfsQgroups")
            )
            | (
                Operation::AddDevice
                    | Operation::Create
                    | Operation::Destroy
                    | Operation::RemoveDevice
                    | Operation::ReplaceDevice
                    | Operation::Rescan
                    | Operation::SetProperty,
                Some("caches" | "lvmCaches")
            )
            | (
                Operation::Create
                    | Operation::Deactivate
                    | Operation::Destroy
                    | Operation::Format
                    | Operation::Grow
                    | Operation::Rescan
                    | Operation::SetProperty,
                Some("swaps")
            )
            | (
                Operation::Attach
                    | Operation::Create
                    | Operation::Destroy
                    | Operation::Detach
                    | Operation::Grow,
                Some("nvmeNamespaces")
            )
            | (
                Operation::Attach
                    | Operation::Create
                    | Operation::Destroy
                    | Operation::Detach
                    | Operation::Grow
                    | Operation::Rescan,
                Some("luns")
            )
            | (
                Operation::Attach
                    | Operation::Create
                    | Operation::Destroy
                    | Operation::Detach
                    | Operation::Grow
                    | Operation::SetProperty
                    | Operation::Rescan,
                Some("targetLuns")
            )
    ) {
        return true;
    }

    matches!(
        step.operation,
        Operation::Rollback
            | Operation::Shrink
            | Operation::Attach
            | Operation::ReplaceDevice
            | Operation::AddDevice
            | Operation::RemoveDevice
            | Operation::Destroy
            | Operation::Detach
            | Operation::Close
            | Operation::Unmount
            | Operation::Logout
            | Operation::Deactivate
            | Operation::Stop
    )
}

fn command_plan_command<'a>(
    report: &'a ExecutionReport,
    result: &ExecutionCommandResult,
) -> Option<&'a ExecutionCommand> {
    command_plan_command_by_result(&report.command_plan, result)
}

fn command_plan_command_by_result<'a>(
    command_plan: &'a [ExecutionStep],
    result: &ExecutionCommandResult,
) -> Option<&'a ExecutionCommand> {
    command_plan
        .iter()
        .find(|step| step.action_id == result.action_id)?
        .commands
        .iter()
        .find(|command| command.argv == result.argv)
}

fn domain_recovery_summary(step: &ExecutionStep) -> String {
    format!(
        "Plan domain-specific recovery for {:?} action {} after partial execution",
        step.operation, step.action_id
    )
}

fn roll_forward_recovery_actions(
    report: &ExecutionReport,
    step: &ExecutionStep,
    failed: &ExecutionCommandResult,
) -> Vec<RecoveryAction> {
    if !requires_domain_recovery(step) {
        return Vec::new();
    }

    let mut commands = vec![manual_probe_current_apply_command()];
    commands.extend(domain_roll_forward_inspection_commands(step));
    commands.extend(verification_commands_for_report(report));

    vec![RecoveryAction {
        kind: RecoveryActionKind::RollForwardReview,
        summary: format!(
            "Review whether completing the remaining {:?} workflow is safer than rollback",
            step.operation
        ),
        commands,
        notes: vec![
            format!(
                "base the roll-forward decision on fresh topology after failed command: {}",
                failed.argv.join(" ")
            ),
            "prefer roll-forward when data placement, metadata generation, or exported state may already have advanced".to_string(),
            "remove or skip commands that current topology proves already succeeded before resuming execution".to_string(),
        ],
    }]
}

fn rollback_recovery_actions(
    step: &ExecutionStep,
    failed: &ExecutionCommandResult,
) -> Vec<RecoveryAction> {
    let Some(action) = rollback_recovery_action(step, failed) else {
        return Vec::new();
    };
    vec![action]
}

fn rollback_recovery_action(
    step: &ExecutionStep,
    failed: &ExecutionCommandResult,
) -> Option<RecoveryAction> {
    let commands = domain_rollback_inspection_commands(step);
    if commands.is_empty() {
        return None;
    }

    Some(RecoveryAction {
        kind: RecoveryActionKind::RollbackReview,
        summary: format!(
            "Review rollback preconditions for {:?} action {}",
            step.operation, step.action_id
        ),
        commands,
        notes: vec![
            format!(
                "rollback review starts from failed command: {}",
                failed.argv.join(" ")
            ),
            "do not run rollback tooling until read-only checks prove the rollback point and consumers are consistent".to_string(),
            "capture a fresh post-failure snapshot, metadata export, or report before attempting any rollback when the domain supports it".to_string(),
        ],
    })
}

fn manual_probe_current_apply_command() -> ExecutionCommand {
    command_with_readiness(
        [
            "disk-nix",
            "apply",
            "--spec",
            "<spec>",
            "--probe-current",
            "--json",
        ],
        false,
        CommandReadiness::ManualOnly,
        ["original spec path"],
        "rerun apply as a fresh current-topology dry run before resuming",
    )
}
