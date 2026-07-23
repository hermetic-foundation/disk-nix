use std::{
    collections::{BTreeMap, BTreeSet},
    process::Command,
};

use disk_nix_plan::{
    evaluate_apply_policy, ApplyPolicy, ApplyReport, Operation, Plan, PlannedAction, RiskClass,
    TopologyComparison, TopologyDiagnosticKind,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_requirements: Vec<ToolRequirement>,
    pub command_plan: Vec<ExecutionStep>,
    pub verification_summary: VerificationPlanSummary,
    pub verification_plan: Vec<VerificationStep>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub execution_results: Vec<ExecutionCommandResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_execution_recovery: Option<PartialExecutionRecovery>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recovery_actions: Vec<RecoveryAction>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rollback_recipes: Vec<RollbackRecipe>,
    pub messages: Vec<String>,
}

impl ExecutionReport {
    #[must_use]
    pub fn can_apply(&self) -> bool {
        self.status == ExecutionStatus::DryRun
            && self.apply.can_execute()
            && graph_dependency_conflict_count(self.topology_comparison.as_ref()) == 0
            && partially_suppressed_reconciliation_group_count(self.topology_comparison.as_ref())
                == 0
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    #[must_use]
    pub fn to_shell_script(&self) -> Option<String> {
        (self.apply.can_execute()
            && graph_dependency_conflict_count(self.topology_comparison.as_ref()) == 0
            && partially_suppressed_reconciliation_group_count(self.topology_comparison.as_ref())
                == 0)
            .then(|| render_shell_script(self))
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provider_capabilities: Vec<String>,
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartialExecutionRecovery {
    pub completed_action_ids: Vec<String>,
    pub failed_action_id: String,
    pub failed_phase: ExecutionPhase,
    pub failed_command: Vec<String>,
    pub retry_review_action_ids: Vec<String>,
    pub remaining_action_ids: Vec<String>,
    pub completed_mutating_command_count: usize,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolRequirement {
    pub tool: String,
    pub command_count: usize,
    pub mutating_count: usize,
    pub verification_count: usize,
    pub phases: Vec<ExecutionPhase>,
    pub availability: ToolAvailability,
    pub message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remediation: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolAvailability {
    Available,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecoveryActionKind {
    ReviewPolicy,
    ResolveInputs,
    InspectCurrentState,
    ReviewExecutionFailure,
    DomainRecovery,
    RollForwardReview,
    RollbackReview,
    RunVerification,
    ResumeAfterFix,
    PreserveRecoveryPoints,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryAction {
    pub kind: RecoveryActionKind,
    pub summary: String,
    pub commands: Vec<ExecutionCommand>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RollbackRecipeStatus {
    ReviewOnly,
    ProvenSafe,
    Refused,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackRecipeSection {
    pub commands: Vec<ExecutionCommand>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackRecipe {
    pub recipe_version: u64,
    pub source_action_id: String,
    pub failed_command: Vec<String>,
    pub status: RollbackRecipeStatus,
    pub receipt_binding_required: bool,
    pub fresh_topology_probe_required: bool,
    pub read_only_validation: RollbackRecipeSection,
    pub reversible_mutations: RollbackRecipeSection,
    pub destructive_mutations: RollbackRecipeSection,
    pub operator_only_handoff: RollbackRecipeSection,
    pub safety_gates: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_topology_evidence: Vec<String>,
    pub refusal_reasons: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RollbackExecutionStatus {
    Succeeded,
    Failed,
    Refused,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackReceiptBinding {
    pub original_receipt_id: String,
    pub source_action_id: String,
    pub failed_command: Vec<String>,
    pub fresh_topology_probe_id: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub topology_evidence: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub topology_payloads: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackExecutionReport {
    pub status: RollbackExecutionStatus,
    pub recipe_version: u64,
    pub source_action_id: String,
    pub receipt_binding: RollbackReceiptBinding,
    pub validation_results: Vec<ExecutionCommandResult>,
    pub rollback_results: Vec<ExecutionCommandResult>,
    pub messages: Vec<String>,
    pub refusal_reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommandRunResult {
    success: bool,
    status_code: Option<i32>,
    stdout: String,
    stderr: String,
}

#[derive(Debug, Clone)]
struct RollbackReplayBindings {
    original_receipt_id: String,
    fresh_topology_probe_id: String,
    topology_evidence: BTreeMap<String, String>,
    topology_payloads: BTreeMap<String, serde_json::Value>,
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

fn rollback_recipes_for_report(report: &ExecutionReport) -> Vec<RollbackRecipe> {
    if report.status != ExecutionStatus::Failed {
        return Vec::new();
    }
    let Some(partial) = report.partial_execution_recovery.as_ref() else {
        return Vec::new();
    };
    let Some(rollback_review) = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
    else {
        return Vec::new();
    };

    if let Some(step) = report
        .command_plan
        .iter()
        .find(|step| step.action_id == partial.failed_action_id)
    {
        if let Some(recipe) = filesystem_rollback_recipe_for_step(partial, rollback_review, step) {
            return vec![recipe];
        }
        if let Some(recipe) = block_stack_rollback_recipe_for_step(partial, rollback_review, step) {
            return vec![recipe];
        }
        if let Some(recipe) =
            advanced_storage_rollback_recipe_for_step(partial, rollback_review, step)
        {
            return vec![recipe];
        }
        if let Some(recipe) =
            network_storage_rollback_recipe_for_step(partial, rollback_review, step)
        {
            return vec![recipe];
        }
    }

    vec![review_only_rollback_recipe(
        partial,
        rollback_review,
        vec![
            "automatic replay refused because this recipe is review-only".to_string(),
            "domain-specific rollback mutation is not proven safe".to_string(),
            "receipt-bound pre-rollback topology comparison has not been evaluated".to_string(),
        ],
        vec![
            "this stable recipe schema separates validation from reversible, destructive, and operator-only rollback sections".to_string(),
            "review-only recipes are evidence carriers for operators and future automation; they are not executable rollback approval".to_string(),
        ],
    )]
}

fn review_only_rollback_recipe(
    partial: &PartialExecutionRecovery,
    rollback_review: &RecoveryAction,
    refusal_reasons: Vec<String>,
    notes: Vec<String>,
) -> RollbackRecipe {
    RollbackRecipe {
        recipe_version: 1,
        source_action_id: partial.failed_action_id.clone(),
        failed_command: partial.failed_command.clone(),
        status: RollbackRecipeStatus::ReviewOnly,
        receipt_binding_required: true,
        fresh_topology_probe_required: true,
        read_only_validation: RollbackRecipeSection {
            commands: rollback_review.commands.clone(),
            notes: vec![
                "all commands in this section must be read-only validation commands".to_string(),
                "compare read-only validation output with the original receipt, failed apply report, and a fresh topology probe".to_string(),
            ],
        },
        reversible_mutations: RollbackRecipeSection {
            commands: Vec::new(),
            notes: vec![
                "no reversible rollback mutation is proven by this schema-only recipe".to_string(),
                "a future rollback engine may populate this section only after domain safety gates prove idempotency and data preservation".to_string(),
            ],
        },
        destructive_mutations: RollbackRecipeSection {
            commands: Vec::new(),
            notes: vec![
                "destructive rollback mutation steps are intentionally empty until a domain recipe proves the operation safe".to_string(),
                "commands that can discard data must remain refused or operator-only without explicit receipt binding and fresh topology evidence".to_string(),
            ],
        },
        operator_only_handoff: RollbackRecipeSection {
            commands: Vec::new(),
            notes: rollback_review.notes.clone(),
        },
        safety_gates: rollback_recipe_safety_gates(),
        required_topology_evidence: vec![
            "expected".to_string(),
            "preApply".to_string(),
            "failedApply".to_string(),
            "current".to_string(),
        ],
        refusal_reasons,
        notes,
    }
}

fn filesystem_rollback_recipe_for_step(
    partial: &PartialExecutionRecovery,
    rollback_review: &RecoveryAction,
    step: &ExecutionStep,
) -> Option<RollbackRecipe> {
    if command_step_collection(step) != Some("filesystems") {
        return None;
    }

    let validation_commands = rollback_review.commands.clone();
    let mut recipe = review_only_rollback_recipe(
        partial,
        rollback_review,
        filesystem_rollback_refusal_reasons(step),
        filesystem_rollback_notes(step),
    );

    if let Some(command) = filesystem_proven_rollback_command(partial, step) {
        recipe.status = RollbackRecipeStatus::ProvenSafe;
        recipe.read_only_validation.notes.push(
            "filesystem rollback validation must prove the target, source, and consumers still match the failed apply receipt".to_string(),
        );
        recipe.reversible_mutations = RollbackRecipeSection {
            commands: vec![command],
            notes: vec![
                "filesystem rollback mutation is limited to a declared old property value, declared old remount options, or undoing a mount whose verification failed".to_string(),
                "grow, scrub, repair, and failed-check boundaries remain refused because they do not have a generic data-preserving inverse".to_string(),
            ],
        };
        recipe.operator_only_handoff = RollbackRecipeSection {
            commands: Vec::new(),
            notes: Vec::new(),
        };
        recipe.refusal_reasons = Vec::new();
        recipe.notes.push(
            "filesystem recipe is proven-safe only because the rollback command is derived from explicit pre-apply metadata or a failed verification boundary".to_string(),
        );
    } else if matches!(
        step.operation,
        Operation::Grow | Operation::Repair | Operation::Scrub | Operation::Check
    ) {
        recipe.status = RollbackRecipeStatus::Refused;
        recipe.operator_only_handoff = RollbackRecipeSection {
            commands: validation_commands,
            notes: vec![
                "filesystem grow, scrub, repair, and failed-check rollback requires operator review of data-preserving state".to_string(),
                "prefer roll-forward validation, fresh topology inspection, backup/snapshot restore, or cloned-device repair instead of automated mutation".to_string(),
            ],
        };
    }

    Some(recipe)
}

fn filesystem_rollback_refusal_reasons(step: &ExecutionStep) -> Vec<String> {
    match step.operation {
        Operation::Grow => vec![
            "filesystem grow rollback is refused because generic filesystem shrink is not data-preserving".to_string(),
        ],
        Operation::Repair => vec![
            "filesystem repair rollback is refused because repair tools can rewrite metadata without a generic inverse".to_string(),
        ],
        Operation::Scrub => vec![
            "filesystem scrub rollback is refused because scrub has no rollback mutation; review health and roll forward".to_string(),
        ],
        Operation::Check => vec![
            "filesystem failed-check rollback is refused because read-only check failure requires diagnosis or repair, not mutation replay".to_string(),
        ],
        Operation::Mount | Operation::Remount | Operation::SetProperty => vec![
            "filesystem rollback metadata is missing or insufficient for proven-safe replay".to_string(),
        ],
        _ => vec!["filesystem rollback for this operation remains review-only".to_string()],
    }
}

fn filesystem_rollback_notes(step: &ExecutionStep) -> Vec<String> {
    let boundary = match step.operation {
        Operation::Grow => "grow",
        Operation::Mount => "mount",
        Operation::Remount => "mount/remount",
        Operation::SetProperty => "property mutation",
        Operation::Scrub => "scrub",
        Operation::Repair => "repair",
        Operation::Check => "failed-check",
        _ => "filesystem",
    };
    vec![
        format!("filesystem-level rollback recipe covers the {boundary} boundary"),
        "read-only validation must be bound to expected, pre-apply, failed-apply, and current topology evidence".to_string(),
    ]
}

fn filesystem_proven_rollback_command(
    partial: &PartialExecutionRecovery,
    step: &ExecutionStep,
) -> Option<ExecutionCommand> {
    match step.operation {
        Operation::Remount => filesystem_remount_rollback_command(step),
        Operation::Mount if partial.failed_phase == ExecutionPhase::Verification => {
            filesystem_mount_verification_rollback_command(step)
        }
        Operation::SetProperty => filesystem_property_rollback_command(step),
        _ => None,
    }
}

fn filesystem_remount_rollback_command(step: &ExecutionStep) -> Option<ExecutionCommand> {
    let target = filesystem_target_from_step(step)?;
    let rollback_options = step_note_value(step, "rollback-options")?;
    Some(command_vec(
        vec![
            "mount".to_string(),
            "-o".to_string(),
            format!("remount,{rollback_options}"),
            target.to_string(),
        ],
        true,
        "restore declared pre-apply filesystem mount options",
    ))
}

fn filesystem_mount_verification_rollback_command(
    step: &ExecutionStep,
) -> Option<ExecutionCommand> {
    let target = filesystem_target_from_step(step)?;
    Some(command_vec(
        vec!["umount".to_string(), target.to_string()],
        true,
        "undo the mount created by the failed apply after read-only validation",
    ))
}

fn filesystem_property_rollback_command(step: &ExecutionStep) -> Option<ExecutionCommand> {
    let rollback_value = step_note_value(step, "rollback-value")?;
    let rollback_command = step.commands.iter().find(|command| command.mutates)?;
    let tool = rollback_command.argv.first()?.as_str();
    let argv = match tool {
        "fatlabel" | "exfatlabel"
            if rollback_command.argv.get(1).is_some_and(|arg| arg == "-i") =>
        {
            vec![
                tool.to_string(),
                "-i".to_string(),
                rollback_command.argv.get(2)?.clone(),
                rollback_value.to_string(),
            ]
        }
        "e2label" | "fatlabel" | "ntfslabel" | "exfatlabel" | "f2fslabel"
            if rollback_command.argv.len() >= 3 =>
        {
            vec![
                tool.to_string(),
                rollback_command.argv[1].clone(),
                rollback_value.to_string(),
            ]
        }
        "tune2fs" if rollback_command.argv.get(1).is_some_and(|arg| arg == "-U") => {
            vec![
                "tune2fs".to_string(),
                "-U".to_string(),
                rollback_value.to_string(),
                rollback_command.argv.get(3)?.clone(),
            ]
        }
        "xfs_admin"
            if rollback_command
                .argv
                .get(1)
                .is_some_and(|arg| arg == "-L" || arg == "-U") =>
        {
            vec![
                "xfs_admin".to_string(),
                rollback_command.argv[1].clone(),
                rollback_value.to_string(),
                rollback_command.argv.get(3)?.clone(),
            ]
        }
        "btrfs"
            if rollback_command
                .argv
                .get(1..3)
                .is_some_and(|args| args == ["filesystem", "label"]) =>
        {
            vec![
                "btrfs".to_string(),
                "filesystem".to_string(),
                "label".to_string(),
                rollback_command.argv.get(3)?.clone(),
                rollback_value.to_string(),
            ]
        }
        "btrfstune" if rollback_command.argv.get(1).is_some_and(|arg| arg == "-U") => {
            vec![
                "btrfstune".to_string(),
                "-U".to_string(),
                rollback_value.to_string(),
                rollback_command.argv.get(3)?.clone(),
            ]
        }
        _ => return None,
    };

    Some(command_vec(
        argv,
        true,
        "restore declared pre-apply filesystem property value",
    ))
}

fn block_stack_rollback_recipe_for_step(
    partial: &PartialExecutionRecovery,
    rollback_review: &RecoveryAction,
    step: &ExecutionStep,
) -> Option<RollbackRecipe> {
    if !block_stack_collection(command_step_collection(step)?) {
        return None;
    }

    let validation_commands = rollback_review.commands.clone();
    let mut recipe = review_only_rollback_recipe(
        partial,
        rollback_review,
        block_stack_rollback_refusal_reasons(step),
        block_stack_rollback_notes(step),
    );

    if let Some(command) = block_stack_proven_rollback_command(partial, step) {
        recipe.status = RollbackRecipeStatus::ProvenSafe;
        recipe.read_only_validation.notes.push(
            "block-stack rollback validation must prove stable target identity, old metadata, and active consumer state still match the failed apply receipt".to_string(),
        );
        recipe.reversible_mutations = RollbackRecipeSection {
            commands: vec![command],
            notes: vec![
                "block-stack rollback mutation is limited to a declared old metadata value, a verification-bound rename/open/loop attach inverse, or swap reactivation".to_string(),
                "partition, LVM growth, MD RAID repair, backing-file growth, formatting, creation, destruction, key, token, and replacement boundaries remain refused without stronger domain proof".to_string(),
            ],
        };
        recipe.operator_only_handoff = RollbackRecipeSection {
            commands: Vec::new(),
            notes: Vec::new(),
        };
        recipe.refusal_reasons = Vec::new();
        recipe.notes.push(
            "block-stack recipe is proven-safe only because the rollback command is derived from explicit pre-apply metadata or a failed verification boundary".to_string(),
        );
    } else if block_stack_refused_operation(step) {
        recipe.status = RollbackRecipeStatus::Refused;
        recipe.operator_only_handoff = RollbackRecipeSection {
            commands: validation_commands,
            notes: vec![
                "block-stack rollback requires operator review of identity, active consumers, redundancy, and data placement before mutation".to_string(),
                "prefer roll-forward validation, fresh topology inspection, backup/header restore, array repair, replacement capacity, or cloned-device recovery instead of automated mutation".to_string(),
            ],
        };
    }

    Some(recipe)
}

fn block_stack_collection(collection: &str) -> bool {
    matches!(
        collection,
        "disks"
            | "partitions"
            | "luks.devices"
            | "luksKeyslots"
            | "luksTokens"
            | "physicalVolumes"
            | "volumeGroups"
            | "volumes"
            | "thinPools"
            | "lvmSnapshots"
            | "mdRaids"
            | "dmMaps"
            | "loopDevices"
            | "backingFiles"
            | "swaps"
            | "zram"
    )
}

fn block_stack_refused_operation(step: &ExecutionStep) -> bool {
    matches!(
        step.operation,
        Operation::AddDevice
            | Operation::AddKey
            | Operation::Assemble
            | Operation::Attach
            | Operation::Close
            | Operation::Create
            | Operation::Deactivate
            | Operation::Destroy
            | Operation::Detach
            | Operation::Export
            | Operation::Format
            | Operation::Grow
            | Operation::Import
            | Operation::ImportToken
            | Operation::RemoveDevice
            | Operation::RemoveKey
            | Operation::RemoveToken
            | Operation::ReplaceDevice
            | Operation::Rollback
            | Operation::SetProperty
            | Operation::Stop
    )
}

fn block_stack_rollback_refusal_reasons(step: &ExecutionStep) -> Vec<String> {
    let collection = command_step_collection(step).unwrap_or("block-stack");
    match (collection, step.operation) {
        ("swaps", Operation::SetProperty) | ("luks.devices", Operation::SetProperty) => vec![
            "block-stack property rollback metadata is missing or insufficient for proven-safe replay".to_string(),
        ],
        ("dmMaps", Operation::Rename) => vec![
            "device-mapper rename rollback requires the verification-failed new name and declared previous name".to_string(),
        ],
        ("luks.devices", Operation::Open) => vec![
            "LUKS open rollback is only automatic when the open command succeeded and verification failed".to_string(),
        ],
        ("loopDevices", Operation::Create) => vec![
            "loop attach rollback is only automatic when the attach command succeeded and verification failed".to_string(),
        ],
        ("swaps", Operation::Deactivate) => vec![
            "swap deactivation rollback is only automatic when swapoff succeeded and verification failed".to_string(),
        ],
        ("partitions" | "disks", Operation::Create | Operation::Grow | Operation::Format) => {
            vec![
                "disk and partition rollback is refused because table and geometry changes have no generic data-preserving inverse".to_string(),
            ]
        }
        ("physicalVolumes" | "volumeGroups" | "volumes" | "thinPools" | "lvmSnapshots", _) => {
            vec![
                "LVM rollback is refused without volume metadata backups, activation state, and current consumer proof".to_string(),
            ]
        }
        ("mdRaids", _) => vec![
            "MD RAID rollback is refused without fresh array health, redundancy, and member role proof".to_string(),
        ],
        ("backingFiles", Operation::Create | Operation::Grow | Operation::Destroy) => vec![
            "backing-file rollback is refused because sparse allocation, truncation, and consumers require operator review".to_string(),
        ],
        ("zram", _) => vec![
            "zram rollback is refused because live compressed swap state is reconciled through NixOS service settings".to_string(),
        ],
        ("luks.devices" | "luksKeyslots" | "luksTokens", _) => vec![
            "LUKS rollback is refused without header backup, keyslot, token, mapper, and consumer proof".to_string(),
        ],
        ("swaps", Operation::Grow | Operation::Format | Operation::Destroy) => vec![
            "swap rollback is refused for grow, format, and signature removal because previous content and active memory pressure must be reviewed".to_string(),
        ],
        _ => vec!["block-stack rollback for this operation remains review-only".to_string()],
    }
}

fn block_stack_rollback_notes(step: &ExecutionStep) -> Vec<String> {
    let collection = command_step_collection(step).unwrap_or("block-stack");
    let boundary = match (collection, step.operation) {
        ("disks" | "partitions", _) => "disk/partition",
        ("luks.devices" | "luksKeyslots" | "luksTokens", _) => "LUKS",
        ("physicalVolumes" | "volumeGroups" | "volumes" | "thinPools" | "lvmSnapshots", _) => "LVM",
        ("mdRaids", _) => "MD RAID",
        ("dmMaps", _) => "device-mapper",
        ("loopDevices", _) => "loop-device",
        ("backingFiles", _) => "backing-file",
        ("swaps", _) => "swap",
        ("zram", _) => "zram",
        _ => "block-stack",
    };
    vec![
        format!("block-stack rollback recipe covers the {boundary} boundary"),
        "read-only validation must be bound to expected, pre-apply, failed-apply, and current topology evidence".to_string(),
    ]
}

fn block_stack_proven_rollback_command(
    partial: &PartialExecutionRecovery,
    step: &ExecutionStep,
) -> Option<ExecutionCommand> {
    let collection = command_step_collection(step)?;
    match (collection, step.operation) {
        ("swaps", Operation::SetProperty) => swap_property_rollback_command(step),
        ("luks.devices", Operation::SetProperty) => luks_property_rollback_command(step),
        ("dmMaps", Operation::Rename) if partial.failed_phase == ExecutionPhase::Verification => {
            dm_rename_verification_rollback_command(step)
        }
        ("luks.devices", Operation::Open)
            if partial.failed_phase == ExecutionPhase::Verification =>
        {
            luks_open_verification_rollback_command(step)
        }
        ("loopDevices", Operation::Create)
            if partial.failed_phase == ExecutionPhase::Verification =>
        {
            loop_attach_verification_rollback_command(step)
        }
        ("swaps", Operation::Deactivate)
            if partial.failed_phase == ExecutionPhase::Verification =>
        {
            swap_deactivate_verification_rollback_command(step)
        }
        _ => None,
    }
}

fn swap_property_rollback_command(step: &ExecutionStep) -> Option<ExecutionCommand> {
    let rollback_value = step_note_value(step, "rollback-value")?;
    let rollback_command = step.commands.iter().find(|command| command.mutates)?;
    let argv = match rollback_command.argv.as_slice() {
        [tool, flag, _, target] if tool == "swaplabel" && flag == "--label" => vec![
            "swaplabel".to_string(),
            "--label".to_string(),
            rollback_value.to_string(),
            target.clone(),
        ],
        [tool, flag, _, target] if tool == "swaplabel" && flag == "--uuid" => vec![
            "swaplabel".to_string(),
            "--uuid".to_string(),
            rollback_value.to_string(),
            target.clone(),
        ],
        _ => return None,
    };
    Some(command_vec(
        argv,
        true,
        "restore declared pre-apply swap signature metadata",
    ))
}

fn luks_property_rollback_command(step: &ExecutionStep) -> Option<ExecutionCommand> {
    let rollback_value = step_note_value(step, "rollback-value")?;
    let rollback_command = step.commands.iter().find(|command| command.mutates)?;
    let argv = match rollback_command.argv.as_slice() {
        [tool, subcommand, device, flag, _]
            if tool == "cryptsetup"
                && subcommand == "config"
                && (flag == "--label" || flag == "--subsystem") =>
        {
            vec![
                "cryptsetup".to_string(),
                "config".to_string(),
                device.clone(),
                flag.clone(),
                rollback_value.to_string(),
            ]
        }
        [tool, subcommand, device, flag, _]
            if tool == "cryptsetup" && subcommand == "luksUUID" && flag == "--uuid" =>
        {
            vec![
                "cryptsetup".to_string(),
                "luksUUID".to_string(),
                device.clone(),
                "--uuid".to_string(),
                rollback_value.to_string(),
            ]
        }
        _ => return None,
    };
    Some(command_vec(
        argv,
        true,
        "restore declared pre-apply LUKS header identity metadata",
    ))
}

fn dm_rename_verification_rollback_command(step: &ExecutionStep) -> Option<ExecutionCommand> {
    let rollback_command = step.commands.iter().find(|command| command.mutates)?;
    let [tool, subcommand, old_name, new_name] = rollback_command.argv.as_slice() else {
        return None;
    };
    if tool != "dmsetup" || subcommand != "rename" {
        return None;
    }
    let rollback_name = step_note_value(step, "rollback-value").unwrap_or(old_name);
    Some(command_vec(
        vec![
            "dmsetup".to_string(),
            "rename".to_string(),
            new_name.clone(),
            rollback_name.to_string(),
        ],
        true,
        "restore declared pre-apply device-mapper name after failed rename verification",
    ))
}

fn luks_open_verification_rollback_command(step: &ExecutionStep) -> Option<ExecutionCommand> {
    let rollback_command = step.commands.iter().find(|command| command.mutates)?;
    let [tool, subcommand, _, mapper] = rollback_command.argv.as_slice() else {
        return None;
    };
    if tool != "cryptsetup" || subcommand != "open" {
        return None;
    }
    Some(command_vec(
        vec![
            "cryptsetup".to_string(),
            "close".to_string(),
            mapper.clone(),
        ],
        true,
        "close LUKS mapper opened by the failed apply after read-only validation",
    ))
}

fn loop_attach_verification_rollback_command(step: &ExecutionStep) -> Option<ExecutionCommand> {
    let rollback_command = step.commands.iter().find(|command| command.mutates)?;
    let tool = rollback_command.argv.first()?;
    if tool != "losetup" {
        return None;
    }
    let loop_device = command_step_target(step)?;
    Some(command_vec(
        vec![
            "losetup".to_string(),
            "-d".to_string(),
            loop_device.to_string(),
        ],
        true,
        "detach loop device attached by the failed apply after read-only validation",
    ))
}

fn swap_deactivate_verification_rollback_command(step: &ExecutionStep) -> Option<ExecutionCommand> {
    let rollback_command = step.commands.iter().find(|command| command.mutates)?;
    let [tool, target] = rollback_command.argv.as_slice() else {
        return None;
    };
    if tool != "swapoff" {
        return None;
    }
    Some(command_vec(
        vec!["swapon".to_string(), target.clone()],
        true,
        "reactivate swap target disabled by the failed apply after read-only validation",
    ))
}

fn advanced_storage_rollback_recipe_for_step(
    partial: &PartialExecutionRecovery,
    rollback_review: &RecoveryAction,
    step: &ExecutionStep,
) -> Option<RollbackRecipe> {
    if !advanced_storage_collection(command_step_collection(step)?) {
        return None;
    }

    let validation_commands = rollback_review.commands.clone();
    let mut recipe = review_only_rollback_recipe(
        partial,
        rollback_review,
        advanced_storage_rollback_refusal_reasons(step),
        advanced_storage_rollback_notes(step),
    );

    if let Some(command) = advanced_storage_proven_rollback_command(partial, step) {
        recipe.status = RollbackRecipeStatus::ProvenSafe;
        recipe.read_only_validation.notes.push(
            "advanced-storage rollback validation must prove object identity, old metadata, and dependent consumers still match the failed apply receipt".to_string(),
        );
        recipe.reversible_mutations = RollbackRecipeSection {
            commands: vec![command],
            notes: vec![
                "advanced-storage rollback mutation is limited to declared old property values or verification-bound rename inverses".to_string(),
                "growth, creation, destruction, snapshot rollback, clone, promotion, cache topology, and pool membership boundaries remain refused without stronger domain proof".to_string(),
            ],
        };
        recipe.operator_only_handoff = RollbackRecipeSection {
            commands: Vec::new(),
            notes: Vec::new(),
        };
        recipe.refusal_reasons = Vec::new();
        recipe.notes.push(
            "advanced-storage recipe is proven-safe only because the rollback command is derived from explicit pre-apply metadata or a failed verification boundary".to_string(),
        );
    } else if advanced_storage_refused_operation(step) {
        recipe.status = RollbackRecipeStatus::Refused;
        recipe.operator_only_handoff = RollbackRecipeSection {
            commands: validation_commands,
            notes: vec![
                "advanced-storage rollback requires operator review of snapshots, clones, cache state, pool topology, allocation, and active consumers before mutation".to_string(),
                "prefer roll-forward validation, fresh topology inspection, retained snapshots, cloned recovery datasets, or cache/pool repair workflows instead of automated mutation".to_string(),
            ],
        };
    }

    Some(recipe)
}

fn advanced_storage_collection(collection: &str) -> bool {
    matches!(
        collection,
        "pools"
            | "datasets"
            | "zvols"
            | "snapshots"
            | "btrfsSubvolumes"
            | "btrfsQgroups"
            | "caches"
            | "lvmCaches"
            | "vdoVolumes"
    )
}

fn advanced_storage_refused_operation(step: &ExecutionStep) -> bool {
    matches!(
        step.operation,
        Operation::AddDevice
            | Operation::Attach
            | Operation::Clone
            | Operation::Create
            | Operation::Destroy
            | Operation::Detach
            | Operation::Export
            | Operation::Grow
            | Operation::Import
            | Operation::Promote
            | Operation::Rebalance
            | Operation::RemoveDevice
            | Operation::ReplaceDevice
            | Operation::Rollback
            | Operation::SetProperty
            | Operation::Start
            | Operation::Stop
    )
}

fn advanced_storage_rollback_refusal_reasons(step: &ExecutionStep) -> Vec<String> {
    let collection = command_step_collection(step).unwrap_or("advanced-storage");
    match (collection, step.operation) {
        ("pools" | "datasets" | "zvols", Operation::SetProperty) => vec![
            "ZFS property rollback metadata is missing or insufficient for proven-safe replay".to_string(),
        ],
        ("datasets" | "zvols" | "snapshots" | "btrfsSubvolumes", Operation::Rename) => vec![
            "advanced-storage rename rollback is only automatic when rename succeeded and verification failed".to_string(),
        ],
        ("caches", Operation::SetProperty) => vec![
            "bcache property rollback metadata is missing or insufficient for proven-safe replay".to_string(),
        ],
        ("vdoVolumes", Operation::SetProperty) => vec![
            "VDO property rollback metadata is missing or insufficient for proven-safe replay".to_string(),
        ],
        ("btrfsSubvolumes", Operation::SetProperty) => vec![
            "Btrfs subvolume property rollback metadata is missing or insufficient for proven-safe replay".to_string(),
        ],
        ("pools", Operation::AddDevice | Operation::RemoveDevice | Operation::Create | Operation::Destroy | Operation::Import | Operation::Export) => vec![
            "ZFS pool rollback is refused because vdev topology, import/export state, and allocation changes require operator review".to_string(),
        ],
        ("datasets" | "zvols", Operation::Create | Operation::Destroy | Operation::Grow | Operation::Promote) => vec![
            "ZFS dataset and zvol rollback is refused for create, destroy, grow, and promote boundaries without retained snapshot or clone proof".to_string(),
        ],
        ("snapshots", Operation::Create | Operation::Destroy | Operation::Clone | Operation::Rollback | Operation::SetProperty) => vec![
            "snapshot rollback is refused because recovery points, holds, clones, and newer data require operator review".to_string(),
        ],
        ("btrfsSubvolumes" | "btrfsQgroups", _) => vec![
            "Btrfs advanced rollback is refused without subvolume, qgroup, snapshot, send/receive, and mount-state proof".to_string(),
        ],
        ("caches" | "lvmCaches", _) => vec![
            "cache rollback is refused without dirty data, cache-set, origin, and active consumer proof".to_string(),
        ],
        ("vdoVolumes", Operation::Create | Operation::Destroy | Operation::Grow | Operation::Start | Operation::Stop) => vec![
            "VDO rollback is refused for lifecycle and growth boundaries because operating mode, backing capacity, and dedupe metadata require operator review".to_string(),
        ],
        _ => vec![
            "advanced-storage rollback for this operation remains review-only".to_string(),
        ],
    }
}

fn advanced_storage_rollback_notes(step: &ExecutionStep) -> Vec<String> {
    let collection = command_step_collection(step).unwrap_or("advanced-storage");
    let boundary = match collection {
        "pools" | "datasets" | "zvols" => "ZFS",
        "snapshots" => "snapshot/clone",
        "btrfsSubvolumes" | "btrfsQgroups" => "Btrfs",
        "caches" => "bcache",
        "lvmCaches" => "LVM cache",
        "vdoVolumes" => "VDO",
        _ => "advanced-storage",
    };
    vec![
        format!("advanced-storage rollback recipe covers the {boundary} boundary"),
        "read-only validation must be bound to expected, pre-apply, failed-apply, and current topology evidence".to_string(),
    ]
}

fn advanced_storage_proven_rollback_command(
    partial: &PartialExecutionRecovery,
    step: &ExecutionStep,
) -> Option<ExecutionCommand> {
    let collection = command_step_collection(step)?;
    match (collection, step.operation) {
        ("pools" | "datasets" | "zvols", Operation::SetProperty) => {
            zfs_property_rollback_command(step)
        }
        ("caches", Operation::SetProperty) => bcache_property_rollback_command(step),
        ("vdoVolumes", Operation::SetProperty) => vdo_property_rollback_command(step),
        ("btrfsSubvolumes", Operation::SetProperty) => {
            btrfs_subvolume_property_rollback_command(step)
        }
        ("datasets" | "zvols" | "snapshots", Operation::Rename)
            if partial.failed_phase == ExecutionPhase::Verification =>
        {
            zfs_rename_verification_rollback_command(step)
        }
        ("btrfsSubvolumes", Operation::Rename)
            if partial.failed_phase == ExecutionPhase::Verification =>
        {
            btrfs_subvolume_rename_verification_rollback_command(step)
        }
        _ => None,
    }
}

fn zfs_property_rollback_command(step: &ExecutionStep) -> Option<ExecutionCommand> {
    let rollback_value = step_note_value(step, "rollback-value")?;
    let rollback_command = step.commands.iter().find(|command| command.mutates)?;
    let [tool, subcommand, assignment, target] = rollback_command.argv.as_slice() else {
        return None;
    };
    if tool != "zfs" || subcommand != "set" {
        return None;
    }
    let property = assignment.split_once('=')?.0;
    Some(command_vec(
        vec![
            "zfs".to_string(),
            "set".to_string(),
            format!("{property}={rollback_value}"),
            target.clone(),
        ],
        true,
        "restore declared pre-apply ZFS property value",
    ))
}

fn bcache_property_rollback_command(step: &ExecutionStep) -> Option<ExecutionCommand> {
    let rollback_value = step_note_value(step, "rollback-value")?;
    let rollback_command = step.commands.iter().find(|command| command.mutates)?;
    let argv = rollback_command.argv.as_slice();
    if argv.len() != 7 || argv.first().is_none_or(|tool| tool != "sh") {
        return None;
    }
    let mut rollback_argv = rollback_command.argv.clone();
    rollback_argv[5] = rollback_value.to_string();
    Some(command_vec(
        rollback_argv,
        true,
        "restore declared pre-apply bcache property value",
    ))
}

fn vdo_property_rollback_command(step: &ExecutionStep) -> Option<ExecutionCommand> {
    let rollback_value = step_note_value(step, "rollback-value")?;
    let rollback_command = step.commands.iter().find(|command| command.mutates)?;
    let argv = match rollback_command.argv.as_slice() {
        [tool, subcommand, name_flag, name, policy_flag, _]
            if tool == "vdo"
                && subcommand == "changeWritePolicy"
                && name_flag == "--name"
                && policy_flag == "--writePolicy" =>
        {
            vec![
                "vdo".to_string(),
                "changeWritePolicy".to_string(),
                "--name".to_string(),
                name.clone(),
                "--writePolicy".to_string(),
                rollback_value.to_string(),
            ]
        }
        _ => return None,
    };
    Some(command_vec(
        argv,
        true,
        "restore declared pre-apply VDO property value",
    ))
}

fn btrfs_subvolume_property_rollback_command(step: &ExecutionStep) -> Option<ExecutionCommand> {
    let rollback_value = step_note_value(step, "rollback-value")?;
    let rollback_command = step.commands.iter().find(|command| command.mutates)?;
    let [tool, subcommand, action, flag, target, property, _] = rollback_command.argv.as_slice()
    else {
        return None;
    };
    if tool != "btrfs"
        || subcommand != "property"
        || action != "set"
        || flag != "-ts"
        || property != "ro"
    {
        return None;
    }
    Some(command_vec(
        vec![
            "btrfs".to_string(),
            "property".to_string(),
            "set".to_string(),
            "-ts".to_string(),
            target.clone(),
            "ro".to_string(),
            rollback_value.to_string(),
        ],
        true,
        "restore declared pre-apply Btrfs subvolume property value",
    ))
}

fn zfs_rename_verification_rollback_command(step: &ExecutionStep) -> Option<ExecutionCommand> {
    let rollback_command = step.commands.iter().find(|command| command.mutates)?;
    let [tool, subcommand, old_name, new_name] = rollback_command.argv.as_slice() else {
        return None;
    };
    if tool != "zfs" || subcommand != "rename" {
        return None;
    }
    let rollback_name = step_note_value(step, "rollback-value").unwrap_or(old_name);
    Some(command_vec(
        vec![
            "zfs".to_string(),
            "rename".to_string(),
            new_name.clone(),
            rollback_name.to_string(),
        ],
        true,
        "restore declared pre-apply ZFS object name after failed rename verification",
    ))
}

fn btrfs_subvolume_rename_verification_rollback_command(
    step: &ExecutionStep,
) -> Option<ExecutionCommand> {
    let rollback_command = step.commands.iter().find(|command| command.mutates)?;
    let [tool, flag, old_path, new_path] = rollback_command.argv.as_slice() else {
        return None;
    };
    if tool != "mv" || flag != "--" {
        return None;
    }
    let rollback_path = step_note_value(step, "rollback-value").unwrap_or(old_path);
    Some(command_vec(
        vec![
            "mv".to_string(),
            "--".to_string(),
            new_path.clone(),
            rollback_path.to_string(),
        ],
        true,
        "restore declared pre-apply Btrfs subvolume path after failed rename verification",
    ))
}

fn step_note_value<'a>(step: &'a ExecutionStep, key: &str) -> Option<&'a str> {
    let prefix = format!("{key}:");
    step.notes.iter().find_map(|note| {
        note.strip_prefix(&prefix)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    })
}

fn network_storage_rollback_recipe_for_step(
    partial: &PartialExecutionRecovery,
    rollback_review: &RecoveryAction,
    step: &ExecutionStep,
) -> Option<RollbackRecipe> {
    if !network_storage_collection(command_step_collection(step)?) {
        return None;
    }

    let validation_commands = rollback_review.commands.clone();
    let mut recipe = review_only_rollback_recipe(
        partial,
        rollback_review,
        network_storage_rollback_refusal_reasons(step),
        network_storage_rollback_notes(step),
    );

    if let Some(command) = network_storage_proven_rollback_command(partial, step) {
        recipe.status = RollbackRecipeStatus::ProvenSafe;
        recipe.read_only_validation.notes.push(
            "network-storage rollback validation must prove export, mount, session, LUN, and target-side identity still match the failed apply receipt".to_string(),
        );
        recipe.reversible_mutations = RollbackRecipeSection {
            commands: vec![command],
            notes: vec![
                "network-storage rollback mutation is limited to declared old option/property values or verification-bound mount/login inverses".to_string(),
                "remote export lifecycle, unmount/logout, growth, attach/detach, and target LUN topology boundaries remain refused without stronger initiator, target, and backing-store proof".to_string(),
            ],
        };
        recipe.operator_only_handoff = RollbackRecipeSection {
            commands: Vec::new(),
            notes: Vec::new(),
        };
        recipe.refusal_reasons = Vec::new();
        recipe.notes.push(
            "network-storage recipe is proven-safe only because the rollback command is derived from explicit pre-apply metadata or a failed verification boundary".to_string(),
        );
    } else if network_storage_refused_operation(step) {
        recipe.status = RollbackRecipeStatus::Refused;
        recipe.operator_only_handoff = RollbackRecipeSection {
            commands: validation_commands,
            notes: vec![
                "network-storage rollback requires operator review of clients, exports, mounts, iSCSI sessions, LUN mappings, target-side state, and active consumers before mutation".to_string(),
                "prefer roll-forward validation, fresh initiator and target inventory, restored export configuration, remount/login repair, or target-side recovery workflows instead of automated mutation".to_string(),
            ],
        };
    }

    Some(recipe)
}

fn network_storage_collection(collection: &str) -> bool {
    matches!(
        collection,
        "exports" | "nfs.mounts" | "iscsiSessions" | "luns" | "targetLuns"
    )
}

fn network_storage_refused_operation(step: &ExecutionStep) -> bool {
    matches!(
        step.operation,
        Operation::Attach
            | Operation::Create
            | Operation::Destroy
            | Operation::Detach
            | Operation::Export
            | Operation::Grow
            | Operation::Login
            | Operation::Logout
            | Operation::Mount
            | Operation::Remount
            | Operation::Rescan
            | Operation::SetProperty
            | Operation::Unmount
            | Operation::Unexport
    )
}

fn network_storage_rollback_refusal_reasons(step: &ExecutionStep) -> Vec<String> {
    let collection = command_step_collection(step).unwrap_or("network-storage");
    match (collection, step.operation) {
        ("nfs.mounts", Operation::Remount) => vec![
            "NFS remount rollback metadata is missing or insufficient for proven-safe replay"
                .to_string(),
        ],
        ("nfs.mounts", Operation::Mount) => vec![
            "NFS mount rollback is only automatic when mount succeeded and verification failed"
                .to_string(),
        ],
        ("exports", Operation::SetProperty) => vec![
            "NFS export option rollback metadata is missing or insufficient for proven-safe replay"
                .to_string(),
        ],
        ("iscsiSessions", Operation::Login) => vec![
            "iSCSI login rollback is only automatic when login succeeded and verification failed"
                .to_string(),
        ],
        ("targetLuns", Operation::SetProperty) => vec![
            "target LUN property rollback metadata is missing or insufficient for proven-safe replay"
                .to_string(),
        ],
        ("nfs.mounts", Operation::Unmount)
        | ("exports", Operation::Create | Operation::Destroy | Operation::Export | Operation::Unexport)
        | ("iscsiSessions", Operation::Logout | Operation::Create | Operation::Destroy)
        | ("luns", Operation::Attach | Operation::Detach | Operation::Grow | Operation::Rescan)
        | ("targetLuns", Operation::Attach | Operation::Create | Operation::Destroy | Operation::Detach | Operation::Grow | Operation::Rescan) => vec![
            "network-storage rollback is refused because client visibility, remote server state, target mapping, and active consumers require operator review".to_string(),
        ],
        _ => vec!["network-storage rollback for this operation remains review-only".to_string()],
    }
}

fn network_storage_rollback_notes(step: &ExecutionStep) -> Vec<String> {
    let collection = command_step_collection(step).unwrap_or("network-storage");
    let boundary = match collection {
        "exports" => "NFS export",
        "nfs.mounts" => "NFS mount",
        "iscsiSessions" => "iSCSI session",
        "luns" => "host LUN",
        "targetLuns" => "target LUN",
        _ => "network-storage",
    };
    vec![
        format!("network-storage rollback recipe covers the {boundary} boundary"),
        "read-only validation must be bound to expected, pre-apply, failed-apply, and current topology evidence".to_string(),
    ]
}

fn network_storage_proven_rollback_command(
    partial: &PartialExecutionRecovery,
    step: &ExecutionStep,
) -> Option<ExecutionCommand> {
    let collection = command_step_collection(step)?;
    match (collection, step.operation) {
        ("nfs.mounts", Operation::Remount) => nfs_mount_remount_rollback_command(step),
        ("nfs.mounts", Operation::Mount)
            if partial.failed_phase == ExecutionPhase::Verification =>
        {
            nfs_mount_verification_rollback_command(step)
        }
        ("exports", Operation::SetProperty) => nfs_export_property_rollback_command(step),
        ("iscsiSessions", Operation::Login)
            if partial.failed_phase == ExecutionPhase::Verification =>
        {
            iscsi_login_verification_rollback_command(step)
        }
        ("targetLuns", Operation::SetProperty) => target_lun_property_rollback_command(step),
        _ => None,
    }
}

fn nfs_mount_remount_rollback_command(step: &ExecutionStep) -> Option<ExecutionCommand> {
    let target = nfs_mount_target_from_step(step)?;
    let rollback_options = step_note_value(step, "rollback-value")
        .or_else(|| step_note_value(step, "rollback-options"))?;
    Some(command_vec(
        vec![
            "mount".to_string(),
            "-o".to_string(),
            format!("remount,{rollback_options}"),
            target.to_string(),
        ],
        true,
        "restore declared pre-apply NFS mount options",
    ))
}

fn nfs_mount_verification_rollback_command(step: &ExecutionStep) -> Option<ExecutionCommand> {
    let target = nfs_mount_target_from_step(step)?;
    Some(command_vec(
        vec!["umount".to_string(), target.to_string()],
        true,
        "undo NFS mount created by the failed apply after read-only validation",
    ))
}

fn nfs_export_property_rollback_command(step: &ExecutionStep) -> Option<ExecutionCommand> {
    let rollback_value = step_note_value(step, "rollback-value")?;
    let rollback_command = step.commands.iter().find(|command| command.mutates)?;
    let [tool, flag, option_flag, _, selector] = rollback_command.argv.as_slice() else {
        return None;
    };
    if tool != "exportfs" || flag != "-i" || option_flag != "-o" {
        return None;
    }
    Some(command_vec(
        vec![
            "exportfs".to_string(),
            "-i".to_string(),
            "-o".to_string(),
            rollback_value.to_string(),
            selector.clone(),
        ],
        true,
        "restore declared pre-apply NFS export options",
    ))
}

fn iscsi_login_verification_rollback_command(step: &ExecutionStep) -> Option<ExecutionCommand> {
    let rollback_command = step
        .commands
        .iter()
        .find(|command| command.mutates && command.argv.iter().any(|arg| arg == "--login"))?;
    let target = iscsi_target_from_command(rollback_command)?;
    let portal = iscsi_portal_from_command(rollback_command);
    let mut argv = vec![
        "iscsiadm".to_string(),
        "--mode".to_string(),
        "node".to_string(),
        "--targetname".to_string(),
        target.to_string(),
    ];
    if let Some(portal) = portal {
        argv.extend(["--portal".to_string(), portal.to_string()]);
    }
    argv.push("--logout".to_string());
    Some(command_vec(
        argv,
        true,
        "logout iSCSI session created by the failed apply after read-only validation",
    ))
}

fn iscsi_target_from_command(command: &ExecutionCommand) -> Option<&str> {
    command
        .argv
        .windows(2)
        .find(|window| window[0] == "--targetname")
        .map(|window| window[1].as_str())
        .filter(|target| !target.starts_with('<'))
}

fn iscsi_portal_from_command(command: &ExecutionCommand) -> Option<&str> {
    command
        .argv
        .iter()
        .position(|arg| arg == "--portal")
        .and_then(|index| command.argv.get(index + 1))
        .map(String::as_str)
        .filter(|portal| !portal.starts_with('<'))
}

fn target_lun_property_rollback_command(step: &ExecutionStep) -> Option<ExecutionCommand> {
    let rollback_value = step_note_value(step, "rollback-value")?;
    let rollback_command = step.commands.iter().find(|command| command.mutates)?;
    match rollback_command.argv.first().map(String::as_str) {
        Some("targetcli") => {
            target_lun_lio_property_rollback_command(rollback_command, rollback_value)
        }
        Some("tgtadm") => {
            target_lun_tgt_property_rollback_command(rollback_command, rollback_value)
        }
        Some("scstadmin") => {
            target_lun_scst_property_rollback_command(rollback_command, rollback_value)
        }
        _ => None,
    }
}

fn target_lun_lio_property_rollback_command(
    rollback_command: &ExecutionCommand,
    rollback_value: &str,
) -> Option<ExecutionCommand> {
    let [tool, backstore_path, subcommand, scope, assignment] = rollback_command.argv.as_slice()
    else {
        return None;
    };
    if tool != "targetcli" || subcommand != "set" || scope != "attribute" {
        return None;
    }
    let property = assignment.split_once('=')?.0;
    Some(command_vec(
        vec![
            "targetcli".to_string(),
            backstore_path.clone(),
            "set".to_string(),
            "attribute".to_string(),
            format!("{property}={rollback_value}"),
        ],
        true,
        "restore declared pre-apply LIO target LUN attribute",
    ))
}

fn target_lun_tgt_property_rollback_command(
    rollback_command: &ExecutionCommand,
    rollback_value: &str,
) -> Option<ExecutionCommand> {
    let property_index = rollback_command
        .argv
        .iter()
        .position(|arg| arg == "--name")?
        + 1;
    let value_index = rollback_command
        .argv
        .iter()
        .position(|arg| arg == "--value")?
        + 1;
    rollback_command.argv.get(property_index)?;
    rollback_command.argv.get(value_index)?;
    let mut argv = rollback_command.argv.clone();
    argv[value_index] = rollback_value.to_string();
    Some(command_vec(
        argv,
        true,
        "restore declared pre-apply Linux tgt logical-unit property",
    ))
}

fn target_lun_scst_property_rollback_command(
    rollback_command: &ExecutionCommand,
    rollback_value: &str,
) -> Option<ExecutionCommand> {
    let attributes_index = rollback_command
        .argv
        .iter()
        .position(|arg| arg == "-attributes")?
        + 1;
    let assignment = rollback_command.argv.get(attributes_index)?;
    let property = assignment.split_once('=')?.0;
    let mut argv = rollback_command.argv.clone();
    argv[attributes_index] = format!("{property}={rollback_value}");
    Some(command_vec(
        argv,
        true,
        "restore declared pre-apply SCST target LUN attribute",
    ))
}

fn rollback_recipe_safety_gates() -> Vec<String> {
    vec![
        "original apply receipt must match this failed apply report".to_string(),
        "fresh topology probe must be captured after the failure".to_string(),
        "expected, pre-apply, failed-apply, and current topology evidence must be bound before automated rollback".to_string(),
        "rollback point identity must still match the failed action target".to_string(),
        "active consumers, mounts, exports, sessions, or open mappings must be reviewed before any mutation".to_string(),
        "missing tools, stale identity data, and ambiguous rollback targets keep the recipe review-only".to_string(),
        "filesystem rollback gates require verified ext, XFS, FAT, exFAT, NTFS, f2fs, mount/remount, trim, scrub, repair, grow, and shrink state before mutation".to_string(),
        "block-stack rollback gates require verified disk label, partition, LUKS, LVM, MD RAID, device-mapper, loop, backing-file, swap, and zram topology before mutation".to_string(),
        "advanced-storage rollback gates require verified ZFS, Btrfs, bcachefs, bcache, LVM cache, VDO, snapshot, clone, and pool-membership topology before mutation".to_string(),
        "network-storage rollback gates require verified NFS, iSCSI, multipath, NVMe-oF, host-side LUN, and target-side LUN provider topology before mutation".to_string(),
    ]
}

#[must_use]
pub fn replay_proven_safe_rollback_recipe(
    failed_report: &ExecutionReport,
    recipe_index: usize,
    original_receipt_id: impl Into<String>,
    fresh_topology_probe_id: impl Into<String>,
) -> RollbackExecutionReport {
    let fresh_topology_probe_id = fresh_topology_probe_id.into();
    let topology_evidence =
        materialize_rollback_topology_evidence(failed_report, &fresh_topology_probe_id);
    replay_proven_safe_rollback_recipe_with_topology_evidence(
        failed_report,
        recipe_index,
        original_receipt_id,
        fresh_topology_probe_id,
        topology_evidence,
    )
}

#[must_use]
pub fn materialize_rollback_topology_evidence(
    failed_report: &ExecutionReport,
    fresh_topology_probe_id: &str,
) -> BTreeMap<String, String> {
    let mut topology_evidence = BTreeMap::from([
        (
            "expected".to_string(),
            topology_evidence_id("expected", &failed_report.apply),
        ),
        (
            "preApply".to_string(),
            topology_evidence_id(
                "pre-apply",
                &(
                    &failed_report.topology_comparison,
                    &failed_report.command_plan,
                    &failed_report.verification_plan,
                ),
            ),
        ),
        (
            "failedApply".to_string(),
            topology_evidence_id(
                "failed-apply",
                &(
                    failed_report.status,
                    &failed_report.partial_execution_recovery,
                    &failed_report.execution_results,
                ),
            ),
        ),
    ]);
    if !fresh_topology_probe_id.trim().is_empty() {
        topology_evidence.insert("current".to_string(), fresh_topology_probe_id.to_string());
    }
    topology_evidence
}

#[must_use]
pub fn materialize_rollback_topology_payloads(
    failed_report: &ExecutionReport,
    current_topology_payload: serde_json::Value,
) -> BTreeMap<String, serde_json::Value> {
    BTreeMap::from([
        (
            "expected".to_string(),
            serde_json::to_value(&failed_report.apply).unwrap_or(serde_json::Value::Null),
        ),
        (
            "preApply".to_string(),
            serde_json::to_value((
                &failed_report.topology_comparison,
                &failed_report.command_plan,
                &failed_report.verification_plan,
            ))
            .unwrap_or(serde_json::Value::Null),
        ),
        (
            "failedApply".to_string(),
            serde_json::to_value((
                failed_report.status,
                &failed_report.partial_execution_recovery,
                &failed_report.execution_results,
            ))
            .unwrap_or(serde_json::Value::Null),
        ),
        ("current".to_string(), current_topology_payload),
    ])
}

#[must_use]
pub fn replay_proven_safe_rollback_recipe_with_topology_evidence(
    failed_report: &ExecutionReport,
    recipe_index: usize,
    original_receipt_id: impl Into<String>,
    fresh_topology_probe_id: impl Into<String>,
    topology_evidence: BTreeMap<String, String>,
) -> RollbackExecutionReport {
    let mut runner = run_command;
    replay_proven_safe_rollback_recipe_with_runner_and_tool_checker(
        failed_report,
        recipe_index,
        RollbackReplayBindings {
            original_receipt_id: original_receipt_id.into(),
            fresh_topology_probe_id: fresh_topology_probe_id.into(),
            topology_evidence,
            topology_payloads: BTreeMap::new(),
        },
        &mut runner,
        command_exists,
    )
}

#[must_use]
pub fn replay_proven_safe_rollback_recipe_with_topology_payloads(
    failed_report: &ExecutionReport,
    recipe_index: usize,
    original_receipt_id: impl Into<String>,
    fresh_topology_probe_id: impl Into<String>,
    topology_evidence: BTreeMap<String, String>,
    topology_payloads: BTreeMap<String, serde_json::Value>,
) -> RollbackExecutionReport {
    let mut runner = run_command;
    replay_proven_safe_rollback_recipe_with_runner_and_tool_checker(
        failed_report,
        recipe_index,
        RollbackReplayBindings {
            original_receipt_id: original_receipt_id.into(),
            fresh_topology_probe_id: fresh_topology_probe_id.into(),
            topology_evidence,
            topology_payloads,
        },
        &mut runner,
        command_exists,
    )
}

#[cfg(test)]
fn replay_proven_safe_rollback_recipe_with_runner(
    failed_report: &ExecutionReport,
    recipe_index: usize,
    original_receipt_id: String,
    fresh_topology_probe_id: String,
    runner: &mut impl FnMut(&[String]) -> CommandRunResult,
) -> RollbackExecutionReport {
    let mut topology_evidence = current_topology_evidence(&fresh_topology_probe_id);
    if !fresh_topology_probe_id.trim().is_empty() {
        topology_evidence.insert("expected".to_string(), "topology:expected-123".to_string());
        topology_evidence.insert("preApply".to_string(), "topology:pre-apply-123".to_string());
        topology_evidence.insert(
            "failedApply".to_string(),
            "topology:failed-apply-123".to_string(),
        );
    }
    replay_proven_safe_rollback_recipe_with_runner_and_tool_checker(
        failed_report,
        recipe_index,
        RollbackReplayBindings {
            original_receipt_id,
            fresh_topology_probe_id,
            topology_evidence,
            topology_payloads: BTreeMap::new(),
        },
        runner,
        |_| true,
    )
}

fn replay_proven_safe_rollback_recipe_with_runner_and_tool_checker(
    failed_report: &ExecutionReport,
    recipe_index: usize,
    bindings: RollbackReplayBindings,
    runner: &mut impl FnMut(&[String]) -> CommandRunResult,
    tool_exists: impl Fn(&str) -> bool,
) -> RollbackExecutionReport {
    let Some(recipe) = failed_report.rollback_recipes.get(recipe_index) else {
        return refused_rollback_report(
            0,
            "",
            &[],
            bindings,
            vec!["rollback recipe index does not exist".to_string()],
        );
    };

    let mut refusal_reasons = proven_safe_rollback_refusal_reasons(
        failed_report,
        recipe,
        &bindings.original_receipt_id,
        &bindings.fresh_topology_probe_id,
        &bindings.topology_evidence,
        tool_exists,
    );
    if !refusal_reasons.is_empty() {
        refusal_reasons.extend(recipe.refusal_reasons.iter().cloned());
        return refused_rollback_report(
            recipe.recipe_version,
            &recipe.source_action_id,
            &recipe.failed_command,
            bindings,
            refusal_reasons,
        );
    }

    let mut validation_results = Vec::new();
    for command in &recipe.read_only_validation.commands {
        let result = run_planned_command(
            ExecutionPhase::Verification,
            &recipe.source_action_id,
            &command.argv,
            runner,
        );
        let success = result.success;
        validation_results.push(result);
        if !success {
            return RollbackExecutionReport {
                status: RollbackExecutionStatus::Failed,
                recipe_version: recipe.recipe_version,
                source_action_id: recipe.source_action_id.clone(),
                receipt_binding: rollback_receipt_binding(recipe, bindings.clone()),
                validation_results,
                rollback_results: Vec::new(),
                messages: vec![
                    "rollback validation failed; reversible mutation steps were not executed"
                        .to_string(),
                ],
                refusal_reasons: Vec::new(),
            };
        }
    }

    let mut rollback_results = Vec::new();
    for command in &recipe.reversible_mutations.commands {
        let result = run_planned_command(
            ExecutionPhase::Command,
            &recipe.source_action_id,
            &command.argv,
            runner,
        );
        let success = result.success;
        rollback_results.push(result);
        if !success {
            return RollbackExecutionReport {
                status: RollbackExecutionStatus::Failed,
                recipe_version: recipe.recipe_version,
                source_action_id: recipe.source_action_id.clone(),
                receipt_binding: rollback_receipt_binding(
                    recipe,
                    bindings.clone(),
                ),
                validation_results,
                rollback_results,
                messages: vec![
                    "proven-safe rollback mutation failed; capture a fresh topology probe before retrying or handoff".to_string(),
                ],
                refusal_reasons: Vec::new(),
            };
        }
    }

    RollbackExecutionReport {
        status: RollbackExecutionStatus::Succeeded,
        recipe_version: recipe.recipe_version,
        source_action_id: recipe.source_action_id.clone(),
        receipt_binding: rollback_receipt_binding(recipe, bindings),
        validation_results,
        rollback_results,
        messages: vec![
            "proven-safe rollback validation and reversible mutation steps completed".to_string(),
            "capture and compare a fresh topology probe after rollback before resuming apply"
                .to_string(),
        ],
        refusal_reasons: Vec::new(),
    }
}

fn proven_safe_rollback_refusal_reasons(
    failed_report: &ExecutionReport,
    recipe: &RollbackRecipe,
    original_receipt_id: &str,
    fresh_topology_probe_id: &str,
    topology_evidence: &BTreeMap<String, String>,
    tool_exists: impl Fn(&str) -> bool,
) -> Vec<String> {
    let mut reasons = Vec::new();

    if failed_report.status != ExecutionStatus::Failed {
        reasons.push("automatic rollback replay requires a failed apply report".to_string());
    }
    if recipe.status != RollbackRecipeStatus::ProvenSafe {
        reasons.push("rollback recipe is not marked proven-safe".to_string());
    }
    if recipe.receipt_binding_required && original_receipt_id.trim().is_empty() {
        reasons.push("original apply receipt binding is required".to_string());
    }
    if recipe.fresh_topology_probe_required && fresh_topology_probe_id.trim().is_empty() {
        reasons.push("fresh post-failure topology probe binding is required".to_string());
    }
    let missing_topology_evidence = missing_required_topology_evidence(recipe, topology_evidence);
    if !missing_topology_evidence.is_empty() {
        reasons.push(format!(
            "automatic rollback replay refuses missing topology evidence binding(s): {}",
            missing_topology_evidence.join(", ")
        ));
    }
    reasons.extend(rollback_topology_comparison_refusal_reasons(failed_report));
    if !recipe.destructive_mutations.commands.is_empty() {
        reasons.push("automatic rollback replay refuses destructive mutation steps".to_string());
    }
    if !recipe.operator_only_handoff.commands.is_empty() {
        reasons.push("automatic rollback replay refuses operator-only handoff steps".to_string());
    }
    if recipe.reversible_mutations.commands.is_empty() {
        reasons.push("rollback recipe has no proven-safe reversible mutation steps".to_string());
    }
    let missing_tools = missing_rollback_replay_tools(recipe, tool_exists);
    if !missing_tools.is_empty() {
        reasons.push(format!(
            "automatic rollback replay refuses missing required tool(s): {}",
            missing_tools.join(", ")
        ));
    }

    for command in &recipe.read_only_validation.commands {
        if command.mutates {
            reasons.push(format!(
                "read-only validation command mutates state: {}",
                command.argv.join(" ")
            ));
        }
        if command.readiness != CommandReadiness::Ready {
            reasons.push(format!(
                "read-only validation command is not ready: {}",
                command.argv.join(" ")
            ));
        }
    }
    for command in &recipe.reversible_mutations.commands {
        if !command.mutates {
            reasons.push(format!(
                "reversible rollback command is not marked mutating: {}",
                command.argv.join(" ")
            ));
        }
        if command.readiness != CommandReadiness::Ready {
            reasons.push(format!(
                "reversible rollback command is not ready: {}",
                command.argv.join(" ")
            ));
        }
        if let Some(reason) = rollback_command_data_loss_risk_reason(command) {
            reasons.push(reason);
        }
        if let Some(reason) = rollback_command_live_use_blocker_reason(command) {
            reasons.push(reason);
        }
        if let Some(reason) = rollback_command_identity_blocker_reason(command) {
            reasons.push(reason);
        }
        if let Some(reason) = rollback_command_idempotency_blocker_reason(command) {
            reasons.push(reason);
        }
    }

    reasons
}

fn rollback_command_data_loss_risk_reason(command: &ExecutionCommand) -> Option<String> {
    let risky_arg_tokens = [
        "destroy",
        "delete",
        "detach",
        "discard",
        "flush",
        "format",
        "kill-slot",
        "remove",
        "rollback",
        "shrink",
        "wipe",
    ];
    if command.argv.iter().any(|part| {
        let part = part.to_ascii_lowercase();
        risky_arg_tokens
            .iter()
            .any(|token| part == *token || part.starts_with(&format!("{token}=")))
    }) {
        return Some(format!(
            "automatic rollback replay refuses plausible data-loss command: {}",
            command.argv.join(" ")
        ));
    }

    let risky_phrases = [
        "data loss",
        "data-loss",
        "destructive",
        "discard data",
        "discard newer data",
        "format",
        "potential data loss",
        "potential-data-loss",
        "shrink",
        "wipe",
    ];
    let mut risk_fields = command
        .provider_capabilities
        .iter()
        .chain(command.unresolved_inputs.iter())
        .chain(std::iter::once(&command.note));
    if risk_fields.any(|field| {
        let field = field.to_ascii_lowercase();
        risky_phrases.iter().any(|phrase| field.contains(phrase))
    }) {
        return Some(format!(
            "automatic rollback replay refuses plausible data-loss command metadata: {}",
            command.argv.join(" ")
        ));
    }

    None
}

fn rollback_command_live_use_blocker_reason(command: &ExecutionCommand) -> Option<String> {
    let blocker_phrases = [
        "active consumer",
        "active consumers",
        "active session",
        "active sessions",
        "exported lun",
        "exported luns",
        "holder",
        "holders",
        "live mapping",
        "live mappings",
        "mounted filesystem",
        "mounted filesystems",
        "mounted",
        "open encrypted mapping",
        "open encrypted mappings",
        "open mapping",
        "open mappings",
    ];
    let mut metadata_fields = command
        .provider_capabilities
        .iter()
        .chain(command.unresolved_inputs.iter())
        .chain(std::iter::once(&command.note));
    if metadata_fields.any(|field| {
        let field = field.to_ascii_lowercase();
        blocker_phrases.iter().any(|phrase| field.contains(phrase))
    }) {
        return Some(format!(
            "automatic rollback replay refuses live-use blocker metadata: {}",
            command.argv.join(" ")
        ));
    }

    None
}

fn rollback_command_identity_blocker_reason(command: &ExecutionCommand) -> Option<String> {
    let blocker_phrases = [
        "ambiguous rollback point",
        "ambiguous rollback target",
        "ambiguous target",
        "rollback point missing",
        "rollback point stale",
        "stale identity",
        "stale identity data",
        "stale rollback point",
        "stale target identity",
        "unbound rollback point",
        "unbound rollback target",
    ];
    let mut metadata_fields = command
        .provider_capabilities
        .iter()
        .chain(command.unresolved_inputs.iter())
        .chain(std::iter::once(&command.note));
    if metadata_fields.any(|field| {
        let field = field.to_ascii_lowercase();
        blocker_phrases.iter().any(|phrase| field.contains(phrase))
    }) {
        return Some(format!(
            "automatic rollback replay refuses ambiguous or stale identity metadata: {}",
            command.argv.join(" ")
        ));
    }

    None
}

fn rollback_command_idempotency_blocker_reason(command: &ExecutionCommand) -> Option<String> {
    let blocker_phrases = [
        "already rolled back",
        "already-rolled-back",
        "external modification",
        "external modifications",
        "externally modified",
        "partially rolled back",
        "partially-rolled-back",
        "rollback already applied",
        "rollback partially applied",
        "rollback state diverged",
        "topology externally modified",
    ];
    let mut metadata_fields = command
        .provider_capabilities
        .iter()
        .chain(command.unresolved_inputs.iter())
        .chain(std::iter::once(&command.note));
    if metadata_fields.any(|field| {
        let field = field.to_ascii_lowercase();
        blocker_phrases.iter().any(|phrase| field.contains(phrase))
    }) {
        return Some(format!(
            "automatic rollback replay refuses idempotency blocker metadata: {}",
            command.argv.join(" ")
        ));
    }

    None
}

#[cfg(test)]
fn current_topology_evidence(fresh_topology_probe_id: &str) -> BTreeMap<String, String> {
    let mut topology_evidence = BTreeMap::new();
    if !fresh_topology_probe_id.trim().is_empty() {
        topology_evidence.insert("current".to_string(), fresh_topology_probe_id.to_string());
    }
    topology_evidence
}

fn topology_evidence_id(label: &str, value: &impl Serialize) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_else(|_| label.as_bytes().to_vec());
    format!("topology:{label}:{:016x}", fnv1a64(&bytes))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn missing_required_topology_evidence(
    recipe: &RollbackRecipe,
    topology_evidence: &BTreeMap<String, String>,
) -> Vec<String> {
    recipe
        .required_topology_evidence
        .iter()
        .filter_map(|label| {
            let present = topology_evidence
                .get(label)
                .is_some_and(|evidence_id| !evidence_id.trim().is_empty());
            (!present).then(|| label.clone())
        })
        .collect()
}

fn rollback_topology_comparison_refusal_reasons(failed_report: &ExecutionReport) -> Vec<String> {
    let Some(comparison) = failed_report.topology_comparison.as_ref() else {
        return Vec::new();
    };
    let summary = &comparison.summary;
    let mut divergences = Vec::new();
    if summary.missing_count > 0 {
        divergences.push(format!("{} missing target(s)", summary.missing_count));
    }
    if summary.size_diagnostic_count > 0 {
        divergences.push(format!(
            "{} size diagnostic(s)",
            summary.size_diagnostic_count
        ));
    }
    if summary.type_conflict_count > 0 {
        divergences.push(format!(
            "{} type conflict diagnostic(s)",
            summary.type_conflict_count
        ));
    }
    if summary.graph_dependency_conflict_count > 0 {
        divergences.push(format!(
            "{} graph dependency conflict(s)",
            summary.graph_dependency_conflict_count
        ));
    }
    if summary.partially_suppressed_group_count > 0 {
        divergences.push(format!(
            "{} partially suppressed reconciliation group(s)",
            summary.partially_suppressed_group_count
        ));
    }
    divergences.extend(rollback_topology_diagnostic_refusal_reasons(comparison));

    if divergences.is_empty() {
        Vec::new()
    } else {
        vec![format!(
            "automatic rollback replay refuses divergent topology comparison: {}",
            divergences.join(", ")
        )]
    }
}

fn rollback_topology_diagnostic_refusal_reasons(comparison: &TopologyComparison) -> Vec<String> {
    let mut live_use = BTreeSet::new();
    let mut stale_identity = BTreeSet::new();
    let mut idempotency = BTreeSet::new();
    let mut data_loss = BTreeSet::new();

    for diagnostic in &comparison.diagnostics {
        if rollback_topology_diagnostic_is_live_use_blocker(diagnostic.kind) {
            live_use.insert(rollback_topology_diagnostic_label(
                &diagnostic.action_id,
                diagnostic.kind,
            ));
        }
        if rollback_topology_diagnostic_is_stale_identity_blocker(diagnostic.kind) {
            stale_identity.insert(rollback_topology_diagnostic_label(
                &diagnostic.action_id,
                diagnostic.kind,
            ));
        }
        if rollback_topology_diagnostic_is_idempotency_blocker(diagnostic.kind) {
            idempotency.insert(rollback_topology_diagnostic_label(
                &diagnostic.action_id,
                diagnostic.kind,
            ));
        }
        if rollback_topology_diagnostic_is_data_loss_risk(diagnostic.kind) {
            data_loss.insert(rollback_topology_diagnostic_label(
                &diagnostic.action_id,
                diagnostic.kind,
            ));
        }
    }

    let mut reasons = Vec::new();
    if !live_use.is_empty() {
        reasons.push(format!(
            "topology diagnostic live-use blocker(s): {}",
            live_use.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    if !stale_identity.is_empty() {
        reasons.push(format!(
            "topology diagnostic stale identity or ambiguous rollback point(s): {}",
            stale_identity.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    if !idempotency.is_empty() {
        reasons.push(format!(
            "topology diagnostic rollback idempotency blocker(s): {}",
            idempotency.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    if !data_loss.is_empty() {
        reasons.push(format!(
            "topology diagnostic plausible data-loss path(s): {}",
            data_loss.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    reasons
}

fn rollback_topology_diagnostic_label(action_id: &str, kind: TopologyDiagnosticKind) -> String {
    format!("{action_id}:{kind:?}")
}

fn rollback_topology_diagnostic_is_live_use_blocker(kind: TopologyDiagnosticKind) -> bool {
    matches!(
        kind,
        TopologyDiagnosticKind::MountRequired
            | TopologyDiagnosticKind::MountOptionsDiffer
            | TopologyDiagnosticKind::UnmountRequired
            | TopologyDiagnosticKind::NfsExportDiffers
            | TopologyDiagnosticKind::NfsExportRequired
            | TopologyDiagnosticKind::NfsUnexportRequired
            | TopologyDiagnosticKind::IscsiLoginRequired
            | TopologyDiagnosticKind::IscsiLogoutRequired
            | TopologyDiagnosticKind::LunAttachRequired
            | TopologyDiagnosticKind::LunDetachRequired
            | TopologyDiagnosticKind::NvmeNamespaceAttachRequired
            | TopologyDiagnosticKind::NvmeNamespaceDetachRequired
            | TopologyDiagnosticKind::LvmActivateRequired
            | TopologyDiagnosticKind::LvmDeactivateRequired
            | TopologyDiagnosticKind::LvmVgExportRequired
            | TopologyDiagnosticKind::LvmVgImportRequired
            | TopologyDiagnosticKind::LuksCloseRequired
            | TopologyDiagnosticKind::LuksOpenRequired
            | TopologyDiagnosticKind::DmMapDestroyRequired
            | TopologyDiagnosticKind::DmMapRenameRequired
            | TopologyDiagnosticKind::MultipathDestroyRequired
            | TopologyDiagnosticKind::MultipathPathAddRequired
            | TopologyDiagnosticKind::MultipathPathRemoveRequired
            | TopologyDiagnosticKind::SwapDeactivateRequired
            | TopologyDiagnosticKind::LoopDetachRequired
            | TopologyDiagnosticKind::MdStopRequired
            | TopologyDiagnosticKind::VdoStartRequired
            | TopologyDiagnosticKind::VdoStopRequired
    )
}

fn rollback_topology_diagnostic_is_stale_identity_blocker(kind: TopologyDiagnosticKind) -> bool {
    matches!(
        kind,
        TopologyDiagnosticKind::Missing
            | TopologyDiagnosticKind::MountSourceConflict
            | TopologyDiagnosticKind::LoopCreateConflict
            | TopologyDiagnosticKind::LuksFormatTargetPresent
            | TopologyDiagnosticKind::SwapFormatTargetPresent
            | TopologyDiagnosticKind::VdoCreateTargetPresent
            | TopologyDiagnosticKind::SnapshotCloneSourceMissing
            | TopologyDiagnosticKind::SnapshotRenameSourceMissing
            | TopologyDiagnosticKind::SnapshotRollbackPointMissing
    )
}

fn rollback_topology_diagnostic_is_idempotency_blocker(kind: TopologyDiagnosticKind) -> bool {
    matches!(
        kind,
        TopologyDiagnosticKind::Matched
            | TopologyDiagnosticKind::SizeAlreadySatisfied
            | TopologyDiagnosticKind::FilesystemFormatAlreadySatisfied
            | TopologyDiagnosticKind::DiskCreateAlreadySatisfied
            | TopologyDiagnosticKind::BtrfsSubvolumeCreateAlreadySatisfied
            | TopologyDiagnosticKind::BtrfsSubvolumeDestroyAlreadySatisfied
            | TopologyDiagnosticKind::BtrfsQgroupCreateAlreadySatisfied
            | TopologyDiagnosticKind::BtrfsQgroupDestroyAlreadySatisfied
            | TopologyDiagnosticKind::BcacheDetachAlreadySatisfied
            | TopologyDiagnosticKind::BackingFileCreateAlreadySatisfied
            | TopologyDiagnosticKind::PartitionCreateAlreadySatisfied
            | TopologyDiagnosticKind::LvmPvCreateAlreadySatisfied
            | TopologyDiagnosticKind::LvmVolumeCreateAlreadySatisfied
            | TopologyDiagnosticKind::LvmVgCreateAlreadySatisfied
            | TopologyDiagnosticKind::IscsiLoginAlreadySatisfied
            | TopologyDiagnosticKind::IscsiLogoutAlreadySatisfied
            | TopologyDiagnosticKind::LunAttachAlreadySatisfied
            | TopologyDiagnosticKind::LunDetachAlreadySatisfied
            | TopologyDiagnosticKind::DmMapDestroyAlreadySatisfied
            | TopologyDiagnosticKind::DmMapRenameAlreadySatisfied
            | TopologyDiagnosticKind::NvmeNamespaceAttachAlreadySatisfied
            | TopologyDiagnosticKind::NvmeNamespaceDetachAlreadySatisfied
            | TopologyDiagnosticKind::LvmActivateAlreadySatisfied
            | TopologyDiagnosticKind::LvmDeactivateAlreadySatisfied
            | TopologyDiagnosticKind::LvmVgExportAlreadySatisfied
            | TopologyDiagnosticKind::LvmVgImportAlreadySatisfied
            | TopologyDiagnosticKind::LvmRenameAlreadySatisfied
            | TopologyDiagnosticKind::LvmCacheDetachAlreadySatisfied
            | TopologyDiagnosticKind::LuksCloseAlreadySatisfied
            | TopologyDiagnosticKind::LuksOpenAlreadySatisfied
            | TopologyDiagnosticKind::LuksKeyslotRemoveAlreadySatisfied
            | TopologyDiagnosticKind::LuksTokenRemoveAlreadySatisfied
            | TopologyDiagnosticKind::MultipathDestroyAlreadySatisfied
            | TopologyDiagnosticKind::MultipathPathAddAlreadySatisfied
            | TopologyDiagnosticKind::MultipathPathRemoveAlreadySatisfied
            | TopologyDiagnosticKind::SwapDeactivateAlreadySatisfied
            | TopologyDiagnosticKind::SwapDestroyAlreadySatisfied
            | TopologyDiagnosticKind::LoopCreateAlreadySatisfied
            | TopologyDiagnosticKind::LoopDetachAlreadySatisfied
            | TopologyDiagnosticKind::MdCreateAlreadySatisfied
            | TopologyDiagnosticKind::MdAssembleAlreadySatisfied
            | TopologyDiagnosticKind::MdStopAlreadySatisfied
            | TopologyDiagnosticKind::MdMemberAddAlreadySatisfied
            | TopologyDiagnosticKind::MdMemberRemoveAlreadySatisfied
            | TopologyDiagnosticKind::MdMemberReplaceAlreadySatisfied
            | TopologyDiagnosticKind::MountAlreadySatisfied
            | TopologyDiagnosticKind::MountOptionsAlreadySatisfied
            | TopologyDiagnosticKind::UnmountAlreadySatisfied
            | TopologyDiagnosticKind::NfsExportAlreadySatisfied
            | TopologyDiagnosticKind::NfsUnexportAlreadySatisfied
            | TopologyDiagnosticKind::PropertyAlreadySatisfied
            | TopologyDiagnosticKind::SnapshotCloneSourceAvailable
            | TopologyDiagnosticKind::SnapshotDestroyAlreadySatisfied
            | TopologyDiagnosticKind::SnapshotRollbackPointAvailable
            | TopologyDiagnosticKind::VdoDestroyAlreadySatisfied
            | TopologyDiagnosticKind::VdoStartAlreadySatisfied
            | TopologyDiagnosticKind::VdoStopAlreadySatisfied
            | TopologyDiagnosticKind::ZfsObjectCreateAlreadySatisfied
            | TopologyDiagnosticKind::ZfsObjectDestroyAlreadySatisfied
            | TopologyDiagnosticKind::ZfsObjectPromoteAlreadySatisfied
            | TopologyDiagnosticKind::ZfsObjectRenameAlreadySatisfied
            | TopologyDiagnosticKind::ZfsPoolCreateAlreadySatisfied
            | TopologyDiagnosticKind::ZfsPoolImportAlreadySatisfied
    )
}

fn rollback_topology_diagnostic_is_data_loss_risk(kind: TopologyDiagnosticKind) -> bool {
    matches!(
        kind,
        TopologyDiagnosticKind::BtrfsSubvolumeDestroyRequired
            | TopologyDiagnosticKind::BtrfsQgroupDestroyRequired
            | TopologyDiagnosticKind::BcacheDetachRequired
            | TopologyDiagnosticKind::LvmCacheDetachRequired
            | TopologyDiagnosticKind::LuksKeyslotRemoveRequired
            | TopologyDiagnosticKind::LuksTokenRemoveRequired
            | TopologyDiagnosticKind::MultipathDestroyRequired
            | TopologyDiagnosticKind::MultipathPathRemoveRequired
            | TopologyDiagnosticKind::SwapDestroyRequired
            | TopologyDiagnosticKind::MdMemberRemoveRequired
            | TopologyDiagnosticKind::SnapshotDestroyRequired
            | TopologyDiagnosticKind::VdoDestroyRequired
            | TopologyDiagnosticKind::ZfsObjectDestroyRequired
    )
}

fn missing_rollback_replay_tools(
    recipe: &RollbackRecipe,
    tool_exists: impl Fn(&str) -> bool,
) -> Vec<String> {
    let mut tools = BTreeSet::new();
    for command in recipe
        .read_only_validation
        .commands
        .iter()
        .chain(recipe.reversible_mutations.commands.iter())
    {
        if let Some(tool) = command.argv.first().filter(|tool| !tool.starts_with('<')) {
            tools.insert(tool.clone());
        }
    }
    tools
        .into_iter()
        .filter(|tool| !tool_exists(tool))
        .collect()
}

fn refused_rollback_report(
    recipe_version: u64,
    source_action_id: &str,
    failed_command: &[String],
    bindings: RollbackReplayBindings,
    refusal_reasons: Vec<String>,
) -> RollbackExecutionReport {
    RollbackExecutionReport {
        status: RollbackExecutionStatus::Refused,
        recipe_version,
        source_action_id: source_action_id.to_string(),
        receipt_binding: RollbackReceiptBinding {
            original_receipt_id: bindings.original_receipt_id,
            source_action_id: source_action_id.to_string(),
            failed_command: failed_command.to_vec(),
            fresh_topology_probe_id: bindings.fresh_topology_probe_id,
            topology_evidence: bindings.topology_evidence,
            topology_payloads: bindings.topology_payloads,
        },
        validation_results: Vec::new(),
        rollback_results: Vec::new(),
        messages: vec!["automatic rollback replay refused before executing commands".to_string()],
        refusal_reasons,
    }
}

fn rollback_receipt_binding(
    recipe: &RollbackRecipe,
    bindings: RollbackReplayBindings,
) -> RollbackReceiptBinding {
    RollbackReceiptBinding {
        original_receipt_id: bindings.original_receipt_id,
        source_action_id: recipe.source_action_id.clone(),
        failed_command: recipe.failed_command.clone(),
        fresh_topology_probe_id: bindings.fresh_topology_probe_id,
        topology_evidence: bindings.topology_evidence,
        topology_payloads: bindings.topology_payloads,
    }
}

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

fn domain_roll_forward_inspection_commands(step: &ExecutionStep) -> Vec<ExecutionCommand> {
    let mut commands = Vec::new();
    if let Some(target) = command_step_target(step) {
        commands.push(command_vec(
            ["disk-nix", "inspect", target, "--json"],
            false,
            "inspect the failed target before choosing roll-forward",
        ));
    }

    match (
        step.operation,
        command_step_collection(step),
        command_step_target(step),
    ) {
        (Operation::Rollback, Some("snapshots"), Some(target)) if is_zfs_snapshot_name(target) => {
            if let Some(dataset) = target.split_once('@').map(|(dataset, _)| dataset) {
                commands.push(command_vec(
                    ["zfs", "list", "-H", "-p", dataset],
                    false,
                    "inspect the dataset that would be rolled forward or retried",
                ));
                commands.push(command_vec(
                    [
                        "zfs",
                        "list",
                        "-t",
                        "snapshot",
                        "-H",
                        "-p",
                        "-o",
                        "name,creation,used,referenced,userrefs",
                        "-r",
                        dataset,
                    ],
                    false,
                    "inspect newer snapshots before completing rollback or choosing roll-forward",
                ));
            }
        }
        (Operation::Rollback, Some("lvmSnapshots"), Some(target)) => {
            commands.push(command_vec(
                ["lvs", "--reportformat", "json", "-a", target],
                false,
                "inspect LVM origin, snapshot, and merge state before roll-forward",
            ));
        }
        (Operation::Create | Operation::Grow | Operation::Rescan, Some("partitions"), _)
        | (Operation::Create | Operation::Rescan, Some("disks"), _) => {
            commands.extend(partition_recovery_inspection_commands(
                step,
                "inspect partition table state before choosing roll-forward",
            ))
        }
        (
            Operation::Create
            | Operation::Destroy
            | Operation::Export
            | Operation::Rescan
            | Operation::SetProperty
            | Operation::Unexport,
            Some("exports"),
            _,
        )
        | (
            Operation::Create
            | Operation::Destroy
            | Operation::Mount
            | Operation::Remount
            | Operation::Rescan
            | Operation::Unmount,
            Some("nfs.mounts"),
            _,
        ) => commands.extend(nfs_recovery_inspection_commands(
            step,
            "inspect NFS state before choosing roll-forward",
        )),
        (Operation::Create | Operation::Grow | Operation::Rescan, Some("backingFiles"), _)
        | (
            Operation::Create | Operation::Destroy | Operation::Grow | Operation::Rescan,
            Some("loopDevices"),
            _,
        )
        | (Operation::Destroy | Operation::Rename | Operation::Rescan, Some("dmMaps"), _) => {
            commands.extend(local_mapping_recovery_inspection_commands(
                step,
                "inspect local mapping state before choosing roll-forward",
            ))
        }
        (
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
            Some("filesystems"),
            _,
        ) => commands.extend(filesystem_recovery_inspection_commands(
            step,
            "inspect filesystem state before choosing roll-forward",
        )),
        (
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
            Some("pools" | "datasets" | "zvols"),
            _,
        ) => commands.extend(zfs_recovery_inspection_commands(
            step,
            "inspect ZFS state before choosing roll-forward",
        )),
        (
            Operation::Clone
            | Operation::Create
            | Operation::Destroy
            | Operation::Rename
            | Operation::Rescan
            | Operation::SetProperty,
            Some("snapshots"),
            _,
        ) => commands.extend(snapshot_recovery_inspection_commands(
            step,
            "inspect snapshot state before choosing roll-forward",
        )),
        (
            Operation::RemoveDevice | Operation::ReplaceDevice,
            Some("volumeGroups"),
            Some(target),
        ) => {
            commands.push(command_vec(
                ["vgs", "--reportformat", "json", target],
                false,
                "inspect VG allocation and free space before completing device migration",
            ));
            commands.push(command_vec(
                ["pvs", "--reportformat", "json"],
                false,
                "inspect PV allocation before retrying pvmove, vgreduce, or replacement",
            ));
        }
        (
            Operation::AddDevice
            | Operation::Create
            | Operation::Destroy
            | Operation::RemoveDevice
            | Operation::ReplaceDevice
            | Operation::Rescan
            | Operation::SetProperty,
            Some("caches" | "lvmCaches"),
            _,
        ) => commands.extend(cache_recovery_inspection_commands(
            step,
            "inspect cache state before choosing roll-forward",
        )),
        (
            Operation::Create
            | Operation::Deactivate
            | Operation::Destroy
            | Operation::Format
            | Operation::Grow
            | Operation::Rescan
            | Operation::SetProperty,
            Some("swaps"),
            _,
        ) => commands.extend(swap_recovery_inspection_commands(
            step,
            "inspect swap state before choosing roll-forward",
        )),
        (
            Operation::Attach
            | Operation::Create
            | Operation::Destroy
            | Operation::Detach
            | Operation::Grow
            | Operation::Rescan
            | Operation::SetProperty,
            Some("targetLuns"),
            Some(target),
        ) => commands.extend(target_lun_recovery_inspection_commands(
            Some(target),
            "inspect target-side LUN provider and host-visible path state before choosing roll-forward",
        )),
        (Operation::Destroy | Operation::RemoveDevice | Operation::Detach, Some("luns"), _) => {
            commands.push(command_vec(
                ["disk-nix", "luns", "--json"],
                false,
                "inspect host-side LUN paths before completing detach or cleanup",
            ));
            commands.push(command_vec(
                ["multipath", "-ll"],
                false,
                "inspect multipath maps before retrying LUN path changes",
            ));
        }
        (
            Operation::Create | Operation::Destroy | Operation::Login | Operation::Logout,
            Some("iscsiSessions"),
            Some(target),
        ) => {
            commands.push(command_vec(
                ["iscsiadm", "--mode", "session"],
                false,
                "inspect active iSCSI sessions before choosing roll-forward",
            ));
            commands.push(command_vec(
                ["iscsiadm", "--mode", "node", "--targetname", target],
                false,
                "inspect iSCSI node records before retrying login or logout",
            ));
            commands.push(lsscsi_lun_inventory_command(
                "inspect host-visible LUN transport and size before retrying session changes",
            ));
            commands.push(command_vec(
                ["multipath", "-ll"],
                false,
                "inspect multipath maps before retrying iSCSI session changes",
            ));
        }
        (
            Operation::Create
            | Operation::Destroy
            | Operation::Grow
            | Operation::Attach
            | Operation::Detach,
            Some("nvmeNamespaces"),
            Some(target),
        ) => {
            commands.push(nvme_list_namespaces_command(
                Some(target),
                "inspect NVMe namespace inventory before completing namespace changes",
            ));
            commands.push(nvme_list_subsystems_command(
                "inspect NVMe subsystem and controller attachments before retrying namespace changes",
            ));
        }
        (
            Operation::Create
            | Operation::Destroy
            | Operation::Grow
            | Operation::SetProperty
            | Operation::Start
            | Operation::Stop,
            Some("vdoVolumes"),
            Some(target),
        ) => {
            commands.push(command_vec(
                ["vdo", "status", "--name", target],
                false,
                "inspect VDO volume status before choosing roll-forward",
            ));
            commands.push(command_vec(
                ["vdostats", "--human-readable", target],
                false,
                "inspect VDO utilization and savings counters before retrying",
            ));
            commands.push(command_vec(
                ["disk-nix", "vdo", "--json"],
                false,
                "inspect modeled VDO inventory before retrying lifecycle changes",
            ));
        }
        (
            Operation::AddDevice
            | Operation::Destroy
            | Operation::Grow
            | Operation::RemoveDevice
            | Operation::ReplaceDevice
            | Operation::Rescan,
            Some("multipathMaps"),
            Some(target),
        ) => {
            commands.push(command_vec(
                ["multipath", "-ll", target],
                false,
                "inspect multipath map paths, policy, and size before choosing roll-forward",
            ));
            commands.push(lsscsi_lun_inventory_command(
                "inspect host-visible SCSI path transport and size before retrying multipath changes",
            ));
            commands.push(command_vec(
                ["disk-nix", "multipath", "--json"],
                false,
                "inspect modeled multipath inventory before retrying lifecycle changes",
            ));
        }
        (
            Operation::Close
            | Operation::Create
            | Operation::Destroy
            | Operation::Format
            | Operation::Grow
            | Operation::Open
            | Operation::SetProperty,
            Some("luks.devices"),
            _,
        )
        | (
            Operation::AddKey
            | Operation::Create
            | Operation::Destroy
            | Operation::RemoveKey
            | Operation::SetProperty,
            Some("luksKeyslots"),
            _,
        )
        | (
            Operation::Create
            | Operation::Destroy
            | Operation::ImportToken
            | Operation::RemoveToken
            | Operation::SetProperty,
            Some("luksTokens"),
            _,
        ) => commands.extend(luks_recovery_inspection_commands(
            step,
            "inspect LUKS state before choosing roll-forward",
        )),
        (
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
            | Operation::Rescan
            | Operation::Shrink,
            Some("volumes" | "thinPools" | "physicalVolumes" | "volumeGroups"),
            _,
        ) => commands.extend(lvm_recovery_inspection_commands(
            step,
            "inspect LVM state before choosing roll-forward",
        )),
        (
            Operation::Stop
            | Operation::RemoveDevice
            | Operation::ReplaceDevice
            | Operation::AddDevice,
            Some("mdRaids"),
            Some(target),
        ) => {
            commands.push(command_vec(
                ["mdadm", "--detail", target],
                false,
                "inspect MD RAID array health before completing recovery",
            ));
            commands.push(command_vec(
                ["cat", "/proc/mdstat"],
                false,
                "inspect MD RAID sync, recovery, or reshape progress before retrying",
            ));
        }
        _ => {}
    }

    commands
}

fn domain_rollback_inspection_commands(step: &ExecutionStep) -> Vec<ExecutionCommand> {
    match (
        step.operation,
        command_step_collection(step),
        command_step_target(step),
    ) {
        (Operation::Rollback, Some("snapshots"), Some(target)) if is_zfs_snapshot_name(target) => {
            let mut commands = vec![command_vec(
                ["zfs", "list", "-t", "snapshot", "-H", "-p", target],
                false,
                "confirm the rollback point still exists before any retry",
            )];
            if let Some(dataset) = target.split_once('@').map(|(dataset, _)| dataset) {
                commands.push(command_vec(
                    ["zfs", "list", "-H", "-p", dataset],
                    false,
                    "inspect the dataset state that rollback would replace",
                ));
            }
            commands
        }
        (Operation::Rollback, Some("lvmSnapshots"), Some(target)) => vec![command_vec(
            ["lvs", "--reportformat", "json", "-a", target],
            false,
            "confirm the LVM snapshot and origin state before retrying merge rollback",
        )],
        (Operation::Create | Operation::Grow | Operation::Rescan, Some("partitions"), _)
        | (Operation::Create | Operation::Rescan, Some("disks"), _) => {
            partition_recovery_inspection_commands(
                step,
                "confirm partition table state before rollback decisions",
            )
        }
        (
            Operation::Create
            | Operation::Destroy
            | Operation::Export
            | Operation::Rescan
            | Operation::SetProperty
            | Operation::Unexport,
            Some("exports"),
            _,
        )
        | (
            Operation::Create
            | Operation::Destroy
            | Operation::Mount
            | Operation::Remount
            | Operation::Rescan
            | Operation::Unmount,
            Some("nfs.mounts"),
            _,
        ) => nfs_recovery_inspection_commands(step, "confirm NFS state before rollback decisions"),
        (Operation::Create | Operation::Grow | Operation::Rescan, Some("backingFiles"), _)
        | (
            Operation::Create | Operation::Destroy | Operation::Grow | Operation::Rescan,
            Some("loopDevices"),
            _,
        )
        | (Operation::Destroy | Operation::Rename | Operation::Rescan, Some("dmMaps"), _) => {
            local_mapping_recovery_inspection_commands(
                step,
                "confirm local mapping state before rollback decisions",
            )
        }
        (
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
            Some("filesystems"),
            _,
        ) => filesystem_recovery_inspection_commands(
            step,
            "confirm filesystem state before rollback decisions",
        ),
        (
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
            Some("pools" | "datasets" | "zvols"),
            _,
        ) => zfs_recovery_inspection_commands(step, "confirm ZFS state before rollback decisions"),
        (
            Operation::Clone
            | Operation::Create
            | Operation::Destroy
            | Operation::Rename
            | Operation::Rescan
            | Operation::SetProperty,
            Some("snapshots"),
            _,
        ) => snapshot_recovery_inspection_commands(
            step,
            "confirm snapshot state before rollback decisions",
        ),
        (
            Operation::Create
            | Operation::Destroy
            | Operation::Rename
            | Operation::Rescan
            | Operation::SetProperty,
            Some("btrfsSubvolumes"),
            Some(target),
        ) => vec![
            command_vec(
                ["btrfs", "subvolume", "show", target],
                false,
                "confirm Btrfs subvolume state before rollback decisions",
            ),
            command_vec(
                ["btrfs", "property", "get", "-ts", target, "ro"],
                false,
                "confirm Btrfs subvolume read-only state before rollback decisions",
            ),
        ],
        (
            Operation::Create | Operation::Destroy | Operation::Rescan | Operation::SetProperty,
            Some("btrfsQgroups"),
            Some(target),
        ) => vec![command_vec(
            ["btrfs", "qgroup", "show", "--raw", "-reF", target],
            false,
            "confirm Btrfs qgroup limits and usage before rollback decisions",
        )],
        (
            Operation::RemoveDevice | Operation::ReplaceDevice,
            Some("volumeGroups"),
            Some(target),
        ) => {
            vec![
                command_vec(
                    ["vgs", "--reportformat", "json", target],
                    false,
                    "confirm VG metadata before undoing a partially completed PV migration",
                ),
                command_vec(
                    ["pvs", "--reportformat", "json"],
                    false,
                    "confirm whether extents remain on the source or replacement PV",
                ),
            ]
        }
        (
            Operation::AddDevice
            | Operation::Create
            | Operation::Destroy
            | Operation::RemoveDevice
            | Operation::ReplaceDevice
            | Operation::Rescan
            | Operation::SetProperty,
            Some("caches" | "lvmCaches"),
            _,
        ) => cache_recovery_inspection_commands(
            step,
            "confirm cache state before rollback decisions",
        ),
        (
            Operation::Create
            | Operation::Deactivate
            | Operation::Destroy
            | Operation::Format
            | Operation::Grow
            | Operation::Rescan
            | Operation::SetProperty,
            Some("swaps"),
            _,
        ) => {
            swap_recovery_inspection_commands(step, "confirm swap state before rollback decisions")
        }
        (
            Operation::Attach
            | Operation::Create
            | Operation::Destroy
            | Operation::Detach
            | Operation::Grow
            | Operation::Rescan
            | Operation::SetProperty,
            Some("targetLuns"),
            Some(target),
        ) => target_lun_recovery_inspection_commands(
            Some(target),
            "confirm target-side LUN provider and host-visible path state before rollback decisions",
        ),
        (
            Operation::Attach
            | Operation::Create
            | Operation::Destroy
            | Operation::Detach
            | Operation::Grow
            | Operation::RemoveDevice
            | Operation::Rescan,
            Some("luns"),
            _,
        ) => vec![
            command_vec(
                ["disk-nix", "luns", "--json"],
                false,
                "confirm host-side LUN paths before rollback decisions",
            ),
            lsscsi_lun_inventory_command(
                "confirm host-visible LUN transport and size before rollback decisions",
            ),
            command_vec(
                ["multipath", "-ll"],
                false,
                "confirm path grouping before restoring or removing multipath maps",
            ),
        ],
        (
            Operation::Create | Operation::Destroy | Operation::Login | Operation::Logout,
            Some("iscsiSessions"),
            Some(target),
        ) => vec![
            command_vec(
                ["iscsiadm", "--mode", "session"],
                false,
                "confirm active iSCSI sessions before rollback decisions",
            ),
            command_vec(
                ["iscsiadm", "--mode", "node", "--targetname", target],
                false,
                "confirm iSCSI node records before undoing or retrying session changes",
            ),
            lsscsi_lun_inventory_command(
                "confirm host-visible LUN paths before rollback decisions",
            ),
            command_vec(
                ["multipath", "-ll"],
                false,
                "confirm multipath path grouping before rollback decisions",
            ),
        ],
        (
            Operation::Create
            | Operation::Destroy
            | Operation::Grow
            | Operation::Attach
            | Operation::Detach,
            Some("nvmeNamespaces"),
            Some(target),
        ) => vec![
            nvme_list_namespaces_command(
                Some(target),
                "confirm NVMe namespace inventory before undoing or retrying namespace changes",
            ),
            nvme_list_subsystems_command(
                "confirm NVMe subsystem attachments before rollback decisions",
            ),
        ],
        (
            Operation::Create
            | Operation::Destroy
            | Operation::Grow
            | Operation::SetProperty
            | Operation::Start
            | Operation::Stop,
            Some("vdoVolumes"),
            Some(target),
        ) => vec![
            command_vec(
                ["vdo", "status", "--name", target],
                false,
                "confirm VDO status before undoing or retrying lifecycle changes",
            ),
            command_vec(
                ["vdostats", "--human-readable", target],
                false,
                "confirm VDO utilization and savings counters before rollback decisions",
            ),
            command_vec(
                ["disk-nix", "vdo", "--json"],
                false,
                "confirm modeled VDO inventory before rollback decisions",
            ),
        ],
        (
            Operation::AddDevice
            | Operation::Destroy
            | Operation::Grow
            | Operation::RemoveDevice
            | Operation::ReplaceDevice
            | Operation::Rescan,
            Some("multipathMaps"),
            Some(target),
        ) => vec![
            command_vec(
                ["multipath", "-ll", target],
                false,
                "confirm multipath map paths, policy, and size before rollback decisions",
            ),
            lsscsi_lun_inventory_command(
                "confirm host-visible SCSI paths before rollback decisions",
            ),
            command_vec(
                ["disk-nix", "multipath", "--json"],
                false,
                "confirm modeled multipath inventory before rollback decisions",
            ),
        ],
        (
            Operation::Close
            | Operation::Create
            | Operation::Destroy
            | Operation::Format
            | Operation::Grow
            | Operation::Open
            | Operation::SetProperty,
            Some("luks.devices"),
            _,
        )
        | (
            Operation::AddKey
            | Operation::Create
            | Operation::Destroy
            | Operation::RemoveKey
            | Operation::SetProperty,
            Some("luksKeyslots"),
            _,
        )
        | (
            Operation::Create
            | Operation::Destroy
            | Operation::ImportToken
            | Operation::RemoveToken
            | Operation::SetProperty,
            Some("luksTokens"),
            _,
        ) => {
            luks_recovery_inspection_commands(step, "confirm LUKS state before rollback decisions")
        }
        (
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
            | Operation::Rescan
            | Operation::Shrink,
            Some("volumes" | "thinPools" | "physicalVolumes" | "volumeGroups"),
            _,
        ) => lvm_recovery_inspection_commands(step, "confirm LVM state before rollback decisions"),
        (
            Operation::Stop
            | Operation::RemoveDevice
            | Operation::ReplaceDevice
            | Operation::AddDevice,
            Some("mdRaids"),
            Some(target),
        ) => vec![
            command_vec(
                ["mdadm", "--detail", target],
                false,
                "confirm MD RAID array health before undoing or retrying member changes",
            ),
            command_vec(
                ["cat", "/proc/mdstat"],
                false,
                "confirm sync, recovery, or reshape state before rollback decisions",
            ),
        ],
        _ => Vec::new(),
    }
}

fn domain_recovery_commands(step: &ExecutionStep) -> Vec<ExecutionCommand> {
    let mut commands = Vec::new();
    match (
        step.operation,
        command_step_collection(step),
        command_step_target(step),
    ) {
        (Operation::Rollback, Some("snapshots"), Some(target)) if is_zfs_snapshot_name(target) => {
            commands.push(command(
                ["zfs", "list", "-t", "snapshot", "-H", "-p", target],
                false,
                "inspect the rollback snapshot before deciding whether to retry or roll forward",
            ));
            if let Some(dataset) = target.split_once('@').map(|(dataset, _)| dataset) {
                commands.push(command(
                    ["zfs", "list", "-H", "-p", dataset],
                    false,
                    "inspect the rolled-back dataset state after the failed rollback attempt",
                ));
            }
        }
        (Operation::Rollback, Some("lvmSnapshots"), Some(target)) => {
            commands.push(command(
                ["lvs", "--reportformat", "json", target],
                false,
                "inspect LVM snapshot and merge state before deciding whether to retry",
            ));
        }
        (Operation::Create | Operation::Grow | Operation::Rescan, Some("partitions"), _)
        | (Operation::Create | Operation::Rescan, Some("disks"), _) => {
            commands.extend(partition_recovery_inspection_commands(
                step,
                "inspect partition table state after the failed command",
            ))
        }
        (
            Operation::Create
            | Operation::Destroy
            | Operation::Export
            | Operation::Rescan
            | Operation::SetProperty
            | Operation::Unexport,
            Some("exports"),
            _,
        )
        | (
            Operation::Create
            | Operation::Destroy
            | Operation::Mount
            | Operation::Remount
            | Operation::Rescan
            | Operation::Unmount,
            Some("nfs.mounts"),
            _,
        ) => commands.extend(nfs_recovery_inspection_commands(
            step,
            "inspect NFS state after the failed command",
        )),
        (Operation::Create | Operation::Grow | Operation::Rescan, Some("backingFiles"), _)
        | (
            Operation::Create | Operation::Destroy | Operation::Grow | Operation::Rescan,
            Some("loopDevices"),
            _,
        )
        | (Operation::Destroy | Operation::Rename | Operation::Rescan, Some("dmMaps"), _) => {
            commands.extend(local_mapping_recovery_inspection_commands(
                step,
                "inspect local mapping state after the failed command",
            ))
        }
        (
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
            Some("filesystems"),
            _,
        ) => commands.extend(filesystem_recovery_inspection_commands(
            step,
            "inspect filesystem state after the failed command",
        )),
        (
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
            Some("pools" | "datasets" | "zvols"),
            _,
        ) => commands.extend(zfs_recovery_inspection_commands(
            step,
            "inspect ZFS state after the failed command",
        )),
        (
            Operation::Clone
            | Operation::Create
            | Operation::Destroy
            | Operation::Rename
            | Operation::Rescan
            | Operation::SetProperty,
            Some("snapshots"),
            _,
        ) => commands.extend(snapshot_recovery_inspection_commands(
            step,
            "inspect snapshot state after the failed command",
        )),
        (
            Operation::Create | Operation::Destroy | Operation::Login | Operation::Logout,
            Some("iscsiSessions"),
            Some(target),
        ) => {
            commands.push(command(
                ["iscsiadm", "--mode", "session"],
                false,
                "inspect active iSCSI sessions before deciding whether to retry",
            ));
            commands.push(command(
                ["iscsiadm", "--mode", "node", "--targetname", target],
                false,
                "inspect iSCSI node records after the failed session command",
            ));
            commands.push(lsscsi_lun_inventory_command(
                "inspect host-visible LUN paths after the failed session command",
            ));
            commands.push(command(
                ["multipath", "-ll"],
                false,
                "inspect multipath maps after the failed session command",
            ));
        }
        (
            Operation::Attach
            | Operation::Create
            | Operation::Destroy
            | Operation::Detach
            | Operation::Grow
            | Operation::Rescan
            | Operation::SetProperty,
            Some("targetLuns"),
            Some(target),
        ) => commands.extend(target_lun_recovery_inspection_commands(
            Some(target),
            "inspect target-side LUN provider and host-visible path state after the failed command",
        )),
        (
            Operation::Create
            | Operation::Destroy
            | Operation::Grow
            | Operation::Attach
            | Operation::Detach,
            Some("nvmeNamespaces"),
            Some(target),
        ) => {
            commands.push(nvme_list_namespaces_command(
                Some(target),
                "inspect NVMe namespace inventory before deciding whether to retry",
            ));
            commands.push(nvme_list_subsystems_command(
                "inspect NVMe subsystem attachments after the failed namespace command",
            ));
        }
        (
            Operation::AddDevice
            | Operation::Create
            | Operation::Destroy
            | Operation::RemoveDevice
            | Operation::ReplaceDevice
            | Operation::Rescan
            | Operation::SetProperty,
            Some("caches" | "lvmCaches"),
            _,
        ) => commands.extend(cache_recovery_inspection_commands(
            step,
            "inspect cache state after the failed command",
        )),
        (
            Operation::Create
            | Operation::Deactivate
            | Operation::Destroy
            | Operation::Format
            | Operation::Grow
            | Operation::Rescan
            | Operation::SetProperty,
            Some("swaps"),
            _,
        ) => commands.extend(swap_recovery_inspection_commands(
            step,
            "inspect swap state after the failed command",
        )),
        (
            Operation::Create
            | Operation::Destroy
            | Operation::Grow
            | Operation::SetProperty
            | Operation::Start
            | Operation::Stop,
            Some("vdoVolumes"),
            Some(target),
        ) => {
            commands.push(command(
                ["vdo", "status", "--name", target],
                false,
                "inspect VDO volume status after the failed lifecycle command",
            ));
            commands.push(command(
                ["vdostats", "--human-readable", target],
                false,
                "inspect VDO utilization and savings counters after the failed lifecycle command",
            ));
            commands.push(command(
                ["disk-nix", "vdo", "--json"],
                false,
                "inspect modeled VDO inventory before deciding whether to retry",
            ));
        }
        (
            Operation::AddDevice
            | Operation::Destroy
            | Operation::Grow
            | Operation::RemoveDevice
            | Operation::ReplaceDevice
            | Operation::Rescan,
            Some("multipathMaps"),
            Some(target),
        ) => {
            commands.push(command(
                ["multipath", "-ll", target],
                false,
                "inspect multipath map paths, policy, and size after the failed command",
            ));
            commands.push(lsscsi_lun_inventory_command(
                "inspect host-visible SCSI paths after the failed multipath command",
            ));
            commands.push(command(
                ["disk-nix", "multipath", "--json"],
                false,
                "inspect modeled multipath inventory before deciding whether to retry",
            ));
        }
        (
            Operation::Close
            | Operation::Create
            | Operation::Destroy
            | Operation::Format
            | Operation::Grow
            | Operation::Open
            | Operation::SetProperty,
            Some("luks.devices"),
            _,
        )
        | (
            Operation::AddKey
            | Operation::Create
            | Operation::Destroy
            | Operation::RemoveKey
            | Operation::SetProperty,
            Some("luksKeyslots"),
            _,
        )
        | (
            Operation::Create
            | Operation::Destroy
            | Operation::ImportToken
            | Operation::RemoveToken
            | Operation::SetProperty,
            Some("luksTokens"),
            _,
        ) => commands.extend(luks_recovery_inspection_commands(
            step,
            "inspect LUKS state after the failed command",
        )),
        (
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
            | Operation::Rescan
            | Operation::Shrink,
            Some("volumes" | "thinPools" | "physicalVolumes" | "volumeGroups"),
            _,
        ) => commands.extend(lvm_recovery_inspection_commands(
            step,
            "inspect LVM state after the failed command",
        )),
        (
            Operation::Stop
            | Operation::RemoveDevice
            | Operation::ReplaceDevice
            | Operation::AddDevice,
            Some("mdRaids"),
            Some(target),
        ) => {
            commands.push(command(
                ["mdadm", "--detail", target],
                false,
                "inspect MD RAID member, failed, spare, and recovery state before deciding whether to retry",
            ));
            commands.push(command(
                ["cat", "/proc/mdstat"],
                false,
                "inspect MD RAID runtime recovery or reshape state after the failed command",
            ));
        }
        (_, _, Some(target)) => {
            commands.push(command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "inspect the failed action target before choosing rollback or roll-forward",
            ));
        }
        _ => {}
    }
    commands.extend(state_inspection_commands());
    commands
}

fn domain_recovery_notes(
    step: &ExecutionStep,
    failed: &ExecutionCommandResult,
    completed_mutating_commands: usize,
) -> Vec<String> {
    let mut notes = vec![
        format!(
            "{completed_mutating_commands} mutating command(s) completed before the failed command in this apply run"
        ),
        format!(
            "failed {:?} command for {}: {}",
            failed.phase,
            failed.action_id,
            failed.argv.join(" ")
        ),
        "do not retry the failed action until fresh topology proves whether the target already changed".to_string(),
    ];

    match (step.operation, command_step_collection(step)) {
        (Operation::Rollback, Some("snapshots")) => {
            notes.push(
                "for ZFS rollback, prefer cloning the snapshot or taking a fresh snapshot of the current dataset before any retry".to_string(),
            );
            notes.push(
                "review newer snapshots, clones, mountpoints, shares, and dependent services before choosing rollback or roll-forward".to_string(),
            );
        }
        (
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
            Some("pools" | "datasets" | "zvols"),
        ) => {
            notes.push(
                "for ZFS changes, inspect pool health, dataset or zvol properties, snapshots, clones, mountpoints, shares, and LUN consumers before retrying".to_string(),
            );
            notes.push(
                "prefer read-only import, clone, or fresh snapshot workflows until pool state and dependent services match the intended topology".to_string(),
            );
        }
        (
            Operation::Clone
            | Operation::Create
            | Operation::Destroy
            | Operation::Rename
            | Operation::Rescan
            | Operation::SetProperty,
            Some("snapshots"),
        ) => {
            notes.push(
                "for snapshot lifecycle changes, inspect source, target, hold tags, read-only state, and dependent clones before retrying".to_string(),
            );
            notes.push(
                "prefer preserving or cloning recovery snapshots until retention, rollback, replication, and mount consumers are verified".to_string(),
            );
        }
        (Operation::Rollback, Some("lvmSnapshots")) => {
            notes.push(
                "for LVM snapshot merge rollback, inspect origin activation and merge status before rerunning lvconvert --merge".to_string(),
            );
            notes.push(
                "keep the origin, snapshot, and VG metadata backups intact until the merge outcome is verified".to_string(),
            );
        }
        (Operation::Create | Operation::Grow | Operation::Rescan, Some("partitions"))
        | (Operation::Create | Operation::Rescan, Some("disks")) => {
            notes.push(
                "for partition-table changes, inspect disk identity, partition geometry, kernel reread state, and dependent LUKS, LVM, filesystem, and mount consumers before retrying".to_string(),
            );
            notes.push(
                "preserve partition table captures and avoid formatting or resizing upper layers until the kernel and modeled topology agree on the new geometry".to_string(),
            );
        }
        (
            Operation::Create
            | Operation::Destroy
            | Operation::Export
            | Operation::Rescan
            | Operation::SetProperty
            | Operation::Unexport,
            Some("exports"),
        )
        | (
            Operation::Create
            | Operation::Destroy
            | Operation::Mount
            | Operation::Remount
            | Operation::Rescan
            | Operation::Unmount,
            Some("nfs.mounts"),
        ) => {
            notes.push(
                "for NFS changes, inspect exported paths, client selectors, negotiated mount options, mount state, and dependent services before retrying".to_string(),
            );
            notes.push(
                "keep local services quiesced and preserve declarative export or mount configuration until live NFS state matches the intended topology".to_string(),
            );
        }
        (Operation::Create | Operation::Grow | Operation::Rescan, Some("backingFiles"))
        | (
            Operation::Create | Operation::Destroy | Operation::Grow | Operation::Rescan,
            Some("loopDevices"),
        )
        | (Operation::Destroy | Operation::Rename | Operation::Rescan, Some("dmMaps")) => {
            notes.push(
                "for local mapping changes, inspect backing file size, loop mappings, device-mapper tables, dependencies, and modeled consumers before retrying".to_string(),
            );
            notes.push(
                "prefer refreshing or repairing the owning LUKS, LVM, VDO, multipath, cache, or filesystem layer before forcing generic map removal or rename retries".to_string(),
            );
        }
        (
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
            Some("filesystems"),
        ) => {
            notes.push(
                "for filesystem changes, inspect mount state, source device signatures, usage, labels, UUIDs, and dependent services before retrying".to_string(),
            );
            notes.push(
                "prefer snapshots, read-only mounts, or cloned-device repair workflows before destructive format, shrink, repair, or device-removal retries".to_string(),
            );
        }
        (
            Operation::Create | Operation::Destroy | Operation::Login | Operation::Logout,
            Some("iscsiSessions"),
        ) => {
            notes.push(
                "for iSCSI session changes, inspect active sessions, node records, LUN paths, and multipath maps before retrying login or logout".to_string(),
            );
            notes.push(
                "keep dependent filesystems, LVM stacks, and services stopped or migrated until host-visible paths match the intended session state".to_string(),
            );
        }
        (
            Operation::Attach
            | Operation::Create
            | Operation::Destroy
            | Operation::Detach
            | Operation::Grow
            | Operation::Rescan
            | Operation::SetProperty,
            Some("targetLuns"),
        ) => {
            notes.push(
                "for target-side LUN changes, inspect provider inventory, target mappings, host-visible SCSI paths, and multipath maps before retrying".to_string(),
            );
            notes.push(
                "stage host-side luns, iSCSI sessions, and multipath rescans only after the target reports the intended mapping and capacity".to_string(),
            );
        }
        (
            Operation::Create
            | Operation::Destroy
            | Operation::Grow
            | Operation::Attach
            | Operation::Detach,
            Some("nvmeNamespaces"),
        ) => {
            notes.push(
                "for NVMe namespace changes, inspect namespace inventory and subsystem attachments before retrying create, grow/rescan, attach, detach, or delete operations".to_string(),
            );
            notes.push(
                "keep dependent filesystems, multipath maps, and consumers quiesced until namespace visibility and attachment state are verified".to_string(),
            );
        }
        (
            Operation::Create
            | Operation::Destroy
            | Operation::Grow
            | Operation::SetProperty
            | Operation::Start
            | Operation::Stop,
            Some("vdoVolumes"),
        ) => {
            notes.push(
                "for VDO lifecycle changes, inspect status, utilization, operating mode, and backing storage before retrying create, grow, start, stop, or removal".to_string(),
            );
            notes.push(
                "keep dependent filesystems, LVM layers, and services inactive until VDO mode and capacity match the intended topology".to_string(),
            );
        }
        (
            Operation::AddDevice
            | Operation::Destroy
            | Operation::Grow
            | Operation::RemoveDevice
            | Operation::ReplaceDevice
            | Operation::Rescan,
            Some("multipathMaps"),
        ) => {
            notes.push(
                "for multipath changes, inspect path grouping, SCSI path state, map size, and modeled consumers before retrying reload, resize, path add, path removal, or flush operations".to_string(),
            );
            notes.push(
                "keep dependent filesystems, LVM layers, and services inactive or migrated until every expected path reports the intended map state".to_string(),
            );
        }
        (
            Operation::AddDevice
            | Operation::Create
            | Operation::Destroy
            | Operation::RemoveDevice
            | Operation::ReplaceDevice
            | Operation::Rescan
            | Operation::SetProperty,
            Some("caches" | "lvmCaches"),
        ) => {
            notes.push(
                "for cache changes, inspect dirty-data, cache mode, attachment, and backing volume state before retrying attach, detach, replacement, or property updates".to_string(),
            );
            notes.push(
                "prefer writethrough or clean-cache state before detaching, replacing, or disabling writeback cache layers".to_string(),
            );
        }
        (
            Operation::Create
            | Operation::Deactivate
            | Operation::Destroy
            | Operation::Format
            | Operation::Grow
            | Operation::Rescan
            | Operation::SetProperty,
            Some("swaps"),
        ) => {
            notes.push(
                "for swap changes, inspect active swapon output, signature metadata, resume references, and backing storage before retrying format, resize, property, or teardown operations".to_string(),
            );
            notes.push(
                "prefer adding temporary swap capacity before disabling or recreating active swap on memory-constrained systems".to_string(),
            );
        }
        (
            Operation::Close
            | Operation::Create
            | Operation::Destroy
            | Operation::Format
            | Operation::Grow
            | Operation::Open
            | Operation::SetProperty,
            Some("luks.devices"),
        )
        | (
            Operation::AddKey
            | Operation::Create
            | Operation::Destroy
            | Operation::RemoveKey
            | Operation::SetProperty,
            Some("luksKeyslots"),
        )
        | (
            Operation::Create
            | Operation::Destroy
            | Operation::ImportToken
            | Operation::RemoveToken
            | Operation::SetProperty,
            Some("luksTokens"),
        ) => {
            notes.push(
                "for LUKS changes, inspect mapper status, header metadata, keyslots, tokens, and dependent consumers before retrying encryption operations".to_string(),
            );
            notes.push(
                "keep header backups and alternate unlock paths available until the mapper and header metadata match the intended state".to_string(),
            );
        }
        (
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
            | Operation::Rescan
            | Operation::Shrink,
            Some("volumes" | "thinPools" | "physicalVolumes" | "volumeGroups"),
        ) => {
            notes.push(
                "for LVM changes, inspect LV, PV, and VG metadata before retrying activation, resize, rename, import, export, create, or removal operations".to_string(),
            );
            notes.push(
                "keep dependent filesystems, encryption layers, and services inactive until LVM metadata and activation state match the intended topology".to_string(),
            );
        }
        (
            Operation::Stop
            | Operation::RemoveDevice
            | Operation::ReplaceDevice
            | Operation::AddDevice,
            Some("mdRaids"),
        ) => {
            notes.push(
                "for MD RAID member changes, inspect mdadm detail and /proc/mdstat before retrying; do not remove old members until sync or replacement state is understood".to_string(),
            );
            notes.push(
                "keep failed, old, and replacement devices attached until redundancy and array metadata are verified".to_string(),
            );
        }
        (Operation::RemoveDevice | Operation::Destroy | Operation::Detach, _) => {
            notes.push(
                "verify consumers, redundancy, and metadata health before retrying teardown or device removal".to_string(),
            );
            notes.push(
                "prefer roll-forward repair of the partially changed topology over blind rollback when data placement may have moved".to_string(),
            );
        }
        _ => {
            notes.push(
                "choose rollback only when domain-specific tooling proves it is safer than completing the remaining plan".to_string(),
            );
        }
    }
    notes
}

fn command_step_collection(step: &ExecutionStep) -> Option<&str> {
    step.action_id
        .split(':')
        .next()
        .map(|collection| match collection {
            "snapshot" => "snapshots",
            "filesystem" => "filesystems",
            "backingfiles" => "backingFiles",
            "btrfsqgroups" => "btrfsQgroups",
            "btrfssubvolumes" => "btrfsSubvolumes",
            "dmmaps" => "dmMaps",
            "iscsisessions" => "iscsiSessions",
            "loopdevices" => "loopDevices",
            "lvmcaches" => "lvmCaches",
            "lukskeyslots" => "luksKeyslots",
            "lukstokens" => "luksTokens",
            "multipathmaps" => "multipathMaps",
            "nvmenamespaces" => "nvmeNamespaces",
            "physicalvolumes" => "physicalVolumes",
            "targetLuns" | "targetluns" => "targetLuns",
            "thinpools" => "thinPools",
            "volumegroups" => "volumeGroups",
            "vdovolumes" => "vdoVolumes",
            "zvols" => "zvols",
            other => other,
        })
}

fn command_step_target(step: &ExecutionStep) -> Option<&str> {
    if command_step_collection(step) == Some("mdRaids") {
        if let Some(target) = md_array_target_from_step(step) {
            return Some(target);
        }
    }
    if command_step_collection(step) == Some("nvmeNamespaces") {
        if let Some(target) = nvme_controller_target_from_step(step) {
            return Some(target);
        }
    }
    if command_step_collection(step) == Some("iscsiSessions") {
        if let Some(target) = iscsi_target_from_step(step) {
            return Some(target);
        }
    }
    if command_step_collection(step) == Some("targetLuns") {
        if let Some(target) = target_lun_target_from_step(step) {
            return Some(target);
        }
    }
    if command_step_collection(step) == Some("multipathMaps") {
        if let Some(target) = multipath_map_target_from_step(step) {
            return Some(target);
        }
    }
    if matches!(
        command_step_collection(step),
        Some("volumes" | "thinPools" | "physicalVolumes" | "volumeGroups")
    ) {
        if let Some(target) = lvm_target_from_step(step) {
            return Some(target);
        }
    }
    if command_step_collection(step) == Some("snapshots") {
        if let Some(target) = snapshot_target_from_step(step) {
            return Some(target);
        }
    }
    if command_step_collection(step) == Some("btrfsSubvolumes") {
        if let Some(target) = btrfs_subvolume_target_from_step(step) {
            return Some(target);
        }
    }
    if command_step_collection(step) == Some("btrfsQgroups") {
        if let Some(target) = btrfs_qgroup_target_from_step(step) {
            return Some(target);
        }
    }
    if matches!(command_step_collection(step), Some("caches" | "lvmCaches")) {
        if let Some(target) = cache_target_from_step(step) {
            return Some(target);
        }
    }
    if command_step_collection(step) == Some("swaps") {
        if let Some(target) = swap_target_from_step(step) {
            return Some(target);
        }
    }
    if matches!(
        command_step_collection(step),
        Some("pools" | "datasets" | "zvols")
    ) {
        if let Some(target) = zfs_target_from_step(step) {
            return Some(target);
        }
    }
    if command_step_collection(step) == Some("filesystems") {
        if let Some(target) = filesystem_target_from_step(step) {
            return Some(target);
        }
    }
    if matches!(command_step_collection(step), Some("disks" | "partitions")) {
        if let Some(target) =
            partition_disk_from_step(step).or_else(|| partition_target_from_step(step))
        {
            return Some(target);
        }
    }
    if matches!(
        command_step_collection(step),
        Some("backingFiles" | "loopDevices" | "dmMaps")
    ) {
        if let Some(target) = local_mapping_target_from_step(step) {
            return Some(target);
        }
    }
    if matches!(
        command_step_collection(step),
        Some("exports" | "nfs.mounts")
    ) {
        if let Some(target) = nfs_target_from_step(step) {
            return Some(target);
        }
    }
    step.action_id
        .split(':')
        .nth(1)
        .filter(|target| !target.is_empty())
}

fn md_array_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command.argv.first().is_some_and(|arg| arg == "mdadm") {
            if command
                .argv
                .get(1)
                .is_some_and(|arg| arg == "--detail" || arg == "--stop")
            {
                return command.argv.get(2).map(String::as_str);
            }
            if command
                .argv
                .get(1)
                .is_some_and(|arg| arg.starts_with("/dev/md"))
            {
                return command.argv.get(1).map(String::as_str);
            }
        }
        None
    })
}

fn multipath_map_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command.argv.first().is_some_and(|arg| arg == "multipath")
            && command
                .argv
                .get(1)
                .is_some_and(|arg| arg == "-ll" || arg == "-f")
        {
            return command.argv.get(2).map(String::as_str);
        }
        if command.argv.first().is_some_and(|arg| arg == "multipathd")
            && command
                .argv
                .get(1..3)
                .is_some_and(|args| args == ["resize", "map"])
        {
            return command.argv.get(3).map(String::as_str);
        }
        None
    })
}

fn luks_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    let mut commands = Vec::new();
    if let Some(device) = luks_device_from_step(step) {
        commands.push(command(["cryptsetup", "luksDump", device], false, note));
        commands.push(command(
            ["disk-nix", "inspect", device, "--json"],
            false,
            note,
        ));
    }
    if let Some(mapper) = luks_mapper_from_step(step) {
        commands.push(command(["cryptsetup", "status", mapper], false, note));
        commands.push(command(
            ["disk-nix", "inspect", mapper, "--json"],
            false,
            note,
        ));
    }
    commands
}

fn luks_mapper_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command.argv.first().is_none_or(|arg| arg != "cryptsetup") {
            return None;
        }
        match command.argv.get(1).map(String::as_str) {
            Some("close" | "resize" | "status") => command.argv.get(2).map(String::as_str),
            Some("open") => command.argv.get(3).map(String::as_str),
            _ => None,
        }
    })
}

fn luks_device_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command.argv.first().is_none_or(|arg| arg != "cryptsetup") {
            return None;
        }
        match command.argv.get(1).map(String::as_str) {
            Some("isLuks" | "luksDump" | "luksFormat" | "luksKillSlot" | "luksUUID") => {
                command.argv.get(2).map(String::as_str)
            }
            Some("open") => command.argv.get(2).map(String::as_str),
            Some("config" | "luksAddKey" | "luksChangeKey") => {
                cryptsetup_positional_arg(command, 0)
            }
            Some("token") => command.argv.last().map(String::as_str),
            _ => None,
        }
        .filter(|target| !target.starts_with('<'))
    })
}

fn cryptsetup_positional_arg(command: &ExecutionCommand, index: usize) -> Option<&str> {
    let mut skip_next = false;
    let mut position = 0;
    for arg in command.argv.iter().skip(2) {
        if skip_next {
            skip_next = false;
            continue;
        }
        if matches!(
            arg.as_str(),
            "--key-file"
                | "--key-slot"
                | "--json-file"
                | "--priority"
                | "--subsystem"
                | "--token-id"
                | "--uuid"
        ) {
            skip_next = true;
            continue;
        }
        if arg.starts_with('-') {
            continue;
        }
        if position == index {
            return Some(arg.as_str());
        }
        position += 1;
    }
    None
}

fn lvm_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    let mut commands = Vec::new();
    match command_step_collection(step) {
        Some("physicalVolumes") => {
            if let Some(target) = lvm_target_from_step(step).or_else(|| {
                step.action_id
                    .split(':')
                    .nth(1)
                    .filter(|target| !target.is_empty())
            }) {
                commands.push(command(
                    ["pvs", "--reportformat", "json", target],
                    false,
                    note,
                ));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            }
            commands.push(command(["vgs", "--reportformat", "json"], false, note));
            commands.push(command(["lvs", "--reportformat", "json"], false, note));
        }
        Some("volumes" | "thinPools") => {
            if let Some(target) = lvm_target_from_step(step).or_else(|| {
                step.action_id
                    .split(':')
                    .nth(1)
                    .filter(|target| !target.is_empty())
            }) {
                commands.push(command(
                    ["lvs", "--reportformat", "json", target],
                    false,
                    note,
                ));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            }
            commands.push(command(["vgs", "--reportformat", "json"], false, note));
            commands.push(command(["pvs", "--reportformat", "json"], false, note));
        }
        Some("volumeGroups") => {
            if let Some(target) = lvm_target_from_step(step).or_else(|| {
                step.action_id
                    .split(':')
                    .nth(1)
                    .filter(|target| !target.is_empty())
            }) {
                commands.push(command(
                    ["vgs", "--reportformat", "json", target],
                    false,
                    note,
                ));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            }
            commands.push(command(["pvs", "--reportformat", "json"], false, note));
            commands.push(command(
                ["lvs", "--reportformat", "json", "-a"],
                false,
                note,
            ));
        }
        _ => {}
    }
    commands
}

fn lvm_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        match tool {
            "lvs" | "pvs" | "vgs" if command.argv.len() > 3 => {
                command.argv.last().map(String::as_str)
            }
            "lvchange" | "lvextend" | "lvremove" | "lvreduce" => {
                command.argv.last().map(String::as_str)
            }
            "lvrename" => command.argv.get(1).map(String::as_str),
            "pvcreate" | "pvremove" | "pvresize" => command.argv.last().map(String::as_str),
            "pvscan" if command.argv.len() > 2 => command.argv.last().map(String::as_str),
            "vgchange" | "vgexport" | "vgimport" | "vgremove" => {
                command.argv.last().map(String::as_str)
            }
            "vgcreate" | "vgextend" | "vgreduce" | "vgrename" => {
                command.argv.get(1).map(String::as_str)
            }
            _ => None,
        }
        .filter(|target| !target.starts_with('<'))
    })
}

fn cache_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    let mut commands = Vec::new();
    let target = cache_target_from_step(step).or_else(|| {
        step.action_id
            .split(':')
            .nth(1)
            .filter(|target| !target.is_empty())
    });

    match command_step_collection(step) {
        Some("lvmCaches") => {
            if let Some(target) = target {
                commands.push(command(
                    [
                        "lvs",
                        "--reportformat",
                        "json",
                        "-a",
                        "-o",
                        "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent",
                        target,
                    ],
                    false,
                    note,
                ));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            }
            commands.push(command(["vgs", "--reportformat", "json"], false, note));
            commands.push(command(["pvs", "--reportformat", "json"], false, note));
        }
        Some("caches") => {
            if let Some(target) = target {
                commands.push(bcache_sysfs_read_command(target, "state", note));
                commands.push(bcache_sysfs_read_command(target, "cache_mode", note));
                commands.push(bcache_sysfs_read_command(target, "dirty_data", note));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            }
            commands.push(command(["disk-nix", "cache", "--json"], false, note));
        }
        _ => {}
    }
    commands
}

fn cache_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        match tool {
            "sh" => command
                .argv
                .get(3)
                .filter(|wrapper| wrapper.starts_with("disk-nix-bcache-"))
                .and_then(|_| command.argv.get(4))
                .map(String::as_str),
            "lvchange" | "lvconvert" => command.argv.last().map(String::as_str),
            "lvs" if command.argv.len() > 3 => command.argv.last().map(String::as_str),
            _ => None,
        }
        .filter(|target| !target.starts_with('<'))
    })
}

fn swap_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    let mut commands = vec![command(
        ["swapon", "--show", "--bytes", "--raw"],
        false,
        note,
    )];
    if let Some(target) = swap_target_from_step(step).or_else(|| {
        step.action_id
            .split(':')
            .nth(1)
            .filter(|target| target.starts_with('/'))
    }) {
        commands.push(command(["blkid", target], false, note));
        commands.push(command(
            ["disk-nix", "inspect", target, "--json"],
            false,
            note,
        ));
    } else {
        commands.push(command(
            ["disk-nix", "swap", "--json"],
            false,
            "inspect modeled swap inventory before retrying",
        ));
    }
    commands
}

fn swap_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        match tool {
            "fallocate" | "mkswap" | "swaplabel" | "swapoff" | "wipefs" => {
                command.argv.last().map(String::as_str)
            }
            "swapon" if command.argv.get(1).is_none_or(|arg| arg != "--show") => {
                command.argv.last().map(String::as_str)
            }
            "sh" if command.argv.get(1).is_some_and(|arg| arg == "-c") => {
                swap_target_from_shell(command.argv.get(2)?)
            }
            _ => None,
        }
        .filter(|target| target.starts_with('/') && !target.starts_with("<"))
    })
}

fn swap_target_from_shell(script: &str) -> Option<&str> {
    let target = script.strip_prefix("swapoff ")?.split_whitespace().next()?;
    target
        .trim_matches('\'')
        .trim_matches('"')
        .strip_prefix("\\")
        .or(Some(target.trim_matches('\'').trim_matches('"')))
}

fn zfs_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    let target = zfs_target_from_step(step).or_else(|| {
        step.action_id
            .split(':')
            .nth(1)
            .filter(|target| !target.is_empty())
    });
    let mut commands = Vec::new();

    match command_step_collection(step) {
        Some("pools") => {
            if let Some(target) = target {
                commands.push(command(["zpool", "status", "-P", target], false, note));
                commands.push(command(["zpool", "list", "-H", "-p", target], false, note));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            } else {
                commands.push(command(["zpool", "status", "-P"], false, note));
                commands.push(command(["zpool", "list", "-H", "-p"], false, note));
            }
        }
        Some("datasets") => {
            if let Some(target) = target {
                commands.push(command(
                    ["zfs", "list", "-H", "-p", "-t", "filesystem", target],
                    false,
                    note,
                ));
                commands.push(command(["zfs", "get", "all", target], false, note));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            } else {
                commands.push(command(
                    ["zfs", "list", "-H", "-p", "-t", "filesystem"],
                    false,
                    note,
                ));
            }
        }
        Some("zvols") => {
            if let Some(target) = target {
                commands.push(command(
                    ["zfs", "list", "-H", "-p", "-t", "volume", target],
                    false,
                    note,
                ));
                commands.push(command(["zfs", "get", "all", target], false, note));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            } else {
                commands.push(command(
                    ["zfs", "list", "-H", "-p", "-t", "volume"],
                    false,
                    note,
                ));
            }
        }
        _ => {}
    }

    commands
}

fn zfs_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        let target = match tool {
            "zpool" => match command.argv.get(1).map(String::as_str) {
                Some("add" | "create" | "destroy" | "export" | "remove" | "replace" | "scrub") => {
                    command.argv.get(2).map(String::as_str)
                }
                Some("import") => command.argv.last().map(String::as_str),
                Some("set") => command.argv.get(3).map(String::as_str),
                Some("list" | "status" | "get") if command.argv.len() > 3 => {
                    command.argv.last().map(String::as_str)
                }
                _ => None,
            },
            "zfs" => match command.argv.get(1).map(String::as_str) {
                Some("create" | "destroy" | "get" | "promote" | "set") => {
                    command.argv.last().map(String::as_str)
                }
                Some("rename") => command.argv.get(2).map(String::as_str),
                Some("list") if command.argv.len() > 4 => command.argv.last().map(String::as_str),
                _ => None,
            },
            _ => None,
        }?;

        Some(target).filter(|target| {
            !target.is_empty()
                && !target.starts_with('-')
                && !target.starts_with('<')
                && *target != "import"
        })
    })
}

fn filesystem_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    let target = filesystem_target_from_step(step).or_else(|| {
        step.action_id
            .split(':')
            .nth(1)
            .filter(|target| !target.is_empty())
    });
    let source = filesystem_source_from_step(step);
    let mut commands = Vec::new();

    if let Some(mountpoint) = target.filter(|target| target.starts_with('/')) {
        commands.push(command(
            ["findmnt", "--json", "--target", mountpoint],
            false,
            note,
        ));
        commands.push(command(
            ["disk-nix", "inspect", mountpoint, "--json"],
            false,
            note,
        ));
    }

    if let Some(source) = source {
        commands.push(command(["blkid", source], false, note));
        commands.push(command(
            ["disk-nix", "inspect", source, "--json"],
            false,
            note,
        ));
    }

    if commands.is_empty() {
        commands.push(command(
            ["disk-nix", "filesystems", "--json"],
            false,
            "inspect modeled filesystem inventory before retrying",
        ));
    }

    commands
}

fn filesystem_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        let target = match tool {
            "xfs_growfs" | "fstrim" | "umount" => command.argv.get(1).map(String::as_str),
            "mount" => command.argv.last().map(String::as_str),
            "findmnt" if command.argv.iter().any(|arg| arg == "--target") => {
                command.argv.last().map(String::as_str)
            }
            "btrfs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["filesystem", "resize"]) =>
            {
                command.argv.get(3).map(String::as_str)
            }
            "btrfs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["filesystem", "usage"]) =>
            {
                command.argv.last().map(String::as_str)
            }
            "btrfs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["balance", "start"]) =>
            {
                command.argv.last().map(String::as_str)
            }
            "btrfs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["scrub", "start"]) =>
            {
                command.argv.last().map(String::as_str)
            }
            "zfs" if command.argv.get(1).is_some_and(|arg| arg == "set") => {
                command.argv.last().map(String::as_str)
            }
            _ => None,
        }?;

        Some(target).filter(|target| {
            !target.is_empty() && !target.starts_with('-') && !target.starts_with('<')
        })
    })
}

fn filesystem_source_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        let source = match tool {
            "blkid" => command.argv.get(1).map(String::as_str),
            "mount" if command.argv.len() >= 3 => {
                command.argv.get(command.argv.len() - 2).map(String::as_str)
            }
            "resize2fs" | "resize.f2fs" | "e2fsck" | "xfs_repair" | "ntfsfix" => {
                command.argv.last().map(String::as_str)
            }
            "fsck.fat" | "fsck.exfat" | "fsck.f2fs" => command.argv.last().map(String::as_str),
            "btrfs" if command.argv.get(1).is_some_and(|arg| arg == "check") => {
                command.argv.last().map(String::as_str)
            }
            "bcachefs"
                if command
                    .argv
                    .get(1)
                    .is_some_and(|arg| arg == "fsck" || arg == "format") =>
            {
                command.argv.last().map(String::as_str)
            }
            "bcachefs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["device", "resize"]) =>
            {
                command.argv.get(3).map(String::as_str)
            }
            tool if tool.starts_with("mkfs.") => command.argv.last().map(String::as_str),
            "mkfs" => command.argv.last().map(String::as_str),
            "e2label" | "fatlabel" | "ntfslabel" | "exfatlabel" | "f2fslabel" => {
                command.argv.get(1).map(String::as_str)
            }
            "xfs_admin" => command.argv.last().map(String::as_str),
            _ => None,
        }?;

        Some(source)
            .filter(|source| source.starts_with('/') && !source.starts_with("<") && *source != "/")
    })
}

fn partition_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    let disk = partition_disk_from_step(step).or_else(|| {
        step.action_id
            .split(':')
            .nth(1)
            .filter(|target| target.starts_with('/'))
    });
    let partition = partition_target_from_step(step);
    let mut commands = Vec::new();

    if let Some(disk) = disk {
        commands.push(command(["parted", "-lm", disk], false, note));
        commands.push(command(
            ["lsblk", "--json", "--bytes", "--output-all", disk],
            false,
            note,
        ));
        commands.push(command(
            ["disk-nix", "inspect", disk, "--json"],
            false,
            note,
        ));
    } else {
        commands.push(command(
            ["parted", "-lm"],
            false,
            "inspect all partition tables before retrying",
        ));
        commands.push(command(
            ["lsblk", "--json", "--bytes", "--output-all"],
            false,
            "inspect kernel disk and partition inventory before retrying",
        ));
    }

    if let Some(partition) = partition {
        commands.push(command(
            ["disk-nix", "inspect", partition, "--json"],
            false,
            note,
        ));
    }

    commands
}

fn partition_disk_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        let disk = match tool {
            "parted" => match command.argv.get(1).map(String::as_str) {
                Some("-s") => command.argv.get(2).map(String::as_str),
                Some("-lm") => command.argv.get(2).map(String::as_str),
                _ => command.argv.last().map(String::as_str),
            },
            "partprobe" => command.argv.get(1).map(String::as_str),
            "blockdev" if command.argv.get(1).is_some_and(|arg| arg == "--rereadpt") => {
                command.argv.get(2).map(String::as_str)
            }
            "growpart" => command.argv.get(1).map(String::as_str),
            _ => None,
        }?;

        Some(disk).filter(|disk| {
            disk.starts_with('/') && !disk.starts_with('<') && !disk.starts_with('-')
        })
    })
}

fn partition_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command
            .argv
            .get(0..2)
            .is_some_and(|args| args == ["disk-nix", "inspect"])
        {
            return command
                .argv
                .get(2)
                .map(String::as_str)
                .filter(|target| target.starts_with('/') && !target.starts_with('<'));
        }
        None
    })
}

fn nfs_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    match command_step_collection(step) {
        Some("exports") => {
            let target = nfs_export_target_from_step(step).or_else(|| {
                step.action_id
                    .split(':')
                    .nth(1)
                    .filter(|target| target.starts_with('/'))
            });
            let mut commands = vec![command(["exportfs", "-v"], false, note)];
            if let Some(target) = target {
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            } else {
                commands.push(command(
                    ["disk-nix", "nfs", "--json"],
                    false,
                    "inspect modeled NFS exports before retrying",
                ));
            }
            commands
        }
        Some("nfs.mounts") => {
            let mountpoint = nfs_mount_target_from_step(step).or_else(|| {
                step.action_id
                    .split(':')
                    .nth(1)
                    .filter(|target| target.starts_with('/'))
            });
            let mut commands = Vec::new();
            if let Some(mountpoint) = mountpoint {
                commands.push(command(["findmnt", "--json", mountpoint], false, note));
                commands.push(command(["nfsstat", "-m", mountpoint], false, note));
                commands.push(command(
                    ["disk-nix", "inspect", mountpoint, "--json"],
                    false,
                    note,
                ));
            } else {
                commands.push(command(
                    ["findmnt", "--json", "--types", "nfs,nfs4"],
                    false,
                    "inspect active NFS mounts before retrying",
                ));
                commands.push(command(
                    ["disk-nix", "nfs", "--json"],
                    false,
                    "inspect modeled NFS mounts before retrying",
                ));
            }
            commands
        }
        _ => Vec::new(),
    }
}

fn nfs_target_from_step(step: &ExecutionStep) -> Option<&str> {
    nfs_export_target_from_step(step).or_else(|| nfs_mount_target_from_step(step))
}

fn nfs_export_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command.argv.first().is_none_or(|arg| arg != "exportfs") {
            return None;
        }
        command
            .argv
            .last()
            .and_then(|target| {
                target
                    .split_once(':')
                    .map(|(_, path)| path)
                    .or(Some(target))
            })
            .filter(|target| target.starts_with('/') && !target.starts_with('<'))
    })
}

fn nfs_mount_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        let target = match tool {
            "mount" | "umount" | "findmnt" | "nfsstat" => command.argv.last().map(String::as_str),
            "disk-nix" if command.argv.get(1).is_some_and(|arg| arg == "inspect") => {
                command.argv.get(2).map(String::as_str)
            }
            _ => None,
        }?;

        Some(target).filter(|target| target.starts_with('/') && !target.starts_with('<'))
    })
}

fn local_mapping_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    match command_step_collection(step) {
        Some("dmMaps") => {
            let target = dm_map_target_from_step(step).or_else(|| {
                step.action_id
                    .split(':')
                    .nth(1)
                    .filter(|target| is_dm_map_target(target))
            });
            vec![
                dmsetup_info_command(target, note),
                dmsetup_deps_command(target),
                dmsetup_table_command(target),
                dmsetup_status_command(target),
                dm_map_inspect_json_command(target, note),
            ]
        }
        Some("loopDevices") => {
            let target = loop_target_from_step(step).or_else(|| {
                step.action_id
                    .split(':')
                    .nth(1)
                    .filter(|target| target.starts_with("/dev/loop"))
            });
            let mut commands = Vec::new();
            if let Some(target) = target {
                commands.push(command(
                    ["losetup", "--json", "--list", target],
                    false,
                    note,
                ));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            } else {
                commands.push(command(
                    ["losetup", "--json", "--list"],
                    false,
                    "inspect loop mappings before retrying",
                ));
            }
            if let Some(backing) = backing_file_from_step(step) {
                commands.push(command(
                    ["stat", "--printf=%n %s %b %B\\n", backing],
                    false,
                    note,
                ));
                commands.push(command(
                    ["disk-nix", "inspect", backing, "--json"],
                    false,
                    note,
                ));
            }
            commands
        }
        Some("backingFiles") => {
            let target = backing_file_from_step(step).or_else(|| {
                step.action_id
                    .split(':')
                    .nth(1)
                    .filter(|target| target.starts_with('/'))
            });
            let mut commands = Vec::new();
            if let Some(target) = target {
                commands.push(command(
                    ["stat", "--printf=%n %s %b %B\\n", target],
                    false,
                    note,
                ));
                commands.push(command(
                    ["du", "--bytes", "--apparent-size", target],
                    false,
                    note,
                ));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            } else {
                commands.push(command(
                    ["disk-nix", "backing-files", "--json"],
                    false,
                    "inspect modeled backing-file inventory before retrying",
                ));
            }
            commands
        }
        _ => Vec::new(),
    }
}

fn local_mapping_target_from_step(step: &ExecutionStep) -> Option<&str> {
    dm_map_target_from_step(step)
        .or_else(|| loop_target_from_step(step))
        .or_else(|| backing_file_from_step(step))
}

fn dm_map_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        let target = match tool {
            "dmsetup" => match command.argv.get(1).map(String::as_str) {
                Some("rename") => command.argv.get(2).map(String::as_str),
                Some("remove" | "deps" | "table" | "status") => {
                    command.argv.get(2).map(String::as_str)
                }
                Some("info") => command.argv.last().map(String::as_str),
                _ => None,
            },
            "disk-nix" if command.argv.get(1).is_some_and(|arg| arg == "inspect") => {
                command.argv.get(2).map(String::as_str)
            }
            _ => None,
        }?;

        Some(target).filter(|target| is_dm_map_target(target))
    })
}

fn loop_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        let target = match tool {
            "losetup" => match command.argv.get(1).map(String::as_str) {
                Some("--detach" | "-c") => command.argv.get(2).map(String::as_str),
                Some("--json") => command.argv.last().map(String::as_str),
                Some(target) if target.starts_with("/dev/loop") => {
                    command.argv.get(1).map(String::as_str)
                }
                _ => None,
            },
            "disk-nix" if command.argv.get(1).is_some_and(|arg| arg == "inspect") => {
                command.argv.get(2).map(String::as_str)
            }
            _ => None,
        }?;

        Some(target).filter(|target| target.starts_with("/dev/loop") && !target.starts_with('<'))
    })
}

fn backing_file_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        let target = match tool {
            "truncate" | "stat" | "du" | "test" => command.argv.last().map(String::as_str),
            "losetup" => command.argv.last().map(String::as_str),
            "disk-nix" if command.argv.get(1).is_some_and(|arg| arg == "inspect") => {
                command.argv.get(2).map(String::as_str)
            }
            _ => None,
        }?;

        Some(target).filter(|target| {
            target.starts_with('/')
                && !target.starts_with('<')
                && !target.starts_with("/dev/loop")
                && !is_dm_map_target(target)
        })
    })
}

fn snapshot_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    let mut commands = Vec::new();
    if let Some(snapshot) = snapshot_target_from_step(step).or_else(|| {
        step.action_id
            .split(':')
            .nth(1)
            .filter(|target| !target.is_empty())
    }) {
        if is_zfs_snapshot_name(snapshot) {
            commands.push(command(
                ["zfs", "list", "-t", "snapshot", "-H", "-p", snapshot],
                false,
                note,
            ));
            commands.push(command(["zfs", "holds", snapshot], false, note));
            if let Some(dataset) = zfs_snapshot_dataset(snapshot) {
                commands.push(command(["zfs", "list", "-H", "-p", dataset], false, note));
                commands.push(command_vec(
                    [
                        "zfs",
                        "list",
                        "-t",
                        "snapshot",
                        "-H",
                        "-p",
                        "-o",
                        "name,creation,used,referenced,userrefs",
                        "-r",
                        dataset,
                    ],
                    false,
                    note,
                ));
            }
            commands.push(command(
                ["disk-nix", "inspect", snapshot, "--json"],
                false,
                note,
            ));
        } else if snapshot.starts_with('/') {
            commands.push(command(
                ["btrfs", "subvolume", "show", snapshot],
                false,
                note,
            ));
            commands.push(command(
                ["btrfs", "property", "get", "-ts", snapshot, "ro"],
                false,
                note,
            ));
            commands.push(command(
                ["disk-nix", "inspect", snapshot, "--json"],
                false,
                note,
            ));
        } else {
            commands.push(command(
                ["disk-nix", "inspect", snapshot, "--json"],
                false,
                note,
            ));
        }
    }
    commands
}

fn snapshot_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        match tool {
            "zfs" => match command.argv.get(1).map(String::as_str) {
                Some("snapshot" | "destroy" | "rollback" | "holds") => {
                    command.argv.last().map(String::as_str)
                }
                Some("clone" | "rename") => command.argv.get(2).map(String::as_str),
                Some("hold" | "release") => command.argv.last().map(String::as_str),
                Some("list")
                    if command
                        .argv
                        .iter()
                        .any(|arg| arg == "-t" || arg == "snapshot") =>
                {
                    command.argv.last().map(String::as_str)
                }
                _ => None,
            },
            "btrfs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["subvolume", "show"]) =>
            {
                command.argv.last().map(String::as_str)
            }
            "btrfs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["subvolume", "delete"]) =>
            {
                command.argv.last().map(String::as_str)
            }
            "btrfs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["subvolume", "snapshot"]) =>
            {
                command
                    .argv
                    .iter()
                    .skip(3)
                    .find(|arg| !arg.starts_with('-'))
                    .map(String::as_str)
            }
            "btrfs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["property", "get"]) =>
            {
                command
                    .argv
                    .iter()
                    .skip(3)
                    .find(|arg| arg.starts_with('/'))
                    .map(String::as_str)
            }
            "mv" => command.argv.iter().skip(1).find_map(|arg| {
                if arg == "--" || arg.starts_with('-') {
                    None
                } else {
                    Some(arg.as_str())
                }
            }),
            _ => None,
        }
        .filter(|target| !target.starts_with('<'))
    })
}

fn btrfs_subvolume_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command
            .argv
            .get(0..3)
            .is_some_and(|args| args == ["btrfs", "subvolume", "show"])
            || command
                .argv
                .get(0..3)
                .is_some_and(|args| args == ["btrfs", "subvolume", "create"])
            || command
                .argv
                .get(0..3)
                .is_some_and(|args| args == ["btrfs", "subvolume", "delete"])
        {
            return command.argv.get(3).map(String::as_str);
        }
        if command
            .argv
            .get(0..4)
            .is_some_and(|args| args == ["btrfs", "property", "set", "-ts"])
            || command
                .argv
                .get(0..4)
                .is_some_and(|args| args == ["btrfs", "property", "get", "-ts"])
        {
            return command.argv.get(4).map(String::as_str);
        }
        if command
            .argv
            .get(0..2)
            .is_some_and(|args| args == ["mv", "--"])
        {
            return command.argv.get(2).map(String::as_str);
        }
        None
    })
}

fn btrfs_qgroup_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command
            .argv
            .get(0..5)
            .is_some_and(|args| args == ["btrfs", "qgroup", "show", "--raw", "-reF"])
        {
            return command.argv.get(5).map(String::as_str);
        }
        if command
            .argv
            .get(0..3)
            .is_some_and(|args| args == ["btrfs", "qgroup", "create"])
            || command
                .argv
                .get(0..3)
                .is_some_and(|args| args == ["btrfs", "qgroup", "destroy"])
        {
            return command.argv.get(4).map(String::as_str);
        }
        if command
            .argv
            .get(0..3)
            .is_some_and(|args| args == ["btrfs", "qgroup", "limit"])
        {
            return if command.argv.get(3).is_some_and(|arg| arg == "-e") {
                command.argv.get(6).map(String::as_str)
            } else {
                command.argv.get(5).map(String::as_str)
            };
        }
        None
    })
}

fn iscsi_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command.argv.first().is_some_and(|arg| arg == "iscsiadm") {
            return command
                .argv
                .windows(2)
                .find(|window| window[0] == "--targetname")
                .map(|window| window[1].as_str());
        }
        None
    })
}

fn nvme_controller_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command.argv.first().is_some_and(|arg| arg == "nvme")
            && command.argv.get(1).is_some_and(|arg| {
                matches!(
                    arg.as_str(),
                    "attach-ns" | "create-ns" | "delete-ns" | "detach-ns" | "list-ns" | "ns-rescan"
                )
            })
        {
            return command
                .argv
                .get(2)
                .map(String::as_str)
                .filter(|target| is_nvme_controller_path(target));
        }
        None
    })
}

fn target_lun_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.action_id
        .strip_prefix("targetluns:")
        .or_else(|| step.action_id.strip_prefix("targetLuns:"))
        .and_then(|rest| {
            rest.split_once(":set-property:")
                .map(|(target, _)| target)
                .or_else(|| rest.rsplit_once(':').map(|(target, _)| target))
        })
        .filter(|target| !target.is_empty())
}

fn failed_result_notes(result: &ExecutionCommandResult) -> Vec<String> {
    let mut notes = vec![
        format!(
            "{:?} phase failed for action {}",
            result.phase, result.action_id
        ),
        format!("command: {}", result.argv.join(" ")),
    ];
    if let Some(status_code) = result.status_code {
        notes.push(format!("exit status: {status_code}"));
    }
    if !result.stderr.trim().is_empty() {
        notes.push(format!("stderr: {}", result.stderr.trim()));
    }
    notes
}

fn target_lun_recovery_inspection_commands(
    target: Option<&str>,
    note: &str,
) -> Vec<ExecutionCommand> {
    let mut commands = vec![
        command_vec(["targetcli", "/iscsi", "ls"], false, note),
        command_vec(
            [
                "tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show",
            ],
            false,
            note,
        ),
        lsscsi_lun_inventory_command(note),
        command_vec(["multipath", "-ll"], false, note),
    ];
    if let Some(target) = target {
        commands.insert(
            1,
            command_vec(
                vec![
                    "targetcli".to_string(),
                    format!("/iscsi/{target}"),
                    "ls".to_string(),
                ],
                false,
                note,
            ),
        );
    }
    commands
}

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
    let cache_target = bcache_target_path(action).unwrap_or(target);
    let mountpoint = action.context.mountpoint.as_deref();
    let fs_type = action.context.fs_type.as_deref();
    let desired_size = action.context.desired_size.as_deref();

    match action.operation {
        Operation::Create
        | Operation::Grow
        | Operation::Attach
        | Operation::Detach
        | Operation::Destroy
        | Operation::SetProperty
        | Operation::Rescan
            if collection == Some("targetLuns") || action.id.starts_with("targetLuns:") =>
        {
            (
                target_lun_verification_commands(action, target),
                vec![
                    "target-side provider inventory shows the reviewed LUN identity, initiator mapping, and capacity"
                        .to_string(),
                    "host-side LUN and multipath consumers are refreshed only after provider verification"
                        .to_string(),
                ],
            )
        }
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
        Operation::Deactivate if collection == Some("swaps") => (
            vec![
                command(
                    ["swapon", "--show", "--bytes", "--raw"],
                    false,
                    "verify active swap inventory after swapoff",
                ),
                command(
                    ["disk-nix", "topology", "--json"],
                    false,
                    "verify modeled swap node is inactive or absent after swapoff",
                ),
            ],
            vec![
                "target is absent from active swapon output".to_string(),
                "swap signature remains intact unless a separate destroy action was requested"
                    .to_string(),
            ],
        ),
        Operation::Destroy if collection == Some("swaps") => (
            vec![
                command(
                    ["swapon", "--show", "--bytes", "--raw"],
                    false,
                    "verify active swap inventory after signature removal",
                ),
                swap_blkid_command(
                    swap_target_path(action),
                    "verify swap signature is absent or intentionally replaced",
                ),
                command(
                    ["disk-nix", "topology", "--json"],
                    false,
                    "verify modeled swap node and consumers after swap destruction",
                ),
            ],
            vec![
                "target is absent from active swapon output".to_string(),
                "NixOS swapDevices, resume, and hibernation references no longer point at the destroyed signature"
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
        Operation::Rescan if collection == Some("zram") => (
            zram_rescan_commands("verify zram compressed swap inventory after refresh"),
            vec![
                "zram devices, algorithms, sizes, memory use, and swap state are reviewed"
                    .to_string(),
                "NixOS zramSwap settings still match the generated compressed swap topology"
                    .to_string(),
            ],
        ),
        Operation::Create if collection == Some("backingFiles") => {
            let target = backing_file_target_path(action);
            (
                vec![
                    backing_file_stat_command(target, "verify backing file metadata after creation"),
                    backing_file_inspect_json_command(
                        target,
                        "verify modeled backing-file relationships after creation",
                    ),
                ],
                vec![
                    "backing file exists at the reviewed path with the requested capacity"
                        .to_string(),
                    "loop devices, swapfiles, and filesystem consumers are created only after the file identity is verified"
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
        Operation::Grow if collection == Some("backingFiles") => {
            let target = backing_file_target_path(action);
            (
                vec![
                    backing_file_stat_command(target, "verify backing file size after growth"),
                    backing_file_inspect_json_command(
                        target,
                        "verify modeled backing-file consumers after growth",
                    ),
                ],
                vec![
                    "backing file reports the requested capacity".to_string(),
                    "dependent loop, swap, mapping, or filesystem consumers are refreshed separately"
                        .to_string(),
                ],
            )
        }
        Operation::Rescan if collection == Some("backingFiles") => {
            let target = backing_file_target_path(action);
            (
                vec![
                    backing_file_stat_command(target, "verify backing file metadata after rescan"),
                    backing_file_usage_command(target),
                    backing_file_inspect_json_command(
                        target,
                        "verify modeled backing-file consumers after rescan",
                    ),
                ],
                vec![
                    "backing file size, allocation, and sparse usage are reviewed".to_string(),
                    "dependent loop, swap, mapping, or filesystem consumers still resolve the file"
                        .to_string(),
                ],
            )
        }
        Operation::Rescan if collection == Some("dmMaps") => {
            let target = dm_map_target_path(action);
            (
                vec![
                    dmsetup_info_command(target, "verify device-mapper identity after rescan"),
                    dmsetup_deps_command(target),
                    dmsetup_table_command(target),
                    dmsetup_status_command(target),
                    dm_map_inspect_json_command(
                        target,
                        "verify modeled device-mapper relationships after rescan",
                    ),
                ],
                vec![
                    "device-mapper name, UUID, dependencies, table, and live status are reviewed"
                        .to_string(),
                    "dependent LUKS, LVM, VDO, multipath, filesystem, or mount consumers still resolve the mapper"
                        .to_string(),
                ],
            )
        }
        Operation::Rename if collection == Some("dmMaps") => {
            let rename_to = dm_map_rename_to(action);
            let renamed_target = rename_to
                .as_ref()
                .map(|name| format!("/dev/mapper/{name}"));
            let renamed_target = renamed_target.as_deref();
            (
                vec![
                    dmsetup_info_command(renamed_target, "verify device-mapper identity after rename"),
                    dmsetup_deps_command(renamed_target),
                    dmsetup_status_command(renamed_target),
                    dm_map_inspect_json_command(
                        renamed_target,
                        "verify modeled device-mapper relationships after rename",
                    ),
                ],
                vec![
                    "renamed device-mapper path resolves with the expected name, UUID, dependencies, and status"
                        .to_string(),
                    "dependent LUKS, LVM, VDO, multipath, filesystem, or mount consumers are updated to the new mapper path"
                        .to_string(),
                ],
            )
        }
        Operation::Destroy if collection == Some("dmMaps") => (
            vec![dmsetup_ls_tree_command(
                "verify device-mapper inventory after removal",
            )],
            vec![
                "removed device-mapper map no longer appears in dmsetup inventory".to_string(),
                "dependent mounts, LUKS, LVM, VDO, multipath, cache, or filesystem consumers were removed or moved first"
                    .to_string(),
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
            let mut commands = vec![
                command(
                    ["lsblk", "--json", "--bytes", "--output-all"],
                    false,
                    if is_rescan {
                        "verify kernel block-device inventory after host rescan"
                    } else {
                        "verify kernel block-device capacity after host rescan"
                    },
                ),
                lsscsi_lun_inventory_command(if is_rescan {
                    "verify host-visible LUN transport and size after rescan"
                } else {
                    "verify host-visible LUN transport and size after growth rescan"
                }),
            ];
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
            let mut commands = vec![
                command(
                    ["lsblk", "--json", "--bytes", "--output-all"],
                    false,
                    "verify kernel block-device inventory after LUN attach",
                ),
                lsscsi_lun_inventory_command(
                    "verify attached LUN transport and size after host rescan",
                ),
            ];
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
            let mut commands = vec![
                command(
                    ["lsblk", "--json", "--bytes", "--output-all"],
                    false,
                    "verify kernel block-device inventory after LUN detach",
                ),
                lsscsi_lun_inventory_command(
                    "verify remaining host-visible LUN transport and size after detach",
                ),
            ];
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
            let target = zfs_pool_command_target(action, Some(target));
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
                    [
                        "zpool",
                        "status",
                        "-P",
                        action.context.name.as_deref().unwrap_or(target),
                    ],
                    false,
                    "verify ZFS pool health and vdev topology after creation",
                ),
                command(
                    [
                        "zpool",
                        "list",
                        "-H",
                        "-p",
                        action.context.name.as_deref().unwrap_or(target),
                    ],
                    false,
                    "verify ZFS pool size, allocation, and free capacity after creation",
                ),
                command(
                    [
                        "disk-nix",
                        "inspect",
                        action.context.name.as_deref().unwrap_or(target),
                        "--json",
                    ],
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
            if action.context.fs_type.as_deref() == Some("bcachefs") {
                (
                    vec![
                        bcachefs_usage_command(
                            target,
                            "verify bcachefs allocation after topology change",
                        ),
                        command(
                            ["disk-nix", "inspect", target, "--json"],
                            false,
                            "verify filesystem graph relationships after topology change",
                        ),
                    ],
                    vec![
                        "bcachefs member list matches desired topology".to_string(),
                        "replication and free-space state are reviewed after topology change"
                            .to_string(),
                    ],
                )
            } else {
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
        Operation::Destroy if collection == Some("multipathMaps") => (
            vec![
                command(
                    ["multipath", "-ll"],
                    false,
                    "verify multipath inventory after map removal",
                ),
                command(
                    ["disk-nix", "inspect", "multipath", "--json"],
                    false,
                    "verify multipath graph relationships after map removal",
                ),
            ],
            vec![
                "removed multipath map no longer appears in host multipath inventory".to_string(),
                "dependent filesystems, LVM, dm, and service consumers were removed or moved first"
                    .to_string(),
            ],
        ),
        Operation::Create
        | Operation::Attach
        | Operation::Grow
        | Operation::Detach
        | Operation::Destroy
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
                        ["disk-nix", "inspect", cache_target, "--json"],
                        false,
                        "verify modeled cache layer relationships after cache update",
                    ),
                    bcache_sysfs_read_command(
                        cache_target,
                        "state",
                        "verify bcache state after update",
                    ),
                    bcache_sysfs_read_command(
                        cache_target,
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
        Operation::SetProperty if collection == Some("zram") => (
            zram_rescan_commands("verify zram compressed swap declaration after inventory refresh"),
            vec![
                "zram device count, algorithm, size, memory use, and swap activation are reviewed"
                    .to_string(),
                "NixOS zramSwap-derived settings match the generated compressed swap topology"
                    .to_string(),
            ],
        ),
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
            let snapshot = snapshot_rescan_identity(action, target);
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
            if is_zfs_snapshot_name(snapshot) {
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
            } else if is_btrfs_snapshot_pair(snapshot, clone_target) {
                (
                    vec![
                        command(
                            ["btrfs", "subvolume", "show", snapshot],
                            false,
                            "verify source Btrfs snapshot subvolume exists before clone",
                        ),
                        command(
                            ["btrfs", "subvolume", "show", clone_target],
                            false,
                            "verify cloned Btrfs subvolume after clone",
                        ),
                        command(
                            ["disk-nix", "inspect", clone_target, "--json"],
                            false,
                            "verify cloned Btrfs subvolume graph state after clone",
                        ),
                    ],
                    vec![
                        "clone subvolume exists at the reviewed path".to_string(),
                        "snapshot lineage and read-only state were reviewed after clone".to_string(),
                    ],
                )
            } else {
                (Vec::new(), Vec::new())
            }
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
        Operation::Import | Operation::Export if collection == Some("pools") => {
            let target = zfs_pool_command_target(action, Some(target));
            (
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
                    "pool import or export state matches the declared lifecycle operation"
                        .to_string(),
                    "datasets, mountpoints, shares, LUN mappings, and services are reviewed after the pool state change"
                        .to_string(),
                ],
            )
        }
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
        Operation::Format if collection == Some("filesystems") => {
            let device = action.context.device.as_deref().unwrap_or(target);
            (
                vec![
                    command(
                        ["blkid", device],
                        false,
                        "verify filesystem signature identity after mkfs",
                    ),
                    command(
                        ["disk-nix", "inspect", device, "--json"],
                        false,
                        "verify formatted filesystem graph relationships",
                    ),
                ],
                vec![
                    "formatted device reports the intended filesystem type".to_string(),
                    "mount, UUID, label, and dependent NixOS references are reviewed after formatting"
                        .to_string(),
                ],
            )
        }
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
        Operation::Mount if action.context.collection.as_deref() == Some("filesystems") => {
            let mountpoint = filesystem_mountpoint(action);
            (
                vec![
                    filesystem_findmnt_command(mountpoint),
                    command(
                        ["disk-nix", "inspect", mountpoint.unwrap_or(target), "--json"],
                        false,
                        "verify filesystem graph state after mount",
                    ),
                ],
                vec![
                    "findmnt reports the mountpoint with the reviewed source and options"
                        .to_string(),
                    "local services see the expected mounted filesystem source and type"
                        .to_string(),
                ],
            )
        }
        Operation::Unmount if action.context.collection.as_deref() == Some("filesystems") => {
            let mountpoint = filesystem_mountpoint(action);
            (
                vec![
                    filesystem_findmnt_command(mountpoint),
                    command(
                        ["disk-nix", "topology", "--json"],
                        false,
                        "re-probe full topology after filesystem unmount",
                    ),
                ],
                vec![
                    "findmnt no longer reports the reviewed filesystem as mounted".to_string(),
                    "dependent services and bind mounts have no stale references".to_string(),
                ],
            )
        }
        Operation::Rescan if action.context.collection.as_deref() == Some("filesystems") => {
            let mountpoint = filesystem_mountpoint(action);
            (
                vec![
                    filesystem_findmnt_command(mountpoint),
                    filesystem_inspect_command(
                        mountpoint,
                        true,
                        "verify filesystem graph state after rescan",
                    ),
                ],
                vec![
                    "findmnt and disk-nix graph state were refreshed without mounting, remounting, or unmounting"
                        .to_string(),
                    "source, filesystem type, options, and consumers match the reviewed inventory"
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
    let cache_target = bcache_target_path(action);
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
        Operation::Format
            if collection == Some("filesystems") || action.id.starts_with("filesystem:") =>
        {
            let target = target.unwrap_or("<filesystem>");
            let fs_type = action
                .context
                .fs_type
                .as_deref()
                .unwrap_or("<filesystem-type>");
            let device = action.context.device.as_deref();
            let mut commands = vec![
                disk_nix_inspect_command(
                    device,
                    "<filesystem-device>",
                    "filesystem source device",
                    "inspect target device before creating a filesystem signature",
                ),
            ];
            if device.is_some_and(|device| device.starts_with("/dev/md/")) {
                commands.push(command(
                    ["udevadm", "settle"],
                    false,
                    "wait for md device events to settle before formatting",
                ));
            }
            commands.push(filesystem_format_command(fs_type, device));
            if matches!(fs_type, "btrfs" | "bcachefs") {
                if let Some(mountpoint) = filesystem_mountpoint(action) {
                    commands.push(filesystem_mount_command(
                        device,
                        Some(mountpoint),
                        Some(fs_type),
                        action.context.options.as_deref(),
                    ));
                }
            }
            (
                commands,
                vec![
                    format!("formatting {target} as {fs_type} destroys existing data on the selected device"),
                    "prefer preserving or migrating data before replacing a filesystem signature"
                        .to_string(),
                    "mount the new filesystem only after its UUID, label, and stable device path are verified"
                        .to_string(),
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
        Operation::Mount
            if collection == Some("filesystems") || action.id.starts_with("filesystems:") =>
        {
            let mountpoint = filesystem_mountpoint(action);
            (
                vec![filesystem_mount_command(
                    action.context.device.as_deref(),
                    mountpoint,
                    action.context.fs_type.as_deref(),
                    action.context.options.as_deref(),
                )],
                vec![
                    "verify the source device, filesystem type, and mountpoint before mounting"
                        .to_string(),
                    "persist long-lived mounts through the NixOS fileSystems entry".to_string(),
                ],
                true,
            )
        }
        Operation::Unmount
            if collection == Some("filesystems") || action.id.starts_with("filesystems:") =>
        {
            let mountpoint = filesystem_mountpoint(action);
            (
                vec![
                    filesystem_findmnt_command(mountpoint),
                    filesystem_unmount_command(mountpoint),
                ],
                vec![
                    "stop services, automount units, and sessions that depend on the mountpoint before unmounting"
                        .to_string(),
                    "verify no open files, bind mounts, or namespaces still reference the mountpoint"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rescan
            if collection == Some("filesystems") || action.id.starts_with("filesystems:") =>
        {
            let mountpoint = filesystem_mountpoint(action);
            (
                vec![
                    filesystem_findmnt_command(mountpoint),
                    filesystem_inspect_command(
                        mountpoint,
                        false,
                        "refresh modeled filesystem graph state",
                    ),
                ],
                vec![
                    "filesystem rescan is read-only and does not mount, remount, unmount, or format storage"
                        .to_string(),
                    "use the refreshed inventory before selecting any mutating lifecycle action"
                        .to_string(),
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
        Operation::Create if collection == Some("backingFiles") => {
            let target = backing_file_target_path(action);
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    backing_file_absent_command(target),
                    backing_file_create_command(target, desired_size),
                    backing_file_stat_command(target, "inspect backing file after creation"),
                ],
                vec![
                    "create only a new file; existing backing images are left untouched"
                        .to_string(),
                    "verify sparse allocation policy and host filesystem free space before attaching consumers"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("backingFiles") => {
            let target = backing_file_target_path(action);
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    backing_file_stat_command(target, "inspect backing file before growth"),
                    backing_file_grow_command(target, desired_size),
                ],
                vec![
                    "verify host filesystem free space and sparse allocation policy before growth"
                        .to_string(),
                    "refresh loop devices, swap signatures, and dependent filesystems after the file grows"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("backingFiles") => {
            let target = backing_file_target_path(action);
            (
                vec![
                    backing_file_stat_command(target, "refresh backing file size and metadata"),
                    backing_file_usage_command(target),
                    backing_file_inspect_command(target),
                ],
                vec![
                    "backing file rescan is read-only and does not resize or detach consumers"
                        .to_string(),
                    "use grow only when file-backed storage capacity must change".to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("dmMaps") => {
            let target = dm_map_target_path(action);
            (
                vec![
                    dmsetup_info_command(target, "refresh device-mapper identity metadata"),
                    dmsetup_deps_command(target),
                    dmsetup_table_command(target),
                    dmsetup_status_command(target),
                    dm_map_inspect_command(target),
                ],
                vec![
                    "device-mapper rescan is read-only and does not reload or remove maps"
                        .to_string(),
                    "use domain-specific LUKS, LVM, VDO, multipath, or cache actions for mutating mapper lifecycle"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rename if collection == Some("dmMaps") => {
            let target = dm_map_target_path(action);
            let rename_to = dm_map_rename_to(action);
            (
                vec![
                    dmsetup_info_command(target, "inspect device-mapper identity before rename"),
                    dmsetup_deps_command(target),
                    dmsetup_rename_command(target, rename_to.as_deref()),
                    dm_map_inspect_command(target),
                ],
                vec![
                    "device-mapper rename changes the visible mapper path and can break consumers until declarations are updated"
                        .to_string(),
                    "prefer LUKS, LVM, VDO, multipath, or cache-specific rename workflows when the mapper is owned by a higher-level domain"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("dmMaps") => {
            let target = dm_map_target_path(action);
            (
                vec![
                    dmsetup_info_command(target, "inspect device-mapper identity before removal"),
                    dmsetup_deps_command(target),
                    dmsetup_status_command(target),
                    dmsetup_remove_command(target),
                ],
                vec![
                    "device-mapper removal destroys the live map and can make dependent data inaccessible"
                        .to_string(),
                    "prefer domain-specific LUKS, LVM, VDO, multipath, or cache teardown when the mapper is owned elsewhere"
                        .to_string(),
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
                lsscsi_lun_inventory_command(
                    "inspect host-visible LUN transport and size before per-device rescans",
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
                lsscsi_lun_inventory_command(
                    "inspect host-visible LUN transport and size before growth rescans",
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
                lsscsi_lun_inventory_command(
                    "inspect host-visible LUN transport and size after session rescan",
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
                    vec!["<scsi-rescan-device>", "<lun-path>"],
                    true,
                    CommandReadiness::NeedsDomainImplementation,
                    ["stable LUN device path"],
                    "rescan the concrete SCSI path after declaring a stable by-path device",
                ));
            }
            for device in &devices {
                commands.push(scsi_device_rescan_command(device));
            }
            commands.push(command(
                ["multipath", "-r"],
                true,
                "reload multipath maps after newly attached LUN paths appear",
            ));
            if devices.is_empty() {
                commands.push(command_vec_with_readiness(
                    vec!["blockdev", "--getsize64", "<lun-path>"],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["stable LUN device path"],
                    "verify the reviewed LUN path after declaring a stable by-path device",
                ));
            }
            for device in &devices {
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
        Operation::Create
        | Operation::Grow
        | Operation::Attach
        | Operation::Detach
        | Operation::Destroy
        | Operation::SetProperty
        | Operation::Rescan
            if collection == Some("targetLuns") || action.id.starts_with("targetLuns:") =>
        {
            let target = target.unwrap_or("<target-lun>");
            (
                target_lun_commands(action, target),
                vec![
                    "target-side LUN work is provider-specific and stays non-ready until an array adapter or reviewed runbook renders concrete commands"
                        .to_string(),
                    "run host-side luns, iscsiSessions, and multipath rescans only after the target reports the intended mapping and capacity"
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
                    lsscsi_lun_inventory_command(
                        "inspect host-visible LUN transport and size after session rescan",
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
        Operation::Deactivate if collection == Some("swaps") => {
            let target = swap_target_path(action);
            (
                vec![
                    command(
                        ["swapon", "--show", "--bytes", "--raw"],
                        false,
                        "inspect active swap state before swapoff",
                    ),
                    swap_command("swapoff", target, "disable active swap without removing its signature"),
                ],
                vec![
                    "verify memory pressure and hibernation dependencies before swapoff"
                        .to_string(),
                    "use destroy only when swap signature metadata should be removed".to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("swaps") => {
            let target = swap_target_path(action);
            (
                vec![
                    disk_nix_inspect_command(
                        target,
                        "<swap>",
                        "swap target path",
                        "inspect target before disabling and wiping swap signature",
                    ),
                    swap_command("swapoff", target, "disable active swap before removing its signature"),
                    swap_wipefs_command(target),
                ],
                vec![
                    "remove or update NixOS swapDevices before wiping the signature".to_string(),
                    "verify resume and hibernation references before deleting swap metadata"
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
        Operation::Rescan if collection == Some("zram") => (
            zram_rescan_commands("refresh zram compressed swap inventory"),
            vec![
                "use services.disk-nix.zram to reconcile generated NixOS zramSwap settings"
                    .to_string(),
                "coordinate swapoff before changing live zram size, algorithm, priority, or writeback device"
                    .to_string(),
            ],
            true,
        ),
        Operation::SetProperty if collection == Some("zram") => (
            zram_rescan_commands("inspect zram compressed swap declaration and current inventory"),
            vec![
                "plain zram declarations inspect generated compressed swap state without mutating it"
                    .to_string(),
                "use operation = \"rescan\" for an explicit zram inventory refresh action"
                    .to_string(),
                "use services.disk-nix.zram options to reconcile generated NixOS zramSwap settings"
                    .to_string(),
            ],
            false,
        ),
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
            let physical_size = action.context.physical_size.as_deref();
            let mut commands = vec![command(
                ["vdo", "status", "--name", target],
                false,
                "inspect VDO logical and physical size before growth",
            )];
            commands.extend(vdo_growth_commands(target, desired_size, physical_size));
            (
                commands,
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
                    lsscsi_lun_inventory_command(
                        "inspect host-visible SCSI path transport and size before multipath growth",
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
                    lsscsi_lun_inventory_command(
                        "inspect host-visible SCSI path transport and size before multipath rescan",
                    ),
                    command(
                        ["multipath", "-r"],
                        true,
                        "reload multipath maps after refreshed backing paths",
                    ),
                    multipath_list_command(target, "verify multipath map paths after rescan"),
                    lsscsi_lun_inventory_command(
                        "verify host-visible SCSI path transport and size after multipath rescan",
                    ),
                ],
                vec![
                    "rescan backing SCSI or iSCSI paths before reloading the map".to_string(),
                    "verify the map WWID and every expected path before exposing consumers"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("multipathMaps") => {
            let target = multipath_map_target(action);
            (
                vec![
                    multipath_list_command(target, "inspect multipath map paths before removal"),
                    multipath_flush_map_command(target),
                ],
                vec![
                    "multipath map removal flushes the host map but does not delete target-side data"
                        .to_string(),
                    "unmount filesystems and deactivate LVM, dm, and service consumers before flushing the map"
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
                    nvme_list_subsystems_command("inspect NVMe subsystem paths before rescan"),
                    nvme_namespace_rescan_command(controller),
                    nvme_list_namespaces_command(controller, "verify NVMe namespaces after rescan"),
                    nvme_list_subsystems_command("verify NVMe subsystem paths after rescan"),
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
                    nvme_list_subsystems_command("inspect NVMe subsystem paths before growth rescan"),
                    nvme_namespace_rescan_command(controller),
                    nvme_list_namespaces_command(controller, "verify NVMe namespaces after rescan"),
                    nvme_list_subsystems_command("verify NVMe subsystem paths after growth rescan"),
                ],
                vec![
                    "perform controller-side namespace resize before host rescan".to_string(),
                    "grow dependent partitions, volumes, or filesystems only after the namespace reports the new size"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Attach if collection == Some("nvmeNamespaces") => {
            let controller = nvme_controller_target(action);
            let namespace_id = action.context.namespace_id.as_deref();
            let controllers = action.context.controllers.as_deref();
            (
                vec![
                    nvme_list_namespaces_command(
                        controller,
                        "inspect NVMe namespace inventory before attach",
                    ),
                    nvme_list_subsystems_command("inspect NVMe subsystem paths before attach"),
                    nvme_attach_namespace_command(controller, namespace_id, controllers),
                    nvme_namespace_rescan_command(controller),
                    nvme_list_namespaces_command(
                        controller,
                        "verify NVMe namespace inventory after attach",
                    ),
                    nvme_list_subsystems_command("verify NVMe subsystem paths after attach"),
                ],
                vec![
                    "attach preserves the namespace and only changes controller visibility"
                        .to_string(),
                    "verify namespace id and controller attachment before exposing consumers"
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
            let target = if collection == Some("caches") {
                cache_target.unwrap_or("<cache-device>")
            } else if collection == Some("pools") {
                zfs_pool_command_target(action, target)
            } else {
                target.unwrap_or("<target>")
            };
            let fs_type = action.context.fs_type.as_deref();
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
                    add_device_command(collection, fs_type, target, device),
                ],
                vec![
                    "verify the new device identity and redundancy policy before attaching it"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::ReplaceDevice
            if collection == Some("filesystems")
                && action.context.fs_type.as_deref() == Some("bcachefs") =>
        {
            let target = target.unwrap_or("<bcachefs-mountpoint>");
            let from = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "replace-device"));
            let to = action.context.replacement.as_deref();
            (
                vec![
                    bcachefs_usage_command(
                        target,
                        "inspect bcachefs allocation before replacement",
                    ),
                    bcachefs_add_device_command(target, to),
                    bcachefs_rereplicate_command(target),
                    bcachefs_remove_device_command(target, from),
                ],
                vec![
                    "add replacement capacity before evacuating the old bcachefs member"
                        .to_string(),
                    "wait for rereplication to converge before removing the old device".to_string(),
                    "keep the old device available until post-apply verification passes"
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
            let target = if collection == Some("caches") {
                cache_target.unwrap_or("<cache-device>")
            } else {
                target.unwrap_or("<target>")
            };
            let fs_type = action.context.fs_type.as_deref();
            let from = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "replace-device"));
            let to = action.context.replacement.as_deref();
            let replacement_cache_set = action.context.cache_set_uuid.as_deref();
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect redundancy and source device health before replacement",
                    ),
                    if collection == Some("caches") {
                        match (from, to) {
                            (Some(from), Some(to)) => {
                                bcache_replace_command(target, from, to, replacement_cache_set)
                            }
                            _ => replace_device_command(collection, fs_type, target, from, to),
                        }
                    } else {
                        replace_device_command(collection, fs_type, target, from, to)
                    },
                ],
                vec![
                    "keep the old device available until post-apply verification passes"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rebalance => {
            let target = if collection == Some("pools") {
                zfs_pool_command_target(action, target)
            } else {
                target.unwrap_or("<target>")
            };
            let rebalance = rebalance_command(
                collection,
                action.context.fs_type.as_deref(),
                target,
                &action.context.property_assignments,
            );
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
                    scrub_command(collection, action.context.fs_type.as_deref(), target),
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
            let target = if collection == Some("caches") {
                cache_target.unwrap_or("<cache-device>")
            } else {
                target.unwrap_or("<target>")
            };
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
                luks_keyslot_property_command(action, property)
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
                let property_target = if collection == Some("pools") {
                    action.context.name.as_deref().unwrap_or(target)
                } else {
                    target
                };
                set_property_command(
                    collection,
                    property_target,
                    property,
                    &property_assignment,
                    action.context.cache_set_uuid.as_deref(),
                )
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
            let snapshot = snapshot_rescan_identity(action, "<snapshot>");
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
            let target = action
                .context
                .name
                .as_deref()
                .or(target)
                .unwrap_or("<zfs-pool>");
            let device = action.context.device.as_deref();
            let devices = pool_create_devices(device, &action.context.devices);
            let mut commands = zfs_pool_preflight_commands(&devices);
            commands.push(zfs_pool_create_command(
                target,
                &devices,
                &action.context.property_assignments,
            ));
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
                        action.context.options.as_deref(),
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
            if label == "zfs" {
                return (
                    vec![
                        disk_nix_inspect_command(
                            disk,
                            "<disk>",
                            "disk path",
                            "inspect disk identity, signatures, and existing consumers before raw ZFS initialization",
                        ),
                        disk_wipe_signatures_command(disk),
                        partition_probe_command(disk),
                    ],
                    vec![
                        "raw ZFS disks do not receive a parted partition table".to_string(),
                        "zpool create writes ZFS labels to the reviewed whole-disk device"
                            .to_string(),
                        "prefer importing an existing pool when the disk already contains ZFS labels"
                            .to_string(),
                    ],
                    true,
                );
            }
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
            let mut commands = vec![disk_nix_inspect_command(
                disk,
                "<disk>",
                "disk path",
                "inspect disk identity and existing partition table before creation",
            )];
            if disk.is_some_and(|disk| disk.starts_with("/dev/md/"))
                && action.context.partition_number.as_deref() == Some("1")
            {
                commands.push(disk_create_label_command(disk, "gpt"));
            }
            commands.extend([
                partition_create_command(disk, partition_type, start, end),
                partition_probe_command(disk),
                partition_table_reread_command(disk),
                disk_nix_inspect_command(
                    partition_target,
                    "<partition>",
                    "partition path",
                    "verify the new partition node before creating higher layers",
                ),
            ]);
            (
                commands,
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
                    swapoff_best_effort_command(
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
            let target = zfs_pool_command_target(action, target);
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
            let target = zfs_pool_command_target(action, target);
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
            let target = zfs_pool_command_target(action, target);
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
                    vec![command_with_readiness(
                        ["<snapshot-rename-tool>", snapshot, rename_to],
                        true,
                        CommandReadiness::NeedsDomainImplementation,
                        ["ZFS snapshot name or Btrfs snapshot path"],
                        "rename the snapshot after selecting the target-specific snapshot tool",
                    )],
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
                    nvme_list_subsystems_command("inspect NVMe subsystem paths before deletion"),
                    nvme_detach_namespace_command(controller, namespace_id, controllers),
                    nvme_delete_namespace_command(controller, namespace_id),
                    nvme_namespace_rescan_command(controller),
                    nvme_list_subsystems_command("verify NVMe subsystem paths after deletion"),
                ],
                vec![
                    "detach namespace consumers and migrate data before delete-ns".to_string(),
                    "prefer detach without delete when target-side namespace data must remain"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Detach if collection == Some("nvmeNamespaces") => {
            let controller = nvme_controller_target(action);
            let namespace_id = action.context.namespace_id.as_deref();
            let controllers = action.context.controllers.as_deref();
            (
                vec![
                    nvme_list_namespaces_command(
                        controller,
                        "inspect NVMe namespace inventory before detach",
                    ),
                    nvme_list_subsystems_command("inspect NVMe subsystem paths before detach"),
                    nvme_detach_namespace_command(controller, namespace_id, controllers),
                    nvme_namespace_rescan_command(controller),
                    nvme_list_namespaces_command(
                        controller,
                        "verify NVMe namespace inventory after detach",
                    ),
                    nvme_list_subsystems_command("verify NVMe subsystem paths after detach"),
                ],
                vec![
                    "detach removes controller access without deleting the namespace".to_string(),
                    "unmount filesystems and deactivate dependent mappings before detach"
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
                    vec![command_with_readiness(
                        ["<snapshot-rollback-tool>", snapshot],
                        true,
                        CommandReadiness::NeedsDomainImplementation,
                        ["ZFS snapshot name"],
                        "roll back the snapshot after selecting a concrete ZFS snapshot name",
                    )],
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
            } else if is_btrfs_snapshot_pair(snapshot, target) {
                (
                    vec![
                        command(
                            ["btrfs", "subvolume", "show", snapshot],
                            false,
                            "inspect source Btrfs snapshot subvolume before clone",
                        ),
                        snapshot_command(
                            Some("snapshots"),
                            snapshot,
                            target,
                            action.context.read_only.unwrap_or(false),
                        ),
                    ],
                    vec![
                        "use the cloned subvolume for inspection, migration, or rollback rehearsal"
                            .to_string(),
                        "delete temporary Btrfs clone subvolumes after validation when they are no longer needed"
                            .to_string(),
                    ],
                    true,
                )
            } else {
                (
                    vec![command_with_readiness(
                        ["<snapshot-clone-tool>", snapshot, target],
                        true,
                        CommandReadiness::NeedsDomainImplementation,
                        ["ZFS snapshot name or Btrfs snapshot path"],
                        "clone the snapshot after selecting a concrete ZFS snapshot name or Btrfs snapshot path",
                    )],
                    vec![
                        "snapshot clone command is rendered for unambiguous ZFS snapshot names or absolute Btrfs snapshot paths"
                            .to_string(),
                    ],
                    true,
                )
            }
        }
        Operation::RemoveDevice if collection == Some("pools") => {
            let target = zfs_pool_command_target(action, target);
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
            let fs_type = action.context.fs_type.as_deref();
            let target = target.unwrap_or(match fs_type {
                Some("bcachefs") => "<bcachefs-mountpoint>",
                _ => "<btrfs-filesystem>",
            });
            let device = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "remove-device"));
            if fs_type == Some("bcachefs") {
                (
                    vec![
                        bcachefs_usage_command(
                            target,
                            "inspect bcachefs allocation and free space before device removal",
                        ),
                        bcachefs_rereplicate_command(target),
                        bcachefs_remove_device_command(target, device),
                    ],
                    vec![
                        "remove a bcachefs device only when remaining replicas and capacity are sufficient"
                            .to_string(),
                        "rereplicate or migrate data before removing the reviewed member"
                            .to_string(),
                    ],
                    true,
                )
            } else {
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
        }
        Operation::RemoveDevice if collection == Some("caches") => {
            let target = cache_target.unwrap_or("<cache-device>");
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
            let target = cache_target.unwrap_or("<cache-device>");
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
            commands.push(lsscsi_lun_inventory_command(
                "inspect host-visible LUN transport and size before detaching paths",
            ));
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
            vec![unimplemented_action_command(action, collection, target)],
            vec!["no domain-specific command plan is generated for this action yet".to_string()],
            true,
        ),
    }
}

fn zfs_pool_command_target<'a>(action: &'a PlannedAction, fallback: Option<&'a str>) -> &'a str {
    action
        .context
        .name
        .as_deref()
        .or(fallback)
        .unwrap_or("<zfs-pool>")
}

fn target_lun_commands(action: &PlannedAction, target: &str) -> Vec<ExecutionCommand> {
    if target_lun_lio_provider(action)
        && matches!(
            action.operation,
            Operation::Create
                | Operation::Attach
                | Operation::Detach
                | Operation::Destroy
                | Operation::Rescan
                | Operation::Grow
                | Operation::SetProperty
        )
    {
        return target_lun_lio_commands(action, target);
    }
    if target_lun_tgt_provider(action)
        && matches!(
            action.operation,
            Operation::Create
                | Operation::Attach
                | Operation::Detach
                | Operation::Destroy
                | Operation::Rescan
                | Operation::Grow
                | Operation::SetProperty
        )
    {
        return target_lun_tgt_commands(action, target);
    }
    if target_lun_scst_provider(action)
        && matches!(
            action.operation,
            Operation::Create
                | Operation::Attach
                | Operation::Detach
                | Operation::Destroy
                | Operation::Rescan
                | Operation::Grow
                | Operation::SetProperty
        )
    {
        return target_lun_scst_commands(action, target);
    }

    let operation = operation_name(action.operation);
    let desired_size = action.context.desired_size.as_deref();
    vec![
        target_lun_inventory_command(
            action,
            target,
            "inspect target-side LUN inventory before provider mutation",
        ),
        target_lun_provider_command(action, target, &operation, desired_size),
        target_lun_inventory_command(
            action,
            target,
            "inspect target-side LUN inventory after provider mutation",
        ),
    ]
}

fn target_lun_verification_commands(action: &PlannedAction, target: &str) -> Vec<ExecutionCommand> {
    if target_lun_lio_provider(action) {
        let mut commands = vec![target_lun_lio_inventory_command(
            target,
            "verify LIO target-side LUN inventory after provider action",
        )];
        if let Some(portal) = action.context.portal.as_deref() {
            commands.push(command_vec(
                vec![
                    "targetcli".to_string(),
                    target_lun_lio_tpg_path(target),
                    "ls".to_string(),
                ],
                false,
                &format!("verify LIO portal mapping for {portal} after provider action"),
            ));
        }
        if action.operation == Operation::Grow {
            commands.extend(target_lun_generic_host_verification_commands(target));
        }
        return commands;
    }
    if target_lun_tgt_provider(action) {
        let mut commands = vec![target_lun_tgt_inventory_command(
            action,
            "verify Linux tgt target-side LUN inventory after tgtadm action",
        )];
        if action.operation == Operation::Grow {
            commands.extend(target_lun_generic_host_verification_commands(target));
        }
        return commands;
    }
    if target_lun_scst_provider(action) {
        return vec![target_lun_scst_target_inventory_command(
            action,
            target,
            "verify SCST target-side LUN inventory after scstadmin action",
        )];
    }

    let mut commands = vec![target_lun_inventory_command(
        action,
        target,
        "verify target-side LUN inventory after provider action",
    )];
    if let Some(portal) = action.context.portal.as_deref() {
        let mut command = command_vec_with_readiness(
            vec![
                target_lun_provider_program(action),
                "show-mapping".to_string(),
                "--portal".to_string(),
                portal.to_string(),
                "--target".to_string(),
                target.to_string(),
            ],
            false,
            CommandReadiness::NeedsDomainImplementation,
            [target_lun_provider_unresolved(action)],
            "verify target-side portal mapping after provider action",
        );
        command.provider_capabilities = target_lun_provider_capabilities(action);
        commands.push(command);
    }
    commands.extend(target_lun_generic_host_verification_commands(target));
    commands
}

fn target_lun_generic_host_verification_commands(target: &str) -> Vec<ExecutionCommand> {
    vec![
        lsscsi_lun_inventory_command(
            "verify host-visible SCSI LUN paths after target-side provider action",
        ),
        command(
            ["multipath", "-ll"],
            false,
            "verify host multipath path grouping after target-side provider action",
        ),
        command_vec(
            ["disk-nix", "inspect", target, "--json"],
            false,
            "verify modeled target-side LUN graph state and consumers after provider action",
        ),
    ]
}

fn target_lun_lio_provider(action: &PlannedAction) -> bool {
    action.context.provider.as_deref().is_some_and(|provider| {
        matches!(
            provider.to_ascii_lowercase().as_str(),
            "lio" | "linux-lio" | "targetcli" | "targetcli-fb"
        )
    })
}

fn target_lun_tgt_provider(action: &PlannedAction) -> bool {
    action.context.provider.as_deref().is_some_and(|provider| {
        matches!(
            provider.to_ascii_lowercase().as_str(),
            "tgt" | "linux-tgt" | "tgtadm"
        )
    })
}

fn target_lun_scst_provider(action: &PlannedAction) -> bool {
    action.context.provider.as_deref().is_some_and(|provider| {
        matches!(
            provider.to_ascii_lowercase().as_str(),
            "scst" | "linux-scst" | "iscsi-scst" | "scstadmin"
        )
    })
}

fn target_lun_lio_commands(action: &PlannedAction, target: &str) -> Vec<ExecutionCommand> {
    let mut commands = vec![if action.operation == Operation::Create {
        target_lun_lio_inventory_root_command(
            "inspect LIO target-side inventory before targetcli mutation",
        )
    } else {
        target_lun_lio_inventory_command(
            target,
            "inspect LIO target-side inventory before targetcli mutation",
        )
    }];
    let backstore = target_lun_lio_backstore_name(action, target);
    let tpg = target_lun_lio_tpg_path(target);
    let lun = target_lun_lio_lun(action);

    match action.operation {
        Operation::Create => {
            commands.push(target_lun_lio_backstore_create_command(
                action,
                &backstore,
                "create LIO block backstore for the reviewed target-side LUN",
            ));
            commands.push(command_vec(
                vec![
                    "targetcli".to_string(),
                    "/iscsi".to_string(),
                    "create".to_string(),
                    target.to_string(),
                ],
                true,
                "create or ensure the reviewed LIO iSCSI target exists",
            ));
            commands.push(target_lun_lio_lun_create_command(
                action,
                &tpg,
                &backstore,
                &lun,
                "map the reviewed LIO backstore as a target LUN",
            ));
            target_lun_lio_acl_commands(action, &tpg, true, &mut commands);
            commands.push(target_lun_lio_saveconfig_command());
        }
        Operation::Attach => {
            if action.context.device.is_some() {
                commands.push(target_lun_lio_lun_create_command(
                    action,
                    &tpg,
                    &backstore,
                    &lun,
                    "map an existing LIO backstore as a target LUN",
                ));
            }
            target_lun_lio_acl_commands(action, &tpg, true, &mut commands);
            commands.push(target_lun_lio_saveconfig_command());
        }
        Operation::Detach => {
            target_lun_lio_acl_commands(action, &tpg, false, &mut commands);
            commands.push(target_lun_lio_lun_delete_command(
                &tpg,
                &lun,
                "unmap the reviewed LIO target LUN without deleting the backstore",
            ));
            commands.push(target_lun_lio_saveconfig_command());
        }
        Operation::Destroy => {
            target_lun_lio_acl_commands(action, &tpg, false, &mut commands);
            commands.push(target_lun_lio_lun_delete_command(
                &tpg,
                &lun,
                "unmap the reviewed LIO target LUN before target removal",
            ));
            commands.push(command_vec(
                vec![
                    "targetcli".to_string(),
                    "/iscsi".to_string(),
                    "delete".to_string(),
                    target.to_string(),
                ],
                true,
                "remove the reviewed LIO iSCSI target",
            ));
            commands.push(target_lun_lio_backstore_delete_command(
                action,
                &backstore,
                "remove the reviewed LIO block backstore after target removal",
            ));
            commands.push(target_lun_lio_saveconfig_command());
        }
        Operation::Rescan => {}
        Operation::Grow => {
            commands.push(target_lun_lio_backstore_inventory_command(
                action,
                &backstore,
                "inspect the reviewed LIO backstore before target-side LUN growth",
            ));
            if let Some(command) = target_lun_lio_forced_backstore_resize_command(
                action,
                target,
                &backstore,
                "force the reviewed LIO fileio backstore to the declared size before target refresh",
            ) {
                commands.push(command);
            }
            commands.push(target_lun_lio_backing_size_command(
                action,
                "validate the reviewed LIO backing object exposes the grown capacity",
            ));
            commands.push(target_lun_lio_lun_inventory_command(
                &tpg,
                "inspect LIO TPG LUN mappings before initiator capacity refresh",
            ));
            commands.push(target_lun_lio_saveconfig_command());
            commands.push(target_lun_lio_lun_inventory_command(
                &tpg,
                "inspect LIO TPG LUN mappings after target-side grow refresh",
            ));
        }
        Operation::SetProperty => {
            commands.push(target_lun_lio_backstore_inventory_command(
                action,
                &backstore,
                "inspect the reviewed LIO backstore before target-side LUN property update",
            ));
            if let Some(command) = target_lun_lio_property_command(
                action,
                &backstore,
                "update the reviewed LIO backstore property",
            ) {
                commands.push(command);
                commands.push(target_lun_lio_saveconfig_command());
            } else {
                commands.push(target_lun_provider_command(
                    action,
                    target,
                    "set-property",
                    action.context.desired_size.as_deref(),
                ));
            }
        }
        _ => {}
    }

    commands.push(target_lun_lio_inventory_command(
        target,
        "inspect LIO target-side inventory after targetcli mutation",
    ));
    commands
}

fn target_lun_lio_inventory_command(target: &str, note: &str) -> ExecutionCommand {
    command_vec(
        vec![
            "targetcli".to_string(),
            target_lun_lio_target_path(target),
            "ls".to_string(),
        ],
        false,
        note,
    )
}

fn target_lun_lio_inventory_root_command(note: &str) -> ExecutionCommand {
    command_vec(
        vec![
            "targetcli".to_string(),
            "/iscsi".to_string(),
            "ls".to_string(),
        ],
        false,
        note,
    )
}

fn target_lun_lio_backstore_inventory_command(
    action: &PlannedAction,
    backstore: &str,
    note: &str,
) -> ExecutionCommand {
    let (backstore_path, readiness, unresolved_inputs) = if action.context.device.is_some() {
        (
            target_lun_lio_backstore_path(action, backstore),
            CommandReadiness::Ready,
            Vec::new(),
        )
    } else {
        (
            "/backstores/block/<backstore>".to_string(),
            CommandReadiness::NeedsDomainImplementation,
            vec!["LIO backstore name or backing device for inventory".to_string()],
        )
    };
    command_vec_with_readiness(
        vec!["targetcli".to_string(), backstore_path, "ls".to_string()],
        false,
        readiness,
        unresolved_inputs,
        note,
    )
}

fn target_lun_lio_lun_inventory_command(tpg: &str, note: &str) -> ExecutionCommand {
    command_vec(
        vec![
            "targetcli".to_string(),
            format!("{tpg}/luns"),
            "ls".to_string(),
        ],
        false,
        note,
    )
}

fn target_lun_lio_backing_size_command(action: &PlannedAction, note: &str) -> ExecutionCommand {
    let fileio_regular_file = lio_backstore_type(action).as_deref() == Some("fileio")
        && action
            .context
            .device
            .as_deref()
            .is_none_or(|device| !device.starts_with("/dev/"));
    if fileio_regular_file {
        return match action.context.device.as_deref() {
            Some(path) => command_vec(
                vec![
                    "stat".to_string(),
                    "--format=%s".to_string(),
                    path.to_string(),
                ],
                false,
                note,
            ),
            None => command_vec_with_readiness(
                vec![
                    "stat".to_string(),
                    "--format=%s".to_string(),
                    "<fileio-backing-file>".to_string(),
                ],
                false,
                CommandReadiness::NeedsDomainImplementation,
                ["LIO fileio backing file for capacity validation"],
                note,
            ),
        };
    }

    match action.context.device.as_deref() {
        Some(device) => command_vec(
            vec![
                "blockdev".to_string(),
                "--getsize64".to_string(),
                device.to_string(),
            ],
            false,
            note,
        ),
        None => command_vec_with_readiness(
            vec![
                "blockdev".to_string(),
                "--getsize64".to_string(),
                "<backing-block-device-or-file>".to_string(),
            ],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["LIO backing block device or file for capacity validation"],
            note,
        ),
    }
}

fn target_lun_lio_backstore_path(action: &PlannedAction, backstore: &str) -> String {
    let backstore_type = lio_backstore_type(action).unwrap_or_else(|| "block".to_string());
    format!("/backstores/{backstore_type}/{backstore}")
}

fn target_lun_lio_forced_backstore_resize_command(
    action: &PlannedAction,
    target: &str,
    backstore: &str,
    note: &str,
) -> Option<ExecutionCommand> {
    let backstore_type = lio_backstore_type(action)?;
    match backstore_type.as_str() {
        "fileio" => Some(target_lun_lio_fileio_resize_command(
            action, backstore, note,
        )),
        "block" => None,
        _ => Some(target_lun_lio_backstore_resize_handoff_command(
            action,
            target,
            backstore,
            &backstore_type,
        )),
    }
}

fn target_lun_lio_backstore_resize_handoff_command(
    action: &PlannedAction,
    target: &str,
    backstore: &str,
    backstore_type: &str,
) -> ExecutionCommand {
    let mut argv = vec![
        target_lun_provider_program(action),
        "grow-lio-backstore".to_string(),
        "--target".to_string(),
        target.to_string(),
        "--backstore-type".to_string(),
        backstore_type.to_string(),
        "--backstore-name".to_string(),
        backstore.to_string(),
    ];
    if let Some(lun) = action.context.lun.as_deref() {
        argv.push("--lun".to_string());
        argv.push(lun.to_string());
    }
    if let Some(device) = action.context.device.as_deref() {
        argv.push("--source".to_string());
        argv.push(device.to_string());
    }
    if let Some(size) = action.context.desired_size.as_deref() {
        argv.push("--size".to_string());
        argv.push(size.to_string());
    }
    let mut command = command_vec_with_readiness(
        argv,
        true,
        CommandReadiness::NeedsDomainImplementation,
        [format!(
            "provider-specific LIO {backstore_type} backstore resize primitive"
        )],
        "handoff LIO backstore resize to a reviewed site provider adapter",
    );
    command.provider_capabilities = target_lun_provider_capabilities(action);
    command
}

fn target_lun_lio_fileio_resize_command(
    action: &PlannedAction,
    backstore: &str,
    note: &str,
) -> ExecutionCommand {
    let mut unresolved_inputs = Vec::new();
    let size = match action.context.desired_size.as_deref() {
        Some(size) => size.to_string(),
        None => {
            unresolved_inputs.push("desired LIO fileio backstore size".to_string());
            "<size>".to_string()
        }
    };
    let path = match action.context.device.as_deref() {
        Some(path) => path.to_string(),
        None => {
            unresolved_inputs.push("LIO fileio backing file path".to_string());
            format!("<fileio-backing-file-for-{backstore}>")
        }
    };
    command_vec_with_readiness(
        vec!["truncate".to_string(), "--size".to_string(), size, path],
        true,
        if unresolved_inputs.is_empty() {
            CommandReadiness::Ready
        } else {
            CommandReadiness::NeedsDomainImplementation
        },
        unresolved_inputs,
        note,
    )
}

fn lio_backstore_type(action: &PlannedAction) -> Option<String> {
    action
        .context
        .backstore_type
        .as_deref()
        .map(|backstore_type| {
            backstore_type
                .trim()
                .trim_matches('"')
                .replace(['-', '_'], "")
                .to_ascii_lowercase()
        })
}

fn target_lun_lio_backstore_create_command(
    action: &PlannedAction,
    backstore: &str,
    note: &str,
) -> ExecutionCommand {
    let mut argv = vec![
        "targetcli".to_string(),
        "/backstores/block".to_string(),
        "create".to_string(),
        format!("name={backstore}"),
    ];
    let mut unresolved_inputs = Vec::new();
    if let Some(device) = action.context.device.as_deref() {
        argv.push(format!("dev={device}"));
    } else {
        argv.push("dev=<backing-block-device-or-file>".to_string());
        unresolved_inputs.push("LIO backing block device or file".to_string());
    }
    command_vec_with_readiness(
        argv,
        true,
        if unresolved_inputs.is_empty() {
            CommandReadiness::Ready
        } else {
            CommandReadiness::NeedsDomainImplementation
        },
        unresolved_inputs,
        note,
    )
}

fn target_lun_lio_lun_create_command(
    action: &PlannedAction,
    tpg: &str,
    backstore: &str,
    lun: &str,
    note: &str,
) -> ExecutionCommand {
    let mut unresolved_inputs = Vec::new();
    let backstore_path = if action.context.device.is_some() {
        format!("/backstores/block/{backstore}")
    } else {
        unresolved_inputs.push("LIO backing block device or file".to_string());
        "/backstores/block/<backstore>".to_string()
    };
    command_vec_with_readiness(
        vec![
            "targetcli".to_string(),
            format!("{tpg}/luns"),
            "create".to_string(),
            backstore_path,
            format!("lun={lun}"),
        ],
        true,
        if unresolved_inputs.is_empty() {
            CommandReadiness::Ready
        } else {
            CommandReadiness::NeedsDomainImplementation
        },
        unresolved_inputs,
        note,
    )
}

fn target_lun_lio_lun_delete_command(tpg: &str, lun: &str, note: &str) -> ExecutionCommand {
    command_vec(
        vec![
            "targetcli".to_string(),
            format!("{tpg}/luns"),
            "delete".to_string(),
            lun.to_string(),
        ],
        true,
        note,
    )
}

fn target_lun_lio_backstore_delete_command(
    action: &PlannedAction,
    backstore: &str,
    note: &str,
) -> ExecutionCommand {
    let (backstore, readiness, unresolved_inputs) = if action.context.device.is_some() {
        (backstore.to_string(), CommandReadiness::Ready, Vec::new())
    } else {
        (
            "<backstore-name>".to_string(),
            CommandReadiness::NeedsDomainImplementation,
            vec!["LIO backstore name or backing device for removal".to_string()],
        )
    };
    command_vec_with_readiness(
        vec![
            "targetcli".to_string(),
            "/backstores/block".to_string(),
            "delete".to_string(),
            backstore,
        ],
        true,
        readiness,
        unresolved_inputs,
        note,
    )
}

fn target_lun_lio_property_command(
    action: &PlannedAction,
    backstore: &str,
    note: &str,
) -> Option<ExecutionCommand> {
    let property = action.context.property.as_deref()?;
    let attribute = target_lun_lio_attribute_for_property(property)?;
    let mut unresolved_inputs = Vec::new();
    let value = match action.context.property_value.as_deref() {
        Some(value) => match normalize_lio_bool_attribute_value(value) {
            Some(value) => value.to_string(),
            None => {
                unresolved_inputs.push("boolean LIO write-cache property value".to_string());
                "<0-or-1>".to_string()
            }
        },
        None => {
            unresolved_inputs.push("boolean LIO write-cache property value".to_string());
            "<0-or-1>".to_string()
        }
    };
    let backstore_path = if action.context.device.is_some() {
        format!("/backstores/block/{backstore}")
    } else {
        unresolved_inputs
            .push("LIO backstore name or backing device for property update".to_string());
        "/backstores/block/<backstore>".to_string()
    };

    Some(command_vec_with_readiness(
        vec![
            "targetcli".to_string(),
            backstore_path,
            "set".to_string(),
            "attribute".to_string(),
            format!("{attribute}={value}"),
        ],
        true,
        if unresolved_inputs.is_empty() {
            CommandReadiness::Ready
        } else {
            CommandReadiness::NeedsDomainImplementation
        },
        unresolved_inputs,
        note,
    ))
}

fn target_lun_lio_attribute_for_property(property: &str) -> Option<&'static str> {
    match property
        .trim()
        .trim_start_matches("lio.")
        .replace(['-', '_'], "")
        .to_ascii_lowercase()
        .as_str()
    {
        "writecache" | "emulatewritecache" => Some("emulate_write_cache"),
        _ => None,
    }
}

fn normalize_lio_bool_attribute_value(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" | "enabled" | "enable" => Some("1"),
        "0" | "false" | "no" | "off" | "disabled" | "disable" => Some("0"),
        _ => None,
    }
}

fn target_lun_lio_acl_commands(
    action: &PlannedAction,
    tpg: &str,
    create: bool,
    commands: &mut Vec<ExecutionCommand>,
) {
    let mut initiators = Vec::new();
    if let Some(client) = action.context.client.as_deref() {
        initiators.push(client.to_string());
    }
    initiators.extend(action.context.devices.iter().cloned());

    if initiators.is_empty() {
        commands.push(command_vec_with_readiness(
            vec![
                "targetcli".to_string(),
                format!("{tpg}/acls"),
                if create { "create" } else { "delete" }.to_string(),
                "<initiator-iqn>".to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["initiator IQN for LIO target ACL"],
            if create {
                "map the LIO target LUN to a reviewed initiator ACL"
            } else {
                "remove the reviewed initiator ACL from the LIO target"
            },
        ));
        return;
    }

    for initiator in initiators {
        commands.push(command_vec(
            vec![
                "targetcli".to_string(),
                format!("{tpg}/acls"),
                if create { "create" } else { "delete" }.to_string(),
                initiator,
            ],
            true,
            if create {
                "map the LIO target LUN to the reviewed initiator ACL"
            } else {
                "remove the reviewed initiator ACL from the LIO target"
            },
        ));
    }
}

fn target_lun_lio_saveconfig_command() -> ExecutionCommand {
    command_vec(
        vec!["targetcli".to_string(), "saveconfig".to_string()],
        true,
        "persist reviewed LIO target configuration",
    )
}

fn target_lun_lio_target_path(target: &str) -> String {
    format!("/iscsi/{target}")
}

fn target_lun_lio_tpg_path(target: &str) -> String {
    format!("{}/tpg1", target_lun_lio_target_path(target))
}

fn target_lun_lio_lun(action: &PlannedAction) -> String {
    action.context.lun.as_deref().unwrap_or("0").to_string()
}

fn target_lun_lio_backstore_name(action: &PlannedAction, target: &str) -> String {
    let raw = action
        .context
        .device
        .as_deref()
        .or(action.context.name.as_deref())
        .unwrap_or(target);
    let sanitized: String = raw
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "disk_nix_lun".to_string()
    } else {
        sanitized
    }
}

fn target_lun_tgt_commands(action: &PlannedAction, target: &str) -> Vec<ExecutionCommand> {
    let mut commands = vec![target_lun_tgt_inventory_command(
        action,
        "inspect Linux tgt target-side inventory before tgtadm mutation",
    )];
    match action.operation {
        Operation::Create => {
            commands.push(target_lun_tgt_target_command(
                action,
                target,
                "new",
                "create or ensure the reviewed Linux tgt iSCSI target exists",
            ));
            commands.push(target_lun_tgt_lun_command(
                action,
                "new",
                "create the reviewed Linux tgt logical unit with the declared backing store",
            ));
            target_lun_tgt_bind_commands(action, true, &mut commands);
        }
        Operation::Attach => {
            if action.context.device.is_some() {
                commands.push(target_lun_tgt_lun_command(
                    action,
                    "new",
                    "map the reviewed backing store as a Linux tgt logical unit",
                ));
            }
            target_lun_tgt_bind_commands(action, true, &mut commands);
        }
        Operation::Detach => {
            target_lun_tgt_bind_commands(action, false, &mut commands);
            commands.push(target_lun_tgt_lun_command(
                action,
                "delete",
                "unmap the reviewed Linux tgt logical unit without deleting target-side data",
            ));
        }
        Operation::Destroy => {
            target_lun_tgt_bind_commands(action, false, &mut commands);
            commands.push(target_lun_tgt_lun_command(
                action,
                "delete",
                "unmap the reviewed Linux tgt logical unit before target removal",
            ));
            commands.push(target_lun_tgt_target_command(
                action,
                target,
                "delete",
                "remove the reviewed Linux tgt iSCSI target",
            ));
        }
        Operation::Rescan => {}
        Operation::Grow => {
            commands.push(target_lun_tgt_backing_size_command(
                action,
                "validate the reviewed Linux tgt backing object exposes the grown capacity",
            ));
            commands.push(target_lun_tgt_logical_unit_refresh_command(
                action,
                "refresh the reviewed Linux tgt logical unit after backing capacity growth",
            ));
            commands.push(target_lun_tgt_persistence_snapshot_command());
            commands.push(target_lun_tgt_inventory_command(
                action,
                "inspect Linux tgt target-side inventory after capacity refresh",
            ));
        }
        Operation::SetProperty => {
            commands.push(target_lun_tgt_property_command(
                action,
                "update the reviewed Linux tgt logical-unit property",
            ));
        }
        _ => {}
    }
    commands.push(target_lun_tgt_inventory_command(
        action,
        "inspect Linux tgt target-side inventory after tgtadm mutation",
    ));
    commands
}

fn target_lun_tgt_inventory_command(action: &PlannedAction, note: &str) -> ExecutionCommand {
    let mut argv = vec![
        "tgtadm".to_string(),
        "--lld".to_string(),
        "iscsi".to_string(),
        "--mode".to_string(),
        "target".to_string(),
        "--op".to_string(),
        "show".to_string(),
    ];
    if let Some(target_id) = action.context.target_id.as_deref() {
        argv.push("--tid".to_string());
        argv.push(target_id.to_string());
    }
    command_vec(argv, false, note)
}

fn target_lun_tgt_target_command(
    action: &PlannedAction,
    target: &str,
    op: &str,
    note: &str,
) -> ExecutionCommand {
    let (target_id, mut unresolved_inputs) = target_lun_tgt_target_id(action);
    let mut argv = vec![
        "tgtadm".to_string(),
        "--lld".to_string(),
        "iscsi".to_string(),
        "--mode".to_string(),
        "target".to_string(),
        "--op".to_string(),
        op.to_string(),
        "--tid".to_string(),
        target_id,
    ];
    if op == "new" {
        argv.push("--targetname".to_string());
        argv.push(target.to_string());
    }
    command_vec_with_readiness(
        argv,
        true,
        if unresolved_inputs.is_empty() {
            CommandReadiness::Ready
        } else {
            CommandReadiness::NeedsDomainImplementation
        },
        std::mem::take(&mut unresolved_inputs),
        note,
    )
}

fn target_lun_tgt_lun_command(action: &PlannedAction, op: &str, note: &str) -> ExecutionCommand {
    let (target_id, mut unresolved_inputs) = target_lun_tgt_target_id(action);
    let (lun, lun_unresolved) = target_lun_tgt_lun(action);
    unresolved_inputs.extend(lun_unresolved);
    let mut argv = vec![
        "tgtadm".to_string(),
        "--lld".to_string(),
        "iscsi".to_string(),
        "--mode".to_string(),
        "logicalunit".to_string(),
        "--op".to_string(),
        op.to_string(),
        "--tid".to_string(),
        target_id,
        "--lun".to_string(),
        lun,
    ];
    if op == "new" {
        match action.context.device.as_deref() {
            Some(device) => {
                argv.push("--backing-store".to_string());
                argv.push(device.to_string());
            }
            None => {
                argv.push("--backing-store".to_string());
                argv.push("<backing-block-device-or-file>".to_string());
                unresolved_inputs.push("Linux tgt backing store path".to_string());
            }
        }
    }
    command_vec_with_readiness(
        argv,
        true,
        if unresolved_inputs.is_empty() {
            CommandReadiness::Ready
        } else {
            CommandReadiness::NeedsDomainImplementation
        },
        unresolved_inputs,
        note,
    )
}

fn target_lun_tgt_backing_size_command(action: &PlannedAction, note: &str) -> ExecutionCommand {
    match action.context.device.as_deref() {
        Some(device) => command_vec(
            vec![
                "blockdev".to_string(),
                "--getsize64".to_string(),
                device.to_string(),
            ],
            false,
            note,
        ),
        None => command_vec_with_readiness(
            vec![
                "blockdev".to_string(),
                "--getsize64".to_string(),
                "<backing-block-device-or-file>".to_string(),
            ],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["Linux tgt backing store path for capacity validation"],
            note,
        ),
    }
}

fn target_lun_tgt_logical_unit_refresh_command(
    action: &PlannedAction,
    note: &str,
) -> ExecutionCommand {
    let (target_id, mut unresolved_inputs) = target_lun_tgt_target_id(action);
    let (lun, lun_unresolved) = target_lun_tgt_lun(action);
    unresolved_inputs.extend(lun_unresolved);
    command_vec_with_readiness(
        vec![
            "tgtadm".to_string(),
            "--lld".to_string(),
            "iscsi".to_string(),
            "--mode".to_string(),
            "logicalunit".to_string(),
            "--op".to_string(),
            "update".to_string(),
            "--tid".to_string(),
            target_id,
            "--lun".to_string(),
            lun,
            "--params".to_string(),
            "online=1".to_string(),
        ],
        true,
        if unresolved_inputs.is_empty() {
            CommandReadiness::Ready
        } else {
            CommandReadiness::NeedsDomainImplementation
        },
        unresolved_inputs,
        note,
    )
}

fn target_lun_tgt_persistence_snapshot_command() -> ExecutionCommand {
    command_vec(
        vec!["tgt-admin".to_string(), "--dump".to_string()],
        false,
        "capture Linux tgt runtime configuration for persistent target state review",
    )
}

fn target_lun_tgt_property_command(action: &PlannedAction, note: &str) -> ExecutionCommand {
    let (target_id, mut unresolved_inputs) = target_lun_tgt_target_id(action);
    let (lun, lun_unresolved) = target_lun_tgt_lun(action);
    unresolved_inputs.extend(lun_unresolved);
    let property = match action.context.property.as_deref() {
        Some(property) => property.to_string(),
        None => {
            unresolved_inputs.push("Linux tgt logical-unit property name".to_string());
            "<property>".to_string()
        }
    };
    let value = match action.context.property_value.as_deref() {
        Some(value) => value.to_string(),
        None => {
            unresolved_inputs.push("Linux tgt logical-unit property value".to_string());
            "<value>".to_string()
        }
    };

    command_vec_with_readiness(
        vec![
            "tgtadm".to_string(),
            "--lld".to_string(),
            "iscsi".to_string(),
            "--mode".to_string(),
            "logicalunit".to_string(),
            "--op".to_string(),
            "update".to_string(),
            "--tid".to_string(),
            target_id,
            "--lun".to_string(),
            lun,
            "--name".to_string(),
            property,
            "--value".to_string(),
            value,
        ],
        true,
        if unresolved_inputs.is_empty() {
            CommandReadiness::Ready
        } else {
            CommandReadiness::NeedsDomainImplementation
        },
        unresolved_inputs,
        note,
    )
}

fn target_lun_tgt_bind_commands(
    action: &PlannedAction,
    bind: bool,
    commands: &mut Vec<ExecutionCommand>,
) {
    let mut initiators = Vec::new();
    if let Some(client) = action.context.client.as_deref() {
        initiators.push(client.to_string());
    }
    initiators.extend(action.context.devices.iter().cloned());

    if initiators.is_empty() {
        let (target_id, mut unresolved_inputs) = target_lun_tgt_target_id(action);
        unresolved_inputs.push("Linux tgt initiator address or ALL ACL value".to_string());
        commands.push(command_vec_with_readiness(
            vec![
                "tgtadm".to_string(),
                "--lld".to_string(),
                "iscsi".to_string(),
                "--mode".to_string(),
                "target".to_string(),
                "--op".to_string(),
                if bind { "bind" } else { "unbind" }.to_string(),
                "--tid".to_string(),
                target_id,
                "--initiator-address".to_string(),
                "<initiator-address-or-ALL>".to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            unresolved_inputs,
            if bind {
                "bind the Linux tgt target to a reviewed initiator address"
            } else {
                "unbind the reviewed initiator address from the Linux tgt target"
            },
        ));
        return;
    }

    for initiator in initiators {
        let (target_id, unresolved_inputs) = target_lun_tgt_target_id(action);
        commands.push(command_vec_with_readiness(
            vec![
                "tgtadm".to_string(),
                "--lld".to_string(),
                "iscsi".to_string(),
                "--mode".to_string(),
                "target".to_string(),
                "--op".to_string(),
                if bind { "bind" } else { "unbind" }.to_string(),
                "--tid".to_string(),
                target_id,
                "--initiator-address".to_string(),
                initiator,
            ],
            true,
            if unresolved_inputs.is_empty() {
                CommandReadiness::Ready
            } else {
                CommandReadiness::NeedsDomainImplementation
            },
            unresolved_inputs,
            if bind {
                "bind the Linux tgt target to the reviewed initiator address"
            } else {
                "unbind the reviewed initiator address from the Linux tgt target"
            },
        ));
    }
}

fn target_lun_tgt_target_id(action: &PlannedAction) -> (String, Vec<String>) {
    match action.context.target_id.as_deref() {
        Some(target_id) => (target_id.to_string(), Vec::new()),
        None => (
            "<tid>".to_string(),
            vec!["Linux tgt numeric target id (targetId or tid)".to_string()],
        ),
    }
}

fn target_lun_tgt_lun(action: &PlannedAction) -> (String, Vec<String>) {
    match action.context.lun.as_deref() {
        Some(lun) => (lun.to_string(), Vec::new()),
        None => (
            "<lun>".to_string(),
            vec!["Linux tgt LUN number".to_string()],
        ),
    }
}

fn target_lun_scst_commands(action: &PlannedAction, target: &str) -> Vec<ExecutionCommand> {
    let mut commands = vec![target_lun_scst_target_inventory_command(
        action,
        target,
        "inspect SCST target-side inventory before scstadmin mutation",
    )];
    let device_name = target_lun_scst_device_name(action, target);

    match action.operation {
        Operation::Create => {
            commands.push(target_lun_scst_open_device_command(
                action,
                &device_name,
                "open the reviewed SCST backing device",
            ));
            commands.push(target_lun_scst_target_command(
                target,
                "add_target",
                "create or ensure the reviewed SCST iSCSI target exists",
            ));
            target_lun_scst_initiator_group_commands(action, target, true, &mut commands);
            commands.push(target_lun_scst_lun_command(
                action,
                target,
                &device_name,
                "add_lun",
                "map the reviewed SCST device as a target LUN",
            ));
            commands.push(target_lun_scst_enable_target_command(
                target,
                "enable the reviewed SCST target after mapping",
            ));
            commands.push(target_lun_scst_write_config_command());
        }
        Operation::Attach => {
            if action.context.device.is_some() {
                commands.push(target_lun_scst_open_device_command(
                    action,
                    &device_name,
                    "open an existing backing object as an SCST device",
                ));
                commands.push(target_lun_scst_lun_command(
                    action,
                    target,
                    &device_name,
                    "add_lun",
                    "map the reviewed SCST device as a target LUN",
                ));
            }
            target_lun_scst_initiator_group_commands(action, target, true, &mut commands);
            commands.push(target_lun_scst_write_config_command());
        }
        Operation::Detach => {
            target_lun_scst_initiator_group_commands(action, target, false, &mut commands);
            commands.push(target_lun_scst_lun_command(
                action,
                target,
                &device_name,
                "rem_lun",
                "unmap the reviewed SCST target LUN without closing the backing device",
            ));
            commands.push(target_lun_scst_write_config_command());
        }
        Operation::Destroy => {
            target_lun_scst_initiator_group_commands(action, target, false, &mut commands);
            commands.push(target_lun_scst_lun_command(
                action,
                target,
                &device_name,
                "rem_lun",
                "unmap the reviewed SCST target LUN before target removal",
            ));
            commands.push(target_lun_scst_target_command(
                target,
                "rem_target",
                "remove the reviewed SCST iSCSI target",
            ));
            commands.push(target_lun_scst_close_device_command(
                action,
                &device_name,
                "close the reviewed SCST backing device after target removal",
            ));
            commands.push(target_lun_scst_write_config_command());
        }
        Operation::Rescan | Operation::Grow => {
            commands.push(target_lun_scst_device_inventory_command(
                action,
                &device_name,
                "inspect the reviewed SCST backing device before resync",
            ));
            commands.push(target_lun_scst_resync_device_command(
                action,
                &device_name,
                "resync SCST cached backing-device size and notify initiators",
            ));
        }
        Operation::SetProperty => {
            commands.push(target_lun_scst_property_command(
                action,
                "update the reviewed SCST LUN attribute",
            ));
            commands.push(target_lun_scst_write_config_command());
        }
        _ => {}
    }

    commands.push(target_lun_scst_target_inventory_command(
        action,
        target,
        "inspect SCST target-side inventory after scstadmin mutation",
    ));
    commands
}

fn target_lun_scst_target_inventory_command(
    _action: &PlannedAction,
    target: &str,
    note: &str,
) -> ExecutionCommand {
    command_vec(
        vec![
            "scstadmin".to_string(),
            "-list_target".to_string(),
            target.to_string(),
            "-driver".to_string(),
            "iscsi".to_string(),
        ],
        false,
        note,
    )
}

fn target_lun_scst_device_inventory_command(
    action: &PlannedAction,
    device_name: &str,
    note: &str,
) -> ExecutionCommand {
    let (device_name, readiness, unresolved_inputs) = target_lun_scst_device_name_readiness(
        action,
        device_name,
        "SCST device name for inventory",
    );
    command_vec_with_readiness(
        vec![
            "scstadmin".to_string(),
            "-list_dev_attr".to_string(),
            device_name,
        ],
        false,
        readiness,
        unresolved_inputs,
        note,
    )
}

fn target_lun_scst_target_command(target: &str, op: &str, note: &str) -> ExecutionCommand {
    command_vec(
        vec![
            "scstadmin".to_string(),
            format!("-{op}"),
            target.to_string(),
            "-driver".to_string(),
            "iscsi".to_string(),
        ],
        true,
        note,
    )
}

fn target_lun_scst_enable_target_command(target: &str, note: &str) -> ExecutionCommand {
    command_vec(
        vec![
            "scstadmin".to_string(),
            "-enable_target".to_string(),
            target.to_string(),
            "-driver".to_string(),
            "iscsi".to_string(),
        ],
        true,
        note,
    )
}

fn target_lun_scst_open_device_command(
    action: &PlannedAction,
    device_name: &str,
    note: &str,
) -> ExecutionCommand {
    let (device_name, mut unresolved_inputs) =
        target_lun_scst_device_name_for_mutation(action, device_name);
    let mut argv = vec![
        "scstadmin".to_string(),
        "-open_dev".to_string(),
        device_name,
        "-handler".to_string(),
        "vdisk_blockio".to_string(),
        "-attributes".to_string(),
    ];
    match action.context.device.as_deref() {
        Some(device) => argv.push(format!("filename={device}")),
        None => {
            argv.push("filename=<backing-block-device-or-file>".to_string());
            unresolved_inputs.push("SCST backing block device or file".to_string());
        }
    }
    command_vec_with_readiness(
        argv,
        true,
        if unresolved_inputs.is_empty() {
            CommandReadiness::Ready
        } else {
            CommandReadiness::NeedsDomainImplementation
        },
        unresolved_inputs,
        note,
    )
}

fn target_lun_scst_close_device_command(
    action: &PlannedAction,
    device_name: &str,
    note: &str,
) -> ExecutionCommand {
    let (device_name, unresolved_inputs) =
        target_lun_scst_device_name_for_mutation(action, device_name);
    command_vec_with_readiness(
        vec![
            "scstadmin".to_string(),
            "-close_dev".to_string(),
            device_name,
            "-handler".to_string(),
            "vdisk_blockio".to_string(),
        ],
        true,
        if unresolved_inputs.is_empty() {
            CommandReadiness::Ready
        } else {
            CommandReadiness::NeedsDomainImplementation
        },
        unresolved_inputs,
        note,
    )
}

fn target_lun_scst_resync_device_command(
    action: &PlannedAction,
    device_name: &str,
    note: &str,
) -> ExecutionCommand {
    let (device_name, unresolved_inputs) =
        target_lun_scst_device_name_for_mutation(action, device_name);
    command_vec_with_readiness(
        vec![
            "scstadmin".to_string(),
            "-resync_dev".to_string(),
            device_name,
        ],
        true,
        if unresolved_inputs.is_empty() {
            CommandReadiness::Ready
        } else {
            CommandReadiness::NeedsDomainImplementation
        },
        unresolved_inputs,
        note,
    )
}

fn target_lun_scst_lun_command(
    action: &PlannedAction,
    target: &str,
    device_name: &str,
    op: &str,
    note: &str,
) -> ExecutionCommand {
    let (lun, mut unresolved_inputs) = target_lun_scst_lun(action);
    let group = target_lun_scst_group(action);
    let mut argv = vec![
        "scstadmin".to_string(),
        format!("-{op}"),
        lun,
        "-driver".to_string(),
        "iscsi".to_string(),
        "-target".to_string(),
        target.to_string(),
    ];
    if let Some(group) = group.as_deref() {
        argv.extend(["-group".to_string(), group.to_string()]);
    }
    if op == "add_lun" {
        let (device_name, device_unresolved) =
            target_lun_scst_device_name_for_mutation(action, device_name);
        unresolved_inputs.extend(device_unresolved);
        argv.extend(["-device".to_string(), device_name]);
    }
    command_vec_with_readiness(
        argv,
        true,
        if unresolved_inputs.is_empty() {
            CommandReadiness::Ready
        } else {
            CommandReadiness::NeedsDomainImplementation
        },
        unresolved_inputs,
        note,
    )
}

fn target_lun_scst_property_command(action: &PlannedAction, note: &str) -> ExecutionCommand {
    let (lun, mut unresolved_inputs) = target_lun_scst_lun(action);
    let target = action.context.target.as_deref().unwrap_or("<target>");
    let group = target_lun_scst_group(action);
    let property = match action.context.property.as_deref() {
        Some(property) => property.to_string(),
        None => {
            unresolved_inputs.push("SCST LUN attribute name".to_string());
            "<property>".to_string()
        }
    };
    let value = match action.context.property_value.as_deref() {
        Some(value) => value.to_string(),
        None => {
            unresolved_inputs.push("SCST LUN attribute value".to_string());
            "<value>".to_string()
        }
    };
    let mut argv = vec![
        "scstadmin".to_string(),
        "-set_lun_attr".to_string(),
        lun,
        "-driver".to_string(),
        "iscsi".to_string(),
        "-target".to_string(),
        target.to_string(),
    ];
    if let Some(group) = group.as_deref() {
        argv.extend(["-group".to_string(), group.to_string()]);
    }
    argv.extend(["-attributes".to_string(), format!("{property}={value}")]);

    command_vec_with_readiness(
        argv,
        true,
        if unresolved_inputs.is_empty() {
            CommandReadiness::Ready
        } else {
            CommandReadiness::NeedsDomainImplementation
        },
        unresolved_inputs,
        note,
    )
}

fn target_lun_scst_initiator_group_commands(
    action: &PlannedAction,
    target: &str,
    create: bool,
    commands: &mut Vec<ExecutionCommand>,
) {
    let group = target_lun_scst_group(action);
    let mut initiators = Vec::new();
    if let Some(client) = action.context.client.as_deref() {
        initiators.push(client.to_string());
    }
    initiators.extend(action.context.devices.iter().cloned());

    if initiators.is_empty() {
        return;
    }

    let group = group.unwrap_or_else(|| "disk-nix".to_string());
    if create {
        commands.push(command_vec(
            vec![
                "scstadmin".to_string(),
                "-add_group".to_string(),
                group.clone(),
                "-driver".to_string(),
                "iscsi".to_string(),
                "-target".to_string(),
                target.to_string(),
            ],
            true,
            "create the reviewed SCST initiator group",
        ));
    }

    for initiator in initiators {
        commands.push(command_vec(
            vec![
                "scstadmin".to_string(),
                if create { "-add_init" } else { "-rem_init" }.to_string(),
                initiator,
                "-driver".to_string(),
                "iscsi".to_string(),
                "-target".to_string(),
                target.to_string(),
                "-group".to_string(),
                group.clone(),
            ],
            true,
            if create {
                "add the reviewed initiator to the SCST group"
            } else {
                "remove the reviewed initiator from the SCST group"
            },
        ));
    }

    if !create {
        commands.push(command_vec(
            vec![
                "scstadmin".to_string(),
                "-rem_group".to_string(),
                group,
                "-driver".to_string(),
                "iscsi".to_string(),
                "-target".to_string(),
                target.to_string(),
            ],
            true,
            "remove the reviewed SCST initiator group after unmapping",
        ));
    }
}

fn target_lun_scst_write_config_command() -> ExecutionCommand {
    command_vec(
        vec![
            "scstadmin".to_string(),
            "-write_config".to_string(),
            "/etc/scst.conf".to_string(),
        ],
        true,
        "persist reviewed SCST target configuration",
    )
}

fn target_lun_scst_lun(action: &PlannedAction) -> (String, Vec<String>) {
    match action.context.lun.as_deref() {
        Some(lun) => (lun.to_string(), Vec::new()),
        None => ("<lun>".to_string(), vec!["SCST LUN number".to_string()]),
    }
}

fn target_lun_scst_group(action: &PlannedAction) -> Option<String> {
    action.context.group.as_deref().map(ToString::to_string)
}

fn target_lun_scst_device_name_for_mutation(
    action: &PlannedAction,
    device_name: &str,
) -> (String, Vec<String>) {
    let (device_name, _, unresolved_inputs) = target_lun_scst_device_name_readiness(
        action,
        device_name,
        "SCST device name or backing device",
    );
    (device_name, unresolved_inputs)
}

fn target_lun_scst_device_name_readiness(
    action: &PlannedAction,
    device_name: &str,
    unresolved: &str,
) -> (String, CommandReadiness, Vec<String>) {
    if action.context.device.is_some() || action.context.name.is_some() {
        (device_name.to_string(), CommandReadiness::Ready, Vec::new())
    } else {
        (
            "<scst-device>".to_string(),
            CommandReadiness::NeedsDomainImplementation,
            vec![unresolved.to_string()],
        )
    }
}

fn target_lun_scst_device_name(action: &PlannedAction, target: &str) -> String {
    target_lun_lio_backstore_name(action, target)
}

fn target_lun_inventory_command(
    action: &PlannedAction,
    target: &str,
    note: &str,
) -> ExecutionCommand {
    let mut command = command_vec_with_readiness(
        vec![
            target_lun_provider_program(action),
            "show-lun".to_string(),
            "--target".to_string(),
            target.to_string(),
        ],
        false,
        CommandReadiness::NeedsDomainImplementation,
        [target_lun_provider_unresolved(action)],
        note,
    );
    command.provider_capabilities = target_lun_provider_capabilities(action);
    command
}

fn target_lun_provider_command(
    action: &PlannedAction,
    target: &str,
    operation: &str,
    desired_size: Option<&str>,
) -> ExecutionCommand {
    let provider_operation = match action.operation {
        Operation::Create => "create-lun",
        Operation::Grow => "grow-lun",
        Operation::Attach => "map-lun",
        Operation::Detach => "unmap-lun",
        Operation::Destroy => "destroy-lun",
        Operation::SetProperty => "set-lun-property",
        Operation::Rescan => "refresh-lun",
        _ => operation,
    };
    let mut argv = vec![
        target_lun_provider_program(action),
        provider_operation.to_string(),
        "--target".to_string(),
        target.to_string(),
    ];
    let mut unresolved_inputs = vec![target_lun_provider_unresolved(action)];
    if let Some(provider) = action.context.provider.as_deref() {
        argv.push("--provider".to_string());
        argv.push(provider.to_string());
    }
    if let Some(vendor) = action.context.vendor.as_deref() {
        argv.push("--vendor".to_string());
        argv.push(vendor.to_string());
    }
    if let Some(array_id) = action.context.array_id.as_deref() {
        argv.push("--array-id".to_string());
        argv.push(array_id.to_string());
    }
    if let Some(storage_pool) = action.context.storage_pool.as_deref() {
        argv.push("--storage-pool".to_string());
        argv.push(storage_pool.to_string());
    }
    if let Some(volume_id) = action.context.volume_id.as_deref() {
        argv.push("--volume-id".to_string());
        argv.push(volume_id.to_string());
    }
    if let Some(snapshot_id) = action.context.snapshot_id.as_deref() {
        argv.push("--snapshot-id".to_string());
        argv.push(snapshot_id.to_string());
    }
    if let Some(clone_source) = action.context.clone_source.as_deref() {
        argv.push("--clone-source".to_string());
        argv.push(clone_source.to_string());
    }
    if let Some(masking_group) = action.context.masking_group.as_deref() {
        argv.push("--masking-group".to_string());
        argv.push(masking_group.to_string());
    }
    if matches!(action.operation, Operation::Create | Operation::Grow) {
        match desired_size {
            Some(size) => {
                argv.push("--size".to_string());
                argv.push(size.to_string());
            }
            None => unresolved_inputs.push("desired LUN size".to_string()),
        }
    }
    if let Some(backing) = action.context.device.as_deref() {
        argv.push("--backing".to_string());
        argv.push(backing.to_string());
    }
    if let Some(target_id) = action.context.target_id.as_deref() {
        argv.push("--target-id".to_string());
        argv.push(target_id.to_string());
    }
    if let Some(lun) = action.context.lun.as_deref() {
        argv.push("--lun".to_string());
        argv.push(lun.to_string());
    }
    if let Some(portal) = action.context.portal.as_deref() {
        argv.push("--portal".to_string());
        argv.push(portal.to_string());
    }
    if let Some(client) = action.context.client.as_deref() {
        argv.push("--initiator".to_string());
        argv.push(client.to_string());
    }
    for initiator in &action.context.devices {
        argv.push("--initiator".to_string());
        argv.push(initiator.clone());
    }
    if let Some(property) = action.context.property.as_deref() {
        argv.push("--property".to_string());
        argv.push(property.to_string());
    }
    if let Some(value) = action.context.property_value.as_deref() {
        argv.push("--value".to_string());
        argv.push(value.to_string());
    }

    let mut command = command_vec_with_readiness(
        argv,
        action.operation != Operation::Rescan,
        CommandReadiness::NeedsDomainImplementation,
        unresolved_inputs,
        &format!("render provider-specific target-side LUN {operation} command"),
    );
    command.provider_capabilities = target_lun_provider_capabilities(action);
    command
}

fn target_lun_provider_program(action: &PlannedAction) -> String {
    action
        .context
        .provider
        .as_deref()
        .map(|provider| format!("<target-lun-provider:{provider}>"))
        .unwrap_or_else(|| "<target-lun-provider>".to_string())
}

fn target_lun_provider_unresolved(action: &PlannedAction) -> String {
    action
        .context
        .provider
        .as_deref()
        .map(|provider| format!("{provider} target LUN provider implementation"))
        .unwrap_or_else(|| "target LUN provider implementation".to_string())
}

fn target_lun_provider_capabilities(action: &PlannedAction) -> Vec<String> {
    let mut capabilities = vec![
        "target-lun.identity".to_string(),
        "target-lun.inventory".to_string(),
        "target-lun.persistence".to_string(),
        "target-lun.verification".to_string(),
        "target-lun.refusal".to_string(),
    ];

    match action.operation {
        Operation::Create => {
            capabilities.extend([
                "target-lun.create".to_string(),
                "target-lun.capacity.declare".to_string(),
                "target-lun.backing.bind".to_string(),
                "target-lun.mapping.create".to_string(),
            ]);
        }
        Operation::Grow => {
            capabilities.extend([
                "target-lun.grow".to_string(),
                "target-lun.capacity.expand".to_string(),
                "target-lun.consumer-refresh.handoff".to_string(),
            ]);
        }
        Operation::Attach => {
            capabilities.extend([
                "target-lun.mapping.create".to_string(),
                "target-lun.initiator.allow".to_string(),
            ]);
        }
        Operation::Detach => {
            capabilities.extend([
                "target-lun.mapping.remove".to_string(),
                "target-lun.initiator.revoke".to_string(),
            ]);
        }
        Operation::Destroy => {
            capabilities.extend([
                "target-lun.mapping.remove".to_string(),
                "target-lun.destroy".to_string(),
                "target-lun.data-loss.guard".to_string(),
            ]);
        }
        Operation::Rescan => {
            capabilities.extend([
                "target-lun.refresh".to_string(),
                "target-lun.consumer-refresh.handoff".to_string(),
            ]);
        }
        Operation::SetProperty => {
            capabilities.extend([
                "target-lun.property.set".to_string(),
                "target-lun.property.validate".to_string(),
            ]);
        }
        _ => {}
    }

    if action.context.target_id.is_some() {
        capabilities.push("target-lun.target-id.declared".to_string());
    }
    if action.context.vendor.is_some() {
        capabilities.push("target-lun.vendor.declared".to_string());
    }
    if action.context.array_id.is_some() {
        capabilities.push("target-lun.array-id.declared".to_string());
    }
    if action.context.storage_pool.is_some() {
        capabilities.push("target-lun.storage-pool.declared".to_string());
    }
    if action.context.volume_id.is_some() {
        capabilities.push("target-lun.volume-id.declared".to_string());
    }
    if action.context.snapshot_id.is_some() {
        capabilities.push("target-lun.snapshot-id.declared".to_string());
    }
    if action.context.clone_source.is_some() {
        capabilities.push("target-lun.clone-source.declared".to_string());
    }
    if action.context.masking_group.is_some() {
        capabilities.push("target-lun.masking-group.declared".to_string());
    }
    if action.context.lun.is_some() {
        capabilities.push("target-lun.lun-id.declared".to_string());
    }
    if action.context.device.is_some() {
        capabilities.push("target-lun.backing.declared".to_string());
    }
    if action.context.portal.is_some() {
        capabilities.push("target-lun.portal.declared".to_string());
    }
    if action.context.client.is_some() || !action.context.devices.is_empty() {
        capabilities.push("target-lun.initiator-scope.declared".to_string());
    }

    capabilities
}

fn unimplemented_action_command(
    action: &PlannedAction,
    collection: Option<&str>,
    target: Option<&str>,
) -> ExecutionCommand {
    let operation = operation_name(action.operation);
    let collection_arg = collection.unwrap_or("<collection>");
    let target_arg = target.unwrap_or("<target>");
    let mut unresolved_inputs = vec!["storage-domain command renderer".to_string()];
    if collection.is_none() {
        unresolved_inputs.push("storage collection".to_string());
    }
    if target.is_none() {
        unresolved_inputs.push("storage target".to_string());
    }

    command_vec_with_readiness(
        vec![
            "disk-nix".to_string(),
            "storage-action".to_string(),
            operation.clone(),
            "--collection".to_string(),
            collection_arg.to_string(),
            "--target".to_string(),
            target_arg.to_string(),
        ],
        true,
        CommandReadiness::NeedsDomainImplementation,
        unresolved_inputs,
        &format!("render a domain-specific {operation} command before execution"),
    )
}

fn operation_name(operation: Operation) -> String {
    match serde_json::to_value(operation) {
        Ok(serde_json::Value::String(value)) => value,
        _ => format!("{operation:?}").to_ascii_lowercase(),
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
        provider_capabilities: Vec::new(),
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
        provider_capabilities: Vec::new(),
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
        "bcachefs" => bcachefs_device_resize_command(device, desired_size),
        "f2fs" => f2fs_filesystem_grow_command(target, device, desired_size),
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

fn filesystem_format_command(fs_type: &str, device: Option<&str>) -> ExecutionCommand {
    let Some(device) = device else {
        return command_with_readiness(
            ["mkfs", "-t", fs_type, "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "create the filesystem signature after selecting the reviewed block device",
        );
    };

    match fs_type {
        "ext2" => command(
            ["mkfs.ext2", "-F", device],
            true,
            "create an ext2 filesystem",
        ),
        "ext3" => command(
            ["mkfs.ext3", "-F", device],
            true,
            "create an ext3 filesystem",
        ),
        "ext4" => command(
            ["mkfs.ext4", "-F", device],
            true,
            "create an ext4 filesystem",
        ),
        "xfs" => command(["mkfs.xfs", "-f", device], true, "create an XFS filesystem"),
        "btrfs" => command(
            ["mkfs.btrfs", "-f", device],
            true,
            "create a Btrfs filesystem",
        ),
        "bcachefs" => command(
            ["bcachefs", "format", "--force", device],
            true,
            "create a bcachefs filesystem",
        ),
        "f2fs" => command(
            ["mkfs.f2fs", "-f", device],
            true,
            "create an F2FS filesystem",
        ),
        "exfat" => command(["mkfs.exfat", device], true, "create an exFAT filesystem"),
        "fat" | "vfat" => command(["mkfs.vfat", "-I", device], true, "create a FAT filesystem"),
        "ntfs" => command(
            ["mkfs.ntfs", "-F", device],
            true,
            "create an NTFS filesystem",
        ),
        "unknown" | "<filesystem-type>" => command_with_readiness(
            ["mkfs", "-t", "<filesystem-type>", device],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem type"],
            "create the filesystem signature after selecting the filesystem type",
        ),
        _ => command_vec_with_readiness(
            vec!["mkfs", "-t", fs_type, device],
            true,
            CommandReadiness::ManualOnly,
            ["review filesystem-specific mkfs options"],
            "review filesystem-specific mkfs flags before formatting this type",
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

fn f2fs_filesystem_grow_command(
    target: &str,
    device: Option<&str>,
    desired_size: Option<&str>,
) -> ExecutionCommand {
    match (filesystem_source_device(target, device), desired_size) {
        (Some(source), Some(size)) => command(
            ["resize.f2fs", "-t", size, source],
            true,
            "grow an F2FS filesystem to the reviewed target sector count",
        ),
        (Some(source), None) => command(
            ["resize.f2fs", source],
            true,
            "grow an F2FS filesystem to the visible backing device size",
        ),
        (None, Some(size)) => command_with_readiness(
            ["resize.f2fs", "-t", size, "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "grow the F2FS filesystem after resolving the source device",
        ),
        (None, None) => command_with_readiness(
            ["resize.f2fs", "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "grow the F2FS filesystem after resolving the source device",
        ),
    }
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
    fs_type: Option<&str>,
    target: &str,
    device: Option<&str>,
) -> ExecutionCommand {
    let Some(device) = device else {
        if collection == Some("filesystems") && fs_type == Some("bcachefs") {
            return bcachefs_add_device_command(target, None);
        }
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
        Some("filesystems") if fs_type == Some("bcachefs") => {
            bcachefs_add_device_command(target, Some(device))
        }
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
    fs_type: Option<&str>,
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
        Some("filesystems") if fs_type == Some("bcachefs") => command_vec(
            vec![
                "sh".to_string(),
                "-c".to_string(),
                format!(
                    "bcachefs device add {} {} && bcachefs data rereplicate {} && bcachefs device remove {} {}",
                    shell_quote(target),
                    shell_quote(to),
                    shell_quote(target),
                    shell_quote(target),
                    shell_quote(from)
                ),
            ],
            true,
            "replace a bcachefs member by adding replacement capacity, rereplicating, then removing the old device",
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
        Some("caches") => bcache_replace_command(target, from, to, None),
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

fn multipath_flush_map_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["multipath", "-f", target],
            true,
            "flush the reviewed multipath map from the host",
        ),
        None => command_with_readiness(
            ["multipath", "-f", "<multipath-map>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["multipath map target"],
            "flush the multipath map after selecting a concrete map target",
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

fn bcachefs_usage_command(target: &str, note: &'static str) -> ExecutionCommand {
    command(["bcachefs", "fs", "usage", target], false, note)
}

fn bcachefs_add_device_command(target: &str, device: Option<&str>) -> ExecutionCommand {
    match device {
        Some(device) => command(
            ["bcachefs", "device", "add", target, device],
            true,
            "add the reviewed device to the mounted bcachefs filesystem",
        ),
        None => command_with_readiness(
            ["bcachefs", "device", "add", target, "<device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["device to add"],
            "add a bcachefs member after selecting the reviewed device",
        ),
    }
}

fn bcachefs_remove_device_command(target: &str, device: Option<&str>) -> ExecutionCommand {
    match device {
        Some(device) => command(
            ["bcachefs", "device", "remove", target, device],
            true,
            "remove the reviewed device from the mounted bcachefs filesystem",
        ),
        None => command_with_readiness(
            ["bcachefs", "device", "remove", target, "<device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["device to remove"],
            "remove a bcachefs member after selecting the reviewed device",
        ),
    }
}

fn bcachefs_rereplicate_command(target: &str) -> ExecutionCommand {
    command(
        ["bcachefs", "data", "rereplicate", target],
        true,
        "rereplicate bcachefs data after topology or replica-policy changes",
    )
}

fn bcachefs_device_resize_command(
    device: Option<&str>,
    desired_size: Option<&str>,
) -> ExecutionCommand {
    match (device, desired_size) {
        (Some(device), Some(size)) => command(
            ["bcachefs", "device", "resize", device, size],
            true,
            "resize the reviewed bcachefs member device to the desired size",
        ),
        (Some(device), None) => command_with_readiness(
            ["bcachefs", "device", "resize", device, "<size>"],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired bcachefs member size"],
            "resize the reviewed bcachefs member after selecting the desired size",
        ),
        (None, Some(size)) => command_vec_with_readiness(
            vec!["bcachefs", "device", "resize", "<bcachefs-device>", size],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["bcachefs member device"],
            "resize the bcachefs member after selecting the device",
        ),
        (None, None) => command_with_readiness(
            [
                "bcachefs",
                "device",
                "resize",
                "<bcachefs-device>",
                "<size>",
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["bcachefs member device", "desired bcachefs member size"],
            "resize the bcachefs member after selecting device and desired size",
        ),
    }
}

fn rebalance_command(
    collection: Option<&str>,
    fs_type: Option<&str>,
    target: &str,
    property_assignments: &[String],
) -> ExecutionCommand {
    match collection {
        Some("pools") => command(
            ["zpool", "scrub", target],
            true,
            "scrub the pool after topology changes; ZFS has no generic rebalance command",
        ),
        Some("filesystems") if fs_type == Some("bcachefs") => bcachefs_rereplicate_command(target),
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

fn scrub_command(
    collection: Option<&str>,
    fs_type: Option<&str>,
    target: &str,
) -> ExecutionCommand {
    match collection {
        Some("pools") => command(
            ["zpool", "scrub", target],
            true,
            "start the reviewed ZFS pool scrub",
        ),
        Some("filesystems") if fs_type == Some("bcachefs") => command(
            ["bcachefs", "scrub", target],
            true,
            "run the reviewed bcachefs scrub",
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
    cache_set_uuid: Option<&str>,
) -> ExecutionCommand {
    match collection {
        Some("pools") if zfs_pool_assignment_is_root_dataset_property(property) => command(
            ["zfs", "set", assignment, target],
            true,
            "set a ZFS root dataset property",
        ),
        Some("pools") => command(
            ["zpool", "set", assignment, target],
            true,
            "set a ZFS pool property",
        ),
        Some("datasets") if zfs_dataset_property_is_create_time_only(property) => {
            zfs_idempotent_set_property_command(
                target,
                property,
                assignment,
                "set a ZFS create-time dataset property when it does not already match",
            )
        }
        Some("datasets") => command(
            ["zfs", "set", assignment, target],
            true,
            "set a ZFS dataset property",
        ),
        Some("zvols") if zfs_zvol_property_is_create_time_only(property) => {
            zfs_idempotent_set_property_command(
                target,
                property,
                assignment,
                "set a ZFS create-time zvol property when it does not already match",
            )
        }
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
        Some("caches") => bcache_property_command(target, property, assignment, cache_set_uuid),
        Some("loopDevices") => loop_property_command(target, property, assignment),
        Some("backingFiles") => backing_file_property_command(target, property, assignment),
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

fn backing_file_property_command(
    target: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let value = assignment
        .split_once('=')
        .map(|(_, value)| value)
        .unwrap_or(assignment);
    match normalize_property_name(property).as_str() {
        "mode" | "filemode" | "file-mode" | "permissions" | "filepermissions"
        | "file-permissions" => command(
            ["chmod", value, target],
            true,
            "set backing-file permissions",
        ),
        _ => command_with_readiness(
            ["<backing-file-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported backing-file property"],
            "apply a backing-file property update after selecting a supported file property",
        ),
    }
}

fn loop_property_command(target: &str, property: &str, assignment: &str) -> ExecutionCommand {
    let value = assignment
        .split_once('=')
        .map(|(_, value)| value)
        .unwrap_or(assignment);
    match normalize_property_name(property).as_str() {
        "readonly" | "read-only" | "loop-read-only" => {
            let tool = if truthy_property_value(value) {
                "--setro"
            } else {
                "--setrw"
            };
            command(
                ["blockdev", tool, target],
                true,
                "set loop device read-only mode",
            )
        }
        "directio" | "direct-io" | "loop-direct-io" => {
            let value = if truthy_property_value(value) {
                "on"
            } else {
                "off"
            };
            command_vec(
                vec![
                    "losetup".to_string(),
                    format!("--direct-io={value}"),
                    target.to_string(),
                ],
                true,
                "set loop device direct I/O mode",
            )
        }
        _ => command_with_readiness(
            ["<loop-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported loop property"],
            "apply a loop-device property update after selecting a supported loop property",
        ),
    }
}

fn truthy_property_value(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on" | "enabled"
    )
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
        Some("f2fs") => f2fs_filesystem_property_command(device, target, property, assignment),
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
        ("priority" | "swap.priority", Some(target), Some(value))
            if value.parse::<i32>().is_ok() =>
        {
            command_vec(
                vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    format!(
                        "swapoff {} 2>/dev/null || true; swapon --priority {} {}",
                        shell_quote(target),
                        shell_quote(value),
                        shell_quote(target)
                    ),
                ],
                true,
                "reactivate the reviewed swap target with the requested priority",
            )
        }
        ("priority" | "swap.priority", None, Some(value)) if value.parse::<i32>().is_ok() => {
            command_with_readiness(
                ["swapon", "--priority", value, "<swap>"],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["swap target path"],
                "reactivate swap with the requested priority after resolving the target",
            )
        }
        ("priority" | "swap.priority", Some(target), Some(_)) => command_with_readiness(
            ["swapon", "--priority", "<priority>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["integer swap priority"],
            "reactivate swap after resolving an integer priority",
        ),
        ("priority" | "swap.priority", Some(target), None) => command_with_readiness(
            ["swapon", "--priority", "<priority>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["integer swap priority"],
            "reactivate swap after resolving the requested priority",
        ),
        ("priority" | "swap.priority", None, _) => command_with_readiness(
            ["swapon", "--priority", "<priority>", "<swap>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["swap target path", "integer swap priority"],
            "reactivate swap after resolving target and priority",
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

fn f2fs_filesystem_property_command(
    device: Option<&str>,
    target: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<f2fs-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["F2FS filesystem property value"],
            "set an F2FS filesystem property after resolving the desired value",
        );
    };
    match (property, filesystem_source_device(target, device)) {
        ("label" | "f2fs.label" | "filesystem.label", Some(source)) => command(
            ["f2fslabel", source, value],
            true,
            "set the F2FS filesystem label on the reviewed backing device",
        ),
        ("label" | "f2fs.label" | "filesystem.label", None) => command_with_readiness(
            ["f2fslabel", "<filesystem-device>", value],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "set the F2FS filesystem label after resolving the backing device",
        ),
        _ => command_with_readiness(
            ["<f2fs-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported F2FS filesystem property"],
            "set an F2FS filesystem property after selecting a supported property mapping",
        ),
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

fn bcache_target_path(action: &PlannedAction) -> Option<&str> {
    [
        action.context.target.as_deref(),
        action.context.device.as_deref(),
        action.context.name.as_deref(),
    ]
    .into_iter()
    .flatten()
    .find(|target| is_bcache_target(target))
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

fn bcache_replace_command(
    target: &str,
    from: &str,
    replacement_device: &str,
    cache_set_uuid: Option<&str>,
) -> ExecutionCommand {
    let cache_set_arg = cache_set_uuid.unwrap_or("<new-cache-set-uuid>");
    let mut missing = Vec::new();
    if !is_bcache_target(target) {
        missing.push("bcache device path");
    }
    if cache_set_uuid.is_none() {
        missing.push("new cache-set UUID");
    }

    let argv = vec![
        "sh".to_string(),
        "-c".to_string(),
        "make-bcache -C \"$2\" --cset-uuid \"$3\" --writeback && printf '1\\n' > \"/sys/block/${1#/dev/}/bcache/detach\" && printf '%s\\n' \"$3\" > \"/sys/block/${1#/dev/}/bcache/attach\""
            .to_string(),
        "disk-nix-bcache-replace".to_string(),
        if is_bcache_target(target) {
            target.to_string()
        } else {
            "<cache-device>".to_string()
        },
        replacement_device.to_string(),
        cache_set_arg.to_string(),
    ];

    if missing.is_empty() {
        command_vec(
            argv,
            true,
            &format!(
                "initialize replacement cache device {replacement_device}, detach {from}, and attach cache-set {cache_set_arg} to {target}"
            ),
        )
    } else {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            &format!(
                "initialize replacement cache device after flushing and detaching {from} from {target}"
            ),
        )
    }
}

fn bcache_property_command(
    target: &str,
    property: &str,
    assignment: &str,
    cache_set_uuid: Option<&str>,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<cache-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["cache property value"],
            "set a cache property after resolving the desired value",
        );
    };
    if let Some(key) = bcache_cache_set_sysfs_key(property) {
        let cache_set_arg = cache_set_uuid.unwrap_or("<cache-set-uuid>");
        let argv = vec![
            "sh".to_string(),
            "-c".to_string(),
            "printf '%s\\n' \"$2\" > \"/sys/fs/bcache/$1/$3\"".to_string(),
            "disk-nix-bcache-set-property".to_string(),
            cache_set_arg.to_string(),
            value.to_string(),
            key,
        ];
        if cache_set_uuid.is_some() {
            return command_vec(
                argv,
                true,
                "set a bcache cache-set sysfs property on the target cache set",
            );
        }
        return command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["cache-set UUID"],
            "set a bcache cache-set property after selecting the cache-set UUID",
        );
    }
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

fn bcache_cache_set_sysfs_key(property: &str) -> Option<String> {
    let property = property.trim();
    let normalized = normalize_property_name(property);
    let known = match normalized.as_str() {
        "setaveragekeysize" => Some("average_key_size"),
        "setbtreecachesize" => Some("btree_cache_size"),
        "setcacheavailablepercent" => Some("cache_available_percent"),
        "setcongested" => Some("congested"),
        "setcongestedreadthresholdus" => Some("congested_read_threshold_us"),
        "setcongestedwritethresholdus" => Some("congested_write_threshold_us"),
        "setioerrorhalflife" => Some("io_error_halflife"),
        "setioerrorlimit" => Some("io_error_limit"),
        "setjournaldelayms" => Some("journal_delay_ms"),
        "setrootusagepercent" => Some("root_usage_percent"),
        _ => None,
    };
    if let Some(property) = known {
        return Some(property.to_string());
    }
    let property = property
        .strip_prefix("bcache.set-")
        .or_else(|| property.strip_prefix("bcache.set."))
        .or_else(|| property.strip_prefix("set-"))
        .or_else(|| property.strip_prefix("set_"))?;
    Some(property.replace('-', "_"))
}

fn lun_rescan_devices(action: &PlannedAction) -> Vec<String> {
    let mut devices = BTreeSet::new();
    if let Some(device) = action.context.device.as_deref() {
        devices.insert(device.to_string());
    }
    devices.extend(action.context.devices.iter().cloned());
    devices.into_iter().collect()
}

fn lsscsi_lun_inventory_command(note: &str) -> ExecutionCommand {
    command(["lsscsi", "-t", "-s"], false, note)
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

fn filesystem_inspect_command(
    mountpoint: Option<&str>,
    json_output: bool,
    note: &str,
) -> ExecutionCommand {
    match mountpoint {
        Some(mountpoint) => {
            if json_output {
                command(["disk-nix", "inspect", mountpoint, "--json"], false, note)
            } else {
                command(["disk-nix", "inspect", mountpoint], false, note)
            }
        }
        None => {
            let argv = if json_output {
                vec![
                    "disk-nix".to_string(),
                    "inspect".to_string(),
                    "<mountpoint>".to_string(),
                    "--json".to_string(),
                ]
            } else {
                vec![
                    "disk-nix".to_string(),
                    "inspect".to_string(),
                    "<mountpoint>".to_string(),
                ]
            };
            command_vec_with_readiness(
                argv,
                false,
                CommandReadiness::NeedsDomainImplementation,
                ["mountpoint path"],
                note,
            )
        }
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

fn filesystem_mount_command(
    source: Option<&str>,
    mountpoint: Option<&str>,
    fs_type: Option<&str>,
    options: Option<&str>,
) -> ExecutionCommand {
    let source_arg = source.unwrap_or("<device>");
    let mountpoint_arg = mountpoint.unwrap_or("<mountpoint>");
    let fs_type = fs_type.filter(|fs_type| !fs_type.is_empty() && *fs_type != "unknown");
    let options = options.filter(|options| !options.is_empty());
    let mut missing = Vec::new();
    if source.is_none() {
        missing.push("filesystem source device");
    }
    if mountpoint.is_none() {
        missing.push("mountpoint path");
    }

    let mut argv = vec!["mount".to_string()];
    if let Some(fs_type) = fs_type {
        argv.push("-t".to_string());
        argv.push(fs_type.to_string());
    }
    if let Some(options) = options {
        argv.push("-o".to_string());
        argv.push(options.to_string());
    }
    argv.push(source_arg.to_string());
    argv.push(mountpoint_arg.to_string());

    if source.is_some() && mountpoint.is_some() {
        command_vec(
            argv,
            true,
            "mount the reviewed filesystem source at the selected mountpoint",
        )
    } else {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "mount the filesystem after selecting a source device and mountpoint",
        )
    }
}

fn filesystem_unmount_command(mountpoint: Option<&str>) -> ExecutionCommand {
    match mountpoint {
        Some(mountpoint) => command(
            ["umount", mountpoint],
            true,
            "unmount the reviewed filesystem without formatting or deleting data",
        ),
        None => command_with_readiness(
            ["umount", "<mountpoint>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["mountpoint path"],
            "unmount the filesystem after selecting the mountpoint",
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

fn snapshot_rescan_identity<'a>(action: &'a PlannedAction, fallback: &'a str) -> &'a str {
    action
        .context
        .snapshot_path
        .as_deref()
        .or(action.context.name.as_deref())
        .unwrap_or(fallback)
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

fn disk_wipe_signatures_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["wipefs", "--all", "--force", target],
            true,
            "clear existing signatures before raw whole-disk ZFS pool creation",
        ),
        None => command_with_readiness(
            ["wipefs", "--all", "--force", "<disk>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["disk path"],
            "clear existing signatures before raw whole-disk ZFS pool creation after selecting the disk",
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

fn zfs_pool_create_argv(
    target: &str,
    devices: &[String],
    property_assignments: &[String],
) -> Vec<String> {
    let mut argv = vec!["zpool".to_string(), "create".to_string()];
    for assignment in property_assignments {
        let option = if zfs_pool_assignment_is_root_dataset_property(assignment) {
            "-O"
        } else {
            "-o"
        };
        argv.extend([option.to_string(), assignment.clone()]);
    }
    argv.push(target.to_string());
    argv.extend(devices.iter().cloned());
    argv
}

fn zfs_pool_assignment_is_root_dataset_property(assignment_or_property: &str) -> bool {
    let property = assignment_or_property
        .split_once('=')
        .map_or(assignment_or_property, |(property, _)| property);
    zfs_property_is_root_dataset_property(property)
}

fn zfs_property_is_root_dataset_property(property: &str) -> bool {
    if property.contains(':') {
        return true;
    }
    matches!(
        property,
        "aclinherit"
            | "aclmode"
            | "acltype"
            | "atime"
            | "canmount"
            | "casesensitivity"
            | "checksum"
            | "compression"
            | "copies"
            | "devices"
            | "dnodesize"
            | "encryption"
            | "exec"
            | "filesystem_count"
            | "filesystem_limit"
            | "jailed"
            | "keyformat"
            | "keylocation"
            | "logbias"
            | "mountpoint"
            | "nbmand"
            | "normalization"
            | "overlay"
            | "primarycache"
            | "quota"
            | "readonly"
            | "recordsize"
            | "redundant_metadata"
            | "refquota"
            | "refreservation"
            | "relatime"
            | "reservation"
            | "secondarycache"
            | "setuid"
            | "sharesmb"
            | "sharenfs"
            | "snapdir"
            | "snapshot_count"
            | "snapshot_limit"
            | "special_small_blocks"
            | "sync"
            | "utf8only"
            | "version"
            | "volblocksize"
            | "volmode"
            | "volsize"
            | "vscan"
            | "xattr"
            | "zoned"
    )
}

fn zfs_pool_create_command(
    target: &str,
    devices: &[String],
    property_assignments: &[String],
) -> ExecutionCommand {
    if devices.is_empty() {
        let mut argv = zfs_pool_create_argv(target, devices, property_assignments);
        argv.push("<vdev-device>".to_string());
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["vdev device or topology"],
            "create a ZFS pool after selecting the vdev topology",
        )
    } else {
        let argv = zfs_pool_create_argv(target, devices, property_assignments);
        command_vec(
            argv,
            true,
            "create a ZFS pool on the reviewed vdev device set with declared pool properties",
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

fn swapoff_best_effort_command(target: Option<&str>, note: &'static str) -> ExecutionCommand {
    match target {
        Some(target) => command_vec(
            vec![
                "sh".to_string(),
                "-c".to_string(),
                format!("swapoff {} 2>/dev/null || true", shell_quote(target)),
            ],
            true,
            note,
        ),
        None => command_with_readiness(
            ["swapoff", "<swap>"],
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

fn swap_wipefs_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["wipefs", "--all", target],
            true,
            "remove the reviewed swap signature metadata",
        ),
        None => command_with_readiness(
            ["wipefs", "--all", "<swap>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["swap target path"],
            "remove the swap signature after resolving the target",
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

fn zram_rescan_commands(note: &'static str) -> Vec<ExecutionCommand> {
    vec![
        command(
            [
                "zramctl",
                "--bytes",
                "--raw",
                "--noheadings",
                "--output-all",
            ],
            false,
            note,
        ),
        command(
            ["swapon", "--show", "--bytes", "--raw"],
            false,
            "refresh active swap view for zram devices",
        ),
        command(
            ["disk-nix", "zram"],
            false,
            "inspect modeled zram swap devices after refresh",
        ),
    ]
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

fn luks_keyslot_property_command(action: &PlannedAction, property: &str) -> ExecutionCommand {
    match normalize_property_name(property).as_str() {
        "keyfile"
        | "key-file"
        | "luks-keyfile"
        | "luks-key-file"
        | "cryptsetup-keyfile"
        | "cryptsetup-key-file" => luks_change_key_command(
            luks_keyslot_device(action),
            luks_keyslot_id(action),
            action.context.key_file.as_deref(),
            action.context.property_value.as_deref(),
        ),
        "priority" | "luks-keyslot-priority" | "cryptsetup-luks-keyslot-priority" => {
            luks_keyslot_priority_command(
                luks_keyslot_device(action),
                luks_keyslot_id(action),
                action.context.property_value.as_deref(),
            )
        }
        _ => command_vec_with_readiness(
            vec![
                "cryptsetup".to_string(),
                "config".to_string(),
                "<luks-device>".to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported LUKS keyslot property"],
            "change LUKS keyslot metadata after selecting a supported property",
        ),
    }
}

fn luks_keyslot_priority_command(
    device: Option<&str>,
    key_slot: Option<&str>,
    priority: Option<&str>,
) -> ExecutionCommand {
    let mut argv = vec![
        "cryptsetup".to_string(),
        "config".to_string(),
        device.unwrap_or("<luks-device>").to_string(),
        "--key-slot".to_string(),
        key_slot.unwrap_or("<key-slot>").to_string(),
        "--priority".to_string(),
        priority.unwrap_or("<priority>").to_string(),
    ];
    let normalized_priority = priority.map(normalize_property_name);
    let valid_priority = normalized_priority
        .as_deref()
        .is_some_and(|value| matches!(value, "prefer" | "normal" | "ignore"));
    if let Some(normalized_priority) = normalized_priority {
        if valid_priority {
            if let Some(last) = argv.last_mut() {
                *last = normalized_priority;
            }
        }
    }

    let missing = missing_luks_keyslot_priority_inputs(device, key_slot, priority, valid_priority);
    if missing.is_empty() {
        command_vec(
            argv,
            true,
            "change LUKS keyslot priority metadata after header backup",
        )
    } else {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "change LUKS keyslot priority after selecting device, slot, and priority",
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

fn missing_luks_keyslot_priority_inputs(
    device: Option<&str>,
    key_slot: Option<&str>,
    priority: Option<&str>,
    valid_priority: bool,
) -> Vec<&'static str> {
    let mut missing = missing_luks_keyslot_inputs(device, key_slot);
    if priority.is_none() {
        missing.push("LUKS keyslot priority");
    } else if !valid_priority {
        missing.push("LUKS keyslot priority prefer, normal, or ignore");
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

fn vdo_growth_commands(
    target: &str,
    desired_size: Option<&str>,
    physical_size: Option<&str>,
) -> Vec<ExecutionCommand> {
    let mut commands = Vec::new();
    if let Some(size) = physical_size {
        commands.push(command(
            ["vdo", "growPhysical", "--name", target],
            true,
            &format!(
                "grow VDO physical capacity after backing storage has grown to reviewed size {size}"
            ),
        ));
    }
    if desired_size.is_some() {
        commands.push(vdo_grow_logical_command(target, desired_size));
    }
    if commands.is_empty() {
        commands.push(command_with_readiness(
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
            ["desired VDO logical size or physicalSize intent"],
            "grow VDO after declaring desiredSize for logical growth or physicalSize for backing growth",
        ));
    }
    commands
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
        let size_flag = desired_size.map(lvm_size_flag).unwrap_or("--size");
        let size_value = desired_size
            .map(lvm_size_value)
            .unwrap_or_else(|| "<size>".to_string());
        return command_vec_with_readiness(
            vec![
                "lvcreate".to_string(),
                "--yes".to_string(),
                "--wipesignatures".to_string(),
                "y".to_string(),
                size_flag.to_string(),
                size_value,
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
        Some(size) if size.contains('%') => command_vec(
            vec![
                "lvcreate".to_string(),
                "--yes".to_string(),
                "--wipesignatures".to_string(),
                "y".to_string(),
                "--extents".to_string(),
                lvm_size_value(size),
                "--name".to_string(),
                logical_volume.to_string(),
                volume_group.to_string(),
            ],
            true,
            "create an LVM logical volume with the desired extent allocation",
        ),
        Some(size) => command_vec(
            vec![
                "lvcreate".to_string(),
                "--yes".to_string(),
                "--wipesignatures".to_string(),
                "y".to_string(),
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
                "--yes".to_string(),
                "--wipesignatures".to_string(),
                "y".to_string(),
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

fn lvm_size_flag(size: &str) -> &'static str {
    if size.contains('%') {
        "--extents"
    } else {
        "--size"
    }
}

fn lvm_size_value(size: &str) -> String {
    if size.ends_with('%') {
        format!("{size}FREE")
    } else {
        size.to_string()
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

fn backing_file_target_path(action: &PlannedAction) -> Option<&str> {
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

fn backing_file_stat_command(target: Option<&str>, description: &'static str) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["stat", "--printf=%n %s %b %B\\n", target],
            false,
            description,
        ),
        None => command_with_readiness(
            ["stat", "--printf=%n %s %b %B\\n", "<backing-file>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["backing file path"],
            description,
        ),
    }
}

fn backing_file_usage_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["du", "--bytes", "--apparent-size", target],
            false,
            "inspect backing file apparent size",
        ),
        None => command_with_readiness(
            ["du", "--bytes", "--apparent-size", "<backing-file>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["backing file path"],
            "inspect backing file apparent size",
        ),
    }
}

fn backing_file_inspect_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["disk-nix", "inspect", target],
            false,
            "inspect modeled backing-file relationships",
        ),
        None => command_with_readiness(
            ["disk-nix", "inspect", "<backing-file>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["backing file path"],
            "inspect modeled backing-file relationships",
        ),
    }
}

fn backing_file_inspect_json_command(
    target: Option<&str>,
    description: &'static str,
) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["disk-nix", "inspect", target, "--json"],
            false,
            description,
        ),
        None => command_with_readiness(
            ["disk-nix", "inspect", "<backing-file>", "--json"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["backing file path"],
            description,
        ),
    }
}

fn backing_file_absent_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["test", "!", "-e", target],
            false,
            "refuse to overwrite an existing backing file",
        ),
        None => command_with_readiness(
            ["test", "!", "-e", "<backing-file>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["backing file path"],
            "refuse to overwrite an existing backing file after selecting the path",
        ),
    }
}

fn backing_file_create_command(
    target: Option<&str>,
    desired_size: Option<&str>,
) -> ExecutionCommand {
    match (target, desired_size) {
        (Some(target), Some(size)) => command_vec(
            vec!["truncate", "--size", size, target],
            true,
            "create the new sparse backing file at the requested size",
        ),
        (Some(target), None) => command_with_readiness(
            ["truncate", "--size", "<size>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["desired backing file size"],
            "create the backing file after selecting a desired size",
        ),
        (None, Some(size)) => command_with_readiness(
            ["truncate", "--size", size, "<backing-file>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["backing file path"],
            "create the selected backing file at the requested size",
        ),
        (None, None) => command_with_readiness(
            ["truncate", "--size", "<size>", "<backing-file>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["backing file path", "desired backing file size"],
            "create the backing file after selecting a path and desired size",
        ),
    }
}

fn backing_file_grow_command(target: Option<&str>, desired_size: Option<&str>) -> ExecutionCommand {
    match (target, desired_size) {
        (Some(target), Some(size)) => command_vec(
            vec!["truncate", "--size", size, target],
            true,
            "extend the backing file to the requested size",
        ),
        (Some(target), None) => command_with_readiness(
            ["truncate", "--size", "<size>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["desired backing file size"],
            "extend the backing file after selecting a desired size",
        ),
        (None, Some(size)) => command_with_readiness(
            ["truncate", "--size", size, "<backing-file>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["backing file path"],
            "extend the selected backing file to the requested size",
        ),
        (None, None) => command_with_readiness(
            ["truncate", "--size", "<size>", "<backing-file>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["backing file path", "desired backing file size"],
            "extend the backing file after selecting a path and desired size",
        ),
    }
}

fn dm_map_target_path(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .target
        .as_deref()
        .filter(|target| is_dm_map_target(target))
        .or_else(|| {
            action
                .context
                .name
                .as_deref()
                .filter(|name| is_dm_map_target(name))
        })
}

fn is_dm_map_target(target: &str) -> bool {
    target.starts_with("/dev/mapper/") || target.starts_with("/dev/dm-")
}

fn dm_map_rename_to(action: &PlannedAction) -> Option<String> {
    action
        .context
        .rename_to
        .as_deref()
        .and_then(|rename_to| rename_to.strip_prefix("/dev/mapper/").or(Some(rename_to)))
        .filter(|rename_to| !rename_to.is_empty() && !rename_to.contains('/'))
        .map(ToString::to_string)
}

fn dmsetup_info_command(target: Option<&str>, description: &'static str) -> ExecutionCommand {
    match target {
        Some(target) => command(
            [
                "dmsetup",
                "info",
                "-c",
                "--noheadings",
                "-o",
                "name,uuid,major,minor,open,segments,events",
                target,
            ],
            false,
            description,
        ),
        None => command_with_readiness(
            [
                "dmsetup",
                "info",
                "-c",
                "--noheadings",
                "-o",
                "name,uuid,major,minor,open,segments,events",
                "<dm-map>",
            ],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["device-mapper path"],
            description,
        ),
    }
}

fn dmsetup_rename_command(target: Option<&str>, rename_to: Option<&str>) -> ExecutionCommand {
    match (target, rename_to) {
        (Some(target), Some(rename_to)) => command_vec(
            vec![
                "dmsetup".to_string(),
                "rename".to_string(),
                target.to_string(),
                rename_to.to_string(),
            ],
            true,
            "rename the reviewed device-mapper map",
        ),
        (target, rename_to) => command_vec_with_readiness(
            vec![
                "dmsetup".to_string(),
                "rename".to_string(),
                target.unwrap_or("<dm-map>").to_string(),
                rename_to.unwrap_or("<new-dm-map-name>").to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_dm_map_rename_inputs(target, rename_to),
            "rename the device-mapper map after selecting a concrete mapper path and new map name",
        ),
    }
}

fn missing_dm_map_rename_inputs(
    target: Option<&str>,
    rename_to: Option<&str>,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if target.is_none() {
        missing.push("device-mapper path");
    }
    if rename_to.is_none() {
        missing.push("new device-mapper name");
    }
    missing
}

fn dmsetup_remove_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["dmsetup", "remove", target],
            true,
            "remove the reviewed device-mapper map",
        ),
        None => command_with_readiness(
            ["dmsetup", "remove", "<dm-map>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["device-mapper path"],
            "remove the device-mapper map after selecting a concrete mapper path",
        ),
    }
}

fn dmsetup_ls_tree_command(description: &'static str) -> ExecutionCommand {
    command(["dmsetup", "ls", "--tree"], false, description)
}

fn dmsetup_deps_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["dmsetup", "deps", "-o", "devname", target],
            false,
            "refresh device-mapper dependency metadata",
        ),
        None => command_with_readiness(
            ["dmsetup", "deps", "-o", "devname", "<dm-map>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["device-mapper path"],
            "refresh device-mapper dependency metadata after selecting the mapper path",
        ),
    }
}

fn dmsetup_table_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["dmsetup", "table", target],
            false,
            "refresh device-mapper table metadata",
        ),
        None => command_with_readiness(
            ["dmsetup", "table", "<dm-map>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["device-mapper path"],
            "refresh device-mapper table metadata after selecting the mapper path",
        ),
    }
}

fn dmsetup_status_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["dmsetup", "status", target],
            false,
            "refresh device-mapper live status metadata",
        ),
        None => command_with_readiness(
            ["dmsetup", "status", "<dm-map>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["device-mapper path"],
            "refresh device-mapper live status metadata after selecting the mapper path",
        ),
    }
}

fn dm_map_inspect_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["disk-nix", "inspect", target],
            false,
            "inspect modeled device-mapper relationships",
        ),
        None => command_with_readiness(
            ["disk-nix", "inspect", "<dm-map>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["device-mapper path"],
            "inspect modeled device-mapper relationships after selecting the mapper path",
        ),
    }
}

fn dm_map_inspect_json_command(
    target: Option<&str>,
    description: &'static str,
) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["disk-nix", "inspect", target, "--json"],
            false,
            description,
        ),
        None => command_with_readiness(
            ["disk-nix", "inspect", "<dm-map>", "--json"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["device-mapper path"],
            description,
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
            let mut argv = zfs_create_wrapper_argv(target, property_assignments);
            argv.push("-V".to_string());
            argv.push(size.to_string());
            command_vec(
                argv,
                true,
                "create a zvol with the desired volume size when it is not already present",
            )
        }
        None => {
            let mut argv = zfs_create_wrapper_argv(target, property_assignments);
            argv.push("-V".to_string());
            argv.push("<size>".to_string());
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
    let argv = zfs_create_wrapper_argv(target, property_assignments);
    command_vec(
        argv,
        true,
        "create the reviewed ZFS filesystem dataset when it is not already present",
    )
}

fn zfs_create_wrapper_argv(target: &str, property_assignments: &[String]) -> Vec<String> {
    let mut argv = vec![
        "bash".to_string(),
        "-c".to_string(),
        "target=\"$1\"; shift; if zfs list -H \"$target\" >/dev/null 2>&1; then exit 0; fi; if zfs create \"$@\" \"$target\"; then exit 0; fi; status=\"$?\"; if zfs list -H \"$target\" >/dev/null 2>&1; then exit 0; fi; exit \"$status\"".to_string(),
        "disk-nix-zfs-create".to_string(),
        target.to_string(),
    ];
    for assignment in property_assignments {
        argv.push("-o".to_string());
        argv.push(assignment.clone());
    }
    argv
}

fn zfs_dataset_property_is_create_time_only(property: &str) -> bool {
    matches!(property, "encryption" | "keyformat")
}

fn zfs_zvol_property_is_create_time_only(property: &str) -> bool {
    matches!(property, "encryption" | "keyformat" | "volblocksize")
}

fn zfs_idempotent_set_property_command(
    target: &str,
    property: &str,
    assignment: &str,
    note: &'static str,
) -> ExecutionCommand {
    let Some((_, desired)) = assignment.split_once('=') else {
        return command(["zfs", "set", assignment, target], true, note);
    };
    command_vec(
        vec![
            "bash",
            "-c",
            "target=\"$1\"; property=\"$2\"; desired=\"$3\"; assignment=\"$property=$desired\"; current=\"$(zfs get -H -p -o value \"$property\" \"$target\" 2>/dev/null || true)\"; if [ \"$current\" = \"$desired\" ]; then exit 0; fi; exec zfs set \"$assignment\" \"$target\"",
            "disk-nix-zfs-set",
            target,
            property,
            desired,
        ],
        true,
        note,
    )
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
    [
        action.context.device.as_deref(),
        action.context.target.as_deref(),
        action.context.name.as_deref(),
    ]
    .into_iter()
    .flatten()
    .find(|target| is_nvme_controller_path(target))
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

fn nvme_list_subsystems_command(description: &'static str) -> ExecutionCommand {
    command(
        ["nvme", "list-subsys", "--output-format=json"],
        false,
        description,
    )
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
    metadata: Option<&str>,
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
        "--bitmap".to_string(),
        "none".to_string(),
    ];
    if let Some(name) = target
        .strip_prefix("/dev/md/")
        .filter(|name| !name.is_empty())
    {
        argv.extend(["--name".to_string(), name.to_string()]);
    }
    if let Some(metadata) = metadata {
        argv.extend(["--metadata".to_string(), metadata.to_string()]);
    }
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
mod tests;
