#[test]
fn topology_comparison_reconciles_md_membership_updates() {
    let plan = plan_from_json_bytes(
        br#"{
              "mdRaids": {
                "root": {
                  "target": "/dev/md/root",
                  "addDevices": ["/dev/sdb1", "/dev/sdd1"],
                  "removeDevices": ["/dev/sdc1", "/dev/sde1"]
                },
                "absent": {
                  "target": "/dev/md/absent",
                  "removeDevices": ["/dev/sdf1"]
                },
                "wrong-kind": {
                  "target": "/dev/md/wrong-kind",
                  "addDevices": ["/dev/sdg1"],
                  "removeDevices": ["/dev/sdh1"]
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("md:/dev/md/root", NodeKind::MdRaid, "/dev/md/root")
            .with_path("/dev/md/root")
            .with_property("md.state", "clean")
            .with_property("md.degraded-devices", "0")
            .with_property("md.failed-devices", "0"),
    );
    graph.add_node(
        Node::new("block:/dev/sdb1", NodeKind::Partition, "/dev/sdb1").with_path("/dev/sdb1"),
    );
    graph.add_node(
        Node::new("block:/dev/sdc1", NodeKind::Partition, "/dev/sdc1").with_path("/dev/sdc1"),
    );
    graph.add_node(
        Node::new(
            "filesystem:/dev/md/wrong-kind",
            NodeKind::Filesystem,
            "/dev/md/wrong-kind",
        )
        .with_path("/dev/md/wrong-kind"),
    );
    graph.add_edge(disk_nix_model::Edge::new(
        "block:/dev/sdb1",
        "md:/dev/md/root",
        Relationship::MemberOf,
    ));
    graph.add_edge(disk_nix_model::Edge::new(
        "block:/dev/sdc1",
        "md:/dev/md/root",
        Relationship::MemberOf,
    ));

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 7);
    assert_eq!(comparison.summary.already_satisfied_count, 3);
    assert_eq!(comparison.summary.suppressed_action_count, 3);
    assert_eq!(plan.summary.action_count, 4);
    for suppressed_id in [
        "mdRaids:root:add-device:/dev/sdb1",
        "mdRaids:root:remove-device:/dev/sde1",
        "mdRaids:absent:remove-device:/dev/sdf1",
    ] {
        assert!(plan.actions.iter().all(|action| action.id != suppressed_id));
    }
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "mdRaids:root:add-device:/dev/sdb1"
            && diagnostic.kind == TopologyDiagnosticKind::MdMemberAddAlreadySatisfied
            && diagnostic
                .message
                .contains("already includes member /dev/sdb1")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "mdRaids:root:add-device:/dev/sdd1"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::MdMemberAddRequired
            && diagnostic
                .message
                .contains("does not currently include member /dev/sdd1")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "mdRaids:root:remove-device:/dev/sdc1"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::MdMemberRemoveRequired
            && diagnostic
                .message
                .contains("still includes member /dev/sdc1")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "mdRaids:root:remove-device:/dev/sde1"
            && diagnostic.kind == TopologyDiagnosticKind::MdMemberRemoveAlreadySatisfied
            && diagnostic
                .message
                .contains("no longer includes member /dev/sde1")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "mdRaids:absent:remove-device:/dev/sdf1"
            && diagnostic.kind == TopologyDiagnosticKind::MdMemberRemoveAlreadySatisfied
            && diagnostic
                .message
                .contains("array /dev/md/absent is absent")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "mdRaids:wrong-kind:add-device:/dev/sdg1"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::MdMemberAddRequired
            && diagnostic.message.contains("not an MD RAID array")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "mdRaids:wrong-kind:remove-device:/dev/sdh1"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::MdMemberRemoveRequired
            && diagnostic.message.contains("not an MD RAID array")
    }));
}

#[test]
fn topology_comparison_reconciles_md_member_replacement() {
    let plan = plan_from_json_bytes(
        br#"{
              "mdRaids": {
                "done": {
                  "target": "/dev/md/done",
                  "replaceDevices": {
                    "/dev/sdb1": "/dev/sdc1"
                  }
                },
                "pending": {
                  "target": "/dev/md/pending",
                  "replaceDevices": {
                    "/dev/sdd1": "/dev/sde1"
                  }
                },
                "both": {
                  "target": "/dev/md/both",
                  "replaceDevices": {
                    "/dev/sdf1": "/dev/sdg1"
                  }
                },
                "missing-new": {
                  "target": "/dev/md/missing-new",
                  "replaceDevices": {
                    "/dev/sdh1": "/dev/sdi1"
                  }
                },
                "wrong-kind": {
                  "target": "/dev/md/wrong-kind",
                  "replaceDevices": {
                    "/dev/sdj1": "/dev/sdk1"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    for target in [
        "/dev/md/done",
        "/dev/md/pending",
        "/dev/md/both",
        "/dev/md/missing-new",
    ] {
        graph.add_node(
            Node::new(format!("md:{target}"), NodeKind::MdRaid, target)
                .with_path(target)
                .with_property("md.state", "clean")
                .with_property("md.degraded-devices", "0")
                .with_property("md.failed-devices", "0"),
        );
    }
    graph.add_node(
        Node::new(
            "filesystem:/dev/md/wrong-kind",
            NodeKind::Filesystem,
            "/dev/md/wrong-kind",
        )
        .with_path("/dev/md/wrong-kind"),
    );

    for (device, target) in [
        ("/dev/sdc1", "/dev/md/done"),
        ("/dev/sdd1", "/dev/md/pending"),
        ("/dev/sdf1", "/dev/md/both"),
        ("/dev/sdg1", "/dev/md/both"),
    ] {
        graph.add_node(
            Node::new(format!("block:{device}"), NodeKind::Partition, device).with_path(device),
        );
        graph.add_edge(disk_nix_model::Edge::new(
            format!("block:{device}"),
            format!("md:{target}"),
            Relationship::MemberOf,
        ));
    }

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 5);
    assert_eq!(comparison.summary.already_satisfied_count, 1);
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert_eq!(plan.summary.action_count, 4);
    assert!(plan
        .actions
        .iter()
        .all(|action| action.id != "mdRaids:done:replace-device:/dev/sdb1"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "mdRaids:done:replace-device:/dev/sdb1"
            && diagnostic.kind == TopologyDiagnosticKind::MdMemberReplaceAlreadySatisfied
            && diagnostic
                .message
                .contains("already replaced member /dev/sdb1 with /dev/sdc1")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "mdRaids:pending:replace-device:/dev/sdd1"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::MdMemberReplaceRequired
            && diagnostic
                .message
                .contains("still includes old member /dev/sdd1")
            && diagnostic
                .message
                .contains("does not include replacement /dev/sde1")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "mdRaids:both:replace-device:/dev/sdf1"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::MdMemberReplaceRequired
            && diagnostic
                .message
                .contains("still includes old member /dev/sdf1")
            && diagnostic
                .message
                .contains("already includes replacement /dev/sdg1")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "mdRaids:missing-new:replace-device:/dev/sdh1"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::MdMemberReplaceRequired
            && diagnostic
                .message
                .contains("no longer includes old member /dev/sdh1")
            && diagnostic
                .message
                .contains("replacement /dev/sdi1 is not attached")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "mdRaids:wrong-kind:replace-device:/dev/sdj1"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::MdMemberReplaceRequired
            && diagnostic.message.contains("not an MD RAID array")
    }));
}

#[test]
fn topology_comparison_keeps_md_assemble_when_degraded() {
    let plan = plan_from_json_bytes(
        br#"{
              "mdRaids": {
                "existing": {
                  "operation": "assemble",
                  "target": "/dev/md/existing",
                  "devices": ["/dev/sdb1", "/dev/sdc1"]
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("md:/dev/md/existing", NodeKind::MdRaid, "/dev/md/existing")
            .with_path("/dev/md/existing")
            .with_property("md.state", "clean, degraded")
            .with_property("md.degraded-devices", "1")
            .with_property("md.failed-devices", "0"),
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
        diagnostic.action_id == "mdraids:existing:assemble"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::MdAssembleRequired
    }));
}

#[test]
fn topology_comparison_keeps_md_assemble_when_inactive() {
    let plan = plan_from_json_bytes(
        br#"{
              "mdRaids": {
                "existing": {
                  "operation": "assemble",
                  "target": "/dev/md/existing",
                  "devices": ["/dev/sdb1", "/dev/sdc1"]
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("md:/dev/md/existing", NodeKind::MdRaid, "/dev/md/existing")
            .with_path("/dev/md/existing")
            .with_property("md.state", "inactive")
            .with_property("md.degraded-devices", "0")
            .with_property("md.failed-devices", "0"),
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
        diagnostic.action_id == "mdraids:existing:assemble"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::MdAssembleRequired
    }));
}

#[test]
fn topology_comparison_reconciles_zfs_pool_create() {
    let plan = plan_from_json_bytes(
        br#"{
              "pools": {
                "tank": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/pool-vdev0"
                },
                "vault": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/pool-vdev1"
                },
                "archive": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/pool-vdev2"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("zfs-pool:tank", NodeKind::ZfsPool, "tank")
            .with_property("zfs.state", "ONLINE")
            .with_property("zfs.health", "ONLINE")
            .with_property("zfs.pool-capacity", "40%")
            .with_property("zfs.pool-fragmentation", "12%"),
    );
    graph.add_node(
        Node::new("zfs-pool:vault", NodeKind::ZfsPool, "vault")
            .with_property("zfs.state", "ONLINE")
            .with_property("zfs.health", "DEGRADED"),
    );
    graph.add_node(Node::new(
        "zfs-dataset:archive",
        NodeKind::ZfsDataset,
        "archive",
    ));

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 3);
    assert_eq!(comparison.summary.already_satisfied_count, 1);
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert_eq!(plan.summary.action_count, 2);
    assert!(plan
        .actions
        .iter()
        .all(|action| action.id != "pools:tank:create"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "pools:tank:create"
            && diagnostic.kind == TopologyDiagnosticKind::ZfsPoolCreateAlreadySatisfied
            && diagnostic.message.contains("capacity 40%")
            && diagnostic.message.contains("fragmentation 12%")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "pools:vault:create"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::ZfsPoolCreateRequired
            && diagnostic.message.contains("state=ONLINE")
            && diagnostic.message.contains("health=DEGRADED")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "pools:archive:create"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::ZfsPoolCreateRequired
            && diagnostic.message.contains("not a ZFS pool")
    }));
}

#[test]
fn topology_comparison_reconciles_zfs_pool_property_aliases() {
    let plan = plan_from_json_bytes(
        br#"{
              "pools": {
                "tank": {
                  "properties": {
                    "autotrim": true,
                    "autoExpand": "enabled",
                    "altroot": "/mnt/rescue"
                  }
                },
                "vault": {
                  "properties": {
                    "autotrim": "off"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("zfs-pool:tank", NodeKind::ZfsPool, "tank")
            .with_property("zfs.pool-autotrim", "on")
            .with_property("zfs.pool-autoexpand", "on")
            .with_property("zfs.pool-altroot", "/mnt/rescue"),
    );
    graph.add_node(
        Node::new("zfs-pool:vault", NodeKind::ZfsPool, "vault").with_property("zfs.autotrim", "on"),
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
        action.id == "pools:vault:set-property:autotrim"
            && action.operation == Operation::SetProperty
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "pools:tank:set-property:autotrim"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "pools:tank:set-property:autoExpand"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "pools:tank:set-property:altroot"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "pools:vault:set-property:autotrim"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::PropertyDiffers
            && diagnostic.message.contains("on")
            && diagnostic.message.contains("off")
    }));
}

#[test]
fn topology_comparison_suppresses_imported_online_zfs_pool() {
    let plan = plan_from_json_bytes(
        br#"{
              "pools": {
                "tank": {
                  "operation": "import"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("zfs-pool:tank", NodeKind::ZfsPool, "tank")
            .with_property("zfs.state", "ONLINE")
            .with_property("zfs.health", "ONLINE"),
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
        diagnostic.action_id == "pools:tank:import"
            && diagnostic.kind == TopologyDiagnosticKind::ZfsPoolImportAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_zfs_pool_import_when_degraded() {
    let plan = plan_from_json_bytes(
        br#"{
              "pools": {
                "tank": {
                  "operation": "import"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("zfs-pool:tank", NodeKind::ZfsPool, "tank")
            .with_property("zfs.state", "DEGRADED")
            .with_property("zfs.health", "DEGRADED"),
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
        diagnostic.action_id == "pools:tank:import"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::ZfsPoolImportRequired
    }));
}

#[test]
fn topology_comparison_reconciles_zfs_object_create() {
    let plan = plan_from_json_bytes(
        br#"{
              "datasets": {
                "tank/home": {
                  "operation": "create",
                  "properties": {
                    "compression": "zstd",
                    "mountpoint": "/home"
                  }
                },
                "tank/conflict": {
                  "operation": "create"
                }
              },
              "zvols": {
                "tank/vm/root": {
                  "operation": "create",
                  "desiredSize": "20GiB"
                },
                "tank/vm/tmp": {
                  "operation": "create",
                  "desiredSize": "20GiB"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("zfs-dataset:tank/home", NodeKind::ZfsDataset, "tank/home")
            .with_property("zfs.type", "filesystem")
            .with_property("zfs.mountpoint", "/home")
            .with_property("zfs.compression", "zstd"),
    );
    graph.add_node(
        Node::new("zvol:tank/conflict", NodeKind::Zvol, "tank/conflict")
            .with_size_bytes(8 * 1024 * 1024 * 1024)
            .with_property("zfs.type", "volume")
            .with_property("zfs.volsize", "8G"),
    );
    graph.add_node(
        Node::new("zvol:tank/vm/root", NodeKind::Zvol, "tank/vm/root")
            .with_size_bytes(20 * 1024 * 1024 * 1024)
            .with_property("zfs.type", "volume")
            .with_property("zfs.volsize", "20G")
            .with_property("zfs.compression", "zstd"),
    );
    graph.add_node(
        Node::new("zvol:tank/vm/tmp", NodeKind::Zvol, "tank/vm/tmp")
            .with_size_bytes(10 * 1024 * 1024 * 1024)
            .with_property("zfs.type", "volume")
            .with_property("zfs.volsize", "10G"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 6);
    assert_eq!(comparison.summary.already_satisfied_count, 4);
    assert_eq!(comparison.summary.suppressed_action_count, 4);
    assert_eq!(plan.summary.action_count, 2);
    assert!(plan.actions.iter().any(|action| {
        action.id == "datasets:tank/conflict:create" && action.operation == Operation::Create
    }));
    assert!(plan.actions.iter().any(|action| {
        action.id == "zvols:tank/vm/tmp:create" && action.operation == Operation::Create
    }));
    assert!(plan
        .actions
        .iter()
        .all(|action| action.id != "datasets:tank/home:set-property:compression"));
    assert!(plan
        .actions
        .iter()
        .all(|action| action.id != "datasets:tank/home:set-property:mountpoint"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "datasets:tank/home:create"
            && diagnostic.kind == TopologyDiagnosticKind::ZfsObjectCreateAlreadySatisfied
            && diagnostic.message.contains("mountpoint /home")
            && diagnostic.message.contains("compression zstd")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "zvols:tank/vm/root:create"
            && diagnostic.kind == TopologyDiagnosticKind::ZfsObjectCreateAlreadySatisfied
            && diagnostic.message.contains("volsize 20G")
            && diagnostic.message.contains("compression zstd")
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
        diagnostic.action_id == "datasets:tank/conflict:create"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::ZfsObjectCreateRequired
            && diagnostic.message.contains("not a ZFS dataset")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "zvols:tank/vm/tmp:create"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::ZfsObjectCreateRequired
            && diagnostic.message.contains("not desired size 20GiB")
    }));
}

#[test]
fn topology_comparison_reconciles_zvol_property_aliases() {
    let plan = plan_from_json_bytes(
        br#"{
              "zvols": {
                "tank/vm/root": {
                  "properties": {
                    "volSize": "20G",
                    "dedup": false,
                    "primaryCache": "metadata"
                  }
                },
                "tank/vm/tmp": {
                  "properties": {
                    "volSize": "12G"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("zvol:tank/vm/root", NodeKind::Zvol, "tank/vm/root")
            .with_property("zfs.volsize", "20G")
            .with_property("zfs.dedup", "off")
            .with_property("zfs.primarycache", "metadata"),
    );
    graph.add_node(
        Node::new("zvol:tank/vm/tmp", NodeKind::Zvol, "tank/vm/tmp")
            .with_property("zfs.volsize", "10G"),
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
        action.id == "zvols:tank/vm/tmp:set-property:volSize"
            && action.operation == Operation::SetProperty
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "zvols:tank/vm/root:set-property:volSize"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "zvols:tank/vm/root:set-property:dedup"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "zvols:tank/vm/root:set-property:primaryCache"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "zvols:tank/vm/tmp:set-property:volSize"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::PropertyDiffers
            && diagnostic.message.contains("10G")
            && diagnostic.message.contains("12G")
    }));
}

#[test]
fn topology_comparison_suppresses_zfs_dataset_destroy_when_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "datasets": {
                "tank/old": {
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
        diagnostic.action_id == "datasets:tank/old:destroy"
            && diagnostic.kind == TopologyDiagnosticKind::ZfsObjectDestroyAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_zfs_dataset_destroy_when_present() {
    let plan = plan_from_json_bytes(
        br#"{
              "datasets": {
                "tank/home": {
                  "destroy": true
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("zfs-dataset:tank/home", NodeKind::ZfsDataset, "tank/home")
            .with_property("zfs.type", "filesystem")
            .with_property("zfs.mountpoint", "/home")
            .with_property("zfs.quota", "500G")
            .with_property("zfs.encryption", "aes-256-gcm")
            .with_property("zfs.keystatus", "available"),
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
        diagnostic.action_id == "datasets:tank/home:destroy"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::ZfsObjectDestroyRequired
            && diagnostic.message.contains("mountpoint /home")
            && diagnostic.message.contains("quota 500G")
            && diagnostic.message.contains("key status available")
    }));
}

#[test]
fn topology_comparison_suppresses_zvol_destroy_when_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "zvols": {
                "tank/vm/old": {
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
        diagnostic.action_id == "zvols:tank/vm/old:destroy"
            && diagnostic.kind == TopologyDiagnosticKind::ZfsObjectDestroyAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_zvol_destroy_when_present() {
    let plan = plan_from_json_bytes(
        br#"{
              "zvols": {
                "tank/vm/root": {
                  "destroy": true
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("zvol:tank/vm/root", NodeKind::Zvol, "tank/vm/root")
            .with_property("zfs.type", "volume")
            .with_property("zfs.volsize", "80G")
            .with_property("zfs.origin", "tank/vm/base@clean")
            .with_property("zfs.compression", "zstd"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.already_satisfied_count, 0);
    assert_eq!(comparison.summary.suppressed_action_count, 0);
    assert_eq!(plan.actions.len(), 1);
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "zvols:tank/vm/root:destroy"
            && diagnostic.kind == TopologyDiagnosticKind::ZfsObjectDestroyRequired
            && diagnostic.message.contains("volsize 80G")
            && diagnostic.message.contains("origin tank/vm/base@clean")
            && diagnostic.message.contains("compression zstd")
    }));
}
