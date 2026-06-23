use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageGraph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl StorageGraph {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: Node) {
        if let Some(existing) = self
            .nodes
            .iter_mut()
            .find(|existing| existing.id == node.id)
        {
            existing.merge(node);
        } else {
            self.nodes.push(node);
        }
    }

    pub fn add_edge(&mut self, edge: Edge) {
        if !self.edges.contains(&edge) {
            self.edges.push(edge);
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    #[must_use]
    pub fn find_nodes(&self, query: &str) -> Vec<&Node> {
        self.nodes
            .iter()
            .filter(|node| node.matches(query))
            .collect()
    }

    #[must_use]
    pub fn related_edges(&self, node_id: &NodeId) -> Vec<&Edge> {
        self.edges
            .iter()
            .filter(|edge| &edge.from == node_id || &edge.to == node_id)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    pub identity: Identity,
    pub properties: Vec<Property>,
}

impl Node {
    #[must_use]
    pub fn new(id: impl Into<String>, kind: NodeKind, name: impl Into<String>) -> Self {
        Self {
            id: NodeId(id.into()),
            kind,
            name: name.into(),
            path: None,
            size_bytes: None,
            usage: None,
            identity: Identity::default(),
            properties: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    #[must_use]
    pub fn with_size_bytes(mut self, size_bytes: u64) -> Self {
        self.size_bytes = Some(size_bytes);
        self
    }

    #[must_use]
    pub fn with_usage(mut self, usage: Usage) -> Self {
        self.usage = Some(usage);
        self
    }

    #[must_use]
    pub fn with_identity(mut self, identity: Identity) -> Self {
        self.identity = identity;
        self
    }

    #[must_use]
    pub fn with_property(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties.push(Property {
            key: key.into(),
            value: value.into(),
        });
        self
    }

    #[must_use]
    pub fn matches(&self, query: &str) -> bool {
        self.id.0 == query
            || self.name == query
            || self.path.as_deref() == Some(query)
            || self.identity.uuid.as_deref() == Some(query)
            || self.identity.partuuid.as_deref() == Some(query)
            || self.identity.label.as_deref() == Some(query)
            || self.identity.serial.as_deref() == Some(query)
            || self.identity.wwn.as_deref() == Some(query)
            || self
                .properties
                .iter()
                .any(|property| property.value == query || property.key == query)
    }

    fn merge(&mut self, other: Node) {
        if self.path.is_none() {
            self.path = other.path;
        }
        if self.size_bytes.is_none() {
            self.size_bytes = other.size_bytes;
        }
        if self.usage.is_none() {
            self.usage = other.usage;
        }

        self.identity.merge(other.identity);

        for property in other.properties {
            if !self.properties.contains(&property) {
                self.properties.push(property);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct NodeId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NodeKind {
    PhysicalDisk,
    Partition,
    Filesystem,
    Mountpoint,
    LuksContainer,
    DeviceMapper,
    LvmPhysicalVolume,
    LvmVolumeGroup,
    LvmLogicalVolume,
    LvmSegment,
    LvmThinPool,
    LvmSnapshot,
    LvmCache,
    VdoVolume,
    MdRaid,
    BtrfsFilesystem,
    BtrfsSubvolume,
    BtrfsSnapshot,
    BtrfsQgroup,
    BcachefsFilesystem,
    BcachefsDevice,
    ZfsPool,
    ZfsVdev,
    ZfsDataset,
    ZfsSnapshot,
    Zvol,
    IscsiSession,
    IscsiTarget,
    Lun,
    NfsExport,
    NfsMount,
    CacheDevice,
    MultipathDevice,
    NvmeNamespace,
    Swap,
    LoopDevice,
    BackingFile,
}

impl fmt::Display for NodeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::PhysicalDisk => "physical-disk",
            Self::Partition => "partition",
            Self::Filesystem => "filesystem",
            Self::Mountpoint => "mountpoint",
            Self::LuksContainer => "luks-container",
            Self::DeviceMapper => "device-mapper",
            Self::LvmPhysicalVolume => "lvm-physical-volume",
            Self::LvmVolumeGroup => "lvm-volume-group",
            Self::LvmLogicalVolume => "lvm-logical-volume",
            Self::LvmSegment => "lvm-segment",
            Self::LvmThinPool => "lvm-thin-pool",
            Self::LvmSnapshot => "lvm-snapshot",
            Self::LvmCache => "lvm-cache",
            Self::VdoVolume => "vdo-volume",
            Self::MdRaid => "md-raid",
            Self::BtrfsFilesystem => "btrfs-filesystem",
            Self::BtrfsSubvolume => "btrfs-subvolume",
            Self::BtrfsSnapshot => "btrfs-snapshot",
            Self::BtrfsQgroup => "btrfs-qgroup",
            Self::BcachefsFilesystem => "bcachefs-filesystem",
            Self::BcachefsDevice => "bcachefs-device",
            Self::ZfsPool => "zfs-pool",
            Self::ZfsVdev => "zfs-vdev",
            Self::ZfsDataset => "zfs-dataset",
            Self::ZfsSnapshot => "zfs-snapshot",
            Self::Zvol => "zvol",
            Self::IscsiSession => "iscsi-session",
            Self::IscsiTarget => "iscsi-target",
            Self::Lun => "lun",
            Self::NfsExport => "nfs-export",
            Self::NfsMount => "nfs-mount",
            Self::CacheDevice => "cache-device",
            Self::MultipathDevice => "multipath-device",
            Self::NvmeNamespace => "nvme-namespace",
            Self::Swap => "swap",
            Self::LoopDevice => "loop-device",
            Self::BackingFile => "backing-file",
        };
        f.write_str(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub relationship: Relationship,
}

impl Edge {
    #[must_use]
    pub fn new(from: impl Into<String>, to: impl Into<String>, relationship: Relationship) -> Self {
        Self {
            from: NodeId(from.into()),
            to: NodeId(to.into()),
            relationship,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Relationship {
    Contains,
    Backs,
    MapsTo,
    MemberOf,
    MountedAt,
    SnapshotOf,
    CacheFor,
    ImportedFrom,
    DependsOn,
    Exports,
}

impl fmt::Display for Relationship {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Contains => "contains",
            Self::Backs => "backs",
            Self::MapsTo => "maps-to",
            Self::MemberOf => "member-of",
            Self::MountedAt => "mounted-at",
            Self::SnapshotOf => "snapshot-of",
            Self::CacheFor => "cache-for",
            Self::ImportedFrom => "imported-from",
            Self::DependsOn => "depends-on",
            Self::Exports => "exports",
        };
        f.write_str(value)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partuuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wwn: Option<String>,
}

impl Identity {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.uuid.is_none()
            && self.partuuid.is_none()
            && self.label.is_none()
            && self.serial.is_none()
            && self.wwn.is_none()
    }

    fn merge(&mut self, other: Identity) {
        if self.uuid.is_none() {
            self.uuid = other.uuid;
        }
        if self.partuuid.is_none() {
            self.partuuid = other.partuuid;
        }
        if self.label.is_none() {
            self.label = other.label;
        }
        if self.serial.is_none() {
            self.serial = other.serial;
        }
        if self.wwn.is_none() {
            self.wwn = other.wwn;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub free_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allocated_bytes: Option<u64>,
}

impl Usage {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            used_bytes: None,
            free_bytes: None,
            allocated_bytes: None,
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.used_bytes.is_none() && self.free_bytes.is_none() && self.allocated_bytes.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Property {
    pub key: String,
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_empty_graph_json() {
        assert_eq!(
            StorageGraph::empty().to_json().expect("json should render"),
            "{\"nodes\":[],\"edges\":[]}"
        );
    }

    #[test]
    fn renders_node_json_with_escaped_values() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("disk:0", NodeKind::PhysicalDisk, "disk \"0\"")
                .with_path("/dev/sda")
                .with_size_bytes(1024),
        );

        let json = graph.to_json().expect("json should render");

        assert!(json.contains("\"name\":\"disk \\\"0\\\"\""));
        assert!(json.contains("\"sizeBytes\":1024"));
    }

    #[test]
    fn finds_nodes_by_identity_and_path() {
        let mut graph = StorageGraph::empty();
        let mut node = Node::new("disk:0", NodeKind::PhysicalDisk, "disk0").with_path("/dev/sda");
        node.identity.uuid = Some("uuid-0".to_string());
        graph.add_node(node);

        assert_eq!(graph.find_nodes("/dev/sda").len(), 1);
        assert_eq!(graph.find_nodes("uuid-0").len(), 1);
        assert!(graph.find_nodes("missing").is_empty());
    }

    #[test]
    fn merges_duplicate_nodes_by_id() {
        let mut graph = StorageGraph::empty();
        graph.add_node(Node::new("mount:/", NodeKind::Mountpoint, "/"));
        graph.add_node(
            Node::new("mount:/", NodeKind::Mountpoint, "/")
                .with_property("filesystem.type", "ext4"),
        );

        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.nodes[0].properties.len(), 1);
    }
}
