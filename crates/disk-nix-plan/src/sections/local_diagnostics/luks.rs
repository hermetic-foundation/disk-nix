fn luks_open_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Open
        || action.context.collection.as_deref() != Some("luks.devices")
    {
        return None;
    }
    let active = luks_node_is_active(node)?;
    let (level, kind, message) = if active {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LuksOpenAlreadySatisfied,
            format!("LUKS mapper {query} is already active"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LuksOpenRequired,
            format!("LUKS mapper {query} is known but not active"),
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level,
        kind,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn luks_absent_diagnostic(action: &PlannedAction, query: &str) -> Option<TopologyDiagnostic> {
    match action.context.collection.as_deref() {
        Some("luks.devices") => match action.operation {
            Operation::Open => {
                let backing = action
                    .context
                    .device
                    .as_deref()
                    .unwrap_or("<unspecified-backing-device>");
                Some(TopologyDiagnostic {
                    action_id: action.id.clone(),
                    level: TopologyDiagnosticLevel::Warning,
                    kind: TopologyDiagnosticKind::LuksOpenRequired,
                    query: query.to_string(),
                    message: format!(
                        "LUKS mapper {query} is absent from current topology; opening backing device {backing} remains actionable"
                    ),
                    current: None,
                })
            }
            Operation::Close => Some(TopologyDiagnostic {
                action_id: action.id.clone(),
                level: TopologyDiagnosticLevel::Info,
                kind: TopologyDiagnosticKind::LuksCloseAlreadySatisfied,
                query: query.to_string(),
                message: format!("LUKS mapper {query} is already inactive or absent"),
                current: None,
            }),
            _ => None,
        },
        Some("luksKeyslots")
            if matches!(action.operation, Operation::Destroy | Operation::RemoveKey) =>
        {
            let key_slot = action
                .context
                .key_slot
                .as_deref()
                .unwrap_or("<unknown-slot>");
            let backing = action
                .context
                .device
                .as_deref()
                .unwrap_or("<unspecified-backing-device>");
            Some(TopologyDiagnostic {
                action_id: action.id.clone(),
                level: TopologyDiagnosticLevel::Warning,
                kind: TopologyDiagnosticKind::LuksKeyslotRemoveRequired,
                query: query.to_string(),
                message: format!(
                    "LUKS container {query} is absent from current topology; keyslot {key_slot} removal on backing device {backing} remains actionable after header review"
                ),
                current: None,
            })
        }
        Some("luksTokens")
            if matches!(
                action.operation,
                Operation::Destroy | Operation::RemoveToken
            ) =>
        {
            let token_id = action
                .context
                .token_id
                .as_deref()
                .unwrap_or("<unknown-token>");
            let backing = action
                .context
                .device
                .as_deref()
                .unwrap_or("<unspecified-backing-device>");
            Some(TopologyDiagnostic {
                action_id: action.id.clone(),
                level: TopologyDiagnosticLevel::Warning,
                kind: TopologyDiagnosticKind::LuksTokenRemoveRequired,
                query: query.to_string(),
                message: format!(
                    "LUKS container {query} is absent from current topology; token {token_id} removal on backing device {backing} remains actionable after header review"
                ),
                current: None,
            })
        }
        _ => None,
    }
}

fn luks_format_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Format
        || action.context.collection.as_deref() != Some("luks.devices")
    {
        return None;
    }

    let message = if node.kind == NodeKind::LuksContainer {
        let details = luks_format_present_details(node);
        if details.is_empty() {
            format!(
                "LUKS format target {query} already contains a LUKS container; format remains destructive and requires review"
            )
        } else {
            format!(
                "LUKS format target {query} already contains a LUKS container with {}; format remains destructive and requires review",
                details.join(", ")
            )
        }
    } else {
        format!(
            "LUKS format target {query} matched current {} node {}; format remains destructive and requires review",
            node.kind, node.name
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::LuksFormatTargetPresent,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn luks_format_present_details(node: &Node) -> Vec<String> {
    [
        ("cryptsetup.luks-version", "version"),
        ("cryptsetup.uuid", "UUID"),
        ("cryptsetup.luks-uuid", "UUID"),
        ("cryptsetup.label", "label"),
        ("cryptsetup.luks-label", "label"),
        ("cryptsetup.luks-keyslot-count", "keyslots"),
        ("cryptsetup.luks-token-count", "tokens"),
        ("cryptsetup.active", "active"),
    ]
    .into_iter()
    .filter_map(|(property, label)| {
        property_value_from_node(node, property).map(|value| format!("{label} {value}"))
    })
    .collect()
}

fn luks_close_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Close
        || action.context.collection.as_deref() != Some("luks.devices")
    {
        return None;
    }
    let active = luks_node_is_active(node)?;
    let (level, kind, message) = if active {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LuksCloseRequired,
            format!("LUKS mapper {query} is known and still active"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LuksCloseAlreadySatisfied,
            format!("LUKS mapper {query} is already inactive"),
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level,
        kind,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn luks_keyslot_remove_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("luksKeyslots")
        || !matches!(action.operation, Operation::Destroy | Operation::RemoveKey)
        || node.kind != NodeKind::LuksContainer
    {
        return None;
    }
    let key_slot = action.context.key_slot.as_deref()?;
    let present = property_list_contains(
        property_value_from_node(node, "cryptsetup.luks-keyslots"),
        key_slot,
    );

    let (level, kind, message) = if present {
        let details = luks_keyslot_remove_details(node, key_slot);
        let message = if details.is_empty() {
            format!("LUKS keyslot {key_slot} is still present on {query}")
        } else {
            format!(
                "LUKS keyslot {key_slot} is still present on {query} with {}",
                details.join(", ")
            )
        };
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LuksKeyslotRemoveRequired,
            message,
        )
    } else {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LuksKeyslotRemoveAlreadySatisfied,
            format!("LUKS keyslot {key_slot} is already absent from {query}"),
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level,
        kind,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn luks_token_remove_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("luksTokens")
        || !matches!(
            action.operation,
            Operation::Destroy | Operation::RemoveToken
        )
        || node.kind != NodeKind::LuksContainer
    {
        return None;
    }
    let token_id = action.context.token_id.as_deref()?;
    let present = property_list_contains(
        property_value_from_node(node, "cryptsetup.luks-tokens"),
        token_id,
    );

    let (level, kind, message) = if present {
        let details = luks_token_remove_details(node, token_id);
        let message = if details.is_empty() {
            format!("LUKS token {token_id} is still present on {query}")
        } else {
            format!(
                "LUKS token {token_id} is still present on {query} with {}",
                details.join(", ")
            )
        };
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LuksTokenRemoveRequired,
            message,
        )
    } else {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LuksTokenRemoveAlreadySatisfied,
            format!("LUKS token {token_id} is already absent from {query}"),
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level,
        kind,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn luks_keyslot_remove_details(node: &Node, key_slot: &str) -> Vec<String> {
    let prefix = format!("cryptsetup.luks-keyslot-{key_slot}-");
    [
        ("type", "type"),
        ("priority", "priority"),
        ("cipher", "cipher"),
        ("cipher-key", "cipher key"),
        ("pbkdf", "PBKDF"),
        ("time-cost", "time cost"),
        ("memory", "memory"),
        ("threads", "threads"),
    ]
    .into_iter()
    .filter_map(|(suffix, label)| {
        property_value_from_node(node, &format!("{prefix}{suffix}"))
            .map(|value| format!("{label} {value}"))
    })
    .collect()
}

fn luks_token_remove_details(node: &Node, token_id: &str) -> Vec<String> {
    let prefix = format!("cryptsetup.luks-token-{token_id}-");
    [("type", "type"), ("keyslot", "keyslot")]
        .into_iter()
        .filter_map(|(suffix, label)| {
            property_value_from_node(node, &format!("{prefix}{suffix}"))
                .map(|value| format!("{label} {value}"))
        })
        .collect()
}

fn property_list_contains(values: Option<&str>, needle: &str) -> bool {
    values
        .into_iter()
        .flat_map(|values| values.split(','))
        .map(str::trim)
        .any(|value| value == needle)
}
