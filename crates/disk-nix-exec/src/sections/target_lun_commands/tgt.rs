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
