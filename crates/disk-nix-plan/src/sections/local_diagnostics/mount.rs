fn mount_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Mount {
        return None;
    }
    let desired_source = action.context.device.as_deref()?;
    let current_source = property_value_from_node(node, "mount.source")?;
    let (level, kind, message) = if current_source == desired_source {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::MountAlreadySatisfied,
            format!("mountpoint {query} already uses source {desired_source}"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::MountSourceConflict,
            format!("mountpoint {query} uses source {current_source}, desired {desired_source}"),
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
