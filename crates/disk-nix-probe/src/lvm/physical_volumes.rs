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
