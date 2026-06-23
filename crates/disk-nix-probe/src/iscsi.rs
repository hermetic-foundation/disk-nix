use disk_nix_model::{Edge, Node, NodeKind, Relationship, StorageGraph};

use crate::ProbeError;

#[derive(Debug, Clone, PartialEq, Eq)]
struct IscsiSession {
    id: String,
    target: Option<String>,
    portal: Option<String>,
    persistent_portal: Option<String>,
    connection_state: Option<String>,
    session_state: Option<String>,
    internal_session_state: Option<String>,
    iface_name: Option<String>,
    iface_transport: Option<String>,
    iface_initiator_name: Option<String>,
    iface_ip_address: Option<String>,
    iface_netdev: Option<String>,
    host_number: Option<String>,
    host_state: Option<String>,
    negotiated_params: Vec<(String, String)>,
    luns: Vec<IscsiLun>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IscsiLun {
    lun: String,
    attached_device: Option<String>,
    attached_device_state: Option<String>,
    host_number: Option<String>,
    scsi_channel: Option<String>,
    scsi_id: Option<String>,
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
                session_state: None,
                internal_session_state: None,
                iface_name: None,
                iface_transport: None,
                iface_initiator_name: None,
                iface_ip_address: None,
                iface_netdev: None,
                host_number: None,
                host_state: None,
                negotiated_params: Vec::new(),
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
        } else if lower.starts_with("iscsi session state:") {
            if let Some(session) = &mut current {
                session.session_state = value_after_colon(trimmed);
            }
        } else if lower.starts_with("internal iscsid session state:") {
            if let Some(session) = &mut current {
                session.internal_session_state = value_after_colon(trimmed);
            }
        } else if lower.starts_with("iface name:") {
            if let Some(session) = &mut current {
                session.iface_name = value_after_colon(trimmed);
            }
        } else if lower.starts_with("iface transport:") {
            if let Some(session) = &mut current {
                session.iface_transport = value_after_colon(trimmed);
            }
        } else if lower.starts_with("iface initiatorname:") {
            if let Some(session) = &mut current {
                session.iface_initiator_name = value_after_colon(trimmed);
            }
        } else if lower.starts_with("iface ipaddress:") {
            if let Some(session) = &mut current {
                session.iface_ip_address = value_after_colon(trimmed);
            }
        } else if lower.starts_with("iface netdev:") {
            if let Some(session) = &mut current {
                session.iface_netdev = value_after_colon(trimmed);
            }
        } else if lower.starts_with("host number:") {
            if let Some(session) = &mut current {
                let (host_number, host_state) = parse_host_line(trimmed);
                session.host_number = host_number;
                session.host_state = host_state;
            }
        } else if lower.starts_with("headerdigest:")
            || lower.starts_with("datadigest:")
            || lower.starts_with("maxrecvdatasegmentlength:")
            || lower.starts_with("maxxmitdatasegmentlength:")
            || lower.starts_with("firstburstlength:")
            || lower.starts_with("maxburstlength:")
            || lower.starts_with("immediatedata:")
            || lower.starts_with("initialr2t:")
            || lower.starts_with("maxoutstandingr2t:")
        {
            if let (Some(session), Some((key, value))) = (&mut current, parse_key_value(trimmed)) {
                session
                    .negotiated_params
                    .push((format!("iscsi.{}", normalize_key(&key)), value));
            }
        } else if lower.starts_with("lun:") {
            flush_lun(&mut current, &mut pending_lun);
            pending_lun = value_after_colon(trimmed).map(|lun| IscsiLun {
                lun,
                attached_device: None,
                attached_device_state: None,
                host_number: None,
                scsi_channel: None,
                scsi_id: None,
            });
        } else if lower.starts_with("scsi") && lower.contains(" lun:") {
            flush_lun(&mut current, &mut pending_lun);
            if let Some(lun) = parse_scsi_lun_line(trimmed) {
                pending_lun = Some(lun);
            }
        } else if lower.starts_with("attached scsi disk") {
            if let Some(lun) = &mut pending_lun {
                lun.attached_device = parse_attached_disk(trimmed);
                lun.attached_device_state = parse_state_after_label(trimmed);
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
    if let Some(state) = &session.session_state {
        session_node = session_node.with_property("iscsi.session-state", state.clone());
    }
    if let Some(state) = &session.internal_session_state {
        session_node = session_node.with_property("iscsi.internal-session-state", state.clone());
    }
    if let Some(iface_name) = &session.iface_name {
        session_node = session_node.with_property("iscsi.iface-name", iface_name.clone());
    }
    if let Some(transport) = &session.iface_transport {
        session_node = session_node.with_property("iscsi.iface-transport", transport.clone());
    }
    if let Some(initiator_name) = &session.iface_initiator_name {
        session_node =
            session_node.with_property("iscsi.iface-initiator-name", initiator_name.clone());
    }
    if let Some(ip_address) = &session.iface_ip_address {
        session_node = session_node.with_property("iscsi.iface-ip-address", ip_address.clone());
    }
    if let Some(netdev) = &session.iface_netdev {
        session_node = session_node.with_property("iscsi.iface-netdev", netdev.clone());
    }
    if let Some(host_number) = &session.host_number {
        session_node = session_node.with_property("iscsi.host-number", host_number.clone());
    }
    if let Some(host_state) = &session.host_state {
        session_node = session_node.with_property("iscsi.host-state", host_state.clone());
    }
    for (key, value) in &session.negotiated_params {
        session_node = session_node.with_property(key.clone(), value.clone());
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
        if let Some(state) = &lun.attached_device_state {
            lun_node = lun_node.with_property("iscsi.attached-disk-state", state.clone());
        }
        if let Some(host_number) = &lun.host_number {
            lun_node = lun_node.with_property("iscsi.host-number", host_number.clone());
        }
        if let Some(channel) = &lun.scsi_channel {
            lun_node = lun_node.with_property("iscsi.scsi-channel", channel.clone());
        }
        if let Some(scsi_id) = &lun.scsi_id {
            lun_node = lun_node.with_property("iscsi.scsi-id", scsi_id.clone());
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

fn parse_key_value(value: &str) -> Option<(String, String)> {
    let (key, value) = value.split_once(':')?;
    let value = value.trim();
    (!value.is_empty()).then(|| (key.trim().to_string(), value.to_string()))
}

fn parse_host_line(value: &str) -> (Option<String>, Option<String>) {
    let mut host_number = None;
    let mut host_state = None;
    let parts: Vec<&str> = value.split_whitespace().collect();

    for window in parts.windows(2) {
        match window[0].trim_end_matches(':') {
            "Number" => host_number = Some(window[1].to_string()),
            "State" => host_state = Some(window[1].to_string()),
            _ => {}
        }
    }

    (host_number, host_state)
}

fn parse_scsi_lun_line(value: &str) -> Option<IscsiLun> {
    let parts: Vec<&str> = value.split_whitespace().collect();
    let host_number = parts.first()?.strip_prefix("scsi").map(str::to_string);
    let mut channel = None;
    let mut scsi_id = None;
    let mut lun = None;

    for window in parts.windows(2) {
        match window[0].trim_end_matches(':') {
            "Channel" => channel = Some(window[1].to_string()),
            "Id" => scsi_id = Some(window[1].to_string()),
            "Lun" => lun = Some(window[1].to_string()),
            _ => {}
        }
    }

    lun.map(|lun| IscsiLun {
        lun,
        attached_device: None,
        attached_device_state: None,
        host_number,
        scsi_channel: channel,
        scsi_id,
    })
}

fn parse_attached_disk(value: &str) -> Option<String> {
    let parts: Vec<&str> = value.split_whitespace().collect();
    parts
        .windows(2)
        .find_map(|window| (window[0] == "disk").then_some(window[1].to_string()))
}

fn parse_state_after_label(value: &str) -> Option<String> {
    let parts: Vec<&str> = value.split_whitespace().collect();
    parts.windows(2).find_map(|window| {
        (window[0].trim_end_matches(':') == "State").then_some(window[1].to_string())
    })
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
    Iface Transport: tcp
    Iface Initiatorname: iqn.2026-06.client:node1
    Iface IPaddress: 10.0.0.20
    Iface Netdev: eno1
    SID: 12
    iSCSI Connection State: LOGGED IN
    iSCSI Session State: LOGGED_IN
    Internal iscsid Session State: NO CHANGE
    HeaderDigest: None
    DataDigest: None
    MaxRecvDataSegmentLength: 262144
    MaxBurstLength: 262144
    Host Number: 4  State: running
    scsi4 Channel 00 Id 0 Lun: 0
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
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.session-state" && property.value == "LOGGED_IN"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.iface-transport" && property.value == "tcp"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.iface-initiator-name"
                        && property.value == "iqn.2026-06.client:node1"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "iscsi.host-number" && property.value == "4")
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.maxrecvdatasegmentlength" && property.value == "262144"
                })
        }));
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::Lun
                && node.name == "0"
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "iscsi.scsi-channel" && property.value == "00")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "iscsi.scsi-id" && property.value == "0")
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.attached-disk-state" && property.value == "running"
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
