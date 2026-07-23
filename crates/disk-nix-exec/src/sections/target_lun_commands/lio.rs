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
