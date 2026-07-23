
fn btrfs_subvolume_property_command(
    target: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<btrfs-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["Btrfs property value"],
            "set a Btrfs subvolume property after resolving the desired value",
        );
    };
    let property_name = match property {
        "ro" | "readonly" | "readOnly" | "btrfs.readonly" | "btrfs.ro" => "ro",
        _ => {
            return command_with_readiness(
                ["<btrfs-property-tool>", target, property],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["supported Btrfs subvolume property"],
                "set a Btrfs subvolume property after selecting a supported property mapping",
            );
        }
    };
    command_vec(
        vec![
            "btrfs".to_string(),
            "property".to_string(),
            "set".to_string(),
            "-ts".to_string(),
            target.to_string(),
            property_name.to_string(),
            normalize_boolish_btrfs_property_value(value),
        ],
        true,
        "set a Btrfs subvolume property",
    )
}

fn btrfs_qgroup_property_command(
    target: &str,
    qgroup_id: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<btrfs-qgroup-tool>", target, qgroup_id],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["Btrfs qgroup limit value"],
            "set a Btrfs qgroup limit after resolving the desired value",
        );
    };
    if target == qgroup_id || target.starts_with("0/") {
        return command_with_readiness(
            ["btrfs", "qgroup", "limit", value, qgroup_id, "<path>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["mounted Btrfs filesystem path"],
            "set a Btrfs qgroup limit after selecting the mounted filesystem path",
        );
    }
    let limit_value = normalize_btrfs_qgroup_limit(value);
    match property {
        "limit" | "maxReferenced" | "max-referenced" | "referenced" | "btrfs.max-referenced" => {
            command_vec(
                vec![
                    "btrfs".to_string(),
                    "qgroup".to_string(),
                    "limit".to_string(),
                    limit_value,
                    qgroup_id.to_string(),
                    target.to_string(),
                ],
                true,
                "set a Btrfs qgroup referenced-byte limit",
            )
        }
        "maxExclusive" | "max-exclusive" | "exclusive" | "btrfs.max-exclusive" => command_vec(
            vec![
                "btrfs".to_string(),
                "qgroup".to_string(),
                "limit".to_string(),
                "-e".to_string(),
                limit_value,
                qgroup_id.to_string(),
                target.to_string(),
            ],
            true,
            "set a Btrfs qgroup exclusive-byte limit",
        ),
        _ => command_with_readiness(
            ["<btrfs-qgroup-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported Btrfs qgroup property"],
            "set a Btrfs qgroup property after selecting a supported property mapping",
        ),
    }
}

fn btrfs_qgroup_target_path<'a>(target: Option<&'a str>, qgroup_id: &str) -> Option<&'a str> {
    let target = target?;
    if target == qgroup_id || target.starts_with("0/") {
        None
    } else {
        Some(target)
    }
}

fn normalize_btrfs_qgroup_limit(value: &str) -> String {
    match value {
        "null" | "none" | "None" | "NONE" | "unlimited" => "none".to_string(),
        other => other.to_string(),
    }
}

fn normalize_boolish_btrfs_property_value(value: &str) -> String {
    match value {
        "1" | "yes" | "on" | "true" => "true".to_string(),
        "0" | "no" | "off" | "false" => "false".to_string(),
        other => other.to_string(),
    }
}
