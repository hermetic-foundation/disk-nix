fn lv_kind(attributes: Option<&str>) -> NodeKind {
    let Some(attributes) = attributes else {
        return NodeKind::LvmLogicalVolume;
    };

    if attributes.contains('V') || attributes.contains("vdo") {
        NodeKind::VdoVolume
    } else if attributes.starts_with('t') {
        NodeKind::LvmThinPool
    } else if attributes.starts_with('s') || attributes.starts_with('S') {
        NodeKind::LvmSnapshot
    } else if attributes.contains('C') {
        NodeKind::LvmCache
    } else {
        NodeKind::LvmLogicalVolume
    }
}

fn pv_id(name: &str) -> String {
    format!("lvm-pv:{name}")
}

fn vg_id(name: &str) -> String {
    format!("lvm-vg:{name}")
}

fn lv_id(vg_name: &str, lv_name: &str) -> String {
    format!("lvm-lv:{vg_name}/{lv_name}")
}

fn dependency_id(vg_name: &str, dependency: &str) -> String {
    if dependency.starts_with("/dev/") {
        format!("block:{dependency}")
    } else {
        lv_id(vg_name, dependency)
    }
}

fn split_lvm_devices(devices: &str) -> Vec<String> {
    devices
        .split(',')
        .filter_map(|device| {
            let device = device.trim();
            if device.is_empty() {
                return None;
            }
            let name = device
                .split_once('(')
                .map_or(device, |(name, _)| name)
                .trim();
            (!name.is_empty()).then(|| name.to_string())
        })
        .collect()
}

fn parse_lvm_size(value: Option<&str>) -> Option<u64> {
    let value = value?.trim();
    if value.is_empty() {
        return None;
    }

    let numeric_end = value
        .char_indices()
        .find_map(|(index, character)| {
            (!character.is_ascii_digit() && character != '.').then_some(index)
        })
        .unwrap_or(value.len());
    let (number, suffix) = value.split_at(numeric_end);
    let number = number.parse::<f64>().ok()?;
    let multiplier = match suffix.trim().to_ascii_lowercase().as_str() {
        "" | "b" => 1.0,
        "k" | "kb" | "kib" => 1024.0,
        "m" | "mb" | "mib" => 1024.0 * 1024.0,
        "g" | "gb" | "gib" => 1024.0 * 1024.0 * 1024.0,
        "t" | "tb" | "tib" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        "p" | "pb" | "pib" => 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };

    Some((number * multiplier) as u64)
}
