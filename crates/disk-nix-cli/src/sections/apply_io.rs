fn prepare_apply_report(
    spec: &str,
    probe_current: bool,
    mode: ExecutionMode,
) -> Result<ExecutionReport, AppError> {
    let bytes = std::fs::read(spec)?;
    let (mut plan, mut policy) = plan_and_policy_from_json_bytes(&bytes)
        .map_err(|error| AppError::Message(format!("failed to parse {spec}: {error}")))?;
    if probe_current {
        plan = compare_plan_with_topology(plan, &collect_graph()?);
    }
    apply_confirmation_file(&mut policy)?;
    Ok(prepare_execution(&plan, policy, mode))
}

fn apply_confirmation_file(policy: &mut ApplyPolicy) -> Result<(), AppError> {
    let Some(path) = policy.require_confirmation_file.as_deref() else {
        return Ok(());
    };

    match std::fs::read_to_string(path) {
        Ok(content) if confirmation_file_accepts(&content) => {
            policy.confirmation = true;
            Ok(())
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::Io(error)),
    }
}

fn confirmation_file_accepts(content: &str) -> bool {
    content
        .lines()
        .any(|line| line.trim() == "disk-nix confirm")
}

fn write_execution_script(path: &str, report: &ExecutionReport) -> Result<(), AppError> {
    let script = report
        .to_shell_script()
        .ok_or_else(|| AppError::Message(script_refusal_message(report)))?;
    std::fs::write(path, script)?;
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

fn script_refusal_message(report: &ExecutionReport) -> String {
    let graph_dependency_conflict_count =
        report.topology_comparison.as_ref().map_or(0, |comparison| {
            comparison.summary.graph_dependency_conflict_count
        });
    let partially_suppressed_group_count =
        report.topology_comparison.as_ref().map_or(0, |comparison| {
            comparison.summary.partially_suppressed_group_count
        });
    let mut reasons = Vec::new();
    if !report.apply.can_execute() {
        reasons.push(format!(
            "apply policy blocks {} action(s)",
            report.apply.blocked_count
        ));
    }
    if graph_dependency_conflict_count > 0 {
        reasons.push(format!(
            "{graph_dependency_conflict_count} graph dependency conflict(s) require plan splitting or ordering review"
        ));
    }
    if partially_suppressed_group_count > 0 {
        reasons.push(format!(
            "{partially_suppressed_group_count} partially suppressed reconciliation group(s) require fresh-topology review or plan splitting"
        ));
    }
    if !report.command_summary.all_commands_ready() {
        reasons.push(format!(
            "{} command(s) need desired size, {} need domain command implementation, {} are manual-only",
            report.command_summary.needs_desired_size_count,
            report.command_summary.needs_domain_implementation_count,
            report.command_summary.manual_only_count
        ));
    }
    if reasons.is_empty() {
        reasons.push("report is not in a scriptable dry-run state".to_string());
    }
    format!(
        "script generation requires a policy-allowed, conflict-free command plan: {}",
        reasons.join("; ")
    )
}

fn write_execution_report(path: &str, report: &ExecutionReport) -> Result<(), AppError> {
    let mut report_json = report
        .to_json()
        .map_err(|error| AppError::Message(error.to_string()))?;
    report_json.push('\n');
    std::fs::write(path, report_json)?;
    Ok(())
}

fn apply_receipt(
    command: &str,
    spec_path: &str,
    probe_current: bool,
    execute_requested: bool,
    generated_at_unix_seconds: u64,
    report: &ExecutionReport,
) -> ApplyReceipt {
    ApplyReceipt {
        receipt_version: 1,
        command: command.to_string(),
        spec_path: spec_path.to_string(),
        probe_current,
        execute_requested,
        generated_at_unix_seconds,
        report: report.clone(),
    }
}

fn write_apply_receipt(path: &str, receipt: ApplyReceipt) -> Result<(), AppError> {
    let mut receipt_json = serde_json::to_string_pretty(&receipt)
        .map_err(|error| AppError::Message(error.to_string()))?;
    receipt_json.push('\n');
    std::fs::write(path, receipt_json)?;
    Ok(())
}

fn current_unix_seconds() -> Result<u64, AppError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| {
            AppError::Message(format!("system clock is before the Unix epoch: {error}"))
        })
}
