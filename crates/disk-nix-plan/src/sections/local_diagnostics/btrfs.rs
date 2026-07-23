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
