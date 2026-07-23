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
    if let Some(username) = &record.username_in {
        target_node = target_node.with_property("iscsi.node-auth-username-in", username.clone());
    }
    if record.password_configured {
        target_node = target_node.with_property("iscsi.node-auth-password-configured", "true");
    }
    if record.password_in_configured {
        target_node = target_node.with_property("iscsi.node-auth-password-in-configured", "true");
    }
    if let Some(auth_method) = &record.discovery_auth_method {
        target_node = target_node.with_property("iscsi.discovery-auth-method", auth_method.clone());
    }
    if let Some(username) = &record.discovery_username {
        target_node = target_node.with_property("iscsi.discovery-auth-username", username.clone());
    }
    if let Some(username) = &record.discovery_username_in {
        target_node =
            target_node.with_property("iscsi.discovery-auth-username-in", username.clone());
    }
    if record.discovery_password_configured {
        target_node = target_node.with_property("iscsi.discovery-auth-password-configured", "true");
    }
    if record.discovery_password_in_configured {
        target_node =
            target_node.with_property("iscsi.discovery-auth-password-in-configured", "true");
    }
    if record.username.is_some() || record.password_configured {
        target_node = target_node.with_property("iscsi.node-auth-direction-out", "true");
    }
    if record.username_in.is_some() || record.password_in_configured {
        target_node = target_node.with_property("iscsi.node-auth-direction-in", "true");
    }

    graph.add_node(target_node);
}

fn add_session(graph: &mut StorageGraph, session: IscsiSession) {
    let mut session_node = Node::new(
        session.id.clone(),
        NodeKind::IscsiSession,
        session.id.clone(),
    );
    if let Some(target) = &session.target {
        session_node = session_node.with_property("iscsi.target", target.clone());
    }
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
            lun_node = lun_node.with_path(device_path(device));
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
            let path = device_path(&device);
            graph.add_node(
                Node::new(
                    format!("block:{path}"),
                    NodeKind::PhysicalDisk,
                    path.clone(),
                )
                .with_path(path.clone()),
            );
            graph.add_edge(Edge::new(
                lun_id,
                format!("block:{path}"),
                Relationship::Backs,
            ));
        }
    }
}

fn device_path(device: &str) -> String {
    if device.starts_with("/dev/") {
        device.to_string()
    } else {
        format!("/dev/{device}")
    }
}
