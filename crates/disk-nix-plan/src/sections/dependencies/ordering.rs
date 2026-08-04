fn order_plan_actions(actions: &mut [PlannedAction]) {
    actions.sort_by_key(action_order_key);
}

fn action_order_key(
    action: &PlannedAction,
) -> (u16, u16, u16, String, u64, u64, u64, u64, String, String) {
    let rank = action_dependency_rank(action);
    let layer = if operation_runs_upper_layers_first(action.operation) {
        u16::MAX - rank
    } else {
        rank
    };
    let partition = partition_order_key(action);

    (
        layer,
        action_dependency_subrank(action),
        operation_dependency_phase(action.operation),
        partition.disk,
        partition.start_mib,
        partition.end_mib,
        partition.number,
        zfs_create_depth(action),
        zfs_create_name(action),
        action.id.clone(),
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

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct PartitionOrderKey {
    disk: String,
    start_mib: u64,
    end_mib: u64,
    number: u64,
}

fn partition_order_key(action: &PlannedAction) -> PartitionOrderKey {
    if action.context.collection.as_deref() != Some("partitions")
        || action.operation != Operation::Create
    {
        return PartitionOrderKey::default();
    }

    PartitionOrderKey {
        disk: action.context.device.clone().unwrap_or_default(),
        start_mib: partition_offset_mib(action.context.start.as_deref()).unwrap_or(u64::MAX),
        end_mib: partition_offset_mib(action.context.end.as_deref()).unwrap_or(u64::MAX),
        number: action
            .context
            .partition_number
            .as_deref()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(u64::MAX),
    }
}

fn partition_offset_mib(value: Option<&str>) -> Option<u64> {
    let value = value?.trim();
    if value == "100%" {
        return Some(u64::MAX);
    }

    let number_len = value
        .char_indices()
        .take_while(|(_, character)| character.is_ascii_digit() || *character == '.')
        .map(|(index, character)| index + character.len_utf8())
        .last()?;
    let number = value[..number_len].parse::<f64>().ok()?;
    let unit = value[number_len..].trim().to_ascii_lowercase();
    let mib = match unit.as_str() {
        "" | "mib" | "mi" => number,
        "g" | "gb" | "gib" | "gi" => number * 1024.0,
        "t" | "tb" | "tib" | "ti" => number * 1024.0 * 1024.0,
        "b" => number / 1024.0 / 1024.0,
        "kb" | "kib" | "ki" => number / 1024.0,
        _ => return None,
    };

    Some(mib.floor() as u64)
}

fn zfs_create_depth(action: &PlannedAction) -> u64 {
    if !matches!(
        action.context.collection.as_deref(),
        Some("datasets" | "zvols")
    ) || action.operation != Operation::Create
    {
        return 0;
    }

    zfs_create_name(action).split('/').count() as u64
}

fn zfs_create_name(action: &PlannedAction) -> String {
    if !matches!(
        action.context.collection.as_deref(),
        Some("datasets" | "zvols")
    ) || action.operation != Operation::Create
    {
        return String::new();
    }

    action
        .context
        .target
        .as_deref()
        .or(action.context.name.as_deref())
        .unwrap_or_default()
        .to_string()
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
