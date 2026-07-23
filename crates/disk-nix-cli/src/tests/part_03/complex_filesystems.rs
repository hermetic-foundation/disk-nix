#[test]
fn complex_filesystems_table_includes_topology_and_domain_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(Node::new(
        "block:/dev/nvme0n1p2",
        NodeKind::Partition,
        "/dev/nvme0n1p2",
    ));
    graph.add_node(
        Node::new("btrfs:fs-uuid", NodeKind::BtrfsFilesystem, "/mnt/persist")
            .with_size_bytes(536_870_912_000)
            .with_usage(Usage {
                used_bytes: Some(214_748_364_800),
                free_bytes: Some(322_122_547_200),
                allocated_bytes: None,
            })
            .with_property("btrfs.mount-target", "/mnt/persist")
            .with_property("btrfs.data-profile", "single")
            .with_property("btrfs.metadata-profile", "DUP"),
    );
    graph.add_node(
        Node::new(
            "bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3",
            NodeKind::BcachefsFilesystem,
            "archive",
        )
        .with_usage(Usage {
            used_bytes: Some(2_147_483_648),
            free_bytes: Some(8_589_934_592),
            allocated_bytes: Some(10_737_418_240),
        })
        .with_property("bcachefs.mount-target", "/mnt/archive")
        .with_property("bcachefs.device-count", "2")
        .with_property("bcachefs.data-user", "2147483648"),
    );
    graph.add_node(
        Node::new("zpool:tank", NodeKind::ZfsPool, "tank")
            .with_size_bytes(1_099_511_627_776)
            .with_usage(Usage {
                used_bytes: Some(274_877_906_944),
                free_bytes: Some(824_633_720_832),
                allocated_bytes: None,
            })
            .with_property("zfs.health", "ONLINE"),
    );
    graph.add_node(
        Node::new("zfs-dataset:tank/home", NodeKind::ZfsDataset, "tank/home")
            .with_property("zfs.compression", "zstd")
            .with_property("zfs.encryption", "aes-256-gcm")
            .with_property("zfs.keystatus", "available")
            .with_property("zfs.recordsize", "1048576")
            .with_property("zfs.dedup", "off")
            .with_property("zfs.checksum", "sha512")
            .with_property("zfs.primarycache", "metadata"),
    );
    graph.add_node(
        Node::new(
            "bcachefs-device:a2d6fc04-efd0-4e36-aece-2475941d09a3:0",
            NodeKind::BcachefsDevice,
            "/dev/sdc",
        )
        .with_property("bcachefs.device-state", "rw")
        .with_property("bcachefs.device-free", "8589934592"),
    );
    graph.add_edge(Edge::new(
        "block:/dev/nvme0n1p2",
        "btrfs:fs-uuid",
        Relationship::Backs,
    ));
    graph.add_edge(Edge::new(
        "bcachefs-device:a2d6fc04-efd0-4e36-aece-2475941d09a3:0",
        "bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3",
        Relationship::MemberOf,
    ));

    let mut output = Vec::new();
    print_complex_filesystems(&mut output, &graph).expect("complex filesystems table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("BACKING"));
    assert!(output.contains("/mnt/persist"));
    assert!(output.contains("500.0 GiB"));
    assert!(output.contains("40.0%"));
    assert!(output.contains("data-profile=single metadata-profile=DUP"));
    assert!(output.contains("archive"));
    assert!(output.contains("20.0%"));
    assert!(output.contains("bcachefs-mount=/mnt/archive bcachefs-devices=2"));
    assert!(output.contains("tank"));
    assert!(output.contains("health=ONLINE"));
    assert!(output.contains("tank/home"));
    assert!(output.contains(
        "compression=zstd encryption=aes-256-gcm keystatus=available recordsize=1048576"
    ));
    assert!(output.contains("dedup=off checksum=sha512 primarycache=metadata"));
    assert!(output.contains("bcachefs-state=rw bcachefs-device-free=8589934592"));
}

#[test]
fn btrfs_table_includes_subvolume_qgroup_and_json_neighbors() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "block:/dev/nvme0n1p2",
            NodeKind::Partition,
            "/dev/nvme0n1p2",
        )
        .with_property("btrfs.device-id", "1")
        .with_property("btrfs.device-stat-write-io-errs", "1")
        .with_property("btrfs.device-stat-read-io-errs", "2")
        .with_property("btrfs.device-stat-flush-io-errs", "3")
        .with_property("btrfs.device-stat-corruption-errs", "4")
        .with_property("btrfs.device-stat-generation-errs", "5"),
    );
    graph.add_node(
        Node::new("btrfs:fs-uuid", NodeKind::BtrfsFilesystem, "/mnt/persist")
            .with_size_bytes(536_870_912_000)
            .with_usage(Usage {
                used_bytes: Some(214_748_364_800),
                free_bytes: Some(322_122_547_200),
                allocated_bytes: None,
            })
            .with_property("btrfs.mount-target", "/mnt/persist")
            .with_property("btrfs.data-profile", "single")
            .with_property("btrfs.metadata-profile", "DUP"),
    );
    graph.add_node(
        Node::new(
            "btrfs-subvolume:fs:@home",
            NodeKind::BtrfsSubvolume,
            "@home",
        )
        .with_property("btrfs.id", "257")
        .with_property("btrfs.parent-id", "5")
        .with_property("btrfs.top-level", "5")
        .with_property("btrfs.mount-target", "/mnt/persist/@home"),
    );
    graph.add_node(
        Node::new(
            "btrfs-snapshot:fs:@home-before",
            NodeKind::BtrfsSnapshot,
            "@home-before",
        )
        .with_property("btrfs.id", "258")
        .with_property("btrfs.parent-uuid", "home-subvol")
        .with_property("btrfs.received-uuid", "received-home"),
    );
    graph.add_node(
        Node::new("btrfs-qgroup:0/257", NodeKind::BtrfsQgroup, "0/257")
            .with_property("btrfs.qgroup-id", "0/257")
            .with_property("btrfs.qgroup-parents", "0/5")
            .with_property("btrfs.max-referenced", "25GiB")
            .with_property("btrfs.max-exclusive", "10GiB"),
    );
    graph.add_edge(Edge::new(
        "block:/dev/nvme0n1p2",
        "btrfs:fs-uuid",
        Relationship::Backs,
    ));
    graph.add_edge(Edge::new(
        "btrfs:fs-uuid",
        "btrfs-subvolume:fs:@home",
        Relationship::Contains,
    ));
    graph.add_edge(Edge::new(
        "btrfs-subvolume:fs:@home",
        "btrfs-snapshot:fs:@home-before",
        Relationship::SnapshotOf,
    ));

    let mut output = Vec::new();
    print_btrfs(&mut output, &graph).expect("Btrfs table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("MOUNT"));
    assert!(output.contains("/mnt/persist"));
    assert!(output.contains("500.0 GiB"));
    assert!(output.contains("40.0%"));
    assert!(output.contains("/dev/nvme0n1p2"));
    assert!(output.contains("device-id=1 write-io-errs=1 read-io-errs=2"));
    assert!(output.contains("flush-io-errs=3 corruption-errs=4 generation-errs=5"));
    assert!(output.contains("data-profile=single metadata-profile=DUP"));
    assert!(output.contains("@home"));
    assert!(output.contains("subvol-id=257 parent-id=5 top-level=5"));
    assert!(output.contains("@home-before"));
    assert!(output.contains("parent-uuid=home-subvol received-uuid=received-home"));
    assert!(output.contains("qgroup=0/257 qgroup-parents=0/5"));
    assert!(output.contains("max-rfer=25GiB max-excl=10GiB"));

    let mut json = Vec::new();
    print_filtered_json(&mut json, &graph, is_btrfs_node).expect("Btrfs json renders");
    let json = String::from_utf8(json).expect("json is utf8");
    assert!(json.contains("btrfs:fs-uuid"));
    assert!(json.contains("btrfs-subvolume:fs:@home"));
    assert!(json.contains("btrfs-snapshot:fs:@home-before"));
    assert!(json.contains("block:/dev/nvme0n1p2"));
}

#[test]
fn bcachefs_table_includes_member_usage_and_json_neighbors() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3",
            NodeKind::BcachefsFilesystem,
            "archive",
        )
        .with_size_bytes(10_737_418_240)
        .with_usage(Usage {
            used_bytes: Some(2_147_483_648),
            free_bytes: Some(8_589_934_592),
            allocated_bytes: Some(10_737_418_240),
        })
        .with_property(
            "bcachefs.external-uuid",
            "a2d6fc04-efd0-4e36-aece-2475941d09a3",
        )
        .with_property(
            "bcachefs.internal-uuid",
            "55083d1e-27cf-4929-ada4-3fe6e45cf02c",
        )
        .with_property("bcachefs.mount-target", "/mnt/archive")
        .with_property("bcachefs.device-count", "2")
        .with_property("bcachefs.version", "1.20: (unknown version)")
        .with_property("bcachefs.data-user", "2147483648")
        .with_property("bcachefs.data-cached", "1048576"),
    );
    graph.add_node(
        Node::new(
            "bcachefs-device:a2d6fc04-efd0-4e36-aece-2475941d09a3:0",
            NodeKind::BcachefsDevice,
            "/dev/sdc",
        )
        .with_size_bytes(16_000_900_661_248)
        .with_property("bcachefs.device-label", "hdd.archive")
        .with_property("bcachefs.device-state", "rw")
        .with_property("bcachefs.device-free", "1649975230464")
        .with_property("bcachefs.device-capacity", "16000900661248")
        .with_property("bcachefs.device-data-user", "2147483648"),
    );
    graph.add_edge(Edge::new(
        "bcachefs-device:a2d6fc04-efd0-4e36-aece-2475941d09a3:0",
        "bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3",
        Relationship::MemberOf,
    ));

    let filesystem = graph
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::BcachefsFilesystem)
        .expect("bcachefs filesystem exists");
    assert_eq!(member_count(&graph, filesystem), 1);

    let mut output = Vec::new();
    print_bcachefs(&mut output, &graph).expect("bcachefs table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("MEMBERS"));
    assert!(output.contains("archive"));
    assert!(output.contains("10.0 GiB"));
    assert!(output.contains("20.0%"));
    assert!(output.contains("/mnt/archive"));
    assert!(output.contains("bcachefs-uuid=a2d6fc04-efd0-4e36-aece-2475941d09a3"));
    assert!(output.contains("bcachefs-internal=55083d1e-27cf-4929-ada4-3fe6e45cf02c"));
    assert!(output.contains("bcachefs-version=1.20: (unknown version)"));
    assert!(output.contains("bcachefs-user=2147483648 bcachefs-cached=1048576"));
    assert!(output.contains("hdd.archive"));
    assert!(output.contains("14.6 TiB"));
    assert!(output.contains("bcachefs-label=hdd.archive bcachefs-state=rw"));
    assert!(output.contains("bcachefs-device-free=1649975230464"));
    assert!(output.contains("bcachefs-device-user=2147483648"));

    let mut json = Vec::new();
    print_filtered_json(&mut json, &graph, is_bcachefs_node).expect("bcachefs json renders");
    let json = String::from_utf8(json).expect("json is utf8");
    assert!(json.contains("bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3"));
    assert!(json.contains("bcachefs-device:a2d6fc04-efd0-4e36-aece-2475941d09a3:0"));
}
