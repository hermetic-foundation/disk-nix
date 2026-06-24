use disk_nix_model::{Edge, Node, NodeKind, Relationship, StorageGraph};
use serde_json::Value;

use crate::ProbeError;

pub fn normalize_losetup_json(bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let document: Value = serde_json::from_slice(bytes)
        .map_err(|error| ProbeError::Adapter(format!("failed to parse losetup JSON: {error}")))?;
    let mut graph = StorageGraph::empty();
    let Some(devices) = document.get("loopdevices").and_then(Value::as_array) else {
        return Ok(graph);
    };

    for device in devices {
        add_loop_device(&mut graph, device)?;
    }

    Ok(graph)
}

fn add_loop_device(graph: &mut StorageGraph, device: &Value) -> Result<(), ProbeError> {
    let Some(name) = string_field(device, "name") else {
        return Err(ProbeError::Adapter(
            "losetup loop device row is missing name".to_string(),
        ));
    };

    let id = format!("block:{name}");
    let mut node = Node::new(id.clone(), NodeKind::LoopDevice, name.clone()).with_path(name);
    if let Some(size_limit) = number_field(device, "sizelimit").filter(|size| *size > 0) {
        node = node.with_size_bytes(size_limit);
    }

    for (key, value) in loop_properties(device) {
        node = node.with_property(key, value);
    }
    graph.add_node(node);

    if let Some(backing_file) = string_field(device, "back-file") {
        let backing_id = backing_file_id(&backing_file);
        let backing_kind = if backing_file.starts_with("/dev/") {
            NodeKind::PhysicalDisk
        } else {
            NodeKind::BackingFile
        };
        graph.add_node(
            Node::new(backing_id.clone(), backing_kind, backing_file.clone())
                .with_path(backing_file.clone())
                .with_property("loop.backing", "true"),
        );
        graph.add_edge(Edge::new(backing_id, id, Relationship::Backs));
    }

    Ok(())
}

fn loop_properties(device: &Value) -> Vec<(&'static str, String)> {
    let mut properties = Vec::new();
    push_string(device, &mut properties, "back-file", "loop.back-file");
    push_number(device, &mut properties, "back-ino", "loop.backing-inode");
    push_string(
        device,
        &mut properties,
        "back-maj:min",
        "loop.backing-major-minor",
    );
    push_string(device, &mut properties, "maj:min", "loop.major-minor");
    push_number(device, &mut properties, "offset", "loop.offset");
    push_number(device, &mut properties, "sizelimit", "loop.sizelimit");
    push_number(
        device,
        &mut properties,
        "log-sec",
        "loop.logical-sector-size",
    );
    push_bool(device, &mut properties, "autoclear", "loop.autoclear");
    push_bool(device, &mut properties, "partscan", "loop.partscan");
    push_bool(device, &mut properties, "ro", "loop.read-only");
    push_bool(device, &mut properties, "dio", "loop.direct-io");
    properties
}

fn push_string(
    device: &Value,
    properties: &mut Vec<(&'static str, String)>,
    field: &str,
    property: &'static str,
) {
    if let Some(value) = string_field(device, field) {
        properties.push((property, value));
    }
}

fn push_number(
    device: &Value,
    properties: &mut Vec<(&'static str, String)>,
    field: &str,
    property: &'static str,
) {
    if let Some(value) = number_field(device, field) {
        properties.push((property, value.to_string()));
    }
}

fn push_bool(
    device: &Value,
    properties: &mut Vec<(&'static str, String)>,
    field: &str,
    property: &'static str,
) {
    if let Some(value) = device.get(field).and_then(Value::as_bool) {
        properties.push((property, value.to_string()));
    }
}

fn string_field(device: &Value, field: &str) -> Option<String> {
    device
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn number_field(device: &Value, field: &str) -> Option<u64> {
    device.get(field).and_then(Value::as_u64)
}

fn backing_file_id(backing_file: &str) -> String {
    if backing_file.starts_with("/dev/") {
        format!("block:{backing_file}")
    } else {
        format!("file:{backing_file}")
    }
}

#[cfg(test)]
mod tests {
    use disk_nix_model::Relationship;

    use super::*;

    const LOSETUP: &[u8] = br#"{
      "loopdevices": [
        {
          "name": "/dev/loop0",
          "sizelimit": 0,
          "offset": 1048576,
          "autoclear": true,
          "ro": false,
          "back-file": "/var/lib/images/root.img",
          "back-ino": 12345,
          "back-maj:min": "0:45",
          "dio": true,
          "partscan": true,
          "log-sec": 512
        },
        {
          "name": "/dev/loop1",
          "sizelimit": 1073741824,
          "offset": 0,
          "back-file": "/dev/disk/by-id/nvme-loop-backing"
        }
      ]
    }"#;

    #[test]
    fn normalizes_loop_devices_and_backing_files() {
        let graph = normalize_losetup_json(LOSETUP).expect("fixture should parse");
        let loop0 = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/loop0")
            .expect("loop node exists");

        assert_eq!(loop0.kind, NodeKind::LoopDevice);
        assert_eq!(loop0.path.as_deref(), Some("/dev/loop0"));
        assert!(loop0.size_bytes.is_none());
        assert!(loop0.properties.iter().any(|property| {
            property.key == "loop.back-file" && property.value == "/var/lib/images/root.img"
        }));
        assert!(
            loop0.properties.iter().any(|property| {
                property.key == "loop.backing-inode" && property.value == "12345"
            })
        );
        assert!(loop0.properties.iter().any(|property| {
            property.key == "loop.backing-major-minor" && property.value == "0:45"
        }));
        assert!(
            loop0
                .properties
                .iter()
                .any(|property| { property.key == "loop.autoclear" && property.value == "true" })
        );
        assert!(
            loop0
                .properties
                .iter()
                .any(|property| { property.key == "loop.partscan" && property.value == "true" })
        );

        let backing = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "file:/var/lib/images/root.img")
            .expect("backing file node exists");
        assert_eq!(backing.kind, NodeKind::BackingFile);
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "file:/var/lib/images/root.img"
                && edge.to.0 == "block:/dev/loop0"
                && edge.relationship == Relationship::Backs
        }));

        let loop1 = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/loop1")
            .expect("sized loop node exists");
        assert_eq!(loop1.size_bytes, Some(1_073_741_824));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "block:/dev/disk/by-id/nvme-loop-backing"
                && edge.to.0 == "block:/dev/loop1"
                && edge.relationship == Relationship::Backs
        }));
    }
}
