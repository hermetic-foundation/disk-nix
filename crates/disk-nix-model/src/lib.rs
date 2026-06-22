use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
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
        self.nodes.push(node);
    }

    pub fn add_edge(&mut self, edge: Edge) {
        self.edges.push(edge);
    }

    #[must_use]
    pub fn to_json(&self) -> String {
        let mut out = String::from("{\"nodes\":[");
        for (index, node) in self.nodes.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push_str(&node.to_json());
        }
        out.push_str("],\"edges\":[");
        for (index, edge) in self.edges.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push_str(&edge.to_json());
        }
        out.push_str("]}");
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    pub name: String,
    pub path: Option<String>,
    pub size_bytes: Option<u64>,
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
    pub fn with_property(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties.push(Property {
            key: key.into(),
            value: value.into(),
        });
        self
    }

    fn to_json(&self) -> String {
        let mut fields = vec![
            format!("\"id\":\"{}\"", escape_json(&self.id.0)),
            format!("\"kind\":\"{}\"", self.kind),
            format!("\"name\":\"{}\"", escape_json(&self.name)),
        ];

        if let Some(path) = &self.path {
            fields.push(format!("\"path\":\"{}\"", escape_json(path)));
        }
        if let Some(size_bytes) = self.size_bytes {
            fields.push(format!("\"sizeBytes\":{size_bytes}"));
        }
        if let Some(usage) = &self.usage {
            fields.push(format!("\"usage\":{}", usage.to_json()));
        }
        fields.push(format!("\"identity\":{}", self.identity.to_json()));
        fields.push(format!(
            "\"properties\":{}",
            properties_to_json(&self.properties)
        ));

        format!("{{{}}}", fields.join(","))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    LvmThinPool,
    LvmSnapshot,
    LvmCache,
    VdoVolume,
    MdRaid,
    BtrfsFilesystem,
    BtrfsSubvolume,
    BtrfsSnapshot,
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
            Self::LvmThinPool => "lvm-thin-pool",
            Self::LvmSnapshot => "lvm-snapshot",
            Self::LvmCache => "lvm-cache",
            Self::VdoVolume => "vdo-volume",
            Self::MdRaid => "md-raid",
            Self::BtrfsFilesystem => "btrfs-filesystem",
            Self::BtrfsSubvolume => "btrfs-subvolume",
            Self::BtrfsSnapshot => "btrfs-snapshot",
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
        };
        f.write_str(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

    fn to_json(&self) -> String {
        format!(
            "{{\"from\":\"{}\",\"to\":\"{}\",\"relationship\":\"{}\"}}",
            escape_json(&self.from.0),
            escape_json(&self.to.0),
            self.relationship
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Identity {
    pub uuid: Option<String>,
    pub partuuid: Option<String>,
    pub label: Option<String>,
    pub serial: Option<String>,
    pub wwn: Option<String>,
}

impl Identity {
    fn to_json(&self) -> String {
        let mut fields = Vec::new();
        if let Some(uuid) = &self.uuid {
            fields.push(format!("\"uuid\":\"{}\"", escape_json(uuid)));
        }
        if let Some(partuuid) = &self.partuuid {
            fields.push(format!("\"partuuid\":\"{}\"", escape_json(partuuid)));
        }
        if let Some(label) = &self.label {
            fields.push(format!("\"label\":\"{}\"", escape_json(label)));
        }
        if let Some(serial) = &self.serial {
            fields.push(format!("\"serial\":\"{}\"", escape_json(serial)));
        }
        if let Some(wwn) = &self.wwn {
            fields.push(format!("\"wwn\":\"{}\"", escape_json(wwn)));
        }
        format!("{{{}}}", fields.join(","))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Usage {
    pub used_bytes: Option<u64>,
    pub free_bytes: Option<u64>,
    pub allocated_bytes: Option<u64>,
}

impl Usage {
    fn to_json(&self) -> String {
        let mut fields = Vec::new();
        if let Some(used_bytes) = self.used_bytes {
            fields.push(format!("\"usedBytes\":{used_bytes}"));
        }
        if let Some(free_bytes) = self.free_bytes {
            fields.push(format!("\"freeBytes\":{free_bytes}"));
        }
        if let Some(allocated_bytes) = self.allocated_bytes {
            fields.push(format!("\"allocatedBytes\":{allocated_bytes}"));
        }
        format!("{{{}}}", fields.join(","))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    pub key: String,
    pub value: String,
}

fn properties_to_json(properties: &[Property]) -> String {
    let mut out = String::from("[");
    for (index, property) in properties.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"key\":\"{}\",\"value\":\"{}\"}}",
            escape_json(&property.key),
            escape_json(&property.value)
        ));
    }
    out.push(']');
    out
}

fn escape_json(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            other => vec![other],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_empty_graph_json() {
        assert_eq!(
            StorageGraph::empty().to_json(),
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

        assert!(graph.to_json().contains("\"name\":\"disk \\\"0\\\"\""));
        assert!(graph.to_json().contains("\"sizeBytes\":1024"));
    }
}
