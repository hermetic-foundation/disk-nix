#[test]
fn plan_classifies_multipath_map_lifecycle_with_path_advice() {
    let plan = plan_from_json_bytes(
        br#"{
              "multipathMaps": {
                "mpatha": {
                  "target": "mpatha",
                  "operation": "grow",
                  "addDevices": ["/dev/sdb"],
                  "replaceDevices": {
                    "/dev/sdc": "/dev/sdd"
                  }
                },
                "mpathb": {
                  "target": "mpathb",
                  "operation": "rescan"
                },
                "mpath-old": {
                  "target": "mpath-old",
                  "operation": "destroy"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 5);
    assert_eq!(plan.summary.offline_required_count, 2);
    let grow = plan
        .actions
        .iter()
        .find(|action| action.id == "multipathmaps:mpatha:grow")
        .expect("multipath grow action exists");
    assert_eq!(grow.risk, RiskClass::Online);
    assert!(grow.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("rescan"))
    }));
    let add = plan
        .actions
        .iter()
        .find(|action| action.id == "multipathMaps:mpatha:add-device:/dev/sdb")
        .expect("multipath add action exists");
    assert_eq!(add.risk, RiskClass::Online);
    let replace = plan
        .actions
        .iter()
        .find(|action| action.id == "multipathMaps:mpatha:replace-device:/dev/sdc")
        .expect("multipath replace action exists");
    assert_eq!(replace.risk, RiskClass::OfflineRequired);
    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "multipathmaps:mpathb:rescan")
        .expect("multipath rescan action exists");
    assert_eq!(rescan.risk, RiskClass::Online);
    assert!(rescan.advice.as_ref().is_some_and(|advice| {
        advice
            .summary
            .contains("refreshes existing storage paths without deleting target data")
    }));
    let destroy = plan
        .actions
        .iter()
        .find(|action| action.id == "multipathmaps:mpath-old:destroy")
        .expect("multipath destroy action exists");
    assert_eq!(destroy.risk, RiskClass::OfflineRequired);
    assert!(!destroy.destructive);
    assert!(destroy.advice.as_ref().is_some_and(|advice| {
        advice
            .summary
            .contains("flushes the host map without deleting target-side data")
    }));
}

#[test]
fn plan_classifies_thin_pool_lifecycle_with_metadata_advice() {
    let plan = plan_from_json_bytes(
        br#"{
              "thinPools": {
                "vg0/newpool": {
                  "operation": "create",
                  "desiredSize": "100GiB"
                },
                "vg0/pool": {
                  "operation": "grow",
                  "desiredSize": "500GiB"
                },
                "vg0/reporting": {
                  "operation": "rescan"
                },
                "vg0/oldpool": {
                  "destroy": true
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 4);
    assert_eq!(plan.summary.offline_required_count, 0);
    assert_eq!(plan.summary.destructive_count, 1);
    let create = plan
        .actions
        .iter()
        .find(|action| action.id == "thinpools:vg0/newpool:create")
        .expect("thin pool create action exists");
    assert_eq!(create.risk, RiskClass::Online);
    assert_eq!(create.context.desired_size.as_deref(), Some("100GiB"));
    let grow = plan
        .actions
        .iter()
        .find(|action| action.id == "thinpools:vg0/pool:grow")
        .expect("thin pool grow action exists");
    assert_eq!(grow.id, "thinpools:vg0/pool:grow");
    assert_eq!(grow.risk, RiskClass::Online);
    assert_eq!(grow.context.desired_size.as_deref(), Some("500GiB"));
    assert!(grow.advice.as_ref().is_some_and(|advice| {
        advice.summary.contains("metadata")
            && advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("overcommit"))
    }));
    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "thinpools:vg0/reporting:rescan")
        .expect("thin pool rescan action exists");
    assert_eq!(rescan.operation, Operation::Rescan);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert!(!rescan.destructive);
    assert!(rescan
        .advice
        .as_ref()
        .is_some_and(|advice| { advice.summary.contains("thin pool rescan refreshes") }));
    let destroy = plan
        .actions
        .iter()
        .find(|action| action.id == "thinpools:vg0/oldpool:destroy")
        .expect("thin pool destroy action exists");
    assert_eq!(destroy.risk, RiskClass::Destructive);
    assert!(destroy.destructive);
}

#[test]
fn plan_classifies_lvm_snapshot_lifecycle() {
    let plan = plan_from_json_bytes(
        br#"{
              "lvmSnapshots": {
                "vg0/root-snap": {
                  "operation": "snapshot",
                  "target": "vg0/root",
                  "desiredSize": "20GiB"
                },
                "vg0/root-rollback": {
                  "operation": "rollback"
                },
                "vg0/root-inspect": {
                  "operation": "rescan"
                },
                "vg0/old-snap": {
                  "destroy": true
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 4);
    assert_eq!(plan.summary.potential_data_loss_count, 1);
    assert_eq!(plan.summary.destructive_count, 1);
    let snapshot = plan
        .actions
        .iter()
        .find(|action| action.id == "lvmsnapshots:vg0/root-snap:snapshot")
        .expect("snapshot action exists");
    assert_eq!(snapshot.risk, RiskClass::Reversible);
    assert_eq!(snapshot.context.target.as_deref(), Some("vg0/root"));
    assert_eq!(snapshot.context.desired_size.as_deref(), Some("20GiB"));
    let rollback = plan
        .actions
        .iter()
        .find(|action| action.id == "lvmsnapshots:vg0/root-rollback:rollback")
        .expect("rollback action exists");
    assert_eq!(rollback.risk, RiskClass::PotentialDataLoss);
    assert!(rollback
        .advice
        .as_ref()
        .is_some_and(|advice| advice.summary.contains("rolls the origin back")));
    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "lvmsnapshots:vg0/root-inspect:rescan")
        .expect("rescan action exists");
    assert_eq!(rescan.operation, Operation::Rescan);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert!(!rescan.destructive);
    assert!(rescan
        .advice
        .as_ref()
        .is_some_and(|advice| { advice.summary.contains("LVM snapshot rescan refreshes") }));
}

#[test]
fn plan_classifies_loop_device_lifecycle() {
    let plan = plan_from_json_bytes(
        br#"{
              "loopDevices": {
                "/dev/loop7": {
                  "operation": "create",
                  "device": "/var/lib/images/root.img"
                },
                "/dev/loop8": {
                  "operation": "grow"
                },
                "/dev/loop10": {
                  "operation": "rescan"
                },
                "/dev/loop9": {
                  "operation": "destroy"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 4);
    assert_eq!(plan.summary.offline_required_count, 1);
    assert_eq!(plan.summary.destructive_count, 0);
    let create = plan
        .actions
        .iter()
        .find(|action| action.id == "loopdevices:/dev/loop7:create")
        .expect("loop create action exists");
    assert_eq!(create.risk, RiskClass::Online);
    assert_eq!(
        create.context.device.as_deref(),
        Some("/var/lib/images/root.img")
    );
    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "loopdevices:/dev/loop10:rescan")
        .expect("loop rescan action exists");
    assert_eq!(rescan.operation, Operation::Rescan);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert!(!rescan.destructive);
    let destroy = plan
        .actions
        .iter()
        .find(|action| action.id == "loopdevices:/dev/loop9:destroy")
        .expect("loop destroy action exists");
    assert_eq!(destroy.risk, RiskClass::OfflineRequired);
    assert!(!destroy.destructive);
}

#[test]
fn plan_classifies_backing_file_lifecycle() {
    let plan = plan_from_json_bytes(
        br#"{
              "backingFiles": {
                "/var/lib/images/new.img": {
                  "operation": "create",
                  "desiredSize": "8GiB"
                },
                "/var/lib/images/root.img": {
                  "operation": "grow",
                  "desiredSize": "16GiB"
                },
                "inventory-image": {
                  "operation": "rescan",
                  "path": "/var/lib/images/inventory.img"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 3);
    assert_eq!(plan.summary.offline_required_count, 0);
    assert_eq!(plan.summary.destructive_count, 0);
    let create = plan
        .actions
        .iter()
        .find(|action| action.id == "backingfiles:/var/lib/images/new.img:create")
        .expect("backing file create action exists");
    assert_eq!(create.operation, Operation::Create);
    assert_eq!(create.risk, RiskClass::Online);
    assert_eq!(
        create.context.target.as_deref(),
        Some("/var/lib/images/new.img")
    );
    assert_eq!(create.context.desired_size.as_deref(), Some("8GiB"));
    assert!(create.advice.as_ref().is_some_and(|advice| {
        advice.summary.contains("backing file creation")
            && advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("existing backing file"))
    }));
    let grow = plan
        .actions
        .iter()
        .find(|action| action.id == "backingfiles:/var/lib/images/root.img:grow")
        .expect("backing file grow action exists");
    assert_eq!(grow.operation, Operation::Grow);
    assert_eq!(grow.risk, RiskClass::Online);
    assert_eq!(
        grow.context.target.as_deref(),
        Some("/var/lib/images/root.img")
    );
    assert_eq!(grow.context.desired_size.as_deref(), Some("16GiB"));
    assert!(grow
        .advice
        .as_ref()
        .is_some_and(|advice| advice.summary.contains("backing file growth")));
    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "backingfiles:inventory-image:rescan")
        .expect("backing file rescan action exists");
    assert_eq!(rescan.operation, Operation::Rescan);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert_eq!(
        rescan.context.target.as_deref(),
        Some("/var/lib/images/inventory.img")
    );
    assert!(!rescan.destructive);
}

#[test]
fn topology_comparison_reconciles_backing_file_create_and_grow() {
    let plan = plan_from_json_bytes(
        br#"{
              "backingFiles": {
                "/var/lib/images/new.img": {
                  "operation": "create",
                  "desiredSize": "8GiB"
                },
                "/var/lib/images/mismatch.img": {
                  "operation": "create",
                  "desiredSize": "8GiB"
                },
                "/var/lib/images/root.img": {
                  "operation": "grow",
                  "desiredSize": "8GiB"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "backing-file:/var/lib/images/new.img",
            NodeKind::BackingFile,
            "/var/lib/images/new.img",
        )
        .with_path("/var/lib/images/new.img")
        .with_size_bytes(8 * 1024 * 1024 * 1024),
    );
    graph.add_node(
        Node::new(
            "backing-file:/var/lib/images/mismatch.img",
            NodeKind::BackingFile,
            "/var/lib/images/mismatch.img",
        )
        .with_path("/var/lib/images/mismatch.img")
        .with_size_bytes(4 * 1024 * 1024 * 1024),
    );
    graph.add_node(
        Node::new(
            "backing-file:/var/lib/images/root.img",
            NodeKind::BackingFile,
            "/var/lib/images/root.img",
        )
        .with_path("/var/lib/images/root.img")
        .with_size_bytes(16 * 1024 * 1024 * 1024),
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
    assert_eq!(plan.summary.action_count, 1);
    assert!(plan
        .actions
        .iter()
        .all(|action| action.id == "backingfiles:/var/lib/images/mismatch.img:create"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "backingfiles:/var/lib/images/new.img:create"
            && diagnostic.kind == TopologyDiagnosticKind::BackingFileCreateAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "backingfiles:/var/lib/images/mismatch.img:create"
            && diagnostic.kind == TopologyDiagnosticKind::BackingFileCreateRequired
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.message.contains("refuse to overwrite")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "backingfiles:/var/lib/images/root.img:grow"
            && diagnostic.kind == TopologyDiagnosticKind::SizeAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_reconciles_partition_grow_from_end_size() {
    let plan = plan_from_json_bytes(
        br#"{
              "partitions": {
                "root": {
                  "operation": "grow",
                  "target": "/dev/disk/by-partuuid/root",
                  "device": "/dev/disk/by-id/nvme-root",
                  "partitionNumber": 2,
                  "end": "64GiB"
                },
                "data": {
                  "operation": "grow",
                  "target": "/dev/disk/by-partuuid/data",
                  "device": "/dev/disk/by-id/nvme-root",
                  "partitionNumber": 3,
                  "end": "128GiB"
                },
                "max": {
                  "operation": "grow",
                  "target": "/dev/disk/by-partuuid/max",
                  "device": "/dev/disk/by-id/nvme-root",
                  "partitionNumber": 4,
                  "end": "100%"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "partition:/dev/disk/by-partuuid/root",
            NodeKind::Partition,
            "/dev/disk/by-partuuid/root",
        )
        .with_path("/dev/disk/by-partuuid/root")
        .with_size_bytes(80 * 1024 * 1024 * 1024),
    );
    graph.add_node(
        Node::new(
            "partition:/dev/disk/by-partuuid/data",
            NodeKind::Partition,
            "/dev/disk/by-partuuid/data",
        )
        .with_path("/dev/disk/by-partuuid/data")
        .with_size_bytes(64 * 1024 * 1024 * 1024),
    );
    graph.add_node(
        Node::new(
            "partition:/dev/disk/by-partuuid/max",
            NodeKind::Partition,
            "/dev/disk/by-partuuid/max",
        )
        .with_path("/dev/disk/by-partuuid/max")
        .with_size_bytes(64 * 1024 * 1024 * 1024),
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
        .all(|action| action.id != "partitions:root:grow"));
    assert!(plan.actions.iter().any(|action| {
        action.id == "partitions:data:grow" && action.operation == Operation::Grow
    }));
    assert!(plan.actions.iter().any(|action| {
        action.id == "partitions:max:grow" && action.operation == Operation::Grow
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "partitions:root:grow"
            && diagnostic.kind == TopologyDiagnosticKind::SizeAlreadySatisfied
            && diagnostic.message.contains("desired size 64GiB")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "partitions:data:grow"
            && diagnostic.kind == TopologyDiagnosticKind::SizeBelowDesired
            && diagnostic.message.contains("desired size 128GiB")
    }));
    assert!(!comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "partitions:max:grow"
            && matches!(
                diagnostic.kind,
                TopologyDiagnosticKind::SizeAlreadySatisfied
                    | TopologyDiagnosticKind::SizeBelowDesired
                    | TopologyDiagnosticKind::SizeConflict
            )
    }));
}

#[test]
fn topology_comparison_reconciles_partition_create_when_target_exists() {
    let plan = plan_from_json_bytes(
        br#"{
              "partitions": {
                "boot": {
                  "operation": "create",
                  "target": "/dev/disk/by-partuuid/boot",
                  "device": "/dev/disk/by-id/nvme-root",
                  "partitionNumber": 1,
                  "desiredSize": "1GiB"
                },
                "root": {
                  "operation": "create",
                  "target": "/dev/disk/by-partuuid/root",
                  "device": "/dev/disk/by-id/nvme-root",
                  "partitionNumber": 2,
                  "desiredSize": "64GiB"
                },
                "scratch": {
                  "operation": "create",
                  "target": "/dev/disk/by-partuuid/scratch",
                  "device": "/dev/disk/by-id/nvme-root",
                  "partitionNumber": 3
                },
                "wrong": {
                  "operation": "create",
                  "target": "/dev/disk/by-partuuid/wrong",
                  "device": "/dev/disk/by-id/nvme-root",
                  "partitionNumber": 4
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "partition:/dev/disk/by-partuuid/boot",
            NodeKind::Partition,
            "/dev/disk/by-partuuid/boot",
        )
        .with_path("/dev/disk/by-partuuid/boot")
        .with_size_bytes(1024 * 1024 * 1024),
    );
    graph.add_node(
        Node::new(
            "partition:/dev/disk/by-partuuid/root",
            NodeKind::Partition,
            "/dev/disk/by-partuuid/root",
        )
        .with_path("/dev/disk/by-partuuid/root")
        .with_size_bytes(32 * 1024 * 1024 * 1024),
    );
    graph.add_node(
        Node::new(
            "partition:/dev/disk/by-partuuid/scratch",
            NodeKind::Partition,
            "/dev/disk/by-partuuid/scratch",
        )
        .with_path("/dev/disk/by-partuuid/scratch"),
    );
    graph.add_node(
        Node::new(
            "block:/dev/disk/by-partuuid/wrong",
            NodeKind::PhysicalDisk,
            "/dev/disk/by-partuuid/wrong",
        )
        .with_path("/dev/disk/by-partuuid/wrong"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 4);
    assert_eq!(comparison.summary.already_satisfied_count, 2);
    assert_eq!(comparison.summary.suppressed_action_count, 2);
    assert_eq!(plan.summary.action_count, 2);
    assert!(plan
        .actions
        .iter()
        .all(|action| action.id != "partitions:boot:create"));
    assert!(plan
        .actions
        .iter()
        .all(|action| action.id != "partitions:scratch:create"));
    assert!(plan.actions.iter().any(|action| {
        action.id == "partitions:root:create" && action.operation == Operation::Create
    }));
    assert!(plan.actions.iter().any(|action| {
        action.id == "partitions:wrong:create" && action.operation == Operation::Create
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "partitions:boot:create"
            && diagnostic.kind == TopologyDiagnosticKind::PartitionCreateAlreadySatisfied
            && diagnostic.message.contains("desired size 1GiB")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "partitions:scratch:create"
            && diagnostic.kind == TopologyDiagnosticKind::PartitionCreateAlreadySatisfied
            && diagnostic.message.contains("already exists")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "partitions:root:create"
            && diagnostic.kind == TopologyDiagnosticKind::PartitionCreateRequired
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.message.contains("not desired size 64GiB")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "partitions:wrong:create"
            && diagnostic.kind == TopologyDiagnosticKind::PartitionCreateRequired
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.message.contains("not a partition")
    }));
}

#[test]
fn topology_comparison_reconciles_disk_create_from_partition_table() {
    let plan = plan_from_json_bytes(
        br#"{
              "disks": {
                "/dev/disk/by-id/system": {
                  "operation": "create",
                  "partitionType": "gpt"
                },
                "/dev/disk/by-id/default-gpt": {
                  "operation": "create"
                },
                "/dev/disk/by-id/legacy": {
                  "operation": "create",
                  "partitionType": "gpt"
                },
                "/dev/disk/by-id/unknown": {
                  "operation": "create",
                  "partitionType": "gpt"
                },
                "/dev/disk/by-id/wrong": {
                  "operation": "create",
                  "partitionType": "gpt"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "block:/dev/disk/by-id/system",
            NodeKind::PhysicalDisk,
            "/dev/disk/by-id/system",
        )
        .with_path("/dev/disk/by-id/system")
        .with_property("partition.table", "gpt"),
    );
    graph.add_node(
        Node::new(
            "block:/dev/disk/by-id/default-gpt",
            NodeKind::PhysicalDisk,
            "/dev/disk/by-id/default-gpt",
        )
        .with_path("/dev/disk/by-id/default-gpt")
        .with_property("partition.table", "gpt"),
    );
    graph.add_node(
        Node::new(
            "block:/dev/disk/by-id/legacy",
            NodeKind::PhysicalDisk,
            "/dev/disk/by-id/legacy",
        )
        .with_path("/dev/disk/by-id/legacy")
        .with_property("partition.table", "msdos"),
    );
    graph.add_node(
        Node::new(
            "block:/dev/disk/by-id/unknown",
            NodeKind::PhysicalDisk,
            "/dev/disk/by-id/unknown",
        )
        .with_path("/dev/disk/by-id/unknown"),
    );
    graph.add_node(
        Node::new(
            "partition:/dev/disk/by-id/wrong",
            NodeKind::Partition,
            "/dev/disk/by-id/wrong",
        )
        .with_path("/dev/disk/by-id/wrong"),
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
        .all(|action| action.id != "disks:/dev/disk/by-id/system:create"));
    assert!(plan
        .actions
        .iter()
        .all(|action| action.id != "disks:/dev/disk/by-id/default-gpt:create"));
    assert!(plan.actions.iter().any(|action| {
        action.id == "disks:/dev/disk/by-id/legacy:create" && action.operation == Operation::Create
    }));
    assert!(plan.actions.iter().any(|action| {
        action.id == "disks:/dev/disk/by-id/unknown:create" && action.operation == Operation::Create
    }));
    assert!(plan.actions.iter().any(|action| {
        action.id == "disks:/dev/disk/by-id/wrong:create" && action.operation == Operation::Create
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "disks:/dev/disk/by-id/system:create"
            && diagnostic.kind == TopologyDiagnosticKind::DiskCreateAlreadySatisfied
            && diagnostic.message.contains("partition table gpt")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "disks:/dev/disk/by-id/default-gpt:create"
            && diagnostic.kind == TopologyDiagnosticKind::DiskCreateAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "disks:/dev/disk/by-id/legacy:create"
            && diagnostic.kind == TopologyDiagnosticKind::DiskCreateRequired
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.message.contains("partition table msdos")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "disks:/dev/disk/by-id/unknown:create"
            && diagnostic.kind == TopologyDiagnosticKind::DiskCreateRequired
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic
                .message
                .contains("current partition table is unknown")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "disks:/dev/disk/by-id/wrong:create"
            && diagnostic.kind == TopologyDiagnosticKind::DiskCreateRequired
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.message.contains("not a physical disk")
    }));
}

#[test]
fn topology_comparison_reconciles_lvm_physical_volume_create() {
    let plan = plan_from_json_bytes(
        br#"{
              "physicalVolumes": {
                "/dev/disk/by-id/pv-present": {
                  "operation": "create"
                },
                "/dev/disk/by-id/plain-device": {
                  "operation": "create"
                },
                "/dev/disk/by-id/duplicate-pv": {
                  "operation": "create"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "block:/dev/disk/by-id/pv-present",
            NodeKind::PhysicalDisk,
            "/dev/disk/by-id/pv-present",
        )
        .with_path("/dev/disk/by-id/pv-present"),
    );
    graph.add_node(
        Node::new(
            "lvm-pv:/dev/disk/by-id/pv-present",
            NodeKind::LvmPhysicalVolume,
            "/dev/disk/by-id/pv-present",
        )
        .with_path("/dev/disk/by-id/pv-present"),
    );
    graph.add_node(
        Node::new(
            "block:/dev/disk/by-id/plain-device",
            NodeKind::PhysicalDisk,
            "/dev/disk/by-id/plain-device",
        )
        .with_path("/dev/disk/by-id/plain-device"),
    );
    graph.add_node(
        Node::new(
            "lvm-pv:/dev/disk/by-id/duplicate-pv",
            NodeKind::LvmPhysicalVolume,
            "/dev/disk/by-id/duplicate-pv",
        )
        .with_path("/dev/disk/by-id/duplicate-pv")
        .with_property("lvm.pv-duplicate", "duplicate"),
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
        .all(|action| { action.id != "physicalvolumes:/dev/disk/by-id/pv-present:create" }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "physicalvolumes:/dev/disk/by-id/pv-present:create"
            && diagnostic.kind == TopologyDiagnosticKind::LvmPvCreateAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "physicalvolumes:/dev/disk/by-id/plain-device:create"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LvmPvCreateRequired
            && diagnostic.message.contains("not an LVM physical volume")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "physicalvolumes:/dev/disk/by-id/duplicate-pv:create"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LvmPvCreateRequired
            && diagnostic.message.contains("lvm.pv-duplicate=duplicate")
    }));
}

#[test]
fn plan_classifies_device_mapper_lifecycle() {
    let plan = plan_from_json_bytes(
        br#"{
              "dmMaps": {
                "cryptroot": {
                  "operation": "rescan",
                  "target": "/dev/mapper/cryptroot"
                },
                "cryptswap": {
                  "operation": "rename",
                  "target": "/dev/mapper/cryptswap",
                  "renameTo": "cryptswap-retired"
                },
                "oldmap": {
                  "operation": "destroy",
                  "target": "/dev/mapper/oldmap"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 3);
    assert_eq!(plan.summary.offline_required_count, 1);
    assert_eq!(plan.summary.destructive_count, 1);
    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "dmmaps:cryptroot:rescan")
        .expect("device-mapper rescan action exists");
    assert_eq!(rescan.operation, Operation::Rescan);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert_eq!(
        rescan.context.target.as_deref(),
        Some("/dev/mapper/cryptroot")
    );
    assert!(!rescan.destructive);
    assert!(rescan
        .advice
        .as_ref()
        .is_some_and(|advice| advice.summary.contains("device-mapper rescan")));
    let rename = plan
        .actions
        .iter()
        .find(|action| action.id == "dmmaps:cryptswap:rename")
        .expect("device-mapper rename action exists");
    assert_eq!(rename.operation, Operation::Rename);
    assert_eq!(rename.risk, RiskClass::OfflineRequired);
    assert_eq!(
        rename.context.target.as_deref(),
        Some("/dev/mapper/cryptswap")
    );
    assert_eq!(
        rename.context.rename_to.as_deref(),
        Some("cryptswap-retired")
    );
    assert!(!rename.destructive);
    let destroy = plan
        .actions
        .iter()
        .find(|action| action.id == "dmmaps:oldmap:destroy")
        .expect("device-mapper destroy action exists");
    assert_eq!(destroy.operation, Operation::Destroy);
    assert_eq!(destroy.risk, RiskClass::Destructive);
    assert_eq!(
        destroy.context.target.as_deref(),
        Some("/dev/mapper/oldmap")
    );
    assert!(destroy.destructive);
    assert!(destroy
        .advice
        .as_ref()
        .is_some_and(|advice| advice.summary.contains("device-mapper removal")));
}
