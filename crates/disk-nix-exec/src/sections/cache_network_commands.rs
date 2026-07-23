fn is_bcache_target(target: &str) -> bool {
    target.starts_with("/dev/bcache")
}

fn bcache_target_path(action: &PlannedAction) -> Option<&str> {
    [
        action.context.target.as_deref(),
        action.context.device.as_deref(),
        action.context.name.as_deref(),
    ]
    .into_iter()
    .flatten()
    .find(|target| is_bcache_target(target))
}

fn lvm_cache_attach_command(target: Option<&str>, cache_pool: Option<&str>) -> ExecutionCommand {
    match (target, cache_pool) {
        (Some(target), Some(cache_pool)) => command(
            [
                "lvconvert",
                "--type",
                "cache",
                "--cachepool",
                cache_pool,
                target,
            ],
            true,
            "attach the reviewed LVM cache pool to the origin logical volume",
        ),
        (target, cache_pool) => command_vec_with_readiness(
            vec![
                "lvconvert".to_string(),
                "--type".to_string(),
                "cache".to_string(),
                "--cachepool".to_string(),
                cache_pool.unwrap_or("<cache-pool>").to_string(),
                target.unwrap_or("<origin-logical-volume>").to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_lvm_cache_inputs(target, cache_pool),
            "attach LVM cache after selecting an origin LV and cache-pool LV",
        ),
    }
}

fn lvm_cache_replace_command(
    target: Option<&str>,
    old_cache_pool: Option<&str>,
    new_cache_pool: Option<&str>,
) -> ExecutionCommand {
    match (target, new_cache_pool) {
        (Some(target), Some(new_cache_pool)) => command_vec(
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "lvconvert --uncache \"$1\" && lvconvert --type cache --cachepool \"$2\" \"$1\""
                    .to_string(),
                "disk-nix-lvm-cache-replace".to_string(),
                target.to_string(),
                new_cache_pool.to_string(),
            ],
            true,
            "detach the old LVM cache and attach the reviewed replacement cache pool",
        ),
        (target, new_cache_pool) => {
            let mut missing = missing_lvm_cache_inputs(target, new_cache_pool);
            if old_cache_pool.is_none() {
                missing.push("cache pool to replace");
            }
            command_vec_with_readiness(
                vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    "lvconvert --uncache \"$1\" && lvconvert --type cache --cachepool \"$2\" \"$1\""
                        .to_string(),
                    "disk-nix-lvm-cache-replace".to_string(),
                    target.unwrap_or("<origin-logical-volume>").to_string(),
                    new_cache_pool.unwrap_or("<replacement-cache-pool>").to_string(),
                ],
                true,
                CommandReadiness::NeedsDomainImplementation,
                missing,
                "replace LVM cache after selecting origin and replacement cache pool",
            )
        }
    }
}

fn lvm_cache_uncache_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["lvconvert", "--uncache", target],
            true,
            "detach LVM cache from the origin logical volume after dirty data is flushed",
        ),
        None => command_with_readiness(
            ["lvconvert", "--uncache", "<origin-logical-volume>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["target in volume-group/logical-volume form"],
            "detach LVM cache after selecting the origin logical volume",
        ),
    }
}

fn lvm_cache_property_command(
    target: Option<&str>,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            [
                "lvchange",
                "<cache-property>",
                "<value>",
                "<origin-logical-volume>",
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["cache property value"],
            "set LVM cache property after resolving the desired value",
        );
    };
    let Some(flag) = lvm_cache_property_flag(property) else {
        return command_with_readiness(
            [
                "lvchange",
                "<cache-property>",
                value,
                target.unwrap_or("<origin-logical-volume>"),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported LVM cache property"],
            "set LVM cache property after mapping it to an lvchange flag",
        );
    };
    match target {
        Some(target) => command(
            ["lvchange", flag, value, target],
            true,
            "set LVM cache mode or policy on the reviewed origin logical volume",
        ),
        None => command_vec_with_readiness(
            vec![
                "lvchange".to_string(),
                flag.to_string(),
                value.to_string(),
                "<origin-logical-volume>".to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["target in volume-group/logical-volume form"],
            "set LVM cache property after selecting the origin logical volume",
        ),
    }
}

fn missing_lvm_cache_inputs(target: Option<&str>, cache_pool: Option<&str>) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if target.is_none() {
        missing.push("target in volume-group/logical-volume form");
    }
    if cache_pool.is_none() {
        missing.push("cache-pool logical volume");
    }
    missing
}

fn lvm_cache_property_flag(property: &str) -> Option<&'static str> {
    match property {
        "cache-mode" | "cacheMode" | "lvm.cache-mode" | "lvm.cacheMode" => Some("--cachemode"),
        "cache-policy" | "cachePolicy" | "lvm.cache-policy" | "lvm.cachePolicy" => {
            Some("--cachepolicy")
        }
        _ => None,
    }
}

fn bcache_attach_command(target: &str, cache_set: &str) -> ExecutionCommand {
    if !is_bcache_target(target) {
        return command_vec_with_readiness(
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/attach\"".to_string(),
                "disk-nix-bcache-attach".to_string(),
                "<cache-device>".to_string(),
                cache_set.to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["bcache device path"],
            "attach an existing bcache cache-set UUID after selecting the backing bcache device",
        );
    }

    command_vec(
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/attach\"".to_string(),
            "disk-nix-bcache-attach".to_string(),
            target.to_string(),
            cache_set.to_string(),
        ],
        true,
        "attach an existing bcache cache-set UUID to the backing bcache device",
    )
}

fn bcache_detach_command(target: &str) -> ExecutionCommand {
    if !is_bcache_target(target) {
        return command_vec_with_readiness(
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "printf '1\\n' > \"/sys/block/${1#/dev/}/bcache/detach\"".to_string(),
                "disk-nix-bcache-detach".to_string(),
                "<cache-device>".to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["bcache device path"],
            "detach the bcache cache set after selecting the backing bcache device",
        );
    }

    command_vec(
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "printf '1\\n' > \"/sys/block/${1#/dev/}/bcache/detach\"".to_string(),
            "disk-nix-bcache-detach".to_string(),
            target.to_string(),
        ],
        true,
        "detach the bcache cache set from the backing device after dirty data is flushed",
    )
}

fn bcache_replace_command(
    target: &str,
    from: &str,
    replacement_device: &str,
    cache_set_uuid: Option<&str>,
) -> ExecutionCommand {
    let cache_set_arg = cache_set_uuid.unwrap_or("<new-cache-set-uuid>");
    let mut missing = Vec::new();
    if !is_bcache_target(target) {
        missing.push("bcache device path");
    }
    if cache_set_uuid.is_none() {
        missing.push("new cache-set UUID");
    }

    let argv = vec![
        "sh".to_string(),
        "-c".to_string(),
        "make-bcache -C \"$2\" --cset-uuid \"$3\" --writeback && printf '1\\n' > \"/sys/block/${1#/dev/}/bcache/detach\" && printf '%s\\n' \"$3\" > \"/sys/block/${1#/dev/}/bcache/attach\""
            .to_string(),
        "disk-nix-bcache-replace".to_string(),
        if is_bcache_target(target) {
            target.to_string()
        } else {
            "<cache-device>".to_string()
        },
        replacement_device.to_string(),
        cache_set_arg.to_string(),
    ];

    if missing.is_empty() {
        command_vec(
            argv,
            true,
            &format!(
                "initialize replacement cache device {replacement_device}, detach {from}, and attach cache-set {cache_set_arg} to {target}"
            ),
        )
    } else {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            &format!(
                "initialize replacement cache device after flushing and detaching {from} from {target}"
            ),
        )
    }
}

fn bcache_property_command(
    target: &str,
    property: &str,
    assignment: &str,
    cache_set_uuid: Option<&str>,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<cache-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["cache property value"],
            "set a cache property after resolving the desired value",
        );
    };
    if let Some(key) = bcache_cache_set_sysfs_key(property) {
        let cache_set_arg = cache_set_uuid.unwrap_or("<cache-set-uuid>");
        let argv = vec![
            "sh".to_string(),
            "-c".to_string(),
            "printf '%s\\n' \"$2\" > \"/sys/fs/bcache/$1/$3\"".to_string(),
            "disk-nix-bcache-set-property".to_string(),
            cache_set_arg.to_string(),
            value.to_string(),
            key,
        ];
        if cache_set_uuid.is_some() {
            return command_vec(
                argv,
                true,
                "set a bcache cache-set sysfs property on the target cache set",
            );
        }
        return command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["cache-set UUID"],
            "set a bcache cache-set property after selecting the cache-set UUID",
        );
    }
    if !is_bcache_target(target) {
        return command_vec_with_readiness(
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/$3\"".to_string(),
                "disk-nix-bcache-property".to_string(),
                "<cache-device>".to_string(),
                value.to_string(),
                bcache_sysfs_key(property),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["bcache device path"],
            "set a bcache sysfs property after selecting the backing bcache device",
        );
    }
    command_vec(
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/$3\"".to_string(),
            "disk-nix-bcache-property".to_string(),
            target.to_string(),
            value.to_string(),
            bcache_sysfs_key(property),
        ],
        true,
        "set a bcache sysfs property on the target cache device",
    )
}

fn bcache_sysfs_read_command(target: &str, key: &str, note: &str) -> ExecutionCommand {
    if !is_bcache_target(target) {
        return command_vec_with_readiness(
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "cat \"/sys/block/${1#/dev/}/bcache/$2\"".to_string(),
                "disk-nix-bcache-read".to_string(),
                "<cache-device>".to_string(),
                key.to_string(),
            ],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["bcache device path"],
            note,
        );
    }

    command_vec(
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "cat \"/sys/block/${1#/dev/}/bcache/$2\"".to_string(),
            "disk-nix-bcache-read".to_string(),
            target.to_string(),
            key.to_string(),
        ],
        false,
        note,
    )
}

fn bcache_sysfs_key(property: &str) -> String {
    property
        .strip_prefix("bcache.")
        .unwrap_or(property)
        .replace('-', "_")
}

fn bcache_cache_set_sysfs_key(property: &str) -> Option<String> {
    let property = property.trim();
    let normalized = normalize_property_name(property);
    let known = match normalized.as_str() {
        "setaveragekeysize" => Some("average_key_size"),
        "setbtreecachesize" => Some("btree_cache_size"),
        "setcacheavailablepercent" => Some("cache_available_percent"),
        "setcongested" => Some("congested"),
        "setcongestedreadthresholdus" => Some("congested_read_threshold_us"),
        "setcongestedwritethresholdus" => Some("congested_write_threshold_us"),
        "setioerrorhalflife" => Some("io_error_halflife"),
        "setioerrorlimit" => Some("io_error_limit"),
        "setjournaldelayms" => Some("journal_delay_ms"),
        "setrootusagepercent" => Some("root_usage_percent"),
        _ => None,
    };
    if let Some(property) = known {
        return Some(property.to_string());
    }
    let property = property
        .strip_prefix("bcache.set-")
        .or_else(|| property.strip_prefix("bcache.set."))
        .or_else(|| property.strip_prefix("set-"))
        .or_else(|| property.strip_prefix("set_"))?;
    Some(property.replace('-', "_"))
}

fn lun_rescan_devices(action: &PlannedAction) -> Vec<String> {
    let mut devices = BTreeSet::new();
    if let Some(device) = action.context.device.as_deref() {
        devices.insert(device.to_string());
    }
    devices.extend(action.context.devices.iter().cloned());
    devices.into_iter().collect()
}

fn lsscsi_lun_inventory_command(note: &str) -> ExecutionCommand {
    command(["lsscsi", "-t", "-s"], false, note)
}

fn scsi_device_rescan_command(device: &str) -> ExecutionCommand {
    command_vec(
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "block=$(basename \"$(readlink -f \"$1\")\"); printf '1\\n' > \"/sys/class/block/${block}/device/rescan\""
                .to_string(),
            "disk-nix-scsi-rescan".to_string(),
            device.to_string(),
        ],
        true,
        "rescan the reviewed SCSI block path after target-side changes",
    )
}

fn scsi_device_delete_command(device: &str) -> ExecutionCommand {
    command_vec(
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "block=$(basename \"$(readlink -f \"$1\")\"); printf '1\\n' > \"/sys/class/block/${block}/device/delete\""
                .to_string(),
            "disk-nix-scsi-delete".to_string(),
            device.to_string(),
        ],
        true,
        "detach the reviewed SCSI block path from the host",
    )
}

fn nfs_export_create_command(
    target: Option<&str>,
    client: Option<&str>,
    options: Option<&str>,
) -> ExecutionCommand {
    let target_arg = target.unwrap_or("<export-path>");
    let mut missing = Vec::new();
    if target.is_none() {
        missing.push("NFS export path");
    }
    if client.is_none() {
        missing.push("NFS client selector");
    }
    if options.is_none() {
        missing.push("NFS export options");
    }

    match (target, client, options) {
        (Some(_), Some(client), Some(options)) => command_vec(
            vec![
                "exportfs".to_string(),
                "-i".to_string(),
                "-o".to_string(),
                options.to_string(),
                format!("{client}:{target_arg}"),
            ],
            true,
            "export an existing path to the selected NFS client set with reviewed options",
        ),
        _ => {
            let client_arg = client.unwrap_or("<client>");
            let options_arg = options.unwrap_or("<options>");
            command_vec_with_readiness(
                vec![
                    "exportfs".to_string(),
                    "-i".to_string(),
                    "-o".to_string(),
                    options_arg.to_string(),
                    format!("{client_arg}:{target_arg}"),
                ],
                true,
                CommandReadiness::NeedsDomainImplementation,
                missing,
                "export the path after selecting clients, options, and a local export path",
            )
        }
    }
}

fn nfs_export_property_command(
    target: &str,
    client: Option<&str>,
    property: &str,
    property_value: Option<&str>,
    existing_options: Option<&str>,
) -> ExecutionCommand {
    match property {
        "options" | "nfs.options" | "exportOptions" | "export-options" => {
            nfs_export_create_command(
                path_like_target(target),
                client,
                property_value.or(existing_options),
            )
        }
        _ => command_with_readiness(
            ["exportfs", "-ra"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported NFS export property"],
            "reload NFS exports after selecting a supported export property mapping",
        ),
    }
}

fn luks_device_property_command(
    device: Option<&str>,
    property: &str,
    value: Option<&str>,
) -> ExecutionCommand {
    let device_arg = device.unwrap_or("<luks-device>");
    let mut missing = Vec::new();
    if device.is_none() {
        missing.push("LUKS backing device");
    }
    if value.is_none() {
        missing.push("LUKS property value");
    }

    let Some(value) = value else {
        return command_vec_with_readiness(
            luks_device_property_argv(device_arg, property, "<value>"),
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "update LUKS header identity after selecting a property value",
        );
    };

    let argv = luks_device_property_argv(device_arg, property, value);
    if !missing.is_empty() {
        return command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "update LUKS header identity after selecting the backing device",
        );
    }

    if luks_device_property_argv_is_supported(property) {
        command_vec(argv, true, "update LUKS header identity metadata")
    } else {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            vec!["supported LUKS header property"],
            "update LUKS header identity after selecting a supported property mapping",
        )
    }
}

fn luks_device_property_argv(device: &str, property: &str, value: &str) -> Vec<String> {
    match property {
        "label" | "luks.label" | "cryptsetup.label" => vec![
            "cryptsetup".to_string(),
            "config".to_string(),
            device.to_string(),
            "--label".to_string(),
            value.to_string(),
        ],
        "subsystem" | "luks.subsystem" | "cryptsetup.subsystem" => vec![
            "cryptsetup".to_string(),
            "config".to_string(),
            device.to_string(),
            "--subsystem".to_string(),
            value.to_string(),
        ],
        "uuid" | "luks.uuid" | "cryptsetup.uuid" => vec![
            "cryptsetup".to_string(),
            "luksUUID".to_string(),
            device.to_string(),
            "--uuid".to_string(),
            value.to_string(),
        ],
        _ => vec![
            "<luks-property-tool>".to_string(),
            device.to_string(),
            property.to_string(),
            value.to_string(),
        ],
    }
}

fn luks_device_property_argv_is_supported(property: &str) -> bool {
    matches!(
        property,
        "label"
            | "luks.label"
            | "cryptsetup.label"
            | "subsystem"
            | "luks.subsystem"
            | "cryptsetup.subsystem"
            | "uuid"
            | "luks.uuid"
            | "cryptsetup.uuid"
    )
}

fn export_target_path(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .target
        .as_deref()
        .and_then(path_like_target)
        .or_else(|| action.context.name.as_deref().and_then(path_like_target))
}

fn path_like_target(target: &str) -> Option<&str> {
    target.starts_with('/').then_some(target)
}

fn nfs_mount_target_path(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .mountpoint
        .as_deref()
        .and_then(path_like_target)
        .or_else(|| action.context.target.as_deref().and_then(path_like_target))
        .or_else(|| action.context.name.as_deref().and_then(path_like_target))
}

fn filesystem_mountpoint(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .mountpoint
        .as_deref()
        .and_then(path_like_target)
        .or_else(|| action.context.target.as_deref().and_then(path_like_target))
        .or_else(|| action.context.name.as_deref().and_then(path_like_target))
}

fn filesystem_findmnt_command(mountpoint: Option<&str>) -> ExecutionCommand {
    match mountpoint {
        Some(mountpoint) => command(
            ["findmnt", "--json", mountpoint],
            false,
            "inspect the filesystem mount after selecting the mountpoint",
        ),
        None => command_with_readiness(
            ["findmnt", "--json", "<mountpoint>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["mountpoint path"],
            "inspect the filesystem mount after selecting the mountpoint",
        ),
    }
}

fn filesystem_inspect_command(
    mountpoint: Option<&str>,
    json_output: bool,
    note: &str,
) -> ExecutionCommand {
    match mountpoint {
        Some(mountpoint) => {
            if json_output {
                command(["disk-nix", "inspect", mountpoint, "--json"], false, note)
            } else {
                command(["disk-nix", "inspect", mountpoint], false, note)
            }
        }
        None => {
            let argv = if json_output {
                vec![
                    "disk-nix".to_string(),
                    "inspect".to_string(),
                    "<mountpoint>".to_string(),
                    "--json".to_string(),
                ]
            } else {
                vec![
                    "disk-nix".to_string(),
                    "inspect".to_string(),
                    "<mountpoint>".to_string(),
                ]
            };
            command_vec_with_readiness(
                argv,
                false,
                CommandReadiness::NeedsDomainImplementation,
                ["mountpoint path"],
                note,
            )
        }
    }
}

fn filesystem_remount_command(mountpoint: Option<&str>, options: Option<&str>) -> ExecutionCommand {
    let mountpoint_arg = mountpoint.unwrap_or("<mountpoint>");
    let remount_options = options
        .filter(|options| !options.is_empty())
        .map(|options| format!("remount,{options}"))
        .unwrap_or_else(|| "remount".to_string());

    match mountpoint {
        Some(_) => command_vec(
            vec![
                "mount".to_string(),
                "-o".to_string(),
                remount_options,
                mountpoint_arg.to_string(),
            ],
            true,
            "remount the filesystem path with the reviewed options",
        ),
        None => command_vec_with_readiness(
            vec![
                "mount".to_string(),
                "-o".to_string(),
                remount_options,
                mountpoint_arg.to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["mountpoint path"],
            "remount the filesystem path after selecting the mountpoint",
        ),
    }
}

fn filesystem_mount_command(
    source: Option<&str>,
    mountpoint: Option<&str>,
    fs_type: Option<&str>,
    options: Option<&str>,
) -> ExecutionCommand {
    let source_arg = source.unwrap_or("<device>");
    let mountpoint_arg = mountpoint.unwrap_or("<mountpoint>");
    let fs_type = fs_type.filter(|fs_type| !fs_type.is_empty() && *fs_type != "unknown");
    let options = options.filter(|options| !options.is_empty());
    let mut missing = Vec::new();
    if source.is_none() {
        missing.push("filesystem source device");
    }
    if mountpoint.is_none() {
        missing.push("mountpoint path");
    }

    let mut argv = vec!["mount".to_string()];
    if let Some(fs_type) = fs_type {
        argv.push("-t".to_string());
        argv.push(fs_type.to_string());
    }
    if let Some(options) = options {
        argv.push("-o".to_string());
        argv.push(options.to_string());
    }
    argv.push(source_arg.to_string());
    argv.push(mountpoint_arg.to_string());

    if source.is_some() && mountpoint.is_some() {
        command_vec(
            argv,
            true,
            "mount the reviewed filesystem source at the selected mountpoint",
        )
    } else {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "mount the filesystem after selecting a source device and mountpoint",
        )
    }
}

fn filesystem_unmount_command(mountpoint: Option<&str>) -> ExecutionCommand {
    match mountpoint {
        Some(mountpoint) => command(
            ["umount", mountpoint],
            true,
            "unmount the reviewed filesystem without formatting or deleting data",
        ),
        None => command_with_readiness(
            ["umount", "<mountpoint>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["mountpoint path"],
            "unmount the filesystem after selecting the mountpoint",
        ),
    }
}

fn nfs_mount_create_command(
    source: Option<&str>,
    mountpoint: Option<&str>,
    fs_type: Option<&str>,
    options: Option<&str>,
) -> ExecutionCommand {
    let source_arg = source.unwrap_or("<nfs-source>");
    let mountpoint_arg = mountpoint.unwrap_or("<mountpoint>");
    let fs_type_arg = fs_type.unwrap_or("nfs4");
    let mut missing = Vec::new();
    if source.is_none() {
        missing.push("NFS source");
    }
    if mountpoint.is_none() {
        missing.push("mountpoint path");
    }

    if source.is_some() && mountpoint.is_some() {
        let mut argv = vec![
            "mount".to_string(),
            "-t".to_string(),
            fs_type_arg.to_string(),
        ];
        if let Some(options) = options {
            argv.push("-o".to_string());
            argv.push(options.to_string());
        }
        argv.push(source_arg.to_string());
        argv.push(mountpoint_arg.to_string());
        command_vec(
            argv,
            true,
            "mount the reviewed NFS source at the selected mountpoint",
        )
    } else {
        let mut argv = vec![
            "mount".to_string(),
            "-t".to_string(),
            fs_type_arg.to_string(),
        ];
        if let Some(options) = options {
            argv.push("-o".to_string());
            argv.push(options.to_string());
        }
        argv.push(source_arg.to_string());
        argv.push(mountpoint_arg.to_string());
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "mount the NFS source after selecting a source and mountpoint",
        )
    }
}

fn nfs_mount_findmnt_command(mountpoint: Option<&str>) -> ExecutionCommand {
    match mountpoint {
        Some(mountpoint) => command(
            ["findmnt", "--json", mountpoint],
            false,
            "inspect the NFS mount before unmounting",
        ),
        None => command_with_readiness(
            ["findmnt", "--json", "<mountpoint>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["mountpoint path"],
            "inspect the NFS mount after selecting the mountpoint",
        ),
    }
}

fn nfs_mount_stats_command(mountpoint: Option<&str>) -> ExecutionCommand {
    match mountpoint {
        Some(mountpoint) => command(
            ["nfsstat", "-m", mountpoint],
            false,
            "inspect NFS client mount statistics and negotiated options",
        ),
        None => command_with_readiness(
            ["nfsstat", "-m", "<mountpoint>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["mountpoint path"],
            "inspect NFS client mount statistics after selecting the mountpoint",
        ),
    }
}

fn nfs_mount_destroy_command(mountpoint: Option<&str>) -> ExecutionCommand {
    match mountpoint {
        Some(mountpoint) => command(
            ["umount", mountpoint],
            true,
            "unmount the reviewed NFS client mount without touching remote data",
        ),
        None => command_with_readiness(
            ["umount", "<mountpoint>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["mountpoint path"],
            "unmount the NFS client mount after selecting the mountpoint",
        ),
    }
}

fn nfs_mount_remount_command(mountpoint: Option<&str>, options: Option<&str>) -> ExecutionCommand {
    let mountpoint_arg = mountpoint.unwrap_or("<mountpoint>");
    let remount_options = options
        .filter(|options| !options.is_empty())
        .map(|options| format!("remount,{options}"))
        .unwrap_or_else(|| "remount".to_string());

    match mountpoint {
        Some(_) => command_vec(
            vec![
                "mount".to_string(),
                "-o".to_string(),
                remount_options,
                mountpoint_arg.to_string(),
            ],
            true,
            "remount the NFS client path with the reviewed options",
        ),
        None => command_vec_with_readiness(
            vec![
                "mount".to_string(),
                "-o".to_string(),
                remount_options,
                mountpoint_arg.to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["mountpoint path"],
            "remount the NFS client path after selecting the mountpoint",
        ),
    }
}

fn nfs_export_destroy_command(target: Option<&str>, client: Option<&str>) -> ExecutionCommand {
    let target_arg = target.unwrap_or("<export-path>");
    let mut missing = Vec::new();
    if target.is_none() {
        missing.push("NFS export path");
    }
    if client.is_none() {
        missing.push("NFS client selector");
    }

    match (target, client) {
        (Some(_), Some(client)) => command_vec(
            vec![
                "exportfs".to_string(),
                "-u".to_string(),
                format!("{client}:{target_arg}"),
            ],
            true,
            "unexport the reviewed NFS path for the selected client set",
        ),
        _ => {
            let client_arg = client.unwrap_or("<client>");
            command_vec_with_readiness(
                vec![
                    "exportfs".to_string(),
                    "-u".to_string(),
                    format!("{client_arg}:{target_arg}"),
                ],
                true,
                CommandReadiness::NeedsDomainImplementation,
                missing,
                "unexport the path after selecting the client and local export path",
            )
        }
    }
}

fn snapshot_property_command(
    snapshot: &str,
    property: &str,
    tag: Option<&str>,
) -> ExecutionCommand {
    let Some(tag) = tag else {
        return command_with_readiness(
            ["zfs", "hold", "<tag>", snapshot],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["ZFS hold tag"],
            "update a ZFS snapshot hold after selecting the hold tag",
        );
    };
    if !is_zfs_snapshot_name(snapshot) {
        return command_with_readiness(
            ["<snapshot-property-tool>", snapshot, tag],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["ZFS snapshot name"],
            "update snapshot retention with the target-specific snapshot property tool",
        );
    }
    match property {
        "zfs.hold" | "hold" | "holdTag" => command(
            ["zfs", "hold", tag, snapshot],
            true,
            "add a ZFS snapshot hold with the reviewed retention tag",
        ),
        "zfs.releaseHold" | "releaseHold" | "release-hold" => command(
            ["zfs", "release", tag, snapshot],
            true,
            "release a ZFS snapshot hold with the reviewed retention tag",
        ),
        _ => command_with_readiness(
            ["<snapshot-property-tool>", snapshot, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported snapshot property"],
            "update a snapshot property after selecting a supported domain mapping",
        ),
    }
}

fn snapshot_rescan_identity<'a>(action: &'a PlannedAction, fallback: &'a str) -> &'a str {
    action
        .context
        .snapshot_path
        .as_deref()
        .or(action.context.name.as_deref())
        .unwrap_or(fallback)
}

fn snapshot_hold_list_command(snapshot: &str) -> ExecutionCommand {
    if is_zfs_snapshot_name(snapshot) {
        command(
            ["zfs", "holds", snapshot],
            false,
            "verify ZFS snapshot hold tags",
        )
    } else {
        command_with_readiness(
            ["<snapshot-hold-list-tool>", snapshot],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["ZFS snapshot name"],
            "verify snapshot hold state with the target-specific tool",
        )
    }
}

fn zfs_snapshot_list_command(snapshot: &str, note: &str) -> ExecutionCommand {
    command(
        ["zfs", "list", "-t", "snapshot", "-H", "-p", snapshot],
        false,
        note,
    )
}

fn zfs_snapshot_rollback_command(snapshot: &str, recursive: bool) -> ExecutionCommand {
    if recursive {
        command(
            ["zfs", "rollback", "-r", snapshot],
            true,
            "recursively roll back the ZFS dataset after explicit review of newer snapshots",
        )
    } else {
        command(
            ["zfs", "rollback", snapshot],
            true,
            "roll back the ZFS dataset to the reviewed snapshot",
        )
    }
}

fn snapshot_command(
    collection: Option<&str>,
    target: &str,
    snapshot: &str,
    read_only: bool,
) -> ExecutionCommand {
    if is_zfs_snapshot_name(snapshot) {
        command(["zfs", "snapshot", snapshot], true, "create a ZFS snapshot")
    } else if collection == Some("btrfsSubvolumes") || is_btrfs_snapshot_pair(target, snapshot) {
        if read_only {
            command(
                ["btrfs", "subvolume", "snapshot", "-r", target, snapshot],
                true,
                "create a read-only Btrfs subvolume snapshot",
            )
        } else {
            command(
                ["btrfs", "subvolume", "snapshot", target, snapshot],
                true,
                "create a Btrfs subvolume snapshot",
            )
        }
    } else {
        command_with_readiness(
            ["<snapshot-tool>", target, snapshot],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["snapshot tool"],
            "create the snapshot with zfs, btrfs, lvm, or the target-specific tool",
        )
    }
}

fn is_zfs_snapshot_name(snapshot: &str) -> bool {
    let Some((dataset, name)) = snapshot.split_once('@') else {
        return false;
    };
    !dataset.is_empty() && !name.is_empty() && !dataset.starts_with('/')
}

fn zfs_snapshot_dataset(snapshot: &str) -> Option<&str> {
    snapshot.split_once('@').map(|(dataset, _)| dataset)
}

fn is_btrfs_snapshot_pair(target: &str, snapshot: &str) -> bool {
    target.starts_with('/') && snapshot.starts_with('/')
}
