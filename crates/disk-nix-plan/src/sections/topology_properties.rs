fn plan_summary(actions: &[PlannedAction]) -> PlanSummary {
    PlanSummary {
        action_count: actions.len(),
        offline_required_count: actions
            .iter()
            .filter(|action| action.risk == RiskClass::OfflineRequired)
            .count(),
        destructive_count: actions
            .iter()
            .filter(|action| action.risk == RiskClass::Destructive || action.destructive)
            .count(),
        potential_data_loss_count: actions
            .iter()
            .filter(|action| action.risk == RiskClass::PotentialDataLoss)
            .count(),
        unsupported_count: actions
            .iter()
            .filter(|action| action.risk == RiskClass::Unsupported)
            .count(),
    }
}

fn already_satisfied_action_ids(
    actions: &[PlannedAction],
    diagnostics: &[TopologyDiagnostic],
) -> Vec<String> {
    let mut ids = Vec::new();
    for action in actions {
        if !matches!(
            action.operation,
            Operation::Create
                | Operation::Grow
                | Operation::Shrink
                | Operation::AddDevice
                | Operation::ReplaceDevice
                | Operation::Attach
                | Operation::Detach
                | Operation::Assemble
                | Operation::Import
                | Operation::Activate
                | Operation::Deactivate
                | Operation::Close
                | Operation::Login
                | Operation::Logout
                | Operation::Open
                | Operation::Mount
                | Operation::Unmount
                | Operation::Remount
                | Operation::Export
                | Operation::Unexport
                | Operation::Start
                | Operation::Stop
                | Operation::Destroy
                | Operation::Promote
                | Operation::Rename
                | Operation::RemoveDevice
                | Operation::RemoveKey
                | Operation::RemoveToken
                | Operation::SetProperty
        ) {
            continue;
        }
        let action_diagnostics = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.action_id == action.id);
        let mut already_satisfied = false;
        let mut has_warning = false;
        for diagnostic in action_diagnostics {
            already_satisfied |= matches!(
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
            );
            has_warning |= diagnostic.level == TopologyDiagnosticLevel::Warning;
        }
        if already_satisfied && !has_warning {
            ids.push(action.id.clone());
        }
    }
    ids
}

fn topology_diagnostics_for_action(
    action: &PlannedAction,
    graph: &StorageGraph,
) -> Vec<TopologyDiagnostic> {
    let Some(query) = topology_query(action) else {
        return Vec::new();
    };

    let mut matches = graph.find_nodes(&query);
    if matches.is_empty() && action.context.collection.as_deref() == Some("zram") {
        matches = zram_topology_nodes(graph);
    }
    if matches.is_empty() {
        if let Some(diagnostic) = bcache_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = snapshot_clone_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = snapshot_destroy_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = snapshot_rename_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = snapshot_rollback_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = btrfs_subvolume_destroy_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = btrfs_qgroup_destroy_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = luks_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = vdo_destroy_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = vdo_grow_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = vdo_start_stop_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = zfs_object_destroy_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = zfs_object_rename_absent_diagnostic(action, graph, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = lvm_activation_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = lvm_rename_absent_diagnostic(action, graph, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = lvm_cache_detach_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = dm_map_rename_absent_diagnostic(action, graph, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = dm_map_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = multipath_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = multipath_path_add_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = multipath_path_remove_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = loop_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = md_stop_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = md_member_remove_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = nvme_namespace_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = lun_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = nfs_export_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = nfs_unexport_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = mount_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = swap_inactive_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = unmount_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        return vec![TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::Missing,
            query,
            message: "no current topology node matched this planned action target".to_string(),
            current: None,
        }];
    }

    let node = preferred_topology_node(action, &matches);
    let mut diagnostics = vec![TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::Matched,
        query: query.clone(),
        message: format!("matched current {} node {}", node.kind, node.name),
        current: Some(current_node_summary(node)),
    }];

    diagnostics.extend(size_diagnostic(action, node, &query));
    diagnostics.extend(filesystem_type_diagnostic(action, node, &query));
    diagnostics.extend(disk_create_diagnostic(action, node, &query));
    diagnostics.extend(iscsi_login_diagnostic(action, &matches, &query));
    diagnostics.extend(iscsi_logout_diagnostic(action, &matches, &query));
    diagnostics.extend(nvme_namespace_present_diagnostic(action, node, &query));
    diagnostics.extend(lun_present_diagnostic(action, node, &query));
    diagnostics.extend(lvm_volume_create_diagnostic(action, node, &query));
    diagnostics.extend(lvm_activate_diagnostic(action, node, &query));
    diagnostics.extend(lvm_deactivate_diagnostic(action, node, &query));
    diagnostics.extend(lvm_pv_create_diagnostic(action, &matches, &query));
    diagnostics.extend(lvm_vg_create_diagnostic(action, node, &query));
    diagnostics.extend(lvm_vg_export_diagnostic(action, node, &query));
    diagnostics.extend(lvm_vg_import_diagnostic(action, node, &query));
    diagnostics.extend(lvm_rename_present_diagnostic(action, node, &query));
    diagnostics.extend(lvm_cache_detach_diagnostic(action, node, &query));
    diagnostics.extend(luks_close_diagnostic(action, node, &query));
    diagnostics.extend(luks_open_diagnostic(action, node, &query));
    diagnostics.extend(luks_keyslot_remove_diagnostic(action, node, &query));
    diagnostics.extend(luks_token_remove_diagnostic(action, node, &query));
    diagnostics.extend(bcache_present_diagnostic(action, node, &query));
    diagnostics.extend(snapshot_clone_present_diagnostic(action, node, &query));
    diagnostics.extend(snapshot_destroy_present_diagnostic(action, node, &query));
    diagnostics.extend(snapshot_rename_present_diagnostic(action, node, &query));
    diagnostics.extend(snapshot_rollback_present_diagnostic(action, node, &query));
    diagnostics.extend(btrfs_subvolume_create_present_diagnostic(
        action, node, &query,
    ));
    diagnostics.extend(btrfs_subvolume_destroy_present_diagnostic(
        action, node, &query,
    ));
    diagnostics.extend(btrfs_qgroup_destroy_present_diagnostic(
        action, node, &query,
    ));
    diagnostics.extend(btrfs_qgroup_create_present_diagnostic(action, node, &query));
    diagnostics.extend(dm_map_rename_present_diagnostic(action, node, &query));
    diagnostics.extend(dm_map_present_diagnostic(action, node, &query));
    diagnostics.extend(multipath_present_diagnostic(action, node, &query));
    diagnostics.extend(multipath_path_add_diagnostic(action, node, graph, &query));
    diagnostics.extend(multipath_path_remove_diagnostic(
        action, node, graph, &query,
    ));
    diagnostics.extend(loop_present_diagnostic(action, node, &query));
    diagnostics.extend(partition_create_diagnostic(action, node, &query));
    diagnostics.extend(backing_file_create_diagnostic(action, node, &query));
    diagnostics.extend(md_create_diagnostic(action, node, &query));
    diagnostics.extend(md_assemble_diagnostic(action, node, &query));
    diagnostics.extend(md_stop_diagnostic(action, node, &query));
    diagnostics.extend(md_member_add_diagnostic(action, node, graph, &query));
    diagnostics.extend(md_member_remove_diagnostic(action, node, graph, &query));
    diagnostics.extend(md_member_replace_diagnostic(action, node, graph, &query));
    diagnostics.extend(mount_diagnostic(action, node, &query));
    diagnostics.extend(mount_options_diagnostic(action, node, &query));
    diagnostics.extend(unmount_diagnostic(action, node, &query));
    diagnostics.extend(nfs_export_diagnostic(action, node, &query));
    diagnostics.extend(nfs_unexport_diagnostic(action, node, &query));
    diagnostics.extend(swap_active_diagnostic(action, node, &query));
    diagnostics.extend(swap_format_present_diagnostic(action, node, &query));
    diagnostics.extend(luks_format_present_diagnostic(action, node, &query));
    diagnostics.extend(snapshot_hold_diagnostic(action, node, &query));
    diagnostics.extend(bcache_cache_set_property_diagnostic(
        action, node, graph, &query,
    ));
    diagnostics.extend(property_diagnostic(action, node, &query));
    diagnostics.extend(vdo_create_present_diagnostic(action, node, &query));
    diagnostics.extend(vdo_destroy_present_diagnostic(action, node, &query));
    diagnostics.extend(vdo_grow_diagnostic(action, node, &query));
    diagnostics.extend(vdo_start_diagnostic(action, node, &query));
    diagnostics.extend(vdo_stop_diagnostic(action, node, &query));
    diagnostics.extend(zfs_object_create_present_diagnostic(action, node, &query));
    diagnostics.extend(zfs_object_destroy_present_diagnostic(action, node, &query));
    diagnostics.extend(zfs_object_promote_diagnostic(action, node, &query));
    diagnostics.extend(zfs_object_rename_present_diagnostic(action, node, &query));
    diagnostics.extend(zfs_pool_create_diagnostic(action, node, &query));
    diagnostics.extend(zfs_pool_import_diagnostic(action, node, &query));
    diagnostics
}

fn topology_query(action: &PlannedAction) -> Option<String> {
    if matches!(
        action.context.collection.as_deref(),
        Some("luns" | "nvmeNamespaces")
    ) {
        return action
            .context
            .device
            .clone()
            .or_else(|| action.context.target.clone())
            .or_else(|| action.context.name.clone());
    }

    if action.context.collection.as_deref() == Some("btrfsQgroups") {
        return action
            .context
            .name
            .clone()
            .or_else(|| action.context.target.clone())
            .or_else(|| action.context.device.clone());
    }

    if action.context.collection.as_deref() == Some("snapshots")
        && matches!(
            action.operation,
            Operation::Clone
                | Operation::Destroy
                | Operation::Rename
                | Operation::Rollback
                | Operation::SetProperty
        )
    {
        return action
            .context
            .snapshot_path
            .clone()
            .or_else(|| action.context.name.clone())
            .or_else(|| action.context.target.clone())
            .or_else(|| action.context.device.clone());
    }

    if matches!(
        action.context.collection.as_deref(),
        Some("luksKeyslots" | "luksTokens")
    ) {
        return action
            .context
            .device
            .clone()
            .or_else(|| action.context.target.clone())
            .or_else(|| action.context.name.clone());
    }

    if action.context.collection.as_deref() == Some("luks.devices")
        && matches!(action.operation, Operation::Format | Operation::SetProperty)
    {
        return action
            .context
            .device
            .clone()
            .or_else(|| action.context.target.clone())
            .or_else(|| action.context.name.clone());
    }

    action
        .context
        .target
        .clone()
        .or_else(|| action.context.name.clone())
        .or_else(|| action.context.device.clone())
}

fn zram_topology_nodes(graph: &StorageGraph) -> Vec<&Node> {
    graph
        .nodes
        .iter()
        .filter(|node| {
            node.kind == NodeKind::ZramDevice
                || node
                    .path
                    .as_deref()
                    .is_some_and(|path| path.starts_with("/dev/zram"))
                || node
                    .properties
                    .iter()
                    .any(|property| property.key.starts_with("zram."))
        })
        .collect()
}

fn preferred_topology_node<'a>(action: &PlannedAction, matches: &'a [&'a Node]) -> &'a Node {
    if action.context.collection.as_deref() == Some("zram")
        && action.operation == Operation::SetProperty
    {
        if let Some(property) = action.context.property.as_deref() {
            if let Some(node) = matches
                .iter()
                .copied()
                .find(|node| current_property_value(action, node, property).is_some())
            {
                return node;
            }
        }
    }

    matches[0]
}

fn current_node_summary(node: &Node) -> CurrentNodeSummary {
    CurrentNodeSummary {
        id: node.id.0.clone(),
        kind: node.kind,
        name: node.name.clone(),
        path: node.path.clone(),
        size_bytes: node.size_bytes,
    }
}

fn size_diagnostic(action: &PlannedAction, node: &Node, query: &str) -> Option<TopologyDiagnostic> {
    let desired = size_diagnostic_desired_size(action)?;
    let desired_bytes = parse_size_bytes(desired)?;
    let current_bytes = node.size_bytes?;

    let (level, kind, message) = match action.operation {
        Operation::Grow if current_bytes >= desired_bytes => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::SizeAlreadySatisfied,
            format!("current size {current_bytes} bytes already satisfies desired size {desired}"),
        ),
        Operation::Grow => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::SizeBelowDesired,
            format!("current size {current_bytes} bytes is below desired size {desired}"),
        ),
        Operation::Shrink if current_bytes <= desired_bytes => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::SizeAlreadySatisfied,
            format!(
                "current size {current_bytes} bytes is already at or below desired size {desired}"
            ),
        ),
        Operation::Shrink => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::SizeConflict,
            format!("current size {current_bytes} bytes is above desired shrink target {desired}"),
        ),
        _ => return None,
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level,
        kind,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn size_diagnostic_desired_size(action: &PlannedAction) -> Option<&str> {
    action.context.desired_size.as_deref().or_else(|| {
        if action.operation == Operation::Grow
            && action.context.collection.as_deref() == Some("partitions")
        {
            action.context.end.as_deref()
        } else {
            None
        }
    })
}

fn filesystem_type_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    let desired = action.context.fs_type.as_deref()?;
    let current = property_value_from_node(node, "filesystem.type")?;
    if current == desired {
        if action.operation == Operation::Format
            && action.context.collection.as_deref() == Some("filesystems")
        {
            return Some(TopologyDiagnostic {
                action_id: action.id.clone(),
                level: TopologyDiagnosticLevel::Info,
                kind: TopologyDiagnosticKind::FilesystemFormatAlreadySatisfied,
                query: query.to_string(),
                message: format!("filesystem {query} already reports type {current}"),
                current: Some(current_node_summary(node)),
            });
        }
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::FilesystemTypeConflict,
        query: query.to_string(),
        message: format!("desired filesystem type {desired} differs from current {current}"),
        current: Some(current_node_summary(node)),
    })
}

fn disk_create_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Create
        || action.context.collection.as_deref() != Some("disks")
    {
        return None;
    }

    let desired_table = action.context.partition_type.as_deref().unwrap_or("gpt");

    if node.kind != NodeKind::PhysicalDisk {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::DiskCreateRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not a physical disk; partition table initialization remains actionable only after target review",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    let Some(current_table) = property_value_from_node(node, "partition.table") else {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::DiskCreateRequired,
            query: query.to_string(),
            message: format!(
                "disk {query} current partition table is unknown; desired {desired_table} remains actionable after disk identity review"
            ),
            current: Some(current_node_summary(node)),
        });
    };

    let (level, kind, message) = if current_table.eq_ignore_ascii_case(desired_table) {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::DiskCreateAlreadySatisfied,
            format!("disk {query} already has partition table {current_table}"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::DiskCreateRequired,
            format!(
                "disk {query} has partition table {current_table}, desired {desired_table}; mklabel remains destructive and requires review"
            ),
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level,
        kind,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn partition_create_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Create
        || action.context.collection.as_deref() != Some("partitions")
    {
        return None;
    }

    if node.kind != NodeKind::Partition {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::PartitionCreateRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not a partition; parted mkpart remains actionable only after target review",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    let desired = action.context.desired_size.as_deref();
    let desired_bytes = desired.and_then(parse_size_bytes);
    let (level, kind, message) = match (desired, desired_bytes, node.size_bytes) {
        (Some(desired), Some(desired_bytes), Some(current_bytes))
            if current_bytes == desired_bytes =>
        {
            (
                TopologyDiagnosticLevel::Info,
                TopologyDiagnosticKind::PartitionCreateAlreadySatisfied,
                format!(
                    "partition {query} already exists with desired size {desired} ({current_bytes} bytes)"
                ),
            )
        }
        (None, _, _) => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::PartitionCreateAlreadySatisfied,
            format!("partition {query} already exists"),
        ),
        (Some(desired), Some(_), Some(current_bytes)) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::PartitionCreateRequired,
            format!(
                "partition {query} already exists with size {current_bytes} bytes, not desired size {desired}; use grow or recreate only after data-preservation review"
            ),
        ),
        (Some(desired), None, _) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::PartitionCreateRequired,
            format!(
                "partition {query} already exists, but desired size {desired} could not be compared"
            ),
        ),
        (Some(desired), Some(_), None) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::PartitionCreateRequired,
            format!(
                "partition {query} already exists, but current size is unknown; desired size is {desired}"
            ),
        ),
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level,
        kind,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn property_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::SetProperty {
        return None;
    }
    let property = action.context.property.as_deref()?;
    let desired = action.context.property_value.as_deref()?;
    let current = current_property_value(action, node, property)?;
    let desired_compare = comparable_property_value(action, property, desired);
    let current_compare = comparable_property_value(action, property, &current);
    let (level, kind, message) = if current_compare == desired_compare {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::PropertyAlreadySatisfied,
            format!("property {property} already has desired value {desired}"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::PropertyDiffers,
            format!("property {property} is {current}, desired {desired}"),
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level,
        kind,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn current_property_value(action: &PlannedAction, node: &Node, property: &str) -> Option<String> {
    let normalized = normalize_storage_property_name(property);
    let aliases: Option<&[&str]> = match action.context.collection.as_deref() {
        Some("vdoVolumes") => Some(match normalized.as_str() {
            "writepolicy" | "write-policy" | "vdo-write-policy" => {
                &["vdo.write-policy", "lvm.vdo-write-policy", property]
            }
            "compression" | "vdo-compression" => {
                &["vdo.compression", "lvm.vdo-compression", property]
            }
            "deduplication" | "dedupe" | "vdo-deduplication" | "vdo-dedupe" => &[
                "vdo.deduplication",
                "vdo.dedupe",
                "lvm.vdo-deduplication",
                "lvm.vdo-dedupe",
                property,
            ],
            _ => &[property],
        }),
        Some("lvmCaches") => Some(match normalized.as_str() {
            "cachemode" | "cache-mode" | "lvm-cache-mode" => {
                &["lvm.cache-mode", "lvm.cacheMode", property]
            }
            "cachepolicy" | "cache-policy" | "lvm-cache-policy" => {
                &["lvm.cache-policy", "lvm.cachePolicy", property]
            }
            _ => &[property],
        }),
        Some("caches") => Some(match normalized.as_str() {
            "cachemode" | "cache-mode" | "bcache-cache-mode" => {
                &["bcache.cache-mode", "bcache.cacheMode", property]
            }
            "cachepolicy" | "cache-policy" | "bcache-cache-policy" => {
                &["bcache.cache-policy", "bcache.cachePolicy", property]
            }
            _ => &[property],
        }),
        Some("pools") => Some(match normalized.as_str() {
            "altroot" => &["zfs.pool-altroot", "zfs.altroot", property],
            "ashift" => &["zfs.pool-ashift", "zfs.ashift", property],
            "autotrim" | "auto-trim" => &["zfs.pool-autotrim", "zfs.autotrim", property],
            "autoexpand" | "auto-expand" => &["zfs.pool-autoexpand", "zfs.autoexpand", property],
            "autoreplace" | "auto-replace" => {
                &["zfs.pool-autoreplace", "zfs.autoreplace", property]
            }
            "bootfs" | "boot-fs" => &["zfs.pool-bootfs", "zfs.bootfs", property],
            "cachefile" | "cache-file" => &["zfs.pool-cachefile", "zfs.cachefile", property],
            "comment" => &["zfs.pool-comment", "zfs.comment", property],
            "delegation" => &["zfs.pool-delegation", "zfs.delegation", property],
            "failmode" | "fail-mode" => &["zfs.pool-failmode", "zfs.failmode", property],
            "listsnapshots" | "list-snapshots" => {
                &["zfs.pool-listsnapshots", "zfs.listsnapshots", property]
            }
            "multihost" | "multi-host" => &["zfs.pool-multihost", "zfs.multihost", property],
            _ => &[property],
        }),
        Some("datasets" | "zvols") => Some(match normalized.as_str() {
            "mountpoint" => &["zfs.mountpoint", property],
            "compression" => &["zfs.compression", property],
            "quota" => &["zfs.quota", property],
            "reservation" => &["zfs.reservation", property],
            "encryption" => &["zfs.encryption", property],
            "keystatus" | "key-status" => &["zfs.keystatus", property],
            "volsize" | "vol-size" => &["zfs.volsize", property],
            "recordsize" | "record-size" => &["zfs.recordsize", property],
            "dedup" => &["zfs.dedup", property],
            "checksum" => &["zfs.checksum", property],
            "copies" => &["zfs.copies", property],
            "sync" => &["zfs.sync", property],
            "primarycache" | "primary-cache" => &["zfs.primarycache", property],
            "secondarycache" | "secondary-cache" => &["zfs.secondarycache", property],
            "atime" => &["zfs.atime", property],
            "relatime" => &["zfs.relatime", property],
            "snapdir" | "snap-dir" => &["zfs.snapdir", property],
            "acltype" | "acl-type" => &["zfs.acltype", property],
            "xattr" => &["zfs.xattr", property],
            _ => &[property],
        }),
        _ => None,
    };

    if action.context.collection.as_deref() == Some("filesystems") {
        return current_filesystem_property_value(action, node, property)
            .or_else(|| property_value_from_node(node, property).map(str::to_string));
    }

    if action.context.collection.as_deref() == Some("swaps") {
        return current_swap_property_value(property, node)
            .or_else(|| property_value_from_node(node, property).map(str::to_string));
    }

    if action.context.collection.as_deref() == Some("zram") {
        return current_zram_property_value(property, node)
            .or_else(|| property_value_from_node(node, property).map(str::to_string));
    }

    if action.context.collection.as_deref() == Some("luksKeyslots") {
        return current_luks_keyslot_property_value(action, node, property)
            .or_else(|| property_value_from_node(node, property).map(str::to_string));
    }

    if action.context.collection.as_deref() == Some("luks.devices") {
        return current_luks_property_value(property, node)
            .or_else(|| property_value_from_node(node, property).map(str::to_string));
    }

    if action.context.collection.as_deref() == Some("btrfsQgroups") {
        return current_btrfs_qgroup_property_value(property, node)
            .or_else(|| property_value_from_node(node, property).map(str::to_string));
    }

    if action.context.collection.as_deref() == Some("caches") {
        if let Some(alias) = bcache_cache_set_property_key(property) {
            return property_value_from_node(node, &alias)
                .or_else(|| property_value_from_node(node, property))
                .map(str::to_string);
        }
    }

    if let Some(aliases) = aliases {
        return aliases
            .iter()
            .find_map(|alias| property_value_from_node(node, alias).map(str::to_string));
    }

    property_value_from_node(node, property).map(str::to_string)
}

fn comparable_property_value(action: &PlannedAction, property: &str, value: &str) -> String {
    let normalized_property = normalize_storage_property_name(property);
    let normalized_value = normalize_storage_property_name(value);
    match action.context.collection.as_deref() {
        Some("vdoVolumes") => match normalized_property.as_str() {
            "compression" | "vdo-compression" | "deduplication" | "dedupe"
            | "vdo-deduplication" | "vdo-dedupe" => {
                normalize_vdo_boolean_property_value(&normalized_value)
                    .map(str::to_string)
                    .unwrap_or(normalized_value)
            }
            "writepolicy" | "write-policy" | "vdo-write-policy" => normalized_value,
            _ => value.to_string(),
        },
        Some("lvmCaches" | "caches") => match normalized_property.as_str() {
            "cachemode"
            | "cache-mode"
            | "lvm-cache-mode"
            | "bcache-cache-mode"
            | "cachepolicy"
            | "cache-policy"
            | "lvm-cache-policy"
            | "bcache-cache-policy" => {
                normalize_cache_property_value(&normalized_property, &normalized_value)
            }
            _ => value.to_string(),
        },
        Some("pools") => {
            normalize_zfs_pool_property_value(&normalized_property, &normalized_value, value)
        }
        Some("datasets" | "zvols") => {
            normalize_zfs_property_value(&normalized_property, &normalized_value, value)
        }
        Some("filesystems") => {
            normalize_filesystem_property_value(action, &normalized_property, value)
                .unwrap_or_else(|| value.to_string())
        }
        Some("swaps") => normalize_swap_property_value(&normalized_property, value)
            .unwrap_or_else(|| value.to_string()),
        Some("zram") => normalize_zram_property_value(&normalized_property, value)
            .unwrap_or_else(|| value.to_string()),
        Some("luks.devices") => normalize_luks_property_value(&normalized_property, value)
            .unwrap_or_else(|| value.to_string()),
        Some("luksKeyslots") => normalize_luks_keyslot_property_value(&normalized_property, value)
            .unwrap_or_else(|| value.to_string()),
        Some("btrfsQgroups") => normalize_btrfs_qgroup_property_value(&normalized_property, value)
            .unwrap_or_else(|| value.to_string()),
        _ => value.to_string(),
    }
}

fn current_luks_keyslot_property_value(
    action: &PlannedAction,
    node: &Node,
    property: &str,
) -> Option<String> {
    match luks_keyslot_property_kind(property)? {
        LuksKeyslotPropertyKind::Priority => {
            let key_slot = action.context.key_slot.as_deref().or_else(|| {
                action
                    .context
                    .name
                    .as_deref()
                    .and_then(|name| name.rsplit_once(':').map(|(_, slot)| slot).or(Some(name)))
                    .filter(|slot| slot.chars().all(|character| character.is_ascii_digit()))
            })?;
            property_value_from_node(
                node,
                &format!("cryptsetup.luks-keyslot-{key_slot}-priority"),
            )
            .or_else(|| property_value_from_node(node, property))
            .map(str::to_string)
        }
        LuksKeyslotPropertyKind::KeyFile => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LuksKeyslotPropertyKind {
    KeyFile,
    Priority,
}

fn luks_keyslot_property_kind(property: &str) -> Option<LuksKeyslotPropertyKind> {
    match normalize_storage_property_name(property).as_str() {
        "keyfile"
        | "key-file"
        | "luks-keyfile"
        | "luks-key-file"
        | "cryptsetup-keyfile"
        | "cryptsetup-key-file" => Some(LuksKeyslotPropertyKind::KeyFile),
        "priority" | "luks-keyslot-priority" | "cryptsetup-luks-keyslot-priority" => {
            Some(LuksKeyslotPropertyKind::Priority)
        }
        _ => None,
    }
}

fn normalize_luks_keyslot_property_value(property: &str, value: &str) -> Option<String> {
    match luks_keyslot_property_kind(property)? {
        LuksKeyslotPropertyKind::KeyFile => Some(value.to_string()),
        LuksKeyslotPropertyKind::Priority => Some(normalize_storage_property_name(value)),
    }
}

fn current_btrfs_qgroup_property_value(property: &str, node: &Node) -> Option<String> {
    match btrfs_qgroup_property_kind(property)? {
        BtrfsQgroupPropertyKind::MaxReferenced => {
            property_value_from_node(node, "btrfs.max-referenced")
                .or_else(|| property_value_from_node(node, "btrfs.referenced-limit"))
                .map(str::to_string)
        }
        BtrfsQgroupPropertyKind::MaxExclusive => {
            property_value_from_node(node, "btrfs.max-exclusive")
                .or_else(|| property_value_from_node(node, "btrfs.exclusive-limit"))
                .map(str::to_string)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BtrfsQgroupPropertyKind {
    MaxReferenced,
    MaxExclusive,
}

fn btrfs_qgroup_property_kind(property: &str) -> Option<BtrfsQgroupPropertyKind> {
    match normalize_storage_property_name(property).as_str() {
        "limit"
        | "referenced"
        | "maxreferenced"
        | "max-referenced"
        | "btrfs-max-referenced"
        | "btrfs-referenced-limit" => Some(BtrfsQgroupPropertyKind::MaxReferenced),
        "exclusive"
        | "maxexclusive"
        | "max-exclusive"
        | "btrfs-max-exclusive"
        | "btrfs-exclusive-limit" => Some(BtrfsQgroupPropertyKind::MaxExclusive),
        _ => None,
    }
}

fn normalize_btrfs_qgroup_property_value(property: &str, value: &str) -> Option<String> {
    match btrfs_qgroup_property_kind(property)? {
        BtrfsQgroupPropertyKind::MaxReferenced | BtrfsQgroupPropertyKind::MaxExclusive => {
            let trimmed = value.trim();
            if matches!(
                normalize_storage_property_name(trimmed).as_str(),
                "none" | "null" | "unlimited" | "---"
            ) {
                Some("none".to_string())
            } else {
                Some(trimmed.to_string())
            }
        }
    }
}

fn current_luks_property_value(property: &str, node: &Node) -> Option<String> {
    match luks_property_kind(property)? {
        LuksPropertyKind::Label => node.identity.label.clone().or_else(|| {
            property_value_from_node(node, "cryptsetup.label")
                .or_else(|| property_value_from_node(node, "cryptsetup.luks-label"))
                .map(str::to_string)
        }),
        LuksPropertyKind::Subsystem => property_value_from_node(node, "cryptsetup.luks-subsystem")
            .or_else(|| property_value_from_node(node, "cryptsetup.subsystem"))
            .map(str::to_string),
        LuksPropertyKind::Uuid => node.identity.uuid.clone().or_else(|| {
            property_value_from_node(node, "cryptsetup.uuid")
                .or_else(|| property_value_from_node(node, "cryptsetup.luks-uuid"))
                .map(str::to_string)
        }),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LuksPropertyKind {
    Label,
    Subsystem,
    Uuid,
}

fn luks_property_kind(property: &str) -> Option<LuksPropertyKind> {
    match normalize_storage_property_name(property).as_str() {
        "label" | "luks-label" | "cryptsetup-label" => Some(LuksPropertyKind::Label),
        "subsystem" | "luks-subsystem" | "cryptsetup-subsystem" => {
            Some(LuksPropertyKind::Subsystem)
        }
        "uuid" | "luks-uuid" | "cryptsetup-uuid" => Some(LuksPropertyKind::Uuid),
        _ => None,
    }
}

fn normalize_luks_property_value(property: &str, value: &str) -> Option<String> {
    match luks_property_kind(property)? {
        LuksPropertyKind::Label | LuksPropertyKind::Subsystem => Some(value.to_string()),
        LuksPropertyKind::Uuid => Some(value.trim().to_ascii_lowercase()),
    }
}

fn current_swap_property_value(property: &str, node: &Node) -> Option<String> {
    match swap_property_kind(property)? {
        SwapPropertyKind::Label => node.identity.label.clone().or_else(|| {
            property_value_from_node(node, "swap.label")
                .or_else(|| property_value_from_node(node, "udev.id-fs-label"))
                .or_else(|| property_value_from_node(node, "udev.id-fs-label-safe"))
                .map(str::to_string)
        }),
        SwapPropertyKind::Uuid => node.identity.uuid.clone().or_else(|| {
            property_value_from_node(node, "swap.uuid")
                .or_else(|| property_value_from_node(node, "udev.id-fs-uuid"))
                .map(str::to_string)
        }),
        SwapPropertyKind::Priority => {
            property_value_from_node(node, "swap.priority").and_then(normalize_swap_priority)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SwapPropertyKind {
    Label,
    Uuid,
    Priority,
}

fn swap_property_kind(property: &str) -> Option<SwapPropertyKind> {
    match normalize_storage_property_name(property).as_str() {
        "label" | "swap-label" => Some(SwapPropertyKind::Label),
        "uuid" | "swap-uuid" => Some(SwapPropertyKind::Uuid),
        "priority" | "swap-priority" => Some(SwapPropertyKind::Priority),
        _ => None,
    }
}

fn normalize_swap_property_value(property: &str, value: &str) -> Option<String> {
    match swap_property_kind(property)? {
        SwapPropertyKind::Label => Some(value.to_string()),
        SwapPropertyKind::Uuid => Some(value.trim().to_ascii_lowercase()),
        SwapPropertyKind::Priority => normalize_swap_priority(value),
    }
}

fn normalize_swap_priority(value: &str) -> Option<String> {
    value
        .trim()
        .parse::<i32>()
        .ok()
        .map(|priority| priority.to_string())
}

fn current_zram_property_value(property: &str, node: &Node) -> Option<String> {
    match zram_property_kind(property)? {
        ZramPropertyKind::Algorithm => property_value_from_node(node, "zram.algorithm")
            .or_else(|| property_value_from_node(node, "zram.compression-algorithm"))
            .map(str::to_string),
        ZramPropertyKind::Streams => {
            property_value_from_node(node, "zram.streams").and_then(normalize_integer_property)
        }
        ZramPropertyKind::DiskSize => property_value_from_node(node, "zram.disksize")
            .or_else(|| property_value_from_node(node, "zram.disk-size"))
            .and_then(normalize_integer_property),
        ZramPropertyKind::MemoryLimit => {
            property_value_from_node(node, "zram.memory-limit").and_then(normalize_integer_property)
        }
        ZramPropertyKind::CompressionRatio => {
            property_value_from_node(node, "zram.compression-ratio")
                .or_else(|| property_value_from_node(node, "zram.ratio"))
                .map(normalize_decimal_property)
        }
        ZramPropertyKind::Priority => {
            property_value_from_node(node, "swap.priority").and_then(normalize_swap_priority)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ZramPropertyKind {
    Algorithm,
    Streams,
    DiskSize,
    MemoryLimit,
    CompressionRatio,
    Priority,
}

fn zram_property_kind(property: &str) -> Option<ZramPropertyKind> {
    match normalize_storage_property_name(property).as_str() {
        "algorithm" | "zram-algorithm" | "compression-algorithm" => {
            Some(ZramPropertyKind::Algorithm)
        }
        "streams" | "zram-streams" => Some(ZramPropertyKind::Streams),
        "disksize" | "disk-size" | "zram-disksize" | "zram-disk-size" => {
            Some(ZramPropertyKind::DiskSize)
        }
        "memorylimit" | "memory-limit" | "zram-memory-limit" => Some(ZramPropertyKind::MemoryLimit),
        "compressionratio"
        | "compression-ratio"
        | "compression-ratio-target"
        | "zram-compression-ratio"
        | "zram-compression-ratio-target"
        | "ratio"
        | "zram-ratio" => Some(ZramPropertyKind::CompressionRatio),
        "priority" | "zram-priority" | "swap-priority" => Some(ZramPropertyKind::Priority),
        _ => None,
    }
}

fn normalize_zram_property_value(property: &str, value: &str) -> Option<String> {
    match zram_property_kind(property)? {
        ZramPropertyKind::Algorithm => Some(normalize_storage_property_name(value)),
        ZramPropertyKind::Streams | ZramPropertyKind::DiskSize | ZramPropertyKind::MemoryLimit => {
            normalize_integer_property(value)
        }
        ZramPropertyKind::CompressionRatio => Some(normalize_decimal_property(value)),
        ZramPropertyKind::Priority => normalize_swap_priority(value),
    }
}

fn normalize_integer_property(value: &str) -> Option<String> {
    value
        .trim()
        .parse::<u64>()
        .ok()
        .map(|number| number.to_string())
}

fn normalize_decimal_property(value: &str) -> String {
    value
        .trim()
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FilesystemPropertyKind {
    Label,
    Uuid,
    FatVolumeId,
    NtfsVolumeSerial,
    ExfatVolumeSerial,
}

fn current_filesystem_property_value(
    action: &PlannedAction,
    node: &Node,
    property: &str,
) -> Option<String> {
    match filesystem_property_kind(action, property)? {
        FilesystemPropertyKind::Label => node.identity.label.clone().or_else(|| {
            property_value_from_node(node, "filesystem.label")
                .or_else(|| property_value_from_node(node, "udev.id-fs-label"))
                .or_else(|| property_value_from_node(node, "udev.id-fs-label-safe"))
                .or_else(|| property_value_from_node(node, "ntfs.volume-name"))
                .map(str::to_string)
        }),
        FilesystemPropertyKind::Uuid => node.identity.uuid.clone().or_else(|| {
            property_value_from_node(node, "filesystem.uuid")
                .or_else(|| property_value_from_node(node, "udev.id-fs-uuid"))
                .map(str::to_string)
        }),
        FilesystemPropertyKind::FatVolumeId => node.identity.uuid.clone().or_else(|| {
            property_value_from_node(node, "filesystem.uuid")
                .or_else(|| property_value_from_node(node, "udev.id-fs-uuid"))
                .map(str::to_string)
        }),
        FilesystemPropertyKind::NtfsVolumeSerial => node
            .identity
            .serial
            .clone()
            .or_else(|| node.identity.uuid.clone())
            .or_else(|| property_value_from_node(node, "ntfs.volume-serial").map(str::to_string)),
        FilesystemPropertyKind::ExfatVolumeSerial => node
            .identity
            .serial
            .clone()
            .or_else(|| node.identity.uuid.clone())
            .or_else(|| property_value_from_node(node, "exfat.volume-serial").map(str::to_string)),
    }
}

fn filesystem_property_kind(
    action: &PlannedAction,
    property: &str,
) -> Option<FilesystemPropertyKind> {
    let normalized = normalize_storage_property_name(property);
    let fs_type = action
        .context
        .fs_type
        .as_deref()
        .map(|value| value.to_ascii_lowercase());
    let fs_type = fs_type.as_deref();

    if matches!(
        normalized.as_str(),
        "label"
            | "filesystem-label"
            | "btrfs-label"
            | "ext-label"
            | "fat-label"
            | "vfat-label"
            | "ntfs-label"
            | "exfat-label"
            | "f2fs-label"
            | "xfs-label"
    ) {
        return Some(FilesystemPropertyKind::Label);
    }

    if matches!(normalized.as_str(), "serial" | "volume-serial") {
        return match fs_type {
            Some("exfat") => Some(FilesystemPropertyKind::ExfatVolumeSerial),
            _ => Some(FilesystemPropertyKind::NtfsVolumeSerial),
        };
    }

    if matches!(normalized.as_str(), "ntfs-serial" | "ntfs-volume-serial") {
        return Some(FilesystemPropertyKind::NtfsVolumeSerial);
    }

    if matches!(normalized.as_str(), "exfat-serial" | "exfat-volume-serial") {
        return Some(FilesystemPropertyKind::ExfatVolumeSerial);
    }

    if matches!(
        normalized.as_str(),
        "volume-id" | "fat-volume-id" | "vfat-volume-id"
    ) {
        return Some(FilesystemPropertyKind::FatVolumeId);
    }

    if matches!(
        normalized.as_str(),
        "uuid"
            | "filesystem-uuid"
            | "btrfs-uuid"
            | "ext-uuid"
            | "fat-uuid"
            | "vfat-uuid"
            | "ntfs-uuid"
            | "exfat-uuid"
            | "xfs-uuid"
    ) {
        return match fs_type {
            Some("fat" | "fat12" | "fat16" | "fat32" | "msdos" | "vfat") => {
                Some(FilesystemPropertyKind::FatVolumeId)
            }
            Some("ntfs" | "ntfs3") => Some(FilesystemPropertyKind::NtfsVolumeSerial),
            Some("exfat") => Some(FilesystemPropertyKind::ExfatVolumeSerial),
            _ => Some(FilesystemPropertyKind::Uuid),
        };
    }

    None
}

fn normalize_filesystem_property_value(
    action: &PlannedAction,
    normalized_property: &str,
    raw_value: &str,
) -> Option<String> {
    match filesystem_property_kind(action, normalized_property)? {
        FilesystemPropertyKind::Label => Some(raw_value.to_string()),
        FilesystemPropertyKind::Uuid => Some(raw_value.trim().to_ascii_lowercase()),
        FilesystemPropertyKind::FatVolumeId => normalize_hex_identity(raw_value, 8),
        FilesystemPropertyKind::NtfsVolumeSerial => normalize_hex_identity(raw_value, 16),
        FilesystemPropertyKind::ExfatVolumeSerial => normalize_hex_identity(raw_value, 8),
    }
}

fn normalize_hex_identity(value: &str, expected_len: usize) -> Option<String> {
    let trimmed = value.trim();
    let without_prefix = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    let normalized: String = without_prefix
        .chars()
        .filter(|character| *character != '-')
        .collect();
    if normalized.len() == expected_len
        && normalized
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        Some(normalized.to_ascii_uppercase())
    } else {
        Some(normalize_storage_property_name(value))
    }
}

fn normalize_cache_property_value(property: &str, value: &str) -> String {
    match property {
        "cachemode" | "cache-mode" | "lvm-cache-mode" | "bcache-cache-mode" => {
            value.replace('-', "")
        }
        _ => value.to_string(),
    }
}

fn normalize_zfs_pool_property_value(
    property: &str,
    normalized_value: &str,
    raw_value: &str,
) -> String {
    match property {
        "autotrim" | "auto-trim" | "autoexpand" | "auto-expand" | "autoreplace"
        | "auto-replace" | "delegation" | "listsnapshots" | "list-snapshots" | "multihost"
        | "multi-host" => normalize_zfs_boolean_property_value(normalized_value)
            .map(str::to_string)
            .unwrap_or_else(|| normalized_value.to_string()),
        "altroot" | "ashift" | "bootfs" | "boot-fs" | "cachefile" | "cache-file" | "comment"
        | "failmode" | "fail-mode" => normalized_value.to_string(),
        _ => raw_value.to_string(),
    }
}

fn normalize_zfs_property_value(property: &str, normalized_value: &str, raw_value: &str) -> String {
    match property {
        "dedup" | "atime" | "relatime" => normalize_zfs_boolean_property_value(normalized_value)
            .map(str::to_string)
            .unwrap_or_else(|| normalized_value.to_string()),
        "primarycache" | "primary-cache" | "secondarycache" | "secondary-cache" => {
            normalized_value.to_string()
        }
        "mountpoint" | "compression" | "quota" | "reservation" | "encryption" | "keystatus"
        | "key-status" | "volsize" | "vol-size" | "recordsize" | "record-size" | "checksum"
        | "copies" | "sync" | "snapdir" | "snap-dir" | "acltype" | "acl-type" | "xattr" => {
            normalized_value.to_string()
        }
        _ => raw_value.to_string(),
    }
}

fn normalize_zfs_boolean_property_value(value: &str) -> Option<&'static str> {
    match value {
        "on" | "yes" | "true" | "enabled" | "enable" | "1" => Some("on"),
        "off" | "no" | "false" | "disabled" | "disable" | "0" => Some("off"),
        _ => None,
    }
}

fn normalize_vdo_boolean_property_value(value: &str) -> Option<&'static str> {
    match value {
        "enabled" | "enable" | "true" | "yes" | "on" | "1" => Some("enabled"),
        "disabled" | "disable" | "false" | "no" | "off" | "0" => Some("disabled"),
        _ => None,
    }
}
