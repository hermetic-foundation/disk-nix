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
