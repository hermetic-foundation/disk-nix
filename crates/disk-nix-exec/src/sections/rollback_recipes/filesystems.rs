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
