#[test]
fn topology_comparison_reconciles_zfs_promote_from_origin_metadata() {
    let plan = plan_from_json_bytes(
        br#"{
              "datasets": {
                "tank/home-review": {
                  "operation": "promote"
                },
                "tank/home-promoted": {
                  "operation": "promote"
                }
              },
              "zvols": {
                "tank/vm/root-review": {
                  "operation": "promote"
                },
                "tank/vm/root-promoted": {
                  "operation": "promote"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "zfs-dataset:tank/home-review",
            NodeKind::ZfsDataset,
            "tank/home-review",
        )
        .with_property("zfs.type", "filesystem")
        .with_property("zfs.origin", "tank/home@before"),
    );
    graph.add_node(
        Node::new(
            "zfs-dataset:tank/home-promoted",
            NodeKind::ZfsDataset,
            "tank/home-promoted",
        )
        .with_property("zfs.type", "filesystem"),
    );
    graph.add_node(
        Node::new(
            "zvol:tank/vm/root-review",
            NodeKind::Zvol,
            "tank/vm/root-review",
        )
        .with_property("zfs.type", "volume")
        .with_property("zfs.origin", "tank/vm/root@clean"),
    );
    graph.add_node(
        Node::new(
            "zvol:tank/vm/root-promoted",
            NodeKind::Zvol,
            "tank/vm/root-promoted",
        )
        .with_property("zfs.type", "volume"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 4);
    assert_eq!(comparison.summary.matched_count, 4);
    assert_eq!(comparison.summary.already_satisfied_count, 2);
    assert_eq!(comparison.summary.suppressed_action_count, 2);
    assert_eq!(plan.actions.len(), 2);
    assert!(plan
        .actions
        .iter()
        .any(|action| action.id == "datasets:tank/home-review:promote"));
    assert!(plan
        .actions
        .iter()
        .any(|action| action.id == "zvols:tank/vm/root-review:promote"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "datasets:tank/home-promoted:promote"
            && diagnostic.kind == TopologyDiagnosticKind::ZfsObjectPromoteAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "zvols:tank/vm/root-promoted:promote"
            && diagnostic.kind == TopologyDiagnosticKind::ZfsObjectPromoteAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "datasets:tank/home-review:promote"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::ZfsObjectPromoteRequired
            && diagnostic.message.contains("tank/home@before")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "zvols:tank/vm/root-review:promote"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::ZfsObjectPromoteRequired
            && diagnostic.message.contains("tank/vm/root@clean")
    }));
}

#[test]
fn topology_comparison_reconciles_zfs_rename_destinations() {
    let plan = plan_from_json_bytes(
        br#"{
              "datasets": {
                "tank/home-old": {
                  "operation": "rename",
                  "renameTo": "tank/home-new"
                },
                "tank/logs-old": {
                  "operation": "rename",
                  "renameTo": "tank/logs-new"
                },
                "tank/missing-old": {
                  "operation": "rename",
                  "renameTo": "tank/missing-new"
                }
              },
              "zvols": {
                "tank/vm/root-old": {
                  "operation": "rename",
                  "renameTo": "tank/vm/root-new"
                },
                "tank/vm/data-old": {
                  "operation": "rename",
                  "renameTo": "tank/vm/data-new"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "zfs-dataset:tank/home-old",
            NodeKind::ZfsDataset,
            "tank/home-old",
        )
        .with_property("zfs.mountpoint", "/home-old")
        .with_property("zfs.used", "10G"),
    );
    graph.add_node(
        Node::new(
            "zfs-dataset:tank/logs-new",
            NodeKind::ZfsDataset,
            "tank/logs-new",
        )
        .with_property("zfs.mountpoint", "/logs"),
    );
    graph.add_node(
        Node::new("zvol:tank/vm/root-old", NodeKind::Zvol, "tank/vm/root-old")
            .with_property("zfs.volsize", "80G")
            .with_property("zfs.origin", "tank/vm/base@clean"),
    );
    graph.add_node(Node::new(
        "zvol:tank/vm/data-new",
        NodeKind::Zvol,
        "tank/vm/data-new",
    ));

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 5);
    assert_eq!(comparison.summary.already_satisfied_count, 2);
    assert_eq!(comparison.summary.suppressed_action_count, 2);
    assert_eq!(comparison.summary.missing_count, 0);
    assert_eq!(plan.actions.len(), 3);
    assert!(plan
        .actions
        .iter()
        .any(|action| action.id == "datasets:tank/home-old:rename"));
    assert!(plan
        .actions
        .iter()
        .any(|action| action.id == "datasets:tank/missing-old:rename"));
    assert!(plan
        .actions
        .iter()
        .any(|action| action.id == "zvols:tank/vm/root-old:rename"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "datasets:tank/logs-old:rename"
            && diagnostic.kind == TopologyDiagnosticKind::ZfsObjectRenameAlreadySatisfied
            && diagnostic.message.contains("tank/logs-new")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "zvols:tank/vm/data-old:rename"
            && diagnostic.kind == TopologyDiagnosticKind::ZfsObjectRenameAlreadySatisfied
            && diagnostic.message.contains("tank/vm/data-new")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "datasets:tank/home-old:rename"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::ZfsObjectRenameRequired
            && diagnostic.message.contains("rename to tank/home-new")
            && diagnostic.message.contains("mountpoint /home-old")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "zvols:tank/vm/root-old:rename"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::ZfsObjectRenameRequired
            && diagnostic.message.contains("rename to tank/vm/root-new")
            && diagnostic.message.contains("volsize 80G")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "datasets:tank/missing-old:rename"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::ZfsObjectRenameRequired
            && diagnostic
                .message
                .contains("destination tank/missing-new is absent")
    }));
}

#[test]
fn topology_comparison_suppresses_logged_in_iscsi_session() {
    let plan = plan_from_json_bytes(
        br#"{
              "iscsiSessions": {
                "iqn.2026-06.example:storage.root": {
                  "operation": "login",
                  "portal": "192.0.2.10:3260"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "iscsi-session:12",
            NodeKind::IscsiSession,
            "iscsi-session:12",
        )
        .with_property("iscsi.target", "iqn.2026-06.example:storage.root")
        .with_property("iscsi.session-state", "LOGGED_IN"),
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
        diagnostic.action_id == "iscsisessions:iqn.2026-06.example:storage.root:login"
            && diagnostic.kind == TopologyDiagnosticKind::IscsiLoginAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_prefers_logged_in_iscsi_session_over_configured_target() {
    let plan = plan_from_json_bytes(
        br#"{
              "iscsiSessions": {
                "iqn.2026-06.example:storage.root": {
                  "operation": "login",
                  "portal": "192.0.2.10:3260"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "iscsi-target:iqn.2026-06.example:storage.root",
            NodeKind::IscsiTarget,
            "iqn.2026-06.example:storage.root",
        )
        .with_property("iscsi.node-configured", "true"),
    );
    graph.add_node(
        Node::new(
            "iscsi-session:12",
            NodeKind::IscsiSession,
            "iscsi-session:12",
        )
        .with_property("iscsi.target", "iqn.2026-06.example:storage.root")
        .with_property("iscsi.connection-state", "LOGGED IN"),
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
        diagnostic.action_id == "iscsisessions:iqn.2026-06.example:storage.root:login"
            && diagnostic.current.as_ref().is_some_and(|current| {
                current.kind == NodeKind::IscsiSession && current.id == "iscsi-session:12"
            })
            && diagnostic.kind == TopologyDiagnosticKind::IscsiLoginAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_iscsi_login_when_target_is_not_logged_in() {
    let plan = plan_from_json_bytes(
        br#"{
              "iscsiSessions": {
                "iqn.2026-06.example:storage.root": {
                  "operation": "login",
                  "portal": "192.0.2.10:3260"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "iscsi-target:iqn.2026-06.example:storage.root",
            NodeKind::IscsiTarget,
            "iqn.2026-06.example:storage.root",
        )
        .with_property("iscsi.node-configured", "true"),
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
        diagnostic.action_id == "iscsisessions:iqn.2026-06.example:storage.root:login"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::IscsiLoginRequired
    }));
}

#[test]
fn topology_comparison_suppresses_iscsi_logout_when_session_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "iscsiSessions": {
                "iqn.2026-06.example:storage.old": {
                  "operation": "logout",
                  "portal": "192.0.2.10:3260"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "iscsi-target:iqn.2026-06.example:storage.old",
            NodeKind::IscsiTarget,
            "iqn.2026-06.example:storage.old",
        )
        .with_property("iscsi.node-configured", "true"),
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
        diagnostic.action_id == "iscsisessions:iqn.2026-06.example:storage.old:logout"
            && diagnostic.kind == TopologyDiagnosticKind::IscsiLogoutAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_iscsi_logout_when_session_is_logged_in() {
    let plan = plan_from_json_bytes(
        br#"{
              "iscsiSessions": {
                "iqn.2026-06.example:storage.old": {
                  "operation": "logout",
                  "portal": "192.0.2.10:3260"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "iscsi-session:19",
            NodeKind::IscsiSession,
            "iscsi-session:19",
        )
        .with_property("iscsi.target", "iqn.2026-06.example:storage.old")
        .with_property("iscsi.connection-state", "LOGGED_IN"),
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
        diagnostic.action_id == "iscsisessions:iqn.2026-06.example:storage.old:logout"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::IscsiLogoutRequired
    }));
}

#[test]
fn topology_comparison_suppresses_bcache_detach_when_concrete_target_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "caches": {
                "/dev/bcache0": {
                  "removeDevices": ["cache-set-uuid"]
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
    assert_eq!(comparison.summary.missing_count, 0);
    assert!(plan.actions.is_empty());
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "caches:/dev/bcache0:remove-device:cache-set-uuid"
            && diagnostic.current.is_none()
            && diagnostic.kind == TopologyDiagnosticKind::BcacheDetachAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_bcache_detach_when_target_exists() {
    let plan = plan_from_json_bytes(
        br#"{
              "caches": {
                "/dev/bcache0": {
                  "removeDevices": ["cache-set-uuid"]
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("block:/dev/bcache0", NodeKind::CacheDevice, "bcache0")
            .with_path("/dev/bcache0")
            .with_property("bcache.dirty-data", "64.0M")
            .with_property("bcache.cache-mode", "writeback")
            .with_property("bcache.set-uuid", "cache-set-uuid"),
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
        diagnostic.action_id == "caches:/dev/bcache0:remove-device:cache-set-uuid"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::BcacheDetachRequired
            && diagnostic.message.contains("dirty data 64.0M")
            && diagnostic.message.contains("cache mode writeback")
    }));
}

#[test]
fn topology_comparison_reconciles_bcache_property_aliases() {
    let plan = plan_from_json_bytes(
        br#"{
              "caches": {
                "/dev/bcache0": {
                  "properties": {
                    "cacheMode": "write-back",
                    "setJournalDelayMs": "100"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("block:/dev/bcache0", NodeKind::CacheDevice, "bcache0")
            .with_path("/dev/bcache0")
            .with_property("bcache.cache-mode", "writeback")
            .with_property("bcache.set-uuid", "cache-set-uuid"),
    );
    graph.add_node(
        Node::new(
            "bcache-set:cache-set-uuid",
            NodeKind::CacheDevice,
            "cache-set-uuid",
        )
        .with_property("bcache.kind", "cache-set")
        .with_property("bcache.set-journal-delay-ms", "100"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 2);
    assert_eq!(comparison.summary.matched_count, 2);
    assert_eq!(comparison.summary.already_satisfied_count, 2);
    assert_eq!(comparison.summary.suppressed_action_count, 2);
    assert!(plan.actions.is_empty());
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "caches:/dev/bcache0:set-property:cacheMode"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "caches:/dev/bcache0:set-property:setJournalDelayMs"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
            && diagnostic
                .current
                .as_ref()
                .is_some_and(|current| current.id == "bcache-set:cache-set-uuid")
    }));
}

#[test]
fn topology_comparison_keeps_logical_bcache_detach_missing() {
    let plan = plan_from_json_bytes(
        br#"{
              "caches": {
                "root-cache": {
                  "removeDevices": ["cache-set-uuid"]
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
    assert_eq!(comparison.summary.already_satisfied_count, 0);
    assert_eq!(comparison.summary.suppressed_action_count, 0);
    assert_eq!(comparison.summary.missing_count, 1);
    assert_eq!(plan.actions.len(), 1);
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "caches:root-cache:remove-device:cache-set-uuid"
            && diagnostic.kind == TopologyDiagnosticKind::Missing
    }));
}

#[test]
fn topology_comparison_suppresses_btrfs_subvolume_destroy_when_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "btrfsSubvolumes": {
                "/mnt/persist/@old": {
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
        diagnostic.action_id == "btrfssubvolumes:/mnt/persist/@old:destroy"
            && diagnostic.kind == TopologyDiagnosticKind::BtrfsSubvolumeDestroyAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_reconciles_btrfs_subvolume_create() {
    let plan = plan_from_json_bytes(
        br#"{
              "btrfsSubvolumes": {
                "/mnt/persist/@home": {
                  "operation": "create"
                },
                "/mnt/persist/plain-dir": {
                  "operation": "create"
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
        .with_path("/mnt/persist/@home")
        .with_property("btrfs.id", "257")
        .with_property("btrfs.generation", "100")
        .with_property("btrfs.parent-id", "5")
        .with_property("btrfs.top-level", "5"),
    );
    graph.add_node(
        Node::new(
            "mount:/mnt/persist/plain-dir",
            NodeKind::Mountpoint,
            "/mnt/persist/plain-dir",
        )
        .with_path("/mnt/persist/plain-dir"),
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
        .all(|action| action.id == "btrfssubvolumes:/mnt/persist/plain-dir:create"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "btrfssubvolumes:/mnt/persist/@home:create"
            && diagnostic.kind == TopologyDiagnosticKind::BtrfsSubvolumeCreateAlreadySatisfied
            && diagnostic.message.contains("subvolume id 257")
            && diagnostic.message.contains("generation 100")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "btrfssubvolumes:/mnt/persist/plain-dir:create"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::BtrfsSubvolumeCreateRequired
            && diagnostic.message.contains("not a Btrfs subvolume")
    }));
}

#[test]
fn topology_comparison_keeps_btrfs_subvolume_destroy_when_present() {
    let plan = plan_from_json_bytes(
        br#"{
              "btrfsSubvolumes": {
                "/mnt/persist/@home": {
                  "destroy": true
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
        .with_path("/mnt/persist/@home")
        .with_property("btrfs.id", "257")
        .with_property("btrfs.generation", "100")
        .with_property("btrfs.parent-id", "5")
        .with_property("btrfs.top-level", "5"),
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
        diagnostic.action_id == "btrfssubvolumes:/mnt/persist/@home:destroy"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::BtrfsSubvolumeDestroyRequired
            && diagnostic.message.contains("subvolume id 257")
            && diagnostic.message.contains("generation 100")
            && diagnostic.message.contains("parent id 5")
    }));
}

#[test]
fn topology_comparison_keeps_logical_btrfs_subvolume_destroy_missing() {
    let plan = plan_from_json_bytes(
        br#"{
              "btrfsSubvolumes": {
                "old-home": {
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
        diagnostic.action_id == "btrfssubvolumes:old-home:destroy"
            && diagnostic.kind == TopologyDiagnosticKind::Missing
    }));
}

#[test]
fn topology_comparison_suppresses_zfs_snapshot_destroy_when_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "tank/home@old": {
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

    assert_eq!(comparison.summary.action_count, 1);
    assert_eq!(comparison.summary.already_satisfied_count, 1);
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert!(plan.actions.is_empty());
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "snapshot:tank/home@old:destroy"
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotDestroyAlreadySatisfied
            && diagnostic.query == "tank/home@old"
    }));
}

#[test]
fn topology_comparison_keeps_zfs_snapshot_destroy_when_present() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "tank/home@old": {
                  "target": "tank/home",
                  "destroy": true
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
        .with_property("zfs.used", "10M")
        .with_property("zfs.referenced", "1G")
        .with_property("zfs.compression", "zstd")
        .with_property("zfs.userrefs", "2"),
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
        diagnostic.action_id == "snapshot:tank/home@old:destroy"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotDestroyRequired
            && diagnostic.query == "tank/home@old"
            && diagnostic.message.contains("ZFS snapshot")
            && diagnostic.message.contains("used 10M")
            && diagnostic.message.contains("referenced 1G")
            && diagnostic.message.contains("user references 2")
    }));
}

#[test]
fn topology_comparison_reconciles_zfs_snapshot_holds() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "tank/home@daily": {
                  "target": "tank/home",
                  "hold": "disk-nix-retain"
                },
                "tank/home@weekly": {
                  "target": "tank/home",
                  "hold": "missing-retain"
                },
                "tank/home@old": {
                  "target": "tank/home",
                  "releaseHold": "expired-retain"
                },
                "tank/home@stale": {
                  "target": "tank/home",
                  "releaseHold": "still-held"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "zfs-snapshot:tank/home@daily",
            NodeKind::ZfsSnapshot,
            "tank/home@daily",
        )
        .with_property("zfs.holds", "disk-nix-retain")
        .with_property("zfs.hold.disk-nix-retain", "Wed Jun 24 18:00 2026"),
    );
    graph.add_node(Node::new(
        "zfs-snapshot:tank/home@weekly",
        NodeKind::ZfsSnapshot,
        "tank/home@weekly",
    ));
    graph.add_node(Node::new(
        "zfs-snapshot:tank/home@old",
        NodeKind::ZfsSnapshot,
        "tank/home@old",
    ));
    graph.add_node(
        Node::new(
            "zfs-snapshot:tank/home@stale",
            NodeKind::ZfsSnapshot,
            "tank/home@stale",
        )
        .with_property("zfs.hold.still-held", "Wed Jun 24 17:00 2026"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 4);
    assert_eq!(comparison.summary.matched_count, 4);
    assert_eq!(comparison.summary.already_satisfied_count, 2);
    assert_eq!(comparison.summary.suppressed_action_count, 2);
    assert_eq!(plan.actions.len(), 2);
    assert!(plan.actions.iter().any(|action| {
        action.id == "snapshot:tank/home@weekly:hold:missing-retain"
            && action.operation == Operation::SetProperty
    }));
    assert!(plan.actions.iter().any(|action| {
        action.id == "snapshot:tank/home@stale:release-hold:still-held"
            && action.operation == Operation::SetProperty
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "snapshot:tank/home@daily:hold:disk-nix-retain"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "snapshot:tank/home@old:release-hold:expired-retain"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "snapshot:tank/home@weekly:hold:missing-retain"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::PropertyDiffers
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "snapshot:tank/home@stale:release-hold:still-held"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::PropertyDiffers
    }));
}
