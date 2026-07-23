fn nfs_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    match command_step_collection(step) {
        Some("exports") => {
            let target = nfs_export_target_from_step(step).or_else(|| {
                step.action_id
                    .split(':')
                    .nth(1)
                    .filter(|target| target.starts_with('/'))
            });
            let mut commands = vec![command(["exportfs", "-v"], false, note)];
            if let Some(target) = target {
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            } else {
                commands.push(command(
                    ["disk-nix", "nfs", "--json"],
                    false,
                    "inspect modeled NFS exports before retrying",
                ));
            }
            commands
        }
        Some("nfs.mounts") => {
            let mountpoint = nfs_mount_target_from_step(step).or_else(|| {
                step.action_id
                    .split(':')
                    .nth(1)
                    .filter(|target| target.starts_with('/'))
            });
            let mut commands = Vec::new();
            if let Some(mountpoint) = mountpoint {
                commands.push(command(["findmnt", "--json", mountpoint], false, note));
                commands.push(command(["nfsstat", "-m", mountpoint], false, note));
                commands.push(command(
                    ["disk-nix", "inspect", mountpoint, "--json"],
                    false,
                    note,
                ));
            } else {
                commands.push(command(
                    ["findmnt", "--json", "--types", "nfs,nfs4"],
                    false,
                    "inspect active NFS mounts before retrying",
                ));
                commands.push(command(
                    ["disk-nix", "nfs", "--json"],
                    false,
                    "inspect modeled NFS mounts before retrying",
                ));
            }
            commands
        }
        _ => Vec::new(),
    }
}

fn nfs_target_from_step(step: &ExecutionStep) -> Option<&str> {
    nfs_export_target_from_step(step).or_else(|| nfs_mount_target_from_step(step))
}

fn nfs_export_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command.argv.first().is_none_or(|arg| arg != "exportfs") {
            return None;
        }
        command
            .argv
            .last()
            .and_then(|target| {
                target
                    .split_once(':')
                    .map(|(_, path)| path)
                    .or(Some(target))
            })
            .filter(|target| target.starts_with('/') && !target.starts_with('<'))
    })
}

fn nfs_mount_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        let target = match tool {
            "mount" | "umount" | "findmnt" | "nfsstat" => command.argv.last().map(String::as_str),
            "disk-nix" if command.argv.get(1).is_some_and(|arg| arg == "inspect") => {
                command.argv.get(2).map(String::as_str)
            }
            _ => None,
        }?;

        Some(target).filter(|target| target.starts_with('/') && !target.starts_with('<'))
    })
}
