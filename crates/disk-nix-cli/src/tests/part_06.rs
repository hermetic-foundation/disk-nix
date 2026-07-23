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
