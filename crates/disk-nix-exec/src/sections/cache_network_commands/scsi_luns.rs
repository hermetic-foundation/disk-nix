fn lun_rescan_devices(action: &PlannedAction) -> Vec<String> {
    let mut devices = BTreeSet::new();
    if let Some(device) = action.context.device.as_deref() {
        devices.insert(device.to_string());
    }
    devices.extend(action.context.devices.iter().cloned());
    devices.into_iter().collect()
}

fn lsscsi_lun_inventory_command(note: &str) -> ExecutionCommand {
    command(["lsscsi", "-t", "-s"], false, note)
}

fn scsi_device_rescan_command(device: &str) -> ExecutionCommand {
    command_vec(
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "block=$(basename \"$(readlink -f \"$1\")\"); printf '1\\n' > \"/sys/class/block/${block}/device/rescan\""
                .to_string(),
            "disk-nix-scsi-rescan".to_string(),
            device.to_string(),
        ],
        true,
        "rescan the reviewed SCSI block path after target-side changes",
    )
}

fn scsi_device_delete_command(device: &str) -> ExecutionCommand {
    command_vec(
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "block=$(basename \"$(readlink -f \"$1\")\"); printf '1\\n' > \"/sys/class/block/${block}/device/delete\""
                .to_string(),
            "disk-nix-scsi-delete".to_string(),
            device.to_string(),
        ],
        true,
        "detach the reviewed SCSI block path from the host",
    )
}
