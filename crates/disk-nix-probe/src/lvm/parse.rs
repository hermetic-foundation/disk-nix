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
