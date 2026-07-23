fn swap_inactive_diagnostic(action: &PlannedAction, query: &str) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("swaps")
        || !matches!(action.operation, Operation::Deactivate | Operation::Destroy)
    {
        return None;
    }

    let (kind, message) = match action.operation {
        Operation::Deactivate => (
            TopologyDiagnosticKind::SwapDeactivateAlreadySatisfied,
            format!("swap target {query} is already inactive or absent from current topology"),
        ),
        Operation::Destroy => (
            TopologyDiagnosticKind::SwapDestroyAlreadySatisfied,
            format!("swap target {query} is already inactive or absent from current topology"),
        ),
        _ => unreachable!("operation checked above"),
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind,
        query: query.to_string(),
        message,
        current: None,
    })
}

fn swap_active_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("swaps")
        || !matches!(action.operation, Operation::Deactivate | Operation::Destroy)
    {
        return None;
    }

    let details = swap_active_details(node);
    let detail_suffix = if details.is_empty() {
        String::new()
    } else {
        format!(" with {}", details.join(", "))
    };
    let (kind, message) = match action.operation {
        Operation::Deactivate => (
            TopologyDiagnosticKind::SwapDeactivateRequired,
            format!("swap target {query} is active{detail_suffix}"),
        ),
        Operation::Destroy => (
            TopologyDiagnosticKind::SwapDestroyRequired,
            format!("swap target {query} is active{detail_suffix}"),
        ),
        _ => unreachable!("operation checked above"),
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn swap_format_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Format
        || action.context.collection.as_deref() != Some("swaps")
    {
        return None;
    }

    let message = if node.kind == NodeKind::Swap {
        let details = swap_active_details(node);
        if details.is_empty() {
            format!(
                "swap format target {query} already has swap metadata; mkswap remains destructive and requires review"
            )
        } else {
            format!(
                "swap format target {query} already has swap metadata with {}; mkswap remains destructive and requires review",
                details.join(", ")
            )
        }
    } else {
        format!(
            "swap format target {query} matched current {} node {}; mkswap remains destructive and requires review",
            node.kind, node.name
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::SwapFormatTargetPresent,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn swap_active_details(node: &Node) -> Vec<String> {
    let mut details = Vec::new();
    if let Some(size) = node.size_bytes {
        details.push(format!("size {size} bytes"));
    }
    if let Some(used) = node.usage.as_ref().and_then(|usage| usage.used_bytes) {
        details.push(format!("used {used} bytes"));
    }
    if let Some(priority) = property_value_from_node(node, "swap.priority") {
        details.push(format!("priority {priority}"));
    }
    if let Some(swap_type) = property_value_from_node(node, "swap.type") {
        details.push(format!("type {swap_type}"));
    }
    details
}
