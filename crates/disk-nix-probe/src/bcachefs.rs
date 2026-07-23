use std::collections::BTreeMap;

use disk_nix_model::{Edge, Identity, Node, NodeKind, Relationship, StorageGraph, Usage};

use crate::ProbeError;

pub fn normalize_show_super(device: &str, bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let fields = parse_key_values(bytes, "bcachefs show-super")?;
    let mut graph = StorageGraph::empty();
    let filesystem_id = fields
        .get("external-uuid")
        .map(|uuid| format!("bcachefs:{uuid}"))
        .unwrap_or_else(|| format!("fs:{device}"));
    let label = fields
        .get("label")
        .filter(|value| !value.is_empty() && *value != "(none)")
        .cloned();
    let mut filesystem = Node::new(
        filesystem_id.clone(),
        NodeKind::BcachefsFilesystem,
        label.clone().unwrap_or_else(|| "bcachefs".to_string()),
    )
    .with_path(device)
    .with_property("filesystem.type", "bcachefs")
    .with_property("bcachefs.member-device", device);

    let identity = Identity {
        uuid: fields.get("external-uuid").cloned(),
        partuuid: None,
        label,
        serial: None,
        wwn: None,
    };
    if !identity.is_empty() {
        filesystem = filesystem.with_identity(identity);
    }

    for (key, value) in &fields {
        filesystem = filesystem.with_property(format!("bcachefs.{key}"), value.clone());
    }

    graph.add_node(filesystem);
    graph.add_node(
        Node::new(
            format!("block:{device}"),
            NodeKind::BcachefsDevice,
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

pub fn normalize_fs_usage(target: &str, bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let usage = parse_fs_usage(bytes)?;
    let mut graph = StorageGraph::empty();
    let filesystem_id = usage
        .filesystem_uuid
        .as_ref()
        .map(|uuid| format!("bcachefs:{uuid}"))
        .unwrap_or_else(|| format!("mount:{target}"));
    let mut filesystem = Node::new(filesystem_id.clone(), NodeKind::BcachefsFilesystem, target)
        .with_property("filesystem.type", "bcachefs")
        .with_property("bcachefs.mount-target", target);

    let identity = Identity {
        uuid: usage.filesystem_uuid.clone(),
        partuuid: None,
        label: None,
        serial: None,
        wwn: None,
    };
    if !identity.is_empty() {
        filesystem = filesystem.with_identity(identity);
    }

    if let Some(size_bytes) = usage.size_bytes {
        filesystem = filesystem.with_size_bytes(size_bytes);
    }

    let fs_usage = Usage {
        used_bytes: usage.used_bytes,
        free_bytes: match (usage.size_bytes, usage.used_bytes) {
            (Some(size), Some(used)) => Some(size.saturating_sub(used)),
            _ => None,
        },
        allocated_bytes: usage.size_bytes,
    };
    if !fs_usage.is_empty() {
        filesystem = filesystem.with_usage(fs_usage);
    }

    if let Some(size) = usage.size_bytes {
        filesystem = filesystem.with_property("bcachefs.size", size.to_string());
    }
    if let Some(used) = usage.used_bytes {
        filesystem = filesystem.with_property("bcachefs.used", used.to_string());
    }
    if let Some(reserved) = usage.online_reserved_bytes {
        filesystem = filesystem.with_property("bcachefs.online-reserved", reserved.to_string());
    }
    for (data_type, bytes) in &usage.data_type_bytes {
        filesystem =
            filesystem.with_property(format!("bcachefs.data-{data_type}"), bytes.to_string());
    }
    if !usage.devices.is_empty() {
        filesystem =
            filesystem.with_property("bcachefs.device-count", usage.devices.len().to_string());
    }

    graph.add_node(filesystem);
    graph.add_node(Node::new(
        format!("mount:{target}"),
        NodeKind::Mountpoint,
        target.to_string(),
    ));
    graph.add_edge(Edge::new(
        filesystem_id.clone(),
        format!("mount:{target}"),
        Relationship::MountedAt,
    ));

    for device in usage.devices {
        let device_id = format!(
            "bcachefs-device:{}:{}",
            usage.filesystem_uuid.as_deref().unwrap_or(target),
            device.index
        );
        let mut node = Node::new(
            device_id.clone(),
            NodeKind::BcachefsDevice,
            device.name.clone(),
        )
        .with_property("filesystem.type", "bcachefs")
        .with_property("bcachefs.device-index", device.index.to_string())
        .with_property("bcachefs.device-label", device.label.clone())
        .with_property("bcachefs.device-state", device.state.clone());

        if let Some(capacity) = device.capacity_bytes {
            node = node.with_size_bytes(capacity);
        }
        let device_usage = Usage {
            used_bytes: match (device.capacity_bytes, device.free_bytes) {
                (Some(capacity), Some(free)) => Some(capacity.saturating_sub(free)),
                _ => None,
            },
            free_bytes: device.free_bytes,
            allocated_bytes: device.capacity_bytes,
        };
        if !device_usage.is_empty() {
            node = node.with_usage(device_usage);
        }
        if let Some(free) = device.free_bytes {
            node = node.with_property("bcachefs.device-free", free.to_string());
        }
        if let Some(capacity) = device.capacity_bytes {
            node = node.with_property("bcachefs.device-capacity", capacity.to_string());
        }
        for (data_type, bytes) in device.data_type_bytes {
            node = node.with_property(
                format!("bcachefs.device-data-{data_type}"),
                bytes.to_string(),
            );
        }

        graph.add_node(node);
        graph.add_edge(Edge::new(
            device_id,
            filesystem_id.clone(),
            Relationship::Backs,
        ));
    }

    Ok(graph)
}

#[derive(Debug, Default)]
struct BcachefsUsage {
    filesystem_uuid: Option<String>,
    size_bytes: Option<u64>,
    used_bytes: Option<u64>,
    online_reserved_bytes: Option<u64>,
    data_type_bytes: BTreeMap<String, u64>,
    devices: Vec<BcachefsDeviceUsage>,
}

#[derive(Debug, Default)]
struct BcachefsDeviceUsage {
    label: String,
    index: u64,
    name: String,
    state: String,
    free_bytes: Option<u64>,
    capacity_bytes: Option<u64>,
    data_type_bytes: BTreeMap<String, u64>,
}

fn parse_key_values(bytes: &[u8], command: &str) -> Result<BTreeMap<String, String>, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read {command} output: {error}"))
    })?;
    let mut fields = BTreeMap::new();

    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let Some((key, value)) = line.split_once(':') else {
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

fn parse_fs_usage(bytes: &[u8]) -> Result<BcachefsUsage, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read bcachefs fs usage output: {error}"))
    })?;
    let mut usage = BcachefsUsage::default();
    let mut current_device: Option<BcachefsDeviceUsage> = None;
    let mut in_data_type_table = false;

    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if let Some(device) = parse_device_header(line) {
            if let Some(device) = current_device.take() {
                usage.devices.push(device);
            }
            current_device = Some(device);
            in_data_type_table = false;
            continue;
        }

        if line.starts_with("Data type") {
            in_data_type_table = true;
            continue;
        }
        if line.starts_with("Device label") {
            in_data_type_table = false;
            continue;
        }

        if let Some(device) = current_device.as_mut() {
            if parse_device_usage_line(line, device) {
                continue;
            }
        }

        if in_data_type_table {
            parse_data_type_line(line, &mut usage.data_type_bytes);
            continue;
        }

        if let Some((key, value)) = line.split_once(':') {
            match normalize_key(key).as_str() {
                "filesystem" => usage.filesystem_uuid = Some(value.trim().to_string()),
                "size" => usage.size_bytes = parse_size(value),
                "used" => usage.used_bytes = parse_size(value),
                "online-reserved" => usage.online_reserved_bytes = parse_size(value),
                _ => {}
            }
        }
    }

    if let Some(device) = current_device {
        usage.devices.push(device);
    }

    Ok(usage)
}

fn parse_device_header(line: &str) -> Option<BcachefsDeviceUsage> {
    let (label, rest) = line.split_once("(device ")?;
    let (index, rest) = rest.split_once("):")?;
    let mut fields = rest.split_whitespace();
    let name = fields.next()?.to_string();
    let state = fields.next().unwrap_or("").to_string();

    Some(BcachefsDeviceUsage {
        label: label.trim().to_string(),
        index: index.trim().parse().ok()?,
        name,
        state,
        ..BcachefsDeviceUsage::default()
    })
}

fn parse_device_usage_line(line: &str, device: &mut BcachefsDeviceUsage) -> bool {
    let Some((key, value)) = line.split_once(':') else {
        return false;
    };
    let key = normalize_key(key);
    let Some(bytes) = parse_size(value) else {
        return false;
    };

    match key.as_str() {
        "free" => device.free_bytes = Some(bytes),
        "capacity" => device.capacity_bytes = Some(bytes),
        _ => {
            device
                .data_type_bytes
                .entry(key)
                .and_modify(|total| *total = total.saturating_add(bytes))
                .or_insert(bytes);
        }
    }
    true
}

fn parse_data_type_line(line: &str, data_type_bytes: &mut BTreeMap<String, u64>) {
    let mut tokens = line.split_whitespace().collect::<Vec<_>>();
    if tokens.len() < 2 {
        return;
    }
    let data_type = normalize_key(tokens.remove(0).trim_end_matches(':'));
    if data_type.is_empty() {
        return;
    }

    let Some(bytes) = tokens.iter().rev().find_map(|token| parse_size(token)) else {
        return;
    };
    data_type_bytes
        .entry(data_type)
        .and_modify(|total| *total = total.saturating_add(bytes))
        .or_insert(bytes);
}

fn parse_size(value: &str) -> Option<u64> {
    let token = value.split_whitespace().next()?.trim_end_matches(',');
    let numeric_len = token
        .char_indices()
        .take_while(|(_, character)| character.is_ascii_digit() || *character == '.')
        .map(|(index, character)| index + character.len_utf8())
        .last()?;
    let (number, suffix) = token.split_at(numeric_len);
    let value = number.parse::<f64>().ok()?;
    let multiplier = match suffix.to_ascii_lowercase().as_str() {
        "" => 1.0,
        "k" | "kb" => 1_000.0,
        "m" | "mb" => 1_000_000.0,
        "g" | "gb" => 1_000_000_000.0,
        "t" | "tb" => 1_000_000_000_000.0,
        "p" | "pb" => 1_000_000_000_000_000.0,
        "ki" | "kib" => 1024.0,
        "mi" | "mib" => 1024.0 * 1024.0,
        "gi" | "gib" => 1024.0 * 1024.0 * 1024.0,
        "ti" | "tib" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        "pi" | "pib" => 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };

    Some((value * multiplier) as u64)
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

    const SHOW_SUPER: &[u8] = br#"
Device:                                     ST12000NM001G-2M
External UUID:                              a2d6fc04-efd0-4e36-aece-2475941d09a3
Internal UUID:                              55083d1e-27cf-4929-ada4-3fe6e45cf02c
Magic number:                               c68573f6-66ce-90a9-d96a-60cf803df7ef
Device index:                               6
Label:                                      archive
Version:                                    1.20: (unknown version)
Version upgrade complete:                   1.20: (unknown version)
"#;

    const FS_USAGE: &[u8] = br#"
Filesystem: a2d6fc04-efd0-4e36-aece-2475941d09a3
Size:                 54282477161984
Used:                 47381969152512
Online reserved:           507957248

Data type       Required/total  Durability    Devices
btree:          1/2             2             [sda sdb]        1048576
user:           1/2             2             [sda sdb]     2147483648
cached:         1/1             1             [sdb]          536870912

hdd.archive (device 6):             sdc              rw
                                data         buckets    fragmented
free:                1649975230464         3147078
sb:                        3149824               7        520192
journal:                4294967296            8192
btree:                   890241024            1698
user:                            0               0
cached:                          0               0
capacity:            16000900661248        30519296
"#;

    #[test]
    fn normalizes_bcachefs_show_super_metadata() {
        let graph = normalize_show_super("/dev/sdc", SHOW_SUPER).expect("fixture should parse");
        let filesystem = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3")
            .expect("filesystem node exists");

        assert_eq!(filesystem.kind, NodeKind::BcachefsFilesystem);
        assert_eq!(
            filesystem.identity.uuid.as_deref(),
            Some("a2d6fc04-efd0-4e36-aece-2475941d09a3")
        );
        assert_eq!(filesystem.identity.label.as_deref(), Some("archive"));
        assert!(filesystem.properties.iter().any(|property| {
            property.key == "bcachefs.internal-uuid"
                && property.value == "55083d1e-27cf-4929-ada4-3fe6e45cf02c"
        }));
        assert!(filesystem
            .properties
            .iter()
            .any(|property| { property.key == "bcachefs.device-index" && property.value == "6" }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "block:/dev/sdc"
                && edge.to.0 == "bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3"
                && edge.relationship == Relationship::Backs
        }));
    }

    #[test]
    fn normalizes_bcachefs_usage_and_devices() {
        let graph = normalize_fs_usage("/mnt/archive", FS_USAGE).expect("fixture should parse");
        let filesystem = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3")
            .expect("filesystem node exists");

        assert_eq!(filesystem.size_bytes, Some(54_282_477_161_984));
        assert_eq!(
            filesystem.usage.as_ref().and_then(|usage| usage.used_bytes),
            Some(47_381_969_152_512)
        );
        assert_eq!(
            filesystem.usage.as_ref().and_then(|usage| usage.free_bytes),
            Some(6_900_508_009_472)
        );
        assert!(filesystem.properties.iter().any(|property| {
            property.key == "bcachefs.online-reserved" && property.value == "507957248"
        }));
        assert!(filesystem.properties.iter().any(|property| {
            property.key == "bcachefs.data-user" && property.value == "2147483648"
        }));

        let device = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "bcachefs-device:a2d6fc04-efd0-4e36-aece-2475941d09a3:6")
            .expect("device node exists");
        assert_eq!(device.kind, NodeKind::BcachefsDevice);
        assert_eq!(device.name, "sdc");
        assert_eq!(device.size_bytes, Some(16_000_900_661_248));
        assert_eq!(
            device.usage.as_ref().and_then(|usage| usage.free_bytes),
            Some(1_649_975_230_464)
        );
        assert!(device.properties.iter().any(|property| {
            property.key == "bcachefs.device-data-btree" && property.value == "890241024"
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3"
                && edge.to.0 == "mount:/mnt/archive"
                && edge.relationship == Relationship::MountedAt
        }));
    }
}
