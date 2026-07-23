fn order_plan_actions(actions: &mut [PlannedAction]) {
    actions.sort_by_key(action_order_key);
}

fn action_order_key(action: &PlannedAction) -> (u16, u16, u16) {
    let rank = action_dependency_rank(action);
    let layer = if operation_runs_upper_layers_first(action.operation) {
        u16::MAX - rank
    } else {
        rank
    };

    (
        layer,
        action_dependency_subrank(action),
        operation_dependency_phase(action.operation),
    )
}

fn action_dependency_rank(action: &PlannedAction) -> u16 {
    if action.context.collection.as_deref() == Some("partitions")
        && action
            .context
            .device
            .as_deref()
            .is_some_and(|device| device.starts_with("/dev/md/"))
    {
        return collection_dependency_rank(Some("volumes")) + 3;
    }

    if action.context.collection.as_deref() == Some("mdRaids")
        && action
            .context
            .devices
            .iter()
            .any(|device| looks_like_lvm_logical_volume_path(device))
    {
        return collection_dependency_rank(Some("volumes")) + 2;
    }

    if action.context.collection.as_deref() == Some("filesystems")
        && action
            .context
            .device
            .as_deref()
            .is_some_and(looks_like_whole_md_array_path)
    {
        return collection_dependency_rank(Some("mdRaids")) + 1;
    }

    collection_dependency_rank(action.context.collection.as_deref())
}

fn looks_like_whole_md_array_path(device: &str) -> bool {
    let Some(name) = device.strip_prefix("/dev/md/") else {
        return false;
    };

    !name.is_empty()
        && !name.rsplit_once('p').is_some_and(|(_, suffix)| {
            !suffix.is_empty() && suffix.chars().all(|character| character.is_ascii_digit())
        })
}

fn action_dependency_subrank(action: &PlannedAction) -> u16 {
    if action.context.collection.as_deref() == Some("partitions")
        && action.operation == Operation::Create
        && action.context.end.as_deref() == Some("100%")
    {
        return 2;
    }

    if action.context.collection.as_deref() == Some("volumes")
        && action.operation == Operation::Create
        && action
            .context
            .desired_size
            .as_deref()
            .is_some_and(|size| size.contains('%'))
    {
        return 1;
    }

    0
}

fn looks_like_lvm_logical_volume_path(device: &str) -> bool {
    let Some(rest) = device.strip_prefix("/dev/") else {
        return false;
    };
    let mut parts = rest.split('/');
    let Some(first) = parts.next() else {
        return false;
    };
    let Some(second) = parts.next() else {
        return false;
    };
    !first.is_empty()
        && !second.is_empty()
        && parts.next().is_none()
        && !matches!(first, "disk" | "mapper" | "md" | "zvol")
}

fn dependency_order_for_actions(actions: &[PlannedAction]) -> Vec<ActionDependencyOrder> {
    let edges = dependency_edges_for_actions(actions);
    dependency_order_for_actions_with_edges(actions, edges)
}

fn dependency_order_for_actions_with_edges(
    actions: &[PlannedAction],
    edges: DependencyEdges,
) -> Vec<ActionDependencyOrder> {
    actions
        .iter()
        .map(|action| {
            let collection = action.context.collection.clone();
            let layer_rank = collection_dependency_rank(collection.as_deref());
            let direction = if operation_runs_upper_layers_first(action.operation) {
                DependencyDirection::UpperLayersFirst
            } else {
                DependencyDirection::LowerLayersFirst
            };
            ActionDependencyOrder {
                action_id: action.id.clone(),
                phase: operation_dependency_phase_kind(action.operation),
                direction,
                layer_rank,
                collection,
                depends_on: edges
                    .depends_on
                    .get(&action.id)
                    .cloned()
                    .unwrap_or_default(),
                unblocks: edges.unblocks.get(&action.id).cloned().unwrap_or_default(),
                recovery_depends_on: edges.unblocks.get(&action.id).cloned().unwrap_or_default(),
                recovery_unblocks: edges
                    .depends_on
                    .get(&action.id)
                    .cloned()
                    .unwrap_or_default(),
                notes: dependency_order_notes(action, direction, layer_rank, &edges),
            }
        })
        .collect()
}
