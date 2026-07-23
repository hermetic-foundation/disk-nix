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
    let mut mount_node = Node::new(mount_id.clone(), NodeKind::NfsMount, mount.target.clone())
        .with_path(mount.target.clone());
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
        .with_path(export.path.clone())
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
    if let Some((server, export)) = split_bracketed_source(source) {
        return (Some(server.to_string()), Some(export.to_string()));
    }

    source
        .split_once(':')
        .map_or((None, Some(source.to_string())), |(server, export)| {
            (Some(server.to_string()), Some(export.to_string()))
        })
}

fn split_bracketed_source(source: &str) -> Option<(&str, &str)> {
    let rest = source.strip_prefix('[')?;
    let (server, export) = rest.split_once("]:")?;
    (!server.is_empty() && !export.is_empty()).then_some((server, export))
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

    const NFS_COMPLEX_MOUNTS: &[u8] = br#"
nas-ref.example:/referrals/projects mounted on /mnt/referral:
   Flags: rw,relatime,vers=4.2,minorversion=2,proto=tcp,sec=krb5i,clientaddr=10.44.0.20,addr=10.44.0.10,port=2049,local_lock=none,lookupcache=positive,referral=true,replicas=nas-a:/exports/projects;nas-b:/exports/projects
   Options: pnfs,layout=nfs4-files,max_connect=4,migration=enabled
   Age: 10
nas-ref.example:/exports/projects mounted on /mnt/referral:
   Flags: ro,relatime,vers=4.2,minorversion=2,proto=tcp,sec=krb5p,clientaddr=10.44.0.20,addr=10.44.0.10,port=2049,local_lock=none,lookupcache=none,noac
   Options: remount,lookupcache=none,sec=krb5p
   Age: 42
nas-pnfs.example:/exports/media mounted on /mnt/media:
   Flags: rw,relatime,vers=4.1,minorversion=1,proto=tcp,sec=sys,clientaddr=10.44.0.21,addr=10.44.0.12,port=2049,local_lock=none
   Options: pnfs,layout=flexfiles,dsaddr=10.44.1.10,dsaddr2=10.44.1.11
   Age: 77
"#;

    const NFS_COMPLEX_EXPORTS: &[u8] = br#"
/exports/projects
        10.44.0.0/16(rw,sync,no_subtree_check,sec=krb5i,fsid=101,refer,replicas=nas-a:/exports/projects:nas-b:/exports/projects)
        10.44.0.0/16(ro,sync,no_subtree_check,sec=krb5p,fsid=101,crossmnt)
/exports/media *(rw,async,no_subtree_check,sec=sys,pnfs,fsid=202)
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
                && node.path.as_deref() == Some("/home")
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
                && node.path.as_deref() == Some("/mnt/backups")
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
    fn splits_bracketed_ipv6_nfs_sources() {
        let (server, export) = split_source("[2001:db8::10]:/srv/share");

        assert_eq!(server.as_deref(), Some("2001:db8::10"));
        assert_eq!(export.as_deref(), Some("/srv/share"));
    }

    #[test]
    fn normalizes_exportfs_verbose_metadata() {
        let graph = normalize_exportfs_verbose(EXPORTFS).expect("fixture should parse");

        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::NfsExport
                && node.name == "/srv/share"
                && node.path.as_deref() == Some("/srv/share")
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
                && node.path.as_deref() == Some("/srv/read-only")
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

    #[test]
    fn normalizes_referral_pnfs_remount_and_export_reload_fixture() {
        let mut graph = StorageGraph::empty();
        merge_test_graph(
            &mut graph,
            normalize_nfsstat_mounts(NFS_COMPLEX_MOUNTS)
                .expect("complex NFS mount fixture should parse"),
        );
        merge_test_graph(
            &mut graph,
            normalize_exportfs_verbose(NFS_COMPLEX_EXPORTS)
                .expect("complex NFS export fixture should parse"),
        );

        let remounted = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "mount:/mnt/referral")
            .expect("merged referral/remount node should exist");
        assert_eq!(remounted.kind, NodeKind::NfsMount);
        assert!(remounted.properties.iter().any(|property| {
            property.key == "nfs.source" && property.value == "nas-ref.example:/referrals/projects"
        }));
        assert!(remounted.properties.iter().any(|property| {
            property.key == "nfs.source" && property.value == "nas-ref.example:/exports/projects"
        }));
        assert!(
            remounted
                .properties
                .iter()
                .any(|property| { property.key == "nfs.sec" && property.value == "krb5i" })
        );
        assert!(
            remounted
                .properties
                .iter()
                .any(|property| { property.key == "nfs.sec" && property.value == "krb5p" })
        );
        assert!(
            remounted
                .properties
                .iter()
                .any(|property| property.key == "nfs.referral" && property.value == "true")
        );
        assert!(
            remounted
                .properties
                .iter()
                .any(|property| property.key == "nfs.pnfs" && property.value == "true")
        );
        assert!(
            remounted
                .properties
                .iter()
                .any(|property| { property.key == "nfs.remount" && property.value == "true" })
        );
        assert!(
            remounted.properties.iter().any(|property| {
                property.key == "nfs.lookupcache" && property.value == "positive"
            })
        );
        assert!(
            remounted
                .properties
                .iter()
                .any(|property| { property.key == "nfs.lookupcache" && property.value == "none" })
        );
        assert!(
            remounted
                .properties
                .iter()
                .any(|property| property.key == "nfs.noac" && property.value == "true")
        );

        let pnfs = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "mount:/mnt/media")
            .expect("pNFS media mount should exist");
        assert!(
            pnfs.properties
                .iter()
                .any(|property| { property.key == "nfs.layout" && property.value == "flexfiles" })
        );
        assert!(
            pnfs.properties
                .iter()
                .any(|property| property.key == "nfs.dsaddr" && property.value == "10.44.1.10")
        );
        assert!(
            pnfs.properties
                .iter()
                .any(|property| property.key == "nfs.dsaddr2" && property.value == "10.44.1.11")
        );

        let reloaded_export = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "nfs-export:/exports/projects:10.44.0.0/16")
            .expect("merged export reload node should exist");
        assert!(reloaded_export.properties.iter().any(|property| {
            property.key == "nfs.export-option-rw" && property.value == "true"
        }));
        assert!(reloaded_export.properties.iter().any(|property| {
            property.key == "nfs.export-option-ro" && property.value == "true"
        }));
        assert!(reloaded_export.properties.iter().any(|property| {
            property.key == "nfs.export-option-sec" && property.value == "krb5i"
        }));
        assert!(reloaded_export.properties.iter().any(|property| {
            property.key == "nfs.export-option-sec" && property.value == "krb5p"
        }));
        assert!(reloaded_export.properties.iter().any(|property| {
            property.key == "nfs.export-option-refer" && property.value == "true"
        }));
        assert!(reloaded_export.properties.iter().any(|property| {
            property.key == "nfs.export-option-crossmnt" && property.value == "true"
        }));

        let pnfs_export = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "nfs-export:/exports/media:*")
            .expect("pNFS export should exist");
        assert!(pnfs_export.properties.iter().any(|property| {
            property.key == "nfs.export-option-pnfs" && property.value == "true"
        }));
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|edge| {
                    edge.to.0 == "mount:/mnt/referral"
                        && edge.relationship == Relationship::MountedAt
                })
                .count(),
            2
        );
    }

    fn merge_test_graph(graph: &mut StorageGraph, other: StorageGraph) {
        for node in other.nodes {
            graph.add_node(node);
        }
        for edge in other.edges {
            graph.add_edge(edge);
        }
    }
}
