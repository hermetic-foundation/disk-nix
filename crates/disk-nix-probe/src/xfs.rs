use disk_nix_model::{Edge, Node, NodeKind, Relationship, StorageGraph, Usage};

use crate::ProbeError;

pub fn normalize_xfs_info(target: &str, bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let info = parse_xfs_info(bytes)?;
    let mut graph = StorageGraph::empty();
    let mut mount = Node::new(
        format!("mount:{target}"),
        NodeKind::Mountpoint,
        target.to_string(),
    )
    .with_property("filesystem.type", "xfs");

    let size_bytes = if let (Some(blocks), Some(block_size)) = (
        info.property("data", "blocks").and_then(parse_u64),
        info.property("data", "bsize").and_then(parse_u64),
    ) {
        Some(blocks.saturating_mul(block_size))
    } else {
        None
    };

    if let Some(size_bytes) = size_bytes {
        mount = mount.with_size_bytes(size_bytes).with_usage(Usage {
            used_bytes: None,
            free_bytes: None,
            allocated_bytes: Some(size_bytes),
        });
    }

    for (section, key, value) in &info.properties {
        mount = mount.with_property(format!("xfs.{section}.{key}"), value.clone());
    }

    graph.add_node(mount);

    if let Some(device) = info.property("meta-data", "meta-data") {
        let filesystem_id = format!("fs-source:{device}");
        let mut filesystem =
            Node::new(filesystem_id.clone(), NodeKind::Filesystem, "xfs").with_path(device);

        if let Some(size_bytes) = size_bytes {
            filesystem = filesystem.with_size_bytes(size_bytes).with_usage(Usage {
                used_bytes: None,
                free_bytes: None,
                allocated_bytes: Some(size_bytes),
            });
        }

        for (section, key, value) in &info.properties {
            filesystem = filesystem.with_property(format!("xfs.{section}.{key}"), value.clone());
        }
        filesystem = filesystem.with_property("filesystem.type", "xfs");

        graph.add_node(filesystem);
        graph.add_edge(Edge::new(
            filesystem_id,
            format!("mount:{target}"),
            Relationship::MountedAt,
        ));
    }

    Ok(graph)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct XfsInfo {
    properties: Vec<(String, String, String)>,
}

impl XfsInfo {
    fn property(&self, section: &str, key: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|(candidate_section, candidate_key, _)| {
                candidate_section == section && candidate_key == key
            })
            .map(|(_, _, value)| value.as_str())
    }
}

fn parse_xfs_info(bytes: &[u8]) -> Result<XfsInfo, ProbeError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| ProbeError::Adapter(format!("failed to read xfs_info output: {error}")))?;
    let mut current_section = String::new();
    let mut properties = Vec::new();

    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        if let Some(section) = section_name(line) {
            current_section = normalize_key(section);
        }
        if current_section.is_empty() {
            continue;
        }

        for token in line.split_whitespace() {
            let token = token.trim_end_matches(',');
            let Some((key, value)) = token.split_once('=') else {
                continue;
            };
            let key = normalize_key(key);
            let value = value.trim_matches(',').trim();
            if key.is_empty() || value.is_empty() {
                continue;
            }
            properties.push((current_section.clone(), key, value.to_string()));
        }
    }

    Ok(XfsInfo { properties })
}

fn section_name(line: &str) -> Option<&str> {
    let before_equals = line.split_once('=')?.0.trim();
    if before_equals.is_empty() {
        return None;
    }
    if before_equals.contains(char::is_whitespace) {
        before_equals.split_whitespace().next()
    } else {
        Some(before_equals)
    }
}

fn normalize_key(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|character| match character {
            'a'..='z' | '0'..='9' => character,
            _ => '-',
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn parse_u64(value: &str) -> Option<u64> {
    value.parse().ok()
}

#[cfg(test)]
mod tests {
    use disk_nix_model::Relationship;

    use super::*;

    const XFS_INFO: &[u8] = br#"
meta-data=/dev/mapper/vg-root  isize=512    agcount=4, agsize=65536 blks
         =                       sectsz=512   attr=2, projid32bit=1
         =                       crc=1        finobt=1, sparse=1, rmapbt=0
         =                       reflink=1    bigtime=1 inobtcount=1 nrext64=0
data     =                       bsize=4096   blocks=262144, imaxpct=25
         =                       sunit=0      swidth=0 blks
naming   =version 2              bsize=4096   ascii-ci=0, ftype=1
log      =internal log           bsize=4096   blocks=2560, version=2
         =                       sectsz=512   sunit=0 blks, lazy-count=1
realtime =none                   extsz=4096   blocks=0, rtextents=0
"#;

    #[test]
    fn normalizes_xfs_info_metadata() {
        let graph = normalize_xfs_info("/", XFS_INFO).expect("fixture should parse");
        let mount = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "mount:/")
            .expect("mount node should exist");

        assert_eq!(mount.kind, NodeKind::Mountpoint);
        assert_eq!(mount.name, "/");
        assert_eq!(mount.size_bytes, Some(1_073_741_824));
        assert_eq!(
            mount.usage.as_ref().and_then(|usage| usage.allocated_bytes),
            Some(1_073_741_824)
        );
        assert!(
            mount.properties.iter().any(|property| {
                property.key == "xfs.meta-data.reflink" && property.value == "1"
            })
        );
        assert!(
            mount
                .properties
                .iter()
                .any(|property| { property.key == "xfs.data.bsize" && property.value == "4096" })
        );
        assert!(
            mount
                .properties
                .iter()
                .any(|property| { property.key == "xfs.log.blocks" && property.value == "2560" })
        );

        let filesystem = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "fs-source:/dev/mapper/vg-root")
            .expect("filesystem source node should exist");
        assert_eq!(filesystem.kind, NodeKind::Filesystem);
        assert_eq!(filesystem.path.as_deref(), Some("/dev/mapper/vg-root"));
        assert_eq!(filesystem.size_bytes, Some(1_073_741_824));
        assert!(
            filesystem.properties.iter().any(|property| {
                property.key == "xfs.meta-data.bigtime" && property.value == "1"
            })
        );
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "fs-source:/dev/mapper/vg-root"
                && edge.to.0 == "mount:/"
                && edge.relationship == Relationship::MountedAt
        }));
    }
}
