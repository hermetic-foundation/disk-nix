use std::collections::BTreeMap;

use disk_nix_model::{Edge, Identity, Node, NodeKind, Relationship, StorageGraph};

use crate::ProbeError;

#[derive(Debug, Clone, PartialEq, Eq)]
struct BlkidRecord {
    devname: String,
    fields: BTreeMap<String, String>,
}

pub fn normalize_blkid_export(bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let records = parse_export(bytes)?;
    let mut graph = StorageGraph::empty();

    for record in records {
        add_record(&mut graph, record);
    }

    Ok(graph)
}

fn parse_export(bytes: &[u8]) -> Result<Vec<BlkidRecord>, ProbeError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| ProbeError::Adapter(format!("failed to read blkid output: {error}")))?;
    let mut records = Vec::new();
    let mut fields = BTreeMap::new();

    for line in text.lines().map(str::trim) {
        if line.is_empty() {
            push_record(&mut records, &mut fields);
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if !value.is_empty() {
            fields.insert(key.to_string(), value.to_string());
        }
    }
    push_record(&mut records, &mut fields);

    Ok(records)
}

fn push_record(records: &mut Vec<BlkidRecord>, fields: &mut BTreeMap<String, String>) {
    let Some(devname) = fields.remove("DEVNAME") else {
        fields.clear();
        return;
    };

    records.push(BlkidRecord {
        devname,
        fields: std::mem::take(fields),
    });
}

fn add_record(graph: &mut StorageGraph, record: BlkidRecord) {
    let id = format!("block:{}", record.devname);
    let mut node = Node::new(id.clone(), block_kind(&record), record.devname.clone())
        .with_path(record.devname.clone());

    let identity = Identity {
        uuid: record.fields.get("UUID").cloned(),
        partuuid: record.fields.get("PARTUUID").cloned(),
        label: record.fields.get("LABEL").cloned(),
        serial: None,
        wwn: None,
    };
    if !identity.is_empty() {
        node = node.with_identity(identity);
    }

    for (key, value) in &record.fields {
        node = node.with_property(format!("blkid.{}", normalize_key(key)), value.clone());
    }

    graph.add_node(node);

    if let Some(filesystem_type) = record.fields.get("TYPE") {
        add_filesystem(graph, &id, &record.devname, filesystem_type, &record.fields);
    }
}

fn add_filesystem(
    graph: &mut StorageGraph,
    block_id: &str,
    devname: &str,
    filesystem_type: &str,
    fields: &BTreeMap<String, String>,
) {
    if filesystem_type == "crypto_LUKS" || filesystem_type == "LVM2_member" {
        return;
    }

    let filesystem_id = format!("fs:{devname}");
    let mut filesystem = Node::new(
        filesystem_id.clone(),
        filesystem_kind(filesystem_type),
        filesystem_type.to_string(),
    )
    .with_property("filesystem.type", filesystem_type.to_string());

    let identity = Identity {
        uuid: fields.get("UUID").cloned(),
        partuuid: None,
        label: fields.get("LABEL").cloned(),
        serial: None,
        wwn: None,
    };
    if !identity.is_empty() {
        filesystem = filesystem.with_identity(identity);
    }

    for key in ["VERSION", "BLOCK_SIZE", "USAGE", "UUID_SUB"] {
        if let Some(value) = fields.get(key) {
            filesystem =
                filesystem.with_property(format!("blkid.{}", normalize_key(key)), value.clone());
        }
    }

    graph.add_node(filesystem);
    graph.add_edge(Edge::new(
        block_id.to_string(),
        filesystem_id,
        Relationship::Backs,
    ));
}

fn block_kind(record: &BlkidRecord) -> NodeKind {
    match record.fields.get("TYPE").map(String::as_str) {
        Some("crypto_LUKS") => NodeKind::LuksContainer,
        Some("LVM2_member") => NodeKind::LvmPhysicalVolume,
        Some("linux_raid_member") => NodeKind::MdRaid,
        Some("swap") => NodeKind::Swap,
        _ if record.fields.contains_key("PARTUUID") || record.fields.contains_key("PARTLABEL") => {
            NodeKind::Partition
        }
        _ => NodeKind::DeviceMapper,
    }
}

fn filesystem_kind(filesystem_type: &str) -> NodeKind {
    match filesystem_type {
        "swap" => NodeKind::Swap,
        "btrfs" => NodeKind::BtrfsFilesystem,
        "zfs_member" => NodeKind::ZfsPool,
        _ => NodeKind::Filesystem,
    }
}

fn normalize_key(key: &str) -> String {
    key.to_ascii_lowercase().replace('_', "-")
}

#[cfg(test)]
mod tests {
    use disk_nix_model::{NodeKind, Relationship};

    use super::*;

    const BLKID: &[u8] = br#"
DEVNAME=/dev/nvme0n1p1
UUID=AAAA-BBBB
TYPE=vfat
LABEL=EFI
PARTLABEL=EFI System Partition
PARTUUID=part-uuid-1
BLOCK_SIZE=512
VERSION=FAT32
USAGE=filesystem

DEVNAME=/dev/nvme0n1p2
UUID=luks-uuid
TYPE=crypto_LUKS
PARTUUID=part-uuid-2
USAGE=crypto
"#;

    #[test]
    fn normalizes_blkid_export_records() {
        let graph = normalize_blkid_export(BLKID).expect("fixture should parse");
        let partition = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/nvme0n1p1")
            .expect("partition exists");

        assert_eq!(partition.kind, NodeKind::Partition);
        assert_eq!(partition.identity.uuid.as_deref(), Some("AAAA-BBBB"));
        assert_eq!(partition.identity.partuuid.as_deref(), Some("part-uuid-1"));
        assert!(partition.properties.iter().any(|property| {
            property.key == "blkid.partlabel" && property.value == "EFI System Partition"
        }));
        assert!(graph.nodes.iter().any(|node| {
            node.id.0 == "fs:/dev/nvme0n1p1"
                && node.kind == NodeKind::Filesystem
                && node.identity.label.as_deref() == Some("EFI")
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "block:/dev/nvme0n1p1"
                && edge.to.0 == "fs:/dev/nvme0n1p1"
                && edge.relationship == Relationship::Backs
        }));
    }

    #[test]
    fn treats_luks_signature_as_luks_container_without_filesystem_node() {
        let graph = normalize_blkid_export(BLKID).expect("fixture should parse");

        assert!(graph.nodes.iter().any(|node| {
            node.id.0 == "block:/dev/nvme0n1p2" && node.kind == NodeKind::LuksContainer
        }));
        assert!(
            !graph
                .nodes
                .iter()
                .any(|node| node.id.0 == "fs:/dev/nvme0n1p2")
        );
    }
}
