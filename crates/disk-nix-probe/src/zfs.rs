use std::collections::BTreeMap;

use disk_nix_model::{Edge, Node, NodeKind, Relationship, StorageGraph, Usage};

use crate::ProbeError;

pub fn normalize_zfs(
    zpool_list: &[u8],
    zpool_get: &[u8],
    zfs_list: &[u8],
    zfs_holds: &[u8],
    zpool_status: &[u8],
) -> Result<StorageGraph, ProbeError> {
    let mut graph = StorageGraph::empty();

    for pool in parse_zpools(zpool_list)? {
        add_pool(&mut graph, pool);
    }
    for property in parse_zpool_properties(zpool_get)? {
        add_pool_property(&mut graph, property);
    }
    let datasets = parse_datasets(zfs_list)?;
    let dataset_kinds = dataset_kinds(&datasets);
    for dataset in datasets {
        add_dataset(&mut graph, dataset, &dataset_kinds);
    }
    for hold in parse_zfs_holds(zfs_holds)? {
        add_snapshot_hold(&mut graph, hold);
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
    capacity: Option<String>,
    dedupratio: Option<String>,
    fragmentation: Option<String>,
    altroot: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZpoolProperty {
    pool: String,
    property: String,
    value: String,
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
    compression: Option<String>,
    quota: Option<String>,
    reservation: Option<String>,
    encryption: Option<String>,
    keystatus: Option<String>,
    volsize: Option<String>,
    recordsize: Option<String>,
    dedup: Option<String>,
    checksum: Option<String>,
    copies: Option<String>,
    sync: Option<String>,
    primarycache: Option<String>,
    secondarycache: Option<String>,
    atime: Option<String>,
    relatime: Option<String>,
    snapdir: Option<String>,
    acltype: Option<String>,
    xattr: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZfsHold {
    snapshot: String,
    tag: String,
    timestamp: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZpoolStatus {
    name: String,
    state: Option<String>,
    status: Option<String>,
    action: Option<String>,
    scan: Option<String>,
    errors: Option<String>,
    read_errors: Option<String>,
    write_errors: Option<String>,
    checksum_errors: Option<String>,
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
            capacity: fields.get(5).and_then(|value| nonempty_dash(value)),
            dedupratio: fields.get(6).and_then(|value| nonempty_dash(value)),
            fragmentation: fields.get(7).and_then(|value| nonempty_dash(value)),
            altroot: fields.get(8).and_then(|value| nonempty_dash(value)),
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
                status: None,
                action: None,
                scan: None,
                errors: None,
                read_errors: None,
                write_errors: None,
                checksum_errors: None,
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
        if let Some(status) = trimmed.strip_prefix("status:").map(str::trim) {
            pool.status = nonempty(status);
            continue;
        }
        if let Some(action) = trimmed.strip_prefix("action:").map(str::trim) {
            pool.action = nonempty(action);
            continue;
        }
        if let Some(scan) = trimmed.strip_prefix("scan:").map(str::trim) {
            pool.scan = nonempty(scan);
            continue;
        }
        if trimmed == "config:" {
            in_config = true;
            continue;
        }
        if let Some(errors) = trimmed.strip_prefix("errors:").map(str::trim) {
            pool.errors = nonempty(errors);
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
        if vdev.name == pool.name {
            pool.read_errors = vdev.read_errors;
            pool.write_errors = vdev.write_errors;
            pool.checksum_errors = vdev.checksum_errors;
        } else {
            pool.vdevs.push(vdev);
        }
    }

    if let Some(pool) = current {
        pools.push(pool);
    }

    Ok(pools)
}

fn parse_zpool_properties(bytes: &[u8]) -> Result<Vec<ZpoolProperty>, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read zpool get output: {error}"))
    })?;
    let mut properties = Vec::new();

    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 3 {
            return Err(ProbeError::Adapter(format!(
                "zpool get row has {} fields, expected at least 3: {line}",
                fields.len()
            )));
        }

        let Some(value) = nonempty_dash(fields[2]) else {
            continue;
        };
        properties.push(ZpoolProperty {
            pool: fields[0].to_string(),
            property: fields[1].to_string(),
            value,
        });
    }

    Ok(properties)
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
            compression: fields.get(8).and_then(|value| nonempty_dash(value)),
            quota: fields.get(9).and_then(|value| nonempty_dash(value)),
            reservation: fields.get(10).and_then(|value| nonempty_dash(value)),
            encryption: fields.get(11).and_then(|value| nonempty_dash(value)),
            keystatus: fields.get(12).and_then(|value| nonempty_dash(value)),
            volsize: fields.get(13).and_then(|value| nonempty_dash(value)),
            recordsize: fields.get(14).and_then(|value| nonempty_dash(value)),
            dedup: fields.get(15).and_then(|value| nonempty_dash(value)),
            checksum: fields.get(16).and_then(|value| nonempty_dash(value)),
            copies: fields.get(17).and_then(|value| nonempty_dash(value)),
            sync: fields.get(18).and_then(|value| nonempty_dash(value)),
            primarycache: fields.get(19).and_then(|value| nonempty_dash(value)),
            secondarycache: fields.get(20).and_then(|value| nonempty_dash(value)),
            atime: fields.get(21).and_then(|value| nonempty_dash(value)),
            relatime: fields.get(22).and_then(|value| nonempty_dash(value)),
            snapdir: fields.get(23).and_then(|value| nonempty_dash(value)),
            acltype: fields.get(24).and_then(|value| nonempty_dash(value)),
            xattr: fields.get(25).and_then(|value| nonempty_dash(value)),
        });
    }

    Ok(rows)
}

fn parse_zfs_holds(bytes: &[u8]) -> Result<Vec<ZfsHold>, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read zfs holds output: {error}"))
    })?;
    let mut holds = Vec::new();

    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 2 {
            return Err(ProbeError::Adapter(format!(
                "zfs holds row has {} fields, expected at least 2: {line}",
                fields.len()
            )));
        }
        holds.push(ZfsHold {
            snapshot: fields[0].to_string(),
            tag: fields[1].to_string(),
            timestamp: fields.get(2).and_then(|value| nonempty_dash(value)),
        });
    }

    Ok(holds)
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
    for (key, value) in [
        ("zfs.pool-capacity", pool.capacity),
        ("zfs.pool-dedupratio", pool.dedupratio),
        ("zfs.pool-fragmentation", pool.fragmentation),
        ("zfs.pool-altroot", pool.altroot),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
    }

    graph.add_node(node);
}

fn add_pool_property(graph: &mut StorageGraph, property: ZpoolProperty) {
    let key = format!(
        "zfs.pool-{}",
        property
            .property
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
    );
    graph.add_node(
        Node::new(pool_id(&property.pool), NodeKind::ZfsPool, property.pool)
            .with_property(key, property.value),
    );
}

fn add_status_pool(graph: &mut StorageGraph, pool: ZpoolStatus) {
    let mut node = Node::new(pool_id(&pool.name), NodeKind::ZfsPool, pool.name.clone());
    if let Some(state) = pool.state {
        node = node.with_property("zfs.state", state);
    }
    for (key, value) in [
        ("zfs.status", pool.status),
        ("zfs.action", pool.action),
        ("zfs.scan", pool.scan),
        ("zfs.errors", pool.errors),
        ("zfs.pool-read-errors", pool.read_errors),
        ("zfs.pool-write-errors", pool.write_errors),
        ("zfs.pool-checksum-errors", pool.checksum_errors),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
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

fn dataset_kinds(datasets: &[ZfsRow]) -> BTreeMap<String, NodeKind> {
    datasets
        .iter()
        .filter(|dataset| dataset.kind != "snapshot")
        .map(|dataset| (dataset.name.clone(), dataset_kind(&dataset.kind)))
        .collect()
}

fn add_dataset(
    graph: &mut StorageGraph,
    dataset: ZfsRow,
    dataset_kinds: &BTreeMap<String, NodeKind>,
) {
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

    if kind == NodeKind::ZfsSnapshot {
        if let Some(source) = dataset.name.split_once('@').map(|(source, _)| source) {
            let source_kind = dataset_kinds
                .get(source)
                .copied()
                .unwrap_or(NodeKind::ZfsDataset);
            graph.add_edge(Edge::new(
                id.clone(),
                dataset_id(source, source_kind),
                Relationship::SnapshotOf,
            ));
        }
    }

    if let Some(userrefs) = dataset.userrefs {
        node = node.with_property("zfs.userrefs", userrefs);
    }
    for (key, value) in [
        ("zfs.compression", dataset.compression),
        ("zfs.quota", dataset.quota),
        ("zfs.reservation", dataset.reservation),
        ("zfs.encryption", dataset.encryption),
        ("zfs.keystatus", dataset.keystatus),
        ("zfs.volsize", dataset.volsize),
        ("zfs.recordsize", dataset.recordsize),
        ("zfs.dedup", dataset.dedup),
        ("zfs.checksum", dataset.checksum),
        ("zfs.copies", dataset.copies),
        ("zfs.sync", dataset.sync),
        ("zfs.primarycache", dataset.primarycache),
        ("zfs.secondarycache", dataset.secondarycache),
        ("zfs.atime", dataset.atime),
        ("zfs.relatime", dataset.relatime),
        ("zfs.snapdir", dataset.snapdir),
        ("zfs.acltype", dataset.acltype),
        ("zfs.xattr", dataset.xattr),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
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

fn add_snapshot_hold(graph: &mut StorageGraph, hold: ZfsHold) {
    let tag_key = normalize_property_suffix(&hold.tag);
    let node = Node::new(
        dataset_id(&hold.snapshot, NodeKind::ZfsSnapshot),
        NodeKind::ZfsSnapshot,
        hold.snapshot,
    )
    .with_property("zfs.holds", hold.tag.clone())
    .with_property(
        format!("zfs.hold.{tag_key}"),
        hold.timestamp.unwrap_or_else(|| "present".to_string()),
    )
    .with_property(format!("zfs.hold-tag.{tag_key}"), hold.tag);
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

#[cfg(test)]
mod tests {
    use disk_nix_model::{NodeKind, Relationship};

    use super::*;

    const ZPOOL: &[u8] = b"tank\t1000\t400\t600\tONLINE\t40%\t1.00x\t12%\t/mnt/rescue\n";
    const ZPOOL_GET: &[u8] = b"tank\taltroot\t/mnt/rescue\n\
tank\tashift\t12\n\
tank\tautotrim\ton\n\
tank\tautoexpand\toff\n\
tank\tautoreplace\toff\n\
tank\tbootfs\ttank/root\n\
tank\tcachefile\t/etc/zfs/zpool.cache\n\
tank\tcomment\tprimary pool\n\
tank\tdelegation\ton\n\
tank\tfailmode\twait\n\
tank\tlistsnapshots\toff\n\
tank\tmultihost\toff\n";
    const ZFS: &[u8] = b"tank\tfilesystem\t100\t900\t100\t/tank\t-\t-\tlz4\tnone\tnone\toff\t-\t-\t131072\toff\ton\t1\tstandard\tall\tall\ton\toff\thidden\toff\tsa\n\
tank/home\tfilesystem\t200\t800\t200\t/home\t-\t-\tzstd\t1073741824\t268435456\taes-256-gcm\tavailable\t-\t1048576\toff\tsha512\t2\tdisabled\tmetadata\tall\toff\ton\tvisible\tposixacl\tsa\n\
tank/home@daily\tsnapshot\t10\t-\t10\t-\t-\t2\tzstd\t-\t-\taes-256-gcm\tavailable\t-\t1048576\toff\tsha512\t2\tdisabled\tmetadata\tall\toff\ton\tvisible\tposixacl\tsa\n\
tank/vm\tvolume\t50\t950\t50\t-\t-\t-\tlz4\t-\t-\toff\t-\t85899345920\t-\ton\tfletcher4\t1\tstandard\tall\tnone\toff\toff\thidden\toff\ton\n\
tank/vm@clean\tsnapshot\t5\t-\t5\t-\t-\t1\tlz4\t-\t-\toff\t-\t-\t-\ton\tfletcher4\t1\tstandard\tall\tnone\toff\toff\thidden\toff\ton\n";
    const ZFS_HOLDS: &[u8] = b"tank/home@daily\tdisk-nix-retain\tWed Jun 24 18:00 2026\n\
tank/home@daily\tbackup-job\tWed Jun 24 18:01 2026\n";
    const ZPOOL_STATUS: &[u8] = br#"
  pool: tank
 state: ONLINE
  scan: scrub repaired 0B in 00:01:02 with 0 errors on Sun Jun 21 00:00:00 2026
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
    const DEGRADED_ZPOOL_STATUS: &[u8] = br#"
  pool: tank
 state: DEGRADED
status: One or more devices could not be used because the label is missing or invalid.
action: Replace the device using 'zpool replace'.
  scan: resilvered 1024B in 00:00:01 with 0 errors on Sun Jun 21 00:00:00 2026
config:

        NAME                                      STATE     READ WRITE CKSUM
        tank                                      DEGRADED     4     5     6
          /dev/disk/by-id/disk-a-part1           ONLINE       0     0     0

errors: No known data errors
"#;

    #[test]
    fn normalizes_zfs_pool_datasets_snapshots_and_zvols() {
        let graph = normalize_zfs(ZPOOL, ZPOOL_GET, ZFS, ZFS_HOLDS, ZPOOL_STATUS)
            .expect("fixture should parse");

        assert!(graph
            .nodes
            .iter()
            .any(|node| node.kind == NodeKind::ZfsPool && node.name == "tank"));
        let pool = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::ZfsPool && node.name == "tank")
            .expect("pool node exists");
        assert!(pool
            .properties
            .iter()
            .any(|property| { property.key == "zfs.pool-capacity" && property.value == "40%" }));
        assert!(pool.properties.iter().any(|property| {
            property.key == "zfs.pool-dedupratio" && property.value == "1.00x"
        }));
        assert!(pool.properties.iter().any(|property| {
            property.key == "zfs.pool-fragmentation" && property.value == "12%"
        }));
        assert!(pool.properties.iter().any(|property| {
            property.key == "zfs.pool-altroot" && property.value == "/mnt/rescue"
        }));
        assert!(pool
            .properties
            .iter()
            .any(|property| property.key == "zfs.pool-ashift" && property.value == "12"));
        assert!(pool
            .properties
            .iter()
            .any(|property| property.key == "zfs.pool-autotrim" && property.value == "on"));
        assert!(pool
            .properties
            .iter()
            .any(|property| { property.key == "zfs.pool-autoexpand" && property.value == "off" }));
        assert!(pool.properties.iter().any(|property| {
            property.key == "zfs.pool-cachefile" && property.value == "/etc/zfs/zpool.cache"
        }));
        assert!(pool
            .properties
            .iter()
            .any(|property| { property.key == "zfs.pool-failmode" && property.value == "wait" }));
        assert!(graph
            .nodes
            .iter()
            .any(|node| node.kind == NodeKind::ZfsDataset && node.name == "tank/home"));
        assert!(graph
            .nodes
            .iter()
            .any(|node| node.kind == NodeKind::ZfsSnapshot && node.name == "tank/home@daily"));
        assert!(graph
            .nodes
            .iter()
            .any(|node| node.kind == NodeKind::ZfsSnapshot && node.name == "tank/vm@clean"));
        let snapshot = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::ZfsSnapshot && node.name == "tank/home@daily")
            .expect("snapshot node exists");
        assert!(snapshot
            .properties
            .iter()
            .any(|property| property.key == "zfs.userrefs" && property.value == "2"));
        assert!(snapshot.properties.iter().any(|property| {
            property.key == "zfs.holds" && property.value == "disk-nix-retain"
        }));
        assert!(snapshot.properties.iter().any(|property| {
            property.key == "zfs.hold.disk-nix-retain" && property.value == "Wed Jun 24 18:00 2026"
        }));
        assert!(snapshot
            .properties
            .iter()
            .any(|property| property.key == "zfs.hold.backup-job"));
        let dataset = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::ZfsDataset && node.name == "tank/home")
            .expect("dataset node exists");
        assert!(dataset
            .properties
            .iter()
            .any(|property| property.key == "zfs.compression" && property.value == "zstd"));
        assert!(dataset.properties.iter().any(|property| {
            property.key == "zfs.encryption" && property.value == "aes-256-gcm"
        }));
        assert!(dataset
            .properties
            .iter()
            .any(|property| { property.key == "zfs.recordsize" && property.value == "1048576" }));
        assert!(dataset
            .properties
            .iter()
            .any(|property| property.key == "zfs.checksum" && property.value == "sha512"));
        assert!(dataset
            .properties
            .iter()
            .any(|property| property.key == "zfs.primarycache" && property.value == "metadata"));
        let zvol = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::Zvol && node.name == "tank/vm")
            .expect("zvol node exists");
        assert!(zvol
            .properties
            .iter()
            .any(|property| property.key == "zfs.volsize" && property.value == "85899345920"));
        assert!(graph
            .nodes
            .iter()
            .any(|node| node.kind == NodeKind::Zvol && node.name == "tank/vm"));
        assert!(graph
            .edges
            .iter()
            .any(|edge| edge.relationship == Relationship::MountedAt));
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
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "zfs-snapshot:tank/home@daily"
                && edge.to.0 == "zfs-dataset:tank/home"
                && edge.relationship == Relationship::SnapshotOf
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "zfs-snapshot:tank/vm@clean"
                && edge.to.0 == "zvol:tank/vm"
                && edge.relationship == Relationship::SnapshotOf
        }));
    }

    #[test]
    fn normalizes_zpool_status_advisory_fields() {
        let graph = normalize_zfs(ZPOOL, ZPOOL_GET, ZFS, ZFS_HOLDS, DEGRADED_ZPOOL_STATUS)
            .expect("fixture should parse");
        let pool = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::ZfsPool && node.name == "tank")
            .expect("pool node exists");

        assert!(pool.properties.iter().any(|property| {
            property.key == "zfs.status"
                && property.value
                    == "One or more devices could not be used because the label is missing or invalid."
        }));
        assert!(pool.properties.iter().any(|property| {
            property.key == "zfs.action"
                && property.value == "Replace the device using 'zpool replace'."
        }));
        assert!(pool.properties.iter().any(|property| {
            property.key == "zfs.scan"
                && property.value
                    == "resilvered 1024B in 00:00:01 with 0 errors on Sun Jun 21 00:00:00 2026"
        }));
        assert!(pool.properties.iter().any(|property| {
            property.key == "zfs.errors" && property.value == "No known data errors"
        }));
        assert!(pool
            .properties
            .iter()
            .any(|property| { property.key == "zfs.pool-read-errors" && property.value == "4" }));
        assert!(pool
            .properties
            .iter()
            .any(|property| { property.key == "zfs.pool-write-errors" && property.value == "5" }));
        assert!(pool.properties.iter().any(|property| {
            property.key == "zfs.pool-checksum-errors" && property.value == "6"
        }));
    }
}
