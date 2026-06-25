use disk_nix_model::{Node, NodeKind, StorageGraph, Usage};

use crate::ProbeError;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZramDevice {
    name: String,
    disk_size: Option<u64>,
    data: Option<u64>,
    compressed: Option<u64>,
    algorithm: Option<String>,
    streams: Option<String>,
    zero_pages: Option<String>,
    total: Option<u64>,
    memory_limit: Option<u64>,
    memory_used: Option<u64>,
    migrated: Option<String>,
    compression_ratio: Option<String>,
    mountpoint: Option<String>,
}

pub fn normalize_zramctl_output(bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let devices = parse_zramctl(bytes)?;
    let mut graph = StorageGraph::empty();
    for device in devices {
        add_device(&mut graph, device);
    }
    Ok(graph)
}

fn parse_zramctl(bytes: &[u8]) -> Result<Vec<ZramDevice>, ProbeError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| ProbeError::Adapter(format!("failed to read zramctl output: {error}")))?;
    let mut devices = Vec::new();

    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 12 {
            return Err(ProbeError::Adapter(format!(
                "zramctl row has {} fields, expected at least 12",
                fields.len()
            )));
        }
        devices.push(ZramDevice {
            name: fields[0].to_string(),
            disk_size: parse_u64_field(fields.get(1)),
            data: parse_u64_field(fields.get(2)),
            compressed: parse_u64_field(fields.get(3)),
            algorithm: non_dash(fields.get(4)),
            streams: non_dash(fields.get(5)),
            zero_pages: non_dash(fields.get(6)),
            total: parse_u64_field(fields.get(7)),
            memory_limit: parse_u64_field(fields.get(8)),
            memory_used: parse_u64_field(fields.get(9)),
            migrated: non_dash(fields.get(10)),
            compression_ratio: non_dash(fields.get(11)),
            mountpoint: if fields.len() > 12 {
                non_dash_value(&fields[12..].join(" "))
            } else {
                None
            },
        });
    }

    Ok(devices)
}

fn add_device(graph: &mut StorageGraph, device: ZramDevice) {
    let path = if device.name.starts_with("/dev/") {
        device.name.clone()
    } else {
        format!("/dev/{}", device.name)
    };
    let mut node = Node::new(format!("block:{path}"), NodeKind::ZramDevice, path.clone())
        .with_path(path)
        .with_property("zram.name", device.name);

    if let Some(size) = device.disk_size {
        node = node.with_size_bytes(size);
    }
    let usage = Usage {
        used_bytes: device.data,
        free_bytes: match (device.disk_size, device.data) {
            (Some(disk_size), Some(data)) => disk_size.checked_sub(data),
            _ => None,
        },
        allocated_bytes: device.total,
    };
    if !usage.is_empty() {
        node = node.with_usage(usage);
    }

    for (key, value) in [
        (
            "zram.disksize",
            device.disk_size.map(|value| value.to_string()),
        ),
        ("zram.data", device.data.map(|value| value.to_string())),
        (
            "zram.compressed",
            device.compressed.map(|value| value.to_string()),
        ),
        ("zram.algorithm", device.algorithm),
        ("zram.streams", device.streams),
        ("zram.zero-pages", device.zero_pages),
        ("zram.total", device.total.map(|value| value.to_string())),
        (
            "zram.memory-limit",
            device.memory_limit.map(|value| value.to_string()),
        ),
        (
            "zram.memory-used",
            device.memory_used.map(|value| value.to_string()),
        ),
        (
            "zram.memory-peak",
            device.memory_used.map(|value| value.to_string()),
        ),
        ("zram.migrated", device.migrated),
        ("zram.compression-ratio", device.compression_ratio),
        ("zram.mountpoint", device.mountpoint.clone()),
        (
            "zram.swap",
            device
                .mountpoint
                .as_deref()
                .map(|mountpoint| (mountpoint == "[SWAP]").to_string()),
        ),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
    }

    graph.add_node(node);
}

fn parse_u64_field(field: Option<&&str>) -> Option<u64> {
    field.and_then(|value| value.parse().ok())
}

fn non_dash(field: Option<&&str>) -> Option<String> {
    field.and_then(|value| non_dash_value(value))
}

fn non_dash_value(value: &str) -> Option<String> {
    if value == "-" || value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ZRAMCTL: &[u8] = br#"
/dev/zram0 8589934592 2147483648 715827882 zstd 8 1024 805306368 0 900000000 3 2.67 [SWAP]
"#;

    #[test]
    fn normalizes_zramctl_output() {
        let graph = normalize_zramctl_output(ZRAMCTL).expect("fixture parses");
        let node = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/zram0")
            .expect("zram node exists");

        assert_eq!(node.kind, NodeKind::ZramDevice);
        assert_eq!(node.size_bytes, Some(8_589_934_592));
        assert_eq!(
            node.usage.as_ref().and_then(|usage| usage.used_bytes),
            Some(2_147_483_648)
        );
        assert_eq!(
            node.usage.as_ref().and_then(|usage| usage.free_bytes),
            Some(6_442_450_944)
        );
        assert!(
            node.properties
                .iter()
                .any(|property| property.key == "zram.algorithm" && property.value == "zstd")
        );
        assert!(
            node.properties
                .iter()
                .any(|property| property.key == "zram.swap" && property.value == "true")
        );
        assert!(node.properties.iter().any(|property| {
            property.key == "zram.memory-peak" && property.value == "900000000"
        }));
    }
}
