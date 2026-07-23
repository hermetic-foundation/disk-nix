use disk_nix_model::{Edge, Identity, Node, NodeKind, Relationship, StorageGraph, Usage};
use serde::Deserialize;

use crate::ProbeError;

include!("lvm/records.rs");
include!("lvm/parse.rs");
include!("lvm/physical_volumes.rs");
include!("lvm/volume_groups.rs");
include!("lvm/logical_volumes.rs");
include!("lvm/segments.rs");
include!("lvm/helpers.rs");

#[cfg(test)]
mod tests {
    use disk_nix_model::{NodeKind, Relationship};

    use super::*;

    const PVS: &[u8] = br#"{
      "report": [{
        "pv": [{
          "pv_name": "/dev/mapper/cryptroot",
          "vg_name": "vg0",
          "pv_fmt": "lvm2",
          "pv_uuid": "pv-uuid",
          "dev_size": "120.00g",
          "pv_major": "253",
          "pv_minor": "5",
          "pv_size": "100.00g",
          "pv_free": "20.00g",
          "pv_used": "80.00g",
          "pe_start": "1.00m",
          "pv_attr": "a--",
          "pv_allocatable": "allocatable",
          "pv_exported": "",
          "pv_missing": "",
          "pv_pe_count": "25600",
          "pv_pe_alloc_count": "20480",
          "pv_tags": "ssd",
          "pv_mda_count": "1",
          "pv_mda_used_count": "1",
          "pv_mda_free": "1020.00k",
          "pv_mda_size": "1024.00k",
          "pv_ba_start": "0",
          "pv_ba_size": "0",
          "pv_in_use": "used",
          "pv_duplicate": "",
          "pv_device_id": "wwn-0x1234",
          "pv_device_id_type": "wwid"
        }]
      }]
    }"#;

    const VGS: &[u8] = br#"{
      "report": [{
        "vg": [{
          "vg_name": "vg0",
          "vg_fmt": "lvm2",
          "vg_uuid": "vg-uuid",
          "vg_attr": "wz--n-",
          "vg_permissions": "writeable",
          "vg_extendable": "extendable",
          "vg_exported": "",
          "vg_autoactivation": "enabled",
          "vg_partial": "",
          "vg_allocation_policy": "normal",
          "vg_clustered": "",
          "vg_shared": "",
          "vg_size": "100.00g",
          "vg_free": "20.00g",
          "vg_sysid": "host-a",
          "vg_lock_type": "none",
          "vg_lock_args": "",
          "vg_extent_size": "4.00m",
          "vg_extent_count": "25600",
          "vg_free_count": "5120",
          "max_lv": "0",
          "max_pv": "0",
          "pv_count": "1",
          "vg_missing_pv_count": "0",
          "lv_count": "3",
          "snap_count": "1",
          "vg_seqno": "17",
          "vg_tags": "system",
          "vg_profile": "",
          "vg_mda_count": "1",
          "vg_mda_used_count": "1",
          "vg_mda_free": "1020.00k",
          "vg_mda_size": "1024.00k",
          "vg_mda_copies": "unmanaged"
        }]
      }]
    }"#;

    const LVS: &[u8] = br#"{
      "report": [{
        "lv": [
          {
            "lv_name": "root",
            "vg_name": "vg0",
            "lv_uuid": "lv-root",
            "lv_path": "/dev/vg0/root",
            "lv_size": "40.00g",
            "lv_attr": "-wi-ao----",
            "lv_layout": "linear",
            "lv_active": "active",
            "lv_active_locally": "active locally",
            "lv_active_remotely": "",
            "lv_active_exclusively": "active exclusively",
            "lv_permissions": "writeable",
            "lv_health_status": "",
            "lv_when_full": "",
            "lv_metadata_size": "",
            "lv_tags": "system",
            "lv_dm_path": "/dev/mapper/vg0-root",
            "lv_parent": "",
            "lv_read_ahead": "auto",
            "lv_kernel_read_ahead": "256",
            "lv_suspended": "not suspended",
            "lv_live_table": "live",
            "lv_inactive_table": "",
            "lv_modules": "linear",
            "lv_host": "host-a",
            "lv_historical": "",
            "lv_kernel_major": "253",
            "lv_kernel_minor": "0",
            "lv_device_open": "open",
            "lv_check_needed": "",
            "lv_role": "public",
            "lv_time": "2026-06-23 10:00:00 -0500",
            "origin": "",
            "pool_lv": "",
            "raid_mismatch_count": "",
            "raid_sync_action": "",
            "raid_write_behind": "",
            "raid_min_recovery_rate": "",
            "raid_max_recovery_rate": "",
            "raidintegritymode": "",
            "raidintegrityblocksize": "",
            "integritymismatches": "",
            "data_percent": "",
            "snap_percent": "",
            "metadata_percent": "",
            "copy_percent": "",
            "sync_percent": "",
            "cache_total_blocks": "",
            "cache_used_blocks": "",
            "cache_dirty_blocks": "",
            "cache_read_hits": "",
            "cache_read_misses": "",
            "cache_write_hits": "",
            "cache_write_misses": "",
            "cache_promotions": "",
            "cache_demotions": "",
            "cache_mode": "",
            "cache_policy": "",
            "kernel_cache_settings": "",
            "kernel_cache_mode": "",
            "kernel_cache_policy": "",
            "kernel_metadata_format": "",
            "kernel_discards": "",
            "vdo_operating_mode": "",
            "vdo_compression_state": "",
            "vdo_index_state": "",
            "vdo_used_size": "",
            "vdo_saving_percent": "",
            "writecache_total_blocks": "",
            "writecache_free_blocks": "",
            "writecache_writeback_blocks": "",
            "writecache_block_size": "",
            "writecache_error": ""
          },
          {
            "lv_name": "root-snap",
            "vg_name": "vg0",
            "lv_uuid": "lv-snap",
            "lv_path": "/dev/vg0/root-snap",
            "lv_size": "10.00g",
            "lv_attr": "swi-a-s---",
            "lv_layout": "snapshot",
            "lv_active": "active",
            "lv_active_locally": "active locally",
            "lv_active_remotely": "active remotely",
            "lv_active_exclusively": "",
            "lv_permissions": "writeable",
            "lv_health_status": "partial",
            "lv_when_full": "queue",
            "lv_metadata_size": "128.00m",
            "lv_tags": "backup,snapshot",
            "lv_dm_path": "/dev/mapper/vg0-root--snap",
            "lv_parent": "root",
            "lv_read_ahead": "auto",
            "lv_kernel_read_ahead": "512",
            "lv_suspended": "suspended",
            "lv_live_table": "live",
            "lv_inactive_table": "inactive",
            "lv_modules": "snapshot",
            "lv_host": "host-b",
            "lv_historical": "historical",
            "lv_kernel_major": "253",
            "lv_kernel_minor": "1",
            "lv_device_open": "open",
            "lv_check_needed": "needed",
            "lv_role": "public",
            "lv_time": "2026-06-23 10:05:00 -0500",
            "origin": "root",
            "pool_lv": "",
            "raid_mismatch_count": "2",
            "raid_sync_action": "repair",
            "raid_write_behind": "256",
            "raid_min_recovery_rate": "1024",
            "raid_max_recovery_rate": "8192",
            "raidintegritymode": "journal",
            "raidintegrityblocksize": "4096",
            "integritymismatches": "1",
            "data_percent": "12.00",
            "snap_percent": "12.00",
            "metadata_percent": "",
            "copy_percent": "",
            "sync_percent": "",
            "cache_total_blocks": "4096",
            "cache_used_blocks": "1024",
            "cache_dirty_blocks": "64",
            "cache_read_hits": "1000",
            "cache_read_misses": "25",
            "cache_write_hits": "900",
            "cache_write_misses": "30",
            "cache_promotions": "128",
            "cache_demotions": "32",
            "cache_mode": "writeback",
            "cache_policy": "smq",
            "kernel_cache_settings": "migration_threshold=2048",
            "kernel_cache_mode": "writeback",
            "kernel_cache_policy": "smq",
            "kernel_metadata_format": "2",
            "kernel_discards": "passdown",
            "vdo_operating_mode": "normal",
            "vdo_compression_state": "online",
            "vdo_index_state": "online",
            "vdo_used_size": "8.00g",
            "vdo_saving_percent": "42.00",
            "writecache_total_blocks": "1024",
            "writecache_free_blocks": "512",
            "writecache_writeback_blocks": "16",
            "writecache_block_size": "4096",
            "writecache_error": "0"
          }
        ]
      }]
    }"#;

    const SEGMENTS: &[u8] = br#"{
      "report": [{
        "seg": [
          {
            "lv_name": "root",
            "vg_name": "vg0",
            "segtype": "linear",
            "stripes": "1",
            "data_stripes": "1",
            "reshape_len": "",
            "reshape_len_le": "",
            "data_copies": "1",
            "data_offset": "0",
            "new_data_offset": "",
            "parity_chunks": "",
            "stripe_size": "",
            "region_size": "",
            "seg_start": "0",
            "seg_start_pe": "0",
            "seg_size": "40.00g",
            "seg_size_pe": "10240",
            "seg_tags": "",
            "chunk_size": "",
            "thin_count": "",
            "discards": "",
            "zero": "",
            "transaction_id": "",
            "thin_id": "",
            "devices": "/dev/mapper/cryptroot(0)",
            "metadata_devices": "",
            "seg_pe_ranges": "/dev/mapper/cryptroot:0-10239",
            "seg_le_ranges": "0-10239",
            "seg_metadata_le_ranges": "",
            "seg_monitor": "monitored",
            "cache_metadata_format": "",
            "cache_mode": "",
            "cache_policy": "",
            "cache_settings": "",
            "integrity_settings": "",
            "vdo_compression": "",
            "vdo_deduplication": "",
            "vdo_minimum_io_size": "",
            "vdo_block_map_cache_size": "",
            "vdo_block_map_era_length": "",
            "vdo_use_sparse_index": "",
            "vdo_index_memory_size": "",
            "vdo_slab_size": "",
            "vdo_ack_threads": "",
            "vdo_bio_threads": "",
            "vdo_bio_rotation": "",
            "vdo_cpu_threads": "",
            "vdo_hash_zone_threads": "",
            "vdo_logical_threads": "",
            "vdo_physical_threads": "",
            "vdo_max_discard": "",
            "vdo_header_size": "",
            "vdo_use_metadata_hints": "",
            "vdo_write_policy": ""
          },
          {
            "lv_name": "root-snap",
            "vg_name": "vg0",
            "segtype": "snapshot",
            "stripes": "2",
            "data_stripes": "2",
            "reshape_len": "128.00m",
            "reshape_len_le": "32",
            "data_copies": "2",
            "data_offset": "2048",
            "new_data_offset": "4096",
            "parity_chunks": "1",
            "stripe_size": "64.00k",
            "region_size": "512.00k",
            "seg_start": "0",
            "seg_start_pe": "0",
            "seg_size": "10.00g",
            "seg_size_pe": "2560",
            "seg_tags": "hot",
            "chunk_size": "64.00k",
            "thin_count": "3",
            "discards": "passdown",
            "zero": "zero",
            "transaction_id": "42",
            "thin_id": "7",
            "devices": "root(0)",
            "metadata_devices": "root_tmeta(0)",
            "seg_pe_ranges": "root:0-2559",
            "seg_le_ranges": "0-2559",
            "seg_metadata_le_ranges": "root_tmeta:0-31",
            "seg_monitor": "monitored",
            "cache_metadata_format": "2",
            "cache_mode": "writeback",
            "cache_policy": "smq",
            "cache_settings": "migration_threshold=2048",
            "integrity_settings": "journal_sectors=2048",
            "vdo_compression": "enabled",
            "vdo_deduplication": "enabled",
            "vdo_minimum_io_size": "4096",
            "vdo_block_map_cache_size": "128.00m",
            "vdo_block_map_era_length": "16380",
            "vdo_use_sparse_index": "enabled",
            "vdo_index_memory_size": "256.00m",
            "vdo_slab_size": "2.00g",
            "vdo_ack_threads": "1",
            "vdo_bio_threads": "4",
            "vdo_bio_rotation": "64",
            "vdo_cpu_threads": "2",
            "vdo_hash_zone_threads": "1",
            "vdo_logical_threads": "2",
            "vdo_physical_threads": "2",
            "vdo_max_discard": "4.00m",
            "vdo_header_size": "512.00k",
            "vdo_use_metadata_hints": "disabled",
            "vdo_write_policy": "auto"
          }
        ]
      }]
    }"#;

    #[test]
    fn normalizes_lvm_reports_into_graph() {
        let graph =
            normalize_lvm_json(PVS, VGS, LVS, Some(SEGMENTS)).expect("fixture should parse");

        assert!(graph
            .nodes
            .iter()
            .any(|node| node.kind == NodeKind::LvmPhysicalVolume));
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::LvmPhysicalVolume
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "lvm.pv-format" && property.value == "lvm2")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "lvm.dev-size" && property.value == "120.00g")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "lvm.pv-pe-count" && property.value == "25600")
                && node.properties.iter().any(|property| {
                    property.key == "lvm.pv-mda-free" && property.value == "1020.00k"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.pv-device-id" && property.value == "wwn-0x1234"
                })
        }));
        assert!(graph
            .nodes
            .iter()
            .any(|node| node.kind == NodeKind::LvmVolumeGroup && node.name == "vg0"));
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::LvmVolumeGroup
                && node.name == "vg0"
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "lvm.vg-format" && property.value == "lvm2")
                && node.properties.iter().any(|property| {
                    property.key == "lvm.permissions" && property.value == "writeable"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.allocation-policy" && property.value == "normal"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "lvm.extent-count" && property.value == "25600")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "lvm.snapshot-count" && property.value == "1")
                && node.properties.iter().any(|property| {
                    property.key == "lvm.vg-mda-copies" && property.value == "unmanaged"
                })
        }));
        assert!(graph
            .nodes
            .iter()
            .any(|node| node.kind == NodeKind::LvmSnapshot && node.name == "vg0/root-snap"));
        assert!(graph
            .edges
            .iter()
            .any(|edge| edge.relationship == Relationship::SnapshotOf));
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::LvmSegment
                && node.properties.iter().any(|property| {
                    property.key == "lvm.segment-type" && property.value == "linear"
                })
        }));
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::LvmSegment
                && node.name == "vg0/root-snap:1"
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "lvm.segment-stripes" && property.value == "2")
                && node.properties.iter().any(|property| {
                    property.key == "lvm.reshape-length" && property.value == "128.00m"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "lvm.data-copies" && property.value == "2")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "lvm.stripe-size" && property.value == "64.00k")
                && node.properties.iter().any(|property| {
                    property.key == "lvm.segment-size-extents" && property.value == "2560"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "lvm.segment-tags" && property.value == "hot")
                && node.properties.iter().any(|property| {
                    property.key == "lvm.metadata-devices" && property.value == "root_tmeta(0)"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.segment-le-ranges" && property.value == "0-2559"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.segment-metadata-le-ranges"
                        && property.value == "root_tmeta:0-31"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "lvm.chunk-size" && property.value == "64.00k")
                && node.properties.iter().any(|property| {
                    property.key == "lvm.segment-monitor" && property.value == "monitored"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.cache-settings"
                        && property.value == "migration_threshold=2048"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.integrity-settings"
                        && property.value == "journal_sectors=2048"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.vdo-block-map-cache-size" && property.value == "128.00m"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.vdo-use-sparse-index" && property.value == "enabled"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "lvm.vdo-bio-threads" && property.value == "4")
                && node.properties.iter().any(|property| {
                    property.key == "lvm.vdo-max-discard" && property.value == "4.00m"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.vdo-write-policy" && property.value == "auto"
                })
        }));
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::LvmSnapshot
                && node.name == "vg0/root-snap"
                && node
                    .usage
                    .as_ref()
                    .is_some_and(|usage| usage.used_bytes == Some(8_589_934_592))
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "lvm.active" && property.value == "active")
                && node.properties.iter().any(|property| {
                    property.key == "lvm.active-locally" && property.value == "active locally"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.active-remotely" && property.value == "active remotely"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "lvm.layout" && property.value == "snapshot")
                && node.properties.iter().any(|property| {
                    property.key == "lvm.dm-path" && property.value == "/dev/mapper/vg0-root--snap"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "lvm.parent" && property.value == "root")
                && node.properties.iter().any(|property| {
                    property.key == "lvm.kernel-read-ahead" && property.value == "512"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.suspended" && property.value == "suspended"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "lvm.live-table" && property.value == "live")
                && node.properties.iter().any(|property| {
                    property.key == "lvm.inactive-table" && property.value == "inactive"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "lvm.modules" && property.value == "snapshot")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "lvm.host" && property.value == "host-b")
                && node.properties.iter().any(|property| {
                    property.key == "lvm.historical" && property.value == "historical"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "lvm.health" && property.value == "partial")
                && node.properties.iter().any(|property| {
                    property.key == "lvm.tags" && property.value == "backup,snapshot"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.raid-mismatch-count" && property.value == "2"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.raid-sync-action" && property.value == "repair"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.raid-write-behind" && property.value == "256"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.raid-min-recovery-rate" && property.value == "1024"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.raid-max-recovery-rate" && property.value == "8192"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.raid-integrity-mode" && property.value == "journal"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.raid-integrity-block-size" && property.value == "4096"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.raid-integrity-mismatches" && property.value == "1"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.cache-mode" && property.value == "writeback"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "lvm.cache-policy" && property.value == "smq")
                && node.properties.iter().any(|property| {
                    property.key == "lvm.cache-total-blocks" && property.value == "4096"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.cache-dirty-blocks" && property.value == "64"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.cache-read-hits" && property.value == "1000"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.cache-write-misses" && property.value == "30"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.cache-promotions" && property.value == "128"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.kernel-cache-settings"
                        && property.value == "migration_threshold=2048"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.kernel-metadata-format" && property.value == "2"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.kernel-discards" && property.value == "passdown"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.vdo-operating-mode" && property.value == "normal"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.vdo-compression-state" && property.value == "online"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.vdo-index-state" && property.value == "online"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.vdo-used-size" && property.value == "8.00g"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.vdo-saving-percent" && property.value == "42.00"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.writecache-writeback-blocks" && property.value == "16"
                })
                && node.properties.iter().any(|property| {
                    property.key == "lvm.writecache-block-size" && property.value == "4096"
                })
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0.starts_with("lvm-seg:vg0/root:")
                && edge.to.0 == "block:/dev/mapper/cryptroot"
                && edge.relationship == Relationship::DependsOn
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0.starts_with("lvm-seg:vg0/root-snap:")
                && edge.to.0 == "lvm-lv:vg0/root"
                && edge.relationship == Relationship::DependsOn
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0.starts_with("lvm-seg:vg0/root-snap:")
                && edge.to.0 == "lvm-lv:vg0/root_tmeta"
                && edge.relationship == Relationship::DependsOn
        }));
    }

    #[test]
    fn parses_lvm_size_suffixes() {
        assert_eq!(parse_lvm_size(Some("1.50g")), Some(1_610_612_736));
        assert_eq!(parse_lvm_size(Some("4.00m")), Some(4_194_304));
        assert_eq!(parse_lvm_size(Some("")), None);
    }

    #[test]
    fn splits_lvm_device_references() {
        assert_eq!(
            split_lvm_devices("/dev/sda2(0), root_cdata(12)"),
            vec!["/dev/sda2".to_string(), "root_cdata".to_string()]
        );
    }
}
