fn filesystem_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    let target = filesystem_target_from_step(step).or_else(|| {
        step.action_id
            .split(':')
            .nth(1)
            .filter(|target| !target.is_empty())
    });
    let source = filesystem_source_from_step(step);
    let mut commands = Vec::new();

    if let Some(mountpoint) = target.filter(|target| target.starts_with('/')) {
        commands.push(command(
            ["findmnt", "--json", "--target", mountpoint],
            false,
            note,
        ));
        commands.push(command(
            ["disk-nix", "inspect", mountpoint, "--json"],
            false,
            note,
        ));
    }

    if let Some(source) = source {
        commands.push(command(["blkid", source], false, note));
        commands.push(command(
            ["disk-nix", "inspect", source, "--json"],
            false,
            note,
        ));
    }

    if commands.is_empty() {
        commands.push(command(
            ["disk-nix", "filesystems", "--json"],
            false,
            "inspect modeled filesystem inventory before retrying",
        ));
    }

    commands
}

fn filesystem_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        let target = match tool {
            "xfs_growfs" | "fstrim" | "umount" => command.argv.get(1).map(String::as_str),
            "mount" => command.argv.last().map(String::as_str),
            "findmnt" if command.argv.iter().any(|arg| arg == "--target") => {
                command.argv.last().map(String::as_str)
            }
            "btrfs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["filesystem", "resize"]) =>
            {
                command.argv.get(3).map(String::as_str)
            }
            "btrfs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["filesystem", "usage"]) =>
            {
                command.argv.last().map(String::as_str)
            }
            "btrfs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["balance", "start"]) =>
            {
                command.argv.last().map(String::as_str)
            }
            "btrfs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["scrub", "start"]) =>
            {
                command.argv.last().map(String::as_str)
            }
            "zfs" if command.argv.get(1).is_some_and(|arg| arg == "set") => {
                command.argv.last().map(String::as_str)
            }
            _ => None,
        }?;

        Some(target).filter(|target| {
            !target.is_empty() && !target.starts_with('-') && !target.starts_with('<')
        })
    })
}

fn filesystem_source_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        let source = match tool {
            "blkid" => command.argv.get(1).map(String::as_str),
            "mount" if command.argv.len() >= 3 => {
                command.argv.get(command.argv.len() - 2).map(String::as_str)
            }
            "resize2fs" | "resize.f2fs" | "e2fsck" | "xfs_repair" | "ntfsfix" => {
                command.argv.last().map(String::as_str)
            }
            "fsck.fat" | "fsck.exfat" | "fsck.f2fs" => command.argv.last().map(String::as_str),
            "btrfs" if command.argv.get(1).is_some_and(|arg| arg == "check") => {
                command.argv.last().map(String::as_str)
            }
            "bcachefs"
                if command
                    .argv
                    .get(1)
                    .is_some_and(|arg| arg == "fsck" || arg == "format") =>
            {
                command.argv.last().map(String::as_str)
            }
            "bcachefs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["device", "resize"]) =>
            {
                command.argv.get(3).map(String::as_str)
            }
            tool if tool.starts_with("mkfs.") => command.argv.last().map(String::as_str),
            "mkfs" => command.argv.last().map(String::as_str),
            "e2label" | "fatlabel" | "ntfslabel" | "exfatlabel" | "f2fslabel" => {
                command.argv.get(1).map(String::as_str)
            }
            "xfs_admin" => command.argv.last().map(String::as_str),
            _ => None,
        }?;

        Some(source)
            .filter(|source| source.starts_with('/') && !source.starts_with("<") && *source != "/")
    })
}

fn partition_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    let disk = partition_disk_from_step(step).or_else(|| {
        step.action_id
            .split(':')
            .nth(1)
            .filter(|target| target.starts_with('/'))
    });
    let partition = partition_target_from_step(step);
    let mut commands = Vec::new();

    if let Some(disk) = disk {
        commands.push(command(["parted", "-lm", disk], false, note));
        commands.push(command(
            ["lsblk", "--json", "--bytes", "--output-all", disk],
            false,
            note,
        ));
        commands.push(command(
            ["disk-nix", "inspect", disk, "--json"],
            false,
            note,
        ));
    } else {
        commands.push(command(
            ["parted", "-lm"],
            false,
            "inspect all partition tables before retrying",
        ));
        commands.push(command(
            ["lsblk", "--json", "--bytes", "--output-all"],
            false,
            "inspect kernel disk and partition inventory before retrying",
        ));
    }

    if let Some(partition) = partition {
        commands.push(command(
            ["disk-nix", "inspect", partition, "--json"],
            false,
            note,
        ));
    }

    commands
}

fn partition_disk_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        let disk = match tool {
            "parted" => match command.argv.get(1).map(String::as_str) {
                Some("-s") => command.argv.get(2).map(String::as_str),
                Some("-lm") => command.argv.get(2).map(String::as_str),
                _ => command.argv.last().map(String::as_str),
            },
            "partprobe" => command.argv.get(1).map(String::as_str),
            "blockdev" if command.argv.get(1).is_some_and(|arg| arg == "--rereadpt") => {
                command.argv.get(2).map(String::as_str)
            }
            "growpart" => command.argv.get(1).map(String::as_str),
            _ => None,
        }?;

        Some(disk).filter(|disk| {
            disk.starts_with('/') && !disk.starts_with('<') && !disk.starts_with('-')
        })
    })
}

fn partition_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command
            .argv
            .get(0..2)
            .is_some_and(|args| args == ["disk-nix", "inspect"])
        {
            return command
                .argv
                .get(2)
                .map(String::as_str)
                .filter(|target| target.starts_with('/') && !target.starts_with('<'));
        }
        None
    })
}
