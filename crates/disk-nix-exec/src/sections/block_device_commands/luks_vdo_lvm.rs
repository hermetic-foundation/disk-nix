fn luks_token_remove_command(device: Option<&str>, token_id: Option<&str>) -> ExecutionCommand {
    match (device, token_id) {
        (Some(device), Some(token_id)) => command(
            [
                "cryptsetup",
                "token",
                "remove",
                "--token-id",
                token_id,
                device,
            ],
            true,
            "remove the reviewed LUKS token after alternate unlock paths are verified",
        ),
        (device, token_id) => command_vec_with_readiness(
            vec![
                "cryptsetup".to_string(),
                "token".to_string(),
                "remove".to_string(),
                "--token-id".to_string(),
                token_id.unwrap_or("<token-id>").to_string(),
                device.unwrap_or("<luks-device>").to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_luks_token_remove_inputs(device, token_id),
            "remove LUKS token after selecting the device and token id",
        ),
    }
}

fn missing_luks_add_key_inputs(
    device: Option<&str>,
    new_key_file: Option<&str>,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if device.is_none() {
        missing.push("LUKS backing device");
    }
    if new_key_file.is_none() {
        missing.push("new key file");
    }
    missing
}

fn missing_luks_keyslot_inputs(device: Option<&str>, key_slot: Option<&str>) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if device.is_none() {
        missing.push("LUKS backing device");
    }
    if key_slot.is_none() {
        missing.push("LUKS keyslot number");
    }
    missing
}

fn missing_luks_keyslot_priority_inputs(
    device: Option<&str>,
    key_slot: Option<&str>,
    priority: Option<&str>,
    valid_priority: bool,
) -> Vec<&'static str> {
    let mut missing = missing_luks_keyslot_inputs(device, key_slot);
    if priority.is_none() {
        missing.push("LUKS keyslot priority");
    } else if !valid_priority {
        missing.push("LUKS keyslot priority prefer, normal, or ignore");
    }
    missing
}

fn missing_luks_token_import_inputs(
    device: Option<&str>,
    token_file: Option<&str>,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if device.is_none() {
        missing.push("LUKS backing device");
    }
    if token_file.is_none() {
        missing.push("token JSON file");
    }
    missing
}

fn missing_luks_token_remove_inputs(
    device: Option<&str>,
    token_id: Option<&str>,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if device.is_none() {
        missing.push("LUKS backing device");
    }
    if token_id.is_none() {
        missing.push("LUKS token id");
    }
    missing
}

fn vdo_grow_logical_command(target: &str, desired_size: Option<&str>) -> ExecutionCommand {
    match desired_size {
        Some(size) => command_vec(
            vec![
                "vdo",
                "growLogical",
                "--name",
                target,
                "--vdoLogicalSize",
                size,
            ],
            true,
            "grow VDO logical size to the desired value",
        ),
        None => command_with_readiness(
            [
                "vdo",
                "growLogical",
                "--name",
                target,
                "--vdoLogicalSize",
                "<size>",
            ],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired VDO logical size"],
            "grow VDO logical size after selecting the desired size",
        ),
    }
}

fn vdo_growth_commands(
    target: &str,
    desired_size: Option<&str>,
    physical_size: Option<&str>,
) -> Vec<ExecutionCommand> {
    let mut commands = Vec::new();
    if let Some(size) = physical_size {
        commands.push(command(
            ["vdo", "growPhysical", "--name", target],
            true,
            &format!(
                "grow VDO physical capacity after backing storage has grown to reviewed size {size}"
            ),
        ));
    }
    if desired_size.is_some() {
        commands.push(vdo_grow_logical_command(target, desired_size));
    }
    if commands.is_empty() {
        commands.push(command_with_readiness(
            [
                "vdo",
                "growLogical",
                "--name",
                target,
                "--vdoLogicalSize",
                "<size>",
            ],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired VDO logical size or physicalSize intent"],
            "grow VDO after declaring desiredSize for logical growth or physicalSize for backing growth",
        ));
    }
    commands
}

fn vdo_create_command(
    target: &str,
    device: Option<&str>,
    desired_size: Option<&str>,
) -> ExecutionCommand {
    match (device, desired_size) {
        (Some(device), Some(size)) => command_vec(
            vec![
                "vdo",
                "create",
                "--name",
                target,
                "--device",
                device,
                "--vdoLogicalSize",
                size,
            ],
            true,
            "create a VDO volume on the reviewed backing device",
        ),
        (Some(device), None) => command_vec_with_readiness(
            vec![
                "vdo",
                "create",
                "--name",
                target,
                "--device",
                device,
                "--vdoLogicalSize",
                "<size>",
            ],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired VDO logical size"],
            "create a VDO volume after selecting the logical size",
        ),
        (None, Some(size)) => command_vec_with_readiness(
            vec![
                "vdo",
                "create",
                "--name",
                target,
                "--device",
                "<backing-device>",
                "--vdoLogicalSize",
                size,
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["backing device"],
            "create a VDO volume after selecting the backing device",
        ),
        (None, None) => command_with_readiness(
            [
                "vdo",
                "create",
                "--name",
                target,
                "--device",
                "<backing-device>",
                "--vdoLogicalSize",
                "<size>",
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["backing device", "desired VDO logical size"],
            "create a VDO volume after selecting backing device and logical size",
        ),
    }
}

fn vdo_backing_inspect_command(device: Option<&str>) -> ExecutionCommand {
    match device {
        Some(device) => command(
            ["disk-nix", "inspect", device],
            false,
            "inspect backing device before creating the VDO volume",
        ),
        None => command_with_readiness(
            ["disk-nix", "inspect", "<backing-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["backing device"],
            "inspect backing device before creating the VDO volume",
        ),
    }
}

fn thin_pool_create_command(target: &str, desired_size: Option<&str>) -> ExecutionCommand {
    let Some((volume_group, thin_pool)) = target.split_once('/') else {
        return command_vec_with_readiness(
            vec![
                "lvcreate".to_string(),
                "--type".to_string(),
                "thin-pool".to_string(),
                "--size".to_string(),
                desired_size.unwrap_or("<size>").to_string(),
                "--name".to_string(),
                "<thin-pool>".to_string(),
                "<volume-group>".to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_lvm_create_inputs(
                "target in volume-group/thin-pool form",
                "desired thin pool size",
                desired_size,
            ),
            "create an LVM thin pool after resolving volume group and pool name",
        );
    };

    match desired_size {
        Some(size) => command_vec(
            vec![
                "lvcreate".to_string(),
                "--type".to_string(),
                "thin-pool".to_string(),
                "--size".to_string(),
                size.to_string(),
                "--name".to_string(),
                thin_pool.to_string(),
                volume_group.to_string(),
            ],
            true,
            "create an LVM thin pool with the desired size",
        ),
        None => command_vec_with_readiness(
            vec![
                "lvcreate".to_string(),
                "--type".to_string(),
                "thin-pool".to_string(),
                "--size".to_string(),
                "<size>".to_string(),
                "--name".to_string(),
                thin_pool.to_string(),
                volume_group.to_string(),
            ],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired thin pool size"],
            "create an LVM thin pool after selecting the desired size",
        ),
    }
}

fn lvm_volume_target_path(target: Option<&str>) -> Option<&str> {
    target.filter(|target| {
        let Some((volume_group, volume)) = target.split_once('/') else {
            return false;
        };
        !volume_group.is_empty() && !volume.is_empty()
    })
}

fn lvm_lvs_report_command(
    target: Option<&str>,
    columns: Option<&str>,
    description: &'static str,
) -> ExecutionCommand {
    match (target, columns) {
        (Some(target), Some(columns)) => command(
            ["lvs", "--reportformat", "json", "-o", columns, target],
            false,
            description,
        ),
        (Some(target), None) => command(
            ["lvs", "--reportformat", "json", target],
            false,
            description,
        ),
        (None, Some(columns)) => command_with_readiness(
            [
                "lvs",
                "--reportformat",
                "json",
                "-o",
                columns,
                "<logical-volume>",
            ],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["target in volume-group/logical-volume form"],
            description,
        ),
        (None, None) => command_with_readiness(
            ["lvs", "--reportformat", "json", "<logical-volume>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["target in volume-group/logical-volume form"],
            description,
        ),
    }
}

fn lvm_logical_volume_extend_command(
    target: Option<&str>,
    desired_size: Option<&str>,
) -> ExecutionCommand {
    match (target, desired_size) {
        (Some(target), Some(size)) => command_vec(
            vec!["lvextend", "--resizefs", "--size", size, target],
            true,
            "grow the logical volume and filesystem to the desired size",
        ),
        (Some(target), None) => command_with_readiness(
            ["lvextend", "--resizefs", "--size", "+<size>", target],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired size delta"],
            "grow the logical volume and filesystem together",
        ),
        (None, desired_size) => command_vec_with_readiness(
            vec![
                "lvextend".to_string(),
                "--resizefs".to_string(),
                "--size".to_string(),
                desired_size.unwrap_or("+<size>").to_string(),
                "<logical-volume>".to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_lvm_resize_inputs(
                "target in volume-group/logical-volume form",
                "desired size delta",
                desired_size,
            ),
            "grow the logical volume and filesystem after resolving the target",
        ),
    }
}

fn thin_pool_extend_command(target: Option<&str>, desired_size: Option<&str>) -> ExecutionCommand {
    match (target, desired_size) {
        (Some(target), Some(size)) => command_vec(
            vec!["lvextend", "--size", size, target],
            true,
            "extend the LVM thin pool data volume to the desired size",
        ),
        (Some(target), None) => command_with_readiness(
            ["lvextend", "--size", "+<size>", target],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired thin pool size or size delta"],
            "extend the LVM thin pool after selecting the desired size",
        ),
        (None, desired_size) => command_vec_with_readiness(
            vec![
                "lvextend".to_string(),
                "--size".to_string(),
                desired_size.unwrap_or("+<size>").to_string(),
                "<thin-pool>".to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_lvm_resize_inputs(
                "target in volume-group/thin-pool form",
                "desired thin pool size or size delta",
                desired_size,
            ),
            "extend the LVM thin pool after resolving the target",
        ),
    }
}

fn missing_lvm_resize_inputs(
    target_input: &'static str,
    size_input: &'static str,
    desired_size: Option<&str>,
) -> Vec<&'static str> {
    let mut missing = vec![target_input];
    if desired_size.is_none() {
        missing.push(size_input);
    }
    missing
}

fn lvm_lvremove_command(
    target: Option<&str>,
    placeholder: &'static str,
    target_input: &'static str,
    description: &'static str,
) -> ExecutionCommand {
    match target {
        Some(target) => command(["lvremove", "--yes", target], true, description),
        None => command_with_readiness(
            ["lvremove", "--yes", placeholder],
            true,
            CommandReadiness::NeedsDomainImplementation,
            [target_input],
            description,
        ),
    }
}

fn lvm_lvrename_command(
    target: Option<&str>,
    rename_to: Option<&str>,
    placeholder: &'static str,
    target_input: &'static str,
    rename_input: &'static str,
    description: &'static str,
) -> ExecutionCommand {
    match (target, rename_to) {
        (Some(target), Some(rename_to)) => {
            command(["lvrename", target, rename_to], true, description)
        }
        (target, rename_to) => command_vec_with_readiness(
            vec![
                "lvrename".to_string(),
                target.unwrap_or(placeholder).to_string(),
                rename_to.unwrap_or("<new-logical-volume>").to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_rename_inputs(target_input, rename_input, target, rename_to),
            description,
        ),
    }
}

fn lvm_lvchange_activate_command(
    target: Option<&str>,
    flag: &'static str,
    placeholder: &'static str,
    target_input: &'static str,
) -> ExecutionCommand {
    let description = if flag == "y" {
        "activate the reviewed LVM logical volume"
    } else {
        "deactivate the reviewed LVM logical volume without deleting data"
    };
    match target {
        Some(target) => command(["lvchange", "--activate", flag, target], true, description),
        None => command_vec_with_readiness(
            vec![
                "lvchange".to_string(),
                "--activate".to_string(),
                flag.to_string(),
                placeholder.to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            [target_input],
            if flag == "y" {
                "activate the LVM logical volume after selecting the target"
            } else {
                "deactivate the LVM logical volume after selecting the target"
            },
        ),
    }
}

fn missing_rename_inputs(
    target_input: &'static str,
    rename_input: &'static str,
    target: Option<&str>,
    rename_to: Option<&str>,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if target.is_none() {
        missing.push(target_input);
    }
    if rename_to.is_none() {
        missing.push(rename_input);
    }
    missing
}

fn lvm_snapshot_create_command(
    origin: &str,
    snapshot: &str,
    desired_size: Option<&str>,
) -> ExecutionCommand {
    match desired_size {
        Some(size) => command_vec(
            vec![
                "lvcreate",
                "--snapshot",
                "--size",
                size,
                "--name",
                snapshot,
                origin,
            ],
            true,
            "create an LVM snapshot of the origin logical volume",
        ),
        None => command_with_readiness(
            [
                "lvcreate",
                "--snapshot",
                "--size",
                "<size>",
                "--name",
                snapshot,
                origin,
            ],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired LVM snapshot size"],
            "create an LVM snapshot after selecting the snapshot size",
        ),
    }
}

fn lvm_logical_volume_create_command(target: &str, desired_size: Option<&str>) -> ExecutionCommand {
    let Some((volume_group, logical_volume)) = target.split_once('/') else {
        let size_flag = desired_size.map(lvm_size_flag).unwrap_or("--size");
        let size_value = desired_size
            .map(lvm_size_value)
            .unwrap_or_else(|| "<size>".to_string());
        return command_vec_with_readiness(
            vec![
                "lvcreate".to_string(),
                "--yes".to_string(),
                "--wipesignatures".to_string(),
                "y".to_string(),
                size_flag.to_string(),
                size_value,
                "--name".to_string(),
                "<logical-volume>".to_string(),
                "<volume-group>".to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_lvm_create_inputs(
                "target in volume-group/logical-volume form",
                "desired logical volume size",
                desired_size,
            ),
            "create an LVM logical volume after resolving volume group and LV name",
        );
    };

    match desired_size {
        Some(size) if size.contains('%') => command_vec(
            vec![
                "lvcreate".to_string(),
                "--yes".to_string(),
                "--wipesignatures".to_string(),
                "y".to_string(),
                "--extents".to_string(),
                lvm_size_value(size),
                "--name".to_string(),
                logical_volume.to_string(),
                volume_group.to_string(),
            ],
            true,
            "create an LVM logical volume with the desired extent allocation",
        ),
        Some(size) => command_vec(
            vec![
                "lvcreate".to_string(),
                "--yes".to_string(),
                "--wipesignatures".to_string(),
                "y".to_string(),
                "--size".to_string(),
                size.to_string(),
                "--name".to_string(),
                logical_volume.to_string(),
                volume_group.to_string(),
            ],
            true,
            "create an LVM logical volume with the desired size",
        ),
        None => command_vec_with_readiness(
            vec![
                "lvcreate".to_string(),
                "--yes".to_string(),
                "--wipesignatures".to_string(),
                "y".to_string(),
                "--size".to_string(),
                "<size>".to_string(),
                "--name".to_string(),
                logical_volume.to_string(),
                volume_group.to_string(),
            ],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired logical volume size"],
            "create an LVM logical volume after selecting the desired size",
        ),
    }
}

fn lvm_size_flag(size: &str) -> &'static str {
    if size.contains('%') {
        "--extents"
    } else {
        "--size"
    }
}

fn lvm_size_value(size: &str) -> String {
    if size.ends_with('%') {
        format!("{size}FREE")
    } else {
        size.to_string()
    }
}

fn missing_lvm_create_inputs(
    target_input: &'static str,
    size_input: &'static str,
    desired_size: Option<&str>,
) -> Vec<&'static str> {
    let mut missing = vec![target_input];
    if desired_size.is_none() {
        missing.push(size_input);
    }
    missing
}

fn lvm_volume_group_create_command(target: &str, device: Option<&str>) -> ExecutionCommand {
    match device {
        Some(device) => command(
            ["vgcreate", target, device],
            true,
            "create an LVM volume group on the reviewed physical volume",
        ),
        None => command_with_readiness(
            ["vgcreate", target, "<physical-volume>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["physical volume device"],
            "create an LVM volume group after selecting the physical volume",
        ),
    }
}

fn lvm_physical_volume_target(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .device
        .as_deref()
        .or(action.context.target.as_deref())
        .or(action.context.name.as_deref())
        .filter(|target| is_path_like(target))
}

fn is_path_like(target: &str) -> bool {
    target.starts_with('/')
}

fn lvm_physical_volume_create_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["pvcreate", target],
            true,
            "create LVM physical volume metadata on the reviewed device",
        ),
        None => command_with_readiness(
            ["pvcreate", "<physical-volume>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["physical volume device"],
            "create LVM physical volume metadata after selecting the device",
        ),
    }
}

fn lvm_physical_volume_resize_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["pvresize", target],
            true,
            "resize the LVM physical volume after backing storage growth",
        ),
        None => command_with_readiness(
            ["pvresize", "<physical-volume>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["physical volume device"],
            "resize the LVM physical volume after selecting the device",
        ),
    }
}

fn lvm_physical_volume_rescan_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["pvscan", "--cache", target],
            true,
            "refresh LVM physical volume metadata cache for the reviewed device",
        ),
        None => command(
            ["pvscan", "--cache"],
            true,
            "refresh the LVM physical volume metadata cache",
        ),
    }
}

fn lvm_physical_volume_remove_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["pvremove", "--yes", target],
            true,
            "remove LVM physical volume metadata from the reviewed device",
        ),
        None => command_with_readiness(
            ["pvremove", "--yes", "<physical-volume>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["physical volume device"],
            "remove LVM physical volume metadata after selecting the device",
        ),
    }
}

fn volume_group_extend_command(target: &str, device: Option<&str>) -> ExecutionCommand {
    match device {
        Some(device) => command(
            ["vgextend", target, device],
            true,
            "extend the LVM volume group with the reviewed physical volume",
        ),
        None => command_with_readiness(
            ["vgextend", target, "<physical-volume>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["physical volume device"],
            "extend the LVM volume group after selecting the physical volume",
        ),
    }
}

fn lvm_volume_group_extend_replacement_command(
    target: &str,
    replacement: Option<&str>,
) -> ExecutionCommand {
    match replacement {
        Some(replacement) => command(
            ["vgextend", target, replacement],
            true,
            "extend the LVM volume group with the replacement physical volume",
        ),
        None => command_with_readiness(
            ["vgextend", target, "<replacement-physical-volume>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["replacement physical volume"],
            "extend the LVM volume group after selecting the replacement physical volume",
        ),
    }
}

fn lvm_physical_volume_move_to_command(
    source: Option<&str>,
    destination: Option<&str>,
) -> ExecutionCommand {
    let source_arg = source.unwrap_or("<physical-volume>");
    let destination_arg = destination.unwrap_or("<replacement-physical-volume>");
    let mut missing = Vec::new();
    if source.is_none() {
        missing.push("physical volume to replace");
    }
    if destination.is_none() {
        missing.push("replacement physical volume");
    }

    if missing.is_empty() {
        command(
            ["pvmove", source_arg, destination_arg],
            true,
            "move allocated extents from the old PV to the replacement PV",
        )
    } else {
        command_vec_with_readiness(
            vec![
                "pvmove".to_string(),
                source_arg.to_string(),
                destination_arg.to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "move extents after selecting the old and replacement physical volumes",
        )
    }
}

fn loop_device_create_command(target: &str, backing: Option<&str>) -> ExecutionCommand {
    match backing {
        Some(backing) if target.starts_with("/dev/loop") => command(
            ["losetup", target, backing],
            true,
            "create the requested loop-device mapping",
        ),
        Some(backing) => command(
            ["losetup", "--find", "--show", backing],
            true,
            "create a loop-device mapping with the next available loop device",
        ),
        None => command_with_readiness(
            ["losetup", "--find", "--show", "<backing-file>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["backing file or block device"],
            "create a loop-device mapping after selecting the backing path",
        ),
    }
}

fn loop_device_target_path(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .target
        .as_deref()
        .filter(|target| target.starts_with("/dev/loop"))
        .or_else(|| {
            action
                .context
                .name
                .as_deref()
                .filter(|name| name.starts_with("/dev/loop"))
        })
}

fn loop_device_list_command(target: Option<&str>, description: &'static str) -> ExecutionCommand {
    match target {
        Some(target) => command(["losetup", "--json", "--list", target], false, description),
        None => command_with_readiness(
            ["losetup", "--json", "--list", "<loop-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["loop device path"],
            description,
        ),
    }
}
