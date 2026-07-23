fn luks_node_is_active(node: &Node) -> Option<bool> {
    property_value_from_node(node, "cryptsetup.active").map(|value| value == "true")
}

fn is_lvm_activation_collection(action: &PlannedAction) -> bool {
    matches!(
        action.context.collection.as_deref(),
        Some("volumes" | "thinPools" | "lvmSnapshots")
    )
}

fn is_mount_collection(action: &PlannedAction) -> bool {
    matches!(
        action.context.collection.as_deref(),
        Some("filesystems" | "nfs.mounts")
    )
}

fn lvm_node_is_active(node: &Node) -> Option<bool> {
    property_value_from_node(node, "lvm.active").map(|value| {
        value
            .split_whitespace()
            .next()
            .is_some_and(|state| state.eq_ignore_ascii_case("active"))
    })
}

fn lvm_vg_is_exported(node: &Node) -> bool {
    property_value_from_node(node, "lvm.vg-exported").is_some_and(|value| {
        let normalized = value.trim();
        normalized.eq_ignore_ascii_case("exported")
            || normalized.eq_ignore_ascii_case("true")
            || normalized.eq_ignore_ascii_case("yes")
            || normalized == "1"
    })
}

fn lvm_pv_review_reasons(node: &Node) -> Vec<String> {
    [
        ("lvm.pv-missing", "PV is marked missing"),
        ("lvm.pv-duplicate", "PV is marked duplicate"),
    ]
    .iter()
    .filter_map(|(property, reason)| {
        property_value_from_node(node, property)
            .filter(|value| lvm_truthy_or_named_state(value, reason))
            .map(|value| format!("{reason} ({property}={value})"))
    })
    .collect()
}

fn lvm_vg_review_reasons(node: &Node) -> Vec<String> {
    let mut reasons = Vec::new();

    if let Some(value) = property_value_from_node(node, "lvm.vg-exported")
        .filter(|value| lvm_truthy_or_named_state(value, "VG is marked exported"))
    {
        reasons.push(format!("VG is marked exported (lvm.vg-exported={value})"));
    }

    if let Some(value) = property_value_from_node(node, "lvm.vg-partial")
        .filter(|value| lvm_truthy_or_named_state(value, "VG is marked partial"))
    {
        reasons.push(format!("VG is marked partial (lvm.vg-partial={value})"));
    }

    if let Some(count) = property_value_from_node(node, "lvm.missing-pv-count")
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|count| *count > 0)
    {
        reasons.push(format!("VG reports {count} missing physical volume(s)"));
    }

    reasons
}

fn lvm_truthy_or_named_state(value: &str, reason: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    let named_state = reason
        .trim_start_matches("PV is marked ")
        .trim_start_matches("VG is marked ")
        .to_ascii_lowercase();
    normalized == "1" || normalized == "true" || normalized == "yes" || normalized == named_state
}

fn md_state_indicates_active(value: &str) -> bool {
    value
        .split(|character: char| !character.is_ascii_alphanumeric())
        .any(|token| matches!(token.to_ascii_lowercase().as_str(), "clean" | "active"))
}

fn md_device_count_property(node: &Node, key: &str) -> Option<u64> {
    property_value_from_node(node, key).and_then(|value| value.trim().parse().ok())
}

fn zfs_status_is_online(value: &str) -> bool {
    value.trim().eq_ignore_ascii_case("online")
}

fn vdo_operating_mode_is_stopped(value: &str) -> bool {
    let normalized = value
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_whitespace() || character == '_' {
                '-'
            } else {
                character
            }
        })
        .collect::<String>()
        .to_ascii_lowercase();
    matches!(normalized.as_str(), "stopped" | "not-running" | "inactive")
}

fn property_value_from_node<'a>(node: &'a Node, key: &str) -> Option<&'a str> {
    node.properties
        .iter()
        .find(|property| property.key == key)
        .map(|property| property.value.as_str())
}
