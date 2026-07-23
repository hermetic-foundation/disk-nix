fn dm_map_absent_diagnostic(action: &PlannedAction, query: &str) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("dmMaps")
        || action.operation != Operation::Destroy
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::DmMapDestroyAlreadySatisfied,
        query: query.to_string(),
        message: format!("device-mapper map {query} is already absent from current topology"),
        current: None,
    })
}

fn dm_map_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("dmMaps")
        || action.operation != Operation::Destroy
    {
        return None;
    }

    let message = match property_value_from_node(node, "dm.open-count") {
        Some(open_count) => {
            format!("device-mapper map {query} is still present with open count {open_count}")
        }
        None => format!("device-mapper map {query} is still present"),
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::DmMapDestroyRequired,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}
