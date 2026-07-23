fn operation_dependency_phase_kind(operation: Operation) -> DependencyPhase {
    match operation {
        Operation::Create
        | Operation::Import
        | Operation::Login
        | Operation::Attach
        | Operation::Open
        | Operation::Activate
        | Operation::Assemble
        | Operation::Start => DependencyPhase::BuildLowerLayers,
        Operation::Format
        | Operation::Grow
        | Operation::AddDevice
        | Operation::ReplaceDevice
        | Operation::AddKey
        | Operation::ImportToken
        | Operation::SetProperty
        | Operation::Snapshot
        | Operation::Clone
        | Operation::Promote
        | Operation::Mount
        | Operation::Remount
        | Operation::Check
        | Operation::Repair
        | Operation::Scrub
        | Operation::Trim
        | Operation::Rescan
        | Operation::Rename
        | Operation::Rebalance => DependencyPhase::MutateInPlace,
        Operation::Shrink
        | Operation::RemoveDevice
        | Operation::RemoveKey
        | Operation::RemoveToken
        | Operation::Rollback
        | Operation::Unmount
        | Operation::Close
        | Operation::Logout
        | Operation::Deactivate
        | Operation::Stop
        | Operation::Detach
        | Operation::Export
        | Operation::Unexport
        | Operation::Destroy => DependencyPhase::TearDownUpperLayers,
    }
}

fn operation_dependency_phase(operation: Operation) -> u16 {
    match operation_dependency_phase_kind(operation) {
        DependencyPhase::BuildLowerLayers => 10,
        DependencyPhase::MutateInPlace => 20,
        DependencyPhase::TearDownUpperLayers => 30,
    }
}

fn operation_runs_upper_layers_first(operation: Operation) -> bool {
    matches!(
        operation,
        Operation::Shrink
            | Operation::RemoveDevice
            | Operation::RemoveKey
            | Operation::RemoveToken
            | Operation::Rollback
            | Operation::Unmount
            | Operation::Close
            | Operation::Logout
            | Operation::Deactivate
            | Operation::Stop
            | Operation::Detach
            | Operation::Export
            | Operation::Unexport
            | Operation::Destroy
    )
}

fn collection_dependency_rank(collection: Option<&str>) -> u16 {
    match collection {
        Some("backingFiles") => 10,
        Some("loopDevices") => 15,
        Some("disks") => 20,
        Some("iscsiSessions") => 25,
        Some("nvmeNamespaces") => 30,
        Some("targetLuns") => 32,
        Some("luns") => 35,
        Some("partitions") => 40,
        Some("mdRaids") | Some("multipathMaps") => 45,
        Some("luks.devices") | Some("dmMaps") => 50,
        Some("physicalVolumes") => 55,
        Some("volumeGroups") => 60,
        Some("thinPools") | Some("volumes") | Some("lvmCaches") | Some("lvmSnapshots") => 65,
        Some("vdoVolumes") | Some("caches") => 70,
        Some("pools") => 75,
        Some("datasets") | Some("zvols") => 80,
        Some("btrfsQgroups") => 85,
        Some("filesystems") | Some("swaps") | Some("zram") | Some("nfs.mounts") => 90,
        Some("btrfsSubvolumes") => 92,
        Some("snapshots") | Some("exports") => 95,
        Some(_) | None => 100,
    }
}
