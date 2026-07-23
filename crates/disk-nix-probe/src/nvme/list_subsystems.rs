pub fn normalize_nvme_list_json(bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let list: NvmeList = serde_json::from_slice(bytes)
        .map_err(|error| ProbeError::Adapter(format!("failed to parse nvme JSON: {error}")))?;
    let mut graph = StorageGraph::empty();

    for device in list.devices {
        add_device(&mut graph, device);
    }

    Ok(graph)
}

pub fn normalize_nvme_subsystems_json(bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to parse nvme list-subsys JSON: {error}"))
    })?;
    let mut graph = StorageGraph::empty();
    let Some(subsystems) = value
        .get("Subsystems")
        .or_else(|| value.get("subsystems"))
        .and_then(Value::as_array)
    else {
        return Ok(graph);
    };

    for (index, subsystem) in subsystems.iter().enumerate() {
        let name = field_string_any(subsystem, &["Name", "name"])
            .or_else(|| field_string_any(subsystem, &["Subsystem", "SubSystem"]));
        let nqn = field_string_any(subsystem, &["NQN", "SubsystemNQN", "subnqn", "nqn"]);
        let hostnqn = field_string_any(subsystem, &["HostNQN", "hostnqn"]);
        let node_name = name
            .clone()
            .or_else(|| nqn.clone())
            .unwrap_or_else(|| format!("subsystem-{index}"));
        let subsystem_id = nvme_subsystem_id(&node_name);
        let mut node = Node::new(
            subsystem_id.clone(),
            NodeKind::NvmeSubsystem,
            node_name.clone(),
        )
        .with_property("nvme.subsystem-name", node_name.clone());

        if let Some(name) = name {
            node = node.with_property("nvme.subsystem", name);
        }
        if let Some(nqn) = nqn.clone() {
            node = node.with_property("nvme.subsystem-nqn", nqn);
        }
        if let Some(hostnqn) = hostnqn {
            node = node.with_property("nvme.hostnqn", hostnqn);
        }
        graph.add_node(node);

        for path in subsystem_paths(subsystem) {
            add_subsystem_path(&mut graph, &subsystem_id, &node_name, nqn.as_deref(), path);
        }
    }

    Ok(graph)
}
