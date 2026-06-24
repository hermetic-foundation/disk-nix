use disk_nix_model::{Edge, Node, NodeKind, Relationship, StorageGraph};

use crate::ProbeError;

#[derive(Debug, Clone, PartialEq, Eq)]
struct IscsiSession {
    id: String,
    target: Option<String>,
    portal: Option<String>,
    persistent_portal: Option<String>,
    target_portal_group_tag: Option<String>,
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
    connection_params: Vec<(String, String)>,
    negotiated_params: Vec<(String, String)>,
    luns: Vec<IscsiLun>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IscsiNodeRecord {
    target: String,
    portal: Option<String>,
    persistent_portal: Option<String>,
    target_portal_group_tag: Option<String>,
    iface_name: Option<String>,
    startup: Option<String>,
    leading_login: Option<String>,
    auth_method: Option<String>,
    username: Option<String>,
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

pub fn normalize_iscsi_node_output(bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let nodes = parse_node_records(bytes)?;
    let mut graph = StorageGraph::empty();

    for record in nodes {
        add_node_record(&mut graph, record);
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
                target_portal_group_tag: None,
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
                connection_params: Vec::new(),
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
        } else if lower.starts_with("target portal group tag:") {
            if let Some(session) = &mut current {
                session.target_portal_group_tag = value_after_colon(trimmed);
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
        } else if lower.starts_with("cid:")
            || lower.starts_with("connection state:")
            || lower.starts_with("local address:")
            || lower.starts_with("peer address:")
        {
            if let (Some(session), Some((key, value))) = (&mut current, parse_key_value(trimmed)) {
                session
                    .connection_params
                    .push((connection_property_key(&key), value));
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

fn parse_node_records(bytes: &[u8]) -> Result<Vec<IscsiNodeRecord>, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read iscsiadm node output: {error}"))
    })?;
    let mut records = Vec::new();
    let mut current: Option<IscsiNodeRecord> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();

        if lower.starts_with("target:") {
            flush_node_record(&mut records, &mut current);
            current = value_after_colon(trimmed).map(|target| IscsiNodeRecord {
                target,
                portal: None,
                persistent_portal: None,
                target_portal_group_tag: None,
                iface_name: None,
                startup: None,
                leading_login: None,
                auth_method: None,
                username: None,
            });
        } else if lower.starts_with("portal:") {
            if let Some(record) = &mut current {
                record.portal = value_after_colon(trimmed);
            }
        } else if lower.starts_with("persistent portal:") {
            if let Some(record) = &mut current {
                record.persistent_portal = value_after_colon(trimmed);
            }
        } else if lower.starts_with("tpgt:") || lower.starts_with("target portal group tag:") {
            if let Some(record) = &mut current {
                record.target_portal_group_tag = value_after_colon(trimmed);
            }
        } else if lower.starts_with("iface name:") {
            if let Some(record) = &mut current {
                record.iface_name = value_after_colon(trimmed);
            }
        } else if lower.starts_with("startup:") || lower.starts_with("node.startup:") {
            if let Some(record) = &mut current {
                record.startup = value_after_colon(trimmed);
            }
        } else if lower.starts_with("leading login:") || lower.starts_with("node.leading_login:") {
            if let Some(record) = &mut current {
                record.leading_login = value_after_colon(trimmed);
            }
        } else if lower.starts_with("authmethod:")
            || lower.starts_with("auth method:")
            || lower.starts_with("node.session.auth.authmethod:")
        {
            if let Some(record) = &mut current {
                record.auth_method = value_after_colon(trimmed);
            }
        } else if lower.starts_with("username:") || lower.starts_with("node.session.auth.username:")
        {
            if let Some(record) = &mut current {
                record.username = value_after_colon(trimmed);
            }
        } else if let Some(record) = parse_concise_node_record(trimmed) {
            flush_node_record(&mut records, &mut current);
            records.push(record);
        }
    }

    flush_node_record(&mut records, &mut current);

    Ok(records)
}

fn parse_concise_node_record(value: &str) -> Option<IscsiNodeRecord> {
    let mut parts = value.split_whitespace();
    let portal = parts.next()?.to_string();
    let target = parts.next()?.to_string();
    is_iscsi_target_name(&target).then_some(IscsiNodeRecord {
        target,
        portal: Some(portal),
        persistent_portal: None,
        target_portal_group_tag: None,
        iface_name: None,
        startup: None,
        leading_login: None,
        auth_method: None,
        username: None,
    })
}

fn is_iscsi_target_name(value: &str) -> bool {
    value.starts_with("iqn.") || value.starts_with("eui.") || value.starts_with("naa.")
}

fn add_node_record(graph: &mut StorageGraph, record: IscsiNodeRecord) {
    let target_id = format!("iscsi-target:{}", record.target);
    let mut target_node = Node::new(target_id, NodeKind::IscsiTarget, record.target)
        .with_property("iscsi.node-configured", "true");

    if let Some(portal) = &record.portal {
        target_node = target_node.with_property("iscsi.node-portal", portal.clone());
        for (key, value) in portal_parts("iscsi.node-portal", portal) {
            target_node = target_node.with_property(key, value);
        }
    }
    if let Some(portal) = &record.persistent_portal {
        target_node = target_node.with_property("iscsi.node-persistent-portal", portal.clone());
        for (key, value) in portal_parts("iscsi.node-persistent-portal", portal) {
            target_node = target_node.with_property(key, value);
        }
    }
    if let Some(tag) = &record.target_portal_group_tag {
        target_node = target_node.with_property("iscsi.node-tpgt", tag.clone());
    }
    if let Some(iface_name) = &record.iface_name {
        target_node = target_node.with_property("iscsi.node-iface-name", iface_name.clone());
    }
    if let Some(startup) = &record.startup {
        target_node = target_node.with_property("iscsi.node-startup", startup.clone());
    }
    if let Some(leading_login) = &record.leading_login {
        target_node = target_node.with_property("iscsi.node-leading-login", leading_login.clone());
    }
    if let Some(auth_method) = &record.auth_method {
        target_node = target_node.with_property("iscsi.node-auth-method", auth_method.clone());
    }
    if let Some(username) = &record.username {
        target_node = target_node.with_property("iscsi.node-auth-username", username.clone());
    }

    graph.add_node(target_node);
}

fn add_session(graph: &mut StorageGraph, session: IscsiSession) {
    let mut session_node = Node::new(
        session.id.clone(),
        NodeKind::IscsiSession,
        session.id.clone(),
    );
    if let Some(portal) = &session.portal {
        session_node = session_node.with_property("iscsi.portal", portal.clone());
        for (key, value) in portal_parts("iscsi.portal", portal) {
            session_node = session_node.with_property(key, value);
        }
    }
    if let Some(portal) = &session.persistent_portal {
        session_node = session_node.with_property("iscsi.persistent-portal", portal.clone());
        for (key, value) in portal_parts("iscsi.persistent-portal", portal) {
            session_node = session_node.with_property(key, value);
        }
    }
    if let Some(tag) = &session.target_portal_group_tag {
        session_node = session_node.with_property("iscsi.target-portal-group-tag", tag.clone());
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
    for (key, value) in &session.connection_params {
        session_node = session_node.with_property(key.clone(), value.clone());
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

fn flush_node_record(records: &mut Vec<IscsiNodeRecord>, current: &mut Option<IscsiNodeRecord>) {
    if let Some(record) = current.take() {
        records.push(record);
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

fn portal_parts(prefix: &str, portal: &str) -> Vec<(String, String)> {
    let Some((endpoint, tpgt)) = portal.rsplit_once(',') else {
        return endpoint_parts(prefix, portal);
    };
    let mut parts = endpoint_parts(prefix, endpoint);
    if !tpgt.trim().is_empty() {
        parts.push((format!("{prefix}-tpgt"), tpgt.trim().to_string()));
    }
    parts
}

fn endpoint_parts(prefix: &str, endpoint: &str) -> Vec<(String, String)> {
    let endpoint = endpoint.trim();
    if endpoint.is_empty() {
        return Vec::new();
    }

    if let Some((host, port)) = bracketed_endpoint(endpoint) {
        return vec![
            (format!("{prefix}-address"), host.to_string()),
            (format!("{prefix}-port"), port.to_string()),
        ];
    }

    if endpoint.matches(':').count() == 1 {
        let Some((host, port)) = endpoint.rsplit_once(':') else {
            return vec![(format!("{prefix}-address"), endpoint.to_string())];
        };
        if !host.is_empty()
            && !port.is_empty()
            && port.chars().all(|character| character.is_ascii_digit())
        {
            return vec![
                (format!("{prefix}-address"), host.to_string()),
                (format!("{prefix}-port"), port.to_string()),
            ];
        }
    }

    vec![(format!("{prefix}-address"), endpoint.to_string())]
}

fn bracketed_endpoint(endpoint: &str) -> Option<(&str, &str)> {
    let host = endpoint.strip_prefix('[')?.split_once(']')?.0;
    let port = endpoint.strip_prefix('[')?.split_once("]:")?.1.trim();
    (!host.is_empty()
        && !port.is_empty()
        && port.chars().all(|character| character.is_ascii_digit()))
    .then_some((host, port))
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

fn connection_property_key(key: &str) -> String {
    match normalize_key(key).as_str() {
        "connection-state" => "iscsi.connection-detail-state".to_string(),
        key => format!("iscsi.connection-{key}"),
    }
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
    Target Portal Group Tag: 1
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
    CID: 0
    Connection State: LOGGED IN
    Local Address: 10.0.0.20
    Peer Address: 10.0.0.10
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
                    property.key == "iscsi.portal-address" && property.value == "10.0.0.10"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "iscsi.portal-port" && property.value == "3260")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "iscsi.portal-tpgt" && property.value == "1")
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.persistent-portal-address"
                        && property.value == "10.0.0.10"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.persistent-portal-port" && property.value == "3260"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.persistent-portal-tpgt" && property.value == "1"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.connection-state" && property.value == "LOGGED IN"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.target-portal-group-tag" && property.value == "1"
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
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "iscsi.connection-cid" && property.value == "0")
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.connection-detail-state" && property.value == "LOGGED IN"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.connection-local-address"
                        && property.value == "10.0.0.20"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.connection-peer-address" && property.value == "10.0.0.10"
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

    #[test]
    fn parses_bracketed_ipv6_iscsi_portal_parts() {
        assert_eq!(
            portal_parts("iscsi.portal", "[2001:db8::10]:3260,2"),
            vec![
                (
                    "iscsi.portal-address".to_string(),
                    "2001:db8::10".to_string()
                ),
                ("iscsi.portal-port".to_string(), "3260".to_string()),
                ("iscsi.portal-tpgt".to_string(), "2".to_string())
            ]
        );
        assert_eq!(
            portal_parts("iscsi.portal", "2001:db8::10"),
            vec![(
                "iscsi.portal-address".to_string(),
                "2001:db8::10".to_string()
            )]
        );
    }

    #[test]
    fn normalizes_configured_iscsi_nodes() {
        let graph = normalize_iscsi_node_output(
            br#"
Target: iqn.2026-06.example:storage.disk1
    Portal: 10.0.0.10:3260,1
    Persistent Portal: 10.0.0.11:3260,1
    TPGT: 1
    Iface Name: default
    Startup: automatic
    Leading Login: Yes
    AuthMethod: CHAP
    Username: node-user
10.0.0.12:3260,2 iqn.2026-06.example:storage.disk2
"#,
        )
        .expect("fixture should parse");

        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::IscsiTarget
                && node.name == "iqn.2026-06.example:storage.disk1"
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "iscsi.node-configured")
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.node-portal" && property.value == "10.0.0.10:3260,1"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.node-portal-address" && property.value == "10.0.0.10"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.node-persistent-portal"
                        && property.value == "10.0.0.11:3260,1"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "iscsi.node-tpgt" && property.value == "1")
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.node-startup" && property.value == "automatic"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.node-auth-method" && property.value == "CHAP"
                })
        }));
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::IscsiTarget
                && node.name == "iqn.2026-06.example:storage.disk2"
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.node-portal" && property.value == "10.0.0.12:3260,2"
                })
        }));
    }
}
