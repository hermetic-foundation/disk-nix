use disk_nix_model::{Identity, Node, NodeKind, StorageGraph};
use serde_json::Value;

use crate::ProbeError;

pub fn normalize_smartctl_json(path: &str, bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to parse smartctl JSON for {path}: {error}"))
    })?;
    let mut node = Node::new(format!("block:{path}"), NodeKind::PhysicalDisk, path).with_path(path);

    let identity = Identity {
        serial: field_string(&value, &["serial_number"]),
        wwn: wwn(&value),
        ..Identity::default()
    };
    if !identity.is_empty() {
        node = node.with_identity(identity);
    }

    if let Some(size_bytes) = field_u64(&value, &["user_capacity", "bytes"]) {
        node = node.with_size_bytes(size_bytes);
    }

    for (path, property) in [
        (&["smartctl", "svn_revision"][..], "smartctl.svn-revision"),
        (&["smartctl", "platform_info"][..], "smartctl.platform"),
        (&["smartctl", "exit_status"][..], "smartctl.exit-status"),
        (&["device", "name"][..], "smartctl.device-name"),
        (&["device", "type"][..], "smartctl.device-type"),
        (&["device", "protocol"][..], "smartctl.protocol"),
        (&["model_name"][..], "smartctl.model"),
        (&["model_family"][..], "smartctl.model-family"),
        (&["vendor"][..], "smartctl.vendor"),
        (&["product"][..], "smartctl.product"),
        (&["revision"][..], "smartctl.revision"),
        (&["firmware_version"][..], "smartctl.firmware-version"),
        (&["serial_number"][..], "smartctl.serial"),
        (&["wwn", "naa"][..], "smartctl.wwn-naa"),
        (&["wwn", "oui"][..], "smartctl.wwn-oui"),
        (&["wwn", "id"][..], "smartctl.wwn-id"),
        (
            &["user_capacity", "bytes"][..],
            "smartctl.user-capacity-bytes",
        ),
        (&["logical_block_size"][..], "smartctl.logical-block-size"),
        (&["physical_block_size"][..], "smartctl.physical-block-size"),
        (&["rotation_rate"][..], "smartctl.rotation-rate-rpm"),
        (&["form_factor", "name"][..], "smartctl.form-factor"),
        (&["sata_version", "string"][..], "smartctl.sata-version"),
        (
            &["interface_speed", "max", "sata_value"][..],
            "smartctl.interface-speed-max",
        ),
        (
            &["interface_speed", "current", "sata_value"][..],
            "smartctl.interface-speed-current",
        ),
        (&["smart_status", "passed"][..], "smartctl.health.passed"),
        (&["power_on_time", "hours"][..], "smartctl.power-on-hours"),
        (&["power_cycle_count"][..], "smartctl.power-cycle-count"),
        (
            &["temperature", "current"][..],
            "smartctl.temperature-current-celsius",
        ),
        (
            &["temperature", "highest"][..],
            "smartctl.temperature-highest-celsius",
        ),
        (
            &["temperature", "lowest"][..],
            "smartctl.temperature-lowest-celsius",
        ),
        (
            &[
                "ata_smart_data",
                "offline_data_collection",
                "status",
                "string",
            ][..],
            "smartctl.offline-data-collection-status",
        ),
        (
            &["ata_smart_data", "self_test", "status", "string"][..],
            "smartctl.self-test-status",
        ),
        (
            &["ata_smart_error_log", "summary", "count"][..],
            "smartctl.error-log-summary-count",
        ),
        (
            &["ata_smart_self_test_log", "standard", "count"][..],
            "smartctl.self-test-log-count",
        ),
        (
            &["ata_smart_data", "capabilities", "error_logging_supported"][..],
            "smartctl.error-logging-supported",
        ),
        (
            &["ata_smart_data", "capabilities", "gp_logging_supported"][..],
            "smartctl.gp-logging-supported",
        ),
        (
            &["ata_sct_capabilities", "value"][..],
            "smartctl.sct-capabilities",
        ),
        (
            &["scsi_grown_defect_list"][..],
            "smartctl.scsi-grown-defect-list",
        ),
    ] {
        if let Some(value) = field_string(&value, path) {
            node = node.with_property(property, value);
        }
    }

    for attribute in value
        .pointer("/ata_smart_attributes/table")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(name) = field_string(attribute, &["name"]) else {
            continue;
        };
        let key = smart_attribute_key(&name);
        if let Some(raw) = field_string(attribute, &["raw", "value"]) {
            node = node.with_property(format!("smartctl.attribute.{key}.raw"), raw);
        }
        if let Some(normalized) = field_string(attribute, &["value"]) {
            node = node.with_property(format!("smartctl.attribute.{key}.value"), normalized);
        }
        if let Some(worst) = field_string(attribute, &["worst"]) {
            node = node.with_property(format!("smartctl.attribute.{key}.worst"), worst);
        }
        if let Some(thresh) = field_string(attribute, &["thresh"]) {
            node = node.with_property(format!("smartctl.attribute.{key}.threshold"), thresh);
        }
        if let Some(when_failed) = field_string(attribute, &["when_failed"]) {
            node = node.with_property(format!("smartctl.attribute.{key}.when-failed"), when_failed);
        }
    }

    let mut graph = StorageGraph::empty();
    graph.add_node(node);
    Ok(graph)
}

fn field_string(value: &Value, path: &[impl AsRef<str>]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(key.as_ref())?;
    }
    match current {
        Value::Null => None,
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        Value::String(value) if value.is_empty() => None,
        Value::String(value) => Some(value.clone()),
        other => Some(other.to_string()),
    }
}

fn field_u64(value: &Value, path: &[impl AsRef<str>]) -> Option<u64> {
    let mut current = value;
    for key in path {
        current = current.get(key.as_ref())?;
    }
    current.as_u64().or_else(|| current.as_str()?.parse().ok())
}

fn wwn(value: &Value) -> Option<String> {
    let naa = field_string(value, &["wwn", "naa"])?;
    let oui = field_string(value, &["wwn", "oui"])?;
    let id = field_string(value, &["wwn", "id"])?;
    Some(format!("{naa}:{oui}:{id}"))
}

fn smart_attribute_key(name: &str) -> String {
    name.to_ascii_lowercase()
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
    use disk_nix_model::NodeKind;

    use super::*;

    const SMARTCTL: &[u8] = br#"
{
  "smartctl": {
    "version": [7, 4],
    "svn_revision": "5530",
    "platform_info": "x86_64-linux",
    "exit_status": 0
  },
  "device": {
    "name": "/dev/sda",
    "type": "sat",
    "protocol": "ATA"
  },
  "model_family": "Example SSDs",
  "model_name": "Example SSD",
  "serial_number": "SATA123",
  "firmware_version": "1.2.3",
  "user_capacity": {
    "bytes": 1000204886016
  },
  "logical_block_size": 512,
  "physical_block_size": 4096,
  "rotation_rate": 0,
  "form_factor": {
    "name": "2.5 inches"
  },
  "wwn": {
    "naa": 5,
    "oui": 12345,
    "id": 67890
  },
  "sata_version": {
    "string": "SATA 3.3"
  },
  "interface_speed": {
    "max": { "sata_value": 6.0 },
    "current": { "sata_value": 6.0 }
  },
  "smart_status": {
    "passed": true
  },
  "power_on_time": {
    "hours": 4242
  },
  "power_cycle_count": 12,
  "temperature": {
    "current": 31,
    "highest": 44,
    "lowest": 20
  },
  "ata_smart_data": {
    "offline_data_collection": {
      "status": { "string": "was completed without error" }
    },
    "self_test": {
      "status": { "string": "completed without error" }
    },
    "capabilities": {
      "error_logging_supported": true,
      "gp_logging_supported": true
    }
  },
  "ata_sct_capabilities": {
    "value": 61
  },
  "ata_smart_error_log": {
    "summary": {
      "count": 3
    }
  },
  "ata_smart_self_test_log": {
    "standard": {
      "count": 2
    }
  },
  "ata_smart_attributes": {
    "table": [
      {
        "id": 5,
        "name": "Reallocated_Sector_Ct",
        "value": 100,
        "worst": 100,
        "thresh": 10,
        "when_failed": "",
        "raw": { "value": 0 }
      },
      {
        "id": 9,
        "name": "Power_On_Hours",
        "value": 99,
        "worst": 99,
        "thresh": 0,
        "raw": { "value": 4242 }
      }
    ]
  }
}
"#;

    #[test]
    fn normalizes_smartctl_json() {
        let graph = normalize_smartctl_json("/dev/sda", SMARTCTL).expect("fixture parses");
        let node = graph.nodes.first().expect("node exists");

        assert_eq!(node.kind, NodeKind::PhysicalDisk);
        assert_eq!(node.path.as_deref(), Some("/dev/sda"));
        assert_eq!(node.size_bytes, Some(1_000_204_886_016));
        assert_eq!(node.identity.serial.as_deref(), Some("SATA123"));
        assert_eq!(node.identity.wwn.as_deref(), Some("5:12345:67890"));
        assert!(node.properties.iter().any(|property| {
            property.key == "smartctl.health.passed" && property.value == "true"
        }));
        assert!(node.properties.iter().any(|property| {
            property.key == "smartctl.temperature-current-celsius" && property.value == "31"
        }));
        assert!(node.properties.iter().any(|property| {
            property.key == "smartctl.error-log-summary-count" && property.value == "3"
        }));
        assert!(node.properties.iter().any(|property| {
            property.key == "smartctl.self-test-log-count" && property.value == "2"
        }));
        assert!(node.properties.iter().any(|property| {
            property.key == "smartctl.attribute.reallocated-sector-ct.raw" && property.value == "0"
        }));
        assert!(node.properties.iter().any(|property| {
            property.key == "smartctl.attribute.power-on-hours.raw" && property.value == "4242"
        }));
    }
}
