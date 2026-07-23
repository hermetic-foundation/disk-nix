fn snapshot_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    let mut commands = Vec::new();
    if let Some(snapshot) = snapshot_target_from_step(step).or_else(|| {
        step.action_id
            .split(':')
            .nth(1)
            .filter(|target| !target.is_empty())
    }) {
        if is_zfs_snapshot_name(snapshot) {
            commands.push(command(
                ["zfs", "list", "-t", "snapshot", "-H", "-p", snapshot],
                false,
                note,
            ));
            commands.push(command(["zfs", "holds", snapshot], false, note));
            if let Some(dataset) = zfs_snapshot_dataset(snapshot) {
                commands.push(command(["zfs", "list", "-H", "-p", dataset], false, note));
                commands.push(command_vec(
                    [
                        "zfs",
                        "list",
                        "-t",
                        "snapshot",
                        "-H",
                        "-p",
                        "-o",
                        "name,creation,used,referenced,userrefs",
                        "-r",
                        dataset,
                    ],
                    false,
                    note,
                ));
            }
            commands.push(command(
                ["disk-nix", "inspect", snapshot, "--json"],
                false,
                note,
            ));
        } else if snapshot.starts_with('/') {
            commands.push(command(
                ["btrfs", "subvolume", "show", snapshot],
                false,
                note,
            ));
            commands.push(command(
                ["btrfs", "property", "get", "-ts", snapshot, "ro"],
                false,
                note,
            ));
            commands.push(command(
                ["disk-nix", "inspect", snapshot, "--json"],
                false,
                note,
            ));
        } else {
            commands.push(command(
                ["disk-nix", "inspect", snapshot, "--json"],
                false,
                note,
            ));
        }
    }
    commands
}

fn snapshot_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        match tool {
            "zfs" => match command.argv.get(1).map(String::as_str) {
                Some("snapshot" | "destroy" | "rollback" | "holds") => {
                    command.argv.last().map(String::as_str)
                }
                Some("clone" | "rename") => command.argv.get(2).map(String::as_str),
                Some("hold" | "release") => command.argv.last().map(String::as_str),
                Some("list")
                    if command
                        .argv
                        .iter()
                        .any(|arg| arg == "-t" || arg == "snapshot") =>
                {
                    command.argv.last().map(String::as_str)
                }
                _ => None,
            },
            "btrfs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["subvolume", "show"]) =>
            {
                command.argv.last().map(String::as_str)
            }
            "btrfs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["subvolume", "delete"]) =>
            {
                command.argv.last().map(String::as_str)
            }
            "btrfs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["subvolume", "snapshot"]) =>
            {
                command
                    .argv
                    .iter()
                    .skip(3)
                    .find(|arg| !arg.starts_with('-'))
                    .map(String::as_str)
            }
            "btrfs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["property", "get"]) =>
            {
                command
                    .argv
                    .iter()
                    .skip(3)
                    .find(|arg| arg.starts_with('/'))
                    .map(String::as_str)
            }
            "mv" => command.argv.iter().skip(1).find_map(|arg| {
                if arg == "--" || arg.starts_with('-') {
                    None
                } else {
                    Some(arg.as_str())
                }
            }),
            _ => None,
        }
        .filter(|target| !target.starts_with('<'))
    })
}

fn btrfs_subvolume_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command
            .argv
            .get(0..3)
            .is_some_and(|args| args == ["btrfs", "subvolume", "show"])
            || command
                .argv
                .get(0..3)
                .is_some_and(|args| args == ["btrfs", "subvolume", "create"])
            || command
                .argv
                .get(0..3)
                .is_some_and(|args| args == ["btrfs", "subvolume", "delete"])
        {
            return command.argv.get(3).map(String::as_str);
        }
        if command
            .argv
            .get(0..4)
            .is_some_and(|args| args == ["btrfs", "property", "set", "-ts"])
            || command
                .argv
                .get(0..4)
                .is_some_and(|args| args == ["btrfs", "property", "get", "-ts"])
        {
            return command.argv.get(4).map(String::as_str);
        }
        if command
            .argv
            .get(0..2)
            .is_some_and(|args| args == ["mv", "--"])
        {
            return command.argv.get(2).map(String::as_str);
        }
        None
    })
}

fn btrfs_qgroup_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command
            .argv
            .get(0..5)
            .is_some_and(|args| args == ["btrfs", "qgroup", "show", "--raw", "-reF"])
        {
            return command.argv.get(5).map(String::as_str);
        }
        if command
            .argv
            .get(0..3)
            .is_some_and(|args| args == ["btrfs", "qgroup", "create"])
            || command
                .argv
                .get(0..3)
                .is_some_and(|args| args == ["btrfs", "qgroup", "destroy"])
        {
            return command.argv.get(4).map(String::as_str);
        }
        if command
            .argv
            .get(0..3)
            .is_some_and(|args| args == ["btrfs", "qgroup", "limit"])
        {
            return if command.argv.get(3).is_some_and(|arg| arg == "-e") {
                command.argv.get(6).map(String::as_str)
            } else {
                command.argv.get(5).map(String::as_str)
            };
        }
        None
    })
}

fn iscsi_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command.argv.first().is_some_and(|arg| arg == "iscsiadm") {
            return command
                .argv
                .windows(2)
                .find(|window| window[0] == "--targetname")
                .map(|window| window[1].as_str());
        }
        None
    })
}

fn nvme_controller_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command.argv.first().is_some_and(|arg| arg == "nvme")
            && command.argv.get(1).is_some_and(|arg| {
                matches!(
                    arg.as_str(),
                    "attach-ns" | "create-ns" | "delete-ns" | "detach-ns" | "list-ns" | "ns-rescan"
                )
            })
        {
            return command
                .argv
                .get(2)
                .map(String::as_str)
                .filter(|target| is_nvme_controller_path(target));
        }
        None
    })
}

fn target_lun_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.action_id
        .strip_prefix("targetluns:")
        .or_else(|| step.action_id.strip_prefix("targetLuns:"))
        .and_then(|rest| {
            rest.split_once(":set-property:")
                .map(|(target, _)| target)
                .or_else(|| rest.rsplit_once(':').map(|(target, _)| target))
        })
        .filter(|target| !target.is_empty())
}

fn failed_result_notes(result: &ExecutionCommandResult) -> Vec<String> {
    let mut notes = vec![
        format!(
            "{:?} phase failed for action {}",
            result.phase, result.action_id
        ),
        format!("command: {}", result.argv.join(" ")),
    ];
    if let Some(status_code) = result.status_code {
        notes.push(format!("exit status: {status_code}"));
    }
    if !result.stderr.trim().is_empty() {
        notes.push(format!("stderr: {}", result.stderr.trim()));
    }
    notes
}
