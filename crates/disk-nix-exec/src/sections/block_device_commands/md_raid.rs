fn missing_md_raid_create_inputs(
    missing_target: bool,
    missing_level: bool,
    missing_devices: bool,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if missing_target {
        missing.push("MD array path");
    }
    if missing_level {
        missing.push("RAID level");
    }
    if missing_devices {
        missing.push("member devices");
    }
    missing
}

fn md_raid_grow_command(target: Option<&str>, desired_size: Option<&str>) -> ExecutionCommand {
    let target_arg = target.unwrap_or("<md-array>");
    match (target, desired_size) {
        (Some(_), Some(size)) => command_vec(
            vec!["mdadm", "--grow", target_arg, "--size", size],
            true,
            "grow or reshape the MD RAID array to the desired component size",
        ),
        (Some(_), None) => command_with_readiness(
            ["mdadm", "--grow", target_arg, "--size", "<size-or-max>"],
            true,
            CommandReadiness::NeedsDesiredSize,
            ["desired MD RAID component size or max"],
            "grow or reshape the MD RAID array after selecting the desired size",
        ),
        (None, desired_size) => command_vec_with_readiness(
            vec![
                "mdadm",
                "--grow",
                target_arg,
                "--size",
                desired_size.unwrap_or("<size-or-max>"),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_md_raid_grow_inputs(target, desired_size),
            "grow or reshape the MD RAID array after selecting the array and desired size",
        ),
    }
}

fn missing_md_raid_grow_inputs(
    target: Option<&str>,
    desired_size: Option<&str>,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if target.is_none() {
        missing.push("MD array path");
    }
    if desired_size.is_none() {
        missing.push("desired MD RAID component size or max");
    }
    missing
}

fn property_assignment(action: &PlannedAction) -> String {
    let key = action.context.property.as_deref().unwrap_or("<key>");
    let value = action
        .context
        .property_value
        .as_deref()
        .unwrap_or("<value>");
    format!("{key}={value}")
}
