fn lvm_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    let mut commands = Vec::new();
    match command_step_collection(step) {
        Some("physicalVolumes") => {
            if let Some(target) = lvm_target_from_step(step).or_else(|| {
                step.action_id
                    .split(':')
                    .nth(1)
                    .filter(|target| !target.is_empty())
            }) {
                commands.push(command(
                    ["pvs", "--reportformat", "json", target],
                    false,
                    note,
                ));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            }
            commands.push(command(["vgs", "--reportformat", "json"], false, note));
            commands.push(command(["lvs", "--reportformat", "json"], false, note));
        }
        Some("volumes" | "thinPools") => {
            if let Some(target) = lvm_target_from_step(step).or_else(|| {
                step.action_id
                    .split(':')
                    .nth(1)
                    .filter(|target| !target.is_empty())
            }) {
                commands.push(command(
                    ["lvs", "--reportformat", "json", target],
                    false,
                    note,
                ));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            }
            commands.push(command(["vgs", "--reportformat", "json"], false, note));
            commands.push(command(["pvs", "--reportformat", "json"], false, note));
        }
        Some("volumeGroups") => {
            if let Some(target) = lvm_target_from_step(step).or_else(|| {
                step.action_id
                    .split(':')
                    .nth(1)
                    .filter(|target| !target.is_empty())
            }) {
                commands.push(command(
                    ["vgs", "--reportformat", "json", target],
                    false,
                    note,
                ));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            }
            commands.push(command(["pvs", "--reportformat", "json"], false, note));
            commands.push(command(
                ["lvs", "--reportformat", "json", "-a"],
                false,
                note,
            ));
        }
        _ => {}
    }
    commands
}

fn lvm_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        match tool {
            "lvs" | "pvs" | "vgs" if command.argv.len() > 3 => {
                command.argv.last().map(String::as_str)
            }
            "lvchange" | "lvextend" | "lvremove" | "lvreduce" => {
                command.argv.last().map(String::as_str)
            }
            "lvrename" => command.argv.get(1).map(String::as_str),
            "pvcreate" | "pvremove" | "pvresize" => command.argv.last().map(String::as_str),
            "pvscan" if command.argv.len() > 2 => command.argv.last().map(String::as_str),
            "vgchange" | "vgexport" | "vgimport" | "vgremove" => {
                command.argv.last().map(String::as_str)
            }
            "vgcreate" | "vgextend" | "vgreduce" | "vgrename" => {
                command.argv.get(1).map(String::as_str)
            }
            _ => None,
        }
        .filter(|target| !target.starts_with('<'))
    })
}

fn cache_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    let mut commands = Vec::new();
    let target = cache_target_from_step(step).or_else(|| {
        step.action_id
            .split(':')
            .nth(1)
            .filter(|target| !target.is_empty())
    });

    match command_step_collection(step) {
        Some("lvmCaches") => {
            if let Some(target) = target {
                commands.push(command(
                    [
                        "lvs",
                        "--reportformat",
                        "json",
                        "-a",
                        "-o",
                        "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent",
                        target,
                    ],
                    false,
                    note,
                ));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            }
            commands.push(command(["vgs", "--reportformat", "json"], false, note));
            commands.push(command(["pvs", "--reportformat", "json"], false, note));
        }
        Some("caches") => {
            if let Some(target) = target {
                commands.push(bcache_sysfs_read_command(target, "state", note));
                commands.push(bcache_sysfs_read_command(target, "cache_mode", note));
                commands.push(bcache_sysfs_read_command(target, "dirty_data", note));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            }
            commands.push(command(["disk-nix", "cache", "--json"], false, note));
        }
        _ => {}
    }
    commands
}

fn cache_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        match tool {
            "sh" => command
                .argv
                .get(3)
                .filter(|wrapper| wrapper.starts_with("disk-nix-bcache-"))
                .and_then(|_| command.argv.get(4))
                .map(String::as_str),
            "lvchange" | "lvconvert" => command.argv.last().map(String::as_str),
            "lvs" if command.argv.len() > 3 => command.argv.last().map(String::as_str),
            _ => None,
        }
        .filter(|target| !target.starts_with('<'))
    })
}

fn swap_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    let mut commands = vec![command(
        ["swapon", "--show", "--bytes", "--raw"],
        false,
        note,
    )];
    if let Some(target) = swap_target_from_step(step).or_else(|| {
        step.action_id
            .split(':')
            .nth(1)
            .filter(|target| target.starts_with('/'))
    }) {
        commands.push(command(["blkid", target], false, note));
        commands.push(command(
            ["disk-nix", "inspect", target, "--json"],
            false,
            note,
        ));
    } else {
        commands.push(command(
            ["disk-nix", "swap", "--json"],
            false,
            "inspect modeled swap inventory before retrying",
        ));
    }
    commands
}

fn swap_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        match tool {
            "fallocate" | "mkswap" | "swaplabel" | "swapoff" | "wipefs" => {
                command.argv.last().map(String::as_str)
            }
            "swapon" if command.argv.get(1).is_none_or(|arg| arg != "--show") => {
                command.argv.last().map(String::as_str)
            }
            "sh" if command.argv.get(1).is_some_and(|arg| arg == "-c") => {
                swap_target_from_shell(command.argv.get(2)?)
            }
            _ => None,
        }
        .filter(|target| target.starts_with('/') && !target.starts_with("<"))
    })
}

fn swap_target_from_shell(script: &str) -> Option<&str> {
    let target = script.strip_prefix("swapoff ")?.split_whitespace().next()?;
    target
        .trim_matches('\'')
        .trim_matches('"')
        .strip_prefix("\\")
        .or(Some(target.trim_matches('\'').trim_matches('"')))
}
