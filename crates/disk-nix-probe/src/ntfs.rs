use disk_nix_model::{Edge, Identity, Node, NodeKind, Relationship, StorageGraph, Usage};

use crate::ProbeError;

pub fn normalize_ntfsinfo(device: &str, bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let fields = parse_ntfsinfo(bytes)?;
    let mut graph = StorageGraph::empty();
    let filesystem_id = format!("fs:{device}");
    let mut filesystem = Node::new(filesystem_id.clone(), NodeKind::Filesystem, "ntfs")
        .with_path(device.to_string())
        .with_property("filesystem.type", "ntfs");

    let serial = volume_serial(&fields).map(normalize_serial);
    let identity = Identity {
        uuid: serial.clone(),
        partuuid: None,
        label: fields
            .field("Volume Information", "Volume Name")
            .or_else(|| fields.field("Volume Information", "Volume Label"))
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        serial,
        wwn: None,
    };
    if !identity.is_empty() {
        filesystem = filesystem.with_identity(identity);
    }

    if let Some(size_bytes) = size_bytes(&fields) {
        filesystem = filesystem.with_size_bytes(size_bytes).with_usage(Usage {
            used_bytes: None,
            free_bytes: None,
            allocated_bytes: Some(size_bytes),
        });
    }

    for (section, key, value) in &fields.entries {
        filesystem = filesystem.with_property(
            format!("ntfs.{}.{}", normalize_key(section), normalize_key(key)),
            value.clone(),
        );
    }

    for (property, value) in [
        (
            "ntfs.volume-name",
            fields
                .field("Volume Information", "Volume Name")
                .or_else(|| fields.field("Volume Information", "Volume Label")),
        ),
        (
            "ntfs.volume-state",
            fields.field("Volume Information", "Volume State"),
        ),
        ("ntfs.volume-serial", volume_serial(&fields)),
        (
            "ntfs.volume-flags",
            fields.field("Volume Information", "Volume Flags"),
        ),
        (
            "ntfs.version",
            fields.field("Volume Information", "Volume Version"),
        ),
        (
            "ntfs.sector-size",
            fields.field("Volume Information", "Sector Size"),
        ),
        (
            "ntfs.cluster-size",
            fields.field("Volume Information", "Cluster Size"),
        ),
        (
            "ntfs.volume-size-clusters",
            fields.field("Volume Information", "Volume Size in Clusters"),
        ),
        (
            "ntfs.mft-record-size",
            fields.field("MFT Information", "MFT Record Size"),
        ),
        (
            "ntfs.index-block-size",
            fields.field("Volume Information", "Index Block Size"),
        ),
    ] {
        if let Some(value) = value.filter(|value| !value.is_empty()) {
            filesystem = filesystem.with_property(property, value.to_string());
        }
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct NtfsInfo {
    entries: Vec<(String, String, String)>,
}

impl NtfsInfo {
    fn field(&self, section: &str, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(candidate_section, candidate_key, _)| {
                candidate_section == section && candidate_key == key
            })
            .map(|(_, _, value)| value.as_str())
    }
}

fn parse_ntfsinfo(bytes: &[u8]) -> Result<NtfsInfo, ProbeError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| ProbeError::Adapter(format!("failed to read ntfsinfo output: {error}")))?;
    let mut section = String::new();
    let mut entries = Vec::new();

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if !raw_line.starts_with(char::is_whitespace) && !line.contains(':') {
            section = line.to_string();
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if section.is_empty() || key.is_empty() || value.is_empty() {
            continue;
        }
        entries.push((section.clone(), key.to_string(), value.to_string()));
    }

    Ok(NtfsInfo { entries })
}

fn size_bytes(fields: &NtfsInfo) -> Option<u64> {
    let clusters = fields
        .field("Volume Information", "Volume Size in Clusters")
        .and_then(parse_u64)?;
    let cluster_size = fields
        .field("Volume Information", "Cluster Size")
        .and_then(parse_u64)?;
    Some(clusters.saturating_mul(cluster_size))
}

fn volume_serial(fields: &NtfsInfo) -> Option<&str> {
    fields
        .field("Volume Information", "Volume Serial Number")
        .or_else(|| fields.field("MFT Information", "Volume Serial Number"))
}

fn normalize_serial(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("0x")
        .trim_start_matches("0X")
        .chars()
        .filter(|character| character.is_ascii_hexdigit())
        .collect::<String>()
        .to_ascii_uppercase()
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

    const NTFSINFO: &[u8] = br#"
Volume Information
	Name of device: /dev/sda1
	Device state: 11
	Volume Name: Windows
	Volume State: 91
	Volume Flags: 0x0000
	Volume Version: 3.1
	Sector Size: 512
	Cluster Size: 4096
	Index Block Size: 4096
	Volume Size in Clusters: 262144

MFT Information
	MFT Record Size: 1024
	MFT Zone Multiplier: 0
	Volume Serial Number: 01234567-89abcdef
"#;

    #[test]
    fn normalizes_ntfsinfo_metadata() {
        let graph = normalize_ntfsinfo("/dev/sda1", NTFSINFO).expect("fixture should parse");
        let filesystem = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "fs:/dev/sda1")
            .expect("filesystem node exists");

        assert_eq!(filesystem.kind, NodeKind::Filesystem);
        assert_eq!(filesystem.identity.label.as_deref(), Some("Windows"));
        assert_eq!(
            filesystem.identity.serial.as_deref(),
            Some("0123456789ABCDEF")
        );
        assert_eq!(filesystem.size_bytes, Some(1_073_741_824));
        assert_eq!(
            filesystem
                .usage
                .as_ref()
                .and_then(|usage| usage.allocated_bytes),
            Some(1_073_741_824)
        );
        assert!(
            filesystem.properties.iter().any(|property| {
                property.key == "ntfs.volume-name" && property.value == "Windows"
            })
        );
        assert!(
            filesystem.properties.iter().any(|property| {
                property.key == "ntfs.cluster-size" && property.value == "4096"
            })
        );
        assert!(filesystem.properties.iter().any(|property| {
            property.key == "ntfs.mft-record-size" && property.value == "1024"
        }));
        assert!(filesystem.properties.iter().any(|property| {
            property.key == "ntfs.volume-serial" && property.value == "01234567-89abcdef"
        }));
        assert!(filesystem.properties.iter().any(|property| {
            property.key == "ntfs.mft-information.volume-serial-number"
                && property.value == "01234567-89abcdef"
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "block:/dev/sda1"
                && edge.to.0 == "fs:/dev/sda1"
                && edge.relationship == Relationship::Backs
        }));
    }
}
