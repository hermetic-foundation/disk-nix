use std::collections::BTreeMap;

use disk_nix_model::{Edge, Identity, Node, NodeKind, Relationship, StorageGraph, Usage};

use crate::ProbeError;

pub fn normalize_exfat_metadata(
    device: &str,
    label: Option<&[u8]>,
    guid: Option<&[u8]>,
    serial: Option<&[u8]>,
    dump: Option<&[u8]>,
) -> Result<StorageGraph, ProbeError> {
    let dump_fields = match dump {
        Some(bytes) => parse_dump_exfat(bytes)?,
        None => BTreeMap::new(),
    };
    let serial_value = serial
        .and_then(output_line)
        .or_else(|| dump_fields.get("Volume Serial").cloned());
    let mut graph = StorageGraph::empty();
    let filesystem_id = format!("fs:{device}");
    let mut filesystem = Node::new(filesystem_id.clone(), NodeKind::Filesystem, "exfat")
        .with_path(device.to_string())
        .with_property("filesystem.type", "exfat");

    let identity = Identity {
        uuid: normalized_serial(serial_value.as_deref()),
        partuuid: None,
        label: label
            .and_then(output_line)
            .filter(|value| !value.is_empty()),
        serial: normalized_serial(serial_value.as_deref()),
        wwn: None,
    };
    if !identity.is_empty() {
        filesystem = filesystem.with_identity(identity);
    }

    if let Some(size_bytes) = size_bytes(&dump_fields) {
        filesystem = filesystem.with_size_bytes(size_bytes);
    }

    let usage = usage(&dump_fields);
    if !usage.is_empty() {
        filesystem = filesystem.with_usage(usage);
    }

    if let Some(guid) = guid.and_then(output_line).filter(|value| !value.is_empty()) {
        filesystem = filesystem.with_property("exfat.guid", guid);
    }

    for (key, value) in dump_fields {
        filesystem = filesystem.with_property(format!("exfat.{}", normalize_key(&key)), value);
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

fn parse_dump_exfat(bytes: &[u8]) -> Result<BTreeMap<String, String>, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read dump.exfat output: {error}"))
    })?;
    let mut fields = BTreeMap::new();

    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if key.is_empty() || value.is_empty() || key == "exfatprogs version" {
            continue;
        }
        fields.insert(key.to_string(), value.to_string());
    }

    Ok(fields)
}

fn output_line(bytes: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(bytes).ok()?;
    let value = text.lines().map(str::trim).find(|line| !line.is_empty())?;
    Some(value.to_string())
}

fn normalized_serial(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    let value = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    if value.len() == 8 && value.chars().all(|character| character.is_ascii_hexdigit()) {
        Some(value.to_ascii_uppercase())
    } else {
        None
    }
}

fn size_bytes(fields: &BTreeMap<String, String>) -> Option<u64> {
    let sectors = parse_u64(fields.get("Volume Length(sectors)")?)?;
    let bytes_per_sector = bytes_per_sector(fields)?;
    Some(sectors.saturating_mul(bytes_per_sector))
}

fn usage(fields: &BTreeMap<String, String>) -> Usage {
    let Some(cluster_count) = fields
        .get("Cluster Count")
        .and_then(|value| parse_u64(value))
    else {
        return Usage::empty();
    };
    let Some(free_clusters) = fields
        .get("Free Clusters")
        .and_then(|value| parse_u64(value))
    else {
        return Usage::empty();
    };
    let Some(cluster_size) = cluster_size(fields) else {
        return Usage::empty();
    };

    Usage {
        used_bytes: Some(
            cluster_count
                .saturating_sub(free_clusters)
                .saturating_mul(cluster_size),
        ),
        free_bytes: Some(free_clusters.saturating_mul(cluster_size)),
        allocated_bytes: Some(cluster_count.saturating_mul(cluster_size)),
    }
}

fn cluster_size(fields: &BTreeMap<String, String>) -> Option<u64> {
    let bytes_per_sector = bytes_per_sector(fields)?;
    let sectors_per_cluster = fields
        .get("Sectors per Cluster")
        .and_then(|value| parse_u64(value))
        .or_else(|| {
            fields
                .get("Sector per Cluster bits")
                .and_then(|value| parse_u64(value))
                .and_then(|bits| 1u64.checked_shl(u32::try_from(bits).ok()?))
        })?;
    Some(bytes_per_sector.saturating_mul(sectors_per_cluster))
}

fn bytes_per_sector(fields: &BTreeMap<String, String>) -> Option<u64> {
    fields
        .get("Bytes per Sector")
        .and_then(|value| parse_u64(value))
        .or_else(|| {
            fields
                .get("Sector Size Bits")
                .and_then(|value| parse_u64(value))
                .and_then(|bits| 1u64.checked_shl(u32::try_from(bits).ok()?))
        })
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

    const DUMP_EXFAT: &[u8] = br#"
exfatprogs version : 1.2.4
-------------- Dump Boot sector region --------------
Volume Length(sectors):                 3203072
FAT Offset(sector offset):              2048
FAT Length(sectors):                    448
Cluster Heap Offset (sector offset):    4096
Cluster Count:                          49984
Free Clusters:                          1024
Root Cluster (cluster offset):          4
Volume Serial:                          0x6eef953b
Bytes per Sector:                       512
Sectors per Cluster:                    64
"#;

    #[test]
    fn normalizes_exfat_tune_and_dump_metadata() {
        let graph = normalize_exfat_metadata(
            "/dev/sdb1",
            Some(b"SHARED\n"),
            Some(b"01234567-89ab-cdef-0123-456789abcdef\n"),
            Some(b"0x6eef953b\n"),
            Some(DUMP_EXFAT),
        )
        .expect("fixture should parse");

        let filesystem = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "fs:/dev/sdb1")
            .expect("filesystem exists");

        assert_eq!(filesystem.kind, NodeKind::Filesystem);
        assert_eq!(filesystem.path.as_deref(), Some("/dev/sdb1"));
        assert_eq!(filesystem.identity.label.as_deref(), Some("SHARED"));
        assert_eq!(filesystem.identity.uuid.as_deref(), Some("6EEF953B"));
        assert_eq!(filesystem.identity.serial.as_deref(), Some("6EEF953B"));
        assert_eq!(filesystem.size_bytes, Some(1_639_972_864));
        assert_eq!(
            filesystem
                .usage
                .as_ref()
                .and_then(|usage| usage.allocated_bytes),
            Some(1_637_875_712)
        );
        assert!(filesystem.properties.iter().any(|property| {
            property.key == "exfat.guid" && property.value == "01234567-89ab-cdef-0123-456789abcdef"
        }));
        assert!(filesystem.properties.iter().any(|property| {
            property.key == "exfat.cluster-count" && property.value == "49984"
        }));
        assert!(filesystem.properties.iter().any(|property| {
            property.key == "exfat.volume-serial" && property.value == "0x6eef953b"
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "block:/dev/sdb1"
                && edge.to.0 == "fs:/dev/sdb1"
                && edge.relationship == Relationship::Backs
        }));
    }

    #[test]
    fn normalizes_older_exfat_bit_sized_dump_metadata() {
        let graph = normalize_exfat_metadata(
            "/dev/sdc1",
            None,
            None,
            None,
            Some(
                br#"
Volume Length(sectors):                 262144
Cluster Count:                          32720
Free Clusters:                          32716
Volume Serial:                          0x765cf1c4
Sector Size Bits:                       9
Sector per Cluster bits:                3
"#,
            ),
        )
        .expect("fixture should parse");

        let filesystem = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "fs:/dev/sdc1")
            .expect("filesystem exists");

        assert_eq!(filesystem.identity.uuid.as_deref(), Some("765CF1C4"));
        assert_eq!(filesystem.size_bytes, Some(134_217_728));
        assert_eq!(
            filesystem.usage.as_ref().and_then(|usage| usage.used_bytes),
            Some(16_384)
        );
    }
}
