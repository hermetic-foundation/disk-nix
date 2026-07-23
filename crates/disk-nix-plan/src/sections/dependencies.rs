#[must_use]
pub fn plan_from_value(value: &Value) -> Plan {
    let spec = value.get("spec").unwrap_or(value);
    let mut actions = Vec::new();

    if let Some(filesystems) = spec.get("filesystems").and_then(Value::as_object) {
        for (name, filesystem) in filesystems {
            add_filesystem_actions(&mut actions, name, filesystem);
        }
    }
    if let Some(nfs_mounts) = spec
        .get("nfs")
        .and_then(|nfs| nfs.get("mounts"))
        .and_then(Value::as_object)
    {
        for (name, mount) in nfs_mounts {
            add_lifecycle_actions(&mut actions, "nfs.mounts", name, mount);
        }
    }
    if let Some(iscsi_sessions) = spec
        .get("iscsi")
        .and_then(|iscsi| iscsi.get("sessions"))
        .and_then(Value::as_object)
    {
        for (name, session) in iscsi_sessions {
            add_lifecycle_actions(&mut actions, "iscsiSessions", name, session);
        }
    }
    if let Some(swaps) = spec.get("swaps").and_then(Value::as_object) {
        for (name, swap) in swaps {
            add_swap_actions(&mut actions, name, swap);
        }
    }
    if let Some(zram) = spec.get("zram").and_then(Value::as_object) {
        if !zram.is_empty() {
            add_zram_actions(&mut actions, zram);
        }
    }
    if let Some(luks) = spec
        .get("luks")
        .and_then(|luks| luks.get("devices"))
        .and_then(Value::as_object)
    {
        for (name, luks) in luks {
            add_luks_actions(&mut actions, name, luks);
        }
    }
    for collection in [
        "disks",
        "partitions",
        "btrfsSubvolumes",
        "btrfsQgroups",
        "vdoVolumes",
        "physicalVolumes",
        "luksKeyslots",
        "luksTokens",
        "volumes",
        "volumeGroups",
        "thinPools",
        "lvmSnapshots",
        "lvmCaches",
        "loopDevices",
        "backingFiles",
        "dmMaps",
        "mdRaids",
        "multipathMaps",
        "pools",
        "datasets",
        "zvols",
        "luns",
        "targetLuns",
        "nvmeNamespaces",
        "iscsiSessions",
        "exports",
        "caches",
    ] {
        if let Some(objects) = spec.get(collection).and_then(Value::as_object) {
            for (name, object) in objects {
                add_lifecycle_actions(&mut actions, collection, name, object);
            }
        }
    }
    if let Some(snapshots) = spec.get("snapshots").and_then(Value::as_object) {
        for (name, snapshot) in snapshots {
            add_snapshot_actions(&mut actions, name, snapshot);
        }
    }
    order_plan_actions(&mut actions);

    Plan {
        summary: plan_summary(&actions),
        dependency_order: dependency_order_for_actions(&actions),
        actions,
        topology_comparison: None,
    }
}

#[must_use]
pub fn compare_plan_with_topology(mut plan: Plan, graph: &StorageGraph) -> Plan {
    let original_action_count = plan.actions.len();
    let mut diagnostics: Vec<TopologyDiagnostic> = plan
        .actions
        .iter()
        .flat_map(|action| topology_diagnostics_for_action(action, graph))
        .collect();

    let suppressed_action_ids = already_satisfied_action_ids(&plan.actions, &diagnostics);
    let suppressed_action_count = suppressed_action_ids.len();
    let reconciliation_groups =
        topology_reconciliation_groups_for_actions(&plan.actions, &suppressed_action_ids);
    let reconciliation_group_count = reconciliation_groups.len();
    let partially_suppressed_group_count = reconciliation_groups
        .iter()
        .filter(|group| group.partially_suppressed)
        .count();

    if suppressed_action_count > 0 {
        plan.actions
            .retain(|action| !suppressed_action_ids.contains(&action.id));
        plan.summary = plan_summary(&plan.actions);
    }
    let graph_edges = graph_dependency_edges_for_actions(&plan.actions, graph);
    let graph_dependency_edge_count = graph_edges.graph_edges.len();
    let graph_dependency_order_diagnostics =
        graph_dependency_order_diagnostics_for_actions(&plan.actions, graph);
    let graph_dependency_conflicts =
        graph_dependency_conflict_diagnostics_for_actions(&plan.actions, graph);
    let graph_dependency_conflict_count = graph_dependency_conflicts.len();
    let graph_dependency_conflict_resolutions =
        graph_dependency_conflict_resolutions_for_actions(&plan.actions, graph);
    diagnostics.extend(graph_dependency_order_diagnostics);
    diagnostics.extend(graph_dependency_conflicts);
    let lifecycle_groups = topology_lifecycle_groups_for_actions(&plan.actions, &graph_edges);
    let lifecycle_group_count = lifecycle_groups.len();
    let graph_derived_lifecycle_group_count = lifecycle_groups
        .iter()
        .filter(|group| group.graph_derived_edge_count > 0)
        .count();
    plan.dependency_order = dependency_order_for_actions_with_edges(&plan.actions, graph_edges);

    let summary = TopologyComparisonSummary {
        action_count: original_action_count,
        matched_count: diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.kind == TopologyDiagnosticKind::Matched)
            .count(),
        missing_count: diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.kind == TopologyDiagnosticKind::Missing)
            .count(),
        size_diagnostic_count: diagnostics
            .iter()
            .filter(|diagnostic| {
                matches!(
                    diagnostic.kind,
                    TopologyDiagnosticKind::SizeConflict
                        | TopologyDiagnosticKind::SizeBelowDesired
                        | TopologyDiagnosticKind::SizeAlreadySatisfied
                )
            })
            .count(),
        type_conflict_count: diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.kind == TopologyDiagnosticKind::FilesystemTypeConflict)
            .count(),
        already_satisfied_count: diagnostics
            .iter()
            .filter(|diagnostic| {
                matches!(
                    diagnostic.kind,
                    TopologyDiagnosticKind::SizeAlreadySatisfied
                        | TopologyDiagnosticKind::FilesystemFormatAlreadySatisfied
                        | TopologyDiagnosticKind::DiskCreateAlreadySatisfied
                        | TopologyDiagnosticKind::BtrfsSubvolumeCreateAlreadySatisfied
                        | TopologyDiagnosticKind::BtrfsQgroupCreateAlreadySatisfied
                        | TopologyDiagnosticKind::BackingFileCreateAlreadySatisfied
                        | TopologyDiagnosticKind::PartitionCreateAlreadySatisfied
                        | TopologyDiagnosticKind::LvmPvCreateAlreadySatisfied
                        | TopologyDiagnosticKind::LvmVolumeCreateAlreadySatisfied
                        | TopologyDiagnosticKind::LvmVgCreateAlreadySatisfied
                        | TopologyDiagnosticKind::BtrfsSubvolumeDestroyAlreadySatisfied
                        | TopologyDiagnosticKind::BtrfsQgroupDestroyAlreadySatisfied
                        | TopologyDiagnosticKind::BcacheDetachAlreadySatisfied
                        | TopologyDiagnosticKind::DmMapDestroyAlreadySatisfied
                        | TopologyDiagnosticKind::DmMapRenameAlreadySatisfied
                        | TopologyDiagnosticKind::IscsiLoginAlreadySatisfied
                        | TopologyDiagnosticKind::IscsiLogoutAlreadySatisfied
                        | TopologyDiagnosticKind::LunAttachAlreadySatisfied
                        | TopologyDiagnosticKind::LunDetachAlreadySatisfied
                        | TopologyDiagnosticKind::NvmeNamespaceAttachAlreadySatisfied
                        | TopologyDiagnosticKind::NvmeNamespaceDetachAlreadySatisfied
                        | TopologyDiagnosticKind::LvmActivateAlreadySatisfied
                        | TopologyDiagnosticKind::LvmDeactivateAlreadySatisfied
                        | TopologyDiagnosticKind::LvmVgExportAlreadySatisfied
                        | TopologyDiagnosticKind::LvmVgImportAlreadySatisfied
                        | TopologyDiagnosticKind::LvmRenameAlreadySatisfied
                        | TopologyDiagnosticKind::LvmCacheDetachAlreadySatisfied
                        | TopologyDiagnosticKind::LuksCloseAlreadySatisfied
                        | TopologyDiagnosticKind::LuksOpenAlreadySatisfied
                        | TopologyDiagnosticKind::LuksKeyslotRemoveAlreadySatisfied
                        | TopologyDiagnosticKind::LuksTokenRemoveAlreadySatisfied
                        | TopologyDiagnosticKind::MultipathDestroyAlreadySatisfied
                        | TopologyDiagnosticKind::MultipathPathAddAlreadySatisfied
                        | TopologyDiagnosticKind::MultipathPathRemoveAlreadySatisfied
                        | TopologyDiagnosticKind::SwapDeactivateAlreadySatisfied
                        | TopologyDiagnosticKind::SwapDestroyAlreadySatisfied
                        | TopologyDiagnosticKind::LoopCreateAlreadySatisfied
                        | TopologyDiagnosticKind::LoopDetachAlreadySatisfied
                        | TopologyDiagnosticKind::MdCreateAlreadySatisfied
                        | TopologyDiagnosticKind::MdAssembleAlreadySatisfied
                        | TopologyDiagnosticKind::MdStopAlreadySatisfied
                        | TopologyDiagnosticKind::MdMemberAddAlreadySatisfied
                        | TopologyDiagnosticKind::MdMemberRemoveAlreadySatisfied
                        | TopologyDiagnosticKind::MdMemberReplaceAlreadySatisfied
                        | TopologyDiagnosticKind::MountAlreadySatisfied
                        | TopologyDiagnosticKind::MountOptionsAlreadySatisfied
                        | TopologyDiagnosticKind::UnmountAlreadySatisfied
                        | TopologyDiagnosticKind::NfsExportAlreadySatisfied
                        | TopologyDiagnosticKind::NfsUnexportAlreadySatisfied
                        | TopologyDiagnosticKind::PropertyAlreadySatisfied
                        | TopologyDiagnosticKind::SnapshotDestroyAlreadySatisfied
                        | TopologyDiagnosticKind::VdoDestroyAlreadySatisfied
                        | TopologyDiagnosticKind::VdoStartAlreadySatisfied
                        | TopologyDiagnosticKind::VdoStopAlreadySatisfied
                        | TopologyDiagnosticKind::ZfsObjectCreateAlreadySatisfied
                        | TopologyDiagnosticKind::ZfsObjectDestroyAlreadySatisfied
                        | TopologyDiagnosticKind::ZfsObjectPromoteAlreadySatisfied
                        | TopologyDiagnosticKind::ZfsObjectRenameAlreadySatisfied
                        | TopologyDiagnosticKind::ZfsPoolCreateAlreadySatisfied
                        | TopologyDiagnosticKind::ZfsPoolImportAlreadySatisfied
                )
            })
            .count(),
        suppressed_action_count,
        graph_dependency_edge_count,
        graph_dependency_conflict_count,
        reconciliation_group_count,
        partially_suppressed_group_count,
        lifecycle_group_count,
        graph_derived_lifecycle_group_count,
    };

    plan.topology_comparison = Some(TopologyComparison {
        summary,
        diagnostics,
        reconciliation_groups,
        lifecycle_groups,
        graph_dependency_conflict_resolutions,
    });
    plan
}

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

#[derive(Debug, Default)]
struct DependencyEdges {
    depends_on: BTreeMap<String, Vec<String>>,
    unblocks: BTreeMap<String, Vec<String>>,
    graph_edges: BTreeSet<(String, String)>,
}

fn dependency_edges_for_actions(actions: &[PlannedAction]) -> DependencyEdges {
    let mut edges = DependencyEdges::default();
    for consumer in actions {
        let consumer_inputs = action_dependency_inputs(consumer);
        if consumer_inputs.is_empty() {
            continue;
        }
        let consumer_rank = collection_dependency_rank(consumer.context.collection.as_deref());
        let consumer_direction = if operation_runs_upper_layers_first(consumer.operation) {
            DependencyDirection::UpperLayersFirst
        } else {
            DependencyDirection::LowerLayersFirst
        };

        for provider in actions {
            if provider.id == consumer.id {
                continue;
            }
            let provider_identities = action_dependency_identities(provider);
            if provider_identities.is_empty()
                || !consumer_inputs
                    .iter()
                    .any(|input| provider_identities.contains(input))
            {
                continue;
            }

            let provider_rank = collection_dependency_rank(provider.context.collection.as_deref());
            let provider_direction = if operation_runs_upper_layers_first(provider.operation) {
                DependencyDirection::UpperLayersFirst
            } else {
                DependencyDirection::LowerLayersFirst
            };
            let edge = match consumer_direction {
                DependencyDirection::LowerLayersFirst
                    if provider_direction == DependencyDirection::LowerLayersFirst
                        && provider_rank < consumer_rank =>
                {
                    Some((provider.id.as_str(), consumer.id.as_str()))
                }
                DependencyDirection::UpperLayersFirst
                    if provider_direction == DependencyDirection::UpperLayersFirst
                        && provider_rank > consumer_rank =>
                {
                    Some((provider.id.as_str(), consumer.id.as_str()))
                }
                _ => None,
            };

            if let Some((depends_on, action_id)) = edge {
                insert_unique_sorted(&mut edges.depends_on, action_id, depends_on);
                insert_unique_sorted(&mut edges.unblocks, depends_on, action_id);
            }
        }
    }
    edges
}

fn graph_dependency_edges_for_actions(
    actions: &[PlannedAction],
    graph: &StorageGraph,
) -> DependencyEdges {
    let mut edges = dependency_edges_for_actions(actions);
    let matches = graph_action_matches(actions, graph);
    let reachability = graph_storage_reachability(graph);
    for (lower_id, upper_ids) in reachability {
        for upper_id in upper_ids {
            for lower_action in actions_for_node(&matches, &lower_id) {
                for upper_action in actions_for_node(&matches, &upper_id) {
                    if lower_action.id == upper_action.id {
                        continue;
                    }
                    let lower_direction = dependency_direction(lower_action.operation);
                    let upper_direction = dependency_direction(upper_action.operation);
                    if lower_direction != upper_direction {
                        continue;
                    }
                    let (depends_on, action_id) = match lower_direction {
                        DependencyDirection::LowerLayersFirst => {
                            (lower_action.id.as_str(), upper_action.id.as_str())
                        }
                        DependencyDirection::UpperLayersFirst => {
                            (upper_action.id.as_str(), lower_action.id.as_str())
                        }
                    };
                    insert_unique_sorted(&mut edges.depends_on, action_id, depends_on);
                    insert_unique_sorted(&mut edges.unblocks, depends_on, action_id);
                    edges
                        .graph_edges
                        .insert((action_id.to_string(), depends_on.to_string()));
                }
            }
        }
    }
    edges
}

fn topology_lifecycle_groups_for_actions(
    actions: &[PlannedAction],
    edges: &DependencyEdges,
) -> Vec<TopologyLifecycleGroup> {
    let action_ids: BTreeSet<String> = actions.iter().map(|action| action.id.clone()).collect();
    let actions_by_id: BTreeMap<&str, &PlannedAction> = actions
        .iter()
        .map(|action| (action.id.as_str(), action))
        .collect();
    let mut adjacency: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for (action_id, dependencies) in &edges.depends_on {
        if !action_ids.contains(action_id) {
            continue;
        }
        for dependency in dependencies {
            if !action_ids.contains(dependency) {
                continue;
            }
            adjacency
                .entry(action_id.clone())
                .or_default()
                .insert(dependency.clone());
            adjacency
                .entry(dependency.clone())
                .or_default()
                .insert(action_id.clone());
        }
    }

    let mut visited = BTreeSet::new();
    let mut groups = Vec::new();
    for action_id in adjacency.keys() {
        if visited.contains(action_id) {
            continue;
        }

        let mut stack = vec![action_id.clone()];
        let mut group_action_ids = BTreeSet::new();
        while let Some(current) = stack.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }
            group_action_ids.insert(current.clone());
            if let Some(neighbors) = adjacency.get(&current) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        stack.push(neighbor.clone());
                    }
                }
            }
        }

        if group_action_ids.len() < 2 {
            continue;
        }

        let edge_count = edges
            .depends_on
            .iter()
            .filter(|(dependent, dependencies)| {
                group_action_ids.contains(*dependent)
                    && dependencies
                        .iter()
                        .any(|dependency| group_action_ids.contains(dependency))
            })
            .map(|(_dependent, dependencies)| {
                dependencies
                    .iter()
                    .filter(|dependency| group_action_ids.contains(*dependency))
                    .count()
            })
            .sum();
        let graph_derived_edge_count = edges
            .graph_edges
            .iter()
            .filter(|(dependent, dependency)| {
                group_action_ids.contains(dependent) && group_action_ids.contains(dependency)
            })
            .count();
        let phases = group_action_ids
            .iter()
            .filter_map(|id| actions_by_id.get(id.as_str()))
            .map(|action| operation_dependency_phase_kind(action.operation))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let directions = group_action_ids
            .iter()
            .filter_map(|id| actions_by_id.get(id.as_str()))
            .map(|action| dependency_direction(action.operation))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let action_ids = group_action_ids.into_iter().collect::<Vec<_>>();
        let group_id = format!(
            "lifecycle:{}",
            action_ids
                .first()
                .expect("lifecycle group has at least two action ids")
        );

        groups.push(TopologyLifecycleGroup {
            group_id,
            action_count: action_ids.len(),
            action_ids,
            edge_count,
            graph_derived_edge_count,
            phases,
            directions,
            recommendation: "review and apply this connected lifecycle group as one ordered mutation or split it into independently verified passes".to_string(),
        });
    }

    groups
}

fn graph_dependency_conflict_diagnostics_for_actions(
    actions: &[PlannedAction],
    graph: &StorageGraph,
) -> Vec<TopologyDiagnostic> {
    graph_dependency_conflict_resolutions_for_actions(actions, graph)
        .into_iter()
        .map(|resolution| TopologyDiagnostic {
            action_id: resolution.lower_action_id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::GraphDependencyConflict,
            query: resolution.path.clone(),
            message: format!(
                "current topology path has mixed dependency directions: {} runs {:?} while {} runs {:?}; split into build/update pass [{}] and teardown/recovery pass [{}] before execution",
                resolution.lower_action_id,
                resolution.lower_direction,
                resolution.upper_action_id,
                resolution.upper_direction,
                resolution.build_or_update_pass.join(", "),
                resolution.teardown_or_recovery_pass.join(", ")
            ),
            current: None,
        })
        .collect()
}

fn graph_dependency_conflict_resolutions_for_actions(
    actions: &[PlannedAction],
    graph: &StorageGraph,
) -> Vec<GraphDependencyConflictResolution> {
    let matches = graph_action_matches(actions, graph);
    let reachability = graph_storage_reachability(graph);
    let mut resolutions = Vec::new();
    let mut seen = BTreeSet::new();
    for (lower_id, upper_ids) in reachability {
        for upper_id in upper_ids {
            for lower_action in actions_for_node(&matches, &lower_id) {
                for upper_action in actions_for_node(&matches, &upper_id) {
                    if lower_action.id == upper_action.id {
                        continue;
                    }
                    let lower_direction = dependency_direction(lower_action.operation);
                    let upper_direction = dependency_direction(upper_action.operation);
                    if lower_direction == upper_direction {
                        continue;
                    }
                    let key = (
                        lower_action.id.clone(),
                        upper_action.id.clone(),
                        lower_id.clone(),
                        upper_id.clone(),
                    );
                    if !seen.insert(key) {
                        continue;
                    }
                    let build_or_update_pass = graph_conflict_pass_actions(
                        lower_action,
                        lower_direction,
                        upper_action,
                        upper_direction,
                        DependencyDirection::LowerLayersFirst,
                    );
                    let teardown_or_recovery_pass = graph_conflict_pass_actions(
                        lower_action,
                        lower_direction,
                        upper_action,
                        upper_direction,
                        DependencyDirection::UpperLayersFirst,
                    );
                    resolutions.push(GraphDependencyConflictResolution {
                        path: format!("{lower_id} -> {upper_id}"),
                        lower_action_id: lower_action.id.clone(),
                        upper_action_id: upper_action.id.clone(),
                        lower_direction,
                        upper_direction,
                        build_or_update_pass,
                        teardown_or_recovery_pass,
                        recommendation:
                            "split mixed-direction graph-path work into separate reviewed passes; run build/update actions lower-to-upper and teardown/recovery actions upper-to-lower"
                                .to_string(),
                    });
                }
            }
        }
    }
    resolutions
}

fn graph_conflict_pass_actions(
    lower_action: &PlannedAction,
    lower_direction: DependencyDirection,
    upper_action: &PlannedAction,
    upper_direction: DependencyDirection,
    selected_direction: DependencyDirection,
) -> Vec<String> {
    [
        (lower_action, lower_direction),
        (upper_action, upper_direction),
    ]
    .into_iter()
    .filter(|(_, direction)| *direction == selected_direction)
    .map(|(action, _)| action.id.clone())
    .collect()
}

fn graph_dependency_order_diagnostics_for_actions(
    actions: &[PlannedAction],
    graph: &StorageGraph,
) -> Vec<TopologyDiagnostic> {
    let matches = graph_action_matches(actions, graph);
    let reachability = graph_storage_reachability(graph);
    let mut diagnostics = Vec::new();
    let mut seen = BTreeSet::new();
    for (lower_id, upper_ids) in reachability {
        for upper_id in upper_ids {
            for lower_action in actions_for_node(&matches, &lower_id) {
                for upper_action in actions_for_node(&matches, &upper_id) {
                    if lower_action.id == upper_action.id {
                        continue;
                    }
                    let lower_direction = dependency_direction(lower_action.operation);
                    let upper_direction = dependency_direction(upper_action.operation);
                    if lower_direction != upper_direction {
                        continue;
                    }
                    let (depends_on, action_id, ordering) = match lower_direction {
                        DependencyDirection::LowerLayersFirst => (
                            lower_action.id.as_str(),
                            upper_action.id.as_str(),
                            "lower layer before consumer",
                        ),
                        DependencyDirection::UpperLayersFirst => (
                            upper_action.id.as_str(),
                            lower_action.id.as_str(),
                            "consumer before backing layer",
                        ),
                    };
                    let key = (
                        action_id.to_string(),
                        depends_on.to_string(),
                        lower_id.clone(),
                        upper_id.clone(),
                    );
                    if !seen.insert(key) {
                        continue;
                    }
                    diagnostics.push(TopologyDiagnostic {
                        action_id: action_id.to_string(),
                        level: TopologyDiagnosticLevel::Info,
                        kind: TopologyDiagnosticKind::GraphDependencyOrder,
                        query: format!("{lower_id} -> {upper_id}"),
                        message: format!(
                            "current topology path orders {action_id} after {depends_on} ({ordering})"
                        ),
                        current: None,
                    });
                }
            }
        }
    }
    diagnostics
}

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

fn topology_reconciliation_groups_for_actions(
    actions: &[PlannedAction],
    suppressed_action_ids: &[String],
) -> Vec<TopologyReconciliationGroup> {
    let suppressed: BTreeSet<&str> = suppressed_action_ids.iter().map(String::as_str).collect();
    let mut groups: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for action in actions {
        for identity in action_reconciliation_group_identities(action) {
            groups
                .entry(identity)
                .or_default()
                .insert(action.id.clone());
        }
    }

    groups
        .into_iter()
        .filter_map(|(identity, action_ids)| {
            if action_ids.len() < 2 {
                return None;
            }
            let action_ids: Vec<String> = action_ids.into_iter().collect();
            let suppressed_action_ids: Vec<String> = action_ids
                .iter()
                .filter(|action_id| suppressed.contains(action_id.as_str()))
                .cloned()
                .collect();
            let planned_action_ids: Vec<String> = action_ids
                .iter()
                .filter(|action_id| !suppressed.contains(action_id.as_str()))
                .cloned()
                .collect();
            let action_count = action_ids.len();
            let planned_count = planned_action_ids.len();
            let suppressed_count = suppressed_action_ids.len();
            let partially_suppressed = planned_count > 0 && suppressed_count > 0;
            Some(TopologyReconciliationGroup {
                identity,
                action_ids,
                planned_action_ids,
                suppressed_action_ids,
                action_count,
                planned_count,
                suppressed_count,
                partially_suppressed,
                recommendation: if partially_suppressed {
                    "review the remaining planned actions against the fresh topology because related actions in this identity group were already satisfied and suppressed"
                        .to_string()
                } else if suppressed_count == action_count {
                    "all actions in this identity group were already satisfied and suppressed before command rendering"
                        .to_string()
                } else {
                    "related actions share this identity and remain planned together before command rendering"
                        .to_string()
                },
            })
        })
        .collect()
}

fn action_reconciliation_group_identities(action: &PlannedAction) -> BTreeSet<String> {
    let mut identities = action_dependency_identities(action);
    identities.extend(action_dependency_inputs(action));
    insert_cross_domain_reconciliation_aliases(&mut identities, action);
    identities
}

fn insert_cross_domain_reconciliation_aliases(
    identities: &mut BTreeSet<String>,
    action: &PlannedAction,
) {
    match action.context.collection.as_deref() {
        Some("exports") => {
            insert_nfs_export_alias(identities, action.context.target.as_deref());
            insert_nfs_export_alias(identities, action.context.name.as_deref());
        }
        Some("nfs.mounts") => {
            insert_nfs_source_alias(identities, action.context.device.as_deref());
            insert_nfs_export_alias(identities, action.context.device.as_deref());
        }
        Some("dmMaps") => {
            insert_dm_map_alias(identities, action.context.target.as_deref());
            insert_dm_map_alias(identities, action.context.name.as_deref());
            insert_dm_map_alias(identities, action.context.rename_to.as_deref());
        }
        Some("filesystems" | "swaps" | "luks.devices" | "physicalVolumes" | "vdoVolumes") => {
            insert_dm_map_alias(identities, action.context.target.as_deref());
            insert_dm_map_alias(identities, action.context.device.as_deref());
        }
        _ => {}
    }
}

fn insert_nfs_source_alias(identities: &mut BTreeSet<String>, value: Option<&str>) {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    identities.insert(format!("nfs-source:{value}"));
}

fn insert_nfs_export_alias(identities: &mut BTreeSet<String>, value: Option<&str>) {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    let export_path = value
        .rsplit_once(':')
        .map(|(_server, path)| path)
        .unwrap_or(value)
        .trim();
    if export_path.starts_with('/') {
        identities.insert(format!("nfs-export:{export_path}"));
    }
}

fn insert_dm_map_alias(identities: &mut BTreeSet<String>, value: Option<&str>) {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    if let Some(name) = value.strip_prefix("/dev/mapper/") {
        if !name.is_empty() {
            identities.insert(format!("dm-map:{name}"));
        }
    } else if !value.starts_with("/dev/") && !value.contains('/') {
        identities.insert(format!("dm-map:{value}"));
    }
}

fn insert_lvm_parent_identities(identities: &mut BTreeSet<String>, value: Option<&str>) {
    let Some(value) = value else {
        return;
    };
    if let Some((vg, _lv)) = value.split_once('/') {
        insert_identity(identities, Some(vg));
    }
}

fn insert_zfs_parent_identities(identities: &mut BTreeSet<String>, value: Option<&str>) {
    let Some(value) = value else {
        return;
    };
    if let Some((pool, _rest)) = value.split_once('/') {
        insert_identity(identities, Some(pool));
    }
    if let Some((dataset, _snapshot)) = value.split_once('@') {
        insert_identity(identities, Some(dataset));
        insert_zfs_parent_identities(identities, Some(dataset));
    }
}

fn insert_snapshot_source_identity(identities: &mut BTreeSet<String>, value: Option<&str>) {
    let Some(value) = value else {
        return;
    };
    if let Some((dataset, _snapshot)) = value.split_once('@') {
        insert_identity(identities, Some(dataset));
    }
}

fn insert_identity(identities: &mut BTreeSet<String>, value: Option<&str>) {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    identities.insert(value.to_string());
}

fn insert_unique_sorted(map: &mut BTreeMap<String, Vec<String>>, key: &str, value: &str) {
    let values = map.entry(key.to_string()).or_default();
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
        values.sort();
    }
}

fn dependency_order_notes(
    action: &PlannedAction,
    direction: DependencyDirection,
    layer_rank: u16,
    edges: &DependencyEdges,
) -> Vec<String> {
    let mut notes = vec![format!(
        "collection layer rank {layer_rank} orders {} actions",
        action
            .context
            .collection
            .as_deref()
            .unwrap_or("unclassified")
    )];
    match direction {
        DependencyDirection::LowerLayersFirst => notes.push(
            "lower storage layers are planned before consumers for build, grow, rescan, and property work"
                .to_string(),
        ),
        DependencyDirection::UpperLayersFirst => notes.push(
            "consumer layers are planned before backing layers for teardown, shrink, rollback, detach, and destroy work"
                .to_string(),
        ),
    }
    if let Some(depends_on) = edges.depends_on.get(&action.id) {
        notes.push(format!(
            "explicit dependency edge requires {} before this action",
            depends_on.join(", ")
        ));
        let graph_depends_on: Vec<&str> = depends_on
            .iter()
            .filter(|depends_on| {
                edges
                    .graph_edges
                    .contains(&(action.id.clone(), (*depends_on).clone()))
            })
            .map(String::as_str)
            .collect();
        if !graph_depends_on.is_empty() {
            notes.push(format!(
                "current topology graph path requires {} before this action",
                graph_depends_on.join(", ")
            ));
        }
    }
    if let Some(unblocks) = edges.unblocks.get(&action.id) {
        notes.push(format!(
            "this action unblocks explicit dependent action(s): {}",
            unblocks.join(", ")
        ));
        let graph_unblocks: Vec<&str> = unblocks
            .iter()
            .filter(|unblocks| {
                edges
                    .graph_edges
                    .contains(&((*unblocks).clone(), action.id.clone()))
            })
            .map(String::as_str)
            .collect();
        if !graph_unblocks.is_empty() {
            notes.push(format!(
                "current topology graph path shows this action unblocks {}",
                graph_unblocks.join(", ")
            ));
        }
    }
    if let Some(recovery_depends_on) = edges.unblocks.get(&action.id) {
        notes.push(format!(
            "recovery review waits for dependent action(s): {}",
            recovery_depends_on.join(", ")
        ));
    }
    if let Some(recovery_unblocks) = edges.depends_on.get(&action.id) {
        notes.push(format!(
            "recovery review unblocks prerequisite action(s): {}",
            recovery_unblocks.join(", ")
        ));
    }
    notes
}

fn operation_dependency_phase_kind(operation: Operation) -> DependencyPhase {
    match operation {
        Operation::Create
        | Operation::Import
        | Operation::Login
        | Operation::Attach
        | Operation::Open
        | Operation::Activate
        | Operation::Assemble
        | Operation::Start => DependencyPhase::BuildLowerLayers,
        Operation::Format
        | Operation::Grow
        | Operation::AddDevice
        | Operation::ReplaceDevice
        | Operation::AddKey
        | Operation::ImportToken
        | Operation::SetProperty
        | Operation::Snapshot
        | Operation::Clone
        | Operation::Promote
        | Operation::Mount
        | Operation::Remount
        | Operation::Check
        | Operation::Repair
        | Operation::Scrub
        | Operation::Trim
        | Operation::Rescan
        | Operation::Rename
        | Operation::Rebalance => DependencyPhase::MutateInPlace,
        Operation::Shrink
        | Operation::RemoveDevice
        | Operation::RemoveKey
        | Operation::RemoveToken
        | Operation::Rollback
        | Operation::Unmount
        | Operation::Close
        | Operation::Logout
        | Operation::Deactivate
        | Operation::Stop
        | Operation::Detach
        | Operation::Export
        | Operation::Unexport
        | Operation::Destroy => DependencyPhase::TearDownUpperLayers,
    }
}

fn operation_dependency_phase(operation: Operation) -> u16 {
    match operation_dependency_phase_kind(operation) {
        DependencyPhase::BuildLowerLayers => 10,
        DependencyPhase::MutateInPlace => 20,
        DependencyPhase::TearDownUpperLayers => 30,
    }
}

fn operation_runs_upper_layers_first(operation: Operation) -> bool {
    matches!(
        operation,
        Operation::Shrink
            | Operation::RemoveDevice
            | Operation::RemoveKey
            | Operation::RemoveToken
            | Operation::Rollback
            | Operation::Unmount
            | Operation::Close
            | Operation::Logout
            | Operation::Deactivate
            | Operation::Stop
            | Operation::Detach
            | Operation::Export
            | Operation::Unexport
            | Operation::Destroy
    )
}

fn collection_dependency_rank(collection: Option<&str>) -> u16 {
    match collection {
        Some("backingFiles") => 10,
        Some("loopDevices") => 15,
        Some("disks") => 20,
        Some("iscsiSessions") => 25,
        Some("nvmeNamespaces") => 30,
        Some("targetLuns") => 32,
        Some("luns") => 35,
        Some("partitions") => 40,
        Some("mdRaids") | Some("multipathMaps") => 45,
        Some("luks.devices") | Some("dmMaps") => 50,
        Some("physicalVolumes") => 55,
        Some("volumeGroups") => 60,
        Some("thinPools") | Some("volumes") | Some("lvmCaches") | Some("lvmSnapshots") => 65,
        Some("vdoVolumes") | Some("caches") => 70,
        Some("pools") => 75,
        Some("datasets") | Some("zvols") => 80,
        Some("btrfsQgroups") => 85,
        Some("filesystems") | Some("swaps") | Some("zram") | Some("nfs.mounts") => 90,
        Some("btrfsSubvolumes") => 92,
        Some("snapshots") | Some("exports") => 95,
        Some(_) | None => 100,
    }
}
