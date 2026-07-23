#[test]
fn iscsi_table_includes_session_target_lun_and_disk_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "iscsi-session:12",
            NodeKind::IscsiSession,
            "iscsi-session:12",
        )
        .with_property("iscsi.portal", "10.0.0.10:3260,1")
        .with_property("iscsi.target", "iqn.2026-06.example:storage")
        .with_property("iscsi.portal-address", "10.0.0.10")
        .with_property("iscsi.portal-port", "3260")
        .with_property("iscsi.portal-tpgt", "1")
        .with_property("iscsi.persistent-portal", "10.0.0.11:3260,1")
        .with_property("iscsi.persistent-portal-address", "10.0.0.11")
        .with_property("iscsi.persistent-portal-port", "3260")
        .with_property("iscsi.persistent-portal-tpgt", "1")
        .with_property("iscsi.target-portal-group-tag", "1")
        .with_property("iscsi.connection-state", "LOGGED IN")
        .with_property("iscsi.connection-cid", "0")
        .with_property("iscsi.connection-detail-state", "LOGGED IN")
        .with_property("iscsi.connection-local-address", "10.0.0.20")
        .with_property("iscsi.connection-peer-address", "10.0.0.10"),
    );
    graph.add_node(
        Node::new(
            "iscsi-target:iqn.2026-06.example:storage",
            NodeKind::IscsiTarget,
            "iqn.2026-06.example:storage",
        )
        .with_property("iscsi.node-configured", "true")
        .with_property("iscsi.node-portal", "10.0.0.10:3260,1")
        .with_property("iscsi.node-portal-address", "10.0.0.10")
        .with_property("iscsi.node-portal-port", "3260")
        .with_property("iscsi.node-portal-tpgt", "1")
        .with_property("iscsi.node-startup", "automatic")
        .with_property("iscsi.node-iface-name", "default")
        .with_property("iscsi.node-auth-method", "CHAP"),
    );
    graph.add_node(
        Node::new(
            "iscsi-lun:iqn.2026-06.example:storage:0",
            NodeKind::Lun,
            "0",
        )
        .with_path("/dev/sdb")
        .with_size_bytes(1_073_741_824)
        .with_property("iscsi.attached-disk", "sdb")
        .with_property("scsi.address", "4:0:0:0")
        .with_property("scsi.transport", "iscsi")
        .with_property("scsi.generic-device", "/dev/sg2")
        .with_property("scsi.state", "running")
        .with_property("scsi.queue-depth", "64"),
    );
    graph.add_node(
        Node::new("block:/dev/sdb", NodeKind::PhysicalDisk, "/dev/sdb")
            .with_path("/dev/sdb")
            .with_property("iscsi.attached-disk", "sdb"),
    );
    graph.add_edge(Edge::new(
        "iscsi-session:12",
        "iscsi-target:iqn.2026-06.example:storage",
        Relationship::ImportedFrom,
    ));
    graph.add_edge(Edge::new(
        "iscsi-target:iqn.2026-06.example:storage",
        "iscsi-lun:iqn.2026-06.example:storage:0",
        Relationship::Contains,
    ));
    graph.add_edge(Edge::new(
        "iscsi-lun:iqn.2026-06.example:storage:0",
        "block:/dev/sdb",
        Relationship::Backs,
    ));

    let session = graph
        .nodes
        .iter()
        .find(|node| node.id.0 == "iscsi-session:12")
        .expect("session fixture exists");
    let target = graph
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::IscsiTarget)
        .expect("target fixture exists");
    assert_eq!(iscsi_lun_count(&graph, session), 1);
    assert_eq!(iscsi_lun_count(&graph, target), 1);

    let mut output = Vec::new();
    print_iscsi(&mut output, &graph).expect("iscsi table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("PORTAL"));
    assert!(output.contains("STATE"));
    assert!(output.contains("LUNS"));
    assert!(output.contains("PATH"));
    assert!(output.contains("iscsi-session:12"));
    assert!(output.contains("10.0.0.10:3260,1"));
    assert!(output.contains("LOGGED IN"));
    assert!(output
        .lines()
        .any(|line| { line.contains("lun") && line.contains("0") && line.contains("/dev/sdb") }));
    assert!(output.contains("target=iqn.2026-06.example:storage"));
    assert!(output.contains("portal-address=10.0.0.10 portal-port=3260 portal-tpgt=1"));
    assert!(
        output.contains("persistent-portal=10.0.0.11:3260,1 persistent-portal-address=10.0.0.11")
    );
    assert!(output.contains("persistent-portal-port=3260 persistent-portal-tpgt=1"));
    assert!(output.contains("tpgt=1 connection-state=LOGGED IN"));
    assert!(output.contains("cid=0 connection-detail-state=LOGGED IN"));
    assert!(output.contains("local-address=10.0.0.20 peer-address=10.0.0.10"));
    assert!(output.contains("iqn.2026-06.example:storage"));
    assert!(output.contains("configured=true node-portal=10.0.0.10:3260,1"));
    assert!(output.contains("node-portal-address=10.0.0.10 node-portal-port=3260"));
    assert!(output.contains("node-portal-tpgt=1 node-iface=default startup=automatic"));
    assert!(output.contains("auth-method=CHAP"));
    assert!(output.contains("1.0 GiB"));
    assert!(output.contains("attached-disk=sdb"));
    assert!(output.contains("scsi-address=4:0:0:0 scsi-generic=/dev/sg2"));
    assert!(output.contains("scsi-transport=iscsi scsi-state=running scsi-queue-depth=64"));
}

#[test]
fn luns_table_includes_scsi_path_and_json_neighbors() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "iscsi-target:iqn.2026-06.example:storage",
            NodeKind::IscsiTarget,
            "iqn.2026-06.example:storage",
        )
        .with_property("iscsi.node-portal", "10.0.0.10:3260,1"),
    );
    graph.add_node(
        Node::new(
            "iscsi-lun:iqn.2026-06.example:storage:0",
            NodeKind::Lun,
            "0",
        )
        .with_path("/dev/sdb")
        .with_size_bytes(1_073_741_824)
        .with_property("iscsi.attached-disk", "sdb")
        .with_property("iscsi.attached-disk-state", "running")
        .with_property("scsi.address", "4:0:0:0")
        .with_property("scsi.host", "4")
        .with_property("scsi.channel", "0")
        .with_property("scsi.target", "0")
        .with_property("scsi.lun", "0")
        .with_property("scsi.transport", "iscsi")
        .with_property("scsi.generic-device", "/dev/sg2")
        .with_property("scsi.state", "running")
        .with_property("scsi.queue-depth", "64"),
    );
    graph.add_node(Node::new(
        "block:/dev/sdb",
        NodeKind::PhysicalDisk,
        "/dev/sdb",
    ));
    graph.add_edge(Edge::new(
        "iscsi-target:iqn.2026-06.example:storage",
        "iscsi-lun:iqn.2026-06.example:storage:0",
        Relationship::Contains,
    ));
    graph.add_edge(Edge::new(
        "iscsi-lun:iqn.2026-06.example:storage:0",
        "block:/dev/sdb",
        Relationship::Backs,
    ));

    let mut output = Vec::new();
    print_luns(&mut output, &graph).expect("LUN table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("TRANSPORT"));
    assert!(output.contains("GENERIC"));
    assert!(output.contains("1.0 GiB"));
    assert!(output.contains("/dev/sdb"));
    assert!(output.contains("iscsi"));
    assert!(output.contains("/dev/sg2"));
    assert!(output.contains("scsi-address=4:0:0:0 scsi-host=4 scsi-channel=0"));
    assert!(output.contains("scsi-target=0 scsi-lun=0"));
    assert!(output.contains("attached-disk=sdb attached-disk-state=running"));

    let mut json = Vec::new();
    print_filtered_json(&mut json, &graph, is_lun_node).expect("LUN json renders");
    let json = String::from_utf8(json).expect("json is utf8");
    assert!(json.contains("iscsi-lun:iqn.2026-06.example:storage:0"));
    assert!(json.contains("iscsi-target:iqn.2026-06.example:storage"));
    assert!(json.contains("block:/dev/sdb"));
}

#[test]
fn nfs_table_includes_exports_mounts_and_transport_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "nfs-export:storage.example:/export/home",
            NodeKind::NfsExport,
            "storage.example:/export/home",
        )
        .with_property("nfs.server", "storage.example")
        .with_property("nfs.export", "/export/home"),
    );
    graph.add_node(
        Node::new(
            "nfs-export:/srv/share:192.0.2.0/24",
            NodeKind::NfsExport,
            "/srv/share",
        )
        .with_property("nfs.export", "/srv/share")
        .with_property("nfs.export-client", "192.0.2.0/24")
        .with_property("nfs.exportfs", "true")
        .with_property("nfs.export-option-rw", "true")
        .with_property("nfs.export-option-sync", "true")
        .with_property("nfs.export-option-no-subtree-check", "true")
        .with_property("nfs.export-option-sec", "sys")
        .with_property("nfs.export-option-root-squash", "true"),
    );
    graph.add_node(
        Node::new("mount:/home", NodeKind::NfsMount, "/home")
            .with_size_bytes(1_099_511_627_776)
            .with_usage(Usage {
                used_bytes: Some(274_877_906_944),
                free_bytes: Some(824_633_720_832),
                allocated_bytes: None,
            })
            .with_property("nfs.source", "storage.example:/export/home")
            .with_property("nfs.server", "storage.example")
            .with_property("nfs.export", "/export/home")
            .with_property("nfs.vers", "4.2")
            .with_property("nfs.proto", "tcp")
            .with_property("nfs.sec", "sys")
            .with_property("nfs.clientaddr", "10.0.0.20")
            .with_property("nfs.addr", "10.0.0.10")
            .with_property("nfs.port", "2049")
            .with_property("nfs.mountaddr", "10.0.0.10")
            .with_property("nfs.mountvers", "3")
            .with_property("nfs.mountproto", "tcp")
            .with_property("nfs.rsize", "1048576")
            .with_property("nfs.wsize", "1048576")
            .with_property("nfs.timeo", "600")
            .with_property("nfs.retrans", "2")
            .with_property("nfs.local-lock", "none")
            .with_property("nfs.lookupcache", "positive")
            .with_property("nfs.fsc", "true")
            .with_property("nfs.caps", "0x3fffdf")
            .with_property("nfs.wtmult", "512")
            .with_property("nfs.dtsize", "32768")
            .with_property("nfs.bsize", "0")
            .with_property("nfs.flavor", "1")
            .with_property("nfs.pseudoflavor", "1")
            .with_property("nfs.age", "123"),
    );
    graph.add_edge(Edge::new(
        "nfs-export:storage.example:/export/home",
        "mount:/home",
        Relationship::MountedAt,
    ));

    let export = graph
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::NfsExport)
        .expect("export fixture exists");
    let mount = graph
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::NfsMount)
        .expect("mount fixture exists");
    assert_eq!(nfs_mount_count(&graph, export), 1);
    assert_eq!(nfs_mount_count(&graph, mount), 0);

    let mut output = Vec::new();
    print_nfs(&mut output, &graph).expect("nfs table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("SOURCE"));
    assert!(output.contains("SERVER"));
    assert!(output.contains("EXPORT"));
    assert!(output.contains("MOUNTS"));
    assert!(output.contains("storage.example:/export/home"));
    assert!(output.contains("storage.example"));
    assert!(output.contains("/export/home"));
    assert!(output.contains("/home"));
    assert!(output.contains("source=storage.example:/export/home"));
    assert!(output.contains("vers=4.2 proto=tcp sec=sys"));
    assert!(output.contains("clientaddr=10.0.0.20 addr=10.0.0.10 port=2049"));
    assert!(output.contains("mountaddr=10.0.0.10 mountvers=3 mountproto=tcp"));
    assert!(output.contains("rsize=1048576 wsize=1048576 timeo=600 retrans=2"));
    assert!(output.contains("local-lock=none lookupcache=positive fsc=true age=123"));
    assert!(output.contains("caps=0x3fffdf wtmult=512 dtsize=32768 bsize=0"));
    assert!(output.contains("flavor=1 pseudoflavor=1"));
    assert!(output.contains("/srv/share"));
    assert!(output.contains("export-client=192.0.2.0/24 exportfs=true"));
    assert!(output.contains("export-rw=true export-sync=true"));
    assert!(output.contains("export-no-subtree-check=true export-sec=sys"));
    assert!(output.contains("export-root-squash=true"));
}

#[test]
fn network_storage_table_includes_domain_metadata_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("iscsi-session:1", NodeKind::IscsiSession, "iscsi-session:1")
            .with_property("iscsi.portal", "10.0.0.10:3260,1")
            .with_property("iscsi.portal-address", "10.0.0.10")
            .with_property("iscsi.portal-port", "3260")
            .with_property("iscsi.portal-tpgt", "1")
            .with_property("iscsi.persistent-portal", "10.0.0.11:3260,1")
            .with_property("iscsi.persistent-portal-address", "10.0.0.11")
            .with_property("iscsi.persistent-portal-port", "3260")
            .with_property("iscsi.persistent-portal-tpgt", "1")
            .with_property("iscsi.connection-state", "LOGGED IN")
            .with_property("iscsi.session-state", "LOGGED_IN")
            .with_property("iscsi.internal-session-state", "NO CHANGE")
            .with_property("iscsi.iface-name", "default")
            .with_property("iscsi.iface-transport", "tcp")
            .with_property("iscsi.iface-initiator-name", "iqn.2026-06.client:node1")
            .with_property("iscsi.iface-ip-address", "10.0.0.20")
            .with_property("iscsi.iface-netdev", "eno1")
            .with_property("iscsi.host-number", "4")
            .with_property("iscsi.host-state", "running")
            .with_property("iscsi.headerdigest", "None")
            .with_property("iscsi.datadigest", "None")
            .with_property("iscsi.maxrecvdatasegmentlength", "262144")
            .with_property("iscsi.maxburstlength", "262144"),
    );
    graph.add_node(Node::new(
        "iscsi-target:iqn.2026-06.example:storage",
        NodeKind::IscsiTarget,
        "iqn.2026-06.example:storage",
    ));
    graph.add_node(
        Node::new(
            "iscsi-lun:iqn.2026-06.example:storage:0",
            NodeKind::Lun,
            "0",
        )
        .with_size_bytes(1_073_741_824)
        .with_property("iscsi.host-number", "4")
        .with_property("iscsi.scsi-channel", "00")
        .with_property("iscsi.scsi-id", "0")
        .with_property("iscsi.attached-disk", "sdb")
        .with_property("iscsi.attached-disk-state", "running"),
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
    graph.add_node(
        Node::new("mount:/home", NodeKind::NfsMount, "/home")
            .with_property("nfs.source", "storage.example:/export/home")
            .with_property("nfs.server", "storage.example")
            .with_property("nfs.export", "/export/home")
            .with_property("nfs.vers", "4.2")
            .with_property("nfs.proto", "tcp")
            .with_property("nfs.sec", "sys")
            .with_property("nfs.clientaddr", "10.0.0.20")
            .with_property("nfs.addr", "10.0.0.10")
            .with_property("nfs.port", "2049")
            .with_property("nfs.mountaddr", "10.0.0.10")
            .with_property("nfs.mountvers", "3")
            .with_property("nfs.mountproto", "tcp")
            .with_property("nfs.rsize", "1048576")
            .with_property("nfs.wsize", "1048576")
            .with_property("nfs.timeo", "600")
            .with_property("nfs.retrans", "2")
            .with_property("nfs.local-lock", "none")
            .with_property("nfs.lookupcache", "positive")
            .with_property("nfs.fsc", "true")
            .with_property("nfs.caps", "0x3fffdf")
            .with_property("nfs.wtmult", "512")
            .with_property("nfs.dtsize", "32768")
            .with_property("nfs.bsize", "0")
            .with_property("nfs.flavor", "1")
            .with_property("nfs.pseudoflavor", "1")
            .with_property("nfs.age", "123"),
    );

    let mut output = Vec::new();
    print_network_storage(&mut output, &graph).expect("network storage table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("DETAILS"));
    assert!(output.contains("portal=10.0.0.10:3260,1"));
    assert!(output.contains("portal-address=10.0.0.10 portal-port=3260 portal-tpgt=1"));
    assert!(output.contains("persistent-portal=10.0.0.11:3260,1"));
    assert!(output.contains(
        "persistent-portal-address=10.0.0.11 persistent-portal-port=3260 persistent-portal-tpgt=1"
    ));
    assert!(output.contains("connection-state=LOGGED IN"));
    assert!(output.contains("session-state=LOGGED_IN"));
    assert!(output.contains("internal-session-state=NO CHANGE"));
    assert!(output.contains("iface=default transport=tcp"));
    assert!(output.contains("initiator=iqn.2026-06.client:node1"));
    assert!(output.contains("iface-ip=10.0.0.20 netdev=eno1"));
    assert!(output.contains("host=4 host-state=running"));
    assert!(output.contains("header-digest=None data-digest=None"));
    assert!(output.contains("max-recv-data-segment=262144"));
    assert!(output.contains("max-burst=262144"));
    assert!(output.contains("scsi-channel=00 scsi-id=0"));
    assert!(output.contains("attached-disk=sdb attached-disk-state=running"));
    assert!(output.contains("server=storage.example export=/export/home"));
    assert!(output.contains(
        "source=storage.example:/export/home server=storage.example export=/export/home vers=4.2"
    ));
    assert!(output.contains("proto=tcp sec=sys clientaddr=10.0.0.20 addr=10.0.0.10"));
    assert!(output.contains("mountaddr=10.0.0.10 mountvers=3 mountproto=tcp"));
    assert!(output.contains("rsize=1048576 wsize=1048576 timeo=600 retrans=2"));
    assert!(output.contains("local-lock=none lookupcache=positive fsc=true age=123"));
    assert!(output.contains("caps=0x3fffdf wtmult=512 dtsize=32768 bsize=0"));
    assert!(output.contains("flavor=1 pseudoflavor=1"));
}

#[test]
fn snapshots_table_includes_domain_metadata_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(Node::new(
        "dataset:tank/home",
        NodeKind::ZfsDataset,
        "tank/home",
    ));
    graph.add_node(
        Node::new(
            "zfs-snapshot:tank/home@daily",
            NodeKind::ZfsSnapshot,
            "tank/home@daily",
        )
        .with_size_bytes(1_073_741_824)
        .with_property("zfs.userrefs", "2")
        .with_property("zfs.holds", "disk-nix-retain")
        .with_property("zfs.compression", "zstd")
        .with_property("zfs.encryption", "aes-256-gcm")
        .with_property("zfs.keystatus", "available")
        .with_property("zfs.checksum", "sha512")
        .with_property("zfs.copies", "2"),
    );
    graph.add_edge(Edge::new(
        "zfs-snapshot:tank/home@daily",
        "dataset:tank/home",
        Relationship::SnapshotOf,
    ));
    graph.add_node(
        Node::new("lvm-lv:vg/root-snap", NodeKind::LvmSnapshot, "vg/root-snap")
            .with_property("lvm.origin", "root")
            .with_property("lvm.pool", "thinpool")
            .with_property("lvm.data-percent", "12.50"),
    );
    graph.add_node(
        Node::new(
            "btrfs-subvolume:fs:@/.snapshots/1/snapshot",
            NodeKind::BtrfsSnapshot,
            "@/.snapshots/1/snapshot",
        )
        .with_property("btrfs.id", "257")
        .with_property("btrfs.generation", "11")
        .with_property("btrfs.created-generation", "8")
        .with_property("btrfs.parent-id", "256")
        .with_property("btrfs.top-level", "5")
        .with_property("btrfs.parent-uuid", "subvol-root")
        .with_property("btrfs.received-uuid", "received-snap"),
    );

    let mut output = Vec::new();
    print_snapshots(&mut output, &graph).expect("snapshots table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("DETAILS"));
    assert!(output.contains("tank/home"));
    assert!(
            output
                .contains("userrefs=2 holds=disk-nix-retain compression=zstd encryption=aes-256-gcm keystatus=available")
        );
    assert!(output.contains("checksum=sha512 copies=2"));
    assert!(output.contains("data=12.50 origin=root pool=thinpool"));
    assert!(output.contains("subvol-id=257 generation=11 created-generation=8 parent-id=256"));
    assert!(output.contains("top-level=5 parent-uuid=subvol-root received-uuid=received-snap"));
}

#[test]
fn pools_table_includes_domain_metadata_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("zpool:tank", NodeKind::ZfsPool, "tank")
            .with_property("zfs.health", "ONLINE")
            .with_property("zfs.state", "ONLINE"),
    );
    graph.add_node(
        Node::new(
            "zfs-vdev:tank:cache0",
            NodeKind::ZfsVdev,
            "/dev/disk/by-id/cache0",
        )
        .with_property("zfs.vdev-role", "cache")
        .with_property("zfs.vdev-state", "ONLINE")
        .with_property("zfs.read-errors", "0")
        .with_property("zfs.write-errors", "1")
        .with_property("zfs.checksum-errors", "2"),
    );
    graph.add_node(
        Node::new("lvm-vg:vg0", NodeKind::LvmVolumeGroup, "vg0")
            .with_property("lvm.extent-size", "4.00m")
            .with_property("lvm.pv-count", "2")
            .with_property("lvm.lv-count", "8"),
    );
    graph.add_node(
        Node::new("btrfs-qgroup:0/257", NodeKind::BtrfsQgroup, "0/257")
            .with_property("btrfs.qgroup-id", "0/257")
            .with_property("btrfs.qgroup-parents", "0/5")
            .with_property("btrfs.qgroup-children", "1/257")
            .with_property("btrfs.max-referenced", "25GiB"),
    );
    graph.add_node(
        Node::new("md:/dev/md/root", NodeKind::MdRaid, "/dev/md/root")
            .with_property("md.version", "1.2")
            .with_property("md.uuid", "aaaa:bbbb:cccc:dddd")
            .with_property("md.level", "raid1")
            .with_property("md.state", "clean")
            .with_property("md.raid-devices", "2")
            .with_property("md.total-devices", "2")
            .with_property("md.name", "host:root")
            .with_property("md.events", "17"),
    );

    let mut output = Vec::new();
    print_pools(&mut output, &graph).expect("pools table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("DETAILS"));
    assert!(output.contains("health=ONLINE state=ONLINE"));
    assert!(output.contains(
        "vdev-role=cache vdev-state=ONLINE read-errors=0 write-errors=1 checksum-errors=2"
    ));
    assert!(output.contains("extent=4.00m pvs=2 lvs=8"));
    assert!(output.contains("qgroup=0/257 qgroup-parents=0/5 qgroup-children=1/257"));
    assert!(output.contains("max-rfer=25GiB"));
    assert!(output.contains(
            "md-version=1.2 level=raid1 state=clean raid-devices=2 total-devices=2 md-name=host:root events=17"
        ));
}

#[test]
fn encryption_table_includes_luks_header_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "block:/dev/mapper/cryptroot",
            NodeKind::LuksContainer,
            "cryptroot",
        )
        .with_path("/dev/mapper/cryptroot")
        .with_property("cryptsetup.active", "true")
        .with_property("cryptsetup.in-use", "true")
        .with_property("cryptsetup.cipher", "aes-xts-plain64")
        .with_property("cryptsetup.luks-version", "2")
        .with_property("cryptsetup.luks-epoch", "7")
        .with_property("cryptsetup.luks-metadata-area", "16384 [bytes]")
        .with_property("cryptsetup.luks-keyslots-area", "16744448 [bytes]")
        .with_property("cryptsetup.luks-keyslot-count", "2")
        .with_property("cryptsetup.luks-token-count", "1")
        .with_property("cryptsetup.luks-keyslots", "0,1")
        .with_property("cryptsetup.luks-tokens", "0")
        .with_property("cryptsetup.luks-keyslot-0-type", "luks2")
        .with_property("cryptsetup.luks-keyslot-0-priority", "normal")
        .with_property("cryptsetup.luks-keyslot-0-cipher", "aes-xts-plain64")
        .with_property("cryptsetup.luks-keyslot-0-cipher-key", "512 bits")
        .with_property("cryptsetup.luks-keyslot-0-pbkdf", "argon2id")
        .with_property("cryptsetup.luks-keyslot-0-time-cost", "4")
        .with_property("cryptsetup.luks-keyslot-0-memory", "1048576")
        .with_property("cryptsetup.luks-keyslot-0-threads", "4")
        .with_property("cryptsetup.luks-keyslot-0-salt", "00 11 22 33")
        .with_property("cryptsetup.luks-keyslot-0-af-stripes", "4000")
        .with_property("cryptsetup.luks-keyslot-0-area-offset", "32768 [bytes]")
        .with_property("cryptsetup.luks-keyslot-0-area-length", "258048 [bytes]")
        .with_property("cryptsetup.luks-keyslot-0-digest-id", "0")
        .with_property("cryptsetup.luks-keyslot-1-type", "luks2")
        .with_property("cryptsetup.luks-keyslot-1-priority", "ignored")
        .with_property("cryptsetup.luks-token-0-type", "systemd-tpm2")
        .with_property("cryptsetup.luks-token-0-keyslot", "0")
        .with_property("cryptsetup.luks-token-0-keyslots", "0")
        .with_property("cryptsetup.luks-token-0-tpm2-pcrs", "0+7")
        .with_property("cryptsetup.luks-token-0-tpm2-hash", "sha256")
        .with_property("cryptsetup.luks-digest-count", "1")
        .with_property("cryptsetup.luks-digests", "0")
        .with_property("cryptsetup.luks-digest-0-type", "pbkdf2")
        .with_property("cryptsetup.luks-digest-0-hash", "sha256")
        .with_property("cryptsetup.luks-digest-0-iterations", "1000")
        .with_property("cryptsetup.luks-digest-0-salt", "aa bb cc dd")
        .with_property("cryptsetup.luks-digest-0-digest", "ee ff 00 11"),
    );

    let mut output = Vec::new();
    print_encryption(&mut output, &graph).expect("encryption table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("CIPHER"));
    assert!(output.contains("KEYSLOTS"));
    assert!(output.contains("TOKENS"));
    assert!(output.contains("cryptroot"));
    assert!(output.contains("aes-xts-plain64"));
    assert!(output.contains(" 2         "));
    assert!(output.contains(" 1         "));
    assert!(output.contains("active=true in-use=true cipher=aes-xts-plain64"));
    assert!(output.contains("luks=2 epoch=7 metadata-area=16384 [bytes]"));
    assert!(output.contains("keyslot-ids=0,1 token-ids=0"));
    assert!(output
        .contains("keyslot-0=luks2 keyslot-0-priority=normal keyslot-0-cipher=aes-xts-plain64"));
    assert!(output.contains(
            "keyslot-0-cipher-key=512 bits keyslot-0-pbkdf=argon2id keyslot-0-time=4 keyslot-0-memory=1048576 keyslot-0-threads=4"
        ));
    assert!(output.contains("keyslot-0-salt=00 11 22 33 keyslot-0-af-stripes=4000"));
    assert!(output.contains(
            "keyslot-0-area-offset=32768 [bytes] keyslot-0-area-length=258048 [bytes] keyslot-0-digest=0"
        ));
    assert!(output.contains(
        "keyslot-1=luks2 keyslot-1-priority=ignored token-0=systemd-tpm2 token-0-keyslot=0"
    ));
    assert!(output.contains("token-0-keyslots=0 token-0-tpm2-pcrs=0+7 token-0-tpm2-hash=sha256"));
    assert!(output.contains("digests=1 digest-ids=0 digest-0=pbkdf2"));
    assert!(output.contains("digest-0-hash=sha256 digest-0-iterations=1000"));
    assert!(output.contains("digest-0-salt=aa bb cc dd digest-0-digest=ee ff 00 11"));
}

#[test]
fn cache_table_includes_cache_layer_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("block:/dev/bcache0", NodeKind::CacheDevice, "bcache0")
            .with_path("/dev/bcache0")
            .with_property("bcache.role", "backing")
            .with_property("bcache.kind", "cache-set")
            .with_property("bcache.backing-device", "/dev/sdb1")
            .with_property("bcache.set-uuid", "cache-set-uuid")
            .with_property("bcache.set-average-key-size", "16.0k")
            .with_property("bcache.set-root-usage-percent", "3")
            .with_property("bcache.state", "clean")
            .with_property("bcache.running", "1")
            .with_property("bcache.cache-available-percent", "78")
            .with_property("bcache.cache-mode", "writeback")
            .with_property("bcache.cache-replacement-policy", "lru")
            .with_property("bcache.dirty-data", "64.0M")
            .with_property("bcache.io-errors", "0")
            .with_property("bcache.metadata-written", "128.0M")
            .with_property("bcache.writeback-delay", "30")
            .with_property("bcache.writeback-running", "1"),
    );
    graph.add_node(
        Node::new("lvm-lv:vg/root", NodeKind::LvmLogicalVolume, "vg/root")
            .with_property("lvm.cache-mode", "writethrough")
            .with_property("lvm.cache-policy", "smq")
            .with_property("lvm.cache-total-blocks", "4096")
            .with_property("lvm.cache-used-blocks", "1024")
            .with_property("lvm.cache-dirty-blocks", "64")
            .with_property("lvm.cache-read-hits", "1000")
            .with_property("lvm.cache-read-misses", "25")
            .with_property("lvm.cache-write-hits", "900")
            .with_property("lvm.cache-write-misses", "30")
            .with_property("lvm.cache-promotions", "128")
            .with_property("lvm.cache-demotions", "32")
            .with_property("lvm.kernel-cache-settings", "migration_threshold=2048")
            .with_property("lvm.kernel-metadata-format", "2")
            .with_property("lvm.writecache-total-blocks", "1024")
            .with_property("lvm.writecache-free-blocks", "512")
            .with_property("lvm.writecache-writeback-blocks", "16")
            .with_property("lvm.writecache-error", "0"),
    );
    graph.add_node(
        Node::new(
            "zfs-vdev:tank:cache0",
            NodeKind::ZfsVdev,
            "/dev/disk/by-id/cache0",
        )
        .with_property("zfs.vdev-role", "cache")
        .with_property("zfs.vdev-state", "ONLINE"),
    );

    let mut output = Vec::new();
    print_cache(&mut output, &graph).expect("cache table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("MODE"));
    assert!(output.contains("POLICY"));
    assert!(output.contains("DIRTY"));
    assert!(output.contains("bcache0"));
    assert!(output.contains("writeback"));
    assert!(output.contains("lru"));
    assert!(output.contains("backing-device=/dev/sdb1"));
    assert!(output.contains("set-average-key-size=16.0k set-root-usage-percent=3"));
    assert!(output.contains("dirty=64.0M"));
    assert!(output.contains("running=1 available-percent=78"));
    assert!(output.contains("io-errors=0 metadata-written=128.0M"));
    assert!(output.contains("writeback-delay=30"));
    assert!(output.contains("writeback-running=1"));
    assert!(output.contains("vg/root"));
    assert!(output.contains("writethrough"));
    assert!(output.contains("cache-policy=smq"));
    assert!(output.contains("cache-total=4096"));
    assert!(output.contains("cache-used=1024"));
    assert!(output.contains("cache-dirty=64"));
    assert!(output.contains("cache-read-hits=1000"));
    assert!(output.contains("cache-read-misses=25"));
    assert!(output.contains("cache-write-hits=900"));
    assert!(output.contains("cache-write-misses=30"));
    assert!(output.contains("cache-promotions=128"));
    assert!(output.contains("cache-demotions=32"));
    assert!(output.contains("kernel-cache-settings=migration_threshold=2048"));
    assert!(output.contains("kernel-metadata-format=2"));
    assert!(output.contains("writecache-total=1024"));
    assert!(output.contains("writecache-free=512"));
    assert!(output.contains("writecache-writeback=16"));
    assert!(output.contains("writecache-error=0"));
    assert!(output.contains("/dev/disk/by-id/cache0"));
    assert!(output.contains("vdev-role=cache vdev-state=ONLINE"));
}

#[test]
fn vdo_table_includes_vdo_reduction_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("vdo:archive", NodeKind::VdoVolume, "archive")
            .with_size_bytes(1_099_511_627_776)
            .with_usage(Usage {
                used_bytes: Some(268_435_456_000),
                free_bytes: Some(805_306_368_000),
                allocated_bytes: Some(1_073_741_824_000),
            })
            .with_property("vdo.storage-device", "/dev/sdb")
            .with_property("vdo.logical-size", "1T")
            .with_property("vdo.physical-size", "250G")
            .with_property("vdo.stats-size", "268435456")
            .with_property("vdo.stats-used", "134217728")
            .with_property("vdo.stats-available", "134217728")
            .with_property("vdo.use-percent", "50%")
            .with_property("vdo.space-saving-percent", "75%")
            .with_property("vdo.operating-mode", "normal")
            .with_property("vdo.recovery-percentage", "100%")
            .with_property("vdo.write-policy", "sync")
            .with_property("vdo.configured-write-policy", "auto")
            .with_property("vdo.index-memory-setting", "0.25")
            .with_property("vdo.block-map-cache-size", "128M")
            .with_property("vdo.compression", "enabled")
            .with_property("vdo.deduplication", "enabled")
            .with_property("vdo.version", "47")
            .with_property("vdo.release-version", "133524")
            .with_property("vdo.data-blocks-used", "65536")
            .with_property("vdo.data-blocks-used-bytes", "268435456")
            .with_property("vdo.overhead-blocks-used", "4096")
            .with_property("vdo.overhead-blocks-used-bytes", "16777216")
            .with_property("vdo.logical-blocks-used", "262144")
            .with_property("vdo.logical-blocks-used-bytes", "1073741824"),
    );
    graph.add_node(
        Node::new(
            "lvm-seg:vg0/archive:0",
            NodeKind::LvmSegment,
            "vg0/archive:0",
        )
        .with_size_bytes(10 * 1024 * 1024 * 1024)
        .with_usage(Usage {
            used_bytes: Some(8 * 1024 * 1024 * 1024),
            free_bytes: None,
            allocated_bytes: None,
        })
        .with_property("lvm.segment-type", "vdo")
        .with_property("lvm.vdo-operating-mode", "normal")
        .with_property("lvm.vdo-compression", "enabled")
        .with_property("lvm.vdo-compression-state", "online")
        .with_property("lvm.vdo-deduplication", "disabled")
        .with_property("lvm.vdo-index-state", "online")
        .with_property("lvm.vdo-used-size", "8.00g")
        .with_property("lvm.vdo-saving-percent", "42.00")
        .with_property("lvm.vdo-write-policy", "auto"),
    );

    let mut output = Vec::new();
    print_vdo(&mut output, &graph).expect("vdo table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("LOGICAL"));
    assert!(output.contains("PHYSICAL"));
    assert!(output.contains("USED"));
    assert!(output.contains("FREE"));
    assert!(output.contains("USE%"));
    assert!(output.contains("WRITE"));
    assert!(output.contains("archive"));
    assert!(output.contains("          1T"));
    assert!(output.contains("        250G"));
    assert!(output.contains("   250.0 GiB"));
    assert!(output.contains("   750.0 GiB"));
    assert!(output.contains("  24.4%"));
    assert!(output.contains("normal"));
    assert!(output.contains("sync"));
    assert!(output.contains("backing=/dev/sdb logical=1T physical=250G"));
    assert!(output.contains("stats-size=268435456 stats-used=134217728"));
    assert!(output.contains("vdo-use=50% saving=75%"));
    assert!(output.contains("recovery=100% write-policy=sync configured-write-policy=auto"));
    assert!(output.contains("index-memory=0.25 block-map-cache=128M"));
    assert!(output.contains("compression=enabled deduplication=enabled"));
    assert!(output.contains("vdo-version=47 vdo-release=133524"));
    assert!(output.contains("data-blocks=65536 data-bytes=268435456"));
    assert!(output.contains("overhead-blocks=4096 overhead-bytes=16777216"));
    assert!(output.contains("logical-blocks=262144 logical-bytes=1073741824"));
    assert!(output.contains("vg0/archive:0"));
    assert!(output.contains("    10.0 GiB      8.0 GiB      8.0 GiB"));
    assert!(output.contains("vdo-mode=normal"));
    assert!(output.contains("vdo-compression-state=online"));
    assert!(output.contains("vdo-index-state=online"));
    assert!(output.contains("vdo-used=8.00g"));
    assert!(output.contains("vdo-saving=42.00"));
    assert!(output.contains("vdo-write-policy=auto"));
}

#[test]
fn multipath_table_includes_map_and_path_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("multipath:mpatha", NodeKind::MultipathDevice, "mpatha")
            .with_path("/dev/mapper/mpatha")
            .with_property("multipath.dm", "dm-2")
            .with_property("multipath.wwid", "3600508b400105e210000900000490000")
            .with_property("multipath.vendor-product", "IBM,2145")
            .with_property("multipath.size", "100G")
            .with_property("multipath.features", "1 queue_if_no_path")
            .with_property("multipath.hwhandler", "1 alua")
            .with_property("multipath.write-protect", "rw"),
    );
    graph.add_node(
        Node::new("block:/dev/sdb", NodeKind::PhysicalDisk, "/dev/sdb")
            .with_path("/dev/sdb")
            .with_property("multipath.host-path", "2:0:0:1")
            .with_property("multipath.scsi-host", "2")
            .with_property("multipath.scsi-channel", "0")
            .with_property("multipath.scsi-id", "0")
            .with_property("multipath.scsi-lun", "1")
            .with_property("major-minor", "8:16")
            .with_property("multipath.group-policy", "service-time 0")
            .with_property("multipath.group-prio", "50")
            .with_property("multipath.group-status", "active")
            .with_property("multipath.dm-state", "active")
            .with_property("multipath.checker-state", "ready")
            .with_property("multipath.online-state", "running")
            .with_property("multipath.path-flags", "ghost")
            .with_property("multipath.path-state", "active ready running ghost"),
    );
    graph.add_node(
        Node::new("block:/dev/sdc", NodeKind::PhysicalDisk, "/dev/sdc")
            .with_path("/dev/sdc")
            .with_property("multipath.host-path", "3:0:0:1")
            .with_property("multipath.scsi-host", "3")
            .with_property("multipath.scsi-channel", "0")
            .with_property("multipath.scsi-id", "0")
            .with_property("multipath.scsi-lun", "1")
            .with_property("major-minor", "8:32")
            .with_property("multipath.group-policy", "service-time 0")
            .with_property("multipath.group-prio", "10")
            .with_property("multipath.group-status", "enabled")
            .with_property("multipath.dm-state", "active")
            .with_property("multipath.checker-state", "ready")
            .with_property("multipath.online-state", "running")
            .with_property("multipath.path-flags", "faulty shaky")
            .with_property("multipath.path-state", "active ready running faulty shaky"),
    );
    graph.add_edge(Edge::new(
        "block:/dev/sdb",
        "multipath:mpatha",
        Relationship::Backs,
    ));
    graph.add_edge(Edge::new(
        "block:/dev/sdc",
        "multipath:mpatha",
        Relationship::Backs,
    ));

    let mut output = Vec::new();
    print_multipath(&mut output, &graph).expect("multipath table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("WWID"));
    assert!(output.contains("PATHS"));
    assert!(output.contains("GROUP"));
    assert!(output.contains("PATH-STATE"));
    assert!(output.contains("mpatha"));
    assert!(output.contains("3600508b400105e210000900000490000"));
    assert!(output.contains("dm=dm-2 wwid=3600508b400105e210000900000490000"));
    assert!(output.contains("vendor=IBM,2145 size=100G"));
    assert!(output.contains("features=1 queue_if_no_path handler=1 alua wp=rw"));
    assert!(output.contains("/dev/sdb"));
    assert!(output.contains("host-path=2:0:0:1 scsi-host=2"));
    assert!(output.contains("scsi-host=2 scsi-channel=0 scsi-id=0 scsi-lun=1"));
    assert!(output.contains("scsi-lun=1 major-minor=8:16"));
    assert!(output.contains("group-policy=service-time 0 group-prio=50 group-status=active"));
    assert!(output
        .contains("dm-state=active checker-state=ready online-state=running path-flags=ghost"));
    assert!(output.contains("path-state=active ready running ghost"));
    assert!(output.contains("path-flags=faulty shaky"));
    assert!(output.contains("path-state=active ready running faulty shaky"));
    assert!(output.contains("/dev/sdc"));
    assert!(output.contains("host-path=3:0:0:1 scsi-host=3"));
    assert!(output.contains("scsi-host=3 scsi-channel=0 scsi-id=0 scsi-lun=1"));
    assert!(output.contains("scsi-lun=1 major-minor=8:32"));
    assert!(output.contains("group-policy=service-time 0 group-prio=10 group-status=enabled"));
}
