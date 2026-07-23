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

fn zfs_object_destroy_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    let object_label = zfs_object_destroy_label(action)?;
    if action.operation != Operation::Destroy || !is_concrete_zfs_object_target(query) {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::ZfsObjectDestroyAlreadySatisfied,
        query: query.to_string(),
        message: format!("ZFS {object_label} {query} is already absent from current topology"),
        current: None,
    })
}

fn zfs_object_create_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    let object_label = zfs_object_destroy_label(action)?;
    let expected_kind = zfs_object_expected_kind(action)?;
    if action.operation != Operation::Create {
        return None;
    }

    if node.kind != expected_kind {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::ZfsObjectCreateRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not a ZFS {object_label}; zfs create remains actionable",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    if expected_kind == NodeKind::Zvol {
        if let Some(desired) = action.context.desired_size.as_deref() {
            match (parse_size_bytes(desired), node.size_bytes) {
                (Some(desired_bytes), Some(current_bytes)) if current_bytes >= desired_bytes => {}
                (Some(_), Some(current_bytes)) => {
                    return Some(TopologyDiagnostic {
                        action_id: action.id.clone(),
                        level: TopologyDiagnosticLevel::Warning,
                        kind: TopologyDiagnosticKind::ZfsObjectCreateRequired,
                        query: query.to_string(),
                        message: format!(
                            "ZFS zvol {query} already exists with size {current_bytes} bytes, not desired size {desired}; use grow or shrink lifecycle instead of create when preserving data"
                        ),
                        current: Some(current_node_summary(node)),
                    });
                }
                (Some(_), None) => {
                    return Some(TopologyDiagnostic {
                        action_id: action.id.clone(),
                        level: TopologyDiagnosticLevel::Warning,
                        kind: TopologyDiagnosticKind::ZfsObjectCreateRequired,
                        query: query.to_string(),
                        message: format!(
                            "ZFS zvol {query} already exists, but current size is unknown; use rescan or grow/shrink lifecycle instead of create when preserving data"
                        ),
                        current: Some(current_node_summary(node)),
                    });
                }
                (None, _) => {
                    return Some(TopologyDiagnostic {
                        action_id: action.id.clone(),
                        level: TopologyDiagnosticLevel::Warning,
                        kind: TopologyDiagnosticKind::ZfsObjectCreateRequired,
                        query: query.to_string(),
                        message: format!(
                            "ZFS zvol {query} already exists, but desired size {desired} could not be parsed; review before treating create as satisfied"
                        ),
                        current: Some(current_node_summary(node)),
                    });
                }
            }
        }
    }

    let details = zfs_object_destroy_details(node);
    let message = if details.is_empty() {
        format!("ZFS {object_label} {query} already exists")
    } else {
        format!(
            "ZFS {object_label} {query} already exists with {}",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::ZfsObjectCreateAlreadySatisfied,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn zfs_object_destroy_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    let object_label = zfs_object_destroy_label(action)?;
    if action.operation != Operation::Destroy {
        return None;
    }

    match (action.context.collection.as_deref(), node.kind) {
        (Some("datasets"), NodeKind::ZfsDataset) | (Some("zvols"), NodeKind::Zvol) => {}
        _ => return None,
    }

    let details = zfs_object_destroy_details(node);
    let message = if details.is_empty() {
        format!("ZFS {object_label} {query} is still present")
    } else {
        format!(
            "ZFS {object_label} {query} is still present with {}",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::ZfsObjectDestroyRequired,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn zfs_object_promote_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    let object_label = zfs_object_destroy_label(action)?;
    if action.operation != Operation::Promote {
        return None;
    }

    let expected_kind = zfs_object_expected_kind(action)?;
    if node.kind != expected_kind {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::ZfsObjectPromoteRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not a ZFS {object_label}; zfs promote remains actionable",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    if let Some(origin) = property_value_from_node(node, "zfs.origin") {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::ZfsObjectPromoteRequired,
            query: query.to_string(),
            message: format!("ZFS {object_label} {query} is still a clone of {origin}"),
            current: Some(current_node_summary(node)),
        });
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::ZfsObjectPromoteAlreadySatisfied,
        query: query.to_string(),
        message: format!("ZFS {object_label} {query} no longer reports a clone origin"),
        current: Some(current_node_summary(node)),
    })
}

fn zfs_object_rename_absent_diagnostic(
    action: &PlannedAction,
    graph: &StorageGraph,
    query: &str,
) -> Option<TopologyDiagnostic> {
    let object_label = zfs_object_destroy_label(action)?;
    if action.operation != Operation::Rename || !is_concrete_zfs_object_target(query) {
        return None;
    }

    let destination = action.context.rename_to.as_deref()?;
    let expected_kind = zfs_object_expected_kind(action)?;
    let destination_node = graph
        .find_nodes(destination)
        .into_iter()
        .find(|node| node.kind == expected_kind);
    if let Some(node) = destination_node {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Info,
            kind: TopologyDiagnosticKind::ZfsObjectRenameAlreadySatisfied,
            query: query.to_string(),
            message: format!(
                "ZFS {object_label} rename from {query} to {destination} is already reflected in current topology"
            ),
            current: Some(current_node_summary(node)),
        });
    }

    if let Some(node) = graph.find_nodes(destination).into_iter().next() {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::ZfsObjectRenameRequired,
            query: query.to_string(),
            message: format!(
                "ZFS {object_label} rename source {query} is missing, but destination {destination} matched current {} node {}; zfs rename remains actionable for review",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::ZfsObjectRenameRequired,
        query: query.to_string(),
        message: format!(
            "ZFS {object_label} rename source {query} is missing and destination {destination} is absent; zfs rename remains actionable after ZFS metadata review"
        ),
        current: None,
    })
}

fn zfs_object_rename_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    let object_label = zfs_object_destroy_label(action)?;
    if action.operation != Operation::Rename {
        return None;
    }

    let expected_kind = zfs_object_expected_kind(action)?;
    if node.kind != expected_kind {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::ZfsObjectRenameRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not a ZFS {object_label}; zfs rename remains actionable",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    let destination = action
        .context
        .rename_to
        .as_deref()
        .unwrap_or("<rename-target>");
    let details = zfs_object_destroy_details(node);
    let message = if details.is_empty() {
        format!(
            "ZFS {object_label} rename source {query} is present; rename to {destination} remains offline-required"
        )
    } else {
        format!(
            "ZFS {object_label} rename source {query} is present with {}; rename to {destination} remains offline-required",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::ZfsObjectRenameRequired,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn zfs_object_destroy_label(action: &PlannedAction) -> Option<&'static str> {
    match action.context.collection.as_deref() {
        Some("datasets") => Some("dataset"),
        Some("zvols") => Some("zvol"),
        _ => None,
    }
}

fn zfs_object_expected_kind(action: &PlannedAction) -> Option<NodeKind> {
    match action.context.collection.as_deref() {
        Some("datasets") => Some(NodeKind::ZfsDataset),
        Some("zvols") => Some(NodeKind::Zvol),
        _ => None,
    }
}

fn is_concrete_zfs_object_target(query: &str) -> bool {
    query.contains('/') && !query.starts_with('/')
}

fn zfs_object_destroy_details(node: &Node) -> Vec<String> {
    [
        ("zfs.type", "type"),
        ("zfs.mountpoint", "mountpoint"),
        ("zfs.origin", "origin"),
        ("zfs.used", "used"),
        ("zfs.available", "available"),
        ("zfs.referenced", "referenced"),
        ("zfs.quota", "quota"),
        ("zfs.reservation", "reservation"),
        ("zfs.volsize", "volsize"),
        ("zfs.encryption", "encryption"),
        ("zfs.keystatus", "key status"),
        ("zfs.compression", "compression"),
    ]
    .into_iter()
    .filter_map(|(property, label)| {
        property_value_from_node(node, property).map(|value| format!("{label} {value}"))
    })
    .collect()
}

fn zfs_snapshot_destroy_details(node: &Node) -> Vec<String> {
    let mut details = zfs_object_destroy_details(node);
    if let Some(userrefs) = property_value_from_node(node, "zfs.userrefs") {
        details.push(format!("user references {userrefs}"));
    }
    details
}

fn zfs_pool_create_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Create
        || action.context.collection.as_deref() != Some("pools")
    {
        return None;
    }

    if node.kind != NodeKind::ZfsPool {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::ZfsPoolCreateRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not a ZFS pool; zpool create remains actionable",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    let state = property_value_from_node(node, "zfs.state");
    let health = property_value_from_node(node, "zfs.health");
    let online =
        state.is_some_and(zfs_status_is_online) && health.is_some_and(zfs_status_is_online);
    if !online {
        let state = state.unwrap_or("unknown");
        let health = health.unwrap_or("unknown");
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::ZfsPoolCreateRequired,
            query: query.to_string(),
            message: format!(
                "ZFS pool {query} already exists, but pool state needs review before treating create as satisfied: state={state}, health={health}"
            ),
            current: Some(current_node_summary(node)),
        });
    }

    let details = zfs_pool_details(node);
    let message = if details.is_empty() {
        format!("ZFS pool {query} already exists and is online")
    } else {
        format!(
            "ZFS pool {query} already exists and is online with {}",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::ZfsPoolCreateAlreadySatisfied,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn zfs_pool_details(node: &Node) -> Vec<String> {
    [
        ("zfs.state", "state"),
        ("zfs.health", "health"),
        ("zfs.pool-capacity", "capacity"),
        ("zfs.pool-dedupratio", "dedup ratio"),
        ("zfs.pool-fragmentation", "fragmentation"),
        ("zfs.pool-altroot", "altroot"),
    ]
    .into_iter()
    .filter_map(|(property, label)| {
        property_value_from_node(node, property).map(|value| format!("{label} {value}"))
    })
    .collect()
}

fn zfs_pool_import_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Import
        || action.context.collection.as_deref() != Some("pools")
    {
        return None;
    }
    let state = property_value_from_node(node, "zfs.state")?;
    let health = property_value_from_node(node, "zfs.health")?;
    let online = zfs_status_is_online(state) && zfs_status_is_online(health);
    let (level, kind, message) = if online {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::ZfsPoolImportAlreadySatisfied,
            format!("ZFS pool {query} is already imported and online"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::ZfsPoolImportRequired,
            format!("ZFS pool {query} is visible but not online: state={state}, health={health}"),
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

fn nvme_namespace_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("nvmeNamespaces") {
        return None;
    }

    let (level, kind, message) = match action.operation {
        Operation::Attach => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::NvmeNamespaceAttachRequired,
            format!("NVMe namespace path {query} is not currently visible on this host"),
        ),
        Operation::Detach => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::NvmeNamespaceDetachAlreadySatisfied,
            format!("NVMe namespace path {query} is already absent from current topology"),
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

fn nvme_namespace_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("nvmeNamespaces") {
        return None;
    }

    let (level, kind, message) = match action.operation {
        Operation::Attach => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::NvmeNamespaceAttachAlreadySatisfied,
            format!("NVMe namespace path {query} is already visible on this host"),
        ),
        Operation::Detach => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::NvmeNamespaceDetachRequired,
            format!("NVMe namespace path {query} is still visible on this host"),
        ),
        _ => return None,
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

fn lun_absent_diagnostic(action: &PlannedAction, query: &str) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("luns") {
        return None;
    }

    let (level, kind, message) = match action.operation {
        Operation::Attach => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LunAttachRequired,
            format!("LUN path {query} is not currently visible on this host"),
        ),
        Operation::Detach => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LunDetachAlreadySatisfied,
            format!("LUN path {query} is already absent from current topology"),
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

fn lun_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("luns") {
        return None;
    }

    let (level, kind, message) = match action.operation {
        Operation::Attach => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LunAttachAlreadySatisfied,
            format!("LUN path {query} is already visible on this host"),
        ),
        Operation::Detach => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LunDetachRequired,
            format!("LUN path {query} is still visible on this host"),
        ),
        _ => return None,
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

fn iscsi_login_diagnostic(
    action: &PlannedAction,
    matches: &[&Node],
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Login
        || action.context.collection.as_deref() != Some("iscsiSessions")
    {
        return None;
    }

    let logged_in = matches
        .iter()
        .copied()
        .find(|node| iscsi_node_is_logged_in(node));
    let current = logged_in
        .or_else(|| matches.first().copied())
        .map(current_node_summary);
    let (level, kind, message) = if logged_in.is_some() {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::IscsiLoginAlreadySatisfied,
            format!("iSCSI target {query} already has a logged-in session"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::IscsiLoginRequired,
            format!("iSCSI target {query} is known but no logged-in session was matched"),
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level,
        kind,
        query: query.to_string(),
        message,
        current,
    })
}

fn iscsi_logout_diagnostic(
    action: &PlannedAction,
    matches: &[&Node],
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Logout
        || action.context.collection.as_deref() != Some("iscsiSessions")
    {
        return None;
    }

    let logged_in = matches
        .iter()
        .copied()
        .find(|node| iscsi_node_is_logged_in(node));
    let current = logged_in
        .or_else(|| matches.first().copied())
        .map(current_node_summary);
    let (level, kind, message) = if logged_in.is_some() {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::IscsiLogoutRequired,
            format!("iSCSI target {query} still has a logged-in session"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::IscsiLogoutAlreadySatisfied,
            format!("iSCSI target {query} has no logged-in session"),
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level,
        kind,
        query: query.to_string(),
        message,
        current,
    })
}

fn iscsi_node_is_logged_in(node: &Node) -> bool {
    property_value_from_node(node, "iscsi.connection-state")
        .or_else(|| property_value_from_node(node, "iscsi.session-state"))
        .is_some_and(is_logged_in_iscsi_state)
}

fn is_logged_in_iscsi_state(value: &str) -> bool {
    let normalized = value
        .trim()
        .chars()
        .filter(|character| {
            !character.is_ascii_whitespace() && *character != '-' && *character != '_'
        })
        .collect::<String>()
        .to_ascii_lowercase();
    normalized == "loggedin"
}

fn luks_node_is_active(node: &Node) -> Option<bool> {
    property_value_from_node(node, "cryptsetup.active").map(|value| value == "true")
}

fn is_lvm_activation_collection(action: &PlannedAction) -> bool {
    matches!(
        action.context.collection.as_deref(),
        Some("volumes" | "thinPools" | "lvmSnapshots")
    )
}

fn is_mount_collection(action: &PlannedAction) -> bool {
    matches!(
        action.context.collection.as_deref(),
        Some("filesystems" | "nfs.mounts")
    )
}

fn lvm_node_is_active(node: &Node) -> Option<bool> {
    property_value_from_node(node, "lvm.active").map(|value| {
        value
            .split_whitespace()
            .next()
            .is_some_and(|state| state.eq_ignore_ascii_case("active"))
    })
}

fn lvm_vg_is_exported(node: &Node) -> bool {
    property_value_from_node(node, "lvm.vg-exported").is_some_and(|value| {
        let normalized = value.trim();
        normalized.eq_ignore_ascii_case("exported")
            || normalized.eq_ignore_ascii_case("true")
            || normalized.eq_ignore_ascii_case("yes")
            || normalized == "1"
    })
}

fn lvm_pv_review_reasons(node: &Node) -> Vec<String> {
    [
        ("lvm.pv-missing", "PV is marked missing"),
        ("lvm.pv-duplicate", "PV is marked duplicate"),
    ]
    .iter()
    .filter_map(|(property, reason)| {
        property_value_from_node(node, property)
            .filter(|value| lvm_truthy_or_named_state(value, reason))
            .map(|value| format!("{reason} ({property}={value})"))
    })
    .collect()
}

fn lvm_vg_review_reasons(node: &Node) -> Vec<String> {
    let mut reasons = Vec::new();

    if let Some(value) = property_value_from_node(node, "lvm.vg-exported")
        .filter(|value| lvm_truthy_or_named_state(value, "VG is marked exported"))
    {
        reasons.push(format!("VG is marked exported (lvm.vg-exported={value})"));
    }

    if let Some(value) = property_value_from_node(node, "lvm.vg-partial")
        .filter(|value| lvm_truthy_or_named_state(value, "VG is marked partial"))
    {
        reasons.push(format!("VG is marked partial (lvm.vg-partial={value})"));
    }

    if let Some(count) = property_value_from_node(node, "lvm.missing-pv-count")
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|count| *count > 0)
    {
        reasons.push(format!("VG reports {count} missing physical volume(s)"));
    }

    reasons
}

fn lvm_truthy_or_named_state(value: &str, reason: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    let named_state = reason
        .trim_start_matches("PV is marked ")
        .trim_start_matches("VG is marked ")
        .to_ascii_lowercase();
    normalized == "1" || normalized == "true" || normalized == "yes" || normalized == named_state
}

fn md_state_indicates_active(value: &str) -> bool {
    value
        .split(|character: char| !character.is_ascii_alphanumeric())
        .any(|token| matches!(token.to_ascii_lowercase().as_str(), "clean" | "active"))
}

fn md_device_count_property(node: &Node, key: &str) -> Option<u64> {
    property_value_from_node(node, key).and_then(|value| value.trim().parse().ok())
}

fn zfs_status_is_online(value: &str) -> bool {
    value.trim().eq_ignore_ascii_case("online")
}

fn vdo_operating_mode_is_stopped(value: &str) -> bool {
    let normalized = value
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_whitespace() || character == '_' {
                '-'
            } else {
                character
            }
        })
        .collect::<String>()
        .to_ascii_lowercase();
    matches!(normalized.as_str(), "stopped" | "not-running" | "inactive")
}

fn property_value_from_node<'a>(node: &'a Node, key: &str) -> Option<&'a str> {
    node.properties
        .iter()
        .find(|property| property.key == key)
        .map(|property| property.value.as_str())
}

fn bcache_cache_set_property_diagnostic(
    action: &PlannedAction,
    node: &Node,
    graph: &StorageGraph,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::SetProperty
        || action.context.collection.as_deref() != Some("caches")
    {
        return None;
    }
    let property = action.context.property.as_deref()?;
    let property_key = bcache_cache_set_property_key(property)?;
    let desired = action.context.property_value.as_deref()?;
    let set_uuid = action
        .context
        .cache_set_uuid
        .as_deref()
        .or_else(|| property_value_from_node(node, "bcache.set-uuid"))?;
    let set_query = format!("bcache-set:{set_uuid}");
    let set_node = graph
        .find_nodes(&set_query)
        .into_iter()
        .next()
        .or_else(|| graph.find_nodes(set_uuid).into_iter().next())?;
    let current = property_value_from_node(set_node, &property_key)?;
    let (level, kind, message) = if current == desired {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::PropertyAlreadySatisfied,
            format!("cache-set property {property} already has desired value {desired}"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::PropertyDiffers,
            format!("cache-set property {property} is {current}, desired {desired}"),
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level,
        kind,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(set_node)),
    })
}

fn bcache_cache_set_property_key(property: &str) -> Option<String> {
    let normalized = normalize_storage_property_name(property);
    let known = match normalized.as_str() {
        "setaveragekeysize" => Some("average-key-size"),
        "setbtreecachesize" => Some("btree-cache-size"),
        "setcacheavailablepercent" => Some("cache-available-percent"),
        "setcongested" => Some("congested"),
        "setcongestedreadthresholdus" => Some("congested-read-threshold-us"),
        "setcongestedwritethresholdus" => Some("congested-write-threshold-us"),
        "setioerrorhalflife" => Some("io-error-halflife"),
        "setioerrorlimit" => Some("io-error-limit"),
        "setjournaldelayms" => Some("journal-delay-ms"),
        "setrootusagepercent" => Some("root-usage-percent"),
        _ => None,
    };
    if let Some(property) = known {
        return Some(format!("bcache.set-{property}"));
    }
    let property = normalized
        .strip_prefix("bcache-set-")
        .or_else(|| normalized.strip_prefix("set-"))?;
    Some(format!("bcache.set-{property}"))
}

fn current_mount_option_map(node: &Node) -> BTreeMap<String, String> {
    let mut options = property_value_from_node(node, "mount.options")
        .map(parse_mount_option_map)
        .unwrap_or_default();

    for property in &node.properties {
        if let Some(option) = property.key.strip_prefix("nfs.") {
            options
                .entry(normalize_mount_option_name(option))
                .or_insert_with(|| property.value.clone());
        }
    }
    if property_value_from_node(node, "mount.read-only") == Some("true") {
        options
            .entry("ro".to_string())
            .or_insert("true".to_string());
    }
    if property_value_from_node(node, "mount.read-write") == Some("true") {
        options
            .entry("rw".to_string())
            .or_insert("true".to_string());
    }
    if property_value_from_node(node, "mount.bind") == Some("true") {
        options
            .entry("bind".to_string())
            .or_insert("true".to_string());
    }

    options
}

fn current_nfs_export_option_map(node: &Node) -> BTreeMap<String, String> {
    node.properties
        .iter()
        .filter_map(|property| {
            property
                .key
                .strip_prefix("nfs.export-option-")
                .map(|option| (normalize_mount_option_name(option), property.value.clone()))
        })
        .filter(|(option, _)| !option.is_empty())
        .collect()
}

fn option_differences(
    desired_options: &BTreeMap<String, String>,
    current_options: &BTreeMap<String, String>,
) -> Vec<String> {
    desired_options
        .iter()
        .filter_map(|(option, desired)| match current_options.get(option) {
            Some(current) if current == desired => None,
            _ => Some(format!("{option}={desired}")),
        })
        .collect()
}

fn parse_mount_option_map(options: &str) -> BTreeMap<String, String> {
    options
        .split(',')
        .filter_map(|option| {
            let option = option.trim();
            if option.is_empty() {
                return None;
            }
            Some(option.split_once('=').map_or_else(
                || (normalize_mount_option_name(option), "true".to_string()),
                |(key, value)| (normalize_mount_option_name(key), value.trim().to_string()),
            ))
        })
        .filter(|(key, _)| !key.is_empty())
        .collect()
}

fn normalize_mount_option_name(option: &str) -> String {
    option
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|character| match character {
            'a'..='z' | '0'..='9' => character,
            _ => '-',
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
