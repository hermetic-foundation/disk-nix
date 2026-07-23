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
        ("lvm.vg-lock-status", vg.vg_lock_status),
        ("lvm.vg-lock-failure", vg.vg_lock_failure),
        ("lvm.vg-lock-reason", vg.vg_lock_reason),
        ("lvm.vg-split-brain", vg.vg_split_brain),
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
