#[test]
fn plan_warns_for_pool_device_removal_and_dataset_destroy() {
    let plan = plan_from_json_bytes(
        br#"{
              "spec": {
                "pools": {
                  "tank": {
                    "removeDevices": ["/dev/sdb"],
                    "properties": {
                      "autotrim": "on"
                    }
                  }
                },
                "datasets": {
                  "tank/old": {
                    "destroy": true
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 3);
    assert_eq!(plan.summary.destructive_count, 1);
    assert_eq!(plan.summary.potential_data_loss_count, 1);
    assert!(plan.actions.iter().any(|action| {
        action.operation == Operation::RemoveDevice
            && action.risk == RiskClass::PotentialDataLoss
            && action.context.device.as_deref() == Some("/dev/sdb")
            && action.advice.is_some()
    }));
}

#[test]
fn plan_classifies_zfs_pool_lifecycle() {
    let plan = plan_from_json_bytes(
        br#"{
              "pools": {
                "tank": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/pool-vdev0"
                },
                "oldtank": {
                  "destroy": true
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 2);
    assert_eq!(plan.summary.destructive_count, 2);
    let create = plan
        .actions
        .iter()
        .find(|action| action.id == "pools:tank:create")
        .expect("pool create action exists");
    assert_eq!(create.risk, RiskClass::Destructive);
    assert!(create.destructive);
    assert_eq!(
        create.context.device.as_deref(),
        Some("/dev/disk/by-id/pool-vdev0")
    );
    assert!(create.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("zpool create"))
    }));
    let destroy = plan
        .actions
        .iter()
        .find(|action| action.id == "pools:oldtank:destroy")
        .expect("pool destroy action exists");
    assert_eq!(destroy.risk, RiskClass::Destructive);
}

#[test]
fn plan_accepts_zfs_pool_import_export_lifecycle() {
    let plan = plan_from_json_bytes(
        br#"{
              "pools": {
                "tank": {
                  "operation": "export"
                },
                "vault": {
                  "operation": "import",
                  "readOnly": true
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 2);
    assert_eq!(plan.summary.offline_required_count, 2);
    assert_eq!(plan.summary.destructive_count, 0);
    let export = plan
        .actions
        .iter()
        .find(|action| action.id == "pools:tank:export")
        .expect("pool export action exists");
    assert_eq!(export.risk, RiskClass::OfflineRequired);
    assert!(!export.destructive);
    assert!(export.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("instead of destroying"))
    }));
    let import = plan
        .actions
        .iter()
        .find(|action| action.id == "pools:vault:import")
        .expect("pool import action exists");
    assert_eq!(import.risk, RiskClass::OfflineRequired);
    assert_eq!(import.context.read_only, Some(true));
    assert!(import.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("read-only"))
    }));
}

#[test]
fn zfs_pool_import_export_capabilities_are_advertised() {
    let capabilities = default_capabilities();

    assert!(capabilities.iter().any(|capability| {
        capability.node_kind == NodeKind::ZfsPool
            && capability.operation == Operation::Import
            && capability.risk == RiskClass::OfflineRequired
    }));
    assert!(capabilities.iter().any(|capability| {
        capability.node_kind == NodeKind::ZfsPool
            && capability.operation == Operation::Export
            && capability.risk == RiskClass::OfflineRequired
    }));
}

#[test]
fn plan_classifies_snapshot_rollback_as_potential_data_loss() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "tank/home@before-upgrade": {
                  "target": "tank/home",
                  "rollback": true,
                  "recursiveRollback": true
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 1);
    assert_eq!(plan.summary.potential_data_loss_count, 1);
    assert_eq!(plan.actions[0].operation, Operation::Rollback);
    assert_eq!(plan.actions[0].context.recursive_rollback, Some(true));
}

#[test]
fn plan_accepts_snapshot_clone_as_reversible() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "tank/home@before-upgrade": {
                  "target": "tank/home",
                  "cloneTo": "tank/home-review"
                },
                "/mnt/persist/@home-before": {
                  "target": "/mnt/persist/@home",
                  "cloneTo": "/mnt/persist/@home-review",
                  "readOnly": true
                }
              }
            }"#,
    )
    .expect("document should parse");

    assert_eq!(plan.summary.action_count, 2);
    let zfs_clone = plan
        .actions
        .iter()
        .find(|action| action.id == "snapshot:tank/home@before-upgrade:clone:tank/home-review")
        .expect("ZFS clone action exists");
    assert_eq!(zfs_clone.operation, Operation::Clone);
    assert_eq!(zfs_clone.risk, RiskClass::Reversible);
    assert_eq!(
        zfs_clone.context.name.as_deref(),
        Some("tank/home@before-upgrade")
    );
    assert_eq!(
        zfs_clone.context.target.as_deref(),
        Some("tank/home-review")
    );
    let btrfs_clone = plan
        .actions
        .iter()
        .find(|action| {
            action.id == "snapshot:/mnt/persist/@home-before:clone:/mnt/persist/@home-review"
        })
        .expect("Btrfs clone action exists");
    assert_eq!(btrfs_clone.operation, Operation::Clone);
    assert_eq!(btrfs_clone.risk, RiskClass::Reversible);
    assert_eq!(
        btrfs_clone.context.name.as_deref(),
        Some("/mnt/persist/@home-before")
    );
    assert_eq!(
        btrfs_clone.context.target.as_deref(),
        Some("/mnt/persist/@home-review")
    );
    assert_eq!(btrfs_clone.context.read_only, Some(true));
}

#[test]
fn plan_accepts_storage_rename_as_offline_non_destructive() {
    let plan = plan_from_json_bytes(
        br#"{
              "spec": {
                "datasets": {
                  "tank/home": {
                    "operation": "rename",
                    "renameTo": "tank/home-staged"
                  }
                },
                "volumes": {
                  "vg0/old": {
                    "operation": "rename",
                    "renameTo": "vg0/new"
                  }
                },
                "snapshots": {
                  "tank/home@before-prune": {
                    "target": "tank/home",
                    "renameTo": "tank/home@retained"
                  }
                }
              }
            }"#,
    )
    .expect("document should parse");

    assert_eq!(plan.summary.action_count, 3);
    assert_eq!(plan.summary.offline_required_count, 3);
    assert_eq!(plan.summary.destructive_count, 0);
    assert!(plan.actions.iter().all(|action| {
        action.operation == Operation::Rename
            && action.risk == RiskClass::OfflineRequired
            && !action.destructive
    }));
    assert!(plan.actions.iter().any(|action| {
        action.id == "datasets:tank/home:rename"
            && action.context.rename_to.as_deref() == Some("tank/home-staged")
    }));
    assert!(plan.actions.iter().any(|action| {
        action.id == "snapshot:tank/home@before-prune:rename:tank/home@retained"
            && action.context.rename_to.as_deref() == Some("tank/home@retained")
    }));
}

#[test]
fn plan_accepts_zfs_clone_promotion_as_offline_non_destructive() {
    let plan = plan_from_json_bytes(
        br#"{
              "spec": {
                "datasets": {
                  "tank/home-review": {
                    "operation": "promote"
                  }
                },
                "zvols": {
                  "tank/vm/root-review": {
                    "operation": "promote"
                  }
                }
              }
            }"#,
    )
    .expect("document should parse");

    assert_eq!(plan.summary.action_count, 2);
    assert_eq!(plan.summary.offline_required_count, 2);
    assert_eq!(plan.summary.destructive_count, 0);
    assert!(plan.actions.iter().all(|action| {
        action.operation == Operation::Promote
            && action.risk == RiskClass::OfflineRequired
            && !action.destructive
    }));
    assert!(plan.actions.iter().any(|action| {
        action.id == "datasets:tank/home-review:promote"
            && action.context.target.as_deref() == Some("tank/home-review")
    }));
    assert!(plan.actions.iter().any(|action| {
        action.id == "zvols:tank/vm/root-review:promote"
            && action.context.target.as_deref() == Some("tank/vm/root-review")
    }));
}

#[test]
fn plan_accepts_zfs_snapshot_holds_as_safe_property_actions() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "tank/home@before-upgrade": {
                  "target": "tank/home",
                  "hold": "disk-nix-retain"
                },
                "tank/home@old": {
                  "target": "tank/home",
                  "releaseHold": "old-retention"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 2);
    assert_eq!(plan.summary.destructive_count, 0);
    assert_eq!(plan.summary.potential_data_loss_count, 0);
    let hold = plan
        .actions
        .iter()
        .find(|action| action.id == "snapshot:tank/home@before-upgrade:hold:disk-nix-retain")
        .expect("snapshot hold action exists");
    assert_eq!(hold.operation, Operation::SetProperty);
    assert_eq!(hold.risk, RiskClass::Safe);
    assert_eq!(hold.context.property.as_deref(), Some("zfs.hold"));
    assert_eq!(
        hold.context.property_value.as_deref(),
        Some("disk-nix-retain")
    );
    let release = plan
        .actions
        .iter()
        .find(|action| action.id == "snapshot:tank/home@old:release-hold:old-retention")
        .expect("snapshot hold release action exists");
    assert_eq!(release.operation, Operation::SetProperty);
    assert_eq!(release.risk, RiskClass::Safe);
    assert_eq!(release.context.property.as_deref(), Some("zfs.releaseHold"));
    assert_eq!(
        release.context.property_value.as_deref(),
        Some("old-retention")
    );
}

#[test]
fn plan_preserves_btrfs_read_only_snapshot_context() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "/mnt/persist/@home-before": {
                  "target": "/mnt/persist/@home",
                  "readOnly": true
                }
              }
            }"#,
    )
    .expect("plan should parse");

    let action = plan
        .actions
        .iter()
        .find(|action| action.id == "snapshot:/mnt/persist/@home-before:create")
        .expect("snapshot action exists");

    assert_eq!(action.operation, Operation::Snapshot);
    assert_eq!(action.context.target.as_deref(), Some("/mnt/persist/@home"));
    assert_eq!(action.context.read_only, Some(true));
}

#[test]
fn plan_classifies_snapshot_rescan_as_online_refresh() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "tank/home@before-upgrade": {
                  "operation": "rescan",
                  "target": "tank/home"
                },
                "/mnt/persist/@home-before-upgrade": {
                  "operation": "rescan",
                  "target": "/mnt/persist/@home",
                  "readOnly": true
                },
                "home-before-friendly": {
                  "operation": "rescan",
                  "target": "/mnt/persist/@home",
                  "snapshotPath": "/mnt/persist/@home-before-friendly"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 3);
    assert_eq!(plan.summary.destructive_count, 0);
    assert_eq!(plan.summary.offline_required_count, 0);
    let zfs_rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "snapshot:tank/home@before-upgrade:rescan")
        .expect("ZFS snapshot rescan action exists");
    assert_eq!(zfs_rescan.operation, Operation::Rescan);
    assert_eq!(zfs_rescan.risk, RiskClass::Online);
    assert!(!zfs_rescan.destructive);
    assert_eq!(zfs_rescan.context.target.as_deref(), Some("tank/home"));
    let btrfs_rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "snapshot:/mnt/persist/@home-before-upgrade:rescan")
        .expect("Btrfs snapshot rescan action exists");
    assert_eq!(btrfs_rescan.operation, Operation::Rescan);
    assert_eq!(btrfs_rescan.context.read_only, Some(true));
    assert!(btrfs_rescan
        .advice
        .as_ref()
        .is_some_and(|advice| { advice.summary.contains("without mutating data") }));
    let friendly_btrfs_rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "snapshot:home-before-friendly:rescan")
        .expect("friendly-key Btrfs snapshot rescan action exists");
    assert_eq!(
        friendly_btrfs_rescan.context.target.as_deref(),
        Some("/mnt/persist/@home")
    );
    assert_eq!(
        friendly_btrfs_rescan.context.snapshot_path.as_deref(),
        Some("/mnt/persist/@home-before-friendly")
    );
}

#[test]
fn plan_accepts_snapshot_name_aliases_for_logical_keys() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "before-hold": {
                  "name": "tank/home@before",
                  "target": "tank/home",
                  "hold": "keep"
                },
                "before-clone": {
                  "snapshotName": "tank/home@before",
                  "target": "tank/home",
                  "cloneTo": "tank/home-review"
                },
                "before-rescan": {
                  "snapshot-name": "tank/home@before",
                  "target": "tank/home",
                  "operation": "rescan"
                },
                "before-destroy": {
                  "name": "tank/home@old",
                  "target": "tank/home",
                  "destroy": true
                }
              }
            }"#,
    )
    .expect("plan should parse");

    let hold = plan
        .actions
        .iter()
        .find(|action| action.id == "snapshot:before-hold:hold:keep")
        .expect("logical-key hold action exists");
    assert_eq!(hold.context.name.as_deref(), Some("tank/home@before"));
    assert_eq!(hold.context.target.as_deref(), Some("tank/home"));

    let clone = plan
        .actions
        .iter()
        .find(|action| action.id == "snapshot:before-clone:clone:tank/home-review")
        .expect("logical-key clone action exists");
    assert_eq!(clone.context.name.as_deref(), Some("tank/home@before"));
    assert_eq!(clone.context.target.as_deref(), Some("tank/home-review"));

    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "snapshot:before-rescan:rescan")
        .expect("logical-key rescan action exists");
    assert_eq!(rescan.context.name.as_deref(), Some("tank/home@before"));
    assert_eq!(rescan.context.target.as_deref(), Some("tank/home"));

    let destroy = plan
        .actions
        .iter()
        .find(|action| action.id == "snapshot:before-destroy:destroy")
        .expect("logical-key destroy action exists");
    assert_eq!(destroy.context.name.as_deref(), Some("tank/home@old"));
    assert_eq!(destroy.context.target.as_deref(), Some("tank/home"));
    assert_eq!(destroy.risk, RiskClass::Destructive);
}

#[test]
fn plan_classifies_lun_growth_as_offline_required() {
    let plan = plan_from_json_bytes(
            br#"{
              "luns": {
                "iqn.2026-06.example:storage/root:0": {
                  "operation": "grow",
                  "desiredSize": "2TiB",
                  "device": "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0",
                  "devices": [
                    "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-0"
                  ]
                }
              }
            }"#,
        )
        .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 1);
    assert_eq!(plan.summary.offline_required_count, 1);
    assert_eq!(plan.actions[0].operation, Operation::Grow);
    assert_eq!(plan.actions[0].risk, RiskClass::OfflineRequired);
    assert_eq!(
        plan.actions[0].context.desired_size.as_deref(),
        Some("2TiB")
    );
    assert_eq!(
        plan.actions[0].context.device.as_deref(),
        Some("/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0")
    );
    assert_eq!(
        plan.actions[0].context.devices,
        vec![
            "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-0"
                .to_string()
        ]
    );
    assert!(plan.actions[0].advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("multipath"))
    }));
}

#[test]
fn plan_accepts_target_side_lun_provisioning_requests() {
    let plan = plan_from_json_bytes(
        br#"{
              "targetLuns": {
                "array-a/root": {
                  "operation": "create",
                  "desiredSize": "2TiB",
                  "source": "pool-a/volumes/root",
                  "provider": "netapp-ontap",
                  "backstoreType": "array",
                  "vendor": "netapp",
                  "arrayId": "ontap-cluster-a",
                  "storagePool": "aggr1",
                  "volumeId": "vol-root",
                  "snapshotId": "snap-before",
                  "cloneSource": "vol-root@snap-before",
                  "maskingGroup": "linux-hosts",
                  "lun": 7,
                  "portal": "192.0.2.10:3260",
                  "client": "iqn.2026-06.example:host.primary",
                  "initiators": [
                    "iqn.2026-06.example:host.secondary"
                  ],
                  "properties": {
                    "thinProvisioned": true
                  }
                },
                "array-a/old": {
                  "operation": "detach",
                  "preserveData": true
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 3);
    assert_eq!(plan.summary.offline_required_count, 1);
    assert_eq!(plan.summary.potential_data_loss_count, 1);

    let create = plan
        .actions
        .iter()
        .find(|action| action.id == "targetluns:array-a/root:create")
        .expect("target-side LUN create action exists");
    assert_eq!(create.operation, Operation::Create);
    assert_eq!(create.risk, RiskClass::OfflineRequired);
    assert_eq!(create.context.collection.as_deref(), Some("targetLuns"));
    assert_eq!(create.context.desired_size.as_deref(), Some("2TiB"));
    assert_eq!(
        create.context.device.as_deref(),
        Some("pool-a/volumes/root")
    );
    assert_eq!(create.context.provider.as_deref(), Some("netapp-ontap"));
    assert_eq!(create.context.backstore_type.as_deref(), Some("array"));
    assert_eq!(create.context.vendor.as_deref(), Some("netapp"));
    assert_eq!(create.context.array_id.as_deref(), Some("ontap-cluster-a"));
    assert_eq!(create.context.storage_pool.as_deref(), Some("aggr1"));
    assert_eq!(create.context.volume_id.as_deref(), Some("vol-root"));
    assert_eq!(create.context.snapshot_id.as_deref(), Some("snap-before"));
    assert_eq!(
        create.context.clone_source.as_deref(),
        Some("vol-root@snap-before")
    );
    assert_eq!(create.context.masking_group.as_deref(), Some("linux-hosts"));
    assert_eq!(create.context.lun.as_deref(), Some("7"));
    assert_eq!(create.context.portal.as_deref(), Some("192.0.2.10:3260"));
    assert_eq!(
        create.context.client.as_deref(),
        Some("iqn.2026-06.example:host.primary")
    );
    assert_eq!(
        create.context.devices,
        vec!["iqn.2026-06.example:host.secondary".to_string()]
    );
    assert!(create
        .advice
        .as_ref()
        .is_some_and(|advice| { advice.summary.contains("target-side LUN provisioning") }));

    let property = plan
        .actions
        .iter()
        .find(|action| action.id == "targetLuns:array-a/root:set-property:thinProvisioned")
        .expect("target-side LUN property action exists");
    assert_eq!(property.operation, Operation::SetProperty);
    assert_eq!(property.risk, RiskClass::Safe);
    assert_eq!(
        property.context.property.as_deref(),
        Some("thinProvisioned")
    );

    let detach = plan
        .actions
        .iter()
        .find(|action| action.id == "targetluns:array-a/old:detach")
        .expect("target-side LUN detach action exists");
    assert_eq!(detach.operation, Operation::Detach);
    assert_eq!(detach.risk, RiskClass::PotentialDataLoss);
    assert!(!detach.destructive);
}

#[test]
fn plan_classifies_lun_attach_and_detach() {
    let plan = plan_from_json_bytes(
            br#"{
              "luns": {
                "iqn.2026-06.example:storage/root:0": {
                  "operation": "attach",
                  "device": "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0"
                },
                "iqn.2026-06.example:storage/old:1": {
                  "operation": "detach",
                  "devices": [
                    "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-1"
                  ]
                }
              }
            }"#,
        )
        .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 2);
    assert_eq!(plan.summary.offline_required_count, 1);
    assert_eq!(plan.summary.destructive_count, 0);

    let attach = plan
        .actions
        .iter()
        .find(|action| action.id == "luns:iqn.2026-06.example:storage/root:0:attach")
        .expect("LUN attach action exists");
    assert_eq!(attach.operation, Operation::Attach);
    assert_eq!(attach.risk, RiskClass::Online);
    assert!(attach.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("stable by-path"))
    }));

    let detach = plan
        .actions
        .iter()
        .find(|action| action.id == "luns:iqn.2026-06.example:storage/old:1:detach")
        .expect("LUN detach action exists");
    assert_eq!(detach.operation, Operation::Detach);
    assert_eq!(detach.risk, RiskClass::OfflineRequired);
    assert!(!detach.destructive);
    assert!(detach.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("deactivate"))
    }));
}

#[test]
fn plan_classifies_nvme_namespace_lifecycle() {
    let plan = plan_from_json_bytes(
        br#"{
              "nvmeNamespaces": {
                "/dev/nvme0": {
                  "operation": "create",
                  "desiredSize": "100G",
                  "namespaceId": "4",
                  "controllers": "0x1"
                },
                "/dev/nvme1": {
                  "operation": "grow"
                },
                "/dev/nvme2": {
                  "operation": "attach",
                  "namespaceId": "7",
                  "controllers": "0x2"
                },
                "/dev/nvme3": {
                  "operation": "detach",
                  "namespaceId": "8",
                  "controllers": "0x3"
                },
                "/dev/nvme4": {
                  "destroy": true,
                  "namespaceId": "9",
                  "controllers": "0x4"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 5);
    assert_eq!(plan.summary.destructive_count, 2);
    assert_eq!(plan.summary.offline_required_count, 2);

    let create = plan
        .actions
        .iter()
        .find(|action| action.id == "nvmenamespaces:/dev/nvme0:create")
        .expect("NVMe namespace create action exists");
    assert_eq!(create.operation, Operation::Create);
    assert_eq!(create.risk, RiskClass::Destructive);
    assert_eq!(create.context.namespace_id.as_deref(), Some("4"));
    assert_eq!(create.context.controllers.as_deref(), Some("0x1"));

    let grow = plan
        .actions
        .iter()
        .find(|action| action.id == "nvmenamespaces:/dev/nvme1:grow")
        .expect("NVMe namespace grow action exists");
    assert_eq!(grow.operation, Operation::Grow);
    assert_eq!(grow.risk, RiskClass::OfflineRequired);

    let attach = plan
        .actions
        .iter()
        .find(|action| action.id == "nvmenamespaces:/dev/nvme2:attach")
        .expect("NVMe namespace attach action exists");
    assert_eq!(attach.operation, Operation::Attach);
    assert_eq!(attach.risk, RiskClass::Online);
    assert_eq!(attach.context.namespace_id.as_deref(), Some("7"));
    assert_eq!(attach.context.controllers.as_deref(), Some("0x2"));

    let detach = plan
        .actions
        .iter()
        .find(|action| action.id == "nvmenamespaces:/dev/nvme3:detach")
        .expect("NVMe namespace detach action exists");
    assert_eq!(detach.operation, Operation::Detach);
    assert_eq!(detach.risk, RiskClass::OfflineRequired);
    assert!(!detach.destructive);
    assert_eq!(detach.context.namespace_id.as_deref(), Some("8"));
    assert_eq!(detach.context.controllers.as_deref(), Some("0x3"));

    let destroy = plan
        .actions
        .iter()
        .find(|action| action.id == "nvmenamespaces:/dev/nvme4:destroy")
        .expect("NVMe namespace destroy action exists");
    assert_eq!(destroy.operation, Operation::Destroy);
    assert_eq!(destroy.risk, RiskClass::Destructive);
}

#[test]
fn plan_classifies_iscsi_session_growth_as_offline_required() {
    let plan = plan_from_json_bytes(
        br#"{
              "iscsiSessions": {
                "iqn.2026-06.example:storage.root": {
                  "operation": "grow"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 1);
    assert_eq!(plan.summary.offline_required_count, 1);
    assert_eq!(plan.actions[0].operation, Operation::Grow);
    assert_eq!(plan.actions[0].risk, RiskClass::OfflineRequired);
}

#[test]
fn plan_classifies_host_storage_rescans_as_online() {
    let plan = plan_from_json_bytes(
        br#"{
              "iscsiSessions": {
                "iqn.2026-06.example:storage.root": {
                  "operation": "rescan"
                }
              },
              "luns": {
                "iqn.2026-06.example:storage/root:0": {
                  "operation": "rescan",
                  "devices": [
                    "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0"
                  ]
                }
              },
              "nvmeNamespaces": {
                "/dev/nvme0": {
                  "operation": "rescan"
                }
              },
              "physicalVolumes": {
                "/dev/disk/by-id/nvme-pv-refresh": {
                  "operation": "rescan"
                }
              },
              "volumeGroups": {
                "vgrefresh": {
                  "operation": "rescan"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 5);
    assert_eq!(plan.summary.offline_required_count, 0);
    assert!(plan.actions.iter().all(|action| {
        action.operation == Operation::Rescan && action.risk == RiskClass::Online
    }));
}
