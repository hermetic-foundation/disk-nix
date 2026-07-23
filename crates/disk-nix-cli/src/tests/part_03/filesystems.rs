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
