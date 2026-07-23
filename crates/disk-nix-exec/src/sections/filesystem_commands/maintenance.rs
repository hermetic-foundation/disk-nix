fn rebalance_command(
    collection: Option<&str>,
    fs_type: Option<&str>,
    target: &str,
    property_assignments: &[String],
) -> ExecutionCommand {
    match collection {
        Some("pools") => command(
            ["zpool", "scrub", target],
            true,
            "scrub the pool after topology changes; ZFS has no generic rebalance command",
        ),
        Some("filesystems") if fs_type == Some("bcachefs") => bcachefs_rereplicate_command(target),
        Some("filesystems") => {
            let mut argv = vec![
                "btrfs".to_string(),
                "balance".to_string(),
                "start".to_string(),
            ];
            argv.extend(btrfs_balance_filters(property_assignments));
            argv.push(target.to_string());
            command_vec(
                argv,
                true,
                "rebalance Btrfs chunks across available devices",
            )
        }
        _ => command_with_readiness(
            ["<rebalance-tool>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["rebalance tool"],
            "run the storage-domain rebalance command",
        ),
    }
}

fn scrub_command(
    collection: Option<&str>,
    fs_type: Option<&str>,
    target: &str,
) -> ExecutionCommand {
    match collection {
        Some("pools") => command(
            ["zpool", "scrub", target],
            true,
            "start the reviewed ZFS pool scrub",
        ),
        Some("filesystems") if fs_type == Some("bcachefs") => command(
            ["bcachefs", "scrub", target],
            true,
            "run the reviewed bcachefs scrub",
        ),
        Some("filesystems") => command(
            ["btrfs", "scrub", "start", "-B", target],
            true,
            "run the reviewed Btrfs scrub and wait for completion",
        ),
        _ => command_with_readiness(
            ["<scrub-tool>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["scrub tool"],
            "run the storage-domain scrub command",
        ),
    }
}

fn filesystem_trim_command(collection: Option<&str>, target: &str) -> ExecutionCommand {
    match collection {
        Some("filesystems") => command(
            ["fstrim", "-v", target],
            true,
            "trim unused blocks from the mounted filesystem",
        ),
        _ => command_with_readiness(
            ["<trim-tool>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["trim tool"],
            "run the storage-domain trim or discard command",
        ),
    }
}
