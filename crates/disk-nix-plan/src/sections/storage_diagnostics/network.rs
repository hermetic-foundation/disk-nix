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
