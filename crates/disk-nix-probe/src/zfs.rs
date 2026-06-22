use disk_nix_model::{Edge, Node, NodeKind, Relationship, StorageGraph, Usage};

use crate::ProbeError;

pub fn normalize_zfs(zpool_list: &[u8], zfs_list: &[u8]) -> Result<StorageGraph, ProbeError> {
    let mut graph = StorageGraph::empty();

    for pool in parse_zpools(zpool_list)? {
        add_pool(&mut graph, pool);
    }
    for dataset in parse_datasets(zfs_list)? {
        add_dataset(&mut graph, dataset);
    }

    Ok(graph)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZpoolRow {
    name: String,
    size: Option<u64>,
    allocated: Option<u64>,
    free: Option<u64>,
    health: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZfsRow {
    name: String,
    kind: String,
    used: Option<u64>,
    available: Option<u64>,
    referenced: Option<u64>,
    mountpoint: Option<String>,
    origin: Option<String>,
}

fn parse_zpools(bytes: &[u8]) -> Result<Vec<ZpoolRow>, ProbeError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| ProbeError::Adapter(format!("failed to read zpool output: {error}")))?;
    let mut rows = Vec::new();

    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 5 {
            return Err(ProbeError::Adapter(format!(
                "zpool row has {} fields, expected at least 5: {line}",
                fields.len()
            )));
        }

        rows.push(ZpoolRow {
            name: fields[0].to_string(),
            size: parse_u64_field(fields[1]),
            allocated: parse_u64_field(fields[2]),
            free: parse_u64_field(fields[3]),
            health: nonempty(fields[4]),
        });
    }

    Ok(rows)
}

fn parse_datasets(bytes: &[u8]) -> Result<Vec<ZfsRow>, ProbeError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| ProbeError::Adapter(format!("failed to read zfs output: {error}")))?;
    let mut rows = Vec::new();

    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 7 {
            return Err(ProbeError::Adapter(format!(
                "zfs row has {} fields, expected at least 7: {line}",
                fields.len()
            )));
        }

        rows.push(ZfsRow {
            name: fields[0].to_string(),
            kind: fields[1].to_string(),
            used: parse_u64_field(fields[2]),
            available: parse_u64_field(fields[3]),
            referenced: parse_u64_field(fields[4]),
            mountpoint: nonempty_dash(fields[5]),
            origin: nonempty_dash(fields[6]),
        });
    }

    Ok(rows)
}

fn add_pool(graph: &mut StorageGraph, pool: ZpoolRow) {
    let mut node = Node::new(pool_id(&pool.name), NodeKind::ZfsPool, pool.name);

    if let Some(size_bytes) = pool.size {
        node = node.with_size_bytes(size_bytes);
    }

    let usage = Usage {
        used_bytes: pool.allocated,
        free_bytes: pool.free,
        allocated_bytes: pool.allocated,
    };
    if !usage.is_empty() {
        node = node.with_usage(usage);
    }

    if let Some(health) = pool.health {
        node = node.with_property("zfs.health", health);
    }

    graph.add_node(node);
}

fn add_dataset(graph: &mut StorageGraph, dataset: ZfsRow) {
    let kind = dataset_kind(&dataset.kind);
    let id = dataset_id(&dataset.name, kind);
    let mut node = Node::new(id.clone(), kind, dataset.name.clone())
        .with_property("zfs.type", dataset.kind.clone());

    let usage = Usage {
        used_bytes: dataset.used,
        free_bytes: dataset.available,
        allocated_bytes: dataset.referenced,
    };
    if !usage.is_empty() {
        node = node.with_usage(usage);
    }

    if let Some(mountpoint) = dataset
        .mountpoint
        .filter(|mountpoint| mountpoint != "legacy")
    {
        let mount_id = format!("mount:{mountpoint}");
        graph.add_node(Node::new(
            mount_id.clone(),
            NodeKind::Mountpoint,
            mountpoint,
        ));
        graph.add_edge(Edge::new(id.clone(), mount_id, Relationship::MountedAt));
    }

    if let Some(origin) = dataset.origin {
        graph.add_edge(Edge::new(
            id.clone(),
            dataset_id(&origin, NodeKind::ZfsSnapshot),
            Relationship::SnapshotOf,
        ));
    }

    if let Some(pool) = dataset
        .name
        .split('/')
        .next()
        .and_then(|value| value.split('@').next())
    {
        graph.add_edge(Edge::new(pool_id(pool), id.clone(), Relationship::Contains));
    }

    graph.add_node(node);
}

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

#[cfg(test)]
mod tests {
    use disk_nix_model::{NodeKind, Relationship};

    use super::*;

    const ZPOOL: &[u8] = b"tank\t1000\t400\t600\tONLINE\n";
    const ZFS: &[u8] = b"tank\tfilesystem\t100\t900\t100\t/tank\t-\n\
tank/home\tfilesystem\t200\t800\t200\t/home\t-\n\
tank/home@daily\tsnapshot\t10\t-\t10\t-\t-\n\
tank/vm\tvolume\t50\t950\t50\t-\t-\n";

    #[test]
    fn normalizes_zfs_pool_datasets_snapshots_and_zvols() {
        let graph = normalize_zfs(ZPOOL, ZFS).expect("fixture should parse");

        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::ZfsPool && node.name == "tank")
        );
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::ZfsDataset && node.name == "tank/home")
        );
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::ZfsSnapshot && node.name == "tank/home@daily")
        );
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::Zvol && node.name == "tank/vm")
        );
        assert!(
            graph
                .edges
                .iter()
                .any(|edge| edge.relationship == Relationship::MountedAt)
        );
    }
}
