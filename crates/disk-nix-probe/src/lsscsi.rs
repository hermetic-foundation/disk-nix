use disk_nix_model::{Edge, Identity, Node, NodeKind, Relationship, StorageGraph};

use crate::ProbeError;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScsiRecord {
    tuple: String,
    peripheral_type: String,
    vendor: Option<String>,
    model: Option<String>,
    revision: Option<String>,
    device: Option<String>,
    generic: Option<String>,
    size: Option<String>,
    transport: Option<String>,
    unit_name: Option<String>,
    scsi_id: Option<String>,
    wwn: Option<String>,
    attributes: Vec<(String, String)>,
}

pub fn normalize_lsscsi_list_output(bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let records = parse_list_records(bytes)?;
    let mut graph = StorageGraph::empty();
    for record in records {
        add_record(&mut graph, record);
    }
    Ok(graph)
}

pub fn normalize_lsscsi_transport_output(bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    normalize_simple_records(bytes, SimpleMode::Transport)
}

pub fn normalize_lsscsi_unit_output(bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    normalize_simple_records(bytes, SimpleMode::Unit)
}

fn normalize_simple_records(bytes: &[u8], mode: SimpleMode) -> Result<StorageGraph, ProbeError> {
    let records = parse_simple_records(bytes, mode)?;
    let mut graph = StorageGraph::empty();
    for record in records {
        add_record(&mut graph, record);
    }
    Ok(graph)
}

fn add_record(graph: &mut StorageGraph, record: ScsiRecord) {
    let lun_id = format!("scsi-lun:{}", record.tuple);
    let mut lun = Node::new(lun_id.clone(), NodeKind::Lun, record.tuple.clone())
        .with_property("scsi.address", record.tuple.clone())
        .with_property("scsi.peripheral-type", record.peripheral_type.clone());

    for (index, label) in ["host", "channel", "target", "lun"].iter().enumerate() {
        if let Some(value) = record.tuple.split(':').nth(index) {
            lun = lun.with_property(format!("scsi.{label}"), value);
        }
    }

    if let Some(value) = &record.vendor {
        lun = lun.with_property("scsi.vendor", value);
    }
    if let Some(value) = &record.model {
        lun = lun.with_property("scsi.model", value);
    }
    if let Some(value) = &record.revision {
        lun = lun.with_property("scsi.revision", value);
    }
    if let Some(value) = &record.device {
        lun = lun.with_property("scsi.block-device", value);
    }
    if let Some(value) = &record.generic {
        lun = lun.with_property("scsi.generic-device", value);
    }
    if let Some(value) = &record.size {
        lun = lun.with_property("scsi.size", value);
    }
    if let Some(value) = &record.transport {
        lun = lun.with_property("scsi.transport", value);
    }
    if let Some(value) = &record.unit_name {
        lun = lun.with_property("scsi.unit-name", value);
    }
    if let Some(value) = &record.scsi_id {
        lun = lun.with_property("scsi.by-id", value);
    }
    if let Some(value) = &record.wwn {
        lun = lun.with_identity(Identity {
            wwn: Some(value.clone()),
            ..Identity::default()
        });
        lun = lun.with_property("scsi.wwn", value);
    }
    for (key, value) in &record.attributes {
        lun = lun.with_property(format!("scsi.{key}"), value);
    }
    graph.add_node(lun);

    if let Some(device) = &record.device {
        if device.starts_with("/dev/") {
            let mut block = Node::new(format!("block:{device}"), NodeKind::PhysicalDisk, device)
                .with_path(device)
                .with_property("scsi.address", record.tuple.clone());
            if let Some(value) = &record.generic {
                block = block.with_property("scsi.generic-device", value);
            }
            if let Some(value) = &record.transport {
                block = block.with_property("scsi.transport", value);
            }
            if let Some(value) = &record.unit_name {
                block = block.with_property("scsi.unit-name", value);
            }
            if let Some(value) = &record.scsi_id {
                block = block.with_property("scsi.by-id", value);
            }
            if let Some(value) = &record.wwn {
                block = block.with_identity(Identity {
                    wwn: Some(value.clone()),
                    ..Identity::default()
                });
                block = block.with_property("scsi.wwn", value);
            }
            graph.add_node(block);
            graph.add_edge(Edge::new(
                lun_id,
                format!("block:{device}"),
                Relationship::Backs,
            ));
        }
    }
}

fn parse_list_records(bytes: &[u8]) -> Result<Vec<ScsiRecord>, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read lsscsi list output: {error}"))
    })?;
    let mut records = Vec::new();
    let mut current: Option<ScsiRecord> = None;

    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if line.starts_with('[') {
            if let Some(record) = current.take() {
                records.push(record);
            }
            current = parse_list_line(line);
        } else if let Some((key, value)) = parse_attribute_line(line) {
            if let Some(record) = &mut current {
                record.attributes.push((normalize_key(&key), value));
            }
        }
    }
    if let Some(record) = current {
        records.push(record);
    }
    Ok(records)
}

fn parse_list_line(line: &str) -> Option<ScsiRecord> {
    let (tuple, rest) = parse_tuple(line)?;
    let tokens = rest.split_whitespace().collect::<Vec<_>>();
    if tokens.len() < 2 {
        return None;
    }
    let peripheral_type = tokens[0].to_string();
    let dev_index = tokens.iter().position(|token| token.starts_with("/dev/"));
    let (vendor, model, revision, device, generic, size) = if let Some(dev_index) = dev_index {
        let device = tokens.get(dev_index).map(|value| (*value).to_string());
        let generic = tokens.get(dev_index + 1).and_then(optional_token);
        let size = tokens.get(dev_index + 2).and_then(optional_token);
        (
            tokens.get(1).and_then(optional_token),
            tokens
                .get(2..dev_index.saturating_sub(1))
                .map(|parts| parts.join(" "))
                .filter(|value| !value.is_empty()),
            tokens
                .get(dev_index.saturating_sub(1))
                .and_then(optional_token),
            device,
            generic,
            size,
        )
    } else {
        (
            tokens.get(1).and_then(optional_token),
            None,
            None,
            None,
            None,
            None,
        )
    };
    Some(ScsiRecord {
        tuple,
        peripheral_type,
        vendor,
        model,
        revision,
        device,
        generic,
        size,
        transport: None,
        unit_name: None,
        scsi_id: None,
        wwn: None,
        attributes: Vec::new(),
    })
}

#[derive(Debug, Clone, Copy)]
enum SimpleMode {
    Transport,
    Unit,
}

fn parse_simple_records(bytes: &[u8], mode: SimpleMode) -> Result<Vec<ScsiRecord>, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read lsscsi simple output: {error}"))
    })?;
    Ok(text
        .lines()
        .filter_map(|line| parse_simple_line(line, mode))
        .collect())
}

fn parse_simple_line(line: &str, mode: SimpleMode) -> Option<ScsiRecord> {
    let (tuple, rest) = parse_tuple(line)?;
    let tokens = rest.split_whitespace().collect::<Vec<_>>();
    if tokens.len() < 2 {
        return None;
    }
    let peripheral_type = tokens[0].to_string();
    let device_index = tokens.iter().position(|token| token.starts_with("/dev/"))?;
    let identity = tokens.get(1..device_index)?.join(" ");
    let device = tokens.get(device_index).map(|value| (*value).to_string());
    let generic = tokens.get(device_index + 1).and_then(optional_token);
    let scsi_id = tokens.get(device_index + 2).and_then(optional_token);
    let wwn = tokens.get(device_index + 3).and_then(optional_token);
    let size = tokens.get(device_index + 4).and_then(optional_token);
    let (transport, unit_name) = match mode {
        SimpleMode::Transport => (Some(identity), None),
        SimpleMode::Unit => (Some(String::new()), Some(identity)),
    };
    Some(ScsiRecord {
        tuple,
        peripheral_type,
        vendor: None,
        model: None,
        revision: None,
        device,
        generic,
        size,
        transport: transport.filter(|value| !value.is_empty()),
        unit_name,
        scsi_id,
        wwn,
        attributes: Vec::new(),
    })
}

fn parse_tuple(line: &str) -> Option<(String, &str)> {
    let end = line.find(']')?;
    let tuple = line.get(1..end)?.to_string();
    Some((tuple, line.get(end + 1..)?.trim()))
}

fn parse_attribute_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    let (key, value) = trimmed.split_once('=')?;
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    Some((key.trim().to_string(), value.to_string()))
}

fn optional_token(token: &&str) -> Option<String> {
    if *token == "-" {
        None
    } else {
        Some((*token).to_string())
    }
}

fn normalize_key(key: &str) -> String {
    key.to_ascii_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &[u8] = br#"
[1:0:0:0]    disk    ATA      Example SSD      1.23  /dev/sdb   /dev/sg1   1.00TB
  device_blocked=0
  queue_depth=32
  queue_type=simple
  scsi_level=6
  state=running
  timeout=30
"#;

    const TRANSPORT: &[u8] = br#"
[1:0:0:0]    disk    sata:5000c500a5a461dc                                           /dev/sdb   /dev/sg1  /dev/disk/by-id/scsi-35000c500a5a461dc  /dev/disk/by-id/wwn-0x5000c500a5a461dc  1.00TB
"#;

    const UNIT: &[u8] = br#"
[1:0:0:0]    disk    5000c500a5a461dc                                                  /dev/sdb   /dev/sg1  /dev/disk/by-id/scsi-35000c500a5a461dc  /dev/disk/by-id/wwn-0x5000c500a5a461dc  1.00TB
"#;

    #[test]
    fn normalizes_lsscsi_list_output() {
        let graph = normalize_lsscsi_list_output(LIST).expect("fixture parses");
        let lun = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "scsi-lun:1:0:0:0")
            .expect("lun exists");
        assert_eq!(lun.kind, NodeKind::Lun);
        assert!(
            lun.properties
                .iter()
                .any(|property| property.key == "scsi.queue-depth" && property.value == "32")
        );
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "scsi-lun:1:0:0:0"
                && edge.to.0 == "block:/dev/sdb"
                && edge.relationship == Relationship::Backs
        }));
    }

    #[test]
    fn normalizes_lsscsi_transport_and_unit_output() {
        let transport = normalize_lsscsi_transport_output(TRANSPORT).expect("fixture parses");
        let unit = normalize_lsscsi_unit_output(UNIT).expect("fixture parses");
        let transport_lun = transport
            .nodes
            .iter()
            .find(|node| node.id.0 == "scsi-lun:1:0:0:0")
            .expect("transport lun exists");
        assert!(transport_lun.properties.iter().any(|property| {
            property.key == "scsi.transport" && property.value == "sata:5000c500a5a461dc"
        }));
        assert_eq!(
            transport_lun.identity.wwn.as_deref(),
            Some("/dev/disk/by-id/wwn-0x5000c500a5a461dc")
        );

        let unit_lun = unit
            .nodes
            .iter()
            .find(|node| node.id.0 == "scsi-lun:1:0:0:0")
            .expect("unit lun exists");
        assert!(unit_lun.properties.iter().any(|property| {
            property.key == "scsi.unit-name" && property.value == "5000c500a5a461dc"
        }));
    }
}
