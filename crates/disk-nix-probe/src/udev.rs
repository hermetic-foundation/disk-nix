use std::collections::BTreeMap;

use disk_nix_model::{Identity, Node, NodeKind, StorageGraph};

use crate::ProbeError;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct UdevRecord {
    devpath: Option<String>,
    name: Option<String>,
    symlinks: Vec<String>,
    fields: BTreeMap<String, String>,
}

pub fn normalize_udev_export_db(bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let records = parse_export_db(bytes)?;
    let mut graph = StorageGraph::empty();

    for record in records.into_iter().filter(UdevRecord::is_block_device) {
        add_record(&mut graph, record);
    }

    Ok(graph)
}

fn parse_export_db(bytes: &[u8]) -> Result<Vec<UdevRecord>, ProbeError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| ProbeError::Adapter(format!("failed to read udev output: {error}")))?;
    let mut records = Vec::new();
    let mut record = UdevRecord::default();

    for line in text.lines().map(str::trim) {
        if line.is_empty() {
            push_record(&mut records, &mut record);
            continue;
        }

        let Some((prefix, value)) = line.split_once(": ") else {
            continue;
        };

        match prefix {
            "P" => record.devpath = non_empty(value),
            "N" => record.name = non_empty(value),
            "S" if !value.is_empty() => record.symlinks.push(value.to_string()),
            "E" => {
                if let Some((key, field_value)) = value.split_once('=') {
                    if !field_value.is_empty() {
                        record
                            .fields
                            .insert(key.to_string(), field_value.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    push_record(&mut records, &mut record);

    Ok(records)
}

fn push_record(records: &mut Vec<UdevRecord>, record: &mut UdevRecord) {
    if record.name.is_some() || record.fields.contains_key("DEVNAME") {
        records.push(std::mem::take(record));
    } else {
        *record = UdevRecord::default();
    }
}

fn add_record(graph: &mut StorageGraph, record: UdevRecord) {
    let Some(devname) = record.devname() else {
        return;
    };
    let id = format!("block:{devname}");
    let mut node = Node::new(id, node_kind(&record), devname.clone()).with_path(devname);

    let identity = identity(&record.fields);
    if !identity.is_empty() {
        node = node.with_identity(identity);
    }

    if let Some(devpath) = &record.devpath {
        node = node.with_property("udev.devpath", devpath.clone());
    }

    for symlink in &record.symlinks {
        node = node.with_property("udev.symlink", symlink.clone());
    }

    for (key, value) in &record.fields {
        if should_keep_property(key) {
            node = node.with_property(format!("udev.{}", normalize_key(key)), value.clone());
        }
    }

    graph.add_node(node);
}

fn node_kind(record: &UdevRecord) -> NodeKind {
    if record.fields.contains_key("DM_NAME") || record.fields.contains_key("DM_UUID") {
        return NodeKind::DeviceMapper;
    }

    match record.fields.get("DEVTYPE").map(String::as_str) {
        Some("disk") => NodeKind::PhysicalDisk,
        Some("partition") => NodeKind::Partition,
        _ => NodeKind::DeviceMapper,
    }
}

fn identity(fields: &BTreeMap<String, String>) -> Identity {
    Identity {
        uuid: fields.get("ID_FS_UUID").cloned(),
        partuuid: fields
            .get("ID_PART_ENTRY_UUID")
            .or_else(|| fields.get("ID_PART_TABLE_UUID"))
            .cloned(),
        label: fields.get("ID_FS_LABEL").cloned(),
        serial: fields
            .get("ID_SERIAL_SHORT")
            .or_else(|| fields.get("ID_SERIAL"))
            .cloned(),
        wwn: fields.get("ID_WWN").cloned(),
    }
}

fn should_keep_property(key: &str) -> bool {
    matches!(
        key,
        "DEVLINKS"
            | "DEVNAME"
            | "DEVTYPE"
            | "DM_LV_NAME"
            | "DM_NAME"
            | "DM_SUBSYSTEM_UDEV_FLAG0"
            | "DM_SUBSYSTEM_UDEV_FLAG1"
            | "DM_UDEV_DISABLE_OTHER_RULES_FLAG"
            | "DM_UDEV_PRIMARY_SOURCE_FLAG"
            | "DM_UDEV_RULES_VSN"
            | "DM_UUID"
            | "DM_VG_NAME"
            | "ID_BUS"
            | "ID_FS_LABEL"
            | "ID_FS_LABEL_ENC"
            | "ID_FS_LABEL_SAFE"
            | "ID_FS_TYPE"
            | "ID_FS_USAGE"
            | "ID_FS_UUID"
            | "ID_FS_UUID_ENC"
            | "ID_FS_UUID_SUB"
            | "ID_FS_VERSION"
            | "ID_MODEL"
            | "ID_MODEL_ID"
            | "ID_PART_ENTRY_DISK"
            | "ID_PART_ENTRY_FLAGS"
            | "ID_PART_ENTRY_NAME"
            | "ID_PART_ENTRY_NUMBER"
            | "ID_PART_ENTRY_OFFSET"
            | "ID_PART_ENTRY_SCHEME"
            | "ID_PART_ENTRY_SIZE"
            | "ID_PART_ENTRY_TYPE"
            | "ID_PART_ENTRY_UUID"
            | "ID_PART_TABLE_TYPE"
            | "ID_PART_TABLE_UUID"
            | "ID_PATH"
            | "ID_PATH_TAG"
            | "ID_REVISION"
            | "ID_SERIAL"
            | "ID_SERIAL_SHORT"
            | "ID_TYPE"
            | "ID_VENDOR"
            | "ID_VENDOR_ID"
            | "ID_WWN"
            | "MAJOR"
            | "MINOR"
            | "SUBSYSTEM"
    )
}

fn normalize_key(key: &str) -> String {
    key.to_ascii_lowercase().replace('_', "-")
}

fn non_empty(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

impl UdevRecord {
    fn devname(&self) -> Option<String> {
        self.fields
            .get("DEVNAME")
            .cloned()
            .or_else(|| self.name.as_ref().map(|name| format!("/dev/{name}")))
    }

    fn is_block_device(&self) -> bool {
        self.fields.get("SUBSYSTEM").map(String::as_str) == Some("block")
            && self
                .devname()
                .is_some_and(|devname| devname.starts_with("/dev/"))
    }
}

#[cfg(test)]
mod tests {
    use disk_nix_model::NodeKind;

    use super::*;

    const UDEV_EXPORT: &[u8] = br#"
P: /devices/pci0000:00/0000:00:17.0/ata1/host0/target0:0:0/0:0:0:0/block/sda/sda1
N: sda1
S: disk/by-id/ata-Samsung_SSD_SERIAL-part1
S: disk/by-partuuid/part-uuid
E: DEVNAME=/dev/sda1
E: DEVTYPE=partition
E: SUBSYSTEM=block
E: ID_BUS=ata
E: ID_MODEL=Samsung_SSD
E: ID_SERIAL=Samsung_SSD_SERIAL
E: ID_SERIAL_SHORT=SERIAL
E: ID_WWN=0x5002538d00000000
E: ID_FS_TYPE=vfat
E: ID_FS_UUID=AAAA-BBBB
E: ID_FS_UUID_ENC=AAAA-BBBB
E: ID_FS_UUID_SUB=CCCC-DDDD
E: ID_FS_LABEL=BOOT
E: ID_FS_LABEL_ENC=BOOT
E: ID_FS_LABEL_SAFE=BOOT
E: ID_FS_VERSION=FAT32
E: ID_FS_USAGE=filesystem
E: ID_PART_ENTRY_DISK=8:0
E: ID_PART_ENTRY_NUMBER=1
E: ID_PART_ENTRY_OFFSET=2048
E: ID_PART_ENTRY_SIZE=1048576
E: ID_PART_ENTRY_SCHEME=gpt
E: ID_PART_ENTRY_UUID=part-uuid
E: ID_PART_ENTRY_TYPE=uefi
E: ID_PART_ENTRY_NAME=EFI System Partition
E: ID_PART_ENTRY_FLAGS=0x1
E: ID_PART_TABLE_TYPE=gpt
E: ID_PART_TABLE_UUID=table-uuid
E: ID_PATH=pci-0000:00:17.0-ata-1
E: ID_PATH_TAG=pci-0000_00_17_0-ata-1
E: ID_REVISION=1B6Q
E: ID_TYPE=disk
E: ID_VENDOR=Samsung
E: ID_VENDOR_ID=144d
E: ID_MODEL_ID=a808
E: MAJOR=8
E: MINOR=1

P: /devices/virtual/block/dm-0
N: dm-0
E: DEVNAME=/dev/dm-0
E: DEVTYPE=disk
E: SUBSYSTEM=block
E: DM_NAME=cryptroot
E: DM_UUID=CRYPT-LUKS2-luks-uuid-cryptroot
E: DM_VG_NAME=vg0
E: DM_LV_NAME=root
E: DM_UDEV_RULES_VSN=3
E: DM_UDEV_PRIMARY_SOURCE_FLAG=1
E: DM_UDEV_DISABLE_OTHER_RULES_FLAG=0
E: DM_SUBSYSTEM_UDEV_FLAG0=1
"#;

    #[test]
    fn normalizes_block_device_identity_and_symlinks() {
        let graph = normalize_udev_export_db(UDEV_EXPORT).expect("fixture should parse");
        let partition = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/sda1")
            .expect("partition exists");

        assert_eq!(partition.kind, NodeKind::Partition);
        assert_eq!(partition.path.as_deref(), Some("/dev/sda1"));
        assert_eq!(partition.identity.uuid.as_deref(), Some("AAAA-BBBB"));
        assert_eq!(partition.identity.label.as_deref(), Some("BOOT"));
        assert_eq!(partition.identity.partuuid.as_deref(), Some("part-uuid"));
        assert_eq!(partition.identity.serial.as_deref(), Some("SERIAL"));
        assert_eq!(
            partition.identity.wwn.as_deref(),
            Some("0x5002538d00000000")
        );
        assert!(partition.properties.iter().any(|property| {
            property.key == "udev.symlink"
                && property.value == "disk/by-id/ata-Samsung_SSD_SERIAL-part1"
        }));
        assert!(
            partition
                .properties
                .iter()
                .any(|property| { property.key == "udev.id-fs-type" && property.value == "vfat" })
        );
        assert!(partition.properties.iter().any(|property| {
            property.key == "udev.id-fs-uuid-sub" && property.value == "CCCC-DDDD"
        }));
        assert!(partition.properties.iter().any(|property| {
            property.key == "udev.id-part-entry-flags" && property.value == "0x1"
        }));
        assert!(partition.properties.iter().any(|property| {
            property.key == "udev.id-path" && property.value == "pci-0000:00:17.0-ata-1"
        }));
        assert!(
            partition
                .properties
                .iter()
                .any(|property| { property.key == "udev.major" && property.value == "8" })
        );
    }

    #[test]
    fn treats_dm_records_as_device_mapper_nodes() {
        let graph = normalize_udev_export_db(UDEV_EXPORT).expect("fixture should parse");
        let mapper = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/dm-0")
            .expect("mapper exists");

        assert_eq!(mapper.kind, NodeKind::DeviceMapper);
        assert!(
            mapper.properties.iter().any(|property| {
                property.key == "udev.dm-name" && property.value == "cryptroot"
            })
        );
        assert!(
            mapper
                .properties
                .iter()
                .any(|property| { property.key == "udev.dm-vg-name" && property.value == "vg0" })
        );
        assert!(mapper.properties.iter().any(|property| {
            property.key == "udev.dm-subsystem-udev-flag0" && property.value == "1"
        }));
    }
}
