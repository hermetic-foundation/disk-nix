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

#[derive(Debug, Clone, PartialEq, Eq)]
struct NfsExport {
    path: String,
    client: String,
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

pub fn normalize_exportfs_verbose(bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let exports = parse_exports(bytes)?;
    let mut graph = StorageGraph::empty();

    for export in exports {
        add_export(&mut graph, export);
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

fn parse_exports(bytes: &[u8]) -> Result<Vec<NfsExport>, ProbeError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| ProbeError::Adapter(format!("failed to read exportfs output: {error}")))?;
    let mut exports = Vec::new();
    let mut current_path: Option<String> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let first_is_whitespace = line
            .chars()
            .next()
            .is_some_and(|character| character.is_whitespace());
        if !first_is_whitespace {
            current_path = trimmed.split_whitespace().next().map(ToOwned::to_owned);
            if let Some(export) = parse_inline_export(trimmed) {
                exports.push(export);
            }
            continue;
        }

        if let (Some(path), Some((client, options))) =
            (&current_path, parse_client_options(trimmed))
        {
            exports.push(NfsExport {
                path: path.clone(),
                client,
                options,
            });
        }
    }

    Ok(exports)
}

fn parse_inline_export(value: &str) -> Option<NfsExport> {
    let (path, rest) = value.split_once(char::is_whitespace)?;
    let (client, options) = parse_client_options(rest.trim())?;
    Some(NfsExport {
        path: path.to_string(),
        client,
        options,
    })
}

fn parse_client_options(value: &str) -> Option<(String, Vec<(String, String)>)> {
    let (client, options) = value.split_once('(')?;
    let options = options.trim_end_matches(')').trim();
    Some((client.trim().to_string(), parse_option_list(options)))
        .filter(|(client, _)| !client.is_empty())
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

fn add_export(graph: &mut StorageGraph, export: NfsExport) {
    let export_id = format!("nfs-export:{}:{}", export.path, export.client);
    let mut export_node = Node::new(export_id, NodeKind::NfsExport, export.path.clone())
        .with_property("nfs.export", export.path)
        .with_property("nfs.export-client", export.client)
        .with_property("nfs.exportfs", "true");

    for (key, value) in export.options {
        export_node = export_node.with_property(format!("nfs.export-option-{key}"), value);
    }

    graph.add_node(export_node);
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
   Caps: caps=0x3fffdf,wtmult=512,dtsize=32768,bsize=0
   Sec: flavor=1,pseudoflavor=1
   Age: 123

/mnt/backups from 10.0.0.11:/srv/backups
   Options: ro,vers=3,proto=tcp,addr=10.0.0.11,local_lock=all
"#;

    const EXPORTFS: &[u8] = br#"
/srv/share
        192.0.2.0/24(sync,wdelay,hide,no_subtree_check,sec=sys,rw,secure,root_squash,no_all_squash)
/srv/read-only 198.51.100.10(ro,sync,no_subtree_check,fsid=12)
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
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "nfs.caps" && property.value == "0x3fffdf")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "nfs.wtmult" && property.value == "512")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "nfs.dtsize" && property.value == "32768")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "nfs.bsize" && property.value == "0")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "nfs.flavor" && property.value == "1")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "nfs.pseudoflavor" && property.value == "1")
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

    #[test]
    fn normalizes_exportfs_verbose_metadata() {
        let graph = normalize_exportfs_verbose(EXPORTFS).expect("fixture should parse");

        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::NfsExport
                && node.name == "/srv/share"
                && node.properties.iter().any(|property| {
                    property.key == "nfs.export-client" && property.value == "192.0.2.0/24"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "nfs.exportfs" && property.value == "true")
                && node.properties.iter().any(|property| {
                    property.key == "nfs.export-option-rw" && property.value == "true"
                })
                && node.properties.iter().any(|property| {
                    property.key == "nfs.export-option-sec" && property.value == "sys"
                })
                && node.properties.iter().any(|property| {
                    property.key == "nfs.export-option-root-squash" && property.value == "true"
                })
        }));
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::NfsExport
                && node.name == "/srv/read-only"
                && node.properties.iter().any(|property| {
                    property.key == "nfs.export-client" && property.value == "198.51.100.10"
                })
                && node.properties.iter().any(|property| {
                    property.key == "nfs.export-option-fsid" && property.value == "12"
                })
                && node.properties.iter().any(|property| {
                    property.key == "nfs.export-option-ro" && property.value == "true"
                })
        }));
    }
}
