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
