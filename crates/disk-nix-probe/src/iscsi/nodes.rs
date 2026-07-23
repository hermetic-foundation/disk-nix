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
                username_in: None,
                password_configured: false,
                password_in_configured: false,
                discovery_auth_method: None,
                discovery_username: None,
                discovery_username_in: None,
                discovery_password_configured: false,
                discovery_password_in_configured: false,
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
        } else if lower.starts_with("username_in:")
            || lower.starts_with("username in:")
            || lower.starts_with("node.session.auth.username_in:")
        {
            if let Some(record) = &mut current {
                record.username_in = value_after_colon(trimmed);
            }
        } else if lower.starts_with("password:") || lower.starts_with("node.session.auth.password:")
        {
            if let Some(record) = &mut current {
                record.password_configured = secret_is_configured(value_after_colon(trimmed));
            }
        } else if lower.starts_with("password_in:")
            || lower.starts_with("password in:")
            || lower.starts_with("node.session.auth.password_in:")
        {
            if let Some(record) = &mut current {
                record.password_in_configured = secret_is_configured(value_after_colon(trimmed));
            }
        } else if lower.starts_with("discovery.sendtargets.auth.authmethod:") {
            if let Some(record) = &mut current {
                record.discovery_auth_method = value_after_colon(trimmed);
            }
        } else if lower.starts_with("discovery.sendtargets.auth.username:") {
            if let Some(record) = &mut current {
                record.discovery_username = value_after_colon(trimmed);
            }
        } else if lower.starts_with("discovery.sendtargets.auth.username_in:") {
            if let Some(record) = &mut current {
                record.discovery_username_in = value_after_colon(trimmed);
            }
        } else if lower.starts_with("discovery.sendtargets.auth.password:") {
            if let Some(record) = &mut current {
                record.discovery_password_configured =
                    secret_is_configured(value_after_colon(trimmed));
            }
        } else if lower.starts_with("discovery.sendtargets.auth.password_in:") {
            if let Some(record) = &mut current {
                record.discovery_password_in_configured =
                    secret_is_configured(value_after_colon(trimmed));
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
        username_in: None,
        password_configured: false,
        password_in_configured: false,
        discovery_auth_method: None,
        discovery_username: None,
        discovery_username_in: None,
        discovery_password_configured: false,
        discovery_password_in_configured: false,
    })
}

fn is_iscsi_target_name(value: &str) -> bool {
    value.starts_with("iqn.") || value.starts_with("eui.") || value.starts_with("naa.")
}
