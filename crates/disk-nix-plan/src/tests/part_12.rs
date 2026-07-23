#[test]
fn topology_comparison_suppresses_btrfs_snapshot_destroy_when_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "/mnt/persist/@home-old": {
                  "target": "/mnt/persist/@home",
                  "destroy": true
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
    assert_eq!(comparison.summary.already_satisfied_count, 1);
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert!(plan.actions.is_empty());
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "snapshot:/mnt/persist/@home-old:destroy"
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotDestroyAlreadySatisfied
            && diagnostic.query == "/mnt/persist/@home-old"
    }));
}

#[test]
fn topology_comparison_keeps_btrfs_snapshot_destroy_when_present() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "/mnt/persist/@home-old": {
                  "target": "/mnt/persist/@home",
                  "destroy": true
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "btrfs-snapshot:fs-uuid:@home-old",
            NodeKind::BtrfsSnapshot,
            "@home-old",
        )
        .with_path("/mnt/persist/@home-old")
        .with_property("btrfs.id", "258")
        .with_property("btrfs.generation", "120")
        .with_property("btrfs.parent-uuid", "source-uuid"),
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
        diagnostic.action_id == "snapshot:/mnt/persist/@home-old:destroy"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotDestroyRequired
            && diagnostic.query == "/mnt/persist/@home-old"
            && diagnostic.message.contains("Btrfs snapshot")
            && diagnostic.message.contains("subvolume id 258")
            && diagnostic.message.contains("generation 120")
            && diagnostic.message.contains("parent UUID source-uuid")
    }));
}

#[test]
fn topology_comparison_keeps_logical_snapshot_destroy_missing() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "old-home": {
                  "target": "tank/home",
                  "destroy": true
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

    assert_eq!(comparison.summary.already_satisfied_count, 0);
    assert_eq!(comparison.summary.suppressed_action_count, 0);
    assert_eq!(comparison.summary.missing_count, 1);
    assert_eq!(plan.actions.len(), 1);
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "snapshot:old-home:destroy"
            && diagnostic.kind == TopologyDiagnosticKind::Missing
            && diagnostic.query == "old-home"
    }));
}

#[test]
fn topology_comparison_warns_when_zfs_rollback_snapshot_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "tank/home@before": {
                  "target": "tank/home",
                  "rollback": true
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(Node::new(
        "zfs-dataset:tank/home",
        NodeKind::ZfsDataset,
        "tank/home",
    ));

    let plan = compare_plan_with_topology(plan, &graph);
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
        diagnostic.action_id == "snapshot:tank/home@before:rollback"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotRollbackPointMissing
            && diagnostic.query == "tank/home@before"
            && diagnostic.current.is_none()
    }));
}

#[test]
fn topology_comparison_warns_when_zfs_rollback_snapshot_present() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "tank/home@before": {
                  "target": "tank/home",
                  "rollback": true,
                  "recursiveRollback": true
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "zfs-snapshot:tank/home@before",
            NodeKind::ZfsSnapshot,
            "tank/home@before",
        )
        .with_property("zfs.used", "64M")
        .with_property("zfs.referenced", "5G")
        .with_property("zfs.userrefs", "1")
        .with_property("zfs.compression", "lz4"),
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
        diagnostic.action_id == "snapshot:tank/home@before:rollback"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotRollbackPointAvailable
            && diagnostic.query == "tank/home@before"
            && diagnostic.message.contains("used 64M")
            && diagnostic.message.contains("referenced 5G")
            && diagnostic.message.contains("user references 1")
            && diagnostic.message.contains("recursive rollback requested")
    }));
}

#[test]
fn topology_comparison_keeps_logical_snapshot_rollback_missing() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "before": {
                  "target": "tank/home",
                  "rollback": true
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

    assert_eq!(comparison.summary.already_satisfied_count, 0);
    assert_eq!(comparison.summary.suppressed_action_count, 0);
    assert_eq!(comparison.summary.missing_count, 1);
    assert_eq!(plan.actions.len(), 1);
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "snapshot:before:rollback"
            && diagnostic.kind == TopologyDiagnosticKind::Missing
            && diagnostic.query == "before"
    }));
}

#[test]
fn topology_comparison_warns_when_zfs_snapshot_clone_source_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "tank/home@before": {
                  "target": "tank/home",
                  "cloneTo": "tank/home-review"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(Node::new(
        "zfs-dataset:tank/home-review",
        NodeKind::ZfsDataset,
        "tank/home-review",
    ));

    let plan = compare_plan_with_topology(plan, &graph);
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
        diagnostic.action_id == "snapshot:tank/home@before:clone:tank/home-review"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotCloneSourceMissing
            && diagnostic.query == "tank/home@before"
            && diagnostic.current.is_none()
    }));
}

#[test]
fn topology_comparison_reports_zfs_snapshot_clone_source_available() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "tank/home@before": {
                  "target": "tank/home",
                  "cloneTo": "tank/home-review"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "zfs-snapshot:tank/home@before",
            NodeKind::ZfsSnapshot,
            "tank/home@before",
        )
        .with_property("zfs.used", "8M")
        .with_property("zfs.referenced", "4G")
        .with_property("zfs.userrefs", "1"),
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
        diagnostic.action_id == "snapshot:tank/home@before:clone:tank/home-review"
            && diagnostic.level == TopologyDiagnosticLevel::Info
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotCloneSourceAvailable
            && diagnostic.query == "tank/home@before"
            && diagnostic.message.contains("clone target tank/home-review")
            && diagnostic.message.contains("used 8M")
            && diagnostic.message.contains("user references 1")
    }));
}

#[test]
fn topology_comparison_warns_when_btrfs_snapshot_clone_source_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "/mnt/persist/@home-before": {
                  "target": "/mnt/persist/@home",
                  "cloneTo": "/mnt/persist/@home-review"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "btrfs-subvolume:fs-uuid:@home-review",
            NodeKind::BtrfsSubvolume,
            "@home-review",
        )
        .with_path("/mnt/persist/@home-review"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
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
        diagnostic.action_id == "snapshot:/mnt/persist/@home-before:clone:/mnt/persist/@home-review"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotCloneSourceMissing
            && diagnostic.query == "/mnt/persist/@home-before"
            && diagnostic.current.is_none()
    }));
}

#[test]
fn topology_comparison_reports_btrfs_snapshot_clone_source_available() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "/mnt/persist/@home-before": {
                  "target": "/mnt/persist/@home",
                  "cloneTo": "/mnt/persist/@home-review"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "btrfs-snapshot:fs-uuid:@home-before",
            NodeKind::BtrfsSnapshot,
            "@home-before",
        )
        .with_path("/mnt/persist/@home-before")
        .with_property("btrfs.id", "300")
        .with_property("btrfs.parent-uuid", "source-uuid"),
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
        diagnostic.action_id == "snapshot:/mnt/persist/@home-before:clone:/mnt/persist/@home-review"
            && diagnostic.level == TopologyDiagnosticLevel::Info
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotCloneSourceAvailable
            && diagnostic.query == "/mnt/persist/@home-before"
            && diagnostic
                .message
                .contains("clone target /mnt/persist/@home-review")
            && diagnostic.message.contains("subvolume id 300")
            && diagnostic.message.contains("parent UUID source-uuid")
    }));
}

#[test]
fn topology_comparison_uses_snapshot_path_for_friendly_btrfs_clone() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "before-home": {
                  "target": "/mnt/persist/@home",
                  "snapshotPath": "/mnt/persist/@home-before",
                  "cloneTo": "/mnt/persist/@home-review"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "btrfs-snapshot:fs-uuid:@home-before",
            NodeKind::BtrfsSnapshot,
            "@home-before",
        )
        .with_path("/mnt/persist/@home-before")
        .with_property("btrfs.id", "300"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    let action = plan
        .actions
        .iter()
        .find(|action| action.id == "snapshot:before-home:clone:/mnt/persist/@home-review")
        .expect("friendly clone action should remain actionable");
    assert_eq!(
        action.context.snapshot_path.as_deref(),
        Some("/mnt/persist/@home-before")
    );
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "snapshot:before-home:clone:/mnt/persist/@home-review"
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotCloneSourceAvailable
            && diagnostic.query == "/mnt/persist/@home-before"
    }));
}

#[test]
fn topology_comparison_warns_when_zfs_snapshot_rename_source_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "tank/home@old": {
                  "target": "tank/home",
                  "renameTo": "tank/home@kept"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(Node::new(
        "zfs-dataset:tank/home",
        NodeKind::ZfsDataset,
        "tank/home",
    ));

    let plan = compare_plan_with_topology(plan, &graph);
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
        diagnostic.action_id == "snapshot:tank/home@old:rename:tank/home@kept"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotRenameSourceMissing
            && diagnostic.query == "tank/home@old"
            && diagnostic.current.is_none()
    }));
}

#[test]
fn topology_comparison_warns_when_zfs_snapshot_rename_source_present() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "tank/home@old": {
                  "target": "tank/home",
                  "renameTo": "tank/home@kept"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "zfs-snapshot:tank/home@old",
            NodeKind::ZfsSnapshot,
            "tank/home@old",
        )
        .with_property("zfs.used", "12M")
        .with_property("zfs.referenced", "2G")
        .with_property("zfs.userrefs", "3"),
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
        diagnostic.action_id == "snapshot:tank/home@old:rename:tank/home@kept"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotRenameRequired
            && diagnostic.query == "tank/home@old"
            && diagnostic.message.contains("rename to tank/home@kept")
            && diagnostic.message.contains("used 12M")
            && diagnostic.message.contains("user references 3")
    }));
}

#[test]
fn topology_comparison_warns_when_btrfs_snapshot_rename_source_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "/mnt/persist/@home-old": {
                  "target": "/mnt/persist/@home",
                  "renameTo": "/mnt/persist/@home-kept"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "btrfs-subvolume:fs-uuid:@home",
            NodeKind::BtrfsSubvolume,
            "@home",
        )
        .with_path("/mnt/persist/@home"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
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
        diagnostic.action_id == "snapshot:/mnt/persist/@home-old:rename:/mnt/persist/@home-kept"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotRenameSourceMissing
            && diagnostic.query == "/mnt/persist/@home-old"
            && diagnostic.current.is_none()
    }));
}

#[test]
fn topology_comparison_warns_when_btrfs_snapshot_rename_source_present() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "/mnt/persist/@home-old": {
                  "target": "/mnt/persist/@home",
                  "renameTo": "/mnt/persist/@home-kept"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "btrfs-snapshot:fs-uuid:@home-old",
            NodeKind::BtrfsSnapshot,
            "@home-old",
        )
        .with_path("/mnt/persist/@home-old")
        .with_property("btrfs.id", "258")
        .with_property("btrfs.parent-uuid", "source-uuid"),
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
        diagnostic.action_id == "snapshot:/mnt/persist/@home-old:rename:/mnt/persist/@home-kept"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotRenameRequired
            && diagnostic.query == "/mnt/persist/@home-old"
            && diagnostic
                .message
                .contains("rename to /mnt/persist/@home-kept")
            && diagnostic.message.contains("subvolume id 258")
            && diagnostic.message.contains("parent UUID source-uuid")
    }));
}

#[test]
fn topology_comparison_suppresses_btrfs_qgroup_destroy_when_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "btrfsQgroups": {
                "0/257": {
                  "destroy": true,
                  "target": "/mnt/persist"
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
    assert_eq!(comparison.summary.already_satisfied_count, 1);
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert!(plan.actions.is_empty());
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "btrfsqgroups:0/257:destroy"
            && diagnostic.kind == TopologyDiagnosticKind::BtrfsQgroupDestroyAlreadySatisfied
            && diagnostic.query == "0/257"
    }));
}

#[test]
fn topology_comparison_reconciles_btrfs_qgroup_create() {
    let plan = plan_from_json_bytes(
        br#"{
              "btrfsQgroups": {
                "0/257": {
                  "operation": "create",
                  "target": "/mnt/persist"
                },
                "0/258": {
                  "operation": "create",
                  "target": "/mnt/persist"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("btrfs-qgroup:fs-uuid:0/257", NodeKind::BtrfsQgroup, "0/257")
            .with_property("btrfs.qgroup-id", "0/257")
            .with_property("btrfs.max-referenced", "21474836480")
            .with_property("btrfs.max-exclusive", "none")
            .with_property("btrfs.qgroup-parents", "1/0")
            .with_usage(disk_nix_model::Usage {
                used_bytes: Some(10_737_418_240),
                free_bytes: None,
                allocated_bytes: Some(2_147_483_648),
            }),
    );
    graph.add_node(
        Node::new("mount:/mnt/persist/0/258", NodeKind::Mountpoint, "0/258")
            .with_path("/mnt/persist/0/258"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 2);
    assert_eq!(comparison.summary.already_satisfied_count, 1);
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert_eq!(plan.summary.action_count, 1);
    assert!(plan
        .actions
        .iter()
        .all(|action| action.id == "btrfsqgroups:0/258:create"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "btrfsqgroups:0/257:create"
            && diagnostic.kind == TopologyDiagnosticKind::BtrfsQgroupCreateAlreadySatisfied
            && diagnostic.message.contains("qgroup id 0/257")
            && diagnostic.message.contains("max referenced 21474836480")
            && diagnostic.message.contains("referenced 10737418240 bytes")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "btrfsqgroups:0/258:create"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::BtrfsQgroupCreateRequired
            && diagnostic.message.contains("not a Btrfs qgroup")
    }));
}

#[test]
fn topology_comparison_reconciles_btrfs_qgroup_property_aliases() {
    let plan = plan_from_json_bytes(
        br#"{
              "btrfsQgroups": {
                "0/257": {
                  "target": "/mnt/persist",
                  "properties": {
                    "limit": "21474836480",
                    "maxExclusive": "unlimited"
                  }
                },
                "0/258": {
                  "target": "/mnt/persist",
                  "properties": {
                    "btrfs.max-exclusive": "10737418240"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("btrfs-qgroup:fs-uuid:0/257", NodeKind::BtrfsQgroup, "0/257")
            .with_property("btrfs.qgroup-id", "0/257")
            .with_property("btrfs.max-referenced", "21474836480")
            .with_property("btrfs.max-exclusive", "none"),
    );
    graph.add_node(
        Node::new("btrfs-qgroup:fs-uuid:0/258", NodeKind::BtrfsQgroup, "0/258")
            .with_property("btrfs.qgroup-id", "0/258")
            .with_property("btrfs.max-exclusive", "5368709120"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 3);
    assert_eq!(comparison.summary.matched_count, 3);
    assert_eq!(comparison.summary.already_satisfied_count, 2);
    assert_eq!(comparison.summary.suppressed_action_count, 2);
    assert_eq!(plan.actions.len(), 1);
    assert!(plan.actions.iter().any(|action| {
        action.id == "btrfsQgroups:0/258:set-property:btrfs.max-exclusive"
            && action.operation == Operation::SetProperty
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "btrfsQgroups:0/257:set-property:limit"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "btrfsQgroups:0/257:set-property:maxExclusive"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "btrfsQgroups:0/258:set-property:btrfs.max-exclusive"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::PropertyDiffers
            && diagnostic.message.contains("5368709120")
            && diagnostic.message.contains("10737418240")
    }));
}

#[test]
fn topology_comparison_keeps_btrfs_qgroup_destroy_when_present() {
    let plan = plan_from_json_bytes(
        br#"{
              "btrfsQgroups": {
                "0/257": {
                  "destroy": true,
                  "target": "/mnt/persist"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("btrfs-qgroup:fs-uuid:0/257", NodeKind::BtrfsQgroup, "0/257")
            .with_property("btrfs.qgroup-id", "0/257")
            .with_property("btrfs.max-referenced", "21474836480")
            .with_property("btrfs.max-exclusive", "none")
            .with_property("btrfs.qgroup-parents", "1/0")
            .with_usage(disk_nix_model::Usage {
                used_bytes: Some(10_737_418_240),
                free_bytes: None,
                allocated_bytes: Some(2_147_483_648),
            }),
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
        diagnostic.action_id == "btrfsqgroups:0/257:destroy"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::BtrfsQgroupDestroyRequired
            && diagnostic.query == "0/257"
            && diagnostic.message.contains("qgroup id 0/257")
            && diagnostic.message.contains("max referenced 21474836480")
            && diagnostic.message.contains("parents 1/0")
            && diagnostic.message.contains("referenced 10737418240 bytes")
    }));
}

#[test]
fn topology_comparison_keeps_logical_btrfs_qgroup_destroy_missing() {
    let plan = plan_from_json_bytes(
        br#"{
              "btrfsQgroups": {
                "old-qgroup": {
                  "destroy": true,
                  "target": "/mnt/persist"
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

    assert_eq!(comparison.summary.already_satisfied_count, 0);
    assert_eq!(comparison.summary.suppressed_action_count, 0);
    assert_eq!(comparison.summary.missing_count, 1);
    assert_eq!(plan.actions.len(), 1);
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "btrfsqgroups:old-qgroup:destroy"
            && diagnostic.kind == TopologyDiagnosticKind::Missing
            && diagnostic.query == "old-qgroup"
    }));
}
