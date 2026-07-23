#[test]
fn plan_accepts_filesystem_rescan_operation() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "scratch": {
                  "mountpoint": "/scratch",
                  "device": "/dev/disk/by-label/scratch",
                  "fsType": "xfs",
                  "operation": "rescan"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 2);
    assert_eq!(plan.summary.offline_required_count, 0);
    assert_eq!(plan.summary.unsupported_count, 0);
    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:scratch:rescan")
        .expect("filesystem rescan action exists");
    assert_eq!(rescan.operation, Operation::Rescan);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert!(!rescan.destructive);
    assert_eq!(rescan.context.target.as_deref(), Some("/scratch"));
    assert_eq!(
        rescan.context.device.as_deref(),
        Some("/dev/disk/by-label/scratch")
    );
    assert!(rescan
        .advice
        .as_ref()
        .is_some_and(|advice| advice.summary.contains("refreshes mount")));
}

#[test]
fn plan_accepts_filesystem_remount_operation() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "scratch": {
                  "mountpoint": "/scratch",
                  "fsType": "xfs",
                  "operation": "remount",
                  "options": ["rw", "noatime", "discard=async"]
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 2);
    assert_eq!(plan.summary.offline_required_count, 0);
    assert_eq!(plan.summary.unsupported_count, 0);
    let remount = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:scratch:remount")
        .expect("filesystem remount action exists");
    assert_eq!(remount.operation, Operation::Remount);
    assert_eq!(remount.risk, RiskClass::Online);
    assert_eq!(remount.context.target.as_deref(), Some("/scratch"));
    assert_eq!(
        remount.context.options.as_deref(),
        Some("rw,noatime,discard=async")
    );
    assert!(remount
        .advice
        .as_ref()
        .is_some_and(|advice| advice.summary.contains("updates local mount options")));
}

#[test]
fn plan_accepts_filesystem_mount_lifecycle() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "backup": {
                  "mountpoint": "/backup",
                  "device": "/dev/disk/by-label/backup",
                  "fsType": "xfs",
                  "operation": "mount",
                  "options": ["rw", "noatime"]
                },
                "archive": {
                  "mountpoint": "/archive",
                  "device": "/dev/disk/by-label/archive",
                  "fsType": "ext4",
                  "operation": "unmount"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 4);
    assert_eq!(plan.summary.offline_required_count, 1);
    assert_eq!(plan.summary.unsupported_count, 0);
    assert_eq!(plan.summary.destructive_count, 0);
    let mount = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:backup:mount")
        .expect("filesystem mount action exists");
    assert_eq!(mount.operation, Operation::Mount);
    assert_eq!(mount.risk, RiskClass::Online);
    assert_eq!(
        mount.context.device.as_deref(),
        Some("/dev/disk/by-label/backup")
    );
    assert_eq!(mount.context.mountpoint.as_deref(), Some("/backup"));
    assert_eq!(mount.context.fs_type.as_deref(), Some("xfs"));
    assert_eq!(mount.context.options.as_deref(), Some("rw,noatime"));

    let unmount = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:archive:unmount")
        .expect("filesystem unmount action exists");
    assert_eq!(unmount.operation, Operation::Unmount);
    assert_eq!(unmount.risk, RiskClass::OfflineRequired);
    assert!(!unmount.destructive);
    assert_eq!(unmount.context.mountpoint.as_deref(), Some("/archive"));
}

#[test]
fn plan_carries_desired_size_context_for_resize_actions() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "home": {
                  "mountpoint": "/home",
                  "fsType": "btrfs",
                  "resizePolicy": "grow-only",
                  "desiredSize": "750GiB"
                }
              },
              "volumes": {
                "vg/home": {
                  "operation": "grow",
                  "size": "800GiB"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    let filesystem = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystem:home:grow")
        .expect("filesystem grow action exists");
    assert_eq!(filesystem.context.desired_size.as_deref(), Some("750GiB"));

    let volume = plan
        .actions
        .iter()
        .find(|action| action.id == "volumes:vg/home:grow")
        .expect("volume grow action exists");
    assert_eq!(volume.context.desired_size.as_deref(), Some("800GiB"));
}

#[test]
fn plan_classifies_lvm_logical_volume_lifecycle() {
    let plan = plan_from_json_bytes(
        br#"{
              "volumes": {
                "vg0/scratch": {
                  "operation": "create",
                  "desiredSize": "10GiB"
                },
                "vg0/home": {
                  "operation": "activate"
                },
                "vg0/archive": {
                  "operation": "deactivate"
                },
                "vg0/reporting": {
                  "operation": "rescan"
                },
                "vg0/old": {
                  "destroy": true
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 5);
    assert_eq!(plan.summary.offline_required_count, 2);
    assert_eq!(plan.summary.destructive_count, 1);
    let create = plan
        .actions
        .iter()
        .find(|action| action.id == "volumes:vg0/scratch:create")
        .expect("LV create action exists");
    assert_eq!(create.risk, RiskClass::Online);
    assert_eq!(create.context.desired_size.as_deref(), Some("10GiB"));
    let destroy = plan
        .actions
        .iter()
        .find(|action| action.id == "volumes:vg0/old:destroy")
        .expect("LV destroy action exists");
    assert_eq!(destroy.risk, RiskClass::Destructive);
    assert!(destroy.destructive);
    let activate = plan
        .actions
        .iter()
        .find(|action| action.id == "volumes:vg0/home:activate")
        .expect("LV activate action exists");
    assert_eq!(activate.risk, RiskClass::OfflineRequired);
    assert!(!activate.destructive);
    let deactivate = plan
        .actions
        .iter()
        .find(|action| action.id == "volumes:vg0/archive:deactivate")
        .expect("LV deactivate action exists");
    assert_eq!(deactivate.risk, RiskClass::OfflineRequired);
    assert!(!deactivate.destructive);
    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "volumes:vg0/reporting:rescan")
        .expect("LV rescan action exists");
    assert_eq!(rescan.operation, Operation::Rescan);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert!(!rescan.destructive);
}

#[test]
fn plan_classifies_lvm_physical_volume_lifecycle() {
    let plan = plan_from_json_bytes(
        br#"{
              "physicalVolumes": {
                "/dev/disk/by-id/nvme-pv-new": {
                  "operation": "create"
                },
                "/dev/disk/by-id/nvme-pv-grow": {
                  "operation": "grow"
                },
                "/dev/disk/by-id/nvme-pv-old": {
                  "destroy": true
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 3);
    assert_eq!(plan.summary.destructive_count, 2);
    let create = plan
        .actions
        .iter()
        .find(|action| action.id == "physicalvolumes:/dev/disk/by-id/nvme-pv-new:create")
        .expect("PV create action exists");
    assert_eq!(create.risk, RiskClass::Destructive);
    assert!(create.destructive);
    let grow = plan
        .actions
        .iter()
        .find(|action| action.id == "physicalvolumes:/dev/disk/by-id/nvme-pv-grow:grow")
        .expect("PV grow action exists");
    assert_eq!(grow.risk, RiskClass::Online);
    let destroy = plan
        .actions
        .iter()
        .find(|action| action.id == "physicalvolumes:/dev/disk/by-id/nvme-pv-old:destroy")
        .expect("PV destroy action exists");
    assert_eq!(destroy.risk, RiskClass::Destructive);
    assert!(destroy.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("pvmove"))
    }));
}

#[test]
fn plan_classifies_lvm_volume_group_lifecycle() {
    let plan = plan_from_json_bytes(
        br#"{
              "volumeGroups": {
                "vg0": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/nvme-vg0"
                },
                "vgdata": {
                  "replaceDevices": {
                    "/dev/disk/by-id/old-pv": "/dev/disk/by-id/new-pv"
                  }
                },
                "importvg": {
                  "operation": "import"
                },
                "exportvg": {
                  "operation": "export"
                },
                "activevg": {
                  "operation": "activate"
                },
                "coldvg": {
                  "operation": "deactivate"
                },
                "oldvg": {
                  "destroy": true
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 7);
    assert_eq!(plan.summary.offline_required_count, 5);
    assert_eq!(plan.summary.destructive_count, 2);
    let create = plan
        .actions
        .iter()
        .find(|action| action.id == "volumegroups:vg0:create")
        .expect("volume group create action exists");
    assert_eq!(create.risk, RiskClass::Destructive);
    assert!(create.destructive);
    assert_eq!(
        create.context.device.as_deref(),
        Some("/dev/disk/by-id/nvme-vg0")
    );
    assert!(create.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("pvs"))
    }));
    let destroy = plan
        .actions
        .iter()
        .find(|action| action.id == "volumegroups:oldvg:destroy")
        .expect("volume group destroy action exists");
    assert_eq!(destroy.risk, RiskClass::Destructive);

    let replace = plan
        .actions
        .iter()
        .find(|action| action.id == "volumeGroups:vgdata:replace-device:/dev/disk/by-id/old-pv")
        .expect("volume group replacement action exists");
    assert_eq!(replace.risk, RiskClass::OfflineRequired);
    assert_eq!(
        replace.context.device.as_deref(),
        Some("/dev/disk/by-id/old-pv")
    );
    assert_eq!(
        replace.context.replacement.as_deref(),
        Some("/dev/disk/by-id/new-pv")
    );
    assert!(replace
        .advice
        .as_ref()
        .is_some_and(|advice| { advice.summary.contains("migrate extents before vgreduce") }));
    let import = plan
        .actions
        .iter()
        .find(|action| action.id == "volumegroups:importvg:import")
        .expect("volume group import action exists");
    assert_eq!(import.risk, RiskClass::OfflineRequired);
    assert!(!import.destructive);
    assert!(import.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("vgimport"))
    }));
    let export = plan
        .actions
        .iter()
        .find(|action| action.id == "volumegroups:exportvg:export")
        .expect("volume group export action exists");
    assert_eq!(export.risk, RiskClass::OfflineRequired);
    assert!(!export.destructive);
    let activate = plan
        .actions
        .iter()
        .find(|action| action.id == "volumegroups:activevg:activate")
        .expect("volume group activate action exists");
    assert_eq!(activate.risk, RiskClass::OfflineRequired);
    assert!(!activate.destructive);
    let deactivate = plan
        .actions
        .iter()
        .find(|action| action.id == "volumegroups:coldvg:deactivate")
        .expect("volume group deactivate action exists");
    assert_eq!(deactivate.risk, RiskClass::OfflineRequired);
    assert!(!deactivate.destructive);
}

#[test]
fn plan_classifies_disk_and_partition_lifecycle_safely() {
    let plan = plan_from_json_bytes(
        br#"{
              "disks": {
                "/dev/disk/by-id/nvme-root": {
                  "operation": "create",
                  "partitionType": "gpt"
                },
                "/dev/disk/by-id/nvme-data": {
                  "operation": "rescan"
                }
              },
              "partitions": {
                "root": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/nvme-root",
                  "start": "1MiB",
                  "end": "100%",
                  "partitionType": "linux"
                },
                "home": {
                  "operation": "grow",
                  "device": "/dev/disk/by-id/nvme-root",
                  "partitionNumber": 2,
                  "end": "100%"
                },
                "data-table": {
                  "operation": "rescan",
                  "device": "/dev/disk/by-id/nvme-data"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 5);
    assert_eq!(plan.summary.offline_required_count, 2);
    assert_eq!(plan.summary.destructive_count, 1);

    let root = plan
        .actions
        .iter()
        .find(|action| action.id == "partitions:root:create")
        .expect("partition create action exists");
    assert_eq!(root.risk, RiskClass::OfflineRequired);
    assert_eq!(
        root.context.device.as_deref(),
        Some("/dev/disk/by-id/nvme-root")
    );
    assert_eq!(root.context.start.as_deref(), Some("1MiB"));
    assert_eq!(root.context.end.as_deref(), Some("100%"));
    assert_eq!(root.context.partition_type.as_deref(), Some("linux"));

    let home = plan
        .actions
        .iter()
        .find(|action| action.id == "partitions:home:grow")
        .expect("partition grow action exists");
    assert_eq!(home.risk, RiskClass::OfflineRequired);
    assert_eq!(
        home.context.device.as_deref(),
        Some("/dev/disk/by-id/nvme-root")
    );
    assert_eq!(home.context.partition_number.as_deref(), Some("2"));
    assert_eq!(home.context.end.as_deref(), Some("100%"));

    let disk = plan
        .actions
        .iter()
        .find(|action| action.id == "disks:/dev/disk/by-id/nvme-root:create")
        .expect("disk create action exists");
    assert_eq!(disk.risk, RiskClass::Destructive);
    assert!(disk.destructive);

    let disk_rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "disks:/dev/disk/by-id/nvme-data:rescan")
        .expect("disk rescan action exists");
    assert_eq!(disk_rescan.risk, RiskClass::Online);
    assert!(!disk_rescan.destructive);

    let partition_rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "partitions:data-table:rescan")
        .expect("partition rescan action exists");
    assert_eq!(partition_rescan.risk, RiskClass::Online);
    assert_eq!(
        partition_rescan.context.device.as_deref(),
        Some("/dev/disk/by-id/nvme-data")
    );
}

#[test]
fn plan_classifies_swap_and_luks_lifecycle_safely() {
    let plan = plan_from_json_bytes(
        br#"{
              "swaps": {
                "primary": {
                  "device": "/dev/disk/by-label/swap",
                  "preserveData": false
                },
                "scratch": {
                  "device": "/swapfile",
                  "operation": "grow",
                  "desiredSize": "16GiB"
                },
                "inventory": {
                  "device": "/dev/disk/by-label/swap-inventory",
                  "operation": "rescan"
                },
                "retired": {
                  "device": "/dev/disk/by-label/old-swap",
                  "operation": "deactivate"
                },
                "remove": {
                  "device": "/dev/disk/by-label/remove-swap",
                  "operation": "destroy"
                }
              },
              "luks": {
                "devices": {
                  "cryptroot": {
                    "name": "cryptroot",
                    "device": "/dev/disk/by-partuuid/root",
                    "operation": "grow"
                  },
                  "cryptdata": {
                    "name": "cryptdata",
                    "device": "/dev/disk/by-id/data-luks",
                    "operation": "create"
                  },
                  "cryptarchive": {
                    "name": "cryptarchive",
                    "device": "/dev/disk/by-id/archive-luks",
                    "operation": "open",
                    "preserveData": false
                  },
                  "cryptmissing": {
                    "name": "cryptmissing",
                    "operation": "create"
                  },
                  "cryptscratch": {
                    "name": "cryptscratch",
                    "device": "/dev/disk/by-id/scratch",
                    "preserveData": false
                  },
                  "cryptold": {
                    "name": "cryptold",
                    "device": "/dev/disk/by-id/old-luks",
                    "operation": "destroy"
                  },
                  "cryptclosed": {
                    "name": "cryptclosed",
                    "device": "/dev/disk/by-id/closed-luks",
                    "operation": "close"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 12);
    assert_eq!(plan.summary.offline_required_count, 8);
    assert_eq!(plan.summary.destructive_count, 3);

    let swap = plan
        .actions
        .iter()
        .find(|action| action.id == "swaps:primary:format")
        .expect("swap format action exists");
    assert_eq!(swap.risk, RiskClass::Destructive);
    assert_eq!(
        swap.context.device.as_deref(),
        Some("/dev/disk/by-label/swap")
    );

    let swap_rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "swaps:inventory:rescan")
        .expect("swap rescan action exists");
    assert_eq!(swap_rescan.operation, Operation::Rescan);
    assert_eq!(swap_rescan.risk, RiskClass::Online);
    assert!(!swap_rescan.destructive);
    assert_eq!(
        swap_rescan.context.device.as_deref(),
        Some("/dev/disk/by-label/swap-inventory")
    );

    let swap_deactivate = plan
        .actions
        .iter()
        .find(|action| action.id == "swaps:retired:deactivate")
        .expect("swap deactivate action exists");
    assert_eq!(swap_deactivate.operation, Operation::Deactivate);
    assert_eq!(swap_deactivate.risk, RiskClass::OfflineRequired);
    assert!(!swap_deactivate.destructive);

    let swap_destroy = plan
        .actions
        .iter()
        .find(|action| action.id == "swaps:remove:destroy")
        .expect("swap destroy action exists");
    assert_eq!(swap_destroy.operation, Operation::Destroy);
    assert_eq!(swap_destroy.risk, RiskClass::Destructive);
    assert!(swap_destroy.destructive);
    assert!(swap_destroy.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("deactivate"))
    }));

    let luks = plan
        .actions
        .iter()
        .find(|action| action.id == "luks.devices:cryptroot:grow")
        .expect("luks grow action exists");
    assert_eq!(luks.risk, RiskClass::OfflineRequired);
    assert_eq!(luks.context.target.as_deref(), Some("cryptroot"));
    assert_eq!(
        luks.context.device.as_deref(),
        Some("/dev/disk/by-partuuid/root")
    );

    let open = plan
        .actions
        .iter()
        .find(|action| action.id == "luks.devices:cryptdata:create")
        .expect("luks open action exists");
    assert_eq!(open.risk, RiskClass::OfflineRequired);
    assert!(!open.destructive);
    assert_eq!(open.context.target.as_deref(), Some("cryptdata"));
    assert_eq!(
        open.context.device.as_deref(),
        Some("/dev/disk/by-id/data-luks")
    );

    let explicit_open = plan
        .actions
        .iter()
        .find(|action| action.id == "luks.devices:cryptarchive:open")
        .expect("explicit luks open action exists");
    assert_eq!(explicit_open.operation, Operation::Open);
    assert_eq!(explicit_open.risk, RiskClass::OfflineRequired);
    assert!(!explicit_open.destructive);
    assert_eq!(
        explicit_open.context.device.as_deref(),
        Some("/dev/disk/by-id/archive-luks")
    );

    let missing = plan
        .actions
        .iter()
        .find(|action| action.id == "luks.devices:cryptmissing:create")
        .expect("underspecified luks open action exists");
    assert_eq!(missing.risk, RiskClass::OfflineRequired);
    assert_eq!(missing.context.target.as_deref(), Some("cryptmissing"));
    assert_eq!(missing.context.device, None);

    let close = plan
        .actions
        .iter()
        .find(|action| action.id == "luks.devices:cryptold:destroy")
        .expect("luks close action exists");
    assert_eq!(close.risk, RiskClass::OfflineRequired);
    assert!(!close.destructive);
    assert_eq!(close.context.target.as_deref(), Some("cryptold"));
    assert_eq!(
        close.context.device.as_deref(),
        Some("/dev/disk/by-id/old-luks")
    );

    let explicit_close = plan
        .actions
        .iter()
        .find(|action| action.id == "luks.devices:cryptclosed:close")
        .expect("explicit luks close action exists");
    assert_eq!(explicit_close.operation, Operation::Close);
    assert_eq!(explicit_close.risk, RiskClass::OfflineRequired);
    assert!(!explicit_close.destructive);
    assert_eq!(
        explicit_close.context.target.as_deref(),
        Some("cryptclosed")
    );
}

#[test]
fn plan_accepts_luks_mapper_aliases_for_logical_keys() {
    let plan = plan_from_json_bytes(
        br#"{
              "luks": {
                "devices": {
                  "rootMapping": {
                    "target": "cryptroot",
                    "device": "/dev/disk/by-id/root-luks",
                    "operation": "grow"
                  },
                  "archiveMapping": {
                    "mapperName": "cryptarchive",
                    "device": "/dev/disk/by-id/archive-luks",
                    "operation": "open"
                  },
                  "backupMapping": {
                    "mapper": "cryptbackup",
                    "device": "/dev/disk/by-id/backup-luks",
                    "operation": "close"
                  },
                  "hyphenMapping": {
                    "mapper-name": "crypthyphen",
                    "device": "/dev/disk/by-id/hyphen-luks",
                    "operation": "open"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    let root = plan
        .actions
        .iter()
        .find(|action| action.id == "luks.devices:rootMapping:grow")
        .expect("target alias grow action exists");
    assert_eq!(root.context.target.as_deref(), Some("cryptroot"));

    let archive = plan
        .actions
        .iter()
        .find(|action| action.id == "luks.devices:archiveMapping:open")
        .expect("mapperName alias open action exists");
    assert_eq!(archive.context.target.as_deref(), Some("cryptarchive"));

    let backup = plan
        .actions
        .iter()
        .find(|action| action.id == "luks.devices:backupMapping:close")
        .expect("mapper alias close action exists");
    assert_eq!(backup.context.target.as_deref(), Some("cryptbackup"));

    let hyphen = plan
        .actions
        .iter()
        .find(|action| action.id == "luks.devices:hyphenMapping:open")
        .expect("hyphenated mapper alias open action exists");
    assert_eq!(hyphen.context.target.as_deref(), Some("crypthyphen"));
}

#[test]
fn plan_accepts_swap_label_and_uuid_properties() {
    let plan = plan_from_json_bytes(
        br#"{
              "swaps": {
                "primary": {
                  "device": "/dev/disk/by-label/swap-old",
                  "properties": {
                    "label": "swap-new",
                    "swap.uuid": "01234567-89ab-cdef-0123-456789abcdef",
                    "priority": "10"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 4);
    assert_eq!(plan.summary.offline_required_count, 3);
    assert_eq!(plan.summary.unsupported_count, 0);

    let label = plan
        .actions
        .iter()
        .find(|action| action.id == "swaps:primary:set-property:label")
        .expect("swap label action exists");
    assert_eq!(label.operation, Operation::SetProperty);
    assert_eq!(label.risk, RiskClass::OfflineRequired);
    assert_eq!(label.context.property_value.as_deref(), Some("swap-new"));
    assert!(label.advice.as_ref().is_some_and(|advice| {
        advice
            .summary
            .contains("swap label and UUID updates mutate swap signature identity")
    }));

    let uuid = plan
        .actions
        .iter()
        .find(|action| action.id == "swaps:primary:set-property:swap.uuid")
        .expect("swap UUID action exists");
    assert_eq!(uuid.risk, RiskClass::OfflineRequired);
    assert_eq!(
        uuid.context.property_value.as_deref(),
        Some("01234567-89ab-cdef-0123-456789abcdef")
    );

    let priority = plan
        .actions
        .iter()
        .find(|action| action.id == "swaps:primary:set-property:priority")
        .expect("swap priority property action exists");
    assert_eq!(priority.risk, RiskClass::OfflineRequired);
    assert!(priority.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("NixOS swapDevices priority"))
    }));
}

#[test]
fn plan_accepts_swap_path_aliases_for_logical_keys() {
    let plan = plan_from_json_bytes(
        br#"{
              "swaps": {
                "scratch": {
                  "path": "/swapfile",
                  "operation": "grow",
                  "desiredSize": "16GiB"
                },
                "inventory": {
                  "target": "/dev/disk/by-label/swap-inventory",
                  "operation": "rescan"
                },
                "primary": {
                  "path": "/dev/disk/by-label/swap",
                  "preserveData": false
                }
              }
            }"#,
    )
    .expect("plan should parse");

    let grow = plan
        .actions
        .iter()
        .find(|action| action.id == "swaps:scratch:grow")
        .expect("logical-key swap grow action exists");
    assert_eq!(grow.context.target.as_deref(), Some("/swapfile"));
    assert_eq!(grow.context.device.as_deref(), Some("/swapfile"));
    assert_eq!(grow.context.desired_size.as_deref(), Some("16GiB"));

    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "swaps:inventory:rescan")
        .expect("logical-key swap rescan action exists");
    assert_eq!(
        rescan.context.target.as_deref(),
        Some("/dev/disk/by-label/swap-inventory")
    );

    let format = plan
        .actions
        .iter()
        .find(|action| action.id == "swaps:primary:format")
        .expect("logical-key swap format action exists");
    assert_eq!(
        format.context.target.as_deref(),
        Some("/dev/disk/by-label/swap")
    );
    assert_eq!(format.risk, RiskClass::Destructive);
}
