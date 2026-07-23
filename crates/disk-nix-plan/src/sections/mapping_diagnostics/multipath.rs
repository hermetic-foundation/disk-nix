fn multipath_absent_diagnostic(action: &PlannedAction, query: &str) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("multipathMaps")
        || action.operation != Operation::Destroy
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::MultipathDestroyAlreadySatisfied,
        query: query.to_string(),
        message: format!("multipath map {query} is already absent from current topology"),
        current: None,
    })
}

fn multipath_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("multipathMaps")
        || action.operation != Operation::Destroy
    {
        return None;
    }

    let message = multipath_identity_detail(node)
        .map(|detail| format!("multipath map {query} is still present with {detail}"))
        .unwrap_or_else(|| format!("multipath map {query} is still present"));

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::MultipathDestroyRequired,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn multipath_identity_detail(node: &Node) -> Option<String> {
    if let Some(wwid) = property_value_from_node(node, "multipath.wwid") {
        return Some(format!("WWID {wwid}"));
    }
    property_value_from_node(node, "multipath.dm").map(|dm_name| format!("dm map {dm_name}"))
}

fn multipath_path_remove_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::RemoveDevice
        || action.context.collection.as_deref() != Some("multipathMaps")
    {
        return None;
    }

    let device = action.context.device.as_deref().unwrap_or("<unknown-path>");
    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::MultipathPathRemoveAlreadySatisfied,
        query: query.to_string(),
        message: format!("multipath map {query} is absent, so path {device} is already removed"),
        current: None,
    })
}

fn multipath_path_add_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::AddDevice
        || action.context.collection.as_deref() != Some("multipathMaps")
    {
        return None;
    }

    let device = action.context.device.as_deref().unwrap_or("<unknown-path>");
    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::MultipathPathAddRequired,
        query: query.to_string(),
        message: format!(
            "multipath map {query} is absent, so path {device} cannot be confirmed attached; path add remains actionable after map review"
        ),
        current: None,
    })
}

fn multipath_path_add_diagnostic(
    action: &PlannedAction,
    node: &Node,
    graph: &StorageGraph,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::AddDevice
        || action.context.collection.as_deref() != Some("multipathMaps")
    {
        return None;
    }
    let device = action.context.device.as_deref()?;

    if node.kind != NodeKind::MultipathDevice {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MultipathPathAddRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not a multipath map; path add remains actionable only after target review",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    if multipath_map_has_path(graph, node, device) {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Info,
            kind: TopologyDiagnosticKind::MultipathPathAddAlreadySatisfied,
            query: query.to_string(),
            message: format!("multipath map {query} already includes path {device}"),
            current: Some(current_node_summary(node)),
        });
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::MultipathPathAddRequired,
        query: query.to_string(),
        message: format!("multipath map {query} does not currently include path {device}"),
        current: Some(current_node_summary(node)),
    })
}

fn multipath_path_remove_diagnostic(
    action: &PlannedAction,
    node: &Node,
    graph: &StorageGraph,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::RemoveDevice
        || action.context.collection.as_deref() != Some("multipathMaps")
    {
        return None;
    }
    let device = action.context.device.as_deref()?;

    if node.kind != NodeKind::MultipathDevice {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MultipathPathRemoveRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not a multipath map; path removal remains actionable only after target review",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    if multipath_map_has_path(graph, node, device) {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MultipathPathRemoveRequired,
            query: query.to_string(),
            message: format!("multipath map {query} still includes path {device}"),
            current: Some(current_node_summary(node)),
        });
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::MultipathPathRemoveAlreadySatisfied,
        query: query.to_string(),
        message: format!("multipath map {query} no longer includes path {device}"),
        current: Some(current_node_summary(node)),
    })
}

fn multipath_map_has_path(graph: &StorageGraph, map: &Node, device: &str) -> bool {
    graph.edges.iter().any(|edge| {
        edge.relationship == Relationship::Backs
            && edge.to == map.id
            && graph
                .nodes
                .iter()
                .find(|node| node.id == edge.from)
                .is_some_and(|path| path.matches(device))
    })
}
