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
