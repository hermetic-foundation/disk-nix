use disk_nix_model::{Edge, Identity, Node, NodeKind, Relationship, StorageGraph};

use crate::ProbeError;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CryptStatus {
    mapper_path: String,
    active: Option<bool>,
    in_use: Option<bool>,
    backing_device: Option<String>,
    sector_size: Option<u64>,
    sector_count: Option<u64>,
    properties: Vec<(String, String)>,
    uuid: Option<String>,
}

pub fn normalize_cryptsetup_status(
    mapper_path: &str,
    bytes: &[u8],
) -> Result<StorageGraph, ProbeError> {
    let status = parse_status(mapper_path, bytes)?;
    let mut graph = StorageGraph::empty();
    add_status(&mut graph, status);
    Ok(graph)
}

fn parse_status(mapper_path: &str, bytes: &[u8]) -> Result<CryptStatus, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read cryptsetup status: {error}"))
    })?;
    let mut status = CryptStatus {
        mapper_path: mapper_path.to_string(),
        active: None,
        in_use: None,
        backing_device: None,
        sector_size: None,
        sector_count: None,
        properties: Vec::new(),
        uuid: None,
    };

    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if index == 0 {
            parse_header(trimmed, &mut status);
            continue;
        }

        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if value.is_empty() {
            continue;
        }

        match key {
            "device" => status.backing_device = Some(value.to_string()),
            "sector size" => status.sector_size = parse_leading_u64(value),
            "size" => status.sector_count = parse_leading_u64(value),
            "uuid" | "UUID" => status.uuid = Some(value.to_string()),
            _ => status.properties.push((
                format!("cryptsetup.{}", normalize_key(key)),
                value.to_string(),
            )),
        }
    }

    Ok(status)
}

fn parse_header(line: &str, status: &mut CryptStatus) {
    if let Some((path, rest)) = line.split_once(" is ") {
        status.mapper_path = path.to_string();
        status.active = Some(rest.starts_with("active"));
        status.in_use = Some(rest.contains("in use"));
    }
}

fn add_status(graph: &mut StorageGraph, status: CryptStatus) {
    let id = format!("block:{}", status.mapper_path);
    let name = status
        .mapper_path
        .strip_prefix("/dev/mapper/")
        .unwrap_or(&status.mapper_path)
        .to_string();
    let mut node =
        Node::new(id.clone(), NodeKind::LuksContainer, name).with_path(status.mapper_path);

    if let Some(size_bytes) = status
        .sector_count
        .zip(status.sector_size)
        .map(|(sectors, sector_size)| sectors.saturating_mul(sector_size))
    {
        node = node.with_size_bytes(size_bytes);
    }

    if let Some(uuid) = status.uuid {
        node = node.with_identity(Identity {
            uuid: Some(uuid),
            ..Identity::default()
        });
    }

    for (key, value) in [
        (
            "cryptsetup.active",
            status.active.map(|value| value.to_string()),
        ),
        (
            "cryptsetup.in-use",
            status.in_use.map(|value| value.to_string()),
        ),
        (
            "cryptsetup.sector-size",
            status.sector_size.map(|value| value.to_string()),
        ),
        (
            "cryptsetup.sector-count",
            status.sector_count.map(|value| value.to_string()),
        ),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
    }
    for (key, value) in status.properties {
        node = node.with_property(key, value);
    }

    graph.add_node(node);

    if let Some(backing_device) = status.backing_device {
        let backing_id = format!("block:{backing_device}");
        graph.add_node(
            Node::new(
                backing_id.clone(),
                NodeKind::DeviceMapper,
                backing_device.clone(),
            )
            .with_path(backing_device),
        );
        graph.add_edge(Edge::new(backing_id, id, Relationship::Backs));
    }
}

fn parse_leading_u64(value: &str) -> Option<u64> {
    value
        .split_whitespace()
        .next()
        .and_then(|number| number.parse().ok())
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

#[cfg(test)]
mod tests {
    use disk_nix_model::{NodeKind, Relationship};

    use super::*;

    const STATUS: &[u8] = br#"
/dev/mapper/cryptroot is active and is in use.
  type:    LUKS2
  cipher:  aes-xts-plain64
  keysize: 512 bits
  key location: keyring
  device:  /dev/nvme0n1p2
  sector size:  512
  offset:  32768 sectors
  size:    2097152 sectors
  mode:    read/write
"#;

    #[test]
    fn normalizes_cryptsetup_status() {
        let graph =
            normalize_cryptsetup_status("/dev/mapper/cryptroot", STATUS).expect("status parses");
        let container = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::LuksContainer && node.name == "cryptroot")
            .expect("container node should exist");

        assert_eq!(container.path.as_deref(), Some("/dev/mapper/cryptroot"));
        assert_eq!(container.size_bytes, Some(1_073_741_824));
        assert!(container.properties.iter().any(|property| {
            property.key == "cryptsetup.cipher" && property.value == "aes-xts-plain64"
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "block:/dev/nvme0n1p2"
                && edge.to.0 == "block:/dev/mapper/cryptroot"
                && edge.relationship == Relationship::Backs
        }));
    }

    #[test]
    fn normalizes_property_keys() {
        assert_eq!(normalize_key("key location"), "key-location");
        assert_eq!(normalize_key("PBKDF2 Hash"), "pbkdf2-hash");
    }
}
