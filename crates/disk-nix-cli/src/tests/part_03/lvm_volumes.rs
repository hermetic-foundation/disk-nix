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
