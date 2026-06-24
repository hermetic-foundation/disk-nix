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

#[derive(Debug, Clone, PartialEq, Eq)]
struct VdoStats {
    device: String,
    size: Option<String>,
    used: Option<String>,
    available: Option<String>,
    use_percent: Option<String>,
    saving_percent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VdoVerboseStats {
    device: String,
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

pub fn normalize_vdostats_table(bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let stats = parse_vdostats_table(bytes)?;
    let mut graph = StorageGraph::empty();

    for stat in stats {
        add_stats(&mut graph, stat);
    }

    Ok(graph)
}

pub fn normalize_vdostats_verbose(bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let stats = parse_vdostats_verbose(bytes)?;
    let mut graph = StorageGraph::empty();

    for stat in stats {
        add_verbose_stats(&mut graph, stat);
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

fn parse_vdostats_table(bytes: &[u8]) -> Result<Vec<VdoStats>, ProbeError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| ProbeError::Adapter(format!("failed to read vdostats output: {error}")))?;
    let mut rows = Vec::new();

    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if line.starts_with("Device") || line.starts_with('-') {
            continue;
        }

        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 4 {
            continue;
        }

        rows.push(VdoStats {
            device: fields[0].to_string(),
            size: fields.get(1).map(|value| (*value).to_string()),
            used: fields.get(2).map(|value| (*value).to_string()),
            available: fields.get(3).map(|value| (*value).to_string()),
            use_percent: fields.get(4).map(|value| trim_percent(value)),
            saving_percent: fields.get(5).map(|value| trim_percent(value)),
        });
    }

    Ok(rows)
}

fn parse_vdostats_verbose(bytes: &[u8]) -> Result<Vec<VdoVerboseStats>, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read verbose vdostats output: {error}"))
    })?;
    let mut rows = Vec::new();
    let mut current: Option<VdoVerboseStats> = None;

    for line in text.lines() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }

        let indent = line
            .chars()
            .take_while(|character| character.is_ascii_whitespace())
            .count();
        let trimmed = line.trim();

        if indent == 0 && trimmed.ends_with(':') {
            if let Some(row) = current.take() {
                rows.push(row);
            }
            let device = trimmed.trim_end_matches(':').trim().trim_matches('"');
            if !device.is_empty() {
                current = Some(VdoVerboseStats {
                    device: device.to_string(),
                    properties: Vec::new(),
                });
            }
            continue;
        }

        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        let Some(row) = &mut current else {
            continue;
        };
        let key = normalize_key(key);
        let value = value.trim().trim_matches('"').trim_matches('\'');
        if key.is_empty() || value.is_empty() {
            continue;
        }

        row.properties
            .push((format!("vdo.{key}"), value.to_string()));
    }

    if let Some(row) = current {
        rows.push(row);
    }

    Ok(rows)
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
        ("vdo.storage-device", volume.storage_device.clone()),
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

fn add_stats(graph: &mut StorageGraph, stats: VdoStats) {
    let id = vdo_id_from_path(&stats.device);
    let mut node = Node::new(id, NodeKind::VdoVolume, vdo_name_from_path(&stats.device))
        .with_path(stats.device.clone());

    if let Some(size_bytes) = parse_stats_size(stats.size.as_deref()) {
        node = node.with_size_bytes(size_bytes);
    }

    let usage = Usage {
        used_bytes: parse_stats_size(stats.used.as_deref()),
        free_bytes: parse_stats_size(stats.available.as_deref()),
        allocated_bytes: parse_stats_size(stats.size.as_deref()),
    };
    if !usage.is_empty() {
        node = node.with_usage(usage);
    }

    for (key, value) in [
        ("vdo.stats-size", stats.size),
        ("vdo.stats-used", stats.used),
        ("vdo.stats-available", stats.available),
        ("vdo.use-percent", stats.use_percent),
        ("vdo.space-saving-percent", stats.saving_percent),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
    }

    graph.add_node(node);
}

fn add_verbose_stats(graph: &mut StorageGraph, stats: VdoVerboseStats) {
    let mut node = Node::new(
        vdo_id_from_path(&stats.device),
        NodeKind::VdoVolume,
        vdo_name_from_path(&stats.device),
    )
    .with_path(stats.device);

    let mut data_blocks_used = None;
    let mut overhead_blocks_used = None;
    let mut logical_blocks_used = None;
    for (key, value) in stats.properties {
        match key.as_str() {
            "vdo.data-blocks-used" => data_blocks_used = parse_block_count(&value),
            "vdo.overhead-blocks-used" => overhead_blocks_used = parse_block_count(&value),
            "vdo.logical-blocks-used" => logical_blocks_used = parse_block_count(&value),
            _ => {}
        }
        node = node.with_property(key, value);
    }

    if let Some(bytes) = data_blocks_used.and_then(blocks_to_bytes) {
        node = node.with_property("vdo.data-blocks-used-bytes", bytes.to_string());
    }
    if let Some(bytes) = overhead_blocks_used.and_then(blocks_to_bytes) {
        node = node.with_property("vdo.overhead-blocks-used-bytes", bytes.to_string());
    }
    if let Some(bytes) = logical_blocks_used.and_then(blocks_to_bytes) {
        node = node.with_property("vdo.logical-blocks-used-bytes", bytes.to_string());
    }

    let physical_used_bytes = match (
        data_blocks_used.and_then(blocks_to_bytes),
        overhead_blocks_used.and_then(blocks_to_bytes),
    ) {
        (Some(data), Some(overhead)) => data.checked_add(overhead),
        (Some(data), None) => Some(data),
        (None, Some(overhead)) => Some(overhead),
        (None, None) => None,
    };
    let usage = Usage {
        used_bytes: physical_used_bytes,
        free_bytes: None,
        allocated_bytes: logical_blocks_used.and_then(blocks_to_bytes),
    };
    if !usage.is_empty() {
        node = node.with_usage(usage);
    }

    graph.add_node(node);
}

fn vdo_id_from_path(path: &str) -> String {
    format!("vdo:{}", vdo_name_from_path(path))
}

fn vdo_name_from_path(path: &str) -> String {
    path.strip_prefix("/dev/mapper/")
        .or_else(|| path.strip_prefix("/dev/"))
        .unwrap_or(path)
        .to_string()
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

fn parse_stats_size(value: Option<&str>) -> Option<u64> {
    let value = value?;
    if value
        .chars()
        .any(|character| character.is_ascii_alphabetic())
    {
        parse_size(Some(value))
    } else {
        value
            .parse::<u64>()
            .ok()
            .map(|blocks| blocks.saturating_mul(1024))
    }
}

fn parse_block_count(value: &str) -> Option<u64> {
    value
        .split_whitespace()
        .next()
        .map(|value| value.replace(',', ""))
        .and_then(|value| value.parse::<u64>().ok())
}

fn blocks_to_bytes(blocks: u64) -> Option<u64> {
    blocks.checked_mul(4096)
}

fn trim_percent(value: &str) -> String {
    value.trim_end_matches('%').to_string()
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
    const VDOSTATS: &[u8] = br#"
Device                    1K-blocks    Used Available Use% Space saving%
/dev/mapper/archive             1T    250G      750G  25%           60%
/dev/mapper/raw              1048576  262144    786432  25%            0%
"#;
    const VDOSTATS_VERBOSE: &[u8] = br#"
/dev/mapper/archive:
  version: 47
  release version: 133524
  operating mode: normal
  recovery percentage: 100
  write policy: sync
  data blocks used: 65536
  overhead blocks used: 8192
  logical blocks used: 262144
/dev/mapper/recovering:
  operating mode: recovering
  recovery percentage: 42
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
        assert!(volume.properties.iter().any(|property| {
            property.key == "vdo.storage-device" && property.value == "/dev/sdb"
        }));
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

    #[test]
    fn normalizes_vdostats_table() {
        let graph = normalize_vdostats_table(VDOSTATS).expect("fixture should parse");
        let archive = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "vdo:archive")
            .expect("archive stats should exist");

        assert_eq!(archive.size_bytes, Some(1_099_511_627_776));
        assert_eq!(
            archive.usage.as_ref().and_then(|usage| usage.used_bytes),
            Some(268_435_456_000)
        );
        assert!(archive.properties.iter().any(|property| {
            property.key == "vdo.space-saving-percent" && property.value == "60"
        }));

        let raw = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "vdo:raw")
            .expect("raw stats should exist");
        assert_eq!(raw.size_bytes, Some(1_073_741_824));
    }

    #[test]
    fn normalizes_verbose_vdostats_metadata() {
        let graph = normalize_vdostats_verbose(VDOSTATS_VERBOSE).expect("fixture should parse");
        let archive = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "vdo:archive")
            .expect("archive verbose stats should exist");

        assert_eq!(archive.path.as_deref(), Some("/dev/mapper/archive"));
        assert!(archive.properties.iter().any(|property| {
            property.key == "vdo.operating-mode" && property.value == "normal"
        }));
        assert!(
            archive
                .properties
                .iter()
                .any(|property| property.key == "vdo.write-policy" && property.value == "sync")
        );
        assert!(archive.properties.iter().any(|property| {
            property.key == "vdo.overhead-blocks-used" && property.value == "8192"
        }));
        assert!(archive.properties.iter().any(|property| {
            property.key == "vdo.data-blocks-used-bytes" && property.value == "268435456"
        }));
        assert!(archive.properties.iter().any(|property| {
            property.key == "vdo.overhead-blocks-used-bytes" && property.value == "33554432"
        }));
        assert!(archive.properties.iter().any(|property| {
            property.key == "vdo.logical-blocks-used-bytes" && property.value == "1073741824"
        }));
        assert_eq!(
            archive.usage.as_ref().and_then(|usage| usage.used_bytes),
            Some(301_989_888)
        );
        assert_eq!(
            archive
                .usage
                .as_ref()
                .and_then(|usage| usage.allocated_bytes),
            Some(1_073_741_824)
        );

        let recovering = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "vdo:recovering")
            .expect("recovering verbose stats should exist");
        assert!(recovering.properties.iter().any(|property| {
            property.key == "vdo.recovery-percentage" && property.value == "42"
        }));
    }
}
