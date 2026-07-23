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
