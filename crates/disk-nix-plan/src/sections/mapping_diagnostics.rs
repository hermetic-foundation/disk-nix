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

fn mount_options_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Remount {
        return None;
    }
    let desired_options = parse_mount_option_map(action.context.options.as_deref()?);
    if desired_options.is_empty() {
        return None;
    }
    let current_options = current_mount_option_map(node);
    if current_options.is_empty() {
        return None;
    }

    let missing_or_different = option_differences(&desired_options, &current_options);

    let (level, kind, message) = if missing_or_different.is_empty() {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::MountOptionsAlreadySatisfied,
            format!("mountpoint {query} already includes desired remount options"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::MountOptionsDiffer,
            format!(
                "mountpoint {query} is missing or differs on desired options: {}",
                missing_or_different.join(",")
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

fn mount_absent_diagnostic(action: &PlannedAction, query: &str) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Mount || !is_mount_collection(action) {
        return None;
    }

    let source = action
        .context
        .device
        .as_deref()
        .unwrap_or("<unspecified-source>");
    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::MountRequired,
        query: query.to_string(),
        message: format!(
            "mountpoint {query} is absent from current topology; mounting source {source} remains actionable"
        ),
        current: None,
    })
}

fn unmount_absent_diagnostic(action: &PlannedAction, query: &str) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Unmount || !is_mount_collection(action) {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::UnmountAlreadySatisfied,
        query: query.to_string(),
        message: format!("mountpoint {query} is already absent from current topology"),
        current: None,
    })
}

fn unmount_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Unmount || !is_mount_collection(action) {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::UnmountRequired,
        query: query.to_string(),
        message: format!("mountpoint {query} is currently mounted"),
        current: Some(current_node_summary(node)),
    })
}

fn nfs_export_absent_diagnostic(action: &PlannedAction, query: &str) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Export
        || action.context.collection.as_deref() != Some("exports")
    {
        return None;
    }

    let desired_client = action.context.client.as_deref().unwrap_or("<any-client>");
    let options = action
        .context
        .options
        .as_deref()
        .filter(|options| !options.is_empty())
        .unwrap_or("<default-options>");
    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::NfsExportRequired,
        query: query.to_string(),
        message: format!(
            "NFS export {query} is absent; export for {desired_client} with options {options} remains actionable"
        ),
        current: None,
    })
}

fn nfs_export_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Export
        || action.context.collection.as_deref() != Some("exports")
    {
        return None;
    }
    let desired_client = action.context.client.as_deref()?;
    let desired_options = parse_mount_option_map(action.context.options.as_deref()?);
    if desired_options.is_empty() {
        return None;
    }
    let current_client = property_value_from_node(node, "nfs.export-client")?;
    let current_options = current_nfs_export_option_map(node);
    if current_options.is_empty() {
        return None;
    }

    let mut differences = Vec::new();
    if current_client != desired_client {
        differences.push(format!("client={desired_client}"));
    }
    differences.extend(option_differences(&desired_options, &current_options));

    let (level, kind, message) = if differences.is_empty() {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::NfsExportAlreadySatisfied,
            format!("NFS export {query} already grants {desired_client} desired options"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::NfsExportDiffers,
            format!(
                "NFS export {query} differs from desired client/options: {}",
                differences.join(",")
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

fn nfs_unexport_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Unexport
        || action.context.collection.as_deref() != Some("exports")
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::NfsUnexportAlreadySatisfied,
        query: query.to_string(),
        message: format!("NFS export {query} is already absent from current topology"),
        current: None,
    })
}

fn nfs_unexport_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Unexport
        || action.context.collection.as_deref() != Some("exports")
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::NfsUnexportRequired,
        query: query.to_string(),
        message: format!("NFS export {query} is currently published"),
        current: Some(current_node_summary(node)),
    })
}

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
