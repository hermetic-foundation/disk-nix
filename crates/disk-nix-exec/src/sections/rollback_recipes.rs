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
