use disk_nix_model::{Edge, Identity, Node, NodeKind, Relationship, StorageGraph, Usage};
use serde::Deserialize;

use crate::ProbeError;

#[derive(Debug, Deserialize)]
struct LvmDocument {
    report: Vec<LvmReport>,
}

#[derive(Debug, Deserialize)]
struct LvmReport {
    #[serde(default)]
    pv: Vec<PhysicalVolume>,
    #[serde(default)]
    vg: Vec<VolumeGroup>,
    #[serde(default)]
    lv: Vec<LogicalVolume>,
    #[serde(default)]
    seg: Vec<LogicalVolumeSegment>,
}

#[derive(Debug, Deserialize)]
struct PhysicalVolume {
    pv_name: String,
    vg_name: Option<String>,
    pv_fmt: Option<String>,
    pv_uuid: Option<String>,
    dev_size: Option<String>,
    pv_major: Option<String>,
    pv_minor: Option<String>,
    pv_size: Option<String>,
    pv_free: Option<String>,
    pv_used: Option<String>,
    pe_start: Option<String>,
    pv_attr: Option<String>,
    pv_allocatable: Option<String>,
    pv_exported: Option<String>,
    pv_missing: Option<String>,
    pv_pe_count: Option<String>,
    pv_pe_alloc_count: Option<String>,
    pv_tags: Option<String>,
    pv_mda_count: Option<String>,
    pv_mda_used_count: Option<String>,
    pv_mda_free: Option<String>,
    pv_mda_size: Option<String>,
    pv_ba_start: Option<String>,
    pv_ba_size: Option<String>,
    pv_in_use: Option<String>,
    pv_duplicate: Option<String>,
    pv_device_id: Option<String>,
    pv_device_id_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VolumeGroup {
    vg_name: String,
    vg_fmt: Option<String>,
    vg_uuid: Option<String>,
    vg_attr: Option<String>,
    vg_permissions: Option<String>,
    vg_extendable: Option<String>,
    vg_exported: Option<String>,
    vg_autoactivation: Option<String>,
    vg_partial: Option<String>,
    vg_allocation_policy: Option<String>,
    vg_clustered: Option<String>,
    vg_shared: Option<String>,
    vg_size: Option<String>,
    vg_free: Option<String>,
    vg_sysid: Option<String>,
    vg_lock_type: Option<String>,
    vg_lock_args: Option<String>,
    vg_extent_size: Option<String>,
    vg_extent_count: Option<String>,
    vg_free_count: Option<String>,
    max_lv: Option<String>,
    max_pv: Option<String>,
    pv_count: Option<String>,
    vg_missing_pv_count: Option<String>,
    lv_count: Option<String>,
    snap_count: Option<String>,
    vg_seqno: Option<String>,
    vg_tags: Option<String>,
    vg_profile: Option<String>,
    vg_mda_count: Option<String>,
    vg_mda_used_count: Option<String>,
    vg_mda_free: Option<String>,
    vg_mda_size: Option<String>,
    vg_mda_copies: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LogicalVolume {
    lv_name: String,
    vg_name: String,
    lv_uuid: Option<String>,
    lv_path: Option<String>,
    lv_size: Option<String>,
    lv_attr: Option<String>,
    lv_layout: Option<String>,
    lv_active: Option<String>,
    lv_active_locally: Option<String>,
    lv_active_remotely: Option<String>,
    lv_active_exclusively: Option<String>,
    lv_permissions: Option<String>,
    lv_health_status: Option<String>,
    lv_when_full: Option<String>,
    lv_metadata_size: Option<String>,
    lv_tags: Option<String>,
    lv_dm_path: Option<String>,
    lv_parent: Option<String>,
    lv_read_ahead: Option<String>,
    lv_kernel_read_ahead: Option<String>,
    lv_suspended: Option<String>,
    lv_live_table: Option<String>,
    lv_inactive_table: Option<String>,
    lv_modules: Option<String>,
    lv_host: Option<String>,
    lv_historical: Option<String>,
    lv_kernel_major: Option<String>,
    lv_kernel_minor: Option<String>,
    lv_device_open: Option<String>,
    lv_check_needed: Option<String>,
    lv_role: Option<String>,
    lv_time: Option<String>,
    origin: Option<String>,
    pool_lv: Option<String>,
    raid_mismatch_count: Option<String>,
    raid_sync_action: Option<String>,
    raid_write_behind: Option<String>,
    raid_min_recovery_rate: Option<String>,
    raid_max_recovery_rate: Option<String>,
    raidintegritymode: Option<String>,
    raidintegrityblocksize: Option<String>,
    integritymismatches: Option<String>,
    data_percent: Option<String>,
    snap_percent: Option<String>,
    metadata_percent: Option<String>,
    copy_percent: Option<String>,
    sync_percent: Option<String>,
    cache_total_blocks: Option<String>,
    cache_used_blocks: Option<String>,
    cache_dirty_blocks: Option<String>,
    cache_read_hits: Option<String>,
    cache_read_misses: Option<String>,
    cache_write_hits: Option<String>,
    cache_write_misses: Option<String>,
    cache_promotions: Option<String>,
    cache_demotions: Option<String>,
    cache_mode: Option<String>,
    cache_policy: Option<String>,
    kernel_cache_settings: Option<String>,
    kernel_cache_mode: Option<String>,
    kernel_cache_policy: Option<String>,
    kernel_metadata_format: Option<String>,
    kernel_discards: Option<String>,
    vdo_operating_mode: Option<String>,
    vdo_compression_state: Option<String>,
    vdo_index_state: Option<String>,
    vdo_used_size: Option<String>,
    vdo_saving_percent: Option<String>,
    writecache_total_blocks: Option<String>,
    writecache_free_blocks: Option<String>,
    writecache_writeback_blocks: Option<String>,
    writecache_block_size: Option<String>,
    writecache_error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LogicalVolumeSegment {
    lv_name: String,
    vg_name: String,
    segtype: Option<String>,
    seg_start: Option<String>,
    seg_size: Option<String>,
    chunk_size: Option<String>,
    thin_count: Option<String>,
    discards: Option<String>,
    zero: Option<String>,
    transaction_id: Option<String>,
    thin_id: Option<String>,
    devices: Option<String>,
    metadata_devices: Option<String>,
    seg_pe_ranges: Option<String>,
    seg_monitor: Option<String>,
    cache_metadata_format: Option<String>,
    cache_mode: Option<String>,
    cache_policy: Option<String>,
    cache_settings: Option<String>,
    vdo_compression: Option<String>,
    vdo_deduplication: Option<String>,
    vdo_write_policy: Option<String>,
}

pub fn normalize_lvm_json(
    pvs: &[u8],
    vgs: &[u8],
    lvs: &[u8],
    segments: Option<&[u8]>,
) -> Result<StorageGraph, ProbeError> {
    let mut graph = StorageGraph::empty();

    for pv in parse_pvs(pvs)? {
        add_physical_volume(&mut graph, pv);
    }
    for vg in parse_vgs(vgs)? {
        add_volume_group(&mut graph, vg);
    }
    for lv in parse_lvs(lvs)? {
        add_logical_volume(&mut graph, lv);
    }
    if let Some(segments) = segments {
        for (index, segment) in parse_segments(segments)?.into_iter().enumerate() {
            add_logical_volume_segment(&mut graph, segment, index);
        }
    }

    Ok(graph)
}

fn parse_document(bytes: &[u8], report_name: &str) -> Result<LvmDocument, ProbeError> {
    let document: LvmDocument = serde_json::from_slice(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to parse {report_name} JSON: {error}"))
    })?;
    Ok(document)
}

fn parse_pvs(bytes: &[u8]) -> Result<Vec<PhysicalVolume>, ProbeError> {
    let document = parse_document(bytes, "pv")?;
    Ok(document
        .report
        .into_iter()
        .flat_map(|report| report.pv)
        .collect())
}

fn parse_vgs(bytes: &[u8]) -> Result<Vec<VolumeGroup>, ProbeError> {
    let document = parse_document(bytes, "vg")?;
    Ok(document
        .report
        .into_iter()
        .flat_map(|report| report.vg)
        .collect())
}

fn parse_lvs(bytes: &[u8]) -> Result<Vec<LogicalVolume>, ProbeError> {
    let document = parse_document(bytes, "lv")?;
    Ok(document
        .report
        .into_iter()
        .flat_map(|report| report.lv)
        .collect())
}

fn parse_segments(bytes: &[u8]) -> Result<Vec<LogicalVolumeSegment>, ProbeError> {
    let document = parse_document(bytes, "lv segment")?;
    Ok(document
        .report
        .into_iter()
        .flat_map(|report| report.seg)
        .collect())
}

fn add_physical_volume(graph: &mut StorageGraph, pv: PhysicalVolume) {
    let id = pv_id(&pv.pv_name);
    let mut node = Node::new(id.clone(), NodeKind::LvmPhysicalVolume, pv.pv_name.clone())
        .with_path(pv.pv_name.clone());

    if let Some(size_bytes) = parse_lvm_size(pv.pv_size.as_deref()) {
        node = node.with_size_bytes(size_bytes);
    }

    let usage = Usage {
        used_bytes: parse_lvm_size(pv.pv_used.as_deref()),
        free_bytes: parse_lvm_size(pv.pv_free.as_deref()),
        allocated_bytes: None,
    };
    if !usage.is_empty() {
        node = node.with_usage(usage);
    }

    if let Some(uuid) = pv.pv_uuid {
        node = node.with_identity(Identity {
            uuid: Some(uuid),
            ..Identity::default()
        });
    }

    for (key, value) in [
        ("lvm.pv-format", pv.pv_fmt),
        ("lvm.dev-size", pv.dev_size),
        ("lvm.pv-major", pv.pv_major),
        ("lvm.pv-minor", pv.pv_minor),
        ("lvm.pe-start", pv.pe_start),
        ("lvm.pv-attr", pv.pv_attr),
        ("lvm.pv-allocatable", pv.pv_allocatable),
        ("lvm.pv-exported", pv.pv_exported),
        ("lvm.pv-missing", pv.pv_missing),
        ("lvm.pv-pe-count", pv.pv_pe_count),
        ("lvm.pv-pe-allocated", pv.pv_pe_alloc_count),
        ("lvm.pv-tags", pv.pv_tags),
        ("lvm.pv-mda-count", pv.pv_mda_count),
        ("lvm.pv-mda-used-count", pv.pv_mda_used_count),
        ("lvm.pv-mda-free", pv.pv_mda_free),
        ("lvm.pv-mda-size", pv.pv_mda_size),
        ("lvm.pv-bootloader-area-start", pv.pv_ba_start),
        ("lvm.pv-bootloader-area-size", pv.pv_ba_size),
        ("lvm.pv-in-use", pv.pv_in_use),
        ("lvm.pv-duplicate", pv.pv_duplicate),
        ("lvm.pv-device-id", pv.pv_device_id),
        ("lvm.pv-device-id-type", pv.pv_device_id_type),
    ] {
        if let Some(value) = value.filter(|value| !value.is_empty()) {
            node = node.with_property(key, value);
        }
    }

    if let Some(vg_name) = pv.vg_name.filter(|name| !name.is_empty()) {
        graph.add_edge(Edge::new(
            id.clone(),
            vg_id(&vg_name),
            Relationship::MemberOf,
        ));
        node = node.with_property("lvm.vg", vg_name);
    }

    graph.add_node(node);
}

fn add_volume_group(graph: &mut StorageGraph, vg: VolumeGroup) {
    let id = vg_id(&vg.vg_name);
    let mut node = Node::new(id, NodeKind::LvmVolumeGroup, vg.vg_name);

    if let Some(size_bytes) = parse_lvm_size(vg.vg_size.as_deref()) {
        node = node.with_size_bytes(size_bytes);
    }

    let usage = Usage {
        used_bytes: None,
        free_bytes: parse_lvm_size(vg.vg_free.as_deref()),
        allocated_bytes: None,
    };
    if !usage.is_empty() {
        node = node.with_usage(usage);
    }

    if let Some(uuid) = vg.vg_uuid {
        node = node.with_identity(Identity {
            uuid: Some(uuid),
            ..Identity::default()
        });
    }

    for (key, value) in [
        ("lvm.vg-format", vg.vg_fmt),
        ("lvm.vg-attr", vg.vg_attr),
        ("lvm.permissions", vg.vg_permissions),
        ("lvm.vg-extendable", vg.vg_extendable),
        ("lvm.vg-exported", vg.vg_exported),
        ("lvm.vg-autoactivation", vg.vg_autoactivation),
        ("lvm.vg-partial", vg.vg_partial),
        ("lvm.allocation-policy", vg.vg_allocation_policy),
        ("lvm.vg-clustered", vg.vg_clustered),
        ("lvm.vg-shared", vg.vg_shared),
        ("lvm.vg-system-id", vg.vg_sysid),
        ("lvm.vg-lock-type", vg.vg_lock_type),
        ("lvm.vg-lock-args", vg.vg_lock_args),
        ("lvm.extent-size", vg.vg_extent_size),
        ("lvm.extent-count", vg.vg_extent_count),
        ("lvm.free-count", vg.vg_free_count),
        ("lvm.max-lvs", vg.max_lv),
        ("lvm.max-pvs", vg.max_pv),
        ("lvm.pv-count", vg.pv_count),
        ("lvm.missing-pv-count", vg.vg_missing_pv_count),
        ("lvm.lv-count", vg.lv_count),
        ("lvm.snapshot-count", vg.snap_count),
        ("lvm.vg-seqno", vg.vg_seqno),
        ("lvm.tags", vg.vg_tags),
        ("lvm.vg-profile", vg.vg_profile),
        ("lvm.vg-mda-count", vg.vg_mda_count),
        ("lvm.vg-mda-used-count", vg.vg_mda_used_count),
        ("lvm.vg-mda-free", vg.vg_mda_free),
        ("lvm.vg-mda-size", vg.vg_mda_size),
        ("lvm.vg-mda-copies", vg.vg_mda_copies),
    ] {
        if let Some(value) = value.filter(|value| !value.is_empty()) {
            node = node.with_property(key, value);
        }
    }

    graph.add_node(node);
}

fn add_logical_volume(graph: &mut StorageGraph, lv: LogicalVolume) {
    let id = lv_id(&lv.vg_name, &lv.lv_name);
    let kind = lv_kind(lv.lv_attr.as_deref());
    let mut node = Node::new(id.clone(), kind, format!("{}/{}", lv.vg_name, lv.lv_name));

    if let Some(path) = &lv.lv_path {
        node = node.with_path(path.clone());
    }
    if let Some(size_bytes) = parse_lvm_size(lv.lv_size.as_deref()) {
        node = node.with_size_bytes(size_bytes);
    }
    if let Some(uuid) = &lv.lv_uuid {
        node = node.with_identity(Identity {
            uuid: Some(uuid.clone()),
            ..Identity::default()
        });
    }

    for (key, value) in [
        ("lvm.attr", lv.lv_attr.clone()),
        ("lvm.layout", lv.lv_layout.clone()),
        ("lvm.active", lv.lv_active.clone()),
        ("lvm.active-locally", lv.lv_active_locally.clone()),
        ("lvm.active-remotely", lv.lv_active_remotely.clone()),
        ("lvm.active-exclusively", lv.lv_active_exclusively.clone()),
        ("lvm.permissions", lv.lv_permissions.clone()),
        ("lvm.health", lv.lv_health_status.clone()),
        ("lvm.when-full", lv.lv_when_full.clone()),
        ("lvm.metadata-size", lv.lv_metadata_size.clone()),
        ("lvm.tags", lv.lv_tags.clone()),
        ("lvm.dm-path", lv.lv_dm_path.clone()),
        ("lvm.parent", lv.lv_parent.clone()),
        ("lvm.read-ahead", lv.lv_read_ahead.clone()),
        ("lvm.kernel-read-ahead", lv.lv_kernel_read_ahead.clone()),
        ("lvm.suspended", lv.lv_suspended.clone()),
        ("lvm.live-table", lv.lv_live_table.clone()),
        ("lvm.inactive-table", lv.lv_inactive_table.clone()),
        ("lvm.modules", lv.lv_modules.clone()),
        ("lvm.host", lv.lv_host.clone()),
        ("lvm.historical", lv.lv_historical.clone()),
        ("lvm.kernel-major", lv.lv_kernel_major.clone()),
        ("lvm.kernel-minor", lv.lv_kernel_minor.clone()),
        ("lvm.device-open", lv.lv_device_open.clone()),
        ("lvm.check-needed", lv.lv_check_needed.clone()),
        ("lvm.role", lv.lv_role.clone()),
        ("lvm.time", lv.lv_time.clone()),
        ("lvm.origin", lv.origin.clone()),
        ("lvm.pool", lv.pool_lv.clone()),
        ("lvm.raid-mismatch-count", lv.raid_mismatch_count.clone()),
        ("lvm.raid-sync-action", lv.raid_sync_action.clone()),
        ("lvm.raid-write-behind", lv.raid_write_behind.clone()),
        (
            "lvm.raid-min-recovery-rate",
            lv.raid_min_recovery_rate.clone(),
        ),
        (
            "lvm.raid-max-recovery-rate",
            lv.raid_max_recovery_rate.clone(),
        ),
        ("lvm.raid-integrity-mode", lv.raidintegritymode.clone()),
        (
            "lvm.raid-integrity-block-size",
            lv.raidintegrityblocksize.clone(),
        ),
        (
            "lvm.raid-integrity-mismatches",
            lv.integritymismatches.clone(),
        ),
        ("lvm.data-percent", lv.data_percent.clone()),
        ("lvm.snap-percent", lv.snap_percent.clone()),
        ("lvm.metadata-percent", lv.metadata_percent.clone()),
        ("lvm.copy-percent", lv.copy_percent.clone()),
        ("lvm.sync-percent", lv.sync_percent.clone()),
        ("lvm.cache-total-blocks", lv.cache_total_blocks.clone()),
        ("lvm.cache-used-blocks", lv.cache_used_blocks.clone()),
        ("lvm.cache-dirty-blocks", lv.cache_dirty_blocks.clone()),
        ("lvm.cache-read-hits", lv.cache_read_hits.clone()),
        ("lvm.cache-read-misses", lv.cache_read_misses.clone()),
        ("lvm.cache-write-hits", lv.cache_write_hits.clone()),
        ("lvm.cache-write-misses", lv.cache_write_misses.clone()),
        ("lvm.cache-promotions", lv.cache_promotions.clone()),
        ("lvm.cache-demotions", lv.cache_demotions.clone()),
        ("lvm.cache-mode", lv.cache_mode.clone()),
        ("lvm.cache-policy", lv.cache_policy.clone()),
        (
            "lvm.kernel-cache-settings",
            lv.kernel_cache_settings.clone(),
        ),
        ("lvm.kernel-cache-mode", lv.kernel_cache_mode.clone()),
        ("lvm.kernel-cache-policy", lv.kernel_cache_policy.clone()),
        (
            "lvm.kernel-metadata-format",
            lv.kernel_metadata_format.clone(),
        ),
        ("lvm.kernel-discards", lv.kernel_discards.clone()),
        ("lvm.vdo-operating-mode", lv.vdo_operating_mode.clone()),
        (
            "lvm.vdo-compression-state",
            lv.vdo_compression_state.clone(),
        ),
        ("lvm.vdo-index-state", lv.vdo_index_state.clone()),
        ("lvm.vdo-used-size", lv.vdo_used_size.clone()),
        ("lvm.vdo-saving-percent", lv.vdo_saving_percent.clone()),
        (
            "lvm.writecache-total-blocks",
            lv.writecache_total_blocks.clone(),
        ),
        (
            "lvm.writecache-free-blocks",
            lv.writecache_free_blocks.clone(),
        ),
        (
            "lvm.writecache-writeback-blocks",
            lv.writecache_writeback_blocks.clone(),
        ),
        (
            "lvm.writecache-block-size",
            lv.writecache_block_size.clone(),
        ),
        ("lvm.writecache-error", lv.writecache_error.clone()),
    ] {
        if let Some(value) = value.filter(|value| !value.is_empty()) {
            node = node.with_property(key, value);
        }
    }

    graph.add_edge(Edge::new(
        vg_id(&lv.vg_name),
        id.clone(),
        Relationship::Contains,
    ));

    if let Some(origin) = lv.origin.filter(|origin| !origin.is_empty()) {
        graph.add_edge(Edge::new(
            id.clone(),
            lv_id(&lv.vg_name, &origin),
            Relationship::SnapshotOf,
        ));
    }
    if let Some(pool) = lv.pool_lv.filter(|pool| !pool.is_empty()) {
        graph.add_edge(Edge::new(
            id.clone(),
            lv_id(&lv.vg_name, &pool),
            Relationship::DependsOn,
        ));
    }

    graph.add_node(node);
}

fn add_logical_volume_segment(
    graph: &mut StorageGraph,
    segment: LogicalVolumeSegment,
    index: usize,
) {
    let lv_id = lv_id(&segment.vg_name, &segment.lv_name);
    let id = format!("lvm-seg:{}/{}:{index}", segment.vg_name, segment.lv_name);
    let mut node = Node::new(
        id.clone(),
        NodeKind::LvmSegment,
        format!("{}/{}:{index}", segment.vg_name, segment.lv_name),
    );

    if let Some(size_bytes) = parse_lvm_size(segment.seg_size.as_deref()) {
        node = node.with_size_bytes(size_bytes);
    }

    for (key, value) in [
        ("lvm.segment-type", segment.segtype.clone()),
        ("lvm.segment-start", segment.seg_start.clone()),
        ("lvm.segment-size", segment.seg_size.clone()),
        ("lvm.chunk-size", segment.chunk_size.clone()),
        ("lvm.thin-count", segment.thin_count.clone()),
        ("lvm.discards", segment.discards.clone()),
        ("lvm.zero", segment.zero.clone()),
        ("lvm.transaction-id", segment.transaction_id.clone()),
        ("lvm.thin-id", segment.thin_id.clone()),
        ("lvm.devices", segment.devices.clone()),
        ("lvm.metadata-devices", segment.metadata_devices.clone()),
        ("lvm.segment-pe-ranges", segment.seg_pe_ranges.clone()),
        ("lvm.segment-monitor", segment.seg_monitor.clone()),
        (
            "lvm.cache-metadata-format",
            segment.cache_metadata_format.clone(),
        ),
        ("lvm.segment-cache-mode", segment.cache_mode.clone()),
        ("lvm.segment-cache-policy", segment.cache_policy.clone()),
        ("lvm.cache-settings", segment.cache_settings.clone()),
        ("lvm.vdo-compression", segment.vdo_compression.clone()),
        ("lvm.vdo-deduplication", segment.vdo_deduplication.clone()),
        ("lvm.vdo-write-policy", segment.vdo_write_policy.clone()),
    ] {
        if let Some(value) = value.filter(|value| !value.is_empty()) {
            node = node.with_property(key, value);
        }
    }

    graph.add_node(node);
    graph.add_edge(Edge::new(lv_id.clone(), id.clone(), Relationship::Contains));

    if let Some(devices) = &segment.devices {
        for device in split_lvm_devices(devices) {
            graph.add_edge(Edge::new(
                id.clone(),
                dependency_id(&segment.vg_name, &device),
                Relationship::DependsOn,
            ));
        }
    }
    if let Some(metadata_devices) = &segment.metadata_devices {
        for device in split_lvm_devices(metadata_devices) {
            graph.add_edge(Edge::new(
                id.clone(),
                dependency_id(&segment.vg_name, &device),
                Relationship::DependsOn,
            ));
        }
    }
}

fn lv_kind(attributes: Option<&str>) -> NodeKind {
    let Some(attributes) = attributes else {
        return NodeKind::LvmLogicalVolume;
    };

    if attributes.contains('V') || attributes.contains("vdo") {
        NodeKind::VdoVolume
    } else if attributes.starts_with('t') {
        NodeKind::LvmThinPool
    } else if attributes.starts_with('s') || attributes.starts_with('S') {
        NodeKind::LvmSnapshot
    } else if attributes.contains('C') {
        NodeKind::LvmCache
    } else {
        NodeKind::LvmLogicalVolume
    }
}

fn pv_id(name: &str) -> String {
    format!("lvm-pv:{name}")
}

fn vg_id(name: &str) -> String {
    format!("lvm-vg:{name}")
}

fn lv_id(vg_name: &str, lv_name: &str) -> String {
    format!("lvm-lv:{vg_name}/{lv_name}")
}

fn dependency_id(vg_name: &str, dependency: &str) -> String {
    if dependency.starts_with("/dev/") {
        format!("block:{dependency}")
    } else {
        lv_id(vg_name, dependency)
    }
}

fn split_lvm_devices(devices: &str) -> Vec<String> {
    devices
        .split(',')
        .filter_map(|device| {
            let device = device.trim();
            if device.is_empty() {
                return None;
            }
            let name = device
                .split_once('(')
                .map_or(device, |(name, _)| name)
                .trim();
            (!name.is_empty()).then(|| name.to_string())
        })
        .collect()
}

fn parse_lvm_size(value: Option<&str>) -> Option<u64> {
    let value = value?.trim();
    if value.is_empty() {
        return None;
    }

    let numeric_end = value
        .char_indices()
        .find_map(|(index, character)| {
            (!character.is_ascii_digit() && character != '.').then_some(index)
        })
        .unwrap_or(value.len());
    let (number, suffix) = value.split_at(numeric_end);
    let number = number.parse::<f64>().ok()?;
    let multiplier = match suffix.trim().to_ascii_lowercase().as_str() {
        "" | "b" => 1.0,
        "k" | "kb" | "kib" => 1024.0,
        "m" | "mb" | "mib" => 1024.0 * 1024.0,
        "g" | "gb" | "gib" => 1024.0 * 1024.0 * 1024.0,
        "t" | "tb" | "tib" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        "p" | "pb" | "pib" => 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };

    Some((number * multiplier) as u64)
}

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
            "seg_start": "0",
            "seg_size": "40.00g",
            "chunk_size": "",
            "thin_count": "",
            "discards": "",
            "zero": "",
            "transaction_id": "",
            "thin_id": "",
            "devices": "/dev/mapper/cryptroot(0)",
            "metadata_devices": "",
            "seg_pe_ranges": "/dev/mapper/cryptroot:0-10239",
            "seg_monitor": "monitored",
            "cache_metadata_format": "",
            "cache_mode": "",
            "cache_policy": "",
            "cache_settings": "",
            "vdo_compression": "",
            "vdo_deduplication": "",
            "vdo_write_policy": ""
          },
          {
            "lv_name": "root-snap",
            "vg_name": "vg0",
            "segtype": "snapshot",
            "seg_start": "0",
            "seg_size": "10.00g",
            "chunk_size": "64.00k",
            "thin_count": "3",
            "discards": "passdown",
            "zero": "zero",
            "transaction_id": "42",
            "thin_id": "7",
            "devices": "root(0)",
            "metadata_devices": "root_tmeta(0)",
            "seg_pe_ranges": "root:0-2559",
            "seg_monitor": "monitored",
            "cache_metadata_format": "2",
            "cache_mode": "writeback",
            "cache_policy": "smq",
            "cache_settings": "migration_threshold=2048",
            "vdo_compression": "enabled",
            "vdo_deduplication": "enabled",
            "vdo_write_policy": "auto"
          }
        ]
      }]
    }"#;

    #[test]
    fn normalizes_lvm_reports_into_graph() {
        let graph =
            normalize_lvm_json(PVS, VGS, LVS, Some(SEGMENTS)).expect("fixture should parse");

        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::LvmPhysicalVolume)
        );
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
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::LvmVolumeGroup && node.name == "vg0")
        );
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
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::LvmSnapshot && node.name == "vg0/root-snap")
        );
        assert!(
            graph
                .edges
                .iter()
                .any(|edge| edge.relationship == Relationship::SnapshotOf)
        );
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::LvmSegment
                && node.properties.iter().any(|property| {
                    property.key == "lvm.segment-type" && property.value == "linear"
                })
        }));
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::LvmSegment
                && node.name == "vg0/root-snap:1"
                && node.properties.iter().any(|property| {
                    property.key == "lvm.metadata-devices" && property.value == "root_tmeta(0)"
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
                    property.key == "lvm.vdo-write-policy" && property.value == "auto"
                })
        }));
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::LvmSnapshot
                && node.name == "vg0/root-snap"
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
