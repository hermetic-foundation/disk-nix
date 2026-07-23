fn snapshot_property_command(
    snapshot: &str,
    property: &str,
    tag: Option<&str>,
) -> ExecutionCommand {
    let Some(tag) = tag else {
        return command_with_readiness(
            ["zfs", "hold", "<tag>", snapshot],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["ZFS hold tag"],
            "update a ZFS snapshot hold after selecting the hold tag",
        );
    };
    if !is_zfs_snapshot_name(snapshot) {
        return command_with_readiness(
            ["<snapshot-property-tool>", snapshot, tag],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["ZFS snapshot name"],
            "update snapshot retention with the target-specific snapshot property tool",
        );
    }
    match property {
        "zfs.hold" | "hold" | "holdTag" => command(
            ["zfs", "hold", tag, snapshot],
            true,
            "add a ZFS snapshot hold with the reviewed retention tag",
        ),
        "zfs.releaseHold" | "releaseHold" | "release-hold" => command(
            ["zfs", "release", tag, snapshot],
            true,
            "release a ZFS snapshot hold with the reviewed retention tag",
        ),
        _ => command_with_readiness(
            ["<snapshot-property-tool>", snapshot, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported snapshot property"],
            "update a snapshot property after selecting a supported domain mapping",
        ),
    }
}

fn snapshot_rescan_identity<'a>(action: &'a PlannedAction, fallback: &'a str) -> &'a str {
    action
        .context
        .snapshot_path
        .as_deref()
        .or(action.context.name.as_deref())
        .unwrap_or(fallback)
}

fn snapshot_hold_list_command(snapshot: &str) -> ExecutionCommand {
    if is_zfs_snapshot_name(snapshot) {
        command(
            ["zfs", "holds", snapshot],
            false,
            "verify ZFS snapshot hold tags",
        )
    } else {
        command_with_readiness(
            ["<snapshot-hold-list-tool>", snapshot],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["ZFS snapshot name"],
            "verify snapshot hold state with the target-specific tool",
        )
    }
}

fn zfs_snapshot_list_command(snapshot: &str, note: &str) -> ExecutionCommand {
    command(
        ["zfs", "list", "-t", "snapshot", "-H", "-p", snapshot],
        false,
        note,
    )
}

fn zfs_snapshot_rollback_command(snapshot: &str, recursive: bool) -> ExecutionCommand {
    if recursive {
        command(
            ["zfs", "rollback", "-r", snapshot],
            true,
            "recursively roll back the ZFS dataset after explicit review of newer snapshots",
        )
    } else {
        command(
            ["zfs", "rollback", snapshot],
            true,
            "roll back the ZFS dataset to the reviewed snapshot",
        )
    }
}

fn snapshot_command(
    collection: Option<&str>,
    target: &str,
    snapshot: &str,
    read_only: bool,
) -> ExecutionCommand {
    if is_zfs_snapshot_name(snapshot) {
        command(["zfs", "snapshot", snapshot], true, "create a ZFS snapshot")
    } else if collection == Some("btrfsSubvolumes") || is_btrfs_snapshot_pair(target, snapshot) {
        if read_only {
            command(
                ["btrfs", "subvolume", "snapshot", "-r", target, snapshot],
                true,
                "create a read-only Btrfs subvolume snapshot",
            )
        } else {
            command(
                ["btrfs", "subvolume", "snapshot", target, snapshot],
                true,
                "create a Btrfs subvolume snapshot",
            )
        }
    } else {
        command_with_readiness(
            ["<snapshot-tool>", target, snapshot],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["snapshot tool"],
            "create the snapshot with zfs, btrfs, lvm, or the target-specific tool",
        )
    }
}

fn is_zfs_snapshot_name(snapshot: &str) -> bool {
    let Some((dataset, name)) = snapshot.split_once('@') else {
        return false;
    };
    !dataset.is_empty() && !name.is_empty() && !dataset.starts_with('/')
}

fn zfs_snapshot_dataset(snapshot: &str) -> Option<&str> {
    snapshot.split_once('@').map(|(dataset, _)| dataset)
}

fn is_btrfs_snapshot_pair(target: &str, snapshot: &str) -> bool {
    target.starts_with('/') && snapshot.starts_with('/')
}
