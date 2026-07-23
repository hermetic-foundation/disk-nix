fn subsystem_paths(subsystem: &Value) -> Vec<&Value> {
    ["Paths", "Controllers", "paths", "controllers"]
        .iter()
        .filter_map(|key| subsystem.get(key).and_then(Value::as_array))
        .flat_map(|values| values.iter())
        .collect()
}

fn subsystem_namespaces(path: &Value) -> Vec<&Value> {
    ["Namespaces", "NameSpaces", "namespaces"]
        .iter()
        .filter_map(|key| path.get(key).and_then(Value::as_array))
        .flat_map(|values| values.iter())
        .collect()
}

fn nvme_subsystem_id(name: &str) -> String {
    format!("nvme-subsystem:{name}")
}

fn nvme_controller_id(controller: &str) -> String {
    format!("nvme-controller:{}", controller.trim_start_matches("/dev/"))
}

fn controller_path(controller: &str) -> String {
    format!("/dev/{}", controller.trim_start_matches("/dev/"))
}

fn field_u64(value: &Value, key: &str) -> Option<u64> {
    let value = value.get(key)?;
    if let Some(number) = value.as_u64() {
        return Some(number);
    }
    let text = value.as_str()?.trim();
    if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).ok()
    } else {
        text.parse().ok()
    }
}

fn field_string(value: &Value, key: &str) -> Option<String> {
    let value = value.get(key)?;
    if let Some(text) = value.as_str() {
        if text.is_empty() {
            None
        } else {
            Some(text.to_string())
        }
    } else if value.is_null() {
        None
    } else {
        Some(value.to_string())
    }
}

fn field_string_any(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| field_string(value, key))
}
