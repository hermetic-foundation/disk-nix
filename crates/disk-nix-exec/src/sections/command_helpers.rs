fn unimplemented_action_command(
    action: &PlannedAction,
    collection: Option<&str>,
    target: Option<&str>,
) -> ExecutionCommand {
    let operation = operation_name(action.operation);
    let collection_arg = collection.unwrap_or("<collection>");
    let target_arg = target.unwrap_or("<target>");
    let mut unresolved_inputs = vec!["storage-domain command renderer".to_string()];
    if collection.is_none() {
        unresolved_inputs.push("storage collection".to_string());
    }
    if target.is_none() {
        unresolved_inputs.push("storage target".to_string());
    }

    command_vec_with_readiness(
        vec![
            "disk-nix".to_string(),
            "storage-action".to_string(),
            operation.clone(),
            "--collection".to_string(),
            collection_arg.to_string(),
            "--target".to_string(),
            target_arg.to_string(),
        ],
        true,
        CommandReadiness::NeedsDomainImplementation,
        unresolved_inputs,
        &format!("render a domain-specific {operation} command before execution"),
    )
}

fn operation_name(operation: Operation) -> String {
    match serde_json::to_value(operation) {
        Ok(serde_json::Value::String(value)) => value,
        _ => format!("{operation:?}").to_ascii_lowercase(),
    }
}

fn command<const N: usize>(argv: [&str; N], mutates: bool, note: &str) -> ExecutionCommand {
    command_with_readiness(argv, mutates, CommandReadiness::Ready, [], note)
}

fn command_vec<I, S>(argv: I, mutates: bool, note: &str) -> ExecutionCommand
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    command_vec_with_readiness(
        argv,
        mutates,
        CommandReadiness::Ready,
        Vec::<&str>::new(),
        note,
    )
}

fn command_with_readiness<const N: usize, const M: usize>(
    argv: [&str; N],
    mutates: bool,
    readiness: CommandReadiness,
    unresolved_inputs: [&str; M],
    note: &str,
) -> ExecutionCommand {
    ExecutionCommand {
        argv: argv.iter().map(|value| (*value).to_string()).collect(),
        mutates,
        readiness,
        unresolved_inputs: unresolved_inputs
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        provider_capabilities: Vec::new(),
        note: note.to_string(),
    }
}

fn command_vec_with_readiness<I, S, U, T>(
    argv: I,
    mutates: bool,
    readiness: CommandReadiness,
    unresolved_inputs: U,
    note: &str,
) -> ExecutionCommand
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
    U: IntoIterator<Item = T>,
    T: Into<String>,
{
    ExecutionCommand {
        argv: argv.into_iter().map(Into::into).collect(),
        mutates,
        readiness,
        unresolved_inputs: unresolved_inputs.into_iter().map(Into::into).collect(),
        provider_capabilities: Vec::new(),
        note: note.to_string(),
    }
}

fn filesystem_grow_command(
    fs_type: &str,
    target: &str,
    device: Option<&str>,
    desired_size: Option<&str>,
) -> ExecutionCommand {
    match fs_type {
        "xfs" => command(
            ["xfs_growfs", target],
            true,
            "grow an already-mounted XFS filesystem",
        ),
        "ext2" | "ext3" | "ext4" => ext_filesystem_grow_command(target, device, desired_size),
        "btrfs" => command_vec(
            vec![
                "btrfs",
                "filesystem",
                "resize",
                desired_size.unwrap_or("max"),
                target,
            ],
            true,
            "grow a Btrfs filesystem to the requested or maximum visible device size",
        ),
        "bcachefs" => bcachefs_device_resize_command(device, desired_size),
        "f2fs" => f2fs_filesystem_grow_command(target, device, desired_size),
        "zfs" => match desired_size {
            Some(size) => command_vec(
                vec![
                    "zfs".to_string(),
                    "set".to_string(),
                    format!("volsize={size}"),
                    target.to_string(),
                ],
                true,
                "set the ZFS volume size to the desired size",
            ),
            None => command_with_readiness(
                ["zfs", "set", "volsize=<size>", target],
                true,
                CommandReadiness::NeedsDesiredSize,
                ["desired zvol size"],
                "set the ZFS volume size after selecting the desired size",
            ),
        },
        _ => command_with_readiness(
            ["<filesystem-grow-tool>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem grow tool"],
            "run the filesystem-specific online grow command after device growth is visible",
        ),
    }
}

fn filesystem_format_command(fs_type: &str, device: Option<&str>) -> ExecutionCommand {
    let Some(device) = device else {
        return command_with_readiness(
            ["mkfs", "-t", fs_type, "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "create the filesystem signature after selecting the reviewed block device",
        );
    };

    match fs_type {
        "ext2" => command(
            ["mkfs.ext2", "-F", device],
            true,
            "create an ext2 filesystem",
        ),
        "ext3" => command(
            ["mkfs.ext3", "-F", device],
            true,
            "create an ext3 filesystem",
        ),
        "ext4" => command(
            ["mkfs.ext4", "-F", device],
            true,
            "create an ext4 filesystem",
        ),
        "xfs" => command(["mkfs.xfs", "-f", device], true, "create an XFS filesystem"),
        "btrfs" => command(
            ["mkfs.btrfs", "-f", device],
            true,
            "create a Btrfs filesystem",
        ),
        "bcachefs" => command(
            ["bcachefs", "format", "--force", device],
            true,
            "create a bcachefs filesystem",
        ),
        "f2fs" => command(
            ["mkfs.f2fs", "-f", device],
            true,
            "create an F2FS filesystem",
        ),
        "exfat" => command(["mkfs.exfat", device], true, "create an exFAT filesystem"),
        "fat" | "vfat" => command(["mkfs.vfat", "-I", device], true, "create a FAT filesystem"),
        "ntfs" => command(
            ["mkfs.ntfs", "-F", device],
            true,
            "create an NTFS filesystem",
        ),
        "unknown" | "<filesystem-type>" => command_with_readiness(
            ["mkfs", "-t", "<filesystem-type>", device],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem type"],
            "create the filesystem signature after selecting the filesystem type",
        ),
        _ => command_vec_with_readiness(
            vec!["mkfs", "-t", fs_type, device],
            true,
            CommandReadiness::ManualOnly,
            ["review filesystem-specific mkfs options"],
            "review filesystem-specific mkfs flags before formatting this type",
        ),
    }
}

fn filesystem_shrink_commands(
    fs_type: &str,
    target: &str,
    device: Option<&str>,
    desired_size: Option<&str>,
) -> Vec<ExecutionCommand> {
    let mut commands = vec![command(
        ["disk-nix", "inspect", target],
        false,
        "inspect filesystem usage, mount state, and consumers before shrinking",
    )];
    match fs_type {
        "btrfs" => {
            commands.push(command(
                ["btrfs", "filesystem", "usage", "-b", target],
                false,
                "inspect Btrfs allocation slack before shrinking",
            ));
            commands.push(btrfs_filesystem_shrink_command(target, desired_size));
        }
        "ext2" | "ext3" | "ext4" => {
            commands.push(command(
                [
                    "findmnt",
                    "--noheadings",
                    "--output",
                    "SOURCE,FSTYPE,SIZE,USED,AVAIL",
                    "--target",
                    target,
                ],
                false,
                "resolve the ext filesystem source device and capacity before offline shrink",
            ));
            commands.push(command(
                ["umount", target],
                true,
                "unmount the ext filesystem before fsck and shrink",
            ));
            commands.push(ext_filesystem_check_command(target, device));
            commands.push(ext_filesystem_shrink_command(target, device, desired_size));
        }
        "xfs" => {
            commands.push(command_with_readiness(
                ["<migrate-to-smaller-filesystem>", target],
                true,
                CommandReadiness::ManualOnly,
                ["replacement filesystem", "migration plan"],
                "XFS cannot shrink in place; create a smaller filesystem and migrate data",
            ));
        }
        _ => commands.push(command_with_readiness(
            ["<filesystem-shrink-tool>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem shrink tool", "filesystem source device"],
            "shrink with the filesystem-specific offline or migration workflow",
        )),
    }
    commands
}

fn btrfs_filesystem_shrink_command(target: &str, desired_size: Option<&str>) -> ExecutionCommand {
    match desired_size {
        Some(size) => command_vec(
            vec!["btrfs", "filesystem", "resize", size, target],
            true,
            "shrink the Btrfs filesystem to the reviewed size",
        ),
        None => command_with_readiness(
            ["btrfs", "filesystem", "resize", "<size>", target],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired filesystem size"],
            "shrink the Btrfs filesystem after selecting the target size",
        ),
    }
}

fn ext_filesystem_device<'a>(target: &'a str, device: Option<&'a str>) -> Option<&'a str> {
    device.or_else(|| target.starts_with("/dev/").then_some(target))
}

fn filesystem_source_device<'a>(target: &'a str, device: Option<&'a str>) -> Option<&'a str> {
    device.or_else(|| target.starts_with("/dev/").then_some(target))
}

fn f2fs_filesystem_grow_command(
    target: &str,
    device: Option<&str>,
    desired_size: Option<&str>,
) -> ExecutionCommand {
    match (filesystem_source_device(target, device), desired_size) {
        (Some(source), Some(size)) => command(
            ["resize.f2fs", "-t", size, source],
            true,
            "grow an F2FS filesystem to the reviewed target sector count",
        ),
        (Some(source), None) => command(
            ["resize.f2fs", source],
            true,
            "grow an F2FS filesystem to the visible backing device size",
        ),
        (None, Some(size)) => command_with_readiness(
            ["resize.f2fs", "-t", size, "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "grow the F2FS filesystem after resolving the source device",
        ),
        (None, None) => command_with_readiness(
            ["resize.f2fs", "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "grow the F2FS filesystem after resolving the source device",
        ),
    }
}

fn filesystem_check_commands(
    fs_type: &str,
    target: &str,
    device: Option<&str>,
) -> Vec<ExecutionCommand> {
    vec![
        command(
            ["disk-nix", "inspect", target],
            false,
            "inspect filesystem identity, mount state, and consumers before check",
        ),
        filesystem_check_command(fs_type, target, device),
    ]
}

fn filesystem_repair_commands(
    fs_type: &str,
    target: &str,
    device: Option<&str>,
) -> Vec<ExecutionCommand> {
    vec![
        command(
            ["disk-nix", "inspect", target],
            false,
            "inspect filesystem identity, mount state, and consumers before repair",
        ),
        command(
            ["findmnt", "--json", "--target", target],
            false,
            "confirm mount state before offline repair",
        ),
        filesystem_repair_command(fs_type, target, device),
    ]
}

fn filesystem_check_command(fs_type: &str, target: &str, device: Option<&str>) -> ExecutionCommand {
    let source = filesystem_source_device(target, device);
    match (fs_type, source) {
        ("ext2" | "ext3" | "ext4", Some(source)) => command(
            ["e2fsck", "-n", source],
            false,
            "run a read-only ext filesystem consistency check",
        ),
        ("ext2" | "ext3" | "ext4", None) => command_with_readiness(
            ["e2fsck", "-n", "<filesystem-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "run ext filesystem check after resolving the source device",
        ),
        ("xfs", Some(source)) => command(
            ["xfs_repair", "-n", source],
            false,
            "run a no-modify XFS metadata check",
        ),
        ("xfs", None) => command_with_readiness(
            ["xfs_repair", "-n", "<filesystem-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "run XFS check after resolving the source device",
        ),
        ("btrfs", Some(source)) => command(
            ["btrfs", "check", "--readonly", source],
            false,
            "run a read-only Btrfs metadata check",
        ),
        ("btrfs", None) => command_with_readiness(
            ["btrfs", "check", "--readonly", "<filesystem-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "run Btrfs check after resolving the source device",
        ),
        ("fat" | "fat12" | "fat16" | "fat32" | "msdos" | "vfat", Some(source)) => command(
            ["fsck.fat", "-n", source],
            false,
            "run a no-write FAT filesystem consistency check",
        ),
        ("fat" | "fat12" | "fat16" | "fat32" | "msdos" | "vfat", None) => command_with_readiness(
            ["fsck.fat", "-n", "<filesystem-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "run FAT filesystem check after resolving the source device",
        ),
        ("exfat", Some(source)) => command(
            ["fsck.exfat", "-n", source],
            false,
            "run a no-write exFAT filesystem consistency check",
        ),
        ("exfat", None) => command_with_readiness(
            ["fsck.exfat", "-n", "<filesystem-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "run exFAT filesystem check after resolving the source device",
        ),
        ("ntfs" | "ntfs3", Some(source)) => command(
            ["ntfsfix", "--no-action", source],
            false,
            "run a no-action NTFS consistency probe",
        ),
        ("ntfs" | "ntfs3", None) => command_with_readiness(
            ["ntfsfix", "--no-action", "<filesystem-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "run NTFS consistency probe after resolving the source device",
        ),
        ("f2fs", Some(source)) => command(
            ["fsck.f2fs", "--dry-run", source],
            false,
            "run a dry-run F2FS filesystem consistency check",
        ),
        ("f2fs", None) => command_with_readiness(
            ["fsck.f2fs", "--dry-run", "<filesystem-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "run F2FS filesystem check after resolving the source device",
        ),
        ("bcachefs", Some(source)) => command(
            ["bcachefs", "fsck", "-n", source],
            false,
            "run a no-repair bcachefs filesystem consistency check",
        ),
        ("bcachefs", None) => command_with_readiness(
            ["bcachefs", "fsck", "-n", "<filesystem-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "run bcachefs filesystem check after resolving the source device",
        ),
        (_, Some(source)) => command_vec_with_readiness(
            vec!["<filesystem-check-tool>", source],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem check tool"],
            "run the filesystem-specific read-only check command",
        ),
        (_, None) => command_with_readiness(
            ["<filesystem-check-tool>", "<filesystem-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem check tool", "filesystem source device"],
            "run the filesystem-specific read-only check command",
        ),
    }
}

fn filesystem_repair_command(
    fs_type: &str,
    target: &str,
    device: Option<&str>,
) -> ExecutionCommand {
    let source = filesystem_source_device(target, device);
    match (fs_type, source) {
        ("ext2" | "ext3" | "ext4", Some(source)) => command(
            ["e2fsck", "-f", "-y", source],
            true,
            "repair ext filesystem metadata after offline review",
        ),
        ("ext2" | "ext3" | "ext4", None) => command_with_readiness(
            ["e2fsck", "-f", "-y", "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "repair ext filesystem after resolving the source device",
        ),
        ("xfs", Some(source)) => command(
            ["xfs_repair", source],
            true,
            "repair XFS metadata after offline review",
        ),
        ("xfs", None) => command_with_readiness(
            ["xfs_repair", "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "repair XFS after resolving the source device",
        ),
        ("btrfs", Some(source)) => command(
            ["btrfs", "check", "--repair", source],
            true,
            "repair Btrfs metadata only after explicit offline review",
        ),
        ("btrfs", None) => command_with_readiness(
            ["btrfs", "check", "--repair", "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "repair Btrfs after resolving the source device",
        ),
        ("fat" | "fat12" | "fat16" | "fat32" | "msdos" | "vfat", Some(source)) => command(
            ["fsck.fat", "-a", source],
            true,
            "repair FAT filesystem metadata after offline review",
        ),
        ("fat" | "fat12" | "fat16" | "fat32" | "msdos" | "vfat", None) => command_with_readiness(
            ["fsck.fat", "-a", "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "repair FAT filesystem after resolving the source device",
        ),
        ("exfat", Some(source)) => command(
            ["fsck.exfat", "-p", source],
            true,
            "repair exFAT filesystem metadata after offline review",
        ),
        ("exfat", None) => command_with_readiness(
            ["fsck.exfat", "-p", "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "repair exFAT filesystem after resolving the source device",
        ),
        ("ntfs" | "ntfs3", Some(source)) => command(
            ["ntfsfix", source],
            true,
            "apply limited NTFS fixes and schedule Windows consistency check after offline review",
        ),
        ("ntfs" | "ntfs3", None) => command_with_readiness(
            ["ntfsfix", "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "run limited NTFS repair after resolving the source device",
        ),
        ("f2fs", Some(source)) => command(
            ["fsck.f2fs", "-f", "-y", source],
            true,
            "repair F2FS filesystem metadata after offline review",
        ),
        ("f2fs", None) => command_with_readiness(
            ["fsck.f2fs", "-f", "-y", "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "repair F2FS filesystem after resolving the source device",
        ),
        ("bcachefs", Some(source)) => command(
            ["bcachefs", "fsck", "-y", source],
            true,
            "repair bcachefs metadata after offline review",
        ),
        ("bcachefs", None) => command_with_readiness(
            ["bcachefs", "fsck", "-y", "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "repair bcachefs after resolving the source device",
        ),
        (_, Some(source)) => command_vec_with_readiness(
            vec!["<filesystem-repair-tool>", source],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem repair tool"],
            "run the filesystem-specific repair command",
        ),
        (_, None) => command_with_readiness(
            ["<filesystem-repair-tool>", "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem repair tool", "filesystem source device"],
            "run the filesystem-specific repair command after resolving the source device",
        ),
    }
}

fn ext_filesystem_grow_command(
    target: &str,
    device: Option<&str>,
    desired_size: Option<&str>,
) -> ExecutionCommand {
    match (ext_filesystem_device(target, device), desired_size) {
        (Some(device), Some(size)) => command_vec(
            vec!["resize2fs", device, size],
            true,
            "grow an ext filesystem to the desired size after the backing block device has grown",
        ),
        (Some(device), None) => command(
            ["resize2fs", device],
            true,
            "grow an ext filesystem after the backing block device has grown",
        ),
        (None, Some(size)) => command_vec_with_readiness(
            vec!["resize2fs", "<filesystem-device>", size],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "grow the ext filesystem after resolving the source block device",
        ),
        (None, None) => command_with_readiness(
            ["resize2fs", "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "grow the ext filesystem after resolving the source block device",
        ),
    }
}

fn ext_filesystem_check_command(target: &str, device: Option<&str>) -> ExecutionCommand {
    if let Some(device) = ext_filesystem_device(target, device) {
        command(
            ["e2fsck", "-f", device],
            true,
            "run a forced ext filesystem check before shrinking",
        )
    } else {
        command_with_readiness(
            ["e2fsck", "-f", "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "run a forced ext filesystem check after resolving the source device",
        )
    }
}

fn ext_filesystem_shrink_command(
    target: &str,
    device: Option<&str>,
    desired_size: Option<&str>,
) -> ExecutionCommand {
    match (ext_filesystem_device(target, device), desired_size) {
        (Some(device), Some(size)) => command(
            ["resize2fs", device, size],
            true,
            "shrink the ext filesystem to the reviewed size",
        ),
        (Some(device), None) => command_with_readiness(
            ["resize2fs", device, "<size>"],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired filesystem size"],
            "shrink the ext filesystem after selecting the target size",
        ),
        (None, Some(size)) => command_vec_with_readiness(
            vec!["resize2fs", "<filesystem-device>", size],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "shrink the ext filesystem after resolving the source device",
        ),
        (None, None) => command_with_readiness(
            ["resize2fs", "<filesystem-device>", "<size>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device", "desired filesystem size"],
            "shrink the ext filesystem after resolving source device and target size",
        ),
    }
}

fn action_id_suffix<'a>(action_id: &'a str, operation: &str) -> Option<&'a str> {
    let marker = format!(":{operation}:");
    let (_, suffix) = action_id.split_once(&marker)?;
    (!suffix.is_empty()).then_some(suffix)
}
