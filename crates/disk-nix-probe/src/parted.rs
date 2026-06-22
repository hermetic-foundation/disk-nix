use disk_nix_model::{Edge, Node, NodeKind, Relationship, StorageGraph};

use crate::ProbeError;

#[derive(Debug, Clone, PartialEq, Eq)]
struct PartedDisk {
    path: String,
    size: Option<String>,
    transport: Option<String>,
    logical_sector_size: Option<String>,
    physical_sector_size: Option<String>,
    partition_table: Option<String>,
    model: Option<String>,
    flags: Option<String>,
    partitions: Vec<PartedPartition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PartedPartition {
    number: String,
    start: Option<String>,
    end: Option<String>,
    size: Option<String>,
    partition_type: Option<String>,
    name: Option<String>,
    flags: Option<String>,
}

pub fn normalize_parted_machine(bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let disks = parse_parted_machine(bytes)?;
    let mut graph = StorageGraph::empty();

    for disk in disks {
        add_disk(&mut graph, disk);
    }

    Ok(graph)
}

fn parse_parted_machine(bytes: &[u8]) -> Result<Vec<PartedDisk>, ProbeError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| ProbeError::Adapter(format!("failed to read parted output: {error}")))?;
    let mut disks = Vec::new();
    let mut current: Option<PartedDisk> = None;

    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if line == "BYT;" {
            continue;
        }

        let fields: Vec<&str> = line.trim_end_matches(';').split(':').collect();
        let Some(first) = fields.first() else {
            continue;
        };

        if first.starts_with("/dev/") {
            if let Some(disk) = current.take() {
                disks.push(disk);
            }
            current = Some(parse_disk(&fields)?);
        } else if first.chars().all(|character| character.is_ascii_digit()) {
            if let Some(disk) = &mut current {
                disk.partitions.push(parse_partition(&fields)?);
            }
        }
    }

    if let Some(disk) = current {
        disks.push(disk);
    }

    Ok(disks)
}

fn parse_disk(fields: &[&str]) -> Result<PartedDisk, ProbeError> {
    if fields.len() < 6 {
        return Err(ProbeError::Adapter(format!(
            "parted disk row has {} fields, expected at least 6",
            fields.len()
        )));
    }

    Ok(PartedDisk {
        path: fields[0].to_string(),
        size: field(fields, 1),
        transport: field(fields, 2),
        logical_sector_size: field(fields, 3),
        physical_sector_size: field(fields, 4),
        partition_table: field(fields, 5),
        model: field(fields, 6),
        flags: field(fields, 7),
        partitions: Vec::new(),
    })
}

fn parse_partition(fields: &[&str]) -> Result<PartedPartition, ProbeError> {
    if fields.len() < 4 {
        return Err(ProbeError::Adapter(format!(
            "parted partition row has {} fields, expected at least 4",
            fields.len()
        )));
    }

    Ok(PartedPartition {
        number: fields[0].to_string(),
        start: field(fields, 1),
        end: field(fields, 2),
        size: field(fields, 3),
        partition_type: field(fields, 4),
        name: field(fields, 5),
        flags: field(fields, 6),
    })
}

fn add_disk(graph: &mut StorageGraph, disk: PartedDisk) {
    let id = block_id(&disk.path);
    let mut node = Node::new(id.clone(), NodeKind::PhysicalDisk, disk.path.clone())
        .with_path(disk.path.clone());

    if let Some(size_bytes) = parse_size_bytes(disk.size.as_deref()) {
        node = node.with_size_bytes(size_bytes);
    }

    for (key, value) in [
        ("parted.transport", disk.transport),
        ("parted.logical-sector-size", disk.logical_sector_size),
        ("parted.physical-sector-size", disk.physical_sector_size),
        ("partition.table", disk.partition_table),
        ("model", disk.model),
        ("partition.disk-flags", disk.flags),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
    }

    graph.add_node(node);

    for partition in disk.partitions {
        add_partition(graph, &disk.path, &id, partition);
    }
}

fn add_partition(
    graph: &mut StorageGraph,
    disk_path: &str,
    disk_id: &str,
    partition: PartedPartition,
) {
    let partition_path = partition_path(disk_path, &partition.number);
    let id = block_id(&partition_path);
    let mut node = Node::new(id.clone(), NodeKind::Partition, partition_path.clone())
        .with_path(partition_path.clone());

    if let Some(size_bytes) = parse_size_bytes(partition.size.as_deref()) {
        node = node.with_size_bytes(size_bytes);
    }

    for (key, value) in [
        ("partition.number", Some(partition.number)),
        ("partition.start", partition.start),
        ("partition.end", partition.end),
        ("partition.type", partition.partition_type),
        ("partition.name", partition.name),
        ("partition.flags", partition.flags),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
    }

    graph.add_node(node);
    graph.add_edge(Edge::new(disk_id.to_string(), id, Relationship::Contains));
}

fn field(fields: &[&str], index: usize) -> Option<String> {
    fields
        .get(index)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn block_id(path: &str) -> String {
    format!("block:{path}")
}

fn partition_path(disk_path: &str, number: &str) -> String {
    let needs_separator = disk_path
        .chars()
        .last()
        .is_some_and(|character| character.is_ascii_digit());
    if needs_separator {
        format!("{disk_path}p{number}")
    } else {
        format!("{disk_path}{number}")
    }
}

fn parse_size_bytes(value: Option<&str>) -> Option<u64> {
    let value = value?.trim();
    let numeric = value.strip_suffix('B').unwrap_or(value);
    numeric.parse().ok()
}

#[cfg(test)]
mod tests {
    use disk_nix_model::Relationship;

    use super::*;

    const PARTED: &[u8] = br#"
BYT;
/dev/nvme0n1:1000204886016B:nvme:512:4096:gpt:Samsung SSD:;
1:1048576B:1074790399B:1073741824B:fat32:EFI System Partition:boot, esp;
2:1074790400B:1000203091967B:999128301568B:ext4:nixos-root:;
/dev/sdb:500107862016B:scsi:512:512:msdos:ATA Disk:;
1:1048576B:500107862015B:500106813440B:primary::lvm;
"#;

    #[test]
    fn normalizes_parted_disks_and_partitions() {
        let graph = normalize_parted_machine(PARTED).expect("fixture should parse");

        let disk = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/nvme0n1")
            .expect("disk exists");
        assert_eq!(disk.size_bytes, Some(1_000_204_886_016));
        assert!(
            disk.properties
                .iter()
                .any(|property| { property.key == "partition.table" && property.value == "gpt" })
        );

        let partition = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/nvme0n1p1")
            .expect("partition exists");
        assert_eq!(partition.size_bytes, Some(1_073_741_824));
        assert!(partition.properties.iter().any(|property| {
            property.key == "partition.flags" && property.value == "boot, esp"
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "block:/dev/nvme0n1"
                && edge.to.0 == "block:/dev/nvme0n1p1"
                && edge.relationship == Relationship::Contains
        }));
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.id.0 == "block:/dev/sdb1")
        );
    }

    #[test]
    fn appends_p_separator_for_disk_names_ending_in_digits() {
        assert_eq!(partition_path("/dev/nvme0n1", "1"), "/dev/nvme0n1p1");
        assert_eq!(partition_path("/dev/sda", "1"), "/dev/sda1");
    }
}
