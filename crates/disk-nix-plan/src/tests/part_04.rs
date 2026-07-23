#[test]
fn plan_classifies_zram_rescan_and_properties() {
    let plan = plan_from_json_bytes(
        br#"{
              "zram": {
                "enable": true,
                "operation": "rescan",
                "swapDevices": 2,
                "memoryPercent": 40,
                "memoryMax": 8589934592,
                "priority": 20,
                "algorithm": "zstd",
                "properties": {
                  "zram.compression-ratio-target": "2.0"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 2);
    assert_eq!(plan.summary.offline_required_count, 1);
    assert_eq!(plan.summary.unsupported_count, 0);

    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "zram:rescan")
        .expect("zram rescan action exists");
    assert_eq!(rescan.operation, Operation::Rescan);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert!(!rescan.destructive);
    assert_eq!(rescan.context.collection.as_deref(), Some("zram"));

    let property = plan
        .actions
        .iter()
        .find(|action| action.id == "zram:set-property:zram.compression-ratio-target")
        .expect("zram property action exists");
    assert_eq!(property.operation, Operation::SetProperty);
    assert_eq!(property.risk, RiskClass::OfflineRequired);
    assert_eq!(property.context.property_value.as_deref(), Some("2.0"));
    assert!(property.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("zramSwap"))
    }));
}

#[test]
fn topology_comparison_reconciles_zram_property_aliases() {
    let plan = plan_from_json_bytes(
        br#"{
              "zram": {
                "enable": true,
                "properties": {
                  "algorithm": "zstd",
                  "zram.compression-ratio-target": "2.0",
                  "priority": "20"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("block:/dev/zram0", NodeKind::ZramDevice, "/dev/zram0")
            .with_path("/dev/zram0")
            .with_property("zram.algorithm", "zstd")
            .with_property("zram.compression-ratio", "2.00")
            .with_property("zram.swap", "true"),
    );
    graph.add_node(
        Node::new("swap:/dev/zram0", NodeKind::Swap, "/dev/zram0")
            .with_path("/dev/zram0")
            .with_property("swap.priority", "10"),
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
    assert!(plan.actions.iter().any(|action| {
        action.id == "zram:set-property:priority" && action.operation == Operation::SetProperty
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "zram:set-property:algorithm"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "zram:set-property:zram.compression-ratio-target"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "zram:set-property:priority"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::PropertyDiffers
            && diagnostic.message.contains("is 10")
            && diagnostic.message.contains("desired 20")
    }));
}

#[test]
fn plan_accepts_luks_header_identity_properties() {
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
                      "luks.uuid": "01234567-89ab-cdef-0123-456789abcdef",
                      "priority": "prefer"
                    }
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.unsupported_count, 1);

    let label = plan
        .actions
        .iter()
        .find(|action| action.id == "luks.devices:cryptroot:set-property:label")
        .expect("LUKS label property action exists");
    assert_eq!(label.operation, Operation::SetProperty);
    assert_eq!(label.risk, RiskClass::OfflineRequired);
    assert_eq!(label.context.target.as_deref(), Some("cryptroot"));
    assert_eq!(
        label.context.device.as_deref(),
        Some("/dev/disk/by-id/root-luks")
    );
    assert_eq!(label.context.property_value.as_deref(), Some("root"));

    let subsystem = plan
        .actions
        .iter()
        .find(|action| action.id == "luks.devices:cryptroot:set-property:luks.subsystem")
        .expect("LUKS subsystem property action exists");
    assert_eq!(subsystem.risk, RiskClass::OfflineRequired);
    assert_eq!(subsystem.context.property_value.as_deref(), Some("nixos"));

    let uuid = plan
        .actions
        .iter()
        .find(|action| action.id == "luks.devices:cryptroot:set-property:luks.uuid")
        .expect("LUKS UUID property action exists");
    assert_eq!(uuid.risk, RiskClass::OfflineRequired);
    assert_eq!(
        uuid.context.property_value.as_deref(),
        Some("01234567-89ab-cdef-0123-456789abcdef")
    );

    let unsupported = plan
        .actions
        .iter()
        .find(|action| action.id == "luks.devices:cryptroot:set-property:priority")
        .expect("unsupported LUKS property action exists");
    assert_eq!(unsupported.risk, RiskClass::Unsupported);
    assert!(unsupported.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("luksKeyslots or luksTokens"))
    }));
}

#[test]
fn plan_classifies_luks_keyslot_lifecycle() {
    let plan = plan_from_json_bytes(
        br#"{
              "luksKeyslots": {
                "cryptroot:1": {
                  "operation": "add-key",
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "keySlot": "1",
                    "newKeyFile": "/run/keys/root-new"
                  }
                },
                "cryptroot:2": {
                  "operation": "remove-key",
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "keySlot": "2"
                  }
                },
                "cryptroot:3": {
                  "properties": {
                    "keyFile": "/run/keys/root-rotated"
                  },
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "keySlot": "3",
                    "keyFile": "/run/keys/root-old"
                  }
                },
                "cryptroot:4": {
                  "properties": {
                    "priority": "prefer"
                  },
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "keySlot": "4"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 4);
    assert_eq!(plan.summary.offline_required_count, 3);
    assert_eq!(plan.summary.potential_data_loss_count, 1);
    let create = plan
        .actions
        .iter()
        .find(|action| action.id == "lukskeyslots:cryptroot:1:add-key")
        .expect("LUKS keyslot add-key action exists");
    assert_eq!(create.risk, RiskClass::OfflineRequired);
    assert_eq!(
        create.context.device.as_deref(),
        Some("/dev/disk/by-id/root-luks")
    );
    assert_eq!(create.context.key_slot.as_deref(), Some("1"));
    assert_eq!(
        create.context.new_key_file.as_deref(),
        Some("/run/keys/root-new")
    );

    let destroy = plan
        .actions
        .iter()
        .find(|action| action.id == "lukskeyslots:cryptroot:2:remove-key")
        .expect("LUKS keyslot remove-key action exists");
    assert_eq!(destroy.risk, RiskClass::PotentialDataLoss);
    assert!(!destroy.destructive);

    let change = plan
        .actions
        .iter()
        .find(|action| action.id == "luksKeyslots:cryptroot:3:set-property:keyFile")
        .expect("LUKS keyslot change action exists");
    assert_eq!(change.risk, RiskClass::OfflineRequired);
    assert_eq!(change.context.key_slot.as_deref(), Some("3"));
    assert_eq!(
        change.context.key_file.as_deref(),
        Some("/run/keys/root-old")
    );
    assert_eq!(
        change.context.property_value.as_deref(),
        Some("/run/keys/root-rotated")
    );

    let priority = plan
        .actions
        .iter()
        .find(|action| action.id == "luksKeyslots:cryptroot:4:set-property:priority")
        .expect("LUKS keyslot priority action exists");
    assert_eq!(priority.risk, RiskClass::OfflineRequired);
    assert_eq!(priority.context.key_slot.as_deref(), Some("4"));
    assert_eq!(priority.context.property_value.as_deref(), Some("prefer"));
}

#[test]
fn plan_rejects_unsupported_luks_keyslot_properties() {
    let plan = plan_from_json_bytes(
        br#"{
              "luksKeyslots": {
                "cryptroot:1": {
                  "properties": {
                    "pbkdf": "argon2id",
                    "priority": "urgent"
                  },
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "keySlot": "1"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 2);
    assert_eq!(plan.summary.unsupported_count, 2);
    assert!(plan.actions.iter().all(|action| {
        action.risk == RiskClass::Unsupported
            && action.advice.as_ref().is_some_and(|advice| {
                advice
                    .alternatives
                    .iter()
                    .any(|alternative| alternative.contains("keyslot"))
            })
    }));
}

#[test]
fn plan_classifies_luks_token_lifecycle() {
    let plan = plan_from_json_bytes(
        br#"{
              "luksTokens": {
                "cryptroot:0": {
                  "operation": "import-token",
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "tokenId": "0",
                    "tokenFile": "/run/keys/root-token.json"
                  }
                },
                "cryptroot:1": {
                  "operation": "remove-token",
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "tokenId": "1"
                  }
                },
                "cryptroot:2": {
                  "properties": {
                    "tokenFile": "/run/keys/root-token-new.json"
                  },
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "tokenId": "2"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 3);
    assert_eq!(plan.summary.offline_required_count, 2);
    assert_eq!(plan.summary.potential_data_loss_count, 1);
    let create = plan
        .actions
        .iter()
        .find(|action| action.id == "lukstokens:cryptroot:0:import-token")
        .expect("LUKS token import-token action exists");
    assert_eq!(create.risk, RiskClass::OfflineRequired);
    assert_eq!(
        create.context.device.as_deref(),
        Some("/dev/disk/by-id/root-luks")
    );
    assert_eq!(create.context.token_id.as_deref(), Some("0"));
    assert_eq!(
        create.context.token_file.as_deref(),
        Some("/run/keys/root-token.json")
    );

    let destroy = plan
        .actions
        .iter()
        .find(|action| action.id == "lukstokens:cryptroot:1:remove-token")
        .expect("LUKS token remove-token action exists");
    assert_eq!(destroy.risk, RiskClass::PotentialDataLoss);
    assert!(!destroy.destructive);

    let change = plan
        .actions
        .iter()
        .find(|action| action.id == "luksTokens:cryptroot:2:set-property:tokenFile")
        .expect("LUKS token change action exists");
    assert_eq!(change.risk, RiskClass::OfflineRequired);
    assert_eq!(change.context.token_id.as_deref(), Some("2"));
    assert_eq!(
        change.context.property_value.as_deref(),
        Some("/run/keys/root-token-new.json")
    );
}

#[test]
fn plan_classifies_vdo_lifecycle_with_vdo_advice() {
    let plan = plan_from_json_bytes(
        br#"{
              "vdoVolumes": {
                "new-cache": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/vdo-backing",
                  "desiredSize": "2TiB"
                },
                "archive": {
                  "operation": "grow",
                  "desiredSize": "4TiB",
                  "physicalSize": "6TiB",
                  "properties": {
                    "writePolicy": "sync",
                    "compression": "enabled",
                    "deduplication": "disabled"
                  }
                },
                "warmArchive": {
                  "operation": "start"
                },
                "coldArchive": {
                  "operation": "stop"
                },
                "refreshArchive": {
                  "operation": "rescan"
                },
                "old-cache": {
                  "destroy": true
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 9);
    assert_eq!(plan.summary.offline_required_count, 2);
    assert_eq!(plan.summary.destructive_count, 2);
    let create = plan
        .actions
        .iter()
        .find(|action| action.id == "vdovolumes:new-cache:create")
        .expect("VDO create action exists");
    assert_eq!(create.risk, RiskClass::Destructive);
    assert!(create.destructive);
    assert_eq!(
        create.context.device.as_deref(),
        Some("/dev/disk/by-id/vdo-backing")
    );
    assert_eq!(create.context.desired_size.as_deref(), Some("2TiB"));
    let grow = plan
        .actions
        .iter()
        .find(|action| action.id == "vdovolumes:archive:grow")
        .expect("VDO grow action exists");
    assert_eq!(grow.risk, RiskClass::Online);
    assert_eq!(grow.context.desired_size.as_deref(), Some("4TiB"));
    assert_eq!(grow.context.physical_size.as_deref(), Some("6TiB"));
    assert!(grow.advice.as_ref().is_some_and(|advice| {
        advice.summary.contains("logical size")
            && advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("vdostats"))
    }));
    let write_policy = plan
        .actions
        .iter()
        .find(|action| action.id == "vdoVolumes:archive:set-property:writePolicy")
        .expect("VDO write policy property action exists");
    assert_eq!(write_policy.risk, RiskClass::Safe);
    assert_eq!(write_policy.context.property_value.as_deref(), Some("sync"));
    assert!(plan.actions.iter().any(|action| {
        action.id == "vdoVolumes:archive:set-property:compression" && action.risk == RiskClass::Safe
    }));
    assert!(plan.actions.iter().any(|action| {
        action.id == "vdoVolumes:archive:set-property:deduplication"
            && action.risk == RiskClass::Safe
    }));
    let start = plan
        .actions
        .iter()
        .find(|action| action.id == "vdovolumes:warmarchive:start")
        .expect("VDO start action exists");
    assert_eq!(start.operation, Operation::Start);
    assert_eq!(start.risk, RiskClass::OfflineRequired);
    assert!(!start.destructive);
    assert!(start.advice.as_ref().is_some_and(|advice| {
        advice.summary.contains("activates")
            && advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("backing device"))
    }));
    let stop = plan
        .actions
        .iter()
        .find(|action| action.id == "vdovolumes:coldarchive:stop")
        .expect("VDO stop action exists");
    assert_eq!(stop.operation, Operation::Stop);
    assert_eq!(stop.risk, RiskClass::OfflineRequired);
    assert!(!stop.destructive);
    assert!(stop.advice.as_ref().is_some_and(|advice| {
        advice.summary.contains("preserving VDO metadata")
            && advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("stop over remove"))
    }));
    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "vdovolumes:refresharchive:rescan")
        .expect("VDO rescan action exists");
    assert_eq!(rescan.operation, Operation::Rescan);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert!(!rescan.destructive);
    let destroy = plan
        .actions
        .iter()
        .find(|action| action.id == "vdovolumes:old-cache:destroy")
        .expect("VDO destroy action exists");
    assert_eq!(destroy.risk, RiskClass::Destructive);
}

#[test]
fn plan_rejects_unsupported_vdo_property_updates() {
    let plan = plan_from_json_bytes(
        br#"{
              "vdoVolumes": {
                "archive": {
                  "properties": {
                    "writePolicy": "eventual",
                    "compression": "maybe",
                    "deduplication": "off",
                    "indexMemory": "0.5"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 4);
    assert_eq!(plan.summary.unsupported_count, 3);
    assert!(plan.actions.iter().any(|action| {
        action.id == "vdoVolumes:archive:set-property:deduplication"
            && action.risk == RiskClass::Safe
    }));

    let write_policy = plan
        .actions
        .iter()
        .find(|action| action.id == "vdoVolumes:archive:set-property:writePolicy")
        .expect("VDO write policy property action exists");
    assert_eq!(write_policy.risk, RiskClass::Unsupported);
    assert!(write_policy.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("auto, sync, or async"))
    }));

    assert!(plan.actions.iter().any(|action| {
        action.id == "vdoVolumes:archive:set-property:compression"
            && action.risk == RiskClass::Unsupported
    }));
    assert!(plan.actions.iter().any(|action| {
        action.id == "vdoVolumes:archive:set-property:indexMemory"
            && action.risk == RiskClass::Unsupported
    }));
}

#[test]
fn plan_accepts_btrfs_subvolume_lifecycle_with_target_path() {
    let plan = plan_from_json_bytes(
        br#"{
              "btrfsSubvolumes": {
                "/mnt/persist/@home": {
                  "operation": "create",
                  "path": "/mnt/persist/@home"
                },
                "/mnt/persist/@inventory": {
                  "operation": "rescan",
                  "path": "/mnt/persist/@inventory"
                },
                "/mnt/persist/@old-name": {
                  "operation": "rename",
                  "renameTo": "/mnt/persist/@new-name"
                },
                "/mnt/persist/@old": {
                  "destroy": true,
                  "preserveData": false
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 4);
    assert_eq!(plan.summary.offline_required_count, 1);
    let create = plan
        .actions
        .iter()
        .find(|action| {
            action.id == "btrfsSubvolumes:/mnt/persist/@home:create".to_ascii_lowercase()
        })
        .expect("create action exists");
    assert_eq!(create.risk, RiskClass::Online);
    assert_eq!(create.context.target.as_deref(), Some("/mnt/persist/@home"));
    let rescan = plan
        .actions
        .iter()
        .find(|action| {
            action.id == "btrfsSubvolumes:/mnt/persist/@inventory:rescan".to_ascii_lowercase()
        })
        .expect("rescan action exists");
    assert_eq!(rescan.operation, Operation::Rescan);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert!(!rescan.destructive);
    assert_eq!(
        rescan.context.target.as_deref(),
        Some("/mnt/persist/@inventory")
    );
    let rename = plan
        .actions
        .iter()
        .find(|action| {
            action.id == "btrfsSubvolumes:/mnt/persist/@old-name:rename".to_ascii_lowercase()
        })
        .expect("rename action exists");
    assert_eq!(rename.operation, Operation::Rename);
    assert_eq!(rename.risk, RiskClass::OfflineRequired);
    assert_eq!(
        rename.context.rename_to.as_deref(),
        Some("/mnt/persist/@new-name")
    );
    let destroy = plan
        .actions
        .iter()
        .find(|action| {
            action.id == "btrfsSubvolumes:/mnt/persist/@old:destroy".to_ascii_lowercase()
        })
        .expect("destroy action exists");
    assert_eq!(destroy.risk, RiskClass::Destructive);
    assert!(destroy.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("read-only snapshot"))
    }));
}

#[test]
fn plan_accepts_btrfs_qgroup_rescan_as_online_refresh() {
    let plan = plan_from_json_bytes(
        br#"{
              "btrfsQgroups": {
                "0/257": {
                  "operation": "rescan",
                  "target": "/mnt/persist"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 1);
    assert_eq!(plan.summary.offline_required_count, 0);
    assert_eq!(plan.summary.destructive_count, 0);
    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "btrfsqgroups:0/257:rescan")
        .expect("Btrfs qgroup rescan action exists");
    assert_eq!(rescan.operation, Operation::Rescan);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert!(!rescan.destructive);
    assert_eq!(rescan.context.target.as_deref(), Some("/mnt/persist"));
    assert!(rescan
        .advice
        .as_ref()
        .is_some_and(|advice| { advice.summary.contains("Btrfs qgroup rescan refreshes") }));
}

#[test]
fn plan_classifies_btrfs_subvolume_property_support() {
    let plan = plan_from_json_bytes(
        br#"{
              "btrfsSubvolumes": {
                "/mnt/persist/@home": {
                  "path": "/mnt/persist/@home",
                  "properties": {
                    "readonly": true,
                    "compression": "zstd"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 2);
    assert_eq!(plan.summary.unsupported_count, 1);
    let readonly = plan
        .actions
        .iter()
        .find(|action| action.id == "btrfsSubvolumes:/mnt/persist/@home:set-property:readonly")
        .expect("readonly property action exists");
    assert_eq!(readonly.risk, RiskClass::Safe);

    let compression = plan
        .actions
        .iter()
        .find(|action| action.id == "btrfsSubvolumes:/mnt/persist/@home:set-property:compression")
        .expect("unsupported property action exists");
    assert_eq!(compression.risk, RiskClass::Unsupported);
    assert!(compression.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("readOnly"))
    }));
}

#[test]
fn plan_accepts_zvol_lifecycle_with_zfs_advice() {
    let plan = plan_from_json_bytes(
        br#"{
              "zvols": {
                "tank/vm/root": {
                  "operation": "grow",
                  "desiredSize": "80GiB"
                },
                "tank/vm/tmp": {
                  "operation": "create",
                  "desiredSize": "20GiB"
                },
                "tank/vm/inventory": {
                  "operation": "rescan"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 3);
    assert_eq!(plan.summary.offline_required_count, 0);
    let grow = plan
        .actions
        .iter()
        .find(|action| action.id == "zvols:tank/vm/root:grow")
        .expect("zvol grow action exists");
    assert_eq!(grow.risk, RiskClass::Online);
    assert_eq!(grow.context.desired_size.as_deref(), Some("80GiB"));
    assert!(grow.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("rescan dependent"))
    }));
    let create = plan
        .actions
        .iter()
        .find(|action| action.id == "zvols:tank/vm/tmp:create")
        .expect("zvol create action exists");
    assert_eq!(create.risk, RiskClass::Online);
    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "zvols:tank/vm/inventory:rescan")
        .expect("zvol rescan action exists");
    assert_eq!(rescan.operation, Operation::Rescan);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert!(!rescan.destructive);
}

#[test]
fn plan_classifies_zfs_dataset_lifecycle() {
    let plan = plan_from_json_bytes(
        br#"{
              "datasets": {
                "tank/home": {
                  "operation": "create",
                  "mountpoint": "/home",
                  "properties": {
                    "compression": "zstd",
                    "mountpoint": "/home"
                  }
                },
                "tank/inventory": {
                  "operation": "rescan"
                },
                "tank/archive": {
                  "destroy": true
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 5);
    assert_eq!(plan.summary.destructive_count, 1);
    let create = plan
        .actions
        .iter()
        .find(|action| action.id == "datasets:tank/home:create")
        .expect("dataset create action exists");
    assert_eq!(create.risk, RiskClass::Online);
    assert_eq!(create.context.target.as_deref(), Some("tank/home"));
    assert_eq!(
        create.context.property_assignments,
        vec![
            "compression=zstd".to_string(),
            "mountpoint=/home".to_string()
        ]
    );
    assert!(create.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("mountpoint"))
    }));
    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "datasets:tank/inventory:rescan")
        .expect("dataset rescan action exists");
    assert_eq!(rescan.operation, Operation::Rescan);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert!(!rescan.destructive);
    let destroy = plan
        .actions
        .iter()
        .find(|action| action.id == "datasets:tank/archive:destroy")
        .expect("dataset destroy action exists");
    assert_eq!(destroy.risk, RiskClass::Destructive);
    assert!(destroy.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("recursive snapshot"))
    }));
}

#[test]
fn plan_classifies_md_raid_lifecycle_with_redundancy_advice() {
    let plan = plan_from_json_bytes(
        br#"{
              "mdRaids": {
                "newroot": {
                  "target": "/dev/md/newroot",
                  "operation": "create",
                  "level": "1",
                  "devices": [
                    "/dev/disk/by-id/nvme-a",
                    "/dev/disk/by-id/nvme-b"
                  ]
                },
                "existing": {
                  "target": "/dev/md/existing",
                  "operation": "assemble",
                  "devices": [
                    "/dev/disk/by-id/existing-a",
                    "/dev/disk/by-id/existing-b"
                  ]
                },
                "oldroot": {
                  "target": "/dev/md/oldroot",
                  "operation": "stop"
                },
                "inventory": {
                  "operation": "rescan"
                },
                "root": {
                  "target": "/dev/md/root",
                  "operation": "grow",
                  "desiredSize": "max",
                  "addDevices": ["/dev/disk/by-id/nvme-spare"],
                  "replaceDevices": {
                    "/dev/disk/by-id/old-md-member": "/dev/disk/by-id/new-md-member"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 7);
    assert_eq!(plan.summary.destructive_count, 1);
    assert_eq!(plan.summary.offline_required_count, 4);
    let create = plan
        .actions
        .iter()
        .find(|action| action.id == "mdraids:newroot:create")
        .expect("md create action exists");
    assert_eq!(create.risk, RiskClass::Destructive);
    assert_eq!(create.context.level.as_deref(), Some("1"));
    assert_eq!(
        create.context.devices,
        vec![
            "/dev/disk/by-id/nvme-a".to_string(),
            "/dev/disk/by-id/nvme-b".to_string(),
        ]
    );
    let assemble = plan
        .actions
        .iter()
        .find(|action| action.id == "mdraids:existing:assemble")
        .expect("md assemble action exists");
    assert_eq!(assemble.operation, Operation::Assemble);
    assert_eq!(assemble.risk, RiskClass::OfflineRequired);
    assert!(!assemble.destructive);
    assert_eq!(assemble.context.target.as_deref(), Some("/dev/md/existing"));
    assert_eq!(
        assemble.context.devices,
        vec![
            "/dev/disk/by-id/existing-a".to_string(),
            "/dev/disk/by-id/existing-b".to_string(),
        ]
    );
    let stop = plan
        .actions
        .iter()
        .find(|action| action.id == "mdraids:oldroot:stop")
        .expect("md stop action exists");
    assert_eq!(stop.operation, Operation::Stop);
    assert_eq!(stop.risk, RiskClass::OfflineRequired);
    assert!(!stop.destructive);
    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "mdraids:inventory:rescan")
        .expect("md rescan action exists");
    assert_eq!(rescan.operation, Operation::Rescan);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert!(!rescan.destructive);
    let grow = plan
        .actions
        .iter()
        .find(|action| action.id == "mdraids:root:grow")
        .expect("md grow action exists");
    assert_eq!(grow.risk, RiskClass::OfflineRequired);
    assert_eq!(grow.context.target.as_deref(), Some("/dev/md/root"));
    assert!(grow.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("/proc/mdstat"))
    }));
    let add = plan
        .actions
        .iter()
        .find(|action| action.id == "mdRaids:root:add-device:/dev/disk/by-id/nvme-spare")
        .expect("md add action exists");
    assert_eq!(add.risk, RiskClass::Online);
    let replace = plan
        .actions
        .iter()
        .find(|action| action.id == "mdRaids:root:replace-device:/dev/disk/by-id/old-md-member")
        .expect("md replace action exists");
    assert_eq!(replace.risk, RiskClass::OfflineRequired);
}
