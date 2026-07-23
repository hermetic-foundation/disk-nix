#[test]
fn topology_comparison_suppresses_luks_token_remove_when_token_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "luksTokens": {
                "cryptroot:3": {
                  "operation": "remove-token",
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "tokenId": "3"
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
        .with_property("cryptsetup.luks-tokens", "0,1")
        .with_property("cryptsetup.luks-token-count", "2"),
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
        diagnostic.action_id == "lukstokens:cryptroot:3:remove-token"
            && diagnostic.kind == TopologyDiagnosticKind::LuksTokenRemoveAlreadySatisfied
            && diagnostic.query == "/dev/disk/by-id/root-luks"
    }));
}

#[test]
fn topology_comparison_keeps_luks_token_remove_when_token_present() {
    let plan = plan_from_json_bytes(
        br#"{
              "luksTokens": {
                "cryptroot:3": {
                  "operation": "remove-token",
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "tokenId": "3"
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
        .with_property("cryptsetup.luks-tokens", "1,3")
        .with_property("cryptsetup.luks-token-3-type", "systemd-tpm2")
        .with_property("cryptsetup.luks-token-3-keyslot", "2"),
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
        diagnostic.action_id == "lukstokens:cryptroot:3:remove-token"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LuksTokenRemoveRequired
            && diagnostic.message.contains("type systemd-tpm2")
            && diagnostic.message.contains("keyslot 2")
    }));
}

#[test]
fn topology_comparison_keeps_luks_keyslot_remove_missing_without_container() {
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
              },
              "luksTokens": {
                "cryptroot:3": {
                  "operation": "remove-token",
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "tokenId": "3"
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

    assert_eq!(comparison.summary.already_satisfied_count, 0);
    assert_eq!(comparison.summary.suppressed_action_count, 0);
    assert_eq!(comparison.summary.missing_count, 0);
    assert_eq!(plan.actions.len(), 2);
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "lukskeyslots:cryptroot:2:remove-key"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LuksKeyslotRemoveRequired
            && diagnostic.message.contains("keyslot 2 removal")
            && diagnostic
                .message
                .contains("backing device /dev/disk/by-id/root-luks")
            && diagnostic.query == "/dev/disk/by-id/root-luks"
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "lukstokens:cryptroot:3:remove-token"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LuksTokenRemoveRequired
            && diagnostic.message.contains("token 3 removal")
            && diagnostic
                .message
                .contains("backing device /dev/disk/by-id/root-luks")
            && diagnostic.query == "/dev/disk/by-id/root-luks"
    }));
}

#[test]
fn topology_comparison_suppresses_active_lvm_activate_action() {
    let plan = plan_from_json_bytes(
        br#"{
              "volumes": {
                "vg0/home": {
                  "operation": "activate"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("lvm:lv:vg0/home", NodeKind::LvmLogicalVolume, "vg0/home")
            .with_path("/dev/vg0/home")
            .with_property("lvm.active", "active")
            .with_property("lvm.active-locally", "active locally"),
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
        diagnostic.action_id == "volumes:vg0/home:activate"
            && diagnostic.kind == TopologyDiagnosticKind::LvmActivateAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_lvm_activate_action_when_inactive() {
    let plan = plan_from_json_bytes(
        br#"{
              "volumes": {
                "vg0/home": {
                  "operation": "activate"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("lvm:lv:vg0/home", NodeKind::LvmLogicalVolume, "vg0/home")
            .with_path("/dev/vg0/home")
            .with_property("lvm.active", "inactive"),
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
        diagnostic.action_id == "volumes:vg0/home:activate"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LvmActivateRequired
    }));
}

#[test]
fn topology_comparison_suppresses_lvm_deactivate_action_when_inactive() {
    let plan = plan_from_json_bytes(
        br#"{
              "volumes": {
                "vg0/archive": {
                  "operation": "deactivate"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "lvm:lv:vg0/archive",
            NodeKind::LvmLogicalVolume,
            "vg0/archive",
        )
        .with_path("/dev/vg0/archive")
        .with_property("lvm.active", "inactive"),
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
        diagnostic.action_id == "volumes:vg0/archive:deactivate"
            && diagnostic.kind == TopologyDiagnosticKind::LvmDeactivateAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_lvm_deactivate_action_when_active() {
    let plan = plan_from_json_bytes(
        br#"{
              "volumes": {
                "vg0/archive": {
                  "operation": "deactivate"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "lvm:lv:vg0/archive",
            NodeKind::LvmLogicalVolume,
            "vg0/archive",
        )
        .with_path("/dev/vg0/archive")
        .with_property("lvm.active", "active")
        .with_property("lvm.active-locally", "active locally"),
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
        diagnostic.action_id == "volumes:vg0/archive:deactivate"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LvmDeactivateRequired
    }));
}

#[test]
fn topology_comparison_reconciles_absent_lvm_activation_lifecycle() {
    let plan = plan_from_json_bytes(
        br#"{
              "volumes": {
                "vg0/home": {
                  "operation": "activate"
                },
                "vg0/archive": {
                  "operation": "deactivate"
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
        .any(|action| action.id == "volumes:vg0/home:activate"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "volumes:vg0/home:activate"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LvmActivateRequired
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "volumes:vg0/archive:deactivate"
            && diagnostic.level == TopologyDiagnosticLevel::Info
            && diagnostic.kind == TopologyDiagnosticKind::LvmDeactivateAlreadySatisfied
            && diagnostic.current.is_none()
    }));
}

#[test]
fn topology_comparison_reconciles_lvm_volume_and_thin_pool_create() {
    let plan = plan_from_json_bytes(
        br#"{
              "volumes": {
                "vg0/home": {
                  "operation": "create",
                  "desiredSize": "8GiB"
                },
                "vg0/archive": {
                  "operation": "create",
                  "desiredSize": "8GiB"
                }
              },
              "thinPools": {
                "vg0/pool": {
                  "operation": "create",
                  "desiredSize": "16GiB"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("lvm-lv:vg0/home", NodeKind::LvmLogicalVolume, "vg0/home")
            .with_path("/dev/vg0/home")
            .with_size_bytes(8 * 1024 * 1024 * 1024),
    );
    graph.add_node(
        Node::new(
            "lvm-lv:vg0/archive",
            NodeKind::LvmLogicalVolume,
            "vg0/archive",
        )
        .with_path("/dev/vg0/archive")
        .with_size_bytes(4 * 1024 * 1024 * 1024),
    );
    graph.add_node(
        Node::new("lvm-thin-pool:vg0/pool", NodeKind::LvmThinPool, "vg0/pool")
            .with_path("/dev/vg0/pool")
            .with_size_bytes(16 * 1024 * 1024 * 1024),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 3);
    assert_eq!(comparison.summary.already_satisfied_count, 2);
    assert_eq!(comparison.summary.suppressed_action_count, 2);
    assert_eq!(plan.summary.action_count, 1);
    assert!(plan
        .actions
        .iter()
        .all(|action| action.id == "volumes:vg0/archive:create"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "volumes:vg0/home:create"
            && diagnostic.kind == TopologyDiagnosticKind::LvmVolumeCreateAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "thinpools:vg0/pool:create"
            && diagnostic.kind == TopologyDiagnosticKind::LvmVolumeCreateAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "volumes:vg0/archive:create"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LvmVolumeCreateRequired
            && diagnostic.message.contains("not desired size 8GiB")
            && diagnostic.message.contains("grow or shrink")
    }));
}

#[test]
fn topology_comparison_reconciles_lvm_rename_destinations() {
    let plan = plan_from_json_bytes(
        br#"{
              "volumes": {
                "vg0/home-old": {
                  "operation": "rename",
                  "renameTo": "home-new"
                },
                "vg0/logs-old": {
                  "operation": "rename",
                  "renameTo": "vg0/logs-new"
                },
                "vg0/missing-old": {
                  "operation": "rename",
                  "renameTo": "vg0/missing-new"
                }
              },
              "thinPools": {
                "vg0/thin-old": {
                  "operation": "rename",
                  "renameTo": "thin-new"
                },
                "vg0/pool-old": {
                  "operation": "rename",
                  "renameTo": "vg0/pool-new"
                }
              },
              "volumeGroups": {
                "vg-old": {
                  "operation": "rename",
                  "renameTo": "vg-new"
                },
                "vg-archive-old": {
                  "operation": "rename",
                  "renameTo": "vg-archive-new"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "lvm-lv:vg0/home-old",
            NodeKind::LvmLogicalVolume,
            "vg0/home-old",
        )
        .with_size_bytes(8 * 1024 * 1024 * 1024)
        .with_property("lvm.lv-active", "active"),
    );
    graph.add_node(Node::new(
        "lvm-lv:vg0/logs-new",
        NodeKind::LvmLogicalVolume,
        "vg0/logs-new",
    ));
    graph.add_node(
        Node::new(
            "lvm-thin-pool:vg0/thin-old",
            NodeKind::LvmThinPool,
            "vg0/thin-old",
        )
        .with_property("lvm.data-percent", "12.5")
        .with_property("lvm.metadata-percent", "2.0"),
    );
    graph.add_node(Node::new(
        "lvm-thin-pool:vg0/pool-new",
        NodeKind::LvmThinPool,
        "vg0/pool-new",
    ));
    graph.add_node(
        Node::new("lvm-vg:vg-old", NodeKind::LvmVolumeGroup, "vg-old")
            .with_property("lvm.vg-partial", "complete"),
    );
    graph.add_node(Node::new(
        "lvm-vg:vg-archive-new",
        NodeKind::LvmVolumeGroup,
        "vg-archive-new",
    ));

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 7);
    assert_eq!(comparison.summary.already_satisfied_count, 3);
    assert_eq!(comparison.summary.suppressed_action_count, 3);
    assert_eq!(comparison.summary.missing_count, 0);
    assert_eq!(plan.actions.len(), 4);
    assert!(plan
        .actions
        .iter()
        .any(|action| action.id == "volumes:vg0/home-old:rename"));
    assert!(plan
        .actions
        .iter()
        .any(|action| action.id == "volumes:vg0/missing-old:rename"));
    assert!(plan
        .actions
        .iter()
        .any(|action| action.id == "thinpools:vg0/thin-old:rename"));
    assert!(plan
        .actions
        .iter()
        .any(|action| action.id == "volumegroups:vg-old:rename"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "volumes:vg0/logs-old:rename"
            && diagnostic.kind == TopologyDiagnosticKind::LvmRenameAlreadySatisfied
            && diagnostic.message.contains("vg0/logs-new")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "thinpools:vg0/pool-old:rename"
            && diagnostic.kind == TopologyDiagnosticKind::LvmRenameAlreadySatisfied
            && diagnostic.message.contains("vg0/pool-new")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "volumegroups:vg-archive-old:rename"
            && diagnostic.kind == TopologyDiagnosticKind::LvmRenameAlreadySatisfied
            && diagnostic.message.contains("vg-archive-new")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "volumes:vg0/home-old:rename"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LvmRenameRequired
            && diagnostic.message.contains("rename to vg0/home-new")
            && diagnostic.message.contains("active active")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "thinpools:vg0/thin-old:rename"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LvmRenameRequired
            && diagnostic.message.contains("rename to vg0/thin-new")
            && diagnostic.message.contains("data 12.5")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "volumegroups:vg-old:rename"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LvmRenameRequired
            && diagnostic.message.contains("rename to vg-new")
            && diagnostic.message.contains("partial complete")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "volumes:vg0/missing-old:rename"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LvmRenameRequired
            && diagnostic
                .message
                .contains("destination vg0/missing-new is absent")
    }));
}

#[test]
fn topology_comparison_suppresses_imported_lvm_volume_group() {
    let plan = plan_from_json_bytes(
        br#"{
              "volumeGroups": {
                "vg0": {
                  "operation": "import"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(Node::new("lvm-vg:vg0", NodeKind::LvmVolumeGroup, "vg0"));

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
        diagnostic.action_id == "volumegroups:vg0:import"
            && diagnostic.kind == TopologyDiagnosticKind::LvmVgImportAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_reconciles_lvm_volume_group_create() {
    let plan = plan_from_json_bytes(
        br#"{
              "volumeGroups": {
                "vg-present": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/pv-present"
                },
                "vg-exported": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/pv-exported"
                },
                "vg-partial": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/pv-partial"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(Node::new(
        "lvm-vg:vg-present",
        NodeKind::LvmVolumeGroup,
        "vg-present",
    ));
    graph.add_node(
        Node::new(
            "lvm-vg:vg-exported",
            NodeKind::LvmVolumeGroup,
            "vg-exported",
        )
        .with_property("lvm.vg-exported", "exported"),
    );
    graph.add_node(
        Node::new("lvm-vg:vg-partial", NodeKind::LvmVolumeGroup, "vg-partial")
            .with_property("lvm.vg-partial", "partial")
            .with_property("lvm.missing-pv-count", "1"),
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
        .all(|action| { action.id != "volumegroups:vg-present:create" }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "volumegroups:vg-present:create"
            && diagnostic.kind == TopologyDiagnosticKind::LvmVgCreateAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "volumegroups:vg-exported:create"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LvmVgCreateRequired
            && diagnostic.message.contains("lvm.vg-exported=exported")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "volumegroups:vg-partial:create"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LvmVgCreateRequired
            && diagnostic.message.contains("lvm.vg-partial=partial")
            && diagnostic.message.contains("1 missing physical volume")
    }));
}

#[test]
fn topology_comparison_keeps_lvm_volume_group_import_when_exported() {
    let plan = plan_from_json_bytes(
        br#"{
              "volumeGroups": {
                "vg0": {
                  "operation": "import"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("lvm-vg:vg0", NodeKind::LvmVolumeGroup, "vg0")
            .with_property("lvm.vg-exported", "exported"),
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
        diagnostic.action_id == "volumegroups:vg0:import"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LvmVgImportRequired
    }));
}

#[test]
fn topology_comparison_suppresses_exported_lvm_volume_group() {
    let plan = plan_from_json_bytes(
        br#"{
              "volumeGroups": {
                "vg0": {
                  "operation": "export"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("lvm-vg:vg0", NodeKind::LvmVolumeGroup, "vg0")
            .with_property("lvm.vg-exported", "exported"),
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
        diagnostic.action_id == "volumegroups:vg0:export"
            && diagnostic.kind == TopologyDiagnosticKind::LvmVgExportAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_lvm_volume_group_export_when_imported() {
    let plan = plan_from_json_bytes(
        br#"{
              "volumeGroups": {
                "vg0": {
                  "operation": "export"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(Node::new("lvm-vg:vg0", NodeKind::LvmVolumeGroup, "vg0"));

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
        diagnostic.action_id == "volumegroups:vg0:export"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LvmVgExportRequired
    }));
}

#[test]
fn topology_comparison_suppresses_lvm_cache_detach_when_origin_uncached() {
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
    let mut graph = StorageGraph::empty();
    graph.add_node(Node::new(
        "lvm-lv:vg0/root",
        NodeKind::LvmLogicalVolume,
        "vg0/root",
    ));

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
        diagnostic.action_id == "lvmCaches:vg0/root:remove-device:vg0/root-cache"
            && diagnostic.kind == TopologyDiagnosticKind::LvmCacheDetachAlreadySatisfied
            && diagnostic.query == "vg0/root"
    }));
}

#[test]
fn topology_comparison_keeps_lvm_cache_detach_when_origin_cached() {
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
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("lvm-lv:vg0/root", NodeKind::LvmCache, "vg0/root")
            .with_property("lvm.pool", "root-cache")
            .with_property("lvm.cache-mode", "writeback")
            .with_property("lvm.cache-policy", "smq")
            .with_property("lvm.cache-dirty-blocks", "64")
            .with_property("lvm.data-percent", "12.00"),
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
        diagnostic.action_id == "lvmCaches:vg0/root:remove-device:vg0/root-cache"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LvmCacheDetachRequired
            && diagnostic.message.contains("cache pool root-cache")
            && diagnostic.message.contains("cache mode writeback")
            && diagnostic.message.contains("dirty blocks 64")
    }));
}
