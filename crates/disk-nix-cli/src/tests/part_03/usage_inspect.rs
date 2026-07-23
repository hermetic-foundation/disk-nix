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
