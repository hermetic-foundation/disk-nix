pub fn normalize_nvme_smart_log_json(
    controller: &str,
    bytes: &[u8],
) -> Result<StorageGraph, ProbeError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|error| {
        ProbeError::Adapter(format!(
            "failed to parse nvme smart-log JSON for {controller}: {error}"
        ))
    })?;
    let name = controller.trim_start_matches("/dev/");
    let mut node = Node::new(
        nvme_controller_id(name),
        NodeKind::NvmeController,
        name.to_string(),
    )
    .with_path(controller_path(name))
    .with_property("nvme.controller", name.to_string());

    for (key, property) in [
        ("critical_warning", "nvme.smart.critical-warning"),
        ("temperature", "nvme.smart.temperature-kelvin"),
        ("avail_spare", "nvme.smart.available-spare-percent"),
        ("spare_thresh", "nvme.smart.spare-threshold-percent"),
        ("percent_used", "nvme.smart.percent-used"),
        ("data_units_read", "nvme.smart.data-units-read"),
        ("data_units_written", "nvme.smart.data-units-written"),
        ("host_read_commands", "nvme.smart.host-read-commands"),
        ("host_write_commands", "nvme.smart.host-write-commands"),
        ("controller_busy_time", "nvme.smart.controller-busy-time"),
        ("power_cycles", "nvme.smart.power-cycles"),
        ("power_on_hours", "nvme.smart.power-on-hours"),
        ("unsafe_shutdowns", "nvme.smart.unsafe-shutdowns"),
        ("media_errors", "nvme.smart.media-errors"),
        ("num_err_log_entries", "nvme.smart.error-log-entries"),
        ("warning_temp_time", "nvme.smart.warning-temperature-time"),
        (
            "critical_comp_time",
            "nvme.smart.critical-composite-temperature-time",
        ),
        (
            "temperature_sensor_1",
            "nvme.smart.temperature-sensor-1-kelvin",
        ),
        (
            "temperature_sensor_2",
            "nvme.smart.temperature-sensor-2-kelvin",
        ),
        (
            "temperature_sensor_3",
            "nvme.smart.temperature-sensor-3-kelvin",
        ),
        (
            "temperature_sensor_4",
            "nvme.smart.temperature-sensor-4-kelvin",
        ),
        (
            "temperature_sensor_5",
            "nvme.smart.temperature-sensor-5-kelvin",
        ),
        (
            "temperature_sensor_6",
            "nvme.smart.temperature-sensor-6-kelvin",
        ),
        (
            "temperature_sensor_7",
            "nvme.smart.temperature-sensor-7-kelvin",
        ),
        (
            "temperature_sensor_8",
            "nvme.smart.temperature-sensor-8-kelvin",
        ),
        (
            "thm_temp1_trans_count",
            "nvme.smart.thermal-temp1-transition-count",
        ),
        (
            "thm_temp2_trans_count",
            "nvme.smart.thermal-temp2-transition-count",
        ),
        (
            "thm_temp1_total_time",
            "nvme.smart.thermal-temp1-total-time",
        ),
        (
            "thm_temp2_total_time",
            "nvme.smart.thermal-temp2-total-time",
        ),
    ] {
        if let Some(value) = field_string(&value, key) {
            node = node.with_property(property, value);
        }
    }

    let mut graph = StorageGraph::empty();
    graph.add_node(node);
    Ok(graph)
}
