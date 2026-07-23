fn vdo_destroy_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Destroy
        || action.context.collection.as_deref() != Some("vdoVolumes")
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::VdoDestroyAlreadySatisfied,
        query: query.to_string(),
        message: format!("VDO volume {query} is already absent from current topology"),
        current: None,
    })
}

fn vdo_destroy_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Destroy
        || action.context.collection.as_deref() != Some("vdoVolumes")
    {
        return None;
    }

    let details = vdo_destroy_details(node);
    let message = if details.is_empty() {
        format!("VDO volume {query} is still present")
    } else {
        format!(
            "VDO volume {query} is still present with {}",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::VdoDestroyRequired,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn vdo_create_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Create
        || action.context.collection.as_deref() != Some("vdoVolumes")
    {
        return None;
    }

    let message = if node.kind == NodeKind::VdoVolume {
        let details = vdo_destroy_details(node);
        if details.is_empty() {
            format!(
                "VDO create target {query} already has VDO metadata; create remains destructive and requires review"
            )
        } else {
            format!(
                "VDO create target {query} already has VDO metadata with {}; create remains destructive and requires review",
                details.join(", ")
            )
        }
    } else {
        format!(
            "VDO create target {query} matched current {} node {}; create remains destructive and requires review",
            node.kind, node.name
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::VdoCreateTargetPresent,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn vdo_destroy_details(node: &Node) -> Vec<String> {
    [
        ("vdo.operating-mode", "operating mode"),
        ("vdo.logical-size", "logical size"),
        ("vdo.physical-size", "physical size"),
        ("vdo.storage-device", "backing device"),
        ("vdo.backing-device", "backing device"),
        ("vdo.write-policy", "write policy"),
        ("lvm.vdo-operating-mode", "operating mode"),
        ("lvm.vdo-logical-size", "logical size"),
        ("lvm.vdo-physical-size", "physical size"),
        ("lvm.vdo-used-size", "used"),
        ("lvm.vdo-used", "used"),
        ("lvm.vdo-saving-percent", "saving"),
        ("lvm.vdo-write-policy", "write policy"),
    ]
    .into_iter()
    .filter_map(|(property, label)| {
        property_value_from_node(node, property).map(|value| format!("{label} {value}"))
    })
    .collect()
}

fn vdo_grow_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Grow
        || action.context.collection.as_deref() != Some("vdoVolumes")
        || node.size_bytes.is_some()
    {
        return None;
    }

    let desired = action.context.desired_size.as_deref()?;
    let desired_bytes = parse_size_bytes(desired);
    let current = vdo_logical_size(node);

    let (level, kind, message) = match (desired_bytes, current) {
        (Some(desired_bytes), Some((current, current_bytes))) if current_bytes >= desired_bytes => {
            (
                TopologyDiagnosticLevel::Info,
                TopologyDiagnosticKind::SizeAlreadySatisfied,
                format!(
                    "VDO volume {query} logical size {current} already satisfies desired size {desired}"
                ),
            )
        }
        (Some(_), Some((current, _))) => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::SizeBelowDesired,
            format!("VDO volume {query} logical size {current} is below desired size {desired}"),
        ),
        (None, _) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::VdoGrowRequired,
            format!(
                "VDO volume {query} desired size {desired} could not be parsed; grow remains actionable"
            ),
        ),
        (Some(_), None) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::VdoGrowRequired,
            format!(
                "VDO volume {query} current logical size is unknown; grow to {desired} remains actionable"
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

fn vdo_grow_absent_diagnostic(action: &PlannedAction, query: &str) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Grow
        || action.context.collection.as_deref() != Some("vdoVolumes")
    {
        return None;
    }

    let desired = action
        .context
        .desired_size
        .as_deref()
        .unwrap_or("<unspecified-size>");
    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::VdoGrowRequired,
        query: query.to_string(),
        message: format!(
            "VDO volume {query} is absent from current topology; grow to {desired} requires an existing VDO volume"
        ),
        current: None,
    })
}

fn vdo_start_stop_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("vdoVolumes") {
        return None;
    }

    match action.operation {
        Operation::Start => Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::VdoStartRequired,
            query: query.to_string(),
            message: format!(
                "VDO volume {query} is absent from current topology; start requires an existing VDO volume"
            ),
            current: None,
        }),
        Operation::Stop => Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Info,
            kind: TopologyDiagnosticKind::VdoStopAlreadySatisfied,
            query: query.to_string(),
            message: format!("VDO volume {query} is already stopped or absent"),
            current: None,
        }),
        _ => None,
    }
}

fn vdo_logical_size(node: &Node) -> Option<(&str, u64)> {
    ["vdo.logical-size", "lvm.vdo-logical-size"]
        .into_iter()
        .find_map(|property| {
            let value = property_value_from_node(node, property)?;
            parse_size_bytes(value).map(|bytes| (value, bytes))
        })
}

fn vdo_start_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Start
        || action.context.collection.as_deref() != Some("vdoVolumes")
    {
        return None;
    }
    let operating_mode = vdo_operating_mode(node)?;
    let normal = operating_mode.eq_ignore_ascii_case("normal");
    let (level, kind, message) = if normal {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::VdoStartAlreadySatisfied,
            format!("VDO volume {query} is already running in normal mode"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::VdoStartRequired,
            format!("VDO volume {query} operating mode is {operating_mode}, desired normal"),
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

fn vdo_stop_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Stop
        || action.context.collection.as_deref() != Some("vdoVolumes")
    {
        return None;
    }
    let operating_mode = vdo_operating_mode(node)?;
    let stopped = vdo_operating_mode_is_stopped(operating_mode);
    let (level, kind, message) = if stopped {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::VdoStopAlreadySatisfied,
            format!("VDO volume {query} is already stopped"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::VdoStopRequired,
            format!("VDO volume {query} operating mode is {operating_mode}, desired stopped"),
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

fn vdo_operating_mode(node: &Node) -> Option<&str> {
    property_value_from_node(node, "vdo.operating-mode")
        .or_else(|| property_value_from_node(node, "lvm.vdo-operating-mode"))
}
