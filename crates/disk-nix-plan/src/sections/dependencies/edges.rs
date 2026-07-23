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
