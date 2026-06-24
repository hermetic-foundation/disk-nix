use disk_nix_model::{Edge, Node, NodeKind, Relationship, StorageGraph};

use crate::ProbeError;

#[derive(Debug, Clone, PartialEq, Eq)]
struct MultipathMap {
    name: String,
    wwid: Option<String>,
    dm_name: Option<String>,
    vendor_product: Option<String>,
    size: Option<String>,
    features: Option<String>,
    hwhandler: Option<String>,
    wp: Option<String>,
    paths: Vec<MultipathPath>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MultipathPath {
    host_path: Option<String>,
    device: String,
    major_minor: Option<String>,
    group_policy: Option<String>,
    group_prio: Option<String>,
    group_status: Option<String>,
    state: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MultipathHeader {
    name: String,
    wwid: Option<String>,
    dm_name: Option<String>,
    vendor_product: Option<String>,
}

pub fn normalize_multipath_output(bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let maps = parse_maps(bytes)?;
    let mut graph = StorageGraph::empty();

    for map in maps {
        add_map(&mut graph, map);
    }

    Ok(graph)
}

fn parse_maps(bytes: &[u8]) -> Result<Vec<MultipathMap>, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read multipath output: {error}"))
    })?;
    let mut maps = Vec::new();
    let mut current: Option<MultipathMap> = None;
    let mut current_group = MultipathPathGroup::default();

    for line in text.lines() {
        let trimmed = strip_tree_prefix(line.trim());
        if trimmed.is_empty() {
            continue;
        }

        if let Some(header) = parse_header(trimmed) {
            if let Some(map) = current.take() {
                maps.push(map);
            }
            current = Some(MultipathMap {
                name: header.name,
                wwid: header.wwid,
                dm_name: header.dm_name,
                vendor_product: header.vendor_product,
                size: None,
                features: None,
                hwhandler: None,
                wp: None,
                paths: Vec::new(),
            });
            current_group = MultipathPathGroup::default();
        } else if let Some(map) = &mut current {
            if trimmed.starts_with("size=") {
                parse_properties(map, trimmed);
            } else if trimmed.starts_with("policy=") {
                current_group = parse_path_group(trimmed);
            } else if let Some(path) = parse_path(trimmed) {
                map.paths.push(path.with_group(&current_group));
            } else if map.vendor_product.is_none() && trimmed.contains(',') {
                map.vendor_product = Some(trimmed.to_string());
            }
        }
    }

    if let Some(map) = current {
        maps.push(map);
    }

    Ok(maps)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct MultipathPathGroup {
    policy: Option<String>,
    prio: Option<String>,
    status: Option<String>,
}

fn add_map(graph: &mut StorageGraph, map: MultipathMap) {
    let id = format!("multipath:{}", map.name);
    let mut node = Node::new(id.clone(), NodeKind::MultipathDevice, map.name.clone());

    if let Some(dm_name) = map.dm_name {
        node = node
            .with_path(format!("/dev/mapper/{}", map.name))
            .with_property("multipath.dm", dm_name);
    }

    for (key, value) in [
        ("multipath.wwid", map.wwid),
        ("multipath.vendor-product", map.vendor_product),
        ("multipath.size", map.size),
        ("multipath.features", map.features),
        ("multipath.hwhandler", map.hwhandler),
        ("multipath.write-protect", map.wp),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
    }

    graph.add_node(node);

    for path in map.paths {
        let path_id = format!("block:/dev/{}", path.device);
        let mut path_node = Node::new(
            path_id.clone(),
            NodeKind::PhysicalDisk,
            format!("/dev/{}", path.device),
        )
        .with_path(format!("/dev/{}", path.device));

        if let Some(host_path) = path.host_path {
            for (key, value) in scsi_coordinates(&host_path) {
                path_node = path_node.with_property(key, value);
            }
            path_node = path_node.with_property("multipath.host-path", host_path);
        }
        if let Some(major_minor) = path.major_minor {
            path_node = path_node.with_property("major-minor", major_minor);
        }
        if let Some(policy) = path.group_policy {
            path_node = path_node.with_property("multipath.group-policy", policy);
        }
        if let Some(prio) = path.group_prio {
            path_node = path_node.with_property("multipath.group-prio", prio);
        }
        if let Some(status) = path.group_status {
            path_node = path_node.with_property("multipath.group-status", status);
        }
        if !path.state.is_empty() {
            for (key, value) in path_state_columns(&path.state) {
                path_node = path_node.with_property(key, value);
            }
            path_node = path_node.with_property("multipath.path-state", path.state.join(" "));
        }

        graph.add_node(path_node);
        graph.add_edge(Edge::new(path_id, id.clone(), Relationship::Backs));
    }
}

fn parse_header(line: &str) -> Option<MultipathHeader> {
    let first = line.split_whitespace().next()?;
    if !line.contains("dm-") || first.contains('=') {
        return None;
    }

    let name = first.to_string();
    let wwid = line
        .split_once('(')
        .and_then(|(_, rest)| rest.split_once(')'))
        .map(|(wwid, _)| wwid.to_string());
    let dm_name = line
        .split_whitespace()
        .find(|part| part.starts_with("dm-"))
        .map(ToOwned::to_owned);
    let vendor_product = dm_name
        .as_deref()
        .and_then(|dm_name| line.split_once(dm_name))
        .map(|(_, rest)| rest.trim().to_string())
        .filter(|value| !value.is_empty());

    Some(MultipathHeader {
        name,
        wwid,
        dm_name,
        vendor_product,
    })
}

fn parse_properties(map: &mut MultipathMap, line: &str) {
    for (key, value) in parse_key_values(line) {
        match key.as_str() {
            "size" => map.size = Some(value),
            "features" => map.features = Some(value),
            "hwhandler" => map.hwhandler = Some(value),
            "wp" => map.wp = Some(value),
            _ => {}
        }
    }
}

fn parse_path_group(line: &str) -> MultipathPathGroup {
    let mut group = MultipathPathGroup::default();
    for (key, value) in parse_key_values(line) {
        match key.as_str() {
            "policy" => group.policy = Some(value),
            "prio" => group.prio = Some(value),
            "status" => group.status = Some(value),
            _ => {}
        }
    }
    group
}

fn parse_path(line: &str) -> Option<MultipathPath> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    let device_index = parts.iter().position(|part| looks_like_kernel_disk(part))?;
    let device = parts[device_index].to_string();
    let host_path = (device_index > 0).then(|| parts[device_index - 1].to_string());
    let major_minor = parts
        .get(device_index + 1)
        .map(|value| (*value).to_string());
    let state = parts
        .get(device_index + 2..)
        .unwrap_or_default()
        .iter()
        .map(|value| (*value).to_string())
        .collect();

    Some(MultipathPath {
        host_path,
        device,
        major_minor,
        group_policy: None,
        group_prio: None,
        group_status: None,
        state,
    })
}

fn scsi_coordinates(host_path: &str) -> Vec<(String, String)> {
    let parts: Vec<&str> = host_path.split(':').collect();
    if parts.len() != 4 || parts.iter().any(|part| part.is_empty()) {
        return Vec::new();
    }
    [
        ("multipath.scsi-host", parts[0]),
        ("multipath.scsi-channel", parts[1]),
        ("multipath.scsi-id", parts[2]),
        ("multipath.scsi-lun", parts[3]),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_string(), value.to_string()))
    .collect()
}

fn path_state_columns(state: &[String]) -> Vec<(String, String)> {
    let mut columns: Vec<(String, String)> = [
        ("multipath.dm-state", state.first()),
        ("multipath.checker-state", state.get(1)),
        ("multipath.online-state", state.get(2)),
    ]
    .into_iter()
    .filter_map(|(key, value)| value.map(|value| (key.to_string(), value.clone())))
    .collect();

    if state.len() > 3 {
        columns.push(("multipath.path-flags".to_string(), state[3..].join(" ")));
    }

    columns
}

impl MultipathPath {
    fn with_group(mut self, group: &MultipathPathGroup) -> Self {
        self.group_policy.clone_from(&group.policy);
        self.group_prio.clone_from(&group.prio);
        self.group_status.clone_from(&group.status);
        self
    }
}

fn parse_key_values(line: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let mut iter = line.char_indices().peekable();

    while let Some((_, character)) = iter.peek().copied() {
        if character.is_whitespace() {
            iter.next();
            continue;
        }

        let key_start = iter.peek().map(|(index, _)| *index).unwrap_or(line.len());
        while let Some((_, character)) = iter.peek().copied() {
            if character == '=' || character.is_whitespace() {
                break;
            }
            iter.next();
        }
        let key_end = iter.peek().map(|(index, _)| *index).unwrap_or(line.len());

        while let Some((_, character)) = iter.peek().copied() {
            if character == '=' {
                iter.next();
                break;
            }
            if !character.is_whitespace() {
                break;
            }
            iter.next();
        }

        let Some((_, character)) = iter.peek().copied() else {
            break;
        };
        if character != '=' && key_end == key_start {
            iter.next();
            continue;
        }

        let value = if character == '\'' || character == '"' {
            iter.next();
            let quote = character;
            let value_start = iter.peek().map(|(index, _)| *index).unwrap_or(line.len());
            while let Some((_, character)) = iter.peek().copied() {
                if character == quote {
                    break;
                }
                iter.next();
            }
            let value_end = iter.peek().map(|(index, _)| *index).unwrap_or(line.len());
            if iter.peek().is_some() {
                iter.next();
            }
            line[value_start..value_end].to_string()
        } else {
            let value_start = iter.peek().map(|(index, _)| *index).unwrap_or(line.len());
            while let Some((_, character)) = iter.peek().copied() {
                if character.is_whitespace() {
                    break;
                }
                iter.next();
            }
            let value_end = iter.peek().map(|(index, _)| *index).unwrap_or(line.len());
            line[value_start..value_end].to_string()
        };

        if key_end > key_start {
            pairs.push((line[key_start..key_end].to_string(), value));
        }
    }

    pairs
}

fn strip_tree_prefix(line: &str) -> &str {
    line.trim_start_matches(['|', '`', '-', '+', ' '])
}

fn looks_like_kernel_disk(value: &str) -> bool {
    value.starts_with("sd")
        || value.starts_with("vd")
        || value.starts_with("xvd")
        || value.starts_with("nvme")
}

#[cfg(test)]
mod tests {
    use disk_nix_model::{NodeKind, Relationship};

    use super::*;

    const MULTIPATH: &[u8] = br#"
mpatha (3600508b400105e210000900000490000) dm-2 IBM,2145
size=100G features='1 queue_if_no_path' hwhandler='1 alua' wp=rw
|-+- policy='service-time 0' prio=50 status=active
| `- 2:0:0:1 sdb 8:16 active ready running ghost
`-+- policy='service-time 0' prio=10 status=enabled
  `- 3:0:0:1 sdc 8:32 active ready running faulty shaky
"#;

    #[test]
    fn normalizes_multipath_map_and_paths() {
        let graph = normalize_multipath_output(MULTIPATH).expect("fixture should parse");

        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::MultipathDevice && node.name == "mpatha")
        );
        let map = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::MultipathDevice && node.name == "mpatha")
            .expect("multipath map should exist");
        assert!(map.properties.iter().any(|property| {
            property.key == "multipath.features" && property.value == "1 queue_if_no_path"
        }));
        assert!(map.properties.iter().any(|property| {
            property.key == "multipath.vendor-product" && property.value == "IBM,2145"
        }));
        assert!(map.properties.iter().any(|property| {
            property.key == "multipath.hwhandler" && property.value == "1 alua"
        }));
        let active_path = graph
            .nodes
            .iter()
            .find(|node| node.path.as_deref() == Some("/dev/sdb"))
            .expect("active path should exist");
        assert!(active_path.properties.iter().any(|property| {
            property.key == "multipath.group-policy" && property.value == "service-time 0"
        }));
        assert!(
            active_path
                .properties
                .iter()
                .any(|property| { property.key == "multipath.scsi-host" && property.value == "2" })
        );
        assert!(
            active_path.properties.iter().any(|property| {
                property.key == "multipath.scsi-channel" && property.value == "0"
            })
        );
        assert!(
            active_path
                .properties
                .iter()
                .any(|property| { property.key == "multipath.scsi-id" && property.value == "0" })
        );
        assert!(
            active_path
                .properties
                .iter()
                .any(|property| { property.key == "multipath.scsi-lun" && property.value == "1" })
        );
        assert!(
            active_path.properties.iter().any(|property| {
                property.key == "multipath.group-prio" && property.value == "50"
            })
        );
        assert!(active_path.properties.iter().any(|property| {
            property.key == "multipath.group-status" && property.value == "active"
        }));
        assert!(
            active_path
                .properties
                .iter()
                .any(|property| property.key == "multipath.dm-state" && property.value == "active")
        );
        assert!(active_path.properties.iter().any(|property| {
            property.key == "multipath.checker-state" && property.value == "ready"
        }));
        assert!(active_path.properties.iter().any(|property| {
            property.key == "multipath.online-state" && property.value == "running"
        }));
        assert!(
            active_path
                .properties
                .iter()
                .any(|property| property.key == "multipath.path-flags" && property.value == "ghost")
        );
        let enabled_path = graph
            .nodes
            .iter()
            .find(|node| node.path.as_deref() == Some("/dev/sdc"))
            .expect("enabled path should exist");
        assert!(
            enabled_path.properties.iter().any(|property| {
                property.key == "multipath.group-prio" && property.value == "10"
            })
        );
        assert!(enabled_path.properties.iter().any(|property| {
            property.key == "multipath.group-status" && property.value == "enabled"
        }));
        assert!(enabled_path.properties.iter().any(|property| {
            property.key == "multipath.path-flags" && property.value == "faulty shaky"
        }));
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|edge| edge.relationship == Relationship::Backs)
                .count(),
            2
        );
    }
}
