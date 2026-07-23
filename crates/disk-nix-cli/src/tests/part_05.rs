#[test]
fn nvme_table_includes_namespace_identity_and_geometry() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("nvme-controller:nvme0", NodeKind::NvmeController, "nvme0")
            .with_path("/dev/nvme0")
            .with_identity(Identity {
                serial: Some("SERIAL123".to_string()),
                ..Identity::default()
            })
            .with_property("nvme.controller", "nvme0")
            .with_property("nvme.model", "Example NVMe")
            .with_property("nvme.firmware", "1.0")
            .with_property("nvme.subsystem", "nqn.2014-08.org.nvmexpress:uuid:12345678")
            .with_property("nvme.controller-id", "1")
            .with_property("nvme.id-ctrl.vid", "5197")
            .with_property("nvme.id-ctrl.ssvid", "5197")
            .with_property("nvme.id-ctrl.mdts", "9")
            .with_property("nvme.id-ctrl.controller-type", "1")
            .with_property("nvme.id-ctrl.oacs", "31")
            .with_property("nvme.id-ctrl.fuses", "1")
            .with_property("nvme.id-ctrl.fna", "4")
            .with_property("nvme.id-ctrl.awun", "255")
            .with_property("nvme.id-ctrl.awupf", "0")
            .with_property("nvme.id-ctrl.acwu", "0")
            .with_property("nvme.id-ctrl.sgls", "131073")
            .with_property("nvme.id-ctrl.namespace-set-id-max", "32")
            .with_property("nvme.id-ctrl.endurance-group-id-max", "8")
            .with_property("nvme.id-ctrl.ana-transition-time", "10")
            .with_property("nvme.id-ctrl.ana-group-max", "4")
            .with_property("nvme.id-ctrl.persistent-event-log-size", "4096")
            .with_property("nvme.id-ctrl.domain-id", "2")
            .with_property("nvme.id-ctrl.warning-composite-temp", "343")
            .with_property("nvme.id-ctrl.critical-composite-temp", "353")
            .with_property("nvme.id-ctrl.minimum-thermal-management-temp", "273")
            .with_property("nvme.id-ctrl.maximum-thermal-management-temp", "358")
            .with_property("nvme.id-ctrl.total-nvm-capacity", "1000000000")
            .with_property("nvme.id-ctrl.unallocated-nvm-capacity", "500000000")
            .with_property("nvme.id-ctrl.namespace-count", "16")
            .with_property("nvme.id-ctrl.oncs", "95")
            .with_property("nvme.id-ctrl.volatile-write-cache", "1")
            .with_property("nvme.id-ctrl.sanitize-capabilities", "7")
            .with_property("nvme.id-ctrl.ana-capabilities", "3")
            .with_property("nvme.smart.critical-warning", "0")
            .with_property("nvme.smart.temperature-kelvin", "301")
            .with_property("nvme.smart.available-spare-percent", "100")
            .with_property("nvme.smart.percent-used", "2")
            .with_property("nvme.smart.data-units-read", "123456")
            .with_property("nvme.smart.data-units-written", "654321")
            .with_property("nvme.smart.power-on-hours", "1200")
            .with_property("nvme.smart.unsafe-shutdowns", "3")
            .with_property("nvme.smart.media-errors", "0")
            .with_property("nvme.smart.error-log-entries", "4")
            .with_property("nvme.smart.temperature-sensor-1-kelvin", "300")
            .with_property("nvme.smart.temperature-sensor-2-kelvin", "302")
            .with_property("nvme.smart.temperature-sensor-3-kelvin", "303")
            .with_property("nvme.smart.temperature-sensor-4-kelvin", "304")
            .with_property("nvme.smart.thermal-temp1-transition-count", "5")
            .with_property("nvme.smart.thermal-temp2-transition-count", "6")
            .with_property("nvme.smart.thermal-temp1-total-time", "70")
            .with_property("nvme.smart.thermal-temp2-total-time", "80"),
    );
    graph.add_node(
        Node::new(
            "block:/dev/nvme0n1",
            NodeKind::NvmeNamespace,
            "/dev/nvme0n1",
        )
        .with_path("/dev/nvme0n1")
        .with_size_bytes(1_000_000_000_000)
        .with_usage(Usage {
            used_bytes: Some(400_000_000_000),
            free_bytes: Some(600_000_000_000),
            allocated_bytes: Some(400_000_000_000),
        })
        .with_identity(Identity {
            serial: Some("SERIAL123".to_string()),
            ..Identity::default()
        })
        .with_property("nvme.generic-path", "/dev/ng0n1")
        .with_property("nvme.model", "Example NVMe")
        .with_property("nvme.product", "Example Controller")
        .with_property("nvme.firmware", "1.0")
        .with_property("nvme.index", "0")
        .with_property("nvme.namespace", "1")
        .with_property("nvme.namespace-id", "1")
        .with_property(
            "nvme.namespace-uuid",
            "12345678-1234-1234-1234-123456789abc",
        )
        .with_property("nvme.eui64", "0011223344556677")
        .with_property("nvme.nguid", "00112233445566778899aabbccddeeff")
        .with_property("nvme.subsystem", "nvme-subsys0")
        .with_property("nvme.controller", "nvme0")
        .with_property("nvme.address", "0000:01:00.0")
        .with_property("nvme.transport", "pcie")
        .with_property("nvme.controller-id", "1")
        .with_property("nvme.namespace-capacity", "900000000000")
        .with_property("nvme.lba-format", "512 B + 0 B")
        .with_property("nvme.maximum-lba", "1953125")
        .with_property("nvme.sector-size", "512")
        .with_property("nvme.ana-state", "optimized")
        .with_property("nvme.formatted-lba-index", "0")
        .with_property("nvme.formatted-lba-data-size", "512")
        .with_property("nvme.formatted-lba-metadata-size", "0")
        .with_property("nvme.formatted-lba-relative-performance", "0")
        .with_property("nvme.id-ns.nsze", "1953125")
        .with_property("nvme.id-ns.ncap", "1800000")
        .with_property("nvme.id-ns.nuse", "900000")
        .with_property("nvme.id-ns.nsfeat", "0")
        .with_property("nvme.id-ns.nlbaf", "1")
        .with_property("nvme.id-ns.flbas", "0")
        .with_property("nvme.id-ns.nmic", "1")
        .with_property("nvme.id-ns.nvmcap", "1000000000"),
    );

    let mut output = Vec::new();
    print_nvme(&mut output, &graph).expect("nvme table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("SERIAL"));
    assert!(output.contains("CONTROLLER"));
    assert!(output.contains("USE%"));
    assert!(output.contains("nvme-controller"));
    assert!(output.contains("nvme0"));
    assert!(output.contains("nqn.2014-08.org.nvmexpress:uuid:12345678"));
    assert!(output.contains("vid=5197 ssvid=5197 mdts=9 controller-type=1"));
    assert!(
        output.contains("optional-admin-commands=31 fused-operations=1 format-nvm-attributes=4")
    );
    assert!(output.contains(
            "atomic-write-unit-normal=255 atomic-write-unit-powerfail=0 atomic-compare-write-unit=0 sgl-support=131073"
        ));
    assert!(output.contains(
        "namespace-set-id-max=32 endurance-group-id-max=8 ana-transition-time=10 ana-group-max=4"
    ));
    assert!(output.contains("persistent-event-log-size=4096 domain-id=2"));
    assert!(output.contains(
            "warning-composite-temp=343 critical-composite-temp=353 min-thermal-management-temp=273 max-thermal-management-temp=358"
        ));
    assert!(output.contains("total-nvm-capacity=1000000000 unallocated-nvm-capacity=500000000"));
    assert!(output.contains("namespace-count=16 oncs=95 volatile-write-cache=1"));
    assert!(output.contains("sanitize-capabilities=7 ana-capabilities=3"));
    assert!(output.contains("critical-warning=0 temperature-k=301 available-spare-percent=100"));
    assert!(output.contains("percent-used=2 data-units-read=123456"));
    assert!(output.contains("data-units-written=654321"));
    assert!(output.contains("power-on-hours=1200 unsafe-shutdowns=3 media-errors=0"));
    assert!(output.contains("error-log-entries=4 temp-sensor-1-k=300 temp-sensor-2-k=302"));
    assert!(output.contains("temp-sensor-3-k=303 temp-sensor-4-k=304"));
    assert!(output.contains(
            "thermal-temp1-transitions=5 thermal-temp2-transitions=6 thermal-temp1-total-time=70 thermal-temp2-total-time=80"
        ));
    assert!(output.contains("/dev/nvme0n1"));
    assert!(output.contains("SERIAL123"));
    assert!(output.contains("nvme0"));
    assert!(output.contains("40.0%"));
    assert!(output.contains("generic=/dev/ng0n1 nvme-model=Example NVMe"));
    assert!(output.contains("product=Example Controller firmware=1.0"));
    assert!(output
        .contains("ns-index=0 namespace=1 nsid=1 ns-uuid=12345678-1234-1234-1234-123456789abc"));
    assert!(output.contains(
        "eui64=0011223344556677 nguid=00112233445566778899aabbccddeeff subsystem=nvme-subsys0"
    ));
    assert!(output.contains("controller=nvme0 address=0000:01:00.0"));
    assert!(output.contains(
        "transport=pcie controller-id=1 namespace-capacity=900000000000 lba-format=512 B + 0 B"
    ));
    assert!(output.contains("max-lba=1953125 sector-size=512 ana-state=optimized"));
    assert!(
        output.contains("flba-index=0 flba-data=512 flba-metadata=0 flba-relative-performance=0")
    );
    assert!(output.contains("nsze=1953125 ncap=1800000 nuse=900000 nsfeat=0"));
    assert!(output.contains("nlbaf=1 flbas=0 nmic=1 nvmcap=1000000000"));
}

#[test]
fn raid_table_includes_array_and_member_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("md:/dev/md0", NodeKind::MdRaid, "/dev/md0")
            .with_path("/dev/md0")
            .with_size_bytes(1_071_644_672)
            .with_identity(Identity {
                uuid: Some("aaaa:bbbb:cccc:dddd".to_string()),
                ..Identity::default()
            })
            .with_property("md.version", "1.2")
            .with_property("md.uuid", "aaaa:bbbb:cccc:dddd")
            .with_property("md.level", "raid1")
            .with_property("md.state", "clean")
            .with_property("md.raid-devices", "2")
            .with_property("md.total-devices", "2")
            .with_property("md.array-devices", "2")
            .with_property("md.active-devices", "1")
            .with_property("md.working-devices", "2")
            .with_property("md.failed-devices", "1")
            .with_property("md.spare-devices", "1")
            .with_property("md.degraded-devices", "1")
            .with_property("md.name", "host:0")
            .with_property("md.creation-time", "Tue Jun 23 10:15:00 2026")
            .with_property("md.update-time", "Tue Jun 23 10:16:00 2026")
            .with_property("md.events", "17")
            .with_property("md.chunk-size", "512K")
            .with_property("md.layout", "near=2")
            .with_property("md.consistency-policy", "bitmap")
            .with_property("md.rebuild-status", "42% complete")
            .with_property("md.resync-status", "delayed")
            .with_property("md.check-status", "10% complete")
            .with_property("md.intent-bitmap", "Internal")
            .with_property("md.persistence", "Superblock is persistent")
            .with_property("md.bitmap", "0/8 pages [0KB], 65536KB chunk")
            .with_property("md.mdstat-state", "active")
            .with_property("md.mdstat-level", "raid1")
            .with_property("md.mdstat-devices", "2/1")
            .with_property("md.mdstat-health", "U_")
            .with_property("md.mdstat-progress", "recovery")
            .with_property("md.mdstat-progress-percent", "20.0%")
            .with_property("md.mdstat-progress-blocks", "209305/1046528")
            .with_property("md.mdstat-finish", "1.2min")
            .with_property("md.mdstat-speed", "12345K/sec")
            .with_property("md.mdstat-bitmap", "0/8 pages [0KB], 65536KB chunk"),
    );
    graph.add_node(
        Node::new("md:/dev/md/root", NodeKind::MdRaid, "/dev/md/root")
            .with_path("/dev/md/root")
            .with_identity(Identity {
                uuid: Some("eeee:ffff:1111:2222".to_string()),
                ..Identity::default()
            })
            .with_property("md.scan-metadata", "1.2")
            .with_property("md.uuid", "eeee:ffff:1111:2222")
            .with_property("md.scan-name", "host:root")
            .with_property("md.scan-spares", "1")
            .with_property("md.scan-devices", "/dev/sdc1,/dev/sdd1"),
    );
    graph.add_node(
        Node::new("block:/dev/sda1", NodeKind::Partition, "/dev/sda1")
            .with_path("/dev/sda1")
            .with_property("md.member-number", "0")
            .with_property("md.member-major", "8")
            .with_property("md.member-minor", "1")
            .with_property("md.member-raid-device", "0")
            .with_property("md.member-state", "active sync"),
    );
    graph.add_node(
        Node::new("block:/dev/sdb1", NodeKind::Partition, "/dev/sdb1")
            .with_path("/dev/sdb1")
            .with_property("md.member-number", "1")
            .with_property("md.member-major", "8")
            .with_property("md.member-minor", "17")
            .with_property("md.member-raid-device", "1")
            .with_property("md.member-state", "active sync")
            .with_property("md.mdstat-member-slot", "1")
            .with_property("md.mdstat-member-flags", "F"),
    );
    graph.add_edge(Edge::new(
        "block:/dev/sda1",
        "md:/dev/md0",
        Relationship::MemberOf,
    ));
    graph.add_edge(Edge::new(
        "block:/dev/sdb1",
        "md:/dev/md0",
        Relationship::MemberOf,
    ));

    let mut output = Vec::new();
    print_raid(&mut output, &graph).expect("raid table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("LEVEL"));
    assert!(output.contains("STATE"));
    assert!(output.contains("ACTIVE"));
    assert!(output.contains("FAILED"));
    assert!(output.contains("SPARE"));
    assert!(output.contains("MEMBERS"));
    assert!(output.contains("/dev/md0"));
    assert!(output.contains("raid1"));
    assert!(output.contains("clean"));
    assert!(output.contains("md-uuid=aaaa:bbbb:cccc:dddd"));
    assert!(output.contains("md-version=1.2 level=raid1 state=clean"));
    assert!(output.contains("raid-devices=2 total-devices=2 array-devices=2"));
    assert!(output.contains("active-devices=1 working-devices=2 failed-devices=1"));
    assert!(output.contains("spare-devices=1 degraded-devices=1"));
    assert!(output.contains("md-name=host:0"));
    assert!(output.contains("created=Tue Jun 23 10:15:00 2026"));
    assert!(output.contains("updated=Tue Jun 23 10:16:00 2026"));
    assert!(output.contains("events=17"));
    assert!(output.contains("chunk=512K layout=near=2"));
    assert!(output.contains("consistency=bitmap rebuild=42% complete"));
    assert!(output.contains("resync=delayed check=10% complete bitmap=Internal"));
    assert!(output.contains(
        "persistence=Superblock is persistent bitmap-detail=0/8 pages [0KB], 65536KB chunk"
    ));
    assert!(output.contains("mdstat-state=active mdstat-level=raid1"));
    assert!(output.contains("mdstat-devices=2/1 mdstat-health=U_"));
    assert!(output.contains("mdstat-progress=recovery mdstat-progress-percent=20.0%"));
    assert!(output.contains("mdstat-progress-blocks=209305/1046528"));
    assert!(output.contains("mdstat-finish=1.2min mdstat-speed=12345K/sec"));
    assert!(output.contains("mdstat-bitmap=0/8 pages [0KB], 65536KB chunk"));
    assert!(output.contains("/dev/md/root"));
    assert!(output.contains("md-uuid=eeee:ffff:1111:2222"));
    assert!(output.contains("scan-metadata=1.2 scan-name=host:root"));
    assert!(output.contains("scan-spares=1 scan-devices=/dev/sdc1,/dev/sdd1"));
    assert!(output.contains("/dev/sda1"));
    assert!(output.contains("active sync"));
    assert!(output.contains("member-number=0 member-major=8 member-minor=1 member-raid-device=0"));
    assert!(output.contains("member-state=active sync"));
    assert!(output.contains("/dev/sdb1"));
    assert!(output.contains("member-number=1 member-major=8 member-minor=17 member-raid-device=1"));
    assert!(output.contains("mdstat-member-slot=1 mdstat-member-flags=F"));
}

#[test]
fn loop_table_includes_mapping_and_backing_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("block:/dev/loop0", NodeKind::LoopDevice, "/dev/loop0")
            .with_path("/dev/loop0")
            .with_property("loop.back-file", "/var/lib/images/root.img")
            .with_property("loop.backing-inode", "12345")
            .with_property("loop.backing-major-minor", "0:45")
            .with_property("loop.major-minor", "7:0")
            .with_property("loop.offset", "1048576")
            .with_property("loop.sizelimit", "0")
            .with_property("loop.logical-sector-size", "512")
            .with_property("loop.autoclear", "true")
            .with_property("loop.partscan", "true")
            .with_property("loop.read-only", "false")
            .with_property("loop.direct-io", "true"),
    );
    graph.add_node(
        Node::new(
            "file:/var/lib/images/root.img",
            NodeKind::BackingFile,
            "/var/lib/images/root.img",
        )
        .with_path("/var/lib/images/root.img")
        .with_property("loop.backing", "true"),
    );
    graph.add_node(
        Node::new("block:/dev/loop1", NodeKind::LoopDevice, "/dev/loop1")
            .with_path("/dev/loop1")
            .with_size_bytes(1_073_741_824)
            .with_property("loop.back-file", "/dev/disk/by-id/nvme-loop-backing")
            .with_property("loop.offset", "0")
            .with_property("loop.sizelimit", "1073741824")
            .with_property("loop.read-only", "true"),
    );
    graph.add_edge(Edge::new(
        "file:/var/lib/images/root.img",
        "block:/dev/loop0",
        Relationship::Backs,
    ));

    let mut output = Vec::new();
    print_loop(&mut output, &graph).expect("loop table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("BACKING"));
    assert!(output.contains("OFFSET"));
    assert!(output.contains("/dev/loop0"));
    assert!(output.contains("/var/lib/images/root.img"));
    assert!(output.contains("1048576"));
    assert!(output.contains("ro=false"));
    assert!(output.contains(
        "back-file=/var/lib/images/root.img back-ino=12345 back-major-minor=0:45 major-minor=7:0"
    ));
    assert!(output.contains("logical-sector=512 autoclear=true partscan=true ro=false dio=true"));
    assert!(output.contains("loop-backing=true"));
    assert!(output.contains("/dev/loop1"));
    assert!(output.contains("1.0 GiB"));
    assert!(output.contains("/dev/disk/by-id/nvme-loop-backing"));
    assert!(output.contains("sizelimit=1073741824"));
}

#[test]
fn backing_files_table_includes_consumers_and_json_neighbors() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "file:/var/lib/images/root.img",
            NodeKind::BackingFile,
            "/var/lib/images/root.img",
        )
        .with_path("/var/lib/images/root.img")
        .with_size_bytes(4_294_967_296)
        .with_usage(Usage {
            used_bytes: Some(1_073_741_824),
            free_bytes: Some(3_221_225_472),
            allocated_bytes: Some(4_294_967_296),
        })
        .with_property("loop.backing", "true"),
    );
    graph.add_node(
        Node::new("block:/dev/loop0", NodeKind::LoopDevice, "/dev/loop0")
            .with_path("/dev/loop0")
            .with_property("loop.back-file", "/var/lib/images/root.img")
            .with_property("loop.offset", "0")
            .with_property("loop.read-only", "false"),
    );
    graph.add_edge(Edge::new(
        "file:/var/lib/images/root.img",
        "block:/dev/loop0",
        Relationship::Backs,
    ));

    let file = graph
        .nodes
        .iter()
        .find(|node| node.id.0 == "file:/var/lib/images/root.img")
        .expect("backing file exists");
    assert_eq!(consumer_count(&graph, file), 1);

    let mut output = Vec::new();
    print_backing_files(&mut output, &graph).expect("backing files table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("CONSUMERS"));
    assert!(output.contains("/var/lib/images/root.img"));
    assert!(output.contains("4.0 GiB"));
    assert!(output.contains("25.0%"));
    assert!(output.contains("loop-backing=true"));
    assert!(!output.contains("/dev/loop0"));

    let mut json = Vec::new();
    print_filtered_json(&mut json, &graph, is_backing_file_node)
        .expect("backing files json renders");
    let json = String::from_utf8(json).expect("json is utf8");
    assert!(json.contains("file:/var/lib/images/root.img"));
    assert!(json.contains("block:/dev/loop0"));
    assert!(json.contains("\"relationship\":\"backs\""));
}

#[test]
fn swap_table_includes_active_swap_usage_and_priority() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("block:/dev/sda3", NodeKind::Swap, "/dev/sda3")
            .with_path("/dev/sda3")
            .with_size_bytes(9_448_955_904)
            .with_usage(Usage {
                used_bytes: Some(53_592_064),
                free_bytes: Some(9_395_363_840),
                allocated_bytes: Some(9_448_955_904),
            })
            .with_property("swap.active", "true")
            .with_property("swap.type", "partition")
            .with_property("swap.priority", "-2"),
    );
    graph.add_node(
        Node::new("swap:/dev/sda3", NodeKind::Swap, "/dev/sda3")
            .with_path("/dev/sda3")
            .with_size_bytes(9_448_955_904)
            .with_usage(Usage {
                used_bytes: Some(53_592_064),
                free_bytes: Some(9_395_363_840),
                allocated_bytes: Some(9_448_955_904),
            })
            .with_property("swap.active", "true")
            .with_property("swap.type", "partition")
            .with_property("swap.priority", "-2"),
    );
    graph.add_node(
        Node::new("swap:/swapfile", NodeKind::Swap, "/swapfile")
            .with_path("/swapfile")
            .with_size_bytes(1_073_741_824)
            .with_usage(Usage {
                used_bytes: Some(0),
                free_bytes: Some(1_073_741_824),
                allocated_bytes: Some(1_073_741_824),
            })
            .with_property("swap.active", "true")
            .with_property("swap.type", "file")
            .with_property("swap.priority", "10"),
    );
    graph.add_node(
        Node::new("block:/dev/zram0", NodeKind::ZramDevice, "/dev/zram0")
            .with_path("/dev/zram0")
            .with_size_bytes(8_589_934_592)
            .with_usage(Usage {
                used_bytes: Some(2_147_483_648),
                free_bytes: Some(6_442_450_944),
                allocated_bytes: Some(805_306_368),
            })
            .with_property("zram.algorithm", "zstd")
            .with_property("zram.streams", "8")
            .with_property("zram.compressed", "715827882")
            .with_property("zram.total", "805306368")
            .with_property("zram.memory-used", "900000000")
            .with_property("zram.memory-peak", "900000000")
            .with_property("zram.compression-ratio", "2.67")
            .with_property("zram.mountpoint", "[SWAP]")
            .with_property("zram.swap", "true"),
    );
    graph.add_edge(Edge::new(
        "block:/dev/sda3",
        "swap:/dev/sda3",
        Relationship::Backs,
    ));

    let mut output = Vec::new();
    print_swap(&mut output, &graph).expect("swap table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("TYPE"));
    assert!(output.contains("PRIO"));
    assert!(output.contains("/dev/sda3"));
    assert!(output.contains("partition"));
    assert!(output.contains("-2"));
    assert!(output.contains("swap-active=true swap-type=partition swap-priority=-2"));
    assert!(output.contains("/swapfile"));
    assert!(output.contains("file"));
    assert!(output.contains("10"));
    assert!(output.contains("swap-active=true swap-type=file swap-priority=10"));
    assert!(output.contains("/dev/zram0"));
    assert!(output.contains("zram-algorithm=zstd zram-streams=8 zram-compressed=715827882"));
    assert!(output
        .contains("zram-total=805306368 zram-memory-used=900000000 zram-memory-peak=900000000"));
    assert!(output.contains("zram-ratio=2.67 zram-mountpoint=[SWAP] zram-swap=true"));
    assert!(output.contains("0.0%"));
}

#[test]
fn zram_table_includes_compressed_swap_memory_accounting() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("block:/dev/zram0", NodeKind::ZramDevice, "/dev/zram0")
            .with_path("/dev/zram0")
            .with_size_bytes(8_589_934_592)
            .with_usage(Usage {
                used_bytes: Some(2_147_483_648),
                free_bytes: Some(6_442_450_944),
                allocated_bytes: Some(805_306_368),
            })
            .with_property("zram.algorithm", "zstd")
            .with_property("zram.streams", "8")
            .with_property("zram.compressed", "715827882")
            .with_property("zram.data", "2147483648")
            .with_property("zram.total", "805306368")
            .with_property("zram.memory-limit", "0")
            .with_property("zram.memory-used", "900000000")
            .with_property("zram.memory-peak", "900000000")
            .with_property("zram.compression-ratio", "2.67")
            .with_property("zram.mountpoint", "[SWAP]")
            .with_property("zram.swap", "true"),
    );
    graph.add_node(
        Node::new("swap:/dev/sda3", NodeKind::Swap, "/dev/sda3")
            .with_path("/dev/sda3")
            .with_property("swap.type", "partition"),
    );

    let mut output = Vec::new();
    print_zram(&mut output, &graph).expect("zram table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("ALGO"));
    assert!(output.contains("RATIO"));
    assert!(output.contains("MEM-PEAK"));
    assert!(output.contains("/dev/zram0"));
    assert!(output.contains("8.0 GiB"));
    assert!(output.contains("2.0 GiB"));
    assert!(output.contains("768.0 MiB"));
    assert!(output.contains("zstd"));
    assert!(output.contains("2.67"));
    assert!(output.contains("900000000"));
    assert!(output.contains("[SWAP]"));
    assert!(output.contains("zram-compressed=715827882"));
    assert!(output.contains("zram-memory-limit=0"));
    assert!(output.contains("zram-memory-peak=900000000"));
    assert!(!output.contains("/dev/sda3"));

    let mut json = Vec::new();
    print_filtered_json(&mut json, &graph, is_zram_node).expect("zram json renders");
    let json = String::from_utf8(json).expect("json is utf8");
    assert!(json.contains("block:/dev/zram0"));
    assert!(!json.contains("swap:/dev/sda3"));
}

#[test]
fn mappings_table_includes_domain_metadata_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(Node::new(
        "block:/dev/nvme0n1p2",
        NodeKind::Partition,
        "/dev/nvme0n1p2",
    ));
    graph.add_node(
        Node::new(
            "block:/dev/mapper/cryptroot",
            NodeKind::LuksContainer,
            "cryptroot",
        )
        .with_path("/dev/mapper/cryptroot")
        .with_property("dm.name", "cryptroot")
        .with_property("dm.uuid", "CRYPT-LUKS2-crypt-uuid-cryptroot")
        .with_property("dm.major", "253")
        .with_property("dm.minor", "0")
        .with_property("dm.open-count", "1")
        .with_property("dm.segments", "1")
        .with_property("dm.events", "0")
        .with_property("dm.table.targets", "crypt")
        .with_property("dm.table.segment-count", "1")
        .with_property("dm.table.segment.0.start", "0")
        .with_property("dm.table.segment.0.length", "2097152")
        .with_property("dm.table.segment.0.target", "crypt")
        .with_property("dm.table.segment.0.crypt.cipher", "aes-xts-plain64")
        .with_property("dm.table.segment.0.crypt.device", "259:2")
        .with_property("dm.table.segment.0.crypt.offset", "4096")
        .with_property("dm.status.targets", "crypt")
        .with_property("dm.status.segment-count", "1")
        .with_property("dm.status.segment.0.target", "crypt")
        .with_property("dm.status.segment.0.payload", "0 2097152")
        .with_property("cryptsetup.active", "true")
        .with_property("cryptsetup.in-use", "true")
        .with_property("cryptsetup.cipher", "aes-xts-plain64")
        .with_property("cryptsetup.luks-version", "2")
        .with_property("cryptsetup.luks-epoch", "7")
        .with_property("cryptsetup.luks-metadata-area", "16384 [bytes]")
        .with_property("cryptsetup.luks-keyslots-area", "16744448 [bytes]")
        .with_property("cryptsetup.luks-subsystem", "(no subsystem)")
        .with_property("cryptsetup.luks-flags", "allow-discards")
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
        .with_property("cryptsetup.luks-keyslot-1-type", "luks2")
        .with_property("cryptsetup.luks-keyslot-1-priority", "ignored")
        .with_property("cryptsetup.luks-token-0-type", "systemd-tpm2")
        .with_property("cryptsetup.luks-token-0-keyslot", "0")
        .with_property("cryptsetup.luks-data-cipher", "aes-xts-plain64")
        .with_property("cryptsetup.luks-data-offset", "32768 [bytes]")
        .with_property("cryptsetup.luks-data-length", "(whole device)")
        .with_property("cryptsetup.luks-data-sector", "4096 [bytes]"),
    );
    graph.add_edge(Edge::new(
        "block:/dev/nvme0n1p2",
        "block:/dev/mapper/cryptroot",
        Relationship::Backs,
    ));
    graph.add_node(
        Node::new(
            "block:/dev/mapper/cachevol",
            NodeKind::DeviceMapper,
            "cachevol",
        )
        .with_path("/dev/mapper/cachevol")
        .with_property("dm.name", "cachevol")
        .with_property("dm.table.targets", "cache")
        .with_property("dm.table.segment-count", "1")
        .with_property("dm.table.segment.0.target", "cache")
        .with_property("dm.table.segment.0.metadata-device", "253:10")
        .with_property("dm.table.segment.0.cache-device", "253:11")
        .with_property("dm.table.segment.0.origin-device", "253:12")
        .with_property("dm.table.segment.0.block-size", "128")
        .with_property("dm.status.targets", "cache")
        .with_property("dm.status.segment-count", "1")
        .with_property("dm.status.segment.0.target", "cache")
        .with_property("dm.status.segment.0.metadata-used-blocks", "64")
        .with_property("dm.status.segment.0.metadata-total-blocks", "256")
        .with_property("dm.status.segment.0.cache-used-blocks", "32")
        .with_property("dm.status.segment.0.cache-total-blocks", "1024")
        .with_property("dm.status.segment.0.read-hits", "900")
        .with_property("dm.status.segment.0.read-misses", "100")
        .with_property("dm.status.segment.0.write-hits", "700")
        .with_property("dm.status.segment.0.write-misses", "50")
        .with_property("dm.status.segment.0.dirty-blocks", "4"),
    );
    graph.add_node(
        Node::new("multipath:mpatha", NodeKind::MultipathDevice, "mpatha")
            .with_path("/dev/mapper/mpatha")
            .with_property("multipath.dm", "dm-2")
            .with_property("multipath.wwid", "3600508b400105e210000900000490000")
            .with_property("multipath.vendor-product", "IBM,2145")
            .with_property("multipath.size", "100G")
            .with_property("multipath.features", "'1 queue_if_no_path'")
            .with_property("multipath.write-protect", "rw"),
    );
    graph.add_node(
        Node::new("vdo:archive", NodeKind::VdoVolume, "archive")
            .with_property("vdo.storage-device", "/dev/sdb")
            .with_property("vdo.logical-size", "1T")
            .with_property("vdo.physical-size", "250G")
            .with_property("vdo.operating-mode", "normal")
            .with_property("vdo.write-policy", "sync")
            .with_property("vdo.compression", "enabled")
            .with_property("vdo.deduplication", "disabled"),
    );
    let segment = Node::new(
        "lvm-seg:vg0/thinpool:0",
        NodeKind::LvmSegment,
        "vg0/thinpool:0",
    )
    .with_property("lvm.segment-type", "thin-pool")
    .with_property("lvm.segment-start", "0")
    .with_property("lvm.segment-size", "100.00g")
    .with_property("lvm.chunk-size", "64.00k")
    .with_property("lvm.thin-count", "3")
    .with_property("lvm.discards", "passdown")
    .with_property("lvm.zero", "zero")
    .with_property("lvm.transaction-id", "42")
    .with_property("lvm.devices", "thinpool_tdata(0)")
    .with_property("lvm.metadata-devices", "thinpool_tmeta(0)")
    .with_property("lvm.segment-monitor", "monitored")
    .with_property("lvm.cache-metadata-format", "2")
    .with_property("lvm.segment-cache-mode", "writeback")
    .with_property("lvm.segment-cache-policy", "smq")
    .with_property("lvm.cache-settings", "migration_threshold=2048")
    .with_property("lvm.vdo-compression", "enabled")
    .with_property("lvm.vdo-deduplication", "enabled")
    .with_property("lvm.vdo-write-policy", "auto");
    let segment_details = usage_details(&segment);
    assert!(segment_details.contains("segment-type=thin-pool"));
    assert!(segment_details.contains("metadata-devices=thinpool_tmeta(0)"));
    assert!(segment_details.contains("segment-cache-policy=smq"));
    assert!(segment_details.contains("vdo-write-policy=auto"));
    graph.add_node(segment);
    graph.add_node(
        Node::new("block:/dev/bcache0", NodeKind::CacheDevice, "bcache0")
            .with_path("/dev/bcache0")
            .with_property("bcache.role", "backing")
            .with_property("bcache.kind", "cache-set")
            .with_property("bcache.label", "fast-cache")
            .with_property("bcache.state", "clean")
            .with_property("bcache.running", "1")
            .with_property("bcache.cache-available-percent", "78")
            .with_property("bcache.cache-mode", "writeback")
            .with_property("bcache.discard", "true")
            .with_property("bcache.io-errors", "0")
            .with_property("bcache.readahead", "0")
            .with_property("bcache.sequential-cutoff", "4.0M")
            .with_property("bcache.written", "512.0M")
            .with_property("bcache.writeback-rate", "1.0M/sec"),
    );

    let mut output = Vec::new();
    print_mappings(&mut output, &graph).expect("mappings table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("DETAILS"));
    assert!(output.contains("cryptroot"));
    assert!(output.contains(
            "dm-name=cryptroot dm-uuid=CRYPT-LUKS2-crypt-uuid-cryptroot dm-major=253 dm-minor=0 open=1 segments=1 events=0"
        ));
    assert!(
            output.contains(
                "active=true in-use=true cipher=aes-xts-plain64 luks=2 epoch=7 metadata-area=16384 [bytes] keyslots-area=16744448 [bytes] subsystem=(no subsystem) flags=allow-discards keyslots=2 tokens=1 keyslot-ids=0,1 token-ids=0 keyslot-0=luks2 keyslot-0-priority=normal"
            )
        );
    assert!(output.contains(
            "keyslot-0-cipher=aes-xts-plain64 keyslot-0-cipher-key=512 bits keyslot-0-pbkdf=argon2id keyslot-0-time=4 keyslot-0-memory=1048576 keyslot-0-threads=4"
        ));
    assert!(output.contains(
            "keyslot-1=luks2 keyslot-1-priority=ignored token-0=systemd-tpm2 token-0-keyslot=0 data-cipher=aes-xts-plain64"
        ));
    assert!(output.contains(
            "dm-table-targets=crypt dm-table-segments=1 dm-table-start=0 dm-table-length=2097152 dm-table-target=crypt"
        ));
    assert!(output
        .contains("dm-crypt-cipher=aes-xts-plain64 dm-crypt-device=259:2 dm-crypt-offset=4096"));
    assert!(output.contains(
            "dm-status-targets=crypt dm-status-segments=1 dm-status-target=crypt dm-status-payload=0 2097152"
        ));
    assert!(output.contains("cachevol"));
    assert!(output.contains(
        "dm-name=cachevol dm-table-targets=cache dm-table-segments=1 dm-table-target=cache"
    ));
    assert!(output.contains(
            "dm-table-metadata-device=253:10 dm-table-cache-device=253:11 dm-table-origin-device=253:12 dm-table-block-size=128"
        ));
    assert!(output.contains("dm-status-targets=cache dm-status-segments=1 dm-status-target=cache"));
    assert!(output.contains(
            "dm-status-metadata-used=64 dm-status-metadata-total=256 dm-status-cache-used=32 dm-status-cache-total=1024"
        ));
    assert!(output.contains(
            "dm-status-read-hits=900 dm-status-read-misses=100 dm-status-write-hits=700 dm-status-write-misses=50 dm-status-dirty=4"
        ));
    assert!(
        output.contains("dm=dm-2 wwid=3600508b400105e210000900000490000 vendor=IBM,2145 size=100G")
    );
    assert!(
            output.contains(
                "backing=/dev/sdb logical=1T physical=250G mode=normal write-policy=sync compression=enabled deduplication=disabled"
            )
        );
    assert!(output.contains("vg0/thinpool:0"));
    assert!(output.contains("segment-type=thin-pool"));
    assert!(output.contains("metadata-devices=thinpool_tmeta(0)"));
    assert!(output.contains("segment-cache-policy=smq"));
    assert!(output.contains("vdo-write-policy=auto"));
    assert!(output.contains(
            "role=backing kind=cache-set label=fast-cache state=clean running=1 available-percent=78 cache-mode=writeback discard=true io-errors=0 readahead=0 sequential-cutoff=4.0M written=512.0M writeback-rate=1.0M/sec"
        ));
}

#[test]
fn dm_table_includes_table_status_and_json_neighbors() {
    let mut graph = StorageGraph::empty();
    graph.add_node(Node::new(
        "block:/dev/nvme0n1p2",
        NodeKind::Partition,
        "/dev/nvme0n1p2",
    ));
    graph.add_node(
        Node::new(
            "block:/dev/mapper/cryptroot",
            NodeKind::DeviceMapper,
            "cryptroot",
        )
        .with_path("/dev/mapper/cryptroot")
        .with_property("dm.name", "cryptroot")
        .with_property("dm.uuid", "CRYPT-LUKS2-crypt-uuid-cryptroot")
        .with_property("dm.major", "253")
        .with_property("dm.minor", "0")
        .with_property("dm.open-count", "1")
        .with_property("dm.segments", "1")
        .with_property("dm.events", "0")
        .with_property("dm.table.targets", "crypt")
        .with_property("dm.table.segment-count", "1")
        .with_property("dm.table.segment.0.start", "0")
        .with_property("dm.table.segment.0.length", "2097152")
        .with_property("dm.table.segment.0.target", "crypt")
        .with_property("dm.table.segment.0.crypt.cipher", "aes-xts-plain64")
        .with_property("dm.table.segment.0.crypt.device", "259:2")
        .with_property("dm.table.segment.0.crypt.offset", "4096")
        .with_property("dm.status.targets", "crypt")
        .with_property("dm.status.segment-count", "1")
        .with_property("dm.status.segment.0.target", "crypt")
        .with_property("dm.status.segment.0.payload", "0 2097152"),
    );
    graph.add_edge(Edge::new(
        "block:/dev/nvme0n1p2",
        "block:/dev/mapper/cryptroot",
        Relationship::Backs,
    ));
    graph.add_node(
        Node::new(
            "block:/dev/mapper/cachevol",
            NodeKind::DeviceMapper,
            "cachevol",
        )
        .with_path("/dev/mapper/cachevol")
        .with_property("dm.name", "cachevol")
        .with_property("dm.table.targets", "cache")
        .with_property("dm.table.segment-count", "1")
        .with_property("dm.table.segment.0.target", "cache")
        .with_property("dm.table.segment.0.metadata-device", "253:10")
        .with_property("dm.table.segment.0.cache-device", "253:11")
        .with_property("dm.table.segment.0.origin-device", "253:12")
        .with_property("dm.table.segment.0.block-size", "128")
        .with_property("dm.status.targets", "cache")
        .with_property("dm.status.segment-count", "1")
        .with_property("dm.status.segment.0.target", "cache")
        .with_property("dm.status.segment.0.metadata-used-blocks", "64")
        .with_property("dm.status.segment.0.metadata-total-blocks", "256")
        .with_property("dm.status.segment.0.cache-used-blocks", "32")
        .with_property("dm.status.segment.0.cache-total-blocks", "1024")
        .with_property("dm.status.segment.0.read-hits", "900")
        .with_property("dm.status.segment.0.read-misses", "100")
        .with_property("dm.status.segment.0.write-hits", "700")
        .with_property("dm.status.segment.0.write-misses", "50")
        .with_property("dm.status.segment.0.dirty-blocks", "4"),
    );

    let mut output = Vec::new();
    print_dm(&mut output, &graph).expect("dm table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("TARGETS"));
    assert!(output.contains("STATUS"));
    assert!(output.contains("MAJOR:MINOR"));
    assert!(output.contains("cryptroot"));
    assert!(output.contains("crypt"));
    assert!(output.contains("253:0"));
    assert!(output.contains(
            "dm-name=cryptroot dm-uuid=CRYPT-LUKS2-crypt-uuid-cryptroot dm-major=253 dm-minor=0 open=1 segments=1 events=0"
        ));
    assert!(output.contains(
            "dm-table-targets=crypt dm-table-segments=1 dm-table-start=0 dm-table-length=2097152 dm-table-target=crypt"
        ));
    assert!(output
        .contains("dm-crypt-cipher=aes-xts-plain64 dm-crypt-device=259:2 dm-crypt-offset=4096"));
    assert!(output.contains(
            "dm-status-targets=crypt dm-status-segments=1 dm-status-target=crypt dm-status-payload=0 2097152"
        ));
    assert!(output.contains("cachevol"));
    assert!(output.contains("cache"));
    assert!(output.contains(
            "dm-table-metadata-device=253:10 dm-table-cache-device=253:11 dm-table-origin-device=253:12 dm-table-block-size=128"
        ));
    assert!(output.contains(
            "dm-status-read-hits=900 dm-status-read-misses=100 dm-status-write-hits=700 dm-status-write-misses=50 dm-status-dirty=4"
        ));

    let mut json = Vec::new();
    print_filtered_json(&mut json, &graph, is_dm_node).expect("dm json renders");
    let json = String::from_utf8(json).expect("json is utf8");
    assert!(json.contains("block:/dev/mapper/cryptroot"));
    assert!(json.contains("block:/dev/nvme0n1p2"));
    assert!(json.contains("\"relationship\":\"backs\""));
}
