fn is_bcache_target(target: &str) -> bool {
    target.starts_with("/dev/bcache")
}

fn bcache_target_path(action: &PlannedAction) -> Option<&str> {
    [
        action.context.target.as_deref(),
        action.context.device.as_deref(),
        action.context.name.as_deref(),
    ]
    .into_iter()
    .flatten()
    .find(|target| is_bcache_target(target))
}

fn lvm_cache_attach_command(target: Option<&str>, cache_pool: Option<&str>) -> ExecutionCommand {
    match (target, cache_pool) {
        (Some(target), Some(cache_pool)) => command(
            [
                "lvconvert",
                "--type",
                "cache",
                "--cachepool",
                cache_pool,
                target,
            ],
            true,
            "attach the reviewed LVM cache pool to the origin logical volume",
        ),
        (target, cache_pool) => command_vec_with_readiness(
            vec![
                "lvconvert".to_string(),
                "--type".to_string(),
                "cache".to_string(),
                "--cachepool".to_string(),
                cache_pool.unwrap_or("<cache-pool>").to_string(),
                target.unwrap_or("<origin-logical-volume>").to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing_lvm_cache_inputs(target, cache_pool),
            "attach LVM cache after selecting an origin LV and cache-pool LV",
        ),
    }
}

fn lvm_cache_replace_command(
    target: Option<&str>,
    old_cache_pool: Option<&str>,
    new_cache_pool: Option<&str>,
) -> ExecutionCommand {
    match (target, new_cache_pool) {
        (Some(target), Some(new_cache_pool)) => command_vec(
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "lvconvert --uncache \"$1\" && lvconvert --type cache --cachepool \"$2\" \"$1\""
                    .to_string(),
                "disk-nix-lvm-cache-replace".to_string(),
                target.to_string(),
                new_cache_pool.to_string(),
            ],
            true,
            "detach the old LVM cache and attach the reviewed replacement cache pool",
        ),
        (target, new_cache_pool) => {
            let mut missing = missing_lvm_cache_inputs(target, new_cache_pool);
            if old_cache_pool.is_none() {
                missing.push("cache pool to replace");
            }
            command_vec_with_readiness(
                vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    "lvconvert --uncache \"$1\" && lvconvert --type cache --cachepool \"$2\" \"$1\""
                        .to_string(),
                    "disk-nix-lvm-cache-replace".to_string(),
                    target.unwrap_or("<origin-logical-volume>").to_string(),
                    new_cache_pool.unwrap_or("<replacement-cache-pool>").to_string(),
                ],
                true,
                CommandReadiness::NeedsDomainImplementation,
                missing,
                "replace LVM cache after selecting origin and replacement cache pool",
            )
        }
    }
}

fn lvm_cache_uncache_command(target: Option<&str>) -> ExecutionCommand {
    match target {
        Some(target) => command(
            ["lvconvert", "--uncache", target],
            true,
            "detach LVM cache from the origin logical volume after dirty data is flushed",
        ),
        None => command_with_readiness(
            ["lvconvert", "--uncache", "<origin-logical-volume>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["target in volume-group/logical-volume form"],
            "detach LVM cache after selecting the origin logical volume",
        ),
    }
}

fn lvm_cache_property_command(
    target: Option<&str>,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            [
                "lvchange",
                "<cache-property>",
                "<value>",
                "<origin-logical-volume>",
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["cache property value"],
            "set LVM cache property after resolving the desired value",
        );
    };
    let Some(flag) = lvm_cache_property_flag(property) else {
        return command_with_readiness(
            [
                "lvchange",
                "<cache-property>",
                value,
                target.unwrap_or("<origin-logical-volume>"),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported LVM cache property"],
            "set LVM cache property after mapping it to an lvchange flag",
        );
    };
    match target {
        Some(target) => command(
            ["lvchange", flag, value, target],
            true,
            "set LVM cache mode or policy on the reviewed origin logical volume",
        ),
        None => command_vec_with_readiness(
            vec![
                "lvchange".to_string(),
                flag.to_string(),
                value.to_string(),
                "<origin-logical-volume>".to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["target in volume-group/logical-volume form"],
            "set LVM cache property after selecting the origin logical volume",
        ),
    }
}

fn missing_lvm_cache_inputs(target: Option<&str>, cache_pool: Option<&str>) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if target.is_none() {
        missing.push("target in volume-group/logical-volume form");
    }
    if cache_pool.is_none() {
        missing.push("cache-pool logical volume");
    }
    missing
}

fn lvm_cache_property_flag(property: &str) -> Option<&'static str> {
    match property {
        "cache-mode" | "cacheMode" | "lvm.cache-mode" | "lvm.cacheMode" => Some("--cachemode"),
        "cache-policy" | "cachePolicy" | "lvm.cache-policy" | "lvm.cachePolicy" => {
            Some("--cachepolicy")
        }
        _ => None,
    }
}

fn bcache_attach_command(target: &str, cache_set: &str) -> ExecutionCommand {
    if !is_bcache_target(target) {
        return command_vec_with_readiness(
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/attach\"".to_string(),
                "disk-nix-bcache-attach".to_string(),
                "<cache-device>".to_string(),
                cache_set.to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["bcache device path"],
            "attach an existing bcache cache-set UUID after selecting the backing bcache device",
        );
    }

    command_vec(
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/attach\"".to_string(),
            "disk-nix-bcache-attach".to_string(),
            target.to_string(),
            cache_set.to_string(),
        ],
        true,
        "attach an existing bcache cache-set UUID to the backing bcache device",
    )
}

fn bcache_detach_command(target: &str) -> ExecutionCommand {
    if !is_bcache_target(target) {
        return command_vec_with_readiness(
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "printf '1\\n' > \"/sys/block/${1#/dev/}/bcache/detach\"".to_string(),
                "disk-nix-bcache-detach".to_string(),
                "<cache-device>".to_string(),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["bcache device path"],
            "detach the bcache cache set after selecting the backing bcache device",
        );
    }

    command_vec(
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "printf '1\\n' > \"/sys/block/${1#/dev/}/bcache/detach\"".to_string(),
            "disk-nix-bcache-detach".to_string(),
            target.to_string(),
        ],
        true,
        "detach the bcache cache set from the backing device after dirty data is flushed",
    )
}

fn bcache_replace_command(
    target: &str,
    from: &str,
    replacement_device: &str,
    cache_set_uuid: Option<&str>,
) -> ExecutionCommand {
    let cache_set_arg = cache_set_uuid.unwrap_or("<new-cache-set-uuid>");
    let mut missing = Vec::new();
    if !is_bcache_target(target) {
        missing.push("bcache device path");
    }
    if cache_set_uuid.is_none() {
        missing.push("new cache-set UUID");
    }

    let argv = vec![
        "sh".to_string(),
        "-c".to_string(),
        "make-bcache -C \"$2\" --cset-uuid \"$3\" --writeback && printf '1\\n' > \"/sys/block/${1#/dev/}/bcache/detach\" && printf '%s\\n' \"$3\" > \"/sys/block/${1#/dev/}/bcache/attach\""
            .to_string(),
        "disk-nix-bcache-replace".to_string(),
        if is_bcache_target(target) {
            target.to_string()
        } else {
            "<cache-device>".to_string()
        },
        replacement_device.to_string(),
        cache_set_arg.to_string(),
    ];

    if missing.is_empty() {
        command_vec(
            argv,
            true,
            &format!(
                "initialize replacement cache device {replacement_device}, detach {from}, and attach cache-set {cache_set_arg} to {target}"
            ),
        )
    } else {
        command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            missing,
            &format!(
                "initialize replacement cache device after flushing and detaching {from} from {target}"
            ),
        )
    }
}

fn bcache_property_command(
    target: &str,
    property: &str,
    assignment: &str,
    cache_set_uuid: Option<&str>,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<cache-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["cache property value"],
            "set a cache property after resolving the desired value",
        );
    };
    if let Some(key) = bcache_cache_set_sysfs_key(property) {
        let cache_set_arg = cache_set_uuid.unwrap_or("<cache-set-uuid>");
        let argv = vec![
            "sh".to_string(),
            "-c".to_string(),
            "printf '%s\\n' \"$2\" > \"/sys/fs/bcache/$1/$3\"".to_string(),
            "disk-nix-bcache-set-property".to_string(),
            cache_set_arg.to_string(),
            value.to_string(),
            key,
        ];
        if cache_set_uuid.is_some() {
            return command_vec(
                argv,
                true,
                "set a bcache cache-set sysfs property on the target cache set",
            );
        }
        return command_vec_with_readiness(
            argv,
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["cache-set UUID"],
            "set a bcache cache-set property after selecting the cache-set UUID",
        );
    }
    if !is_bcache_target(target) {
        return command_vec_with_readiness(
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/$3\"".to_string(),
                "disk-nix-bcache-property".to_string(),
                "<cache-device>".to_string(),
                value.to_string(),
                bcache_sysfs_key(property),
            ],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["bcache device path"],
            "set a bcache sysfs property after selecting the backing bcache device",
        );
    }
    command_vec(
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/$3\"".to_string(),
            "disk-nix-bcache-property".to_string(),
            target.to_string(),
            value.to_string(),
            bcache_sysfs_key(property),
        ],
        true,
        "set a bcache sysfs property on the target cache device",
    )
}

fn bcache_sysfs_read_command(target: &str, key: &str, note: &str) -> ExecutionCommand {
    if !is_bcache_target(target) {
        return command_vec_with_readiness(
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "cat \"/sys/block/${1#/dev/}/bcache/$2\"".to_string(),
                "disk-nix-bcache-read".to_string(),
                "<cache-device>".to_string(),
                key.to_string(),
            ],
            false,
            CommandReadiness::NeedsDomainImplementation,
            ["bcache device path"],
            note,
        );
    }

    command_vec(
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "cat \"/sys/block/${1#/dev/}/bcache/$2\"".to_string(),
            "disk-nix-bcache-read".to_string(),
            target.to_string(),
            key.to_string(),
        ],
        false,
        note,
    )
}

fn bcache_sysfs_key(property: &str) -> String {
    property
        .strip_prefix("bcache.")
        .unwrap_or(property)
        .replace('-', "_")
}

fn bcache_cache_set_sysfs_key(property: &str) -> Option<String> {
    let property = property.trim();
    let normalized = normalize_property_name(property);
    let known = match normalized.as_str() {
        "setaveragekeysize" => Some("average_key_size"),
        "setbtreecachesize" => Some("btree_cache_size"),
        "setcacheavailablepercent" => Some("cache_available_percent"),
        "setcongested" => Some("congested"),
        "setcongestedreadthresholdus" => Some("congested_read_threshold_us"),
        "setcongestedwritethresholdus" => Some("congested_write_threshold_us"),
        "setioerrorhalflife" => Some("io_error_halflife"),
        "setioerrorlimit" => Some("io_error_limit"),
        "setjournaldelayms" => Some("journal_delay_ms"),
        "setrootusagepercent" => Some("root_usage_percent"),
        _ => None,
    };
    if let Some(property) = known {
        return Some(property.to_string());
    }
    let property = property
        .strip_prefix("bcache.set-")
        .or_else(|| property.strip_prefix("bcache.set."))
        .or_else(|| property.strip_prefix("set-"))
        .or_else(|| property.strip_prefix("set_"))?;
    Some(property.replace('-', "_"))
}
