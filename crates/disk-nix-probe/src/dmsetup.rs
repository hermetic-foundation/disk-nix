use disk_nix_model::{Edge, Node, NodeKind, Relationship, StorageGraph};

use crate::ProbeError;

#[derive(Debug, Clone, PartialEq, Eq)]
struct DmDevice {
    name: String,
    uuid: Option<String>,
    major: Option<String>,
    minor: Option<String>,
    open_count: Option<String>,
    segments: Option<String>,
    events: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DmDependency {
    name: String,
    devices: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DmTargetLine {
    name: String,
    start: String,
    length: String,
    target: String,
    payload: Option<String>,
}

pub fn normalize_dmsetup(
    info: &[u8],
    deps: &[u8],
    table: Option<&[u8]>,
    status: Option<&[u8]>,
) -> Result<StorageGraph, ProbeError> {
    let devices = parse_info(info)?;
    let dependencies = parse_deps(deps)?;
    let table = table.map(parse_target_lines).transpose()?;
    let status = status.map(parse_target_lines).transpose()?;
    let mut graph = StorageGraph::empty();

    for device in devices {
        add_device(&mut graph, device);
    }
    for dependency in dependencies {
        add_dependency(&mut graph, dependency);
    }
    if let Some(table) = table {
        add_target_lines(&mut graph, table, "table");
    }
    if let Some(status) = status {
        add_target_lines(&mut graph, status, "status");
    }

    Ok(graph)
}

fn parse_info(bytes: &[u8]) -> Result<Vec<DmDevice>, ProbeError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| ProbeError::Adapter(format!("failed to read dmsetup info: {error}")))?;
    let mut devices = Vec::new();

    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let fields: Vec<&str> = line.split('|').map(str::trim).collect();
        if fields.len() < 7 {
            return Err(ProbeError::Adapter(format!(
                "dmsetup info row has {} fields, expected 7",
                fields.len()
            )));
        }

        devices.push(DmDevice {
            name: fields[0].to_string(),
            uuid: non_empty(fields[1]),
            major: non_empty(fields[2]),
            minor: non_empty(fields[3]),
            open_count: non_empty(fields[4]),
            segments: non_empty(fields[5]),
            events: non_empty(fields[6]),
        });
    }

    Ok(devices)
}

fn parse_deps(bytes: &[u8]) -> Result<Vec<DmDependency>, ProbeError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| ProbeError::Adapter(format!("failed to read dmsetup deps: {error}")))?;
    let mut dependencies = Vec::new();

    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let Some((name, rest)) = line.split_once(':') else {
            continue;
        };
        let devices = parse_dependency_devices(rest);
        dependencies.push(DmDependency {
            name: name.to_string(),
            devices,
        });
    }

    Ok(dependencies)
}

fn parse_target_lines(bytes: &[u8]) -> Result<Vec<DmTargetLine>, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read dmsetup target output: {error}"))
    })?;
    let mut targets = Vec::new();

    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let Some((name, rest)) = line.split_once(':') else {
            continue;
        };
        let mut fields = rest.split_whitespace();
        let Some(start) = fields.next() else {
            continue;
        };
        let Some(length) = fields.next() else {
            continue;
        };
        let Some(target) = fields.next() else {
            continue;
        };
        let payload = fields.collect::<Vec<_>>().join(" ");
        targets.push(DmTargetLine {
            name: name.trim().to_string(),
            start: start.to_string(),
            length: length.to_string(),
            target: target.to_string(),
            payload: (!payload.is_empty()).then_some(payload),
        });
    }

    Ok(targets)
}

fn add_device(graph: &mut StorageGraph, device: DmDevice) {
    let id = dm_id(&device.name);
    let mut node = Node::new(
        id,
        kind_from_uuid(device.uuid.as_deref()),
        device.name.clone(),
    )
    .with_path(format!("/dev/mapper/{}", device.name));

    for (key, value) in [
        ("dm.name", Some(device.name)),
        ("dm.uuid", device.uuid),
        ("dm.major", device.major),
        ("dm.minor", device.minor),
        ("dm.open-count", device.open_count),
        ("dm.segments", device.segments),
        ("dm.events", device.events),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
    }

    graph.add_node(node);
}

fn add_dependency(graph: &mut StorageGraph, dependency: DmDependency) {
    let dm_id = dm_id(&dependency.name);
    graph.add_node(
        Node::new(
            dm_id.clone(),
            NodeKind::DeviceMapper,
            dependency.name.clone(),
        )
        .with_path(format!("/dev/mapper/{}", dependency.name)),
    );

    for device in dependency.devices {
        let path = format!("/dev/{device}");
        let backing_id = format!("block:{path}");
        graph.add_node(
            Node::new(backing_id.clone(), backing_kind(&device), path.clone()).with_path(path),
        );
        graph.add_edge(Edge::new(backing_id, dm_id.clone(), Relationship::Backs));
    }
}

fn add_target_lines(graph: &mut StorageGraph, lines: Vec<DmTargetLine>, namespace: &str) {
    let mut grouped = std::collections::BTreeMap::<String, Vec<DmTargetLine>>::new();
    for line in lines {
        grouped.entry(line.name.clone()).or_default().push(line);
    }

    for (name, lines) in grouped {
        let mut node = Node::new(dm_id(&name), NodeKind::DeviceMapper, name.clone())
            .with_path(format!("/dev/mapper/{name}"))
            .with_property(
                format!("dm.{namespace}.segment-count"),
                lines.len().to_string(),
            );
        let mut targets = Vec::new();
        for (index, line) in lines.iter().enumerate() {
            if !targets.contains(&line.target) {
                targets.push(line.target.clone());
            }
            let prefix = format!("dm.{namespace}.segment.{index}");
            node = node
                .with_property(format!("{prefix}.start"), line.start.clone())
                .with_property(format!("{prefix}.length"), line.length.clone())
                .with_property(format!("{prefix}.target"), line.target.clone());
            if let Some(payload) = &line.payload {
                if line.target == "crypt" && namespace == "table" {
                    for (key, value) in crypt_table_properties(payload) {
                        node = node.with_property(format!("{prefix}.crypt.{key}"), value);
                    }
                } else {
                    node = node.with_property(format!("{prefix}.payload"), payload.clone());
                }
            }
        }
        node = node.with_property(format!("dm.{namespace}.targets"), targets.join(","));
        graph.add_node(node);
    }
}

fn crypt_table_properties(payload: &str) -> Vec<(String, String)> {
    let fields = payload.split_whitespace().collect::<Vec<_>>();
    let mut properties = Vec::new();
    if let Some(cipher) = fields.first() {
        properties.push(("cipher".to_string(), (*cipher).to_string()));
    }
    if let Some(iv_offset) = fields.get(2) {
        properties.push(("iv-offset".to_string(), (*iv_offset).to_string()));
    }
    if let Some(device) = fields.get(3) {
        properties.push(("device".to_string(), (*device).to_string()));
    }
    if let Some(offset) = fields.get(4) {
        properties.push(("offset".to_string(), (*offset).to_string()));
    }
    if fields.len() > 5 {
        properties.push(("options".to_string(), fields[5..].join(" ")));
    }
    properties
}

fn parse_dependency_devices(value: &str) -> Vec<String> {
    value
        .split('(')
        .filter_map(|part| part.split_once(')').map(|(inside, _)| inside.trim()))
        .filter(|inside| !inside.is_empty() && !inside.contains(','))
        .map(ToOwned::to_owned)
        .collect()
}

fn dm_id(name: &str) -> String {
    format!("block:/dev/mapper/{name}")
}

fn kind_from_uuid(uuid: Option<&str>) -> NodeKind {
    match uuid {
        Some(uuid) if uuid.starts_with("CRYPT-") => NodeKind::LuksContainer,
        Some(uuid) if uuid.starts_with("LVM-") => NodeKind::LvmLogicalVolume,
        Some(uuid) if uuid.starts_with("mpath-") => NodeKind::MultipathDevice,
        _ => NodeKind::DeviceMapper,
    }
}

fn backing_kind(device: &str) -> NodeKind {
    if device.starts_with("dm-") {
        NodeKind::DeviceMapper
    } else if (device.starts_with("nvme") && device.contains('p'))
        || (device.starts_with("sd") && device.chars().last().is_some_and(|c| c.is_ascii_digit()))
    {
        NodeKind::Partition
    } else {
        NodeKind::PhysicalDisk
    }
}

fn non_empty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use disk_nix_model::{NodeKind, Relationship};

    use super::*;

    const INFO: &[u8] = br#"
cryptroot|CRYPT-LUKS2-crypt-uuid-cryptroot|253|0|1|1|0
vg-root|LVM-vg-root|253|1|1|2|0
"#;

    const DEPS: &[u8] = br#"
cryptroot: 1 dependencies  : (259, 2) (nvme0n1p2)
vg-root: 1 dependencies  : (253, 0) (dm-0)
"#;

    #[test]
    fn normalizes_dmsetup_info_and_dependencies() {
        let graph = normalize_dmsetup(INFO, DEPS, Some(TABLE), Some(STATUS))
            .expect("fixtures should parse");
        let cryptroot = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/mapper/cryptroot")
            .expect("cryptroot should exist");

        assert_eq!(cryptroot.kind, NodeKind::LuksContainer);
        assert!(
            cryptroot
                .properties
                .iter()
                .any(|property| property.key == "dm.uuid"
                    && property.value == "CRYPT-LUKS2-crypt-uuid-cryptroot")
        );
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "block:/dev/nvme0n1p2"
                && edge.to.0 == "block:/dev/mapper/cryptroot"
                && edge.relationship == Relationship::Backs
        }));
        assert!(
            cryptroot
                .properties
                .iter()
                .any(|property| property.key == "dm.table.targets" && property.value == "crypt")
        );
        assert!(cryptroot.properties.iter().any(|property| {
            property.key == "dm.table.segment.0.crypt.cipher" && property.value == "aes-xts-plain64"
        }));
        assert!(
            !cryptroot
                .properties
                .iter()
                .any(|property| property.value.contains("0123456789abcdef"))
        );

        let root = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/mapper/vg-root")
            .expect("vg-root should exist");
        assert!(root.properties.iter().any(|property| {
            property.key == "dm.status.segment.0.payload" && property.value == "A"
        }));
    }

    #[test]
    fn parses_dependency_device_names() {
        assert_eq!(
            parse_dependency_devices(" 1 dependencies : (8, 2) (sda2)"),
            vec!["sda2"]
        );
    }

    const TABLE: &[u8] = br#"
cryptroot: 0 2097152 crypt aes-xts-plain64 0123456789abcdef 0 259:2 4096
vg-root: 0 1048576 linear 253:0 2048
vg-root: 1048576 1048576 linear 259:3 4096
"#;

    const STATUS: &[u8] = br#"
cryptroot: 0 2097152 crypt 0 2097152
vg-root: 0 2097152 linear A
"#;
}
