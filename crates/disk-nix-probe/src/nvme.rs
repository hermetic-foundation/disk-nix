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
    #[serde(alias = "Name")]
    device_path: Option<String>,
    #[serde(alias = "GenericDevice")]
    generic: Option<String>,
    model_number: Option<String>,
    product_name: Option<String>,
    serial_number: Option<String>,
    #[serde(alias = "FirmwareRevision", alias = "FWRev")]
    firmware: Option<String>,
    index: Option<u64>,
    #[serde(alias = "NameSpace", alias = "Namespace")]
    namespace: Option<u64>,
    #[serde(rename = "SubSystem", alias = "Subsystem", alias = "SubsystemNQN")]
    subsystem: Option<String>,
    #[serde(alias = "Controller")]
    controller: Option<String>,
    #[serde(alias = "Address", alias = "TransportAddress")]
    address: Option<String>,
    #[serde(alias = "NamespaceSize")]
    physical_size: Option<u64>,
    #[serde(alias = "NamespaceUsage")]
    used_bytes: Option<u64>,
    #[serde(alias = "NamespaceCapacity")]
    namespace_capacity: Option<u64>,
    maximum_lba: Option<u64>,
    sector_size: Option<u64>,
    #[serde(alias = "Transport", alias = "TrType")]
    transport: Option<String>,
    #[serde(alias = "ControllerID", alias = "CNTLID")]
    controller_id: Option<u64>,
    #[serde(alias = "Format", alias = "LBAFormat")]
    lba_format: Option<String>,
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

    let size_bytes = device.physical_size.or(device.namespace_capacity);
    if let Some(size_bytes) = size_bytes {
        node = node.with_size_bytes(size_bytes);
    }

    let usage = Usage {
        used_bytes: device.used_bytes,
        free_bytes: match (size_bytes, device.used_bytes) {
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
        ("nvme.generic-path", device.generic),
        ("nvme.model", device.model_number),
        ("nvme.product", device.product_name),
        ("nvme.firmware", device.firmware),
        ("nvme.index", device.index.map(|value| value.to_string())),
        (
            "nvme.namespace",
            device.namespace.map(|value| value.to_string()),
        ),
        ("nvme.subsystem", device.subsystem),
        ("nvme.controller", device.controller),
        ("nvme.address", device.address),
        ("nvme.transport", device.transport),
        (
            "nvme.controller-id",
            device.controller_id.map(|value| value.to_string()),
        ),
        (
            "nvme.namespace-capacity",
            device.namespace_capacity.map(|value| value.to_string()),
        ),
        ("nvme.lba-format", device.lba_format),
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
      "Generic": "/dev/ng0n1",
      "ModelNumber": "Example NVMe",
      "ProductName": "Example Controller",
      "SerialNumber": "SERIAL123",
      "Firmware": "1.0",
      "Index": 0,
      "NameSpace": 1,
      "SubSystem": "nvme-subsys0",
      "Controller": "nvme0",
      "Address": "0000:01:00.0",
      "Transport": "pcie",
      "ControllerID": 1,
      "PhysicalSize": 1000,
      "UsedBytes": 400,
      "NamespaceCapacity": 900,
      "Format": "512 B + 0 B",
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
        assert!(node.properties.iter().any(|property| {
            property.key == "nvme.generic-path" && property.value == "/dev/ng0n1"
        }));
        assert!(node.properties.iter().any(|property| {
            property.key == "nvme.product" && property.value == "Example Controller"
        }));
        assert!(
            node.properties
                .iter()
                .any(|property| property.key == "nvme.namespace" && property.value == "1")
        );
        assert!(node.properties.iter().any(|property| {
            property.key == "nvme.subsystem" && property.value == "nvme-subsys0"
        }));
        assert!(
            node.properties
                .iter()
                .any(|property| property.key == "nvme.controller" && property.value == "nvme0")
        );
        assert!(
            node.properties
                .iter()
                .any(|property| property.key == "nvme.address" && property.value == "0000:01:00.0")
        );
        assert!(
            node.properties
                .iter()
                .any(|property| property.key == "nvme.transport" && property.value == "pcie")
        );
        assert!(
            node.properties
                .iter()
                .any(|property| { property.key == "nvme.controller-id" && property.value == "1" })
        );
        assert!(node.properties.iter().any(|property| {
            property.key == "nvme.namespace-capacity" && property.value == "900"
        }));
        assert!(node.properties.iter().any(|property| {
            property.key == "nvme.lba-format" && property.value == "512 B + 0 B"
        }));
    }
}
