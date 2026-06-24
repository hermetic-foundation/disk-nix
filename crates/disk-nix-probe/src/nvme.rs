use disk_nix_model::{Edge, Identity, Node, NodeKind, Relationship, StorageGraph, Usage};
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

pub fn normalize_nvme_id_ctrl_json(
    controller: &str,
    bytes: &[u8],
) -> Result<StorageGraph, ProbeError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|error| {
        ProbeError::Adapter(format!(
            "failed to parse nvme id-ctrl JSON for {controller}: {error}"
        ))
    })?;
    let name = controller.trim_start_matches("/dev/");
    let mut node = Node::new(
        nvme_controller_id(name),
        NodeKind::NvmeController,
        name.to_string(),
    )
    .with_path(controller_path(name));

    let serial = field_string(&value, "sn");
    if serial.is_some() {
        node = node.with_identity(Identity {
            serial,
            ..Identity::default()
        });
    }

    for (key, property) in [
        ("mn", "nvme.model"),
        ("fr", "nvme.firmware"),
        ("cntlid", "nvme.controller-id"),
        ("vid", "nvme.id-ctrl.vid"),
        ("ssvid", "nvme.id-ctrl.ssvid"),
        ("rab", "nvme.id-ctrl.rab"),
        ("ieee", "nvme.id-ctrl.ieee"),
        ("cmic", "nvme.id-ctrl.cmic"),
        ("mdts", "nvme.id-ctrl.mdts"),
        ("ver", "nvme.id-ctrl.version"),
        ("rtd3r", "nvme.id-ctrl.rtd3r"),
        ("rtd3e", "nvme.id-ctrl.rtd3e"),
        ("oaes", "nvme.id-ctrl.oaes"),
        ("ctratt", "nvme.id-ctrl.ctratt"),
        ("rrls", "nvme.id-ctrl.rrls"),
        ("cntrltype", "nvme.id-ctrl.controller-type"),
        ("fguid", "nvme.id-ctrl.fguid"),
        ("crdt1", "nvme.id-ctrl.crdt1"),
        ("crdt2", "nvme.id-ctrl.crdt2"),
        ("crdt3", "nvme.id-ctrl.crdt3"),
        ("nvmsr", "nvme.id-ctrl.nvmsr"),
        ("vwci", "nvme.id-ctrl.vwci"),
        ("mec", "nvme.id-ctrl.mec"),
        ("oacs", "nvme.id-ctrl.oacs"),
        ("acl", "nvme.id-ctrl.acl"),
        ("aerl", "nvme.id-ctrl.aerl"),
        ("frmw", "nvme.id-ctrl.frmw"),
        ("lpa", "nvme.id-ctrl.lpa"),
        ("elpe", "nvme.id-ctrl.elpe"),
        ("npss", "nvme.id-ctrl.npss"),
        ("avscc", "nvme.id-ctrl.avscc"),
        ("apsta", "nvme.id-ctrl.apsta"),
        ("wctemp", "nvme.id-ctrl.warning-composite-temp"),
        ("cctemp", "nvme.id-ctrl.critical-composite-temp"),
        ("mtfa", "nvme.id-ctrl.mtfa"),
        ("hmpre", "nvme.id-ctrl.hmpre"),
        ("hmmin", "nvme.id-ctrl.hmmin"),
        ("tnvmcap", "nvme.id-ctrl.total-nvm-capacity"),
        ("unvmcap", "nvme.id-ctrl.unallocated-nvm-capacity"),
        ("rpmbs", "nvme.id-ctrl.rpmbs"),
        ("edstt", "nvme.id-ctrl.edstt"),
        ("dsto", "nvme.id-ctrl.dsto"),
        ("fwug", "nvme.id-ctrl.fwug"),
        ("kas", "nvme.id-ctrl.kas"),
        ("hctma", "nvme.id-ctrl.hctma"),
        ("mntmt", "nvme.id-ctrl.minimum-thermal-management-temp"),
        ("mxtmt", "nvme.id-ctrl.maximum-thermal-management-temp"),
        ("sanicap", "nvme.id-ctrl.sanitize-capabilities"),
        ("hmminds", "nvme.id-ctrl.hmminds"),
        ("hmmaxd", "nvme.id-ctrl.hmmaxd"),
        ("nsetidmax", "nvme.id-ctrl.namespace-set-id-max"),
        ("endgidmax", "nvme.id-ctrl.endurance-group-id-max"),
        ("anatt", "nvme.id-ctrl.ana-transition-time"),
        ("anacap", "nvme.id-ctrl.ana-capabilities"),
        ("anagrpmax", "nvme.id-ctrl.ana-group-max"),
        ("nanagrpid", "nvme.id-ctrl.ana-group-identifiers"),
        ("pels", "nvme.id-ctrl.persistent-event-log-size"),
        ("domainid", "nvme.id-ctrl.domain-id"),
        ("sqes", "nvme.id-ctrl.sqes"),
        ("cqes", "nvme.id-ctrl.cqes"),
        ("maxcmd", "nvme.id-ctrl.maxcmd"),
        ("nn", "nvme.id-ctrl.namespace-count"),
        ("oncs", "nvme.id-ctrl.oncs"),
        ("fuses", "nvme.id-ctrl.fuses"),
        ("fna", "nvme.id-ctrl.fna"),
        ("vwc", "nvme.id-ctrl.volatile-write-cache"),
        ("awun", "nvme.id-ctrl.awun"),
        ("awupf", "nvme.id-ctrl.awupf"),
        ("icsvscc", "nvme.id-ctrl.icsvscc"),
        ("nwpc", "nvme.id-ctrl.nwpc"),
        ("acwu", "nvme.id-ctrl.acwu"),
        ("sgls", "nvme.id-ctrl.sgls"),
        ("mnan", "nvme.id-ctrl.mnan"),
        ("subnqn", "nvme.subsystem"),
    ] {
        if let Some(value) = field_string(&value, key) {
            node = node.with_property(property, value);
        }
    }

    node = node.with_property("nvme.controller", name.to_string());

    let mut graph = StorageGraph::empty();
    graph.add_node(node);
    Ok(graph)
}

fn add_device(graph: &mut StorageGraph, device: NvmeDevice) {
    let Some(path) = device.device_path else {
        return;
    };
    let id = format!("block:{path}");
    let mut node = Node::new(id, NodeKind::NvmeNamespace, path.clone()).with_path(path.clone());

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

    let controller = device.controller.clone();
    let controller_id = device.controller_id;
    let serial = device.serial_number.clone();
    let model = device.model_number.clone();
    let product = device.product_name.clone();
    let firmware = device.firmware.clone();
    let subsystem = device.subsystem.clone();
    let address = device.address.clone();
    let transport = device.transport.clone();

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

    if let Some(controller) = controller {
        add_controller(
            graph,
            &controller,
            ControllerSummary {
                serial,
                model,
                product,
                firmware,
                subsystem,
                address,
                transport,
                controller_id,
            },
        );
        graph.add_edge(Edge::new(
            nvme_controller_id(&controller),
            format!("block:{path}"),
            Relationship::Contains,
        ));
    }
}

struct ControllerSummary {
    serial: Option<String>,
    model: Option<String>,
    product: Option<String>,
    firmware: Option<String>,
    subsystem: Option<String>,
    address: Option<String>,
    transport: Option<String>,
    controller_id: Option<u64>,
}

fn add_controller(graph: &mut StorageGraph, controller: &str, summary: ControllerSummary) {
    let name = controller.trim_start_matches("/dev/");
    let mut node = Node::new(
        nvme_controller_id(name),
        NodeKind::NvmeController,
        name.to_string(),
    )
    .with_path(controller_path(name));

    if summary.serial.is_some() {
        node = node.with_identity(Identity {
            serial: summary.serial,
            ..Identity::default()
        });
    }

    for (key, value) in [
        ("nvme.controller", Some(name.to_string())),
        ("nvme.model", summary.model),
        ("nvme.product", summary.product),
        ("nvme.firmware", summary.firmware),
        ("nvme.subsystem", summary.subsystem),
        ("nvme.address", summary.address),
        ("nvme.transport", summary.transport),
        (
            "nvme.controller-id",
            summary.controller_id.map(|value| value.to_string()),
        ),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
    }

    graph.add_node(node);
}

fn nvme_controller_id(controller: &str) -> String {
    format!("nvme-controller:{}", controller.trim_start_matches("/dev/"))
}

fn controller_path(controller: &str) -> String {
    format!("/dev/{}", controller.trim_start_matches("/dev/"))
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
    use disk_nix_model::{NodeKind, Relationship};

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

    const NVME_ID_CTRL: &[u8] = br#"
{
  "vid": 5197,
  "ssvid": 5197,
  "sn": "SERIAL123",
  "mn": "Example NVMe",
  "fr": "1.0",
  "rab": 6,
  "ieee": 7358820,
  "cmic": 0,
  "mdts": 9,
  "cntlid": 1,
  "ver": 66560,
  "rtd3r": 100000,
  "rtd3e": 500000,
  "oaes": 512,
  "ctratt": 4,
  "rrls": 0,
  "cntrltype": 1,
  "fguid": "12345678-1234-1234-1234-123456789abc",
  "oacs": 23,
  "acl": 3,
  "aerl": 7,
  "frmw": 18,
  "lpa": 6,
  "elpe": 63,
  "npss": 4,
  "wctemp": 343,
  "cctemp": 353,
  "tnvmcap": "1000000000",
  "unvmcap": "500000000",
  "sanicap": 7,
  "anacap": 3,
  "anagrpmax": 32,
  "nanagrpid": 8,
  "sqes": 102,
  "cqes": 68,
  "nn": 16,
  "oncs": 95,
  "vwc": 1,
  "sgls": 1,
  "subnqn": "nqn.2014-08.org.nvmexpress:uuid:12345678"
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

        let controller = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::NvmeController)
            .expect("controller node should exist");
        assert_eq!(controller.name, "nvme0");
        assert_eq!(controller.path.as_deref(), Some("/dev/nvme0"));
        assert_eq!(controller.identity.serial.as_deref(), Some("SERIAL123"));
        assert!(
            controller
                .properties
                .iter()
                .any(|property| property.key == "nvme.controller" && property.value == "nvme0")
        );
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "nvme-controller:nvme0"
                && edge.to.0 == "block:/dev/nvme0n1"
                && edge.relationship == Relationship::Contains
        }));
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

    #[test]
    fn normalizes_nvme_id_ctrl_json() {
        let graph =
            normalize_nvme_id_ctrl_json("/dev/nvme0", NVME_ID_CTRL).expect("fixture should parse");

        let node = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::NvmeController)
            .expect("controller node should exist");

        assert_eq!(node.name, "nvme0");
        assert_eq!(node.path.as_deref(), Some("/dev/nvme0"));
        assert_eq!(node.identity.serial.as_deref(), Some("SERIAL123"));
        assert!(
            node.properties
                .iter()
                .any(|property| property.key == "nvme.model" && property.value == "Example NVMe")
        );
        assert!(
            node.properties
                .iter()
                .any(|property| property.key == "nvme.controller-id" && property.value == "1")
        );
        assert!(node.properties.iter().any(|property| {
            property.key == "nvme.id-ctrl.total-nvm-capacity" && property.value == "1000000000"
        }));
        assert!(node.properties.iter().any(|property| property.key
            == "nvme.id-ctrl.namespace-count"
            && property.value == "16"));
        assert!(node.properties.iter().any(|property| {
            property.key == "nvme.subsystem"
                && property.value == "nqn.2014-08.org.nvmexpress:uuid:12345678"
        }));
    }
}
