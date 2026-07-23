#[test]
fn topology_comparison_reports_current_state_diagnostics() {
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
              "datasets": {
                "tank/home": {
                  "properties": {
                    "compression": "zstd"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("filesystem:/home", NodeKind::Filesystem, "/home")
            .with_path("/home")
            .with_size_bytes(500 * 1024 * 1024 * 1024)
            .with_property("filesystem.type", "ext4"),
    );
    graph.add_node(
        Node::new("zfs:dataset:tank/home", NodeKind::ZfsDataset, "tank/home")
            .with_property("compression", "zstd"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 2);
    assert_eq!(comparison.summary.matched_count, 2);
    assert_eq!(comparison.summary.missing_count, 0);
    assert_eq!(comparison.summary.type_conflict_count, 1);
    assert_eq!(comparison.summary.already_satisfied_count, 1);
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert_eq!(plan.summary.action_count, 1);
    assert!(plan
        .actions
        .iter()
        .all(|action| action.id != "datasets:tank/home:set-property:compression"));
    assert!(plan.actions.iter().any(|action| {
        action.id == "filesystem:home:grow" && action.operation == Operation::Grow
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == TopologyDiagnosticKind::SizeBelowDesired
            && diagnostic.action_id == "filesystem:home:grow"
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == TopologyDiagnosticKind::FilesystemTypeConflict
            && diagnostic.action_id == "filesystem:home:grow"
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
            && diagnostic.action_id == "datasets:tank/home:set-property:compression"
    }));
}

#[test]
fn topology_comparison_reconciles_zfs_dataset_property_aliases() {
    let plan = plan_from_json_bytes(
        br#"{
              "datasets": {
                "tank/home": {
                  "properties": {
                    "compression": "zstd",
                    "mountpoint": "/home",
                    "atime": true
                  }
                },
                "tank/archive": {
                  "properties": {
                    "compression": "lz4"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("zfs:dataset:tank/home", NodeKind::ZfsDataset, "tank/home")
            .with_property("zfs.compression", "zstd")
            .with_property("zfs.mountpoint", "/home")
            .with_property("zfs.atime", "on"),
    );
    graph.add_node(
        Node::new(
            "zfs:dataset:tank/archive",
            NodeKind::ZfsDataset,
            "tank/archive",
        )
        .with_property("zfs.compression", "zstd"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 4);
    assert_eq!(comparison.summary.matched_count, 4);
    assert_eq!(comparison.summary.already_satisfied_count, 3);
    assert_eq!(comparison.summary.suppressed_action_count, 3);
    assert_eq!(plan.actions.len(), 1);
    assert!(plan.actions.iter().any(|action| {
        action.id == "datasets:tank/archive:set-property:compression"
            && action.operation == Operation::SetProperty
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "datasets:tank/home:set-property:compression"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "datasets:tank/home:set-property:mountpoint"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "datasets:tank/home:set-property:atime"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "datasets:tank/archive:set-property:compression"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::PropertyDiffers
            && diagnostic.message.contains("zstd")
            && diagnostic.message.contains("lz4")
    }));
}

#[test]
fn topology_comparison_reconciles_filesystem_identity_property_aliases() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "home": {
                  "mountpoint": "/home",
                  "device": "/dev/disk/by-label/home",
                  "fsType": "ext4",
                  "properties": {
                    "filesystem.label": "homefs",
                    "ext.uuid": "11111111-2222-3333-4444-555555555555"
                  }
                },
                "scratch": {
                  "mountpoint": "/scratch",
                  "device": "/dev/disk/by-label/scratch",
                  "fsType": "xfs",
                  "properties": {
                    "xfs.label": "scratch-new"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("filesystem:/home", NodeKind::Filesystem, "/home")
            .with_path("/home")
            .with_identity(Identity {
                uuid: Some("11111111-2222-3333-4444-555555555555".to_string()),
                partuuid: None,
                label: Some("homefs".to_string()),
                serial: None,
                wwn: None,
            })
            .with_property("filesystem.type", "ext4"),
    );
    graph.add_node(
        Node::new("filesystem:/scratch", NodeKind::Filesystem, "/scratch")
            .with_path("/scratch")
            .with_property("filesystem.type", "xfs")
            .with_property("filesystem.label", "scratch-old"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 5);
    assert_eq!(comparison.summary.matched_count, 5);
    assert_eq!(comparison.summary.already_satisfied_count, 2);
    assert_eq!(comparison.summary.suppressed_action_count, 2);
    assert_eq!(plan.actions.len(), 3);
    assert!(plan.actions.iter().any(|action| {
        action.id == "filesystems:scratch:set-property:xfs.label"
            && action.operation == Operation::SetProperty
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystems:home:set-property:filesystem.label"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystems:home:set-property:ext.uuid"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystems:scratch:set-property:xfs.label"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::PropertyDiffers
            && diagnostic.message.contains("scratch-old")
            && diagnostic.message.contains("scratch-new")
    }));
}

#[test]
fn topology_comparison_reconciles_filesystem_serial_property_aliases() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "efi": {
                  "mountpoint": "/boot",
                  "device": "/dev/disk/by-partlabel/EFI",
                  "fsType": "vfat",
                  "properties": {
                    "vfat.uuid": "a1b2-c3d4"
                  }
                },
                "windows": {
                  "mountpoint": "/mnt/windows",
                  "device": "/dev/disk/by-label/Windows",
                  "fsType": "ntfs",
                  "properties": {
                    "ntfs.volume-serial": "0123456789ABCDEF"
                  }
                },
                "shared": {
                  "mountpoint": "/mnt/shared",
                  "device": "/dev/disk/by-label/Shared",
                  "fsType": "exfat",
                  "properties": {
                    "exfat.uuid": "6EEF-953B"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("filesystem:/boot", NodeKind::Filesystem, "/boot")
            .with_path("/boot")
            .with_identity(Identity {
                uuid: Some("A1B2-C3D4".to_string()),
                partuuid: None,
                label: None,
                serial: None,
                wwn: None,
            })
            .with_property("filesystem.type", "vfat"),
    );
    graph.add_node(
        Node::new(
            "filesystem:/mnt/windows",
            NodeKind::Filesystem,
            "/mnt/windows",
        )
        .with_path("/mnt/windows")
        .with_property("filesystem.type", "ntfs")
        .with_property("ntfs.volume-serial", "01234567-89abcdef"),
    );
    graph.add_node(
        Node::new(
            "filesystem:/mnt/shared",
            NodeKind::Filesystem,
            "/mnt/shared",
        )
        .with_path("/mnt/shared")
        .with_property("filesystem.type", "exfat")
        .with_property("exfat.volume-serial", "0x6eef953b"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 6);
    assert_eq!(comparison.summary.matched_count, 6);
    assert_eq!(comparison.summary.already_satisfied_count, 3);
    assert_eq!(comparison.summary.suppressed_action_count, 3);
    assert_eq!(plan.actions.len(), 3);
    assert!(plan.actions.iter().all(|action| {
        !action.id.contains(":set-property:")
            || !matches!(
                action.id.as_str(),
                "filesystems:efi:set-property:vfat.uuid"
                    | "filesystems:windows:set-property:ntfs.volume-serial"
                    | "filesystems:shared:set-property:exfat.uuid"
            )
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystems:efi:set-property:vfat.uuid"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystems:windows:set-property:ntfs.volume-serial"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystems:shared:set-property:exfat.uuid"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_reports_matching_filesystem_format_type() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "data": {
                  "mountpoint": "/data",
                  "device": "/dev/disk/by-label/data",
                  "fsType": "ext4",
                  "preserveData": false
                },
                "legacy": {
                  "mountpoint": "/legacy",
                  "device": "/dev/disk/by-label/legacy",
                  "fsType": "xfs",
                  "preserveData": false
                },
                "small": {
                  "mountpoint": "/small",
                  "device": "/dev/disk/by-label/small",
                  "fsType": "ext4",
                  "desiredSize": "2GiB",
                  "preserveData": false
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("filesystem:/data", NodeKind::Filesystem, "/data")
            .with_path("/data")
            .with_property("filesystem.type", "ext4"),
    );
    graph.add_node(
        Node::new("filesystem:/legacy", NodeKind::Filesystem, "/legacy")
            .with_path("/legacy")
            .with_property("filesystem.type", "ext4"),
    );
    graph.add_node(
        Node::new("filesystem:/small", NodeKind::Filesystem, "/small")
            .with_path("/small")
            .with_size_bytes(1024 * 1024 * 1024)
            .with_property("filesystem.type", "ext4"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 6);
    assert_eq!(comparison.summary.matched_count, 6);
    assert_eq!(comparison.summary.type_conflict_count, 2);
    assert_eq!(comparison.summary.already_satisfied_count, 2);
    assert_eq!(comparison.summary.suppressed_action_count, 0);
    assert_eq!(plan.summary.action_count, 6);
    assert!(plan.actions.iter().any(|action| {
        action.id == "filesystem:data:preserve-data-disabled"
            && action.operation == Operation::Format
    }));
    assert!(plan.actions.iter().any(|action| {
        action.id == "filesystem:legacy:preserve-data-disabled"
            && action.operation == Operation::Format
    }));
    assert!(plan.actions.iter().any(|action| {
        action.id == "filesystem:small:preserve-data-disabled"
            && action.operation == Operation::Format
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystem:data:preserve-data-disabled"
            && diagnostic.kind == TopologyDiagnosticKind::FilesystemFormatAlreadySatisfied
            && diagnostic.message.contains("type ext4")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystem:legacy:preserve-data-disabled"
            && diagnostic.kind == TopologyDiagnosticKind::FilesystemTypeConflict
            && diagnostic.level == TopologyDiagnosticLevel::Warning
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystem:small:preserve-data-disabled"
            && diagnostic.kind == TopologyDiagnosticKind::FilesystemFormatAlreadySatisfied
            && diagnostic.message.contains("type ext4")
    }));
}

#[test]
fn topology_comparison_suppresses_already_mounted_sources() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "backup": {
                  "operation": "mount",
                  "mountpoint": "/backup",
                  "device": "/dev/disk/by-label/backup",
                  "fsType": "xfs"
                }
              },
              "nfs": {
                "mounts": {
                  "/srv/shared": {
                    "operation": "mount",
                    "source": "nas.example.com:/srv/shared",
                    "fsType": "nfs4"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("mount:/backup", NodeKind::Mountpoint, "/backup")
            .with_property("mount.source", "/dev/disk/by-label/backup"),
    );
    graph.add_node(
        Node::new("mount:/srv/shared", NodeKind::NfsMount, "/srv/shared")
            .with_property("mount.source", "nas.example.com:/srv/shared"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 3);
    assert_eq!(comparison.summary.already_satisfied_count, 2);
    assert_eq!(comparison.summary.suppressed_action_count, 2);
    assert!(plan
        .actions
        .iter()
        .all(|action| action.id != "filesystems:backup:mount"));
    assert!(plan
        .actions
        .iter()
        .all(|action| action.id != "nfs.mounts:/srv/shared:mount"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystems:backup:mount"
            && diagnostic.kind == TopologyDiagnosticKind::MountAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "nfs.mounts:/srv/shared:mount"
            && diagnostic.kind == TopologyDiagnosticKind::MountAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_mount_action_when_source_differs() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "backup": {
                  "operation": "mount",
                  "mountpoint": "/backup",
                  "device": "/dev/disk/by-label/backup",
                  "fsType": "xfs"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("mount:/backup", NodeKind::Mountpoint, "/backup")
            .with_property("mount.source", "/dev/disk/by-label/other"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 2);
    assert_eq!(comparison.summary.already_satisfied_count, 0);
    assert_eq!(comparison.summary.suppressed_action_count, 0);
    assert!(plan
        .actions
        .iter()
        .any(|action| action.id == "filesystems:backup:mount"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystems:backup:mount"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::MountSourceConflict
    }));
}

#[test]
fn topology_comparison_keeps_absent_nfs_mount_actionable() {
    let plan = plan_from_json_bytes(
        br#"{
              "nfs": {
                "mounts": {
                  "/srv/shared": {
                    "operation": "mount",
                    "source": "nas.example.com:/srv/shared"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    let plan = compare_plan_with_topology(plan, &StorageGraph::empty());
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 1);
    assert_eq!(comparison.summary.already_satisfied_count, 0);
    assert_eq!(comparison.summary.suppressed_action_count, 0);
    assert_eq!(comparison.summary.missing_count, 0);
    assert_eq!(plan.actions.len(), 1);
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "nfs.mounts:/srv/shared:mount"
            && diagnostic.level == TopologyDiagnosticLevel::Info
            && diagnostic.kind == TopologyDiagnosticKind::MountRequired
            && diagnostic.message.contains("nas.example.com:/srv/shared")
    }));
}

#[test]
fn topology_comparison_suppresses_unmount_when_mountpoint_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "archive": {
                  "operation": "unmount",
                  "mountpoint": "/archive"
                }
              },
              "nfs": {
                "mounts": {
                  "/srv/old": {
                    "operation": "unmount",
                    "source": "nas.example.com:/srv/old"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let graph = StorageGraph::empty();

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 3);
    assert_eq!(comparison.summary.missing_count, 1);
    assert_eq!(comparison.summary.already_satisfied_count, 2);
    assert_eq!(comparison.summary.suppressed_action_count, 2);
    assert!(plan
        .actions
        .iter()
        .all(|action| action.id != "filesystems:archive:unmount"));
    assert!(plan
        .actions
        .iter()
        .all(|action| action.id != "nfs.mounts:/srv/old:unmount"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystems:archive:unmount"
            && diagnostic.current.is_none()
            && diagnostic.kind == TopologyDiagnosticKind::UnmountAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "nfs.mounts:/srv/old:unmount"
            && diagnostic.current.is_none()
            && diagnostic.kind == TopologyDiagnosticKind::UnmountAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_unmount_when_mountpoint_exists() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "archive": {
                  "operation": "unmount",
                  "mountpoint": "/archive"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("mount:/archive", NodeKind::Mountpoint, "/archive")
            .with_property("mount.source", "/dev/disk/by-label/archive"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 2);
    assert_eq!(comparison.summary.already_satisfied_count, 0);
    assert_eq!(comparison.summary.suppressed_action_count, 0);
    assert!(plan
        .actions
        .iter()
        .any(|action| action.id == "filesystems:archive:unmount"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystems:archive:unmount"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::UnmountRequired
    }));
}

#[test]
fn topology_comparison_suppresses_inactive_swap_teardown() {
    let plan = plan_from_json_bytes(
        br#"{
              "swaps": {
                "old-file": {
                  "path": "/swapfile.old",
                  "operation": "deactivate"
                },
                "old-device": {
                  "device": "/dev/disk/by-label/old-swap",
                  "operation": "destroy"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let graph = StorageGraph::empty();

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 2);
    assert_eq!(comparison.summary.already_satisfied_count, 2);
    assert_eq!(comparison.summary.suppressed_action_count, 2);
    assert!(plan.actions.is_empty());
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "swaps:old-file:deactivate"
            && diagnostic.current.is_none()
            && diagnostic.kind == TopologyDiagnosticKind::SwapDeactivateAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "swaps:old-device:destroy"
            && diagnostic.current.is_none()
            && diagnostic.kind == TopologyDiagnosticKind::SwapDestroyAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_active_swap_teardown() {
    let plan = plan_from_json_bytes(
        br#"{
              "swaps": {
                "scratch": {
                  "path": "/swapfile",
                  "operation": "deactivate"
                },
                "remove": {
                  "device": "/dev/disk/by-label/remove-swap",
                  "operation": "destroy"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("swap:/swapfile", NodeKind::Swap, "/swapfile")
            .with_path("/swapfile")
            .with_size_bytes(1_073_741_824)
            .with_usage(Usage {
                used_bytes: Some(134_217_728),
                free_bytes: Some(939_524_096),
                allocated_bytes: Some(1_073_741_824),
            })
            .with_property("swap.active", "true")
            .with_property("swap.type", "file")
            .with_property("swap.priority", "10"),
    );
    graph.add_node(
        Node::new(
            "swap:/dev/disk/by-label/remove-swap",
            NodeKind::Swap,
            "/dev/disk/by-label/remove-swap",
        )
        .with_path("/dev/disk/by-label/remove-swap")
        .with_property("swap.active", "true")
        .with_property("swap.type", "partition"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 2);
    assert_eq!(comparison.summary.already_satisfied_count, 0);
    assert_eq!(comparison.summary.suppressed_action_count, 0);
    assert_eq!(plan.actions.len(), 2);
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "swaps:scratch:deactivate"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SwapDeactivateRequired
            && diagnostic.message.contains("priority 10")
            && diagnostic.message.contains("type file")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "swaps:remove:destroy"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SwapDestroyRequired
            && diagnostic.message.contains("type partition")
    }));
}

#[test]
fn topology_comparison_reports_swap_format_target_metadata() {
    let plan = plan_from_json_bytes(
        br#"{
              "swaps": {
                "scratch": {
                  "path": "/swapfile",
                  "operation": "format"
                },
                "device": {
                  "device": "/dev/disk/by-label/swap",
                  "operation": "format"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("swap:/swapfile", NodeKind::Swap, "/swapfile")
            .with_path("/swapfile")
            .with_size_bytes(2_147_483_648)
            .with_usage(Usage {
                used_bytes: Some(268_435_456),
                free_bytes: Some(1_879_048_192),
                allocated_bytes: Some(2_147_483_648),
            })
            .with_property("swap.active", "true")
            .with_property("swap.type", "file")
            .with_property("swap.priority", "5"),
    );
    graph.add_node(
        Node::new(
            "filesystem:/dev/disk/by-label/swap",
            NodeKind::Filesystem,
            "/dev/disk/by-label/swap",
        )
        .with_path("/dev/disk/by-label/swap")
        .with_property("filesystem.type", "ext4"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 2);
    assert_eq!(comparison.summary.matched_count, 2);
    assert_eq!(comparison.summary.already_satisfied_count, 0);
    assert_eq!(comparison.summary.suppressed_action_count, 0);
    assert_eq!(plan.summary.action_count, 2);
    assert!(plan.actions.iter().any(|action| {
        action.id == "swaps:scratch:format" && action.operation == Operation::Format
    }));
    assert!(plan.actions.iter().any(|action| {
        action.id == "swaps:device:format" && action.operation == Operation::Format
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "swaps:scratch:format"
            && diagnostic.query == "/swapfile"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SwapFormatTargetPresent
            && diagnostic.message.contains("size 2147483648 bytes")
            && diagnostic.message.contains("used 268435456 bytes")
            && diagnostic.message.contains("priority 5")
            && diagnostic.message.contains("type file")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "swaps:device:format"
            && diagnostic.query == "/dev/disk/by-label/swap"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SwapFormatTargetPresent
            && diagnostic.message.contains("filesystem")
    }));
}

#[test]
fn topology_comparison_reconciles_swap_identity_property_aliases() {
    let plan = plan_from_json_bytes(
        br#"{
              "swaps": {
                "primary": {
                  "device": "/dev/disk/by-label/swap-old",
                  "properties": {
                    "label": "swap-new",
                    "swap.uuid": "01234567-89AB-CDEF-0123-456789ABCDEF",
                    "priority": "10"
                  }
                },
                "scratch": {
                  "device": "/dev/disk/by-label/scratch-swap",
                  "properties": {
                    "uuid": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                    "swap.priority": "20"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "swap:/dev/disk/by-label/swap-old",
            NodeKind::Swap,
            "swap-old",
        )
        .with_path("/dev/disk/by-label/swap-old")
        .with_identity(Identity {
            uuid: Some("01234567-89ab-cdef-0123-456789abcdef".to_string()),
            partuuid: None,
            label: Some("swap-new".to_string()),
            serial: None,
            wwn: None,
        })
        .with_property("swap.active", "false")
        .with_property("swap.priority", "10"),
    );
    graph.add_node(
        Node::new(
            "swap:/dev/disk/by-label/scratch-swap",
            NodeKind::Swap,
            "scratch-swap",
        )
        .with_path("/dev/disk/by-label/scratch-swap")
        .with_property("swap.uuid", "ffffffff-1111-2222-3333-444444444444")
        .with_property("swap.priority", "5"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 7);
    assert_eq!(comparison.summary.matched_count, 7);
    assert_eq!(comparison.summary.already_satisfied_count, 3);
    assert_eq!(comparison.summary.suppressed_action_count, 3);
    assert_eq!(plan.actions.len(), 4);
    assert!(plan.actions.iter().any(|action| {
        action.id == "swaps:scratch:set-property:uuid" && action.operation == Operation::SetProperty
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "swaps:primary:set-property:label"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "swaps:primary:set-property:swap.uuid"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "swaps:primary:set-property:priority"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "swaps:scratch:set-property:uuid"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::PropertyDiffers
            && diagnostic
                .message
                .contains("ffffffff-1111-2222-3333-444444444444")
            && diagnostic
                .message
                .contains("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "swaps:scratch:set-property:swap.priority"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::PropertyDiffers
            && diagnostic.message.contains("is 5")
            && diagnostic.message.contains("desired 20")
    }));
}
