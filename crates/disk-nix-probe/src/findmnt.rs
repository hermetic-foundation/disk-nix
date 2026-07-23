use disk_nix_model::{Edge, Node, NodeKind, Relationship, StorageGraph};
use serde::Deserialize;

use crate::{ProbeError, ProbeReport, ProbeStatus};

#[derive(Debug, Deserialize)]
struct FindmntDocument {
    filesystems: Vec<FindmntFilesystem>,
}

#[derive(Debug, Deserialize)]
struct FindmntFilesystem {
    target: String,
    source: Option<String>,
    fstype: Option<String>,
    options: Option<String>,
    size: Option<u64>,
    used: Option<u64>,
    avail: Option<u64>,
    children: Option<Vec<FindmntFilesystem>>,
}

pub fn normalize_findmnt_json(bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let document: FindmntDocument = serde_json::from_slice(bytes)
        .map_err(|error| ProbeError::Adapter(format!("failed to parse findmnt JSON: {error}")))?;
    let mut graph = StorageGraph::empty();

    for filesystem in &document.filesystems {
        add_filesystem(&mut graph, filesystem, None);
    }

    Ok(graph)
}

pub fn available_report(node_count: usize) -> ProbeReport {
    ProbeReport {
        adapter: "findmnt".to_string(),
        status: ProbeStatus::Available,
        message: Some(format!(
            "normalized {node_count} graph nodes from findmnt JSON"
        )),
    }
}

fn add_filesystem(
    graph: &mut StorageGraph,
    filesystem: &FindmntFilesystem,
    parent_mount_id: Option<String>,
) {
    let mount_id = mount_id(&filesystem.target);
    let mut mount = Node::new(
        mount_id.clone(),
        mount_kind(filesystem),
        filesystem.target.clone(),
    );

    if let Some(fstype) = &filesystem.fstype {
        mount = mount.with_property("filesystem.type", fstype.clone());
    }
    if let Some(options) = &filesystem.options {
        mount = mount.with_property("mount.options", options.clone());
    }
    if let Some(source) = &filesystem.source {
        mount = mount.with_property("mount.source", source.clone());
    }
    for (key, value) in mount_option_properties(filesystem) {
        mount = mount.with_property(key, value);
    }
    if let Some(size) = filesystem.size {
        mount = mount.with_size_bytes(size);
    }

    let usage = disk_nix_model::Usage {
        used_bytes: filesystem.used,
        free_bytes: filesystem.avail,
        allocated_bytes: None,
    };
    if !usage.is_empty() {
        mount = mount.with_usage(usage);
    }

    graph.add_node(mount);

    if let Some(parent_id) = parent_mount_id {
        graph.add_edge(Edge::new(
            parent_id,
            mount_id.clone(),
            Relationship::Contains,
        ));
    }

    if let Some(source) = &filesystem.source {
        add_source(graph, filesystem, source, &mount_id);
    }

    if let Some(children) = &filesystem.children {
        for child in children {
            add_filesystem(graph, child, Some(mount_id.clone()));
        }
    }
}

fn add_source(
    graph: &mut StorageGraph,
    filesystem: &FindmntFilesystem,
    source: &str,
    mount_id: &str,
) {
    let kind = source_kind(filesystem, source);
    let source_id = source_id(kind, source);
    let mut node = Node::new(source_id.clone(), kind, source.to_string());

    if source.starts_with('/') {
        node = node.with_path(source.to_string());
    }
    if let Some(fstype) = &filesystem.fstype {
        node = node.with_property("filesystem.type", fstype.clone());
    }

    graph.add_node(node);
    graph.add_edge(Edge::new(
        source_id,
        mount_id.to_string(),
        Relationship::MountedAt,
    ));
}

fn mount_kind(filesystem: &FindmntFilesystem) -> NodeKind {
    match filesystem.fstype.as_deref() {
        Some("nfs" | "nfs4") => NodeKind::NfsMount,
        _ => NodeKind::Mountpoint,
    }
}

fn source_kind(filesystem: &FindmntFilesystem, source: &str) -> NodeKind {
    match filesystem.fstype.as_deref() {
        Some("nfs" | "nfs4") => NodeKind::NfsExport,
        Some("bcachefs") => NodeKind::BcachefsFilesystem,
        _ if source.starts_with("/dev/") => NodeKind::Filesystem,
        _ => NodeKind::Filesystem,
    }
}

fn mount_id(target: &str) -> String {
    format!("mount:{target}")
}

fn source_id(kind: NodeKind, source: &str) -> String {
    match kind {
        NodeKind::NfsExport => format!("nfs-export:{source}"),
        _ if source.starts_with("/dev/") => format!("fs-source:{source}"),
        _ => format!("fs-source:{source}"),
    }
}

fn mount_option_properties(filesystem: &FindmntFilesystem) -> Vec<(String, String)> {
    let mut properties = Vec::new();
    let Some(options) = &filesystem.options else {
        return properties;
    };

    for option in parse_options(options) {
        match option.name.as_str() {
            "ro" => properties.push(("mount.read-only".to_string(), "true".to_string())),
            "rw" => properties.push(("mount.read-write".to_string(), "true".to_string())),
            "bind" | "rbind" => properties.push(("mount.bind".to_string(), "true".to_string())),
            "private" | "rprivate" | "shared" | "rshared" | "slave" | "rslave" | "unbindable"
            | "runbindable" => properties.push((
                "mount.propagation".to_string(),
                option.name.trim_start_matches('r').to_string(),
            )),
            _ if option.name.starts_with("shared:")
                || option.name.starts_with("master:")
                || option.name.starts_with("propagate_from:") =>
            {
                properties.push(("mount.propagation.id".to_string(), option.name.clone()));
            }
            _ => {}
        }

        if filesystem.fstype.as_deref() == Some("tmpfs")
            && matches!(
                option.name.as_str(),
                "size" | "nr_inodes" | "nr-inodes" | "mode" | "uid" | "gid" | "mpol"
            )
        {
            if let Some(value) = &option.value {
                properties.push((
                    format!("tmpfs.{}", option.name.replace('_', "-")),
                    value.clone(),
                ));
            }
        }

        if filesystem.fstype.as_deref() == Some("overlay")
            && matches!(
                option.name.as_str(),
                "lowerdir"
                    | "upperdir"
                    | "workdir"
                    | "index"
                    | "metacopy"
                    | "redirect_dir"
                    | "xino"
                    | "uuid"
            )
        {
            if let Some(value) = &option.value {
                properties.push((format!("overlay.{}", option.name), value.clone()));
            }
        }
    }

    properties
}

fn parse_options(options: &str) -> Vec<MountOption> {
    options
        .split(',')
        .filter(|option| !option.is_empty())
        .map(|option| {
            if let Some((name, value)) = option.split_once('=') {
                MountOption {
                    name: name.to_string(),
                    value: Some(value.to_string()),
                }
            } else {
                MountOption {
                    name: option.to_string(),
                    value: None,
                }
            }
        })
        .collect()
}

struct MountOption {
    name: String,
    value: Option<String>,
}

#[cfg(test)]
mod tests {
    use disk_nix_model::{NodeKind, Relationship};

    use super::*;

    const FIXTURE: &[u8] = br#"
{
  "filesystems": [
    {
      "target": "/",
      "source": "/dev/mapper/vg-root",
      "fstype": "xfs",
      "options": "rw,relatime",
      "size": 1000,
      "used": 400,
      "avail": 600,
      "children": [
        {
          "target": "/mnt/share",
          "source": "storage.example:/export/share",
          "fstype": "nfs4",
          "options": "rw,vers=4.2"
        },
        {
          "target": "/run",
          "source": "tmpfs",
          "fstype": "tmpfs",
          "options": "rw,nosuid,nodev,mode=755,size=16777216,nr_inodes=4096"
        },
        {
          "target": "/srv/bind",
          "source": "/srv/source",
          "fstype": "none",
          "options": "rw,bind"
        },
        {
          "target": "/merged",
          "source": "overlay",
          "fstype": "overlay",
          "options": "rw,lowerdir=/lower:/lower2,upperdir=/upper,workdir=/work,index=off"
        }
      ]
    }
  ]
}
"#;

    #[test]
    fn normalizes_mounts_and_nfs_exports() {
        let graph = normalize_findmnt_json(FIXTURE).expect("fixture should parse");

        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::Mountpoint && node.name == "/")
        );
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::NfsMount && node.name == "/mnt/share")
        );
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::NfsExport && node.name == "storage.example:/export/share"
        }));
        assert!(
            graph
                .edges
                .iter()
                .any(|edge| edge.relationship == Relationship::MountedAt)
        );

        let run = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "mount:/run")
            .expect("tmpfs mount should exist");
        assert!(has_property(run, "tmpfs.size", "16777216"));
        assert!(has_property(run, "tmpfs.nr-inodes", "4096"));

        let bind = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "mount:/srv/bind")
            .expect("bind mount should exist");
        assert!(has_property(bind, "mount.bind", "true"));

        let bind_source = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "fs-source:/srv/source")
            .expect("bind source should exist");
        assert_eq!(bind_source.path.as_deref(), Some("/srv/source"));

        let overlay = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "mount:/merged")
            .expect("overlay mount should exist");
        assert!(has_property(overlay, "overlay.lowerdir", "/lower:/lower2"));
        assert!(has_property(overlay, "overlay.upperdir", "/upper"));
        assert!(has_property(overlay, "overlay.workdir", "/work"));
    }

    fn has_property(node: &disk_nix_model::Node, key: &str, value: &str) -> bool {
        node.properties
            .iter()
            .any(|property| property.key == key && property.value == value)
    }
}
