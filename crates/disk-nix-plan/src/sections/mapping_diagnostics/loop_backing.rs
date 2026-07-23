fn loop_absent_diagnostic(action: &PlannedAction, query: &str) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("loopDevices") {
        return None;
    }

    let (level, kind, message) = match action.operation {
        Operation::Create => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LoopCreateRequired,
            format!("loop device {query} is not currently mapped"),
        ),
        Operation::Destroy | Operation::Detach => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LoopDetachAlreadySatisfied,
            format!("loop device {query} is already absent from current topology"),
        ),
        _ => return None,
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level,
        kind,
        query: query.to_string(),
        message,
        current: None,
    })
}

fn loop_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("loopDevices") {
        return None;
    }

    match action.operation {
        Operation::Create => loop_create_diagnostic(action, node, query),
        Operation::Destroy | Operation::Detach => Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::LoopDetachRequired,
            query: query.to_string(),
            message: format!("loop device {query} is still mapped"),
            current: Some(current_node_summary(node)),
        }),
        _ => None,
    }
}

fn loop_create_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    let desired_backing = action.context.device.as_deref();
    let current_backing = property_value_from_node(node, "loop.back-file");
    let (level, kind, message) = match (desired_backing, current_backing) {
        (Some(desired), Some(current)) if desired == current => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LoopCreateAlreadySatisfied,
            format!("loop device {query} already maps backing file {desired}"),
        ),
        (Some(desired), Some(current)) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LoopCreateConflict,
            format!("loop device {query} maps backing file {current}, desired {desired}"),
        ),
        (Some(desired), None) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LoopCreateConflict,
            format!("loop device {query} is present but does not report backing file {desired}"),
        ),
        (None, Some(current)) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LoopCreateConflict,
            format!("loop device {query} is already mapped to backing file {current}"),
        ),
        (None, None) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LoopCreateConflict,
            format!("loop device {query} is already present with unknown backing file"),
        ),
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

fn backing_file_create_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Create
        || action.context.collection.as_deref() != Some("backingFiles")
        || node.kind != NodeKind::BackingFile
    {
        return None;
    }

    let desired = action.context.desired_size.as_deref();
    let desired_bytes = desired.and_then(parse_size_bytes);
    let (level, kind, message) = match (desired, desired_bytes, node.size_bytes) {
        (Some(desired), Some(desired_bytes), Some(current_bytes))
            if current_bytes == desired_bytes =>
        {
            (
                TopologyDiagnosticLevel::Info,
                TopologyDiagnosticKind::BackingFileCreateAlreadySatisfied,
                format!(
                    "backing file {query} already exists with desired size {desired} ({current_bytes} bytes)"
                ),
            )
        }
        (Some(desired), Some(_), Some(current_bytes)) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::BackingFileCreateRequired,
            format!(
                "backing file {query} already exists with size {current_bytes} bytes, not desired size {desired}; create would refuse to overwrite it"
            ),
        ),
        (Some(desired), None, _) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::BackingFileCreateRequired,
            format!(
                "backing file {query} already exists, but desired size {desired} could not be compared"
            ),
        ),
        (None, _, Some(current_bytes)) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::BackingFileCreateRequired,
            format!(
                "backing file {query} already exists with size {current_bytes} bytes, but create has no desired size to compare"
            ),
        ),
        (None, _, None) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::BackingFileCreateRequired,
            format!(
                "backing file {query} already exists, but create has no desired size to compare"
            ),
        ),
        (Some(desired), Some(_), None) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::BackingFileCreateRequired,
            format!(
                "backing file {query} already exists, but current size is unknown; desired size is {desired}"
            ),
        ),
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
