use disk_nix_model::{Edge, Identity, Node, NodeKind, Relationship, StorageGraph, Usage};
use serde::Deserialize;

use crate::{ProbeError, ProbeReport, ProbeStatus};

#[derive(Debug, Deserialize)]
struct LsblkDocument {
    blockdevices: Vec<LsblkDevice>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct LsblkDevice {
    name: Option<String>,
    path: Option<String>,
    kname: Option<String>,
    #[serde(rename = "type")]
    device_type: Option<String>,
    size: Option<u64>,
    fsused: Option<u64>,
    fsavail: Option<u64>,
    fstype: Option<String>,
    fsver: Option<String>,
    label: Option<String>,
    uuid: Option<String>,
    partuuid: Option<String>,
    serial: Option<String>,
    wwn: Option<String>,
    mountpoint: Option<String>,
    mountpoints: Option<Vec<Option<String>>>,
    pkname: Option<String>,
    tran: Option<String>,
    rota: Option<bool>,
    rm: Option<bool>,
    model: Option<String>,
    vendor: Option<String>,
    children: Option<Vec<LsblkDevice>>,
}

pub fn normalize_lsblk_json(bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let document: LsblkDocument = serde_json::from_slice(bytes)
        .map_err(|error| ProbeError::Adapter(format!("failed to parse lsblk JSON: {error}")))?;
    let mut graph = StorageGraph::empty();

    for device in &document.blockdevices {
        add_device(&mut graph, device, None);
    }

    Ok(graph)
}

pub fn available_report(node_count: usize) -> ProbeReport {
    ProbeReport {
        adapter: "lsblk".to_string(),
        status: ProbeStatus::Available,
        message: Some(format!(
            "normalized {node_count} graph nodes from lsblk JSON"
        )),
    }
}

fn add_device(graph: &mut StorageGraph, device: &LsblkDevice, parent: Option<String>) {
    let Some(name) = preferred_name(device) else {
        return;
    };
    let id = block_id(&name);
    let kind = node_kind(device);
    let mut node = Node::new(id.clone(), kind, name.clone());

    if let Some(path) = &device.path {
        node = node.with_path(path.clone());
    }
    if let Some(size_bytes) = device.size {
        node = node.with_size_bytes(size_bytes);
    }

    let identity = identity(device);
    if !identity.is_empty() {
        node = node.with_identity(identity);
    }

    let usage = usage(device);
    if !usage.is_empty() {
        node = node.with_usage(usage);
    }

    for (key, value) in properties(device) {
        node = node.with_property(key, value);
    }

    graph.add_node(node);

    if let Some(parent_id) = parent {
        graph.add_edge(Edge::new(parent_id, id.clone(), Relationship::Contains));
    } else if let Some(parent_name) = &device.pkname {
        graph.add_edge(Edge::new(
            block_id(parent_name),
            id.clone(),
            Relationship::Contains,
        ));
    }

    add_filesystem_and_mounts(graph, device, &id, &name);

    if let Some(children) = &device.children {
        for child in children {
            add_device(graph, child, Some(id.clone()));
        }
    }
}

fn add_filesystem_and_mounts(
    graph: &mut StorageGraph,
    device: &LsblkDevice,
    block_id: &str,
    block_name: &str,
) {
    let Some(fstype) = &device.fstype else {
        return;
    };

    let filesystem_id = format!("fs:{block_name}");
    let mut filesystem = Node::new(filesystem_id.clone(), NodeKind::Filesystem, fstype.clone());
    if let Some(fsver) = &device.fsver {
        filesystem = filesystem.with_property("version", fsver.clone());
    }
    if let Some(uuid) = &device.uuid {
        filesystem.identity.uuid = Some(uuid.clone());
    }
    if let Some(label) = &device.label {
        filesystem.identity.label = Some(label.clone());
    }

    graph.add_node(filesystem);
    graph.add_edge(Edge::new(
        block_id.to_string(),
        filesystem_id.clone(),
        Relationship::Backs,
    ));

    for mountpoint in mountpoints(device) {
        let mount_id = format!("mount:{mountpoint}");
        graph.add_node(Node::new(
            mount_id.clone(),
            NodeKind::Mountpoint,
            mountpoint.clone(),
        ));
        graph.add_edge(Edge::new(
            filesystem_id.clone(),
            mount_id,
            Relationship::MountedAt,
        ));
    }
}

fn preferred_name(device: &LsblkDevice) -> Option<String> {
    device
        .path
        .as_ref()
        .or(device.name.as_ref())
        .or(device.kname.as_ref())
        .cloned()
}

fn block_id(name: &str) -> String {
    format!("block:{name}")
}

fn node_kind(device: &LsblkDevice) -> NodeKind {
    match device.device_type.as_deref() {
        Some("disk") => {
            if device.tran.as_deref() == Some("nvme") || device.name_matches("nvme") {
                NodeKind::NvmeNamespace
            } else {
                NodeKind::PhysicalDisk
            }
        }
        Some("part") => NodeKind::Partition,
        Some("crypt") => NodeKind::LuksContainer,
        Some("lvm") => NodeKind::LvmLogicalVolume,
        Some("raid0" | "raid1" | "raid4" | "raid5" | "raid6" | "raid10") => NodeKind::MdRaid,
        Some("mpath") => NodeKind::MultipathDevice,
        Some("loop") => NodeKind::LoopDevice,
        Some("rom") => NodeKind::PhysicalDisk,
        Some(other) if other.starts_with("dm") => NodeKind::DeviceMapper,
        _ => NodeKind::DeviceMapper,
    }
}

fn identity(device: &LsblkDevice) -> Identity {
    Identity {
        uuid: device.uuid.clone(),
        partuuid: device.partuuid.clone(),
        label: device.label.clone(),
        serial: device.serial.clone(),
        wwn: device.wwn.clone(),
    }
}

fn usage(device: &LsblkDevice) -> Usage {
    Usage {
        used_bytes: device.fsused,
        free_bytes: device.fsavail,
        allocated_bytes: None,
    }
}

fn properties(device: &LsblkDevice) -> Vec<(&'static str, String)> {
    let mut properties = Vec::new();

    if let Some(device_type) = &device.device_type {
        properties.push(("lsblk.type", device_type.clone()));
    }
    if let Some(fstype) = &device.fstype {
        properties.push(("filesystem.type", fstype.clone()));
    }
    if let Some(tran) = &device.tran {
        properties.push(("transport", tran.clone()));
    }
    if let Some(rota) = device.rota {
        properties.push(("rotational", rota.to_string()));
    }
    if let Some(removable) = device.rm {
        properties.push(("removable", removable.to_string()));
    }
    if let Some(model) = &device.model {
        properties.push(("model", model.clone()));
    }
    if let Some(vendor) = &device.vendor {
        properties.push(("vendor", vendor.clone()));
    }

    properties
}

fn mountpoints(device: &LsblkDevice) -> Vec<String> {
    let mut mountpoints = Vec::new();

    if let Some(mountpoint) = &device.mountpoint {
        if !mountpoint.is_empty() {
            mountpoints.push(mountpoint.clone());
        }
    }

    if let Some(values) = &device.mountpoints {
        for value in values.iter().flatten() {
            if !value.is_empty() && !mountpoints.contains(value) {
                mountpoints.push(value.clone());
            }
        }
    }

    mountpoints
}

trait DeviceNameMatch {
    fn name_matches(&self, prefix: &str) -> bool;
}

impl DeviceNameMatch for LsblkDevice {
    fn name_matches(&self, prefix: &str) -> bool {
        self.name
            .as_ref()
            .or(self.kname.as_ref())
            .or(self.path.as_ref())
            .is_some_and(|name| {
                name.starts_with(prefix) || name.starts_with(&format!("/dev/{prefix}"))
            })
    }
}

#[cfg(test)]
mod tests {
    use disk_nix_model::{NodeKind, Relationship};

    use super::*;

    const FIXTURE: &[u8] = br#"
{
  "blockdevices": [
    {
      "name": "nvme0n1",
      "path": "/dev/nvme0n1",
      "type": "disk",
      "size": 1024,
      "tran": "nvme",
      "serial": "SERIAL",
      "wwn": "eui.1234",
      "children": [
        {
          "name": "nvme0n1p1",
          "path": "/dev/nvme0n1p1",
          "type": "part",
          "size": 512,
          "fstype": "vfat",
          "fsver": "FAT32",
          "label": "EFI",
          "uuid": "AAAA-BBBB",
          "partuuid": "part-1",
          "mountpoints": ["/boot"]
        },
        {
          "name": "cryptroot",
          "path": "/dev/mapper/cryptroot",
          "type": "crypt",
          "size": 512,
          "children": [
            {
              "name": "vg-root",
              "path": "/dev/mapper/vg-root",
              "type": "lvm",
              "size": 512,
              "fstype": "xfs",
              "uuid": "root-fs",
              "fsused": 128,
              "fsavail": 384,
              "mountpoint": "/"
            }
          ]
        }
      ]
    }
  ]
}
"#;

    #[test]
    fn normalizes_lsblk_fixture_into_storage_graph() {
        let graph = normalize_lsblk_json(FIXTURE).expect("fixture should parse");

        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::NvmeNamespace
                    && node.path.as_deref() == Some("/dev/nvme0n1"))
        );
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::Filesystem && node.name == "xfs")
        );
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::Mountpoint && node.name == "/")
        );
        assert!(
            graph
                .edges
                .iter()
                .any(|edge| edge.relationship == Relationship::MountedAt)
        );
    }
}
