#[must_use]
pub fn prepare_execution(plan: &Plan, policy: ApplyPolicy, mode: ExecutionMode) -> ExecutionReport {
    prepare_execution_with_runner_and_tool_checker(plan, policy, mode, run_command, command_exists)
}

#[cfg(test)]
fn prepare_execution_with_runner(
    plan: &Plan,
    policy: ApplyPolicy,
    mode: ExecutionMode,
    mut runner: impl FnMut(&[String]) -> CommandRunResult,
) -> ExecutionReport {
    prepare_execution_with_runner_and_tool_checker(plan, policy, mode, &mut runner, |_| true)
}

fn prepare_execution_with_runner_and_tool_checker(
    plan: &Plan,
    policy: ApplyPolicy,
    mode: ExecutionMode,
    mut runner: impl FnMut(&[String]) -> CommandRunResult,
    tool_exists: impl Fn(&str) -> bool,
) -> ExecutionReport {
    let apply = evaluate_apply_policy(plan, policy);
    let topology_comparison = plan.topology_comparison.clone();
    let command_plan = command_plan(plan, &apply);
    let command_summary = summarize_command_plan(&command_plan);
    let verification_plan = verification_plan(plan, &apply);
    let verification_summary = summarize_verification_plan(&verification_plan);
    let tool_requirements =
        summarize_tool_requirements(&command_plan, &verification_plan, tool_exists);
    let partially_suppressed_group_count =
        partially_suppressed_reconciliation_group_count(topology_comparison.as_ref());
    if !apply.can_execute() {
        let blocked_count = apply.blocked_count;
        return attach_recovery_actions(ExecutionReport {
            apply,
            status: ExecutionStatus::Blocked,
            topology_comparison,
            command_summary,
            tool_requirements,
            command_plan,
            verification_summary,
            verification_plan,
            execution_results: Vec::new(),
            partial_execution_recovery: None,
            recovery_actions: Vec::new(),
            rollback_recipes: Vec::new(),
            messages: vec![format!("apply policy blocked {blocked_count} action(s)")],
        });
    }

    match mode {
        ExecutionMode::DryRun => {
            let mut messages = vec![format!(
                "dry run only: generated {} command plan step(s) and {} verification step(s), no storage commands were run",
                command_plan.len(),
                verification_plan.len()
            )];
            if partially_suppressed_group_count > 0 {
                messages.push(format!(
                    "dry run requires reconciliation review before execution: {partially_suppressed_group_count} partially suppressed reconciliation group(s) need fresh-topology review or plan splitting"
                ));
            }
            attach_recovery_actions(ExecutionReport {
                apply,
                status: ExecutionStatus::DryRun,
                topology_comparison,
                command_summary,
                tool_requirements,
                verification_summary,
                messages,
                command_plan,
                verification_plan,
                execution_results: Vec::new(),
                partial_execution_recovery: None,
                recovery_actions: Vec::new(),
                rollback_recipes: Vec::new(),
            })
        }
        ExecutionMode::Execute => {
            if !command_summary.all_commands_ready() {
                return attach_recovery_actions(ExecutionReport {
                    apply,
                    status: ExecutionStatus::NotReady,
                    topology_comparison,
                    command_summary,
                    tool_requirements,
                    command_plan,
                    verification_summary,
                    verification_plan,
                    execution_results: Vec::new(),
                    partial_execution_recovery: None,
                    recovery_actions: Vec::new(),
                    rollback_recipes: Vec::new(),
                    messages: vec![
                        "execute refused: every planned command must be ready before mutating storage"
                            .to_string(),
                    ],
                });
            }
            let graph_dependency_conflict_count =
                graph_dependency_conflict_count(topology_comparison.as_ref());
            if graph_dependency_conflict_count > 0 {
                return attach_recovery_actions(ExecutionReport {
                    apply,
                    status: ExecutionStatus::NotReady,
                    topology_comparison,
                    command_summary,
                    tool_requirements,
                    command_plan,
                    verification_summary,
                    verification_plan,
                    execution_results: Vec::new(),
                    partial_execution_recovery: None,
                    recovery_actions: Vec::new(),
                    rollback_recipes: Vec::new(),
                    messages: vec![format!(
                        "execute refused: current topology comparison reported {graph_dependency_conflict_count} graph dependency conflict(s); split the plan or review ordering before mutating storage"
                    )],
                });
            }
            if partially_suppressed_group_count > 0 {
                return attach_recovery_actions(ExecutionReport {
                    apply,
                    status: ExecutionStatus::NotReady,
                    topology_comparison,
                    command_summary,
                    tool_requirements,
                    command_plan,
                    verification_summary,
                    verification_plan,
                    execution_results: Vec::new(),
                    partial_execution_recovery: None,
                    recovery_actions: Vec::new(),
                    rollback_recipes: Vec::new(),
                    messages: vec![format!(
                        "execute refused: current topology comparison reported {partially_suppressed_group_count} partially suppressed reconciliation group(s); re-plan against fresh topology or split the grouped mutation before mutating storage"
                    )],
                });
            }
            if let Some(missing_tools_message) = missing_tools_message(&tool_requirements) {
                return attach_recovery_actions(ExecutionReport {
                    apply,
                    status: ExecutionStatus::NotReady,
                    topology_comparison,
                    command_summary,
                    tool_requirements,
                    command_plan,
                    verification_summary,
                    verification_plan,
                    execution_results: Vec::new(),
                    partial_execution_recovery: None,
                    recovery_actions: Vec::new(),
                    rollback_recipes: Vec::new(),
                    messages: vec![format!(
                        "execute refused: required tool(s) are not available: {missing_tools_message}"
                    )],
                });
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
            let partial_execution_recovery =
                partial_execution_recovery_for_results(status, &command_plan, &execution_results);

            attach_recovery_actions(ExecutionReport {
                apply,
                status,
                topology_comparison,
                command_summary,
                tool_requirements,
                command_plan,
                verification_summary,
                verification_plan,
                execution_results,
                partial_execution_recovery,
                recovery_actions: Vec::new(),
                rollback_recipes: Vec::new(),
                messages,
            })
        }
    }
}

fn missing_tools_message(tool_requirements: &[ToolRequirement]) -> Option<String> {
    let missing = tool_requirements
        .iter()
        .filter(|requirement| requirement.availability == ToolAvailability::Missing)
        .map(|requirement| requirement.tool.as_str())
        .collect::<Vec<_>>();
    (!missing.is_empty()).then(|| missing.join(", "))
}

fn graph_dependency_conflict_count(comparison: Option<&TopologyComparison>) -> usize {
    comparison.map_or(0, |comparison| {
        comparison.summary.graph_dependency_conflict_count
    })
}

fn partially_suppressed_reconciliation_group_count(
    comparison: Option<&TopologyComparison>,
) -> usize {
    comparison.map_or(0, |comparison| {
        let summary_count = comparison.summary.partially_suppressed_group_count;
        if summary_count > 0 {
            summary_count
        } else {
            comparison
                .reconciliation_groups
                .iter()
                .filter(|group| group.partially_suppressed)
                .count()
        }
    })
}

fn partial_execution_recovery_for_results(
    status: ExecutionStatus,
    command_plan: &[ExecutionStep],
    execution_results: &[ExecutionCommandResult],
) -> Option<PartialExecutionRecovery> {
    if status != ExecutionStatus::Failed {
        return None;
    }
    let failed = execution_results.iter().find(|result| !result.success)?;
    let failed_index = command_plan
        .iter()
        .position(|step| step.action_id == failed.action_id);
    let completed_action_ids = command_plan
        .iter()
        .take(failed_index.unwrap_or(command_plan.len()))
        .filter(|step| {
            step.commands.iter().all(|command| {
                execution_results.iter().any(|result| {
                    result.phase == ExecutionPhase::Command
                        && result.action_id == step.action_id
                        && result.argv == command.argv
                        && result.success
                })
            })
        })
        .map(|step| step.action_id.clone())
        .collect::<Vec<_>>();
    let retry_review_action_ids = failed_index
        .map(|index| {
            command_plan
                .iter()
                .skip(index)
                .map(|step| step.action_id.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec![failed.action_id.clone()]);
    let remaining_action_ids = failed_index
        .map(|index| {
            command_plan
                .iter()
                .skip(index + 1)
                .map(|step| step.action_id.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let completed_mutating_command_count = execution_results
        .iter()
        .take_while(|result| result.action_id != failed.action_id || result.argv != failed.argv)
        .filter(|result| {
            result.success
                && result.phase == ExecutionPhase::Command
                && command_plan_command_by_result(command_plan, result)
                    .is_some_and(|command| command.mutates)
        })
        .count();

    Some(PartialExecutionRecovery {
        completed_action_ids,
        failed_action_id: failed.action_id.clone(),
        failed_phase: failed.phase,
        failed_command: failed.argv.clone(),
        retry_review_action_ids,
        remaining_action_ids,
        completed_mutating_command_count,
        notes: vec![
            "treat completed actions as changed until fresh topology proves otherwise".to_string(),
            "review the failed action and remaining actions against current topology before resuming"
                .to_string(),
            "remove, suppress, or split already-satisfied actions before retrying mutating commands"
                .to_string(),
        ],
    })
}

fn attach_recovery_actions(mut report: ExecutionReport) -> ExecutionReport {
    report.recovery_actions = recovery_actions_for_report(&report);
    report.rollback_recipes = rollback_recipes_for_report(&report);
    report
}
