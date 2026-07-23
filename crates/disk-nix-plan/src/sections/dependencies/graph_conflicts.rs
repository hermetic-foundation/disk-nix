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
