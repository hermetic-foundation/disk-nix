
use disk_nix_model::{Identity, Usage};
use serde::Deserialize;

use super::*;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MigrationExampleFixture {
    name: String,
    base_example: String,
    description: String,
    target_spec: serde_json::Value,
    current_graph: StorageGraph,
    expected_remaining_action_ids: Vec<String>,
    expected_suppressed_action_ids: Vec<String>,
}

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

#[test]
fn dependency_order_reports_explicit_edges_for_layered_block_growth() {
    let plan = plan_from_json_bytes(
        br#"{
              "backingFiles": {
                "/var/lib/images/root.img": {
                  "operation": "grow",
                  "desiredSize": "32GiB"
                }
              },
              "loopDevices": {
                "/dev/loop7": {
                  "operation": "grow",
                  "device": "/var/lib/images/root.img"
                }
              },
              "filesystems": {
                "root": {
                  "operation": "grow",
                  "device": "/dev/loop7",
                  "desiredSize": "100%"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    let backing = plan
        .dependency_order
        .iter()
        .find(|order| order.action_id == "backingfiles:/var/lib/images/root.img:grow")
        .expect("backing file dependency order entry exists");
    let loop_device = plan
        .dependency_order
        .iter()
        .find(|order| order.action_id == "loopdevices:/dev/loop7:grow")
        .expect("loop device dependency order entry exists");
    let filesystem = plan
        .dependency_order
        .iter()
        .find(|order| order.action_id == "filesystem:root:inspect")
        .expect("filesystem dependency order entry exists");

    assert!(backing.depends_on.is_empty());
    assert_eq!(
        backing.unblocks,
        vec!["loopdevices:/dev/loop7:grow".to_string()]
    );
    assert_eq!(
        backing.recovery_depends_on,
        vec!["loopdevices:/dev/loop7:grow".to_string()]
    );
    assert!(backing.recovery_unblocks.is_empty());
    assert_eq!(
        loop_device.depends_on,
        vec!["backingfiles:/var/lib/images/root.img:grow".to_string()]
    );
    assert_eq!(
        loop_device.unblocks,
        vec!["filesystem:root:inspect".to_string()]
    );
    assert_eq!(
        loop_device.recovery_depends_on,
        vec!["filesystem:root:inspect".to_string()]
    );
    assert_eq!(
        loop_device.recovery_unblocks,
        vec!["backingfiles:/var/lib/images/root.img:grow".to_string()]
    );
    assert_eq!(
        filesystem.depends_on,
        vec!["loopdevices:/dev/loop7:grow".to_string()]
    );
    assert!(filesystem.unblocks.is_empty());
    assert!(filesystem.recovery_depends_on.is_empty());
    assert_eq!(
        filesystem.recovery_unblocks,
        vec!["loopdevices:/dev/loop7:grow".to_string()]
    );
    assert!(loop_device
        .notes
        .iter()
        .any(|note| note.contains("explicit dependency edge")));
    assert!(loop_device.notes.iter().any(|note| {
        note.contains("recovery review waits for dependent action")
            && note.contains("filesystem:root:inspect")
    }));
}

#[test]
fn dependency_order_reports_explicit_edges_for_pool_dataset_snapshot_layers() {
    let plan = plan_from_json_bytes(
        br#"{
              "pools": {
                "tank": {
                  "operation": "import"
                }
              },
              "datasets": {
                "tank/home": {
                  "operation": "create"
                }
              },
              "snapshots": {
                "home-before": {
                  "target": "tank/home",
                  "name": "tank/home@before"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    let pool = plan
        .dependency_order
        .iter()
        .find(|order| order.action_id == "pools:tank:import")
        .expect("pool dependency order entry exists");
    let dataset = plan
        .dependency_order
        .iter()
        .find(|order| order.action_id == "datasets:tank/home:create")
        .expect("dataset dependency order entry exists");
    let snapshot = plan
        .dependency_order
        .iter()
        .find(|order| order.action_id == "snapshot:home-before:create")
        .expect("snapshot dependency order entry exists");

    assert_eq!(pool.unblocks, vec!["datasets:tank/home:create".to_string()]);
    assert_eq!(dataset.depends_on, vec!["pools:tank:import".to_string()]);
    assert_eq!(
        dataset.unblocks,
        vec!["snapshot:home-before:create".to_string()]
    );
    assert_eq!(
        snapshot.depends_on,
        vec!["datasets:tank/home:create".to_string()]
    );
}

#[test]
fn plan_warns_for_shrink_and_disabled_preservation() {
    let plan = plan_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "home": {
                    "mountpoint": "/home",
                    "fsType": "xfs",
                    "resizePolicy": "shrink-allowed",
                    "preserveData": false
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 2);
    assert_eq!(plan.summary.destructive_count, 1);
    assert_eq!(plan.summary.potential_data_loss_count, 0);
    assert_eq!(plan.summary.offline_required_count, 0);
    assert_eq!(plan.summary.unsupported_count, 1);
    assert!(plan.actions.iter().any(|action| {
        action.operation == Operation::Shrink
            && action.risk == RiskClass::Unsupported
            && action
                .advice
                .as_ref()
                .is_some_and(|advice| advice.summary.contains("XFS"))
    }));
}

#[test]
fn plan_keeps_ext_shrink_as_potential_data_loss() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "home": {
                  "mountpoint": "/home",
                  "fsType": "ext4",
                  "resizePolicy": "shrink-allowed"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 1);
    assert_eq!(plan.summary.potential_data_loss_count, 1);
    assert_eq!(plan.summary.unsupported_count, 0);
    assert_eq!(plan.actions[0].risk, RiskClass::PotentialDataLoss);
    assert_eq!(plan.actions[0].context.fs_type.as_deref(), Some("ext4"));
    assert_eq!(plan.actions[0].context.mountpoint.as_deref(), Some("/home"));
}

#[test]
fn plan_carries_filesystem_device_for_lifecycle_actions() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "home": {
                  "mountpoint": "/home",
                  "device": "/dev/disk/by-label/home",
                  "fsType": "ext4",
                  "resizePolicy": "shrink-allowed",
                  "desiredSize": "100G"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 1);
    assert_eq!(plan.actions[0].operation, Operation::Shrink);
    assert_eq!(
        plan.actions[0].context.device.as_deref(),
        Some("/dev/disk/by-label/home")
    );
    assert_eq!(plan.actions[0].context.target.as_deref(), Some("/home"));
    assert_eq!(
        plan.actions[0].context.desired_size.as_deref(),
        Some("100G")
    );
}

#[test]
fn plan_filesystem_properties_keep_filesystem_context() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "data": {
                  "mountpoint": "/data",
                  "device": "/dev/disk/by-label/data",
                  "fsType": "btrfs",
                  "properties": {
                    "label": "bulk-data",
                    "compression": "zstd"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 3);
    assert_eq!(plan.summary.unsupported_count, 1);
    let action = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:data:set-property:label")
        .expect("filesystem label property action should exist");

    assert_eq!(action.operation, Operation::SetProperty);
    assert_eq!(action.risk, RiskClass::Safe);
    assert_eq!(action.context.target.as_deref(), Some("/data"));
    assert_eq!(
        action.context.device.as_deref(),
        Some("/dev/disk/by-label/data")
    );
    assert_eq!(action.context.fs_type.as_deref(), Some("btrfs"));
    assert_eq!(action.context.mountpoint.as_deref(), Some("/data"));
    assert_eq!(action.context.property.as_deref(), Some("label"));
    assert_eq!(action.context.property_value.as_deref(), Some("bulk-data"));

    let unsupported = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:data:set-property:compression")
        .expect("unsupported filesystem property action should exist");
    assert_eq!(unsupported.operation, Operation::SetProperty);
    assert_eq!(unsupported.risk, RiskClass::Unsupported);
    assert!(unsupported.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("ZFS dataset"))
    }));
}

#[test]
fn plan_accepts_xfs_filesystem_label_property() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "scratch": {
                  "mountpoint": "/scratch",
                  "device": "/dev/disk/by-label/scratch",
                  "fsType": "xfs",
                  "properties": {
                    "xfs.label": "scratch-new",
                    "xfs.reflink": "1"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 3);
    assert_eq!(plan.summary.unsupported_count, 1);
    let label = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:scratch:set-property:xfs.label")
        .expect("XFS label property action should exist");

    assert_eq!(label.operation, Operation::SetProperty);
    assert_eq!(label.risk, RiskClass::Safe);
    assert_eq!(label.context.target.as_deref(), Some("/scratch"));
    assert_eq!(
        label.context.device.as_deref(),
        Some("/dev/disk/by-label/scratch")
    );
    assert_eq!(label.context.fs_type.as_deref(), Some("xfs"));
    assert_eq!(label.context.property_value.as_deref(), Some("scratch-new"));

    let unsupported = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:scratch:set-property:xfs.reflink")
        .expect("unsupported XFS property action should exist");
    assert_eq!(unsupported.risk, RiskClass::Unsupported);
}

#[test]
fn plan_accepts_fat_label_and_volume_id_properties() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "efi": {
                  "mountpoint": "/boot",
                  "device": "/dev/disk/by-partlabel/EFI",
                  "fsType": "vfat",
                  "properties": {
                    "vfat.label": "NIXBOOT",
                    "vfat.uuid": "A1B2-C3D4",
                    "fat.volume-id": "not-a-fat-id"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 4);
    assert_eq!(plan.summary.offline_required_count, 1);
    assert_eq!(plan.summary.unsupported_count, 1);

    let label = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:efi:set-property:vfat.label")
        .expect("FAT label property action should exist");
    assert_eq!(label.operation, Operation::SetProperty);
    assert_eq!(label.risk, RiskClass::Safe);
    assert_eq!(label.context.fs_type.as_deref(), Some("vfat"));
    assert_eq!(label.context.property_value.as_deref(), Some("NIXBOOT"));

    let volume_id = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:efi:set-property:vfat.uuid")
        .expect("FAT volume ID property action should exist");
    assert_eq!(volume_id.risk, RiskClass::OfflineRequired);
    assert!(volume_id.advice.as_ref().is_some_and(|advice| {
        advice.summary.contains("UUID")
            && advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("NixOS fileSystems"))
    }));

    let invalid_volume_id = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:efi:set-property:fat.volume-id")
        .expect("invalid FAT volume ID property action should exist");
    assert_eq!(invalid_volume_id.risk, RiskClass::Unsupported);
    assert!(invalid_volume_id.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("8-hex-digit FAT volume ID"))
    }));
}

#[test]
fn plan_accepts_ntfs_label_and_volume_serial_properties() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "windows": {
                  "mountpoint": "/mnt/windows",
                  "device": "/dev/disk/by-label/Windows",
                  "fsType": "ntfs",
                  "properties": {
                    "ntfs.label": "Windows",
                    "ntfs.uuid": "01234567-89abcdef",
                    "ntfs.volume-serial": "not-a-serial"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 4);
    assert_eq!(plan.summary.offline_required_count, 1);
    assert_eq!(plan.summary.unsupported_count, 1);

    let label = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:windows:set-property:ntfs.label")
        .expect("NTFS label property action should exist");
    assert_eq!(label.operation, Operation::SetProperty);
    assert_eq!(label.risk, RiskClass::Safe);
    assert_eq!(label.context.fs_type.as_deref(), Some("ntfs"));
    assert_eq!(label.context.property_value.as_deref(), Some("Windows"));

    let serial = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:windows:set-property:ntfs.uuid")
        .expect("NTFS serial property action should exist");
    assert_eq!(serial.risk, RiskClass::OfflineRequired);
    assert!(serial.advice.as_ref().is_some_and(|advice| {
        advice.summary.contains("UUID")
            && advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("NixOS fileSystems"))
    }));

    let invalid_serial = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:windows:set-property:ntfs.volume-serial")
        .expect("invalid NTFS serial property action should exist");
    assert_eq!(invalid_serial.risk, RiskClass::Unsupported);
    assert!(invalid_serial.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("16-hex-digit NTFS volume serial"))
    }));
}

#[test]
fn plan_accepts_exfat_label_and_volume_serial_properties() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "shared": {
                  "mountpoint": "/mnt/shared",
                  "device": "/dev/disk/by-label/Shared",
                  "fsType": "exfat",
                  "properties": {
                    "exfat.label": "Shared",
                    "exfat.uuid": "A1B2-C3D4",
                    "exfat.volume-serial": "not-a-serial"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 4);
    assert_eq!(plan.summary.offline_required_count, 1);
    assert_eq!(plan.summary.unsupported_count, 1);

    let label = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:shared:set-property:exfat.label")
        .expect("exFAT label property action should exist");
    assert_eq!(label.operation, Operation::SetProperty);
    assert_eq!(label.risk, RiskClass::Safe);
    assert_eq!(label.context.fs_type.as_deref(), Some("exfat"));
    assert_eq!(label.context.property_value.as_deref(), Some("Shared"));

    let serial = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:shared:set-property:exfat.uuid")
        .expect("exFAT serial property action should exist");
    assert_eq!(serial.risk, RiskClass::OfflineRequired);
    assert!(serial.advice.as_ref().is_some_and(|advice| {
        advice.summary.contains("UUID")
            && advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("NixOS fileSystems"))
    }));

    let invalid_serial = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:shared:set-property:exfat.volume-serial")
        .expect("invalid exFAT serial property action should exist");
    assert_eq!(invalid_serial.risk, RiskClass::Unsupported);
    assert!(invalid_serial.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("8-hex-digit exFAT volume serial"))
    }));
}

#[test]
fn plan_accepts_f2fs_label_property() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "mobile": {
                  "mountpoint": "/mnt/mobile",
                  "device": "/dev/disk/by-label/mobile",
                  "fsType": "f2fs",
                  "properties": {
                    "f2fs.label": "mobile-new",
                    "f2fs.uuid": "11111111-2222-3333-4444-555555555555"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 3);
    assert_eq!(plan.summary.unsupported_count, 1);

    let label = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:mobile:set-property:f2fs.label")
        .expect("F2FS label property action should exist");
    assert_eq!(label.operation, Operation::SetProperty);
    assert_eq!(label.risk, RiskClass::Safe);
    assert_eq!(label.context.fs_type.as_deref(), Some("f2fs"));
    assert_eq!(label.context.property_value.as_deref(), Some("mobile-new"));

    let unsupported = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:mobile:set-property:f2fs.uuid")
        .expect("unsupported F2FS property action should exist");
    assert_eq!(unsupported.risk, RiskClass::Unsupported);
    assert!(unsupported.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("f2fs.label"))
    }));
}

#[test]
fn plan_classifies_filesystem_uuid_updates_as_offline_required() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "home": {
                  "mountpoint": "/home",
                  "device": "/dev/disk/by-label/home",
                  "fsType": "ext4",
                  "properties": {
                    "ext.uuid": "11111111-2222-3333-4444-555555555555"
                  }
                },
                "scratch": {
                  "mountpoint": "/scratch",
                  "device": "/dev/disk/by-label/scratch",
                  "fsType": "xfs",
                  "properties": {
                    "filesystem.uuid": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
                  }
                },
                "data": {
                  "mountpoint": "/data",
                  "device": "/dev/disk/by-label/data",
                  "fsType": "btrfs",
                  "properties": {
                    "btrfs.uuid": "bbbbbbbb-1111-2222-3333-cccccccccccc"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 6);
    assert_eq!(plan.summary.offline_required_count, 3);
    assert_eq!(plan.summary.unsupported_count, 0);
    let ext_uuid = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:home:set-property:ext.uuid")
        .expect("Ext UUID property action should exist");
    assert_eq!(ext_uuid.operation, Operation::SetProperty);
    assert_eq!(ext_uuid.risk, RiskClass::OfflineRequired);
    assert_eq!(
        ext_uuid.context.property_value.as_deref(),
        Some("11111111-2222-3333-4444-555555555555")
    );
    assert!(ext_uuid.advice.as_ref().is_some_and(|advice| {
        advice.summary.contains("UUID")
            && advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("NixOS fileSystems"))
    }));

    let xfs_uuid = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:scratch:set-property:filesystem.uuid")
        .expect("XFS UUID property action should exist");
    assert_eq!(xfs_uuid.risk, RiskClass::OfflineRequired);
    assert_eq!(xfs_uuid.context.fs_type.as_deref(), Some("xfs"));

    let btrfs_uuid = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:data:set-property:btrfs.uuid")
        .expect("Btrfs UUID property action should exist");
    assert_eq!(btrfs_uuid.risk, RiskClass::OfflineRequired);
    assert_eq!(btrfs_uuid.context.fs_type.as_deref(), Some("btrfs"));
    assert_eq!(
        btrfs_uuid.context.property_value.as_deref(),
        Some("bbbbbbbb-1111-2222-3333-cccccccccccc")
    );
}

#[test]
fn plan_warns_for_filesystem_device_removal() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "data": {
                  "mountpoint": "/data",
                  "fsType": "btrfs",
                  "removeDevices": ["/dev/disk/by-id/old-btrfs-device"]
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 2);
    assert_eq!(plan.summary.potential_data_loss_count, 1);
    assert!(plan.actions.iter().any(|action| {
        action.id == "filesystems:data:remove-device:/dev/disk/by-id/old-btrfs-device"
            && action.operation == Operation::RemoveDevice
            && action.risk == RiskClass::PotentialDataLoss
            && action.context.collection.as_deref() == Some("filesystems")
            && action.context.target.as_deref() == Some("/data")
            && action.context.device.as_deref() == Some("/dev/disk/by-id/old-btrfs-device")
            && action.advice.is_some()
    }));
}

#[test]
fn plan_accepts_filesystem_rebalance_with_filters() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "data": {
                  "mountpoint": "/data",
                  "fsType": "btrfs",
                  "operation": "rebalance",
                  "properties": {
                    "balance.data": "usage=50",
                    "balance.metadata": "usage=75"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 2);
    assert_eq!(plan.summary.unsupported_count, 0);
    let action = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:data:rebalance")
        .expect("filesystem rebalance action exists");

    assert_eq!(action.operation, Operation::Rebalance);
    assert_eq!(action.risk, RiskClass::Online);
    assert_eq!(action.context.collection.as_deref(), Some("filesystems"));
    assert_eq!(action.context.target.as_deref(), Some("/data"));
    assert_eq!(
        action.context.property_assignments,
        vec![
            "balance.data=usage=50".to_string(),
            "balance.metadata=usage=75".to_string()
        ]
    );
}

#[test]
fn plan_accepts_filesystem_check_and_repair_operations() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "home": {
                  "mountpoint": "/home",
                  "device": "/dev/disk/by-label/home",
                  "fsType": "ext4",
                  "operation": "check"
                },
                "data": {
                  "mountpoint": "/data",
                  "device": "/dev/disk/by-label/data",
                  "fsType": "btrfs",
                  "operation": "repair"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 4);
    assert_eq!(plan.summary.offline_required_count, 2);
    assert_eq!(plan.summary.unsupported_count, 0);
    let check = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:home:check")
        .expect("filesystem check action exists");
    assert_eq!(check.operation, Operation::Check);
    assert_eq!(check.risk, RiskClass::OfflineRequired);
    assert!(!check.destructive);
    assert_eq!(check.context.fs_type.as_deref(), Some("ext4"));
    assert_eq!(
        check.context.device.as_deref(),
        Some("/dev/disk/by-label/home")
    );

    let repair = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:data:repair")
        .expect("filesystem repair action exists");
    assert_eq!(repair.operation, Operation::Repair);
    assert_eq!(repair.risk, RiskClass::OfflineRequired);
    assert!(!repair.destructive);
    assert_eq!(repair.context.fs_type.as_deref(), Some("btrfs"));
    assert!(repair
        .advice
        .as_ref()
        .is_some_and(|advice| { advice.summary.contains("repair mutates metadata") }));
}

#[test]
fn plan_accepts_scrub_lifecycle_for_btrfs_and_pools() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "data": {
                  "mountpoint": "/data",
                  "fsType": "btrfs",
                  "operation": "scrub"
                },
                "bulk": {
                  "mountpoint": "/bulk",
                  "fsType": "bcachefs",
                  "operation": "scrub"
                },
                "archive": {
                  "mountpoint": "/archive",
                  "fsType": "ext4",
                  "operation": "scrub"
                }
              },
              "pools": {
                "tank": {
                  "operation": "scrub"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 7);
    assert_eq!(plan.summary.unsupported_count, 1);
    let btrfs = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:data:scrub")
        .expect("Btrfs scrub action exists");
    assert_eq!(btrfs.operation, Operation::Scrub);
    assert_eq!(btrfs.risk, RiskClass::Online);
    assert_eq!(btrfs.context.target.as_deref(), Some("/data"));

    let bcachefs = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:bulk:scrub")
        .expect("bcachefs scrub action exists");
    assert_eq!(bcachefs.operation, Operation::Scrub);
    assert_eq!(bcachefs.risk, RiskClass::Online);
    assert_eq!(bcachefs.context.target.as_deref(), Some("/bulk"));
    assert!(bcachefs
        .advice
        .as_ref()
        .is_some_and(|advice| { advice.summary.contains("bcachefs scrub verifies") }));

    let pool = plan
        .actions
        .iter()
        .find(|action| action.id == "pools:tank:scrub")
        .expect("pool scrub action exists");
    assert_eq!(pool.operation, Operation::Scrub);
    assert_eq!(pool.risk, RiskClass::Online);

    let unsupported = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:archive:scrub")
        .expect("unsupported filesystem scrub action exists");
    assert_eq!(unsupported.risk, RiskClass::Unsupported);
    assert!(unsupported
        .advice
        .as_ref()
        .is_some_and(|advice| { advice.summary.contains("Btrfs and bcachefs") }));
}

#[test]
fn plan_accepts_filesystem_trim_operation() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "scratch": {
                  "mountpoint": "/scratch",
                  "fsType": "xfs",
                  "operation": "trim"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 2);
    assert_eq!(plan.summary.offline_required_count, 0);
    assert_eq!(plan.summary.unsupported_count, 0);
    let trim = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:scratch:trim")
        .expect("filesystem trim action exists");
    assert_eq!(trim.operation, Operation::Trim);
    assert_eq!(trim.risk, RiskClass::Online);
    assert_eq!(trim.context.target.as_deref(), Some("/scratch"));
    assert!(trim
        .advice
        .as_ref()
        .is_some_and(|advice| { advice.summary.contains("discards unused blocks") }));
}

#[test]
fn plan_accepts_filesystem_rescan_operation() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "scratch": {
                  "mountpoint": "/scratch",
                  "device": "/dev/disk/by-label/scratch",
                  "fsType": "xfs",
                  "operation": "rescan"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 2);
    assert_eq!(plan.summary.offline_required_count, 0);
    assert_eq!(plan.summary.unsupported_count, 0);
    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:scratch:rescan")
        .expect("filesystem rescan action exists");
    assert_eq!(rescan.operation, Operation::Rescan);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert!(!rescan.destructive);
    assert_eq!(rescan.context.target.as_deref(), Some("/scratch"));
    assert_eq!(
        rescan.context.device.as_deref(),
        Some("/dev/disk/by-label/scratch")
    );
    assert!(rescan
        .advice
        .as_ref()
        .is_some_and(|advice| advice.summary.contains("refreshes mount")));
}

#[test]
fn plan_accepts_filesystem_remount_operation() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "scratch": {
                  "mountpoint": "/scratch",
                  "fsType": "xfs",
                  "operation": "remount",
                  "options": ["rw", "noatime", "discard=async"]
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 2);
    assert_eq!(plan.summary.offline_required_count, 0);
    assert_eq!(plan.summary.unsupported_count, 0);
    let remount = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:scratch:remount")
        .expect("filesystem remount action exists");
    assert_eq!(remount.operation, Operation::Remount);
    assert_eq!(remount.risk, RiskClass::Online);
    assert_eq!(remount.context.target.as_deref(), Some("/scratch"));
    assert_eq!(
        remount.context.options.as_deref(),
        Some("rw,noatime,discard=async")
    );
    assert!(remount
        .advice
        .as_ref()
        .is_some_and(|advice| advice.summary.contains("updates local mount options")));
}

#[test]
fn plan_accepts_filesystem_mount_lifecycle() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "backup": {
                  "mountpoint": "/backup",
                  "device": "/dev/disk/by-label/backup",
                  "fsType": "xfs",
                  "operation": "mount",
                  "options": ["rw", "noatime"]
                },
                "archive": {
                  "mountpoint": "/archive",
                  "device": "/dev/disk/by-label/archive",
                  "fsType": "ext4",
                  "operation": "unmount"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 4);
    assert_eq!(plan.summary.offline_required_count, 1);
    assert_eq!(plan.summary.unsupported_count, 0);
    assert_eq!(plan.summary.destructive_count, 0);
    let mount = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:backup:mount")
        .expect("filesystem mount action exists");
    assert_eq!(mount.operation, Operation::Mount);
    assert_eq!(mount.risk, RiskClass::Online);
    assert_eq!(
        mount.context.device.as_deref(),
        Some("/dev/disk/by-label/backup")
    );
    assert_eq!(mount.context.mountpoint.as_deref(), Some("/backup"));
    assert_eq!(mount.context.fs_type.as_deref(), Some("xfs"));
    assert_eq!(mount.context.options.as_deref(), Some("rw,noatime"));

    let unmount = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystems:archive:unmount")
        .expect("filesystem unmount action exists");
    assert_eq!(unmount.operation, Operation::Unmount);
    assert_eq!(unmount.risk, RiskClass::OfflineRequired);
    assert!(!unmount.destructive);
    assert_eq!(unmount.context.mountpoint.as_deref(), Some("/archive"));
}

#[test]
fn plan_carries_desired_size_context_for_resize_actions() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "home": {
                  "mountpoint": "/home",
                  "fsType": "btrfs",
                  "resizePolicy": "grow-only",
                  "desiredSize": "750GiB"
                }
              },
              "volumes": {
                "vg/home": {
                  "operation": "grow",
                  "size": "800GiB"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    let filesystem = plan
        .actions
        .iter()
        .find(|action| action.id == "filesystem:home:grow")
        .expect("filesystem grow action exists");
    assert_eq!(filesystem.context.desired_size.as_deref(), Some("750GiB"));

    let volume = plan
        .actions
        .iter()
        .find(|action| action.id == "volumes:vg/home:grow")
        .expect("volume grow action exists");
    assert_eq!(volume.context.desired_size.as_deref(), Some("800GiB"));
}

#[test]
fn plan_classifies_lvm_logical_volume_lifecycle() {
    let plan = plan_from_json_bytes(
        br#"{
              "volumes": {
                "vg0/scratch": {
                  "operation": "create",
                  "desiredSize": "10GiB"
                },
                "vg0/home": {
                  "operation": "activate"
                },
                "vg0/archive": {
                  "operation": "deactivate"
                },
                "vg0/reporting": {
                  "operation": "rescan"
                },
                "vg0/old": {
                  "destroy": true
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 5);
    assert_eq!(plan.summary.offline_required_count, 2);
    assert_eq!(plan.summary.destructive_count, 1);
    let create = plan
        .actions
        .iter()
        .find(|action| action.id == "volumes:vg0/scratch:create")
        .expect("LV create action exists");
    assert_eq!(create.risk, RiskClass::Online);
    assert_eq!(create.context.desired_size.as_deref(), Some("10GiB"));
    let destroy = plan
        .actions
        .iter()
        .find(|action| action.id == "volumes:vg0/old:destroy")
        .expect("LV destroy action exists");
    assert_eq!(destroy.risk, RiskClass::Destructive);
    assert!(destroy.destructive);
    let activate = plan
        .actions
        .iter()
        .find(|action| action.id == "volumes:vg0/home:activate")
        .expect("LV activate action exists");
    assert_eq!(activate.risk, RiskClass::OfflineRequired);
    assert!(!activate.destructive);
    let deactivate = plan
        .actions
        .iter()
        .find(|action| action.id == "volumes:vg0/archive:deactivate")
        .expect("LV deactivate action exists");
    assert_eq!(deactivate.risk, RiskClass::OfflineRequired);
    assert!(!deactivate.destructive);
    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "volumes:vg0/reporting:rescan")
        .expect("LV rescan action exists");
    assert_eq!(rescan.operation, Operation::Rescan);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert!(!rescan.destructive);
}

#[test]
fn plan_classifies_lvm_physical_volume_lifecycle() {
    let plan = plan_from_json_bytes(
        br#"{
              "physicalVolumes": {
                "/dev/disk/by-id/nvme-pv-new": {
                  "operation": "create"
                },
                "/dev/disk/by-id/nvme-pv-grow": {
                  "operation": "grow"
                },
                "/dev/disk/by-id/nvme-pv-old": {
                  "destroy": true
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 3);
    assert_eq!(plan.summary.destructive_count, 2);
    let create = plan
        .actions
        .iter()
        .find(|action| action.id == "physicalvolumes:/dev/disk/by-id/nvme-pv-new:create")
        .expect("PV create action exists");
    assert_eq!(create.risk, RiskClass::Destructive);
    assert!(create.destructive);
    let grow = plan
        .actions
        .iter()
        .find(|action| action.id == "physicalvolumes:/dev/disk/by-id/nvme-pv-grow:grow")
        .expect("PV grow action exists");
    assert_eq!(grow.risk, RiskClass::Online);
    let destroy = plan
        .actions
        .iter()
        .find(|action| action.id == "physicalvolumes:/dev/disk/by-id/nvme-pv-old:destroy")
        .expect("PV destroy action exists");
    assert_eq!(destroy.risk, RiskClass::Destructive);
    assert!(destroy.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("pvmove"))
    }));
}

#[test]
fn plan_classifies_lvm_volume_group_lifecycle() {
    let plan = plan_from_json_bytes(
        br#"{
              "volumeGroups": {
                "vg0": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/nvme-vg0"
                },
                "vgdata": {
                  "replaceDevices": {
                    "/dev/disk/by-id/old-pv": "/dev/disk/by-id/new-pv"
                  }
                },
                "importvg": {
                  "operation": "import"
                },
                "exportvg": {
                  "operation": "export"
                },
                "activevg": {
                  "operation": "activate"
                },
                "coldvg": {
                  "operation": "deactivate"
                },
                "oldvg": {
                  "destroy": true
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 7);
    assert_eq!(plan.summary.offline_required_count, 5);
    assert_eq!(plan.summary.destructive_count, 2);
    let create = plan
        .actions
        .iter()
        .find(|action| action.id == "volumegroups:vg0:create")
        .expect("volume group create action exists");
    assert_eq!(create.risk, RiskClass::Destructive);
    assert!(create.destructive);
    assert_eq!(
        create.context.device.as_deref(),
        Some("/dev/disk/by-id/nvme-vg0")
    );
    assert!(create.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("pvs"))
    }));
    let destroy = plan
        .actions
        .iter()
        .find(|action| action.id == "volumegroups:oldvg:destroy")
        .expect("volume group destroy action exists");
    assert_eq!(destroy.risk, RiskClass::Destructive);

    let replace = plan
        .actions
        .iter()
        .find(|action| action.id == "volumeGroups:vgdata:replace-device:/dev/disk/by-id/old-pv")
        .expect("volume group replacement action exists");
    assert_eq!(replace.risk, RiskClass::OfflineRequired);
    assert_eq!(
        replace.context.device.as_deref(),
        Some("/dev/disk/by-id/old-pv")
    );
    assert_eq!(
        replace.context.replacement.as_deref(),
        Some("/dev/disk/by-id/new-pv")
    );
    assert!(replace
        .advice
        .as_ref()
        .is_some_and(|advice| { advice.summary.contains("migrate extents before vgreduce") }));
    let import = plan
        .actions
        .iter()
        .find(|action| action.id == "volumegroups:importvg:import")
        .expect("volume group import action exists");
    assert_eq!(import.risk, RiskClass::OfflineRequired);
    assert!(!import.destructive);
    assert!(import.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("vgimport"))
    }));
    let export = plan
        .actions
        .iter()
        .find(|action| action.id == "volumegroups:exportvg:export")
        .expect("volume group export action exists");
    assert_eq!(export.risk, RiskClass::OfflineRequired);
    assert!(!export.destructive);
    let activate = plan
        .actions
        .iter()
        .find(|action| action.id == "volumegroups:activevg:activate")
        .expect("volume group activate action exists");
    assert_eq!(activate.risk, RiskClass::OfflineRequired);
    assert!(!activate.destructive);
    let deactivate = plan
        .actions
        .iter()
        .find(|action| action.id == "volumegroups:coldvg:deactivate")
        .expect("volume group deactivate action exists");
    assert_eq!(deactivate.risk, RiskClass::OfflineRequired);
    assert!(!deactivate.destructive);
}

#[test]
fn plan_classifies_disk_and_partition_lifecycle_safely() {
    let plan = plan_from_json_bytes(
        br#"{
              "disks": {
                "/dev/disk/by-id/nvme-root": {
                  "operation": "create",
                  "partitionType": "gpt"
                },
                "/dev/disk/by-id/nvme-data": {
                  "operation": "rescan"
                }
              },
              "partitions": {
                "root": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/nvme-root",
                  "start": "1MiB",
                  "end": "100%",
                  "partitionType": "linux"
                },
                "home": {
                  "operation": "grow",
                  "device": "/dev/disk/by-id/nvme-root",
                  "partitionNumber": 2,
                  "end": "100%"
                },
                "data-table": {
                  "operation": "rescan",
                  "device": "/dev/disk/by-id/nvme-data"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 5);
    assert_eq!(plan.summary.offline_required_count, 2);
    assert_eq!(plan.summary.destructive_count, 1);

    let root = plan
        .actions
        .iter()
        .find(|action| action.id == "partitions:root:create")
        .expect("partition create action exists");
    assert_eq!(root.risk, RiskClass::OfflineRequired);
    assert_eq!(
        root.context.device.as_deref(),
        Some("/dev/disk/by-id/nvme-root")
    );
    assert_eq!(root.context.start.as_deref(), Some("1MiB"));
    assert_eq!(root.context.end.as_deref(), Some("100%"));
    assert_eq!(root.context.partition_type.as_deref(), Some("linux"));

    let home = plan
        .actions
        .iter()
        .find(|action| action.id == "partitions:home:grow")
        .expect("partition grow action exists");
    assert_eq!(home.risk, RiskClass::OfflineRequired);
    assert_eq!(
        home.context.device.as_deref(),
        Some("/dev/disk/by-id/nvme-root")
    );
    assert_eq!(home.context.partition_number.as_deref(), Some("2"));
    assert_eq!(home.context.end.as_deref(), Some("100%"));

    let disk = plan
        .actions
        .iter()
        .find(|action| action.id == "disks:/dev/disk/by-id/nvme-root:create")
        .expect("disk create action exists");
    assert_eq!(disk.risk, RiskClass::Destructive);
    assert!(disk.destructive);

    let disk_rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "disks:/dev/disk/by-id/nvme-data:rescan")
        .expect("disk rescan action exists");
    assert_eq!(disk_rescan.risk, RiskClass::Online);
    assert!(!disk_rescan.destructive);

    let partition_rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "partitions:data-table:rescan")
        .expect("partition rescan action exists");
    assert_eq!(partition_rescan.risk, RiskClass::Online);
    assert_eq!(
        partition_rescan.context.device.as_deref(),
        Some("/dev/disk/by-id/nvme-data")
    );
}

#[test]
fn plan_classifies_swap_and_luks_lifecycle_safely() {
    let plan = plan_from_json_bytes(
        br#"{
              "swaps": {
                "primary": {
                  "device": "/dev/disk/by-label/swap",
                  "preserveData": false
                },
                "scratch": {
                  "device": "/swapfile",
                  "operation": "grow",
                  "desiredSize": "16GiB"
                },
                "inventory": {
                  "device": "/dev/disk/by-label/swap-inventory",
                  "operation": "rescan"
                },
                "retired": {
                  "device": "/dev/disk/by-label/old-swap",
                  "operation": "deactivate"
                },
                "remove": {
                  "device": "/dev/disk/by-label/remove-swap",
                  "operation": "destroy"
                }
              },
              "luks": {
                "devices": {
                  "cryptroot": {
                    "name": "cryptroot",
                    "device": "/dev/disk/by-partuuid/root",
                    "operation": "grow"
                  },
                  "cryptdata": {
                    "name": "cryptdata",
                    "device": "/dev/disk/by-id/data-luks",
                    "operation": "create"
                  },
                  "cryptarchive": {
                    "name": "cryptarchive",
                    "device": "/dev/disk/by-id/archive-luks",
                    "operation": "open",
                    "preserveData": false
                  },
                  "cryptmissing": {
                    "name": "cryptmissing",
                    "operation": "create"
                  },
                  "cryptscratch": {
                    "name": "cryptscratch",
                    "device": "/dev/disk/by-id/scratch",
                    "preserveData": false
                  },
                  "cryptold": {
                    "name": "cryptold",
                    "device": "/dev/disk/by-id/old-luks",
                    "operation": "destroy"
                  },
                  "cryptclosed": {
                    "name": "cryptclosed",
                    "device": "/dev/disk/by-id/closed-luks",
                    "operation": "close"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 12);
    assert_eq!(plan.summary.offline_required_count, 8);
    assert_eq!(plan.summary.destructive_count, 3);

    let swap = plan
        .actions
        .iter()
        .find(|action| action.id == "swaps:primary:format")
        .expect("swap format action exists");
    assert_eq!(swap.risk, RiskClass::Destructive);
    assert_eq!(
        swap.context.device.as_deref(),
        Some("/dev/disk/by-label/swap")
    );

    let swap_rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "swaps:inventory:rescan")
        .expect("swap rescan action exists");
    assert_eq!(swap_rescan.operation, Operation::Rescan);
    assert_eq!(swap_rescan.risk, RiskClass::Online);
    assert!(!swap_rescan.destructive);
    assert_eq!(
        swap_rescan.context.device.as_deref(),
        Some("/dev/disk/by-label/swap-inventory")
    );

    let swap_deactivate = plan
        .actions
        .iter()
        .find(|action| action.id == "swaps:retired:deactivate")
        .expect("swap deactivate action exists");
    assert_eq!(swap_deactivate.operation, Operation::Deactivate);
    assert_eq!(swap_deactivate.risk, RiskClass::OfflineRequired);
    assert!(!swap_deactivate.destructive);

    let swap_destroy = plan
        .actions
        .iter()
        .find(|action| action.id == "swaps:remove:destroy")
        .expect("swap destroy action exists");
    assert_eq!(swap_destroy.operation, Operation::Destroy);
    assert_eq!(swap_destroy.risk, RiskClass::Destructive);
    assert!(swap_destroy.destructive);
    assert!(swap_destroy.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("deactivate"))
    }));

    let luks = plan
        .actions
        .iter()
        .find(|action| action.id == "luks.devices:cryptroot:grow")
        .expect("luks grow action exists");
    assert_eq!(luks.risk, RiskClass::OfflineRequired);
    assert_eq!(luks.context.target.as_deref(), Some("cryptroot"));
    assert_eq!(
        luks.context.device.as_deref(),
        Some("/dev/disk/by-partuuid/root")
    );

    let open = plan
        .actions
        .iter()
        .find(|action| action.id == "luks.devices:cryptdata:create")
        .expect("luks open action exists");
    assert_eq!(open.risk, RiskClass::OfflineRequired);
    assert!(!open.destructive);
    assert_eq!(open.context.target.as_deref(), Some("cryptdata"));
    assert_eq!(
        open.context.device.as_deref(),
        Some("/dev/disk/by-id/data-luks")
    );

    let explicit_open = plan
        .actions
        .iter()
        .find(|action| action.id == "luks.devices:cryptarchive:open")
        .expect("explicit luks open action exists");
    assert_eq!(explicit_open.operation, Operation::Open);
    assert_eq!(explicit_open.risk, RiskClass::OfflineRequired);
    assert!(!explicit_open.destructive);
    assert_eq!(
        explicit_open.context.device.as_deref(),
        Some("/dev/disk/by-id/archive-luks")
    );

    let missing = plan
        .actions
        .iter()
        .find(|action| action.id == "luks.devices:cryptmissing:create")
        .expect("underspecified luks open action exists");
    assert_eq!(missing.risk, RiskClass::OfflineRequired);
    assert_eq!(missing.context.target.as_deref(), Some("cryptmissing"));
    assert_eq!(missing.context.device, None);

    let close = plan
        .actions
        .iter()
        .find(|action| action.id == "luks.devices:cryptold:destroy")
        .expect("luks close action exists");
    assert_eq!(close.risk, RiskClass::OfflineRequired);
    assert!(!close.destructive);
    assert_eq!(close.context.target.as_deref(), Some("cryptold"));
    assert_eq!(
        close.context.device.as_deref(),
        Some("/dev/disk/by-id/old-luks")
    );

    let explicit_close = plan
        .actions
        .iter()
        .find(|action| action.id == "luks.devices:cryptclosed:close")
        .expect("explicit luks close action exists");
    assert_eq!(explicit_close.operation, Operation::Close);
    assert_eq!(explicit_close.risk, RiskClass::OfflineRequired);
    assert!(!explicit_close.destructive);
    assert_eq!(
        explicit_close.context.target.as_deref(),
        Some("cryptclosed")
    );
}

#[test]
fn plan_accepts_luks_mapper_aliases_for_logical_keys() {
    let plan = plan_from_json_bytes(
        br#"{
              "luks": {
                "devices": {
                  "rootMapping": {
                    "target": "cryptroot",
                    "device": "/dev/disk/by-id/root-luks",
                    "operation": "grow"
                  },
                  "archiveMapping": {
                    "mapperName": "cryptarchive",
                    "device": "/dev/disk/by-id/archive-luks",
                    "operation": "open"
                  },
                  "backupMapping": {
                    "mapper": "cryptbackup",
                    "device": "/dev/disk/by-id/backup-luks",
                    "operation": "close"
                  },
                  "hyphenMapping": {
                    "mapper-name": "crypthyphen",
                    "device": "/dev/disk/by-id/hyphen-luks",
                    "operation": "open"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    let root = plan
        .actions
        .iter()
        .find(|action| action.id == "luks.devices:rootMapping:grow")
        .expect("target alias grow action exists");
    assert_eq!(root.context.target.as_deref(), Some("cryptroot"));

    let archive = plan
        .actions
        .iter()
        .find(|action| action.id == "luks.devices:archiveMapping:open")
        .expect("mapperName alias open action exists");
    assert_eq!(archive.context.target.as_deref(), Some("cryptarchive"));

    let backup = plan
        .actions
        .iter()
        .find(|action| action.id == "luks.devices:backupMapping:close")
        .expect("mapper alias close action exists");
    assert_eq!(backup.context.target.as_deref(), Some("cryptbackup"));

    let hyphen = plan
        .actions
        .iter()
        .find(|action| action.id == "luks.devices:hyphenMapping:open")
        .expect("hyphenated mapper alias open action exists");
    assert_eq!(hyphen.context.target.as_deref(), Some("crypthyphen"));
}

#[test]
fn plan_accepts_swap_label_and_uuid_properties() {
    let plan = plan_from_json_bytes(
        br#"{
              "swaps": {
                "primary": {
                  "device": "/dev/disk/by-label/swap-old",
                  "properties": {
                    "label": "swap-new",
                    "swap.uuid": "01234567-89ab-cdef-0123-456789abcdef",
                    "priority": "10"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 4);
    assert_eq!(plan.summary.offline_required_count, 3);
    assert_eq!(plan.summary.unsupported_count, 0);

    let label = plan
        .actions
        .iter()
        .find(|action| action.id == "swaps:primary:set-property:label")
        .expect("swap label action exists");
    assert_eq!(label.operation, Operation::SetProperty);
    assert_eq!(label.risk, RiskClass::OfflineRequired);
    assert_eq!(label.context.property_value.as_deref(), Some("swap-new"));
    assert!(label.advice.as_ref().is_some_and(|advice| {
        advice
            .summary
            .contains("swap label and UUID updates mutate swap signature identity")
    }));

    let uuid = plan
        .actions
        .iter()
        .find(|action| action.id == "swaps:primary:set-property:swap.uuid")
        .expect("swap UUID action exists");
    assert_eq!(uuid.risk, RiskClass::OfflineRequired);
    assert_eq!(
        uuid.context.property_value.as_deref(),
        Some("01234567-89ab-cdef-0123-456789abcdef")
    );

    let priority = plan
        .actions
        .iter()
        .find(|action| action.id == "swaps:primary:set-property:priority")
        .expect("swap priority property action exists");
    assert_eq!(priority.risk, RiskClass::OfflineRequired);
    assert!(priority.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("NixOS swapDevices priority"))
    }));
}

#[test]
fn plan_accepts_swap_path_aliases_for_logical_keys() {
    let plan = plan_from_json_bytes(
        br#"{
              "swaps": {
                "scratch": {
                  "path": "/swapfile",
                  "operation": "grow",
                  "desiredSize": "16GiB"
                },
                "inventory": {
                  "target": "/dev/disk/by-label/swap-inventory",
                  "operation": "rescan"
                },
                "primary": {
                  "path": "/dev/disk/by-label/swap",
                  "preserveData": false
                }
              }
            }"#,
    )
    .expect("plan should parse");

    let grow = plan
        .actions
        .iter()
        .find(|action| action.id == "swaps:scratch:grow")
        .expect("logical-key swap grow action exists");
    assert_eq!(grow.context.target.as_deref(), Some("/swapfile"));
    assert_eq!(grow.context.device.as_deref(), Some("/swapfile"));
    assert_eq!(grow.context.desired_size.as_deref(), Some("16GiB"));

    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "swaps:inventory:rescan")
        .expect("logical-key swap rescan action exists");
    assert_eq!(
        rescan.context.target.as_deref(),
        Some("/dev/disk/by-label/swap-inventory")
    );

    let format = plan
        .actions
        .iter()
        .find(|action| action.id == "swaps:primary:format")
        .expect("logical-key swap format action exists");
    assert_eq!(
        format.context.target.as_deref(),
        Some("/dev/disk/by-label/swap")
    );
    assert_eq!(format.risk, RiskClass::Destructive);
}

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

#[test]
fn topology_comparison_reports_current_state_diagnostics() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "home": {
                  "mountpoint": "/home",
                  "fsType": "btrfs",
                  "resizePolicy": "grow-only",
                  "desiredSize": "750GiB"
                }
              },
              "datasets": {
                "tank/home": {
                  "properties": {
                    "compression": "zstd"
                  }
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
    graph.add_node(
        Node::new("zfs:dataset:tank/home", NodeKind::ZfsDataset, "tank/home")
            .with_property("compression", "zstd"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 2);
    assert_eq!(comparison.summary.matched_count, 2);
    assert_eq!(comparison.summary.missing_count, 0);
    assert_eq!(comparison.summary.type_conflict_count, 1);
    assert_eq!(comparison.summary.already_satisfied_count, 1);
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert_eq!(plan.summary.action_count, 1);
    assert!(plan
        .actions
        .iter()
        .all(|action| action.id != "datasets:tank/home:set-property:compression"));
    assert!(plan.actions.iter().any(|action| {
        action.id == "filesystem:home:grow" && action.operation == Operation::Grow
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == TopologyDiagnosticKind::SizeBelowDesired
            && diagnostic.action_id == "filesystem:home:grow"
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == TopologyDiagnosticKind::FilesystemTypeConflict
            && diagnostic.action_id == "filesystem:home:grow"
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
            && diagnostic.action_id == "datasets:tank/home:set-property:compression"
    }));
}

#[test]
fn topology_comparison_reconciles_zfs_dataset_property_aliases() {
    let plan = plan_from_json_bytes(
        br#"{
              "datasets": {
                "tank/home": {
                  "properties": {
                    "compression": "zstd",
                    "mountpoint": "/home",
                    "atime": true
                  }
                },
                "tank/archive": {
                  "properties": {
                    "compression": "lz4"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("zfs:dataset:tank/home", NodeKind::ZfsDataset, "tank/home")
            .with_property("zfs.compression", "zstd")
            .with_property("zfs.mountpoint", "/home")
            .with_property("zfs.atime", "on"),
    );
    graph.add_node(
        Node::new(
            "zfs:dataset:tank/archive",
            NodeKind::ZfsDataset,
            "tank/archive",
        )
        .with_property("zfs.compression", "zstd"),
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
        action.id == "datasets:tank/archive:set-property:compression"
            && action.operation == Operation::SetProperty
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
        diagnostic.action_id == "datasets:tank/home:set-property:atime"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "datasets:tank/archive:set-property:compression"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::PropertyDiffers
            && diagnostic.message.contains("zstd")
            && diagnostic.message.contains("lz4")
    }));
}

#[test]
fn topology_comparison_reconciles_filesystem_identity_property_aliases() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "home": {
                  "mountpoint": "/home",
                  "device": "/dev/disk/by-label/home",
                  "fsType": "ext4",
                  "properties": {
                    "filesystem.label": "homefs",
                    "ext.uuid": "11111111-2222-3333-4444-555555555555"
                  }
                },
                "scratch": {
                  "mountpoint": "/scratch",
                  "device": "/dev/disk/by-label/scratch",
                  "fsType": "xfs",
                  "properties": {
                    "xfs.label": "scratch-new"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("filesystem:/home", NodeKind::Filesystem, "/home")
            .with_path("/home")
            .with_identity(Identity {
                uuid: Some("11111111-2222-3333-4444-555555555555".to_string()),
                partuuid: None,
                label: Some("homefs".to_string()),
                serial: None,
                wwn: None,
            })
            .with_property("filesystem.type", "ext4"),
    );
    graph.add_node(
        Node::new("filesystem:/scratch", NodeKind::Filesystem, "/scratch")
            .with_path("/scratch")
            .with_property("filesystem.type", "xfs")
            .with_property("filesystem.label", "scratch-old"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 5);
    assert_eq!(comparison.summary.matched_count, 5);
    assert_eq!(comparison.summary.already_satisfied_count, 2);
    assert_eq!(comparison.summary.suppressed_action_count, 2);
    assert_eq!(plan.actions.len(), 3);
    assert!(plan.actions.iter().any(|action| {
        action.id == "filesystems:scratch:set-property:xfs.label"
            && action.operation == Operation::SetProperty
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystems:home:set-property:filesystem.label"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystems:home:set-property:ext.uuid"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystems:scratch:set-property:xfs.label"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::PropertyDiffers
            && diagnostic.message.contains("scratch-old")
            && diagnostic.message.contains("scratch-new")
    }));
}

#[test]
fn topology_comparison_reconciles_filesystem_serial_property_aliases() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "efi": {
                  "mountpoint": "/boot",
                  "device": "/dev/disk/by-partlabel/EFI",
                  "fsType": "vfat",
                  "properties": {
                    "vfat.uuid": "a1b2-c3d4"
                  }
                },
                "windows": {
                  "mountpoint": "/mnt/windows",
                  "device": "/dev/disk/by-label/Windows",
                  "fsType": "ntfs",
                  "properties": {
                    "ntfs.volume-serial": "0123456789ABCDEF"
                  }
                },
                "shared": {
                  "mountpoint": "/mnt/shared",
                  "device": "/dev/disk/by-label/Shared",
                  "fsType": "exfat",
                  "properties": {
                    "exfat.uuid": "6EEF-953B"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("filesystem:/boot", NodeKind::Filesystem, "/boot")
            .with_path("/boot")
            .with_identity(Identity {
                uuid: Some("A1B2-C3D4".to_string()),
                partuuid: None,
                label: None,
                serial: None,
                wwn: None,
            })
            .with_property("filesystem.type", "vfat"),
    );
    graph.add_node(
        Node::new(
            "filesystem:/mnt/windows",
            NodeKind::Filesystem,
            "/mnt/windows",
        )
        .with_path("/mnt/windows")
        .with_property("filesystem.type", "ntfs")
        .with_property("ntfs.volume-serial", "01234567-89abcdef"),
    );
    graph.add_node(
        Node::new(
            "filesystem:/mnt/shared",
            NodeKind::Filesystem,
            "/mnt/shared",
        )
        .with_path("/mnt/shared")
        .with_property("filesystem.type", "exfat")
        .with_property("exfat.volume-serial", "0x6eef953b"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 6);
    assert_eq!(comparison.summary.matched_count, 6);
    assert_eq!(comparison.summary.already_satisfied_count, 3);
    assert_eq!(comparison.summary.suppressed_action_count, 3);
    assert_eq!(plan.actions.len(), 3);
    assert!(plan.actions.iter().all(|action| {
        !action.id.contains(":set-property:")
            || !matches!(
                action.id.as_str(),
                "filesystems:efi:set-property:vfat.uuid"
                    | "filesystems:windows:set-property:ntfs.volume-serial"
                    | "filesystems:shared:set-property:exfat.uuid"
            )
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystems:efi:set-property:vfat.uuid"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystems:windows:set-property:ntfs.volume-serial"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystems:shared:set-property:exfat.uuid"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_reports_matching_filesystem_format_type() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "data": {
                  "mountpoint": "/data",
                  "device": "/dev/disk/by-label/data",
                  "fsType": "ext4",
                  "preserveData": false
                },
                "legacy": {
                  "mountpoint": "/legacy",
                  "device": "/dev/disk/by-label/legacy",
                  "fsType": "xfs",
                  "preserveData": false
                },
                "small": {
                  "mountpoint": "/small",
                  "device": "/dev/disk/by-label/small",
                  "fsType": "ext4",
                  "desiredSize": "2GiB",
                  "preserveData": false
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("filesystem:/data", NodeKind::Filesystem, "/data")
            .with_path("/data")
            .with_property("filesystem.type", "ext4"),
    );
    graph.add_node(
        Node::new("filesystem:/legacy", NodeKind::Filesystem, "/legacy")
            .with_path("/legacy")
            .with_property("filesystem.type", "ext4"),
    );
    graph.add_node(
        Node::new("filesystem:/small", NodeKind::Filesystem, "/small")
            .with_path("/small")
            .with_size_bytes(1024 * 1024 * 1024)
            .with_property("filesystem.type", "ext4"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 6);
    assert_eq!(comparison.summary.matched_count, 6);
    assert_eq!(comparison.summary.type_conflict_count, 2);
    assert_eq!(comparison.summary.already_satisfied_count, 2);
    assert_eq!(comparison.summary.suppressed_action_count, 0);
    assert_eq!(plan.summary.action_count, 6);
    assert!(plan.actions.iter().any(|action| {
        action.id == "filesystem:data:preserve-data-disabled"
            && action.operation == Operation::Format
    }));
    assert!(plan.actions.iter().any(|action| {
        action.id == "filesystem:legacy:preserve-data-disabled"
            && action.operation == Operation::Format
    }));
    assert!(plan.actions.iter().any(|action| {
        action.id == "filesystem:small:preserve-data-disabled"
            && action.operation == Operation::Format
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystem:data:preserve-data-disabled"
            && diagnostic.kind == TopologyDiagnosticKind::FilesystemFormatAlreadySatisfied
            && diagnostic.message.contains("type ext4")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystem:legacy:preserve-data-disabled"
            && diagnostic.kind == TopologyDiagnosticKind::FilesystemTypeConflict
            && diagnostic.level == TopologyDiagnosticLevel::Warning
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystem:small:preserve-data-disabled"
            && diagnostic.kind == TopologyDiagnosticKind::FilesystemFormatAlreadySatisfied
            && diagnostic.message.contains("type ext4")
    }));
}

#[test]
fn topology_comparison_suppresses_already_mounted_sources() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "backup": {
                  "operation": "mount",
                  "mountpoint": "/backup",
                  "device": "/dev/disk/by-label/backup",
                  "fsType": "xfs"
                }
              },
              "nfs": {
                "mounts": {
                  "/srv/shared": {
                    "operation": "mount",
                    "source": "nas.example.com:/srv/shared",
                    "fsType": "nfs4"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("mount:/backup", NodeKind::Mountpoint, "/backup")
            .with_property("mount.source", "/dev/disk/by-label/backup"),
    );
    graph.add_node(
        Node::new("mount:/srv/shared", NodeKind::NfsMount, "/srv/shared")
            .with_property("mount.source", "nas.example.com:/srv/shared"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 3);
    assert_eq!(comparison.summary.already_satisfied_count, 2);
    assert_eq!(comparison.summary.suppressed_action_count, 2);
    assert!(plan
        .actions
        .iter()
        .all(|action| action.id != "filesystems:backup:mount"));
    assert!(plan
        .actions
        .iter()
        .all(|action| action.id != "nfs.mounts:/srv/shared:mount"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystems:backup:mount"
            && diagnostic.kind == TopologyDiagnosticKind::MountAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "nfs.mounts:/srv/shared:mount"
            && diagnostic.kind == TopologyDiagnosticKind::MountAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_mount_action_when_source_differs() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "backup": {
                  "operation": "mount",
                  "mountpoint": "/backup",
                  "device": "/dev/disk/by-label/backup",
                  "fsType": "xfs"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("mount:/backup", NodeKind::Mountpoint, "/backup")
            .with_property("mount.source", "/dev/disk/by-label/other"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 2);
    assert_eq!(comparison.summary.already_satisfied_count, 0);
    assert_eq!(comparison.summary.suppressed_action_count, 0);
    assert!(plan
        .actions
        .iter()
        .any(|action| action.id == "filesystems:backup:mount"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystems:backup:mount"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::MountSourceConflict
    }));
}

#[test]
fn topology_comparison_keeps_absent_nfs_mount_actionable() {
    let plan = plan_from_json_bytes(
        br#"{
              "nfs": {
                "mounts": {
                  "/srv/shared": {
                    "operation": "mount",
                    "source": "nas.example.com:/srv/shared"
                  }
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
        diagnostic.action_id == "nfs.mounts:/srv/shared:mount"
            && diagnostic.level == TopologyDiagnosticLevel::Info
            && diagnostic.kind == TopologyDiagnosticKind::MountRequired
            && diagnostic.message.contains("nas.example.com:/srv/shared")
    }));
}

#[test]
fn topology_comparison_suppresses_unmount_when_mountpoint_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "archive": {
                  "operation": "unmount",
                  "mountpoint": "/archive"
                }
              },
              "nfs": {
                "mounts": {
                  "/srv/old": {
                    "operation": "unmount",
                    "source": "nas.example.com:/srv/old"
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

    assert_eq!(comparison.summary.action_count, 3);
    assert_eq!(comparison.summary.missing_count, 1);
    assert_eq!(comparison.summary.already_satisfied_count, 2);
    assert_eq!(comparison.summary.suppressed_action_count, 2);
    assert!(plan
        .actions
        .iter()
        .all(|action| action.id != "filesystems:archive:unmount"));
    assert!(plan
        .actions
        .iter()
        .all(|action| action.id != "nfs.mounts:/srv/old:unmount"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystems:archive:unmount"
            && diagnostic.current.is_none()
            && diagnostic.kind == TopologyDiagnosticKind::UnmountAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "nfs.mounts:/srv/old:unmount"
            && diagnostic.current.is_none()
            && diagnostic.kind == TopologyDiagnosticKind::UnmountAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_unmount_when_mountpoint_exists() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "archive": {
                  "operation": "unmount",
                  "mountpoint": "/archive"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("mount:/archive", NodeKind::Mountpoint, "/archive")
            .with_property("mount.source", "/dev/disk/by-label/archive"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 2);
    assert_eq!(comparison.summary.already_satisfied_count, 0);
    assert_eq!(comparison.summary.suppressed_action_count, 0);
    assert!(plan
        .actions
        .iter()
        .any(|action| action.id == "filesystems:archive:unmount"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystems:archive:unmount"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::UnmountRequired
    }));
}

#[test]
fn topology_comparison_suppresses_inactive_swap_teardown() {
    let plan = plan_from_json_bytes(
        br#"{
              "swaps": {
                "old-file": {
                  "path": "/swapfile.old",
                  "operation": "deactivate"
                },
                "old-device": {
                  "device": "/dev/disk/by-label/old-swap",
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

    assert_eq!(comparison.summary.action_count, 2);
    assert_eq!(comparison.summary.already_satisfied_count, 2);
    assert_eq!(comparison.summary.suppressed_action_count, 2);
    assert!(plan.actions.is_empty());
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "swaps:old-file:deactivate"
            && diagnostic.current.is_none()
            && diagnostic.kind == TopologyDiagnosticKind::SwapDeactivateAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "swaps:old-device:destroy"
            && diagnostic.current.is_none()
            && diagnostic.kind == TopologyDiagnosticKind::SwapDestroyAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_active_swap_teardown() {
    let plan = plan_from_json_bytes(
        br#"{
              "swaps": {
                "scratch": {
                  "path": "/swapfile",
                  "operation": "deactivate"
                },
                "remove": {
                  "device": "/dev/disk/by-label/remove-swap",
                  "operation": "destroy"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("swap:/swapfile", NodeKind::Swap, "/swapfile")
            .with_path("/swapfile")
            .with_size_bytes(1_073_741_824)
            .with_usage(Usage {
                used_bytes: Some(134_217_728),
                free_bytes: Some(939_524_096),
                allocated_bytes: Some(1_073_741_824),
            })
            .with_property("swap.active", "true")
            .with_property("swap.type", "file")
            .with_property("swap.priority", "10"),
    );
    graph.add_node(
        Node::new(
            "swap:/dev/disk/by-label/remove-swap",
            NodeKind::Swap,
            "/dev/disk/by-label/remove-swap",
        )
        .with_path("/dev/disk/by-label/remove-swap")
        .with_property("swap.active", "true")
        .with_property("swap.type", "partition"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 2);
    assert_eq!(comparison.summary.already_satisfied_count, 0);
    assert_eq!(comparison.summary.suppressed_action_count, 0);
    assert_eq!(plan.actions.len(), 2);
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "swaps:scratch:deactivate"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SwapDeactivateRequired
            && diagnostic.message.contains("priority 10")
            && diagnostic.message.contains("type file")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "swaps:remove:destroy"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SwapDestroyRequired
            && diagnostic.message.contains("type partition")
    }));
}

#[test]
fn topology_comparison_reports_swap_format_target_metadata() {
    let plan = plan_from_json_bytes(
        br#"{
              "swaps": {
                "scratch": {
                  "path": "/swapfile",
                  "operation": "format"
                },
                "device": {
                  "device": "/dev/disk/by-label/swap",
                  "operation": "format"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("swap:/swapfile", NodeKind::Swap, "/swapfile")
            .with_path("/swapfile")
            .with_size_bytes(2_147_483_648)
            .with_usage(Usage {
                used_bytes: Some(268_435_456),
                free_bytes: Some(1_879_048_192),
                allocated_bytes: Some(2_147_483_648),
            })
            .with_property("swap.active", "true")
            .with_property("swap.type", "file")
            .with_property("swap.priority", "5"),
    );
    graph.add_node(
        Node::new(
            "filesystem:/dev/disk/by-label/swap",
            NodeKind::Filesystem,
            "/dev/disk/by-label/swap",
        )
        .with_path("/dev/disk/by-label/swap")
        .with_property("filesystem.type", "ext4"),
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
    assert_eq!(plan.summary.action_count, 2);
    assert!(plan.actions.iter().any(|action| {
        action.id == "swaps:scratch:format" && action.operation == Operation::Format
    }));
    assert!(plan.actions.iter().any(|action| {
        action.id == "swaps:device:format" && action.operation == Operation::Format
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "swaps:scratch:format"
            && diagnostic.query == "/swapfile"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SwapFormatTargetPresent
            && diagnostic.message.contains("size 2147483648 bytes")
            && diagnostic.message.contains("used 268435456 bytes")
            && diagnostic.message.contains("priority 5")
            && diagnostic.message.contains("type file")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "swaps:device:format"
            && diagnostic.query == "/dev/disk/by-label/swap"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SwapFormatTargetPresent
            && diagnostic.message.contains("filesystem")
    }));
}

#[test]
fn topology_comparison_reconciles_swap_identity_property_aliases() {
    let plan = plan_from_json_bytes(
        br#"{
              "swaps": {
                "primary": {
                  "device": "/dev/disk/by-label/swap-old",
                  "properties": {
                    "label": "swap-new",
                    "swap.uuid": "01234567-89AB-CDEF-0123-456789ABCDEF",
                    "priority": "10"
                  }
                },
                "scratch": {
                  "device": "/dev/disk/by-label/scratch-swap",
                  "properties": {
                    "uuid": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                    "swap.priority": "20"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "swap:/dev/disk/by-label/swap-old",
            NodeKind::Swap,
            "swap-old",
        )
        .with_path("/dev/disk/by-label/swap-old")
        .with_identity(Identity {
            uuid: Some("01234567-89ab-cdef-0123-456789abcdef".to_string()),
            partuuid: None,
            label: Some("swap-new".to_string()),
            serial: None,
            wwn: None,
        })
        .with_property("swap.active", "false")
        .with_property("swap.priority", "10"),
    );
    graph.add_node(
        Node::new(
            "swap:/dev/disk/by-label/scratch-swap",
            NodeKind::Swap,
            "scratch-swap",
        )
        .with_path("/dev/disk/by-label/scratch-swap")
        .with_property("swap.uuid", "ffffffff-1111-2222-3333-444444444444")
        .with_property("swap.priority", "5"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 7);
    assert_eq!(comparison.summary.matched_count, 7);
    assert_eq!(comparison.summary.already_satisfied_count, 3);
    assert_eq!(comparison.summary.suppressed_action_count, 3);
    assert_eq!(plan.actions.len(), 4);
    assert!(plan.actions.iter().any(|action| {
        action.id == "swaps:scratch:set-property:uuid" && action.operation == Operation::SetProperty
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "swaps:primary:set-property:label"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "swaps:primary:set-property:swap.uuid"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "swaps:primary:set-property:priority"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "swaps:scratch:set-property:uuid"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::PropertyDiffers
            && diagnostic
                .message
                .contains("ffffffff-1111-2222-3333-444444444444")
            && diagnostic
                .message
                .contains("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "swaps:scratch:set-property:swap.priority"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::PropertyDiffers
            && diagnostic.message.contains("is 5")
            && diagnostic.message.contains("desired 20")
    }));
}

#[test]
fn topology_comparison_reconciles_luks_identity_property_aliases() {
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
                      "luks.uuid": "01234567-89AB-CDEF-0123-456789ABCDEF"
                    }
                  },
                  "cryptdata": {
                    "name": "cryptdata",
                    "device": "/dev/disk/by-id/data-luks",
                    "properties": {
                      "cryptsetup.label": "data-new"
                    }
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
        .with_identity(Identity {
            uuid: Some("01234567-89ab-cdef-0123-456789abcdef".to_string()),
            partuuid: None,
            label: Some("root".to_string()),
            serial: None,
            wwn: None,
        })
        .with_property("cryptsetup.luks-subsystem", "nixos"),
    );
    graph.add_node(
        Node::new(
            "block:/dev/disk/by-id/data-luks",
            NodeKind::LuksContainer,
            "data-luks",
        )
        .with_path("/dev/disk/by-id/data-luks")
        .with_property("cryptsetup.label", "data-old"),
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
        action.id == "luks.devices:cryptdata:set-property:cryptsetup.label"
            && action.operation == Operation::SetProperty
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "luks.devices:cryptroot:set-property:label"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "luks.devices:cryptroot:set-property:luks.subsystem"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "luks.devices:cryptroot:set-property:luks.uuid"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "luks.devices:cryptdata:set-property:cryptsetup.label"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::PropertyDiffers
            && diagnostic.message.contains("data-old")
            && diagnostic.message.contains("data-new")
    }));
}

#[test]
fn topology_comparison_suppresses_remount_when_options_are_present() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "scratch": {
                  "operation": "remount",
                  "mountpoint": "/scratch",
                  "options": ["rw", "noatime", "discard=async"]
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("mount:/scratch", NodeKind::Mountpoint, "/scratch")
            .with_property("mount.options", "rw,relatime,noatime,discard=async"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 2);
    assert_eq!(comparison.summary.already_satisfied_count, 1);
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert!(plan
        .actions
        .iter()
        .all(|action| action.id != "filesystems:scratch:remount"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystems:scratch:remount"
            && diagnostic.kind == TopologyDiagnosticKind::MountOptionsAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_suppresses_nfs_remount_from_nfs_option_properties() {
    let plan = plan_from_json_bytes(
        br#"{
              "nfs": {
                "mounts": {
                  "/srv/shared": {
                    "operation": "remount",
                    "options": ["rw", "vers=4.2", "_netdev"]
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("mount:/srv/shared", NodeKind::NfsMount, "/srv/shared")
            .with_property("nfs.rw", "true")
            .with_property("nfs.vers", "4.2")
            .with_property("nfs.netdev", "true"),
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
        diagnostic.action_id == "nfs.mounts:/srv/shared:remount"
            && diagnostic.kind == TopologyDiagnosticKind::MountOptionsAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_remount_when_options_differ() {
    let plan = plan_from_json_bytes(
        br#"{
              "filesystems": {
                "scratch": {
                  "operation": "remount",
                  "mountpoint": "/scratch",
                  "options": ["ro", "noatime"]
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("mount:/scratch", NodeKind::Mountpoint, "/scratch")
            .with_property("mount.options", "rw,relatime"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 2);
    assert_eq!(comparison.summary.already_satisfied_count, 0);
    assert_eq!(comparison.summary.suppressed_action_count, 0);
    assert!(plan
        .actions
        .iter()
        .any(|action| action.id == "filesystems:scratch:remount"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "filesystems:scratch:remount"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::MountOptionsDiffer
    }));
}

#[test]
fn topology_comparison_keeps_absent_nfs_export_actionable() {
    let plan = plan_from_json_bytes(
        br#"{
              "exports": {
                "/srv/share": {
                  "operation": "export",
                  "client": "192.0.2.0/24",
                  "options": "rw,sync,no_subtree_check"
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
        diagnostic.action_id == "exports:/srv/share:export"
            && diagnostic.level == TopologyDiagnosticLevel::Info
            && diagnostic.kind == TopologyDiagnosticKind::NfsExportRequired
            && diagnostic.message.contains("192.0.2.0/24")
            && diagnostic.message.contains("rw,sync,no_subtree_check")
    }));
}

#[test]
fn topology_comparison_suppresses_already_exported_nfs_path() {
    let plan = plan_from_json_bytes(
        br#"{
              "exports": {
                "/srv/share": {
                  "operation": "export",
                  "client": "192.0.2.0/24",
                  "options": "rw,sync,no_subtree_check"
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

    assert_eq!(comparison.summary.action_count, 1);
    assert_eq!(comparison.summary.already_satisfied_count, 1);
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert!(plan.actions.is_empty());
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "exports:/srv/share:export"
            && diagnostic.kind == TopologyDiagnosticKind::NfsExportAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_nfs_export_when_client_or_options_differ() {
    let plan = plan_from_json_bytes(
        br#"{
              "exports": {
                "/srv/share": {
                  "operation": "export",
                  "client": "192.0.2.0/24",
                  "options": "rw,sync,no_subtree_check"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "nfs-export:/srv/share:198.51.100.10",
            NodeKind::NfsExport,
            "/srv/share",
        )
        .with_property("nfs.export", "/srv/share")
        .with_property("nfs.export-client", "198.51.100.10")
        .with_property("nfs.exportfs", "true")
        .with_property("nfs.export-option-ro", "true")
        .with_property("nfs.export-option-sync", "true"),
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
        diagnostic.action_id == "exports:/srv/share:export"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::NfsExportDiffers
    }));
}

#[test]
fn topology_comparison_suppresses_absent_nfs_unexport() {
    let plan = plan_from_json_bytes(
        br#"{
              "exports": {
                "/srv/old": {
                  "operation": "unexport",
                  "client": "192.0.2.0/24"
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
    assert_eq!(comparison.summary.missing_count, 0);
    assert_eq!(comparison.summary.already_satisfied_count, 1);
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert!(plan.actions.is_empty());
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "exports:/srv/old:unexport"
            && diagnostic.current.is_none()
            && diagnostic.kind == TopologyDiagnosticKind::NfsUnexportAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_nfs_unexport_when_export_exists() {
    let plan = plan_from_json_bytes(
        br#"{
              "exports": {
                "/srv/old": {
                  "operation": "unexport",
                  "client": "192.0.2.0/24"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "nfs-export:/srv/old:192.0.2.0/24",
            NodeKind::NfsExport,
            "/srv/old",
        )
        .with_property("nfs.export", "/srv/old")
        .with_property("nfs.export-client", "192.0.2.0/24")
        .with_property("nfs.exportfs", "true"),
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
        diagnostic.action_id == "exports:/srv/old:unexport"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::NfsUnexportRequired
    }));
}

#[test]
fn topology_comparison_reports_luks_format_target_metadata() {
    let plan = plan_from_json_bytes(
        br#"{
              "luks": {
                "devices": {
                  "cryptroot": {
                    "operation": "format",
                    "device": "/dev/disk/by-partuuid/root",
                    "target": "cryptroot"
                  },
                  "cryptdata": {
                    "operation": "format",
                    "device": "/dev/disk/by-id/data",
                    "target": "cryptdata"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "block:/dev/disk/by-partuuid/root",
            NodeKind::LuksContainer,
            "/dev/disk/by-partuuid/root",
        )
        .with_path("/dev/disk/by-partuuid/root")
        .with_property("cryptsetup.luks-version", "2")
        .with_property("cryptsetup.uuid", "11111111-2222-3333-4444-555555555555")
        .with_property("cryptsetup.luks-keyslot-count", "2")
        .with_property("cryptsetup.luks-token-count", "1")
        .with_property("cryptsetup.active", "false"),
    );
    graph.add_node(
        Node::new(
            "block:/dev/disk/by-id/data",
            NodeKind::Partition,
            "/dev/disk/by-id/data",
        )
        .with_path("/dev/disk/by-id/data"),
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
    assert_eq!(plan.summary.action_count, 2);
    assert!(plan.actions.iter().any(|action| {
        action.id == "luks.devices:cryptroot:format" && action.operation == Operation::Format
    }));
    assert!(plan.actions.iter().any(|action| {
        action.id == "luks.devices:cryptdata:format" && action.operation == Operation::Format
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "luks.devices:cryptroot:format"
            && diagnostic.query == "/dev/disk/by-partuuid/root"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LuksFormatTargetPresent
            && diagnostic.message.contains("version 2")
            && diagnostic.message.contains("keyslots 2")
            && diagnostic.message.contains("tokens 1")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "luks.devices:cryptdata:format"
            && diagnostic.query == "/dev/disk/by-id/data"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LuksFormatTargetPresent
            && diagnostic.message.contains("partition")
    }));
}

#[test]
fn topology_comparison_suppresses_open_luks_mapper_when_active() {
    let plan = plan_from_json_bytes(
        br#"{
              "luks": {
                "devices": {
                  "cryptroot": {
                    "operation": "open",
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
        Node::new(
            "block:/dev/mapper/cryptroot",
            NodeKind::LuksContainer,
            "cryptroot",
        )
        .with_path("/dev/mapper/cryptroot")
        .with_property("cryptsetup.active", "true")
        .with_property("cryptsetup.in-use", "true"),
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
        diagnostic.action_id == "luks.devices:cryptroot:open"
            && diagnostic.kind == TopologyDiagnosticKind::LuksOpenAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_open_luks_mapper_when_inactive() {
    let plan = plan_from_json_bytes(
        br#"{
              "luks": {
                "devices": {
                  "cryptroot": {
                    "operation": "open",
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
        Node::new(
            "block:/dev/mapper/cryptroot",
            NodeKind::LuksContainer,
            "cryptroot",
        )
        .with_path("/dev/mapper/cryptroot")
        .with_property("cryptsetup.active", "false"),
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
        diagnostic.action_id == "luks.devices:cryptroot:open"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LuksOpenRequired
    }));
}

#[test]
fn topology_comparison_reconciles_absent_luks_open_and_close() {
    let plan = plan_from_json_bytes(
        br#"{
              "luks": {
                "devices": {
                  "cryptroot": {
                    "operation": "open",
                    "device": "/dev/disk/by-partuuid/root",
                    "target": "cryptroot"
                  },
                  "cryptold": {
                    "operation": "close",
                    "target": "cryptold"
                  }
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
        .any(|action| action.id == "luks.devices:cryptroot:open"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "luks.devices:cryptroot:open"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LuksOpenRequired
            && diagnostic.message.contains("/dev/disk/by-partuuid/root")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "luks.devices:cryptold:close"
            && diagnostic.level == TopologyDiagnosticLevel::Info
            && diagnostic.kind == TopologyDiagnosticKind::LuksCloseAlreadySatisfied
            && diagnostic.current.is_none()
    }));
}

#[test]
fn topology_comparison_suppresses_close_luks_mapper_when_inactive() {
    let plan = plan_from_json_bytes(
        br#"{
              "luks": {
                "devices": {
                  "cryptroot": {
                    "operation": "close",
                    "target": "cryptroot"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "block:/dev/mapper/cryptroot",
            NodeKind::LuksContainer,
            "cryptroot",
        )
        .with_path("/dev/mapper/cryptroot")
        .with_property("cryptsetup.active", "false"),
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
        diagnostic.action_id == "luks.devices:cryptroot:close"
            && diagnostic.kind == TopologyDiagnosticKind::LuksCloseAlreadySatisfied
    }));
}

#[test]
fn topology_comparison_keeps_close_luks_mapper_when_active() {
    let plan = plan_from_json_bytes(
        br#"{
              "luks": {
                "devices": {
                  "cryptroot": {
                    "operation": "close",
                    "target": "cryptroot"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "block:/dev/mapper/cryptroot",
            NodeKind::LuksContainer,
            "cryptroot",
        )
        .with_path("/dev/mapper/cryptroot")
        .with_property("cryptsetup.active", "true"),
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
        diagnostic.action_id == "luks.devices:cryptroot:close"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LuksCloseRequired
    }));
}

#[test]
fn topology_comparison_suppresses_luks_keyslot_remove_when_slot_absent() {
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
        .with_property("cryptsetup.luks-keyslots", "0,1")
        .with_property("cryptsetup.luks-keyslot-count", "2"),
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
        diagnostic.action_id == "lukskeyslots:cryptroot:2:remove-key"
            && diagnostic.kind == TopologyDiagnosticKind::LuksKeyslotRemoveAlreadySatisfied
            && diagnostic.query == "/dev/disk/by-id/root-luks"
    }));
}

#[test]
fn topology_comparison_reconciles_luks_keyslot_priority_properties() {
    let plan = plan_from_json_bytes(
        br#"{
              "luksKeyslots": {
                "cryptroot:1": {
                  "properties": {
                    "priority": "prefer"
                  },
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "keySlot": "1"
                  }
                },
                "cryptroot:2": {
                  "properties": {
                    "priority": "ignore"
                  },
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "keySlot": "2"
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
        .with_property("cryptsetup.luks-keyslots", "1,2")
        .with_property("cryptsetup.luks-keyslot-1-priority", "prefer")
        .with_property("cryptsetup.luks-keyslot-2-priority", "normal"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    assert_eq!(comparison.summary.action_count, 2);
    assert_eq!(comparison.summary.already_satisfied_count, 1);
    assert_eq!(comparison.summary.suppressed_action_count, 1);
    assert_eq!(plan.actions.len(), 1);
    assert!(plan
        .actions
        .iter()
        .any(|action| action.id == "luksKeyslots:cryptroot:2:set-property:priority"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "luksKeyslots:cryptroot:1:set-property:priority"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "luksKeyslots:cryptroot:2:set-property:priority"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyDiffers
            && diagnostic.message.contains("normal")
            && diagnostic.message.contains("ignore")
    }));
}

#[test]
fn topology_comparison_keeps_luks_keyslot_remove_when_slot_present() {
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
        .with_property("cryptsetup.luks-keyslots", "0,2")
        .with_property("cryptsetup.luks-keyslot-2-type", "luks2")
        .with_property("cryptsetup.luks-keyslot-2-priority", "normal")
        .with_property("cryptsetup.luks-keyslot-2-pbkdf", "argon2id")
        .with_property("cryptsetup.luks-keyslot-2-time-cost", "4"),
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
        diagnostic.action_id == "lukskeyslots:cryptroot:2:remove-key"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::LuksKeyslotRemoveRequired
            && diagnostic.message.contains("type luks2")
            && diagnostic.message.contains("priority normal")
            && diagnostic.message.contains("PBKDF argon2id")
    }));
}

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

#[test]
fn topology_comparison_suppresses_btrfs_snapshot_destroy_when_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "/mnt/persist/@home-old": {
                  "target": "/mnt/persist/@home",
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
        diagnostic.action_id == "snapshot:/mnt/persist/@home-old:destroy"
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotDestroyAlreadySatisfied
            && diagnostic.query == "/mnt/persist/@home-old"
    }));
}

#[test]
fn topology_comparison_keeps_btrfs_snapshot_destroy_when_present() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "/mnt/persist/@home-old": {
                  "target": "/mnt/persist/@home",
                  "destroy": true
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "btrfs-snapshot:fs-uuid:@home-old",
            NodeKind::BtrfsSnapshot,
            "@home-old",
        )
        .with_path("/mnt/persist/@home-old")
        .with_property("btrfs.id", "258")
        .with_property("btrfs.generation", "120")
        .with_property("btrfs.parent-uuid", "source-uuid"),
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
        diagnostic.action_id == "snapshot:/mnt/persist/@home-old:destroy"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotDestroyRequired
            && diagnostic.query == "/mnt/persist/@home-old"
            && diagnostic.message.contains("Btrfs snapshot")
            && diagnostic.message.contains("subvolume id 258")
            && diagnostic.message.contains("generation 120")
            && diagnostic.message.contains("parent UUID source-uuid")
    }));
}

#[test]
fn topology_comparison_keeps_logical_snapshot_destroy_missing() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "old-home": {
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

    assert_eq!(comparison.summary.already_satisfied_count, 0);
    assert_eq!(comparison.summary.suppressed_action_count, 0);
    assert_eq!(comparison.summary.missing_count, 1);
    assert_eq!(plan.actions.len(), 1);
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "snapshot:old-home:destroy"
            && diagnostic.kind == TopologyDiagnosticKind::Missing
            && diagnostic.query == "old-home"
    }));
}

#[test]
fn topology_comparison_warns_when_zfs_rollback_snapshot_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "tank/home@before": {
                  "target": "tank/home",
                  "rollback": true
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(Node::new(
        "zfs-dataset:tank/home",
        NodeKind::ZfsDataset,
        "tank/home",
    ));

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
        diagnostic.action_id == "snapshot:tank/home@before:rollback"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotRollbackPointMissing
            && diagnostic.query == "tank/home@before"
            && diagnostic.current.is_none()
    }));
}

#[test]
fn topology_comparison_warns_when_zfs_rollback_snapshot_present() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "tank/home@before": {
                  "target": "tank/home",
                  "rollback": true,
                  "recursiveRollback": true
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "zfs-snapshot:tank/home@before",
            NodeKind::ZfsSnapshot,
            "tank/home@before",
        )
        .with_property("zfs.used", "64M")
        .with_property("zfs.referenced", "5G")
        .with_property("zfs.userrefs", "1")
        .with_property("zfs.compression", "lz4"),
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
        diagnostic.action_id == "snapshot:tank/home@before:rollback"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotRollbackPointAvailable
            && diagnostic.query == "tank/home@before"
            && diagnostic.message.contains("used 64M")
            && diagnostic.message.contains("referenced 5G")
            && diagnostic.message.contains("user references 1")
            && diagnostic.message.contains("recursive rollback requested")
    }));
}

#[test]
fn topology_comparison_keeps_logical_snapshot_rollback_missing() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "before": {
                  "target": "tank/home",
                  "rollback": true
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
        diagnostic.action_id == "snapshot:before:rollback"
            && diagnostic.kind == TopologyDiagnosticKind::Missing
            && diagnostic.query == "before"
    }));
}

#[test]
fn topology_comparison_warns_when_zfs_snapshot_clone_source_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "tank/home@before": {
                  "target": "tank/home",
                  "cloneTo": "tank/home-review"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(Node::new(
        "zfs-dataset:tank/home-review",
        NodeKind::ZfsDataset,
        "tank/home-review",
    ));

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
        diagnostic.action_id == "snapshot:tank/home@before:clone:tank/home-review"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotCloneSourceMissing
            && diagnostic.query == "tank/home@before"
            && diagnostic.current.is_none()
    }));
}

#[test]
fn topology_comparison_reports_zfs_snapshot_clone_source_available() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "tank/home@before": {
                  "target": "tank/home",
                  "cloneTo": "tank/home-review"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "zfs-snapshot:tank/home@before",
            NodeKind::ZfsSnapshot,
            "tank/home@before",
        )
        .with_property("zfs.used", "8M")
        .with_property("zfs.referenced", "4G")
        .with_property("zfs.userrefs", "1"),
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
        diagnostic.action_id == "snapshot:tank/home@before:clone:tank/home-review"
            && diagnostic.level == TopologyDiagnosticLevel::Info
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotCloneSourceAvailable
            && diagnostic.query == "tank/home@before"
            && diagnostic.message.contains("clone target tank/home-review")
            && diagnostic.message.contains("used 8M")
            && diagnostic.message.contains("user references 1")
    }));
}

#[test]
fn topology_comparison_warns_when_btrfs_snapshot_clone_source_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "/mnt/persist/@home-before": {
                  "target": "/mnt/persist/@home",
                  "cloneTo": "/mnt/persist/@home-review"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "btrfs-subvolume:fs-uuid:@home-review",
            NodeKind::BtrfsSubvolume,
            "@home-review",
        )
        .with_path("/mnt/persist/@home-review"),
    );

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
        diagnostic.action_id == "snapshot:/mnt/persist/@home-before:clone:/mnt/persist/@home-review"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotCloneSourceMissing
            && diagnostic.query == "/mnt/persist/@home-before"
            && diagnostic.current.is_none()
    }));
}

#[test]
fn topology_comparison_reports_btrfs_snapshot_clone_source_available() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "/mnt/persist/@home-before": {
                  "target": "/mnt/persist/@home",
                  "cloneTo": "/mnt/persist/@home-review"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "btrfs-snapshot:fs-uuid:@home-before",
            NodeKind::BtrfsSnapshot,
            "@home-before",
        )
        .with_path("/mnt/persist/@home-before")
        .with_property("btrfs.id", "300")
        .with_property("btrfs.parent-uuid", "source-uuid"),
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
        diagnostic.action_id == "snapshot:/mnt/persist/@home-before:clone:/mnt/persist/@home-review"
            && diagnostic.level == TopologyDiagnosticLevel::Info
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotCloneSourceAvailable
            && diagnostic.query == "/mnt/persist/@home-before"
            && diagnostic
                .message
                .contains("clone target /mnt/persist/@home-review")
            && diagnostic.message.contains("subvolume id 300")
            && diagnostic.message.contains("parent UUID source-uuid")
    }));
}

#[test]
fn topology_comparison_uses_snapshot_path_for_friendly_btrfs_clone() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "before-home": {
                  "target": "/mnt/persist/@home",
                  "snapshotPath": "/mnt/persist/@home-before",
                  "cloneTo": "/mnt/persist/@home-review"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "btrfs-snapshot:fs-uuid:@home-before",
            NodeKind::BtrfsSnapshot,
            "@home-before",
        )
        .with_path("/mnt/persist/@home-before")
        .with_property("btrfs.id", "300"),
    );

    let plan = compare_plan_with_topology(plan, &graph);
    let comparison = plan
        .topology_comparison
        .as_ref()
        .expect("comparison should be present");

    let action = plan
        .actions
        .iter()
        .find(|action| action.id == "snapshot:before-home:clone:/mnt/persist/@home-review")
        .expect("friendly clone action should remain actionable");
    assert_eq!(
        action.context.snapshot_path.as_deref(),
        Some("/mnt/persist/@home-before")
    );
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "snapshot:before-home:clone:/mnt/persist/@home-review"
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotCloneSourceAvailable
            && diagnostic.query == "/mnt/persist/@home-before"
    }));
}

#[test]
fn topology_comparison_warns_when_zfs_snapshot_rename_source_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "tank/home@old": {
                  "target": "tank/home",
                  "renameTo": "tank/home@kept"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(Node::new(
        "zfs-dataset:tank/home",
        NodeKind::ZfsDataset,
        "tank/home",
    ));

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
        diagnostic.action_id == "snapshot:tank/home@old:rename:tank/home@kept"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotRenameSourceMissing
            && diagnostic.query == "tank/home@old"
            && diagnostic.current.is_none()
    }));
}

#[test]
fn topology_comparison_warns_when_zfs_snapshot_rename_source_present() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "tank/home@old": {
                  "target": "tank/home",
                  "renameTo": "tank/home@kept"
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
        .with_property("zfs.used", "12M")
        .with_property("zfs.referenced", "2G")
        .with_property("zfs.userrefs", "3"),
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
        diagnostic.action_id == "snapshot:tank/home@old:rename:tank/home@kept"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotRenameRequired
            && diagnostic.query == "tank/home@old"
            && diagnostic.message.contains("rename to tank/home@kept")
            && diagnostic.message.contains("used 12M")
            && diagnostic.message.contains("user references 3")
    }));
}

#[test]
fn topology_comparison_warns_when_btrfs_snapshot_rename_source_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "/mnt/persist/@home-old": {
                  "target": "/mnt/persist/@home",
                  "renameTo": "/mnt/persist/@home-kept"
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
        .with_path("/mnt/persist/@home"),
    );

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
        diagnostic.action_id == "snapshot:/mnt/persist/@home-old:rename:/mnt/persist/@home-kept"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotRenameSourceMissing
            && diagnostic.query == "/mnt/persist/@home-old"
            && diagnostic.current.is_none()
    }));
}

#[test]
fn topology_comparison_warns_when_btrfs_snapshot_rename_source_present() {
    let plan = plan_from_json_bytes(
        br#"{
              "snapshots": {
                "/mnt/persist/@home-old": {
                  "target": "/mnt/persist/@home",
                  "renameTo": "/mnt/persist/@home-kept"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "btrfs-snapshot:fs-uuid:@home-old",
            NodeKind::BtrfsSnapshot,
            "@home-old",
        )
        .with_path("/mnt/persist/@home-old")
        .with_property("btrfs.id", "258")
        .with_property("btrfs.parent-uuid", "source-uuid"),
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
        diagnostic.action_id == "snapshot:/mnt/persist/@home-old:rename:/mnt/persist/@home-kept"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::SnapshotRenameRequired
            && diagnostic.query == "/mnt/persist/@home-old"
            && diagnostic
                .message
                .contains("rename to /mnt/persist/@home-kept")
            && diagnostic.message.contains("subvolume id 258")
            && diagnostic.message.contains("parent UUID source-uuid")
    }));
}

#[test]
fn topology_comparison_suppresses_btrfs_qgroup_destroy_when_absent() {
    let plan = plan_from_json_bytes(
        br#"{
              "btrfsQgroups": {
                "0/257": {
                  "destroy": true,
                  "target": "/mnt/persist"
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
        diagnostic.action_id == "btrfsqgroups:0/257:destroy"
            && diagnostic.kind == TopologyDiagnosticKind::BtrfsQgroupDestroyAlreadySatisfied
            && diagnostic.query == "0/257"
    }));
}

#[test]
fn topology_comparison_reconciles_btrfs_qgroup_create() {
    let plan = plan_from_json_bytes(
        br#"{
              "btrfsQgroups": {
                "0/257": {
                  "operation": "create",
                  "target": "/mnt/persist"
                },
                "0/258": {
                  "operation": "create",
                  "target": "/mnt/persist"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("btrfs-qgroup:fs-uuid:0/257", NodeKind::BtrfsQgroup, "0/257")
            .with_property("btrfs.qgroup-id", "0/257")
            .with_property("btrfs.max-referenced", "21474836480")
            .with_property("btrfs.max-exclusive", "none")
            .with_property("btrfs.qgroup-parents", "1/0")
            .with_usage(disk_nix_model::Usage {
                used_bytes: Some(10_737_418_240),
                free_bytes: None,
                allocated_bytes: Some(2_147_483_648),
            }),
    );
    graph.add_node(
        Node::new("mount:/mnt/persist/0/258", NodeKind::Mountpoint, "0/258")
            .with_path("/mnt/persist/0/258"),
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
        .all(|action| action.id == "btrfsqgroups:0/258:create"));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "btrfsqgroups:0/257:create"
            && diagnostic.kind == TopologyDiagnosticKind::BtrfsQgroupCreateAlreadySatisfied
            && diagnostic.message.contains("qgroup id 0/257")
            && diagnostic.message.contains("max referenced 21474836480")
            && diagnostic.message.contains("referenced 10737418240 bytes")
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "btrfsqgroups:0/258:create"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::BtrfsQgroupCreateRequired
            && diagnostic.message.contains("not a Btrfs qgroup")
    }));
}

#[test]
fn topology_comparison_reconciles_btrfs_qgroup_property_aliases() {
    let plan = plan_from_json_bytes(
        br#"{
              "btrfsQgroups": {
                "0/257": {
                  "target": "/mnt/persist",
                  "properties": {
                    "limit": "21474836480",
                    "maxExclusive": "unlimited"
                  }
                },
                "0/258": {
                  "target": "/mnt/persist",
                  "properties": {
                    "btrfs.max-exclusive": "10737418240"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("btrfs-qgroup:fs-uuid:0/257", NodeKind::BtrfsQgroup, "0/257")
            .with_property("btrfs.qgroup-id", "0/257")
            .with_property("btrfs.max-referenced", "21474836480")
            .with_property("btrfs.max-exclusive", "none"),
    );
    graph.add_node(
        Node::new("btrfs-qgroup:fs-uuid:0/258", NodeKind::BtrfsQgroup, "0/258")
            .with_property("btrfs.qgroup-id", "0/258")
            .with_property("btrfs.max-exclusive", "5368709120"),
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
    assert_eq!(plan.actions.len(), 1);
    assert!(plan.actions.iter().any(|action| {
        action.id == "btrfsQgroups:0/258:set-property:btrfs.max-exclusive"
            && action.operation == Operation::SetProperty
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "btrfsQgroups:0/257:set-property:limit"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "btrfsQgroups:0/257:set-property:maxExclusive"
            && diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
    }));
    assert!(comparison.diagnostics.iter().any(|diagnostic| {
        diagnostic.action_id == "btrfsQgroups:0/258:set-property:btrfs.max-exclusive"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::PropertyDiffers
            && diagnostic.message.contains("5368709120")
            && diagnostic.message.contains("10737418240")
    }));
}

#[test]
fn topology_comparison_keeps_btrfs_qgroup_destroy_when_present() {
    let plan = plan_from_json_bytes(
        br#"{
              "btrfsQgroups": {
                "0/257": {
                  "destroy": true,
                  "target": "/mnt/persist"
                }
              }
            }"#,
    )
    .expect("plan should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("btrfs-qgroup:fs-uuid:0/257", NodeKind::BtrfsQgroup, "0/257")
            .with_property("btrfs.qgroup-id", "0/257")
            .with_property("btrfs.max-referenced", "21474836480")
            .with_property("btrfs.max-exclusive", "none")
            .with_property("btrfs.qgroup-parents", "1/0")
            .with_usage(disk_nix_model::Usage {
                used_bytes: Some(10_737_418_240),
                free_bytes: None,
                allocated_bytes: Some(2_147_483_648),
            }),
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
        diagnostic.action_id == "btrfsqgroups:0/257:destroy"
            && diagnostic.level == TopologyDiagnosticLevel::Warning
            && diagnostic.kind == TopologyDiagnosticKind::BtrfsQgroupDestroyRequired
            && diagnostic.query == "0/257"
            && diagnostic.message.contains("qgroup id 0/257")
            && diagnostic.message.contains("max referenced 21474836480")
            && diagnostic.message.contains("parents 1/0")
            && diagnostic.message.contains("referenced 10737418240 bytes")
    }));
}

#[test]
fn topology_comparison_keeps_logical_btrfs_qgroup_destroy_missing() {
    let plan = plan_from_json_bytes(
        br#"{
              "btrfsQgroups": {
                "old-qgroup": {
                  "destroy": true,
                  "target": "/mnt/persist"
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
        diagnostic.action_id == "btrfsqgroups:old-qgroup:destroy"
            && diagnostic.kind == TopologyDiagnosticKind::Missing
            && diagnostic.query == "old-qgroup"
    }));
}

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

#[test]
fn plan_classifies_iscsi_session_login_and_logout() {
    let plan = plan_from_json_bytes(
        br#"{
              "iscsiSessions": {
                "iqn.2026-06.example:storage.root": {
                  "operation": "login",
                  "metadata": {
                    "portal": "192.0.2.10:3260"
                  }
                },
                "iqn.2026-06.example:storage.old": {
                  "operation": "logout",
                  "portal": "192.0.2.11:3260"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 2);
    assert_eq!(plan.summary.offline_required_count, 1);
    assert_eq!(plan.summary.destructive_count, 0);
    let create = plan
        .actions
        .iter()
        .find(|action| action.id == "iscsisessions:iqn.2026-06.example:storage.root:login")
        .expect("iSCSI login action exists");
    assert_eq!(create.operation, Operation::Login);
    assert_eq!(create.risk, RiskClass::Online);
    assert_eq!(create.context.portal.as_deref(), Some("192.0.2.10:3260"));

    let destroy = plan
        .actions
        .iter()
        .find(|action| action.id == "iscsisessions:iqn.2026-06.example:storage.old:logout")
        .expect("iSCSI logout action exists");
    assert_eq!(destroy.operation, Operation::Logout);
    assert_eq!(destroy.risk, RiskClass::OfflineRequired);
    assert_eq!(destroy.context.portal.as_deref(), Some("192.0.2.11:3260"));
}

#[test]
fn plan_classifies_nfs_export_lifecycle_without_data_destruction() {
    let plan = plan_from_json_bytes(
        br#"{
              "exports": {
                "/srv/share": {
                  "operation": "export",
                  "client": "192.0.2.0/24",
                  "options": "rw,sync,no_subtree_check"
                },
                "/srv/inventory": {
                  "operation": "rescan"
                },
                "/srv/old": {
                  "operation": "unexport",
                  "client": "192.0.2.55"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 3);
    assert_eq!(plan.summary.offline_required_count, 1);
    assert_eq!(plan.summary.destructive_count, 0);
    let create = plan
        .actions
        .iter()
        .find(|action| action.id == "exports:/srv/share:export")
        .expect("export action exists");
    assert_eq!(create.operation, Operation::Export);
    assert_eq!(create.risk, RiskClass::Online);
    assert_eq!(create.context.client.as_deref(), Some("192.0.2.0/24"));
    assert_eq!(
        create.context.options.as_deref(),
        Some("rw,sync,no_subtree_check")
    );
    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "exports:/srv/inventory:rescan")
        .expect("export rescan action exists");
    assert_eq!(rescan.operation, Operation::Rescan);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert_eq!(rescan.context.target.as_deref(), Some("/srv/inventory"));
    let destroy = plan
        .actions
        .iter()
        .find(|action| action.id == "exports:/srv/old:unexport")
        .expect("unexport action exists");
    assert_eq!(destroy.operation, Operation::Unexport);
    assert_eq!(destroy.risk, RiskClass::OfflineRequired);
    assert!(!destroy.destructive);
}

#[test]
fn plan_classifies_nfs_mount_lifecycle_without_remote_data_destruction() {
    let plan = plan_from_json_bytes(
        br#"{
              "nfs": {
                "mounts": {
                  "/srv/shared": {
                    "operation": "mount",
                    "source": "nas.example.com:/srv/shared",
                    "fsType": "nfs4",
                    "options": ["_netdev", "vers=4.2"]
                  },
                  "/srv/old": {
                    "operation": "unmount",
                    "source": "nas.example.com:/srv/old"
                  },
                  "/srv/tuned": {
                    "operation": "remount",
                    "source": "nas.example.com:/srv/tuned",
                    "options": ["_netdev", "ro", "vers=4.2"]
                  },
                  "/srv/inventory": {
                    "operation": "rescan",
                    "source": "nas.example.com:/srv/inventory"
                  }
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
        .find(|action| action.id == "nfs.mounts:/srv/shared:mount")
        .expect("NFS mount action exists");
    assert_eq!(create.operation, Operation::Mount);
    assert_eq!(create.risk, RiskClass::Online);
    assert_eq!(
        create.context.device.as_deref(),
        Some("nas.example.com:/srv/shared")
    );
    assert_eq!(create.context.mountpoint.as_deref(), Some("/srv/shared"));
    assert_eq!(create.context.fs_type.as_deref(), Some("nfs4"));
    assert_eq!(create.context.options.as_deref(), Some("_netdev,vers=4.2"));

    let destroy = plan
        .actions
        .iter()
        .find(|action| action.id == "nfs.mounts:/srv/old:unmount")
        .expect("NFS unmount action exists");
    assert_eq!(destroy.operation, Operation::Unmount);
    assert_eq!(destroy.risk, RiskClass::OfflineRequired);
    assert!(!destroy.destructive);
    assert_eq!(destroy.context.mountpoint.as_deref(), Some("/srv/old"));

    let remount = plan
        .actions
        .iter()
        .find(|action| action.id == "nfs.mounts:/srv/tuned:remount")
        .expect("NFS mount remount exists");
    assert_eq!(remount.operation, Operation::Remount);
    assert_eq!(remount.risk, RiskClass::Online);
    assert_eq!(remount.context.mountpoint.as_deref(), Some("/srv/tuned"));
    assert_eq!(
        remount.context.options.as_deref(),
        Some("_netdev,ro,vers=4.2")
    );

    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "nfs.mounts:/srv/inventory:rescan")
        .expect("NFS mount rescan exists");
    assert_eq!(rescan.operation, Operation::Rescan);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert_eq!(rescan.context.mountpoint.as_deref(), Some("/srv/inventory"));
}

#[test]
fn plan_classifies_cache_replacement_as_offline_required() {
    let plan = plan_from_json_bytes(
        br#"{
              "caches": {
                "vg0/root-cache": {
                  "operation": "replace-device",
                  "removeDevices": ["/dev/sdd"],
                  "replaceDevices": {
                    "/dev/sdb": "/dev/sdc"
                  }
                },
                "/dev/bcache0": {
                  "operation": "rescan"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 4);
    assert_eq!(plan.summary.offline_required_count, 3);
    assert!(plan
        .actions
        .iter()
        .filter(|action| action.operation == Operation::ReplaceDevice)
        .all(|action| {
            action.operation == Operation::ReplaceDevice
                && action.risk == RiskClass::OfflineRequired
                && action.advice.as_ref().is_some_and(|advice| {
                    advice
                        .alternatives
                        .iter()
                        .any(|alternative| alternative.contains("flush dirty data"))
                })
        }));
    let detach = plan
        .actions
        .iter()
        .find(|action| action.id == "caches:vg0/root-cache:remove-device:/dev/sdd")
        .expect("cache detach action exists");
    assert_eq!(detach.operation, Operation::RemoveDevice);
    assert_eq!(detach.risk, RiskClass::OfflineRequired);
    assert!(detach.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("dirty data"))
    }));
    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "caches:/dev/bcache0:rescan")
        .expect("cache rescan action exists");
    assert_eq!(rescan.operation, Operation::Rescan);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert!(!rescan.destructive);
    assert!(rescan.advice.as_ref().is_some_and(|advice| {
        advice
            .summary
            .contains("bcache rescan refreshes cache state")
    }));
}

#[test]
fn plan_classifies_lvm_cache_attach_and_detach() {
    let plan = plan_from_json_bytes(
        br#"{
              "lvmCaches": {
                "vg0/root": {
                  "operation": "create",
                  "device": "vg0/root-cache",
                  "addDevices": ["vg0/root-cache"],
                  "removeDevices": ["vg0/root-cache"],
                  "properties": {
                    "lvm.cache-mode": "writethrough"
                  }
                },
                "vg0/archive": {
                  "operation": "rescan"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    let create = plan
        .actions
        .iter()
        .find(|action| action.id == "lvmcaches:vg0/root:create")
        .expect("LVM cache create action exists");
    let add = plan
        .actions
        .iter()
        .find(|action| action.id == "lvmCaches:vg0/root:add-device:vg0/root-cache")
        .expect("LVM cache add action exists");
    let remove = plan
        .actions
        .iter()
        .find(|action| action.id == "lvmCaches:vg0/root:remove-device:vg0/root-cache")
        .expect("LVM cache remove action exists");
    let property = plan
        .actions
        .iter()
        .find(|action| action.id == "lvmCaches:vg0/root:set-property:lvm.cache-mode")
        .expect("LVM cache property action exists");
    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "lvmcaches:vg0/archive:rescan")
        .expect("LVM cache rescan action exists");

    assert_eq!(plan.summary.action_count, 5);
    assert_eq!(plan.summary.offline_required_count, 3);
    assert_eq!(create.risk, RiskClass::OfflineRequired);
    assert_eq!(add.risk, RiskClass::OfflineRequired);
    assert_eq!(remove.risk, RiskClass::OfflineRequired);
    assert_eq!(property.risk, RiskClass::Safe);
    assert_eq!(rescan.operation, Operation::Rescan);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert!(!rescan.destructive);
    assert!(remove.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("dirty data"))
    }));
}

#[test]
fn apply_policy_blocks_destructive_and_potential_data_loss_actions() {
    let (plan, mut policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "datasets": {
                  "tank/old": { "destroy": true }
                },
                "pools": {
                  "tank": { "removeDevices": ["/dev/sdb"] }
                }
              },
              "apply": {
                "mode": "manual",
                "allowDestructive": false,
                "allowFormat": false,
                "allowShrink": false,
                "allowPotentialDataLoss": false,
                "allowGrow": true,
                "allowOffline": false,
                "allowPropertyChanges": true
              }
            }"#,
    )
    .expect("document should parse");

    let report = evaluate_apply_policy(&plan, policy.clone());

    assert_eq!(report.blocked_count, 2);
    assert_eq!(report.blocked_summary.destructive_count, 1);
    assert_eq!(report.blocked_summary.potential_data_loss_count, 1);
    assert!(report.blocked.iter().any(|blocked| {
        blocked.reason == "potential-data-loss actions require allowPotentialDataLoss=true"
    }));
    assert!(!report.can_execute());

    policy.allow_destructive = true;
    policy.allow_potential_data_loss = true;
    let report = evaluate_apply_policy(&plan, policy);
    assert_eq!(report.blocked_count, 0);
    assert!(report.can_execute());
}

#[test]
fn apply_policy_requires_backup_and_confirmation_for_allowed_potential_data_loss() {
    let (plan, mut policy) = plan_and_policy_from_json_bytes(
        br#"{
              "filesystems": {
                "home": {
                  "mountpoint": "/home",
                  "fsType": "ext4",
                  "resizePolicy": "shrink-allowed"
                }
              },
              "apply": {
                "allowShrink": true,
                "allowPotentialDataLoss": true,
                "requireBackup": true,
                "backupVerified": false,
                "requireConfirmation": true,
                "confirmation": false
              }
            }"#,
    )
    .expect("document should parse");

    let report = evaluate_apply_policy(&plan, policy.clone());
    assert_eq!(report.blocked_count, 1);
    assert_eq!(
        report.blocked[0].reason,
        "backup-required actions require backupVerified=true"
    );

    policy.backup_verified = true;
    let report = evaluate_apply_policy(&plan, policy.clone());
    assert_eq!(report.blocked_count, 1);
    assert_eq!(
        report.blocked[0].reason,
        "confirmation-required actions require confirmation=true"
    );

    policy.confirmation = true;
    let report = evaluate_apply_policy(&plan, policy);
    assert_eq!(report.blocked_count, 0);
    assert!(report.can_execute());
}

#[test]
fn apply_policy_blocks_unsupported_actions_even_when_permissive() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "filesystems": {
                "archive": {
                  "mountpoint": "/archive",
                  "fsType": "xfs",
                  "resizePolicy": "shrink-allowed"
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowFormat": true,
                "allowShrink": true,
                "allowGrow": true,
                "allowOffline": true,
                "allowPropertyChanges": true
              }
            }"#,
    )
    .expect("document should parse");

    let report = evaluate_apply_policy(&plan, policy);

    assert_eq!(report.blocked_count, 1);
    assert_eq!(report.blocked_summary.unsupported_count, 1);
    assert_eq!(report.blocked[0].risk, RiskClass::Unsupported);
    assert_eq!(
        report.blocked[0].reason,
        "unsupported actions cannot be applied"
    );
}

#[test]
fn apply_policy_allows_grow_when_enabled() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "volumes": {
                  "vg/root": { "operation": "grow" }
                }
              },
              "apply": {
                "allowGrow": true
              }
            }"#,
    )
    .expect("document should parse");

    let report = evaluate_apply_policy(&plan, policy);

    assert_eq!(report.blocked_count, 0);
    assert!(report.can_execute());
}

#[test]
fn apply_policy_requires_offline_permission_for_offline_required_actions() {
    let (plan, mut policy) = plan_and_policy_from_json_bytes(
        br#"{
              "luns": {
                "iqn.2026-06.example:storage/root:0": {
                  "operation": "grow"
                }
              },
              "apply": {
                "allowGrow": true,
                "allowOffline": false
              }
            }"#,
    )
    .expect("document should parse");

    let report = evaluate_apply_policy(&plan, policy.clone());
    assert_eq!(report.blocked_count, 1);
    assert_eq!(report.blocked_summary.offline_required_count, 1);
    assert_eq!(report.blocked[0].risk, RiskClass::OfflineRequired);
    assert_eq!(
        report.blocked[0].reason,
        "offline-required actions require allowOffline=true"
    );

    policy.allow_offline = true;
    let report = evaluate_apply_policy(&plan, policy);
    assert_eq!(report.blocked_count, 0);
    assert!(report.can_execute());
}

#[test]
fn apply_policy_requires_format_and_destructive_permission_for_format() {
    let (plan, mut policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "scratch": {
                    "mountpoint": "/scratch",
                    "fsType": "xfs",
                    "preserveData": false
                  }
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowFormat": false
              }
            }"#,
    )
    .expect("document should parse");

    let report = evaluate_apply_policy(&plan, policy.clone());
    assert!(report
        .blocked
        .iter()
        .any(|blocked| blocked.reason == "format actions require allowFormat=true"));

    policy.allow_format = true;
    let report = evaluate_apply_policy(&plan, policy);
    assert_eq!(report.blocked_count, 0);
}

#[test]
fn apply_policy_can_require_verified_backup_for_high_risk_actions() {
    let (plan, mut policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "datasets": {
                  "tank/old": { "destroy": true }
                }
              },
              "apply": {
                "allowDestructive": true,
                "requireBackup": true,
                "backupVerified": false
              }
            }"#,
    )
    .expect("document should parse");

    let report = evaluate_apply_policy(&plan, policy.clone());
    assert_eq!(report.blocked_count, 1);
    assert_eq!(
        report.blocked[0].reason,
        "backup-required actions require backupVerified=true"
    );

    policy.backup_verified = true;
    let report = evaluate_apply_policy(&plan, policy);
    assert_eq!(report.blocked_count, 0);
}

#[test]
fn apply_policy_can_require_confirmation_for_offline_actions() {
    let (plan, mut policy) = plan_and_policy_from_json_bytes(
        br#"{
              "luns": {
                "iqn.2026-06.example:storage/root:0": {
                  "operation": "grow"
                }
              },
              "apply": {
                "allowGrow": true,
                "allowOffline": true,
                "requireConfirmation": true,
                "confirmation": false
              }
            }"#,
    )
    .expect("document should parse");

    let report = evaluate_apply_policy(&plan, policy.clone());
    assert_eq!(report.blocked_count, 1);
    assert_eq!(
        report.blocked[0].reason,
        "confirmation-required actions require confirmation=true"
    );

    policy.confirmation = true;
    let report = evaluate_apply_policy(&plan, policy);
    assert_eq!(report.blocked_count, 0);
}

#[test]
fn apply_policy_can_require_confirmation_file_for_offline_actions() {
    let (plan, mut policy) = plan_and_policy_from_json_bytes(
        br#"{
              "luns": {
                "iqn.2026-06.example:storage/root:0": {
                  "operation": "grow"
                }
              },
              "apply": {
                "allowGrow": true,
                "allowOffline": true,
                "requireConfirmationFile": "/run/disk-nix/confirm"
              }
            }"#,
    )
    .expect("document should parse");

    let report = evaluate_apply_policy(&plan, policy.clone());
    assert_eq!(report.blocked_count, 1);
    assert_eq!(
        report.blocked[0].reason,
        "confirmation-file policy requires confirmation=true after checking the configured file"
    );

    policy.confirmation = true;
    let report = evaluate_apply_policy(&plan, policy);
    assert_eq!(report.blocked_count, 0);
}

#[test]
fn apply_policy_can_disable_device_topology_changes_and_rebalance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "pools": {
                "tank": {
                  "operation": "rebalance",
                  "addDevices": ["/dev/disk/by-id/new"],
                  "replaceDevices": {
                    "/dev/disk/by-id/old": "/dev/disk/by-id/new"
                  }
                }
              },
              "apply": {
                "allowGrow": true,
                "allowDeviceReplacement": false,
                "allowRebalance": false
              }
            }"#,
    )
    .expect("document should parse");

    let report = evaluate_apply_policy(&plan, policy);

    assert_eq!(report.blocked_count, 3);
    assert!(report.blocked.iter().any(|blocked| {
        blocked.reason == "device topology changes require allowDeviceReplacement=true"
    }));
    assert!(report
        .blocked
        .iter()
        .any(|blocked| blocked.reason == "rebalance actions require allowRebalance=true"));
}
