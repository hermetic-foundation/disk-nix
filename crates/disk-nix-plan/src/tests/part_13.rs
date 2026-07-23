#[test]
fn topology_comparison_suppresses_dm_map_destroy_when_map_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "dmMaps": {
                "oldmap": {
                  "operation": "destroy",
                  "target": "/dev/mapper/oldmap"
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
        diagnostic.action_id == "dmmaps:oldmap:destroy"
            && diagnostic.current.is_none()
            && diagnostic.kind == TopologyDiagnosticKind::DmMapDestroyAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_dm_map_destroy_when_map_exists() {
    let plan = plan_from_json_bytes(
        br#"{
              "dmMaps": {
                "oldmap": {
                  "operation": "destroy",
                  "target": "/dev/mapper/oldmap"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("dm:oldmap", NodeKind::DeviceMapper, "oldmap")
            .with_path("/dev/mapper/oldmap")
            .with_property("dm.open-count", "2"),
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
        diagnostic.action_id == "dmmaps:oldmap:destroy"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::DmMapDestroyRequired
            && diagnostic
                .message
                .contains("still present with open count 2")
    }));
}

#[test]
fn topology_comparison_reconciles_dm_map_rename_destinations() {
    let plan = plan_from_json_bytes(
        br#"{
              "dmMaps": {
                "cryptswap": {
                  "operation": "rename",
                  "target": "/dev/mapper/cryptswap",
                  "renameTo": "cryptswap-retired"
                },
                "cryptold": {
                  "operation": "rename",
                  "target": "/dev/mapper/cryptold",
                  "renameTo": "/dev/mapper/cryptnew"
                },
                "missing": {
                  "operation": "rename",
                  "target": "/dev/mapper/missing",
                  "renameTo": "missing-new"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("dm:cryptswap", NodeKind::DeviceMapper, "cryptswap")
            .with_path("/dev/mapper/cryptswap")
            .with_property("dm.open-count", "1")
            .with_property("dm.uuid", "CRYPT-LUKS2-root"),
    );
    graph.add_node(
        Node::new("dm:cryptnew", NodeKind::DeviceMapper, "cryptnew")
            .with_path("/dev/mapper/cryptnew")
            .with_property("dm.open-count", "0"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 3);
    assert_eq!(comparison.summary.already_satisfied_count, 1);
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert_eq!(comparison.summary.missing_count, 0);
    assert_eq!(plan.actions.len(), 2);
    assert!(plan
        .actions
        .iter()
        .any(|action| action.id == "dmmaps:cryptswap:rename"));
    assert!(plan
        .actions
        .iter()
        .any(|action| action.id == "dmmaps:missing:rename"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "dmmaps:cryptold:rename"
            && diagnostic.kind == TopologyDiagnosticKind::DmMapRenameAlreadySatisfied
            && diagnostic.message.contains("/dev/mapper/cryptnew")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "dmmaps:cryptswap:rename"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::DmMapRenameRequired
            && diagnostic
                .message
                .contains("rename to /dev/mapper/cryptswap-retired")
            && diagnostic.message.contains("open count 1")
            && diagnostic.message.contains("uuid CRYPT-LUKS2-root")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "dmmaps:missing:rename"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::DmMapRenameRequired
            && diagnostic
                .message
                .contains("destination /dev/mapper/missing-new is absent")
    }));
}

#[test]
fn topology_comparison_suppresses_multipath_destroy_when_map_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "multipathMaps": {
                "mpath-old": {
                  "operation": "destroy",
                  "target": "mpath-old"
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
        diagnostic.action_id == "multipathmaps:mpath-old:destroy"
            && diagnostic.current.is_none()
            && diagnostic.kind == TopologyDiagnosticKind::MultipathDestroyAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_multipath_destroy_when_map_exists() {
    let plan = plan_from_json_bytes(
        br#"{
              "multipathMaps": {
                "mpatha": {
                  "operation": "destroy",
                  "target": "/dev/mapper/mpatha"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("multipath:mpatha", NodeKind::MultipathDevice, "mpatha")
            .with_path("/dev/mapper/mpatha")
            .with_property("multipath.wwid", "3600508b400105e210000900000490000")
            .with_property("multipath.dm", "dm-3"),
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
        diagnostic.action_id == "multipathmaps:mpatha:destroy"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::MultipathDestroyRequired
            && diagnostic
                .message
                .contains("WWID 3600508b400105e210000900000490000")
    }));
}

#[test]
fn topology_comparison_reconciles_multipath_path_membership() {
    let plan = plan_from_json_bytes(
        br#"{
              "multipathMaps": {
                "mpatha": {
                  "target": "/dev/mapper/mpatha",
                  "addDevices": ["/dev/sdb", "/dev/sdd"],
                  "removeDevices": ["/dev/sdc", "/dev/sde"]
                },
                "absent": {
                  "target": "/dev/mapper/absent",
                  "addDevices": ["/dev/sdi"],
                  "removeDevices": ["/dev/sdf"]
                },
                "wrong-kind": {
                  "target": "/dev/mapper/wrong-kind",
                  "addDevices": ["/dev/sdg"],
                  "removeDevices": ["/dev/sdh"]
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("multipath:mpatha", NodeKind::MultipathDevice, "mpatha")
            .with_path("/dev/mapper/mpatha")
            .with_property("multipath.wwid", "3600508b400105e210000900000490000"),
    );
    graph.add_node(
        Node::new("block:/dev/sdb", NodeKind::PhysicalDisk, "/dev/sdb").with_path("/dev/sdb"),
    );
    graph.add_node(
        Node::new("block:/dev/sdc", NodeKind::PhysicalDisk, "/dev/sdc").with_path("/dev/sdc"),
    );
    graph.add_node(
        Node::new(
            "dm:/dev/mapper/wrong-kind",
            NodeKind::DeviceMapper,
            "/dev/mapper/wrong-kind",
        )
        .with_path("/dev/mapper/wrong-kind"),
    );
    graph.add_edge(disk_nix_model::Edge::new(
        "block:/dev/sdb",
        "multipath:mpatha",
        Relationship::Backs,
    ));
    graph.add_edge(disk_nix_model::Edge::new(
        "block:/dev/sdc",
        "multipath:mpatha",
        Relationship::Backs,
    ));

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 8);
    assert_eq!(comparison.summary.already_satisfied_count, 3);
    assert_eq!(comparison.summary.suppressed_action_count, 3);
    assert_eq!(plan.summary.action_count, 5);
    for suppressed_id in [
        "multipathMaps:mpatha:add-device:/dev/sdb",
        "multipathMaps:mpatha:remove-device:/dev/sde",
        "multipathMaps:absent:remove-device:/dev/sdf",
    ] {
        assert!(plan.actions.iter().all(|action| action.id != suppressed_id));
    }
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "multipathMaps:mpatha:add-device:/dev/sdb"
            && diagnostic.kind == TopologyDiagnosticKind::MultipathPathAddAlreadySatisfied
            && diagnostic
                .message
                .contains("already includes path /dev/sdb")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "multipathMaps:mpatha:add-device:/dev/sdd"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::MultipathPathAddRequired
            && diagnostic
                .message
                .contains("does not currently include path /dev/sdd")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "multipathMaps:mpatha:remove-device:/dev/sdc"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::MultipathPathRemoveRequired
            && diagnostic.message.contains("still includes path /dev/sdc")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "multipathMaps:mpatha:remove-device:/dev/sde"
            && diagnostic.kind == TopologyDiagnosticKind::MultipathPathRemoveAlreadySatisfied
            && diagnostic
                .message
                .contains("no longer includes path /dev/sde")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "multipathMaps:absent:remove-device:/dev/sdf"
            && diagnostic.kind == TopologyDiagnosticKind::MultipathPathRemoveAlreadySatisfied
            && diagnostic
                .message
                .contains("map /dev/mapper/absent is absent")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "multipathMaps:absent:add-device:/dev/sdi"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::MultipathPathAddRequired
            && diagnostic
                .message
                .contains("path /dev/sdi cannot be confirmed attached")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "multipathMaps:wrong-kind:add-device:/dev/sdg"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::MultipathPathAddRequired
            && diagnostic.message.contains("not a multipath map")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "multipathMaps:wrong-kind:remove-device:/dev/sdh"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::MultipathPathRemoveRequired
            && diagnostic.message.contains("not a multipath map")
    }));
}

#[test]
fn topology_comparison_suppresses_loop_create_when_mapping_matches() {
    let plan = plan_from_json_bytes(
        br#"{
              "loopDevices": {
                "/dev/loop7": {
                  "operation": "create",
                  "device": "/var/lib/images/root.img"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("block:/dev/loop7", NodeKind::LoopDevice, "/dev/loop7")
            .with_path("/dev/loop7")
            .with_property("loop.back-file", "/var/lib/images/root.img"),
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
        diagnostic.action_id == "loopdevices:/dev/loop7:create"
            && diagnostic.kind == TopologyDiagnosticKind::LoopCreateAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_loop_create_when_mapping_differs() {
    let plan = plan_from_json_bytes(
        br#"{
              "loopDevices": {
                "/dev/loop7": {
                  "operation": "create",
                  "device": "/var/lib/images/root.img"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("block:/dev/loop7", NodeKind::LoopDevice, "/dev/loop7")
            .with_path("/dev/loop7")
            .with_property("loop.back-file", "/var/lib/images/other.img"),
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
        diagnostic.action_id == "loopdevices:/dev/loop7:create"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LoopCreateConflict
    }));
}

#[test]
fn topology_comparison_keeps_loop_create_when_mapping_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "loopDevices": {
                "/dev/loop7": {
                  "operation": "create",
                  "device": "/var/lib/images/root.img"
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
    assert_eq!(comparison.summary.missing_count, 0);
    assert_eq!(plan.actions.len(), 1);
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "loopdevices:/dev/loop7:create"
            && diagnostic.level == TopologyDiagnosticLevel::Info
            && diagnostic.kind == TopologyDiagnosticKind::LoopCreateRequired
    }));
}

#[test]
fn topology_comparison_suppresses_loop_destroy_when_mapping_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "loopDevices": {
                "/dev/loop9": {
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

    assert_eq!(comparison.summary.action_count, 1);
    assert_eq!(comparison.summary.already_satisfied_count, 1);
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert_eq!(comparison.summary.missing_count, 0);
    assert!(plan.actions.is_empty());
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "loopdevices:/dev/loop9:destroy"
            && diagnostic.current.is_none()
            && diagnostic.kind == TopologyDiagnosticKind::LoopDetachAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_loop_destroy_when_mapping_exists() {
    let plan = plan_from_json_bytes(
        br#"{
              "loopDevices": {
                "/dev/loop9": {
                  "operation": "destroy"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("block:/dev/loop9", NodeKind::LoopDevice, "/dev/loop9")
            .with_path("/dev/loop9")
            .with_property("loop.back-file", "/var/lib/images/old.img"),
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
        diagnostic.action_id == "loopdevices:/dev/loop9:destroy"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LoopDetachRequired
    }));
}

#[test]
fn topology_comparison_suppresses_nvme_namespace_attach_when_path_exists() {
    let plan = plan_from_json_bytes(
        br#"{
              "nvmeNamespaces": {
                "root-ns": {
                  "operation": "attach",
                  "target": "/dev/nvme0",
                  "device": "/dev/nvme0n1",
                  "namespaceId": "1",
                  "controllers": "0x1"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "block:/dev/nvme0n1",
            NodeKind::NvmeNamespace,
            "/dev/nvme0n1",
        )
        .with_path("/dev/nvme0n1")
        .with_property("nvme.namespace-id", "1"),
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
        diagnostic.action_id == "nvmenamespaces:root-ns:attach"
            && diagnostic.kind == TopologyDiagnosticKind::NvmeNamespaceAttachAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_nvme_namespace_attach_when_path_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "nvmeNamespaces": {
                "root-ns": {
                  "operation": "attach",
                  "target": "/dev/nvme0",
                  "device": "/dev/nvme0n1",
                  "namespaceId": "1",
                  "controllers": "0x1"
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
    assert_eq!(comparison.summary.missing_count, 0);
    assert_eq!(plan.actions.len(), 1);
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "nvmenamespaces:root-ns:attach"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::NvmeNamespaceAttachRequired
    }));
}

#[test]
fn topology_comparison_suppresses_nvme_namespace_detach_when_path_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "nvmeNamespaces": {
                "old-ns": {
                  "operation": "detach",
                  "target": "/dev/nvme1",
                  "device": "/dev/nvme1n1",
                  "namespaceId": "2",
                  "controllers": "0x2"
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
        diagnostic.action_id == "nvmenamespaces:old-ns:detach"
            && diagnostic.current.is_none()
            && diagnostic.kind == TopologyDiagnosticKind::NvmeNamespaceDetachAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_nvme_namespace_detach_when_path_exists() {
    let plan = plan_from_json_bytes(
        br#"{
              "nvmeNamespaces": {
                "old-ns": {
                  "operation": "detach",
                  "target": "/dev/nvme1",
                  "device": "/dev/nvme1n1",
                  "namespaceId": "2",
                  "controllers": "0x2"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "block:/dev/nvme1n1",
            NodeKind::NvmeNamespace,
            "/dev/nvme1n1",
        )
        .with_path("/dev/nvme1n1")
        .with_property("nvme.namespace-id", "2"),
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
        diagnostic.action_id == "nvmenamespaces:old-ns:detach"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::NvmeNamespaceDetachRequired
    }));
}

#[test]
fn topology_comparison_suppresses_lun_attach_when_path_exists() {
    let plan = plan_from_json_bytes(
        br#"{
              "luns": {
                "iqn.2026-06.example:storage/root:0": {
                  "operation": "attach",
                  "device": "/dev/disk/by-path/ip-192.0.2.10-lun-0"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("lun:0", NodeKind::Lun, "0")
            .with_path("/dev/disk/by-path/ip-192.0.2.10-lun-0")
            .with_property("iscsi.attached-disk", "sdb"),
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
        diagnostic.action_id == "luns:iqn.2026-06.example:storage/root:0:attach"
            && diagnostic.kind == TopologyDiagnosticKind::LunAttachAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_reports_partially_suppressed_reconciliation_groups() {
    let lun_path = "/dev/disk/by-path/ip-192.0.2.10-lun-0";
    let plan = plan_from_json_bytes(
        br#"{
              "luns": {
                "attach-root": {
                  "operation": "attach",
                  "device": "/dev/disk/by-path/ip-192.0.2.10-lun-0"
                },
                "grow-root": {
                  "operation": "grow",
                  "device": "/dev/disk/by-path/ip-192.0.2.10-lun-0",
                  "desiredSize": "200GiB"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("lun:0", NodeKind::Lun, "0")
            .with_path(lun_path)
            .with_size_bytes(100 * 1024 * 1024 * 1024)
            .with_property("iscsi.attached-disk", "sdb"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 2);
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert_eq!(comparison.summary.reconciliation_group_count, 1);
    assert_eq!(comparison.summary.partially_suppressed_group_count, 1);
    assert_eq!(plan.actions.len(), 1);
    assert_eq!(plan.actions[0].id, "luns:grow-root:grow");

    let group = comparison
        .reconciliation_groups
        .iter()
        .find(|group| group.identity == lun_path)
        .expect("shared LUN path reconciliation group exists");
    assert_eq!(group.action_count, 2);
    assert_eq!(group.planned_count, 1);
    assert_eq!(group.suppressed_count, 1);
    assert!(group.partially_suppressed);
    assert_eq!(group.planned_action_ids, vec!["luns:grow-root:grow"]);
    assert_eq!(group.suppressed_action_ids, vec!["luns:attach-root:attach"]);
    assert!(group.recommendation.contains("fresh topology"));

    let json = serde_json::to_value(comparison).expect("comparison serializes");
    assert_eq!(json["summary"]["reconciliationGroupCount"], 1);
    assert_eq!(json["summary"]["partiallySuppressedGroupCount"], 1);
    assert_eq!(json["reconciliationGroups"][0]["identity"], lun_path);
    assert_eq!(json["reconciliationGroups"][0]["partiallySuppressed"], true);
}

#[test]
fn topology_comparison_groups_nfs_export_and_client_mount_reconciliation() {
    let plan = plan_from_json_bytes(
        br#"{
              "exports": {
                "/srv/share": {
                  "operation": "export",
                  "client": "192.0.2.0/24",
                  "options": "rw,sync,no_subtree_check"
                }
              },
              "nfs": {
                "mounts": {
                  "/mnt/share": {
                    "operation": "mount",
                    "source": "nas.example.com:/srv/share",
                    "fsType": "nfs4"
                  }
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

    assert_eq!(comparison.summary.action_count, 2);
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert_eq!(comparison.summary.partially_suppressed_group_count, 1);
    assert_eq!(plan.actions.len(), 1);
    assert_eq!(plan.actions[0].id, "nfs.mounts:/mnt/share:mount");

    let group = comparison
        .reconciliation_groups
        .iter()
        .find(|group| group.identity == "nfs-export:/srv/share")
        .expect("NFS export and mount reconciliation group exists");
    assert_eq!(
        group.planned_action_ids,
        vec!["nfs.mounts:/mnt/share:mount"]
    );
    assert_eq!(
        group.suppressed_action_ids,
        vec!["exports:/srv/share:export"]
    );
    assert!(group.partially_suppressed);
}
