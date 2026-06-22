use disk_nix_model::{Identity, Node, NodeKind, StorageGraph, Usage};
use serde::Deserialize;

use crate::ProbeError;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct NvmeList {
    #[serde(default)]
    devices: Vec<NvmeDevice>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct NvmeDevice {
    device_path: Option<String>,
    model_number: Option<String>,
    serial_number: Option<String>,
    firmware: Option<String>,
    index: Option<u64>,
    physical_size: Option<u64>,
    used_bytes: Option<u64>,
    maximum_lba: Option<u64>,
    sector_size: Option<u64>,
}

pub fn normalize_nvme_list_json(bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let list: NvmeList = serde_json::from_slice(bytes)
        .map_err(|error| ProbeError::Adapter(format!("failed to parse nvme JSON: {error}")))?;
    let mut graph = StorageGraph::empty();

    for device in list.devices {
        add_device(&mut graph, device);
    }

    Ok(graph)
}

fn add_device(graph: &mut StorageGraph, device: NvmeDevice) {
    let Some(path) = device.device_path else {
        return;
    };
    let id = format!("block:{path}");
    let mut node = Node::new(id, NodeKind::NvmeNamespace, path.clone()).with_path(path);

    if let Some(size_bytes) = device.physical_size {
        node = node.with_size_bytes(size_bytes);
    }

    let usage = Usage {
        used_bytes: device.used_bytes,
        free_bytes: match (device.physical_size, device.used_bytes) {
            (Some(size), Some(used)) => size.checked_sub(used),
            _ => None,
        },
        allocated_bytes: device.used_bytes,
    };
    if !usage.is_empty() {
        node = node.with_usage(usage);
    }

    if let Some(serial) = device.serial_number {
        node = node.with_identity(Identity {
            serial: Some(serial),
            ..Identity::default()
        });
    }

    for (key, value) in [
        ("nvme.model", device.model_number),
        ("nvme.firmware", device.firmware),
        ("nvme.index", device.index.map(|value| value.to_string())),
        (
            "nvme.maximum-lba",
            device.maximum_lba.map(|value| value.to_string()),
        ),
        (
            "nvme.sector-size",
            device.sector_size.map(|value| value.to_string()),
        ),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
    }

    graph.add_node(node);
}

#[cfg(test)]
mod tests {
    use disk_nix_model::NodeKind;

    use super::*;

    const NVME_LIST: &[u8] = br#"
{
  "Devices": [
    {
      "DevicePath": "/dev/nvme0n1",
      "ModelNumber": "Example NVMe",
      "SerialNumber": "SERIAL123",
      "Firmware": "1.0",
      "Index": 0,
      "PhysicalSize": 1000,
      "UsedBytes": 400,
      "MaximumLBA": 1953125,
      "SectorSize": 512
    }
  ]
}
"#;

    #[test]
    fn normalizes_nvme_list_json() {
        let graph = normalize_nvme_list_json(NVME_LIST).expect("fixture should parse");

        let node = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::NvmeNamespace)
            .expect("nvme node should exist");

        assert_eq!(node.path.as_deref(), Some("/dev/nvme0n1"));
        assert_eq!(node.identity.serial.as_deref(), Some("SERIAL123"));
        assert!(
            node.properties
                .iter()
                .any(|property| property.key == "nvme.model" && property.value == "Example NVMe")
        );
    }
}
