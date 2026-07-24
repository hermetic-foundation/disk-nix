use clap::Parser;
use disk_nix_exec::{prepare_execution, ExecutionMode};
use disk_nix_model::{Edge, Identity, Node, NodeKind, Relationship, StorageGraph, Usage};
use disk_nix_plan::{compare_plan_with_topology, plan_and_policy_from_json_bytes, ApplyMode};
use disk_nix_probe::{ProbeReport, ProbeStatus};
use serde_json::Value;

use super::{
    apply_receipt, command_stdout_first_line, confirmation_file_accepts, consumer_count,
    install_mount_script_from_spec, install_zfs_root_spec, is_backing_file_node, is_bcachefs_node,
    is_btrfs_node, is_cache_node, is_complex_filesystem_node, is_device_node, is_dm_node,
    is_encryption_node, is_filesystem_node, is_iscsi_node, is_loop_node, is_lun_node, is_lvm_node,
    is_mapping_node, is_multipath_node, is_network_storage_node, is_nfs_node, is_nvme_node,
    is_partition_node, is_pool_node, is_raid_node, is_snapshot_node, is_swap_node, is_vdo_node,
    is_volume_node, is_zfs_node, is_zram_node, iscsi_lun_count, member_count,
    migration_report_from_json_bytes, mount_details, nfs_mount_count, parse_os_release,
    print_backing_files, print_bcachefs, print_btrfs, print_cache, print_complex_filesystems,
    print_devices, print_dm, print_encryption, print_filesystems, print_filtered_json,
    print_inspect, print_inspect_json, print_iscsi, print_loop, print_luns, print_lvm,
    print_mappings, print_migration_report, print_mounts, print_multipath, print_network_storage,
    print_nfs, print_nvme, print_partitions, print_pools, print_probe_preflight_checks,
    print_probe_preflight_environment, print_probe_reports, print_raid, print_snapshots,
    print_swap, print_usage, print_vdo, print_volumes, print_zfs, print_zram,
    probe_preflight_checks, run, script_refusal_message, snapshot_source, storage_tool_version_report,
    usage_details, usage_percent, zfs_child_count, Cli, InstallZfsRootOptions,
    ProbePreflightEnvironment, ProbeStatusPreflightReport, ToolVersionReport, ToolVersionStatus,
};

fn assert_mapping(
    mappings: &[super::LegacyMigrationMapping],
    source: &str,
    target: &str,
    scope: &str,
) {
    assert!(
        mappings.iter().any(|mapping| {
            mapping.source == source && mapping.target == target && mapping.scope == scope
        }),
        "missing mapping {source} -> {target} ({scope}) in {mappings:?}"
    );
}

include!("tests/part_01.rs");
include!("tests/part_02.rs");
include!("tests/part_03.rs");
include!("tests/part_04.rs");
include!("tests/part_05.rs");
include!("tests/part_06.rs");
