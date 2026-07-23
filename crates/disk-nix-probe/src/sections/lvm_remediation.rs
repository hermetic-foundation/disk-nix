fn collect_lvm(result: &mut ProbeResult) {
    let pvs = run_report(
        "pvs",
        &[
            "--reportformat",
            "json",
            "-o",
            "pv_name,vg_name,pv_fmt,pv_uuid,dev_size,pv_major,pv_minor,pv_size,pv_free,pv_used,pe_start,pv_attr,pv_allocatable,pv_exported,pv_missing,pv_pe_count,pv_pe_alloc_count,pv_tags,pv_mda_count,pv_mda_used_count,pv_mda_free,pv_mda_size,pv_ba_start,pv_ba_size,pv_in_use,pv_duplicate,pv_device_id,pv_device_id_type",
        ],
    );
    let vgs = run_report(
        "vgs",
        &[
            "--reportformat",
            "json",
            "-o",
            "vg_name,vg_fmt,vg_uuid,vg_attr,vg_permissions,vg_extendable,vg_exported,vg_autoactivation,vg_partial,vg_allocation_policy,vg_clustered,vg_shared,vg_size,vg_free,vg_sysid,vg_lock_type,vg_lock_args,vg_extent_size,vg_extent_count,vg_free_count,max_lv,max_pv,pv_count,vg_missing_pv_count,lv_count,snap_count,vg_seqno,vg_tags,vg_profile,vg_mda_count,vg_mda_used_count,vg_mda_free,vg_mda_size,vg_mda_copies",
        ],
    );
    let lvs = run_report(
        "lvs",
        &[
            "--reportformat",
            "json",
            "-o",
            "lv_name,vg_name,lv_uuid,lv_path,lv_size,lv_attr,lv_layout,lv_active,lv_active_locally,lv_active_remotely,lv_active_exclusively,lv_permissions,lv_health_status,lv_when_full,lv_metadata_size,lv_tags,lv_dm_path,lv_parent,lv_read_ahead,lv_kernel_read_ahead,lv_suspended,lv_live_table,lv_inactive_table,lv_modules,lv_host,lv_historical,lv_kernel_major,lv_kernel_minor,lv_device_open,lv_check_needed,lv_role,lv_time,origin,pool_lv,raid_mismatch_count,raid_sync_action,raid_write_behind,raid_min_recovery_rate,raid_max_recovery_rate,raidintegritymode,raidintegrityblocksize,integritymismatches,data_percent,snap_percent,metadata_percent,copy_percent,sync_percent,cache_total_blocks,cache_used_blocks,cache_dirty_blocks,cache_read_hits,cache_read_misses,cache_write_hits,cache_write_misses,cache_promotions,cache_demotions,cache_mode,cache_policy,kernel_cache_settings,kernel_cache_mode,kernel_cache_policy,kernel_metadata_format,kernel_discards,vdo_operating_mode,vdo_compression_state,vdo_index_state,vdo_used_size,vdo_saving_percent,writecache_total_blocks,writecache_free_blocks,writecache_writeback_blocks,writecache_block_size,writecache_error",
        ],
    );
    let segments = run_report(
        "lvs",
        &[
            "--segments",
            "--reportformat",
            "json",
            "-o",
            "lv_name,vg_name,segtype,stripes,data_stripes,reshape_len,reshape_len_le,data_copies,data_offset,new_data_offset,parity_chunks,stripe_size,region_size,seg_start,seg_start_pe,seg_size,seg_size_pe,seg_tags,chunk_size,thin_count,discards,zero,transaction_id,thin_id,devices,metadata_devices,seg_pe_ranges,seg_le_ranges,seg_metadata_le_ranges,seg_monitor,cache_metadata_format,cache_mode,cache_policy,cache_settings,integrity_settings,vdo_compression,vdo_deduplication,vdo_minimum_io_size,vdo_block_map_cache_size,vdo_block_map_era_length,vdo_use_sparse_index,vdo_index_memory_size,vdo_slab_size,vdo_ack_threads,vdo_bio_threads,vdo_bio_rotation,vdo_cpu_threads,vdo_hash_zone_threads,vdo_logical_threads,vdo_physical_threads,vdo_max_discard,vdo_header_size,vdo_use_metadata_hints,vdo_write_policy",
        ],
    );

    match (pvs, vgs, lvs) {
        (Ok(pvs), Ok(vgs), Ok(lvs)) => {
            let segment_error = segments.as_ref().err().cloned();
            let segments = segments.as_deref().ok();
            match lvm::normalize_lvm_json(&pvs, &vgs, &lvs, segments) {
                Ok(graph) => {
                    let node_count = graph.nodes.len();
                    merge_graph(&mut result.graph, graph);
                    let status = if segment_error.is_some() {
                        ProbeStatus::Partial
                    } else {
                        ProbeStatus::Available
                    };
                    let suffix = segment_error
                        .map(|message| format!("; segment query failed: {message}"))
                        .unwrap_or_default();
                    result.reports.push(ProbeReport {
                        adapter: "lvm".to_string(),
                        status,
                        message: Some(format!(
                            "normalized {node_count} graph nodes from LVM JSON{suffix}"
                        )),
                    });
                }
                Err(error) => result.reports.push(ProbeReport {
                    adapter: "lvm".to_string(),
                    status: ProbeStatus::Failed,
                    message: Some(error.to_string()),
                }),
            }
        }
        (Err(message), _, _) | (_, Err(message), _) | (_, _, Err(message)) => {
            result.reports.push(ProbeReport {
                adapter: "lvm".to_string(),
                status: if message.contains("not found") {
                    ProbeStatus::Unavailable
                } else {
                    ProbeStatus::Partial
                },
                message: Some(message),
            });
        }
    }
}

fn run_report(command: &str, args: &[&str]) -> Result<Vec<u8>, String> {
    match Command::new(command).args(args).output() {
        Ok(output) if output.status.success() => Ok(output.stdout),
        Ok(output) => Err(String::from_utf8_lossy(&output.stderr).trim().to_string()),
        Err(error) => Err(format!("{command} not found or failed to run: {error}")),
    }
}

fn probe_category_for_message(message: &str) -> ProbeIssueCategory {
    let lower = message.to_ascii_lowercase();
    if lower.contains("not found")
        || lower.contains("no such file")
        || lower.contains("enoent")
        || lower.contains("not in path")
        || lower.contains("not in $path")
    {
        ProbeIssueCategory::MissingTool
    } else if lower.contains("permission denied")
        || lower.contains("access denied")
        || lower.contains("operation not permitted")
        || lower.contains("not permitted")
        || lower.contains("only root")
        || lower.contains("must be root")
        || lower.contains("are you root")
        || lower.contains("requires root")
        || lower.contains("requires superuser")
        || lower.contains("need superuser")
        || lower.contains("insufficient privileges")
        || lower.contains("insufficient privilege")
    {
        ProbeIssueCategory::PermissionDenied
    } else if lower.contains("inaccessible") || lower.contains("failed to access") {
        ProbeIssueCategory::InaccessibleData
    } else {
        ProbeIssueCategory::CommandFailed
    }
}

fn probe_category_for_status(status: &ProbeStatus, message: &str) -> ProbeIssueCategory {
    let category = probe_category_for_message(message);
    if matches!(status, ProbeStatus::Failed)
        && category == ProbeIssueCategory::CommandFailed
        && message_looks_like_parse_failure(message)
    {
        ProbeIssueCategory::ParseFailed
    } else {
        category
    }
}

fn remediation_for_category(adapter: &str, category: ProbeIssueCategory) -> Vec<String> {
    match category {
        ProbeIssueCategory::None => Vec::new(),
        ProbeIssueCategory::MissingTool => {
            let tools = adapter_tools(adapter);
            let packages = adapter_nix_packages(adapter);
            let mut remediation = vec![if tools.is_empty() {
                format!("install or expose the command-line tools required by the {adapter} adapter")
            } else {
                format!(
                    "install or expose required {adapter} tool(s): {}",
                    tools.join(", ")
                )
            }];
            if packages.is_empty() {
                remediation.push(
                    "on NixOS, include the matching storage tool package in services.disk-nix.toolPackages"
                        .to_string(),
                );
            } else {
                remediation.push(format!(
                    "on NixOS, include {} in services.disk-nix.toolPackages",
                    packages.join(", ")
                ));
            }
            remediation
        }
        ProbeIssueCategory::PermissionDenied => vec![
            format!("rerun {adapter} probing with privileges that can read the relevant storage metadata"),
            adapter_privilege_hint(adapter),
            "check device node permissions, udev rules, container sandboxing, and LSM policy before treating the topology as complete".to_string(),
        ],
        ProbeIssueCategory::ParseFailed => vec![
            format!("capture the raw {adapter} command output for fixture coverage"),
            adapter_parse_hint(adapter),
            "check whether the installed tool version changed its output format".to_string(),
        ],
        ProbeIssueCategory::InaccessibleData => vec![
            format!("verify the kernel surface, service, mountpoint, or device required by the {adapter} adapter is present"),
            adapter_data_hint(adapter),
            "load the relevant kernel module or start the relevant storage service before probing again".to_string(),
        ],
        ProbeIssueCategory::CommandFailed => vec![
            format!("rerun the failing {adapter} command manually and inspect its exit status and stderr"),
            adapter_command_hint(adapter),
            "treat this storage domain as degraded until the command failure is understood".to_string(),
        ],
    }
}

fn canonical_adapter(adapter: &str) -> &str {
    match adapter {
        "mdadm-scan" | "mdadm-examine" => "mdraid",
        "nvme-list-subsys" | "nvme-smart-log" | "nvme-id-ctrl" | "nvme-id-ns" => "nvme",
        "vdostats" | "vdostats-verbose" => "vdo",
        "iscsi-nodes" => "iscsi",
        "nfs-exports" => "nfs",
        "loopdev" => "loop",
        "zramctl" => "zram",
        other => other,
    }
}

fn adapter_tools(adapter: &str) -> Vec<&'static str> {
    match canonical_adapter(adapter) {
        "bcache" => vec!["bcache"],
        "bcachefs" => vec!["bcachefs"],
        "blkid" => vec!["blkid"],
        "btrfs" => vec!["btrfs"],
        "cryptsetup" => vec!["cryptsetup"],
        "dmsetup" => vec!["dmsetup"],
        "exfat" => vec!["exfatlabel", "dump.exfat"],
        "ext" => vec!["tune2fs", "dumpe2fs"],
        "f2fs" => vec!["dump.f2fs"],
        "findmnt" => vec!["findmnt"],
        "iscsi" => vec!["iscsiadm"],
        "loop" => vec!["losetup"],
        "lsblk" => vec!["lsblk"],
        "lsscsi" => vec!["lsscsi"],
        "lvm" => vec!["pvs", "vgs", "lvs"],
        "mdraid" => vec!["mdadm"],
        "mdstat" => Vec::new(),
        "multipath" => vec!["multipath"],
        "nfs" => vec!["findmnt", "exportfs", "nfsstat"],
        "ntfs" => vec!["ntfsinfo"],
        "nvme" => vec!["nvme"],
        "parted" => vec!["parted"],
        "smartctl" => vec!["smartctl"],
        "swaps" => vec!["swapon"],
        "udev" => vec!["udevadm"],
        "vdo" => vec!["vdo", "vdostats"],
        "xfs" => vec!["xfs_info"],
        "zfs" => vec!["zpool", "zfs"],
        "zram" => vec!["zramctl"],
        _ => Vec::new(),
    }
}

fn adapter_nix_packages(adapter: &str) -> Vec<&'static str> {
    match canonical_adapter(adapter) {
        "bcache" => vec!["pkgs.bcache-tools"],
        "bcachefs" => vec!["pkgs.bcachefs-tools"],
        "blkid" | "findmnt" | "loop" | "lsblk" | "swaps" | "zram" => {
            vec!["pkgs.util-linux"]
        }
        "btrfs" => vec!["pkgs.btrfs-progs"],
        "cryptsetup" => vec!["pkgs.cryptsetup"],
        "dmsetup" | "lvm" => vec!["pkgs.lvm2"],
        "exfat" => vec!["pkgs.exfatprogs"],
        "ext" => vec!["pkgs.e2fsprogs"],
        "f2fs" => vec!["pkgs.f2fs-tools"],
        "iscsi" => vec!["pkgs.openiscsi"],
        "lsscsi" => vec!["pkgs.lsscsi"],
        "mdraid" => vec!["pkgs.mdadm"],
        "mdstat" => Vec::new(),
        "multipath" => vec!["pkgs.multipath-tools"],
        "nfs" => vec!["pkgs.nfs-utils", "pkgs.util-linux"],
        "ntfs" => vec!["pkgs.ntfs3g"],
        "nvme" => vec!["pkgs.nvme-cli"],
        "parted" => vec!["pkgs.parted"],
        "smartctl" => vec!["pkgs.smartmontools"],
        "udev" => vec!["pkgs.systemd"],
        "vdo" => vec!["pkgs.vdo"],
        "xfs" => vec!["pkgs.xfsprogs"],
        "zfs" => vec!["pkgs.zfs"],
        _ => Vec::new(),
    }
}

fn adapter_privilege_hint(adapter: &str) -> String {
    match canonical_adapter(adapter) {
        "dmsetup" => "device-mapper probing needs access to /dev/mapper, /sys/block/dm-*, and dmsetup table/status metadata".to_string(),
        "lvm" => "LVM probing needs access to device-mapper state, LVM metadata devices, and any configured lvmetad/lvmdevices state".to_string(),
        "cryptsetup" => "LUKS probing needs permission to read block devices and cryptsetup status/header metadata".to_string(),
        "zfs" => "ZFS probing needs permission to run zpool and zfs list/status commands and read imported pool metadata".to_string(),
        "btrfs" => "Btrfs probing needs permission to inspect mounted Btrfs filesystems and query subvolume, qgroup, and device state".to_string(),
        "iscsi" => "iSCSI probing needs access to open-iscsi node and session state, usually under /etc/iscsi and /sys/class/iscsi_session".to_string(),
        "nvme" => "NVMe probing needs access to controller character devices and /sys/class/nvme metadata".to_string(),
        "multipath" => "multipath probing needs access to multipathd/device-mapper state and path devices".to_string(),
        "mdraid" | "mdstat" => "MD RAID probing needs access to /proc/mdstat, mdadm detail output, and member block devices".to_string(),
        "vdo" => "VDO probing needs access to VDO management state and device-mapper-backed VDO volumes".to_string(),
        "smartctl" => "SMART probing often needs root or device-specific capabilities to read health and controller metadata".to_string(),
        "udev" => "udev probing needs permission to read udev database records for block devices".to_string(),
        _ => format!("{adapter} probing needs privileges for its command output and related kernel metadata"),
    }
}

fn adapter_parse_hint(adapter: &str) -> String {
    match canonical_adapter(adapter) {
        "lvm" => "include the failing pvs/vgs/lvs JSON payload and LVM version in the fixture"
            .to_string(),
        "zfs" => {
            "include zpool/zfs command output, pool feature flags, and ZFS version in the fixture"
                .to_string()
        }
        "btrfs" => {
            "include btrfs filesystem, subvolume, qgroup, and device command output in the fixture"
                .to_string()
        }
        "vdo" => {
            "include vdo status or vdostats output from the installed VDO version in the fixture"
                .to_string()
        }
        "nvme" => "include nvme-cli JSON output and nvme-cli version in the fixture".to_string(),
        "iscsi" => {
            "include iscsiadm node/session output and open-iscsi version in the fixture".to_string()
        }
        "nfs" => {
            "include findmnt, exportfs, and nfsstat output for the failing host in the fixture"
                .to_string()
        }
        _ => {
            format!("include raw {adapter} command output and tool version in a regression fixture")
        }
    }
}

fn adapter_data_hint(adapter: &str) -> String {
    match canonical_adapter(adapter) {
        "bcache" => "verify bcache devices are registered under /sys/fs/bcache or /sys/block before probing".to_string(),
        "bcachefs" => "verify bcachefs filesystems are mounted or member devices are visible before probing".to_string(),
        "btrfs" => "verify Btrfs filesystems are mounted and qgroup/subvolume metadata is accessible".to_string(),
        "dmsetup" => "verify device-mapper is loaded and expected /dev/mapper nodes exist".to_string(),
        "iscsi" => "verify iscsid/open-iscsi state exists and expected sessions or configured nodes are present".to_string(),
        "lvm" => "verify LVM devices are visible, filters permit scanning, and volume groups are not hidden by system-id or devices-file policy".to_string(),
        "multipath" => "verify multipathd is running when required and path devices are visible to the host".to_string(),
        "nfs" => "verify NFS mounts, exports, rpc services, and /proc/fs/nfsd state are available where expected".to_string(),
        "nvme" => "verify NVMe controllers, namespaces, and fabrics sessions are visible under /sys/class/nvme".to_string(),
        "vdo" => "verify VDO services, management metadata, and mapped VDO devices are present".to_string(),
        "zfs" => "verify ZFS kernel support is loaded and expected pools are imported or visible to zpool import".to_string(),
        "zram" => "verify zram devices are configured before expecting zram inventory".to_string(),
        _ => format!("verify the storage resources expected by the {adapter} adapter exist on this host"),
    }
}

fn adapter_command_hint(adapter: &str) -> String {
    match canonical_adapter(adapter) {
        "lvm" => "rerun pvs, vgs, and lvs with --reportformat json to identify which LVM query failed".to_string(),
        "zfs" => "rerun zpool status/list and zfs list/get commands to identify pool import or dataset failures".to_string(),
        "btrfs" => "rerun btrfs filesystem, subvolume, qgroup, and device commands against the mounted filesystem".to_string(),
        "iscsi" => "rerun iscsiadm node and session queries and verify iscsid service health".to_string(),
        "multipath" => "rerun multipath -ll and verify multipathd plus device-mapper state".to_string(),
        "nvme" => "rerun nvme list/subsystem/id/smart-log commands for the affected controller or namespace".to_string(),
        "vdo" => "rerun vdo status and vdostats to distinguish service failure from missing VDO volumes".to_string(),
        _ => format!("rerun the {adapter} adapter command set manually with stderr captured"),
    }
}

fn message_looks_like_parse_failure(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("parse")
        || lower.contains("json")
        || lower.contains("expected")
        || lower.contains("invalid")
        || lower.contains("missing field")
        || lower.contains("unknown field")
}

fn run_report_accept_stdout_without_stderr(
    command: &str,
    args: &[&str],
) -> Result<Vec<u8>, String> {
    match Command::new(command).args(args).output() {
        Ok(output)
            if output.status.success() || !output.stdout.is_empty() && output.stderr.is_empty() =>
        {
            Ok(output.stdout)
        }
        Ok(output) => Err(String::from_utf8_lossy(&output.stderr).trim().to_string()),
        Err(error) => Err(format!("{command} not found or failed to run: {error}")),
    }
}
