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
