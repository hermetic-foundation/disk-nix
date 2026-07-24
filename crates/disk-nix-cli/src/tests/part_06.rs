#[test]
fn mounts_table_includes_source_and_pseudo_mount_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("mount:/run", NodeKind::Mountpoint, "/run")
            .with_property("filesystem.type", "tmpfs")
            .with_property("mount.source", "tmpfs")
            .with_property("mount.read-write", "true")
            .with_property("tmpfs.size", "64M")
            .with_property("tmpfs.mode", "0755"),
    );
    graph.add_node(
        Node::new("mount:/srv/cache", NodeKind::Mountpoint, "/srv/cache")
            .with_property("filesystem.type", "none")
            .with_property("mount.source", "/var/cache/disk-nix")
            .with_property("mount.bind", "true"),
    );
    graph.add_node(
        Node::new("mount:/merged", NodeKind::Mountpoint, "/merged")
            .with_property("filesystem.type", "overlay")
            .with_property("mount.source", "overlay")
            .with_property("overlay.lowerdir", "/lower")
            .with_property("overlay.upperdir", "/upper")
            .with_property("overlay.workdir", "/work")
            .with_property("overlay.index", "off"),
    );

    assert_eq!(
        mount_details(
            graph
                .nodes
                .iter()
                .find(|node| node.name == "/run")
                .expect("tmpfs mount fixture should exist")
        ),
        "source=tmpfs rw=true tmpfs-size=64M mode=0755"
    );

    let mut output = Vec::new();
    print_mounts(&mut output, &graph).expect("mount table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("DETAILS"));
    assert!(output.contains("source=/var/cache/disk-nix bind=true"));
    assert!(
        output.contains("source=overlay lowerdir=/lower upperdir=/upper workdir=/work index=off")
    );
}

#[test]
fn install_zfs_root_template_includes_planner_and_mount_metadata() {
    let spec = install_zfs_root_spec(&InstallZfsRootOptions {
        disk: "/dev/disk/by-id/nvme-test".to_string(),
        pool: "zroot".to_string(),
        root_dataset: "zroot/root".to_string(),
        boot_label: "BOOT".to_string(),
        swap_label: "swap".to_string(),
        efi_start: "1MiB".to_string(),
        efi_end: "1025MiB".to_string(),
        swap_start: "1025MiB".to_string(),
        swap_end: "129GiB".to_string(),
        zfs_start: "129GiB".to_string(),
        part_prefix: None,
        encrypt: true,
    });

    assert_eq!(spec["version"], 1);
    assert_eq!(spec["apply"]["mode"], "install");
    assert_eq!(spec["apply"]["allowDestructive"], true);
    assert_eq!(spec["install"]["kind"], "nixos-zfs-root");
    assert_eq!(spec["install"]["zfs"]["loadKeyDataset"], "zroot/root");
    assert_eq!(
        spec["install"]["boot"]["fallbackDevice"],
        "/dev/disk/by-id/nvme-test-part1"
    );
    assert_eq!(
        spec["install"]["swap"]["fallbackDevice"],
        "/dev/disk/by-id/nvme-test-part2"
    );
    assert_eq!(
        spec["partitions"]["/dev/disk/by-id/nvme-test-zfs"]["start"],
        "129GiB"
    );
    assert_eq!(
        spec["datasets"]["zroot/root"]["properties"]["encryption"],
        "aes-256-gcm"
    );
    assert!(spec["pools"]["zroot"].get("preserveData").is_none());
    assert!(spec["datasets"]["zroot/root"].get("preserveData").is_none());

    let bytes = serde_json::to_vec(&spec).expect("install spec serializes");
    let (_plan, policy) =
        plan_and_policy_from_json_bytes(&bytes).expect("install spec is accepted by planner");
    assert!(policy.allow_destructive);
    assert!(policy.allow_format);
    assert_eq!(policy.mode, ApplyMode::Install);
}

#[test]
fn install_mount_script_renders_zfs_handoff_commands() {
    let spec = install_zfs_root_spec(&InstallZfsRootOptions {
        disk: "/dev/disk/by-id/nvme-test".to_string(),
        pool: "tank".to_string(),
        root_dataset: "tank/root".to_string(),
        boot_label: "ESP".to_string(),
        swap_label: "swap0".to_string(),
        efi_start: "1MiB".to_string(),
        efi_end: "1025MiB".to_string(),
        swap_start: "1025MiB".to_string(),
        swap_end: "65GiB".to_string(),
        zfs_start: "65GiB".to_string(),
        part_prefix: Some("/dev/disk/by-id/nvme-test-part".to_string()),
        encrypt: false,
    });

    let script =
        install_mount_script_from_spec(&spec, "/mnt").expect("mount script should render");

    assert!(script.contains("zpool export 'tank'"));
    assert!(script.contains("zpool import -R \"$target\" 'tank'"));
    assert!(!script.contains("zfs load-key"));
    assert!(script.contains("mount -t zfs 'tank/root' \"$target\""));
    assert!(script.contains("mount -t zfs 'tank/root/home' \"$target/home\""));
    assert!(script.contains("udevadm trigger --subsystem-match=block --action=change"));
    assert!(script.contains("udevadm settle"));
    assert!(script.contains("mount '/dev/disk/by-id/nvme-test-part1' \"$target/boot\""));
    assert!(script.contains("swapon '/dev/disk/by-id/nvme-test-part2'"));
}

#[test]
fn install_zfs_root_template_can_follow_custom_pool_dataset_names() {
    let spec = install_zfs_root_spec(&InstallZfsRootOptions {
        disk: "/dev/vdb".to_string(),
        pool: "disknix_install_e2e".to_string(),
        root_dataset: "disknix_install_e2e/root".to_string(),
        boot_label: "BOOT".to_string(),
        swap_label: "swap".to_string(),
        efi_start: "1MiB".to_string(),
        efi_end: "1025MiB".to_string(),
        swap_start: "1025MiB".to_string(),
        swap_end: "129MiB".to_string(),
        zfs_start: "129MiB".to_string(),
        part_prefix: Some("/dev/vdb".to_string()),
        encrypt: false,
    });

    assert_eq!(
        spec["install"]["zfs"]["rootDataset"],
        "disknix_install_e2e/root"
    );
    assert!(spec["datasets"]["disknix_install_e2e/root"].is_object());
    assert!(spec["datasets"]["disknix_install_e2e/root/home"].is_object());
}

#[test]
fn install_zfs_root_template_rejects_fat_labels_that_cannot_round_trip() {
    let mut output = Vec::new();
    let result = run(
        Cli::parse_from([
            "disk-nix",
            "install",
            "template",
            "zfs-root",
            "--disk",
            "/dev/vdb",
            "--boot-label",
            "DISKNIX-E2E-BOOT",
        ]),
        &mut output,
    );

    let error = result.expect_err("long FAT boot labels should be rejected");
    assert!(
        error
            .to_string()
            .contains("boot label \"DISKNIX-E2E-BOOT\" is too long")
    );
}
