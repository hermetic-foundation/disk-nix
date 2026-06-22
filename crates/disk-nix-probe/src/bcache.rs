use std::{
    fs,
    path::{Path, PathBuf},
};

use disk_nix_model::{Edge, Node, NodeKind, Relationship, StorageGraph};

use crate::ProbeError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BcacheSnapshot {
    pub devices: Vec<BcacheDevice>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BcacheDevice {
    pub name: String,
    pub role: BcacheRole,
    pub backing_device: Option<String>,
    pub set_uuid: Option<String>,
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BcacheRole {
    Backing,
    Cache,
}

pub fn read_sysfs_snapshot(sys_block: &Path) -> Result<BcacheSnapshot, ProbeError> {
    let entries = match fs::read_dir(sys_block) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(BcacheSnapshot {
                devices: Vec::new(),
            });
        }
        Err(error) => {
            return Err(ProbeError::Adapter(format!(
                "failed to read {}: {error}",
                sys_block.display()
            )));
        }
    };
    let mut devices = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|error| {
            ProbeError::Adapter(format!("failed to read sysfs block entry: {error}"))
        })?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let bcache_dir = entry.path().join("bcache");
        if !bcache_dir.exists() {
            continue;
        }

        devices.push(read_device(&name, &bcache_dir));
    }

    Ok(BcacheSnapshot { devices })
}

pub fn normalize_bcache_snapshot(snapshot: &BcacheSnapshot) -> StorageGraph {
    let mut graph = StorageGraph::empty();

    for device in &snapshot.devices {
        add_device(&mut graph, device);
    }

    graph
}

fn read_device(name: &str, bcache_dir: &Path) -> BcacheDevice {
    let role = if name.starts_with("bcache") {
        BcacheRole::Backing
    } else {
        BcacheRole::Cache
    };
    let set_uuid =
        read_trimmed(bcache_dir.join("set_uuid")).or_else(|| cache_set_from_link(bcache_dir));
    let backing_device = read_trimmed(bcache_dir.join("backing_dev_name"));
    let mut properties = Vec::new();

    for key in [
        "cache_mode",
        "cache_replacement_policy",
        "dirty_data",
        "label",
        "readahead",
        "sequential_cutoff",
        "state",
        "writeback_percent",
        "writeback_rate",
    ] {
        if let Some(value) = read_trimmed(bcache_dir.join(key)) {
            properties.push((format!("bcache.{}", key.replace('_', "-")), value));
        }
    }

    BcacheDevice {
        name: name.to_string(),
        role,
        backing_device,
        set_uuid,
        properties,
    }
}

fn add_device(graph: &mut StorageGraph, device: &BcacheDevice) {
    let path = format!("/dev/{}", device.name);
    let id = format!("block:{path}");
    let mut node = Node::new(id.clone(), NodeKind::CacheDevice, device.name.clone())
        .with_path(path)
        .with_property("bcache.role", role_label(device.role));

    for (key, value) in &device.properties {
        node = node.with_property(key.clone(), value.clone());
    }
    if let Some(set_uuid) = &device.set_uuid {
        node = node.with_property("bcache.set-uuid", set_uuid.clone());
    }

    graph.add_node(node);

    if let Some(backing_device) = &device.backing_device {
        let backing_path = dev_path(backing_device);
        let backing_id = format!("block:{backing_path}");
        graph.add_node(
            Node::new(
                backing_id.clone(),
                NodeKind::PhysicalDisk,
                backing_path.clone(),
            )
            .with_path(backing_path),
        );
        graph.add_edge(Edge::new(backing_id, id.clone(), Relationship::Backs));
    }

    if let Some(set_uuid) = &device.set_uuid {
        let set_id = format!("bcache-set:{set_uuid}");
        graph.add_node(
            Node::new(set_id.clone(), NodeKind::CacheDevice, set_uuid.clone())
                .with_property("bcache.kind", "cache-set"),
        );
        graph.add_edge(Edge::new(
            set_id.clone(),
            id.clone(),
            Relationship::CacheFor,
        ));
        if device.role == BcacheRole::Cache {
            graph.add_edge(Edge::new(id, set_id, Relationship::MemberOf));
        }
    }
}

fn role_label(role: BcacheRole) -> &'static str {
    match role {
        BcacheRole::Backing => "backing",
        BcacheRole::Cache => "cache",
    }
}

fn dev_path(name: &str) -> String {
    if name.starts_with("/dev/") {
        name.to_string()
    } else {
        format!("/dev/{name}")
    }
}

fn cache_set_from_link(bcache_dir: &Path) -> Option<String> {
    let target = fs::read_link(bcache_dir.join("cache")).ok()?;
    target
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
}

fn read_trimmed(path: impl Into<PathBuf>) -> Option<String> {
    fs::read_to_string(path.into())
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use disk_nix_model::Relationship;

    use super::*;

    #[test]
    fn normalizes_bcache_backing_and_cache_devices() {
        let snapshot = BcacheSnapshot {
            devices: vec![
                BcacheDevice {
                    name: "bcache0".to_string(),
                    role: BcacheRole::Backing,
                    backing_device: Some("sdb1".to_string()),
                    set_uuid: Some("cache-set-uuid".to_string()),
                    properties: vec![
                        ("bcache.cache-mode".to_string(), "writeback".to_string()),
                        ("bcache.state".to_string(), "clean".to_string()),
                    ],
                },
                BcacheDevice {
                    name: "nvme0n1p3".to_string(),
                    role: BcacheRole::Cache,
                    backing_device: None,
                    set_uuid: Some("cache-set-uuid".to_string()),
                    properties: vec![("bcache.label".to_string(), "fast-cache".to_string())],
                },
            ],
        };

        let graph = normalize_bcache_snapshot(&snapshot);

        assert!(graph.nodes.iter().any(|node| {
            node.id.0 == "block:/dev/bcache0"
                && node.properties.iter().any(|property| {
                    property.key == "bcache.cache-mode" && property.value == "writeback"
                })
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "block:/dev/sdb1"
                && edge.to.0 == "block:/dev/bcache0"
                && edge.relationship == Relationship::Backs
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "bcache-set:cache-set-uuid"
                && edge.to.0 == "block:/dev/bcache0"
                && edge.relationship == Relationship::CacheFor
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "block:/dev/nvme0n1p3"
                && edge.to.0 == "bcache-set:cache-set-uuid"
                && edge.relationship == Relationship::MemberOf
        }));
    }
}
