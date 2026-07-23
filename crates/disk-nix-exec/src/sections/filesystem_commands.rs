fn add_device_command(
    collection: Option<&str>,
    fs_type: Option<&str>,
    target: &str,
    device: Option<&str>,
) -> ExecutionCommand {
    let Some(device) = device else {
        if collection == Some("filesystems") && fs_type == Some("bcachefs") {
            return bcachefs_add_device_command(target, None);
        }
        return command_with_readiness(
            ["<add-device-tool>", target, "<device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["device to add"],
            "attach the new device after selecting the reviewed device path or cache-set UUID",
        );
    };
    match collection {
        Some("pools") => command(
            ["zpool", "add", target, device],
            true,
            "attach a vdev or device to a ZFS pool when the pool layout supports it",
        ),
        Some("volumeGroups") => command(
            ["vgextend", target, device],
            true,
            "add a physical volume to an LVM volume group",
        ),
        Some("mdRaids") => command(
            ["mdadm", target, "--add", device],
            true,
            "add a member or spare to an MD RAID array",
        ),
        Some("multipathMaps") => command(
            ["multipathd", "add", "path", device],
            true,
            "add or re-add a path to multipathd",
        ),
        Some("lvmCaches") => {
            lvm_cache_attach_command(lvm_volume_target_path(Some(target)), Some(device))
        }
        Some("caches") => bcache_attach_command(target, device),
        Some("filesystems") if fs_type == Some("bcachefs") => {
            bcachefs_add_device_command(target, Some(device))
        }
        Some("filesystems") => command(
            ["btrfs", "device", "add", device, target],
            true,
            "add a device to a mounted Btrfs filesystem",
        ),
        _ => command_with_readiness(
            ["<add-device-tool>", target, device],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["add-device tool"],
            "attach the new device with the storage-domain-specific tool",
        ),
    }
}

fn replace_device_command(
    collection: Option<&str>,
    fs_type: Option<&str>,
    target: &str,
    from: Option<&str>,
    to: Option<&str>,
) -> ExecutionCommand {
    let from_arg = from.unwrap_or("<old-device>");
    let to_arg = to.unwrap_or("<new-device>");
    let missing = missing_replacement_inputs(from, to);
    if !missing.is_empty() {
        return command_vec_with_readiness(
            vec!["<replace-device-tool>", target, from_arg, to_arg],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "start the storage-domain replacement operation after selecting both devices",
        );
    }
    let from = from.expect("missing replacement source is handled above");
    let to = to.expect("missing replacement target is handled above");
    match collection {
        Some("pools") => command(
            ["zpool", "replace", target, from, to],
            true,
            "replace a ZFS pool device and resilver before detaching the old device",
        ),
        Some("filesystems") if fs_type == Some("bcachefs") => command_vec(
            vec![
                "sh".to_string(),
                "-c".to_string(),
                format!(
                    "bcachefs device add {} {} && bcachefs data rereplicate {} && bcachefs device remove {} {}",
                    shell_quote(target),
                    shell_quote(to),
                    shell_quote(target),
                    shell_quote(target),
                    shell_quote(from)
                ),
            ],
            true,
            "replace a bcachefs member by adding replacement capacity, rereplicating, then removing the old device",
        ),
        Some("filesystems") => command(
            ["btrfs", "replace", "start", from, to, target],
            true,
            "replace a Btrfs filesystem device",
        ),
        Some("mdRaids") => command(
            ["mdadm", target, "--replace", from, "--with", to],
            true,
            "replace an MD RAID member while preserving array redundancy",
        ),
        Some("multipathMaps") => command_vec(
            vec![
                "sh".to_string(),
                "-c".to_string(),
                format!(
                    "multipathd add path {} && multipathd del path {}",
                    shell_quote(to),
                    shell_quote(from)
                ),
            ],
            true,
            "add the replacement multipath path before deleting the old path",
        ),
        Some("lvmCaches") => {
            lvm_cache_replace_command(lvm_volume_target_path(Some(target)), Some(from), Some(to))
        }
        Some("caches") => bcache_replace_command(target, from, to, None),
        _ => command_with_readiness(
            ["<replace-device-tool>", target, from, to],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["replace-device tool"],
            "start the storage-domain replacement operation",
        ),
    }
}

fn missing_replacement_inputs(from: Option<&str>, to: Option<&str>) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if from.is_none() {
        missing.push("device to replace");
    }
    if to.is_none() {
        missing.push("replacement device");
    }
    missing
}

fn zpool_remove_device_command(target: &str, device: Option<&str>) -> ExecutionCommand {
    match device {
        Some(device) => command(
            ["zpool", "remove", target, device],
            true,
            "remove the reviewed device from the ZFS pool when the layout supports evacuation",
        ),
        None => command_with_readiness(
            ["zpool", "remove", target, "<device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["device to remove"],
            "remove a ZFS pool device after selecting the reviewed vdev or device",
        ),
    }
}

fn lvm_physical_volume_inspect_command(device: Option<&str>) -> ExecutionCommand {
    match device {
        Some(device) => command(
            ["pvs", "--reportformat", "json", device],
            false,
            "inspect physical volume allocation before vgreduce",
        ),
        None => command_with_readiness(
            ["pvs", "--reportformat", "json", "<physical-volume>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["physical volume to remove"],
            "inspect physical volume allocation after selecting the reviewed PV",
        ),
    }
}

fn lvm_physical_volume_move_command(device: Option<&str>) -> ExecutionCommand {
    match device {
        Some(device) => command(
            ["pvmove", device],
            true,
            "evacuate allocated extents from the reviewed physical volume before vgreduce",
        ),
        None => command_with_readiness(
            ["pvmove", "<physical-volume>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["physical volume to remove"],
            "evacuate allocated extents after selecting the reviewed physical volume",
        ),
    }
}

fn lvm_volume_group_reduce_command(target: &str, device: Option<&str>) -> ExecutionCommand {
    match device {
        Some(device) => command(
            ["vgreduce", target, device],
            true,
            "remove the reviewed physical volume from the LVM volume group after extents are evacuated",
        ),
        None => command_with_readiness(
            ["vgreduce", target, "<physical-volume>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["physical volume to remove"],
            "remove the physical volume from the volume group after selecting it",
        ),
    }
}

fn md_array_target_path(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .target
        .as_deref()
        .filter(|target| target.starts_with("/dev/md"))
        .or_else(|| {
            action
                .context
                .name
                .as_deref()
                .filter(|name| name.starts_with("/dev/md"))
        })
}

fn md_raid_detail_command(target: Option<&str>, note: &'static str) -> ExecutionCommand {
    match target {
        Some(target) => command(["mdadm", "--detail", target], false, note),
        None => command_with_readiness(
            ["mdadm", "--detail", "<md-array>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["MD array path"],
            note,
        ),
    }
}

fn md_raid_add_member_command(target: Option<&str>, device: Option<&str>) -> ExecutionCommand {
    let target_arg = target.unwrap_or("<md-array>");
    match (target, device) {
        (Some(_), Some(device)) => command(
            ["mdadm", target_arg, "--add", device],
            true,
            "add the reviewed member or spare to the MD RAID array",
        ),
        _ => command_vec_with_readiness(
            vec!["mdadm", target_arg, "--add", device.unwrap_or("<device>")],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_md_member_operation_inputs(target, device),
            "add the MD RAID member after selecting the array and member",
        ),
    }
}

fn md_raid_fail_member_command(target: Option<&str>, device: Option<&str>) -> ExecutionCommand {
    let target_arg = target.unwrap_or("<md-array>");
    match (target, device) {
        (Some(_), Some(device)) => command(
            ["mdadm", target_arg, "--fail", device],
            true,
            "mark the MD RAID member failed before removal",
        ),
        _ => command_vec_with_readiness(
            vec!["mdadm", target_arg, "--fail", device.unwrap_or("<device>")],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_md_member_operation_inputs(target, device),
            "mark the MD RAID member failed after selecting the array and member",
        ),
    }
}

fn md_raid_remove_member_command(target: Option<&str>, device: Option<&str>) -> ExecutionCommand {
    let target_arg = target.unwrap_or("<md-array>");
    match (target, device) {
        (Some(_), Some(device)) => command(
            ["mdadm", target_arg, "--remove", device],
            true,
            "remove the reviewed MD RAID member",
        ),
        _ => command_vec_with_readiness(
            vec![
                "mdadm",
                target_arg,
                "--remove",
                device.unwrap_or("<device>"),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_md_member_operation_inputs(target, device),
            "remove the MD RAID member after selecting the array and member",
        ),
    }
}

fn md_raid_replace_member_command(
    target: Option<&str>,
    source: Option<&str>,
    replacement: Option<&str>,
) -> ExecutionCommand {
    let target_arg = target.unwrap_or("<md-array>");
    let source_arg = source.unwrap_or("<old-device>");
    let replacement_arg = replacement.unwrap_or("<new-device>");
    let mut missing = Vec::new();
    if target.is_none() {
        missing.push("MD array path");
    }
    if source.is_none() {
        missing.push("device to replace");
    }
    if replacement.is_none() {
        missing.push("replacement device");
    }

    if missing.is_empty() {
        command(
            [
                "mdadm",
                target_arg,
                "--replace",
                source_arg,
                "--with",
                replacement_arg,
            ],
            true,
            "replace an MD RAID member while preserving array redundancy",
        )
    } else {
        command_vec_with_readiness(
            vec![
                "mdadm",
                target_arg,
                "--replace",
                source_arg,
                "--with",
                replacement_arg,
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "replace the MD RAID member after selecting the array, old member, and replacement",
        )
    }
}

fn missing_md_member_operation_inputs(
    target: Option<&str>,
    device: Option<&str>,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if target.is_none() {
        missing.push("MD array path");
    }
    if device.is_none() {
        missing.push("member device to remove");
    }
    missing
}

fn multipath_add_path_command(path: Option<&str>) -> ExecutionCommand {
    match path {
        Some(path) => command(
            ["multipathd", "add", "path", path],
            true,
            "add or re-add the reviewed path to multipathd",
        ),
        None => command_with_readiness(
            ["multipathd", "add", "path", "<path>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["multipath path to add"],
            "add the multipath path after selecting the reviewed path",
        ),
    }
}

fn multipath_delete_path_command(path: Option<&str>) -> ExecutionCommand {
    match path {
        Some(path) => command(
            ["multipathd", "del", "path", path],
            true,
            "delete the reviewed path from multipathd",
        ),
        None => command_with_readiness(
            ["multipathd", "del", "path", "<path>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["multipath path to remove"],
            "delete the multipath path after selecting the reviewed path",
        ),
    }
}

fn multipath_map_target(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .target
        .as_deref()
        .filter(|target| is_multipath_map_target(target))
        .or_else(|| {
            action
                .context
                .name
                .as_deref()
                .filter(|name| is_multipath_map_target(name))
        })
}

fn is_multipath_map_target(target: &str) -> bool {
    target.starts_with("mpath") || target.starts_with("/dev/mapper/")
}

fn multipath_list_command(target: Option<&str>, note: &'static str) -> ExecutionCommand {
    match target {
        Some(target) => command(["multipath", "-ll", target], false, note),
        None => command_with_readiness(
            ["multipath", "-ll", "<multipath-map>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["multipath map target"],
            note,
        ),
    }
}

fn multipath_resize_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["multipathd", "resize", "map", target],
            true,
            "resize the multipath map after every backing path sees the new LUN size",
        ),
        None => command_with_readiness(
            ["multipathd", "resize", "map", "<multipath-map>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["multipath map target"],
            "resize the multipath map after every backing path sees the new LUN size",
        ),
    }
}

fn multipath_flush_map_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["multipath", "-f", target],
            true,
            "flush the reviewed multipath map from the host",
        ),
        None => command_with_readiness(
            ["multipath", "-f", "<multipath-map>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["multipath map target"],
            "flush the multipath map after selecting a concrete map target",
        ),
    }
}

fn btrfs_remove_device_command(target: &str, device: Option<&str>) -> ExecutionCommand {
    match device {
        Some(device) => command(
            ["btrfs", "device", "remove", device, target],
            true,
            "remove the reviewed device from the Btrfs filesystem after data evacuation checks",
        ),
        None => command_with_readiness(
            ["btrfs", "device", "remove", "<device>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["device to remove"],
            "remove the Btrfs device after selecting the reviewed device",
        ),
    }
}

fn bcachefs_usage_command(target: &str, note: &'static str) -> ExecutionCommand {
    command(["bcachefs", "fs", "usage", target], false, note)
}

fn bcachefs_add_device_command(target: &str, device: Option<&str>) -> ExecutionCommand {
    match device {
        Some(device) => command(
            ["bcachefs", "device", "add", target, device],
            true,
            "add the reviewed device to the mounted bcachefs filesystem",
        ),
        None => command_with_readiness(
            ["bcachefs", "device", "add", target, "<device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["device to add"],
            "add a bcachefs member after selecting the reviewed device",
        ),
    }
}

fn bcachefs_remove_device_command(target: &str, device: Option<&str>) -> ExecutionCommand {
    match device {
        Some(device) => command(
            ["bcachefs", "device", "remove", target, device],
            true,
            "remove the reviewed device from the mounted bcachefs filesystem",
        ),
        None => command_with_readiness(
            ["bcachefs", "device", "remove", target, "<device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["device to remove"],
            "remove a bcachefs member after selecting the reviewed device",
        ),
    }
}

fn bcachefs_rereplicate_command(target: &str) -> ExecutionCommand {
    command(
        ["bcachefs", "data", "rereplicate", target],
        true,
        "rereplicate bcachefs data after topology or replica-policy changes",
    )
}

fn bcachefs_device_resize_command(
    device: Option<&str>,
    desired_size: Option<&str>,
) -> ExecutionCommand {
    match (device, desired_size) {
        (Some(device), Some(size)) => command(
            ["bcachefs", "device", "resize", device, size],
            true,
            "resize the reviewed bcachefs member device to the desired size",
        ),
        (Some(device), None) => command_with_readiness(
            ["bcachefs", "device", "resize", device, "<size>"],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired bcachefs member size"],
            "resize the reviewed bcachefs member after selecting the desired size",
        ),
        (None, Some(size)) => command_vec_with_readiness(
            vec!["bcachefs", "device", "resize", "<bcachefs-device>", size],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["bcachefs member device"],
            "resize the bcachefs member after selecting the device",
        ),
        (None, None) => command_with_readiness(
            [
                "bcachefs",
                "device",
                "resize",
                "<bcachefs-device>",
                "<size>",
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["bcachefs member device", "desired bcachefs member size"],
            "resize the bcachefs member after selecting device and desired size",
        ),
    }
}

fn rebalance_command(
    collection: Option<&str>,
    fs_type: Option<&str>,
    target: &str,
    property_assignments: &[String],
) -> ExecutionCommand {
    match collection {
        Some("pools") => command(
            ["zpool", "scrub", target],
            true,
            "scrub the pool after topology changes; ZFS has no generic rebalance command",
        ),
        Some("filesystems") if fs_type == Some("bcachefs") => bcachefs_rereplicate_command(target),
        Some("filesystems") => {
            let mut argv = vec![
                "btrfs".to_string(),
                "balance".to_string(),
                "start".to_string(),
            ];
            argv.extend(btrfs_balance_filters(property_assignments));
            argv.push(target.to_string());
            command_vec(
                argv,
                true,
                "rebalance Btrfs chunks across available devices",
            )
        }
        _ => command_with_readiness(
            ["<rebalance-tool>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["rebalance tool"],
            "run the storage-domain rebalance command",
        ),
    }
}

fn scrub_command(
    collection: Option<&str>,
    fs_type: Option<&str>,
    target: &str,
) -> ExecutionCommand {
    match collection {
        Some("pools") => command(
            ["zpool", "scrub", target],
            true,
            "start the reviewed ZFS pool scrub",
        ),
        Some("filesystems") if fs_type == Some("bcachefs") => command(
            ["bcachefs", "scrub", target],
            true,
            "run the reviewed bcachefs scrub",
        ),
        Some("filesystems") => command(
            ["btrfs", "scrub", "start", "-B", target],
            true,
            "run the reviewed Btrfs scrub and wait for completion",
        ),
        _ => command_with_readiness(
            ["<scrub-tool>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["scrub tool"],
            "run the storage-domain scrub command",
        ),
    }
}

fn filesystem_trim_command(collection: Option<&str>, target: &str) -> ExecutionCommand {
    match collection {
        Some("filesystems") => command(
            ["fstrim", "-v", target],
            true,
            "trim unused blocks from the mounted filesystem",
        ),
        _ => command_with_readiness(
            ["<trim-tool>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["trim tool"],
            "run the storage-domain trim or discard command",
        ),
    }
}

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

fn filesystem_property_command(
    fs_type: Option<&str>,
    target: &str,
    device: Option<&str>,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    match fs_type {
        Some("btrfs") => btrfs_filesystem_property_command(target, device, property, assignment),
        Some("ext2" | "ext3" | "ext4") => {
            ext_filesystem_property_command(device, target, property, assignment)
        }
        Some("fat" | "fat12" | "fat16" | "fat32" | "msdos" | "vfat") => {
            fat_filesystem_property_command(device, target, property, assignment)
        }
        Some("ntfs" | "ntfs3") => {
            ntfs_filesystem_property_command(device, target, property, assignment)
        }
        Some("exfat") => exfat_filesystem_property_command(device, target, property, assignment),
        Some("f2fs") => f2fs_filesystem_property_command(device, target, property, assignment),
        Some("xfs") => xfs_filesystem_property_command(device, target, property, assignment),
        Some("zfs") => command(
            ["zfs", "set", assignment, target],
            true,
            "set a ZFS filesystem property",
        ),
        _ => command_with_readiness(
            ["<filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem type", "supported filesystem property"],
            "set a filesystem property after selecting the filesystem-specific command",
        ),
    }
}

fn swap_property_command(
    target: Option<&str>,
    property: &str,
    value: Option<&str>,
) -> ExecutionCommand {
    match (property, target, value) {
        ("label" | "swap.label", Some(target), Some(value)) => command(
            ["swaplabel", "--label", value, target],
            true,
            "set the swap signature label on the reviewed inactive swap target",
        ),
        ("label" | "swap.label", None, Some(value)) => command_with_readiness(
            ["swaplabel", "--label", value, "<swap>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["swap target path"],
            "set the swap label after resolving the swap target",
        ),
        ("label" | "swap.label", Some(target), None) => command_with_readiness(
            ["swaplabel", "--label", "<label>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["swap label"],
            "set the swap label after resolving the desired label",
        ),
        ("label" | "swap.label", None, None) => command_with_readiness(
            ["swaplabel", "--label", "<label>", "<swap>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["swap target path", "swap label"],
            "set the swap label after resolving target and label",
        ),
        ("uuid" | "swap.uuid", Some(target), Some(value)) => command(
            ["swaplabel", "--uuid", value, target],
            true,
            "set the swap signature UUID on the reviewed inactive swap target",
        ),
        ("uuid" | "swap.uuid", None, Some(value)) => command_with_readiness(
            ["swaplabel", "--uuid", value, "<swap>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["swap target path"],
            "set the swap UUID after resolving the swap target",
        ),
        ("uuid" | "swap.uuid", Some(target), None) => command_with_readiness(
            ["swaplabel", "--uuid", "<uuid>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["swap UUID"],
            "set the swap UUID after resolving the desired UUID",
        ),
        ("uuid" | "swap.uuid", None, None) => command_with_readiness(
            ["swaplabel", "--uuid", "<uuid>", "<swap>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["swap target path", "swap UUID"],
            "set the swap UUID after resolving target and UUID",
        ),
        ("priority" | "swap.priority", Some(target), Some(value))
            if value.parse::<i32>().is_ok() =>
        {
            command_vec(
                vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    format!(
                        "swapoff {} 2>/dev/null || true; swapon --priority {} {}",
                        shell_quote(target),
                        shell_quote(value),
                        shell_quote(target)
                    ),
                ],
                true,
                "reactivate the reviewed swap target with the requested priority",
            )
        }
        ("priority" | "swap.priority", None, Some(value)) if value.parse::<i32>().is_ok() => {
            command_with_readiness(
                ["swapon", "--priority", value, "<swap>"],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["swap target path"],
                "reactivate swap with the requested priority after resolving the target",
            )
        }
        ("priority" | "swap.priority", Some(target), Some(_)) => command_with_readiness(
            ["swapon", "--priority", "<priority>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["integer swap priority"],
            "reactivate swap after resolving an integer priority",
        ),
        ("priority" | "swap.priority", Some(target), None) => command_with_readiness(
            ["swapon", "--priority", "<priority>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["integer swap priority"],
            "reactivate swap after resolving the requested priority",
        ),
        ("priority" | "swap.priority", None, _) => command_with_readiness(
            ["swapon", "--priority", "<priority>", "<swap>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["swap target path", "integer swap priority"],
            "reactivate swap after resolving target and priority",
        ),
        _ => command_with_readiness(
            ["<swap-property-tool>", target.unwrap_or("<swap>"), property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported swap property"],
            "set a swap property after selecting a supported property mapping",
        ),
    }
}

fn fat_filesystem_property_command(
    device: Option<&str>,
    target: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<fat-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["FAT filesystem property value"],
            "set a FAT filesystem property after resolving the desired value",
        );
    };
    match (property, device) {
        ("label" | "fat.label" | "vfat.label" | "filesystem.label", Some(device)) => command(
            ["fatlabel", device, value],
            true,
            "set the FAT filesystem label on the reviewed backing device",
        ),
        ("label" | "fat.label" | "vfat.label" | "filesystem.label", None) => {
            command_with_readiness(
                ["fatlabel", "<filesystem-device>", value],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["filesystem source device"],
                "set the FAT filesystem label after resolving the backing device",
            )
        }
        (
            "uuid" | "fat.uuid" | "vfat.uuid" | "filesystem.uuid" | "volumeId" | "volume-id"
            | "fat.volume-id" | "vfat.volume-id",
            Some(device),
        ) => match fat_volume_id(value) {
            Some(volume_id) => command_vec(
                ["fatlabel", "-i", device, volume_id.as_str()],
                true,
                "set the FAT filesystem volume ID on the reviewed unmounted backing device",
            ),
            None => command_with_readiness(
                ["<fat-filesystem-property-tool>", target, property],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["8-hex-digit FAT volume ID"],
                "set a FAT filesystem volume ID after resolving a valid value",
            ),
        },
        (
            "uuid" | "fat.uuid" | "vfat.uuid" | "filesystem.uuid" | "volumeId" | "volume-id"
            | "fat.volume-id" | "vfat.volume-id",
            None,
        ) => match fat_volume_id(value) {
            Some(volume_id) => command_vec_with_readiness(
                ["fatlabel", "-i", "<filesystem-device>", volume_id.as_str()],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["filesystem source device"],
                "set the FAT filesystem volume ID after resolving the backing device",
            ),
            None => command_with_readiness(
                ["<fat-filesystem-property-tool>", target, property],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["filesystem source device", "8-hex-digit FAT volume ID"],
                "set a FAT filesystem volume ID after resolving device and value",
            ),
        },
        _ => command_with_readiness(
            ["<fat-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported FAT filesystem property"],
            "set a FAT filesystem property after selecting a supported property mapping",
        ),
    }
}

fn fat_volume_id(value: &str) -> Option<String> {
    let normalized: String = value
        .trim()
        .chars()
        .filter(|character| *character != '-')
        .collect();
    if normalized.len() == 8
        && normalized
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        Some(normalized.to_ascii_uppercase())
    } else {
        None
    }
}

fn ntfs_filesystem_property_command(
    device: Option<&str>,
    target: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<ntfs-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["NTFS filesystem property value"],
            "set an NTFS filesystem property after resolving the desired value",
        );
    };
    match (property, device) {
        ("label" | "ntfs.label" | "filesystem.label", Some(device)) => command(
            ["ntfslabel", device, value],
            true,
            "set the NTFS filesystem label on the reviewed backing device",
        ),
        ("label" | "ntfs.label" | "filesystem.label", None) => command_with_readiness(
            ["ntfslabel", "<filesystem-device>", value],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "set the NTFS filesystem label after resolving the backing device",
        ),
        (
            "uuid" | "ntfs.uuid" | "filesystem.uuid" | "serial" | "volumeSerial" | "volume-serial"
            | "ntfs.serial" | "ntfs.volume-serial",
            Some(device),
        ) => match ntfs_volume_serial(value) {
            Some(serial) => command_vec(
                vec![
                    "ntfslabel".to_string(),
                    format!("--new-serial={serial}"),
                    device.to_string(),
                ],
                true,
                "set the NTFS filesystem volume serial on the reviewed unmounted backing device",
            ),
            None => command_with_readiness(
                ["<ntfs-filesystem-property-tool>", target, property],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["16-hex-digit NTFS volume serial"],
                "set an NTFS filesystem volume serial after resolving a valid value",
            ),
        },
        (
            "uuid" | "ntfs.uuid" | "filesystem.uuid" | "serial" | "volumeSerial" | "volume-serial"
            | "ntfs.serial" | "ntfs.volume-serial",
            None,
        ) => match ntfs_volume_serial(value) {
            Some(serial) => command_vec_with_readiness(
                vec![
                    "ntfslabel".to_string(),
                    format!("--new-serial={serial}"),
                    "<filesystem-device>".to_string(),
                ],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["filesystem source device"],
                "set the NTFS filesystem volume serial after resolving the backing device",
            ),
            None => command_with_readiness(
                ["<ntfs-filesystem-property-tool>", target, property],
                true,
                CommandReadiness::NeedsDomainImplementation,
                [
                    "filesystem source device",
                    "16-hex-digit NTFS volume serial",
                ],
                "set an NTFS filesystem volume serial after resolving device and value",
            ),
        },
        _ => command_with_readiness(
            ["<ntfs-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported NTFS filesystem property"],
            "set an NTFS filesystem property after selecting a supported property mapping",
        ),
    }
}

fn ntfs_volume_serial(value: &str) -> Option<String> {
    let normalized: String = value
        .trim()
        .chars()
        .filter(|character| *character != '-')
        .collect();
    if normalized.len() == 16
        && normalized
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        Some(normalized.to_ascii_uppercase())
    } else {
        None
    }
}

fn exfat_filesystem_property_command(
    device: Option<&str>,
    target: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<exfat-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["exFAT filesystem property value"],
            "set an exFAT filesystem property after resolving the desired value",
        );
    };
    match (property, device) {
        ("label" | "exfat.label" | "filesystem.label", Some(device)) => command(
            ["exfatlabel", device, value],
            true,
            "set the exFAT filesystem label on the reviewed backing device",
        ),
        ("label" | "exfat.label" | "filesystem.label", None) => command_with_readiness(
            ["exfatlabel", "<filesystem-device>", value],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "set the exFAT filesystem label after resolving the backing device",
        ),
        (
            "uuid"
            | "exfat.uuid"
            | "filesystem.uuid"
            | "serial"
            | "volumeSerial"
            | "volume-serial"
            | "exfat.serial"
            | "exfat.volume-serial",
            Some(device),
        ) => match exfat_volume_serial(value) {
            Some(serial) => command_vec(
                ["exfatlabel", "-i", device, serial.as_str()],
                true,
                "set the exFAT filesystem volume serial on the reviewed unmounted backing device",
            ),
            None => command_with_readiness(
                ["<exfat-filesystem-property-tool>", target, property],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["8-hex-digit exFAT volume serial"],
                "set an exFAT filesystem volume serial after resolving a valid value",
            ),
        },
        (
            "uuid"
            | "exfat.uuid"
            | "filesystem.uuid"
            | "serial"
            | "volumeSerial"
            | "volume-serial"
            | "exfat.serial"
            | "exfat.volume-serial",
            None,
        ) => match exfat_volume_serial(value) {
            Some(serial) => command_vec_with_readiness(
                ["exfatlabel", "-i", "<filesystem-device>", serial.as_str()],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["filesystem source device"],
                "set the exFAT filesystem volume serial after resolving the backing device",
            ),
            None => command_with_readiness(
                ["<exfat-filesystem-property-tool>", target, property],
                true,
                CommandReadiness::NeedsDomainImplementation,
                [
                    "filesystem source device",
                    "8-hex-digit exFAT volume serial",
                ],
                "set an exFAT filesystem volume serial after resolving device and value",
            ),
        },
        _ => command_with_readiness(
            ["<exfat-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported exFAT filesystem property"],
            "set an exFAT filesystem property after selecting a supported property mapping",
        ),
    }
}

fn exfat_volume_serial(value: &str) -> Option<String> {
    let normalized: String = value
        .trim()
        .chars()
        .filter(|character| *character != '-')
        .collect();
    if normalized.len() == 8
        && normalized
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        Some(normalized.to_ascii_uppercase())
    } else {
        None
    }
}

fn f2fs_filesystem_property_command(
    device: Option<&str>,
    target: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<f2fs-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["F2FS filesystem property value"],
            "set an F2FS filesystem property after resolving the desired value",
        );
    };
    match (property, filesystem_source_device(target, device)) {
        ("label" | "f2fs.label" | "filesystem.label", Some(source)) => command(
            ["f2fslabel", source, value],
            true,
            "set the F2FS filesystem label on the reviewed backing device",
        ),
        ("label" | "f2fs.label" | "filesystem.label", None) => command_with_readiness(
            ["f2fslabel", "<filesystem-device>", value],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "set the F2FS filesystem label after resolving the backing device",
        ),
        _ => command_with_readiness(
            ["<f2fs-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported F2FS filesystem property"],
            "set an F2FS filesystem property after selecting a supported property mapping",
        ),
    }
}

fn xfs_filesystem_property_command(
    device: Option<&str>,
    target: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<xfs-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["XFS filesystem property value"],
            "set an XFS filesystem property after resolving the desired value",
        );
    };
    match (property, device) {
        ("label" | "xfs.label" | "filesystem.label", Some(device)) => command(
            ["xfs_admin", "-L", value, device],
            true,
            "set the XFS filesystem label on the reviewed backing device",
        ),
        ("label" | "xfs.label" | "filesystem.label", None) => command_with_readiness(
            ["xfs_admin", "-L", value, "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "set the XFS filesystem label after resolving the backing device",
        ),
        ("uuid" | "xfs.uuid" | "filesystem.uuid", Some(device)) => command(
            ["xfs_admin", "-U", value, device],
            true,
            "set the XFS filesystem UUID on the reviewed unmounted backing device",
        ),
        ("uuid" | "xfs.uuid" | "filesystem.uuid", None) => command_with_readiness(
            ["xfs_admin", "-U", value, "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "set the XFS filesystem UUID after resolving the backing device",
        ),
        _ => command_with_readiness(
            ["<xfs-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported XFS filesystem property"],
            "set an XFS filesystem property after selecting a supported property mapping",
        ),
    }
}

fn ext_filesystem_property_command(
    device: Option<&str>,
    target: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<ext-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["Ext filesystem property value"],
            "set an Ext filesystem property after resolving the desired value",
        );
    };
    match (property, device) {
        ("label" | "ext.label" | "filesystem.label", Some(device)) => command(
            ["e2label", device, value],
            true,
            "set the Ext filesystem label on the reviewed backing device",
        ),
        ("label" | "ext.label" | "filesystem.label", None) => command_with_readiness(
            ["e2label", "<filesystem-device>", value],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "set the Ext filesystem label after resolving the backing device",
        ),
        ("uuid" | "ext.uuid" | "filesystem.uuid", Some(device)) => command(
            ["tune2fs", "-U", value, device],
            true,
            "set the Ext filesystem UUID on the reviewed unmounted backing device",
        ),
        ("uuid" | "ext.uuid" | "filesystem.uuid", None) => command_with_readiness(
            ["tune2fs", "-U", value, "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "set the Ext filesystem UUID after resolving the backing device",
        ),
        _ => command_with_readiness(
            ["<ext-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported Ext filesystem property"],
            "set an Ext filesystem property after selecting a supported property mapping",
        ),
    }
}

fn btrfs_filesystem_property_command(
    target: &str,
    device: Option<&str>,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<btrfs-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["Btrfs filesystem property value"],
            "set a Btrfs filesystem property after resolving the desired value",
        );
    };
    match property {
        "label" | "btrfs.label" | "filesystem.label" => command(
            ["btrfs", "filesystem", "label", target, value],
            true,
            "set the Btrfs filesystem label",
        ),
        "uuid" | "btrfs.uuid" | "filesystem.uuid" => match device {
            Some(device) => command(
                ["btrfstune", "-U", value, device],
                true,
                "set the Btrfs filesystem UUID on the reviewed unmounted backing device",
            ),
            None => command_with_readiness(
                ["btrfstune", "-U", value, "<filesystem-device>"],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["filesystem source device"],
                "set the Btrfs filesystem UUID after resolving the backing device",
            ),
        },
        _ => command_with_readiness(
            ["<btrfs-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported Btrfs filesystem property"],
            "set a Btrfs filesystem property after selecting a supported property mapping",
        ),
    }
}

fn btrfs_subvolume_property_command(
    target: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<btrfs-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["Btrfs property value"],
            "set a Btrfs subvolume property after resolving the desired value",
        );
    };
    let property_name = match property {
        "ro" | "readonly" | "readOnly" | "btrfs.readonly" | "btrfs.ro" => "ro",
        _ => {
            return command_with_readiness(
                ["<btrfs-property-tool>", target, property],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["supported Btrfs subvolume property"],
                "set a Btrfs subvolume property after selecting a supported property mapping",
            );
        }
    };
    command_vec(
        vec![
            "btrfs".to_string(),
            "property".to_string(),
            "set".to_string(),
            "-ts".to_string(),
            target.to_string(),
            property_name.to_string(),
            normalize_boolish_btrfs_property_value(value),
        ],
        true,
        "set a Btrfs subvolume property",
    )
}

fn btrfs_qgroup_property_command(
    target: &str,
    qgroup_id: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<btrfs-qgroup-tool>", target, qgroup_id],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["Btrfs qgroup limit value"],
            "set a Btrfs qgroup limit after resolving the desired value",
        );
    };
    if target == qgroup_id || target.starts_with("0/") {
        return command_with_readiness(
            ["btrfs", "qgroup", "limit", value, qgroup_id, "<path>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["mounted Btrfs filesystem path"],
            "set a Btrfs qgroup limit after selecting the mounted filesystem path",
        );
    }
    let limit_value = normalize_btrfs_qgroup_limit(value);
    match property {
        "limit" | "maxReferenced" | "max-referenced" | "referenced" | "btrfs.max-referenced" => {
            command_vec(
                vec![
                    "btrfs".to_string(),
                    "qgroup".to_string(),
                    "limit".to_string(),
                    limit_value,
                    qgroup_id.to_string(),
                    target.to_string(),
                ],
                true,
                "set a Btrfs qgroup referenced-byte limit",
            )
        }
        "maxExclusive" | "max-exclusive" | "exclusive" | "btrfs.max-exclusive" => command_vec(
            vec![
                "btrfs".to_string(),
                "qgroup".to_string(),
                "limit".to_string(),
                "-e".to_string(),
                limit_value,
                qgroup_id.to_string(),
                target.to_string(),
            ],
            true,
            "set a Btrfs qgroup exclusive-byte limit",
        ),
        _ => command_with_readiness(
            ["<btrfs-qgroup-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported Btrfs qgroup property"],
            "set a Btrfs qgroup property after selecting a supported property mapping",
        ),
    }
}

fn btrfs_qgroup_target_path<'a>(target: Option<&'a str>, qgroup_id: &str) -> Option<&'a str> {
    let target = target?;
    if target == qgroup_id || target.starts_with("0/") {
        None
    } else {
        Some(target)
    }
}

fn normalize_btrfs_qgroup_limit(value: &str) -> String {
    match value {
        "null" | "none" | "None" | "NONE" | "unlimited" => "none".to_string(),
        other => other.to_string(),
    }
}

fn normalize_boolish_btrfs_property_value(value: &str) -> String {
    match value {
        "1" | "yes" | "on" | "true" => "true".to_string(),
        "0" | "no" | "off" | "false" => "false".to_string(),
        other => other.to_string(),
    }
}
