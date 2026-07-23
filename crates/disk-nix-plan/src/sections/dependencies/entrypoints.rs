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
