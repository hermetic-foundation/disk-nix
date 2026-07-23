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

    const NVME_SUBSYSTEMS: &[u8] = br#"
{
  "Subsystems": [
    {
      "Name": "nvme-subsys0",
      "NQN": "nqn.2014-08.org.nvmexpress:uuid:12345678",
      "HostNQN": "nqn.2014-08.org.nvmexpress:host:disk-nix",
      "Paths": [
        {
          "Name": "nvme0",
          "Transport": "pcie",
          "Address": "0000:01:00.0",
          "State": "live",
          "ANAState": "optimized",
          "Namespaces": [
            { "Name": "/dev/nvme0n1", "NSID": 1 }
          ]
        },
        {
          "Name": "nvme1",
          "Transport": "tcp",
          "TRADDR": "192.0.2.10",
          "TRSVCID": "4420",
          "State": "connecting",
          "ANAState": "inaccessible"
        }
      ]
    }
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

    const NVME_SMART_LOG: &[u8] = br#"
{
  "critical_warning": 0,
  "temperature": 301,
  "avail_spare": 100,
  "spare_thresh": 10,
  "percent_used": 2,
  "data_units_read": 123456,
  "data_units_written": 654321,
  "host_read_commands": 1000000,
  "host_write_commands": 2000000,
  "controller_busy_time": 17,
  "power_cycles": 42,
  "power_on_hours": 1200,
  "unsafe_shutdowns": 3,
  "media_errors": 0,
  "num_err_log_entries": 4,
  "warning_temp_time": 0,
  "critical_comp_time": 0,
  "temperature_sensor_1": 300,
  "temperature_sensor_2": 302,
  "thm_temp1_trans_count": 1,
  "thm_temp2_trans_count": 0,
  "thm_temp1_total_time": 8,
  "thm_temp2_total_time": 0
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
        assert_eq!(
            node.identity.uuid.as_deref(),
            Some("12345678-1234-1234-1234-123456789abc")
        );
        assert_eq!(
            node.identity.wwn.as_deref(),
            Some("00112233445566778899aabbccddeeff")
        );
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
    fn normalizes_nvme_subsystem_topology_json() {
        let graph = normalize_nvme_subsystems_json(NVME_SUBSYSTEMS).expect("fixture should parse");

        let subsystem = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::NvmeSubsystem)
            .expect("subsystem node should exist");
        assert_eq!(subsystem.name, "nvme-subsys0");
        assert!(subsystem.properties.iter().any(|property| {
            property.key == "nvme.subsystem-nqn"
                && property.value == "nqn.2014-08.org.nvmexpress:uuid:12345678"
        }));
        assert!(subsystem.properties.iter().any(|property| {
            property.key == "nvme.hostnqn"
                && property.value == "nqn.2014-08.org.nvmexpress:host:disk-nix"
        }));

        let tcp_controller = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "nvme-controller:nvme1")
            .expect("tcp controller node should exist");
        assert!(
            tcp_controller.properties.iter().any(|property| {
                property.key == "nvme.traddr" && property.value == "192.0.2.10"
            })
        );
        assert!(tcp_controller.properties.iter().any(|property| {
            property.key == "nvme.path-state" && property.value == "connecting"
        }));
        assert!(tcp_controller.properties.iter().any(|property| {
            property.key == "nvme.ana-state" && property.value == "inaccessible"
        }));

        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "nvme-subsystem:nvme-subsys0"
                && edge.to.0 == "nvme-controller:nvme0"
                && edge.relationship == Relationship::Contains
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "nvme-controller:nvme0"
                && edge.to.0 == "block:/dev/nvme0n1"
                && edge.relationship == Relationship::Contains
        }));
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

    #[test]
    fn normalizes_nvme_smart_log_json() {
        let graph = normalize_nvme_smart_log_json("/dev/nvme0", NVME_SMART_LOG)
            .expect("fixture should parse");

        let node = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::NvmeController)
            .expect("controller node should exist");

        assert_eq!(node.name, "nvme0");
        assert_eq!(node.path.as_deref(), Some("/dev/nvme0"));
        assert!(
            node.properties
                .iter()
                .any(|property| property.key == "nvme.smart.temperature-kelvin"
                    && property.value == "301")
        );
        assert!(
            node.properties
                .iter()
                .any(|property| property.key == "nvme.smart.percent-used" && property.value == "2")
        );
        assert!(
            node.properties
                .iter()
                .any(|property| property.key == "nvme.smart.media-errors" && property.value == "0")
        );
        assert!(node.properties.iter().any(|property| {
            property.key == "nvme.smart.temperature-sensor-2-kelvin" && property.value == "302"
        }));
    }
}
