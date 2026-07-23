fn btrfs_balance_filters(property_assignments: &[String]) -> Vec<String> {
    property_assignments
        .iter()
        .filter_map(|assignment| {
            let (property, value) = assignment.split_once('=')?;
            let property = property
                .strip_prefix("btrfs.balance.")
                .or_else(|| property.strip_prefix("balance."))
                .or_else(|| property.strip_prefix("btrfs."))
                .unwrap_or(property);
            match property {
                "data" | "d" => Some(format!("-d{value}")),
                "metadata" | "meta" | "m" => Some(format!("-m{value}")),
                "system" | "s" => Some(format!("-s{value}")),
                _ => None,
            }
        })
        .collect()
}

fn set_property_command(
    collection: Option<&str>,
    target: &str,
    property: &str,
    assignment: &str,
    cache_set_uuid: Option<&str>,
) -> ExecutionCommand {
    match collection {
        Some("pools") if zfs_pool_assignment_is_root_dataset_property(property) => command(
            ["zfs", "set", assignment, target],
            true,
            "set a ZFS root dataset property",
        ),
        Some("pools") => command(
            ["zpool", "set", assignment, target],
            true,
            "set a ZFS pool property",
        ),
        Some("datasets") if zfs_dataset_property_is_create_time_only(property) => {
            zfs_idempotent_set_property_command(
                target,
                property,
                assignment,
                "set a ZFS create-time dataset property when it does not already match",
            )
        }
        Some("datasets") => command(
            ["zfs", "set", assignment, target],
            true,
            "set a ZFS dataset property",
        ),
        Some("zvols") if zfs_zvol_property_is_create_time_only(property) => {
            zfs_idempotent_set_property_command(
                target,
                property,
                assignment,
                "set a ZFS create-time zvol property when it does not already match",
            )
        }
        Some("zvols") => command(
            ["zfs", "set", assignment, target],
            true,
            "set a zvol property",
        ),
        Some("btrfsSubvolumes") => btrfs_subvolume_property_command(target, property, assignment),
        Some("exports") => command(
            ["exportfs", "-ra"],
            true,
            "reload NFS exports after export property changes",
        ),
        Some("lvmCaches") => {
            lvm_cache_property_command(lvm_volume_target_path(Some(target)), property, assignment)
        }
        Some("caches") => bcache_property_command(target, property, assignment, cache_set_uuid),
        Some("loopDevices") => loop_property_command(target, property, assignment),
        Some("backingFiles") => backing_file_property_command(target, property, assignment),
        Some("vdoVolumes") => vdo_property_command(target, property, assignment),
        _ => command_with_readiness(
            ["<set-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["property update tool"],
            "apply the storage-domain property update",
        ),
    }
}

fn backing_file_property_command(
    target: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let value = assignment
        .split_once('=')
        .map(|(_, value)| value)
        .unwrap_or(assignment);
    match normalize_property_name(property).as_str() {
        "mode" | "filemode" | "file-mode" | "permissions" | "filepermissions"
        | "file-permissions" => command(
            ["chmod", value, target],
            true,
            "set backing-file permissions",
        ),
        _ => command_with_readiness(
            ["<backing-file-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported backing-file property"],
            "apply a backing-file property update after selecting a supported file property",
        ),
    }
}

fn loop_property_command(target: &str, property: &str, assignment: &str) -> ExecutionCommand {
    let value = assignment
        .split_once('=')
        .map(|(_, value)| value)
        .unwrap_or(assignment);
    match normalize_property_name(property).as_str() {
        "readonly" | "read-only" | "loop-read-only" => {
            let tool = if truthy_property_value(value) {
                "--setro"
            } else {
                "--setrw"
            };
            command(
                ["blockdev", tool, target],
                true,
                "set loop device read-only mode",
            )
        }
        "directio" | "direct-io" | "loop-direct-io" => {
            let value = if truthy_property_value(value) {
                "on"
            } else {
                "off"
            };
            command_vec(
                vec![
                    "losetup".to_string(),
                    format!("--direct-io={value}"),
                    target.to_string(),
                ],
                true,
                "set loop device direct I/O mode",
            )
        }
        _ => command_with_readiness(
            ["<loop-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported loop property"],
            "apply a loop-device property update after selecting a supported loop property",
        ),
    }
}

fn truthy_property_value(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on" | "enabled"
    )
}

fn vdo_property_command(target: &str, property: &str, assignment: &str) -> ExecutionCommand {
    let value = assignment
        .split_once('=')
        .map(|(_, value)| value)
        .unwrap_or(assignment);
    match normalize_property_name(property).as_str() {
        "writepolicy" | "write-policy" | "vdo-write-policy" => {
            vdo_write_policy_command(target, value)
        }
        "compression" | "vdo-compression" => vdo_boolean_toggle_command(
            target,
            value,
            "enableCompression",
            "disableCompression",
            "compression",
        ),
        "deduplication" | "dedupe" | "vdo-deduplication" | "vdo-dedupe" => {
            vdo_boolean_toggle_command(
                target,
                value,
                "enableDeduplication",
                "disableDeduplication",
                "deduplication",
            )
        }
        _ => command_with_readiness(
            ["<vdo-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported VDO property"],
            "apply a VDO property update after selecting the domain-specific command",
        ),
    }
}

fn vdo_write_policy_command(target: &str, value: &str) -> ExecutionCommand {
    let policy = normalize_property_name(value);
    match policy.as_str() {
        "auto" | "sync" | "async" => command_vec(
            [
                "vdo",
                "changeWritePolicy",
                "--name",
                target,
                "--writePolicy",
                policy.as_str(),
            ],
            true,
            "change VDO write policy",
        ),
        _ => command_with_readiness(
            ["<vdo-property-tool>", target, "writePolicy"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["VDO write policy value"],
            "apply a VDO write policy after choosing auto, sync, or async",
        ),
    }
}

fn vdo_boolean_toggle_command(
    target: &str,
    value: &str,
    enable_command: &'static str,
    disable_command: &'static str,
    label: &'static str,
) -> ExecutionCommand {
    match normalize_property_name(value).as_str() {
        "enabled" | "enable" | "true" | "yes" | "on" => command(
            ["vdo", enable_command, "--name", target],
            true,
            &format!("enable VDO {label}"),
        ),
        "disabled" | "disable" | "false" | "no" | "off" => command(
            ["vdo", disable_command, "--name", target],
            true,
            &format!("disable VDO {label}"),
        ),
        _ => command_with_readiness(
            ["<vdo-property-tool>", target, label],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["boolean VDO property value"],
            "apply a VDO boolean property after choosing enabled or disabled",
        ),
    }
}

fn normalize_property_name(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("vdo.")
        .chars()
        .map(|character| match character {
            'A'..='Z' => character.to_ascii_lowercase(),
            'a'..='z' | '0'..='9' => character,
            _ => '-',
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
