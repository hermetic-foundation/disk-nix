#[test]
fn snapshot_source_follows_snapshot_relationships() {
    let mut graph = StorageGraph::empty();
    graph.add_node(Node::new(
        "dataset:tank/home",
        NodeKind::ZfsDataset,
        "tank/home",
    ));
    graph.add_node(Node::new(
        "snapshot:tank/home@before",
        NodeKind::ZfsSnapshot,
        "tank/home@before",
    ));
    graph.add_edge(Edge::new(
        "snapshot:tank/home@before",
        "dataset:tank/home",
        Relationship::SnapshotOf,
    ));

    let snapshot = graph
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::ZfsSnapshot)
        .expect("snapshot exists");
    assert_eq!(snapshot_source(&graph, snapshot), Some("tank/home"));
}

#[test]
fn focused_json_includes_direct_relationship_neighbors() {
    let mut graph = StorageGraph::empty();
    graph.add_node(Node::new(
        "filesystem:root",
        NodeKind::Filesystem,
        "/dev/mapper/vg-root",
    ));
    graph.add_node(Node::new("mount:/", NodeKind::Mountpoint, "/"));
    graph.add_node(Node::new(
        "block:/dev/nvme0n1",
        NodeKind::PhysicalDisk,
        "/dev/nvme0n1",
    ));
    graph.add_edge(Edge::new(
        "filesystem:root",
        "mount:/",
        Relationship::MountedAt,
    ));

    let mut output = Vec::new();
    print_filtered_json(&mut output, &graph, is_filesystem_node).expect("filtered graph renders");
    let output = String::from_utf8(output).expect("json is utf8");
    let graph: StorageGraph = serde_json::from_str(&output).expect("valid storage graph json");

    assert_eq!(graph.nodes.len(), 2);
    assert!(graph
        .nodes
        .iter()
        .any(|node| node.id.0 == "filesystem:root"));
    assert!(graph.nodes.iter().any(|node| node.id.0 == "mount:/"));
    assert!(graph
        .nodes
        .iter()
        .all(|node| node.id.0 != "block:/dev/nvme0n1"));
    assert_eq!(
        graph.edges,
        vec![Edge::new(
            "filesystem:root",
            "mount:/",
            Relationship::MountedAt
        )]
    );
}

#[test]
fn devices_table_includes_probe_metadata_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("block:/dev/nvme0n1", NodeKind::PhysicalDisk, "/dev/nvme0n1")
            .with_path("/dev/nvme0n1")
            .with_size_bytes(1_000_000_000_000)
            .with_property("model", "FastDisk")
            .with_property("vendor", "Acme")
            .with_property("transport", "nvme")
            .with_property("rotational", "false")
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
            .with_property("nvme.transport", "pcie")
            .with_property("nvme.controller-id", "1")
            .with_property("nvme.namespace-capacity", "900000000000")
            .with_property("nvme.lba-format", "512 B + 0 B")
            .with_property("nvme.maximum-lba", "1953125")
            .with_property("nvme.sector-size", "512")
            .with_property("nvme.ana-state", "optimized")
            .with_property("lsblk.logical-sector-size", "512")
            .with_property("lsblk.physical-sector-size", "4096")
            .with_property("lsblk.minimum-io-size", "4096")
            .with_property("lsblk.optimal-io-size", "1048576")
            .with_property("lsblk.discard-alignment", "0")
            .with_property("lsblk.discard-granularity", "4096")
            .with_property("lsblk.discard-max", "2147483648")
            .with_property("lsblk.discard-zeroes-data", "false")
            .with_property("lsblk.scheduler", "none")
            .with_property("lsblk.request-queue-size", "1023")
            .with_property("lsblk.write-same-max", "0")
            .with_property("lsblk.zoned", "host-managed")
            .with_property("lsblk.zone-size", "268435456")
            .with_property("lsblk.zone-write-granularity", "4096")
            .with_property("lsblk.zone-append-max", "65536")
            .with_property("lsblk.zone-count", "64")
            .with_property("lsblk.zone-open-max", "32")
            .with_property("lsblk.zone-active-max", "48")
            .with_property("lsblk.dax", "false")
            .with_property("lsblk.hotplug", "false")
            .with_property("partition.table", "gpt")
            .with_property("udev.symlink", "disk/by-id/nvme-Acme_FastDisk")
            .with_property("udev.devname", "/dev/nvme0n1")
            .with_property("udev.devtype", "disk")
            .with_property("udev.id-bus", "nvme")
            .with_property("udev.id-model", "FastDisk")
            .with_property("udev.id-model-id", "a808")
            .with_property("udev.id-vendor", "Acme")
            .with_property("udev.id-vendor-id", "144d")
            .with_property("udev.id-revision", "1.0")
            .with_property("udev.id-serial", "Acme_FastDisk_SERIAL")
            .with_property("udev.id-serial-short", "SERIAL")
            .with_property("udev.id-wwn", "eui.1234")
            .with_property("udev.id-path", "pci-0000:01:00.0-nvme-1")
            .with_property("udev.id-path-tag", "pci-0000_01_00_0-nvme-1")
            .with_property("udev.major", "259")
            .with_property("udev.minor", "0")
            .with_property("udev.subsystem", "block"),
    );
    graph.add_node(
        Node::new(
            "block:/dev/nvme0n1p1",
            NodeKind::Partition,
            "/dev/nvme0n1p1",
        )
        .with_path("/dev/nvme0n1p1")
        .with_property("lsblk.type", "part")
        .with_property("filesystem.type", "vfat")
        .with_property("partition.number", "1")
        .with_property("udev.id-fs-type", "vfat")
        .with_property("udev.id-fs-version", "FAT32")
        .with_property("udev.id-fs-usage", "filesystem")
        .with_property("udev.id-fs-uuid", "AAAA-BBBB")
        .with_property("udev.id-fs-uuid-enc", "AAAA-BBBB")
        .with_property("udev.id-fs-uuid-sub", "CCCC-DDDD")
        .with_property("udev.id-fs-label", "EFI")
        .with_property("udev.id-fs-label-enc", "EFI")
        .with_property("udev.id-fs-label-safe", "EFI")
        .with_property("udev.id-fs-block-size", "512")
        .with_property("udev.id-fs-lastblock", "1048575")
        .with_property("udev.id-part-entry-disk", "259:0")
        .with_property("udev.id-part-entry-number", "1")
        .with_property("udev.id-part-entry-offset", "2048")
        .with_property("udev.id-part-entry-size", "1048576")
        .with_property("udev.id-part-entry-scheme", "gpt")
        .with_property("udev.id-part-entry-type", "uefi")
        .with_property("udev.id-part-entry-name", "EFI System Partition")
        .with_property("udev.id-part-entry-uuid", "part-uuid")
        .with_property("udev.id-part-entry-flags", "0x1")
        .with_property("udev.id-part-table-type", "gpt")
        .with_property("udev.id-part-table-uuid", "table-uuid"),
    );
    graph.add_node(
        Node::new("block:/dev/loop0", NodeKind::LoopDevice, "/dev/loop0")
            .with_path("/dev/loop0")
            .with_property("lsblk.type", "loop")
            .with_property("loop.back-file", "/var/lib/images/root.img")
            .with_property("loop.backing-inode", "12345")
            .with_property("loop.backing-major-minor", "0:45")
            .with_property("loop.offset", "1048576")
            .with_property("loop.autoclear", "true")
            .with_property("loop.partscan", "true")
            .with_property("loop.direct-io", "true"),
    );
    graph.add_node(
        Node::new("block:/dev/dm-0", NodeKind::DeviceMapper, "/dev/dm-0")
            .with_path("/dev/dm-0")
            .with_property("udev.dm-name", "cryptroot")
            .with_property("udev.dm-uuid", "CRYPT-LUKS2-luks-uuid-cryptroot")
            .with_property("udev.dm-vg-name", "vg0")
            .with_property("udev.dm-lv-name", "root")
            .with_property("udev.dm-udev-rules-vsn", "3")
            .with_property("udev.dm-udev-primary-source-flag", "1")
            .with_property("udev.dm-udev-disable-other-rules-flag", "0")
            .with_property("udev.dm-subsystem-udev-flag0", "1")
            .with_property("udev.dm-subsystem-udev-flag1", "0"),
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
        Node::new("swap:/dev/zram0", NodeKind::Swap, "/dev/zram0")
            .with_path("/dev/zram0")
            .with_property("swap.active", "true")
            .with_property("swap.type", "partition")
            .with_property("swap.priority", "100"),
    );
    graph.add_node(
        Node::new("block:/dev/sda1", NodeKind::Partition, "/dev/sda1")
            .with_path("/dev/sda1")
            .with_property("md.member-state", "active sync"),
    );
    graph.add_node(
        Node::new("block:/dev/sdb", NodeKind::PhysicalDisk, "/dev/sdb")
            .with_path("/dev/sdb")
            .with_property("smartctl.svn-revision", "5530")
            .with_property("smartctl.platform", "x86_64-linux")
            .with_property("smartctl.exit-status", "0")
            .with_property("smartctl.device-name", "/dev/sdb")
            .with_property("smartctl.health.passed", "true")
            .with_property("smartctl.device-type", "sat")
            .with_property("smartctl.protocol", "ATA")
            .with_property("smartctl.model", "Example SSD")
            .with_property("smartctl.model-family", "Example SSDs")
            .with_property("smartctl.serial", "SATA123")
            .with_property("smartctl.revision", "A1")
            .with_property("smartctl.firmware-version", "1.2.3")
            .with_property("smartctl.wwn-naa", "5")
            .with_property("smartctl.wwn-oui", "12345")
            .with_property("smartctl.wwn-id", "67890")
            .with_property("smartctl.user-capacity-bytes", "1000204886016")
            .with_property("smartctl.logical-block-size", "512")
            .with_property("smartctl.physical-block-size", "4096")
            .with_property("smartctl.rotation-rate-rpm", "0")
            .with_property("smartctl.form-factor", "2.5 inches")
            .with_property("smartctl.sata-version", "SATA 3.3")
            .with_property("smartctl.interface-speed-current", "6.0")
            .with_property("smartctl.interface-speed-max", "6.0")
            .with_property("smartctl.power-on-hours", "4242")
            .with_property("smartctl.power-cycle-count", "12")
            .with_property("smartctl.temperature-current-celsius", "31")
            .with_property("smartctl.temperature-highest-celsius", "44")
            .with_property("smartctl.temperature-lowest-celsius", "20")
            .with_property(
                "smartctl.offline-data-collection-status",
                "was completed without error",
            )
            .with_property("smartctl.self-test-status", "completed without error")
            .with_property("smartctl.error-log-summary-count", "3")
            .with_property("smartctl.self-test-log-count", "2")
            .with_property("smartctl.error-logging-supported", "true")
            .with_property("smartctl.gp-logging-supported", "true")
            .with_property("smartctl.sct-capabilities", "61")
            .with_property("smartctl.scsi-grown-defect-list", "0")
            .with_property("smartctl.attribute.reallocated-sector-ct.raw", "0")
            .with_property("smartctl.attribute.reallocated-sector-ct.value", "100")
            .with_property("smartctl.attribute.reallocated-sector-ct.worst", "100")
            .with_property("smartctl.attribute.reallocated-sector-ct.threshold", "10")
            .with_property(
                "smartctl.attribute.reallocated-sector-ct.when-failed",
                "never",
            )
            .with_property("smartctl.attribute.current-pending-sector.raw", "1")
            .with_property("smartctl.attribute.current-pending-sector.value", "99")
            .with_property("smartctl.attribute.current-pending-sector.worst", "98")
            .with_property("smartctl.attribute.current-pending-sector.threshold", "0")
            .with_property(
                "smartctl.attribute.current-pending-sector.when-failed",
                "past",
            )
            .with_property("smartctl.attribute.offline-uncorrectable.raw", "2")
            .with_property("smartctl.attribute.offline-uncorrectable.value", "97")
            .with_property("smartctl.attribute.offline-uncorrectable.worst", "96")
            .with_property("smartctl.attribute.offline-uncorrectable.threshold", "0")
            .with_property(
                "smartctl.attribute.offline-uncorrectable.when-failed",
                "past",
            )
            .with_property("scsi.address", "1:0:0:0")
            .with_property("scsi.generic-device", "/dev/sg1")
            .with_property("scsi.transport", "sata:5000c500a5a461dc")
            .with_property("scsi.unit-name", "5000c500a5a461dc")
            .with_property("scsi.queue-depth", "32")
            .with_property("multipath.host-path", "2:0:0:1")
            .with_property("major-minor", "8:16")
            .with_property("multipath.path-flags", "ghost")
            .with_property("multipath.path-state", "active ready running ghost"),
    );

    let mut output = Vec::new();
    print_devices(&mut output, &graph).expect("devices table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("DETAILS"));
    assert!(output.contains("model=FastDisk vendor=Acme transport=nvme rotational=false"));
    assert!(output.contains("nvme-model=Example NVMe product=Example Controller firmware=1.0"));
    assert!(output
        .contains("ns-index=0 namespace=1 nsid=1 ns-uuid=12345678-1234-1234-1234-123456789abc"));
    assert!(output.contains(
            "eui64=0011223344556677 nguid=00112233445566778899aabbccddeeff subsystem=nvme-subsys0 controller=nvme0"
        ));
    assert!(output.contains(
        "transport=pcie controller-id=1 namespace-capacity=900000000000 lba-format=512 B + 0 B"
    ));
    assert!(output.contains("max-lba=1953125 sector-size=512 ana-state=optimized"));
    assert!(output
        .contains("logical-sector=512 physical-sector=4096 minimum-io=4096 optimal-io=1048576"));
    assert!(output.contains(
        "discard-alignment=0 discard-granularity=4096 discard-max=2147483648 discard-zeroes=false"
    ));
    assert!(output.contains("scheduler=none rq-size=1023 write-same-max=0 zoned=host-managed"));
    assert!(output.contains(
        "zone-size=268435456 zone-write-granularity=4096 zone-append-max=65536 zone-count=64"
    ));
    assert!(output.contains("zone-open-max=32 zone-active-max=48 dax=false hotplug=false"));
    assert!(output.contains("ptable=gpt"));
    assert!(output.contains("udev-link=disk/by-id/nvme-Acme_FastDisk"));
    assert!(output.contains("udev-devname=/dev/nvme0n1 udev-devtype=disk"));
    assert!(output.contains("udev-bus=nvme udev-model=FastDisk udev-model-id=a808"));
    assert!(output.contains("udev-vendor=Acme udev-vendor-id=144d udev-revision=1.0"));
    assert!(output.contains("udev-serial=Acme_FastDisk_SERIAL udev-serial-short=SERIAL"));
    assert!(output.contains("udev-wwn=eui.1234 udev-path=pci-0000:01:00.0-nvme-1"));
    assert!(output.contains("udev-path-tag=pci-0000_01_00_0-nvme-1"));
    assert!(output.contains("major=259 minor=0 subsystem=block"));
    assert!(output.contains("lsblk-type=part fstype=vfat partno=1 udev-fstype=vfat"));
    assert!(output.contains("udev-fs-version=FAT32 udev-fs-usage=filesystem"));
    assert!(output.contains("udev-fs-uuid=AAAA-BBBB udev-fs-uuid-enc=AAAA-BBBB"));
    assert!(output.contains("udev-fs-uuid-sub=CCCC-DDDD"));
    assert!(output.contains("udev-label=EFI udev-label-enc=EFI udev-label-safe=EFI"));
    assert!(output.contains("udev-fs-block-size=512 udev-fs-lastblock=1048575"));
    assert!(output.contains("udev-part-disk=259:0 udev-part-number=1"));
    assert!(output.contains("udev-part-offset=2048 udev-part-size=1048576"));
    assert!(output.contains("udev-part-scheme=gpt udev-part-type=uefi"));
    assert!(output.contains("udev-part-name=EFI System Partition udev-part-uuid=part-uuid"));
    assert!(output.contains("udev-part-flags=0x1 udev-table-type=gpt"));
    assert!(output.contains("udev-table-uuid=table-uuid"));
    assert!(output.contains("dm-name=cryptroot dm-uuid=CRYPT-LUKS2-luks-uuid-cryptroot"));
    assert!(output.contains("dm-vg=vg0 dm-lv=root dm-rules=3"));
    assert!(output.contains("dm-primary-source=1 dm-disable-other-rules=0"));
    assert!(output.contains("dm-subsystem-flag0=1 dm-subsystem-flag1=0"));
    assert!(output.contains(
            "lsblk-type=loop back-file=/var/lib/images/root.img back-ino=12345 back-major-minor=0:45 offset=1048576 autoclear=true partscan=true dio=true"
        ));
    assert!(output.contains("loop-backing=true"));
    assert!(output.contains("swap-active=true swap-type=partition swap-priority=100"));
    assert!(output.contains("member-state=active sync"));
    assert!(output.contains(
        "smart-svn=5530 smart-platform=x86_64-linux smart-exit-status=0 smart-device-name=/dev/sdb"
    ));
    assert!(output.contains(
        "smart-health-passed=true smart-device-type=sat smart-protocol=ATA smart-model=Example SSD"
    ));
    assert!(output.contains("smart-family=Example SSDs"));
    assert!(output.contains("smart-revision=A1 smart-firmware=1.2.3"));
    assert!(output
        .contains("smart-serial=SATA123 smart-wwn-naa=5 smart-wwn-oui=12345 smart-wwn-id=67890"));
    assert!(output.contains("smart-capacity=1000204886016 smart-logical-block=512"));
    assert!(output.contains(
        "smart-physical-block=4096 smart-rpm=0 smart-form-factor=2.5 inches sata-version=SATA 3.3"
    ));
    assert!(output.contains("interface-speed-current=6.0 interface-speed-max=6.0"));
    assert!(output.contains("smart-power-on-hours=4242"));
    assert!(
            output.contains(
                "smart-power-cycles=12 smart-temperature-c=31 smart-temperature-highest-c=44 smart-temperature-lowest-c=20"
            )
        );
    assert!(output.contains(
        "smart-offline-status=was completed without error smart-self-test=completed without error"
    ));
    assert!(output.contains(
            "smart-error-log-count=3 smart-self-test-count=2 smart-error-logging=true smart-gp-logging=true"
        ));
    assert!(output.contains("smart-sct-capabilities=61 smart-scsi-grown-defects=0"));
    assert!(output.contains(
            "reallocated-sectors=0 reallocated-value=100 reallocated-worst=100 reallocated-threshold=10 reallocated-failed=never"
        ));
    assert!(output.contains(
            "pending-sectors=1 pending-value=99 pending-worst=98 pending-threshold=0 pending-failed=past"
        ));
    assert!(
            output.contains(
                "offline-uncorrectable=2 offline-uncorrectable-value=97 offline-uncorrectable-worst=96 offline-uncorrectable-threshold=0 offline-uncorrectable-failed=past"
            )
        );
    assert!(output.contains(
        "scsi-address=1:0:0:0 scsi-generic=/dev/sg1 scsi-transport=sata:5000c500a5a461dc"
    ));
    assert!(output.contains("scsi-unit=5000c500a5a461dc scsi-queue-depth=32"));
    assert!(output.contains(
        "host-path=2:0:0:1 major-minor=8:16 path-flags=ghost path-state=active ready running ghost"
    ));
    assert!(output.contains("/var/lib/images/root.img"));
}

#[test]
fn partitions_table_includes_geometry_metadata_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "block:/dev/nvme0n1p1",
            NodeKind::Partition,
            "/dev/nvme0n1p1",
        )
        .with_path("/dev/nvme0n1p1")
        .with_size_bytes(536_870_912)
        .with_identity(Identity {
            partuuid: Some("1111-2222".to_string()),
            ..Default::default()
        })
        .with_property("partition.number", "1")
        .with_property("partition.start", "1049kB")
        .with_property("partition.start-bytes", "1049000")
        .with_property("partition.end", "538MB")
        .with_property("partition.end-bytes", "538000000")
        .with_property("partition.type", "fat32")
        .with_property("partition.name", "ESP")
        .with_property("partition.flags", "boot, esp")
        .with_property("filesystem.type", "vfat")
        .with_property("blkid.type", "vfat")
        .with_property("blkid.version", "FAT32")
        .with_property("blkid.block-size", "512")
        .with_property("blkid.usage", "filesystem")
        .with_property("blkid.partlabel", "EFI System Partition"),
    );

    let mut output = Vec::new();
    print_partitions(&mut output, &graph).expect("partitions table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("DETAILS"));
    assert!(output.contains("1111-2222"));
    assert!(output.contains(
            "fstype=vfat blkid-type=vfat version=FAT32 blkid-block-size=512 usage=filesystem partlabel=EFI System Partition partno=1 start=1049kB start-bytes=1049000 end=538MB end-bytes=538000000 type=fat32 part-name=ESP flags=boot, esp"
        ));
}

#[test]
fn usage_percent_prefers_size_then_allocated_then_used_plus_free() {
    let sized = Node::new("filesystem:root", NodeKind::Filesystem, "/")
        .with_size_bytes(100)
        .with_usage(Usage {
            used_bytes: Some(25),
            free_bytes: Some(75),
            allocated_bytes: Some(50),
        });
    assert_eq!(usage_percent(&sized), "25.0%");

    let allocated = Node::new("btrfs:data", NodeKind::BtrfsFilesystem, "data").with_usage(Usage {
        used_bytes: Some(25),
        free_bytes: None,
        allocated_bytes: Some(50),
    });
    assert_eq!(usage_percent(&allocated), "50.0%");

    let used_free = Node::new("swap:/dev/sda3", NodeKind::Swap, "/dev/sda3").with_usage(Usage {
        used_bytes: Some(25),
        free_bytes: Some(75),
        allocated_bytes: None,
    });
    assert_eq!(usage_percent(&used_free), "25.0%");
}

#[test]
fn usage_details_surfaces_storage_metadata() {
    let lv = Node::new("lv:vg/thin", NodeKind::LvmLogicalVolume, "vg/thin")
        .with_size_bytes(100)
        .with_usage(Usage {
            used_bytes: Some(25),
            free_bytes: Some(75),
            allocated_bytes: None,
        })
        .with_property("lvm.data-percent", "12.50")
        .with_property("lvm.metadata-percent", "3.00")
        .with_property("lvm.snap-percent", "4.00")
        .with_property("lvm.copy-percent", "99.00")
        .with_property("lvm.active", "active")
        .with_property("lvm.layout", "thin")
        .with_property("lvm.health", "ok")
        .with_property("lvm.when-full", "queue")
        .with_property("lvm.metadata-size", "128.00m")
        .with_property("lvm.role", "public")
        .with_property("lvm.cache-mode", "writeback")
        .with_property("lvm.cache-policy", "smq")
        .with_property("lvm.kernel-discards", "passdown")
        .with_property("lvm.writecache-writeback-blocks", "16");
    assert_eq!(
            usage_details(&lv),
            "data=12.50 metadata=3.00 snap=4.00 copy=99.00 layout=thin active=active health=ok when-full=queue metadata-size=128.00m role=public cache-mode=writeback cache-policy=smq kernel-discards=passdown writecache-writeback=16"
        );

    let pool = Node::new("zpool:tank", NodeKind::ZfsPool, "tank")
        .with_size_bytes(100)
        .with_property("zfs.health", "ONLINE");
    assert_eq!(usage_details(&pool), "health=ONLINE");

    let snapshot = Node::new(
        "zfs-snapshot:tank/home@daily",
        NodeKind::ZfsSnapshot,
        "tank/home@daily",
    )
    .with_property("zfs.userrefs", "2");
    let snapshot = snapshot.with_property("zfs.holds", "disk-nix-retain");
    assert_eq!(usage_details(&snapshot), "userrefs=2 holds=disk-nix-retain");

    let dataset = Node::new("zfs-dataset:tank/home", NodeKind::ZfsDataset, "tank/home")
        .with_property("zfs.compression", "zstd")
        .with_property("zfs.encryption", "aes-256-gcm")
        .with_property("zfs.keystatus", "available");
    assert_eq!(
        usage_details(&dataset),
        "compression=zstd encryption=aes-256-gcm keystatus=available"
    );

    let xfs = Node::new("mount:/", NodeKind::Mountpoint, "/")
        .with_property("xfs.meta-data.meta-data", "/dev/mapper/vg-root")
        .with_property("xfs.meta-data.isize", "512")
        .with_property("xfs.meta-data.agcount", "4")
        .with_property("xfs.meta-data.agsize", "65536")
        .with_property("xfs.meta-data.sectsz", "512")
        .with_property("xfs.meta-data.attr", "2")
        .with_property("xfs.meta-data.projid32bit", "1")
        .with_property("xfs.meta-data.crc", "1")
        .with_property("xfs.meta-data.finobt", "1")
        .with_property("xfs.meta-data.sparse", "1")
        .with_property("xfs.meta-data.rmapbt", "0")
        .with_property("xfs.data.blocks", "262144")
        .with_property("xfs.data.bsize", "4096")
        .with_property("xfs.data.imaxpct", "25")
        .with_property("xfs.data.sunit", "0")
        .with_property("xfs.data.swidth", "0")
        .with_property("xfs.meta-data.reflink", "1")
        .with_property("xfs.meta-data.bigtime", "1")
        .with_property("xfs.meta-data.inobtcount", "1")
        .with_property("xfs.meta-data.nrext64", "0")
        .with_property("xfs.naming.version", "2")
        .with_property("xfs.naming.bsize", "4096")
        .with_property("xfs.naming.ascii-ci", "0")
        .with_property("xfs.naming.ftype", "1")
        .with_property("xfs.log.type", "internal log")
        .with_property("xfs.log.bsize", "4096")
        .with_property("xfs.log.blocks", "2560")
        .with_property("xfs.log.version", "2")
        .with_property("xfs.log.sectsz", "512")
        .with_property("xfs.log.sunit", "0")
        .with_property("xfs.log.lazy-count", "1")
        .with_property("xfs.realtime.type", "none")
        .with_property("xfs.realtime.extsz", "4096")
        .with_property("xfs.realtime.blocks", "0")
        .with_property("xfs.realtime.rtextents", "0");
    assert_eq!(
            usage_details(&xfs),
            "xfs-source=/dev/mapper/vg-root xfs-isize=512 xfs-agcount=4 xfs-agsize=65536 xfs-sectsz=512 xfs-attr=2 xfs-projid32bit=1 xfs-crc=1 xfs-finobt=1 xfs-sparse=1 xfs-rmapbt=0 xfs-blocks=262144 xfs-bsize=4096 xfs-imaxpct=25 xfs-sunit=0 xfs-swidth=0 reflink=1 bigtime=1 xfs-inobtcount=1 xfs-nrext64=0 xfs-naming-version=2 xfs-naming-bsize=4096 xfs-ascii-ci=0 xfs-ftype=1 xfs-log-type=internal log xfs-log-bsize=4096 log-blocks=2560 xfs-log-version=2 xfs-log-sectsz=512 xfs-log-sunit=0 xfs-log-lazy-count=1 xfs-realtime-type=none xfs-realtime-extsz=4096 xfs-realtime-blocks=0 xfs-realtime-rtextents=0"
        );

    let ext = Node::new("fs:/dev/sda2", NodeKind::Filesystem, "ext4")
        .with_property("filesystem.type", "ext4")
        .with_property("blkid.version", "1.0")
        .with_property("blkid.block-size", "4096")
        .with_property("blkid.usage", "filesystem")
        .with_property("blkid.uuid-sub", "subvol-uuid")
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
        .with_property("ext.fragment-size", "4096")
        .with_property("ext.blocks-per-group", "32768")
        .with_property("ext.fragments-per-group", "32768")
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
        .with_property("ext.created", "Mon Jan 01 00:00:00 2024")
        .with_property("ext.last-mount-time", "Mon Jun 22 12:00:00 2026")
        .with_property("ext.last-write-time", "Mon Jun 22 12:00:00 2026")
        .with_property("ext.mount-count", "12")
        .with_property("ext.maximum-mount-count", "-1")
        .with_property("ext.last-checked", "Mon Jan 01 00:00:00 2024")
        .with_property("ext.check-interval", "0 (<none>)")
        .with_property("ext.lifetime-writes", "189 GB")
        .with_property("ext.reserved-blocks-uid", "0 (user root)")
        .with_property("ext.reserved-blocks-gid", "0 (group root)")
        .with_property("ext.first-inode", "11")
        .with_property("ext.inode-size", "256")
        .with_property("ext.journal-inode", "8")
        .with_property("ext.journal-uuid", "99999999-aaaa-bbbb-cccc-dddddddddddd")
        .with_property("ext.journal-backup", "inode blocks")
        .with_property("ext.journal-features", "journal_incompat_revoke")
        .with_property("ext.journal-size", "1024M")
        .with_property("ext.first-error-time", "Mon Jun 22 12:30:00 2026")
        .with_property("ext.first-error-function", "ext4_lookup")
        .with_property("ext.first-error-line", "1234")
        .with_property("ext.first-error-inode", "42")
        .with_property("ext.first-error-block", "9001")
        .with_property("ext.last-error-time", "Mon Jun 22 12:45:00 2026")
        .with_property("ext.last-error-function", "ext4_journal_check_start")
        .with_property("ext.last-error-line", "5678")
        .with_property("ext.last-error-inode", "43")
        .with_property("ext.last-error-block", "9002")
        .with_property("ext.checksum-type", "crc32c")
        .with_property("ext.checksum", "0x12345678");
    assert_eq!(
            usage_details(&ext),
            "fstype=ext4 version=1.0 blkid-block-size=4096 usage=filesystem uuid-sub=subvol-uuid ext-state=clean ext-magic=0xEF53 ext-revision=1 (dynamic) errors=Continue fs-error-count=2 os=Linux blocks=122096646 reserved-blocks=6104832 overhead-clusters=123456 free-blocks=73328197 first-block=0 block-size=4096 fragment-size=4096 blocks-per-group=32768 fragments-per-group=32768 inodes=30531584 free-inodes=27187554 inodes-per-group=8192 raid-stride=128 raid-stripe-width=256 features=has_journal extent metadata_csum flags=signed_directory_hash dir-hash=half_md4 dir-hash-seed=11111111-2222-3333-4444-555555555555 default-mount=user_xattr acl created=Mon Jan 01 00:00:00 2024 last-mounted=Mon Jun 22 12:00:00 2026 last-written=Mon Jun 22 12:00:00 2026 mount-count=12 max-mount-count=-1 last-checked=Mon Jan 01 00:00:00 2024 check-interval=0 (<none>) lifetime-writes=189 GB reserved-uid=0 (user root) reserved-gid=0 (group root) first-inode=11 inode-size=256 journal-inode=8 journal-uuid=99999999-aaaa-bbbb-cccc-dddddddddddd journal-backup=inode blocks journal-features=journal_incompat_revoke journal-size=1024M first-error-time=Mon Jun 22 12:30:00 2026 first-error-function=ext4_lookup first-error-line=1234 first-error-inode=42 first-error-block=9001 last-error-time=Mon Jun 22 12:45:00 2026 last-error-function=ext4_journal_check_start last-error-line=5678 last-error-inode=43 last-error-block=9002 checksum-type=crc32c checksum=0x12345678"
        );

    let exfat = Node::new("fs:/dev/sdb1", NodeKind::Filesystem, "exfat")
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
        .with_property("exfat.bytes-per-cluster", "32768");
    assert_eq!(
            usage_details(&exfat),
            "guid=01234567-89ab-cdef-0123-456789abcdef exfat-label=SHARED exfatprogs=1.2.4 serial=0x6eef953b sectors=3203072 fat-offset=2048 fat-length=448 cluster-heap-offset=4096 clusters=49984 used-clusters=48960 free-clusters=1024 root-cluster=4 sector-bytes=512 sectors-per-cluster=64 cluster-bytes=32768"
        );

    let ntfs = Node::new("fs:/dev/sda1", NodeKind::Filesystem, "ntfs")
        .with_property("ntfs.device-name", "/dev/sda1")
        .with_property("ntfs.device-state", "11")
        .with_property("ntfs.volume-name", "Windows")
        .with_property("ntfs.volume-serial", "01234567-89abcdef")
        .with_property("ntfs.version", "3.1")
        .with_property("ntfs.sector-size", "512")
        .with_property("ntfs.cluster-size", "4096")
        .with_property("ntfs.volume-size-clusters", "262144")
        .with_property("ntfs.mft-record-size", "1024")
        .with_property("ntfs.mft-zone-multiplier", "0")
        .with_property("ntfs.mft-zone-start", "786432")
        .with_property("ntfs.mft-zone-end", "819200")
        .with_property("ntfs.mft-data-position", "786944")
        .with_property("ntfs.mft-lcn", "4");
    assert_eq!(
            usage_details(&ntfs),
            "ntfs-device=/dev/sda1 ntfs-device-state=11 ntfs-name=Windows ntfs-serial=01234567-89abcdef ntfs-version=3.1 ntfs-sector=512 ntfs-cluster=4096 ntfs-clusters=262144 ntfs-mft-record=1024 ntfs-mft-zone-multiplier=0 ntfs-mft-zone-start=786432 ntfs-mft-zone-end=819200 ntfs-mft-data-position=786944 ntfs-mft-lcn=4"
        );

    let f2fs = Node::new("fs:/dev/sdb2", NodeKind::Filesystem, "f2fs")
        .with_property("f2fs.filesystem-volume-name", "mobile")
        .with_property(
            "f2fs.filesystem-uuid",
            "01234567-89ab-cdef-0123-456789abcdef",
        )
        .with_property("f2fs.block-size", "4096")
        .with_property("f2fs.block-count", "262144")
        .with_property("f2fs.user-block-count", "245760")
        .with_property("f2fs.valid-block-count", "65536")
        .with_property("f2fs.total-valid-block-count", "65540")
        .with_property("f2fs.valid-node-count", "4096")
        .with_property("f2fs.valid-inode-count", "2048")
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
        .with_property("f2fs.log-sectorsize", "9")
        .with_property("f2fs.log-sectors-per-block", "3")
        .with_property("f2fs.log-blocksize", "12")
        .with_property("f2fs.log-blocks-per-seg", "9")
        .with_property("f2fs.cp-payload", "0")
        .with_property("f2fs.version", "Linux version 6.12")
        .with_property("f2fs.init-version", "Linux version 6.1")
        .with_property("f2fs.extension-count", "29")
        .with_property("f2fs.hot-ext-count", "5");
    assert_eq!(
            usage_details(&f2fs),
            "f2fs-uuid=01234567-89ab-cdef-0123-456789abcdef f2fs-name=mobile f2fs-block-size=4096 f2fs-blocks=262144 f2fs-user-blocks=245760 f2fs-valid-blocks=65536 f2fs-total-valid-blocks=65540 f2fs-valid-nodes=4096 f2fs-valid-inodes=2048 f2fs-segments=2048 f2fs-main-segments=1984 f2fs-ckpt-segments=2 f2fs-sit-segments=2 f2fs-nat-segments=4 f2fs-ssa-segments=1 f2fs-overprov=64 f2fs-sections=1984 f2fs-segs-per-sec=1 f2fs-secs-per-zone=1 f2fs-log-sector=9 f2fs-log-sectors-block=3 f2fs-log-block=12 f2fs-log-blocks-seg=9 f2fs-cp-payload=0 f2fs-version=Linux version 6.12 f2fs-init-version=Linux version 6.1 f2fs-extensions=29 f2fs-hot-extensions=5"
        );

    let bcachefs = Node::new(
        "bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3",
        NodeKind::BcachefsFilesystem,
        "archive",
    )
    .with_property(
        "bcachefs.external-uuid",
        "a2d6fc04-efd0-4e36-aece-2475941d09a3",
    )
    .with_property(
        "bcachefs.internal-uuid",
        "55083d1e-27cf-4929-ada4-3fe6e45cf02c",
    )
    .with_property(
        "bcachefs.magic-number",
        "c68573f6-66ce-90a9-d96a-60cf803df7ef",
    )
    .with_property("bcachefs.device", "ST12000NM001G-2M")
    .with_property("bcachefs.member-device", "/dev/sdc")
    .with_property("bcachefs.mount-target", "/mnt/archive")
    .with_property("bcachefs.device-index", "6")
    .with_property("bcachefs.version", "1.20: (unknown version)")
    .with_property(
        "bcachefs.version-upgrade-complete",
        "1.20: (unknown version)",
    )
    .with_property("bcachefs.online-reserved", "507957248")
    .with_property("bcachefs.device-count", "2")
    .with_property("bcachefs.data-sb", "3149824")
    .with_property("bcachefs.data-journal", "4294967296")
    .with_property("bcachefs.data-btree", "1048576")
    .with_property("bcachefs.data-user", "2147483648");
    assert_eq!(
            usage_details(&bcachefs),
            "bcachefs-uuid=a2d6fc04-efd0-4e36-aece-2475941d09a3 bcachefs-internal=55083d1e-27cf-4929-ada4-3fe6e45cf02c bcachefs-magic=c68573f6-66ce-90a9-d96a-60cf803df7ef bcachefs-super-device=ST12000NM001G-2M bcachefs-member=/dev/sdc bcachefs-mount=/mnt/archive bcachefs-device=6 bcachefs-version=1.20: (unknown version) bcachefs-upgrade-complete=1.20: (unknown version) bcachefs-reserved=507957248 bcachefs-devices=2 bcachefs-sb=3149824 bcachefs-journal=4294967296 bcachefs-btree=1048576 bcachefs-user=2147483648"
        );

    let bcachefs_device = Node::new(
        "bcachefs-device:a2d6fc04-efd0-4e36-aece-2475941d09a3:6",
        NodeKind::BcachefsDevice,
        "sdc",
    )
    .with_property("bcachefs.device-label", "hdd.archive")
    .with_property("bcachefs.device-state", "rw")
    .with_property("bcachefs.device-free", "1649975230464")
    .with_property("bcachefs.device-capacity", "16000900661248")
    .with_property("bcachefs.device-data-sb", "3149824")
    .with_property("bcachefs.device-data-journal", "4294967296")
    .with_property("bcachefs.device-data-btree", "890241024")
    .with_property("bcachefs.device-data-user", "0");
    assert_eq!(
            usage_details(&bcachefs_device),
            "bcachefs-label=hdd.archive bcachefs-state=rw bcachefs-device-free=1649975230464 bcachefs-device-capacity=16000900661248 bcachefs-device-sb=3149824 bcachefs-device-journal=4294967296 bcachefs-device-btree=890241024 bcachefs-device-user=0"
        );

    let bcache = Node::new("block:/dev/bcache0", NodeKind::CacheDevice, "bcache0")
        .with_property("bcache.role", "backing")
        .with_property("bcache.kind", "cache-set")
        .with_property("bcache.backing-device", "/dev/sdb1")
        .with_property("bcache.set-uuid", "cache-set-uuid")
        .with_property("bcache.label", "fast-cache")
        .with_property("bcache.state", "clean")
        .with_property("bcache.running", "1")
        .with_property("bcache.cache-available-percent", "78")
        .with_property("bcache.cache-mode", "writeback")
        .with_property("bcache.cache-replacement-policy", "lru")
        .with_property("bcache.congested-read-threshold-us", "2000")
        .with_property("bcache.congested-write-threshold-us", "20000")
        .with_property("bcache.discard", "true")
        .with_property("bcache.dirty-data", "64.0M")
        .with_property("bcache.io-errors", "0")
        .with_property("bcache.metadata-written", "128.0M")
        .with_property("bcache.priority-stats", "Unused: 0% Metadata: 1%")
        .with_property("bcache.readahead", "0")
        .with_property("bcache.sequential-cutoff", "4.0M")
        .with_property("bcache.written", "512.0M")
        .with_property("bcache.writeback-delay", "30")
        .with_property("bcache.writeback-metadata", "true")
        .with_property("bcache.writeback-percent", "10")
        .with_property("bcache.writeback-rate", "1.0M/sec")
        .with_property("bcache.writeback-rate-debug", "rate=1024")
        .with_property("bcache.writeback-rate-d-term", "30")
        .with_property("bcache.writeback-rate-i-term-inverse", "10000")
        .with_property("bcache.writeback-rate-minimum", "4.0k")
        .with_property("bcache.writeback-rate-p-term-inverse", "40")
        .with_property("bcache.writeback-rate-update-seconds", "5")
        .with_property("bcache.writeback-running", "1");
    assert_eq!(
            usage_details(&bcache),
            "role=backing kind=cache-set backing-device=/dev/sdb1 set-uuid=cache-set-uuid label=fast-cache state=clean running=1 available-percent=78 cache-mode=writeback replacement=lru congested-read-us=2000 congested-write-us=20000 discard=true dirty=64.0M io-errors=0 metadata-written=128.0M priority-stats=Unused: 0% Metadata: 1% readahead=0 sequential-cutoff=4.0M written=512.0M writeback-delay=30 writeback-metadata=true writeback-percent=10 writeback-rate=1.0M/sec writeback-rate-debug=rate=1024 writeback-rate-d-term=30 writeback-rate-i-inverse=10000 writeback-rate-min=4.0k writeback-rate-p-inverse=40 writeback-rate-update=5 writeback-running=1"
        );

    let swap = Node::new("swap:/dev/zram0", NodeKind::Swap, "/dev/zram0")
        .with_property("swap.active", "true")
        .with_property("swap.type", "partition")
        .with_property("swap.priority", "100");
    assert_eq!(
        usage_details(&swap),
        "swap-active=true swap-type=partition swap-priority=100"
    );

    let loop_device = Node::new("block:/dev/loop0", NodeKind::LoopDevice, "/dev/loop0")
        .with_property("loop.back-file", "/var/lib/images/root.img")
        .with_property("loop.backing-inode", "12345")
        .with_property("loop.backing-major-minor", "0:45")
        .with_property("loop.major-minor", "7:0")
        .with_property("loop.offset", "1048576")
        .with_property("loop.sizelimit", "1073741824")
        .with_property("loop.logical-sector-size", "512")
        .with_property("loop.autoclear", "true")
        .with_property("loop.partscan", "true")
        .with_property("loop.read-only", "false")
        .with_property("loop.direct-io", "true");
    assert_eq!(
            usage_details(&loop_device),
            "back-file=/var/lib/images/root.img back-ino=12345 back-major-minor=0:45 major-minor=7:0 offset=1048576 sizelimit=1073741824 logical-sector=512 autoclear=true partscan=true ro=false dio=true"
        );

    let nvme = Node::new(
        "block:/dev/nvme0n1",
        NodeKind::NvmeNamespace,
        "/dev/nvme0n1",
    )
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
    .with_property("nvme.ana-state", "optimized");
    assert_eq!(
            usage_details(&nvme),
            "generic=/dev/ng0n1 nvme-model=Example NVMe product=Example Controller firmware=1.0 ns-index=0 namespace=1 nsid=1 ns-uuid=12345678-1234-1234-1234-123456789abc eui64=0011223344556677 nguid=00112233445566778899aabbccddeeff subsystem=nvme-subsys0 controller=nvme0 address=0000:01:00.0 transport=pcie controller-id=1 namespace-capacity=900000000000 lba-format=512 B + 0 B max-lba=1953125 sector-size=512 ana-state=optimized"
        );
}
