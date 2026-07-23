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
