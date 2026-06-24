use disk_nix_model::{Identity, Node, NodeKind, StorageGraph, Usage};
use serde::Deserialize;
use serde_json::Value;

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
    #[serde(alias = "NSID")]
    namespace_id: Option<u64>,
    #[serde(alias = "NamespaceUUID", alias = "NSUUID")]
    namespace_uuid: Option<String>,
    #[serde(alias = "EUI64")]
    eui64: Option<String>,
    #[serde(alias = "NGUID")]
    nguid: Option<String>,
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
    #[serde(alias = "ANAState")]
    ana_state: Option<String>,
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

pub fn normalize_nvme_id_ns_json(path: &str, bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|error| {
        ProbeError::Adapter(format!(
            "failed to parse nvme id-ns JSON for {path}: {error}"
        ))
    })?;
    let mut node =
        Node::new(format!("block:{path}"), NodeKind::NvmeNamespace, path).with_path(path);

    let formatted_lba = field_u64(&value, "flbas").map(|value| value & 0xf);
    let lba = formatted_lba.and_then(|index| {
        value
            .get("lbafs")
            .and_then(Value::as_array)?
            .get(index as usize)
    });
    let block_size = lba
        .and_then(|lba| field_u64(lba, "ds"))
        .and_then(|shift| 1_u64.checked_shl(shift as u32));

    let namespace_size = field_u64(&value, "nsze");
    let namespace_capacity = field_u64(&value, "ncap");
    let namespace_used = field_u64(&value, "nuse");
    if let (Some(block_size), Some(namespace_size)) = (block_size, namespace_size) {
        if let Some(size_bytes) = namespace_size.checked_mul(block_size) {
            node = node.with_size_bytes(size_bytes);
        }
    }
    let usage = Usage {
        used_bytes: match (block_size, namespace_used) {
            (Some(block_size), Some(namespace_used)) => namespace_used.checked_mul(block_size),
            _ => None,
        },
        free_bytes: match (block_size, namespace_capacity, namespace_used) {
            (Some(block_size), Some(namespace_capacity), Some(namespace_used)) => {
                namespace_capacity
                    .checked_sub(namespace_used)
                    .and_then(|free_blocks| free_blocks.checked_mul(block_size))
            }
            _ => None,
        },
        allocated_bytes: match (block_size, namespace_capacity) {
            (Some(block_size), Some(namespace_capacity)) => {
                namespace_capacity.checked_mul(block_size)
            }
            _ => None,
        },
    };
    if !usage.is_empty() {
        node = node.with_usage(usage);
    }

    if let Some(nguid) = field_string(&value, "nguid") {
        node = node.with_property("nvme.nguid", nguid);
    }
    if let Some(eui64) = field_string(&value, "eui64") {
        node = node.with_property("nvme.eui64", eui64);
    }
    if let Some(index) = formatted_lba {
        node = node.with_property("nvme.formatted-lba-index", index.to_string());
    }
    if let Some(block_size) = block_size {
        node = node.with_property("nvme.formatted-lba-data-size", block_size.to_string());
    }
    if let Some(metadata_size) = lba.and_then(|lba| field_u64(lba, "ms")) {
        node = node.with_property(
            "nvme.formatted-lba-metadata-size",
            metadata_size.to_string(),
        );
    }
    if let Some(relative_performance) = lba.and_then(|lba| field_u64(lba, "rp")) {
        node = node.with_property(
            "nvme.formatted-lba-relative-performance",
            relative_performance.to_string(),
        );
    }

    for key in [
        "nsze", "ncap", "nuse", "nsfeat", "nlbaf", "flbas", "mc", "dpc", "dps", "nmic", "rescap",
        "fpi", "dlfeat", "nawun", "nawupf", "nacwu", "nabsn", "nabo", "nabspf", "noiob", "nvmcap",
    ] {
        if let Some(value) = field_string(&value, key) {
            node = node.with_property(format!("nvme.id-ns.{key}"), value);
        }
    }

    let mut graph = StorageGraph::empty();
    graph.add_node(node);
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
        (
            "nvme.namespace-id",
            device.namespace_id.map(|value| value.to_string()),
        ),
        ("nvme.namespace-uuid", device.namespace_uuid),
        ("nvme.eui64", device.eui64),
        ("nvme.nguid", device.nguid),
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
        ("nvme.ana-state", device.ana_state),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
    }

    graph.add_node(node);
}

fn field_u64(value: &Value, key: &str) -> Option<u64> {
    let value = value.get(key)?;
    if let Some(number) = value.as_u64() {
        return Some(number);
    }
    let text = value.as_str()?.trim();
    if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).ok()
    } else {
        text.parse().ok()
    }
}

fn field_string(value: &Value, key: &str) -> Option<String> {
    let value = value.get(key)?;
    if let Some(text) = value.as_str() {
        if text.is_empty() {
            None
        } else {
            Some(text.to_string())
        }
    } else if value.is_null() {
        None
    } else {
        Some(value.to_string())
    }
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
      "NSID": 1,
      "NamespaceUUID": "12345678-1234-1234-1234-123456789abc",
      "EUI64": "0011223344556677",
      "NGUID": "00112233445566778899aabbccddeeff",
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
      "SectorSize": 512,
      "ANAState": "optimized"
    }
  ]
}
"#;

    const NVME_ID_NS: &[u8] = br#"
{
  "nsze": 1953125,
  "ncap": 1800000,
  "nuse": 900000,
  "nsfeat": 0,
  "nlbaf": 1,
  "flbas": 0,
  "mc": 0,
  "dpc": 0,
  "dps": 0,
  "nmic": 1,
  "rescap": 0,
  "fpi": 128,
  "dlfeat": 9,
  "nawun": 255,
  "nawupf": 255,
  "nacwu": 0,
  "nabsn": 0,
  "nabo": 0,
  "nabspf": 0,
  "noiob": 0,
  "nvmcap": "1000000000",
  "nguid": "00112233445566778899aabbccddeeff",
  "eui64": "0011223344556677",
  "lbafs": [
    { "ms": 0, "ds": 9, "rp": 0 },
    { "ms": 8, "ds": 12, "rp": 1 }
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
        assert!(
            node.properties
                .iter()
                .any(|property| { property.key == "nvme.namespace-id" && property.value == "1" })
        );
        assert!(node.properties.iter().any(|property| {
            property.key == "nvme.namespace-uuid"
                && property.value == "12345678-1234-1234-1234-123456789abc"
        }));
        assert!(
            node.properties.iter().any(
                |property| property.key == "nvme.eui64" && property.value == "0011223344556677"
            )
        );
        assert!(node.properties.iter().any(|property| {
            property.key == "nvme.nguid" && property.value == "00112233445566778899aabbccddeeff"
        }));
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
        assert!(
            node.properties
                .iter()
                .any(|property| property.key == "nvme.ana-state" && property.value == "optimized")
        );
    }

    #[test]
    fn normalizes_nvme_id_ns_json() {
        let graph =
            normalize_nvme_id_ns_json("/dev/nvme0n1", NVME_ID_NS).expect("fixture should parse");

        let node = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::NvmeNamespace)
            .expect("nvme node should exist");

        assert_eq!(node.path.as_deref(), Some("/dev/nvme0n1"));
        assert_eq!(node.size_bytes, Some(1_000_000_000));
        assert_eq!(
            node.usage.as_ref().and_then(|usage| usage.used_bytes),
            Some(460_800_000)
        );
        assert_eq!(
            node.usage.as_ref().and_then(|usage| usage.free_bytes),
            Some(460_800_000)
        );
        assert!(node.properties.iter().any(|property| {
            property.key == "nvme.formatted-lba-index" && property.value == "0"
        }));
        assert!(node.properties.iter().any(|property| {
            property.key == "nvme.formatted-lba-data-size" && property.value == "512"
        }));
        assert!(node.properties.iter().any(|property| {
            property.key == "nvme.formatted-lba-metadata-size" && property.value == "0"
        }));
        assert!(
            node.properties
                .iter()
                .any(|property| property.key == "nvme.id-ns.nsze" && property.value == "1953125")
        );
        assert!(
            node.properties
                .iter()
                .any(|property| property.key == "nvme.id-ns.nvmcap"
                    && property.value == "1000000000")
        );
    }
}
