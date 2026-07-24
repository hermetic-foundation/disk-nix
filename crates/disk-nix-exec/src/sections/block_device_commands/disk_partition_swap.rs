fn disk_create_label_command(target: Option<&str>, label: &str) -> ExecutionCommand {
    match target {
        Some(target) => command_vec(
            vec!["parted", "-s", target, "mklabel", label],
            true,
            "create the reviewed disk partition table label",
        ),
        None => command_vec_with_readiness(
            vec!["parted", "-s", "<disk>", "mklabel", label],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["disk path"],
            "create the reviewed disk partition table label after selecting the disk",
        ),
    }
}

fn disk_wipe_signatures_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["wipefs", "--all", "--force", target],
            true,
            "clear existing signatures before raw whole-disk ZFS pool creation",
        ),
        None => command_with_readiness(
            ["wipefs", "--all", "--force", "<disk>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["disk path"],
            "clear existing signatures before raw whole-disk ZFS pool creation after selecting the disk",
        ),
    }
}

fn disk_nix_inspect_command(
    target: Option<&str>,
    placeholder: &'static str,
    missing_input: &'static str,
    description: &'static str,
) -> ExecutionCommand {
    match target {
        Some(target) => command(["disk-nix", "inspect", target], false, description),
        None => command_with_readiness(
            ["disk-nix", "inspect", placeholder],
            false,
            CommandReadiness::NeedsDomainImplementation,
            [missing_input],
            description,
        ),
    }
}

fn partition_target_path(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .target
        .as_deref()
        .filter(|target| target.starts_with('/'))
        .or_else(|| {
            action
                .context
                .name
                .as_deref()
                .filter(|name| name.starts_with('/'))
        })
}

fn disk_target_path(action: &PlannedAction) -> Option<&str> {
    partition_target_path(action)
}

fn partition_rescan_disk(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .device
        .as_deref()
        .or_else(|| disk_target_path(action))
}

fn partition_create_command(
    disk: Option<&str>,
    partition_type: Option<&str>,
    start: Option<&str>,
    end: Option<&str>,
) -> ExecutionCommand {
    let argv = vec![
        "parted",
        "-s",
        disk.unwrap_or("<disk>"),
        "mkpart",
        partition_type.unwrap_or("<partition-type>"),
        start.unwrap_or("<start>"),
        end.unwrap_or("<end>"),
    ];
    let missing = missing_partition_create_inputs(disk, partition_type, start, end);
    if missing.is_empty() {
        command_vec(argv, true, "create a partition in the reviewed free region")
    } else {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "create a partition after resolving the disk, type, and offsets",
        )
    }
}

fn pool_create_devices(device: Option<&str>, devices: &[String]) -> Vec<String> {
    if devices.is_empty() {
        device.into_iter().map(ToString::to_string).collect()
    } else {
        devices.to_vec()
    }
}

fn zfs_pool_create_argv(
    target: &str,
    devices: &[String],
    property_assignments: &[String],
) -> Vec<String> {
    let mut argv = vec!["zpool".to_string(), "create".to_string()];
    for assignment in property_assignments {
        let option = if zfs_pool_assignment_is_root_dataset_property(assignment) {
            "-O"
        } else {
            "-o"
        };
        argv.extend([option.to_string(), assignment.clone()]);
    }
    argv.push(target.to_string());
    argv.extend(devices.iter().cloned());
    argv
}

fn zfs_pool_assignment_is_root_dataset_property(assignment_or_property: &str) -> bool {
    let property = assignment_or_property
        .split_once('=')
        .map_or(assignment_or_property, |(property, _)| property);
    zfs_property_is_root_dataset_property(property)
}

fn zfs_property_is_root_dataset_property(property: &str) -> bool {
    if property.contains(':') {
        return true;
    }
    matches!(
        property,
        "aclinherit"
            | "aclmode"
            | "acltype"
            | "atime"
            | "canmount"
            | "casesensitivity"
            | "checksum"
            | "compression"
            | "copies"
            | "devices"
            | "dnodesize"
            | "encryption"
            | "exec"
            | "filesystem_count"
            | "filesystem_limit"
            | "jailed"
            | "keyformat"
            | "keylocation"
            | "logbias"
            | "mountpoint"
            | "nbmand"
            | "normalization"
            | "overlay"
            | "primarycache"
            | "quota"
            | "readonly"
            | "recordsize"
            | "redundant_metadata"
            | "refquota"
            | "refreservation"
            | "relatime"
            | "reservation"
            | "secondarycache"
            | "setuid"
            | "sharesmb"
            | "sharenfs"
            | "snapdir"
            | "snapshot_count"
            | "snapshot_limit"
            | "special_small_blocks"
            | "sync"
            | "utf8only"
            | "version"
            | "volblocksize"
            | "volmode"
            | "volsize"
            | "vscan"
            | "xattr"
            | "zoned"
    )
}

fn zfs_pool_create_command(
    target: &str,
    devices: &[String],
    property_assignments: &[String],
) -> ExecutionCommand {
    if devices.is_empty() {
        let mut argv = zfs_pool_create_argv(target, devices, property_assignments);
        argv.push("<vdev-device>".to_string());
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["vdev device or topology"],
            "create a ZFS pool after selecting the vdev topology",
        )
    } else {
        let argv = zfs_pool_create_argv(target, devices, property_assignments);
        command_vec(
            argv,
            true,
            "create a ZFS pool on the reviewed vdev device set with declared pool properties",
        )
    }
}

fn zfs_pool_import_command(target: &str, read_only: bool) -> ExecutionCommand {
    let mut argv = vec!["zpool".to_string(), "import".to_string()];
    if read_only {
        argv.extend(["-o".to_string(), "readonly=on".to_string()]);
    }
    argv.push(target.to_string());
    command_vec(
        argv,
        true,
        "import the reviewed ZFS pool without recreating it",
    )
}

fn zfs_pool_preflight_commands(devices: &[String]) -> Vec<ExecutionCommand> {
    let inspect_targets: Vec<&str> = devices
        .iter()
        .map(String::as_str)
        .filter(|device| device.starts_with('/'))
        .collect();
    if inspect_targets.is_empty() {
        vec![command_with_readiness(
            ["disk-nix", "inspect", "<vdev-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["vdev device or topology"],
            "inspect vdev device identity before creating the ZFS pool",
        )]
    } else {
        inspect_targets
            .into_iter()
            .map(|device| {
                command_vec(
                    vec!["disk-nix", "inspect", device],
                    false,
                    "inspect vdev device identity before creating the ZFS pool",
                )
            })
            .collect()
    }
}

fn partition_grow_command(
    disk: Option<&str>,
    partition_number: Option<&str>,
    desired_end: Option<&str>,
) -> ExecutionCommand {
    match (disk, partition_number, desired_end) {
        (Some(disk), Some(number), Some(end)) => command_vec(
            vec!["parted", "-s", disk, "resizepart", number, end],
            true,
            "grow the partition to the reviewed end offset after backing capacity is visible",
        ),
        (Some(disk), Some(number), None) => command_vec(
            vec!["growpart", disk, number],
            true,
            "grow the partition to the maximum visible backing capacity",
        ),
        (disk, partition_number, Some(end)) => command_vec_with_readiness(
            vec![
                "parted",
                "-s",
                disk.unwrap_or("<disk>"),
                "resizepart",
                partition_number.unwrap_or("<partition-number>"),
                end,
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_partition_resize_inputs(disk, partition_number),
            "grow a partition to the desired end offset or size after backing capacity is visible",
        ),
        (disk, partition_number, None) => command_vec_with_readiness(
            vec![
                "growpart",
                disk.unwrap_or("<disk>"),
                partition_number.unwrap_or("<partition-number>"),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_partition_resize_inputs(disk, partition_number),
            "grow a partition after backing capacity is visible",
        ),
    }
}

fn missing_partition_resize_inputs(
    disk: Option<&str>,
    partition_number: Option<&str>,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if disk.is_none() {
        missing.push("disk path");
    }
    if partition_number.is_none() {
        missing.push("partition number");
    }
    missing
}

fn missing_partition_create_inputs(
    disk: Option<&str>,
    partition_type: Option<&str>,
    start: Option<&str>,
    end: Option<&str>,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if disk.is_none() {
        missing.push("disk path");
    }
    if partition_type.is_none() {
        missing.push("partition type");
    }
    if start.is_none() {
        missing.push("partition start offset");
    }
    if end.is_none() {
        missing.push("partition end offset");
    }
    missing
}

fn partition_probe_command(disk: Option<&str>) -> ExecutionCommand {
    match disk {
        Some(disk) => command(
            ["partprobe", disk],
            true,
            "ask the kernel to reread the changed partition table",
        ),
        None => command_with_readiness(
            ["partprobe", "<disk>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["disk path"],
            "ask the kernel to reread the changed partition table after selecting the disk",
        ),
    }
}

fn partition_table_reread_command(disk: Option<&str>) -> ExecutionCommand {
    match disk {
        Some(disk) => command_vec(
            vec![
                "sh",
                "-c",
                "blockdev --rereadpt \"$1\" || true",
                "disk-nix-rereadpt",
                disk,
            ],
            true,
            "best-effort partition table reread before udev and partition-node verification",
        ),
        None => command_with_readiness(
            ["blockdev", "--rereadpt", "<disk>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["disk path"],
            "force a partition table reread when supported by the block device",
        ),
    }
}

fn partition_udev_settle_command() -> ExecutionCommand {
    command(
        ["udevadm", "settle"],
        false,
        "wait for udev to publish partition device nodes after the reread",
    )
}

fn disk_parted_machine_list_command(
    disk: Option<&str>,
    description: &'static str,
) -> ExecutionCommand {
    match disk {
        Some(disk) => command(["parted", "-lm", disk], false, description),
        None => command_with_readiness(
            ["parted", "-lm", "<disk>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["disk path"],
            description,
        ),
    }
}

fn swap_target_path(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .target
        .as_deref()
        .filter(|target| target.starts_with('/'))
        .or_else(|| {
            action
                .context
                .device
                .as_deref()
                .filter(|device| device.starts_with('/'))
        })
}

fn swap_command(
    command_name: &'static str,
    target: Option<&str>,
    note: &'static str,
) -> ExecutionCommand {
    match target {
        Some(target) => command([command_name, target], true, note),
        None => command_with_readiness(
            [command_name, "<swap>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["swap target path"],
            note,
        ),
    }
}

fn swapoff_best_effort_command(target: Option<&str>, note: &'static str) -> ExecutionCommand {
    match target {
        Some(target) => command_vec(
            vec![
                "sh".to_string(),
                "-c".to_string(),
                format!("swapoff {} 2>/dev/null || true", shell_quote(target)),
            ],
            true,
            note,
        ),
        None => command_with_readiness(
            ["swapoff", "<swap>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["swap target path"],
            note,
        ),
    }
}

fn swap_blkid_command(target: Option<&str>, note: &'static str) -> ExecutionCommand {
    match target {
        Some(target) => command(["blkid", target], false, note),
        None => command_with_readiness(
            ["blkid", "<swap>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["swap target path"],
            note,
        ),
    }
}

fn swap_wipefs_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["wipefs", "--all", target],
            true,
            "remove the reviewed swap signature metadata",
        ),
        None => command_with_readiness(
            ["wipefs", "--all", "<swap>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["swap target path"],
            "remove the swap signature after resolving the target",
        ),
    }
}

fn swap_inspect_command(target: Option<&str>, note: &'static str) -> ExecutionCommand {
    disk_nix_inspect_command(target, "<swap>", "swap target path", note)
}

fn swap_inspect_json_command(target: Option<&str>, note: &'static str) -> ExecutionCommand {
    match target {
        Some(target) => command(["disk-nix", "inspect", target, "--json"], false, note),
        None => command_with_readiness(
            ["disk-nix", "inspect", "<swap>", "--json"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["swap target path"],
            note,
        ),
    }
}

fn zram_rescan_commands(note: &'static str) -> Vec<ExecutionCommand> {
    vec![
        command(
            [
                "zramctl",
                "--bytes",
                "--raw",
                "--noheadings",
                "--output-all",
            ],
            false,
            note,
        ),
        command(
            ["swapon", "--show", "--bytes", "--raw"],
            false,
            "refresh active swap view for zram devices",
        ),
        command(
            ["disk-nix", "zram"],
            false,
            "inspect modeled zram swap devices after refresh",
        ),
    ]
}

fn swap_resize_command(target: Option<&str>, desired_size: Option<&str>) -> ExecutionCommand {
    let Some(target) = target else {
        return command_vec_with_readiness(
            vec![
                "<resize-swap-backing-storage>".to_string(),
                "<swap>".to_string(),
                desired_size.unwrap_or("<size>").to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_swap_resize_inputs(desired_size),
            "resize the swap backing device or file after selecting the target",
        );
    };

    if !target.starts_with("/dev/") {
        return match desired_size {
            Some(size) => command(
                ["fallocate", "--length", size, target],
                true,
                "resize the swap file to the desired length before recreating the signature",
            ),
            None => command_with_readiness(
                ["fallocate", "--length", "<size>", target],
                true,
                CommandReadiness::NeedsDesiredSize,
                ["desired swap file size"],
                "resize the swap file after selecting the desired size",
            ),
        };
    }

    match desired_size {
        Some(size) => command_vec_with_readiness(
            vec!["<resize-swap-backing-storage>", target, size],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["backing storage domain"],
            "resize the swap backing device or file before recreating the swap signature",
        ),
        None => command_vec_with_readiness(
            vec!["<resize-swap-backing-storage>", target, "<size>"],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired swap size", "backing storage domain"],
            "resize the swap backing device or file before recreating the swap signature",
        ),
    }
}

fn missing_swap_resize_inputs(desired_size: Option<&str>) -> Vec<&'static str> {
    let mut missing = vec!["swap target path"];
    if desired_size.is_none() {
        missing.push("desired swap size");
    }
    missing.push("backing storage domain");
    missing
}
fn luks_backing_inspect_command(device: Option<&str>, note: &str) -> ExecutionCommand {
    match device {
        Some(device) => command(["disk-nix", "inspect", device], false, note),
        None => command_with_readiness(
            ["disk-nix", "inspect", "<device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["LUKS backing device"],
            note,
        ),
    }
}

fn luks_is_luks_command(device: Option<&str>) -> ExecutionCommand {
    match device {
        Some(device) => command(
            ["cryptsetup", "isLuks", device],
            false,
            "verify the backing device has a LUKS header",
        ),
        None => command_with_readiness(
            ["cryptsetup", "isLuks", "<device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["LUKS backing device"],
            "verify the backing device has a LUKS header after selecting it",
        ),
    }
}

fn luks_format_command(device: Option<&str>) -> ExecutionCommand {
    match device {
        Some(device) => command(
            ["cryptsetup", "luksFormat", device],
            true,
            "create a LUKS container on the target device",
        ),
        None => command_with_readiness(
            ["cryptsetup", "luksFormat", "<device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["LUKS backing device"],
            "create a LUKS container after selecting the backing device",
        ),
    }
}

fn luks_open_command(device: Option<&str>, mapper: &str, note: &str) -> ExecutionCommand {
    match device {
        Some(device) => command_vec(vec!["cryptsetup", "open", device, mapper], true, note),
        None => command_vec_with_readiness(
            vec!["cryptsetup", "open", "<device>", mapper],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["LUKS backing device"],
            note,
        ),
    }
}

fn luks_keyslot_device(action: &PlannedAction) -> Option<&str> {
    action.context.device.as_deref().or(action
        .context
        .target
        .as_deref()
        .filter(|target| target.starts_with('/')))
}

fn luks_keyslot_id(action: &PlannedAction) -> Option<&str> {
    action.context.key_slot.as_deref().or_else(|| {
        action
            .context
            .name
            .as_deref()
            .and_then(|name| name.rsplit_once(':').map(|(_, slot)| slot).or(Some(name)))
            .filter(|slot| slot.chars().all(|character| character.is_ascii_digit()))
    })
}

fn luks_new_key_file(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .new_key_file
        .as_deref()
        .or(action.context.key_file.as_deref())
}

fn luks_token_device(action: &PlannedAction) -> Option<&str> {
    action.context.device.as_deref().or(action
        .context
        .target
        .as_deref()
        .filter(|target| target.starts_with('/')))
}

fn luks_token_id(action: &PlannedAction) -> Option<&str> {
    action.context.token_id.as_deref().or_else(|| {
        action
            .context
            .name
            .as_deref()
            .and_then(|name| name.rsplit_once(':').map(|(_, token)| token).or(Some(name)))
            .filter(|token| token.chars().all(|character| character.is_ascii_digit()))
    })
}

fn luks_token_file(action: &PlannedAction) -> Option<&str> {
    action.context.token_file.as_deref()
}

fn luks_dump_command(device: Option<&str>, note: &'static str) -> ExecutionCommand {
    match device {
        Some(device) => command(["cryptsetup", "luksDump", device], false, note),
        None => command_with_readiness(
            ["cryptsetup", "luksDump", "<luks-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["LUKS backing device"],
            note,
        ),
    }
}

fn luks_add_key_command(
    device: Option<&str>,
    key_slot: Option<&str>,
    new_key_file: Option<&str>,
) -> ExecutionCommand {
    let mut argv = vec!["cryptsetup".to_string(), "luksAddKey".to_string()];
    if let Some(key_slot) = key_slot {
        argv.extend(["--key-slot".to_string(), key_slot.to_string()]);
    }
    argv.push(device.unwrap_or("<luks-device>").to_string());
    argv.push(new_key_file.unwrap_or("<new-key-file>").to_string());

    let missing = missing_luks_add_key_inputs(device, new_key_file);
    if missing.is_empty() {
        command_vec(argv, true, "add reviewed key material to the LUKS header")
    } else {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "add LUKS key material after selecting the device and new key file",
        )
    }
}

fn luks_kill_slot_command(device: Option<&str>, key_slot: Option<&str>) -> ExecutionCommand {
    match (device, key_slot) {
        (Some(device), Some(key_slot)) => command(
            ["cryptsetup", "luksKillSlot", device, key_slot],
            true,
            "remove the reviewed LUKS keyslot after alternate unlock paths are verified",
        ),
        (device, key_slot) => command_vec_with_readiness(
            vec![
                "cryptsetup".to_string(),
                "luksKillSlot".to_string(),
                device.unwrap_or("<luks-device>").to_string(),
                key_slot.unwrap_or("<key-slot>").to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_luks_keyslot_inputs(device, key_slot),
            "remove LUKS keyslot after selecting the device and slot number",
        ),
    }
}

fn luks_change_key_command(
    device: Option<&str>,
    key_slot: Option<&str>,
    old_key_file: Option<&str>,
    new_key_file: Option<&str>,
) -> ExecutionCommand {
    let mut argv = vec!["cryptsetup".to_string(), "luksChangeKey".to_string()];
    if let Some(key_slot) = key_slot {
        argv.extend(["--key-slot".to_string(), key_slot.to_string()]);
    }
    if let Some(old_key_file) = old_key_file {
        argv.extend(["--key-file".to_string(), old_key_file.to_string()]);
    }
    argv.push(device.unwrap_or("<luks-device>").to_string());
    argv.push(new_key_file.unwrap_or("<new-key-file>").to_string());

    let missing = missing_luks_add_key_inputs(device, new_key_file);
    if missing.is_empty() {
        command_vec(
            argv,
            true,
            "change LUKS key material for the reviewed keyslot",
        )
    } else {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "change LUKS key material after selecting the device and replacement key file",
        )
    }
}

fn luks_keyslot_property_command(action: &PlannedAction, property: &str) -> ExecutionCommand {
    match normalize_property_name(property).as_str() {
        "keyfile"
        | "key-file"
        | "luks-keyfile"
        | "luks-key-file"
        | "cryptsetup-keyfile"
        | "cryptsetup-key-file" => luks_change_key_command(
            luks_keyslot_device(action),
            luks_keyslot_id(action),
            action.context.key_file.as_deref(),
            action.context.property_value.as_deref(),
        ),
        "priority" | "luks-keyslot-priority" | "cryptsetup-luks-keyslot-priority" => {
            luks_keyslot_priority_command(
                luks_keyslot_device(action),
                luks_keyslot_id(action),
                action.context.property_value.as_deref(),
            )
        }
        _ => command_vec_with_readiness(
            vec![
                "cryptsetup".to_string(),
                "config".to_string(),
                "<luks-device>".to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported LUKS keyslot property"],
            "change LUKS keyslot metadata after selecting a supported property",
        ),
    }
}

fn luks_keyslot_priority_command(
    device: Option<&str>,
    key_slot: Option<&str>,
    priority: Option<&str>,
) -> ExecutionCommand {
    let mut argv = vec![
        "cryptsetup".to_string(),
        "config".to_string(),
        device.unwrap_or("<luks-device>").to_string(),
        "--key-slot".to_string(),
        key_slot.unwrap_or("<key-slot>").to_string(),
        "--priority".to_string(),
        priority.unwrap_or("<priority>").to_string(),
    ];
    let normalized_priority = priority.map(normalize_property_name);
    let valid_priority = normalized_priority
        .as_deref()
        .is_some_and(|value| matches!(value, "prefer" | "normal" | "ignore"));
    if let Some(normalized_priority) = normalized_priority {
        if valid_priority {
            if let Some(last) = argv.last_mut() {
                *last = normalized_priority;
            }
        }
    }

    let missing = missing_luks_keyslot_priority_inputs(device, key_slot, priority, valid_priority);
    if missing.is_empty() {
        command_vec(
            argv,
            true,
            "change LUKS keyslot priority metadata after header backup",
        )
    } else {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "change LUKS keyslot priority after selecting device, slot, and priority",
        )
    }
}

fn luks_token_import_command(
    device: Option<&str>,
    token_id: Option<&str>,
    token_file: Option<&str>,
) -> ExecutionCommand {
    let mut argv = vec![
        "cryptsetup".to_string(),
        "token".to_string(),
        "import".to_string(),
    ];
    if let Some(token_id) = token_id {
        argv.extend(["--token-id".to_string(), token_id.to_string()]);
    }
    argv.extend([
        "--json-file".to_string(),
        token_file.unwrap_or("<token-json-file>").to_string(),
        device.unwrap_or("<luks-device>").to_string(),
    ]);

    let missing = missing_luks_token_import_inputs(device, token_file);
    if missing.is_empty() {
        command_vec(argv, true, "import reviewed LUKS token metadata")
    } else {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "import LUKS token after selecting the device and token JSON file",
        )
    }
}
