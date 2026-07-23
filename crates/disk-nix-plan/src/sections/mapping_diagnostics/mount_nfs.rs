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
