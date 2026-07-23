include!("usage_details/detail_keys.rs");

fn usage_details(node: &Node) -> String {
    let details = DETAIL_KEY_GROUPS
        .iter()
        .flat_map(|group| group.iter())
        .filter_map(|(key, label)| {
            property_value(node, key).map(|value| format!("{label}={value}"))
        })
        .collect::<Vec<_>>();

    if details.is_empty() {
        "-".to_string()
    } else {
        details.join(" ")
    }
}
