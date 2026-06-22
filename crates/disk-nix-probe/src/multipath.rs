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
    state: Vec<String>,
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

    for line in text.lines() {
        let trimmed = strip_tree_prefix(line.trim());
        if trimmed.is_empty() {
            continue;
        }

        if let Some((name, wwid, dm_name)) = parse_header(trimmed) {
            if let Some(map) = current.take() {
                maps.push(map);
            }
            current = Some(MultipathMap {
                name,
                wwid,
                dm_name,
                vendor_product: None,
                size: None,
                features: None,
                hwhandler: None,
                wp: None,
                paths: Vec::new(),
            });
        } else if let Some(map) = &mut current {
            if trimmed.starts_with("size=") {
                parse_properties(map, trimmed);
            } else if let Some(path) = parse_path(trimmed) {
                map.paths.push(path);
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
            path_node = path_node.with_property("multipath.host-path", host_path);
        }
        if let Some(major_minor) = path.major_minor {
            path_node = path_node.with_property("major-minor", major_minor);
        }
        if !path.state.is_empty() {
            path_node = path_node.with_property("multipath.path-state", path.state.join(" "));
        }

        graph.add_node(path_node);
        graph.add_edge(Edge::new(path_id, id.clone(), Relationship::Backs));
    }
}

fn parse_header(line: &str) -> Option<(String, Option<String>, Option<String>)> {
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

    Some((name, wwid, dm_name))
}

fn parse_properties(map: &mut MultipathMap, line: &str) {
    for part in line.split_whitespace() {
        if let Some((key, value)) = part.split_once('=') {
            match key {
                "size" => map.size = Some(value.to_string()),
                "features" => map.features = Some(value.to_string()),
                "hwhandler" => map.hwhandler = Some(value.to_string()),
                "wp" => map.wp = Some(value.to_string()),
                _ => {}
            }
        }
    }
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
        state,
    })
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
| `- 2:0:0:1 sdb 8:16 active ready running
`-+- policy='service-time 0' prio=10 status=enabled
  `- 3:0:0:1 sdc 8:32 active ready running
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
