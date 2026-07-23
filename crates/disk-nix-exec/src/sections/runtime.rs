fn state_inspection_commands() -> Vec<ExecutionCommand> {
    vec![
        command(
            ["disk-nix", "probe-status", "--json"],
            false,
            "inspect probe tool availability and degradation categories",
        ),
        command(
            ["disk-nix", "topology", "--json"],
            false,
            "capture the current storage graph before resuming or rolling back",
        ),
    ]
}

fn verification_commands_for_report(report: &ExecutionReport) -> Vec<ExecutionCommand> {
    report
        .verification_plan
        .iter()
        .flat_map(|step| step.commands.iter().cloned())
        .collect()
}

fn report_has_mutating_or_risky_steps(report: &ExecutionReport) -> bool {
    report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| command.mutates)
            || matches!(
                step.risk,
                RiskClass::Destructive | RiskClass::PotentialDataLoss | RiskClass::Irreversible
            )
    })
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

fn command_exists(tool: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg("command -v -- \"$1\" >/dev/null 2>&1")
        .arg("disk-nix-command-exists")
        .arg(tool)
        .status()
        .is_ok_and(|status| status.success())
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
    let success = result.success
        || verification_result_matches_expected_absence(phase, action_id, argv, &result);
    ExecutionCommandResult {
        phase,
        action_id: action_id.to_string(),
        argv: argv.to_vec(),
        success,
        status_code: result.status_code,
        stdout: result.stdout,
        stderr: result.stderr,
    }
}

fn verification_result_matches_expected_absence(
    phase: ExecutionPhase,
    action_id: &str,
    argv: &[String],
    result: &CommandRunResult,
) -> bool {
    if phase != ExecutionPhase::Verification
        || !action_id.starts_with("luks.devices:")
        || !(action_id.ends_with(":close") || action_id.ends_with(":destroy"))
        || argv.len() != 3
        || argv[0] != "cryptsetup"
        || argv[1] != "status"
        || result.status_code != Some(4)
    {
        return false;
    }

    let output = format!("{}{}", result.stdout, result.stderr).to_ascii_lowercase();
    output.contains("inactive")
        || output.contains("not active")
        || output.contains("does not exist")
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

fn summarize_tool_requirements(
    command_plan: &[ExecutionStep],
    verification_plan: &[VerificationStep],
    tool_exists: impl Fn(&str) -> bool,
) -> Vec<ToolRequirement> {
    let mut requirements = BTreeMap::<String, ToolRequirement>::new();

    for command in command_plan.iter().flat_map(|step| &step.commands) {
        register_tool_requirement(&mut requirements, ExecutionPhase::Command, command);
    }
    for command in verification_plan.iter().flat_map(|step| &step.commands) {
        register_tool_requirement(&mut requirements, ExecutionPhase::Verification, command);
    }

    requirements
        .into_values()
        .map(|mut requirement| {
            let available = tool_exists(&requirement.tool);
            requirement.availability = if available {
                ToolAvailability::Available
            } else {
                ToolAvailability::Missing
            };
            requirement.message = if available {
                format!("{} is available on PATH", requirement.tool)
            } else {
                format!("{} is missing from PATH", requirement.tool)
            };
            requirement.remediation = tool_remediation(&requirement.tool, available);
            requirement
        })
        .collect()
}

fn tool_remediation(tool: &str, available: bool) -> Vec<String> {
    if tool == "disk-nix" {
        return if available {
            vec![
                "disk-nix was found on PATH; keep the configured disk-nix package available to verification commands".to_string(),
            ]
        } else {
            vec![
                "make the configured disk-nix package available on PATH for verification commands".to_string(),
                "when using the NixOS module, keep services.disk-nix.package installed in the apply service environment".to_string(),
            ]
        };
    }

    let Some(package) = nix_package_for_tool(tool) else {
        return vec![format!(
            "install a package that provides {tool}, then rerun disk-nix apply"
        )];
    };

    let package_hint =
        format!("install a package that provides {tool}; on NixOS this is pkgs.{package}");
    if available {
        vec![format!(
            "{tool} was found on PATH; keep pkgs.{package} available to the disk-nix apply environment"
        )]
    } else if disk_nix_default_tool_package(package) {
        vec![
            package_hint,
            format!(
                "when using the NixOS module, keep pkgs.{package} in services.disk-nix.toolPackages or environment.systemPackages"
            ),
        ]
    } else {
        vec![package_hint]
    }
}

fn nix_package_for_tool(tool: &str) -> Option<&'static str> {
    match tool {
        "bcache" | "make-bcache" => Some("bcache-tools"),
        "bcachefs" | "mkfs.bcachefs" => Some("bcachefs-tools"),
        "blkid" | "blockdev" | "fallocate" | "findmnt" | "fstrim" | "losetup" | "lsblk"
        | "mkfs" | "mkswap" | "mount" | "partprobe" | "swaplabel" | "swapoff" | "swapon"
        | "umount" | "wipefs" | "zramctl" => Some("util-linux"),
        "cat" | "du" | "mv" | "stat" | "test" | "truncate" => Some("coreutils"),
        "growpart" => Some("cloud-utils"),
        "sh" => Some("bash"),
        "btrfs" | "btrfstune" | "mkfs.btrfs" => Some("btrfs-progs"),
        "cryptsetup" => Some("cryptsetup"),
        "dmsetup" | "fsadm" | "lvchange" | "lvconvert" | "lvcreate" | "lvextend" | "lvreduce"
        | "lvremove" | "lvrename" | "lvs" | "pvcreate" | "pvremove" | "pvresize" | "pvscan"
        | "pvmove" | "pvs" | "vgchange" | "vgcreate" | "vgexport" | "vgextend" | "vgimport"
        | "vgremove" | "vgrename" | "vgreduce" | "vgs" | "vgscan" => Some("lvm2"),
        "dumpe2fs" | "e2fsck" | "e2label" | "mkfs.ext2" | "mkfs.ext3" | "mkfs.ext4"
        | "resize2fs" | "tune2fs" => Some("e2fsprogs"),
        "exfatlabel" | "fsck.exfat" | "mkfs.exfat" => Some("exfatprogs"),
        "f2fslabel" | "fsck.f2fs" | "mkfs.f2fs" | "resize.f2fs" => Some("f2fs-tools"),
        "fatlabel" | "fsck.fat" | "mkfs.fat" | "mkfs.vfat" => Some("dosfstools"),
        "exportfs" | "mount.nfs" | "mount.nfs4" | "nfsstat" | "showmount" => Some("nfs-utils"),
        "iscsiadm" => Some("openiscsi"),
        "lsscsi" => Some("lsscsi"),
        "mdadm" => Some("mdadm"),
        "multipath" | "multipathd" => Some("multipath-tools"),
        "mkfs.ntfs" | "ntfsfix" | "ntfsinfo" | "ntfslabel" => Some("ntfs3g"),
        "nvme" => Some("nvme-cli"),
        "parted" => Some("parted"),
        "smartctl" => Some("smartmontools"),
        "targetcli" => Some("targetcli-fb"),
        "tgtadm" => Some("tgt"),
        "udevadm" => Some("systemd"),
        "vdo" | "vdostats" => Some("vdo"),
        "mkfs.xfs" | "xfs_admin" | "xfs_growfs" | "xfs_info" | "xfs_repair" => Some("xfsprogs"),
        "zfs" | "zpool" => Some("zfs"),
        _ => None,
    }
}

fn disk_nix_default_tool_package(package: &str) -> bool {
    matches!(
        package,
        "bash"
            | "bcache-tools"
            | "bcachefs-tools"
            | "btrfs-progs"
            | "cloud-utils"
            | "coreutils"
            | "cryptsetup"
            | "dosfstools"
            | "e2fsprogs"
            | "exfatprogs"
            | "f2fs-tools"
            | "lvm2"
            | "lsscsi"
            | "mdadm"
            | "multipath-tools"
            | "nfs-utils"
            | "ntfs3g"
            | "nvme-cli"
            | "openiscsi"
            | "parted"
            | "smartmontools"
            | "targetcli-fb"
            | "tgt"
            | "util-linux"
            | "vdo"
            | "xfsprogs"
            | "zfs"
    )
}

fn register_tool_requirement(
    requirements: &mut BTreeMap<String, ToolRequirement>,
    phase: ExecutionPhase,
    command: &ExecutionCommand,
) {
    let Some(tool) = command.argv.first().filter(|tool| !tool.starts_with('<')) else {
        return;
    };
    let requirement = requirements
        .entry(tool.clone())
        .or_insert_with(|| ToolRequirement {
            tool: tool.clone(),
            command_count: 0,
            mutating_count: 0,
            verification_count: 0,
            phases: Vec::new(),
            availability: ToolAvailability::Missing,
            message: String::new(),
            remediation: Vec::new(),
        });
    requirement.command_count += 1;
    if command.mutates {
        requirement.mutating_count += 1;
    }
    if phase == ExecutionPhase::Verification {
        requirement.verification_count += 1;
    }
    if !requirement.phases.contains(&phase) {
        requirement.phases.push(phase);
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
    if let Some(rollback_value) = action.context.rollback_value.as_deref() {
        notes.push(format!("rollback-value: {rollback_value}"));
    }
    if let Some(rollback_options) = action.context.rollback_options.as_deref() {
        notes.push(format!("rollback-options: {rollback_options}"));
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
            "# Topology comparison: {} matched, {} missing, {} size diagnostics, {} type conflicts, {} already satisfied, {} suppressed, {} graph dependency conflicts.\n\n",
            comparison.summary.matched_count,
            comparison.summary.missing_count,
            comparison.summary.size_diagnostic_count,
            comparison.summary.type_conflict_count,
            comparison.summary.already_satisfied_count,
            comparison.summary.suppressed_action_count,
            comparison.summary.graph_dependency_conflict_count
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
    if !command.provider_capabilities.is_empty() {
        script.push_str("# Provider capabilities: ");
        script.push_str(&command.provider_capabilities.join(", "));
        script.push('\n');
    }
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
