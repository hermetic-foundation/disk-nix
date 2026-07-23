fn nfs_export_create_command(
    target: Option<&str>,
    client: Option<&str>,
    options: Option<&str>,
) -> ExecutionCommand {
    let target_arg = target.unwrap_or("<export-path>");
    let mut missing = Vec::new();
    if target.is_none() {
        missing.push("NFS export path");
    }
    if client.is_none() {
        missing.push("NFS client selector");
    }
    if options.is_none() {
        missing.push("NFS export options");
    }

    match (target, client, options) {
        (Some(_), Some(client), Some(options)) => command_vec(
            vec![
                "exportfs".to_string(),
                "-i".to_string(),
                "-o".to_string(),
                options.to_string(),
                format!("{client}:{target_arg}"),
            ],
            true,
            "export an existing path to the selected NFS client set with reviewed options",
        ),
        _ => {
            let client_arg = client.unwrap_or("<client>");
            let options_arg = options.unwrap_or("<options>");
            command_vec_with_readiness(
                vec![
                    "exportfs".to_string(),
                    "-i".to_string(),
                    "-o".to_string(),
                    options_arg.to_string(),
                    format!("{client_arg}:{target_arg}"),
                ],
                true,
                CommandReadiness::NeedsDomainImplementation,
                missing,
                "export the path after selecting clients, options, and a local export path",
            )
        }
    }
}

fn nfs_export_property_command(
    target: &str,
    client: Option<&str>,
    property: &str,
    property_value: Option<&str>,
    existing_options: Option<&str>,
) -> ExecutionCommand {
    match property {
        "options" | "nfs.options" | "exportOptions" | "export-options" => {
            nfs_export_create_command(
                path_like_target(target),
                client,
                property_value.or(existing_options),
            )
        }
        _ => command_with_readiness(
            ["exportfs", "-ra"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported NFS export property"],
            "reload NFS exports after selecting a supported export property mapping",
        ),
    }
}

fn luks_device_property_command(
    device: Option<&str>,
    property: &str,
    value: Option<&str>,
) -> ExecutionCommand {
    let device_arg = device.unwrap_or("<luks-device>");
    let mut missing = Vec::new();
    if device.is_none() {
        missing.push("LUKS backing device");
    }
    if value.is_none() {
        missing.push("LUKS property value");
    }

    let Some(value) = value else {
        return command_vec_with_readiness(
            luks_device_property_argv(device_arg, property, "<value>"),
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "update LUKS header identity after selecting a property value",
        );
    };

    let argv = luks_device_property_argv(device_arg, property, value);
    if !missing.is_empty() {
        return command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            "update LUKS header identity after selecting the backing device",
        );
    }

    if luks_device_property_argv_is_supported(property) {
        command_vec(argv, true, "update LUKS header identity metadata")
    } else {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            vec!["supported LUKS header property"],
            "update LUKS header identity after selecting a supported property mapping",
        )
    }
}

fn luks_device_property_argv(device: &str, property: &str, value: &str) -> Vec<String> {
    match property {
        "label" | "luks.label" | "cryptsetup.label" => vec![
            "cryptsetup".to_string(),
            "config".to_string(),
            device.to_string(),
            "--label".to_string(),
            value.to_string(),
        ],
        "subsystem" | "luks.subsystem" | "cryptsetup.subsystem" => vec![
            "cryptsetup".to_string(),
            "config".to_string(),
            device.to_string(),
            "--subsystem".to_string(),
            value.to_string(),
        ],
        "uuid" | "luks.uuid" | "cryptsetup.uuid" => vec![
            "cryptsetup".to_string(),
            "luksUUID".to_string(),
            device.to_string(),
            "--uuid".to_string(),
            value.to_string(),
        ],
        _ => vec![
            "<luks-property-tool>".to_string(),
            device.to_string(),
            property.to_string(),
            value.to_string(),
        ],
    }
}

fn luks_device_property_argv_is_supported(property: &str) -> bool {
    matches!(
        property,
        "label"
            | "luks.label"
            | "cryptsetup.label"
            | "subsystem"
            | "luks.subsystem"
            | "cryptsetup.subsystem"
            | "uuid"
            | "luks.uuid"
            | "cryptsetup.uuid"
    )
}
