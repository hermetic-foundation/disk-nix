use disk_nix_model::{Edge, Identity, Node, NodeKind, Relationship, StorageGraph, Usage};

use crate::ProbeError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdArrayReport {
    pub name: String,
    pub detail: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MdScanArray {
    name: String,
    metadata: Option<String>,
    uuid: Option<String>,
    name_property: Option<String>,
    spares: Option<String>,
    devices: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MdArray {
    name: String,
    uuid: Option<String>,
    version: Option<String>,
    level: Option<String>,
    state: Option<String>,
    size_bytes: Option<u64>,
    used_devices: Option<String>,
    total_devices: Option<String>,
    array_devices: Option<String>,
    active_devices: Option<String>,
    working_devices: Option<String>,
    failed_devices: Option<String>,
    spare_devices: Option<String>,
    degraded_devices: Option<String>,
    preferred_minor: Option<String>,
    name_property: Option<String>,
    creation_time: Option<String>,
    update_time: Option<String>,
    events: Option<String>,
    chunk_size: Option<String>,
    layout: Option<String>,
    consistency_policy: Option<String>,
    rebuild_status: Option<String>,
    reshape_status: Option<String>,
    resync_status: Option<String>,
    check_status: Option<String>,
    intent_bitmap: Option<String>,
    persistence: Option<String>,
    bitmap: Option<String>,
    members: Vec<MdMember>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MdMember {
    path: String,
    number: Option<String>,
    major: Option<String>,
    minor: Option<String>,
    raid_device: Option<String>,
    state: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MdStatArray {
    name: String,
    state: Option<String>,
    level: Option<String>,
    members: Vec<MdStatMember>,
    blocks: Option<String>,
    superblock: Option<String>,
    layout: Option<String>,
    chunk_size: Option<String>,
    device_count: Option<String>,
    health: Option<String>,
    progress: Option<String>,
    progress_percent: Option<String>,
    progress_blocks: Option<String>,
    finish: Option<String>,
    speed: Option<String>,
    bitmap: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MdStatMember {
    path: String,
    slot: Option<String>,
    flags: Option<String>,
}

pub fn arrays_from_scan(bytes: &[u8]) -> Result<Vec<String>, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read mdadm scan output: {error}"))
    })?;
    Ok(text.lines().filter_map(array_name_from_scan_line).collect())
}

pub fn normalize_mdstat(bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let arrays = parse_mdstat(bytes)?;
    let mut graph = StorageGraph::empty();

    for array in arrays {
        add_mdstat_array(&mut graph, array);
    }

    Ok(graph)
}

pub fn normalize_md_scan(bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let arrays = parse_scan(bytes)?;
    let mut graph = StorageGraph::empty();

    for array in arrays {
        add_scan_array(&mut graph, array);
    }

    Ok(graph)
}

pub fn normalize_md_arrays(reports: &[MdArrayReport]) -> Result<StorageGraph, ProbeError> {
    let mut graph = StorageGraph::empty();

    for report in reports {
        let array = parse_detail(&report.name, &report.detail)?;
        add_array(&mut graph, array);
    }

    Ok(graph)
}

fn array_name_from_scan_line(line: &str) -> Option<String> {
    let mut parts = line.split_whitespace();
    if parts.next()? == "ARRAY" {
        parts.next().map(ToOwned::to_owned)
    } else {
        None
    }
}

fn parse_scan(bytes: &[u8]) -> Result<Vec<MdScanArray>, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read mdadm scan output: {error}"))
    })?;
    Ok(text.lines().filter_map(parse_scan_line).collect())
}

fn parse_scan_line(line: &str) -> Option<MdScanArray> {
    let mut parts = line.split_whitespace();
    (parts.next()? == "ARRAY").then_some(())?;
    let name = parts.next()?.to_string();
    let mut array = MdScanArray {
        name,
        metadata: None,
        uuid: None,
        name_property: None,
        spares: None,
        devices: None,
    };

    for part in parts {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        let value = value.trim_matches('"').to_string();
        match key {
            "metadata" => array.metadata = Some(value),
            "UUID" | "uuid" => array.uuid = Some(value),
            "name" => array.name_property = Some(value),
            "spares" => array.spares = Some(value),
            "devices" => array.devices = Some(value),
            _ => {}
        }
    }

    Some(array)
}

fn parse_mdstat(bytes: &[u8]) -> Result<Vec<MdStatArray>, ProbeError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| ProbeError::Adapter(format!("failed to read mdstat output: {error}")))?;
    let mut arrays = Vec::new();
    let mut current: Option<MdStatArray> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("Personalities") || trimmed.starts_with("unused devices:") {
            continue;
        }

        if let Some((name, rest)) = trimmed
            .split_once(':')
            .filter(|(name, _)| looks_like_md_name(name.trim()))
        {
            if let Some(array) = current.take() {
                arrays.push(array);
            }
            current = Some(parse_mdstat_array_header(name.trim(), rest.trim()));
        } else if let Some(array) = &mut current {
            parse_mdstat_detail_line(array, trimmed);
        }
    }

    if let Some(array) = current {
        arrays.push(array);
    }

    Ok(arrays)
}

fn looks_like_md_name(name: &str) -> bool {
    name.starts_with("md")
        && name[2..].chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '/')
        })
}

fn parse_mdstat_array_header(name: &str, rest: &str) -> MdStatArray {
    let mut parts = rest.split_whitespace();
    let state = parts.next().map(ToOwned::to_owned);
    let level = parts.next().map(ToOwned::to_owned);
    let members = parts.filter_map(parse_mdstat_member).collect();

    MdStatArray {
        name: format!("/dev/{name}"),
        state,
        level,
        members,
        blocks: None,
        superblock: None,
        layout: None,
        chunk_size: None,
        device_count: None,
        health: None,
        progress: None,
        progress_percent: None,
        progress_blocks: None,
        finish: None,
        speed: None,
        bitmap: None,
    }
}

fn parse_mdstat_detail_line(array: &mut MdStatArray, line: &str) {
    if line.starts_with('[') {
        parse_mdstat_progress(array, line);
    } else if let Some(bitmap) = line.strip_prefix("bitmap:") {
        array.bitmap = Some(bitmap.trim().to_string());
    } else {
        parse_mdstat_size_line(array, line);
    }
}

fn parse_mdstat_size_line(array: &mut MdStatArray, line: &str) {
    let parts: Vec<&str> = line.split_whitespace().collect();
    array.blocks = parts
        .first()
        .filter(|value| value.chars().all(|character| character.is_ascii_digit()))
        .map(|value| (*value).to_string());

    for window in parts.windows(2) {
        match window {
            ["super", value] => array.superblock = Some((*value).to_string()),
            ["chunk", value] => array.chunk_size = Some((*value).to_string()),
            _ => {}
        }
    }

    for part in &parts {
        if part.starts_with('[') && part.ends_with(']') && part.contains('/') {
            array.device_count = Some(part.trim_matches(&['[', ']'][..]).to_string());
        } else if part.starts_with('[')
            && part.ends_with(']')
            && part
                .trim_matches(&['[', ']'][..])
                .chars()
                .all(|character| matches!(character, 'U' | '_'))
        {
            array.health = Some(part.trim_matches(&['[', ']'][..]).to_string());
        } else if part.contains("layout") || part.starts_with("near=") || part.starts_with("far=") {
            array.layout = Some((*part).to_string());
        }
    }
}

fn parse_mdstat_progress(array: &mut MdStatArray, line: &str) {
    let without_bar = line
        .split_once(']')
        .map(|(_, rest)| rest.trim())
        .unwrap_or(line);
    let Some((operation, rest)) = without_bar.split_once('=') else {
        return;
    };
    let operation = operation.trim();
    if operation.is_empty() {
        return;
    }

    array.progress = Some(operation.to_string());
    let mut parts = rest.split_whitespace();
    if let Some(percent) = parts.next() {
        array.progress_percent = Some(percent.to_string());
    }
    if let Some(blocks) = parts.next() {
        array.progress_blocks = Some(blocks.trim_matches(&['(', ')'][..]).to_string());
    }
    for part in parts {
        if let Some(value) = part.strip_prefix("finish=") {
            array.finish = Some(value.to_string());
        } else if let Some(value) = part.strip_prefix("speed=") {
            array.speed = Some(value.to_string());
        }
    }
}

fn parse_mdstat_member(token: &str) -> Option<MdStatMember> {
    let (device, rest) = token.split_once('[')?;
    let (slot, flags) = rest.split_once(']')?;
    if device.is_empty() || slot.is_empty() {
        return None;
    }

    Some(MdStatMember {
        path: format!("/dev/{device}"),
        slot: Some(slot.to_string()),
        flags: flags
            .strip_prefix('(')
            .and_then(|value| value.strip_suffix(')'))
            .map(ToOwned::to_owned)
            .filter(|value| !value.is_empty()),
    })
}

fn add_mdstat_array(graph: &mut StorageGraph, array: MdStatArray) {
    let id = format!("md:{}", array.name);
    let mut node =
        Node::new(id.clone(), NodeKind::MdRaid, array.name.clone()).with_path(array.name);

    if let Some(blocks) = array
        .blocks
        .as_deref()
        .and_then(|value| value.parse::<u64>().ok())
    {
        node = node.with_size_bytes(blocks * 1024);
    }

    for (key, value) in [
        ("md.mdstat-state", array.state),
        ("md.mdstat-level", array.level),
        ("md.mdstat-blocks", array.blocks),
        ("md.mdstat-superblock", array.superblock),
        ("md.mdstat-layout", array.layout),
        ("md.mdstat-chunk-size", array.chunk_size),
        ("md.mdstat-devices", array.device_count),
        ("md.mdstat-health", array.health),
        ("md.mdstat-progress", array.progress),
        ("md.mdstat-progress-percent", array.progress_percent),
        ("md.mdstat-progress-blocks", array.progress_blocks),
        ("md.mdstat-finish", array.finish),
        ("md.mdstat-speed", array.speed),
        ("md.mdstat-bitmap", array.bitmap),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
    }

    graph.add_node(node);

    for member in array.members {
        let member_id = format!("block:{}", member.path);
        let mut member_node =
            Node::new(member_id.clone(), NodeKind::Partition, member.path.clone())
                .with_path(member.path);
        for (key, value) in [
            ("md.mdstat-member-slot", member.slot),
            ("md.mdstat-member-flags", member.flags),
        ] {
            if let Some(value) = value {
                member_node = member_node.with_property(key, value);
            }
        }
        graph.add_node(member_node);
        graph.add_edge(Edge::new(member_id, id.clone(), Relationship::MemberOf));
    }
}

fn add_scan_array(graph: &mut StorageGraph, array: MdScanArray) {
    let id = format!("md:{}", array.name);
    let mut node =
        Node::new(id, NodeKind::MdRaid, array.name.clone()).with_path(array.name.clone());

    if let Some(uuid) = array.uuid.clone() {
        node = node.with_identity(Identity {
            uuid: Some(uuid.clone()),
            ..Identity::default()
        });
        node = node.with_property("md.uuid", uuid);
    }

    for (key, value) in [
        ("md.scan-metadata", array.metadata),
        ("md.scan-name", array.name_property),
        ("md.scan-spares", array.spares),
        ("md.scan-devices", array.devices),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
    }

    graph.add_node(node);
}

fn parse_detail(name: &str, bytes: &[u8]) -> Result<MdArray, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read mdadm detail output: {error}"))
    })?;
    let mut array = MdArray {
        name: name.to_string(),
        uuid: None,
        version: None,
        level: None,
        state: None,
        size_bytes: None,
        used_devices: None,
        total_devices: None,
        array_devices: None,
        active_devices: None,
        working_devices: None,
        failed_devices: None,
        spare_devices: None,
        degraded_devices: None,
        preferred_minor: None,
        name_property: None,
        creation_time: None,
        update_time: None,
        events: None,
        chunk_size: None,
        layout: None,
        consistency_policy: None,
        rebuild_status: None,
        reshape_status: None,
        resync_status: None,
        check_status: None,
        intent_bitmap: None,
        persistence: None,
        bitmap: None,
        members: Vec::new(),
    };
    let mut in_member_table = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Version :") {
            array.version = value_after_colon(trimmed);
        } else if trimmed.starts_with("Raid Level :") {
            array.level = value_after_colon(trimmed);
        } else if trimmed.starts_with("Array Size :") {
            array.size_bytes = parse_array_size(trimmed);
        } else if trimmed.starts_with("State :") {
            array.state = value_after_colon(trimmed);
        } else if trimmed.starts_with("UUID :") {
            array.uuid = value_after_colon(trimmed);
        } else if trimmed.starts_with("Name :") {
            array.name_property = value_after_colon(trimmed);
        } else if trimmed.starts_with("Creation Time :") {
            array.creation_time = value_after_colon(trimmed);
        } else if trimmed.starts_with("Update Time :") {
            array.update_time = value_after_colon(trimmed);
        } else if trimmed.starts_with("Events :") {
            array.events = value_after_colon(trimmed);
        } else if trimmed.starts_with("Chunk Size :") {
            array.chunk_size = value_after_colon(trimmed);
        } else if trimmed.starts_with("Layout :") {
            array.layout = value_after_colon(trimmed);
        } else if trimmed.starts_with("Raid Devices :") {
            array.used_devices = value_after_colon(trimmed);
        } else if trimmed.starts_with("Total Devices :") {
            array.total_devices = value_after_colon(trimmed);
        } else if trimmed.starts_with("Array Devices :") {
            array.array_devices = value_after_colon(trimmed);
        } else if trimmed.starts_with("Active Devices :") {
            array.active_devices = value_after_colon(trimmed);
        } else if trimmed.starts_with("Working Devices :") {
            array.working_devices = value_after_colon(trimmed);
        } else if trimmed.starts_with("Failed Devices :") {
            array.failed_devices = value_after_colon(trimmed);
        } else if trimmed.starts_with("Spare Devices :") {
            array.spare_devices = value_after_colon(trimmed);
        } else if trimmed.starts_with("Degraded Devices :") {
            array.degraded_devices = value_after_colon(trimmed);
        } else if trimmed.starts_with("Preferred Minor :") {
            array.preferred_minor = value_after_colon(trimmed);
        } else if trimmed.starts_with("Consistency Policy :") {
            array.consistency_policy = value_after_colon(trimmed);
        } else if trimmed.starts_with("Rebuild Status :") {
            array.rebuild_status = value_after_colon(trimmed);
        } else if trimmed.starts_with("Reshape Status :") {
            array.reshape_status = value_after_colon(trimmed);
        } else if trimmed.starts_with("Resync Status :") {
            array.resync_status = value_after_colon(trimmed);
        } else if trimmed.starts_with("Check Status :") {
            array.check_status = value_after_colon(trimmed);
        } else if trimmed.starts_with("Intent Bitmap :") {
            array.intent_bitmap = value_after_colon(trimmed);
        } else if trimmed.starts_with("Persistence :") {
            array.persistence = value_after_colon(trimmed);
        } else if trimmed.starts_with("Bitmap :") {
            array.bitmap = value_after_colon(trimmed);
        } else if trimmed.starts_with("Number")
            && trimmed.contains("Major")
            && trimmed.contains("RaidDevice")
            && trimmed.contains("State")
        {
            in_member_table = true;
        } else if in_member_table {
            if let Some(member) = parse_member_line(trimmed) {
                array.members.push(member);
            }
        }
    }

    Ok(array)
}

fn add_array(graph: &mut StorageGraph, array: MdArray) {
    let id = format!("md:{}", array.name);
    let mut node =
        Node::new(id.clone(), NodeKind::MdRaid, array.name.clone()).with_path(array.name);

    if let Some(size_bytes) = array.size_bytes {
        node = node.with_size_bytes(size_bytes);
    }
    if let Some(uuid) = array.uuid.clone() {
        node = node.with_identity(Identity {
            uuid: Some(uuid.clone()),
            ..Identity::default()
        });
        node = node.with_property("md.uuid", uuid);
    }

    let usage = Usage {
        used_bytes: node.size_bytes,
        free_bytes: None,
        allocated_bytes: node.size_bytes,
    };
    if !usage.is_empty() {
        node = node.with_usage(usage);
    }

    for (key, value) in [
        ("md.version", array.version),
        ("md.level", array.level),
        ("md.state", array.state),
        ("md.raid-devices", array.used_devices),
        ("md.total-devices", array.total_devices),
        ("md.array-devices", array.array_devices),
        ("md.active-devices", array.active_devices),
        ("md.working-devices", array.working_devices),
        ("md.failed-devices", array.failed_devices),
        ("md.spare-devices", array.spare_devices),
        ("md.degraded-devices", array.degraded_devices),
        ("md.preferred-minor", array.preferred_minor),
        ("md.name", array.name_property),
        ("md.creation-time", array.creation_time),
        ("md.update-time", array.update_time),
        ("md.events", array.events),
        ("md.chunk-size", array.chunk_size),
        ("md.layout", array.layout),
        ("md.consistency-policy", array.consistency_policy),
        ("md.rebuild-status", array.rebuild_status),
        ("md.reshape-status", array.reshape_status),
        ("md.resync-status", array.resync_status),
        ("md.check-status", array.check_status),
        ("md.intent-bitmap", array.intent_bitmap),
        ("md.persistence", array.persistence),
        ("md.bitmap", array.bitmap),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
    }

    graph.add_node(node);

    for member in array.members {
        let member_id = format!("block:{}", member.path);
        let mut member_node =
            Node::new(member_id.clone(), NodeKind::Partition, member.path.clone())
                .with_path(member.path);
        for (key, value) in [
            ("md.member-number", member.number),
            ("md.member-major", member.major),
            ("md.member-minor", member.minor),
            ("md.member-raid-device", member.raid_device),
            ("md.member-state", member.state),
        ] {
            if let Some(value) = value {
                member_node = member_node.with_property(key, value);
            }
        }
        graph.add_node(member_node);
        graph.add_edge(Edge::new(member_id, id.clone(), Relationship::MemberOf));
    }
}

fn parse_member_line(line: &str) -> Option<MdMember> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    let path = parts.iter().rev().find(|part| part.starts_with("/dev/"))?;
    let path_index = parts.iter().position(|part| part == path)?;
    let state_start = if path_index >= 5 { 4 } else { 0 };
    let state = (path_index > state_start).then(|| parts[state_start..path_index].join(" "));

    Some(MdMember {
        path: (*path).to_string(),
        number: parts.first().copied().map(ToOwned::to_owned),
        major: parts.get(1).copied().map(ToOwned::to_owned),
        minor: parts.get(2).copied().map(ToOwned::to_owned),
        raid_device: parts.get(3).copied().map(ToOwned::to_owned),
        state,
    })
}

fn parse_array_size(line: &str) -> Option<u64> {
    let value = value_after_colon(line)?;
    value
        .split_whitespace()
        .next()
        .and_then(|kib| kib.parse::<u64>().ok())
        .map(|kib| kib * 1024)
}

fn value_after_colon(value: &str) -> Option<String> {
    value
        .split_once(':')
        .map(|(_, value)| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use disk_nix_model::{NodeKind, Relationship};

    use super::*;

    const SCAN: &[u8] = b"ARRAY /dev/md0 metadata=1.2 UUID=aaaa:bbbb:cccc:dddd name=host:0\n";
    const EXAMINE_SCAN: &[u8] =
        b"ARRAY /dev/md/root metadata=1.2 UUID=eeee:ffff:1111:2222 name=host:root spares=1 devices=/dev/sdc1,/dev/sdd1\n";
    const MDSTAT: &[u8] = b"Personalities : [raid1] [raid10]\n\
md0 : active raid1 sdb1[1](F) sda1[0]\n\
      1046528 blocks super 1.2 [2/1] [U_]\n\
      [====>................]  recovery = 20.0% (209305/1046528) finish=1.2min speed=12345K/sec\n\
      bitmap: 0/8 pages [0KB], 65536KB chunk\n\
\n\
unused devices: <none>\n";
    const DETAIL: &[u8] = b"/dev/md0:\n\
           Version : 1.2\n\
     Creation Time : Tue Jun 23 10:15:00 2026\n\
        Raid Level : raid1\n\
        Array Size : 1046528 (1022.00 MiB 1071.64 MB)\n\
       Raid Devices : 2\n\
      Total Devices : 2\n\
      Array Devices : 2\n\
     Active Devices : 2\n\
    Working Devices : 2\n\
     Failed Devices : 0\n\
      Spare Devices : 1\n\
   Degraded Devices : 0\n\
    Preferred Minor : 0\n\
              State : clean\n\
 Consistency Policy : bitmap\n\
    Rebuild Status : 42% complete\n\
    Reshape Status : 25% complete\n\
      Resync Status : delayed\n\
       Check Status : 10% complete\n\
      Intent Bitmap : Internal\n\
       Persistence : Superblock is persistent\n\
            Bitmap : 0/8 pages [0KB], 65536KB chunk\n\
        Update Time : Tue Jun 23 10:16:00 2026\n\
               UUID : aaaa:bbbb:cccc:dddd\n\
               Name : host:0\n\
             Events : 17\n\
\n\
    Number   Major   Minor   RaidDevice State\n\
       0       8        1        0      active sync   /dev/sda1\n\
       1       8       17        1      active sync   /dev/sdb1\n";

    #[test]
    fn extracts_arrays_from_scan() {
        assert_eq!(
            arrays_from_scan(SCAN).expect("scan should parse"),
            vec!["/dev/md0"]
        );
    }

    #[test]
    fn normalizes_md_scan_inventory() {
        let graph = normalize_md_scan(EXAMINE_SCAN).expect("scan should parse");

        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::MdRaid
                && node.name == "/dev/md/root"
                && node.identity.uuid.as_deref() == Some("eeee:ffff:1111:2222")
                && node.properties.iter().any(|property| {
                    property.key == "md.uuid" && property.value == "eeee:ffff:1111:2222"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "md.scan-metadata" && property.value == "1.2")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "md.scan-name" && property.value == "host:root")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "md.scan-spares" && property.value == "1")
                && node.properties.iter().any(|property| {
                    property.key == "md.scan-devices" && property.value == "/dev/sdc1,/dev/sdd1"
                })
        }));
    }

    #[test]
    fn normalizes_mdstat_runtime_state() {
        let graph = normalize_mdstat(MDSTAT).expect("mdstat should parse");
        let array = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::MdRaid && node.name == "/dev/md0")
            .expect("mdstat array exists");

        assert_eq!(array.size_bytes, Some(1_071_644_672));
        assert_property(array, "md.mdstat-state", "active");
        assert_property(array, "md.mdstat-level", "raid1");
        assert_property(array, "md.mdstat-devices", "2/1");
        assert_property(array, "md.mdstat-health", "U_");
        assert_property(array, "md.mdstat-progress", "recovery");
        assert_property(array, "md.mdstat-progress-percent", "20.0%");
        assert_property(array, "md.mdstat-progress-blocks", "209305/1046528");
        assert_property(array, "md.mdstat-finish", "1.2min");
        assert_property(array, "md.mdstat-speed", "12345K/sec");
        assert_property(array, "md.mdstat-bitmap", "0/8 pages [0KB], 65536KB chunk");
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|edge| edge.relationship == Relationship::MemberOf)
                .count(),
            2
        );
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::Partition
                && node.name == "/dev/sdb1"
                && node.properties.iter().any(|property| {
                    property.key == "md.mdstat-member-slot" && property.value == "1"
                })
                && node.properties.iter().any(|property| {
                    property.key == "md.mdstat-member-flags" && property.value == "F"
                })
        }));
    }

    fn assert_property(node: &Node, key: &str, value: &str) {
        assert!(
            node.properties
                .iter()
                .any(|property| property.key == key && property.value == value),
            "missing property {key}={value}; properties: {:?}",
            node.properties
        );
    }

    #[test]
    fn normalizes_md_detail_into_graph() {
        let graph = normalize_md_arrays(&[MdArrayReport {
            name: "/dev/md0".to_string(),
            detail: DETAIL.to_vec(),
        }])
        .expect("detail should parse");

        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::MdRaid && node.name == "/dev/md0")
        );
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::MdRaid
                && node.name == "/dev/md0"
                && node.identity.uuid.as_deref() == Some("aaaa:bbbb:cccc:dddd")
                && node.properties.iter().any(|property| {
                    property.key == "md.uuid" && property.value == "aaaa:bbbb:cccc:dddd"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "md.version" && property.value == "1.2")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "md.events" && property.value == "17")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "md.name" && property.value == "host:0")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "md.active-devices" && property.value == "2")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "md.array-devices" && property.value == "2")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "md.spare-devices" && property.value == "1")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "md.degraded-devices" && property.value == "0")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "md.preferred-minor" && property.value == "0")
                && node.properties.iter().any(|property| {
                    property.key == "md.consistency-policy" && property.value == "bitmap"
                })
                && node.properties.iter().any(|property| {
                    property.key == "md.rebuild-status" && property.value == "42% complete"
                })
                && node.properties.iter().any(|property| {
                    property.key == "md.reshape-status" && property.value == "25% complete"
                })
                && node.properties.iter().any(|property| {
                    property.key == "md.resync-status" && property.value == "delayed"
                })
                && node.properties.iter().any(|property| {
                    property.key == "md.check-status" && property.value == "10% complete"
                })
                && node.properties.iter().any(|property| {
                    property.key == "md.intent-bitmap" && property.value == "Internal"
                })
                && node.properties.iter().any(|property| {
                    property.key == "md.persistence" && property.value == "Superblock is persistent"
                })
                && node.properties.iter().any(|property| {
                    property.key == "md.bitmap"
                        && property.value == "0/8 pages [0KB], 65536KB chunk"
                })
        }));
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|edge| edge.relationship == Relationship::MemberOf)
                .count(),
            2
        );
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::Partition
                && node.name == "/dev/sda1"
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "md.member-number" && property.value == "0")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "md.member-major" && property.value == "8")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "md.member-minor" && property.value == "1")
                && node.properties.iter().any(|property| {
                    property.key == "md.member-raid-device" && property.value == "0"
                })
                && node.properties.iter().any(|property| {
                    property.key == "md.member-state" && property.value == "active sync"
                })
        }));
    }
}
