#[test]
fn topology_comparison_reconciles_lvm_cache_property_aliases() {
    let plan = plan_from_json_bytes(
        br#"{
              "lvmCaches": {
                "vg0/root": {
                  "properties": {
                    "cacheMode": "write-through",
                    "cachePolicy": "smq"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("lvm-lv:vg0/root", NodeKind::LvmCache, "vg0/root")
            .with_property("lvm.cache-mode", "writethrough")
            .with_property("lvm.cache-policy", "smq"),
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
        diagnostic.action_id == "lvmCaches:vg0/root:set-property:cacheMode"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "lvmCaches:vg0/root:set-property:cachePolicy"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_lvm_cache_detach_missing_without_origin() {
    let plan = plan_from_json_bytes(
        br#"{
              "lvmCaches": {
                "vg0/root": {
                  "removeDevices": ["vg0/root-cache"]
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
    assert_eq!(comparison.summary.missing_count, 0);
    assert_eq!(plan.actions.len(), 1);
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "lvmCaches:vg0/root:remove-device:vg0/root-cache"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LvmCacheDetachRequired
            && diagnostic
                .message
                .contains("LVM cache origin vg0/root is absent")
            && diagnostic.message.contains("cache device vg0/root-cache")
            && diagnostic.query == "vg0/root"
    }));
}

#[test]
fn topology_comparison_suppresses_vdo_start_when_normal() {
    let plan = plan_from_json_bytes(
        br#"{
              "vdoVolumes": {
                "archive": {
                  "operation": "start"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("vdo:archive", NodeKind::VdoVolume, "archive")
            .with_path("/dev/mapper/archive")
            .with_property("vdo.operating-mode", "normal"),
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
        diagnostic.action_id == "vdovolumes:archive:start"
            && diagnostic.kind == TopologyDiagnosticKind::VdoStartAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_suppresses_lvm_vdo_start_when_normal() {
    let plan = plan_from_json_bytes(
        br#"{
              "vdoVolumes": {
                "vg0/archive": {
                  "operation": "start"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("lvm:vg0/archive", NodeKind::VdoVolume, "vg0/archive")
            .with_path("/dev/vg0/archive")
            .with_property("lvm.vdo-operating-mode", "normal"),
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
        diagnostic.action_id == "vdovolumes:vg0/archive:start"
            && diagnostic.kind == TopologyDiagnosticKind::VdoStartAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_vdo_start_when_not_normal() {
    let plan = plan_from_json_bytes(
        br#"{
              "vdoVolumes": {
                "archive": {
                  "operation": "start"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("vdo:archive", NodeKind::VdoVolume, "archive")
            .with_path("/dev/mapper/archive")
            .with_property("vdo.operating-mode", "recovering"),
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
        diagnostic.action_id == "vdovolumes:archive:start"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::VdoStartRequired
    }));
}

#[test]
fn topology_comparison_reconciles_absent_vdo_start_and_stop() {
    let plan = plan_from_json_bytes(
        br#"{
              "vdoVolumes": {
                "archive": {
                  "operation": "start"
                },
                "old": {
                  "operation": "stop"
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
        .any(|action| action.id == "vdovolumes:archive:start"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "vdovolumes:archive:start"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::VdoStartRequired
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "vdovolumes:old:stop"
            && diagnostic.level == TopologyDiagnosticLevel::Info
            && diagnostic.kind == TopologyDiagnosticKind::VdoStopAlreadySatisfied
            && diagnostic.current.is_none()
    }));
}

#[test]
fn topology_comparison_suppresses_vdo_stop_when_stopped() {
    let plan = plan_from_json_bytes(
        br#"{
              "vdoVolumes": {
                "archive": {
                  "operation": "stop"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("vdo:archive", NodeKind::VdoVolume, "archive")
            .with_path("/dev/mapper/archive")
            .with_property("vdo.operating-mode", "stopped"),
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
        diagnostic.action_id == "vdovolumes:archive:stop"
            && diagnostic.kind == TopologyDiagnosticKind::VdoStopAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_suppresses_lvm_vdo_stop_when_not_running() {
    let plan = plan_from_json_bytes(
        br#"{
              "vdoVolumes": {
                "vg0/archive": {
                  "operation": "stop"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("lvm:vg0/archive", NodeKind::VdoVolume, "vg0/archive")
            .with_path("/dev/vg0/archive")
            .with_property("lvm.vdo-operating-mode", "not running"),
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
        diagnostic.action_id == "vdovolumes:vg0/archive:stop"
            && diagnostic.kind == TopologyDiagnosticKind::VdoStopAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_reports_vdo_create_target_metadata() {
    let plan = plan_from_json_bytes(
        br#"{
              "vdoVolumes": {
                "archive": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/vdo-backing",
                  "desiredSize": "2TiB"
                },
                "data": {
                  "operation": "create",
                  "target": "/dev/disk/by-label/data"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("vdo:archive", NodeKind::VdoVolume, "archive")
            .with_path("/dev/mapper/archive")
            .with_property("vdo.operating-mode", "normal")
            .with_property("vdo.storage-device", "/dev/disk/by-id/vdo-backing")
            .with_property("vdo.logical-size", "2TiB")
            .with_property("vdo.write-policy", "sync"),
    );
    graph.add_node(
        Node::new(
            "filesystem:/dev/disk/by-label/data",
            NodeKind::Filesystem,
            "/dev/disk/by-label/data",
        )
        .with_path("/dev/disk/by-label/data")
        .with_property("filesystem.type", "xfs"),
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
    assert_eq!(plan.actions.len(), 2);
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "vdovolumes:archive:create"
            && diagnostic.query == "archive"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::VdoCreateTargetPresent
            && diagnostic.message.contains("operating mode normal")
            && diagnostic
                .message
                .contains("backing device /dev/disk/by-id/vdo-backing")
            && diagnostic.message.contains("logical size 2TiB")
            && diagnostic.message.contains("write policy sync")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "vdovolumes:data:create"
            && diagnostic.query == "/dev/disk/by-label/data"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::VdoCreateTargetPresent
            && diagnostic.message.contains("filesystem")
    }));
}

#[test]
fn topology_comparison_reconciles_vdo_property_aliases() {
    let plan = plan_from_json_bytes(
        br#"{
              "vdoVolumes": {
                "archive": {
                  "properties": {
                    "writePolicy": "sync",
                    "compression": "enabled",
                    "deduplication": "disabled"
                  }
                },
                "vg0/lv": {
                  "properties": {
                    "writePolicy": "async"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("vdo:archive", NodeKind::VdoVolume, "archive")
            .with_property("vdo.write-policy", "sync")
            .with_property("vdo.compression", "true")
            .with_property("vdo.deduplication", "off"),
    );
    graph.add_node(
        Node::new("lvm:vg0/lv", NodeKind::VdoVolume, "vg0/lv")
            .with_property("lvm.vdo-write-policy", "sync"),
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
        action.id == "vdoVolumes:vg0/lv:set-property:writePolicy"
            && action.operation == Operation::SetProperty
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "vdoVolumes:archive:set-property:writePolicy"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "vdoVolumes:archive:set-property:compression"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "vdoVolumes:archive:set-property:deduplication"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "vdoVolumes:vg0/lv:set-property:writePolicy"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::PropertyDiffers
            && diagnostic.message.contains("sync")
            && diagnostic.message.contains("async")
    }));
}

#[test]
fn topology_comparison_keeps_vdo_stop_when_normal() {
    let plan = plan_from_json_bytes(
        br#"{
              "vdoVolumes": {
                "archive": {
                  "operation": "stop"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("vdo:archive", NodeKind::VdoVolume, "archive")
            .with_path("/dev/mapper/archive")
            .with_property("vdo.operating-mode", "normal"),
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
        diagnostic.action_id == "vdovolumes:archive:stop"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::VdoStopRequired
    }));
}

#[test]
fn topology_comparison_suppresses_vdo_destroy_when_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "vdoVolumes": {
                "archive": {
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
        diagnostic.action_id == "vdovolumes:archive:destroy"
            && diagnostic.kind == TopologyDiagnosticKind::VdoDestroyAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_vdo_destroy_when_present() {
    let plan = plan_from_json_bytes(
        br#"{
              "vdoVolumes": {
                "archive": {
                  "destroy": true
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("vdo:archive", NodeKind::VdoVolume, "archive")
            .with_path("/dev/mapper/archive")
            .with_property("vdo.operating-mode", "normal")
            .with_property("vdo.storage-device", "/dev/sdb")
            .with_property("vdo.logical-size", "4TiB")
            .with_property("vdo.physical-size", "1TiB")
            .with_property("vdo.write-policy", "sync"),
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
        diagnostic.action_id == "vdovolumes:archive:destroy"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::VdoDestroyRequired
            && diagnostic.message.contains("operating mode normal")
            && diagnostic.message.contains("backing device /dev/sdb")
            && diagnostic.message.contains("logical size 4TiB")
            && diagnostic.message.contains("write policy sync")
    }));
}

#[test]
fn topology_comparison_reports_lvm_vdo_destroy_metadata() {
    let plan = plan_from_json_bytes(
        br#"{
              "vdoVolumes": {
                "vg0/archive": {
                  "destroy": true
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("lvm:vg0/archive", NodeKind::VdoVolume, "vg0/archive")
            .with_property("lvm.vdo-operating-mode", "normal")
            .with_property("lvm.vdo-used-size", "128.00m")
            .with_property("lvm.vdo-saving-percent", "72.50"),
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
        diagnostic.action_id == "vdovolumes:vg0/archive:destroy"
            && diagnostic.kind == TopologyDiagnosticKind::VdoDestroyRequired
            && diagnostic.message.contains("used 128.00m")
            && diagnostic.message.contains("saving 72.50")
    }));
}

#[test]
fn topology_comparison_reconciles_vdo_grow_from_logical_size_metadata() {
    let plan = plan_from_json_bytes(
        br#"{
              "vdoVolumes": {
                "archive": {
                  "operation": "grow",
                  "desiredSize": "2TiB"
                },
                "small": {
                  "operation": "grow",
                  "desiredSize": "4TiB"
                },
                "unknown": {
                  "operation": "grow",
                  "desiredSize": "4TiB"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("vdo:archive", NodeKind::VdoVolume, "archive")
            .with_path("/dev/mapper/archive")
            .with_property("vdo.logical-size", "4TiB"),
    );
    graph.add_node(
        Node::new("vdo:small", NodeKind::VdoVolume, "small")
            .with_path("/dev/mapper/small")
            .with_property("vdo.logical-size", "1TiB"),
    );
    graph.add_node(
        Node::new("vdo:unknown", NodeKind::VdoVolume, "unknown").with_path("/dev/mapper/unknown"),
    );

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
        .all(|action| action.id != "vdovolumes:archive:grow"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "vdovolumes:archive:grow"
            && diagnostic.level == TopologyDiagnosticLevel::Info
            && diagnostic.kind == TopologyDiagnosticKind::SizeAlreadySatisfied
            && diagnostic
                .message
                .contains("logical size 4TiB already satisfies desired size 2TiB")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "vdovolumes:small:grow"
            && diagnostic.level == TopologyDiagnosticLevel::Info
            && diagnostic.kind == TopologyDiagnosticKind::SizeBelowDesired
            && diagnostic
                .message
                .contains("logical size 1TiB is below desired size 4TiB")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "vdovolumes:unknown:grow"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::VdoGrowRequired
            && diagnostic
                .message
                .contains("current logical size is unknown")
    }));
}

#[test]
fn topology_comparison_keeps_absent_vdo_grow_actionable() {
    let plan = plan_from_json_bytes(
        br#"{
              "vdoVolumes": {
                "archive": {
                  "operation": "grow",
                  "desiredSize": "4TiB"
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
        diagnostic.action_id == "vdovolumes:archive:grow"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::VdoGrowRequired
            && diagnostic.message.contains("grow to 4TiB")
            && diagnostic
                .message
                .contains("requires an existing VDO volume")
    }));
}

#[test]
fn topology_comparison_suppresses_md_assemble_when_clean() {
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
            .with_property("md.state", "clean")
            .with_property("md.degraded-devices", "0")
            .with_property("md.failed-devices", "0"),
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
        diagnostic.action_id == "mdraids:existing:assemble"
            && diagnostic.kind == TopologyDiagnosticKind::MdAssembleAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_reconciles_md_create() {
    let plan = plan_from_json_bytes(
        br#"{
              "mdRaids": {
                "existing": {
                  "operation": "create",
                  "target": "/dev/md/existing",
                  "level": "1",
                  "devices": ["/dev/sdb1", "/dev/sdc1"]
                },
                "degraded": {
                  "operation": "create",
                  "target": "/dev/md/degraded",
                  "level": "1",
                  "devices": ["/dev/sdd1", "/dev/sde1"]
                },
                "wrong-kind": {
                  "operation": "create",
                  "target": "/dev/md/wrong-kind",
                  "level": "1",
                  "devices": ["/dev/sdf1", "/dev/sdg1"]
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("md:/dev/md/existing", NodeKind::MdRaid, "/dev/md/existing")
            .with_path("/dev/md/existing")
            .with_property("md.state", "clean")
            .with_property("md.degraded-devices", "0")
            .with_property("md.failed-devices", "0"),
    );
    graph.add_node(
        Node::new("md:/dev/md/degraded", NodeKind::MdRaid, "/dev/md/degraded")
            .with_path("/dev/md/degraded")
            .with_property("md.state", "clean, degraded")
            .with_property("md.degraded-devices", "1")
            .with_property("md.failed-devices", "0"),
    );
    graph.add_node(
        Node::new(
            "filesystem:/dev/md/wrong-kind",
            NodeKind::Filesystem,
            "/dev/md/wrong-kind",
        )
        .with_path("/dev/md/wrong-kind"),
    );

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
        .all(|action| action.id != "mdraids:existing:create"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "mdraids:existing:create"
            && diagnostic.kind == TopologyDiagnosticKind::MdCreateAlreadySatisfied
            && diagnostic.message.contains("cleanly active")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "mdraids:degraded:create"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::MdCreateRequired
            && diagnostic.message.contains("state=clean, degraded")
            && diagnostic.message.contains("degradedDevices=1")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "mdraids:wrong-kind:create"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::MdCreateRequired
            && diagnostic.message.contains("not an MD RAID array")
    }));
}

#[test]
fn topology_comparison_reconciles_md_stop() {
    let plan = plan_from_json_bytes(
        br#"{
              "mdRaids": {
                "absent": {
                  "operation": "stop",
                  "target": "/dev/md/absent"
                },
                "inactive": {
                  "operation": "stop",
                  "target": "/dev/md/inactive"
                },
                "active": {
                  "operation": "stop",
                  "target": "/dev/md/active"
                },
                "unknown": {
                  "operation": "stop",
                  "target": "/dev/md/unknown"
                },
                "wrong-kind": {
                  "operation": "stop",
                  "target": "/dev/md/wrong-kind"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("md:/dev/md/inactive", NodeKind::MdRaid, "/dev/md/inactive")
            .with_path("/dev/md/inactive")
            .with_property("md.state", "inactive")
            .with_property("md.degraded-devices", "0")
            .with_property("md.failed-devices", "0"),
    );
    graph.add_node(
        Node::new("md:/dev/md/active", NodeKind::MdRaid, "/dev/md/active")
            .with_path("/dev/md/active")
            .with_property("md.state", "clean")
            .with_property("md.degraded-devices", "0")
            .with_property("md.failed-devices", "0"),
    );
    graph.add_node(
        Node::new("md:/dev/md/unknown", NodeKind::MdRaid, "/dev/md/unknown")
            .with_path("/dev/md/unknown"),
    );
    graph.add_node(
        Node::new(
            "filesystem:/dev/md/wrong-kind",
            NodeKind::Filesystem,
            "/dev/md/wrong-kind",
        )
        .with_path("/dev/md/wrong-kind"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 5);
    assert_eq!(comparison.summary.already_satisfied_count, 2);
    assert_eq!(comparison.summary.suppressed_action_count, 2);
    assert_eq!(plan.summary.action_count, 3);
    assert!(plan
        .actions
        .iter()
        .all(|action| action.id != "mdraids:absent:stop"));
    assert!(plan
        .actions
        .iter()
        .all(|action| action.id != "mdraids:inactive:stop"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "mdraids:absent:stop"
            && diagnostic.kind == TopologyDiagnosticKind::MdStopAlreadySatisfied
            && diagnostic.message.contains("already absent")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "mdraids:inactive:stop"
            && diagnostic.kind == TopologyDiagnosticKind::MdStopAlreadySatisfied
            && diagnostic.message.contains("already inactive")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "mdraids:active:stop"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::MdStopRequired
            && diagnostic.message.contains("still active")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "mdraids:unknown:stop"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::MdStopRequired
            && diagnostic.message.contains("current state is unknown")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "mdraids:wrong-kind:stop"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::MdStopRequired
            && diagnostic.message.contains("not an MD RAID array")
    }));
}
