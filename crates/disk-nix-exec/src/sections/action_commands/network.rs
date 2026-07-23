fn network_action_commands(
    action: &PlannedAction,
    collection: Option<&str>,
    target: Option<&str>,
    cache_target: Option<&str>,
) -> Option<ActionCommandResult> {
    let _ = cache_target;
    Some(match action.operation {
        Operation::Rescan if collection == Some("luns") || action.id.starts_with("luns:") => {
            let target = target.unwrap_or("<lun>");
            let mut commands = vec![
                command(
                    ["iscsiadm", "--mode", "session", "--rescan"],
                    true,
                    "rescan iSCSI sessions to refresh existing LUN paths",
                ),
                lsscsi_lun_inventory_command(
                    "inspect host-visible LUN transport and size before per-device rescans",
                ),
                command(
                    ["disk-nix", "inspect", target],
                    false,
                    "inspect current LUN paths before per-device rescans",
                ),
            ];
            let devices = lun_rescan_devices(action);
            if devices.is_empty() {
                commands.push(command_with_readiness(
                    ["<scsi-rescan-device>", "<lun-path>"],
                    true,
                    CommandReadiness::NeedsDomainImplementation,
                    ["stable LUN device path"],
                    "rescan the concrete SCSI path after declaring a stable by-path LUN device",
                ));
            }
            for device in devices {
                commands.push(scsi_device_rescan_command(&device));
            }
            commands.extend([
                command(
                    ["multipath", "-r"],
                    true,
                    "reload multipath maps after refreshed LUN paths",
                ),
                command(
                    ["disk-nix", "inspect", target],
                    false,
                    "verify that refreshed paths and consumers are visible",
                ),
            ]);
            (
                commands,
                vec![
                    "declare stable LUN path devices to render per-path SCSI rescans".to_string(),
                    "verify multipath maps before exposing dependent consumers".to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("luns") || action.id.starts_with("luns:") => {
            let target = target.unwrap_or("<lun>");
            let mut commands = vec![
                command(
                    ["iscsiadm", "--mode", "session", "--rescan"],
                    true,
                    "rescan iSCSI sessions after target-side LUN growth",
                ),
                lsscsi_lun_inventory_command(
                    "inspect host-visible LUN transport and size before growth rescans",
                ),
                command(
                    ["disk-nix", "inspect", target],
                    false,
                    "inspect current LUN paths before per-device rescans",
                ),
            ];
            let devices = lun_rescan_devices(action);
            if devices.is_empty() {
                commands.push(command_with_readiness(
                    ["<scsi-rescan-device>", "<lun-path>"],
                    true,
                    CommandReadiness::NeedsDomainImplementation,
                    ["stable LUN device path"],
                    "rescan the concrete SCSI path after declaring a stable by-path LUN device",
                ));
            }
            for device in devices {
                commands.push(scsi_device_rescan_command(&device));
            }
            commands.extend([
                command(
                    ["multipath", "-r"],
                    true,
                    "reload multipath maps when the LUN is multipathed",
                ),
                command(
                    ["disk-nix", "inspect", target],
                    false,
                    "verify that consumers see the new capacity",
                ),
            ]);
            (
                commands,
                vec![
                    "coordinate the target-side LUN grow before host rescans".to_string(),
                    "declare stable LUN path devices to render per-path SCSI rescans".to_string(),
                ],
                true,
            )
        }
        Operation::Create | Operation::Attach
            if collection == Some("luns") || action.id.starts_with("luns:") =>
        {
            let target = target.unwrap_or("<lun>");
            let mut commands = vec![
                command(
                    ["iscsiadm", "--mode", "session", "--rescan"],
                    true,
                    "rescan iSCSI sessions after target-side LUN creation",
                ),
                lsscsi_lun_inventory_command(
                    "inspect host-visible LUN transport and size after session rescan",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "inspect the newly attached LUN and consumers",
                ),
            ];
            let devices = lun_rescan_devices(action);
            if devices.is_empty() {
                commands.push(command_vec_with_readiness(
                    vec!["<scsi-rescan-device>", "<lun-path>"],
                    true,
                    CommandReadiness::NeedsDomainImplementation,
                    ["stable LUN device path"],
                    "rescan the concrete SCSI path after declaring a stable by-path device",
                ));
            }
            for device in &devices {
                commands.push(scsi_device_rescan_command(device));
            }
            commands.push(command(
                ["multipath", "-r"],
                true,
                "reload multipath maps after newly attached LUN paths appear",
            ));
            if devices.is_empty() {
                commands.push(command_vec_with_readiness(
                    vec!["blockdev", "--getsize64", "<lun-path>"],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["stable LUN device path"],
                    "verify the reviewed LUN path after declaring a stable by-path device",
                ));
            }
            for device in &devices {
                commands.push(command_vec(
                    vec!["blockdev", "--getsize64", device.as_str()],
                    false,
                    "verify the reviewed LUN path is visible to the kernel",
                ));
            }
            (
                commands,
                vec![
                    "create or map the target-side LUN before host attach".to_string(),
                    "declare stable LUN path devices to verify every expected path".to_string(),
                    "enable filesystems, LVM, or multipath consumers only after verification"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Create
        | Operation::Grow
        | Operation::Attach
        | Operation::Detach
        | Operation::Destroy
        | Operation::SetProperty
        | Operation::Rescan
            if collection == Some("targetLuns") || action.id.starts_with("targetLuns:") =>
        {
            let target = target.unwrap_or("<target-lun>");
            (
                target_lun_commands(action, target),
                vec![
                    "target-side LUN work is provider-specific and stays non-ready until an array adapter or reviewed runbook renders concrete commands"
                        .to_string(),
                    "run host-side luns, iscsiSessions, and multipath rescans only after the target reports the intended mapping and capacity"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Grow | Operation::Rescan
            if collection == Some("iscsiSessions") || action.id.starts_with("iscsiSessions:") =>
        {
            let target = target.unwrap_or("<iscsi-session>");
            (
                vec![
                    command(
                        ["iscsiadm", "--mode", "session", "--rescan"],
                        true,
                        "rescan iSCSI sessions after target-side changes",
                    ),
                    lsscsi_lun_inventory_command(
                        "inspect host-visible LUN transport and size after session rescan",
                    ),
                    command(
                        ["disk-nix", "inspect", target, "--json"],
                        false,
                        "verify updated iSCSI, LUN, and consumer topology",
                    ),
                ],
                vec!["coordinate session rescans with every dependent LUN consumer".to_string()],
                true,
            )
        }
        Operation::Create | Operation::Login
            if collection == Some("iscsiSessions") || action.id.starts_with("iscsiSessions:") =>
        {
            let target = target.unwrap_or("<iscsi-target-iqn>");
            let portal = action.context.portal.as_deref();
            let discovery = match portal {
                Some(portal) => command_vec(
                    vec![
                        "iscsiadm",
                        "--mode",
                        "discovery",
                        "--type",
                        "sendtargets",
                        "--portal",
                        portal,
                    ],
                    true,
                    "discover iSCSI target records from the reviewed portal",
                ),
                None => command_vec_with_readiness(
                    vec![
                        "iscsiadm",
                        "--mode",
                        "discovery",
                        "--type",
                        "sendtargets",
                        "--portal",
                        "<portal>",
                    ],
                    true,
                    CommandReadiness::NeedsDomainImplementation,
                    ["iSCSI portal"],
                    "discover iSCSI target records after selecting the target portal",
                ),
            };
            let login = match portal {
                Some(portal) => command_vec(
                    vec![
                        "iscsiadm",
                        "--mode",
                        "node",
                        "--targetname",
                        target,
                        "--portal",
                        portal,
                        "--login",
                    ],
                    true,
                    "log in to the reviewed iSCSI target through the selected portal",
                ),
                None => command_vec_with_readiness(
                    vec![
                        "iscsiadm",
                        "--mode",
                        "node",
                        "--targetname",
                        target,
                        "--portal",
                        "<portal>",
                        "--login",
                    ],
                    true,
                    CommandReadiness::NeedsDomainImplementation,
                    ["iSCSI portal"],
                    "log in to the iSCSI target after selecting the target portal",
                ),
            };
            (
                vec![discovery, login],
                vec![
                    "verify the target IQN and portal before creating host sessions".to_string(),
                    "rescan and settle multipath paths before exposing dependent volumes"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy | Operation::Logout
            if collection == Some("iscsiSessions") || action.id.starts_with("iscsiSessions:") =>
        {
            let target = target.unwrap_or("<iscsi-target-iqn>");
            let portal = action.context.portal.as_deref();
            let logout = match portal {
                Some(portal) => command_vec(
                    vec![
                        "iscsiadm",
                        "--mode",
                        "node",
                        "--targetname",
                        target,
                        "--portal",
                        portal,
                        "--logout",
                    ],
                    true,
                    "log out from the reviewed iSCSI target and portal",
                ),
                None => command_vec(
                    vec![
                        "iscsiadm",
                        "--mode",
                        "node",
                        "--targetname",
                        target,
                        "--logout",
                    ],
                    true,
                    "log out from all node records for the reviewed iSCSI target",
                ),
            };
            (
                vec![logout],
                vec![
                    "unmount filesystems and deactivate mappings before logging out".to_string(),
                    "verify multipath, LVM, and filesystem consumers have migrated away"
                        .to_string(),
                ],
                true,
            )
        }
        _ => return None,
    })
}
