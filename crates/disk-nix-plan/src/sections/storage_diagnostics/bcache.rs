fn bcache_cache_set_property_diagnostic(
    action: &PlannedAction,
    node: &Node,
    graph: &StorageGraph,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::SetProperty
        || action.context.collection.as_deref() != Some("caches")
    {
        return None;
    }
    let property = action.context.property.as_deref()?;
    let property_key = bcache_cache_set_property_key(property)?;
    let desired = action.context.property_value.as_deref()?;
    let set_uuid = action
        .context
        .cache_set_uuid
        .as_deref()
        .or_else(|| property_value_from_node(node, "bcache.set-uuid"))?;
    let set_query = format!("bcache-set:{set_uuid}");
    let set_node = graph
        .find_nodes(&set_query)
        .into_iter()
        .next()
        .or_else(|| graph.find_nodes(set_uuid).into_iter().next())?;
    let current = property_value_from_node(set_node, &property_key)?;
    let (level, kind, message) = if current == desired {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::PropertyAlreadySatisfied,
            format!("cache-set property {property} already has desired value {desired}"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::PropertyDiffers,
            format!("cache-set property {property} is {current}, desired {desired}"),
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level,
        kind,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(set_node)),
    })
}

fn bcache_cache_set_property_key(property: &str) -> Option<String> {
    let normalized = normalize_storage_property_name(property);
    let known = match normalized.as_str() {
        "setaveragekeysize" => Some("average-key-size"),
        "setbtreecachesize" => Some("btree-cache-size"),
        "setcacheavailablepercent" => Some("cache-available-percent"),
        "setcongested" => Some("congested"),
        "setcongestedreadthresholdus" => Some("congested-read-threshold-us"),
        "setcongestedwritethresholdus" => Some("congested-write-threshold-us"),
        "setioerrorhalflife" => Some("io-error-halflife"),
        "setioerrorlimit" => Some("io-error-limit"),
        "setjournaldelayms" => Some("journal-delay-ms"),
        "setrootusagepercent" => Some("root-usage-percent"),
        _ => None,
    };
    if let Some(property) = known {
        return Some(format!("bcache.set-{property}"));
    }
    let property = normalized
        .strip_prefix("bcache-set-")
        .or_else(|| normalized.strip_prefix("set-"))?;
    Some(format!("bcache.set-{property}"))
}
