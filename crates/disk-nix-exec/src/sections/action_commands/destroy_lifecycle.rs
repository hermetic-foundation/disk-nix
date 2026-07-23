fn destroy_lifecycle_action_commands(
    action: &PlannedAction,
    collection: Option<&str>,
    target: Option<&str>,
    cache_target: Option<&str>,
) -> Option<ActionCommandResult> {
    let _ = cache_target;
    Some(match action.operation {
        Operation::Create => (
            vec![command_with_readiness(
                ["<create-storage-object-tool>", "<target>"],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["create tool", "target"],
                "create the requested storage object",
            )],
            vec![
                "creation commands require target-kind-specific arguments from the desired spec"
                    .to_string(),
            ],
            true,
        ),
        Operation::Destroy if collection == Some("btrfsSubvolumes") => {
            let target = target.unwrap_or("<btrfs-subvolume-path>");
            (
                vec![
                    command(
                        ["btrfs", "subvolume", "show", target],
                        false,
                        "inspect Btrfs subvolume metadata before deletion",
                    ),
                    command(
                        ["btrfs", "subvolume", "delete", target],
                        true,
                        "delete the reviewed Btrfs subvolume",
                    ),
                ],
                vec![
                    "take a read-only snapshot before deletion when data may be needed".to_string(),
                    "unmount or redirect consumers before deleting the subvolume".to_string(),
                ],
                true,
            )
        }
        Operation::Rename if collection == Some("btrfsSubvolumes") => {
            let target = target.unwrap_or("<btrfs-subvolume-path>");
            let rename_to = action
                .context
                .rename_to
                .as_deref()
                .unwrap_or("<new-btrfs-subvolume-path>");
            (
                vec![
                    command(
                        ["btrfs", "subvolume", "show", target],
                        false,
                        "inspect Btrfs subvolume before rename",
                    ),
                    command(
                        ["mv", "--", target, rename_to],
                        true,
                        "rename the reviewed Btrfs subvolume path",
                    ),
                ],
                vec![
                    "update mounts, send/receive jobs, qgroups, and snapshots after rename"
                        .to_string(),
                    "validate consumers on the renamed subvolume before deleting the old path"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("btrfsQgroups") => {
            let qgroup_id = action.context.name.as_deref().unwrap_or("<qgroupid>");
            let target_path = btrfs_qgroup_target_path(action.context.target.as_deref(), qgroup_id);
            let target = target_path.unwrap_or("<btrfs-filesystem-path>");
            let inspect_command = match target_path {
                Some(target) => command(
                    ["btrfs", "qgroup", "show", "--raw", "-reF", target],
                    false,
                    "inspect Btrfs qgroup inventory before destruction",
                ),
                None => command_with_readiness(
                    ["btrfs", "qgroup", "show", "--raw", "-reF", target],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["mounted Btrfs filesystem path"],
                    "inspect Btrfs qgroups after selecting the mounted filesystem path",
                ),
            };
            let destroy_command = match target_path {
                Some(target) => command_vec(
                    vec![
                        "btrfs".to_string(),
                        "qgroup".to_string(),
                        "destroy".to_string(),
                        qgroup_id.to_string(),
                        target.to_string(),
                    ],
                    true,
                    "destroy the reviewed Btrfs qgroup",
                ),
                None => command_vec_with_readiness(
                    vec![
                        "btrfs".to_string(),
                        "qgroup".to_string(),
                        "destroy".to_string(),
                        qgroup_id.to_string(),
                        target.to_string(),
                    ],
                    true,
                    CommandReadiness::NeedsDomainImplementation,
                    ["mounted Btrfs filesystem path"],
                    "destroy the Btrfs qgroup after selecting the mounted filesystem path",
                ),
            };
            (
                vec![inspect_command, destroy_command],
                vec![
                    "verify no subvolume still depends on the qgroup limit".to_string(),
                    "preserve qgroup accounting policy elsewhere before deleting the qgroup"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("zvols") => {
            let target = target.unwrap_or("<zvol>");
            (
                vec![
                    command(
                        ["zfs", "list", "-H", "-p", "-t", "volume", target],
                        false,
                        "inspect zvol metadata before destruction",
                    ),
                    command(
                        ["zfs", "destroy", target],
                        true,
                        "destroy the reviewed zvol after consumers are detached",
                    ),
                ],
                vec![
                    "take a snapshot or clone before destruction when rollback is required"
                        .to_string(),
                    "detach LUN, VM, or filesystem consumers before destroying the zvol"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("pools") => {
            let target = zfs_pool_command_target(action, target);
            (
                vec![
                    command(
                        ["zpool", "status", "-P", target],
                        false,
                        "inspect pool health and dependent vdevs before destruction",
                    ),
                    command(
                        ["zpool", "destroy", target],
                        true,
                        "destroy the reviewed ZFS pool after datasets and consumers are migrated",
                    ),
                ],
                vec![
                    "take recursive snapshots or verified backups before destroying the pool"
                        .to_string(),
                    "export the pool instead of destroying it when moving it to another host"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Import if collection == Some("pools") => {
            let target = zfs_pool_command_target(action, target);
            (
                vec![
                    command(
                        ["zpool", "import"],
                        false,
                        "inspect importable ZFS pools before import",
                    ),
                    zfs_pool_import_command(target, action.context.read_only.unwrap_or(false)),
                ],
                vec![
                    "verify the pool identity, hostid, cachefile, mountpoints, and encryption keys before import"
                        .to_string(),
                    "use readOnly=true first when validating a moved or recovered pool".to_string(),
                ],
                true,
            )
        }
        Operation::Export if collection == Some("pools") => {
            let target = zfs_pool_command_target(action, target);
            (
                vec![
                    command(
                        ["zpool", "status", "-P", target],
                        false,
                        "inspect pool health and active consumers before export",
                    ),
                    command(
                        ["zpool", "export", target],
                        true,
                        "export the reviewed ZFS pool without deleting data",
                    ),
                ],
                vec![
                    "stop mount, share, LUN, VM, and service consumers before export".to_string(),
                    "export instead of destroying a pool that will be moved or recovered elsewhere"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("datasets") => {
            let target = target.unwrap_or("<zfs-dataset>");
            (
                vec![
                    command(
                        ["zfs", "list", "-H", "-p", "-r", target],
                        false,
                        "inspect dataset descendants before destruction",
                    ),
                    command(
                        ["zfs", "destroy", target],
                        true,
                        "destroy the reviewed ZFS dataset after snapshots and consumers are handled",
                    ),
                ],
                vec![
                    "take a recursive snapshot or clone before destruction when rollback is required"
                        .to_string(),
                    "unmount dependents and review child datasets before destroying the dataset"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rename if collection == Some("datasets") || collection == Some("zvols") => {
            let target = target.unwrap_or("<zfs-dataset>");
            let rename_to = action
                .context
                .rename_to
                .as_deref()
                .unwrap_or("<new-zfs-name>");
            (
                vec![
                    command(
                        ["zfs", "list", "-H", "-p", target],
                        false,
                        "inspect ZFS object before rename",
                    ),
                    command(
                        ["zfs", "rename", target, rename_to],
                        true,
                        "rename the reviewed ZFS dataset or zvol",
                    ),
                ],
                vec![
                    "update mountpoints, shares, LUN mappings, and dependent services to the new name"
                        .to_string(),
                    "validate consumers on the renamed object before destroying any old path"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Promote if collection == Some("datasets") || collection == Some("zvols") => {
            let target = target.unwrap_or("<zfs-clone>");
            (
                vec![
                    command(
                        ["zfs", "get", "-H", "-o", "value", "origin", target],
                        false,
                        "inspect ZFS clone origin before promotion",
                    ),
                    command(
                        ["zfs", "promote", target],
                        true,
                        "promote the reviewed ZFS clone",
                    ),
                ],
                vec![
                    "promotion changes clone dependency ownership; review dependent snapshots first"
                        .to_string(),
                    "validate consumers on the promoted clone before destroying or renaming the origin"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("snapshots") => {
            let snapshot = action.context.name.as_deref().unwrap_or("<snapshot>");
            let source = action
                .context
                .target
                .as_deref()
                .unwrap_or("<snapshot-source>");
            if is_zfs_snapshot_name(snapshot) {
                (
                    vec![
                        command(
                            ["zfs", "list", "-t", "snapshot", "-H", "-p", snapshot],
                            false,
                            "inspect ZFS snapshot before destruction",
                        ),
                        command(
                            ["zfs", "destroy", snapshot],
                            true,
                            "destroy the reviewed ZFS snapshot recovery point",
                        ),
                    ],
                    vec![
                        "verify the snapshot is no longer needed as a recovery point".to_string(),
                        "hold, rename, clone, or replicate the snapshot before destruction when retention is uncertain"
                            .to_string(),
                    ],
                    true,
                )
            } else if is_btrfs_snapshot_pair(source, snapshot) {
                (
                    vec![
                        command(
                            ["btrfs", "subvolume", "show", snapshot],
                            false,
                            "inspect Btrfs snapshot subvolume before deletion",
                        ),
                        command(
                            ["btrfs", "subvolume", "delete", snapshot],
                            true,
                            "delete the reviewed Btrfs snapshot subvolume",
                        ),
                    ],
                    vec![
                        "verify the snapshot is no longer needed as a recovery point".to_string(),
                        "keep or clone the read-only snapshot before deletion when retention is uncertain"
                            .to_string(),
                    ],
                    true,
                )
            } else {
                (
                    vec![command_with_readiness(
                        ["<snapshot-destroy-tool>", source, snapshot],
                        true,
                        CommandReadiness::NeedsDomainImplementation,
                        ["snapshot destroy tool"],
                        "destroy the snapshot with zfs, btrfs, lvm, or the target-specific tool",
                    )],
                    vec![
                        "snapshot destruction command is only rendered for unambiguous ZFS names or Btrfs absolute paths"
                            .to_string(),
                    ],
                    true,
                )
            }
        }
        Operation::Rename if collection == Some("snapshots") => {
            let snapshot = action.context.name.as_deref().unwrap_or("<snapshot>");
            let rename_to = action
                .context
                .rename_to
                .as_deref()
                .unwrap_or("<new-snapshot-name>");
            if is_zfs_snapshot_name(snapshot) {
                (
                    vec![
                        command(
                            ["zfs", "list", "-t", "snapshot", "-H", "-p", snapshot],
                            false,
                            "inspect ZFS snapshot before rename",
                        ),
                        command(
                            ["zfs", "rename", snapshot, rename_to],
                            true,
                            "rename the reviewed ZFS snapshot recovery point",
                        ),
                    ],
                    vec![
                        "update retention, replication, and rollback references to the new snapshot name"
                            .to_string(),
                    ],
                    true,
                )
            } else if snapshot.starts_with('/') {
                (
                    vec![
                        command(
                            ["btrfs", "subvolume", "show", snapshot],
                            false,
                            "inspect Btrfs snapshot subvolume before rename",
                        ),
                        command(
                            ["mv", "--", snapshot, rename_to],
                            true,
                            "rename the reviewed Btrfs snapshot subvolume path",
                        ),
                    ],
                    vec![
                        "update retention and restore references to the renamed snapshot path"
                            .to_string(),
                    ],
                    true,
                )
            } else {
                (
                    vec![command_with_readiness(
                        ["<snapshot-rename-tool>", snapshot, rename_to],
                        true,
                        CommandReadiness::NeedsDomainImplementation,
                        ["ZFS snapshot name or Btrfs snapshot path"],
                        "rename the snapshot after selecting the target-specific snapshot tool",
                    )],
                    vec![
                        "snapshot rename command is only rendered for unambiguous ZFS snapshot names or Btrfs absolute paths"
                            .to_string(),
                    ],
                    true,
                )
            }
        }
        Operation::Destroy if collection == Some("lvmSnapshots") => {
            let target = target.unwrap_or("<lvm-snapshot>");
            (
                vec![
                    command(
                        ["lvs", "--reportformat", "json", target],
                        false,
                        "inspect LVM snapshot before removal",
                    ),
                    command(
                        ["lvremove", "--yes", target],
                        true,
                        "remove the reviewed LVM snapshot",
                    ),
                ],
                vec![
                    "verify the snapshot is no longer needed as a recovery point".to_string(),
                    "prefer a fresh snapshot or backup before deleting old snapshots".to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("volumes") => {
            let target = lvm_volume_target_path(target);
            (
                vec![
                    lvm_lvs_report_command(target, None, "inspect logical volume before removal"),
                    lvm_lvremove_command(
                        target,
                        "<logical-volume>",
                        "target in volume-group/logical-volume form",
                        "remove the reviewed logical volume after backups and consumers are verified",
                    ),
                ],
                vec![
                    "snapshot or migrate data before removing the logical volume".to_string(),
                    "unmount filesystems and deactivate dependent mappings before lvremove"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rename if collection == Some("volumes") || collection == Some("thinPools") => {
            let target = lvm_volume_target_path(target);
            let rename_to = action.context.rename_to.as_deref();
            (
                vec![
                    lvm_lvs_report_command(target, None, "inspect logical volume before rename"),
                    lvm_lvrename_command(
                        target,
                        rename_to,
                        "<logical-volume>",
                        "target in volume-group/logical-volume form",
                        "new logical volume name or path",
                        "rename the reviewed logical volume",
                    ),
                ],
                vec![
                    "update filesystems, crypttab, mounts, LUN exports, and services after rename"
                        .to_string(),
                    "keep the old declaration out of destructive mode until consumers are validated"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Activate | Operation::Deactivate
            if collection == Some("volumes")
                || collection == Some("thinPools")
                || collection == Some("lvmSnapshots") =>
        {
            let target = lvm_volume_target_path(target);
            let (flag, verb, placeholder, input) = match collection {
                Some("thinPools") => (
                    "y",
                    "activate",
                    "<thin-pool>",
                    "target in volume-group/thin-pool form",
                ),
                Some("lvmSnapshots") => (
                    "y",
                    "activate",
                    "<lvm-snapshot>",
                    "target in volume-group/snapshot form",
                ),
                _ => (
                    "y",
                    "activate",
                    "<logical-volume>",
                    "target in volume-group/logical-volume form",
                ),
            };
            let (flag, verb) = if action.operation == Operation::Deactivate {
                ("n", "deactivate")
            } else {
                (flag, verb)
            };
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        None,
                        "inspect logical volume before activation change",
                    ),
                    lvm_lvchange_activate_command(target, flag, placeholder, input),
                ],
                vec![
                    format!(
                        "{verb} only after filesystem, mapping, mount, and service consumers are reviewed"
                    ),
                    "activation state changes do not create or delete LV data".to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("lvmSnapshots") => {
            let target = lvm_volume_target_path(target);
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        Some("lv_name,origin,lv_attr,data_percent,metadata_percent,lv_size"),
                        "refresh LVM snapshot origin, attributes, and COW usage",
                    ),
                    command(
                        ["disk-nix", "inspect", target.unwrap_or("<lvm-snapshot>")],
                        false,
                        "inspect modeled LVM snapshot graph relationships after status refresh",
                    ),
                ],
                vec![
                    "use rollback only after reviewing origin and snapshot state".to_string(),
                    "activate the snapshot for recovery inspection before destructive removal"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("thinPools") => {
            let target = lvm_volume_target_path(target);
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        Some("lv_name,lv_attr,data_percent,metadata_percent"),
                        "inspect thin pool before removal",
                    ),
                    lvm_lvremove_command(
                        target,
                        "<thin-pool>",
                        "target in volume-group/thin-pool form",
                        "remove the reviewed thin pool after thin volumes and consumers are migrated",
                    ),
                ],
                vec![
                    "migrate or remove thin volumes before removing the thin pool".to_string(),
                    "unmount filesystems and deactivate mappings that depend on thin volumes"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            (
                vec![
                    command(
                        ["vgs", "--reportformat", "json", target],
                        false,
                        "inspect volume group before removal",
                    ),
                    command(
                        ["vgremove", "--yes", target],
                        true,
                        "remove the reviewed LVM volume group after all consumers are migrated",
                    ),
                ],
                vec![
                    "remove or migrate logical volumes before removing the volume group"
                        .to_string(),
                    "verify no filesystems, mappings, or services still reference the VG"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Import if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            (
                vec![
                    command(
                        ["pvs", "--reportformat", "json"],
                        false,
                        "inspect physical volumes and exported VG metadata before import",
                    ),
                    command(
                        ["vgimport", target],
                        true,
                        "import the reviewed LVM volume group without recreating it",
                    ),
                ],
                vec![
                    "verify PV identities, VG UUID, and metadata backups before vgimport"
                        .to_string(),
                    "activate logical volumes and mount consumers only after the VG is verified"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Export if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            (
                vec![
                    command(
                        ["vgs", "--reportformat", "json", target],
                        false,
                        "inspect volume group before export",
                    ),
                    command(
                        ["vgexport", target],
                        true,
                        "export the reviewed LVM volume group without deleting data",
                    ),
                ],
                vec![
                    "deactivate logical volumes and stop mount, mapping, and service consumers before vgexport"
                        .to_string(),
                    "export instead of removing a VG that will be moved to another host"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rename if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            let rename_to = action
                .context
                .rename_to
                .as_deref()
                .unwrap_or("<new-volume-group>");
            (
                vec![
                    command(
                        ["vgs", "--reportformat", "json", target],
                        false,
                        "inspect volume group before rename",
                    ),
                    command(
                        ["vgrename", target, rename_to],
                        true,
                        "rename the reviewed volume group",
                    ),
                ],
                vec![
                    "update every LV path, initrd reference, mount, and service before reboot"
                        .to_string(),
                    "validate boot and activation with the renamed volume group before cleanup"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Activate | Operation::Deactivate if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            let (flag, verb) = if action.operation == Operation::Deactivate {
                ("n", "deactivate")
            } else {
                ("y", "activate")
            };
            (
                vec![
                    command(
                        ["vgs", "--reportformat", "json", target],
                        false,
                        "inspect volume group before activation change",
                    ),
                    command(
                        ["vgchange", "--activate", flag, target],
                        true,
                        if flag == "y" {
                            "activate the reviewed LVM volume group"
                        } else {
                            "deactivate the reviewed LVM volume group without deleting data"
                        },
                    ),
                ],
                vec![
                    format!(
                        "{verb} the VG only after PV membership and dependent consumers are reviewed"
                    ),
                    "volume group activation changes do not create or remove VG metadata"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("physicalVolumes") => {
            let target = lvm_physical_volume_target(action);
            (
                vec![
                    lvm_physical_volume_inspect_command(target),
                    lvm_physical_volume_remove_command(target),
                ],
                vec![
                    "run pvmove and vgreduce before pvremove when the PV is in a VG".to_string(),
                    "keep the device available for recovery until backups are verified".to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("lvmCaches") => {
            let target = lvm_volume_target_path(target);
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        Some("lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent"),
                        "inspect LVM cache dirty state before removal",
                    ),
                    lvm_cache_uncache_command(target),
                ],
                vec![
                    "switch writeback caches to writethrough before removing cache state"
                        .to_string(),
                    "verify the origin LV after lvconvert --uncache before removing cache media"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("vdoVolumes") => {
            let target = target.unwrap_or("<vdo-volume>");
            (
                vec![
                    command(
                        ["vdo", "status", "--name", target],
                        false,
                        "inspect VDO volume before removal",
                    ),
                    command(
                        ["vdo", "remove", "--name", target],
                        true,
                        "remove the reviewed VDO volume after consumers are migrated",
                    ),
                ],
                vec![
                    "migrate data away from the VDO device before removal".to_string(),
                    "unmount filesystems and deactivate mappings that reference the VDO device"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Stop if collection == Some("vdoVolumes") => {
            let target = target.unwrap_or("<vdo-volume>");
            (
                vec![
                    command(
                        ["vdo", "status", "--name", target],
                        false,
                        "inspect VDO volume before stop",
                    ),
                    command(
                        ["vdo", "stop", "--name", target],
                        true,
                        "stop the existing VDO volume after consumers are inactive",
                    ),
                ],
                vec![
                    "unmount filesystems and deactivate mappings that reference the VDO device"
                        .to_string(),
                    "prefer stop over remove when preserving VDO metadata for later restart"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("nvmeNamespaces") => {
            let controller = nvme_controller_target(action);
            let namespace_id = action.context.namespace_id.as_deref();
            let controllers = action.context.controllers.as_deref();
            (
                vec![
                    nvme_list_namespaces_command(
                        controller,
                        "inspect NVMe namespace inventory before deletion",
                    ),
                    nvme_list_subsystems_command("inspect NVMe subsystem paths before deletion"),
                    nvme_detach_namespace_command(controller, namespace_id, controllers),
                    nvme_delete_namespace_command(controller, namespace_id),
                    nvme_namespace_rescan_command(controller),
                    nvme_list_subsystems_command("verify NVMe subsystem paths after deletion"),
                ],
                vec![
                    "detach namespace consumers and migrate data before delete-ns".to_string(),
                    "prefer detach without delete when target-side namespace data must remain"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Detach if collection == Some("nvmeNamespaces") => {
            let controller = nvme_controller_target(action);
            let namespace_id = action.context.namespace_id.as_deref();
            let controllers = action.context.controllers.as_deref();
            (
                vec![
                    nvme_list_namespaces_command(
                        controller,
                        "inspect NVMe namespace inventory before detach",
                    ),
                    nvme_list_subsystems_command("inspect NVMe subsystem paths before detach"),
                    nvme_detach_namespace_command(controller, namespace_id, controllers),
                    nvme_namespace_rescan_command(controller),
                    nvme_list_namespaces_command(
                        controller,
                        "verify NVMe namespace inventory after detach",
                    ),
                    nvme_list_subsystems_command("verify NVMe subsystem paths after detach"),
                ],
                vec![
                    "detach removes controller access without deleting the namespace".to_string(),
                    "unmount filesystems and deactivate dependent mappings before detach"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("loopDevices") => {
            let target = loop_device_target_path(action);
            (
                vec![
                    loop_device_list_command(
                        target,
                        "inspect loop device and backing file before detach",
                    ),
                    loop_device_detach_command(target),
                ],
                vec![
                    "unmount filesystems and deactivate mappings before detach".to_string(),
                    "verify the backing file remains available after detach".to_string(),
                ],
                true,
            )
        }
        _ => return None,
    })
}
