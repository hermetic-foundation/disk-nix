use disk_nix_model::{Edge, Node, NodeKind, Relationship, StorageGraph};

use crate::ProbeError;

#[derive(Debug, Clone, PartialEq, Eq)]
struct IscsiSession {
    id: String,
    target: Option<String>,
    portal: Option<String>,
    persistent_portal: Option<String>,
    connection_state: Option<String>,
    luns: Vec<IscsiLun>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IscsiLun {
    lun: String,
    attached_device: Option<String>,
}

pub fn normalize_iscsi_session_output(bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let sessions = parse_sessions(bytes)?;
    let mut graph = StorageGraph::empty();

    for session in sessions {
        add_session(&mut graph, session);
    }

    Ok(graph)
}

fn parse_sessions(bytes: &[u8]) -> Result<Vec<IscsiSession>, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read iscsiadm session output: {error}"))
    })?;
    let mut sessions = Vec::new();
    let mut current: Option<IscsiSession> = None;
    let mut pending_lun: Option<IscsiLun> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("target:") {
            flush_lun(&mut current, &mut pending_lun);
            flush_session(&mut sessions, &mut current);
            current = Some(IscsiSession {
                id: format!("iscsi-session:{}", sessions.len() + 1),
                target: value_after_colon(trimmed),
                portal: None,
                persistent_portal: None,
                connection_state: None,
                luns: Vec::new(),
            });
        } else if lower.starts_with("current portal:") || lower.starts_with("portal:") {
            if let Some(session) = &mut current {
                session.portal = value_after_colon(trimmed);
            }
        } else if lower.starts_with("persistent portal:") {
            if let Some(session) = &mut current {
                session.persistent_portal = value_after_colon(trimmed);
            }
        } else if lower.starts_with("sid:") {
            if let (Some(session), Some(sid)) = (&mut current, value_after_colon(trimmed)) {
                session.id = format!("iscsi-session:{sid}");
            }
        } else if lower.starts_with("iscsi connection state:") {
            if let Some(session) = &mut current {
                session.connection_state = value_after_colon(trimmed);
            }
        } else if lower.starts_with("lun:") {
            flush_lun(&mut current, &mut pending_lun);
            pending_lun = value_after_colon(trimmed).map(|lun| IscsiLun {
                lun,
                attached_device: None,
            });
        } else if lower.starts_with("attached scsi disk") {
            if let Some(lun) = &mut pending_lun {
                lun.attached_device = parse_attached_disk(trimmed);
            }
        }
    }

    flush_lun(&mut current, &mut pending_lun);
    flush_session(&mut sessions, &mut current);

    Ok(sessions)
}

fn add_session(graph: &mut StorageGraph, session: IscsiSession) {
    let mut session_node = Node::new(
        session.id.clone(),
        NodeKind::IscsiSession,
        session.id.clone(),
    );
    if let Some(portal) = &session.portal {
        session_node = session_node.with_property("iscsi.portal", portal.clone());
    }
    if let Some(portal) = &session.persistent_portal {
        session_node = session_node.with_property("iscsi.persistent-portal", portal.clone());
    }
    if let Some(state) = &session.connection_state {
        session_node = session_node.with_property("iscsi.connection-state", state.clone());
    }
    graph.add_node(session_node);

    let target_id = session.target.as_ref().map(|target| {
        let target_id = format!("iscsi-target:{target}");
        graph.add_node(Node::new(
            target_id.clone(),
            NodeKind::IscsiTarget,
            target.clone(),
        ));
        graph.add_edge(Edge::new(
            session.id.clone(),
            target_id.clone(),
            Relationship::ImportedFrom,
        ));
        target_id
    });

    for lun in session.luns {
        let lun_id = format!(
            "iscsi-lun:{}:{}",
            session.target.as_deref().unwrap_or(session.id.as_str()),
            lun.lun
        );
        let mut lun_node = Node::new(lun_id.clone(), NodeKind::Lun, lun.lun.clone());
        if let Some(device) = &lun.attached_device {
            lun_node = lun_node.with_property("iscsi.attached-disk", device.clone());
        }
        graph.add_node(lun_node);

        if let Some(target_id) = &target_id {
            graph.add_edge(Edge::new(
                target_id.clone(),
                lun_id.clone(),
                Relationship::Contains,
            ));
        } else {
            graph.add_edge(Edge::new(
                session.id.clone(),
                lun_id.clone(),
                Relationship::Contains,
            ));
        }

        if let Some(device) = lun.attached_device {
            graph.add_node(
                Node::new(
                    format!("block:/dev/{device}"),
                    NodeKind::PhysicalDisk,
                    format!("/dev/{device}"),
                )
                .with_path(format!("/dev/{device}")),
            );
            graph.add_edge(Edge::new(
                lun_id,
                format!("block:/dev/{device}"),
                Relationship::Backs,
            ));
        }
    }
}

fn flush_lun(current: &mut Option<IscsiSession>, pending_lun: &mut Option<IscsiLun>) {
    if let (Some(session), Some(lun)) = (current, pending_lun.take()) {
        session.luns.push(lun);
    }
}

fn flush_session(sessions: &mut Vec<IscsiSession>, current: &mut Option<IscsiSession>) {
    if let Some(session) = current.take() {
        sessions.push(session);
    }
}

fn value_after_colon(value: &str) -> Option<String> {
    value
        .split_once(':')
        .map(|(_, value)| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn parse_attached_disk(value: &str) -> Option<String> {
    let parts: Vec<&str> = value.split_whitespace().collect();
    parts
        .windows(2)
        .find_map(|window| (window[0] == "disk").then_some(window[1].to_string()))
}

#[cfg(test)]
mod tests {
    use disk_nix_model::{NodeKind, Relationship};

    use super::*;

    const SESSION: &[u8] = br#"
Target: iqn.2026-06.example:storage.disk1
    Current Portal: 10.0.0.10:3260,1
    Persistent Portal: 10.0.0.10:3260,1
    **********
    Interface:
    **********
    Iface Name: default
    SID: 12
    iSCSI Connection State: LOGGED IN
    LUN: 0
        Attached scsi disk sdb          State: running
"#;

    #[test]
    fn normalizes_iscsi_session_target_lun_and_disk() {
        let graph = normalize_iscsi_session_output(SESSION).expect("fixture should parse");

        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::IscsiSession && node.name == "iscsi-session:12")
        );
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::IscsiTarget)
        );
        assert!(graph.nodes.iter().any(|node| node.kind == NodeKind::Lun));
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::IscsiSession
                && node.name == "iscsi-session:12"
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.persistent-portal"
                        && property.value == "10.0.0.10:3260,1"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.connection-state" && property.value == "LOGGED IN"
                })
        }));
        assert!(
            graph
                .edges
                .iter()
                .any(|edge| edge.relationship == Relationship::Backs)
        );
    }
}
