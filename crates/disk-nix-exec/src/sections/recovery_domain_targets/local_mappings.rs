fn local_mapping_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    match command_step_collection(step) {
        Some("dmMaps") => {
            let target = dm_map_target_from_step(step).or_else(|| {
                step.action_id
                    .split(':')
                    .nth(1)
                    .filter(|target| is_dm_map_target(target))
            });
            vec![
                dmsetup_info_command(target, note),
                dmsetup_deps_command(target),
                dmsetup_table_command(target),
                dmsetup_status_command(target),
                dm_map_inspect_json_command(target, note),
            ]
        }
        Some("loopDevices") => {
            let target = loop_target_from_step(step).or_else(|| {
                step.action_id
                    .split(':')
                    .nth(1)
                    .filter(|target| target.starts_with("/dev/loop"))
            });
            let mut commands = Vec::new();
            if let Some(target) = target {
                commands.push(command(
                    ["losetup", "--json", "--list", target],
                    false,
                    note,
                ));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            } else {
                commands.push(command(
                    ["losetup", "--json", "--list"],
                    false,
                    "inspect loop mappings before retrying",
                ));
            }
            if let Some(backing) = backing_file_from_step(step) {
                commands.push(command(
                    ["stat", "--printf=%n %s %b %B\\n", backing],
                    false,
                    note,
                ));
                commands.push(command(
                    ["disk-nix", "inspect", backing, "--json"],
                    false,
                    note,
                ));
            }
            commands
        }
        Some("backingFiles") => {
            let target = backing_file_from_step(step).or_else(|| {
                step.action_id
                    .split(':')
                    .nth(1)
                    .filter(|target| target.starts_with('/'))
            });
            let mut commands = Vec::new();
            if let Some(target) = target {
                commands.push(command(
                    ["stat", "--printf=%n %s %b %B\\n", target],
                    false,
                    note,
                ));
                commands.push(command(
                    ["du", "--bytes", "--apparent-size", target],
                    false,
                    note,
                ));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            } else {
                commands.push(command(
                    ["disk-nix", "backing-files", "--json"],
                    false,
                    "inspect modeled backing-file inventory before retrying",
                ));
            }
            commands
        }
        _ => Vec::new(),
    }
}

fn local_mapping_target_from_step(step: &ExecutionStep) -> Option<&str> {
    dm_map_target_from_step(step)
        .or_else(|| loop_target_from_step(step))
        .or_else(|| backing_file_from_step(step))
}

fn dm_map_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        let target = match tool {
            "dmsetup" => match command.argv.get(1).map(String::as_str) {
                Some("rename") => command.argv.get(2).map(String::as_str),
                Some("remove" | "deps" | "table" | "status") => {
                    command.argv.get(2).map(String::as_str)
                }
                Some("info") => command.argv.last().map(String::as_str),
                _ => None,
            },
            "disk-nix" if command.argv.get(1).is_some_and(|arg| arg == "inspect") => {
                command.argv.get(2).map(String::as_str)
            }
            _ => None,
        }?;

        Some(target).filter(|target| is_dm_map_target(target))
    })
}

fn loop_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        let target = match tool {
            "losetup" => match command.argv.get(1).map(String::as_str) {
                Some("--detach" | "-c") => command.argv.get(2).map(String::as_str),
                Some("--json") => command.argv.last().map(String::as_str),
                Some(target) if target.starts_with("/dev/loop") => {
                    command.argv.get(1).map(String::as_str)
                }
                _ => None,
            },
            "disk-nix" if command.argv.get(1).is_some_and(|arg| arg == "inspect") => {
                command.argv.get(2).map(String::as_str)
            }
            _ => None,
        }?;

        Some(target).filter(|target| target.starts_with("/dev/loop") && !target.starts_with('<'))
    })
}

fn backing_file_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        let target = match tool {
            "truncate" | "stat" | "du" | "test" => command.argv.last().map(String::as_str),
            "losetup" => command.argv.last().map(String::as_str),
            "disk-nix" if command.argv.get(1).is_some_and(|arg| arg == "inspect") => {
                command.argv.get(2).map(String::as_str)
            }
            _ => None,
        }?;

        Some(target).filter(|target| {
            target.starts_with('/')
                && !target.starts_with('<')
                && !target.starts_with("/dev/loop")
                && !is_dm_map_target(target)
        })
    })
}
