fn bcache_absent_diagnostic(action: &PlannedAction, query: &str) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("caches")
        || action.operation != Operation::RemoveDevice
        || !is_concrete_bcache_target(query)
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::BcacheDetachAlreadySatisfied,
        query: query.to_string(),
        message: format!("bcache device {query} is already absent from current topology"),
        current: None,
    })
}

fn bcache_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("caches")
        || action.operation != Operation::RemoveDevice
    {
        return None;
    }

    let details = bcache_detach_details(node);
    let message = if details.is_empty() {
        format!("bcache device {query} is still present")
    } else {
        format!(
            "bcache device {query} is still present with {}",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::BcacheDetachRequired,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn bcache_detach_details(node: &Node) -> Vec<String> {
    [
        ("bcache.dirty-data", "dirty data"),
        ("bcache.cache-mode", "cache mode"),
        ("bcache.set-uuid", "cache set"),
    ]
    .into_iter()
    .filter_map(|(property, label)| {
        property_value_from_node(node, property).map(|value| format!("{label} {value}"))
    })
    .collect()
}

fn is_concrete_bcache_target(query: &str) -> bool {
    query.starts_with("/dev/bcache") || query.starts_with("block:/dev/bcache")
}
