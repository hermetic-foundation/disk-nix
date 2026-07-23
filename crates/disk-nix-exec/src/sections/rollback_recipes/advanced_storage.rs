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
