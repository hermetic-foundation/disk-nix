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
    let usage = Usage {
        used_bytes: parse_lvm_size(lv.vdo_used_size.as_deref()),
        free_bytes: None,
        allocated_bytes: None,
    };
    if !usage.is_empty() {
        node = node.with_usage(usage);
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
        ("lvm.lock-status", lv.lv_lock_status.clone()),
        ("lvm.lock-args", lv.lv_lock_args.clone()),
        ("lvm.lock-failure", lv.lv_lock_failure.clone()),
        ("lvm.lock-reason", lv.lv_lock_reason.clone()),
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
