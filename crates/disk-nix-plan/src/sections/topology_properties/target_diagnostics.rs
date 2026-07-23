fn current_node_summary(node: &Node) -> CurrentNodeSummary {
    CurrentNodeSummary {
        id: node.id.0.clone(),
        kind: node.kind,
        name: node.name.clone(),
        path: node.path.clone(),
        size_bytes: node.size_bytes,
    }
}

fn size_diagnostic(action: &PlannedAction, node: &Node, query: &str) -> Option<TopologyDiagnostic> {
    let desired = size_diagnostic_desired_size(action)?;
    let desired_bytes = parse_size_bytes(desired)?;
    let current_bytes = node.size_bytes?;

    let (level, kind, message) = match action.operation {
        Operation::Grow if current_bytes >= desired_bytes => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::SizeAlreadySatisfied,
            format!("current size {current_bytes} bytes already satisfies desired size {desired}"),
        ),
        Operation::Grow => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::SizeBelowDesired,
            format!("current size {current_bytes} bytes is below desired size {desired}"),
        ),
        Operation::Shrink if current_bytes <= desired_bytes => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::SizeAlreadySatisfied,
            format!(
                "current size {current_bytes} bytes is already at or below desired size {desired}"
            ),
        ),
        Operation::Shrink => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::SizeConflict,
            format!("current size {current_bytes} bytes is above desired shrink target {desired}"),
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

fn size_diagnostic_desired_size(action: &PlannedAction) -> Option<&str> {
    action.context.desired_size.as_deref().or_else(|| {
        if action.operation == Operation::Grow
            && action.context.collection.as_deref() == Some("partitions")
        {
            action.context.end.as_deref()
        } else {
            None
        }
    })
}

fn filesystem_type_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    let desired = action.context.fs_type.as_deref()?;
    let current = property_value_from_node(node, "filesystem.type")?;
    if current == desired {
        if action.operation == Operation::Format
            && action.context.collection.as_deref() == Some("filesystems")
        {
            return Some(TopologyDiagnostic {
                action_id: action.id.clone(),
                level: TopologyDiagnosticLevel::Info,
                kind: TopologyDiagnosticKind::FilesystemFormatAlreadySatisfied,
                query: query.to_string(),
                message: format!("filesystem {query} already reports type {current}"),
                current: Some(current_node_summary(node)),
            });
        }
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::FilesystemTypeConflict,
        query: query.to_string(),
        message: format!("desired filesystem type {desired} differs from current {current}"),
        current: Some(current_node_summary(node)),
    })
}

fn disk_create_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Create
        || action.context.collection.as_deref() != Some("disks")
    {
        return None;
    }

    let desired_table = action.context.partition_type.as_deref().unwrap_or("gpt");

    if node.kind != NodeKind::PhysicalDisk {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::DiskCreateRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not a physical disk; partition table initialization remains actionable only after target review",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    let Some(current_table) = property_value_from_node(node, "partition.table") else {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::DiskCreateRequired,
            query: query.to_string(),
            message: format!(
                "disk {query} current partition table is unknown; desired {desired_table} remains actionable after disk identity review"
            ),
            current: Some(current_node_summary(node)),
        });
    };

    let (level, kind, message) = if current_table.eq_ignore_ascii_case(desired_table) {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::DiskCreateAlreadySatisfied,
            format!("disk {query} already has partition table {current_table}"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::DiskCreateRequired,
            format!(
                "disk {query} has partition table {current_table}, desired {desired_table}; mklabel remains destructive and requires review"
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

fn partition_create_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Create
        || action.context.collection.as_deref() != Some("partitions")
    {
        return None;
    }

    if node.kind != NodeKind::Partition {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::PartitionCreateRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not a partition; parted mkpart remains actionable only after target review",
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
                TopologyDiagnosticKind::PartitionCreateAlreadySatisfied,
                format!(
                    "partition {query} already exists with desired size {desired} ({current_bytes} bytes)"
                ),
            )
        }
        (None, _, _) => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::PartitionCreateAlreadySatisfied,
            format!("partition {query} already exists"),
        ),
        (Some(desired), Some(_), Some(current_bytes)) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::PartitionCreateRequired,
            format!(
                "partition {query} already exists with size {current_bytes} bytes, not desired size {desired}; use grow or recreate only after data-preservation review"
            ),
        ),
        (Some(desired), None, _) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::PartitionCreateRequired,
            format!(
                "partition {query} already exists, but desired size {desired} could not be compared"
            ),
        ),
        (Some(desired), Some(_), None) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::PartitionCreateRequired,
            format!(
                "partition {query} already exists, but current size is unknown; desired size is {desired}"
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
