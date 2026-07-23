fn dm_map_rename_absent_diagnostic(
    action: &PlannedAction,
    graph: &StorageGraph,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("dmMaps")
        || action.operation != Operation::Rename
    {
        return None;
    }

    let destination = dm_map_rename_destination(action)?;
    let destination_matches = graph.find_nodes(&destination);
    if let Some(node) = destination_matches
        .iter()
        .copied()
        .find(|node| node.kind == NodeKind::DeviceMapper)
    {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Info,
            kind: TopologyDiagnosticKind::DmMapRenameAlreadySatisfied,
            query: query.to_string(),
            message: format!(
                "device-mapper rename from {query} to {destination} is already reflected in current topology"
            ),
            current: Some(current_node_summary(node)),
        });
    }

    if let Some(node) = destination_matches.first().copied() {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::DmMapRenameRequired,
            query: query.to_string(),
            message: format!(
                "device-mapper rename source {query} is missing, but destination {destination} matched current {} node {}; rename remains actionable for review",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::DmMapRenameRequired,
        query: query.to_string(),
        message: format!(
            "device-mapper rename source {query} is missing and destination {destination} is absent; rename remains actionable after mapper review"
        ),
        current: None,
    })
}

fn dm_map_rename_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("dmMaps")
        || action.operation != Operation::Rename
    {
        return None;
    }

    if node.kind != NodeKind::DeviceMapper {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::DmMapRenameRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not a device-mapper map; rename remains actionable",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    let destination =
        dm_map_rename_destination(action).unwrap_or_else(|| "<new-dm-map-name>".to_string());
    let details = dm_map_details(node);
    let message = if details.is_empty() {
        format!(
            "device-mapper rename source {query} is present; rename to {destination} remains offline-required"
        )
    } else {
        format!(
            "device-mapper rename source {query} is present with {}; rename to {destination} remains offline-required",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::DmMapRenameRequired,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn dm_map_rename_destination(action: &PlannedAction) -> Option<String> {
    let rename_to = action.context.rename_to.as_deref()?;
    if rename_to.starts_with("/dev/mapper/") || rename_to.starts_with("/dev/dm-") {
        Some(rename_to.to_string())
    } else if !rename_to.is_empty() && !rename_to.contains('/') {
        Some(format!("/dev/mapper/{rename_to}"))
    } else {
        None
    }
}

fn dm_map_details(node: &Node) -> Vec<String> {
    [
        ("dm.name", "name"),
        ("dm.uuid", "uuid"),
        ("dm.major", "major"),
        ("dm.minor", "minor"),
        ("dm.open-count", "open count"),
        ("dm.segments", "segments"),
        ("dm.events", "events"),
        ("dm.table", "table"),
        ("dm.status", "status"),
    ]
    .into_iter()
    .filter_map(|(property, label)| {
        property_value_from_node(node, property).map(|value| format!("{label} {value}"))
    })
    .collect()
}
