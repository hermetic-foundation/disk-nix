use disk_nix_model::{Edge, Identity, Node, NodeKind, Relationship, StorageGraph, Usage};
use serde::Deserialize;

use crate::ProbeError;

#[derive(Debug, Deserialize)]
struct LvmDocument {
    report: Vec<LvmReport>,
}

#[derive(Debug, Deserialize)]
struct LvmReport {
    #[serde(default)]
    pv: Vec<PhysicalVolume>,
    #[serde(default)]
    vg: Vec<VolumeGroup>,
    #[serde(default)]
    lv: Vec<LogicalVolume>,
    #[serde(default)]
    seg: Vec<LogicalVolumeSegment>,
}

#[derive(Debug, Deserialize)]
struct PhysicalVolume {
    pv_name: String,
    vg_name: Option<String>,
    pv_uuid: Option<String>,
    pv_size: Option<String>,
    pv_free: Option<String>,
    pv_used: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VolumeGroup {
    vg_name: String,
    vg_uuid: Option<String>,
    vg_size: Option<String>,
    vg_free: Option<String>,
    vg_extent_size: Option<String>,
    pv_count: Option<String>,
    lv_count: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LogicalVolume {
    lv_name: String,
    vg_name: String,
    lv_uuid: Option<String>,
    lv_path: Option<String>,
    lv_size: Option<String>,
    lv_attr: Option<String>,
    lv_active: Option<String>,
    lv_role: Option<String>,
    lv_time: Option<String>,
    origin: Option<String>,
    pool_lv: Option<String>,
    data_percent: Option<String>,
    metadata_percent: Option<String>,
    cache_mode: Option<String>,
    cache_policy: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LogicalVolumeSegment {
    lv_name: String,
    vg_name: String,
    segtype: Option<String>,
    seg_start: Option<String>,
    seg_size: Option<String>,
    devices: Option<String>,
    seg_pe_ranges: Option<String>,
}

pub fn normalize_lvm_json(
    pvs: &[u8],
    vgs: &[u8],
    lvs: &[u8],
    segments: Option<&[u8]>,
) -> Result<StorageGraph, ProbeError> {
    let mut graph = StorageGraph::empty();

    for pv in parse_pvs(pvs)? {
        add_physical_volume(&mut graph, pv);
    }
    for vg in parse_vgs(vgs)? {
        add_volume_group(&mut graph, vg);
    }
    for lv in parse_lvs(lvs)? {
        add_logical_volume(&mut graph, lv);
    }
    if let Some(segments) = segments {
        for (index, segment) in parse_segments(segments)?.into_iter().enumerate() {
            add_logical_volume_segment(&mut graph, segment, index);
        }
    }

    Ok(graph)
}

fn parse_document(bytes: &[u8], report_name: &str) -> Result<LvmDocument, ProbeError> {
    let document: LvmDocument = serde_json::from_slice(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to parse {report_name} JSON: {error}"))
    })?;
    Ok(document)
}

fn parse_pvs(bytes: &[u8]) -> Result<Vec<PhysicalVolume>, ProbeError> {
    let document = parse_document(bytes, "pv")?;
    Ok(document
        .report
        .into_iter()
        .flat_map(|report| report.pv)
        .collect())
}

fn parse_vgs(bytes: &[u8]) -> Result<Vec<VolumeGroup>, ProbeError> {
    let document = parse_document(bytes, "vg")?;
    Ok(document
        .report
        .into_iter()
        .flat_map(|report| report.vg)
        .collect())
}

fn parse_lvs(bytes: &[u8]) -> Result<Vec<LogicalVolume>, ProbeError> {
    let document = parse_document(bytes, "lv")?;
    Ok(document
        .report
        .into_iter()
        .flat_map(|report| report.lv)
        .collect())
}

fn parse_segments(bytes: &[u8]) -> Result<Vec<LogicalVolumeSegment>, ProbeError> {
    let document = parse_document(bytes, "lv segment")?;
    Ok(document
        .report
        .into_iter()
        .flat_map(|report| report.seg)
        .collect())
}

fn add_physical_volume(graph: &mut StorageGraph, pv: PhysicalVolume) {
    let id = pv_id(&pv.pv_name);
    let mut node = Node::new(id.clone(), NodeKind::LvmPhysicalVolume, pv.pv_name.clone())
        .with_path(pv.pv_name.clone());

    if let Some(size_bytes) = parse_lvm_size(pv.pv_size.as_deref()) {
        node = node.with_size_bytes(size_bytes);
    }

    let usage = Usage {
        used_bytes: parse_lvm_size(pv.pv_used.as_deref()),
        free_bytes: parse_lvm_size(pv.pv_free.as_deref()),
        allocated_bytes: None,
    };
    if !usage.is_empty() {
        node = node.with_usage(usage);
    }

    if let Some(uuid) = pv.pv_uuid {
        node = node.with_identity(Identity {
            uuid: Some(uuid),
            ..Identity::default()
        });
    }

    if let Some(vg_name) = pv.vg_name.filter(|name| !name.is_empty()) {
        graph.add_edge(Edge::new(
            id.clone(),
            vg_id(&vg_name),
            Relationship::MemberOf,
        ));
        node = node.with_property("lvm.vg", vg_name);
    }

    graph.add_node(node);
}

fn add_volume_group(graph: &mut StorageGraph, vg: VolumeGroup) {
    let id = vg_id(&vg.vg_name);
    let mut node = Node::new(id, NodeKind::LvmVolumeGroup, vg.vg_name);

    if let Some(size_bytes) = parse_lvm_size(vg.vg_size.as_deref()) {
        node = node.with_size_bytes(size_bytes);
    }

    let usage = Usage {
        used_bytes: None,
        free_bytes: parse_lvm_size(vg.vg_free.as_deref()),
        allocated_bytes: None,
    };
    if !usage.is_empty() {
        node = node.with_usage(usage);
    }

    if let Some(uuid) = vg.vg_uuid {
        node = node.with_identity(Identity {
            uuid: Some(uuid),
            ..Identity::default()
        });
    }

    for (key, value) in [
        ("lvm.extent-size", vg.vg_extent_size),
        ("lvm.pv-count", vg.pv_count),
        ("lvm.lv-count", vg.lv_count),
    ] {
        if let Some(value) = value.filter(|value| !value.is_empty()) {
            node = node.with_property(key, value);
        }
    }

    graph.add_node(node);
}

fn add_logical_volume(graph: &mut StorageGraph, lv: LogicalVolume) {
    let id = lv_id(&lv.vg_name, &lv.lv_name);
    let kind = lv_kind(lv.lv_attr.as_deref());
    let mut node = Node::new(id.clone(), kind, format!("{}/{}", lv.vg_name, lv.lv_name));

    if let Some(path) = &lv.lv_path {
        node = node.with_path(path.clone());
    }
    if let Some(size_bytes) = parse_lvm_size(lv.lv_size.as_deref()) {
        node = node.with_size_bytes(size_bytes);
    }
    if let Some(uuid) = &lv.lv_uuid {
        node = node.with_identity(Identity {
            uuid: Some(uuid.clone()),
            ..Identity::default()
        });
    }

    for (key, value) in [
        ("lvm.attr", lv.lv_attr.clone()),
        ("lvm.active", lv.lv_active.clone()),
        ("lvm.role", lv.lv_role.clone()),
        ("lvm.time", lv.lv_time.clone()),
        ("lvm.origin", lv.origin.clone()),
        ("lvm.pool", lv.pool_lv.clone()),
        ("lvm.data-percent", lv.data_percent.clone()),
        ("lvm.metadata-percent", lv.metadata_percent.clone()),
        ("lvm.cache-mode", lv.cache_mode.clone()),
        ("lvm.cache-policy", lv.cache_policy.clone()),
    ] {
        if let Some(value) = value.filter(|value| !value.is_empty()) {
            node = node.with_property(key, value);
        }
    }

    graph.add_edge(Edge::new(
        vg_id(&lv.vg_name),
        id.clone(),
        Relationship::Contains,
    ));

    if let Some(origin) = lv.origin.filter(|origin| !origin.is_empty()) {
        graph.add_edge(Edge::new(
            id.clone(),
            lv_id(&lv.vg_name, &origin),
            Relationship::SnapshotOf,
        ));
    }
    if let Some(pool) = lv.pool_lv.filter(|pool| !pool.is_empty()) {
        graph.add_edge(Edge::new(
            id.clone(),
            lv_id(&lv.vg_name, &pool),
            Relationship::DependsOn,
        ));
    }

    graph.add_node(node);
}

fn add_logical_volume_segment(
    graph: &mut StorageGraph,
    segment: LogicalVolumeSegment,
    index: usize,
) {
    let lv_id = lv_id(&segment.vg_name, &segment.lv_name);
    let id = format!("lvm-seg:{}/{}:{index}", segment.vg_name, segment.lv_name);
    let mut node = Node::new(
        id.clone(),
        NodeKind::LvmSegment,
        format!("{}/{}:{index}", segment.vg_name, segment.lv_name),
    );

    if let Some(size_bytes) = parse_lvm_size(segment.seg_size.as_deref()) {
        node = node.with_size_bytes(size_bytes);
    }

    for (key, value) in [
        ("lvm.segment-type", segment.segtype.clone()),
        ("lvm.segment-start", segment.seg_start.clone()),
        ("lvm.segment-size", segment.seg_size.clone()),
        ("lvm.devices", segment.devices.clone()),
        ("lvm.segment-pe-ranges", segment.seg_pe_ranges.clone()),
    ] {
        if let Some(value) = value.filter(|value| !value.is_empty()) {
            node = node.with_property(key, value);
        }
    }

    graph.add_node(node);
    graph.add_edge(Edge::new(lv_id.clone(), id.clone(), Relationship::Contains));

    if let Some(devices) = segment.devices {
        for device in split_lvm_devices(&devices) {
            graph.add_edge(Edge::new(
                id.clone(),
                dependency_id(&segment.vg_name, &device),
                Relationship::DependsOn,
            ));
        }
    }
}

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

#[cfg(test)]
mod tests {
    use disk_nix_model::{NodeKind, Relationship};

    use super::*;

    const PVS: &[u8] = br#"{
      "report": [{
        "pv": [{
          "pv_name": "/dev/mapper/cryptroot",
          "vg_name": "vg0",
          "pv_uuid": "pv-uuid",
          "pv_size": "100.00g",
          "pv_free": "20.00g",
          "pv_used": "80.00g"
        }]
      }]
    }"#;

    const VGS: &[u8] = br#"{
      "report": [{
        "vg": [{
          "vg_name": "vg0",
          "vg_uuid": "vg-uuid",
          "vg_size": "100.00g",
          "vg_free": "20.00g",
          "vg_extent_size": "4.00m",
          "pv_count": "1",
          "lv_count": "3"
        }]
      }]
    }"#;

    const LVS: &[u8] = br#"{
      "report": [{
        "lv": [
          {
            "lv_name": "root",
            "vg_name": "vg0",
            "lv_uuid": "lv-root",
            "lv_path": "/dev/vg0/root",
            "lv_size": "40.00g",
            "lv_attr": "-wi-ao----",
            "lv_active": "active",
            "lv_role": "public",
            "lv_time": "2026-06-23 10:00:00 -0500",
            "origin": "",
            "pool_lv": "",
            "data_percent": "",
            "metadata_percent": "",
            "cache_mode": "",
            "cache_policy": ""
          },
          {
            "lv_name": "root-snap",
            "vg_name": "vg0",
            "lv_uuid": "lv-snap",
            "lv_path": "/dev/vg0/root-snap",
            "lv_size": "10.00g",
            "lv_attr": "swi-a-s---",
            "lv_active": "active",
            "lv_role": "public",
            "lv_time": "2026-06-23 10:05:00 -0500",
            "origin": "root",
            "pool_lv": "",
            "data_percent": "12.00",
            "metadata_percent": "",
            "cache_mode": "writeback",
            "cache_policy": "smq"
          }
        ]
      }]
    }"#;

    const SEGMENTS: &[u8] = br#"{
      "report": [{
        "seg": [
          {
            "lv_name": "root",
            "vg_name": "vg0",
            "segtype": "linear",
            "seg_start": "0",
            "seg_size": "40.00g",
            "devices": "/dev/mapper/cryptroot(0)",
            "seg_pe_ranges": "/dev/mapper/cryptroot:0-10239"
          },
          {
            "lv_name": "root-snap",
            "vg_name": "vg0",
            "segtype": "snapshot",
            "seg_start": "0",
            "seg_size": "10.00g",
            "devices": "root(0)",
            "seg_pe_ranges": "root:0-2559"
          }
        ]
      }]
    }"#;

    #[test]
    fn normalizes_lvm_reports_into_graph() {
        let graph =
            normalize_lvm_json(PVS, VGS, LVS, Some(SEGMENTS)).expect("fixture should parse");

        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::LvmPhysicalVolume)
        );
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::LvmVolumeGroup && node.name == "vg0")
        );
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::LvmSnapshot && node.name == "vg0/root-snap")
        );
        assert!(
            graph
                .edges
                .iter()
                .any(|edge| edge.relationship == Relationship::SnapshotOf)
        );
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::LvmSegment
                && node.properties.iter().any(|property| {
                    property.key == "lvm.segment-type" && property.value == "linear"
                })
        }));
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::LvmSnapshot
                && node.name == "vg0/root-snap"
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "lvm.active" && property.value == "active")
                && node.properties.iter().any(|property| {
                    property.key == "lvm.cache-mode" && property.value == "writeback"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "lvm.cache-policy" && property.value == "smq")
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0.starts_with("lvm-seg:vg0/root:")
                && edge.to.0 == "block:/dev/mapper/cryptroot"
                && edge.relationship == Relationship::DependsOn
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0.starts_with("lvm-seg:vg0/root-snap:")
                && edge.to.0 == "lvm-lv:vg0/root"
                && edge.relationship == Relationship::DependsOn
        }));
    }

    #[test]
    fn parses_lvm_size_suffixes() {
        assert_eq!(parse_lvm_size(Some("1.50g")), Some(1_610_612_736));
        assert_eq!(parse_lvm_size(Some("4.00m")), Some(4_194_304));
        assert_eq!(parse_lvm_size(Some("")), None);
    }

    #[test]
    fn splits_lvm_device_references() {
        assert_eq!(
            split_lvm_devices("/dev/sda2(0), root_cdata(12)"),
            vec!["/dev/sda2".to_string(), "root_cdata".to_string()]
        );
    }
}
