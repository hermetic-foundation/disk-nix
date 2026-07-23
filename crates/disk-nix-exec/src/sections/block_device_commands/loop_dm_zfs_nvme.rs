fn loop_device_inspect_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["disk-nix", "inspect", target],
            false,
            "inspect modeled loop device relationships after refresh",
        ),
        None => command_with_readiness(
            ["disk-nix", "inspect", "<loop-device>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["loop device path"],
            "inspect modeled loop device relationships after selecting the loop path",
        ),
    }
}

fn loop_device_refresh_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["losetup", "-c", target],
            true,
            "refresh the loop device size after backing storage growth",
        ),
        None => command_with_readiness(
            ["losetup", "-c", "<loop-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["loop device path"],
            "refresh the loop device size after backing storage growth",
        ),
    }
}

fn loop_device_detach_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["losetup", "--detach", target],
            true,
            "detach the loop device without deleting the backing file",
        ),
        None => command_with_readiness(
            ["losetup", "--detach", "<loop-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["loop device path"],
            "detach the loop device without deleting the backing file",
        ),
    }
}

fn backing_file_target_path(action: &PlannedAction) -> Option<&str> {
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

fn backing_file_stat_command(target: Option<&str>, description: &'static str) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["stat", "--printf=%n %s %b %B\\n", target],
            false,
            description,
        ),
        None => command_with_readiness(
            ["stat", "--printf=%n %s %b %B\\n", "<backing-file>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["backing file path"],
            description,
        ),
    }
}

fn backing_file_usage_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["du", "--bytes", "--apparent-size", target],
            false,
            "inspect backing file apparent size",
        ),
        None => command_with_readiness(
            ["du", "--bytes", "--apparent-size", "<backing-file>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["backing file path"],
            "inspect backing file apparent size",
        ),
    }
}

fn backing_file_inspect_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["disk-nix", "inspect", target],
            false,
            "inspect modeled backing-file relationships",
        ),
        None => command_with_readiness(
            ["disk-nix", "inspect", "<backing-file>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["backing file path"],
            "inspect modeled backing-file relationships",
        ),
    }
}

fn backing_file_inspect_json_command(
    target: Option<&str>,
    description: &'static str,
) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["disk-nix", "inspect", target, "--json"],
            false,
            description,
        ),
        None => command_with_readiness(
            ["disk-nix", "inspect", "<backing-file>", "--json"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["backing file path"],
            description,
        ),
    }
}

fn backing_file_absent_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["test", "!", "-e", target],
            false,
            "refuse to overwrite an existing backing file",
        ),
        None => command_with_readiness(
            ["test", "!", "-e", "<backing-file>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["backing file path"],
            "refuse to overwrite an existing backing file after selecting the path",
        ),
    }
}

fn backing_file_create_command(
    target: Option<&str>,
    desired_size: Option<&str>,
) -> ExecutionCommand {
    match (target, desired_size) {
        (Some(target), Some(size)) => command_vec(
            vec!["truncate", "--size", size, target],
            true,
            "create the new sparse backing file at the requested size",
        ),
        (Some(target), None) => command_with_readiness(
            ["truncate", "--size", "<size>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["desired backing file size"],
            "create the backing file after selecting a desired size",
        ),
        (None, Some(size)) => command_with_readiness(
            ["truncate", "--size", size, "<backing-file>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["backing file path"],
            "create the selected backing file at the requested size",
        ),
        (None, None) => command_with_readiness(
            ["truncate", "--size", "<size>", "<backing-file>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["backing file path", "desired backing file size"],
            "create the backing file after selecting a path and desired size",
        ),
    }
}

fn backing_file_grow_command(target: Option<&str>, desired_size: Option<&str>) -> ExecutionCommand {
    match (target, desired_size) {
        (Some(target), Some(size)) => command_vec(
            vec!["truncate", "--size", size, target],
            true,
            "extend the backing file to the requested size",
        ),
        (Some(target), None) => command_with_readiness(
            ["truncate", "--size", "<size>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["desired backing file size"],
            "extend the backing file after selecting a desired size",
        ),
        (None, Some(size)) => command_with_readiness(
            ["truncate", "--size", size, "<backing-file>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["backing file path"],
            "extend the selected backing file to the requested size",
        ),
        (None, None) => command_with_readiness(
            ["truncate", "--size", "<size>", "<backing-file>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["backing file path", "desired backing file size"],
            "extend the backing file after selecting a path and desired size",
        ),
    }
}

fn dm_map_target_path(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .target
        .as_deref()
        .filter(|target| is_dm_map_target(target))
        .or_else(|| {
            action
                .context
                .name
                .as_deref()
                .filter(|name| is_dm_map_target(name))
        })
}

fn is_dm_map_target(target: &str) -> bool {
    target.starts_with("/dev/mapper/") || target.starts_with("/dev/dm-")
}

fn dm_map_rename_to(action: &PlannedAction) -> Option<String> {
    action
        .context
        .rename_to
        .as_deref()
        .and_then(|rename_to| rename_to.strip_prefix("/dev/mapper/").or(Some(rename_to)))
        .filter(|rename_to| !rename_to.is_empty() && !rename_to.contains('/'))
        .map(ToString::to_string)
}

fn dmsetup_info_command(target: Option<&str>, description: &'static str) -> ExecutionCommand {
    match target {
        Some(target) => command(
            [
                "dmsetup",
                "info",
                "-c",
                "--noheadings",
                "-o",
                "name,uuid,major,minor,open,segments,events",
                target,
            ],
            false,
            description,
        ),
        None => command_with_readiness(
            [
                "dmsetup",
                "info",
                "-c",
                "--noheadings",
                "-o",
                "name,uuid,major,minor,open,segments,events",
                "<dm-map>",
            ],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["device-mapper path"],
            description,
        ),
    }
}

fn dmsetup_rename_command(target: Option<&str>, rename_to: Option<&str>) -> ExecutionCommand {
    match (target, rename_to) {
        (Some(target), Some(rename_to)) => command_vec(
            vec![
                "dmsetup".to_string(),
                "rename".to_string(),
                target.to_string(),
                rename_to.to_string(),
            ],
            true,
            "rename the reviewed device-mapper map",
        ),
        (target, rename_to) => command_vec_with_readiness(
            vec![
                "dmsetup".to_string(),
                "rename".to_string(),
                target.unwrap_or("<dm-map>").to_string(),
                rename_to.unwrap_or("<new-dm-map-name>").to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_dm_map_rename_inputs(target, rename_to),
            "rename the device-mapper map after selecting a concrete mapper path and new map name",
        ),
    }
}

fn missing_dm_map_rename_inputs(
    target: Option<&str>,
    rename_to: Option<&str>,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if target.is_none() {
        missing.push("device-mapper path");
    }
    if rename_to.is_none() {
        missing.push("new device-mapper name");
    }
    missing
}

fn dmsetup_remove_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["dmsetup", "remove", target],
            true,
            "remove the reviewed device-mapper map",
        ),
        None => command_with_readiness(
            ["dmsetup", "remove", "<dm-map>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["device-mapper path"],
            "remove the device-mapper map after selecting a concrete mapper path",
        ),
    }
}

fn dmsetup_ls_tree_command(description: &'static str) -> ExecutionCommand {
    command(["dmsetup", "ls", "--tree"], false, description)
}

fn dmsetup_deps_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["dmsetup", "deps", "-o", "devname", target],
            false,
            "refresh device-mapper dependency metadata",
        ),
        None => command_with_readiness(
            ["dmsetup", "deps", "-o", "devname", "<dm-map>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["device-mapper path"],
            "refresh device-mapper dependency metadata after selecting the mapper path",
        ),
    }
}

fn dmsetup_table_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["dmsetup", "table", target],
            false,
            "refresh device-mapper table metadata",
        ),
        None => command_with_readiness(
            ["dmsetup", "table", "<dm-map>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["device-mapper path"],
            "refresh device-mapper table metadata after selecting the mapper path",
        ),
    }
}

fn dmsetup_status_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["dmsetup", "status", target],
            false,
            "refresh device-mapper live status metadata",
        ),
        None => command_with_readiness(
            ["dmsetup", "status", "<dm-map>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["device-mapper path"],
            "refresh device-mapper live status metadata after selecting the mapper path",
        ),
    }
}

fn dm_map_inspect_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["disk-nix", "inspect", target],
            false,
            "inspect modeled device-mapper relationships",
        ),
        None => command_with_readiness(
            ["disk-nix", "inspect", "<dm-map>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["device-mapper path"],
            "inspect modeled device-mapper relationships after selecting the mapper path",
        ),
    }
}

fn dm_map_inspect_json_command(
    target: Option<&str>,
    description: &'static str,
) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["disk-nix", "inspect", target, "--json"],
            false,
            description,
        ),
        None => command_with_readiness(
            ["disk-nix", "inspect", "<dm-map>", "--json"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["device-mapper path"],
            description,
        ),
    }
}

fn zvol_create_command(
    target: &str,
    desired_size: Option<&str>,
    property_assignments: &[String],
) -> ExecutionCommand {
    match desired_size {
        Some(size) => {
            let mut argv = zfs_create_wrapper_argv(target, property_assignments);
            argv.push("-V".to_string());
            argv.push(size.to_string());
            command_vec(
                argv,
                true,
                "create a zvol with the desired volume size when it is not already present",
            )
        }
        None => {
            let mut argv = zfs_create_wrapper_argv(target, property_assignments);
            argv.push("-V".to_string());
            argv.push("<size>".to_string());
            command_vec_with_readiness(
                argv,
                true,
                CommandReadiness::NeedsDesiredSize,
                ["desired zvol size"],
                "create a zvol after selecting the desired volume size",
            )
        }
    }
}

fn zfs_dataset_create_command(target: &str, property_assignments: &[String]) -> ExecutionCommand {
    let argv = zfs_create_wrapper_argv(target, property_assignments);
    command_vec(
        argv,
        true,
        "create the reviewed ZFS filesystem dataset when it is not already present",
    )
}

fn zfs_create_wrapper_argv(target: &str, property_assignments: &[String]) -> Vec<String> {
    let mut argv = vec![
        "bash".to_string(),
        "-c".to_string(),
        "target=\"$1\"; shift; if zfs list -H \"$target\" >/dev/null 2>&1; then exit 0; fi; if zfs create \"$@\" \"$target\"; then exit 0; fi; status=\"$?\"; if zfs list -H \"$target\" >/dev/null 2>&1; then exit 0; fi; exit \"$status\"".to_string(),
        "disk-nix-zfs-create".to_string(),
        target.to_string(),
    ];
    for assignment in property_assignments {
        argv.push("-o".to_string());
        argv.push(assignment.clone());
    }
    argv
}

fn zfs_dataset_property_is_create_time_only(property: &str) -> bool {
    matches!(property, "encryption" | "keyformat")
}

fn zfs_zvol_property_is_create_time_only(property: &str) -> bool {
    matches!(property, "encryption" | "keyformat" | "volblocksize")
}

fn zfs_idempotent_set_property_command(
    target: &str,
    property: &str,
    assignment: &str,
    note: &'static str,
) -> ExecutionCommand {
    let Some((_, desired)) = assignment.split_once('=') else {
        return command(["zfs", "set", assignment, target], true, note);
    };
    command_vec(
        vec![
            "bash",
            "-c",
            "target=\"$1\"; property=\"$2\"; desired=\"$3\"; assignment=\"$property=$desired\"; current=\"$(zfs get -H -p -o value \"$property\" \"$target\" 2>/dev/null || true)\"; if [ \"$current\" = \"$desired\" ]; then exit 0; fi; exec zfs set \"$assignment\" \"$target\"",
            "disk-nix-zfs-set",
            target,
            property,
            desired,
        ],
        true,
        note,
    )
}

fn zvol_set_volsize_command(target: &str, desired_size: Option<&str>) -> ExecutionCommand {
    match desired_size {
        Some(size) => command_vec(
            vec!["zfs", "set", &format!("volsize={size}"), target],
            true,
            "grow the zvol by setting volsize",
        ),
        None => command_with_readiness(
            ["zfs", "set", "volsize=<size>", target],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired zvol size"],
            "grow the zvol after selecting the desired volume size",
        ),
    }
}

fn nvme_controller_target(action: &PlannedAction) -> Option<&str> {
    [
        action.context.device.as_deref(),
        action.context.target.as_deref(),
        action.context.name.as_deref(),
    ]
    .into_iter()
    .flatten()
    .find(|target| is_nvme_controller_path(target))
}

fn is_nvme_controller_path(target: &str) -> bool {
    target
        .strip_prefix("/dev/nvme")
        .is_some_and(|suffix| !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit()))
}

fn nvme_list_namespaces_command(
    controller: Option<&str>,
    description: &'static str,
) -> ExecutionCommand {
    match controller {
        Some(controller) => command(
            [
                "nvme",
                "list-ns",
                controller,
                "--all",
                "--output-format=json",
            ],
            false,
            description,
        ),
        None => command_with_readiness(
            [
                "nvme",
                "list-ns",
                "<nvme-controller>",
                "--all",
                "--output-format=json",
            ],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["NVMe controller path such as /dev/nvme0"],
            description,
        ),
    }
}

fn nvme_list_subsystems_command(description: &'static str) -> ExecutionCommand {
    command(
        ["nvme", "list-subsys", "--output-format=json"],
        false,
        description,
    )
}

fn nvme_create_namespace_command(
    controller: Option<&str>,
    desired_size: Option<&str>,
) -> ExecutionCommand {
    let controller_arg = controller.unwrap_or("<nvme-controller>");
    let size_arg = desired_size.unwrap_or("<size>");
    let argv = vec![
        "nvme",
        "create-ns",
        controller_arg,
        "--nsze-si",
        size_arg,
        "--ncap-si",
        size_arg,
    ];
    match (controller, desired_size) {
        (Some(_), Some(_)) => command_vec(
            argv,
            true,
            "create an NVMe namespace with the reviewed size and capacity",
        ),
        (Some(_), None) => command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired namespace size"],
            "create an NVMe namespace after selecting size and capacity",
        ),
        (None, desired_size) => command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_nvme_namespace_inputs(true, false, false, desired_size.is_none()),
            "create an NVMe namespace after selecting the controller and size",
        ),
    }
}

fn nvme_attach_namespace_command(
    controller: Option<&str>,
    namespace_id: Option<&str>,
    controllers: Option<&str>,
) -> ExecutionCommand {
    let controller_arg = controller.unwrap_or("<nvme-controller>");
    let namespace_arg = namespace_id.unwrap_or("<namespace-id>");
    let controllers_arg = controllers.unwrap_or("<controller-id-list>");
    let argv = vec![
        "nvme",
        "attach-ns",
        controller_arg,
        "--namespace-id",
        namespace_arg,
        "--controllers",
        controllers_arg,
    ];
    if controller.is_some() && namespace_id.is_some() && controllers.is_some() {
        command_vec(
            argv,
            true,
            "attach the reviewed NVMe namespace to controllers",
        )
    } else {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_nvme_namespace_inputs(
                controller.is_none(),
                namespace_id.is_none(),
                controllers.is_none(),
                false,
            ),
            "attach the NVMe namespace after selecting namespace id and controllers",
        )
    }
}

fn nvme_detach_namespace_command(
    controller: Option<&str>,
    namespace_id: Option<&str>,
    controllers: Option<&str>,
) -> ExecutionCommand {
    let controller_arg = controller.unwrap_or("<nvme-controller>");
    let namespace_arg = namespace_id.unwrap_or("<namespace-id>");
    let controllers_arg = controllers.unwrap_or("<controller-id-list>");
    let argv = vec![
        "nvme",
        "detach-ns",
        controller_arg,
        "--namespace-id",
        namespace_arg,
        "--controllers",
        controllers_arg,
    ];
    if controller.is_some() && namespace_id.is_some() && controllers.is_some() {
        command_vec(
            argv,
            true,
            "detach the reviewed NVMe namespace from controllers before deletion",
        )
    } else {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_nvme_namespace_inputs(
                controller.is_none(),
                namespace_id.is_none(),
                controllers.is_none(),
                false,
            ),
            "detach the NVMe namespace after selecting namespace id and controllers",
        )
    }
}

fn nvme_delete_namespace_command(
    controller: Option<&str>,
    namespace_id: Option<&str>,
) -> ExecutionCommand {
    let controller_arg = controller.unwrap_or("<nvme-controller>");
    let namespace_arg = namespace_id.unwrap_or("<namespace-id>");
    let argv = vec![
        "nvme",
        "delete-ns",
        controller_arg,
        "--namespace-id",
        namespace_arg,
    ];
    if controller.is_some() && namespace_id.is_some() {
        command_vec(argv, true, "delete the reviewed NVMe namespace")
    } else {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_nvme_namespace_inputs(
                controller.is_none(),
                namespace_id.is_none(),
                false,
                false,
            ),
            "delete the NVMe namespace after selecting namespace id",
        )
    }
}

fn nvme_namespace_rescan_command(controller: Option<&str>) -> ExecutionCommand {
    match controller {
        Some(controller) => command(
            ["nvme", "ns-rescan", controller],
            true,
            "rescan NVMe namespaces after controller-side changes",
        ),
        None => command_with_readiness(
            ["nvme", "ns-rescan", "<nvme-controller>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["NVMe controller path such as /dev/nvme0"],
            "rescan NVMe namespaces after selecting the controller",
        ),
    }
}

fn missing_nvme_namespace_inputs(
    missing_controller: bool,
    missing_namespace: bool,
    missing_controllers: bool,
    missing_size: bool,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if missing_controller {
        missing.push("NVMe controller path such as /dev/nvme0");
    }
    if missing_namespace {
        missing.push("namespace id");
    }
    if missing_controllers {
        missing.push("controller id list");
    }
    if missing_size {
        missing.push("desired namespace size");
    }
    missing
}

fn md_raid_create_command(
    target: Option<&str>,
    level: Option<&str>,
    metadata: Option<&str>,
    devices: &[String],
) -> ExecutionCommand {
    let missing_target = target.is_none();
    let missing_level = level.is_none();
    let missing_devices = devices.is_empty();
    let target = target.unwrap_or("<md-array>");
    let level = level.unwrap_or("<level>");
    let raid_devices = if missing_devices {
        "<member-count>".to_string()
    } else {
        devices.len().to_string()
    };
    let mut argv = vec![
        "mdadm".to_string(),
        "--create".to_string(),
        target.to_string(),
        "--level".to_string(),
        level.to_string(),
        "--raid-devices".to_string(),
        raid_devices,
        "--bitmap".to_string(),
        "none".to_string(),
    ];
    if let Some(name) = target
        .strip_prefix("/dev/md/")
        .filter(|name| !name.is_empty())
    {
        argv.extend(["--name".to_string(), name.to_string()]);
    }
    if let Some(metadata) = metadata {
        argv.extend(["--metadata".to_string(), metadata.to_string()]);
    }
    if missing_devices {
        argv.push("<member-device>".to_string());
    } else {
        argv.extend(devices.iter().cloned());
    }

    if missing_target || missing_level || missing_devices {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_md_raid_create_inputs(missing_target, missing_level, missing_devices),
            "create the MD RAID array after selecting level and reviewed member devices",
        )
    } else {
        command_vec(
            argv,
            true,
            "create the reviewed MD RAID array from explicit member devices",
        )
    }
}

fn md_raid_assemble_command(target: Option<&str>, devices: &[String]) -> ExecutionCommand {
    let missing_target = target.is_none();
    let missing_devices = devices.is_empty();
    let target_arg = target.unwrap_or("<md-array>");
    let mut argv = vec![
        "mdadm".to_string(),
        "--assemble".to_string(),
        target_arg.to_string(),
    ];
    if missing_devices {
        argv.push("<member-device>".to_string());
    } else {
        argv.extend(devices.iter().cloned());
    }

    if missing_target || missing_devices {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_md_raid_assemble_inputs(missing_target, missing_devices),
            "assemble the MD RAID array after selecting the array and reviewed member devices",
        )
    } else {
        command_vec(
            argv,
            true,
            "assemble the reviewed MD RAID array from existing member metadata",
        )
    }
}

fn missing_md_raid_assemble_inputs(
    missing_target: bool,
    missing_devices: bool,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if missing_target {
        missing.push("MD array path");
    }
    if missing_devices {
        missing.push("member devices");
    }
    missing
}

fn md_raid_stop_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["mdadm", "--stop", target],
            true,
            "stop the reviewed MD RAID array without removing member metadata",
        ),
        None => command_with_readiness(
            ["mdadm", "--stop", "<md-array>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["MD array path"],
            "stop the MD RAID array after selecting the array path",
        ),
    }
}
