fn current_mount_option_map(node: &Node) -> BTreeMap<String, String> {
    let mut options = property_value_from_node(node, "mount.options")
        .map(parse_mount_option_map)
        .unwrap_or_default();

    for property in &node.properties {
        if let Some(option) = property.key.strip_prefix("nfs.") {
            options
                .entry(normalize_mount_option_name(option))
                .or_insert_with(|| property.value.clone());
        }
    }
    if property_value_from_node(node, "mount.read-only") == Some("true") {
        options
            .entry("ro".to_string())
            .or_insert("true".to_string());
    }
    if property_value_from_node(node, "mount.read-write") == Some("true") {
        options
            .entry("rw".to_string())
            .or_insert("true".to_string());
    }
    if property_value_from_node(node, "mount.bind") == Some("true") {
        options
            .entry("bind".to_string())
            .or_insert("true".to_string());
    }

    options
}

fn current_nfs_export_option_map(node: &Node) -> BTreeMap<String, String> {
    node.properties
        .iter()
        .filter_map(|property| {
            property
                .key
                .strip_prefix("nfs.export-option-")
                .map(|option| (normalize_mount_option_name(option), property.value.clone()))
        })
        .filter(|(option, _)| !option.is_empty())
        .collect()
}

fn option_differences(
    desired_options: &BTreeMap<String, String>,
    current_options: &BTreeMap<String, String>,
) -> Vec<String> {
    desired_options
        .iter()
        .filter_map(|(option, desired)| match current_options.get(option) {
            Some(current) if current == desired => None,
            _ => Some(format!("{option}={desired}")),
        })
        .collect()
}

fn parse_mount_option_map(options: &str) -> BTreeMap<String, String> {
    options
        .split(',')
        .filter_map(|option| {
            let option = option.trim();
            if option.is_empty() {
                return None;
            }
            Some(option.split_once('=').map_or_else(
                || (normalize_mount_option_name(option), "true".to_string()),
                |(key, value)| (normalize_mount_option_name(key), value.trim().to_string()),
            ))
        })
        .filter(|(key, _)| !key.is_empty())
        .collect()
}

fn normalize_mount_option_name(option: &str) -> String {
    option
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|character| match character {
            'a'..='z' | '0'..='9' => character,
            _ => '-',
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
