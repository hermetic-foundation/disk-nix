fn advanced_growth_action_commands(
    action: &PlannedAction,
    collection: Option<&str>,
    target: Option<&str>,
    cache_target: Option<&str>,
) -> Option<ActionCommandResult> {
    let _ = cache_target;
    Some(match action.operation {
        Operation::Grow if collection == Some("swaps") => {
            let target = swap_target_path(action);
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    command(
                        ["swapon", "--show", "--bytes", "--raw"],
                        false,
                        "inspect active swap state before resizing",
                    ),
                    swap_command(
                        "swapoff",
                        target,
                        "disable swap before changing backing storage or signature",
                    ),
                    swap_resize_command(target, desired_size),
                    swap_command(
                        "mkswap",
                        target,
                        "recreate the swap signature after backing storage resize",
                    ),
                    swap_command("swapon", target, "reactivate swap after verification"),
                ],
                vec![
                    "verify memory pressure and hibernation dependencies before swapoff"
                        .to_string(),
                    "prefer adding replacement swap capacity before resizing active swap"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Deactivate if collection == Some("swaps") => {
            let target = swap_target_path(action);
            (
                vec![
                    command(
                        ["swapon", "--show", "--bytes", "--raw"],
                        false,
                        "inspect active swap state before swapoff",
                    ),
                    swap_command("swapoff", target, "disable active swap without removing its signature"),
                ],
                vec![
                    "verify memory pressure and hibernation dependencies before swapoff"
                        .to_string(),
                    "use destroy only when swap signature metadata should be removed".to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("swaps") => {
            let target = swap_target_path(action);
            (
                vec![
                    disk_nix_inspect_command(
                        target,
                        "<swap>",
                        "swap target path",
                        "inspect target before disabling and wiping swap signature",
                    ),
                    swap_command("swapoff", target, "disable active swap before removing its signature"),
                    swap_wipefs_command(target),
                ],
                vec![
                    "remove or update NixOS swapDevices before wiping the signature".to_string(),
                    "verify resume and hibernation references before deleting swap metadata"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("swaps") => {
            let target = swap_target_path(action);
            (
                vec![
                    command(
                        ["swapon", "--show", "--bytes", "--raw"],
                        false,
                        "refresh active swap inventory",
                    ),
                    swap_blkid_command(target, "refresh swap signature label and UUID"),
                    swap_inspect_command(
                        target,
                        "inspect modeled swap relationships after refresh",
                    ),
                ],
                vec![
                    "use grow when backing swap capacity changed".to_string(),
                    "use format only when replacing the swap signature is intended".to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("zram") => (
            zram_rescan_commands("refresh zram compressed swap inventory"),
            vec![
                "use services.disk-nix.zram to reconcile generated NixOS zramSwap settings"
                    .to_string(),
                "coordinate swapoff before changing live zram size, algorithm, priority, or writeback device"
                    .to_string(),
            ],
            true,
        ),
        Operation::SetProperty if collection == Some("zram") => (
            zram_rescan_commands("inspect zram compressed swap declaration and current inventory"),
            vec![
                "plain zram declarations inspect generated compressed swap state without mutating it"
                    .to_string(),
                "use operation = \"rescan\" for an explicit zram inventory refresh action"
                    .to_string(),
                "use services.disk-nix.zram options to reconcile generated NixOS zramSwap settings"
                    .to_string(),
            ],
            false,
        ),
        Operation::Grow if collection == Some("luks.devices") => {
            let mapper = target.unwrap_or("<mapper>");
            let device = action.context.device.as_deref();
            (
                vec![
                    luks_backing_inspect_command(
                        device,
                        "inspect backing device before resizing the LUKS mapper",
                    ),
                    command(
                        ["cryptsetup", "status", mapper],
                        false,
                        "inspect open LUKS mapper before resize",
                    ),
                    command(
                        ["cryptsetup", "resize", mapper],
                        true,
                        "resize the open LUKS mapping after backing capacity changes",
                    ),
                ],
                vec![
                    "grow the backing partition, LUN, or volume before resizing the mapper"
                        .to_string(),
                    "coordinate dependent LVM and filesystem resizing after cryptsetup resize"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("vdoVolumes") => {
            let target = target.unwrap_or("<vdo-volume>");
            let desired_size = action.context.desired_size.as_deref();
            let physical_size = action.context.physical_size.as_deref();
            let mut commands = vec![command(
                ["vdo", "status", "--name", target],
                false,
                "inspect VDO logical and physical size before growth",
            )];
            commands.extend(vdo_growth_commands(target, desired_size, physical_size));
            (
                commands,
                vec![
                    "choose logical and physical growth intentionally; they are separate VDO operations"
                        .to_string(),
                    "confirm backing storage capacity before physical VDO growth".to_string(),
                    "review deduplication, compression, and slab utilization before increasing logical size"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Start if collection == Some("vdoVolumes") => {
            let target = target.unwrap_or("<vdo-volume>");
            (
                vec![
                    command(
                        ["vdo", "status", "--name", target],
                        false,
                        "inspect VDO volume before start",
                    ),
                    command(
                        ["vdo", "start", "--name", target],
                        true,
                        "start the existing VDO volume after backing storage is present",
                    ),
                ],
                vec![
                    "verify the backing device is present and stable before starting VDO".to_string(),
                    "activate dependent filesystems, LVM layers, or mounts only after VDO status is healthy"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("zvols") => {
            let target = target.unwrap_or("<zvol>");
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    command(
                        ["zfs", "get", "-H", "-p", "volsize", target],
                        false,
                        "inspect current zvol size before growth",
                    ),
                    zvol_set_volsize_command(target, desired_size),
                ],
                vec![
                    "verify pool free space and reservation policy before increasing volsize"
                        .to_string(),
                    "rescan dependent block consumers after zvol growth".to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("mdRaids") => {
            let target = md_array_target_path(action);
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    md_raid_detail_command(
                        target,
                        "inspect MD RAID array health before grow or reshape",
                    ),
                    md_raid_grow_command(target, desired_size),
                    command(
                        ["cat", "/proc/mdstat"],
                        false,
                        "monitor MD RAID reshape, recovery, or resync state",
                    ),
                ],
                vec![
                    "verify backups and redundancy before reshape".to_string(),
                    "do not grow dependent filesystems until mdadm reports the new array size"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("mdRaids") => {
            let target = md_array_target_path(action);
            let mut commands = Vec::new();
            if let Some(target) = target {
                commands.push(command(
                    ["mdadm", "--detail", target],
                    false,
                    "inspect targeted MD RAID array before metadata rescan",
                ));
            }
            commands.extend([
                command(
                    ["mdadm", "--detail", "--scan"],
                    false,
                    "list assembled MD RAID arrays from current metadata",
                ),
                command(
                    ["mdadm", "--examine", "--scan"],
                    false,
                    "scan member devices for MD RAID metadata without assembling arrays",
                ),
                command(
                    ["cat", "/proc/mdstat"],
                    false,
                    "inspect kernel MD RAID status after metadata scan",
                ),
            ]);
            (
                commands,
                vec![
                    "use assemble when reviewed member metadata should activate an array"
                        .to_string(),
                    "verify member event counts before any replacement, grow, or assemble operation"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("multipathMaps") => {
            let target = multipath_map_target(action);
            (
                vec![
                    multipath_list_command(
                        target,
                        "inspect multipath map paths and size before growth",
                    ),
                    lsscsi_lun_inventory_command(
                        "inspect host-visible SCSI path transport and size before multipath growth",
                    ),
                    multipath_resize_command(target),
                    command(
                        ["multipath", "-r"],
                        true,
                        "reload multipath maps after path rescans",
                    ),
                ],
                vec![
                    "rescan each SCSI path before resizing the multipath map".to_string(),
                    "grow dependent volumes or filesystems only after the map reports the new size"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("multipathMaps") => {
            let target = multipath_map_target(action);
            (
                vec![
                    multipath_list_command(target, "inspect multipath map paths before rescan"),
                    lsscsi_lun_inventory_command(
                        "inspect host-visible SCSI path transport and size before multipath rescan",
                    ),
                    command(
                        ["multipath", "-r"],
                        true,
                        "reload multipath maps after refreshed backing paths",
                    ),
                    multipath_list_command(target, "verify multipath map paths after rescan"),
                    lsscsi_lun_inventory_command(
                        "verify host-visible SCSI path transport and size after multipath rescan",
                    ),
                ],
                vec![
                    "rescan backing SCSI or iSCSI paths before reloading the map".to_string(),
                    "verify the map WWID and every expected path before exposing consumers"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("multipathMaps") => {
            let target = multipath_map_target(action);
            (
                vec![
                    multipath_list_command(target, "inspect multipath map paths before removal"),
                    multipath_flush_map_command(target),
                ],
                vec![
                    "multipath map removal flushes the host map but does not delete target-side data"
                        .to_string(),
                    "unmount filesystems and deactivate LVM, dm, and service consumers before flushing the map"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("nvmeNamespaces") => {
            let controller = nvme_controller_target(action);
            (
                vec![
                    nvme_list_namespaces_command(
                        controller,
                        "inspect NVMe namespaces before rescan",
                    ),
                    nvme_list_subsystems_command("inspect NVMe subsystem paths before rescan"),
                    nvme_namespace_rescan_command(controller),
                    nvme_list_namespaces_command(controller, "verify NVMe namespaces after rescan"),
                    nvme_list_subsystems_command("verify NVMe subsystem paths after rescan"),
                ],
                vec![
                    "verify namespace inventory before exposing refreshed devices to consumers"
                        .to_string(),
                    "use grow when controller-side namespace capacity changed".to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("nvmeNamespaces") => {
            let controller = nvme_controller_target(action);
            (
                vec![
                    nvme_list_namespaces_command(controller, "inspect NVMe namespaces before rescan"),
                    nvme_list_subsystems_command("inspect NVMe subsystem paths before growth rescan"),
                    nvme_namespace_rescan_command(controller),
                    nvme_list_namespaces_command(controller, "verify NVMe namespaces after rescan"),
                    nvme_list_subsystems_command("verify NVMe subsystem paths after growth rescan"),
                ],
                vec![
                    "perform controller-side namespace resize before host rescan".to_string(),
                    "grow dependent partitions, volumes, or filesystems only after the namespace reports the new size"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Attach if collection == Some("nvmeNamespaces") => {
            let controller = nvme_controller_target(action);
            let namespace_id = action.context.namespace_id.as_deref();
            let controllers = action.context.controllers.as_deref();
            (
                vec![
                    nvme_list_namespaces_command(
                        controller,
                        "inspect NVMe namespace inventory before attach",
                    ),
                    nvme_list_subsystems_command("inspect NVMe subsystem paths before attach"),
                    nvme_attach_namespace_command(controller, namespace_id, controllers),
                    nvme_namespace_rescan_command(controller),
                    nvme_list_namespaces_command(
                        controller,
                        "verify NVMe namespace inventory after attach",
                    ),
                    nvme_list_subsystems_command("verify NVMe subsystem paths after attach"),
                ],
                vec![
                    "attach preserves the namespace and only changes controller visibility"
                        .to_string(),
                    "verify namespace id and controller attachment before exposing consumers"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("partitions") => {
            let partition_target = partition_target_path(action);
            let disk = action.context.device.as_deref();
            let partition_number = action.context.partition_number.as_deref();
            let desired_end = action
                .context
                .end
                .as_deref()
                .or(action.context.desired_size.as_deref());
            (
                vec![
                    disk_nix_inspect_command(
                        partition_target,
                        "<partition>",
                        "partition path",
                        "inspect partition, consumers, and backing device before growth",
                    ),
                    partition_grow_command(disk, partition_number, desired_end),
                    command(
                        ["partprobe"],
                        true,
                        "ask the kernel to reread partition tables after the geometry change",
                    ),
                    partition_table_reread_command(disk),
                ],
                vec![
                    "confirm the backing disk or LUN has already grown".to_string(),
                    "pause dependent consumers when the kernel cannot reread an active table"
                        .to_string(),
                    "resize LUKS, LVM, and filesystems only after the partition reports the new size"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("disks") || collection == Some("partitions") => {
            let disk = partition_rescan_disk(action);
            (
                vec![
                    disk_nix_inspect_command(
                        disk,
                        "<disk>",
                        "disk path",
                        "inspect disk identity before partition-table rescan",
                    ),
                    partition_probe_command(disk),
                    partition_table_reread_command(disk),
                    disk_parted_machine_list_command(
                        disk,
                        "verify the disk partition table after reread",
                    ),
                ],
                vec![
                    "use grow or create when partition geometry changes are still required"
                        .to_string(),
                    "pause dependent consumers when an active kernel table cannot be reread"
                        .to_string(),
                    "verify stable by-id and by-partuuid paths before growing consumers"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Grow => {
            let target = target.unwrap_or("<target>");
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect current target state before growth",
                    ),
                    command_with_readiness(
                        ["<grow-storage-object-tool>", target],
                        true,
                        CommandReadiness::NeedsDomainImplementation,
                        ["grow tool", "desired size"],
                        "grow the storage object with the target-domain-specific command",
                    ),
                ],
                vec![
                    "select the grow command from the target storage domain and desired size"
                        .to_string(),
                ],
                true,
            )
        }
        _ => return None,
    })
}
