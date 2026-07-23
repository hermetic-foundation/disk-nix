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
