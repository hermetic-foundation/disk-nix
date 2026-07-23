fn print_devices(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>12} {:<36} PATH",
        "KIND", "NAME", "SIZE", "DETAILS"
    )?;
    for node in graph.nodes.iter().filter(|node| is_device_node(node)) {
        writeln!(
            output,
            "{:<22} {:<38} {:>12} {:<36} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            usage_details(node),
            node.path.as_deref().unwrap_or("-")
        )?;
    }
    Ok(())
}

fn print_partitions(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>12} {:<24} {:<36} PATH",
        "KIND", "NAME", "SIZE", "PARTUUID", "DETAILS"
    )?;
    for node in graph.nodes.iter().filter(|node| is_partition_node(node)) {
        writeln!(
            output,
            "{:<22} {:<38} {:>12} {:<24} {:<36} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            node.identity.partuuid.as_deref().unwrap_or("-"),
            usage_details(node),
            node.path.as_deref().unwrap_or("-")
        )?;
    }
    Ok(())
}

fn print_filesystems(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<32} {:>12} {:>12} {:<24} DETAILS",
        "KIND", "NAME", "USED", "FREE", "UUID"
    )?;
    for node in graph.nodes.iter().filter(|node| is_filesystem_node(node)) {
        writeln!(
            output,
            "{:<22} {:<32} {:>12} {:>12} {:<24} {}",
            node.kind,
            node.name,
            human_bytes(node.usage.as_ref().and_then(|usage| usage.used_bytes)),
            human_bytes(node.usage.as_ref().and_then(|usage| usage.free_bytes)),
            node.identity.uuid.as_deref().unwrap_or("-"),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_complex_filesystems(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>12} {:>12} {:>12} {:>7} {:>7} DETAILS",
        "KIND", "NAME", "SIZE", "USED", "FREE", "USE%", "BACKING"
    )?;
    for node in graph
        .nodes
        .iter()
        .filter(|node| is_complex_filesystem_node(node))
    {
        let usage = node.usage.as_ref();
        writeln!(
            output,
            "{:<22} {:<38} {:>12} {:>12} {:>12} {:>7} {:>7} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            human_bytes(usage.and_then(|usage| usage.used_bytes)),
            human_bytes(usage.and_then(|usage| usage.free_bytes)),
            usage_percent(node),
            backing_count(graph, node),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_btrfs(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>12} {:>12} {:>12} {:>7} {:<18} {:>7} DETAILS",
        "KIND", "NAME", "SIZE", "USED", "FREE", "USE%", "MOUNT", "BACKING"
    )?;
    for node in graph.nodes.iter().filter(|node| is_btrfs_node(node)) {
        let usage = node.usage.as_ref();
        writeln!(
            output,
            "{:<22} {:<38} {:>12} {:>12} {:>12} {:>7} {:<18} {:>7} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            human_bytes(usage.and_then(|usage| usage.used_bytes)),
            human_bytes(usage.and_then(|usage| usage.free_bytes)),
            usage_percent(node),
            property_value(node, "btrfs.mount-target")
                .or_else(|| property_value(node, "mountpoint"))
                .unwrap_or("-"),
            backing_count(graph, node),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_bcachefs(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>12} {:>12} {:>12} {:>7} {:<18} {:>7} DETAILS",
        "KIND", "NAME", "SIZE", "USED", "FREE", "USE%", "MOUNT", "MEMBERS"
    )?;
    for node in graph.nodes.iter().filter(|node| is_bcachefs_node(node)) {
        let usage = node.usage.as_ref();
        writeln!(
            output,
            "{:<22} {:<38} {:>12} {:>12} {:>12} {:>7} {:<18} {:>7} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            human_bytes(usage.and_then(|usage| usage.used_bytes)),
            human_bytes(usage.and_then(|usage| usage.free_bytes)),
            usage_percent(node),
            property_value(node, "bcachefs.mount-target").unwrap_or("-"),
            member_count(graph, node),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_zfs(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>12} {:>12} {:>12} {:<12} {:<24} {:>8} DETAILS",
        "KIND", "NAME", "SIZE", "USED", "FREE", "HEALTH", "ORIGIN", "CHILDREN"
    )?;
    for node in graph.nodes.iter().filter(|node| is_zfs_node(node)) {
        let usage = node.usage.as_ref();
        writeln!(
            output,
            "{:<22} {:<38} {:>12} {:>12} {:>12} {:<12} {:<24} {:>8} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            human_bytes(usage.and_then(|usage| usage.used_bytes)),
            human_bytes(usage.and_then(|usage| usage.free_bytes)),
            property_value(node, "zfs.health")
                .or_else(|| property_value(node, "zfs.state"))
                .or_else(|| property_value(node, "zfs.vdev-state"))
                .unwrap_or("-"),
            property_value(node, "zfs.origin").unwrap_or("-"),
            zfs_child_count(graph, node),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_volumes(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>12} {:>12} {:>12} DETAILS",
        "KIND", "NAME", "SIZE", "USED", "FREE"
    )?;
    for node in graph.nodes.iter().filter(|node| is_volume_node(node)) {
        writeln!(
            output,
            "{:<22} {:<38} {:>12} {:>12} {:>12} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            human_bytes(node.usage.as_ref().and_then(|usage| usage.used_bytes)),
            human_bytes(node.usage.as_ref().and_then(|usage| usage.free_bytes)),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_pools(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>12} {:>12} {:>12} {:>8} DETAILS",
        "KIND", "NAME", "SIZE", "USED", "FREE", "BACKING"
    )?;
    for node in graph.nodes.iter().filter(|node| is_pool_node(node)) {
        writeln!(
            output,
            "{:<22} {:<38} {:>12} {:>12} {:>12} {:>8} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            human_bytes(node.usage.as_ref().and_then(|usage| usage.used_bytes)),
            human_bytes(node.usage.as_ref().and_then(|usage| usage.free_bytes)),
            backing_count(graph, node),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_snapshots(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<48} {:>12} {:<32} DETAILS",
        "KIND", "NAME", "SIZE", "SOURCE"
    )?;
    for node in graph.nodes.iter().filter(|node| is_snapshot_node(node)) {
        writeln!(
            output,
            "{:<22} {:<48} {:>12} {:<32} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            snapshot_source(graph, node).unwrap_or("-"),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_mappings(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>8} {:<44} PATH",
        "KIND", "NAME", "BACKING", "DETAILS"
    )?;
    for node in graph.nodes.iter().filter(|node| is_mapping_node(node)) {
        writeln!(
            output,
            "{:<22} {:<38} {:>8} {:<44} {}",
            node.kind,
            node.name,
            backing_count(graph, node),
            usage_details(node),
            node.path.as_deref().unwrap_or("-")
        )?;
    }
    Ok(())
}

fn print_dm(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>8} {:<16} {:<16} {:<11} DETAILS",
        "KIND", "NAME", "BACKING", "TARGETS", "STATUS", "MAJOR:MINOR"
    )?;
    for node in graph.nodes.iter().filter(|node| is_dm_node(node)) {
        let major_minor = property_value(node, "dm.major")
            .zip(property_value(node, "dm.minor"))
            .map(|(major, minor)| format!("{major}:{minor}"))
            .unwrap_or_else(|| "-".to_string());
        writeln!(
            output,
            "{:<22} {:<38} {:>8} {:<16} {:<16} {:<11} {}",
            node.kind,
            node.name,
            backing_count(graph, node),
            property_value(node, "dm.table.targets").unwrap_or("-"),
            property_value(node, "dm.status.targets").unwrap_or("-"),
            major_minor,
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_encryption(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:<12} {:<10} {:<10} DETAILS",
        "KIND", "NAME", "CIPHER", "KEYSLOTS", "TOKENS"
    )?;
    for node in graph.nodes.iter().filter(|node| is_encryption_node(node)) {
        writeln!(
            output,
            "{:<22} {:<38} {:<12} {:<10} {:<10} {}",
            node.kind,
            node.name,
            property_value(node, "cryptsetup.cipher")
                .or_else(|| property_value(node, "cryptsetup.luks-data-cipher"))
                .unwrap_or("-"),
            property_value(node, "cryptsetup.luks-keyslot-count").unwrap_or("-"),
            property_value(node, "cryptsetup.luks-token-count").unwrap_or("-"),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_cache(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:<14} {:<14} {:<14} DETAILS",
        "KIND", "NAME", "MODE", "POLICY", "DIRTY"
    )?;
    for node in graph.nodes.iter().filter(|node| is_cache_node(node)) {
        writeln!(
            output,
            "{:<22} {:<38} {:<14} {:<14} {:<14} {}",
            node.kind,
            node.name,
            property_value(node, "bcache.cache-mode")
                .or_else(|| property_value(node, "lvm.cache-mode"))
                .or_else(|| property_value(node, "lvm.segment-cache-mode"))
                .unwrap_or("-"),
            property_value(node, "bcache.cache-replacement-policy")
                .or_else(|| property_value(node, "lvm.cache-policy"))
                .or_else(|| property_value(node, "lvm.segment-cache-policy"))
                .unwrap_or("-"),
            property_value(node, "bcache.dirty-data")
                .or_else(|| property_value(node, "lvm.writecache-writeback-blocks"))
                .unwrap_or("-"),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_lvm(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>12} {:<12} {:<12} {:<12} {:>7} DETAILS",
        "KIND", "NAME", "SIZE", "DATA%", "META%", "ACTIVE", "BACKING"
    )?;
    for node in graph.nodes.iter().filter(|node| is_lvm_node(node)) {
        writeln!(
            output,
            "{:<22} {:<38} {:>12} {:<12} {:<12} {:<12} {:>7} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            property_value(node, "lvm.data-percent").unwrap_or("-"),
            property_value(node, "lvm.metadata-percent").unwrap_or("-"),
            property_value(node, "lvm.active").unwrap_or("-"),
            backing_count(graph, node),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_vdo(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>12} {:>12} {:>12} {:>12} {:>7} {:<12} {:<12} DETAILS",
        "KIND", "NAME", "LOGICAL", "PHYSICAL", "USED", "FREE", "USE%", "MODE", "WRITE"
    )?;
    for node in graph.nodes.iter().filter(|node| is_vdo_node(node)) {
        let usage = node.usage.as_ref();
        writeln!(
            output,
            "{:<22} {:<38} {:>12} {:>12} {:>12} {:>12} {:>7} {:<12} {:<12} {}",
            node.kind,
            node.name,
            vdo_logical_display(node),
            vdo_physical_display(node),
            human_bytes(usage.and_then(|usage| usage.used_bytes)),
            human_bytes(usage.and_then(|usage| usage.free_bytes)),
            usage_percent(node),
            property_value(node, "vdo.operating-mode")
                .or_else(|| property_value(node, "lvm.vdo-operating-mode"))
                .unwrap_or("-"),
            property_value(node, "vdo.write-policy")
                .or_else(|| property_value(node, "lvm.vdo-write-policy"))
                .unwrap_or("-"),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_multipath(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<32} {:<28} {:>5} {:<12} {:<20} DETAILS",
        "KIND", "NAME", "WWID", "PATHS", "GROUP", "PATH-STATE"
    )?;
    for node in graph.nodes.iter().filter(|node| is_multipath_node(node)) {
        writeln!(
            output,
            "{:<22} {:<32} {:<28} {:>5} {:<12} {:<20} {}",
            node.kind,
            node.name,
            property_value(node, "multipath.wwid").unwrap_or("-"),
            backing_count(graph, node),
            property_value(node, "multipath.group-status").unwrap_or("-"),
            property_value(node, "multipath.path-state").unwrap_or("-"),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_nvme(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<24} {:>12} {:>12} {:>7} {:<20} {:<18} DETAILS",
        "KIND", "NAME", "SIZE", "USED", "USE%", "SERIAL", "CONTROLLER"
    )?;
    for node in graph.nodes.iter().filter(|node| is_nvme_node(node)) {
        let usage = node.usage.as_ref();
        writeln!(
            output,
            "{:<22} {:<24} {:>12} {:>12} {:>7} {:<20} {:<18} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            human_bytes(usage.and_then(|usage| usage.used_bytes)),
            usage_percent(node),
            node.identity.serial.as_deref().unwrap_or("-"),
            property_value(node, "nvme.controller").unwrap_or("-"),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_raid(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<32} {:>12} {:<10} {:<14} {:>6} {:>6} {:>6} {:>7} DETAILS",
        "KIND", "NAME", "SIZE", "LEVEL", "STATE", "ACTIVE", "FAILED", "SPARE", "MEMBERS"
    )?;
    for node in graph.nodes.iter().filter(|node| is_raid_node(node)) {
        writeln!(
            output,
            "{:<22} {:<32} {:>12} {:<10} {:<14} {:>6} {:>6} {:>6} {:>7} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            property_value(node, "md.level").unwrap_or("-"),
            property_value(node, "md.state")
                .or_else(|| property_value(node, "md.member-state"))
                .unwrap_or("-"),
            property_value(node, "md.active-devices").unwrap_or("-"),
            property_value(node, "md.failed-devices").unwrap_or("-"),
            property_value(node, "md.spare-devices").unwrap_or("-"),
            member_count(graph, node),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_loop(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<28} {:>12} {:<32} {:<10} {:<8} DETAILS",
        "KIND", "NAME", "SIZE", "BACKING", "OFFSET", "RO"
    )?;
    for node in graph.nodes.iter().filter(|node| is_loop_node(node)) {
        writeln!(
            output,
            "{:<22} {:<28} {:>12} {:<32} {:<10} {:<8} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            property_value(node, "loop.back-file").unwrap_or("-"),
            property_value(node, "loop.offset").unwrap_or("-"),
            property_value(node, "loop.read-only").unwrap_or("-"),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_backing_files(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<44} {:>12} {:>9} {:>7} DETAILS",
        "KIND", "PATH", "SIZE", "CONSUMERS", "USE%"
    )?;
    for node in graph.nodes.iter().filter(|node| is_backing_file_node(node)) {
        writeln!(
            output,
            "{:<22} {:<44} {:>12} {:>9} {:>7} {}",
            node.kind,
            node.path.as_deref().unwrap_or(&node.name),
            human_bytes(node.size_bytes),
            consumer_count(graph, node),
            usage_percent(node),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_swap(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<32} {:>12} {:>12} {:>12} {:>7} {:<10} {:<8} DETAILS",
        "KIND", "NAME", "SIZE", "USED", "FREE", "USE%", "TYPE", "PRIO"
    )?;
    for node in graph.nodes.iter().filter(|node| is_swap_node(node)) {
        let usage = node.usage.as_ref();
        writeln!(
            output,
            "{:<22} {:<32} {:>12} {:>12} {:>12} {:>7} {:<10} {:<8} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            human_bytes(usage.and_then(|usage| usage.used_bytes)),
            human_bytes(usage.and_then(|usage| usage.free_bytes)),
            usage_percent(node),
            property_value(node, "swap.type").unwrap_or("-"),
            property_value(node, "swap.priority").unwrap_or("-"),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_zram(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<32} {:>12} {:>12} {:>12} {:>12} {:<10} {:<8} {:>12} {:<12} DETAILS",
        "KIND", "NAME", "SIZE", "USED", "FREE", "ALLOC", "ALGO", "RATIO", "MEM-PEAK", "MOUNT"
    )?;
    for node in graph.nodes.iter().filter(|node| is_zram_node(node)) {
        let usage = node.usage.as_ref();
        writeln!(
            output,
            "{:<22} {:<32} {:>12} {:>12} {:>12} {:>12} {:<10} {:<8} {:>12} {:<12} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            human_bytes(usage.and_then(|usage| usage.used_bytes)),
            human_bytes(usage.and_then(|usage| usage.free_bytes)),
            human_bytes(usage.and_then(|usage| usage.allocated_bytes)),
            property_value(node, "zram.algorithm").unwrap_or("-"),
            property_value(node, "zram.compression-ratio").unwrap_or("-"),
            property_value(node, "zram.memory-peak")
                .or_else(|| property_value(node, "zram.memory-used"))
                .unwrap_or("-"),
            property_value(node, "zram.mountpoint").unwrap_or("-"),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_iscsi(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<48} {:>12} {:<22} {:<14} {:>5} {:<18} DETAILS",
        "KIND", "NAME", "SIZE", "PORTAL", "STATE", "LUNS", "PATH"
    )?;
    for node in graph.nodes.iter().filter(|node| is_iscsi_node(node)) {
        writeln!(
            output,
            "{:<22} {:<48} {:>12} {:<22} {:<14} {:>5} {:<18} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            property_value(node, "iscsi.portal")
                .or_else(|| property_value(node, "iscsi.node-portal"))
                .or_else(|| property_value(node, "iscsi.persistent-portal"))
                .or_else(|| property_value(node, "iscsi.node-persistent-portal"))
                .unwrap_or("-"),
            property_value(node, "iscsi.connection-state").unwrap_or("-"),
            iscsi_lun_count(graph, node),
            node.path.as_deref().unwrap_or("-"),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_luns(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<40} {:>12} {:<18} {:<10} {:<18} DETAILS",
        "KIND", "NAME", "SIZE", "PATH", "TRANSPORT", "GENERIC"
    )?;
    for node in graph.nodes.iter().filter(|node| is_lun_node(node)) {
        writeln!(
            output,
            "{:<22} {:<40} {:>12} {:<18} {:<10} {:<18} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            node.path
                .as_deref()
                .or_else(|| property_value(node, "scsi.block-device"))
                .or_else(|| property_value(node, "iscsi.attached-disk"))
                .unwrap_or("-"),
            property_value(node, "scsi.transport").unwrap_or("-"),
            property_value(node, "scsi.generic-device").unwrap_or("-"),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_nfs(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<40} {:<34} {:<20} {:<22} {:>6} DETAILS",
        "KIND", "NAME", "SOURCE", "SERVER", "EXPORT", "MOUNTS"
    )?;
    for node in graph.nodes.iter().filter(|node| is_nfs_node(node)) {
        writeln!(
            output,
            "{:<22} {:<40} {:<34} {:<20} {:<22} {:>6} {}",
            node.kind,
            node.name,
            property_value(node, "nfs.source").unwrap_or("-"),
            property_value(node, "nfs.server").unwrap_or("-"),
            property_value(node, "nfs.export").unwrap_or("-"),
            nfs_mount_count(graph, node),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_mounts(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<48} {:<12} DETAILS",
        "KIND", "TARGET", "FSTYPE"
    )?;
    for node in graph.nodes.iter().filter(|node| is_mount_node(node)) {
        writeln!(
            output,
            "{:<22} {:<48} {:<12} {}",
            node.kind,
            node.name,
            property_value(node, "filesystem.type").unwrap_or("-"),
            mount_details(node)
        )?;
    }
    Ok(())
}

fn print_network_storage(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<48} {:>12} {:<36} PATH",
        "KIND", "NAME", "SIZE", "DETAILS"
    )?;
    for node in graph
        .nodes
        .iter()
        .filter(|node| is_network_storage_node(node))
    {
        writeln!(
            output,
            "{:<22} {:<48} {:>12} {:<36} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            usage_details(node),
            node.path.as_deref().unwrap_or("-")
        )?;
    }
    Ok(())
}

fn print_ids(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:<24} {:<24} {:<20} {:<20}",
        "KIND", "NAME", "UUID", "PARTUUID", "LABEL", "SERIAL/WWN"
    )?;
    for node in graph.nodes.iter().filter(|node| has_identity(node)) {
        let hardware_id = node
            .identity
            .serial
            .as_deref()
            .or(node.identity.wwn.as_deref())
            .unwrap_or("-");

        writeln!(
            output,
            "{:<22} {:<38} {:<24} {:<24} {:<20} {:<20}",
            node.kind,
            node.name,
            node.identity.uuid.as_deref().unwrap_or("-"),
            node.identity.partuuid.as_deref().unwrap_or("-"),
            node.identity.label.as_deref().unwrap_or("-"),
            hardware_id
        )?;
    }
    Ok(())
}

fn print_usage(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>12} {:>12} {:>12} {:>12} {:>7} {:<28} PATH",
        "KIND", "NAME", "SIZE", "USED", "FREE", "ALLOC", "USE%", "DETAILS"
    )?;
    for node in graph
        .nodes
        .iter()
        .filter(|node| has_capacity_or_usage(node))
    {
        let usage = node.usage.as_ref();
        writeln!(
            output,
            "{:<22} {:<38} {:>12} {:>12} {:>12} {:>12} {:>7} {:<28} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            human_bytes(usage.and_then(|usage| usage.used_bytes)),
            human_bytes(usage.and_then(|usage| usage.free_bytes)),
            human_bytes(usage.and_then(|usage| usage.allocated_bytes)),
            usage_percent(node),
            usage_details(node),
            node.path.as_deref().unwrap_or("-")
        )?;
    }
    Ok(())
}

fn has_identity(node: &Node) -> bool {
    !node.identity.is_empty()
}
