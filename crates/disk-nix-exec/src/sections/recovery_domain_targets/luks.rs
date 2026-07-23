fn luks_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    let mut commands = Vec::new();
    if let Some(device) = luks_device_from_step(step) {
        commands.push(command(["cryptsetup", "luksDump", device], false, note));
        commands.push(command(
            ["disk-nix", "inspect", device, "--json"],
            false,
            note,
        ));
    }
    if let Some(mapper) = luks_mapper_from_step(step) {
        commands.push(command(["cryptsetup", "status", mapper], false, note));
        commands.push(command(
            ["disk-nix", "inspect", mapper, "--json"],
            false,
            note,
        ));
    }
    commands
}

fn luks_mapper_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command.argv.first().is_none_or(|arg| arg != "cryptsetup") {
            return None;
        }
        match command.argv.get(1).map(String::as_str) {
            Some("close" | "resize" | "status") => command.argv.get(2).map(String::as_str),
            Some("open") => command.argv.get(3).map(String::as_str),
            _ => None,
        }
    })
}

fn luks_device_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command.argv.first().is_none_or(|arg| arg != "cryptsetup") {
            return None;
        }
        match command.argv.get(1).map(String::as_str) {
            Some("isLuks" | "luksDump" | "luksFormat" | "luksKillSlot" | "luksUUID") => {
                command.argv.get(2).map(String::as_str)
            }
            Some("open") => command.argv.get(2).map(String::as_str),
            Some("config" | "luksAddKey" | "luksChangeKey") => {
                cryptsetup_positional_arg(command, 0)
            }
            Some("token") => command.argv.last().map(String::as_str),
            _ => None,
        }
        .filter(|target| !target.starts_with('<'))
    })
}

fn cryptsetup_positional_arg(command: &ExecutionCommand, index: usize) -> Option<&str> {
    let mut skip_next = false;
    let mut position = 0;
    for arg in command.argv.iter().skip(2) {
        if skip_next {
            skip_next = false;
            continue;
        }
        if matches!(
            arg.as_str(),
            "--key-file"
                | "--key-slot"
                | "--json-file"
                | "--priority"
                | "--subsystem"
                | "--token-id"
                | "--uuid"
        ) {
            skip_next = true;
            continue;
        }
        if arg.starts_with('-') {
            continue;
        }
        if position == index {
            return Some(arg.as_str());
        }
        position += 1;
    }
    None
}
