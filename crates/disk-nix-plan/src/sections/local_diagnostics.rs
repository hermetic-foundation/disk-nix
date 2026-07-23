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

fn luks_open_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Open
        || action.context.collection.as_deref() != Some("luks.devices")
    {
        return None;
    }
    let active = luks_node_is_active(node)?;
    let (level, kind, message) = if active {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LuksOpenAlreadySatisfied,
            format!("LUKS mapper {query} is already active"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LuksOpenRequired,
            format!("LUKS mapper {query} is known but not active"),
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

fn luks_absent_diagnostic(action: &PlannedAction, query: &str) -> Option<TopologyDiagnostic> {
    match action.context.collection.as_deref() {
        Some("luks.devices") => match action.operation {
            Operation::Open => {
                let backing = action
                    .context
                    .device
                    .as_deref()
                    .unwrap_or("<unspecified-backing-device>");
                Some(TopologyDiagnostic {
                    action_id: action.id.clone(),
                    level: TopologyDiagnosticLevel::Warning,
                    kind: TopologyDiagnosticKind::LuksOpenRequired,
                    query: query.to_string(),
                    message: format!(
                        "LUKS mapper {query} is absent from current topology; opening backing device {backing} remains actionable"
                    ),
                    current: None,
                })
            }
            Operation::Close => Some(TopologyDiagnostic {
                action_id: action.id.clone(),
                level: TopologyDiagnosticLevel::Info,
                kind: TopologyDiagnosticKind::LuksCloseAlreadySatisfied,
                query: query.to_string(),
                message: format!("LUKS mapper {query} is already inactive or absent"),
                current: None,
            }),
            _ => None,
        },
        Some("luksKeyslots")
            if matches!(action.operation, Operation::Destroy | Operation::RemoveKey) =>
        {
            let key_slot = action
                .context
                .key_slot
                .as_deref()
                .unwrap_or("<unknown-slot>");
            let backing = action
                .context
                .device
                .as_deref()
                .unwrap_or("<unspecified-backing-device>");
            Some(TopologyDiagnostic {
                action_id: action.id.clone(),
                level: TopologyDiagnosticLevel::Warning,
                kind: TopologyDiagnosticKind::LuksKeyslotRemoveRequired,
                query: query.to_string(),
                message: format!(
                    "LUKS container {query} is absent from current topology; keyslot {key_slot} removal on backing device {backing} remains actionable after header review"
                ),
                current: None,
            })
        }
        Some("luksTokens")
            if matches!(
                action.operation,
                Operation::Destroy | Operation::RemoveToken
            ) =>
        {
            let token_id = action
                .context
                .token_id
                .as_deref()
                .unwrap_or("<unknown-token>");
            let backing = action
                .context
                .device
                .as_deref()
                .unwrap_or("<unspecified-backing-device>");
            Some(TopologyDiagnostic {
                action_id: action.id.clone(),
                level: TopologyDiagnosticLevel::Warning,
                kind: TopologyDiagnosticKind::LuksTokenRemoveRequired,
                query: query.to_string(),
                message: format!(
                    "LUKS container {query} is absent from current topology; token {token_id} removal on backing device {backing} remains actionable after header review"
                ),
                current: None,
            })
        }
        _ => None,
    }
}

fn luks_format_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Format
        || action.context.collection.as_deref() != Some("luks.devices")
    {
        return None;
    }

    let message = if node.kind == NodeKind::LuksContainer {
        let details = luks_format_present_details(node);
        if details.is_empty() {
            format!(
                "LUKS format target {query} already contains a LUKS container; format remains destructive and requires review"
            )
        } else {
            format!(
                "LUKS format target {query} already contains a LUKS container with {}; format remains destructive and requires review",
                details.join(", ")
            )
        }
    } else {
        format!(
            "LUKS format target {query} matched current {} node {}; format remains destructive and requires review",
            node.kind, node.name
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::LuksFormatTargetPresent,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn luks_format_present_details(node: &Node) -> Vec<String> {
    [
        ("cryptsetup.luks-version", "version"),
        ("cryptsetup.uuid", "UUID"),
        ("cryptsetup.luks-uuid", "UUID"),
        ("cryptsetup.label", "label"),
        ("cryptsetup.luks-label", "label"),
        ("cryptsetup.luks-keyslot-count", "keyslots"),
        ("cryptsetup.luks-token-count", "tokens"),
        ("cryptsetup.active", "active"),
    ]
    .into_iter()
    .filter_map(|(property, label)| {
        property_value_from_node(node, property).map(|value| format!("{label} {value}"))
    })
    .collect()
}

fn luks_close_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Close
        || action.context.collection.as_deref() != Some("luks.devices")
    {
        return None;
    }
    let active = luks_node_is_active(node)?;
    let (level, kind, message) = if active {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LuksCloseRequired,
            format!("LUKS mapper {query} is known and still active"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LuksCloseAlreadySatisfied,
            format!("LUKS mapper {query} is already inactive"),
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

fn luks_keyslot_remove_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("luksKeyslots")
        || !matches!(action.operation, Operation::Destroy | Operation::RemoveKey)
        || node.kind != NodeKind::LuksContainer
    {
        return None;
    }
    let key_slot = action.context.key_slot.as_deref()?;
    let present = property_list_contains(
        property_value_from_node(node, "cryptsetup.luks-keyslots"),
        key_slot,
    );

    let (level, kind, message) = if present {
        let details = luks_keyslot_remove_details(node, key_slot);
        let message = if details.is_empty() {
            format!("LUKS keyslot {key_slot} is still present on {query}")
        } else {
            format!(
                "LUKS keyslot {key_slot} is still present on {query} with {}",
                details.join(", ")
            )
        };
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LuksKeyslotRemoveRequired,
            message,
        )
    } else {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LuksKeyslotRemoveAlreadySatisfied,
            format!("LUKS keyslot {key_slot} is already absent from {query}"),
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

fn luks_token_remove_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("luksTokens")
        || !matches!(
            action.operation,
            Operation::Destroy | Operation::RemoveToken
        )
        || node.kind != NodeKind::LuksContainer
    {
        return None;
    }
    let token_id = action.context.token_id.as_deref()?;
    let present = property_list_contains(
        property_value_from_node(node, "cryptsetup.luks-tokens"),
        token_id,
    );

    let (level, kind, message) = if present {
        let details = luks_token_remove_details(node, token_id);
        let message = if details.is_empty() {
            format!("LUKS token {token_id} is still present on {query}")
        } else {
            format!(
                "LUKS token {token_id} is still present on {query} with {}",
                details.join(", ")
            )
        };
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LuksTokenRemoveRequired,
            message,
        )
    } else {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LuksTokenRemoveAlreadySatisfied,
            format!("LUKS token {token_id} is already absent from {query}"),
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

fn luks_keyslot_remove_details(node: &Node, key_slot: &str) -> Vec<String> {
    let prefix = format!("cryptsetup.luks-keyslot-{key_slot}-");
    [
        ("type", "type"),
        ("priority", "priority"),
        ("cipher", "cipher"),
        ("cipher-key", "cipher key"),
        ("pbkdf", "PBKDF"),
        ("time-cost", "time cost"),
        ("memory", "memory"),
        ("threads", "threads"),
    ]
    .into_iter()
    .filter_map(|(suffix, label)| {
        property_value_from_node(node, &format!("{prefix}{suffix}"))
            .map(|value| format!("{label} {value}"))
    })
    .collect()
}

fn luks_token_remove_details(node: &Node, token_id: &str) -> Vec<String> {
    let prefix = format!("cryptsetup.luks-token-{token_id}-");
    [("type", "type"), ("keyslot", "keyslot")]
        .into_iter()
        .filter_map(|(suffix, label)| {
            property_value_from_node(node, &format!("{prefix}{suffix}"))
                .map(|value| format!("{label} {value}"))
        })
        .collect()
}

fn property_list_contains(values: Option<&str>, needle: &str) -> bool {
    values
        .into_iter()
        .flat_map(|values| values.split(','))
        .map(str::trim)
        .any(|value| value == needle)
}

fn bcache_absent_diagnostic(action: &PlannedAction, query: &str) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("caches")
        || action.operation != Operation::RemoveDevice
        || !is_concrete_bcache_target(query)
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::BcacheDetachAlreadySatisfied,
        query: query.to_string(),
        message: format!("bcache device {query} is already absent from current topology"),
        current: None,
    })
}

fn bcache_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("caches")
        || action.operation != Operation::RemoveDevice
    {
        return None;
    }

    let details = bcache_detach_details(node);
    let message = if details.is_empty() {
        format!("bcache device {query} is still present")
    } else {
        format!(
            "bcache device {query} is still present with {}",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::BcacheDetachRequired,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn bcache_detach_details(node: &Node) -> Vec<String> {
    [
        ("bcache.dirty-data", "dirty data"),
        ("bcache.cache-mode", "cache mode"),
        ("bcache.set-uuid", "cache set"),
    ]
    .into_iter()
    .filter_map(|(property, label)| {
        property_value_from_node(node, property).map(|value| format!("{label} {value}"))
    })
    .collect()
}

fn is_concrete_bcache_target(query: &str) -> bool {
    query.starts_with("/dev/bcache") || query.starts_with("block:/dev/bcache")
}

fn snapshot_clone_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("snapshots")
        || action.operation != Operation::Clone
        || !is_concrete_snapshot_target(query)
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::SnapshotCloneSourceMissing,
        query: query.to_string(),
        message: format!("snapshot clone source {query} is missing from current topology"),
        current: None,
    })
}

fn snapshot_clone_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("snapshots")
        || action.operation != Operation::Clone
    {
        return None;
    }

    let (label, details) = match node.kind {
        NodeKind::ZfsSnapshot => ("ZFS snapshot", zfs_snapshot_destroy_details(node)),
        NodeKind::BtrfsSnapshot => ("Btrfs snapshot", btrfs_subvolume_destroy_details(node)),
        _ => return None,
    };
    let destination = action.context.target.as_deref().unwrap_or("<clone-target>");
    let message = if details.is_empty() {
        format!("{label} clone source {query} is available for clone to {destination}")
    } else {
        format!(
            "{label} clone source {query} is available with {}; clone target {destination}",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::SnapshotCloneSourceAvailable,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn snapshot_destroy_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("snapshots")
        || action.operation != Operation::Destroy
        || !is_concrete_snapshot_target(query)
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::SnapshotDestroyAlreadySatisfied,
        query: query.to_string(),
        message: format!("snapshot {query} is already absent from current topology"),
        current: None,
    })
}

fn snapshot_destroy_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("snapshots")
        || action.operation != Operation::Destroy
    {
        return None;
    }

    let (label, details) = match node.kind {
        NodeKind::ZfsSnapshot => ("ZFS snapshot", zfs_snapshot_destroy_details(node)),
        NodeKind::BtrfsSnapshot => ("Btrfs snapshot", btrfs_subvolume_destroy_details(node)),
        _ => return None,
    };
    let message = if details.is_empty() {
        format!("{label} {query} is still present")
    } else {
        format!(
            "{label} {query} is still present with {}",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::SnapshotDestroyRequired,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn snapshot_rename_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("snapshots")
        || action.operation != Operation::Rename
        || !is_concrete_snapshot_target(query)
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::SnapshotRenameSourceMissing,
        query: query.to_string(),
        message: format!("snapshot rename source {query} is missing from current topology"),
        current: None,
    })
}

fn snapshot_rename_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("snapshots")
        || action.operation != Operation::Rename
    {
        return None;
    }

    let (label, details) = match node.kind {
        NodeKind::ZfsSnapshot => ("ZFS snapshot", zfs_snapshot_destroy_details(node)),
        NodeKind::BtrfsSnapshot => ("Btrfs snapshot", btrfs_subvolume_destroy_details(node)),
        _ => return None,
    };
    let destination = action
        .context
        .rename_to
        .as_deref()
        .unwrap_or("<rename-target>");
    let message = if details.is_empty() {
        format!(
            "{label} rename source {query} is present; rename to {destination} remains offline-required"
        )
    } else {
        format!(
            "{label} rename source {query} is present with {}; rename to {destination} remains offline-required",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::SnapshotRenameRequired,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn snapshot_hold_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("snapshots")
        || action.operation != Operation::SetProperty
        || node.kind != NodeKind::ZfsSnapshot
    {
        return None;
    }

    let property = action.context.property.as_deref()?;
    let tag = action.context.property_value.as_deref()?;
    let release = matches!(property, "zfs.releaseHold" | "releaseHold" | "release-hold");
    if !release && !matches!(property, "zfs.hold" | "hold" | "holdTag") {
        return None;
    }

    let present = zfs_snapshot_has_hold(node, tag);
    let already_satisfied = if release { !present } else { present };
    let (level, kind, message) = if already_satisfied {
        let action = if release { "released" } else { "held" };
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::PropertyAlreadySatisfied,
            format!("ZFS snapshot {query} is already {action} for tag {tag}"),
        )
    } else {
        let expectation = if release { "present" } else { "absent" };
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::PropertyDiffers,
            format!("ZFS snapshot {query} hold tag {tag} is {expectation}"),
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

fn zfs_snapshot_has_hold(node: &Node, tag: &str) -> bool {
    let tag_key = normalize_storage_property_name(tag);
    let tag_property = format!("zfs.hold.{tag_key}");
    property_value_from_node(node, &tag_property).is_some()
        || node.properties.iter().any(|property| {
            property.key == "zfs.holds"
                && property.value.split(',').any(|value| value.trim() == tag)
        })
}

fn snapshot_rollback_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("snapshots")
        || action.operation != Operation::Rollback
        || !is_concrete_zfs_snapshot_target(query)
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::SnapshotRollbackPointMissing,
        query: query.to_string(),
        message: format!("ZFS rollback snapshot {query} is missing from current topology"),
        current: None,
    })
}

fn snapshot_rollback_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("snapshots")
        || action.operation != Operation::Rollback
        || node.kind != NodeKind::ZfsSnapshot
    {
        return None;
    }

    let mut details = zfs_snapshot_destroy_details(node);
    if action.context.recursive_rollback == Some(true) {
        details.push("recursive rollback requested".to_string());
    }
    let message = if details.is_empty() {
        format!("ZFS rollback snapshot {query} is available; rollback remains potential data loss")
    } else {
        format!(
            "ZFS rollback snapshot {query} is available with {}; rollback remains potential data loss",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::SnapshotRollbackPointAvailable,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn is_concrete_snapshot_target(query: &str) -> bool {
    is_concrete_zfs_snapshot_target(query) || is_concrete_btrfs_snapshot_target(query)
}

fn is_concrete_zfs_snapshot_target(query: &str) -> bool {
    query.contains('@') && query.contains('/') && !query.starts_with('/')
}

fn is_concrete_btrfs_snapshot_target(query: &str) -> bool {
    query.starts_with('/')
}

fn btrfs_subvolume_destroy_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("btrfsSubvolumes")
        || action.operation != Operation::Destroy
        || !is_concrete_btrfs_subvolume_target(query)
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::BtrfsSubvolumeDestroyAlreadySatisfied,
        query: query.to_string(),
        message: format!("Btrfs subvolume {query} is already absent from current topology"),
        current: None,
    })
}

fn btrfs_subvolume_create_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("btrfsSubvolumes")
        || action.operation != Operation::Create
    {
        return None;
    }

    if node.kind != NodeKind::BtrfsSubvolume {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::BtrfsSubvolumeCreateRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not a Btrfs subvolume; btrfs subvolume create remains actionable",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    let details = btrfs_subvolume_destroy_details(node);
    let message = if details.is_empty() {
        format!("Btrfs subvolume {query} already exists")
    } else {
        format!(
            "Btrfs subvolume {query} already exists with {}",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::BtrfsSubvolumeCreateAlreadySatisfied,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn btrfs_subvolume_destroy_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("btrfsSubvolumes")
        || action.operation != Operation::Destroy
        || node.kind != NodeKind::BtrfsSubvolume
    {
        return None;
    }

    let details = btrfs_subvolume_destroy_details(node);
    let message = if details.is_empty() {
        format!("Btrfs subvolume {query} is still present")
    } else {
        format!(
            "Btrfs subvolume {query} is still present with {}",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::BtrfsSubvolumeDestroyRequired,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn btrfs_subvolume_destroy_details(node: &Node) -> Vec<String> {
    let mut details = [
        ("btrfs.id", "subvolume id"),
        ("btrfs.generation", "generation"),
        ("btrfs.created-generation", "created generation"),
        ("btrfs.parent-id", "parent id"),
        ("btrfs.top-level", "top level"),
        ("btrfs.received-uuid", "received UUID"),
        ("btrfs.parent-uuid", "parent UUID"),
    ]
    .into_iter()
    .filter_map(|(property, label)| {
        property_value_from_node(node, property).map(|value| format!("{label} {value}"))
    })
    .collect::<Vec<_>>();

    if let Some(uuid) = node.identity.uuid.as_deref() {
        details.push(format!("UUID {uuid}"));
    }

    details
}

fn is_concrete_btrfs_subvolume_target(query: &str) -> bool {
    query.starts_with('/')
}

fn btrfs_qgroup_destroy_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("btrfsQgroups")
        || action.operation != Operation::Destroy
        || !is_concrete_btrfs_qgroup_target(query)
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::BtrfsQgroupDestroyAlreadySatisfied,
        query: query.to_string(),
        message: format!("Btrfs qgroup {query} is already absent from current topology"),
        current: None,
    })
}

fn btrfs_qgroup_create_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("btrfsQgroups")
        || action.operation != Operation::Create
    {
        return None;
    }

    if node.kind != NodeKind::BtrfsQgroup {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::BtrfsQgroupCreateRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not a Btrfs qgroup; btrfs qgroup create remains actionable",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    let details = btrfs_qgroup_destroy_details(node);
    let message = if details.is_empty() {
        format!("Btrfs qgroup {query} already exists")
    } else {
        format!(
            "Btrfs qgroup {query} already exists with {}",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::BtrfsQgroupCreateAlreadySatisfied,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn btrfs_qgroup_destroy_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("btrfsQgroups")
        || action.operation != Operation::Destroy
        || node.kind != NodeKind::BtrfsQgroup
    {
        return None;
    }

    let details = btrfs_qgroup_destroy_details(node);
    let message = if details.is_empty() {
        format!("Btrfs qgroup {query} is still present")
    } else {
        format!(
            "Btrfs qgroup {query} is still present with {}",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::BtrfsQgroupDestroyRequired,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn btrfs_qgroup_destroy_details(node: &Node) -> Vec<String> {
    let mut details = [
        ("btrfs.qgroup-id", "qgroup id"),
        ("btrfs.max-referenced", "max referenced"),
        ("btrfs.max-exclusive", "max exclusive"),
        ("btrfs.qgroup-parents", "parents"),
        ("btrfs.qgroup-children", "children"),
    ]
    .into_iter()
    .filter_map(|(property, label)| {
        property_value_from_node(node, property).map(|value| format!("{label} {value}"))
    })
    .collect::<Vec<_>>();

    if let Some(used_bytes) = node.usage.as_ref().and_then(|usage| usage.used_bytes) {
        details.push(format!("referenced {used_bytes} bytes"));
    }
    if let Some(allocated_bytes) = node.usage.as_ref().and_then(|usage| usage.allocated_bytes) {
        details.push(format!("exclusive {allocated_bytes} bytes"));
    }

    details
}

fn is_concrete_btrfs_qgroup_target(query: &str) -> bool {
    let Some((level, id)) = query.split_once('/') else {
        return false;
    };

    !level.is_empty()
        && !id.is_empty()
        && level.chars().all(|character| character.is_ascii_digit())
        && id.chars().all(|character| character.is_ascii_digit())
}

fn dm_map_absent_diagnostic(action: &PlannedAction, query: &str) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("dmMaps")
        || action.operation != Operation::Destroy
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::DmMapDestroyAlreadySatisfied,
        query: query.to_string(),
        message: format!("device-mapper map {query} is already absent from current topology"),
        current: None,
    })
}

fn dm_map_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("dmMaps")
        || action.operation != Operation::Destroy
    {
        return None;
    }

    let message = match property_value_from_node(node, "dm.open-count") {
        Some(open_count) => {
            format!("device-mapper map {query} is still present with open count {open_count}")
        }
        None => format!("device-mapper map {query} is still present"),
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::DmMapDestroyRequired,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}
