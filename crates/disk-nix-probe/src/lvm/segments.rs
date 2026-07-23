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
        ("lvm.segment-stripes", segment.stripes.clone()),
        ("lvm.segment-data-stripes", segment.data_stripes.clone()),
        ("lvm.reshape-length", segment.reshape_len.clone()),
        ("lvm.reshape-length-extents", segment.reshape_len_le.clone()),
        ("lvm.data-copies", segment.data_copies.clone()),
        ("lvm.data-offset", segment.data_offset.clone()),
        ("lvm.new-data-offset", segment.new_data_offset.clone()),
        ("lvm.parity-chunks", segment.parity_chunks.clone()),
        ("lvm.stripe-size", segment.stripe_size.clone()),
        ("lvm.region-size", segment.region_size.clone()),
        ("lvm.segment-start", segment.seg_start.clone()),
        ("lvm.segment-start-extent", segment.seg_start_pe.clone()),
        ("lvm.segment-size", segment.seg_size.clone()),
        ("lvm.segment-size-extents", segment.seg_size_pe.clone()),
        ("lvm.segment-tags", segment.seg_tags.clone()),
        ("lvm.chunk-size", segment.chunk_size.clone()),
        ("lvm.thin-count", segment.thin_count.clone()),
        ("lvm.discards", segment.discards.clone()),
        ("lvm.zero", segment.zero.clone()),
        ("lvm.transaction-id", segment.transaction_id.clone()),
        ("lvm.thin-id", segment.thin_id.clone()),
        ("lvm.devices", segment.devices.clone()),
        ("lvm.metadata-devices", segment.metadata_devices.clone()),
        ("lvm.segment-pe-ranges", segment.seg_pe_ranges.clone()),
        ("lvm.segment-le-ranges", segment.seg_le_ranges.clone()),
        (
            "lvm.segment-metadata-le-ranges",
            segment.seg_metadata_le_ranges.clone(),
        ),
        ("lvm.segment-monitor", segment.seg_monitor.clone()),
        (
            "lvm.cache-metadata-format",
            segment.cache_metadata_format.clone(),
        ),
        ("lvm.segment-cache-mode", segment.cache_mode.clone()),
        ("lvm.segment-cache-policy", segment.cache_policy.clone()),
        ("lvm.cache-settings", segment.cache_settings.clone()),
        ("lvm.integrity-settings", segment.integrity_settings.clone()),
        ("lvm.vdo-compression", segment.vdo_compression.clone()),
        ("lvm.vdo-deduplication", segment.vdo_deduplication.clone()),
        (
            "lvm.vdo-minimum-io-size",
            segment.vdo_minimum_io_size.clone(),
        ),
        (
            "lvm.vdo-block-map-cache-size",
            segment.vdo_block_map_cache_size.clone(),
        ),
        (
            "lvm.vdo-block-map-era-length",
            segment.vdo_block_map_era_length.clone(),
        ),
        (
            "lvm.vdo-use-sparse-index",
            segment.vdo_use_sparse_index.clone(),
        ),
        (
            "lvm.vdo-index-memory-size",
            segment.vdo_index_memory_size.clone(),
        ),
        ("lvm.vdo-slab-size", segment.vdo_slab_size.clone()),
        ("lvm.vdo-ack-threads", segment.vdo_ack_threads.clone()),
        ("lvm.vdo-bio-threads", segment.vdo_bio_threads.clone()),
        ("lvm.vdo-bio-rotation", segment.vdo_bio_rotation.clone()),
        ("lvm.vdo-cpu-threads", segment.vdo_cpu_threads.clone()),
        (
            "lvm.vdo-hash-zone-threads",
            segment.vdo_hash_zone_threads.clone(),
        ),
        (
            "lvm.vdo-logical-threads",
            segment.vdo_logical_threads.clone(),
        ),
        (
            "lvm.vdo-physical-threads",
            segment.vdo_physical_threads.clone(),
        ),
        ("lvm.vdo-max-discard", segment.vdo_max_discard.clone()),
        ("lvm.vdo-header-size", segment.vdo_header_size.clone()),
        (
            "lvm.vdo-use-metadata-hints",
            segment.vdo_use_metadata_hints.clone(),
        ),
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
