#[test]
fn destructive_zfs_dataset_destroy_has_advice() {
    let capabilities = default_capabilities();
    let capability = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::ZfsDataset
                && capability.operation == Operation::Destroy
        })
        .expect("zfs dataset destroy capability should exist");

    assert_eq!(capability.risk, RiskClass::Destructive);
    assert!(capability.advice.is_some());
}

#[test]
fn cache_device_capabilities_describe_safe_lifecycle_paths() {
    let capabilities = default_capabilities();
    let add = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::CacheDevice
                && capability.operation == Operation::AddDevice
        })
        .expect("cache add capability should exist");
    let replace = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::CacheDevice
                && capability.operation == Operation::ReplaceDevice
        })
        .expect("cache replace capability should exist");
    let remove = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::CacheDevice
                && capability.operation == Operation::RemoveDevice
        })
        .expect("cache remove capability should exist");
    let rescan = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::CacheDevice
                && capability.operation == Operation::Rescan
        })
        .expect("cache rescan capability should exist");

    assert_eq!(add.risk, RiskClass::Online);
    assert_eq!(replace.risk, RiskClass::OfflineRequired);
    assert_eq!(remove.risk, RiskClass::PotentialDataLoss);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert!(replace.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("flush dirty data"))
    }));
}

#[test]
fn lvm_cache_capabilities_describe_lifecycle_paths() {
    let capabilities = default_capabilities();
    let create = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::LvmCache && capability.operation == Operation::Create
        })
        .expect("LVM cache create capability should exist");
    let add = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::LvmCache
                && capability.operation == Operation::AddDevice
        })
        .expect("LVM cache add-device capability should exist");
    let set_property = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::LvmCache
                && capability.operation == Operation::SetProperty
        })
        .expect("LVM cache property capability should exist");
    let rescan = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::LvmCache && capability.operation == Operation::Rescan
        })
        .expect("LVM cache rescan capability should exist");
    let remove = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::LvmCache
                && capability.operation == Operation::RemoveDevice
        })
        .expect("LVM cache remove-device capability should exist");

    assert_eq!(create.risk, RiskClass::OfflineRequired);
    assert_eq!(add.risk, RiskClass::OfflineRequired);
    assert_eq!(set_property.risk, RiskClass::Safe);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert_eq!(remove.risk, RiskClass::OfflineRequired);
}

#[test]
fn nfs_capabilities_describe_export_and_mount_lifecycle() {
    let capabilities = default_capabilities();
    let export_create = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::NfsExport && capability.operation == Operation::Create
        })
        .expect("NFS export create capability should exist");
    let export = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::NfsExport && capability.operation == Operation::Export
        })
        .expect("NFS export capability should exist");
    let export_destroy = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::NfsExport
                && capability.operation == Operation::Destroy
        })
        .expect("NFS export destroy capability should exist");
    let unexport = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::NfsExport
                && capability.operation == Operation::Unexport
        })
        .expect("NFS unexport capability should exist");
    let export_rescan = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::NfsExport && capability.operation == Operation::Rescan
        })
        .expect("NFS export rescan capability should exist");
    let mount_create = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::NfsMount && capability.operation == Operation::Create
        })
        .expect("NFS mount create capability should exist");
    let mount = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::NfsMount && capability.operation == Operation::Mount
        })
        .expect("NFS mount capability should exist");
    let mount_destroy = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::NfsMount && capability.operation == Operation::Destroy
        })
        .expect("NFS mount destroy capability should exist");
    let unmount = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::NfsMount && capability.operation == Operation::Unmount
        })
        .expect("NFS unmount capability should exist");
    let mount_rescan = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::NfsMount && capability.operation == Operation::Rescan
        })
        .expect("NFS mount rescan capability should exist");

    assert_eq!(export_create.risk, RiskClass::Online);
    assert_eq!(export.risk, RiskClass::Online);
    assert_eq!(export_destroy.risk, RiskClass::OfflineRequired);
    assert_eq!(unexport.risk, RiskClass::OfflineRequired);
    assert_eq!(export_rescan.risk, RiskClass::Online);
    assert_eq!(mount_create.risk, RiskClass::Online);
    assert_eq!(mount_rescan.risk, RiskClass::Online);
    assert_eq!(mount.risk, RiskClass::Online);
    assert_eq!(mount_destroy.risk, RiskClass::OfflineRequired);
    assert_eq!(unmount.risk, RiskClass::OfflineRequired);
    assert!(mount_destroy.advice.is_some());
    assert!(unmount.advice.is_some());
}

#[test]
fn btrfs_qgroup_capabilities_describe_limit_lifecycle() {
    let capabilities = default_capabilities();
    let create = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::BtrfsQgroup
                && capability.operation == Operation::Create
        })
        .expect("Btrfs qgroup create capability should exist");
    let update_limit = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::BtrfsQgroup
                && capability.operation == Operation::SetProperty
        })
        .expect("Btrfs qgroup property capability should exist");
    let rescan = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::BtrfsQgroup
                && capability.operation == Operation::Rescan
        })
        .expect("Btrfs qgroup rescan capability should exist");
    let destroy = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::BtrfsQgroup
                && capability.operation == Operation::Destroy
        })
        .expect("Btrfs qgroup destroy capability should exist");

    assert_eq!(create.risk, RiskClass::Online);
    assert_eq!(update_limit.risk, RiskClass::Safe);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert_eq!(destroy.risk, RiskClass::Destructive);
    assert!(destroy.advice.is_some());
}

#[test]
fn lvm_physical_volume_capabilities_describe_lifecycle() {
    let capabilities = default_capabilities();
    let create = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::LvmPhysicalVolume
                && capability.operation == Operation::Create
        })
        .expect("LVM physical volume create capability should exist");
    let grow = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::LvmPhysicalVolume
                && capability.operation == Operation::Grow
        })
        .expect("LVM physical volume grow capability should exist");
    let rescan = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::LvmPhysicalVolume
                && capability.operation == Operation::Rescan
        })
        .expect("LVM physical volume rescan capability should exist");
    let destroy = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::LvmPhysicalVolume
                && capability.operation == Operation::Destroy
        })
        .expect("LVM physical volume destroy capability should exist");

    assert_eq!(create.risk, RiskClass::Destructive);
    assert_eq!(grow.risk, RiskClass::Online);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert_eq!(destroy.risk, RiskClass::Destructive);
    assert!(destroy.advice.is_some());
}

#[test]
fn luks_keyslot_capabilities_describe_header_lifecycle() {
    let capabilities = default_capabilities();
    let create = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::LuksContainer
                && capability.operation == Operation::Create
        })
        .expect("LUKS keyslot create capability should exist");
    let add_key = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::LuksContainer
                && capability.operation == Operation::AddKey
        })
        .expect("LUKS add-key capability should exist");
    let import_token = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::LuksContainer
                && capability.operation == Operation::ImportToken
        })
        .expect("LUKS import-token capability should exist");
    let change = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::LuksContainer
                && capability.operation == Operation::SetProperty
        })
        .expect("LUKS keyslot change capability should exist");
    let destroy = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::LuksContainer
                && capability.operation == Operation::Destroy
        })
        .expect("LUKS keyslot destroy capability should exist");
    let remove_key = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::LuksContainer
                && capability.operation == Operation::RemoveKey
        })
        .expect("LUKS remove-key capability should exist");
    let remove_token = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::LuksContainer
                && capability.operation == Operation::RemoveToken
        })
        .expect("LUKS remove-token capability should exist");

    assert_eq!(create.risk, RiskClass::OfflineRequired);
    assert_eq!(add_key.risk, RiskClass::OfflineRequired);
    assert_eq!(import_token.risk, RiskClass::OfflineRequired);
    assert_eq!(change.risk, RiskClass::OfflineRequired);
    assert_eq!(destroy.risk, RiskClass::PotentialDataLoss);
    assert_eq!(remove_key.risk, RiskClass::PotentialDataLoss);
    assert_eq!(remove_token.risk, RiskClass::PotentialDataLoss);
}

#[test]
fn iscsi_and_lun_capabilities_describe_host_lifecycle() {
    let capabilities = default_capabilities();
    let lun_create = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::Lun && capability.operation == Operation::Create
        })
        .expect("LUN create capability should exist");
    let lun_attach = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::Lun && capability.operation == Operation::Attach
        })
        .expect("LUN attach capability should exist");
    let lun_rescan = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::Lun && capability.operation == Operation::Rescan
        })
        .expect("LUN rescan capability should exist");
    let lun_destroy = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::Lun && capability.operation == Operation::Destroy
        })
        .expect("LUN destroy capability should exist");
    let lun_detach = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::Lun && capability.operation == Operation::Detach
        })
        .expect("LUN detach capability should exist");
    let session_create = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::IscsiSession
                && capability.operation == Operation::Create
        })
        .expect("iSCSI session create capability should exist");
    let session_login = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::IscsiSession
                && capability.operation == Operation::Login
        })
        .expect("iSCSI session login capability should exist");
    let session_rescan = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::IscsiSession
                && capability.operation == Operation::Rescan
        })
        .expect("iSCSI session rescan capability should exist");
    let session_destroy = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::IscsiSession
                && capability.operation == Operation::Destroy
        })
        .expect("iSCSI session destroy capability should exist");
    let session_logout = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::IscsiSession
                && capability.operation == Operation::Logout
        })
        .expect("iSCSI session logout capability should exist");

    assert_eq!(lun_create.risk, RiskClass::Online);
    assert_eq!(lun_attach.risk, RiskClass::Online);
    assert_eq!(lun_rescan.risk, RiskClass::Online);
    assert_eq!(lun_destroy.risk, RiskClass::OfflineRequired);
    assert_eq!(lun_detach.risk, RiskClass::OfflineRequired);
    assert_eq!(session_create.risk, RiskClass::Online);
    assert_eq!(session_login.risk, RiskClass::Online);
    assert_eq!(session_rescan.risk, RiskClass::Online);
    assert_eq!(session_destroy.risk, RiskClass::OfflineRequired);
    assert_eq!(session_logout.risk, RiskClass::OfflineRequired);
    assert!(lun_destroy
        .advice
        .as_ref()
        .is_some_and(|advice| { advice.summary.contains("without deleting target-side data") }));
}

#[test]
fn nvme_namespace_capabilities_describe_controller_lifecycle() {
    let capabilities = default_capabilities();
    let create = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::NvmeNamespace
                && capability.operation == Operation::Create
        })
        .expect("NVMe namespace create capability should exist");
    let grow = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::NvmeNamespace
                && capability.operation == Operation::Grow
        })
        .expect("NVMe namespace grow capability should exist");
    let rescan = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::NvmeNamespace
                && capability.operation == Operation::Rescan
        })
        .expect("NVMe namespace rescan capability should exist");
    let destroy = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::NvmeNamespace
                && capability.operation == Operation::Destroy
        })
        .expect("NVMe namespace destroy capability should exist");

    assert_eq!(create.risk, RiskClass::Destructive);
    assert_eq!(grow.risk, RiskClass::OfflineRequired);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert_eq!(destroy.risk, RiskClass::Destructive);
    assert!(create.advice.is_some());
}

#[test]
fn zfs_clone_promotion_capabilities_are_advertised() {
    let capabilities = default_capabilities();
    for node_kind in [NodeKind::ZfsDataset, NodeKind::Zvol] {
        let capability = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == node_kind && capability.operation == Operation::Promote
            })
            .unwrap_or_else(|| panic!("{node_kind} promote capability should exist"));

        assert_eq!(capability.risk, RiskClass::OfflineRequired);
        assert!(capability
            .advice
            .as_ref()
            .is_some_and(|advice| { advice.summary.contains("promotion") }));
    }
}

#[test]
fn snapshot_capabilities_cover_zfs_and_btrfs_lifecycle() {
    let capabilities = default_capabilities();
    let zfs_snapshot = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::ZfsSnapshot
                && capability.operation == Operation::Snapshot
        })
        .expect("ZFS snapshot create capability should exist");
    let zfs_hold = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::ZfsSnapshot
                && capability.operation == Operation::SetProperty
        })
        .expect("ZFS snapshot hold capability should exist");
    let zfs_rollback = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::ZfsSnapshot
                && capability.operation == Operation::Rollback
        })
        .expect("ZFS snapshot rollback capability should exist");
    let zfs_clone = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::ZfsSnapshot
                && capability.operation == Operation::Clone
        })
        .expect("ZFS snapshot clone capability should exist");
    let zfs_rescan = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::ZfsSnapshot
                && capability.operation == Operation::Rescan
        })
        .expect("ZFS snapshot rescan capability should exist");
    let btrfs_snapshot = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::BtrfsSnapshot
                && capability.operation == Operation::Snapshot
        })
        .expect("Btrfs snapshot create capability should exist");
    let btrfs_rescan = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::BtrfsSnapshot
                && capability.operation == Operation::Rescan
        })
        .expect("Btrfs snapshot rescan capability should exist");
    let btrfs_clone = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::BtrfsSnapshot
                && capability.operation == Operation::Clone
        })
        .expect("Btrfs snapshot clone capability should exist");
    let btrfs_rename = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::BtrfsSnapshot
                && capability.operation == Operation::Rename
        })
        .expect("Btrfs snapshot rename capability should exist");
    let btrfs_destroy = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::BtrfsSnapshot
                && capability.operation == Operation::Destroy
        })
        .expect("Btrfs snapshot destroy capability should exist");

    assert_eq!(zfs_snapshot.risk, RiskClass::Reversible);
    assert_eq!(zfs_hold.risk, RiskClass::Safe);
    assert_eq!(zfs_clone.risk, RiskClass::Reversible);
    assert_eq!(zfs_rescan.risk, RiskClass::Online);
    assert_eq!(zfs_rollback.risk, RiskClass::PotentialDataLoss);
    assert!(zfs_rollback.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("recursive rollback"))
    }));
    assert_eq!(btrfs_snapshot.risk, RiskClass::Reversible);
    assert_eq!(btrfs_rescan.risk, RiskClass::Online);
    assert_eq!(btrfs_clone.risk, RiskClass::Reversible);
    assert_eq!(btrfs_rename.risk, RiskClass::OfflineRequired);
    assert_eq!(btrfs_destroy.risk, RiskClass::Destructive);
}

#[test]
fn property_capabilities_cover_supported_update_domains() {
    let capabilities = default_capabilities();
    for node_kind in [
        NodeKind::Filesystem,
        NodeKind::BtrfsSubvolume,
        NodeKind::ZfsPool,
        NodeKind::ZfsDataset,
        NodeKind::Zvol,
    ] {
        let capability = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == node_kind && capability.operation == Operation::SetProperty
            })
            .unwrap_or_else(|| panic!("{node_kind} set-property capability should exist"));

        assert_eq!(capability.risk, RiskClass::Safe);
        assert!(capability.advice.is_some());
    }
}

#[test]
fn btrfs_filesystem_capabilities_cover_device_topology_updates() {
    let capabilities = default_capabilities();
    for (operation, risk) in [
        (Operation::Rescan, RiskClass::Online),
        (Operation::AddDevice, RiskClass::Online),
        (Operation::ReplaceDevice, RiskClass::OfflineRequired),
        (Operation::RemoveDevice, RiskClass::PotentialDataLoss),
    ] {
        let capability = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::Filesystem && capability.operation == operation
            })
            .unwrap_or_else(|| panic!("generic filesystem {operation:?} capability should exist"));
        assert_eq!(capability.risk, risk);
        assert!(capability.advice.is_some());
    }

    let add = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::BtrfsFilesystem
                && capability.operation == Operation::AddDevice
        })
        .expect("Btrfs filesystem add-device capability should exist");
    let replace = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::BtrfsFilesystem
                && capability.operation == Operation::ReplaceDevice
        })
        .expect("Btrfs filesystem replace-device capability should exist");
    let remove = capabilities
        .iter()
        .find(|capability| {
            capability.node_kind == NodeKind::BtrfsFilesystem
                && capability.operation == Operation::RemoveDevice
        })
        .expect("Btrfs filesystem remove-device capability should exist");

    assert_eq!(add.risk, RiskClass::Online);
    assert_eq!(replace.risk, RiskClass::OfflineRequired);
    assert_eq!(remove.risk, RiskClass::PotentialDataLoss);
    assert!(replace.advice.is_some());

    for (operation, risk) in [
        (Operation::Grow, RiskClass::Online),
        (Operation::Rescan, RiskClass::Online),
        (Operation::AddDevice, RiskClass::Online),
        (Operation::ReplaceDevice, RiskClass::OfflineRequired),
        (Operation::RemoveDevice, RiskClass::PotentialDataLoss),
        (Operation::Rebalance, RiskClass::Online),
        (Operation::Scrub, RiskClass::Online),
    ] {
        let capability = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::BcachefsFilesystem
                    && capability.operation == operation
            })
            .unwrap_or_else(|| panic!("bcachefs filesystem {operation:?} capability should exist"));
        assert_eq!(capability.risk, risk);
        assert!(capability.advice.is_some());
    }
}

#[test]
fn capability_inventory_covers_rendered_topology_updates() {
    let capabilities = default_capabilities();
    for (node_kind, operation, risk) in [
        (
            NodeKind::ZfsPool,
            Operation::ReplaceDevice,
            RiskClass::OfflineRequired,
        ),
        (
            NodeKind::ZfsPool,
            Operation::RemoveDevice,
            RiskClass::PotentialDataLoss,
        ),
        (
            NodeKind::LvmLogicalVolume,
            Operation::Grow,
            RiskClass::Online,
        ),
        (
            NodeKind::LvmLogicalVolume,
            Operation::Rescan,
            RiskClass::Online,
        ),
        (NodeKind::Swap, Operation::Rescan, RiskClass::Online),
        (NodeKind::ZramDevice, Operation::Rescan, RiskClass::Online),
        (
            NodeKind::BtrfsSubvolume,
            Operation::Rescan,
            RiskClass::Online,
        ),
        (NodeKind::ZfsDataset, Operation::Rescan, RiskClass::Online),
        (NodeKind::Zvol, Operation::Rescan, RiskClass::Online),
        (NodeKind::BtrfsQgroup, Operation::Rescan, RiskClass::Online),
        (NodeKind::LvmVolumeGroup, Operation::Grow, RiskClass::Online),
        (
            NodeKind::LvmVolumeGroup,
            Operation::Rescan,
            RiskClass::Online,
        ),
        (
            NodeKind::LvmVolumeGroup,
            Operation::RemoveDevice,
            RiskClass::PotentialDataLoss,
        ),
        (NodeKind::LvmThinPool, Operation::Rescan, RiskClass::Online),
        (NodeKind::LvmSnapshot, Operation::Rescan, RiskClass::Online),
        (NodeKind::LoopDevice, Operation::Rescan, RiskClass::Online),
        (NodeKind::BackingFile, Operation::Create, RiskClass::Online),
        (NodeKind::BackingFile, Operation::Rescan, RiskClass::Online),
        (NodeKind::BackingFile, Operation::Grow, RiskClass::Online),
        (NodeKind::DeviceMapper, Operation::Rescan, RiskClass::Online),
        (
            NodeKind::DeviceMapper,
            Operation::Rename,
            RiskClass::OfflineRequired,
        ),
        (
            NodeKind::DeviceMapper,
            Operation::Destroy,
            RiskClass::Destructive,
        ),
        (NodeKind::CacheDevice, Operation::Rescan, RiskClass::Online),
        (NodeKind::VdoVolume, Operation::Rescan, RiskClass::Online),
        (NodeKind::ZfsSnapshot, Operation::Rescan, RiskClass::Online),
        (
            NodeKind::BtrfsSnapshot,
            Operation::Rescan,
            RiskClass::Online,
        ),
        (NodeKind::MdRaid, Operation::Create, RiskClass::Destructive),
        (
            NodeKind::MdRaid,
            Operation::Grow,
            RiskClass::OfflineRequired,
        ),
        (NodeKind::MdRaid, Operation::Rescan, RiskClass::Online),
        (NodeKind::MdRaid, Operation::Destroy, RiskClass::Destructive),
        (
            NodeKind::MultipathDevice,
            Operation::Grow,
            RiskClass::Online,
        ),
        (
            NodeKind::MultipathDevice,
            Operation::Rescan,
            RiskClass::Online,
        ),
        (
            NodeKind::MultipathDevice,
            Operation::Destroy,
            RiskClass::OfflineRequired,
        ),
        (
            NodeKind::MultipathDevice,
            Operation::RemoveDevice,
            RiskClass::PotentialDataLoss,
        ),
    ] {
        let capability = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == node_kind && capability.operation == operation
            })
            .unwrap_or_else(|| panic!("{node_kind} {operation:?} capability should exist"));

        assert_eq!(capability.risk, risk);
        assert!(capability.advice.is_some());
    }
}

#[test]
fn plan_accepts_supported_spec_versions() {
    let direct = plan_from_json_bytes(
        br#"{
              "version": 1,
              "filesystems": {
                "root": {
                  "mountpoint": "/",
                  "fsType": "ext4",
                  "resizePolicy": "grow-only"
                }
              }
            }"#,
    )
    .expect("direct spec should parse");

    let wrapped = plan_from_json_bytes(
        br#"{
              "version": 1,
              "spec": {
                "version": 1,
                "filesystems": {
                  "root": {
                    "mountpoint": "/",
                    "fsType": "ext4",
                    "resizePolicy": "grow-only"
                  }
                }
              }
            }"#,
    )
    .expect("wrapped spec should parse");

    assert_eq!(direct.summary.action_count, 1);
    assert_eq!(wrapped.summary.action_count, 1);
}

#[test]
fn plan_rejects_unsupported_spec_versions() {
    let error = plan_from_json_bytes(
        br#"{
              "version": 2,
              "filesystems": {}
            }"#,
    )
    .expect_err("future version should be rejected");

    assert_eq!(
        error.to_string(),
        "unsupported disk-nix spec version 2; supported version is 1"
    );
}

#[test]
fn plan_rejects_invalid_and_conflicting_spec_versions() {
    let invalid = plan_from_json_bytes(
        br#"{
              "spec": {
                "version": "1"
              }
            }"#,
    )
    .expect_err("string version should be rejected");

    let conflicting = plan_from_json_bytes(
        br#"{
              "version": 1,
              "spec": {
                "version": 2
              }
            }"#,
    )
    .expect_err("conflicting versions should be rejected");

    assert_eq!(
        invalid.to_string(),
        "disk-nix spec version at spec.version must be an integer"
    );
    assert_eq!(
        conflicting.to_string(),
        "conflicting disk-nix spec versions: top-level version 1, spec.version 2"
    );
}

#[test]
fn plan_orders_stacked_storage_actions_by_dependency_layer() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "root": {
                  "mountpoint": "/",
                  "fsType": "ext4",
                  "resizePolicy": "grow-only"
                }
              },
              "volumes": {
                "root": {
                  "operation": "create",
                  "device": "/dev/vg/root"
                }
              },
              "volumeGroups": {
                "vg": {
                  "operation": "create"
                }
              },
              "physicalVolumes": {
                "pv0": {
                  "operation": "create",
                  "device": "/dev/mapper/cryptroot"
                }
              },
              "luks": {
                "devices": {
                  "cryptroot": {
                    "operation": "open",
                    "device": "/dev/disk/by-partlabel/root"
                  }
                }
              },
              "partitions": {
                "root": {
                  "operation": "create",
                  "device": "/dev/disk/by-partlabel/root"
                }
              },
              "disks": {
                "system": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/nvme-system"
                }
              },
              "snapshots": {
                "old-root": {
                  "target": "tank/root@old",
                  "destroy": true
                }
              }
            }"#,
    )
    .expect("plan should parse");

    let ids: Vec<&str> = plan
        .actions
        .iter()
        .map(|action| action.id.as_str())
        .collect();

    assert_eq!(
        ids,
        vec![
            "disks:system:create",
            "partitions:root:create",
            "luks.devices:cryptroot:open",
            "physicalvolumes:pv0:create",
            "volumegroups:vg:create",
            "volumes:root:create",
            "filesystem:root:grow",
            "snapshot:old-root:destroy",
        ]
    );
    let dependency_ids: Vec<&str> = plan
        .dependency_order
        .iter()
        .map(|order| order.action_id.as_str())
        .collect();
    assert_eq!(dependency_ids, ids);
    assert_eq!(
        plan.dependency_order.first().map(|order| (
            order.phase,
            order.direction,
            order.layer_rank,
            order.collection.as_deref()
        )),
        Some((
            DependencyPhase::BuildLowerLayers,
            DependencyDirection::LowerLayersFirst,
            20,
            Some("disks")
        ))
    );
    assert_eq!(
        plan.dependency_order.last().map(|order| (
            order.phase,
            order.direction,
            order.layer_rank,
            order.collection.as_deref()
        )),
        Some((
            DependencyPhase::TearDownUpperLayers,
            DependencyDirection::UpperLayersFirst,
            95,
            Some("snapshots")
        ))
    );
    assert!(plan.dependency_order.iter().all(|order| {
        !order.notes.is_empty()
            && order
                .notes
                .iter()
                .any(|note| note.contains("collection layer rank"))
    }));
}
