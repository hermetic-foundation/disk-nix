use std::collections::BTreeMap;

use disk_nix_model::{Edge, Identity, Node, NodeKind, Relationship, StorageGraph, Usage};

use crate::ProbeError;

pub fn normalize_dump_f2fs(device: &str, bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let fields = parse_dump_f2fs(bytes)?;
    let mut graph = StorageGraph::empty();
    let filesystem_id = format!("fs:{device}");
    let mut filesystem = Node::new(filesystem_id.clone(), NodeKind::Filesystem, "f2fs")
        .with_path(device.to_string())
        .with_property("filesystem.type", "f2fs");

    let identity = Identity {
        uuid: fields.get("filesystem-uuid").cloned(),
        partuuid: None,
        label: fields
            .get("filesystem-volume-name")
            .filter(|value| !value.is_empty() && *value != "<none>")
            .cloned(),
        serial: None,
        wwn: None,
    };
    if !identity.is_empty() {
        filesystem = filesystem.with_identity(identity);
    }

    if let Some(size_bytes) = size_bytes(&fields) {
        filesystem = filesystem.with_size_bytes(size_bytes);
    }

    let usage = usage(&fields);
    if !usage.is_empty() {
        filesystem = filesystem.with_usage(usage);
    }

    for (key, value) in &fields {
        filesystem = filesystem.with_property(format!("f2fs.{key}"), value.clone());
    }

    graph.add_node(filesystem);
    graph.add_node(
        Node::new(
            format!("block:{device}"),
            NodeKind::DeviceMapper,
            device.to_string(),
        )
        .with_path(device.to_string()),
    );
    graph.add_edge(Edge::new(
        format!("block:{device}"),
        filesystem_id,
        Relationship::Backs,
    ));

    Ok(graph)
}

fn parse_dump_f2fs(bytes: &[u8]) -> Result<BTreeMap<String, String>, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read dump.f2fs output: {error}"))
    })?;
    let mut fields = BTreeMap::new();

    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let Some((key, value)) = split_field(line) else {
            continue;
        };
        let key = normalize_key(key);
        let value = value.trim();
        if key.is_empty() || value.is_empty() {
            continue;
        }
        fields.insert(key, value.to_string());
    }

    Ok(fields)
}

fn split_field(line: &str) -> Option<(&str, &str)> {
    line.split_once('=')
        .or_else(|| line.split_once(':'))
        .map(|(key, value)| (key.trim(), value.trim()))
}

fn size_bytes(fields: &BTreeMap<String, String>) -> Option<u64> {
    let blocks = fields
        .get("block-count")
        .or_else(|| fields.get("total-valid-block-count"))
        .and_then(|value| parse_u64(value))?;
    Some(blocks.saturating_mul(block_size(fields)))
}

fn usage(fields: &BTreeMap<String, String>) -> Usage {
    let block_size = block_size(fields);
    let used = fields
        .get("valid-block-count")
        .or_else(|| fields.get("valid-user-block-count"))
        .and_then(|value| parse_u64(value))
        .map(|blocks| blocks.saturating_mul(block_size));
    let allocated = fields
        .get("user-block-count")
        .or_else(|| fields.get("block-count"))
        .and_then(|value| parse_u64(value))
        .map(|blocks| blocks.saturating_mul(block_size));
    let free = match (allocated, used) {
        (Some(allocated), Some(used)) => Some(allocated.saturating_sub(used)),
        _ => None,
    };

    Usage {
        used_bytes: used,
        free_bytes: free,
        allocated_bytes: allocated,
    }
}

fn block_size(fields: &BTreeMap<String, String>) -> u64 {
    fields
        .get("block-size")
        .and_then(|value| parse_u64(value))
        .unwrap_or(4096)
}

fn parse_u64(value: &str) -> Option<u64> {
    value
        .split_whitespace()
        .next()
        .and_then(|number| number.parse().ok())
}

fn normalize_key(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|character| match character {
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
    use disk_nix_model::Relationship;

    use super::*;

    const DUMP_F2FS: &[u8] = br#"
Info: Debug level = 0
Info: Label = mobile
Filesystem volume name: mobile
Filesystem UUID: 01234567-89ab-cdef-0123-456789abcdef
block_size = 4096
block_count = 262144
user_block_count = 245760
valid_block_count = 65536
segment_count = 2048
segment_count_main = 1984
overprov_segment_count = 64
segs_per_sec = 1
secs_per_zone = 1
"#;

    #[test]
    fn normalizes_dump_f2fs_metadata() {
        let graph = normalize_dump_f2fs("/dev/sdb2", DUMP_F2FS).expect("fixture should parse");
        let filesystem = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "fs:/dev/sdb2")
            .expect("filesystem node exists");

        assert_eq!(filesystem.kind, NodeKind::Filesystem);
        assert_eq!(filesystem.identity.label.as_deref(), Some("mobile"));
        assert_eq!(
            filesystem.identity.uuid.as_deref(),
            Some("01234567-89ab-cdef-0123-456789abcdef")
        );
        assert_eq!(filesystem.size_bytes, Some(1_073_741_824));
        assert_eq!(
            filesystem.usage.as_ref().and_then(|usage| usage.used_bytes),
            Some(268_435_456)
        );
        assert_eq!(
            filesystem.usage.as_ref().and_then(|usage| usage.free_bytes),
            Some(738_197_504)
        );
        assert!(filesystem
            .properties
            .iter()
            .any(|property| { property.key == "f2fs.segment-count" && property.value == "2048" }));
        assert!(filesystem.properties.iter().any(|property| {
            property.key == "f2fs.overprov-segment-count" && property.value == "64"
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "block:/dev/sdb2"
                && edge.to.0 == "fs:/dev/sdb2"
                && edge.relationship == Relationship::Backs
        }));
    }
}
