fn collect_nfs(result: &mut ProbeResult) {
    match run_report("nfsstat", &["-m"]) {
        Ok(output) if output.is_empty() => result.reports.push(ProbeReport {
            adapter: "nfs".to_string(),
            status: ProbeStatus::Available,
            message: Some("no NFS mounts discovered".to_string()),
        }),
        Ok(output) => match nfs::normalize_nfsstat_mounts(&output) {
            Ok(graph) => {
                let node_count = graph.nodes.len();
                merge_graph(&mut result.graph, graph);
                result.reports.push(ProbeReport {
                    adapter: "nfs".to_string(),
                    status: ProbeStatus::Available,
                    message: Some(format!(
                        "normalized {node_count} graph nodes from NFS mounts"
                    )),
                });
            }
            Err(error) => result.reports.push(ProbeReport {
                adapter: "nfs".to_string(),
                status: ProbeStatus::Failed,
                message: Some(error.to_string()),
            }),
        },
        Err(message) => result.reports.push(ProbeReport {
            adapter: "nfs".to_string(),
            status: if message.contains("not found") || message.contains("No such file") {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Partial
            },
            message: Some(message),
        }),
    }
}

fn collect_nfs_exports(result: &mut ProbeResult) {
    match run_report("exportfs", &["-v"]) {
        Ok(output) if output.is_empty() => result.reports.push(ProbeReport {
            adapter: "nfs-exports".to_string(),
            status: ProbeStatus::Available,
            message: Some("no NFS exports discovered".to_string()),
        }),
        Ok(output) => match nfs::normalize_exportfs_verbose(&output) {
            Ok(graph) => {
                let node_count = graph.nodes.len();
                merge_graph(&mut result.graph, graph);
                result.reports.push(ProbeReport {
                    adapter: "nfs-exports".to_string(),
                    status: ProbeStatus::Available,
                    message: Some(format!(
                        "normalized {node_count} graph nodes from NFS exports"
                    )),
                });
            }
            Err(error) => result.reports.push(ProbeReport {
                adapter: "nfs-exports".to_string(),
                status: ProbeStatus::Failed,
                message: Some(error.to_string()),
            }),
        },
        Err(message) => result.reports.push(ProbeReport {
            adapter: "nfs-exports".to_string(),
            status: if message.contains("not found") || message.contains("No such file") {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Partial
            },
            message: Some(message),
        }),
    }
}

fn collect_iscsi(result: &mut ProbeResult) {
    match run_report("iscsiadm", &["-m", "session", "-P", "3"]) {
        Ok(output) => match iscsi::normalize_iscsi_session_output(&output) {
            Ok(graph) => {
                let node_count = graph.nodes.len();
                merge_graph(&mut result.graph, graph);
                result.reports.push(ProbeReport {
                    adapter: "iscsi".to_string(),
                    status: ProbeStatus::Available,
                    message: Some(format!(
                        "normalized {node_count} graph nodes from iSCSI sessions"
                    )),
                });
            }
            Err(error) => result.reports.push(ProbeReport {
                adapter: "iscsi".to_string(),
                status: ProbeStatus::Failed,
                message: Some(error.to_string()),
            }),
        },
        Err(message) => result.reports.push(ProbeReport {
            adapter: "iscsi".to_string(),
            status: if message.contains("not found") || message.contains("No such file") {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Partial
            },
            message: Some(message),
        }),
    }
}

fn collect_iscsi_nodes(result: &mut ProbeResult) {
    match run_report("iscsiadm", &["-m", "node", "-P", "1"]) {
        Ok(output) => match iscsi::normalize_iscsi_node_output(&output) {
            Ok(graph) if graph.nodes.is_empty() => result.reports.push(ProbeReport {
                adapter: "iscsi-nodes".to_string(),
                status: ProbeStatus::Available,
                message: Some("no configured iSCSI nodes discovered".to_string()),
            }),
            Ok(graph) => {
                let node_count = graph.nodes.len();
                merge_graph(&mut result.graph, graph);
                result.reports.push(ProbeReport {
                    adapter: "iscsi-nodes".to_string(),
                    status: ProbeStatus::Available,
                    message: Some(format!(
                        "normalized {node_count} graph nodes from configured iSCSI nodes"
                    )),
                });
            }
            Err(error) => result.reports.push(ProbeReport {
                adapter: "iscsi-nodes".to_string(),
                status: ProbeStatus::Failed,
                message: Some(error.to_string()),
            }),
        },
        Err(message) => result.reports.push(ProbeReport {
            adapter: "iscsi-nodes".to_string(),
            status: if message.contains("No records found") {
                ProbeStatus::Available
            } else if message.contains("not found") || message.contains("No such file") {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Partial
            },
            message: Some(message),
        }),
    }
}

fn collect_btrfs(result: &mut ProbeResult) {
    let targets = match run_findmnt_targets("btrfs") {
        Ok(targets) => targets,
        Err(message) => {
            result.reports.push(ProbeReport {
                adapter: "btrfs".to_string(),
                status: ProbeStatus::Partial,
                message: Some(format!(
                    "failed to discover mounted Btrfs targets: {message}"
                )),
            });
            return;
        }
    };

    if targets.is_empty() {
        result.reports.push(ProbeReport {
            adapter: "btrfs".to_string(),
            status: if command_exists("btrfs") {
                ProbeStatus::Available
            } else {
                ProbeStatus::Unavailable
            },
            message: Some("no mounted Btrfs filesystems discovered".to_string()),
        });
        return;
    }

    let mut reports = Vec::new();
    for target in targets {
        let show = run_report("btrfs", &["filesystem", "show", &target]);
        let usage = run_report("btrfs", &["filesystem", "usage", "-b", &target]);
        let subvolumes = run_report(
            "btrfs",
            &["subvolume", "list", "-p", "-u", "-q", "-R", "-c", &target],
        );
        let qgroups = run_report(
            "btrfs",
            &["qgroup", "show", "--raw", "-reF", "-p", "-c", &target],
        )
        .unwrap_or_default();
        let device_stats = run_report("btrfs", &["device", "stats", &target]).unwrap_or_default();

        match (show, usage, subvolumes) {
            (Ok(show), Ok(usage), Ok(subvolumes)) => reports.push(btrfs::BtrfsReport {
                target,
                show,
                usage,
                subvolumes,
                qgroups,
                device_stats,
            }),
            (Err(message), _, _) | (_, Err(message), _) | (_, _, Err(message)) => {
                result.reports.push(ProbeReport {
                    adapter: "btrfs".to_string(),
                    status: if message.contains("not found") {
                        ProbeStatus::Unavailable
                    } else {
                        ProbeStatus::Partial
                    },
                    message: Some(message),
                });
                return;
            }
        }
    }

    match btrfs::normalize_btrfs_reports(&reports) {
        Ok(graph) => {
            let node_count = graph.nodes.len();
            merge_graph(&mut result.graph, graph);
            result.reports.push(ProbeReport {
                adapter: "btrfs".to_string(),
                status: ProbeStatus::Available,
                message: Some(format!(
                    "normalized {node_count} graph nodes from Btrfs output"
                )),
            });
        }
        Err(error) => result.reports.push(ProbeReport {
            adapter: "btrfs".to_string(),
            status: ProbeStatus::Failed,
            message: Some(error.to_string()),
        }),
    }
}

fn collect_bcache(result: &mut ProbeResult) {
    match bcache::read_sysfs_snapshot(std::path::Path::new("/sys/block")) {
        Ok(snapshot) if snapshot.devices.is_empty() => result.reports.push(ProbeReport {
            adapter: "bcache".to_string(),
            status: if std::path::Path::new("/sys/fs/bcache").exists() {
                ProbeStatus::Available
            } else {
                ProbeStatus::Unavailable
            },
            message: Some("no bcache devices discovered".to_string()),
        }),
        Ok(snapshot) => {
            let graph = bcache::normalize_bcache_snapshot(&snapshot);
            let node_count = graph.nodes.len();
            merge_graph(&mut result.graph, graph);
            result.reports.push(ProbeReport {
                adapter: "bcache".to_string(),
                status: ProbeStatus::Available,
                message: Some(format!(
                    "normalized {node_count} graph nodes from bcache sysfs"
                )),
            });
        }
        Err(error) => result.reports.push(ProbeReport {
            adapter: "bcache".to_string(),
            status: ProbeStatus::Partial,
            message: Some(error.to_string()),
        }),
    }
}

fn run_findmnt_targets(filesystem_type: &str) -> Result<Vec<String>, String> {
    match Command::new("findmnt")
        .args(["-rn", "-t", filesystem_type, "-o", "TARGET"])
        .output()
    {
        Ok(output) if output.status.success() => Ok(parse_lines(&output.stdout)),
        Ok(output) if output.stdout.is_empty() && output.stderr.is_empty() => Ok(Vec::new()),
        Ok(output) => Err(String::from_utf8_lossy(&output.stderr).trim().to_string()),
        Err(error) => Err(format!("findmnt not found or failed to run: {error}")),
    }
}

fn collect_zfs(result: &mut ProbeResult) {
    let zpool_list = run_report(
        "zpool",
        &[
            "list",
            "-H",
            "-p",
            "-o",
            "name,size,alloc,free,health,capacity,dedupratio,fragmentation,altroot",
        ],
    );
    let zfs_list = run_report(
        "zfs",
        &[
            "list",
            "-H",
            "-p",
            "-t",
            "filesystem,volume,snapshot",
            "-o",
            "name,type,used,available,referenced,mountpoint,origin,userrefs,compression,quota,reservation,encryption,keystatus,volsize,recordsize,dedup,checksum,copies,sync,primarycache,secondarycache,atime,relatime,snapdir,acltype,xattr",
        ],
    );
    let zpool_get = run_report(
        "zpool",
        &[
            "get",
            "-H",
            "-o",
            "name,property,value",
            "altroot,ashift,autotrim,autoexpand,autoreplace,bootfs,cachefile,comment,delegation,failmode,listsnapshots,multihost",
        ],
    );
    let zpool_status = run_report("zpool", &["status", "-P"]);

    match (zpool_list, zpool_get, zfs_list, zpool_status) {
        (Ok(zpool_list), Ok(zpool_get), Ok(zfs_list), Ok(zpool_status)) => {
            let zfs_holds = collect_zfs_holds(&zfs_list);
            match zfs::normalize_zfs(
                &zpool_list,
                &zpool_get,
                &zfs_list,
                &zfs_holds,
                &zpool_status,
            ) {
                Ok(graph) => {
                    let node_count = graph.nodes.len();
                    merge_graph(&mut result.graph, graph);
                    result.reports.push(ProbeReport {
                        adapter: "zfs".to_string(),
                        status: ProbeStatus::Available,
                        message: Some(format!(
                            "normalized {node_count} graph nodes from ZFS output"
                        )),
                    });
                }
                Err(error) => result.reports.push(ProbeReport {
                    adapter: "zfs".to_string(),
                    status: ProbeStatus::Failed,
                    message: Some(error.to_string()),
                }),
            }
        }
        (Err(message), _, _, _)
        | (_, Err(message), _, _)
        | (_, _, Err(message), _)
        | (_, _, _, Err(message)) => {
            result.reports.push(ProbeReport {
                adapter: "zfs".to_string(),
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

fn collect_zfs_holds(zfs_list: &[u8]) -> Vec<u8> {
    let Ok(text) = std::str::from_utf8(zfs_list) else {
        return Vec::new();
    };
    let mut output = Vec::new();
    for snapshot in text.lines().filter_map(zfs_snapshot_name_from_list_line) {
        if let Ok(mut holds) = run_report("zfs", &["holds", "-H", snapshot]) {
            if !holds.ends_with(b"\n") {
                holds.push(b'\n');
            }
            output.extend(holds);
        }
    }
    output
}

fn zfs_snapshot_name_from_list_line(line: &str) -> Option<&str> {
    let mut fields = line.split('\t');
    let name = fields.next()?;
    let kind = fields.next()?;
    (kind == "snapshot").then_some(name)
}
