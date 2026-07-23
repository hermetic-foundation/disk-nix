fn filesystem_action_commands(
    action: &PlannedAction,
    collection: Option<&str>,
    target: Option<&str>,
    cache_target: Option<&str>,
) -> Option<ActionCommandResult> {
    let _ = cache_target;
    Some(match action.operation {
        Operation::Grow
            if collection == Some("filesystems") || action.id.starts_with("filesystem:") =>
        {
            let target = target.unwrap_or("<filesystem>");
            let fs_type = action
                .context
                .fs_type
                .as_deref()
                .unwrap_or("<filesystem-type>");
            let device = action.context.device.as_deref();
            let desired_size = action.context.desired_size.as_deref();
            let grow_command = filesystem_grow_command(fs_type, target, device, desired_size);
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "re-read graph state for the filesystem before resizing",
                    ),
                    grow_command,
                ],
                vec![
                    format!(
                        "select the {fs_type} grow command: xfs_growfs, resize2fs, btrfs filesystem resize, zfs set volsize, or equivalent"
                    ),
                    "verify available backing capacity before running the grow command".to_string(),
                ],
                true,
            )
        }
        Operation::Format
            if collection == Some("filesystems") || action.id.starts_with("filesystem:") =>
        {
            let target = target.unwrap_or("<filesystem>");
            let fs_type = action
                .context
                .fs_type
                .as_deref()
                .unwrap_or("<filesystem-type>");
            let device = action.context.device.as_deref();
            let mut commands = vec![
                disk_nix_inspect_command(
                    device,
                    "<filesystem-device>",
                    "filesystem source device",
                    "inspect target device before creating a filesystem signature",
                ),
            ];
            if device.is_some_and(|device| device.starts_with("/dev/md/")) {
                commands.push(command(
                    ["udevadm", "settle"],
                    false,
                    "wait for md device events to settle before formatting",
                ));
            }
            commands.push(filesystem_format_command(fs_type, device));
            if matches!(fs_type, "btrfs" | "bcachefs") {
                if let Some(mountpoint) = filesystem_mountpoint(action) {
                    commands.push(filesystem_mount_command(
                        device,
                        Some(mountpoint),
                        Some(fs_type),
                        action.context.options.as_deref(),
                    ));
                }
            }
            (
                commands,
                vec![
                    format!("formatting {target} as {fs_type} destroys existing data on the selected device"),
                    "prefer preserving or migrating data before replacing a filesystem signature"
                        .to_string(),
                    "mount the new filesystem only after its UUID, label, and stable device path are verified"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Shrink
            if collection == Some("filesystems") || action.id.starts_with("filesystem:") =>
        {
            let target = target.unwrap_or("<filesystem>");
            let fs_type = action
                .context
                .fs_type
                .as_deref()
                .unwrap_or("<filesystem-type>");
            let device = action.context.device.as_deref();
            let desired_size = action.context.desired_size.as_deref();
            (
                filesystem_shrink_commands(fs_type, target, device, desired_size),
                vec![
                    "shrink only after backups or snapshots are verified".to_string(),
                    "prefer migrate-to-smaller-filesystem workflows when online shrink support is absent"
                        .to_string(),
                    "restore dependent mounts and services only after post-shrink checks pass"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Check
            if collection == Some("filesystems") || action.id.starts_with("filesystems:") =>
        {
            let target = target.unwrap_or("<filesystem>");
            let fs_type = action
                .context
                .fs_type
                .as_deref()
                .unwrap_or("<filesystem-type>");
            let device = action.context.device.as_deref();
            (
                filesystem_check_commands(fs_type, target, device),
                vec![
                    "run read-only consistency checks before any repair workflow".to_string(),
                    "quiesce or unmount the filesystem when the checker requires offline access"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Repair
            if collection == Some("filesystems") || action.id.starts_with("filesystems:") =>
        {
            let target = target.unwrap_or("<filesystem>");
            let fs_type = action
                .context
                .fs_type
                .as_deref()
                .unwrap_or("<filesystem-type>");
            let device = action.context.device.as_deref();
            (
                filesystem_repair_commands(fs_type, target, device),
                vec![
                    "repair only after a read-only check and backup review".to_string(),
                    "prefer repairing a cloned device before the production filesystem when practical"
                        .to_string(),
                    "restore mounts and services only after post-repair verification passes"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Remount
            if collection == Some("filesystems") || action.id.starts_with("filesystems:") =>
        {
            let mountpoint = filesystem_mountpoint(action);
            (
                vec![
                    filesystem_findmnt_command(mountpoint),
                    filesystem_remount_command(mountpoint, action.context.options.as_deref()),
                ],
                vec![
                    "review active services before changing filesystem mount options".to_string(),
                    "persist the final options through the NixOS fileSystems entry".to_string(),
                ],
                true,
            )
        }
        Operation::Mount
            if collection == Some("filesystems") || action.id.starts_with("filesystems:") =>
        {
            let mountpoint = filesystem_mountpoint(action);
            (
                vec![filesystem_mount_command(
                    action.context.device.as_deref(),
                    mountpoint,
                    action.context.fs_type.as_deref(),
                    action.context.options.as_deref(),
                )],
                vec![
                    "verify the source device, filesystem type, and mountpoint before mounting"
                        .to_string(),
                    "persist long-lived mounts through the NixOS fileSystems entry".to_string(),
                ],
                true,
            )
        }
        Operation::Unmount
            if collection == Some("filesystems") || action.id.starts_with("filesystems:") =>
        {
            let mountpoint = filesystem_mountpoint(action);
            (
                vec![
                    filesystem_findmnt_command(mountpoint),
                    filesystem_unmount_command(mountpoint),
                ],
                vec![
                    "stop services, automount units, and sessions that depend on the mountpoint before unmounting"
                        .to_string(),
                    "verify no open files, bind mounts, or namespaces still reference the mountpoint"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rescan
            if collection == Some("filesystems") || action.id.starts_with("filesystems:") =>
        {
            let mountpoint = filesystem_mountpoint(action);
            (
                vec![
                    filesystem_findmnt_command(mountpoint),
                    filesystem_inspect_command(
                        mountpoint,
                        false,
                        "refresh modeled filesystem graph state",
                    ),
                ],
                vec![
                    "filesystem rescan is read-only and does not mount, remount, unmount, or format storage"
                        .to_string(),
                    "use the refreshed inventory before selecting any mutating lifecycle action"
                        .to_string(),
                ],
                true,
            )
        }
        _ => return None,
    })
}
