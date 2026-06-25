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
    pub set_properties: Vec<(String, String)>,
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
    let cache_set_dir = cache_set_dir_from_link(bcache_dir);
    let set_uuid = read_trimmed(bcache_dir.join("set_uuid"))
        .or_else(|| cache_set_dir.as_deref().and_then(cache_set_uuid_from_path));
    let backing_device = read_trimmed(bcache_dir.join("backing_dev_name"));
    let mut properties = Vec::new();

    for key in [
        "block_size",
        "btree_cache_size",
        "bucket_size",
        "cache_available_percent",
        "cache_mode",
        "cache_replacement_policy",
        "cache_read_races",
        "congested_read_threshold_us",
        "congested_write_threshold_us",
        "discard",
        "dirty_data",
        "io_errors",
        "label",
        "metadata_written",
        "priority_stats",
        "readahead",
        "running",
        "sequential_cutoff",
        "state",
        "uuid",
        "written",
        "writeback_delay",
        "writeback_metadata",
        "writeback_percent",
        "writeback_rate",
        "writeback_rate_debug",
        "writeback_rate_d_term",
        "writeback_rate_i_term_inverse",
        "writeback_rate_minimum",
        "writeback_rate_p_term_inverse",
        "writeback_rate_update_seconds",
        "writeback_running",
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
        set_properties: cache_set_dir
            .as_deref()
            .map(read_cache_set_properties)
            .unwrap_or_default(),
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
    if let Some(backing_device) = &device.backing_device {
        node = node.with_property("bcache.backing-device", dev_path(backing_device));
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
        let mut set_node = Node::new(set_id.clone(), NodeKind::CacheDevice, set_uuid.clone())
            .with_property("bcache.kind", "cache-set");
        for (key, value) in &device.set_properties {
            set_node = set_node.with_property(key.clone(), value.clone());
        }
        graph.add_node(set_node);
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

fn cache_set_dir_from_link(bcache_dir: &Path) -> Option<PathBuf> {
    let target = fs::read_link(bcache_dir.join("cache")).ok()?;
    Some(if target.is_absolute() {
        target
    } else {
        bcache_dir.join(target)
    })
}

fn cache_set_uuid_from_path(path: &Path) -> Option<String> {
    path.file_name()
        .map(|value| value.to_string_lossy().into_owned())
}

fn read_cache_set_properties(cache_set_dir: &Path) -> Vec<(String, String)> {
    let mut properties = Vec::new();
    for key in [
        "average_key_size",
        "btree_cache_size",
        "cache_available_percent",
        "congested",
        "congested_read_threshold_us",
        "congested_write_threshold_us",
        "io_error_halflife",
        "io_error_limit",
        "journal_delay_ms",
        "root_usage_percent",
    ] {
        if let Some(value) = read_trimmed(cache_set_dir.join(key)) {
            properties.push((format!("bcache.set-{}", key.replace('_', "-")), value));
        }
    }
    properties
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
                    set_properties: vec![
                        (
                            "bcache.set-average-key-size".to_string(),
                            "16.0k".to_string(),
                        ),
                        ("bcache.set-root-usage-percent".to_string(), "3".to_string()),
                    ],
                    properties: vec![
                        (
                            "bcache.cache-available-percent".to_string(),
                            "78".to_string(),
                        ),
                        ("bcache.cache-mode".to_string(), "writeback".to_string()),
                        (
                            "bcache.congested-read-threshold-us".to_string(),
                            "2000".to_string(),
                        ),
                        (
                            "bcache.congested-write-threshold-us".to_string(),
                            "20000".to_string(),
                        ),
                        ("bcache.dirty-data".to_string(), "64.0M".to_string()),
                        ("bcache.running".to_string(), "1".to_string()),
                        ("bcache.state".to_string(), "clean".to_string()),
                        ("bcache.writeback-delay".to_string(), "30".to_string()),
                        (
                            "bcache.writeback-rate-minimum".to_string(),
                            "4.0k".to_string(),
                        ),
                        (
                            "bcache.writeback-rate-update-seconds".to_string(),
                            "5".to_string(),
                        ),
                        ("bcache.writeback-running".to_string(), "1".to_string()),
                    ],
                },
                BcacheDevice {
                    name: "nvme0n1p3".to_string(),
                    role: BcacheRole::Cache,
                    backing_device: None,
                    set_uuid: Some("cache-set-uuid".to_string()),
                    set_properties: Vec::new(),
                    properties: vec![
                        ("bcache.label".to_string(), "fast-cache".to_string()),
                        ("bcache.discard".to_string(), "true".to_string()),
                        ("bcache.io-errors".to_string(), "0".to_string()),
                        ("bcache.metadata-written".to_string(), "128.0M".to_string()),
                        (
                            "bcache.priority-stats".to_string(),
                            "Unused: 0% Metadata: 1%".to_string(),
                        ),
                        ("bcache.written".to_string(), "512.0M".to_string()),
                    ],
                },
            ],
        };

        let graph = normalize_bcache_snapshot(&snapshot);

        assert!(graph.nodes.iter().any(|node| {
            node.id.0 == "block:/dev/bcache0"
                && node.properties.iter().any(|property| {
                    property.key == "bcache.cache-mode" && property.value == "writeback"
                })
                && node.properties.iter().any(|property| {
                    property.key == "bcache.cache-available-percent" && property.value == "78"
                })
                && node.properties.iter().any(|property| {
                    property.key == "bcache.writeback-running" && property.value == "1"
                })
                && node.properties.iter().any(|property| {
                    property.key == "bcache.congested-read-threshold-us" && property.value == "2000"
                })
                && node.properties.iter().any(|property| {
                    property.key == "bcache.writeback-rate-update-seconds" && property.value == "5"
                })
                && node.properties.iter().any(|property| {
                    property.key == "bcache.backing-device" && property.value == "/dev/sdb1"
                })
        }));
        assert!(graph.nodes.iter().any(|node| {
            node.id.0 == "block:/dev/nvme0n1p3"
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "bcache.io-errors" && property.value == "0")
                && node.properties.iter().any(|property| {
                    property.key == "bcache.priority-stats"
                        && property.value == "Unused: 0% Metadata: 1%"
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
        assert!(graph.nodes.iter().any(|node| {
            node.id.0 == "bcache-set:cache-set-uuid"
                && node.properties.iter().any(|property| {
                    property.key == "bcache.set-average-key-size" && property.value == "16.0k"
                })
                && node.properties.iter().any(|property| {
                    property.key == "bcache.set-root-usage-percent" && property.value == "3"
                })
        }));
    }

    #[test]
    fn reads_bcache_identity_and_sizing_sysfs_fields() {
        let root =
            std::env::temp_dir().join(format!("disk-nix-bcache-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let bcache_dir = root.join("bcache0").join("bcache");
        let cache_set_dir = root.join("cache-set-uuid");
        fs::create_dir_all(&bcache_dir).expect("fixture directory can be created");
        fs::create_dir_all(&cache_set_dir).expect("cache set fixture directory can be created");
        for (name, value) in [
            ("uuid", "backing-uuid"),
            ("block_size", "512"),
            ("bucket_size", "1024"),
            ("btree_cache_size", "128k"),
            ("cache_read_races", "0"),
            ("cache_mode", "writethrough"),
            ("backing_dev_name", "sdc1"),
            ("set_uuid", "cache-set-uuid"),
        ] {
            fs::write(bcache_dir.join(name), value).expect("fixture field can be written");
        }
        for (name, value) in [
            ("average_key_size", "16.0k"),
            ("journal_delay_ms", "100"),
            ("root_usage_percent", "3"),
        ] {
            fs::write(cache_set_dir.join(name), value)
                .expect("cache set fixture field can be written");
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink("../../cache-set-uuid", bcache_dir.join("cache"))
            .expect("cache set fixture link can be created");

        let snapshot = read_sysfs_snapshot(&root).expect("fixture sysfs snapshot parses");
        let _ = fs::remove_dir_all(&root);

        let device = snapshot
            .devices
            .iter()
            .find(|device| device.name == "bcache0")
            .expect("bcache device was discovered");
        assert_eq!(device.backing_device.as_deref(), Some("sdc1"));
        assert_eq!(device.set_uuid.as_deref(), Some("cache-set-uuid"));
        for (key, value) in [
            ("bcache.uuid", "backing-uuid"),
            ("bcache.block-size", "512"),
            ("bcache.bucket-size", "1024"),
            ("bcache.btree-cache-size", "128k"),
            ("bcache.cache-read-races", "0"),
            ("bcache.cache-mode", "writethrough"),
        ] {
            assert!(
                device
                    .properties
                    .iter()
                    .any(|(property, actual)| property == key && actual == value),
                "missing {key}={value:?} in {:?}",
                device.properties
            );
        }
        for (key, value) in [
            ("bcache.set-average-key-size", "16.0k"),
            ("bcache.set-journal-delay-ms", "100"),
            ("bcache.set-root-usage-percent", "3"),
        ] {
            assert!(
                device
                    .set_properties
                    .iter()
                    .any(|(property, actual)| property == key && actual == value),
                "missing {key}={value:?} in {:?}",
                device.set_properties
            );
        }
    }
}
