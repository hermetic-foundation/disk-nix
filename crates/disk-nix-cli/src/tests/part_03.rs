#[test]
fn usage_table_includes_details_column() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("vdo:archive", NodeKind::VdoVolume, "archive")
            .with_size_bytes(100)
            .with_usage(Usage {
                used_bytes: Some(50),
                free_bytes: Some(50),
                allocated_bytes: None,
            })
            .with_property("vdo.storage-device", "/dev/sdb")
            .with_property("vdo.logical-size", "100G")
            .with_property("vdo.physical-size", "50G")
            .with_property("vdo.use-percent", "50%")
            .with_property("vdo.space-saving-percent", "20%")
            .with_property("vdo.operating-mode", "normal")
            .with_property("vdo.write-policy", "sync")
            .with_property("vdo.configured-write-policy", "auto")
            .with_property("vdo.block-map-cache-size", "128M")
            .with_property("vdo.data-blocks-used", "65536")
            .with_property("vdo.logical-blocks-used", "262144"),
    );

    let mut output = Vec::new();
    print_usage(&mut output, &graph).expect("usage table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("DETAILS"));
    assert!(output.contains("backing=/dev/sdb logical=100G physical=50G"));
    assert!(output.contains(
        "vdo-use=50% saving=20% mode=normal write-policy=sync configured-write-policy=auto"
    ));
    assert!(output.contains("block-map-cache=128M data-blocks=65536 logical-blocks=262144"));
}

#[test]
fn inspect_includes_capacity_usage_identity_properties_and_relationships() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "filesystem:/srv/archive",
            NodeKind::Filesystem,
            "/srv/archive",
        )
        .with_path("/srv/archive")
        .with_size_bytes(1024)
        .with_usage(Usage {
            used_bytes: Some(256),
            free_bytes: Some(768),
            allocated_bytes: Some(512),
        })
        .with_identity(Identity {
            uuid: Some("fs-uuid".to_string()),
            partuuid: None,
            label: Some("archive".to_string()),
            serial: None,
            wwn: None,
        })
        .with_property("filesystem.type", "xfs")
        .with_property("mount.source", "/dev/mapper/archive"),
    );
    graph.add_node(Node::new(
        "block:/dev/mapper/archive",
        NodeKind::DeviceMapper,
        "/dev/mapper/archive",
    ));
    graph.add_edge(Edge::new(
        "block:/dev/mapper/archive",
        "filesystem:/srv/archive",
        Relationship::Backs,
    ));

    let mut output = Vec::new();
    print_inspect(&mut output, &graph, "archive", 1).expect("inspect renders");
    let output = String::from_utf8(output).expect("inspect output is utf8");

    assert!(output.contains("filesystem /srv/archive"));
    assert!(output.contains("  path: /srv/archive"));
    assert!(output.contains("  size: 1.0 KiB"));
    assert!(output.contains("  usage: used=256 B free=768 B allocated=512 B use=25.0%"));
    assert!(output.contains("    uuid: fs-uuid"));
    assert!(output.contains("    label: archive"));
    assert!(output.contains("    filesystem.type: xfs"));
    assert!(output.contains("    mount.source: /dev/mapper/archive"));
    assert!(output.contains("    in backs block:/dev/mapper/archive (/dev/mapper/archive)"));
}

#[test]
fn inspect_json_depth_walks_layered_relationships() {
    let mut graph = StorageGraph::empty();
    graph.add_node(Node::new(
        "block:/dev/nvme0n1p2",
        NodeKind::Partition,
        "/dev/nvme0n1p2",
    ));
    graph.add_node(Node::new(
        "block:/dev/mapper/cryptroot",
        NodeKind::LuksContainer,
        "cryptroot",
    ));
    graph.add_node(Node::new(
        "lvm-lv:vg/root",
        NodeKind::LvmLogicalVolume,
        "vg/root",
    ));
    graph.add_node(Node::new("filesystem:/", NodeKind::Filesystem, "/"));
    graph.add_edge(Edge::new(
        "block:/dev/nvme0n1p2",
        "block:/dev/mapper/cryptroot",
        Relationship::Backs,
    ));
    graph.add_edge(Edge::new(
        "block:/dev/mapper/cryptroot",
        "lvm-lv:vg/root",
        Relationship::Backs,
    ));
    graph.add_edge(Edge::new(
        "lvm-lv:vg/root",
        "filesystem:/",
        Relationship::Backs,
    ));

    let mut output = Vec::new();
    print_inspect_json(&mut output, &graph, "/", 2).expect("inspect json renders");
    let output = String::from_utf8(output).expect("json is utf8");
    let graph: StorageGraph = serde_json::from_str(&output).expect("valid storage graph json");

    assert_eq!(graph.nodes.len(), 3);
    assert!(graph.nodes.iter().any(|node| node.id.0 == "filesystem:/"));
    assert!(graph.nodes.iter().any(|node| node.id.0 == "lvm-lv:vg/root"));
    assert!(graph
        .nodes
        .iter()
        .any(|node| node.id.0 == "block:/dev/mapper/cryptroot"));
    assert!(graph
        .nodes
        .iter()
        .all(|node| node.id.0 != "block:/dev/nvme0n1p2"));
    assert_eq!(graph.edges.len(), 2);
}

#[test]
fn filesystems_table_includes_metadata_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("fs-source:/dev/mapper/vg-root", NodeKind::Filesystem, "xfs")
            .with_usage(Usage {
                used_bytes: Some(512),
                free_bytes: Some(512),
                allocated_bytes: None,
            })
            .with_property("xfs.meta-data.meta-data", "/dev/mapper/vg-root")
            .with_property("xfs.meta-data.isize", "512")
            .with_property("xfs.meta-data.agcount", "4")
            .with_property("xfs.meta-data.crc", "1")
            .with_property("xfs.data.blocks", "262144")
            .with_property("xfs.data.bsize", "4096")
            .with_property("xfs.data.imaxpct", "25")
            .with_property("xfs.meta-data.reflink", "1")
            .with_property("xfs.meta-data.bigtime", "1")
            .with_property("xfs.naming.version", "2")
            .with_property("xfs.naming.ftype", "1")
            .with_property("xfs.log.type", "internal log")
            .with_property("xfs.log.blocks", "2560")
            .with_property("xfs.realtime.type", "none")
            .with_property("xfs.realtime.blocks", "0"),
    );
    graph.add_node(
        Node::new("fs:/dev/sda2", NodeKind::Filesystem, "ext4")
            .with_property("filesystem.type", "ext4")
            .with_property("ext.state", "clean")
            .with_property("ext.magic-number", "0xEF53")
            .with_property("ext.revision", "1 (dynamic)")
            .with_property("ext.errors-behavior", "Continue")
            .with_property("ext.fs-error-count", "2")
            .with_property("ext.os-type", "Linux")
            .with_property("ext.block-count", "122096646")
            .with_property("ext.reserved-block-count", "6104832")
            .with_property("ext.overhead-clusters", "123456")
            .with_property("ext.free-blocks", "73328197")
            .with_property("ext.first-block", "0")
            .with_property("ext.block-size", "4096")
            .with_property("ext.blocks-per-group", "32768")
            .with_property("ext.inode-count", "30531584")
            .with_property("ext.free-inodes", "27187554")
            .with_property("ext.inodes-per-group", "8192")
            .with_property("ext.raid-stride", "128")
            .with_property("ext.raid-stripe-width", "256")
            .with_property("ext.features", "has_journal extent metadata_csum")
            .with_property("ext.flags", "signed_directory_hash")
            .with_property("ext.default-directory-hash", "half_md4")
            .with_property(
                "ext.directory-hash-seed",
                "11111111-2222-3333-4444-555555555555",
            )
            .with_property("ext.default-mount-options", "user_xattr acl")
            .with_property("ext.mount-count", "12")
            .with_property("ext.maximum-mount-count", "-1")
            .with_property("ext.check-interval", "0 (<none>)")
            .with_property("ext.inode-size", "256")
            .with_property("ext.journal-inode", "8")
            .with_property("ext.journal-uuid", "99999999-aaaa-bbbb-cccc-dddddddddddd")
            .with_property("ext.journal-size", "1024M")
            .with_property("ext.first-error-function", "ext4_lookup")
            .with_property("ext.first-error-block", "9001")
            .with_property("ext.last-error-function", "ext4_journal_check_start")
            .with_property("ext.last-error-block", "9002")
            .with_property("ext.checksum-type", "crc32c")
            .with_property("ext.checksum", "0x12345678"),
    );
    graph.add_node(
        Node::new("fs:/dev/sdb1", NodeKind::Filesystem, "exfat")
            .with_property("exfat.guid", "01234567-89ab-cdef-0123-456789abcdef")
            .with_property("exfat.volume-label", "SHARED")
            .with_property("exfat.exfatprogs-version", "1.2.4")
            .with_property("exfat.volume-serial", "0x6eef953b")
            .with_property("exfat.volume-length-sectors", "3203072")
            .with_property("exfat.fat-offset-sector-offset", "2048")
            .with_property("exfat.fat-length-sectors", "448")
            .with_property("exfat.cluster-heap-offset-sector-offset", "4096")
            .with_property("exfat.cluster-count", "49984")
            .with_property("exfat.used-clusters", "48960")
            .with_property("exfat.free-clusters", "1024")
            .with_property("exfat.root-cluster-cluster-offset", "4")
            .with_property("exfat.bytes-per-sector", "512")
            .with_property("exfat.sectors-per-cluster", "64")
            .with_property("exfat.bytes-per-cluster", "32768"),
    );
    graph.add_node(
        Node::new("btrfs:fs-uuid", NodeKind::BtrfsFilesystem, "data")
            .with_property("btrfs.mount-target", "/data")
            .with_property("btrfs.data-profile", "single")
            .with_property("btrfs.data-size", "512")
            .with_property("btrfs.data-used", "400")
            .with_property("btrfs.metadata-profile", "DUP")
            .with_property("btrfs.metadata-size", "128")
            .with_property("btrfs.metadata-used", "64")
            .with_property("btrfs.system-profile", "DUP")
            .with_property("btrfs.system-size", "64")
            .with_property("btrfs.system-used", "32"),
    );
    graph.add_node(
        Node::new("fs:/dev/sda1", NodeKind::Filesystem, "ntfs")
            .with_property("ntfs.device-name", "/dev/sda1")
            .with_property("ntfs.device-state", "11")
            .with_property("ntfs.volume-name", "Windows")
            .with_property("ntfs.volume-serial", "01234567-89abcdef")
            .with_property("ntfs.version", "3.1")
            .with_property("ntfs.cluster-size", "4096")
            .with_property("ntfs.mft-record-size", "1024")
            .with_property("ntfs.mft-zone-multiplier", "0")
            .with_property("ntfs.mft-zone-start", "786432")
            .with_property("ntfs.mft-zone-end", "819200")
            .with_property("ntfs.mft-data-position", "786944")
            .with_property("ntfs.mft-lcn", "4"),
    );
    graph.add_node(
        Node::new("fs:/dev/sdb2", NodeKind::Filesystem, "f2fs")
            .with_property("f2fs.filesystem-volume-name", "mobile")
            .with_property("f2fs.block-size", "4096")
            .with_property("f2fs.block-count", "262144")
            .with_property("f2fs.user-block-count", "245760")
            .with_property("f2fs.valid-block-count", "65536")
            .with_property("f2fs.segment-count", "2048")
            .with_property("f2fs.segment-count-main", "1984")
            .with_property("f2fs.segment-count-ckpt", "2")
            .with_property("f2fs.segment-count-sit", "2")
            .with_property("f2fs.segment-count-nat", "4")
            .with_property("f2fs.segment-count-ssa", "1")
            .with_property("f2fs.overprov-segment-count", "64")
            .with_property("f2fs.section-count", "1984")
            .with_property("f2fs.segs-per-sec", "1")
            .with_property("f2fs.secs-per-zone", "1")
            .with_property("f2fs.version", "Linux version 6.12"),
    );
    graph.add_node(
        Node::new(
            "bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3",
            NodeKind::BcachefsFilesystem,
            "archive",
        )
        .with_property(
            "bcachefs.external-uuid",
            "a2d6fc04-efd0-4e36-aece-2475941d09a3",
        )
        .with_property("bcachefs.member-device", "/dev/sdc")
        .with_property("bcachefs.mount-target", "/mnt/archive")
        .with_property("bcachefs.device-index", "6")
        .with_property(
            "bcachefs.magic-number",
            "c68573f6-66ce-90a9-d96a-60cf803df7ef",
        )
        .with_property(
            "bcachefs.version-upgrade-complete",
            "1.20: (unknown version)",
        )
        .with_property("bcachefs.data-sb", "3149824")
        .with_property("bcachefs.data-journal", "4294967296")
        .with_property("bcachefs.data-user", "2147483648"),
    );

    let mut output = Vec::new();
    print_filesystems(&mut output, &graph).expect("filesystems table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("DETAILS"));
    assert!(output.contains("xfs-source=/dev/mapper/vg-root xfs-isize=512 xfs-agcount=4"));
    assert!(output.contains("xfs-crc=1 xfs-blocks=262144 xfs-bsize=4096"));
    assert!(output.contains("xfs-imaxpct=25 reflink=1 bigtime=1"));
    assert!(output
        .contains("xfs-naming-version=2 xfs-ftype=1 xfs-log-type=internal log log-blocks=2560"));
    assert!(output.contains("xfs-realtime-type=none xfs-realtime-blocks=0"));
    assert!(output.contains(
            "fstype=ext4 ext-state=clean ext-magic=0xEF53 ext-revision=1 (dynamic) errors=Continue fs-error-count=2 os=Linux blocks=122096646 reserved-blocks=6104832 overhead-clusters=123456 free-blocks=73328197 first-block=0"
        ));
    assert!(output.contains(
            "first-error-function=ext4_lookup first-error-block=9001 last-error-function=ext4_journal_check_start last-error-block=9002"
        ));
    assert!(output.contains(
            "block-size=4096 blocks-per-group=32768 inodes=30531584 free-inodes=27187554 inodes-per-group=8192 raid-stride=128 raid-stripe-width=256"
        ));
    assert!(output.contains(
            "features=has_journal extent metadata_csum flags=signed_directory_hash dir-hash=half_md4 dir-hash-seed=11111111-2222-3333-4444-555555555555"
        ));
    assert!(output.contains("default-mount=user_xattr acl"));
    assert!(output.contains(
            "mount-count=12 max-mount-count=-1 check-interval=0 (<none>) inode-size=256 journal-inode=8 journal-uuid=99999999-aaaa-bbbb-cccc-dddddddddddd"
        ));
    assert!(output.contains("journal-size=1024M"));
    assert!(output.contains("checksum-type=crc32c checksum=0x12345678"));
    assert!(output.contains(
            "guid=01234567-89ab-cdef-0123-456789abcdef exfat-label=SHARED exfatprogs=1.2.4 serial=0x6eef953b sectors=3203072"
        ));
    assert!(output.contains("fat-offset=2048 fat-length=448 cluster-heap-offset=4096"));
    assert!(output.contains("clusters=49984 used-clusters=48960 free-clusters=1024 root-cluster=4"));
    assert!(output.contains("sector-bytes=512 sectors-per-cluster=64 cluster-bytes=32768"));
    assert!(output.contains(
        "mount-target=/data data-profile=single data-size=512 data-used=400 metadata-profile=DUP"
    ));
    assert!(output.contains(
        "metadata-size=128 metadata-used=64 system-profile=DUP system-size=64 system-used=32"
    ));
    assert!(
            output.contains(
                "ntfs-device=/dev/sda1 ntfs-device-state=11 ntfs-name=Windows ntfs-serial=01234567-89abcdef ntfs-version=3.1 ntfs-cluster=4096 ntfs-mft-record=1024"
            )
        );
    assert!(output.contains(
            "ntfs-mft-zone-multiplier=0 ntfs-mft-zone-start=786432 ntfs-mft-zone-end=819200 ntfs-mft-data-position=786944 ntfs-mft-lcn=4"
        ));
    assert!(output.contains(
            "f2fs-name=mobile f2fs-block-size=4096 f2fs-blocks=262144 f2fs-user-blocks=245760 f2fs-valid-blocks=65536"
        ));
    assert!(output.contains(
            "f2fs-segments=2048 f2fs-main-segments=1984 f2fs-ckpt-segments=2 f2fs-sit-segments=2 f2fs-nat-segments=4 f2fs-ssa-segments=1"
        ));
    assert!(output.contains(
            "f2fs-overprov=64 f2fs-sections=1984 f2fs-segs-per-sec=1 f2fs-secs-per-zone=1 f2fs-version=Linux version 6.12"
        ));
    assert!(output.contains(
            "bcachefs-uuid=a2d6fc04-efd0-4e36-aece-2475941d09a3 bcachefs-magic=c68573f6-66ce-90a9-d96a-60cf803df7ef bcachefs-member=/dev/sdc bcachefs-mount=/mnt/archive"
        ));
    assert!(output.contains(
            "bcachefs-device=6 bcachefs-upgrade-complete=1.20: (unknown version) bcachefs-sb=3149824 bcachefs-journal=4294967296 bcachefs-user=2147483648"
        ));
}

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

#[test]
fn volumes_table_includes_domain_metadata_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("lvm-lv:vg/root-snap", NodeKind::LvmSnapshot, "vg/root-snap")
            .with_property("lvm.origin", "root")
            .with_property("lvm.pool", "thinpool")
            .with_property("lvm.data-percent", "12.50")
            .with_property("lvm.active", "active")
            .with_property("lvm.layout", "snapshot")
            .with_property("lvm.health", "partial")
            .with_property("lvm.tags", "backup,snapshot")
            .with_property("lvm.cache-mode", "writeback")
            .with_property("lvm.cache-policy", "smq"),
    );
    graph.add_node(
        Node::new("md:/dev/md/root", NodeKind::MdRaid, "/dev/md/root")
            .with_property("md.level", "raid1")
            .with_property("md.state", "clean")
            .with_property("md.raid-devices", "2"),
    );
    graph.add_node(
        Node::new("iscsi-lun:iqn.example:0", NodeKind::Lun, "0")
            .with_property("iscsi.attached-disk", "sdb"),
    );
    graph.add_node(
        Node::new(
            "nfs-export:storage.example:/export/home",
            NodeKind::NfsExport,
            "storage.example:/export/home",
        )
        .with_property("nfs.server", "storage.example")
        .with_property("nfs.export", "/export/home"),
    );

    let mut output = Vec::new();
    print_volumes(&mut output, &graph).expect("volumes table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("DETAILS"));
    assert!(output.contains(
            "data=12.50 layout=snapshot origin=root pool=thinpool active=active health=partial tags=backup,snapshot cache-mode=writeback cache-policy=smq"
        ));
    assert!(output.contains("level=raid1 state=clean raid-devices=2"));
    assert!(output.contains("attached-disk=sdb"));
    assert!(output.contains("server=storage.example export=/export/home"));
}

#[test]
fn lvm_table_includes_volume_group_and_segment_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "lvm-pv:/dev/nvme0n1p3",
            NodeKind::LvmPhysicalVolume,
            "/dev/nvme0n1p3",
        )
        .with_path("/dev/nvme0n1p3")
        .with_size_bytes(536_870_912_000)
        .with_property("lvm.active", "active")
        .with_property("lvm.pv-format", "lvm2")
        .with_property("lvm.dev-size", "500.00g")
        .with_property("lvm.pe-start", "1.00m")
        .with_property("lvm.pv-missing", "missing")
        .with_property("lvm.pv-pe-count", "128000")
        .with_property("lvm.pv-pe-allocated", "102400")
        .with_property("lvm.pv-mda-free", "1020.00k")
        .with_property("lvm.pv-device-id", "wwn-0x1234")
        .with_property("lvm.tags", "ssd,system"),
    );
    graph.add_node(
        Node::new("lvm-vg:vg0", NodeKind::LvmVolumeGroup, "vg0")
            .with_size_bytes(1_099_511_627_776)
            .with_property("lvm.vg-format", "lvm2")
            .with_property("lvm.permissions", "writeable")
            .with_property("lvm.vg-autoactivation", "enabled")
            .with_property("lvm.allocation-policy", "normal")
            .with_property("lvm.vg-system-id", "host-a")
            .with_property("lvm.vg-lock-type", "none")
            .with_property("lvm.extent-size", "4.00m")
            .with_property("lvm.extent-count", "262144")
            .with_property("lvm.free-count", "5120")
            .with_property("lvm.pv-count", "2")
            .with_property("lvm.missing-pv-count", "1")
            .with_property("lvm.lv-count", "5")
            .with_property("lvm.snapshot-count", "1")
            .with_property("lvm.vg-seqno", "17")
            .with_property("lvm.vg-mda-free", "1020.00k")
            .with_property("lvm.vg-mda-copies", "unmanaged"),
    );
    graph.add_node(
        Node::new("lvm-thin-pool:vg0/pool", NodeKind::LvmThinPool, "vg0/pool")
            .with_size_bytes(858_993_459_200)
            .with_property("lvm.data-percent", "42.00")
            .with_property("lvm.metadata-percent", "7.50")
            .with_property("lvm.active", "active")
            .with_property("lvm.when-full", "queue")
            .with_property("lvm.metadata-size", "8.00g"),
    );
    graph.add_node(
        Node::new("lvm-lv:vg0/root", NodeKind::LvmLogicalVolume, "vg0/root")
            .with_size_bytes(214_748_364_800)
            .with_property("lvm.active", "active")
            .with_property("lvm.active-locally", "active locally")
            .with_property("lvm.active-exclusively", "active exclusively")
            .with_property("lvm.layout", "thin")
            .with_property("lvm.pool", "pool")
            .with_property("lvm.dm-path", "/dev/mapper/vg0-root")
            .with_property("lvm.read-ahead", "auto")
            .with_property("lvm.kernel-read-ahead", "256")
            .with_property("lvm.suspended", "not suspended")
            .with_property("lvm.live-table", "live")
            .with_property("lvm.modules", "thin")
            .with_property("lvm.host", "host-a")
            .with_property("lvm.health", "ok"),
    );
    graph.add_node(
        Node::new(
            "lvm-snapshot:vg0/root-snap",
            NodeKind::LvmSnapshot,
            "vg0/root-snap",
        )
        .with_property("lvm.origin", "root")
        .with_property("lvm.snap-percent", "12.50")
        .with_property("lvm.active", "active"),
    );
    graph.add_node(
        Node::new("lvm-cache:vg0/root", NodeKind::LvmCache, "vg0/root")
            .with_property("lvm.cache-mode", "writeback")
            .with_property("lvm.cache-policy", "smq")
            .with_property("lvm.raid-mismatch-count", "2")
            .with_property("lvm.raid-sync-action", "repair")
            .with_property("lvm.raid-write-behind", "256")
            .with_property("lvm.raid-min-recovery-rate", "1024")
            .with_property("lvm.raid-max-recovery-rate", "8192")
            .with_property("lvm.raid-integrity-mode", "journal")
            .with_property("lvm.raid-integrity-block-size", "4096")
            .with_property("lvm.raid-integrity-mismatches", "1")
            .with_property("lvm.writecache-block-size", "4096")
            .with_property("lvm.writecache-writeback-blocks", "16"),
    );
    graph.add_node(
        Node::new("lvm-segment:vg0/root:0", NodeKind::LvmSegment, "vg0/root:0")
            .with_property("lvm.segment-type", "thin")
            .with_property("lvm.segment-stripes", "2")
            .with_property("lvm.segment-data-stripes", "2")
            .with_property("lvm.reshape-length", "128.00m")
            .with_property("lvm.data-copies", "2")
            .with_property("lvm.stripe-size", "64.00k")
            .with_property("lvm.segment-start", "0")
            .with_property("lvm.segment-size", "200.00g")
            .with_property("lvm.segment-size-extents", "51200")
            .with_property("lvm.devices", "pool(0)")
            .with_property("lvm.segment-le-ranges", "0-51199")
            .with_property("lvm.segment-metadata-le-ranges", "pool_tmeta:0-31")
            .with_property("lvm.integrity-settings", "journal_sectors=2048")
            .with_property("lvm.vdo-block-map-cache-size", "128.00m")
            .with_property("lvm.vdo-use-sparse-index", "enabled")
            .with_property("lvm.vdo-bio-threads", "4")
            .with_property("lvm.vdo-max-discard", "4.00m"),
    );
    graph.add_edge(Edge::new(
        "lvm-pv:/dev/nvme0n1p3",
        "lvm-vg:vg0",
        Relationship::MemberOf,
    ));
    graph.add_edge(Edge::new(
        "lvm-thin-pool:vg0/pool",
        "lvm-lv:vg0/root",
        Relationship::Backs,
    ));

    let mut output = Vec::new();
    print_lvm(&mut output, &graph).expect("lvm table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("DATA%"));
    assert!(output.contains("META%"));
    assert!(output.contains("/dev/nvme0n1p3"));
    assert!(output.contains("active"));
    assert!(output.contains("tags=ssd,system"));
    assert!(output.contains("pv-format=lvm2 dev-size=500.00g"));
    assert!(output.contains("pe-start=1.00m pv-missing=missing pv-extents=128000"));
    assert!(output.contains("pv-extents-used=102400 pv-mda-free=1020.00k"));
    assert!(output.contains("pv-device-id=wwn-0x1234"));
    assert!(output.contains("vg-format=lvm2"));
    assert!(output.contains("permissions=writeable"));
    assert!(output.contains("vg-autoactivation=enabled allocation=normal"));
    assert!(output.contains("system-id=host-a lock-type=none"));
    assert!(output.contains("extent=4.00m extents=262144 free-extents=5120"));
    assert!(output.contains("pvs=2 missing-pvs=1 lvs=5 snapshots=1 seqno=17"));
    assert!(output.contains("vg-mda-free=1020.00k vg-mda-copies=unmanaged"));
    assert!(output.contains("42.00"));
    assert!(output.contains("7.50"));
    assert!(output.contains("when-full=queue metadata-size=8.00g"));
    assert!(output.contains("layout=thin pool=pool active=active active-local=active locally"));
    assert!(output.contains("active-exclusive=active exclusively"));
    assert!(output.contains("dm-path=/dev/mapper/vg0-root read-ahead=auto"));
    assert!(output.contains("kernel-read-ahead=256 suspended=not suspended"));
    assert!(output.contains("live-table=live modules=thin host=host-a"));
    assert!(output.contains("health=ok"));
    assert!(output.contains("snap=12.50 origin=root active=active"));
    assert!(output.contains("raid-mismatches=2 raid-sync=repair"));
    assert!(output.contains("raid-write-behind=256 raid-min-recovery=1024"));
    assert!(output.contains("raid-max-recovery=8192 raid-integrity=journal"));
    assert!(output.contains("raid-integrity-block=4096 raid-integrity-mismatches=1"));
    assert!(output.contains("cache-mode=writeback cache-policy=smq"));
    assert!(output.contains("writecache-writeback=16 writecache-block-size=4096"));
    assert!(output.contains("segment-type=thin stripes=2 data-stripes=2"));
    assert!(output.contains("reshape-length=128.00m data-copies=2"));
    assert!(output.contains("stripe-size=64.00k segment-start=0 segment-size=200.00g"));
    assert!(output.contains("segment-size-pe=51200 devices=pool(0) le-ranges=0-51199"));
    assert!(output.contains("metadata-le-ranges=pool_tmeta:0-31"));
    assert!(output.contains("integrity-settings=journal_sectors=2048"));
    assert!(output.contains("vdo-block-map-cache=128.00m vdo-sparse-index=enabled"));
    assert!(output.contains("vdo-bio-threads=4 vdo-max-discard=4.00m"));
}
