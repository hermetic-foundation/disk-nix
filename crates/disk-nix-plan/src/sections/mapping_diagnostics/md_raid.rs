fn md_create_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Create
        || action.context.collection.as_deref() != Some("mdRaids")
    {
        return None;
    }

    if node.kind != NodeKind::MdRaid {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MdCreateRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not an MD RAID array; mdadm --create remains actionable",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    let Some(status) = md_array_status(node) else {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MdCreateRequired,
            query: query.to_string(),
            message: format!(
                "MD RAID array {query} already exists, but current state is unknown; rescan before treating create as satisfied"
            ),
            current: Some(current_node_summary(node)),
        });
    };

    if !status.cleanly_active {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MdCreateRequired,
            query: query.to_string(),
            message: format!(
                "MD RAID array {query} already exists, but state needs review before treating create as satisfied: state={}, degradedDevices={}, failedDevices={}",
                status.state, status.degraded_devices, status.failed_devices
            ),
            current: Some(current_node_summary(node)),
        });
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::MdCreateAlreadySatisfied,
        query: query.to_string(),
        message: format!(
            "MD RAID array {query} already exists and is cleanly active: state={}, degradedDevices=0, failedDevices=0",
            status.state
        ),
        current: Some(current_node_summary(node)),
    })
}

fn md_assemble_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Assemble
        || action.context.collection.as_deref() != Some("mdRaids")
    {
        return None;
    }
    let status = md_array_status(node)?;
    let (level, kind, message) = if status.cleanly_active {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::MdAssembleAlreadySatisfied,
            format!("MD RAID array {query} is already cleanly assembled"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::MdAssembleRequired,
            format!(
                "MD RAID array {query} is not cleanly assembled: state={}, degradedDevices={}, failedDevices={}",
                status.state, status.degraded_devices, status.failed_devices
            ),
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

fn md_stop_absent_diagnostic(action: &PlannedAction, query: &str) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Stop
        || action.context.collection.as_deref() != Some("mdRaids")
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::MdStopAlreadySatisfied,
        query: query.to_string(),
        message: format!("MD RAID array {query} is already absent from current topology"),
        current: None,
    })
}

fn md_stop_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Stop
        || action.context.collection.as_deref() != Some("mdRaids")
    {
        return None;
    }

    if node.kind != NodeKind::MdRaid {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MdStopRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not an MD RAID array; mdadm --stop remains actionable only after target review",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    let Some(status) = md_array_status(node) else {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MdStopRequired,
            query: query.to_string(),
            message: format!(
                "MD RAID array {query} is present, but current state is unknown; rescan before treating stop as satisfied"
            ),
            current: Some(current_node_summary(node)),
        });
    };

    if status.active {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MdStopRequired,
            query: query.to_string(),
            message: format!(
                "MD RAID array {query} is still active: state={}, degradedDevices={}, failedDevices={}",
                status.state, status.degraded_devices, status.failed_devices
            ),
            current: Some(current_node_summary(node)),
        });
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::MdStopAlreadySatisfied,
        query: query.to_string(),
        message: format!(
            "MD RAID array {query} is already inactive: state={}",
            status.state
        ),
        current: Some(current_node_summary(node)),
    })
}

fn md_member_remove_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::RemoveDevice
        || action.context.collection.as_deref() != Some("mdRaids")
    {
        return None;
    }

    let device = action
        .context
        .device
        .as_deref()
        .unwrap_or("<unknown-member>");
    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::MdMemberRemoveAlreadySatisfied,
        query: query.to_string(),
        message: format!("MD RAID array {query} is absent, so member {device} is already removed"),
        current: None,
    })
}

fn md_member_add_diagnostic(
    action: &PlannedAction,
    node: &Node,
    graph: &StorageGraph,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::AddDevice
        || action.context.collection.as_deref() != Some("mdRaids")
    {
        return None;
    }
    let device = action.context.device.as_deref()?;

    if node.kind != NodeKind::MdRaid {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MdMemberAddRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not an MD RAID array; mdadm --add remains actionable only after target review",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    if md_array_has_member(graph, node, device) {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Info,
            kind: TopologyDiagnosticKind::MdMemberAddAlreadySatisfied,
            query: query.to_string(),
            message: format!("MD RAID array {query} already includes member {device}"),
            current: Some(current_node_summary(node)),
        });
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::MdMemberAddRequired,
        query: query.to_string(),
        message: format!("MD RAID array {query} does not currently include member {device}"),
        current: Some(current_node_summary(node)),
    })
}

fn md_member_remove_diagnostic(
    action: &PlannedAction,
    node: &Node,
    graph: &StorageGraph,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::RemoveDevice
        || action.context.collection.as_deref() != Some("mdRaids")
    {
        return None;
    }
    let device = action.context.device.as_deref()?;

    if node.kind != NodeKind::MdRaid {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MdMemberRemoveRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not an MD RAID array; mdadm --remove remains actionable only after target review",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    if md_array_has_member(graph, node, device) {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MdMemberRemoveRequired,
            query: query.to_string(),
            message: format!("MD RAID array {query} still includes member {device}"),
            current: Some(current_node_summary(node)),
        });
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::MdMemberRemoveAlreadySatisfied,
        query: query.to_string(),
        message: format!("MD RAID array {query} no longer includes member {device}"),
        current: Some(current_node_summary(node)),
    })
}

fn md_member_replace_diagnostic(
    action: &PlannedAction,
    node: &Node,
    graph: &StorageGraph,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::ReplaceDevice
        || action.context.collection.as_deref() != Some("mdRaids")
    {
        return None;
    }
    let old_device = action.context.device.as_deref()?;
    let new_device = action.context.replacement.as_deref()?;

    if node.kind != NodeKind::MdRaid {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MdMemberReplaceRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not an MD RAID array; mdadm replacement remains actionable only after target review",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    let old_present = md_array_has_member(graph, node, old_device);
    let new_present = md_array_has_member(graph, node, new_device);
    match (old_present, new_present) {
        (false, true) => Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Info,
            kind: TopologyDiagnosticKind::MdMemberReplaceAlreadySatisfied,
            query: query.to_string(),
            message: format!(
                "MD RAID array {query} already replaced member {old_device} with {new_device}"
            ),
            current: Some(current_node_summary(node)),
        }),
        (true, true) => Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MdMemberReplaceRequired,
            query: query.to_string(),
            message: format!(
                "MD RAID array {query} still includes old member {old_device} and already includes replacement {new_device}; review before removing the old member"
            ),
            current: Some(current_node_summary(node)),
        }),
        (true, false) => Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MdMemberReplaceRequired,
            query: query.to_string(),
            message: format!(
                "MD RAID array {query} still includes old member {old_device} and does not include replacement {new_device}"
            ),
            current: Some(current_node_summary(node)),
        }),
        (false, false) => Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MdMemberReplaceRequired,
            query: query.to_string(),
            message: format!(
                "MD RAID array {query} no longer includes old member {old_device}, but replacement {new_device} is not attached"
            ),
            current: Some(current_node_summary(node)),
        }),
    }
}

fn md_array_has_member(graph: &StorageGraph, array: &Node, device: &str) -> bool {
    graph.edges.iter().any(|edge| {
        edge.relationship == Relationship::MemberOf
            && edge.to == array.id
            && graph
                .nodes
                .iter()
                .find(|node| node.id == edge.from)
                .is_some_and(|member| member.matches(device))
    })
}

struct MdArrayStatus<'a> {
    state: &'a str,
    degraded_devices: u64,
    failed_devices: u64,
    active: bool,
    cleanly_active: bool,
}

fn md_array_status(node: &Node) -> Option<MdArrayStatus<'_>> {
    let state = property_value_from_node(node, "md.state")?;
    let degraded_devices = md_device_count_property(node, "md.degraded-devices")?;
    let failed_devices = md_device_count_property(node, "md.failed-devices")?;
    let state_indicates_active = md_state_indicates_active(state);
    Some(MdArrayStatus {
        state,
        degraded_devices,
        failed_devices,
        active: state_indicates_active,
        cleanly_active: state_indicates_active && degraded_devices == 0 && failed_devices == 0,
    })
}
