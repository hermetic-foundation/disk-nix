fn graph_storage_reachability(graph: &StorageGraph) -> BTreeMap<String, BTreeSet<String>> {
    let mut adjacency: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for edge in &graph.edges {
        if let Some((lower_id, upper_id)) = normalized_storage_edge(edge) {
            adjacency
                .entry(lower_id.to_string())
                .or_default()
                .insert(upper_id.to_string());
        }
    }

    let mut reachability = BTreeMap::new();
    for lower_id in adjacency.keys() {
        let mut visited = BTreeSet::new();
        let mut pending: Vec<String> = adjacency
            .get(lower_id)
            .into_iter()
            .flat_map(|upper_ids| upper_ids.iter().cloned())
            .collect();
        while let Some(upper_id) = pending.pop() {
            if !visited.insert(upper_id.clone()) {
                continue;
            }
            if let Some(next_ids) = adjacency.get(&upper_id) {
                pending.extend(next_ids.iter().cloned());
            }
        }
        reachability.insert(lower_id.clone(), visited);
    }
    reachability
}

fn graph_action_matches<'a>(
    actions: &'a [PlannedAction],
    graph: &StorageGraph,
) -> BTreeMap<String, Vec<&'a PlannedAction>> {
    let mut matches: BTreeMap<String, Vec<&PlannedAction>> = BTreeMap::new();
    for action in actions {
        let Some(query) = topology_query(action) else {
            continue;
        };
        for node in graph.find_nodes(&query) {
            matches.entry(node.id.0.clone()).or_default().push(action);
        }
    }
    matches
}

fn actions_for_node<'a>(
    matches: &'a BTreeMap<String, Vec<&'a PlannedAction>>,
    node_id: &str,
) -> &'a [&'a PlannedAction] {
    matches.get(node_id).map(Vec::as_slice).unwrap_or(&[])
}

fn normalized_storage_edge(edge: &disk_nix_model::Edge) -> Option<(&str, &str)> {
    match edge.relationship {
        Relationship::Contains
        | Relationship::Backs
        | Relationship::MapsTo
        | Relationship::MemberOf
        | Relationship::MountedAt
        | Relationship::CacheFor
        | Relationship::ImportedFrom
        | Relationship::Exports => Some((edge.from.0.as_str(), edge.to.0.as_str())),
        Relationship::SnapshotOf | Relationship::DependsOn => {
            Some((edge.to.0.as_str(), edge.from.0.as_str()))
        }
    }
}

fn dependency_direction(operation: Operation) -> DependencyDirection {
    if operation_runs_upper_layers_first(operation) {
        DependencyDirection::UpperLayersFirst
    } else {
        DependencyDirection::LowerLayersFirst
    }
}

fn action_dependency_inputs(action: &PlannedAction) -> BTreeSet<String> {
    let mut inputs = BTreeSet::new();
    insert_identity(&mut inputs, action.context.device.as_deref());
    for device in &action.context.devices {
        insert_identity(&mut inputs, Some(device));
    }
    match action.context.collection.as_deref() {
        Some("loopDevices") => insert_identity(&mut inputs, action.context.device.as_deref()),
        Some("filesystems") | Some("swaps") => {
            insert_identity(&mut inputs, action.context.target.as_deref());
            insert_identity(&mut inputs, action.context.device.as_deref());
        }
        Some("luks.devices")
        | Some("physicalVolumes")
        | Some("vdoVolumes")
        | Some("partitions")
        | Some("multipathMaps")
        | Some("mdRaids")
        | Some("caches") => {
            insert_identity(&mut inputs, action.context.target.as_deref());
            insert_identity(&mut inputs, action.context.device.as_deref());
        }
        Some("luns") | Some("targetLuns") => {
            insert_identity(&mut inputs, action.context.portal.as_deref());
            insert_identity(&mut inputs, action.context.target.as_deref());
            insert_identity(&mut inputs, action.context.device.as_deref());
        }
        Some("volumes") | Some("thinPools") | Some("lvmCaches") | Some("lvmSnapshots") => {
            insert_lvm_parent_identities(&mut inputs, action.context.target.as_deref());
            insert_lvm_parent_identities(&mut inputs, action.context.name.as_deref());
        }
        Some("datasets") | Some("zvols") => {
            insert_zfs_parent_identities(&mut inputs, action.context.target.as_deref());
            insert_zfs_parent_identities(&mut inputs, action.context.name.as_deref());
        }
        Some("snapshots") => {
            insert_identity(&mut inputs, action.context.target.as_deref());
            insert_snapshot_source_identity(&mut inputs, action.context.name.as_deref());
        }
        Some("btrfsSubvolumes") | Some("btrfsQgroups") | Some("nfs.mounts") | Some("exports") => {
            insert_identity(&mut inputs, action.context.target.as_deref());
            insert_identity(&mut inputs, action.context.mountpoint.as_deref());
        }
        _ => {}
    }
    inputs
}

fn action_dependency_identities(action: &PlannedAction) -> BTreeSet<String> {
    let mut identities = BTreeSet::new();
    insert_identity(&mut identities, action.context.name.as_deref());
    insert_identity(&mut identities, action.context.target.as_deref());
    insert_identity(&mut identities, action.context.device.as_deref());
    insert_identity(&mut identities, action.context.mountpoint.as_deref());
    for device in &action.context.devices {
        insert_identity(&mut identities, Some(device));
    }
    if action.context.collection.as_deref() == Some("iscsiSessions") {
        insert_identity(&mut identities, action.context.portal.as_deref());
    }
    identities
}
