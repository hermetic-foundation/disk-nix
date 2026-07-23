fn target_lun_scst_commands(action: &PlannedAction, target: &str) -> Vec<ExecutionCommand> {
    let mut commands = vec![target_lun_scst_target_inventory_command(
        action,
        target,
        "inspect SCST target-side inventory before scstadmin mutation",
    )];
    let device_name = target_lun_scst_device_name(action, target);

    match action.operation {
        Operation::Create => {
            commands.push(target_lun_scst_open_device_command(
                action,
                &device_name,
                "open the reviewed SCST backing device",
            ));
            commands.push(target_lun_scst_target_command(
                target,
                "add_target",
                "create or ensure the reviewed SCST iSCSI target exists",
            ));
            target_lun_scst_initiator_group_commands(action, target, true, &mut commands);
            commands.push(target_lun_scst_lun_command(
                action,
                target,
                &device_name,
                "add_lun",
                "map the reviewed SCST device as a target LUN",
            ));
            commands.push(target_lun_scst_enable_target_command(
                target,
                "enable the reviewed SCST target after mapping",
            ));
            commands.push(target_lun_scst_write_config_command());
        }
        Operation::Attach => {
            if action.context.device.is_some() {
                commands.push(target_lun_scst_open_device_command(
                    action,
                    &device_name,
                    "open an existing backing object as an SCST device",
                ));
                commands.push(target_lun_scst_lun_command(
                    action,
                    target,
                    &device_name,
                    "add_lun",
                    "map the reviewed SCST device as a target LUN",
                ));
            }
            target_lun_scst_initiator_group_commands(action, target, true, &mut commands);
            commands.push(target_lun_scst_write_config_command());
        }
        Operation::Detach => {
            target_lun_scst_initiator_group_commands(action, target, false, &mut commands);
            commands.push(target_lun_scst_lun_command(
                action,
                target,
                &device_name,
                "rem_lun",
                "unmap the reviewed SCST target LUN without closing the backing device",
            ));
            commands.push(target_lun_scst_write_config_command());
        }
        Operation::Destroy => {
            target_lun_scst_initiator_group_commands(action, target, false, &mut commands);
            commands.push(target_lun_scst_lun_command(
                action,
                target,
                &device_name,
                "rem_lun",
                "unmap the reviewed SCST target LUN before target removal",
            ));
            commands.push(target_lun_scst_target_command(
                target,
                "rem_target",
                "remove the reviewed SCST iSCSI target",
            ));
            commands.push(target_lun_scst_close_device_command(
                action,
                &device_name,
                "close the reviewed SCST backing device after target removal",
            ));
            commands.push(target_lun_scst_write_config_command());
        }
        Operation::Rescan | Operation::Grow => {
            commands.push(target_lun_scst_device_inventory_command(
                action,
                &device_name,
                "inspect the reviewed SCST backing device before resync",
            ));
            commands.push(target_lun_scst_resync_device_command(
                action,
                &device_name,
                "resync SCST cached backing-device size and notify initiators",
            ));
        }
        Operation::SetProperty => {
            commands.push(target_lun_scst_property_command(
                action,
                "update the reviewed SCST LUN attribute",
            ));
            commands.push(target_lun_scst_write_config_command());
        }
        _ => {}
    }

    commands.push(target_lun_scst_target_inventory_command(
        action,
        target,
        "inspect SCST target-side inventory after scstadmin mutation",
    ));
    commands
}

fn target_lun_scst_target_inventory_command(
    _action: &PlannedAction,
    target: &str,
    note: &str,
) -> ExecutionCommand {
    command_vec(
        vec![
            "scstadmin".to_string(),
            "-list_target".to_string(),
            target.to_string(),
            "-driver".to_string(),
            "iscsi".to_string(),
        ],
        false,
        note,
    )
}

fn target_lun_scst_device_inventory_command(
    action: &PlannedAction,
    device_name: &str,
    note: &str,
) -> ExecutionCommand {
    let (device_name, readiness, unresolved_inputs) = target_lun_scst_device_name_readiness(
        action,
        device_name,
        "SCST device name for inventory",
    );
    command_vec_with_readiness(
        vec![
            "scstadmin".to_string(),
            "-list_dev_attr".to_string(),
            device_name,
        ],
        false,
        readiness,
        unresolved_inputs,
        note,
    )
}

fn target_lun_scst_target_command(target: &str, op: &str, note: &str) -> ExecutionCommand {
    command_vec(
        vec![
            "scstadmin".to_string(),
            format!("-{op}"),
            target.to_string(),
            "-driver".to_string(),
            "iscsi".to_string(),
        ],
        true,
        note,
    )
}

fn target_lun_scst_enable_target_command(target: &str, note: &str) -> ExecutionCommand {
    command_vec(
        vec![
            "scstadmin".to_string(),
            "-enable_target".to_string(),
            target.to_string(),
            "-driver".to_string(),
            "iscsi".to_string(),
        ],
        true,
        note,
    )
}

fn target_lun_scst_open_device_command(
    action: &PlannedAction,
    device_name: &str,
    note: &str,
) -> ExecutionCommand {
    let (device_name, mut unresolved_inputs) =
        target_lun_scst_device_name_for_mutation(action, device_name);
    let mut argv = vec![
        "scstadmin".to_string(),
        "-open_dev".to_string(),
        device_name,
        "-handler".to_string(),
        "vdisk_blockio".to_string(),
        "-attributes".to_string(),
    ];
    match action.context.device.as_deref() {
        Some(device) => argv.push(format!("filename={device}")),
        None => {
            argv.push("filename=<backing-block-device-or-file>".to_string());
            unresolved_inputs.push("SCST backing block device or file".to_string());
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

fn target_lun_scst_close_device_command(
    action: &PlannedAction,
    device_name: &str,
    note: &str,
) -> ExecutionCommand {
    let (device_name, unresolved_inputs) =
        target_lun_scst_device_name_for_mutation(action, device_name);
    command_vec_with_readiness(
        vec![
            "scstadmin".to_string(),
            "-close_dev".to_string(),
            device_name,
            "-handler".to_string(),
            "vdisk_blockio".to_string(),
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

fn target_lun_scst_resync_device_command(
    action: &PlannedAction,
    device_name: &str,
    note: &str,
) -> ExecutionCommand {
    let (device_name, unresolved_inputs) =
        target_lun_scst_device_name_for_mutation(action, device_name);
    command_vec_with_readiness(
        vec![
            "scstadmin".to_string(),
            "-resync_dev".to_string(),
            device_name,
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

fn target_lun_scst_lun_command(
    action: &PlannedAction,
    target: &str,
    device_name: &str,
    op: &str,
    note: &str,
) -> ExecutionCommand {
    let (lun, mut unresolved_inputs) = target_lun_scst_lun(action);
    let group = target_lun_scst_group(action);
    let mut argv = vec![
        "scstadmin".to_string(),
        format!("-{op}"),
        lun,
        "-driver".to_string(),
        "iscsi".to_string(),
        "-target".to_string(),
        target.to_string(),
    ];
    if let Some(group) = group.as_deref() {
        argv.extend(["-group".to_string(), group.to_string()]);
    }
    if op == "add_lun" {
        let (device_name, device_unresolved) =
            target_lun_scst_device_name_for_mutation(action, device_name);
        unresolved_inputs.extend(device_unresolved);
        argv.extend(["-device".to_string(), device_name]);
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

fn target_lun_scst_property_command(action: &PlannedAction, note: &str) -> ExecutionCommand {
    let (lun, mut unresolved_inputs) = target_lun_scst_lun(action);
    let target = action.context.target.as_deref().unwrap_or("<target>");
    let group = target_lun_scst_group(action);
    let property = match action.context.property.as_deref() {
        Some(property) => property.to_string(),
        None => {
            unresolved_inputs.push("SCST LUN attribute name".to_string());
            "<property>".to_string()
        }
    };
    let value = match action.context.property_value.as_deref() {
        Some(value) => value.to_string(),
        None => {
            unresolved_inputs.push("SCST LUN attribute value".to_string());
            "<value>".to_string()
        }
    };
    let mut argv = vec![
        "scstadmin".to_string(),
        "-set_lun_attr".to_string(),
        lun,
        "-driver".to_string(),
        "iscsi".to_string(),
        "-target".to_string(),
        target.to_string(),
    ];
    if let Some(group) = group.as_deref() {
        argv.extend(["-group".to_string(), group.to_string()]);
    }
    argv.extend(["-attributes".to_string(), format!("{property}={value}")]);

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

fn target_lun_scst_initiator_group_commands(
    action: &PlannedAction,
    target: &str,
    create: bool,
    commands: &mut Vec<ExecutionCommand>,
) {
    let group = target_lun_scst_group(action);
    let mut initiators = Vec::new();
    if let Some(client) = action.context.client.as_deref() {
        initiators.push(client.to_string());
    }
    initiators.extend(action.context.devices.iter().cloned());

    if initiators.is_empty() {
        return;
    }

    let group = group.unwrap_or_else(|| "disk-nix".to_string());
    if create {
        commands.push(command_vec(
            vec![
                "scstadmin".to_string(),
                "-add_group".to_string(),
                group.clone(),
                "-driver".to_string(),
                "iscsi".to_string(),
                "-target".to_string(),
                target.to_string(),
            ],
            true,
            "create the reviewed SCST initiator group",
        ));
    }

    for initiator in initiators {
        commands.push(command_vec(
            vec![
                "scstadmin".to_string(),
                if create { "-add_init" } else { "-rem_init" }.to_string(),
                initiator,
                "-driver".to_string(),
                "iscsi".to_string(),
                "-target".to_string(),
                target.to_string(),
                "-group".to_string(),
                group.clone(),
            ],
            true,
            if create {
                "add the reviewed initiator to the SCST group"
            } else {
                "remove the reviewed initiator from the SCST group"
            },
        ));
    }

    if !create {
        commands.push(command_vec(
            vec![
                "scstadmin".to_string(),
                "-rem_group".to_string(),
                group,
                "-driver".to_string(),
                "iscsi".to_string(),
                "-target".to_string(),
                target.to_string(),
            ],
            true,
            "remove the reviewed SCST initiator group after unmapping",
        ));
    }
}

fn target_lun_scst_write_config_command() -> ExecutionCommand {
    command_vec(
        vec![
            "scstadmin".to_string(),
            "-write_config".to_string(),
            "/etc/scst.conf".to_string(),
        ],
        true,
        "persist reviewed SCST target configuration",
    )
}

fn target_lun_scst_lun(action: &PlannedAction) -> (String, Vec<String>) {
    match action.context.lun.as_deref() {
        Some(lun) => (lun.to_string(), Vec::new()),
        None => ("<lun>".to_string(), vec!["SCST LUN number".to_string()]),
    }
}

fn target_lun_scst_group(action: &PlannedAction) -> Option<String> {
    action.context.group.as_deref().map(ToString::to_string)
}

fn target_lun_scst_device_name_for_mutation(
    action: &PlannedAction,
    device_name: &str,
) -> (String, Vec<String>) {
    let (device_name, _, unresolved_inputs) = target_lun_scst_device_name_readiness(
        action,
        device_name,
        "SCST device name or backing device",
    );
    (device_name, unresolved_inputs)
}

fn target_lun_scst_device_name_readiness(
    action: &PlannedAction,
    device_name: &str,
    unresolved: &str,
) -> (String, CommandReadiness, Vec<String>) {
    if action.context.device.is_some() || action.context.name.is_some() {
        (device_name.to_string(), CommandReadiness::Ready, Vec::new())
    } else {
        (
            "<scst-device>".to_string(),
            CommandReadiness::NeedsDomainImplementation,
            vec![unresolved.to_string()],
        )
    }
}

fn target_lun_scst_device_name(action: &PlannedAction, target: &str) -> String {
    target_lun_lio_backstore_name(action, target)
}

fn target_lun_inventory_command(
    action: &PlannedAction,
    target: &str,
    note: &str,
) -> ExecutionCommand {
    let mut command = command_vec_with_readiness(
        vec![
            target_lun_provider_program(action),
            "show-lun".to_string(),
            "--target".to_string(),
            target.to_string(),
        ],
        false,
        CommandReadiness::NeedsDomainImplementation,
        [target_lun_provider_unresolved(action)],
        note,
    );
    command.provider_capabilities = target_lun_provider_capabilities(action);
    command
}

fn target_lun_provider_command(
    action: &PlannedAction,
    target: &str,
    operation: &str,
    desired_size: Option<&str>,
) -> ExecutionCommand {
    let provider_operation = match action.operation {
        Operation::Create => "create-lun",
        Operation::Grow => "grow-lun",
        Operation::Attach => "map-lun",
        Operation::Detach => "unmap-lun",
        Operation::Destroy => "destroy-lun",
        Operation::SetProperty => "set-lun-property",
        Operation::Rescan => "refresh-lun",
        _ => operation,
    };
    let mut argv = vec![
        target_lun_provider_program(action),
        provider_operation.to_string(),
        "--target".to_string(),
        target.to_string(),
    ];
    let mut unresolved_inputs = vec![target_lun_provider_unresolved(action)];
    if let Some(provider) = action.context.provider.as_deref() {
        argv.push("--provider".to_string());
        argv.push(provider.to_string());
    }
    if let Some(vendor) = action.context.vendor.as_deref() {
        argv.push("--vendor".to_string());
        argv.push(vendor.to_string());
    }
    if let Some(array_id) = action.context.array_id.as_deref() {
        argv.push("--array-id".to_string());
        argv.push(array_id.to_string());
    }
    if let Some(storage_pool) = action.context.storage_pool.as_deref() {
        argv.push("--storage-pool".to_string());
        argv.push(storage_pool.to_string());
    }
    if let Some(volume_id) = action.context.volume_id.as_deref() {
        argv.push("--volume-id".to_string());
        argv.push(volume_id.to_string());
    }
    if let Some(snapshot_id) = action.context.snapshot_id.as_deref() {
        argv.push("--snapshot-id".to_string());
        argv.push(snapshot_id.to_string());
    }
    if let Some(clone_source) = action.context.clone_source.as_deref() {
        argv.push("--clone-source".to_string());
        argv.push(clone_source.to_string());
    }
    if let Some(masking_group) = action.context.masking_group.as_deref() {
        argv.push("--masking-group".to_string());
        argv.push(masking_group.to_string());
    }
    if matches!(action.operation, Operation::Create | Operation::Grow) {
        match desired_size {
            Some(size) => {
                argv.push("--size".to_string());
                argv.push(size.to_string());
            }
            None => unresolved_inputs.push("desired LUN size".to_string()),
        }
    }
    if let Some(backing) = action.context.device.as_deref() {
        argv.push("--backing".to_string());
        argv.push(backing.to_string());
    }
    if let Some(target_id) = action.context.target_id.as_deref() {
        argv.push("--target-id".to_string());
        argv.push(target_id.to_string());
    }
    if let Some(lun) = action.context.lun.as_deref() {
        argv.push("--lun".to_string());
        argv.push(lun.to_string());
    }
    if let Some(portal) = action.context.portal.as_deref() {
        argv.push("--portal".to_string());
        argv.push(portal.to_string());
    }
    if let Some(client) = action.context.client.as_deref() {
        argv.push("--initiator".to_string());
        argv.push(client.to_string());
    }
    for initiator in &action.context.devices {
        argv.push("--initiator".to_string());
        argv.push(initiator.clone());
    }
    if let Some(property) = action.context.property.as_deref() {
        argv.push("--property".to_string());
        argv.push(property.to_string());
    }
    if let Some(value) = action.context.property_value.as_deref() {
        argv.push("--value".to_string());
        argv.push(value.to_string());
    }

    let mut command = command_vec_with_readiness(
        argv,
        action.operation != Operation::Rescan,
        CommandReadiness::NeedsDomainImplementation,
        unresolved_inputs,
        &format!("render provider-specific target-side LUN {operation} command"),
    );
    command.provider_capabilities = target_lun_provider_capabilities(action);
    command
}

fn target_lun_provider_program(action: &PlannedAction) -> String {
    action
        .context
        .provider
        .as_deref()
        .map(|provider| format!("<target-lun-provider:{provider}>"))
        .unwrap_or_else(|| "<target-lun-provider>".to_string())
}

fn target_lun_provider_unresolved(action: &PlannedAction) -> String {
    action
        .context
        .provider
        .as_deref()
        .map(|provider| format!("{provider} target LUN provider implementation"))
        .unwrap_or_else(|| "target LUN provider implementation".to_string())
}
