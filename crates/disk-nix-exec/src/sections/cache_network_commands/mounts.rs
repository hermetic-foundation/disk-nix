fn export_target_path(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .target
        .as_deref()
        .and_then(path_like_target)
        .or_else(|| action.context.name.as_deref().and_then(path_like_target))
}

fn path_like_target(target: &str) -> Option<&str> {
    target.starts_with('/').then_some(target)
}

fn nfs_mount_target_path(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .mountpoint
        .as_deref()
        .and_then(path_like_target)
        .or_else(|| action.context.target.as_deref().and_then(path_like_target))
        .or_else(|| action.context.name.as_deref().and_then(path_like_target))
}

fn filesystem_mountpoint(action: &PlannedAction) -> Option<&str> {
    action
        .context
        .mountpoint
        .as_deref()
        .and_then(path_like_target)
        .or_else(|| action.context.target.as_deref().and_then(path_like_target))
        .or_else(|| action.context.name.as_deref().and_then(path_like_target))
}

fn filesystem_findmnt_command(mountpoint: Option<&str>) -> ExecutionCommand {
    match mountpoint {
        Some(mountpoint) => command(
            ["findmnt", "--json", mountpoint],
            false,
            "inspect the filesystem mount after selecting the mountpoint",
        ),
        None => command_with_readiness(
            ["findmnt", "--json", "<mountpoint>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["mountpoint path"],
            "inspect the filesystem mount after selecting the mountpoint",
        ),
    }
}

fn filesystem_inspect_command(
    mountpoint: Option<&str>,
    json_output: bool,
    note: &str,
) -> ExecutionCommand {
    match mountpoint {
        Some(mountpoint) => {
            if json_output {
                command(["disk-nix", "inspect", mountpoint, "--json"], false, note)
            } else {
                command(["disk-nix", "inspect", mountpoint], false, note)
            }
        }
        None => {
            let argv = if json_output {
                vec![
                    "disk-nix".to_string(),
                    "inspect".to_string(),
                    "<mountpoint>".to_string(),
                    "--json".to_string(),
                ]
            } else {
                vec![
                    "disk-nix".to_string(),
                    "inspect".to_string(),
                    "<mountpoint>".to_string(),
                ]
            };
            command_vec_with_readiness(
                argv,
                false,
                CommandReadiness::NeedsDomainImplementation,
                ["mountpoint path"],
                note,
            )
        }
    }
}

fn filesystem_remount_command(mountpoint: Option<&str>, options: Option<&str>) -> ExecutionCommand {
    let mountpoint_arg = mountpoint.unwrap_or("<mountpoint>");
    let remount_options = options
        .filter(|options| !options.is_empty())
        .map(|options| format!("remount,{options}"))
        .unwrap_or_else(|| "remount".to_string());

    match mountpoint {
        Some(_) => command_vec(
            vec![
                "mount".to_string(),
                "-o".to_string(),
                remount_options,
                mountpoint_arg.to_string(),
            ],
            true,
            "remount the filesystem path with the reviewed options",
        ),
        None => command_vec_with_readiness(
            vec![
                "mount".to_string(),
                "-o".to_string(),
                remount_options,
                mountpoint_arg.to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["mountpoint path"],
            "remount the filesystem path after selecting the mountpoint",
        ),
    }
}

fn filesystem_mount_command(
    source: Option<&str>,
    mountpoint: Option<&str>,
    fs_type: Option<&str>,
    options: Option<&str>,
) -> ExecutionCommand {
    let source_arg = source.unwrap_or("<device>");
    let mountpoint_arg = mountpoint.unwrap_or("<mountpoint>");
    let fs_type = fs_type.filter(|fs_type| !fs_type.is_empty() && *fs_type != "unknown");
    let options = options.filter(|options| !options.is_empty());
    let mut missing = Vec::new();
    if source.is_none() {
        missing.push("filesystem source device");
    }
    if mountpoint.is_none() {
        missing.push("mountpoint path");
    }

    let mut argv = vec!["mount".to_string()];
    if let Some(fs_type) = fs_type {
        argv.push("-t".to_string());
        argv.push(fs_type.to_string());
    }
    if let Some(options) = options {
        argv.push("-o".to_string());
        argv.push(options.to_string());
    }
    argv.push(source_arg.to_string());
    argv.push(mountpoint_arg.to_string());

    if source.is_some() && mountpoint.is_some() {
        command_vec(
            argv,
            true,
            "mount the reviewed filesystem source at the selected mountpoint",
        )
    } else {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "mount the filesystem after selecting a source device and mountpoint",
        )
    }
}

fn filesystem_unmount_command(mountpoint: Option<&str>) -> ExecutionCommand {
    match mountpoint {
        Some(mountpoint) => command(
            ["umount", mountpoint],
            true,
            "unmount the reviewed filesystem without formatting or deleting data",
        ),
        None => command_with_readiness(
            ["umount", "<mountpoint>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["mountpoint path"],
            "unmount the filesystem after selecting the mountpoint",
        ),
    }
}

fn nfs_mount_create_command(
    source: Option<&str>,
    mountpoint: Option<&str>,
    fs_type: Option<&str>,
    options: Option<&str>,
) -> ExecutionCommand {
    let source_arg = source.unwrap_or("<nfs-source>");
    let mountpoint_arg = mountpoint.unwrap_or("<mountpoint>");
    let fs_type_arg = fs_type.unwrap_or("nfs4");
    let mut missing = Vec::new();
    if source.is_none() {
        missing.push("NFS source");
    }
    if mountpoint.is_none() {
        missing.push("mountpoint path");
    }

    if source.is_some() && mountpoint.is_some() {
        let mut argv = vec![
            "mount".to_string(),
            "-t".to_string(),
            fs_type_arg.to_string(),
        ];
        if let Some(options) = options {
            argv.push("-o".to_string());
            argv.push(options.to_string());
        }
        argv.push(source_arg.to_string());
        argv.push(mountpoint_arg.to_string());
        command_vec(
            argv,
            true,
            "mount the reviewed NFS source at the selected mountpoint",
        )
    } else {
        let mut argv = vec![
            "mount".to_string(),
            "-t".to_string(),
            fs_type_arg.to_string(),
        ];
        if let Some(options) = options {
            argv.push("-o".to_string());
            argv.push(options.to_string());
        }
        argv.push(source_arg.to_string());
        argv.push(mountpoint_arg.to_string());
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "mount the NFS source after selecting a source and mountpoint",
        )
    }
}

fn nfs_mount_findmnt_command(mountpoint: Option<&str>) -> ExecutionCommand {
    match mountpoint {
        Some(mountpoint) => command(
            ["findmnt", "--json", mountpoint],
            false,
            "inspect the NFS mount before unmounting",
        ),
        None => command_with_readiness(
            ["findmnt", "--json", "<mountpoint>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["mountpoint path"],
            "inspect the NFS mount after selecting the mountpoint",
        ),
    }
}

fn nfs_mount_stats_command(mountpoint: Option<&str>) -> ExecutionCommand {
    match mountpoint {
        Some(mountpoint) => command(
            ["nfsstat", "-m", mountpoint],
            false,
            "inspect NFS client mount statistics and negotiated options",
        ),
        None => command_with_readiness(
            ["nfsstat", "-m", "<mountpoint>"],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["mountpoint path"],
            "inspect NFS client mount statistics after selecting the mountpoint",
        ),
    }
}

fn nfs_mount_destroy_command(mountpoint: Option<&str>) -> ExecutionCommand {
    match mountpoint {
        Some(mountpoint) => command(
            ["umount", mountpoint],
            true,
            "unmount the reviewed NFS client mount without touching remote data",
        ),
        None => command_with_readiness(
            ["umount", "<mountpoint>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["mountpoint path"],
            "unmount the NFS client mount after selecting the mountpoint",
        ),
    }
}

fn nfs_mount_remount_command(mountpoint: Option<&str>, options: Option<&str>) -> ExecutionCommand {
    let mountpoint_arg = mountpoint.unwrap_or("<mountpoint>");
    let remount_options = options
        .filter(|options| !options.is_empty())
        .map(|options| format!("remount,{options}"))
        .unwrap_or_else(|| "remount".to_string());

    match mountpoint {
        Some(_) => command_vec(
            vec![
                "mount".to_string(),
                "-o".to_string(),
                remount_options,
                mountpoint_arg.to_string(),
            ],
            true,
            "remount the NFS client path with the reviewed options",
        ),
        None => command_vec_with_readiness(
            vec![
                "mount".to_string(),
                "-o".to_string(),
                remount_options,
                mountpoint_arg.to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["mountpoint path"],
            "remount the NFS client path after selecting the mountpoint",
        ),
    }
}

fn nfs_export_destroy_command(target: Option<&str>, client: Option<&str>) -> ExecutionCommand {
    let target_arg = target.unwrap_or("<export-path>");
    let mut missing = Vec::new();
    if target.is_none() {
        missing.push("NFS export path");
    }
    if client.is_none() {
        missing.push("NFS client selector");
    }

    match (target, client) {
        (Some(_), Some(client)) => command_vec(
            vec![
                "exportfs".to_string(),
                "-u".to_string(),
                format!("{client}:{target_arg}"),
            ],
            true,
            "unexport the reviewed NFS path for the selected client set",
        ),
        _ => {
            let client_arg = client.unwrap_or("<client>");
            command_vec_with_readiness(
                vec![
                    "exportfs".to_string(),
                    "-u".to_string(),
                    format!("{client_arg}:{target_arg}"),
                ],
                true,
                CommandReadiness::NeedsDomainImplementation,
                missing,
                "unexport the path after selecting the client and local export path",
            )
        }
    }
}
