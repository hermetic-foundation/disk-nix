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
