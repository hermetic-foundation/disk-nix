use disk_nix_model::{Edge, Node, NodeKind, Relationship, StorageGraph, Usage};

use crate::ProbeError;

#[derive(Debug, Clone, PartialEq, Eq)]
struct VdoVolume {
    name: String,
    vdo_device: Option<String>,
    storage_device: Option<String>,
    logical_size: Option<String>,
    physical_size: Option<String>,
    properties: Vec<(String, String)>,
}

pub fn normalize_vdo_status(bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let volumes = parse_vdo_status(bytes)?;
    let mut graph = StorageGraph::empty();

    for volume in volumes {
        add_volume(&mut graph, volume);
    }

    Ok(graph)
}

fn parse_vdo_status(bytes: &[u8]) -> Result<Vec<VdoVolume>, ProbeError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| ProbeError::Adapter(format!("failed to read VDO status: {error}")))?;
    let mut volumes = Vec::new();
    let mut current: Option<VdoVolume> = None;
    let mut in_vdos = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "VDOs:" {
            in_vdos = true;
            continue;
        }
        if !in_vdos {
            continue;
        }

        let indent = line
            .chars()
            .take_while(|character| *character == ' ')
            .count();
        if indent == 2 && trimmed.ends_with(':') {
            if let Some(volume) = current.take() {
                volumes.push(volume);
            }
            current = Some(VdoVolume {
                name: trimmed.trim_end_matches(':').to_string(),
                vdo_device: None,
                storage_device: None,
                logical_size: None,
                physical_size: None,
                properties: Vec::new(),
            });
            continue;
        }

        if indent < 4 {
            continue;
        }

        if let Some((key, value)) = trimmed.split_once(':') {
            let Some(volume) = &mut current else {
                continue;
            };
            let key = key.trim();
            let value = value.trim();
            if value.is_empty() {
                continue;
            }

            match key {
                "VDO device" => volume.vdo_device = Some(value.to_string()),
                "Storage device" => volume.storage_device = Some(value.to_string()),
                "Logical size" => volume.logical_size = Some(value.to_string()),
                "Physical size" => volume.physical_size = Some(value.to_string()),
                _ => volume
                    .properties
                    .push((format!("vdo.{}", normalize_key(key)), value.to_string())),
            }
        }
    }

    if let Some(volume) = current {
        volumes.push(volume);
    }

    Ok(volumes)
}

fn add_volume(graph: &mut StorageGraph, volume: VdoVolume) {
    let id = format!("vdo:{}", volume.name);
    let mut node = Node::new(id.clone(), NodeKind::VdoVolume, volume.name.clone());

    if let Some(path) = volume
        .vdo_device
        .clone()
        .or_else(|| Some(format!("/dev/mapper/{}", volume.name)))
    {
        node = node.with_path(path);
    }
    if let Some(size_bytes) = parse_size(volume.logical_size.as_deref()) {
        node = node.with_size_bytes(size_bytes);
    }

    let usage = Usage {
        used_bytes: parse_size(volume.physical_size.as_deref()),
        free_bytes: None,
        allocated_bytes: None,
    };
    if !usage.is_empty() {
        node = node.with_usage(usage);
    }

    for (key, value) in [
        ("vdo.logical-size", volume.logical_size),
        ("vdo.physical-size", volume.physical_size),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
    }
    for (key, value) in volume.properties {
        node = node.with_property(key, value);
    }

    graph.add_node(node);

    if let Some(storage_device) = volume.storage_device {
        let backing_id = format!("block:{storage_device}");
        graph.add_node(
            Node::new(
                backing_id.clone(),
                NodeKind::PhysicalDisk,
                storage_device.clone(),
            )
            .with_path(storage_device),
        );
        graph.add_edge(Edge::new(backing_id, id, Relationship::Backs));
    }
}

fn normalize_key(key: &str) -> String {
    key.trim()
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

fn parse_size(value: Option<&str>) -> Option<u64> {
    let value = value?.trim();
    if value.is_empty() {
        return None;
    }

    let numeric_end = value
        .char_indices()
        .find_map(|(index, character)| {
            (!character.is_ascii_digit() && character != '.').then_some(index)
        })
        .unwrap_or(value.len());
    let (number, suffix) = value.split_at(numeric_end);
    let number = number.parse::<f64>().ok()?;
    let multiplier = match suffix.trim().to_ascii_lowercase().as_str() {
        "" | "b" => 1.0,
        "k" | "kb" | "kib" => 1024.0,
        "m" | "mb" | "mib" => 1024.0 * 1024.0,
        "g" | "gb" | "gib" => 1024.0 * 1024.0 * 1024.0,
        "t" | "tb" | "tib" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        "p" | "pb" | "pib" => 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };

    Some((number * multiplier) as u64)
}

#[cfg(test)]
mod tests {
    use disk_nix_model::{NodeKind, Relationship};

    use super::*;

    const VDO_STATUS: &[u8] = br#"
VDO status:
  Date: '2026-06-22 10:00:00-05:00'
VDOs:
  archive:
    VDO device: /dev/mapper/archive
    Storage device: /dev/sdb
    Logical size: 1T
    Physical size: 250G
    Compression: enabled
    Deduplication: enabled
    Configured write policy: auto
    Write policy: sync
    Index memory setting: 0.25
    Block map cache size: 128M
"#;

    #[test]
    fn normalizes_vdo_status() {
        let graph = normalize_vdo_status(VDO_STATUS).expect("fixture should parse");
        let volume = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::VdoVolume && node.name == "archive")
            .expect("VDO volume should exist");

        assert_eq!(volume.path.as_deref(), Some("/dev/mapper/archive"));
        assert_eq!(volume.size_bytes, Some(1_099_511_627_776));
        assert!(
            volume
                .properties
                .iter()
                .any(|property| property.key == "vdo.compression" && property.value == "enabled")
        );
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "block:/dev/sdb"
                && edge.to.0 == "vdo:archive"
                && edge.relationship == Relationship::Backs
        }));
    }

    #[test]
    fn parses_size_suffixes() {
        assert_eq!(parse_size(Some("1.5G")), Some(1_610_612_736));
        assert_eq!(parse_size(Some("128 MiB")), Some(134_217_728));
        assert_eq!(parse_size(Some("4096")), Some(4096));
    }
}
