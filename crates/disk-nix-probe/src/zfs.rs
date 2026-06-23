use disk_nix_model::{Edge, Node, NodeKind, Relationship, StorageGraph, Usage};

use crate::ProbeError;

pub fn normalize_zfs(
    zpool_list: &[u8],
    zfs_list: &[u8],
    zpool_status: &[u8],
) -> Result<StorageGraph, ProbeError> {
    let mut graph = StorageGraph::empty();

    for pool in parse_zpools(zpool_list)? {
        add_pool(&mut graph, pool);
    }
    for dataset in parse_datasets(zfs_list)? {
        add_dataset(&mut graph, dataset);
    }
    for pool in parse_zpool_status(zpool_status)? {
        add_status_pool(&mut graph, pool);
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
    userrefs: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZpoolStatus {
    name: String,
    state: Option<String>,
    vdevs: Vec<ZpoolVdev>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZpoolVdev {
    name: String,
    role: String,
    parent: Option<String>,
    state: Option<String>,
    read_errors: Option<String>,
    write_errors: Option<String>,
    checksum_errors: Option<String>,
    device_path: Option<String>,
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

fn parse_zpool_status(bytes: &[u8]) -> Result<Vec<ZpoolStatus>, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read zpool status output: {error}"))
    })?;
    let mut pools = Vec::new();
    let mut current: Option<ZpoolStatus> = None;
    let mut in_config = false;
    let mut role = "data".to_string();
    let mut stack: Vec<(usize, String)> = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(name) = trimmed.strip_prefix("pool:").map(str::trim) {
            if let Some(pool) = current.take() {
                pools.push(pool);
            }
            current = Some(ZpoolStatus {
                name: name.to_string(),
                state: None,
                vdevs: Vec::new(),
            });
            in_config = false;
            role = "data".to_string();
            stack.clear();
            continue;
        }

        let Some(pool) = &mut current else {
            continue;
        };

        if let Some(state) = trimmed.strip_prefix("state:").map(str::trim) {
            pool.state = nonempty(state);
            continue;
        }
        if trimmed == "config:" {
            in_config = true;
            continue;
        }
        if trimmed == "errors:" || trimmed.starts_with("errors:") {
            in_config = false;
            continue;
        }
        if !in_config || trimmed.starts_with("NAME ") {
            continue;
        }
        if matches!(trimmed, "logs" | "cache" | "spares" | "special" | "dedup") {
            role = trimmed.to_string();
            stack.clear();
            continue;
        }

        let Some(vdev) = parse_vdev_line(&pool.name, &role, line, &mut stack) else {
            continue;
        };
        if vdev.name != pool.name {
            pool.vdevs.push(vdev);
        }
    }

    if let Some(pool) = current {
        pools.push(pool);
    }

    Ok(pools)
}

fn parse_vdev_line(
    pool_name: &str,
    role: &str,
    line: &str,
    stack: &mut Vec<(usize, String)>,
) -> Option<ZpoolVdev> {
    let indent = line
        .chars()
        .take_while(|character| *character == ' ')
        .count();
    let fields: Vec<&str> = line.split_whitespace().collect();
    let name = fields.first()?.to_string();
    let state = fields.get(1).map(|value| (*value).to_string());
    let parent = stack
        .iter()
        .rev()
        .find(|(parent_indent, _)| *parent_indent < indent)
        .map(|(_, parent)| parent.clone());

    stack.retain(|(parent_indent, _)| *parent_indent < indent);
    stack.push((indent, name.clone()));

    Some(ZpoolVdev {
        device_path: name.starts_with("/dev/").then(|| name.clone()),
        name: name.clone(),
        role: if name == pool_name {
            "pool".to_string()
        } else {
            role.to_string()
        },
        parent,
        state,
        read_errors: fields.get(2).map(|value| (*value).to_string()),
        write_errors: fields.get(3).map(|value| (*value).to_string()),
        checksum_errors: fields.get(4).map(|value| (*value).to_string()),
    })
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
            userrefs: fields.get(7).and_then(|value| nonempty_dash(value)),
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

fn add_status_pool(graph: &mut StorageGraph, pool: ZpoolStatus) {
    let mut node = Node::new(pool_id(&pool.name), NodeKind::ZfsPool, pool.name.clone());
    if let Some(state) = pool.state {
        node = node.with_property("zfs.state", state);
    }
    graph.add_node(node);

    for vdev in pool.vdevs {
        add_vdev(graph, &pool.name, vdev);
    }
}

fn add_vdev(graph: &mut StorageGraph, pool_name: &str, vdev: ZpoolVdev) {
    let id = vdev_id(pool_name, &vdev.name);
    let mut node = Node::new(id.clone(), NodeKind::ZfsVdev, vdev.name.clone())
        .with_property("zfs.vdev-role", vdev.role.clone());

    for (key, value) in [
        ("zfs.vdev-state", vdev.state),
        ("zfs.read-errors", vdev.read_errors),
        ("zfs.write-errors", vdev.write_errors),
        ("zfs.checksum-errors", vdev.checksum_errors),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
    }
    if let Some(path) = &vdev.device_path {
        node = node.with_path(path.clone());
    }

    graph.add_node(node);

    if let Some(parent) = vdev.parent.filter(|parent| parent != pool_name) {
        graph.add_edge(Edge::new(
            vdev_id(pool_name, &parent),
            id.clone(),
            Relationship::Contains,
        ));
    } else {
        graph.add_edge(Edge::new(
            pool_id(pool_name),
            id.clone(),
            Relationship::Contains,
        ));
    }

    if let Some(path) = vdev.device_path {
        let block_id = format!("block:{path}");
        graph.add_node(
            Node::new(block_id.clone(), NodeKind::PhysicalDisk, path.clone()).with_path(path),
        );
        graph.add_edge(Edge::new(block_id, id, Relationship::Backs));
    }
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
        node = node.with_property("zfs.origin", origin.clone());
        graph.add_edge(Edge::new(
            id.clone(),
            dataset_id(&origin, NodeKind::ZfsSnapshot),
            Relationship::SnapshotOf,
        ));
    }

    if let Some(userrefs) = dataset.userrefs {
        node = node.with_property("zfs.userrefs", userrefs);
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

#[cfg(test)]
mod tests {
    use disk_nix_model::{NodeKind, Relationship};

    use super::*;

    const ZPOOL: &[u8] = b"tank\t1000\t400\t600\tONLINE\n";
    const ZFS: &[u8] = b"tank\tfilesystem\t100\t900\t100\t/tank\t-\t-\n\
tank/home\tfilesystem\t200\t800\t200\t/home\t-\t-\n\
tank/home@daily\tsnapshot\t10\t-\t10\t-\t-\t2\n\
tank/vm\tvolume\t50\t950\t50\t-\t-\t-\n";
    const ZPOOL_STATUS: &[u8] = br#"
  pool: tank
 state: ONLINE
config:

        NAME                                      STATE     READ WRITE CKSUM
        tank                                      ONLINE       0     0     0
          mirror-0                                ONLINE       0     0     0
            /dev/disk/by-id/disk-a-part1         ONLINE       0     0     0
            /dev/disk/by-id/disk-b-part1         ONLINE       0     0     0
        logs
          /dev/disk/by-id/log0                   ONLINE       0     0     0
        cache
          /dev/disk/by-id/cache0                 ONLINE       0     0     0

errors: No known data errors
"#;

    #[test]
    fn normalizes_zfs_pool_datasets_snapshots_and_zvols() {
        let graph = normalize_zfs(ZPOOL, ZFS, ZPOOL_STATUS).expect("fixture should parse");

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
        let snapshot = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::ZfsSnapshot && node.name == "tank/home@daily")
            .expect("snapshot node exists");
        assert!(
            snapshot
                .properties
                .iter()
                .any(|property| property.key == "zfs.userrefs" && property.value == "2")
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
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::ZfsVdev
                && node.name == "mirror-0"
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "zfs.vdev-role" && property.value == "data")
        }));
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::ZfsVdev
                && node.name == "/dev/disk/by-id/cache0"
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "zfs.vdev-role" && property.value == "cache")
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "block:/dev/disk/by-id/disk-a-part1"
                && edge.to.0 == "zfs-vdev:tank:/dev/disk/by-id/disk-a-part1"
                && edge.relationship == Relationship::Backs
        }));
    }
}
