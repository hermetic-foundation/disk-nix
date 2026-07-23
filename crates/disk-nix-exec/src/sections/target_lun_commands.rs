fn target_lun_commands(action: &PlannedAction, target: &str) -> Vec<ExecutionCommand> {
    if target_lun_lio_provider(action)
        && matches!(
            action.operation,
            Operation::Create
                | Operation::Attach
                | Operation::Detach
                | Operation::Destroy
                | Operation::Rescan
                | Operation::Grow
                | Operation::SetProperty
        )
    {
        return target_lun_lio_commands(action, target);
    }
    if target_lun_tgt_provider(action)
        && matches!(
            action.operation,
            Operation::Create
                | Operation::Attach
                | Operation::Detach
                | Operation::Destroy
                | Operation::Rescan
                | Operation::Grow
                | Operation::SetProperty
        )
    {
        return target_lun_tgt_commands(action, target);
    }
    if target_lun_scst_provider(action)
        && matches!(
            action.operation,
            Operation::Create
                | Operation::Attach
                | Operation::Detach
                | Operation::Destroy
                | Operation::Rescan
                | Operation::Grow
                | Operation::SetProperty
        )
    {
        return target_lun_scst_commands(action, target);
    }

    let operation = operation_name(action.operation);
    let desired_size = action.context.desired_size.as_deref();
    vec![
        target_lun_inventory_command(
            action,
            target,
            "inspect target-side LUN inventory before provider mutation",
        ),
        target_lun_provider_command(action, target, &operation, desired_size),
        target_lun_inventory_command(
            action,
            target,
            "inspect target-side LUN inventory after provider mutation",
        ),
    ]
}

fn target_lun_verification_commands(action: &PlannedAction, target: &str) -> Vec<ExecutionCommand> {
    if target_lun_lio_provider(action) {
        let mut commands = vec![target_lun_lio_inventory_command(
            target,
            "verify LIO target-side LUN inventory after provider action",
        )];
        if let Some(portal) = action.context.portal.as_deref() {
            commands.push(command_vec(
                vec![
                    "targetcli".to_string(),
                    target_lun_lio_tpg_path(target),
                    "ls".to_string(),
                ],
                false,
                &format!("verify LIO portal mapping for {portal} after provider action"),
            ));
        }
        if action.operation == Operation::Grow {
            commands.extend(target_lun_generic_host_verification_commands(target));
        }
        return commands;
    }
    if target_lun_tgt_provider(action) {
        let mut commands = vec![target_lun_tgt_inventory_command(
            action,
            "verify Linux tgt target-side LUN inventory after tgtadm action",
        )];
        if action.operation == Operation::Grow {
            commands.extend(target_lun_generic_host_verification_commands(target));
        }
        return commands;
    }
    if target_lun_scst_provider(action) {
        return vec![target_lun_scst_target_inventory_command(
            action,
            target,
            "verify SCST target-side LUN inventory after scstadmin action",
        )];
    }

    let mut commands = vec![target_lun_inventory_command(
        action,
        target,
        "verify target-side LUN inventory after provider action",
    )];
    if let Some(portal) = action.context.portal.as_deref() {
        let mut command = command_vec_with_readiness(
            vec![
                target_lun_provider_program(action),
                "show-mapping".to_string(),
                "--portal".to_string(),
                portal.to_string(),
                "--target".to_string(),
                target.to_string(),
            ],
            false,
            CommandReadiness::NeedsDomainImplementation,
            [target_lun_provider_unresolved(action)],
            "verify target-side portal mapping after provider action",
        );
        command.provider_capabilities = target_lun_provider_capabilities(action);
        commands.push(command);
    }
    commands.extend(target_lun_generic_host_verification_commands(target));
    commands
}

fn target_lun_generic_host_verification_commands(target: &str) -> Vec<ExecutionCommand> {
    vec![
        lsscsi_lun_inventory_command(
            "verify host-visible SCSI LUN paths after target-side provider action",
        ),
        command(
            ["multipath", "-ll"],
            false,
            "verify host multipath path grouping after target-side provider action",
        ),
        command_vec(
            ["disk-nix", "inspect", target, "--json"],
            false,
            "verify modeled target-side LUN graph state and consumers after provider action",
        ),
    ]
}

fn target_lun_lio_provider(action: &PlannedAction) -> bool {
    action.context.provider.as_deref().is_some_and(|provider| {
        matches!(
            provider.to_ascii_lowercase().as_str(),
            "lio" | "linux-lio" | "targetcli" | "targetcli-fb"
        )
    })
}

fn target_lun_tgt_provider(action: &PlannedAction) -> bool {
    action.context.provider.as_deref().is_some_and(|provider| {
        matches!(
            provider.to_ascii_lowercase().as_str(),
            "tgt" | "linux-tgt" | "tgtadm"
        )
    })
}

fn target_lun_scst_provider(action: &PlannedAction) -> bool {
    action.context.provider.as_deref().is_some_and(|provider| {
        matches!(
            provider.to_ascii_lowercase().as_str(),
            "scst" | "linux-scst" | "iscsi-scst" | "scstadmin"
        )
    })
}

fn target_lun_lio_commands(action: &PlannedAction, target: &str) -> Vec<ExecutionCommand> {
    let mut commands = vec![if action.operation == Operation::Create {
        target_lun_lio_inventory_root_command(
            "inspect LIO target-side inventory before targetcli mutation",
        )
    } else {
        target_lun_lio_inventory_command(
            target,
            "inspect LIO target-side inventory before targetcli mutation",
        )
    }];
    let backstore = target_lun_lio_backstore_name(action, target);
    let tpg = target_lun_lio_tpg_path(target);
    let lun = target_lun_lio_lun(action);

    match action.operation {
        Operation::Create => {
            commands.push(target_lun_lio_backstore_create_command(
                action,
                &backstore,
                "create LIO block backstore for the reviewed target-side LUN",
            ));
            commands.push(command_vec(
                vec![
                    "targetcli".to_string(),
                    "/iscsi".to_string(),
                    "create".to_string(),
                    target.to_string(),
                ],
                true,
                "create or ensure the reviewed LIO iSCSI target exists",
            ));
            commands.push(target_lun_lio_lun_create_command(
                action,
                &tpg,
                &backstore,
                &lun,
                "map the reviewed LIO backstore as a target LUN",
            ));
            target_lun_lio_acl_commands(action, &tpg, true, &mut commands);
            commands.push(target_lun_lio_saveconfig_command());
        }
        Operation::Attach => {
            if action.context.device.is_some() {
                commands.push(target_lun_lio_lun_create_command(
                    action,
                    &tpg,
                    &backstore,
                    &lun,
                    "map an existing LIO backstore as a target LUN",
                ));
            }
            target_lun_lio_acl_commands(action, &tpg, true, &mut commands);
            commands.push(target_lun_lio_saveconfig_command());
        }
        Operation::Detach => {
            target_lun_lio_acl_commands(action, &tpg, false, &mut commands);
            commands.push(target_lun_lio_lun_delete_command(
                &tpg,
                &lun,
                "unmap the reviewed LIO target LUN without deleting the backstore",
            ));
            commands.push(target_lun_lio_saveconfig_command());
        }
        Operation::Destroy => {
            target_lun_lio_acl_commands(action, &tpg, false, &mut commands);
            commands.push(target_lun_lio_lun_delete_command(
                &tpg,
                &lun,
                "unmap the reviewed LIO target LUN before target removal",
            ));
            commands.push(command_vec(
                vec![
                    "targetcli".to_string(),
                    "/iscsi".to_string(),
                    "delete".to_string(),
                    target.to_string(),
                ],
                true,
                "remove the reviewed LIO iSCSI target",
            ));
            commands.push(target_lun_lio_backstore_delete_command(
                action,
                &backstore,
                "remove the reviewed LIO block backstore after target removal",
            ));
            commands.push(target_lun_lio_saveconfig_command());
        }
        Operation::Rescan => {}
        Operation::Grow => {
            commands.push(target_lun_lio_backstore_inventory_command(
                action,
                &backstore,
                "inspect the reviewed LIO backstore before target-side LUN growth",
            ));
            if let Some(command) = target_lun_lio_forced_backstore_resize_command(
                action,
                target,
                &backstore,
                "force the reviewed LIO fileio backstore to the declared size before target refresh",
            ) {
                commands.push(command);
            }
            commands.push(target_lun_lio_backing_size_command(
                action,
                "validate the reviewed LIO backing object exposes the grown capacity",
            ));
            commands.push(target_lun_lio_lun_inventory_command(
                &tpg,
                "inspect LIO TPG LUN mappings before initiator capacity refresh",
            ));
            commands.push(target_lun_lio_saveconfig_command());
            commands.push(target_lun_lio_lun_inventory_command(
                &tpg,
                "inspect LIO TPG LUN mappings after target-side grow refresh",
            ));
        }
        Operation::SetProperty => {
            commands.push(target_lun_lio_backstore_inventory_command(
                action,
                &backstore,
                "inspect the reviewed LIO backstore before target-side LUN property update",
            ));
            if let Some(command) = target_lun_lio_property_command(
                action,
                &backstore,
                "update the reviewed LIO backstore property",
            ) {
                commands.push(command);
                commands.push(target_lun_lio_saveconfig_command());
            } else {
                commands.push(target_lun_provider_command(
                    action,
                    target,
                    "set-property",
                    action.context.desired_size.as_deref(),
                ));
            }
        }
        _ => {}
    }

    commands.push(target_lun_lio_inventory_command(
        target,
        "inspect LIO target-side inventory after targetcli mutation",
    ));
    commands
}

fn target_lun_lio_inventory_command(target: &str, note: &str) -> ExecutionCommand {
    command_vec(
        vec![
            "targetcli".to_string(),
            target_lun_lio_target_path(target),
            "ls".to_string(),
        ],
        false,
        note,
    )
}

fn target_lun_lio_inventory_root_command(note: &str) -> ExecutionCommand {
    command_vec(
        vec![
            "targetcli".to_string(),
            "/iscsi".to_string(),
            "ls".to_string(),
        ],
        false,
        note,
    )
}

fn target_lun_lio_backstore_inventory_command(
    action: &PlannedAction,
    backstore: &str,
    note: &str,
) -> ExecutionCommand {
    let (backstore_path, readiness, unresolved_inputs) = if action.context.device.is_some() {
        (
            target_lun_lio_backstore_path(action, backstore),
            CommandReadiness::Ready,
            Vec::new(),
        )
    } else {
        (
            "/backstores/block/<backstore>".to_string(),
            CommandReadiness::NeedsDomainImplementation,
            vec!["LIO backstore name or backing device for inventory".to_string()],
        )
    };
    command_vec_with_readiness(
        vec!["targetcli".to_string(), backstore_path, "ls".to_string()],
        false,
        readiness,
        unresolved_inputs,
        note,
    )
}

fn target_lun_lio_lun_inventory_command(tpg: &str, note: &str) -> ExecutionCommand {
    command_vec(
        vec![
            "targetcli".to_string(),
            format!("{tpg}/luns"),
            "ls".to_string(),
        ],
        false,
        note,
    )
}

fn target_lun_lio_backing_size_command(action: &PlannedAction, note: &str) -> ExecutionCommand {
    let fileio_regular_file = lio_backstore_type(action).as_deref() == Some("fileio")
        && action
            .context
            .device
            .as_deref()
            .is_none_or(|device| !device.starts_with("/dev/"));
    if fileio_regular_file {
        return match action.context.device.as_deref() {
            Some(path) => command_vec(
                vec![
                    "stat".to_string(),
                    "--format=%s".to_string(),
                    path.to_string(),
                ],
                false,
                note,
            ),
            None => command_vec_with_readiness(
                vec![
                    "stat".to_string(),
                    "--format=%s".to_string(),
                    "<fileio-backing-file>".to_string(),
                ],
                false,
                CommandReadiness::NeedsDomainImplementation,
                ["LIO fileio backing file for capacity validation"],
                note,
            ),
        };
    }

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
            ["LIO backing block device or file for capacity validation"],
            note,
        ),
    }
}

fn target_lun_lio_backstore_path(action: &PlannedAction, backstore: &str) -> String {
    let backstore_type = lio_backstore_type(action).unwrap_or_else(|| "block".to_string());
    format!("/backstores/{backstore_type}/{backstore}")
}

fn target_lun_lio_forced_backstore_resize_command(
    action: &PlannedAction,
    target: &str,
    backstore: &str,
    note: &str,
) -> Option<ExecutionCommand> {
    let backstore_type = lio_backstore_type(action)?;
    match backstore_type.as_str() {
        "fileio" => Some(target_lun_lio_fileio_resize_command(
            action, backstore, note,
        )),
        "block" => None,
        _ => Some(target_lun_lio_backstore_resize_handoff_command(
            action,
            target,
            backstore,
            &backstore_type,
        )),
    }
}

fn target_lun_lio_backstore_resize_handoff_command(
    action: &PlannedAction,
    target: &str,
    backstore: &str,
    backstore_type: &str,
) -> ExecutionCommand {
    let mut argv = vec![
        target_lun_provider_program(action),
        "grow-lio-backstore".to_string(),
        "--target".to_string(),
        target.to_string(),
        "--backstore-type".to_string(),
        backstore_type.to_string(),
        "--backstore-name".to_string(),
        backstore.to_string(),
    ];
    if let Some(lun) = action.context.lun.as_deref() {
        argv.push("--lun".to_string());
        argv.push(lun.to_string());
    }
    if let Some(device) = action.context.device.as_deref() {
        argv.push("--source".to_string());
        argv.push(device.to_string());
    }
    if let Some(size) = action.context.desired_size.as_deref() {
        argv.push("--size".to_string());
        argv.push(size.to_string());
    }
    let mut command = command_vec_with_readiness(
        argv,
        true,
        CommandReadiness::NeedsDomainImplementation,
        [format!(
            "provider-specific LIO {backstore_type} backstore resize primitive"
        )],
        "handoff LIO backstore resize to a reviewed site provider adapter",
    );
    command.provider_capabilities = target_lun_provider_capabilities(action);
    command
}

fn target_lun_lio_fileio_resize_command(
    action: &PlannedAction,
    backstore: &str,
    note: &str,
) -> ExecutionCommand {
    let mut unresolved_inputs = Vec::new();
    let size = match action.context.desired_size.as_deref() {
        Some(size) => size.to_string(),
        None => {
            unresolved_inputs.push("desired LIO fileio backstore size".to_string());
            "<size>".to_string()
        }
    };
    let path = match action.context.device.as_deref() {
        Some(path) => path.to_string(),
        None => {
            unresolved_inputs.push("LIO fileio backing file path".to_string());
            format!("<fileio-backing-file-for-{backstore}>")
        }
    };
    command_vec_with_readiness(
        vec!["truncate".to_string(), "--size".to_string(), size, path],
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

fn lio_backstore_type(action: &PlannedAction) -> Option<String> {
    action
        .context
        .backstore_type
        .as_deref()
        .map(|backstore_type| {
            backstore_type
                .trim()
                .trim_matches('"')
                .replace(['-', '_'], "")
                .to_ascii_lowercase()
        })
}

fn target_lun_lio_backstore_create_command(
    action: &PlannedAction,
    backstore: &str,
    note: &str,
) -> ExecutionCommand {
    let mut argv = vec![
        "targetcli".to_string(),
        "/backstores/block".to_string(),
        "create".to_string(),
        format!("name={backstore}"),
    ];
    let mut unresolved_inputs = Vec::new();
    if let Some(device) = action.context.device.as_deref() {
        argv.push(format!("dev={device}"));
    } else {
        argv.push("dev=<backing-block-device-or-file>".to_string());
        unresolved_inputs.push("LIO backing block device or file".to_string());
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

fn target_lun_lio_lun_create_command(
    action: &PlannedAction,
    tpg: &str,
    backstore: &str,
    lun: &str,
    note: &str,
) -> ExecutionCommand {
    let mut unresolved_inputs = Vec::new();
    let backstore_path = if action.context.device.is_some() {
        format!("/backstores/block/{backstore}")
    } else {
        unresolved_inputs.push("LIO backing block device or file".to_string());
        "/backstores/block/<backstore>".to_string()
    };
    command_vec_with_readiness(
        vec![
            "targetcli".to_string(),
            format!("{tpg}/luns"),
            "create".to_string(),
            backstore_path,
            format!("lun={lun}"),
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

fn target_lun_lio_lun_delete_command(tpg: &str, lun: &str, note: &str) -> ExecutionCommand {
    command_vec(
        vec![
            "targetcli".to_string(),
            format!("{tpg}/luns"),
            "delete".to_string(),
            lun.to_string(),
        ],
        true,
        note,
    )
}

fn target_lun_lio_backstore_delete_command(
    action: &PlannedAction,
    backstore: &str,
    note: &str,
) -> ExecutionCommand {
    let (backstore, readiness, unresolved_inputs) = if action.context.device.is_some() {
        (backstore.to_string(), CommandReadiness::Ready, Vec::new())
    } else {
        (
            "<backstore-name>".to_string(),
            CommandReadiness::NeedsDomainImplementation,
            vec!["LIO backstore name or backing device for removal".to_string()],
        )
    };
    command_vec_with_readiness(
        vec![
            "targetcli".to_string(),
            "/backstores/block".to_string(),
            "delete".to_string(),
            backstore,
        ],
        true,
        readiness,
        unresolved_inputs,
        note,
    )
}

fn target_lun_lio_property_command(
    action: &PlannedAction,
    backstore: &str,
    note: &str,
) -> Option<ExecutionCommand> {
    let property = action.context.property.as_deref()?;
    let attribute = target_lun_lio_attribute_for_property(property)?;
    let mut unresolved_inputs = Vec::new();
    let value = match action.context.property_value.as_deref() {
        Some(value) => match normalize_lio_bool_attribute_value(value) {
            Some(value) => value.to_string(),
            None => {
                unresolved_inputs.push("boolean LIO write-cache property value".to_string());
                "<0-or-1>".to_string()
            }
        },
        None => {
            unresolved_inputs.push("boolean LIO write-cache property value".to_string());
            "<0-or-1>".to_string()
        }
    };
    let backstore_path = if action.context.device.is_some() {
        format!("/backstores/block/{backstore}")
    } else {
        unresolved_inputs
            .push("LIO backstore name or backing device for property update".to_string());
        "/backstores/block/<backstore>".to_string()
    };

    Some(command_vec_with_readiness(
        vec![
            "targetcli".to_string(),
            backstore_path,
            "set".to_string(),
            "attribute".to_string(),
            format!("{attribute}={value}"),
        ],
        true,
        if unresolved_inputs.is_empty() {
            CommandReadiness::Ready
        } else {
            CommandReadiness::NeedsDomainImplementation
        },
        unresolved_inputs,
        note,
    ))
}

fn target_lun_lio_attribute_for_property(property: &str) -> Option<&'static str> {
    match property
        .trim()
        .trim_start_matches("lio.")
        .replace(['-', '_'], "")
        .to_ascii_lowercase()
        .as_str()
    {
        "writecache" | "emulatewritecache" => Some("emulate_write_cache"),
        _ => None,
    }
}

fn normalize_lio_bool_attribute_value(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" | "enabled" | "enable" => Some("1"),
        "0" | "false" | "no" | "off" | "disabled" | "disable" => Some("0"),
        _ => None,
    }
}

fn target_lun_lio_acl_commands(
    action: &PlannedAction,
    tpg: &str,
    create: bool,
    commands: &mut Vec<ExecutionCommand>,
) {
    let mut initiators = Vec::new();
    if let Some(client) = action.context.client.as_deref() {
        initiators.push(client.to_string());
    }
    initiators.extend(action.context.devices.iter().cloned());

    if initiators.is_empty() {
        commands.push(command_vec_with_readiness(
            vec![
                "targetcli".to_string(),
                format!("{tpg}/acls"),
                if create { "create" } else { "delete" }.to_string(),
                "<initiator-iqn>".to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["initiator IQN for LIO target ACL"],
            if create {
                "map the LIO target LUN to a reviewed initiator ACL"
            } else {
                "remove the reviewed initiator ACL from the LIO target"
            },
        ));
        return;
    }

    for initiator in initiators {
        commands.push(command_vec(
            vec![
                "targetcli".to_string(),
                format!("{tpg}/acls"),
                if create { "create" } else { "delete" }.to_string(),
                initiator,
            ],
            true,
            if create {
                "map the LIO target LUN to the reviewed initiator ACL"
            } else {
                "remove the reviewed initiator ACL from the LIO target"
            },
        ));
    }
}

fn target_lun_lio_saveconfig_command() -> ExecutionCommand {
    command_vec(
        vec!["targetcli".to_string(), "saveconfig".to_string()],
        true,
        "persist reviewed LIO target configuration",
    )
}

fn target_lun_lio_target_path(target: &str) -> String {
    format!("/iscsi/{target}")
}

fn target_lun_lio_tpg_path(target: &str) -> String {
    format!("{}/tpg1", target_lun_lio_target_path(target))
}

fn target_lun_lio_lun(action: &PlannedAction) -> String {
    action.context.lun.as_deref().unwrap_or("0").to_string()
}

fn target_lun_lio_backstore_name(action: &PlannedAction, target: &str) -> String {
    let raw = action
        .context
        .device
        .as_deref()
        .or(action.context.name.as_deref())
        .unwrap_or(target);
    let sanitized: String = raw
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "disk_nix_lun".to_string()
    } else {
        sanitized
    }
}

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

fn target_lun_provider_capabilities(action: &PlannedAction) -> Vec<String> {
    let mut capabilities = vec![
        "target-lun.identity".to_string(),
        "target-lun.inventory".to_string(),
        "target-lun.persistence".to_string(),
        "target-lun.verification".to_string(),
        "target-lun.refusal".to_string(),
    ];

    match action.operation {
        Operation::Create => {
            capabilities.extend([
                "target-lun.create".to_string(),
                "target-lun.capacity.declare".to_string(),
                "target-lun.backing.bind".to_string(),
                "target-lun.mapping.create".to_string(),
            ]);
        }
        Operation::Grow => {
            capabilities.extend([
                "target-lun.grow".to_string(),
                "target-lun.capacity.expand".to_string(),
                "target-lun.consumer-refresh.handoff".to_string(),
            ]);
        }
        Operation::Attach => {
            capabilities.extend([
                "target-lun.mapping.create".to_string(),
                "target-lun.initiator.allow".to_string(),
            ]);
        }
        Operation::Detach => {
            capabilities.extend([
                "target-lun.mapping.remove".to_string(),
                "target-lun.initiator.revoke".to_string(),
            ]);
        }
        Operation::Destroy => {
            capabilities.extend([
                "target-lun.mapping.remove".to_string(),
                "target-lun.destroy".to_string(),
                "target-lun.data-loss.guard".to_string(),
            ]);
        }
        Operation::Rescan => {
            capabilities.extend([
                "target-lun.refresh".to_string(),
                "target-lun.consumer-refresh.handoff".to_string(),
            ]);
        }
        Operation::SetProperty => {
            capabilities.extend([
                "target-lun.property.set".to_string(),
                "target-lun.property.validate".to_string(),
            ]);
        }
        _ => {}
    }

    if action.context.target_id.is_some() {
        capabilities.push("target-lun.target-id.declared".to_string());
    }
    if action.context.vendor.is_some() {
        capabilities.push("target-lun.vendor.declared".to_string());
    }
    if action.context.array_id.is_some() {
        capabilities.push("target-lun.array-id.declared".to_string());
    }
    if action.context.storage_pool.is_some() {
        capabilities.push("target-lun.storage-pool.declared".to_string());
    }
    if action.context.volume_id.is_some() {
        capabilities.push("target-lun.volume-id.declared".to_string());
    }
    if action.context.snapshot_id.is_some() {
        capabilities.push("target-lun.snapshot-id.declared".to_string());
    }
    if action.context.clone_source.is_some() {
        capabilities.push("target-lun.clone-source.declared".to_string());
    }
    if action.context.masking_group.is_some() {
        capabilities.push("target-lun.masking-group.declared".to_string());
    }
    if action.context.lun.is_some() {
        capabilities.push("target-lun.lun-id.declared".to_string());
    }
    if action.context.device.is_some() {
        capabilities.push("target-lun.backing.declared".to_string());
    }
    if action.context.portal.is_some() {
        capabilities.push("target-lun.portal.declared".to_string());
    }
    if action.context.client.is_some() || !action.context.devices.is_empty() {
        capabilities.push("target-lun.initiator-scope.declared".to_string());
    }

    capabilities
}
