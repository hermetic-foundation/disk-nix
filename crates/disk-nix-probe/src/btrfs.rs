use disk_nix_model::{Edge, Identity, Node, NodeKind, Relationship, StorageGraph, Usage};

use crate::ProbeError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BtrfsReport {
    pub target: String,
    pub show: Vec<u8>,
    pub usage: Vec<u8>,
    pub subvolumes: Vec<u8>,
    pub qgroups: Vec<u8>,
}

pub fn normalize_btrfs_reports(reports: &[BtrfsReport]) -> Result<StorageGraph, ProbeError> {
    let mut graph = StorageGraph::empty();

    for report in reports {
        add_report(&mut graph, report)?;
    }

    Ok(graph)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FilesystemShow {
    label: Option<String>,
    uuid: Option<String>,
    devices: Vec<BtrfsDevice>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BtrfsDevice {
    id: Option<String>,
    size_bytes: Option<u64>,
    used_bytes: Option<u64>,
    path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FilesystemUsage {
    device_size: Option<u64>,
    device_allocated: Option<u64>,
    device_unallocated: Option<u64>,
    used: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Subvolume {
    id: String,
    parent_uuid: Option<String>,
    uuid: Option<String>,
    path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Qgroup {
    id: String,
    referenced: Option<u64>,
    exclusive: Option<u64>,
    max_referenced: Option<String>,
    max_exclusive: Option<String>,
}

fn add_report(graph: &mut StorageGraph, report: &BtrfsReport) -> Result<(), ProbeError> {
    let show = parse_filesystem_show(&report.show)?;
    let usage = parse_filesystem_usage(&report.usage)?;
    let subvolumes = parse_subvolumes(&report.subvolumes)?;
    let qgroups = parse_qgroups(&report.qgroups)?;
    let filesystem_id = show.uuid.as_ref().map_or_else(
        || format!("btrfs:{}", report.target),
        |uuid| format!("btrfs:{uuid}"),
    );

    let mut filesystem = Node::new(
        filesystem_id.clone(),
        NodeKind::BtrfsFilesystem,
        show.label.clone().unwrap_or_else(|| report.target.clone()),
    );

    filesystem = filesystem.with_property("btrfs.mount-target", report.target.clone());

    if let Some(uuid) = show.uuid {
        filesystem = filesystem.with_identity(Identity {
            uuid: Some(uuid),
            label: show.label,
            ..Identity::default()
        });
    } else if let Some(label) = show.label {
        filesystem.identity.label = Some(label);
    }

    if let Some(size_bytes) = usage.device_size {
        filesystem = filesystem.with_size_bytes(size_bytes);
    }

    let fs_usage = Usage {
        used_bytes: usage.used,
        free_bytes: usage.device_unallocated,
        allocated_bytes: usage.device_allocated,
    };
    if !fs_usage.is_empty() {
        filesystem = filesystem.with_usage(fs_usage);
    }

    graph.add_node(filesystem);

    let mount_id = format!("mount:{}", report.target);
    graph.add_node(Node::new(
        mount_id.clone(),
        NodeKind::Mountpoint,
        report.target.clone(),
    ));
    graph.add_edge(Edge::new(
        filesystem_id.clone(),
        mount_id,
        Relationship::MountedAt,
    ));

    for device in show.devices {
        add_device(graph, &filesystem_id, device);
    }

    for subvolume in subvolumes {
        add_subvolume(graph, &filesystem_id, subvolume);
    }
    for qgroup in qgroups {
        add_qgroup(graph, &filesystem_id, qgroup);
    }

    Ok(())
}

fn add_device(graph: &mut StorageGraph, filesystem_id: &str, device: BtrfsDevice) {
    let block_id = format!("block:{}", device.path);
    let mut node = Node::new(block_id.clone(), NodeKind::Partition, device.path.clone())
        .with_path(device.path);

    if let Some(size_bytes) = device.size_bytes {
        node = node.with_size_bytes(size_bytes);
    }

    let usage = Usage {
        used_bytes: device.used_bytes,
        free_bytes: None,
        allocated_bytes: device.used_bytes,
    };
    if !usage.is_empty() {
        node = node.with_usage(usage);
    }

    if let Some(id) = device.id {
        node = node.with_property("btrfs.device-id", id);
    }

    graph.add_node(node);
    graph.add_edge(Edge::new(
        block_id,
        filesystem_id.to_string(),
        Relationship::MemberOf,
    ));
}

fn add_subvolume(graph: &mut StorageGraph, filesystem_id: &str, subvolume: Subvolume) {
    let kind = if subvolume.parent_uuid.is_some() || subvolume.path.contains("snapshot") {
        NodeKind::BtrfsSnapshot
    } else {
        NodeKind::BtrfsSubvolume
    };
    let id = format!("btrfs-subvolume:{}:{}", filesystem_id, subvolume.path);
    let mut node =
        Node::new(id.clone(), kind, subvolume.path).with_property("btrfs.id", subvolume.id);

    if let Some(uuid) = subvolume.uuid {
        node.identity.uuid = Some(uuid);
    }
    if let Some(parent_uuid) = subvolume.parent_uuid {
        node = node.with_property("btrfs.parent-uuid", parent_uuid.clone());
        graph.add_edge(Edge::new(
            id.clone(),
            format!("btrfs-subvolume-parent:{parent_uuid}"),
            Relationship::SnapshotOf,
        ));
    }

    graph.add_node(node);
    graph.add_edge(Edge::new(
        filesystem_id.to_string(),
        id,
        Relationship::Contains,
    ));
}

fn add_qgroup(graph: &mut StorageGraph, filesystem_id: &str, qgroup: Qgroup) {
    let id = format!("btrfs-qgroup:{filesystem_id}:{}", qgroup.id);
    let mut node = Node::new(id.clone(), NodeKind::BtrfsQgroup, qgroup.id.clone())
        .with_property("btrfs.qgroup-id", qgroup.id);

    let usage = Usage {
        used_bytes: qgroup.referenced,
        free_bytes: None,
        allocated_bytes: qgroup.exclusive,
    };
    if !usage.is_empty() {
        node = node.with_usage(usage);
    }

    for (key, value) in [
        ("btrfs.max-referenced", qgroup.max_referenced),
        ("btrfs.max-exclusive", qgroup.max_exclusive),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
    }

    graph.add_node(node);
    graph.add_edge(Edge::new(
        filesystem_id.to_string(),
        id,
        Relationship::Contains,
    ));
}

fn parse_filesystem_show(bytes: &[u8]) -> Result<FilesystemShow, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read btrfs show output: {error}"))
    })?;
    let mut label = None;
    let mut uuid = None;
    let mut devices = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Label:") {
            label = extract_quoted(trimmed, "Label:");
            uuid = extract_after(trimmed, "uuid:");
        } else if trimmed.starts_with("devid") {
            if let Some(device) = parse_device_line(trimmed) {
                devices.push(device);
            }
        }
    }

    Ok(FilesystemShow {
        label,
        uuid,
        devices,
    })
}

fn parse_device_line(line: &str) -> Option<BtrfsDevice> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    let id = value_after(&parts, "devid").map(str::to_string);
    let size_bytes = value_after(&parts, "size").and_then(parse_u64);
    let used_bytes = value_after(&parts, "used").and_then(parse_u64);
    let path = value_after(&parts, "path")?.to_string();

    Some(BtrfsDevice {
        id,
        size_bytes,
        used_bytes,
        path,
    })
}

fn parse_filesystem_usage(bytes: &[u8]) -> Result<FilesystemUsage, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read btrfs usage output: {error}"))
    })?;
    let mut usage = FilesystemUsage {
        device_size: None,
        device_allocated: None,
        device_unallocated: None,
        used: None,
    };

    for line in text.lines().map(str::trim) {
        if let Some(value) = line.strip_prefix("Device size:") {
            usage.device_size = parse_u64(value.trim());
        } else if let Some(value) = line.strip_prefix("Device allocated:") {
            usage.device_allocated = parse_u64(value.trim());
        } else if let Some(value) = line.strip_prefix("Device unallocated:") {
            usage.device_unallocated = parse_u64(value.trim());
        } else if let Some(value) = line.strip_prefix("Used:") {
            usage.used = parse_u64(value.trim());
        }
    }

    Ok(usage)
}

fn parse_subvolumes(bytes: &[u8]) -> Result<Vec<Subvolume>, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read btrfs subvolume output: {error}"))
    })?;
    let mut subvolumes = Vec::new();

    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        if let Some(subvolume) = parse_subvolume_line(line) {
            subvolumes.push(subvolume);
        }
    }

    Ok(subvolumes)
}

fn parse_subvolume_line(line: &str) -> Option<Subvolume> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    let id = value_after(&parts, "ID")?.to_string();
    let uuid = value_after(&parts, "uuid").map(str::to_string);
    let parent_uuid = value_after(&parts, "parent_uuid")
        .filter(|value| *value != "-")
        .map(str::to_string);
    let path_index = parts.iter().position(|part| *part == "path")?;
    let path = parts.get(path_index + 1..)?.join(" ");

    Some(Subvolume {
        id,
        parent_uuid,
        uuid,
        path,
    })
}

fn parse_qgroups(bytes: &[u8]) -> Result<Vec<Qgroup>, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read btrfs qgroup output: {error}"))
    })?;
    let mut qgroups = Vec::new();

    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if line.starts_with("qgroupid") || line.starts_with('-') {
            continue;
        }

        let fields: Vec<&str> = line.split_whitespace().collect();
        let Some(id) = fields.first() else {
            continue;
        };
        if !id.contains('/') {
            continue;
        }

        qgroups.push(Qgroup {
            id: (*id).to_string(),
            referenced: fields.get(1).and_then(|value| parse_u64(value)),
            exclusive: fields.get(2).and_then(|value| parse_u64(value)),
            max_referenced: fields.get(3).and_then(|value| nonempty_limit(value)),
            max_exclusive: fields.get(4).and_then(|value| nonempty_limit(value)),
        });
    }

    Ok(qgroups)
}

fn extract_quoted(line: &str, prefix: &str) -> Option<String> {
    let rest = line.strip_prefix(prefix)?.trim();
    let start = rest.find('\'')? + 1;
    let end = rest[start..].find('\'')? + start;
    Some(rest[start..end].to_string())
}

fn extract_after(line: &str, marker: &str) -> Option<String> {
    line.split_once(marker)
        .map(|(_, value)| {
            value
                .trim()
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .to_string()
        })
        .filter(|value| !value.is_empty())
}

fn value_after<'a>(parts: &'a [&str], key: &str) -> Option<&'a str> {
    parts
        .iter()
        .position(|part| *part == key)
        .and_then(|index| parts.get(index + 1).copied())
}

fn parse_u64(value: &str) -> Option<u64> {
    value.trim().parse().ok()
}

fn nonempty_limit(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty() && value != "none" && value != "-").then(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use disk_nix_model::{NodeKind, Relationship};

    use super::*;

    const SHOW: &[u8] = b"Label: 'data'  uuid: fs-uuid\n\
\tTotal devices 2 FS bytes used 4096\n\
\tdevid    1 size 1000 used 400 path /dev/sdb1\n\
\tdevid    2 size 1000 used 300 path /dev/sdc1\n";

    const USAGE: &[u8] = b"Overall:\n\
    Device size:\t\t2000\n\
    Device allocated:\t\t700\n\
    Device unallocated:\t\t1300\n\
    Used:\t\t500\n";

    const SUBVOLUMES: &[u8] = b"ID 256 gen 10 top level 5 uuid subvol-root parent_uuid - path @\n\
ID 257 gen 11 top level 5 uuid snap-1 parent_uuid subvol-root path @/.snapshots/1/snapshot\n";
    const QGROUPS: &[u8] = b"qgroupid         rfer         excl     max_rfer     max_excl\n\
--------         ----         ----     --------     --------\n\
0/256            4096         2048         none         none\n\
0/257            1024         512          8192         none\n";

    #[test]
    fn normalizes_btrfs_filesystem_devices_and_subvolumes() {
        let graph = normalize_btrfs_reports(&[BtrfsReport {
            target: "/data".to_string(),
            show: SHOW.to_vec(),
            usage: USAGE.to_vec(),
            subvolumes: SUBVOLUMES.to_vec(),
            qgroups: QGROUPS.to_vec(),
        }])
        .expect("fixture should parse");

        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::BtrfsFilesystem && node.name == "data")
        );
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::BtrfsSubvolume && node.name == "@")
        );
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::BtrfsSnapshot)
        );
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::BtrfsQgroup
                && node.name == "0/257"
                && node.usage.as_ref().and_then(|usage| usage.used_bytes) == Some(1024)
        }));
        assert!(
            graph
                .edges
                .iter()
                .any(|edge| edge.relationship == Relationship::MemberOf)
        );
    }
}
