#[test]
fn dependency_order_reports_explicit_edges_for_layered_block_growth() {
    let plan = plan_from_json_bytes(
        br#"{
              "backingFiles": {
                "/var/lib/images/root.img": {
                  "operation": "grow",
                  "desiredSize": "32GiB"
                }
              },
              "loopDevices": {
                "/dev/loop7": {
                  "operation": "grow",
                  "device": "/var/lib/images/root.img"
                }
              },
              "filesystems": {
                "root": {
                  "operation": "grow",
                  "device": "/dev/loop7",
                  "desiredSize": "100%"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    let backing = plan
        .dependency_order
        .iter()
        .find(|order| order.action_id == "backingfiles:/var/lib/images/root.img:grow")
        .expect("backing file dependency order entry exists");
    let loop_device = plan
        .dependency_order
        .iter()
        .find(|order| order.action_id == "loopdevices:/dev/loop7:grow")
        .expect("loop device dependency order entry exists");
    let filesystem = plan
        .dependency_order
        .iter()
        .find(|order| order.action_id == "filesystem:root:inspect")
        .expect("filesystem dependency order entry exists");

    assert!(backing.depends_on.is_empty());
    assert_eq!(
        backing.unblocks,
        vec!["loopdevices:/dev/loop7:grow".to_string()]
    );
    assert_eq!(
        backing.recovery_depends_on,
        vec!["loopdevices:/dev/loop7:grow".to_string()]
    );
    assert!(backing.recovery_unblocks.is_empty());
    assert_eq!(
        loop_device.depends_on,
        vec!["backingfiles:/var/lib/images/root.img:grow".to_string()]
    );
    assert_eq!(
        loop_device.unblocks,
        vec!["filesystem:root:inspect".to_string()]
    );
    assert_eq!(
        loop_device.recovery_depends_on,
        vec!["filesystem:root:inspect".to_string()]
    );
    assert_eq!(
        loop_device.recovery_unblocks,
        vec!["backingfiles:/var/lib/images/root.img:grow".to_string()]
    );
    assert_eq!(
        filesystem.depends_on,
        vec!["loopdevices:/dev/loop7:grow".to_string()]
    );
    assert!(filesystem.unblocks.is_empty());
    assert!(filesystem.recovery_depends_on.is_empty());
    assert_eq!(
        filesystem.recovery_unblocks,
        vec!["loopdevices:/dev/loop7:grow".to_string()]
    );
    assert!(loop_device
        .notes
        .iter()
        .any(|note| note.contains("explicit dependency edge")));
    assert!(loop_device.notes.iter().any(|note| {
        note.contains("recovery review waits for dependent action")
            && note.contains("filesystem:root:inspect")
    }));
}

#[test]
fn dependency_order_reports_explicit_edges_for_pool_dataset_snapshot_layers() {
    let plan = plan_from_json_bytes(
        br#"{
              "pools": {
                "tank": {
                  "operation": "import"
                }
              },
              "datasets": {
                "tank/home": {
                  "operation": "create"
                }
              },
              "snapshots": {
                "home-before": {
                  "target": "tank/home",
                  "name": "tank/home@before"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    let pool = plan
        .dependency_order
        .iter()
        .find(|order| order.action_id == "pools:tank:import")
        .expect("pool dependency order entry exists");
    let dataset = plan
        .dependency_order
        .iter()
        .find(|order| order.action_id == "datasets:tank/home:create")
        .expect("dataset dependency order entry exists");
    let snapshot = plan
        .dependency_order
        .iter()
        .find(|order| order.action_id == "snapshot:home-before:create")
        .expect("snapshot dependency order entry exists");

    assert_eq!(pool.unblocks, vec!["datasets:tank/home:create".to_string()]);
    assert_eq!(dataset.depends_on, vec!["pools:tank:import".to_string()]);
    assert_eq!(
        dataset.unblocks,
        vec!["snapshot:home-before:create".to_string()]
    );
    assert_eq!(
        snapshot.depends_on,
        vec!["datasets:tank/home:create".to_string()]
    );
}

#[test]
fn plan_warns_for_shrink_and_disabled_preservation() {
    let plan = plan_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "home": {
                    "mountpoint": "/home",
                    "fsType": "xfs",
                    "resizePolicy": "shrink-allowed",
                    "preserveData": false
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 2);
    assert_eq!(plan.summary.destructive_count, 1);
    assert_eq!(plan.summary.potential_data_loss_count, 0);
    assert_eq!(plan.summary.offline_required_count, 0);
    assert_eq!(plan.summary.unsupported_count, 1);
    assert!(plan.actions.iter().any(|action| {
        action.operation == Operation::Shrink
            && action.risk == RiskClass::Unsupported
            && action
                .advice
                .as_ref()
                .is_some_and(|advice| advice.summary.contains("XFS"))
    }));
}

#[test]
fn plan_keeps_ext_shrink_as_potential_data_loss() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "home": {
                  "mountpoint": "/home",
                  "fsType": "ext4",
                  "resizePolicy": "shrink-allowed"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 1);
    assert_eq!(plan.summary.potential_data_loss_count, 1);
    assert_eq!(plan.summary.unsupported_count, 0);
    assert_eq!(plan.actions[0].risk, RiskClass::PotentialDataLoss);
    assert_eq!(plan.actions[0].context.fs_type.as_deref(), Some("ext4"));
    assert_eq!(plan.actions[0].context.mountpoint.as_deref(), Some("/home"));
}

#[test]
fn plan_carries_filesystem_device_for_lifecycle_actions() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "home": {
                  "mountpoint": "/home",
                  "device": "/dev/disk/by-label/home",
                  "fsType": "ext4",
                  "resizePolicy": "shrink-allowed",
                  "desiredSize": "100G"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 1);
    assert_eq!(plan.actions[0].operation, Operation::Shrink);
    assert_eq!(
        plan.actions[0].context.device.as_deref(),
        Some("/dev/disk/by-label/home")
    );
    assert_eq!(plan.actions[0].context.target.as_deref(), Some("/home"));
    assert_eq!(
        plan.actions[0].context.desired_size.as_deref(),
        Some("100G")
    );
}

#[test]
fn plan_filesystem_properties_keep_filesystem_context() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "data": {
                  "mountpoint": "/data",
                  "device": "/dev/disk/by-label/data",
                  "fsType": "btrfs",
                  "properties": {
                    "label": "bulk-data",
                    "compression": "zstd"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 3);
    assert_eq!(plan.summary.unsupported_count, 1);
    let action = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:data:set-property:label")
        .expect("filesystem label property action should exist");

    assert_eq!(action.operation, Operation::SetProperty);
    assert_eq!(action.risk, RiskClass::Safe);
    assert_eq!(action.context.target.as_deref(), Some("/data"));
    assert_eq!(
        action.context.device.as_deref(),
        Some("/dev/disk/by-label/data")
    );
    assert_eq!(action.context.fs_type.as_deref(), Some("btrfs"));
    assert_eq!(action.context.mountpoint.as_deref(), Some("/data"));
    assert_eq!(action.context.property.as_deref(), Some("label"));
    assert_eq!(action.context.property_value.as_deref(), Some("bulk-data"));

    let unsupported = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:data:set-property:compression")
        .expect("unsupported filesystem property action should exist");
    assert_eq!(unsupported.operation, Operation::SetProperty);
    assert_eq!(unsupported.risk, RiskClass::Unsupported);
    assert!(unsupported.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("ZFS dataset"))
    }));
}

#[test]
fn plan_accepts_xfs_filesystem_label_property() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "scratch": {
                  "mountpoint": "/scratch",
                  "device": "/dev/disk/by-label/scratch",
                  "fsType": "xfs",
                  "properties": {
                    "xfs.label": "scratch-new",
                    "xfs.reflink": "1"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 3);
    assert_eq!(plan.summary.unsupported_count, 1);
    let label = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:scratch:set-property:xfs.label")
        .expect("XFS label property action should exist");

    assert_eq!(label.operation, Operation::SetProperty);
    assert_eq!(label.risk, RiskClass::Safe);
    assert_eq!(label.context.target.as_deref(), Some("/scratch"));
    assert_eq!(
        label.context.device.as_deref(),
        Some("/dev/disk/by-label/scratch")
    );
    assert_eq!(label.context.fs_type.as_deref(), Some("xfs"));
    assert_eq!(label.context.property_value.as_deref(), Some("scratch-new"));

    let unsupported = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:scratch:set-property:xfs.reflink")
        .expect("unsupported XFS property action should exist");
    assert_eq!(unsupported.risk, RiskClass::Unsupported);
}

#[test]
fn plan_accepts_fat_label_and_volume_id_properties() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "efi": {
                  "mountpoint": "/boot",
                  "device": "/dev/disk/by-partlabel/EFI",
                  "fsType": "vfat",
                  "properties": {
                    "vfat.label": "NIXBOOT",
                    "vfat.uuid": "A1B2-C3D4",
                    "fat.volume-id": "not-a-fat-id"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 4);
    assert_eq!(plan.summary.offline_required_count, 1);
    assert_eq!(plan.summary.unsupported_count, 1);

    let label = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:efi:set-property:vfat.label")
        .expect("FAT label property action should exist");
    assert_eq!(label.operation, Operation::SetProperty);
    assert_eq!(label.risk, RiskClass::Safe);
    assert_eq!(label.context.fs_type.as_deref(), Some("vfat"));
    assert_eq!(label.context.property_value.as_deref(), Some("NIXBOOT"));

    let volume_id = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:efi:set-property:vfat.uuid")
        .expect("FAT volume ID property action should exist");
    assert_eq!(volume_id.risk, RiskClass::OfflineRequired);
    assert!(volume_id.advice.as_ref().is_some_and(|advice| {
        advice.summary.contains("UUID")
            && advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("NixOS fileSystems"))
    }));

    let invalid_volume_id = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:efi:set-property:fat.volume-id")
        .expect("invalid FAT volume ID property action should exist");
    assert_eq!(invalid_volume_id.risk, RiskClass::Unsupported);
    assert!(invalid_volume_id.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("8-hex-digit FAT volume ID"))
    }));
}

#[test]
fn plan_accepts_ntfs_label_and_volume_serial_properties() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "windows": {
                  "mountpoint": "/mnt/windows",
                  "device": "/dev/disk/by-label/Windows",
                  "fsType": "ntfs",
                  "properties": {
                    "ntfs.label": "Windows",
                    "ntfs.uuid": "01234567-89abcdef",
                    "ntfs.volume-serial": "not-a-serial"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 4);
    assert_eq!(plan.summary.offline_required_count, 1);
    assert_eq!(plan.summary.unsupported_count, 1);

    let label = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:windows:set-property:ntfs.label")
        .expect("NTFS label property action should exist");
    assert_eq!(label.operation, Operation::SetProperty);
    assert_eq!(label.risk, RiskClass::Safe);
    assert_eq!(label.context.fs_type.as_deref(), Some("ntfs"));
    assert_eq!(label.context.property_value.as_deref(), Some("Windows"));

    let serial = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:windows:set-property:ntfs.uuid")
        .expect("NTFS serial property action should exist");
    assert_eq!(serial.risk, RiskClass::OfflineRequired);
    assert!(serial.advice.as_ref().is_some_and(|advice| {
        advice.summary.contains("UUID")
            && advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("NixOS fileSystems"))
    }));

    let invalid_serial = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:windows:set-property:ntfs.volume-serial")
        .expect("invalid NTFS serial property action should exist");
    assert_eq!(invalid_serial.risk, RiskClass::Unsupported);
    assert!(invalid_serial.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("16-hex-digit NTFS volume serial"))
    }));
}

#[test]
fn plan_accepts_exfat_label_and_volume_serial_properties() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "shared": {
                  "mountpoint": "/mnt/shared",
                  "device": "/dev/disk/by-label/Shared",
                  "fsType": "exfat",
                  "properties": {
                    "exfat.label": "Shared",
                    "exfat.uuid": "A1B2-C3D4",
                    "exfat.volume-serial": "not-a-serial"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 4);
    assert_eq!(plan.summary.offline_required_count, 1);
    assert_eq!(plan.summary.unsupported_count, 1);

    let label = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:shared:set-property:exfat.label")
        .expect("exFAT label property action should exist");
    assert_eq!(label.operation, Operation::SetProperty);
    assert_eq!(label.risk, RiskClass::Safe);
    assert_eq!(label.context.fs_type.as_deref(), Some("exfat"));
    assert_eq!(label.context.property_value.as_deref(), Some("Shared"));

    let serial = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:shared:set-property:exfat.uuid")
        .expect("exFAT serial property action should exist");
    assert_eq!(serial.risk, RiskClass::OfflineRequired);
    assert!(serial.advice.as_ref().is_some_and(|advice| {
        advice.summary.contains("UUID")
            && advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("NixOS fileSystems"))
    }));

    let invalid_serial = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:shared:set-property:exfat.volume-serial")
        .expect("invalid exFAT serial property action should exist");
    assert_eq!(invalid_serial.risk, RiskClass::Unsupported);
    assert!(invalid_serial.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("8-hex-digit exFAT volume serial"))
    }));
}

#[test]
fn plan_accepts_f2fs_label_property() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "mobile": {
                  "mountpoint": "/mnt/mobile",
                  "device": "/dev/disk/by-label/mobile",
                  "fsType": "f2fs",
                  "properties": {
                    "f2fs.label": "mobile-new",
                    "f2fs.uuid": "11111111-2222-3333-4444-555555555555"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 3);
    assert_eq!(plan.summary.unsupported_count, 1);

    let label = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:mobile:set-property:f2fs.label")
        .expect("F2FS label property action should exist");
    assert_eq!(label.operation, Operation::SetProperty);
    assert_eq!(label.risk, RiskClass::Safe);
    assert_eq!(label.context.fs_type.as_deref(), Some("f2fs"));
    assert_eq!(label.context.property_value.as_deref(), Some("mobile-new"));

    let unsupported = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:mobile:set-property:f2fs.uuid")
        .expect("unsupported F2FS property action should exist");
    assert_eq!(unsupported.risk, RiskClass::Unsupported);
    assert!(unsupported.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("f2fs.label"))
    }));
}

#[test]
fn plan_classifies_filesystem_uuid_updates_as_offline_required() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "home": {
                  "mountpoint": "/home",
                  "device": "/dev/disk/by-label/home",
                  "fsType": "ext4",
                  "properties": {
                    "ext.uuid": "11111111-2222-3333-4444-555555555555"
                  }
                },
                "scratch": {
                  "mountpoint": "/scratch",
                  "device": "/dev/disk/by-label/scratch",
                  "fsType": "xfs",
                  "properties": {
                    "filesystem.uuid": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
                  }
                },
                "data": {
                  "mountpoint": "/data",
                  "device": "/dev/disk/by-label/data",
                  "fsType": "btrfs",
                  "properties": {
                    "btrfs.uuid": "bbbbbbbb-1111-2222-3333-cccccccccccc"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 6);
    assert_eq!(plan.summary.offline_required_count, 3);
    assert_eq!(plan.summary.unsupported_count, 0);
    let ext_uuid = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:home:set-property:ext.uuid")
        .expect("Ext UUID property action should exist");
    assert_eq!(ext_uuid.operation, Operation::SetProperty);
    assert_eq!(ext_uuid.risk, RiskClass::OfflineRequired);
    assert_eq!(
        ext_uuid.context.property_value.as_deref(),
        Some("11111111-2222-3333-4444-555555555555")
    );
    assert!(ext_uuid.advice.as_ref().is_some_and(|advice| {
        advice.summary.contains("UUID")
            && advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("NixOS fileSystems"))
    }));

    let xfs_uuid = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:scratch:set-property:filesystem.uuid")
        .expect("XFS UUID property action should exist");
    assert_eq!(xfs_uuid.risk, RiskClass::OfflineRequired);
    assert_eq!(xfs_uuid.context.fs_type.as_deref(), Some("xfs"));

    let btrfs_uuid = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:data:set-property:btrfs.uuid")
        .expect("Btrfs UUID property action should exist");
    assert_eq!(btrfs_uuid.risk, RiskClass::OfflineRequired);
    assert_eq!(btrfs_uuid.context.fs_type.as_deref(), Some("btrfs"));
    assert_eq!(
        btrfs_uuid.context.property_value.as_deref(),
        Some("bbbbbbbb-1111-2222-3333-cccccccccccc")
    );
}

#[test]
fn plan_warns_for_filesystem_device_removal() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "data": {
                  "mountpoint": "/data",
                  "fsType": "btrfs",
                  "removeDevices": ["/dev/disk/by-id/old-btrfs-device"]
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 2);
    assert_eq!(plan.summary.potential_data_loss_count, 1);
    assert!(plan.actions.iter().any(|action| {
        action.id == "filesystems:data:remove-device:/dev/disk/by-id/old-btrfs-device"
            && action.operation == Operation::RemoveDevice
            && action.risk == RiskClass::PotentialDataLoss
            && action.context.collection.as_deref() == Some("filesystems")
            && action.context.target.as_deref() == Some("/data")
            && action.context.device.as_deref() == Some("/dev/disk/by-id/old-btrfs-device")
            && action.advice.is_some()
    }));
}

#[test]
fn plan_accepts_filesystem_rebalance_with_filters() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "data": {
                  "mountpoint": "/data",
                  "fsType": "btrfs",
                  "operation": "rebalance",
                  "properties": {
                    "balance.data": "usage=50",
                    "balance.metadata": "usage=75"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 2);
    assert_eq!(plan.summary.unsupported_count, 0);
    let action = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:data:rebalance")
        .expect("filesystem rebalance action exists");

    assert_eq!(action.operation, Operation::Rebalance);
    assert_eq!(action.risk, RiskClass::Online);
    assert_eq!(action.context.collection.as_deref(), Some("filesystems"));
    assert_eq!(action.context.target.as_deref(), Some("/data"));
    assert_eq!(
        action.context.property_assignments,
        vec![
            "balance.data=usage=50".to_string(),
            "balance.metadata=usage=75".to_string()
        ]
    );
}

#[test]
fn plan_accepts_filesystem_check_and_repair_operations() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "home": {
                  "mountpoint": "/home",
                  "device": "/dev/disk/by-label/home",
                  "fsType": "ext4",
                  "operation": "check"
                },
                "data": {
                  "mountpoint": "/data",
                  "device": "/dev/disk/by-label/data",
                  "fsType": "btrfs",
                  "operation": "repair"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 4);
    assert_eq!(plan.summary.offline_required_count, 2);
    assert_eq!(plan.summary.unsupported_count, 0);
    let check = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:home:check")
        .expect("filesystem check action exists");
    assert_eq!(check.operation, Operation::Check);
    assert_eq!(check.risk, RiskClass::OfflineRequired);
    assert!(!check.destructive);
    assert_eq!(check.context.fs_type.as_deref(), Some("ext4"));
    assert_eq!(
        check.context.device.as_deref(),
        Some("/dev/disk/by-label/home")
    );

    let repair = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:data:repair")
        .expect("filesystem repair action exists");
    assert_eq!(repair.operation, Operation::Repair);
    assert_eq!(repair.risk, RiskClass::OfflineRequired);
    assert!(!repair.destructive);
    assert_eq!(repair.context.fs_type.as_deref(), Some("btrfs"));
    assert!(repair
        .advice
        .as_ref()
        .is_some_and(|advice| { advice.summary.contains("repair mutates metadata") }));
}

#[test]
fn plan_accepts_scrub_lifecycle_for_btrfs_and_pools() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "data": {
                  "mountpoint": "/data",
                  "fsType": "btrfs",
                  "operation": "scrub"
                },
                "bulk": {
                  "mountpoint": "/bulk",
                  "fsType": "bcachefs",
                  "operation": "scrub"
                },
                "archive": {
                  "mountpoint": "/archive",
                  "fsType": "ext4",
                  "operation": "scrub"
                }
              },
              "pools": {
                "tank": {
                  "operation": "scrub"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 7);
    assert_eq!(plan.summary.unsupported_count, 1);
    let btrfs = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:data:scrub")
        .expect("Btrfs scrub action exists");
    assert_eq!(btrfs.operation, Operation::Scrub);
    assert_eq!(btrfs.risk, RiskClass::Online);
    assert_eq!(btrfs.context.target.as_deref(), Some("/data"));

    let bcachefs = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:bulk:scrub")
        .expect("bcachefs scrub action exists");
    assert_eq!(bcachefs.operation, Operation::Scrub);
    assert_eq!(bcachefs.risk, RiskClass::Online);
    assert_eq!(bcachefs.context.target.as_deref(), Some("/bulk"));
    assert!(bcachefs
        .advice
        .as_ref()
        .is_some_and(|advice| { advice.summary.contains("bcachefs scrub verifies") }));

    let pool = plan
        .actions
        .iter()
        .find(|action| action.id == "pools:tank:scrub")
        .expect("pool scrub action exists");
    assert_eq!(pool.operation, Operation::Scrub);
    assert_eq!(pool.risk, RiskClass::Online);

    let unsupported = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:archive:scrub")
        .expect("unsupported filesystem scrub action exists");
    assert_eq!(unsupported.risk, RiskClass::Unsupported);
    assert!(unsupported
        .advice
        .as_ref()
        .is_some_and(|advice| { advice.summary.contains("Btrfs and bcachefs") }));
}

#[test]
fn plan_accepts_filesystem_trim_operation() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "scratch": {
                  "mountpoint": "/scratch",
                  "fsType": "xfs",
                  "operation": "trim"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 2);
    assert_eq!(plan.summary.offline_required_count, 0);
    assert_eq!(plan.summary.unsupported_count, 0);
    let trim = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:scratch:trim")
        .expect("filesystem trim action exists");
    assert_eq!(trim.operation, Operation::Trim);
    assert_eq!(trim.risk, RiskClass::Online);
    assert_eq!(trim.context.target.as_deref(), Some("/scratch"));
    assert!(trim
        .advice
        .as_ref()
        .is_some_and(|advice| { advice.summary.contains("discards unused blocks") }));
}
