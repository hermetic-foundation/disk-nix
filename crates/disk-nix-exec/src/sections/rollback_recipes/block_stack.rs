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
