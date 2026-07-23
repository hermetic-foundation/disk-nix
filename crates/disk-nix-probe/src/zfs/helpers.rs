fn dataset_kind(kind: &str) -> NodeKind {
    match kind {
        "filesystem" => NodeKind::ZfsDataset,
        "snapshot" => NodeKind::ZfsSnapshot,
        "volume" => NodeKind::Zvol,
        _ => NodeKind::ZfsDataset,
    }
}

fn pool_id(name: &str) -> String {
    format!("zfs-pool:{name}")
}

fn vdev_id(pool_name: &str, name: &str) -> String {
    format!("zfs-vdev:{pool_name}:{name}")
}

fn dataset_id(name: &str, kind: NodeKind) -> String {
    match kind {
        NodeKind::ZfsSnapshot => format!("zfs-snapshot:{name}"),
        NodeKind::Zvol => format!("zvol:{name}"),
        _ => format!("zfs-dataset:{name}"),
    }
}

fn parse_u64_field(value: &str) -> Option<u64> {
    match value {
        "" | "-" => None,
        _ => value.parse().ok(),
    }
}

fn nonempty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_string())
}

fn nonempty_dash(value: &str) -> Option<String> {
    (!value.is_empty() && value != "-").then(|| value.to_string())
}

fn normalize_property_suffix(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            'A'..='Z' => character.to_ascii_lowercase(),
            'a'..='z' | '0'..='9' => character,
            _ => '-',
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
