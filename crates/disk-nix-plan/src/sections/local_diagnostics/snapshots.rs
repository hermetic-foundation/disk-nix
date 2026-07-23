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
