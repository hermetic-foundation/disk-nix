fn add_pool(graph: &mut StorageGraph, pool: ZpoolRow) {
    let mut node = Node::new(pool_id(&pool.name), NodeKind::ZfsPool, pool.name);

    if let Some(size_bytes) = pool.size {
        node = node.with_size_bytes(size_bytes);
    }

    let usage = Usage {
        used_bytes: pool.allocated,
        free_bytes: pool.free,
        allocated_bytes: pool.allocated,
    };
    if !usage.is_empty() {
        node = node.with_usage(usage);
    }

    if let Some(health) = pool.health {
        node = node.with_property("zfs.health", health);
    }
    for (key, value) in [
        ("zfs.pool-capacity", pool.capacity),
        ("zfs.pool-dedupratio", pool.dedupratio),
        ("zfs.pool-fragmentation", pool.fragmentation),
        ("zfs.pool-altroot", pool.altroot),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
    }

    graph.add_node(node);
}

fn add_pool_property(graph: &mut StorageGraph, property: ZpoolProperty) {
    let key = format!(
        "zfs.pool-{}",
        property
            .property
            .chars()
            .map(|character| match character {
                'A'..='Z' => character.to_ascii_lowercase(),
                'a'..='z' | '0'..='9' => character,
                _ => '-',
            })
            .collect::<String>()
            .split('-')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    );
    graph.add_node(
        Node::new(pool_id(&property.pool), NodeKind::ZfsPool, property.pool)
            .with_property(key, property.value),
    );
}

fn add_status_pool(graph: &mut StorageGraph, pool: ZpoolStatus) {
    let mut node = Node::new(pool_id(&pool.name), NodeKind::ZfsPool, pool.name.clone());
    if let Some(state) = pool.state {
        node = node.with_property("zfs.state", state);
    }
    for (key, value) in [
        ("zfs.status", pool.status),
        ("zfs.action", pool.action),
        ("zfs.scan", pool.scan),
        ("zfs.errors", pool.errors),
        ("zfs.pool-read-errors", pool.read_errors),
        ("zfs.pool-write-errors", pool.write_errors),
        ("zfs.pool-checksum-errors", pool.checksum_errors),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
    }
    graph.add_node(node);

    for vdev in pool.vdevs {
        add_vdev(graph, &pool.name, vdev);
    }
}

fn add_vdev(graph: &mut StorageGraph, pool_name: &str, vdev: ZpoolVdev) {
    let id = vdev_id(pool_name, &vdev.name);
    let mut node = Node::new(id.clone(), NodeKind::ZfsVdev, vdev.name.clone())
        .with_property("zfs.vdev-role", vdev.role.clone());

    for (key, value) in [
        ("zfs.vdev-state", vdev.state),
        ("zfs.read-errors", vdev.read_errors),
        ("zfs.write-errors", vdev.write_errors),
        ("zfs.checksum-errors", vdev.checksum_errors),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
    }
    if let Some(path) = &vdev.device_path {
        node = node.with_path(path.clone());
    }

    graph.add_node(node);

    if let Some(parent) = vdev.parent.filter(|parent| parent != pool_name) {
        graph.add_edge(Edge::new(
            vdev_id(pool_name, &parent),
            id.clone(),
            Relationship::Contains,
        ));
    } else {
        graph.add_edge(Edge::new(
            pool_id(pool_name),
            id.clone(),
            Relationship::Contains,
        ));
    }

    if let Some(path) = vdev.device_path {
        let block_id = format!("block:{path}");
        graph.add_node(
            Node::new(block_id.clone(), NodeKind::PhysicalDisk, path.clone()).with_path(path),
        );
        graph.add_edge(Edge::new(block_id, id, Relationship::Backs));
    }
}

fn dataset_kinds(datasets: &[ZfsRow]) -> BTreeMap<String, NodeKind> {
    datasets
        .iter()
        .filter(|dataset| dataset.kind != "snapshot")
        .map(|dataset| (dataset.name.clone(), dataset_kind(&dataset.kind)))
        .collect()
}

fn add_dataset(
    graph: &mut StorageGraph,
    dataset: ZfsRow,
    dataset_kinds: &BTreeMap<String, NodeKind>,
) {
    let kind = dataset_kind(&dataset.kind);
    let id = dataset_id(&dataset.name, kind);
    let mut node = Node::new(id.clone(), kind, dataset.name.clone())
        .with_property("zfs.type", dataset.kind.clone());

    let usage = Usage {
        used_bytes: dataset.used,
        free_bytes: dataset.available,
        allocated_bytes: dataset.referenced,
    };
    if !usage.is_empty() {
        node = node.with_usage(usage);
    }

    if let Some(mountpoint) = dataset
        .mountpoint
        .filter(|mountpoint| mountpoint != "legacy")
    {
        let mount_id = format!("mount:{mountpoint}");
        graph.add_node(Node::new(
            mount_id.clone(),
            NodeKind::Mountpoint,
            mountpoint,
        ));
        graph.add_edge(Edge::new(id.clone(), mount_id, Relationship::MountedAt));
    }

    if let Some(origin) = dataset.origin {
        node = node.with_property("zfs.origin", origin.clone());
        graph.add_edge(Edge::new(
            id.clone(),
            dataset_id(&origin, NodeKind::ZfsSnapshot),
            Relationship::SnapshotOf,
        ));
    }

    if kind == NodeKind::ZfsSnapshot {
        if let Some(source) = dataset.name.split_once('@').map(|(source, _)| source) {
            let source_kind = dataset_kinds
                .get(source)
                .copied()
                .unwrap_or(NodeKind::ZfsDataset);
            graph.add_edge(Edge::new(
                id.clone(),
                dataset_id(source, source_kind),
                Relationship::SnapshotOf,
            ));
        }
    }

    if let Some(userrefs) = dataset.userrefs {
        node = node.with_property("zfs.userrefs", userrefs);
    }
    for (key, value) in [
        ("zfs.compression", dataset.compression),
        ("zfs.quota", dataset.quota),
        ("zfs.reservation", dataset.reservation),
        ("zfs.encryption", dataset.encryption),
        ("zfs.keystatus", dataset.keystatus),
        ("zfs.volsize", dataset.volsize),
        ("zfs.recordsize", dataset.recordsize),
        ("zfs.dedup", dataset.dedup),
        ("zfs.checksum", dataset.checksum),
        ("zfs.copies", dataset.copies),
        ("zfs.sync", dataset.sync),
        ("zfs.primarycache", dataset.primarycache),
        ("zfs.secondarycache", dataset.secondarycache),
        ("zfs.atime", dataset.atime),
        ("zfs.relatime", dataset.relatime),
        ("zfs.snapdir", dataset.snapdir),
        ("zfs.acltype", dataset.acltype),
        ("zfs.xattr", dataset.xattr),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
    }

    if let Some(pool) = dataset
        .name
        .split('/')
        .next()
        .and_then(|value| value.split('@').next())
    {
        graph.add_edge(Edge::new(pool_id(pool), id.clone(), Relationship::Contains));
    }

    graph.add_node(node);
}

fn add_snapshot_hold(graph: &mut StorageGraph, hold: ZfsHold) {
    let tag_key = normalize_property_suffix(&hold.tag);
    let node = Node::new(
        dataset_id(&hold.snapshot, NodeKind::ZfsSnapshot),
        NodeKind::ZfsSnapshot,
        hold.snapshot,
    )
    .with_property("zfs.holds", hold.tag.clone())
    .with_property(
        format!("zfs.hold.{tag_key}"),
        hold.timestamp.unwrap_or_else(|| "present".to_string()),
    )
    .with_property(format!("zfs.hold-tag.{tag_key}"), hold.tag);
    graph.add_node(node);
}
