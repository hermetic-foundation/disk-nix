fn mount_details(node: &Node) -> String {
    const DETAIL_KEYS: &[(&str, &str)] = &[
        ("mount.source", "source"),
        ("mount.read-only", "ro"),
        ("mount.read-write", "rw"),
        ("mount.bind", "bind"),
        ("mount.propagation", "propagation"),
        ("mount.propagation.id", "propagation-id"),
        ("tmpfs.size", "tmpfs-size"),
        ("tmpfs.mode", "mode"),
        ("tmpfs.uid", "uid"),
        ("tmpfs.gid", "gid"),
        ("tmpfs.nr-inodes", "nr-inodes"),
        ("overlay.lowerdir", "lowerdir"),
        ("overlay.upperdir", "upperdir"),
        ("overlay.workdir", "workdir"),
        ("overlay.index", "index"),
    ];

    let details = DETAIL_KEYS
        .iter()
        .filter_map(|(key, label)| {
            property_value(node, key).map(|value| format!("{label}={value}"))
        })
        .collect::<Vec<_>>();

    if details.is_empty() {
        "-".to_string()
    } else {
        details.join(" ")
    }
}

fn backing_count(graph: &StorageGraph, node: &Node) -> usize {
    graph
        .edges
        .iter()
        .filter(|edge| {
            edge.to == node.id
                && matches!(
                    edge.relationship,
                    disk_nix_model::Relationship::Backs
                        | disk_nix_model::Relationship::DependsOn
                        | disk_nix_model::Relationship::MemberOf
                )
        })
        .count()
}

fn consumer_count(graph: &StorageGraph, node: &Node) -> usize {
    graph
        .edges
        .iter()
        .filter(|edge| {
            edge.from == node.id
                && matches!(
                    edge.relationship,
                    disk_nix_model::Relationship::Backs
                        | disk_nix_model::Relationship::DependsOn
                        | disk_nix_model::Relationship::MemberOf
                )
        })
        .count()
}

fn member_count(graph: &StorageGraph, node: &Node) -> usize {
    graph
        .edges
        .iter()
        .filter(|edge| {
            edge.to == node.id && edge.relationship == disk_nix_model::Relationship::MemberOf
        })
        .count()
}

fn iscsi_lun_count(graph: &StorageGraph, node: &Node) -> usize {
    match node.kind {
        NodeKind::IscsiSession => {
            let direct_luns = graph
                .edges
                .iter()
                .filter(|edge| {
                    edge.from == node.id
                        && edge.relationship == disk_nix_model::Relationship::Contains
                        && graph.nodes.iter().any(|candidate| {
                            candidate.id == edge.to && candidate.kind == NodeKind::Lun
                        })
                })
                .count();
            let target_luns = graph
                .edges
                .iter()
                .filter(|edge| {
                    edge.from == node.id
                        && edge.relationship == disk_nix_model::Relationship::ImportedFrom
                })
                .map(|edge| {
                    graph
                        .edges
                        .iter()
                        .filter(|candidate_edge| {
                            candidate_edge.from == edge.to
                                && candidate_edge.relationship
                                    == disk_nix_model::Relationship::Contains
                                && graph.nodes.iter().any(|candidate_node| {
                                    candidate_node.id == candidate_edge.to
                                        && candidate_node.kind == NodeKind::Lun
                                })
                        })
                        .count()
                })
                .sum::<usize>();
            direct_luns + target_luns
        }
        NodeKind::IscsiTarget => graph
            .edges
            .iter()
            .filter(|edge| {
                edge.from == node.id
                    && edge.relationship == disk_nix_model::Relationship::Contains
                    && graph
                        .nodes
                        .iter()
                        .any(|candidate| candidate.id == edge.to && candidate.kind == NodeKind::Lun)
            })
            .count(),
        _ => 0,
    }
}

fn nfs_mount_count(graph: &StorageGraph, node: &Node) -> usize {
    if node.kind != NodeKind::NfsExport {
        return 0;
    }

    graph
        .edges
        .iter()
        .filter(|edge| {
            edge.from == node.id
                && edge.relationship == disk_nix_model::Relationship::MountedAt
                && graph.nodes.iter().any(|candidate| {
                    candidate.id == edge.to && candidate.kind == NodeKind::NfsMount
                })
        })
        .count()
}

fn zfs_child_count(graph: &StorageGraph, node: &Node) -> usize {
    graph
        .edges
        .iter()
        .filter(|edge| {
            edge.from == node.id
                && matches!(
                    edge.relationship,
                    disk_nix_model::Relationship::Contains
                        | disk_nix_model::Relationship::MountedAt
                        | disk_nix_model::Relationship::SnapshotOf
                )
        })
        .count()
}

fn snapshot_source<'a>(graph: &'a StorageGraph, node: &Node) -> Option<&'a str> {
    graph
        .edges
        .iter()
        .find(|edge| {
            edge.from == node.id && edge.relationship == disk_nix_model::Relationship::SnapshotOf
        })
        .and_then(|edge| graph.nodes.iter().find(|candidate| candidate.id == edge.to))
        .map(|source| source.name.as_str())
}

fn property_value<'a>(node: &'a Node, key: &str) -> Option<&'a str> {
    node.properties
        .iter()
        .find(|property| property.key == key)
        .map(|property| property.value.as_str())
}

fn vdo_logical_display(node: &Node) -> String {
    property_value(node, "vdo.logical-size")
        .or_else(|| property_value(node, "lvm.vdo-logical-size"))
        .map(str::to_string)
        .unwrap_or_else(|| human_bytes(node.size_bytes))
}

fn vdo_physical_display(node: &Node) -> String {
    property_value(node, "vdo.physical-size")
        .or_else(|| property_value(node, "lvm.vdo-physical-size"))
        .map(str::to_string)
        .unwrap_or_else(|| human_bytes(node.usage.as_ref().and_then(|usage| usage.used_bytes)))
}

fn usage_percent(node: &Node) -> String {
    let Some(usage) = &node.usage else {
        return "-".to_string();
    };
    let Some(used) = usage.used_bytes else {
        return "-".to_string();
    };
    let capacity = node
        .size_bytes
        .or(usage.allocated_bytes)
        .or_else(|| usage.free_bytes.map(|free| used.saturating_add(free)));
    let Some(capacity) = capacity else {
        return "-".to_string();
    };
    if capacity == 0 {
        return "-".to_string();
    }

    format!("{:.1}%", (used as f64 / capacity as f64) * 100.0)
}

fn human_bytes(value: Option<u64>) -> String {
    let Some(bytes) = value else {
        return "-".to_string();
    };

    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut size = bytes as f64;
    let mut unit = UNITS[0];
    for next_unit in UNITS.iter().skip(1) {
        if size < 1024.0 {
            break;
        }
        size /= 1024.0;
        unit = next_unit;
    }

    if unit == "B" {
        format!("{bytes} B")
    } else {
        format!("{size:.1} {unit}")
    }
}
