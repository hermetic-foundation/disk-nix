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
