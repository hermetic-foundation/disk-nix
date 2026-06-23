use disk_nix_model::{Edge, Node, NodeKind, Relationship, StorageGraph};

use crate::ProbeError;

#[derive(Debug, Clone, PartialEq, Eq)]
struct NfsMount {
    source: String,
    target: String,
    server: Option<String>,
    export: Option<String>,
    options: Vec<(String, String)>,
}

pub fn normalize_nfsstat_mounts(bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let mounts = parse_mounts(bytes)?;
    let mut graph = StorageGraph::empty();

    for mount in mounts {
        add_mount(&mut graph, mount);
    }

    Ok(graph)
}

fn parse_mounts(bytes: &[u8]) -> Result<Vec<NfsMount>, ProbeError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| ProbeError::Adapter(format!("failed to read nfsstat output: {error}")))?;
    let mut mounts = Vec::new();
    let mut current: Option<NfsMount> = None;

    for line in text.lines() {
        if let Some((source, target)) = parse_header(line) {
            if let Some(mount) = current.take() {
                mounts.push(mount);
            }
            let (server, export) = split_source(&source);
            current = Some(NfsMount {
                source,
                target,
                server,
                export,
                options: Vec::new(),
            });
        } else if let Some(mount) = &mut current {
            mount.options.extend(parse_options(line));
        }
    }

    if let Some(mount) = current {
        mounts.push(mount);
    }

    Ok(mounts)
}

fn add_mount(graph: &mut StorageGraph, mount: NfsMount) {
    let mount_id = format!("mount:{}", mount.target);
    let mut mount_node = Node::new(mount_id.clone(), NodeKind::NfsMount, mount.target.clone());
    mount_node = mount_node.with_property("nfs.source", mount.source.clone());
    for (key, value) in &mount.options {
        mount_node = mount_node.with_property(format!("nfs.{key}"), value.clone());
    }
    if let Some(server) = &mount.server {
        mount_node = mount_node.with_property("nfs.server", server.clone());
    }
    if let Some(export) = &mount.export {
        mount_node = mount_node.with_property("nfs.export", export.clone());
    }
    graph.add_node(mount_node);

    let export_id = format!("nfs-export:{}", mount.source);
    let mut export_node = Node::new(export_id.clone(), NodeKind::NfsExport, mount.source.clone());
    if let Some(server) = mount.server {
        export_node = export_node.with_property("nfs.server", server);
    }
    if let Some(export) = mount.export {
        export_node = export_node.with_property("nfs.export", export);
    }
    graph.add_node(export_node);
    graph.add_edge(Edge::new(export_id, mount_id, Relationship::MountedAt));
}

fn parse_header(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if let Some((source, target)) = trimmed.split_once(" mounted on ") {
        return Some((source.to_string(), target.trim_end_matches(':').to_string()));
    }

    let (target, source) = trimmed.split_once(" from ")?;
    Some((source.trim_end_matches(':').to_string(), target.to_string()))
}

fn split_source(source: &str) -> (Option<String>, Option<String>) {
    source
        .split_once(':')
        .map_or((None, Some(source.to_string())), |(server, export)| {
            (Some(server.to_string()), Some(export.to_string()))
        })
}

fn parse_options(line: &str) -> Vec<(String, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    if let Some((label, values)) = trimmed.split_once(':') {
        let label = normalize_key(label);
        let values = values.trim();
        if values.is_empty() {
            return Vec::new();
        }
        if !values.contains(',') {
            return vec![(label, values.to_string())];
        }
        return parse_option_list(values);
    }

    parse_option_list(trimmed)
}

fn parse_option_list(value: &str) -> Vec<(String, String)> {
    value
        .split(',')
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() || part.ends_with(':') {
                return None;
            }
            Some(part.split_once('=').map_or_else(
                || (normalize_key(part), "true".to_string()),
                |(key, value)| (normalize_key(key), value.to_string()),
            ))
        })
        .collect()
}

fn normalize_key(key: &str) -> String {
    key.trim()
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

#[cfg(test)]
mod tests {
    use disk_nix_model::NodeKind;

    use super::*;

    const NFSSTAT: &[u8] = br#"
storage.example:/export/home mounted on /home:
   Flags: rw,relatime,vers=4.2,rsize=1048576,wsize=1048576,namlen=255,hard,proto=tcp,timeo=600,retrans=2,sec=sys,clientaddr=10.0.0.20,local_lock=none,addr=10.0.0.10,port=2049,mountaddr=10.0.0.10,mountvers=3,mountproto=tcp,lookupcache=positive,fsc
   Age: 123

/mnt/backups from 10.0.0.11:/srv/backups
   Options: ro,vers=3,proto=tcp,addr=10.0.0.11,local_lock=all
"#;

    #[test]
    fn normalizes_nfsstat_mount_metadata() {
        let graph = normalize_nfsstat_mounts(NFSSTAT).expect("fixture should parse");

        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::NfsMount && node.name == "/home")
        );
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::NfsExport
                    && node.name == "10.0.0.11:/srv/backups")
        );
        assert!(graph.nodes.iter().any(|node| {
            node.name == "/home"
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "nfs.vers" && property.value == "4.2")
                && node.properties.iter().any(|property| {
                    property.key == "nfs.source" && property.value == "storage.example:/export/home"
                })
                && node.properties.iter().any(|property| {
                    property.key == "nfs.export" && property.value == "/export/home"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "nfs.fsc" && property.value == "true")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "nfs.age" && property.value == "123")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "nfs.local-lock" && property.value == "none")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "nfs.mountproto" && property.value == "tcp")
        }));
        assert!(graph.nodes.iter().any(|node| {
            node.name == "/mnt/backups"
                && node.properties.iter().any(|property| {
                    property.key == "nfs.source" && property.value == "10.0.0.11:/srv/backups"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "nfs.local-lock" && property.value == "all")
        }));
    }
}
