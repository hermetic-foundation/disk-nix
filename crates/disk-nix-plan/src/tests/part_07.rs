#[test]
fn topology_comparison_reconciles_luks_identity_property_aliases() {
    let plan = plan_from_json_bytes(
        br#"{
              "luks": {
                "devices": {
                  "cryptroot": {
                    "name": "cryptroot",
                    "device": "/dev/disk/by-id/root-luks",
                    "properties": {
                      "label": "root",
                      "luks.subsystem": "nixos",
                      "luks.uuid": "01234567-89AB-CDEF-0123-456789ABCDEF"
                    }
                  },
                  "cryptdata": {
                    "name": "cryptdata",
                    "device": "/dev/disk/by-id/data-luks",
                    "properties": {
                      "cryptsetup.label": "data-new"
                    }
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "block:/dev/disk/by-id/root-luks",
            NodeKind::LuksContainer,
            "root-luks",
        )
        .with_path("/dev/disk/by-id/root-luks")
        .with_identity(Identity {
            uuid: Some("01234567-89ab-cdef-0123-456789abcdef".to_string()),
            partuuid: None,
            label: Some("root".to_string()),
            serial: None,
            wwn: None,
        })
        .with_property("cryptsetup.luks-subsystem", "nixos"),
    );
    graph.add_node(
        Node::new(
            "block:/dev/disk/by-id/data-luks",
            NodeKind::LuksContainer,
            "data-luks",
        )
        .with_path("/dev/disk/by-id/data-luks")
        .with_property("cryptsetup.label", "data-old"),
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
        action.id == "luks.devices:cryptdata:set-property:cryptsetup.label"
            && action.operation == Operation::SetProperty
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "luks.devices:cryptroot:set-property:label"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "luks.devices:cryptroot:set-property:luks.subsystem"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "luks.devices:cryptroot:set-property:luks.uuid"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "luks.devices:cryptdata:set-property:cryptsetup.label"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::PropertyDiffers
            && diagnostic.message.contains("data-old")
            && diagnostic.message.contains("data-new")
    }));
}

#[test]
fn topology_comparison_suppresses_remount_when_options_are_present() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "scratch": {
                  "operation": "remount",
                  "mountpoint": "/scratch",
                  "options": ["rw", "noatime", "discard=async"]
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("mount:/scratch", NodeKind::Mountpoint, "/scratch")
            .with_property("mount.options", "rw,relatime,noatime,discard=async"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 2);
    assert_eq!(comparison.summary.already_satisfied_count, 1);
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert!(plan
        .actions
        .iter()
        .all(|action| action.id != "filesystems:scratch:remount"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystems:scratch:remount"
            && diagnostic.kind == TopologyDiagnosticKind::MountOptionsAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_suppresses_nfs_remount_from_nfs_option_properties() {
    let plan = plan_from_json_bytes(
        br#"{
              "nfs": {
                "mounts": {
                  "/srv/shared": {
                    "operation": "remount",
                    "options": ["rw", "vers=4.2", "_netdev"]
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("mount:/srv/shared", NodeKind::NfsMount, "/srv/shared")
            .with_property("nfs.rw", "true")
            .with_property("nfs.vers", "4.2")
            .with_property("nfs.netdev", "true"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 1);
    assert_eq!(comparison.summary.already_satisfied_count, 1);
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert!(plan.actions.is_empty());
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "nfs.mounts:/srv/shared:remount"
            && diagnostic.kind == TopologyDiagnosticKind::MountOptionsAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_remount_when_options_differ() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "scratch": {
                  "operation": "remount",
                  "mountpoint": "/scratch",
                  "options": ["ro", "noatime"]
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("mount:/scratch", NodeKind::Mountpoint, "/scratch")
            .with_property("mount.options", "rw,relatime"),
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
        .any(|action| action.id == "filesystems:scratch:remount"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystems:scratch:remount"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::MountOptionsDiffer
    }));
}

#[test]
fn topology_comparison_keeps_absent_nfs_export_actionable() {
    let plan = plan_from_json_bytes(
        br#"{
              "exports": {
                "/srv/share": {
                  "operation": "export",
                  "client": "192.0.2.0/24",
                  "options": "rw,sync,no_subtree_check"
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
        diagnostic.action_id == "exports:/srv/share:export"
            && diagnostic.level == TopologyDiagnosticLevel::Info
            && diagnostic.kind == TopologyDiagnosticKind::NfsExportRequired
            && diagnostic.message.contains("192.0.2.0/24")
            && diagnostic.message.contains("rw,sync,no_subtree_check")
    }));
}

#[test]
fn topology_comparison_suppresses_already_exported_nfs_path() {
    let plan = plan_from_json_bytes(
        br#"{
              "exports": {
                "/srv/share": {
                  "operation": "export",
                  "client": "192.0.2.0/24",
                  "options": "rw,sync,no_subtree_check"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "nfs-export:/srv/share:192.0.2.0/24",
            NodeKind::NfsExport,
            "/srv/share",
        )
        .with_property("nfs.export", "/srv/share")
        .with_property("nfs.export-client", "192.0.2.0/24")
        .with_property("nfs.exportfs", "true")
        .with_property("nfs.export-option-rw", "true")
        .with_property("nfs.export-option-sync", "true")
        .with_property("nfs.export-option-no-subtree-check", "true"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 1);
    assert_eq!(comparison.summary.already_satisfied_count, 1);
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert!(plan.actions.is_empty());
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "exports:/srv/share:export"
            && diagnostic.kind == TopologyDiagnosticKind::NfsExportAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_nfs_export_when_client_or_options_differ() {
    let plan = plan_from_json_bytes(
        br#"{
              "exports": {
                "/srv/share": {
                  "operation": "export",
                  "client": "192.0.2.0/24",
                  "options": "rw,sync,no_subtree_check"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "nfs-export:/srv/share:198.51.100.10",
            NodeKind::NfsExport,
            "/srv/share",
        )
        .with_property("nfs.export", "/srv/share")
        .with_property("nfs.export-client", "198.51.100.10")
        .with_property("nfs.exportfs", "true")
        .with_property("nfs.export-option-ro", "true")
        .with_property("nfs.export-option-sync", "true"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 1);
    assert_eq!(comparison.summary.already_satisfied_count, 0);
    assert_eq!(comparison.summary.suppressed_action_count, 0);
    assert_eq!(plan.actions.len(), 1);
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "exports:/srv/share:export"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::NfsExportDiffers
    }));
}

#[test]
fn topology_comparison_suppresses_absent_nfs_unexport() {
    let plan = plan_from_json_bytes(
        br#"{
              "exports": {
                "/srv/old": {
                  "operation": "unexport",
                  "client": "192.0.2.0/24"
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

    assert_eq!(comparison.summary.action_count, 1);
    assert_eq!(comparison.summary.missing_count, 0);
    assert_eq!(comparison.summary.already_satisfied_count, 1);
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert!(plan.actions.is_empty());
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "exports:/srv/old:unexport"
            && diagnostic.current.is_none()
            && diagnostic.kind == TopologyDiagnosticKind::NfsUnexportAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_nfs_unexport_when_export_exists() {
    let plan = plan_from_json_bytes(
        br#"{
              "exports": {
                "/srv/old": {
                  "operation": "unexport",
                  "client": "192.0.2.0/24"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "nfs-export:/srv/old:192.0.2.0/24",
            NodeKind::NfsExport,
            "/srv/old",
        )
        .with_property("nfs.export", "/srv/old")
        .with_property("nfs.export-client", "192.0.2.0/24")
        .with_property("nfs.exportfs", "true"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 1);
    assert_eq!(comparison.summary.already_satisfied_count, 0);
    assert_eq!(comparison.summary.suppressed_action_count, 0);
    assert_eq!(plan.actions.len(), 1);
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "exports:/srv/old:unexport"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::NfsUnexportRequired
    }));
}

#[test]
fn topology_comparison_reports_luks_format_target_metadata() {
    let plan = plan_from_json_bytes(
        br#"{
              "luks": {
                "devices": {
                  "cryptroot": {
                    "operation": "format",
                    "device": "/dev/disk/by-partuuid/root",
                    "target": "cryptroot"
                  },
                  "cryptdata": {
                    "operation": "format",
                    "device": "/dev/disk/by-id/data",
                    "target": "cryptdata"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "block:/dev/disk/by-partuuid/root",
            NodeKind::LuksContainer,
            "/dev/disk/by-partuuid/root",
        )
        .with_path("/dev/disk/by-partuuid/root")
        .with_property("cryptsetup.luks-version", "2")
        .with_property("cryptsetup.uuid", "11111111-2222-3333-4444-555555555555")
        .with_property("cryptsetup.luks-keyslot-count", "2")
        .with_property("cryptsetup.luks-token-count", "1")
        .with_property("cryptsetup.active", "false"),
    );
    graph.add_node(
        Node::new(
            "block:/dev/disk/by-id/data",
            NodeKind::Partition,
            "/dev/disk/by-id/data",
        )
        .with_path("/dev/disk/by-id/data"),
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
        action.id == "luks.devices:cryptroot:format" && action.operation == Operation::Format
    }));
    assert!(plan.actions.iter().any(|action| {
        action.id == "luks.devices:cryptdata:format" && action.operation == Operation::Format
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "luks.devices:cryptroot:format"
            && diagnostic.query == "/dev/disk/by-partuuid/root"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LuksFormatTargetPresent
            && diagnostic.message.contains("version 2")
            && diagnostic.message.contains("keyslots 2")
            && diagnostic.message.contains("tokens 1")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "luks.devices:cryptdata:format"
            && diagnostic.query == "/dev/disk/by-id/data"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LuksFormatTargetPresent
            && diagnostic.message.contains("partition")
    }));
}

#[test]
fn topology_comparison_suppresses_open_luks_mapper_when_active() {
    let plan = plan_from_json_bytes(
        br#"{
              "luks": {
                "devices": {
                  "cryptroot": {
                    "operation": "open",
                    "device": "/dev/disk/by-partuuid/root",
                    "target": "cryptroot"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "block:/dev/mapper/cryptroot",
            NodeKind::LuksContainer,
            "cryptroot",
        )
        .with_path("/dev/mapper/cryptroot")
        .with_property("cryptsetup.active", "true")
        .with_property("cryptsetup.in-use", "true"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 1);
    assert_eq!(comparison.summary.already_satisfied_count, 1);
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert!(plan.actions.is_empty());
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "luks.devices:cryptroot:open"
            && diagnostic.kind == TopologyDiagnosticKind::LuksOpenAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_open_luks_mapper_when_inactive() {
    let plan = plan_from_json_bytes(
        br#"{
              "luks": {
                "devices": {
                  "cryptroot": {
                    "operation": "open",
                    "device": "/dev/disk/by-partuuid/root",
                    "target": "cryptroot"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "block:/dev/mapper/cryptroot",
            NodeKind::LuksContainer,
            "cryptroot",
        )
        .with_path("/dev/mapper/cryptroot")
        .with_property("cryptsetup.active", "false"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 1);
    assert_eq!(comparison.summary.already_satisfied_count, 0);
    assert_eq!(comparison.summary.suppressed_action_count, 0);
    assert_eq!(plan.actions.len(), 1);
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "luks.devices:cryptroot:open"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LuksOpenRequired
    }));
}

#[test]
fn topology_comparison_reconciles_absent_luks_open_and_close() {
    let plan = plan_from_json_bytes(
        br#"{
              "luks": {
                "devices": {
                  "cryptroot": {
                    "operation": "open",
                    "device": "/dev/disk/by-partuuid/root",
                    "target": "cryptroot"
                  },
                  "cryptold": {
                    "operation": "close",
                    "target": "cryptold"
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

    assert_eq!(comparison.summary.action_count, 2);
    assert_eq!(comparison.summary.already_satisfied_count, 1);
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert_eq!(comparison.summary.missing_count, 0);
    assert_eq!(plan.actions.len(), 1);
    assert!(plan
        .actions
        .iter()
        .any(|action| action.id == "luks.devices:cryptroot:open"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "luks.devices:cryptroot:open"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LuksOpenRequired
            && diagnostic.message.contains("/dev/disk/by-partuuid/root")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "luks.devices:cryptold:close"
            && diagnostic.level == TopologyDiagnosticLevel::Info
            && diagnostic.kind == TopologyDiagnosticKind::LuksCloseAlreadySatisfied
            && diagnostic.current.is_none()
    }));
}

#[test]
fn topology_comparison_suppresses_close_luks_mapper_when_inactive() {
    let plan = plan_from_json_bytes(
        br#"{
              "luks": {
                "devices": {
                  "cryptroot": {
                    "operation": "close",
                    "target": "cryptroot"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "block:/dev/mapper/cryptroot",
            NodeKind::LuksContainer,
            "cryptroot",
        )
        .with_path("/dev/mapper/cryptroot")
        .with_property("cryptsetup.active", "false"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 1);
    assert_eq!(comparison.summary.already_satisfied_count, 1);
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert!(plan.actions.is_empty());
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "luks.devices:cryptroot:close"
            && diagnostic.kind == TopologyDiagnosticKind::LuksCloseAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_close_luks_mapper_when_active() {
    let plan = plan_from_json_bytes(
        br#"{
              "luks": {
                "devices": {
                  "cryptroot": {
                    "operation": "close",
                    "target": "cryptroot"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "block:/dev/mapper/cryptroot",
            NodeKind::LuksContainer,
            "cryptroot",
        )
        .with_path("/dev/mapper/cryptroot")
        .with_property("cryptsetup.active", "true"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 1);
    assert_eq!(comparison.summary.already_satisfied_count, 0);
    assert_eq!(comparison.summary.suppressed_action_count, 0);
    assert_eq!(plan.actions.len(), 1);
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "luks.devices:cryptroot:close"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LuksCloseRequired
    }));
}

#[test]
fn topology_comparison_suppresses_luks_keyslot_remove_when_slot_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "luksKeyslots": {
                "cryptroot:2": {
                  "operation": "remove-key",
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "keySlot": "2"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "block:/dev/disk/by-id/root-luks",
            NodeKind::LuksContainer,
            "root-luks",
        )
        .with_path("/dev/disk/by-id/root-luks")
        .with_property("cryptsetup.luks-keyslots", "0,1")
        .with_property("cryptsetup.luks-keyslot-count", "2"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 1);
    assert_eq!(comparison.summary.already_satisfied_count, 1);
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert!(plan.actions.is_empty());
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "lukskeyslots:cryptroot:2:remove-key"
            && diagnostic.kind == TopologyDiagnosticKind::LuksKeyslotRemoveAlreadySatisfied
            && diagnostic.query == "/dev/disk/by-id/root-luks"
    }));
}

#[test]
fn topology_comparison_reconciles_luks_keyslot_priority_properties() {
    let plan = plan_from_json_bytes(
        br#"{
              "luksKeyslots": {
                "cryptroot:1": {
                  "properties": {
                    "priority": "prefer"
                  },
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "keySlot": "1"
                  }
                },
                "cryptroot:2": {
                  "properties": {
                    "priority": "ignore"
                  },
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "keySlot": "2"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "block:/dev/disk/by-id/root-luks",
            NodeKind::LuksContainer,
            "root-luks",
        )
        .with_path("/dev/disk/by-id/root-luks")
        .with_property("cryptsetup.luks-keyslots", "1,2")
        .with_property("cryptsetup.luks-keyslot-1-priority", "prefer")
        .with_property("cryptsetup.luks-keyslot-2-priority", "normal"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 2);
    assert_eq!(comparison.summary.already_satisfied_count, 1);
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert_eq!(plan.actions.len(), 1);
    assert!(plan
        .actions
        .iter()
        .any(|action| action.id == "luksKeyslots:cryptroot:2:set-property:priority"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "luksKeyslots:cryptroot:1:set-property:priority"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "luksKeyslots:cryptroot:2:set-property:priority"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyDiffers
            && diagnostic.message.contains("normal")
            && diagnostic.message.contains("ignore")
    }));
}

#[test]
fn topology_comparison_keeps_luks_keyslot_remove_when_slot_present() {
    let plan = plan_from_json_bytes(
        br#"{
              "luksKeyslots": {
                "cryptroot:2": {
                  "operation": "remove-key",
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "keySlot": "2"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "block:/dev/disk/by-id/root-luks",
            NodeKind::LuksContainer,
            "root-luks",
        )
        .with_path("/dev/disk/by-id/root-luks")
        .with_property("cryptsetup.luks-keyslots", "0,2")
        .with_property("cryptsetup.luks-keyslot-2-type", "luks2")
        .with_property("cryptsetup.luks-keyslot-2-priority", "normal")
        .with_property("cryptsetup.luks-keyslot-2-pbkdf", "argon2id")
        .with_property("cryptsetup.luks-keyslot-2-time-cost", "4"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 1);
    assert_eq!(comparison.summary.already_satisfied_count, 0);
    assert_eq!(comparison.summary.suppressed_action_count, 0);
    assert_eq!(plan.actions.len(), 1);
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "lukskeyslots:cryptroot:2:remove-key"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LuksKeyslotRemoveRequired
            && diagnostic.message.contains("type luks2")
            && diagnostic.message.contains("priority normal")
            && diagnostic.message.contains("PBKDF argon2id")
    }));
}
