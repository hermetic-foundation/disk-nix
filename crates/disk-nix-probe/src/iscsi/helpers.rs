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

fn secret_is_configured(value: Option<String>) -> bool {
    value
        .as_deref()
        .is_some_and(|value| !matches!(value.trim(), "" | "<empty>" | "[]" | "(null)"))
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
