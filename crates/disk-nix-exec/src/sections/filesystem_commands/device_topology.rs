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
