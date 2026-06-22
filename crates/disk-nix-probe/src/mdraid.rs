use disk_nix_model::{Edge, Identity, Node, NodeKind, Relationship, StorageGraph, Usage};

use crate::ProbeError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdArrayReport {
    pub name: String,
    pub detail: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MdArray {
    name: String,
    uuid: Option<String>,
    level: Option<String>,
    state: Option<String>,
    size_bytes: Option<u64>,
    used_devices: Option<String>,
    total_devices: Option<String>,
    members: Vec<MdMember>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MdMember {
    path: String,
    state: Option<String>,
}

pub fn arrays_from_scan(bytes: &[u8]) -> Result<Vec<String>, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read mdadm scan output: {error}"))
    })?;
    Ok(text.lines().filter_map(array_name_from_scan_line).collect())
}

pub fn normalize_md_arrays(reports: &[MdArrayReport]) -> Result<StorageGraph, ProbeError> {
    let mut graph = StorageGraph::empty();

    for report in reports {
        let array = parse_detail(&report.name, &report.detail)?;
        add_array(&mut graph, array);
    }

    Ok(graph)
}

fn array_name_from_scan_line(line: &str) -> Option<String> {
    let mut parts = line.split_whitespace();
    if parts.next()? == "ARRAY" {
        parts.next().map(ToOwned::to_owned)
    } else {
        None
    }
}

fn parse_detail(name: &str, bytes: &[u8]) -> Result<MdArray, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read mdadm detail output: {error}"))
    })?;
    let mut array = MdArray {
        name: name.to_string(),
        uuid: None,
        level: None,
        state: None,
        size_bytes: None,
        used_devices: None,
        total_devices: None,
        members: Vec::new(),
    };
    let mut in_member_table = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Raid Level :") {
            array.level = value_after_colon(trimmed);
        } else if trimmed.starts_with("Array Size :") {
            array.size_bytes = parse_array_size(trimmed);
        } else if trimmed.starts_with("State :") {
            array.state = value_after_colon(trimmed);
        } else if trimmed.starts_with("UUID :") {
            array.uuid = value_after_colon(trimmed);
        } else if trimmed.starts_with("Raid Devices :") {
            array.used_devices = value_after_colon(trimmed);
        } else if trimmed.starts_with("Total Devices :") {
            array.total_devices = value_after_colon(trimmed);
        } else if trimmed.starts_with("Number")
            && trimmed.contains("Major")
            && trimmed.contains("RaidDevice")
            && trimmed.contains("State")
        {
            in_member_table = true;
        } else if in_member_table {
            if let Some(member) = parse_member_line(trimmed) {
                array.members.push(member);
            }
        }
    }

    Ok(array)
}

fn add_array(graph: &mut StorageGraph, array: MdArray) {
    let id = format!("md:{}", array.name);
    let mut node =
        Node::new(id.clone(), NodeKind::MdRaid, array.name.clone()).with_path(array.name);

    if let Some(size_bytes) = array.size_bytes {
        node = node.with_size_bytes(size_bytes);
    }
    if let Some(uuid) = array.uuid {
        node = node.with_identity(Identity {
            uuid: Some(uuid),
            ..Identity::default()
        });
    }

    let usage = Usage {
        used_bytes: node.size_bytes,
        free_bytes: None,
        allocated_bytes: node.size_bytes,
    };
    if !usage.is_empty() {
        node = node.with_usage(usage);
    }

    for (key, value) in [
        ("md.level", array.level),
        ("md.state", array.state),
        ("md.raid-devices", array.used_devices),
        ("md.total-devices", array.total_devices),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
    }

    graph.add_node(node);

    for member in array.members {
        let member_id = format!("block:{}", member.path);
        let mut member_node =
            Node::new(member_id.clone(), NodeKind::Partition, member.path.clone())
                .with_path(member.path);
        if let Some(state) = member.state {
            member_node = member_node.with_property("md.member-state", state);
        }
        graph.add_node(member_node);
        graph.add_edge(Edge::new(member_id, id.clone(), Relationship::MemberOf));
    }
}

fn parse_member_line(line: &str) -> Option<MdMember> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    let path = parts.iter().rev().find(|part| part.starts_with("/dev/"))?;
    let path_index = parts.iter().position(|part| part == path)?;
    let state = (path_index > 0).then(|| parts[5..path_index].join(" "));

    Some(MdMember {
        path: (*path).to_string(),
        state,
    })
}

fn parse_array_size(line: &str) -> Option<u64> {
    let value = value_after_colon(line)?;
    value
        .split_whitespace()
        .next()
        .and_then(|kib| kib.parse::<u64>().ok())
        .map(|kib| kib * 1024)
}

fn value_after_colon(value: &str) -> Option<String> {
    value
        .split_once(':')
        .map(|(_, value)| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use disk_nix_model::{NodeKind, Relationship};

    use super::*;

    const SCAN: &[u8] = b"ARRAY /dev/md0 metadata=1.2 UUID=aaaa:bbbb:cccc:dddd name=host:0\n";
    const DETAIL: &[u8] = b"/dev/md0:\n\
           Version : 1.2\n\
        Raid Level : raid1\n\
        Array Size : 1046528 (1022.00 MiB 1071.64 MB)\n\
       Raid Devices : 2\n\
      Total Devices : 2\n\
              State : clean\n\
               UUID : aaaa:bbbb:cccc:dddd\n\
\n\
    Number   Major   Minor   RaidDevice State\n\
       0       8        1        0      active sync   /dev/sda1\n\
       1       8       17        1      active sync   /dev/sdb1\n";

    #[test]
    fn extracts_arrays_from_scan() {
        assert_eq!(
            arrays_from_scan(SCAN).expect("scan should parse"),
            vec!["/dev/md0"]
        );
    }

    #[test]
    fn normalizes_md_detail_into_graph() {
        let graph = normalize_md_arrays(&[MdArrayReport {
            name: "/dev/md0".to_string(),
            detail: DETAIL.to_vec(),
        }])
        .expect("detail should parse");

        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::MdRaid && node.name == "/dev/md0")
        );
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|edge| edge.relationship == Relationship::MemberOf)
                .count(),
            2
        );
    }
}
