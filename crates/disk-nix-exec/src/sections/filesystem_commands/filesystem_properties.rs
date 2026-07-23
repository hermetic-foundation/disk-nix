fn filesystem_property_command(
    fs_type: Option<&str>,
    target: &str,
    device: Option<&str>,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    match fs_type {
        Some("btrfs") => btrfs_filesystem_property_command(target, device, property, assignment),
        Some("ext2" | "ext3" | "ext4") => {
            ext_filesystem_property_command(device, target, property, assignment)
        }
        Some("fat" | "fat12" | "fat16" | "fat32" | "msdos" | "vfat") => {
            fat_filesystem_property_command(device, target, property, assignment)
        }
        Some("ntfs" | "ntfs3") => {
            ntfs_filesystem_property_command(device, target, property, assignment)
        }
        Some("exfat") => exfat_filesystem_property_command(device, target, property, assignment),
        Some("f2fs") => f2fs_filesystem_property_command(device, target, property, assignment),
        Some("xfs") => xfs_filesystem_property_command(device, target, property, assignment),
        Some("zfs") => command(
            ["zfs", "set", assignment, target],
            true,
            "set a ZFS filesystem property",
        ),
        _ => command_with_readiness(
            ["<filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem type", "supported filesystem property"],
            "set a filesystem property after selecting the filesystem-specific command",
        ),
    }
}

fn swap_property_command(
    target: Option<&str>,
    property: &str,
    value: Option<&str>,
) -> ExecutionCommand {
    match (property, target, value) {
        ("label" | "swap.label", Some(target), Some(value)) => command(
            ["swaplabel", "--label", value, target],
            true,
            "set the swap signature label on the reviewed inactive swap target",
        ),
        ("label" | "swap.label", None, Some(value)) => command_with_readiness(
            ["swaplabel", "--label", value, "<swap>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["swap target path"],
            "set the swap label after resolving the swap target",
        ),
        ("label" | "swap.label", Some(target), None) => command_with_readiness(
            ["swaplabel", "--label", "<label>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["swap label"],
            "set the swap label after resolving the desired label",
        ),
        ("label" | "swap.label", None, None) => command_with_readiness(
            ["swaplabel", "--label", "<label>", "<swap>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["swap target path", "swap label"],
            "set the swap label after resolving target and label",
        ),
        ("uuid" | "swap.uuid", Some(target), Some(value)) => command(
            ["swaplabel", "--uuid", value, target],
            true,
            "set the swap signature UUID on the reviewed inactive swap target",
        ),
        ("uuid" | "swap.uuid", None, Some(value)) => command_with_readiness(
            ["swaplabel", "--uuid", value, "<swap>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["swap target path"],
            "set the swap UUID after resolving the swap target",
        ),
        ("uuid" | "swap.uuid", Some(target), None) => command_with_readiness(
            ["swaplabel", "--uuid", "<uuid>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["swap UUID"],
            "set the swap UUID after resolving the desired UUID",
        ),
        ("uuid" | "swap.uuid", None, None) => command_with_readiness(
            ["swaplabel", "--uuid", "<uuid>", "<swap>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["swap target path", "swap UUID"],
            "set the swap UUID after resolving target and UUID",
        ),
        ("priority" | "swap.priority", Some(target), Some(value))
            if value.parse::<i32>().is_ok() =>
        {
            command_vec(
                vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    format!(
                        "swapoff {} 2>/dev/null || true; swapon --priority {} {}",
                        shell_quote(target),
                        shell_quote(value),
                        shell_quote(target)
                    ),
                ],
                true,
                "reactivate the reviewed swap target with the requested priority",
            )
        }
        ("priority" | "swap.priority", None, Some(value)) if value.parse::<i32>().is_ok() => {
            command_with_readiness(
                ["swapon", "--priority", value, "<swap>"],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["swap target path"],
                "reactivate swap with the requested priority after resolving the target",
            )
        }
        ("priority" | "swap.priority", Some(target), Some(_)) => command_with_readiness(
            ["swapon", "--priority", "<priority>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["integer swap priority"],
            "reactivate swap after resolving an integer priority",
        ),
        ("priority" | "swap.priority", Some(target), None) => command_with_readiness(
            ["swapon", "--priority", "<priority>", target],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["integer swap priority"],
            "reactivate swap after resolving the requested priority",
        ),
        ("priority" | "swap.priority", None, _) => command_with_readiness(
            ["swapon", "--priority", "<priority>", "<swap>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["swap target path", "integer swap priority"],
            "reactivate swap after resolving target and priority",
        ),
        _ => command_with_readiness(
            ["<swap-property-tool>", target.unwrap_or("<swap>"), property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported swap property"],
            "set a swap property after selecting a supported property mapping",
        ),
    }
}

fn fat_filesystem_property_command(
    device: Option<&str>,
    target: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<fat-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["FAT filesystem property value"],
            "set a FAT filesystem property after resolving the desired value",
        );
    };
    match (property, device) {
        ("label" | "fat.label" | "vfat.label" | "filesystem.label", Some(device)) => command(
            ["fatlabel", device, value],
            true,
            "set the FAT filesystem label on the reviewed backing device",
        ),
        ("label" | "fat.label" | "vfat.label" | "filesystem.label", None) => {
            command_with_readiness(
                ["fatlabel", "<filesystem-device>", value],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["filesystem source device"],
                "set the FAT filesystem label after resolving the backing device",
            )
        }
        (
            "uuid" | "fat.uuid" | "vfat.uuid" | "filesystem.uuid" | "volumeId" | "volume-id"
            | "fat.volume-id" | "vfat.volume-id",
            Some(device),
        ) => match fat_volume_id(value) {
            Some(volume_id) => command_vec(
                ["fatlabel", "-i", device, volume_id.as_str()],
                true,
                "set the FAT filesystem volume ID on the reviewed unmounted backing device",
            ),
            None => command_with_readiness(
                ["<fat-filesystem-property-tool>", target, property],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["8-hex-digit FAT volume ID"],
                "set a FAT filesystem volume ID after resolving a valid value",
            ),
        },
        (
            "uuid" | "fat.uuid" | "vfat.uuid" | "filesystem.uuid" | "volumeId" | "volume-id"
            | "fat.volume-id" | "vfat.volume-id",
            None,
        ) => match fat_volume_id(value) {
            Some(volume_id) => command_vec_with_readiness(
                ["fatlabel", "-i", "<filesystem-device>", volume_id.as_str()],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["filesystem source device"],
                "set the FAT filesystem volume ID after resolving the backing device",
            ),
            None => command_with_readiness(
                ["<fat-filesystem-property-tool>", target, property],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["filesystem source device", "8-hex-digit FAT volume ID"],
                "set a FAT filesystem volume ID after resolving device and value",
            ),
        },
        _ => command_with_readiness(
            ["<fat-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported FAT filesystem property"],
            "set a FAT filesystem property after selecting a supported property mapping",
        ),
    }
}

fn fat_volume_id(value: &str) -> Option<String> {
    let normalized: String = value
        .trim()
        .chars()
        .filter(|character| *character != '-')
        .collect();
    if normalized.len() == 8
        && normalized
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        Some(normalized.to_ascii_uppercase())
    } else {
        None
    }
}

fn ntfs_filesystem_property_command(
    device: Option<&str>,
    target: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<ntfs-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["NTFS filesystem property value"],
            "set an NTFS filesystem property after resolving the desired value",
        );
    };
    match (property, device) {
        ("label" | "ntfs.label" | "filesystem.label", Some(device)) => command(
            ["ntfslabel", device, value],
            true,
            "set the NTFS filesystem label on the reviewed backing device",
        ),
        ("label" | "ntfs.label" | "filesystem.label", None) => command_with_readiness(
            ["ntfslabel", "<filesystem-device>", value],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "set the NTFS filesystem label after resolving the backing device",
        ),
        (
            "uuid" | "ntfs.uuid" | "filesystem.uuid" | "serial" | "volumeSerial" | "volume-serial"
            | "ntfs.serial" | "ntfs.volume-serial",
            Some(device),
        ) => match ntfs_volume_serial(value) {
            Some(serial) => command_vec(
                vec![
                    "ntfslabel".to_string(),
                    format!("--new-serial={serial}"),
                    device.to_string(),
                ],
                true,
                "set the NTFS filesystem volume serial on the reviewed unmounted backing device",
            ),
            None => command_with_readiness(
                ["<ntfs-filesystem-property-tool>", target, property],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["16-hex-digit NTFS volume serial"],
                "set an NTFS filesystem volume serial after resolving a valid value",
            ),
        },
        (
            "uuid" | "ntfs.uuid" | "filesystem.uuid" | "serial" | "volumeSerial" | "volume-serial"
            | "ntfs.serial" | "ntfs.volume-serial",
            None,
        ) => match ntfs_volume_serial(value) {
            Some(serial) => command_vec_with_readiness(
                vec![
                    "ntfslabel".to_string(),
                    format!("--new-serial={serial}"),
                    "<filesystem-device>".to_string(),
                ],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["filesystem source device"],
                "set the NTFS filesystem volume serial after resolving the backing device",
            ),
            None => command_with_readiness(
                ["<ntfs-filesystem-property-tool>", target, property],
                true,
                CommandReadiness::NeedsDomainImplementation,
                [
                    "filesystem source device",
                    "16-hex-digit NTFS volume serial",
                ],
                "set an NTFS filesystem volume serial after resolving device and value",
            ),
        },
        _ => command_with_readiness(
            ["<ntfs-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported NTFS filesystem property"],
            "set an NTFS filesystem property after selecting a supported property mapping",
        ),
    }
}

fn ntfs_volume_serial(value: &str) -> Option<String> {
    let normalized: String = value
        .trim()
        .chars()
        .filter(|character| *character != '-')
        .collect();
    if normalized.len() == 16
        && normalized
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        Some(normalized.to_ascii_uppercase())
    } else {
        None
    }
}

fn exfat_filesystem_property_command(
    device: Option<&str>,
    target: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<exfat-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["exFAT filesystem property value"],
            "set an exFAT filesystem property after resolving the desired value",
        );
    };
    match (property, device) {
        ("label" | "exfat.label" | "filesystem.label", Some(device)) => command(
            ["exfatlabel", device, value],
            true,
            "set the exFAT filesystem label on the reviewed backing device",
        ),
        ("label" | "exfat.label" | "filesystem.label", None) => command_with_readiness(
            ["exfatlabel", "<filesystem-device>", value],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "set the exFAT filesystem label after resolving the backing device",
        ),
        (
            "uuid"
            | "exfat.uuid"
            | "filesystem.uuid"
            | "serial"
            | "volumeSerial"
            | "volume-serial"
            | "exfat.serial"
            | "exfat.volume-serial",
            Some(device),
        ) => match exfat_volume_serial(value) {
            Some(serial) => command_vec(
                ["exfatlabel", "-i", device, serial.as_str()],
                true,
                "set the exFAT filesystem volume serial on the reviewed unmounted backing device",
            ),
            None => command_with_readiness(
                ["<exfat-filesystem-property-tool>", target, property],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["8-hex-digit exFAT volume serial"],
                "set an exFAT filesystem volume serial after resolving a valid value",
            ),
        },
        (
            "uuid"
            | "exfat.uuid"
            | "filesystem.uuid"
            | "serial"
            | "volumeSerial"
            | "volume-serial"
            | "exfat.serial"
            | "exfat.volume-serial",
            None,
        ) => match exfat_volume_serial(value) {
            Some(serial) => command_vec_with_readiness(
                ["exfatlabel", "-i", "<filesystem-device>", serial.as_str()],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["filesystem source device"],
                "set the exFAT filesystem volume serial after resolving the backing device",
            ),
            None => command_with_readiness(
                ["<exfat-filesystem-property-tool>", target, property],
                true,
                CommandReadiness::NeedsDomainImplementation,
                [
                    "filesystem source device",
                    "8-hex-digit exFAT volume serial",
                ],
                "set an exFAT filesystem volume serial after resolving device and value",
            ),
        },
        _ => command_with_readiness(
            ["<exfat-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported exFAT filesystem property"],
            "set an exFAT filesystem property after selecting a supported property mapping",
        ),
    }
}

fn exfat_volume_serial(value: &str) -> Option<String> {
    let normalized: String = value
        .trim()
        .chars()
        .filter(|character| *character != '-')
        .collect();
    if normalized.len() == 8
        && normalized
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        Some(normalized.to_ascii_uppercase())
    } else {
        None
    }
}

fn f2fs_filesystem_property_command(
    device: Option<&str>,
    target: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<f2fs-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["F2FS filesystem property value"],
            "set an F2FS filesystem property after resolving the desired value",
        );
    };
    match (property, filesystem_source_device(target, device)) {
        ("label" | "f2fs.label" | "filesystem.label", Some(source)) => command(
            ["f2fslabel", source, value],
            true,
            "set the F2FS filesystem label on the reviewed backing device",
        ),
        ("label" | "f2fs.label" | "filesystem.label", None) => command_with_readiness(
            ["f2fslabel", "<filesystem-device>", value],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "set the F2FS filesystem label after resolving the backing device",
        ),
        _ => command_with_readiness(
            ["<f2fs-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported F2FS filesystem property"],
            "set an F2FS filesystem property after selecting a supported property mapping",
        ),
    }
}

fn xfs_filesystem_property_command(
    device: Option<&str>,
    target: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<xfs-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["XFS filesystem property value"],
            "set an XFS filesystem property after resolving the desired value",
        );
    };
    match (property, device) {
        ("label" | "xfs.label" | "filesystem.label", Some(device)) => command(
            ["xfs_admin", "-L", value, device],
            true,
            "set the XFS filesystem label on the reviewed backing device",
        ),
        ("label" | "xfs.label" | "filesystem.label", None) => command_with_readiness(
            ["xfs_admin", "-L", value, "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "set the XFS filesystem label after resolving the backing device",
        ),
        ("uuid" | "xfs.uuid" | "filesystem.uuid", Some(device)) => command(
            ["xfs_admin", "-U", value, device],
            true,
            "set the XFS filesystem UUID on the reviewed unmounted backing device",
        ),
        ("uuid" | "xfs.uuid" | "filesystem.uuid", None) => command_with_readiness(
            ["xfs_admin", "-U", value, "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "set the XFS filesystem UUID after resolving the backing device",
        ),
        _ => command_with_readiness(
            ["<xfs-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported XFS filesystem property"],
            "set an XFS filesystem property after selecting a supported property mapping",
        ),
    }
}

fn ext_filesystem_property_command(
    device: Option<&str>,
    target: &str,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<ext-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["Ext filesystem property value"],
            "set an Ext filesystem property after resolving the desired value",
        );
    };
    match (property, device) {
        ("label" | "ext.label" | "filesystem.label", Some(device)) => command(
            ["e2label", device, value],
            true,
            "set the Ext filesystem label on the reviewed backing device",
        ),
        ("label" | "ext.label" | "filesystem.label", None) => command_with_readiness(
            ["e2label", "<filesystem-device>", value],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "set the Ext filesystem label after resolving the backing device",
        ),
        ("uuid" | "ext.uuid" | "filesystem.uuid", Some(device)) => command(
            ["tune2fs", "-U", value, device],
            true,
            "set the Ext filesystem UUID on the reviewed unmounted backing device",
        ),
        ("uuid" | "ext.uuid" | "filesystem.uuid", None) => command_with_readiness(
            ["tune2fs", "-U", value, "<filesystem-device>"],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["filesystem source device"],
            "set the Ext filesystem UUID after resolving the backing device",
        ),
        _ => command_with_readiness(
            ["<ext-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported Ext filesystem property"],
            "set an Ext filesystem property after selecting a supported property mapping",
        ),
    }
}

fn btrfs_filesystem_property_command(
    target: &str,
    device: Option<&str>,
    property: &str,
    assignment: &str,
) -> ExecutionCommand {
    let Some((_, value)) = assignment.split_once('=') else {
        return command_with_readiness(
            ["<btrfs-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["Btrfs filesystem property value"],
            "set a Btrfs filesystem property after resolving the desired value",
        );
    };
    match property {
        "label" | "btrfs.label" | "filesystem.label" => command(
            ["btrfs", "filesystem", "label", target, value],
            true,
            "set the Btrfs filesystem label",
        ),
        "uuid" | "btrfs.uuid" | "filesystem.uuid" => match device {
            Some(device) => command(
                ["btrfstune", "-U", value, device],
                true,
                "set the Btrfs filesystem UUID on the reviewed unmounted backing device",
            ),
            None => command_with_readiness(
                ["btrfstune", "-U", value, "<filesystem-device>"],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["filesystem source device"],
                "set the Btrfs filesystem UUID after resolving the backing device",
            ),
        },
        _ => command_with_readiness(
            ["<btrfs-filesystem-property-tool>", target, property],
            true,
            CommandReadiness::NeedsDomainImplementation,
            ["supported Btrfs filesystem property"],
            "set a Btrfs filesystem property after selecting a supported property mapping",
        ),
    }
}
