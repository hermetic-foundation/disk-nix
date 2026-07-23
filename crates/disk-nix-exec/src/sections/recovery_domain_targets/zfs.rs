fn zfs_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    let target = zfs_target_from_step(step).or_else(|| {
        step.action_id
            .split(':')
            .nth(1)
            .filter(|target| !target.is_empty())
    });
    let mut commands = Vec::new();

    match command_step_collection(step) {
        Some("pools") => {
            if let Some(target) = target {
                commands.push(command(["zpool", "status", "-P", target], false, note));
                commands.push(command(["zpool", "list", "-H", "-p", target], false, note));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            } else {
                commands.push(command(["zpool", "status", "-P"], false, note));
                commands.push(command(["zpool", "list", "-H", "-p"], false, note));
            }
        }
        Some("datasets") => {
            if let Some(target) = target {
                commands.push(command(
                    ["zfs", "list", "-H", "-p", "-t", "filesystem", target],
                    false,
                    note,
                ));
                commands.push(command(["zfs", "get", "all", target], false, note));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            } else {
                commands.push(command(
                    ["zfs", "list", "-H", "-p", "-t", "filesystem"],
                    false,
                    note,
                ));
            }
        }
        Some("zvols") => {
            if let Some(target) = target {
                commands.push(command(
                    ["zfs", "list", "-H", "-p", "-t", "volume", target],
                    false,
                    note,
                ));
                commands.push(command(["zfs", "get", "all", target], false, note));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            } else {
                commands.push(command(
                    ["zfs", "list", "-H", "-p", "-t", "volume"],
                    false,
                    note,
                ));
            }
        }
        _ => {}
    }

    commands
}

fn zfs_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        let target = match tool {
            "zpool" => match command.argv.get(1).map(String::as_str) {
                Some("add" | "create" | "destroy" | "export" | "remove" | "replace" | "scrub") => {
                    command.argv.get(2).map(String::as_str)
                }
                Some("import") => command.argv.last().map(String::as_str),
                Some("set") => command.argv.get(3).map(String::as_str),
                Some("list" | "status" | "get") if command.argv.len() > 3 => {
                    command.argv.last().map(String::as_str)
                }
                _ => None,
            },
            "zfs" => match command.argv.get(1).map(String::as_str) {
                Some("create" | "destroy" | "get" | "promote" | "set") => {
                    command.argv.last().map(String::as_str)
                }
                Some("rename") => command.argv.get(2).map(String::as_str),
                Some("list") if command.argv.len() > 4 => command.argv.last().map(String::as_str),
                _ => None,
            },
            _ => None,
        }?;

        Some(target).filter(|target| {
            !target.is_empty()
                && !target.starts_with('-')
                && !target.starts_with('<')
                && *target != "import"
        })
    })
}
