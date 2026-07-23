#[test]
fn topology_comparison_groups_device_mapper_and_filesystem_reconciliation() {
    let plan = plan_from_json_bytes(
        br#"{
              "dmMaps": {
                "cryptdata": {
                  "operation": "destroy",
                  "target": "cryptdata"
                }
              },
              "filesystems": {
                "data": {
                  "operation": "unmount",
                  "mountpoint": "/data",
                  "device": "/dev/mapper/cryptdata",
                  "fsType": "xfs"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("dm:cryptdata", NodeKind::DeviceMapper, "cryptdata")
            .with_path("/dev/mapper/cryptdata")
            .with_property("dm.open-count", "0"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 3);
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert!(comparison.summary.partially_suppressed_group_count >= 1);
    assert!(plan
        .actions
        .iter()
        .any(|action| action.id == "dmmaps:cryptdata:destroy"));
    assert!(plan
        .actions
        .iter()
        .all(|action| action.id != "filesystems:data:unmount"));

    let group = comparison
        .reconciliation_groups
        .iter()
        .find(|group| group.identity == "dm-map:cryptdata")
        .expect("device-mapper and filesystem reconciliation group exists");
    assert!(group
        .planned_action_ids
        .iter()
        .any(|action_id| action_id == "dmmaps:cryptdata:destroy"));
    assert!(group
        .suppressed_action_ids
        .iter()
        .any(|action_id| action_id == "filesystems:data:unmount"));
    assert!(group.partially_suppressed);
}

#[test]
fn topology_comparison_groups_backing_file_and_loop_reconciliation() {
    let backing_path = "/var/lib/images/root.img";
    let plan = plan_from_json_bytes(
        br#"{
              "backingFiles": {
                "/var/lib/images/root.img": {
                  "operation": "create",
                  "desiredSize": "8GiB"
                }
              },
              "loopDevices": {
                "/dev/loop10": {
                  "operation": "create",
                  "device": "/var/lib/images/root.img"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "backing-file:/var/lib/images/root.img",
            NodeKind::BackingFile,
            backing_path,
        )
        .with_path(backing_path)
        .with_size_bytes(8 * 1024 * 1024 * 1024),
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
    assert_eq!(plan.actions[0].id, "loopdevices:/dev/loop10:create");

    let group = comparison
        .reconciliation_groups
        .iter()
        .find(|group| group.identity == backing_path)
        .expect("backing file and loop reconciliation group exists");
    assert_eq!(
        group.planned_action_ids,
        vec!["loopdevices:/dev/loop10:create"]
    );
    assert_eq!(
        group.suppressed_action_ids,
        vec!["backingfiles:/var/lib/images/root.img:create"]
    );
    assert!(group.partially_suppressed);
}

#[test]
fn topology_comparison_keeps_lun_attach_when_path_absent() {
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
        diagnostic.action_id == "luns:iqn.2026-06.example:storage/root:0:attach"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LunAttachRequired
    }));
}

#[test]
fn topology_comparison_suppresses_lun_detach_when_path_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "luns": {
                "iqn.2026-06.example:storage/old:1": {
                  "operation": "detach",
                  "device": "/dev/disk/by-path/ip-192.0.2.10-lun-1"
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
        diagnostic.action_id == "luns:iqn.2026-06.example:storage/old:1:detach"
            && diagnostic.current.is_none()
            && diagnostic.kind == TopologyDiagnosticKind::LunDetachAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_lun_detach_when_path_exists() {
    let plan = plan_from_json_bytes(
        br#"{
              "luns": {
                "iqn.2026-06.example:storage/old:1": {
                  "operation": "detach",
                  "device": "/dev/disk/by-path/ip-192.0.2.10-lun-1"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("lun:1", NodeKind::Lun, "1")
            .with_path("/dev/disk/by-path/ip-192.0.2.10-lun-1")
            .with_property("iscsi.attached-disk", "sdc"),
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
        diagnostic.action_id == "luns:iqn.2026-06.example:storage/old:1:detach"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LunDetachRequired
    }));
}

#[test]
fn topology_comparison_adds_graph_dependency_edges_for_layered_growth() {
    let plan = plan_from_json_bytes(
        br#"{
              "luns": {
                "/dev/disk/by-path/ip-192.0.2.10-lun-0": {
                  "operation": "grow",
                  "desiredSize": "200GiB"
                }
              },
              "multipathMaps": {
                "mpatha": {
                  "operation": "grow",
                  "target": "/dev/mapper/mpatha",
                  "desiredSize": "200GiB"
                }
              },
              "partitions": {
                "root": {
                  "operation": "grow",
                  "target": "/dev/mapper/mpatha-part1",
                  "desiredSize": "200GiB"
                }
              },
              "luks": {
                "devices": {
                  "cryptroot": {
                    "operation": "grow",
                    "device": "/dev/mapper/mpatha-part1",
                    "target": "cryptroot",
                    "desiredSize": "200GiB"
                  }
                }
              },
              "volumes": {
                "vg0/root": {
                  "operation": "grow",
                  "desiredSize": "200GiB"
                }
              },
              "filesystems": {
                "root": {
                  "operation": "grow",
                  "device": "vg0/root",
                  "resizePolicy": "grow-only",
                  "desiredSize": "200GiB"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "lun:0",
            NodeKind::Lun,
            "/dev/disk/by-path/ip-192.0.2.10-lun-0",
        )
        .with_path("/dev/disk/by-path/ip-192.0.2.10-lun-0"),
    );
    graph.add_node(
        Node::new("multipath:mpatha", NodeKind::MultipathDevice, "mpatha")
            .with_path("/dev/mapper/mpatha"),
    );
    graph.add_node(
        Node::new(
            "partition:/dev/mapper/mpatha-part1",
            NodeKind::Partition,
            "/dev/mapper/mpatha-part1",
        )
        .with_path("/dev/mapper/mpatha-part1"),
    );
    graph.add_node(Node::new(
        "luks:cryptroot",
        NodeKind::LuksContainer,
        "cryptroot",
    ));
    graph.add_node(Node::new(
        "lvm:lv:vg0/root",
        NodeKind::LvmLogicalVolume,
        "vg0/root",
    ));
    graph.add_node(Node::new("filesystem:root", NodeKind::Filesystem, "root"));
    graph.add_edge(disk_nix_model::Edge::new(
        "lun:0",
        "multipath:mpatha",
        Relationship::Backs,
    ));
    graph.add_edge(disk_nix_model::Edge::new(
        "multipath:mpatha",
        "partition:/dev/mapper/mpatha-part1",
        Relationship::Contains,
    ));
    graph.add_edge(disk_nix_model::Edge::new(
        "partition:/dev/mapper/mpatha-part1",
        "luks:cryptroot",
        Relationship::Backs,
    ));
    graph.add_edge(disk_nix_model::Edge::new(
        "luks:cryptroot",
        "lvm:lv:vg0/root",
        Relationship::Backs,
    ));
    graph.add_edge(disk_nix_model::Edge::new(
        "lvm:lv:vg0/root",
        "filesystem:root",
        Relationship::Backs,
    ));

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.graph_dependency_edge_count, 15);
    assert_eq!(comparison.summary.lifecycle_group_count, 1);
    assert_eq!(comparison.summary.graph_derived_lifecycle_group_count, 1);
    let lifecycle_group = comparison
        .lifecycle_groups
        .first()
        .expect("layered growth should produce a lifecycle group");
    assert_eq!(lifecycle_group.action_count, 6);
    assert_eq!(lifecycle_group.edge_count, 15);
    assert_eq!(lifecycle_group.graph_derived_edge_count, 15);
    assert_eq!(
        lifecycle_group.action_ids,
        vec![
            "filesystem:root:grow".to_string(),
            "luks.devices:cryptroot:grow".to_string(),
            "luns:/dev/disk/by-path/ip-192.0.2.10-lun-0:grow".to_string(),
            "multipathmaps:mpatha:grow".to_string(),
            "partitions:root:grow".to_string(),
            "volumes:vg0/root:grow".to_string(),
        ]
    );
    assert_eq!(
        lifecycle_group.directions,
        vec![DependencyDirection::LowerLayersFirst]
    );
    let comparison_json =
        serde_json::to_value(comparison).expect("comparison should serialize to json");
    assert_eq!(
        comparison_json["summary"]["lifecycleGroupCount"],
        serde_json::json!(1)
    );
    assert_eq!(
        comparison_json["summary"]["graphDerivedLifecycleGroupCount"],
        serde_json::json!(1)
    );
    assert_eq!(
        comparison_json["lifecycleGroups"][0]["graphDerivedEdgeCount"],
        serde_json::json!(15)
    );
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystem:root:grow"
            && diagnostic.level == TopologyDiagnosticLevel::Info
            && diagnostic.kind == TopologyDiagnosticKind::GraphDependencyOrder
            && diagnostic.query == "lvm:lv:vg0/root -> filesystem:root"
            && diagnostic.message.contains(
                "current topology path orders filesystem:root:grow after volumes:vg0/root:grow",
            )
            && diagnostic.message.contains("lower layer before consumer")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "volumes:vg0/root:grow"
                && diagnostic.kind == TopologyDiagnosticKind::GraphDependencyOrder
                && diagnostic.query == "lun:0 -> lvm:lv:vg0/root"
                && diagnostic.message.contains(
                    "current topology path orders volumes:vg0/root:grow after luns:/dev/disk/by-path/ip-192.0.2.10-lun-0:grow"
                )
        }));
    let filesystem = plan
        .dependency_order
        .iter()
        .find(|order| order.action_id == "filesystem:root:grow")
        .expect("filesystem dependency order exists");
    assert!(filesystem
        .depends_on
        .contains(&"volumes:vg0/root:grow".to_string()));
    assert!(filesystem
        .depends_on
        .contains(&"luns:/dev/disk/by-path/ip-192.0.2.10-lun-0:grow".to_string()));
    assert!(filesystem
        .depends_on
        .contains(&"multipathmaps:mpatha:grow".to_string()));
    assert!(filesystem.recovery_depends_on.is_empty());
    assert!(filesystem
        .recovery_unblocks
        .contains(&"volumes:vg0/root:grow".to_string()));
    assert!(filesystem
        .recovery_unblocks
        .contains(&"luns:/dev/disk/by-path/ip-192.0.2.10-lun-0:grow".to_string()));
    assert!(filesystem.notes.iter().any(|note| {
        note.contains("current topology graph path requires")
            && note.contains("volumes:vg0/root:grow")
    }));
    assert!(filesystem.notes.iter().any(|note| {
        note.contains("recovery review unblocks prerequisite action")
            && note.contains("volumes:vg0/root:grow")
    }));
    let lun = plan
        .dependency_order
        .iter()
        .find(|order| order.action_id == "luns:/dev/disk/by-path/ip-192.0.2.10-lun-0:grow")
        .expect("lun dependency order exists");
    assert_eq!(
        lun.unblocks,
        vec![
            "filesystem:root:grow".to_string(),
            "luks.devices:cryptroot:grow".to_string(),
            "multipathmaps:mpatha:grow".to_string(),
            "partitions:root:grow".to_string(),
            "volumes:vg0/root:grow".to_string(),
        ]
    );
    assert_eq!(lun.recovery_depends_on, lun.unblocks);
    assert!(lun.recovery_unblocks.is_empty());
    assert!(lun
        .notes
        .iter()
        .any(|note| { note.contains("current topology graph path shows this action unblocks") }));
}

#[test]
fn topology_comparison_reverses_graph_dependency_edges_for_teardown() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "root": {
                  "operation": "unmount",
                  "device": "/dev/mapper/cryptroot",
                  "mountpoint": "/"
                }
              },
              "luks": {
                "devices": {
                  "cryptroot": {
                    "operation": "close",
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
        Node::new("luks:cryptroot", NodeKind::LuksContainer, "cryptroot")
            .with_path("/dev/mapper/cryptroot"),
    );
    graph.add_node(
        Node::new("filesystem:/", NodeKind::Filesystem, "root")
            .with_path("/")
            .with_property("filesystem.type", "xfs"),
    );
    graph.add_edge(disk_nix_model::Edge::new(
        "luks:cryptroot",
        "filesystem:/",
        Relationship::Backs,
    ));

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");
    assert_eq!(comparison.summary.graph_dependency_edge_count, 1);
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "luks.devices:cryptroot:close"
                && diagnostic.level == TopologyDiagnosticLevel::Info
                && diagnostic.kind == TopologyDiagnosticKind::GraphDependencyOrder
                && diagnostic.query == "luks:cryptroot -> filesystem:/"
                && diagnostic.message.contains(
                    "current topology path orders luks.devices:cryptroot:close after filesystems:root:unmount"
                )
                && diagnostic.message.contains("consumer before backing layer")
        }));

    let luks = plan
        .dependency_order
        .iter()
        .find(|order| order.action_id == "luks.devices:cryptroot:close")
        .expect("luks close dependency order exists");
    assert_eq!(
        luks.depends_on,
        vec!["filesystems:root:unmount".to_string()]
    );
    assert!(luks.recovery_depends_on.is_empty());
    assert_eq!(
        luks.recovery_unblocks,
        vec!["filesystems:root:unmount".to_string()]
    );
    assert!(luks.notes.iter().any(|note| {
        note.contains("current topology graph path requires filesystems:root:unmount")
    }));
    assert!(luks.notes.iter().any(|note| {
        note.contains("recovery review unblocks prerequisite action")
            && note.contains("filesystems:root:unmount")
    }));
}

#[test]
fn topology_comparison_reports_mixed_direction_graph_dependency_conflicts() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "root": {
                  "operation": "grow",
                  "device": "/dev/mapper/cryptroot",
                  "mountpoint": "/",
                  "resizePolicy": "grow-only",
                  "desiredSize": "200GiB"
                }
              },
              "luks": {
                "devices": {
                  "cryptroot": {
                    "operation": "close",
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
        Node::new("luks:cryptroot", NodeKind::LuksContainer, "cryptroot")
            .with_path("/dev/mapper/cryptroot"),
    );
    graph.add_node(
        Node::new("filesystem:/", NodeKind::Filesystem, "root")
            .with_path("/")
            .with_property("filesystem.type", "xfs")
            .with_size_bytes(100 * 1024 * 1024 * 1024),
    );
    graph.add_edge(disk_nix_model::Edge::new(
        "luks:cryptroot",
        "filesystem:/",
        Relationship::Backs,
    ));

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.graph_dependency_edge_count, 0);
    assert_eq!(comparison.summary.graph_dependency_conflict_count, 1);
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "luks.devices:cryptroot:close"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::GraphDependencyConflict
            && diagnostic.query == "luks:cryptroot -> filesystem:/"
            && diagnostic.message.contains("mixed dependency directions")
            && diagnostic
                .message
                .contains("build/update pass [filesystem:root:grow]")
            && diagnostic
                .message
                .contains("teardown/recovery pass [luks.devices:cryptroot:close]")
            && diagnostic.message.contains("filesystem:root:grow")
    }));
    let resolution = comparison
        .graph_dependency_conflict_resolutions
        .iter()
        .find(|resolution| resolution.path == "luks:cryptroot -> filesystem:/")
        .expect("graph conflict resolution should be reported");
    assert_eq!(
        resolution.build_or_update_pass,
        vec!["filesystem:root:grow".to_string()]
    );
    assert_eq!(
        resolution.teardown_or_recovery_pass,
        vec!["luks.devices:cryptroot:close".to_string()]
    );
    assert_eq!(
        resolution.lower_direction,
        DependencyDirection::UpperLayersFirst
    );
    assert_eq!(
        resolution.upper_direction,
        DependencyDirection::LowerLayersFirst
    );
    assert!(resolution
        .recommendation
        .contains("split mixed-direction graph-path work"));
    let json = serde_json::to_value(comparison).expect("comparison serializes");
    assert_eq!(
        json["graphDependencyConflictResolutions"][0]["buildOrUpdatePass"][0],
        "filesystem:root:grow"
    );
    assert_eq!(
        json["graphDependencyConflictResolutions"][0]["teardownOrRecoveryPass"][0],
        "luks.devices:cryptroot:close"
    );
}

#[test]
fn topology_comparison_ignores_suppressed_actions_for_graph_edges() {
    let plan = plan_from_json_bytes(
        br#"{
              "volumes": {
                "vg0/root": {
                  "operation": "grow",
                  "desiredSize": "100GiB"
                }
              },
              "filesystems": {
                "root": {
                  "operation": "grow",
                  "device": "vg0/root",
                  "desiredSize": "100GiB"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("lvm:lv:vg0/root", NodeKind::LvmLogicalVolume, "vg0/root")
            .with_size_bytes(200 * 1024 * 1024 * 1024),
    );
    graph.add_node(
        Node::new("filesystem:root", NodeKind::Filesystem, "root")
            .with_size_bytes(50 * 1024 * 1024 * 1024),
    );
    graph.add_edge(disk_nix_model::Edge::new(
        "lvm:lv:vg0/root",
        "filesystem:root",
        Relationship::Backs,
    ));

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert_eq!(comparison.summary.graph_dependency_edge_count, 0);
    assert!(plan
        .actions
        .iter()
        .all(|action| action.id != "volumes:vg0/root:grow"));
    assert!(plan
        .dependency_order
        .iter()
        .all(|order| order.depends_on.is_empty() && order.unblocks.is_empty()));
}

#[test]
fn topology_comparison_keeps_satisfied_actions_with_warnings() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "home": {
                  "mountpoint": "/home",
                  "fsType": "btrfs",
                  "resizePolicy": "grow-only",
                  "desiredSize": "100GiB"
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

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 1);
    assert_eq!(comparison.summary.already_satisfied_count, 1);
    assert_eq!(comparison.summary.type_conflict_count, 1);
    assert_eq!(comparison.summary.suppressed_action_count, 0);
    assert_eq!(plan.summary.action_count, 1);
    assert!(plan.actions.iter().any(|action| {
        action.id == "filesystem:home:grow" && action.operation == Operation::Grow
    }));
}

#[test]
fn topology_comparison_reports_missing_targets() {
    let plan = plan_from_json_bytes(
        br#"{
              "volumes": {
                "vg/missing": {
                  "operation": "grow",
                  "desiredSize": "50GiB"
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
    assert_eq!(comparison.summary.missing_count, 1);
    assert_eq!(
        comparison.diagnostics[0].kind,
        TopologyDiagnosticKind::Missing
    );
}

#[test]
fn non_destructive_migration_examples_are_verified() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root resolves");
    let fixture_path = repo_root.join("examples/non-destructive-migrations.json");
    let bytes = std::fs::read(&fixture_path).expect("migration examples fixture exists");
    let fixtures: Vec<MigrationExampleFixture> =
        serde_json::from_slice(&bytes).expect("migration examples parse");

    assert!(
        fixtures.len() >= 20,
        "expected at least 20 migration examples, got {}",
        fixtures.len()
    );

    let mut mismatches = Vec::new();

    for fixture in fixtures {
        assert!(!fixture.name.trim().is_empty(), "fixture name is required");
        assert!(
            !fixture.description.trim().is_empty(),
            "fixture {} description is required",
            fixture.name
        );
        assert!(
            repo_root.join(&fixture.base_example).is_file(),
            "fixture {} base example is missing: {}",
            fixture.name,
            fixture.base_example
        );

        let spec_bytes = serde_json::to_vec(&fixture.target_spec).expect("target spec serializes");
        let plan = plan_from_json_bytes(&spec_bytes)
            .unwrap_or_else(|error| panic!("fixture {} target spec parses: {error}", fixture.name));
        let plan = compare_plan_with_topology(plan, &fixture.current_graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .unwrap_or_else(|| panic!("fixture {} topology comparison is present", fixture.name));

        assert_eq!(
            plan.summary.destructive_count, 0,
            "fixture {} should not leave destructive actions: {:?}",
            fixture.name, plan.actions
        );
        assert_eq!(
            plan.summary.potential_data_loss_count, 0,
            "fixture {} should not leave potential-data-loss actions: {:?}",
            fixture.name, plan.actions
        );
        assert!(
            plan.actions.iter().all(|action| {
                !action.destructive
                    && !matches!(
                        action.risk,
                        RiskClass::PotentialDataLoss
                            | RiskClass::Destructive
                            | RiskClass::Irreversible
                    )
            }),
            "fixture {} left a destructive or loss-prone action: {:?}",
            fixture.name,
            plan.actions
        );

        let actual_ids: Vec<String> = plan
            .actions
            .iter()
            .map(|action| action.id.clone())
            .collect();
        if actual_ids != fixture.expected_remaining_action_ids {
            mismatches.push(format!(
                "{} remaining action ids differ\n  actual: {:?}\nexpected: {:?}",
                fixture.name, actual_ids, fixture.expected_remaining_action_ids
            ));
        }

        for suppressed in &fixture.expected_suppressed_action_ids {
            assert!(
                !actual_ids.contains(suppressed),
                "fixture {} expected {} to be suppressed",
                fixture.name,
                suppressed
            );
            assert!(
                comparison.diagnostics.iter().any(|diagnostic| {
                    diagnostic.action_id == *suppressed
                        && diagnostic.kind != TopologyDiagnosticKind::Missing
                }),
                "fixture {} missing diagnostic for suppressed action {}",
                fixture.name,
                suppressed
            );
        }
        assert_eq!(
            comparison.summary.suppressed_action_count,
            fixture.expected_suppressed_action_ids.len(),
            "fixture {} suppressed action count differs",
            fixture.name
        );
    }

    assert!(
        mismatches.is_empty(),
        "migration fixture mismatches:\n{}",
        mismatches.join("\n")
    );
}
