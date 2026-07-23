fn property_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::SetProperty {
        return None;
    }
    let property = action.context.property.as_deref()?;
    let desired = action.context.property_value.as_deref()?;
    let current = current_property_value(action, node, property)?;
    let desired_compare = comparable_property_value(action, property, desired);
    let current_compare = comparable_property_value(action, property, &current);
    let (level, kind, message) = if current_compare == desired_compare {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::PropertyAlreadySatisfied,
            format!("property {property} already has desired value {desired}"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::PropertyDiffers,
            format!("property {property} is {current}, desired {desired}"),
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level,
        kind,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn current_property_value(action: &PlannedAction, node: &Node, property: &str) -> Option<String> {
    let normalized = normalize_storage_property_name(property);
    let aliases: Option<&[&str]> = match action.context.collection.as_deref() {
        Some("vdoVolumes") => Some(match normalized.as_str() {
            "writepolicy" | "write-policy" | "vdo-write-policy" => {
                &["vdo.write-policy", "lvm.vdo-write-policy", property]
            }
            "compression" | "vdo-compression" => {
                &["vdo.compression", "lvm.vdo-compression", property]
            }
            "deduplication" | "dedupe" | "vdo-deduplication" | "vdo-dedupe" => &[
                "vdo.deduplication",
                "vdo.dedupe",
                "lvm.vdo-deduplication",
                "lvm.vdo-dedupe",
                property,
            ],
            _ => &[property],
        }),
        Some("lvmCaches") => Some(match normalized.as_str() {
            "cachemode" | "cache-mode" | "lvm-cache-mode" => {
                &["lvm.cache-mode", "lvm.cacheMode", property]
            }
            "cachepolicy" | "cache-policy" | "lvm-cache-policy" => {
                &["lvm.cache-policy", "lvm.cachePolicy", property]
            }
            _ => &[property],
        }),
        Some("caches") => Some(match normalized.as_str() {
            "cachemode" | "cache-mode" | "bcache-cache-mode" => {
                &["bcache.cache-mode", "bcache.cacheMode", property]
            }
            "cachepolicy" | "cache-policy" | "bcache-cache-policy" => {
                &["bcache.cache-policy", "bcache.cachePolicy", property]
            }
            _ => &[property],
        }),
        Some("pools") => Some(match normalized.as_str() {
            "altroot" => &["zfs.pool-altroot", "zfs.altroot", property],
            "ashift" => &["zfs.pool-ashift", "zfs.ashift", property],
            "autotrim" | "auto-trim" => &["zfs.pool-autotrim", "zfs.autotrim", property],
            "autoexpand" | "auto-expand" => &["zfs.pool-autoexpand", "zfs.autoexpand", property],
            "autoreplace" | "auto-replace" => {
                &["zfs.pool-autoreplace", "zfs.autoreplace", property]
            }
            "bootfs" | "boot-fs" => &["zfs.pool-bootfs", "zfs.bootfs", property],
            "cachefile" | "cache-file" => &["zfs.pool-cachefile", "zfs.cachefile", property],
            "comment" => &["zfs.pool-comment", "zfs.comment", property],
            "delegation" => &["zfs.pool-delegation", "zfs.delegation", property],
            "failmode" | "fail-mode" => &["zfs.pool-failmode", "zfs.failmode", property],
            "listsnapshots" | "list-snapshots" => {
                &["zfs.pool-listsnapshots", "zfs.listsnapshots", property]
            }
            "multihost" | "multi-host" => &["zfs.pool-multihost", "zfs.multihost", property],
            _ => &[property],
        }),
        Some("datasets" | "zvols") => Some(match normalized.as_str() {
            "mountpoint" => &["zfs.mountpoint", property],
            "compression" => &["zfs.compression", property],
            "quota" => &["zfs.quota", property],
            "reservation" => &["zfs.reservation", property],
            "encryption" => &["zfs.encryption", property],
            "keystatus" | "key-status" => &["zfs.keystatus", property],
            "volsize" | "vol-size" => &["zfs.volsize", property],
            "recordsize" | "record-size" => &["zfs.recordsize", property],
            "dedup" => &["zfs.dedup", property],
            "checksum" => &["zfs.checksum", property],
            "copies" => &["zfs.copies", property],
            "sync" => &["zfs.sync", property],
            "primarycache" | "primary-cache" => &["zfs.primarycache", property],
            "secondarycache" | "secondary-cache" => &["zfs.secondarycache", property],
            "atime" => &["zfs.atime", property],
            "relatime" => &["zfs.relatime", property],
            "snapdir" | "snap-dir" => &["zfs.snapdir", property],
            "acltype" | "acl-type" => &["zfs.acltype", property],
            "xattr" => &["zfs.xattr", property],
            _ => &[property],
        }),
        _ => None,
    };

    if action.context.collection.as_deref() == Some("filesystems") {
        return current_filesystem_property_value(action, node, property)
            .or_else(|| property_value_from_node(node, property).map(str::to_string));
    }

    if action.context.collection.as_deref() == Some("swaps") {
        return current_swap_property_value(property, node)
            .or_else(|| property_value_from_node(node, property).map(str::to_string));
    }

    if action.context.collection.as_deref() == Some("zram") {
        return current_zram_property_value(property, node)
            .or_else(|| property_value_from_node(node, property).map(str::to_string));
    }

    if action.context.collection.as_deref() == Some("luksKeyslots") {
        return current_luks_keyslot_property_value(action, node, property)
            .or_else(|| property_value_from_node(node, property).map(str::to_string));
    }

    if action.context.collection.as_deref() == Some("luks.devices") {
        return current_luks_property_value(property, node)
            .or_else(|| property_value_from_node(node, property).map(str::to_string));
    }

    if action.context.collection.as_deref() == Some("btrfsQgroups") {
        return current_btrfs_qgroup_property_value(property, node)
            .or_else(|| property_value_from_node(node, property).map(str::to_string));
    }

    if action.context.collection.as_deref() == Some("caches") {
        if let Some(alias) = bcache_cache_set_property_key(property) {
            return property_value_from_node(node, &alias)
                .or_else(|| property_value_from_node(node, property))
                .map(str::to_string);
        }
    }

    if let Some(aliases) = aliases {
        return aliases
            .iter()
            .find_map(|alias| property_value_from_node(node, alias).map(str::to_string));
    }

    property_value_from_node(node, property).map(str::to_string)
}

fn comparable_property_value(action: &PlannedAction, property: &str, value: &str) -> String {
    let normalized_property = normalize_storage_property_name(property);
    let normalized_value = normalize_storage_property_name(value);
    match action.context.collection.as_deref() {
        Some("vdoVolumes") => match normalized_property.as_str() {
            "compression" | "vdo-compression" | "deduplication" | "dedupe"
            | "vdo-deduplication" | "vdo-dedupe" => {
                normalize_vdo_boolean_property_value(&normalized_value)
                    .map(str::to_string)
                    .unwrap_or(normalized_value)
            }
            "writepolicy" | "write-policy" | "vdo-write-policy" => normalized_value,
            _ => value.to_string(),
        },
        Some("lvmCaches" | "caches") => match normalized_property.as_str() {
            "cachemode"
            | "cache-mode"
            | "lvm-cache-mode"
            | "bcache-cache-mode"
            | "cachepolicy"
            | "cache-policy"
            | "lvm-cache-policy"
            | "bcache-cache-policy" => {
                normalize_cache_property_value(&normalized_property, &normalized_value)
            }
            _ => value.to_string(),
        },
        Some("pools") => {
            normalize_zfs_pool_property_value(&normalized_property, &normalized_value, value)
        }
        Some("datasets" | "zvols") => {
            normalize_zfs_property_value(&normalized_property, &normalized_value, value)
        }
        Some("filesystems") => {
            normalize_filesystem_property_value(action, &normalized_property, value)
                .unwrap_or_else(|| value.to_string())
        }
        Some("swaps") => normalize_swap_property_value(&normalized_property, value)
            .unwrap_or_else(|| value.to_string()),
        Some("zram") => normalize_zram_property_value(&normalized_property, value)
            .unwrap_or_else(|| value.to_string()),
        Some("luks.devices") => normalize_luks_property_value(&normalized_property, value)
            .unwrap_or_else(|| value.to_string()),
        Some("luksKeyslots") => normalize_luks_keyslot_property_value(&normalized_property, value)
            .unwrap_or_else(|| value.to_string()),
        Some("btrfsQgroups") => normalize_btrfs_qgroup_property_value(&normalized_property, value)
            .unwrap_or_else(|| value.to_string()),
        _ => value.to_string(),
    }
}

fn current_luks_keyslot_property_value(
    action: &PlannedAction,
    node: &Node,
    property: &str,
) -> Option<String> {
    match luks_keyslot_property_kind(property)? {
        LuksKeyslotPropertyKind::Priority => {
            let key_slot = action.context.key_slot.as_deref().or_else(|| {
                action
                    .context
                    .name
                    .as_deref()
                    .and_then(|name| name.rsplit_once(':').map(|(_, slot)| slot).or(Some(name)))
                    .filter(|slot| slot.chars().all(|character| character.is_ascii_digit()))
            })?;
            property_value_from_node(
                node,
                &format!("cryptsetup.luks-keyslot-{key_slot}-priority"),
            )
            .or_else(|| property_value_from_node(node, property))
            .map(str::to_string)
        }
        LuksKeyslotPropertyKind::KeyFile => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LuksKeyslotPropertyKind {
    KeyFile,
    Priority,
}

fn luks_keyslot_property_kind(property: &str) -> Option<LuksKeyslotPropertyKind> {
    match normalize_storage_property_name(property).as_str() {
        "keyfile"
        | "key-file"
        | "luks-keyfile"
        | "luks-key-file"
        | "cryptsetup-keyfile"
        | "cryptsetup-key-file" => Some(LuksKeyslotPropertyKind::KeyFile),
        "priority" | "luks-keyslot-priority" | "cryptsetup-luks-keyslot-priority" => {
            Some(LuksKeyslotPropertyKind::Priority)
        }
        _ => None,
    }
}

fn normalize_luks_keyslot_property_value(property: &str, value: &str) -> Option<String> {
    match luks_keyslot_property_kind(property)? {
        LuksKeyslotPropertyKind::KeyFile => Some(value.to_string()),
        LuksKeyslotPropertyKind::Priority => Some(normalize_storage_property_name(value)),
    }
}

fn current_btrfs_qgroup_property_value(property: &str, node: &Node) -> Option<String> {
    match btrfs_qgroup_property_kind(property)? {
        BtrfsQgroupPropertyKind::MaxReferenced => {
            property_value_from_node(node, "btrfs.max-referenced")
                .or_else(|| property_value_from_node(node, "btrfs.referenced-limit"))
                .map(str::to_string)
        }
        BtrfsQgroupPropertyKind::MaxExclusive => {
            property_value_from_node(node, "btrfs.max-exclusive")
                .or_else(|| property_value_from_node(node, "btrfs.exclusive-limit"))
                .map(str::to_string)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BtrfsQgroupPropertyKind {
    MaxReferenced,
    MaxExclusive,
}

fn btrfs_qgroup_property_kind(property: &str) -> Option<BtrfsQgroupPropertyKind> {
    match normalize_storage_property_name(property).as_str() {
        "limit"
        | "referenced"
        | "maxreferenced"
        | "max-referenced"
        | "btrfs-max-referenced"
        | "btrfs-referenced-limit" => Some(BtrfsQgroupPropertyKind::MaxReferenced),
        "exclusive"
        | "maxexclusive"
        | "max-exclusive"
        | "btrfs-max-exclusive"
        | "btrfs-exclusive-limit" => Some(BtrfsQgroupPropertyKind::MaxExclusive),
        _ => None,
    }
}

fn normalize_btrfs_qgroup_property_value(property: &str, value: &str) -> Option<String> {
    match btrfs_qgroup_property_kind(property)? {
        BtrfsQgroupPropertyKind::MaxReferenced | BtrfsQgroupPropertyKind::MaxExclusive => {
            let trimmed = value.trim();
            if matches!(
                normalize_storage_property_name(trimmed).as_str(),
                "none" | "null" | "unlimited" | "---"
            ) {
                Some("none".to_string())
            } else {
                Some(trimmed.to_string())
            }
        }
    }
}

fn current_luks_property_value(property: &str, node: &Node) -> Option<String> {
    match luks_property_kind(property)? {
        LuksPropertyKind::Label => node.identity.label.clone().or_else(|| {
            property_value_from_node(node, "cryptsetup.label")
                .or_else(|| property_value_from_node(node, "cryptsetup.luks-label"))
                .map(str::to_string)
        }),
        LuksPropertyKind::Subsystem => property_value_from_node(node, "cryptsetup.luks-subsystem")
            .or_else(|| property_value_from_node(node, "cryptsetup.subsystem"))
            .map(str::to_string),
        LuksPropertyKind::Uuid => node.identity.uuid.clone().or_else(|| {
            property_value_from_node(node, "cryptsetup.uuid")
                .or_else(|| property_value_from_node(node, "cryptsetup.luks-uuid"))
                .map(str::to_string)
        }),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LuksPropertyKind {
    Label,
    Subsystem,
    Uuid,
}

fn luks_property_kind(property: &str) -> Option<LuksPropertyKind> {
    match normalize_storage_property_name(property).as_str() {
        "label" | "luks-label" | "cryptsetup-label" => Some(LuksPropertyKind::Label),
        "subsystem" | "luks-subsystem" | "cryptsetup-subsystem" => {
            Some(LuksPropertyKind::Subsystem)
        }
        "uuid" | "luks-uuid" | "cryptsetup-uuid" => Some(LuksPropertyKind::Uuid),
        _ => None,
    }
}

fn normalize_luks_property_value(property: &str, value: &str) -> Option<String> {
    match luks_property_kind(property)? {
        LuksPropertyKind::Label | LuksPropertyKind::Subsystem => Some(value.to_string()),
        LuksPropertyKind::Uuid => Some(value.trim().to_ascii_lowercase()),
    }
}

fn current_swap_property_value(property: &str, node: &Node) -> Option<String> {
    match swap_property_kind(property)? {
        SwapPropertyKind::Label => node.identity.label.clone().or_else(|| {
            property_value_from_node(node, "swap.label")
                .or_else(|| property_value_from_node(node, "udev.id-fs-label"))
                .or_else(|| property_value_from_node(node, "udev.id-fs-label-safe"))
                .map(str::to_string)
        }),
        SwapPropertyKind::Uuid => node.identity.uuid.clone().or_else(|| {
            property_value_from_node(node, "swap.uuid")
                .or_else(|| property_value_from_node(node, "udev.id-fs-uuid"))
                .map(str::to_string)
        }),
        SwapPropertyKind::Priority => {
            property_value_from_node(node, "swap.priority").and_then(normalize_swap_priority)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SwapPropertyKind {
    Label,
    Uuid,
    Priority,
}

fn swap_property_kind(property: &str) -> Option<SwapPropertyKind> {
    match normalize_storage_property_name(property).as_str() {
        "label" | "swap-label" => Some(SwapPropertyKind::Label),
        "uuid" | "swap-uuid" => Some(SwapPropertyKind::Uuid),
        "priority" | "swap-priority" => Some(SwapPropertyKind::Priority),
        _ => None,
    }
}

fn normalize_swap_property_value(property: &str, value: &str) -> Option<String> {
    match swap_property_kind(property)? {
        SwapPropertyKind::Label => Some(value.to_string()),
        SwapPropertyKind::Uuid => Some(value.trim().to_ascii_lowercase()),
        SwapPropertyKind::Priority => normalize_swap_priority(value),
    }
}

fn normalize_swap_priority(value: &str) -> Option<String> {
    value
        .trim()
        .parse::<i32>()
        .ok()
        .map(|priority| priority.to_string())
}

fn current_zram_property_value(property: &str, node: &Node) -> Option<String> {
    match zram_property_kind(property)? {
        ZramPropertyKind::Algorithm => property_value_from_node(node, "zram.algorithm")
            .or_else(|| property_value_from_node(node, "zram.compression-algorithm"))
            .map(str::to_string),
        ZramPropertyKind::Streams => {
            property_value_from_node(node, "zram.streams").and_then(normalize_integer_property)
        }
        ZramPropertyKind::DiskSize => property_value_from_node(node, "zram.disksize")
            .or_else(|| property_value_from_node(node, "zram.disk-size"))
            .and_then(normalize_integer_property),
        ZramPropertyKind::MemoryLimit => {
            property_value_from_node(node, "zram.memory-limit").and_then(normalize_integer_property)
        }
        ZramPropertyKind::CompressionRatio => {
            property_value_from_node(node, "zram.compression-ratio")
                .or_else(|| property_value_from_node(node, "zram.ratio"))
                .map(normalize_decimal_property)
        }
        ZramPropertyKind::Priority => {
            property_value_from_node(node, "swap.priority").and_then(normalize_swap_priority)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ZramPropertyKind {
    Algorithm,
    Streams,
    DiskSize,
    MemoryLimit,
    CompressionRatio,
    Priority,
}

fn zram_property_kind(property: &str) -> Option<ZramPropertyKind> {
    match normalize_storage_property_name(property).as_str() {
        "algorithm" | "zram-algorithm" | "compression-algorithm" => {
            Some(ZramPropertyKind::Algorithm)
        }
        "streams" | "zram-streams" => Some(ZramPropertyKind::Streams),
        "disksize" | "disk-size" | "zram-disksize" | "zram-disk-size" => {
            Some(ZramPropertyKind::DiskSize)
        }
        "memorylimit" | "memory-limit" | "zram-memory-limit" => Some(ZramPropertyKind::MemoryLimit),
        "compressionratio"
        | "compression-ratio"
        | "compression-ratio-target"
        | "zram-compression-ratio"
        | "zram-compression-ratio-target"
        | "ratio"
        | "zram-ratio" => Some(ZramPropertyKind::CompressionRatio),
        "priority" | "zram-priority" | "swap-priority" => Some(ZramPropertyKind::Priority),
        _ => None,
    }
}

fn normalize_zram_property_value(property: &str, value: &str) -> Option<String> {
    match zram_property_kind(property)? {
        ZramPropertyKind::Algorithm => Some(normalize_storage_property_name(value)),
        ZramPropertyKind::Streams | ZramPropertyKind::DiskSize | ZramPropertyKind::MemoryLimit => {
            normalize_integer_property(value)
        }
        ZramPropertyKind::CompressionRatio => Some(normalize_decimal_property(value)),
        ZramPropertyKind::Priority => normalize_swap_priority(value),
    }
}

fn normalize_integer_property(value: &str) -> Option<String> {
    value
        .trim()
        .parse::<u64>()
        .ok()
        .map(|number| number.to_string())
}

fn normalize_decimal_property(value: &str) -> String {
    value
        .trim()
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FilesystemPropertyKind {
    Label,
    Uuid,
    FatVolumeId,
    NtfsVolumeSerial,
    ExfatVolumeSerial,
}

fn current_filesystem_property_value(
    action: &PlannedAction,
    node: &Node,
    property: &str,
) -> Option<String> {
    match filesystem_property_kind(action, property)? {
        FilesystemPropertyKind::Label => node.identity.label.clone().or_else(|| {
            property_value_from_node(node, "filesystem.label")
                .or_else(|| property_value_from_node(node, "udev.id-fs-label"))
                .or_else(|| property_value_from_node(node, "udev.id-fs-label-safe"))
                .or_else(|| property_value_from_node(node, "ntfs.volume-name"))
                .map(str::to_string)
        }),
        FilesystemPropertyKind::Uuid => node.identity.uuid.clone().or_else(|| {
            property_value_from_node(node, "filesystem.uuid")
                .or_else(|| property_value_from_node(node, "udev.id-fs-uuid"))
                .map(str::to_string)
        }),
        FilesystemPropertyKind::FatVolumeId => node.identity.uuid.clone().or_else(|| {
            property_value_from_node(node, "filesystem.uuid")
                .or_else(|| property_value_from_node(node, "udev.id-fs-uuid"))
                .map(str::to_string)
        }),
        FilesystemPropertyKind::NtfsVolumeSerial => node
            .identity
            .serial
            .clone()
            .or_else(|| node.identity.uuid.clone())
            .or_else(|| property_value_from_node(node, "ntfs.volume-serial").map(str::to_string)),
        FilesystemPropertyKind::ExfatVolumeSerial => node
            .identity
            .serial
            .clone()
            .or_else(|| node.identity.uuid.clone())
            .or_else(|| property_value_from_node(node, "exfat.volume-serial").map(str::to_string)),
    }
}

fn filesystem_property_kind(
    action: &PlannedAction,
    property: &str,
) -> Option<FilesystemPropertyKind> {
    let normalized = normalize_storage_property_name(property);
    let fs_type = action
        .context
        .fs_type
        .as_deref()
        .map(|value| value.to_ascii_lowercase());
    let fs_type = fs_type.as_deref();

    if matches!(
        normalized.as_str(),
        "label"
            | "filesystem-label"
            | "btrfs-label"
            | "ext-label"
            | "fat-label"
            | "vfat-label"
            | "ntfs-label"
            | "exfat-label"
            | "f2fs-label"
            | "xfs-label"
    ) {
        return Some(FilesystemPropertyKind::Label);
    }

    if matches!(normalized.as_str(), "serial" | "volume-serial") {
        return match fs_type {
            Some("exfat") => Some(FilesystemPropertyKind::ExfatVolumeSerial),
            _ => Some(FilesystemPropertyKind::NtfsVolumeSerial),
        };
    }

    if matches!(normalized.as_str(), "ntfs-serial" | "ntfs-volume-serial") {
        return Some(FilesystemPropertyKind::NtfsVolumeSerial);
    }

    if matches!(normalized.as_str(), "exfat-serial" | "exfat-volume-serial") {
        return Some(FilesystemPropertyKind::ExfatVolumeSerial);
    }

    if matches!(
        normalized.as_str(),
        "volume-id" | "fat-volume-id" | "vfat-volume-id"
    ) {
        return Some(FilesystemPropertyKind::FatVolumeId);
    }

    if matches!(
        normalized.as_str(),
        "uuid"
            | "filesystem-uuid"
            | "btrfs-uuid"
            | "ext-uuid"
            | "fat-uuid"
            | "vfat-uuid"
            | "ntfs-uuid"
            | "exfat-uuid"
            | "xfs-uuid"
    ) {
        return match fs_type {
            Some("fat" | "fat12" | "fat16" | "fat32" | "msdos" | "vfat") => {
                Some(FilesystemPropertyKind::FatVolumeId)
            }
            Some("ntfs" | "ntfs3") => Some(FilesystemPropertyKind::NtfsVolumeSerial),
            Some("exfat") => Some(FilesystemPropertyKind::ExfatVolumeSerial),
            _ => Some(FilesystemPropertyKind::Uuid),
        };
    }

    None
}

fn normalize_filesystem_property_value(
    action: &PlannedAction,
    normalized_property: &str,
    raw_value: &str,
) -> Option<String> {
    match filesystem_property_kind(action, normalized_property)? {
        FilesystemPropertyKind::Label => Some(raw_value.to_string()),
        FilesystemPropertyKind::Uuid => Some(raw_value.trim().to_ascii_lowercase()),
        FilesystemPropertyKind::FatVolumeId => normalize_hex_identity(raw_value, 8),
        FilesystemPropertyKind::NtfsVolumeSerial => normalize_hex_identity(raw_value, 16),
        FilesystemPropertyKind::ExfatVolumeSerial => normalize_hex_identity(raw_value, 8),
    }
}

fn normalize_hex_identity(value: &str, expected_len: usize) -> Option<String> {
    let trimmed = value.trim();
    let without_prefix = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    let normalized: String = without_prefix
        .chars()
        .filter(|character| *character != '-')
        .collect();
    if normalized.len() == expected_len
        && normalized
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        Some(normalized.to_ascii_uppercase())
    } else {
        Some(normalize_storage_property_name(value))
    }
}

fn normalize_cache_property_value(property: &str, value: &str) -> String {
    match property {
        "cachemode" | "cache-mode" | "lvm-cache-mode" | "bcache-cache-mode" => {
            value.replace('-', "")
        }
        _ => value.to_string(),
    }
}

fn normalize_zfs_pool_property_value(
    property: &str,
    normalized_value: &str,
    raw_value: &str,
) -> String {
    match property {
        "autotrim" | "auto-trim" | "autoexpand" | "auto-expand" | "autoreplace"
        | "auto-replace" | "delegation" | "listsnapshots" | "list-snapshots" | "multihost"
        | "multi-host" => normalize_zfs_boolean_property_value(normalized_value)
            .map(str::to_string)
            .unwrap_or_else(|| normalized_value.to_string()),
        "altroot" | "ashift" | "bootfs" | "boot-fs" | "cachefile" | "cache-file" | "comment"
        | "failmode" | "fail-mode" => normalized_value.to_string(),
        _ => raw_value.to_string(),
    }
}

fn normalize_zfs_property_value(property: &str, normalized_value: &str, raw_value: &str) -> String {
    match property {
        "dedup" | "atime" | "relatime" => normalize_zfs_boolean_property_value(normalized_value)
            .map(str::to_string)
            .unwrap_or_else(|| normalized_value.to_string()),
        "primarycache" | "primary-cache" | "secondarycache" | "secondary-cache" => {
            normalized_value.to_string()
        }
        "mountpoint" | "compression" | "quota" | "reservation" | "encryption" | "keystatus"
        | "key-status" | "volsize" | "vol-size" | "recordsize" | "record-size" | "checksum"
        | "copies" | "sync" | "snapdir" | "snap-dir" | "acltype" | "acl-type" | "xattr" => {
            normalized_value.to_string()
        }
        _ => raw_value.to_string(),
    }
}

fn normalize_zfs_boolean_property_value(value: &str) -> Option<&'static str> {
    match value {
        "on" | "yes" | "true" | "enabled" | "enable" | "1" => Some("on"),
        "off" | "no" | "false" | "disabled" | "disable" | "0" => Some("off"),
        _ => None,
    }
}

fn normalize_vdo_boolean_property_value(value: &str) -> Option<&'static str> {
    match value {
        "enabled" | "enable" | "true" | "yes" | "on" | "1" => Some("enabled"),
        "disabled" | "disable" | "false" | "no" | "off" | "0" => Some("disabled"),
        _ => None,
    }
}
