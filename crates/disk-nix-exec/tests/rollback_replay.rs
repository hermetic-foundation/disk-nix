use std::collections::BTreeMap;

use disk_nix_exec::{
    CommandPlanSummary, CommandReadiness, ExecutionCommand, ExecutionReport, ExecutionStatus,
    PartialExecutionRecovery, RollbackExecutionStatus, RollbackRecipe, RollbackRecipeSection,
    RollbackRecipeStatus, VerificationPlanSummary, materialize_rollback_topology_payloads,
    replay_proven_safe_rollback_recipe_with_topology_payloads,
};
use disk_nix_plan::{ApplyPolicy, ApplyReport, BlockedSummary};

fn command(argv: &[&str], mutates: bool, note: &str) -> ExecutionCommand {
    ExecutionCommand {
        argv: argv.iter().map(|part| (*part).to_string()).collect(),
        mutates,
        readiness: CommandReadiness::Ready,
        unresolved_inputs: Vec::new(),
        provider_capabilities: Vec::new(),
        note: note.to_string(),
    }
}

fn failed_apply_report_with_proven_recipe() -> ExecutionReport {
    let failed_command = vec!["false".to_string()];
    ExecutionReport {
        apply: ApplyReport {
            policy: ApplyPolicy::default(),
            allowed_count: 1,
            blocked_count: 0,
            blocked_summary: BlockedSummary::default(),
            blocked: Vec::new(),
        },
        status: ExecutionStatus::Failed,
        topology_comparison: None,
        command_summary: CommandPlanSummary::default(),
        tool_requirements: Vec::new(),
        command_plan: Vec::new(),
        verification_summary: VerificationPlanSummary::default(),
        verification_plan: Vec::new(),
        execution_results: Vec::new(),
        partial_execution_recovery: Some(PartialExecutionRecovery {
            completed_action_ids: vec!["lower-layer:grow".to_string()],
            failed_action_id: "filesystem:root:grow".to_string(),
            failed_phase: disk_nix_exec::ExecutionPhase::Command,
            failed_command: failed_command.clone(),
            retry_review_action_ids: vec!["filesystem:root:grow".to_string()],
            remaining_action_ids: Vec::new(),
            completed_mutating_command_count: 1,
            notes: vec!["capture a fresh topology probe before rollback".to_string()],
        }),
        recovery_actions: Vec::new(),
        rollback_recipes: vec![RollbackRecipe {
            recipe_version: 1,
            source_action_id: "filesystem:root:grow".to_string(),
            failed_command,
            status: RollbackRecipeStatus::ProvenSafe,
            receipt_binding_required: true,
            fresh_topology_probe_required: true,
            read_only_validation: RollbackRecipeSection {
                commands: vec![command(
                    &["true"],
                    false,
                    "re-probe and verify current topology before rollback",
                )],
                notes: vec!["fresh topology probe was captured".to_string()],
            },
            reversible_mutations: RollbackRecipeSection {
                commands: vec![command(
                    &["true"],
                    true,
                    "run proven data-preserving rollback mutation",
                )],
                notes: vec!["rollback recipe selected from failed apply report".to_string()],
            },
            destructive_mutations: RollbackRecipeSection {
                commands: Vec::new(),
                notes: Vec::new(),
            },
            operator_only_handoff: RollbackRecipeSection {
                commands: Vec::new(),
                notes: Vec::new(),
            },
            safety_gates: vec![
                "original apply receipt must match this failed apply report".to_string(),
                "fresh topology probe must be captured after the failure".to_string(),
            ],
            required_topology_evidence: vec![
                "expected".to_string(),
                "preApply".to_string(),
                "failedApply".to_string(),
                "current".to_string(),
            ],
            refusal_reasons: Vec::new(),
            notes: Vec::new(),
        }],
        messages: Vec::new(),
    }
}

#[test]
fn proven_rollback_recipe_replays_and_emits_receipt_binding() {
    let failed_report = failed_apply_report_with_proven_recipe();
    let current_topology = serde_json::json!({
        "nodes": [],
        "edges": [],
        "probe": "fresh-post-failure"
    });
    let topology_payloads =
        materialize_rollback_topology_payloads(&failed_report, current_topology.clone());
    let topology_evidence = BTreeMap::from([
        ("expected".to_string(), "topology:expected".to_string()),
        ("preApply".to_string(), "topology:pre-apply".to_string()),
        (
            "failedApply".to_string(),
            "topology:failed-apply".to_string(),
        ),
        ("current".to_string(), "topology:current".to_string()),
    ]);

    let replay = replay_proven_safe_rollback_recipe_with_topology_payloads(
        &failed_report,
        0,
        "receipt:failed-apply",
        "topology:current",
        topology_evidence,
        topology_payloads,
    );

    assert_eq!(replay.status, RollbackExecutionStatus::Succeeded);
    assert_eq!(replay.source_action_id, "filesystem:root:grow");
    assert_eq!(replay.validation_results.len(), 1);
    assert_eq!(replay.validation_results[0].argv, ["true"]);
    assert_eq!(replay.rollback_results.len(), 1);
    assert_eq!(replay.rollback_results[0].argv, ["true"]);
    assert_eq!(
        replay.receipt_binding.original_receipt_id,
        "receipt:failed-apply"
    );
    assert_eq!(
        replay.receipt_binding.fresh_topology_probe_id,
        "topology:current"
    );
    assert_eq!(
        replay.receipt_binding.topology_payloads.get("current"),
        Some(&current_topology)
    );

    let receipt = serde_json::to_value(&replay).expect("rollback receipt serializes");
    assert_eq!(receipt["status"], "succeeded");
    assert_eq!(
        receipt["receiptBinding"]["topologyPayloads"]["current"]["probe"],
        "fresh-post-failure"
    );
    assert_eq!(
        receipt["receiptBinding"]["topologyEvidence"]["failedApply"],
        "topology:failed-apply"
    );
    assert!(replay.messages.iter().any(|message| {
        message.contains("proven-safe rollback validation and reversible mutation steps completed")
    }));
    assert!(replay.refusal_reasons.is_empty());
}
