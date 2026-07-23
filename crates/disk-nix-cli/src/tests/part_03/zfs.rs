#[test]
fn zfs_table_includes_pool_vdev_dataset_snapshot_and_zvol_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("zfs-pool:tank", NodeKind::ZfsPool, "tank")
            .with_size_bytes(1_099_511_627_776)
            .with_usage(Usage {
                used_bytes: Some(274_877_906_944),
                free_bytes: Some(824_633_720_832),
                allocated_bytes: Some(274_877_906_944),
            })
            .with_property("zfs.health", "ONLINE")
            .with_property("zfs.state", "ONLINE")
            .with_property("zfs.pool-ashift", "12")
            .with_property("zfs.pool-autotrim", "on")
            .with_property("zfs.pool-autoexpand", "off")
            .with_property("zfs.pool-cachefile", "/etc/zfs/zpool.cache")
            .with_property("zfs.pool-failmode", "wait")
            .with_property("zfs.status", "some devices need attention")
            .with_property("zfs.action", "replace the faulted device")
            .with_property("zfs.scan", "scrub repaired 0B")
            .with_property("zfs.errors", "No known data errors")
            .with_property("zfs.pool-read-errors", "3")
            .with_property("zfs.pool-write-errors", "4")
            .with_property("zfs.pool-checksum-errors", "5"),
    );
    graph.add_node(
        Node::new(
            "zfs-vdev:tank:/dev/disk/by-id/nvme-tank-a",
            NodeKind::ZfsVdev,
            "/dev/disk/by-id/nvme-tank-a",
        )
        .with_path("/dev/disk/by-id/nvme-tank-a")
        .with_property("zfs.vdev-role", "data")
        .with_property("zfs.vdev-state", "ONLINE")
        .with_property("zfs.read-errors", "0")
        .with_property("zfs.write-errors", "1")
        .with_property("zfs.checksum-errors", "2"),
    );
    graph.add_node(
        Node::new("zfs-dataset:tank/home", NodeKind::ZfsDataset, "tank/home")
            .with_usage(Usage {
                used_bytes: Some(107_374_182_400),
                free_bytes: Some(805_306_368_000),
                allocated_bytes: Some(107_374_182_400),
            })
            .with_property("zfs.compression", "zstd")
            .with_property("zfs.quota", "500G")
            .with_property("zfs.reservation", "10G")
            .with_property("zfs.encryption", "aes-256-gcm")
            .with_property("zfs.keystatus", "available")
            .with_property("zfs.recordsize", "1048576")
            .with_property("zfs.dedup", "off")
            .with_property("zfs.checksum", "sha512")
            .with_property("zfs.copies", "2")
            .with_property("zfs.sync", "disabled")
            .with_property("zfs.primarycache", "metadata")
            .with_property("zfs.secondarycache", "all")
            .with_property("zfs.atime", "off")
            .with_property("zfs.relatime", "on")
            .with_property("zfs.snapdir", "visible")
            .with_property("zfs.acltype", "posixacl")
            .with_property("zfs.xattr", "sa"),
    );
    graph.add_node(
        Node::new(
            "zfs-snapshot:tank/home@daily",
            NodeKind::ZfsSnapshot,
            "tank/home@daily",
        )
        .with_property("zfs.userrefs", "2")
        .with_property("zfs.compression", "zstd"),
    );
    graph.add_node(
        Node::new("zvol:tank/vm/root", NodeKind::Zvol, "tank/vm/root")
            .with_size_bytes(85_899_345_920)
            .with_property("zfs.origin", "tank/vm/base@clean")
            .with_property("zfs.volsize", "80G"),
    );
    graph.add_edge(Edge::new(
        "zfs-pool:tank",
        "zfs-vdev:tank:/dev/disk/by-id/nvme-tank-a",
        Relationship::Contains,
    ));
    graph.add_edge(Edge::new(
        "zfs-pool:tank",
        "zfs-dataset:tank/home",
        Relationship::Contains,
    ));
    graph.add_edge(Edge::new(
        "zfs-pool:tank",
        "zvol:tank/vm/root",
        Relationship::Contains,
    ));
    graph.add_edge(Edge::new(
        "zfs-snapshot:tank/home@daily",
        "zfs-dataset:tank/home",
        Relationship::SnapshotOf,
    ));

    let pool = graph
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::ZfsPool)
        .expect("pool fixture exists");
    let snapshot = graph
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::ZfsSnapshot)
        .expect("snapshot fixture exists");
    assert_eq!(zfs_child_count(&graph, pool), 3);
    assert_eq!(zfs_child_count(&graph, snapshot), 1);

    let mut output = Vec::new();
    print_zfs(&mut output, &graph).expect("zfs table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("HEALTH"));
    assert!(output.contains("ORIGIN"));
    assert!(output.contains("CHILDREN"));
    assert!(output.contains("tank"));
    assert!(output.contains("ONLINE"));
    assert!(output.contains(
            "pool-ashift=12 pool-autotrim=on pool-autoexpand=off pool-cachefile=/etc/zfs/zpool.cache pool-failmode=wait"
        ));
    assert!(output.contains(
            "status=some devices need attention action=replace the faulted device scan=scrub repaired 0B errors=No known data errors pool-read-errors=3 pool-write-errors=4 pool-checksum-errors=5"
        ));
    assert!(
        output.contains("data vdev-state=ONLINE read-errors=0 write-errors=1 checksum-errors=2")
    );
    assert!(output.contains("tank/home"));
    assert!(output.contains(
        "compression=zstd quota=500G reservation=10G encryption=aes-256-gcm keystatus=available"
    ));
    assert!(output.contains("recordsize=1048576 dedup=off checksum=sha512 copies=2"));
    assert!(output.contains("sync=disabled primarycache=metadata secondarycache=all"));
    assert!(output.contains("atime=off relatime=on snapdir=visible acltype=posixacl xattr=sa"));
    assert!(output.contains("tank/home@daily"));
    assert!(output.contains("userrefs=2 compression=zstd"));
    assert!(output.contains("tank/vm/root"));
    assert!(output.contains("tank/vm/base@clean"));
    assert!(output.contains("volsize=80G"));
}
