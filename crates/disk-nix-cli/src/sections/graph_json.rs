fn collect_graph() -> Result<StorageGraph, AppError> {
    let probe = LinuxProbe::new();
    Ok(probe
        .collect()
        .map_err(|error| AppError::Message(error.to_string()))?
        .graph)
}

fn print_filtered_json(
    output: &mut impl Write,
    graph: &StorageGraph,
    predicate: fn(&Node) -> bool,
) -> Result<(), AppError> {
    let matched_ids: BTreeSet<String> = graph
        .nodes
        .iter()
        .filter(|node| predicate(node))
        .map(|node| node.id.0.clone())
        .collect();

    let mut node_ids = matched_ids.clone();
    let edges = graph
        .edges
        .iter()
        .filter(|edge| {
            matched_ids.contains(edge.from.0.as_str()) || matched_ids.contains(edge.to.0.as_str())
        })
        .inspect(|edge| {
            node_ids.insert(edge.from.0.clone());
            node_ids.insert(edge.to.0.clone());
        })
        .cloned()
        .collect();
    let nodes = graph
        .nodes
        .iter()
        .filter(|node| node_ids.contains(node.id.0.as_str()))
        .cloned()
        .collect();
    let filtered = StorageGraph { nodes, edges };

    writeln!(
        output,
        "{}",
        filtered
            .to_json()
            .map_err(|error| AppError::Message(error.to_string()))?
    )?;
    Ok(())
}

fn print_inspect_json(
    output: &mut impl Write,
    graph: &StorageGraph,
    query: &str,
    depth: usize,
) -> Result<(), AppError> {
    let matched_ids: BTreeSet<String> = graph
        .find_nodes(query)
        .into_iter()
        .map(|node| node.id.0.clone())
        .collect();

    let subgraph = relationship_subgraph(graph, &matched_ids, depth);
    writeln!(
        output,
        "{}",
        subgraph
            .to_json()
            .map_err(|error| AppError::Message(error.to_string()))?
    )?;
    Ok(())
}

fn relationship_subgraph(
    graph: &StorageGraph,
    initial_ids: &BTreeSet<String>,
    depth: usize,
) -> StorageGraph {
    let mut node_ids = initial_ids.clone();
    let mut edge_indexes = BTreeSet::new();
    let mut queue = initial_ids
        .iter()
        .map(|id| (id.clone(), 0_usize))
        .collect::<VecDeque<_>>();

    while let Some((node_id, distance)) = queue.pop_front() {
        if distance >= depth {
            continue;
        }

        for (index, edge) in graph.edges.iter().enumerate() {
            let neighbor = if edge.from.0 == node_id {
                Some(edge.to.0.as_str())
            } else if edge.to.0 == node_id {
                Some(edge.from.0.as_str())
            } else {
                None
            };

            let Some(neighbor) = neighbor else {
                continue;
            };

            edge_indexes.insert(index);
            if node_ids.insert(neighbor.to_string()) {
                queue.push_back((neighbor.to_string(), distance + 1));
            }
        }
    }

    let nodes = graph
        .nodes
        .iter()
        .filter(|node| node_ids.contains(node.id.0.as_str()))
        .cloned()
        .collect();
    let edges = graph
        .edges
        .iter()
        .enumerate()
        .filter(|(index, _)| edge_indexes.contains(index))
        .map(|(_, edge)| edge.clone())
        .collect();

    StorageGraph { nodes, edges }
}
