fn lvm_activate_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Activate || !is_lvm_activation_collection(action) {
        return None;
    }
    let active = lvm_node_is_active(node)?;
    let (level, kind, message) = if active {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LvmActivateAlreadySatisfied,
            format!("LVM object {query} is already active"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LvmActivateRequired,
            format!("LVM object {query} is known but not active"),
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

fn lvm_deactivate_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Deactivate || !is_lvm_activation_collection(action) {
        return None;
    }
    let active = lvm_node_is_active(node)?;
    let (level, kind, message) = if active {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LvmDeactivateRequired,
            format!("LVM object {query} is still active"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LvmDeactivateAlreadySatisfied,
            format!("LVM object {query} is already inactive"),
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

fn lvm_activation_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if !is_lvm_activation_collection(action) {
        return None;
    }

    match action.operation {
        Operation::Activate => Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::LvmActivateRequired,
            query: query.to_string(),
            message: format!(
                "LVM object {query} is absent from current topology; activation requires an existing LVM object"
            ),
            current: None,
        }),
        Operation::Deactivate => Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Info,
            kind: TopologyDiagnosticKind::LvmDeactivateAlreadySatisfied,
            query: query.to_string(),
            message: format!("LVM object {query} is already inactive or absent"),
            current: None,
        }),
        _ => None,
    }
}

fn lvm_volume_create_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Create
        || !matches!(
            action.context.collection.as_deref(),
            Some("volumes" | "thinPools")
        )
    {
        return None;
    }

    let (expected_kind, label, command) = match action.context.collection.as_deref() {
        Some("volumes") => (NodeKind::LvmLogicalVolume, "logical volume", "lvcreate"),
        Some("thinPools") => (
            NodeKind::LvmThinPool,
            "thin pool",
            "lvcreate --type thin-pool",
        ),
        _ => return None,
    };

    if node.kind != expected_kind {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::LvmVolumeCreateRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not the expected LVM {label}; {command} would create a new object",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    let desired = action.context.desired_size.as_deref();
    let desired_bytes = desired.and_then(parse_size_bytes);
    let (level, kind, message) = match (desired, desired_bytes, node.size_bytes) {
        (Some(desired), Some(desired_bytes), Some(current_bytes))
            if current_bytes == desired_bytes =>
        {
            (
                TopologyDiagnosticLevel::Info,
                TopologyDiagnosticKind::LvmVolumeCreateAlreadySatisfied,
                format!(
                    "LVM {label} {query} already exists with desired size {desired} ({current_bytes} bytes)"
                ),
            )
        }
        (None, _, _) => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LvmVolumeCreateAlreadySatisfied,
            format!("LVM {label} {query} already exists"),
        ),
        (Some(desired), Some(_), Some(current_bytes)) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LvmVolumeCreateRequired,
            format!(
                "LVM {label} {query} already exists with size {current_bytes} bytes, not desired size {desired}; use grow or shrink lifecycle instead of create when preserving data"
            ),
        ),
        (Some(desired), None, _) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LvmVolumeCreateRequired,
            format!(
                "LVM {label} {query} already exists, but desired size {desired} could not be compared"
            ),
        ),
        (Some(desired), Some(_), None) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LvmVolumeCreateRequired,
            format!(
                "LVM {label} {query} already exists, but current size is unknown; desired size is {desired}"
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

fn lvm_pv_create_diagnostic(
    action: &PlannedAction,
    matches: &[&Node],
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Create
        || action.context.collection.as_deref() != Some("physicalVolumes")
    {
        return None;
    }

    if let Some(pv_node) = matches
        .iter()
        .copied()
        .find(|node| node.kind == NodeKind::LvmPhysicalVolume)
    {
        let review_reasons = lvm_pv_review_reasons(pv_node);
        let (level, kind, message) = if review_reasons.is_empty() {
            (
                TopologyDiagnosticLevel::Info,
                TopologyDiagnosticKind::LvmPvCreateAlreadySatisfied,
                format!("physical volume {query} already has LVM PV metadata"),
            )
        } else {
            (
                TopologyDiagnosticLevel::Warning,
                TopologyDiagnosticKind::LvmPvCreateRequired,
                format!(
                    "physical volume {query} already exists, but metadata needs review: {}",
                    review_reasons.join(", ")
                ),
            )
        };

        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level,
            kind,
            query: query.to_string(),
            message,
            current: Some(current_node_summary(pv_node)),
        });
    }

    let node = matches[0];
    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::LvmPvCreateRequired,
        query: query.to_string(),
        message: format!(
            "matched current {} node {}, but it is not an LVM physical volume; pvcreate would write PV metadata",
            node.kind, node.name
        ),
        current: Some(current_node_summary(node)),
    })
}

fn lvm_vg_import_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Import
        || action.context.collection.as_deref() != Some("volumeGroups")
        || node.kind != NodeKind::LvmVolumeGroup
    {
        return None;
    }
    let exported = lvm_vg_is_exported(node);
    let (level, kind, message) = if exported {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LvmVgImportRequired,
            format!("LVM volume group {query} is visible but still exported"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LvmVgImportAlreadySatisfied,
            format!("LVM volume group {query} is already imported"),
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

fn lvm_vg_create_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Create
        || action.context.collection.as_deref() != Some("volumeGroups")
    {
        return None;
    }

    if node.kind != NodeKind::LvmVolumeGroup {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::LvmVgCreateRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not an LVM volume group; vgcreate would write VG metadata",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    let review_reasons = lvm_vg_review_reasons(node);
    let (level, kind, message) = if review_reasons.is_empty() {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LvmVgCreateAlreadySatisfied,
            format!("volume group {query} already exists"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LvmVgCreateRequired,
            format!(
                "volume group {query} already exists, but metadata needs review before treating create as satisfied: {}",
                review_reasons.join(", ")
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

fn lvm_vg_export_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Export
        || action.context.collection.as_deref() != Some("volumeGroups")
        || node.kind != NodeKind::LvmVolumeGroup
    {
        return None;
    }
    let exported = lvm_vg_is_exported(node);
    let (level, kind, message) = if exported {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LvmVgExportAlreadySatisfied,
            format!("LVM volume group {query} is already exported"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LvmVgExportRequired,
            format!("LVM volume group {query} is visible but not exported"),
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

fn lvm_rename_absent_diagnostic(
    action: &PlannedAction,
    graph: &StorageGraph,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Rename {
        return None;
    }
    let (expected_kind, label) = lvm_rename_expected_kind(action)?;
    let destination = lvm_rename_destination(action, query)?;
    let destination_matches = graph.find_nodes(&destination);
    if let Some(node) = destination_matches
        .iter()
        .copied()
        .find(|node| node.kind == expected_kind)
    {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Info,
            kind: TopologyDiagnosticKind::LvmRenameAlreadySatisfied,
            query: query.to_string(),
            message: format!(
                "LVM {label} rename from {query} to {destination} is already reflected in current topology"
            ),
            current: Some(current_node_summary(node)),
        });
    }

    if let Some(node) = destination_matches.first().copied() {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::LvmRenameRequired,
            query: query.to_string(),
            message: format!(
                "LVM {label} rename source {query} is missing, but destination {destination} matched current {} node {}; rename remains actionable for review",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::LvmRenameRequired,
        query: query.to_string(),
        message: format!(
            "LVM {label} rename source {query} is missing and destination {destination} is absent; rename remains actionable after LVM metadata review"
        ),
        current: None,
    })
}

fn lvm_rename_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Rename {
        return None;
    }
    let (expected_kind, label) = lvm_rename_expected_kind(action)?;
    if node.kind != expected_kind {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::LvmRenameRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not an LVM {label}; rename remains actionable",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    let destination =
        lvm_rename_destination(action, query).unwrap_or_else(|| "<rename-target>".to_string());
    let details = lvm_rename_details(node);
    let message = if details.is_empty() {
        format!(
            "LVM {label} rename source {query} is present; rename to {destination} remains offline-required"
        )
    } else {
        format!(
            "LVM {label} rename source {query} is present with {}; rename to {destination} remains offline-required",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::LvmRenameRequired,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn lvm_rename_expected_kind(action: &PlannedAction) -> Option<(NodeKind, &'static str)> {
    match action.context.collection.as_deref() {
        Some("volumes") => Some((NodeKind::LvmLogicalVolume, "logical volume")),
        Some("thinPools") => Some((NodeKind::LvmThinPool, "thin pool")),
        Some("volumeGroups") => Some((NodeKind::LvmVolumeGroup, "volume group")),
        _ => None,
    }
}

fn lvm_rename_destination(action: &PlannedAction, query: &str) -> Option<String> {
    let rename_to = action.context.rename_to.as_deref()?;
    match action.context.collection.as_deref() {
        Some("volumes" | "thinPools") if !rename_to.contains('/') => query
            .split_once('/')
            .map(|(vg, _)| format!("{vg}/{rename_to}"))
            .or_else(|| Some(rename_to.to_string())),
        _ => Some(rename_to.to_string()),
    }
}

fn lvm_rename_details(node: &Node) -> Vec<String> {
    let mut details = Vec::new();
    if let Some(size) = node.size_bytes {
        details.push(format!("size {size} bytes"));
    }
    for (property, label) in [
        ("lvm.vg-name", "vg"),
        ("lvm.lv-role", "role"),
        ("lvm.lv-layout", "layout"),
        ("lvm.lv-active", "active"),
        ("lvm.lv-attr", "attributes"),
        ("lvm.pool", "pool"),
        ("lvm.origin", "origin"),
        ("lvm.data-percent", "data"),
        ("lvm.metadata-percent", "metadata"),
        ("lvm.vg-exported", "exported"),
        ("lvm.vg-partial", "partial"),
    ] {
        if let Some(value) = property_value_from_node(node, property) {
            details.push(format!("{label} {value}"));
        }
    }
    details
}

fn lvm_cache_detach_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("lvmCaches")
        || !matches!(
            action.operation,
            Operation::Destroy | Operation::RemoveDevice
        )
        || !matches!(node.kind, NodeKind::LvmCache | NodeKind::LvmLogicalVolume)
    {
        return None;
    }

    let attached = lvm_cache_is_attached(node);
    let (level, kind, message) = if attached {
        let details = lvm_cache_detach_details(node);
        let message = if details.is_empty() {
            format!("LVM cache remains attached to origin {query}")
        } else {
            format!(
                "LVM cache remains attached to origin {query} with {}",
                details.join(", ")
            )
        };
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LvmCacheDetachRequired,
            message,
        )
    } else {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LvmCacheDetachAlreadySatisfied,
            format!("LVM cache is already detached from origin {query}"),
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

fn lvm_cache_detach_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("lvmCaches")
        || !matches!(
            action.operation,
            Operation::Destroy | Operation::RemoveDevice
        )
    {
        return None;
    }

    let device = action
        .context
        .device
        .as_deref()
        .unwrap_or("<unknown-cache-device>");
    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::LvmCacheDetachRequired,
        query: query.to_string(),
        message: format!(
            "LVM cache origin {query} is absent from current topology; detach of cache device {device} remains actionable after LVM metadata review"
        ),
        current: None,
    })
}

fn lvm_cache_is_attached(node: &Node) -> bool {
    node.kind == NodeKind::LvmCache
        || [
            "lvm.pool",
            "lvm.cache-mode",
            "lvm.cache-policy",
            "lvm.cache-dirty-blocks",
            "lvm.cache-total-blocks",
            "lvm.writecache-writeback-blocks",
            "lvm.writecache-total-blocks",
        ]
        .iter()
        .any(|property| property_value_from_node(node, property).is_some())
}

fn lvm_cache_detach_details(node: &Node) -> Vec<String> {
    [
        ("lvm.pool", "cache pool"),
        ("lvm.cache-mode", "cache mode"),
        ("lvm.cache-policy", "cache policy"),
        ("lvm.cache-dirty-blocks", "dirty blocks"),
        ("lvm.cache-total-blocks", "cache blocks"),
        ("lvm.cache-used-blocks", "used cache blocks"),
        ("lvm.writecache-writeback-blocks", "writeback blocks"),
        ("lvm.writecache-total-blocks", "writecache blocks"),
        ("lvm.writecache-free-blocks", "free writecache blocks"),
        ("lvm.data-percent", "data percent"),
        ("lvm.metadata-percent", "metadata percent"),
        ("lvm.health", "health"),
        ("lvm.attr", "LV attributes"),
    ]
    .into_iter()
    .filter_map(|(property, label)| {
        property_value_from_node(node, property).map(|value| format!("{label} {value}"))
    })
    .collect()
}
