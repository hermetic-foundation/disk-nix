fn collect_lsscsi(result: &mut ProbeResult) {
    let mut node_count = 0_usize;
    let mut failures = Vec::new();
    let mut unavailable = false;

    for (label, args, normalizer) in [
        (
            "list",
            &["-L", "-g", "-s"][..],
            lsscsi::normalize_lsscsi_list_output as fn(&[u8]) -> Result<StorageGraph, ProbeError>,
        ),
        (
            "transport",
            &["-g", "-s", "-t", "-i", "-w"][..],
            lsscsi::normalize_lsscsi_transport_output,
        ),
        (
            "unit",
            &["-g", "-s", "-u", "-i", "-w"][..],
            lsscsi::normalize_lsscsi_unit_output,
        ),
    ] {
        match run_report("lsscsi", args) {
            Ok(output) => match normalizer(&output) {
                Ok(graph) => {
                    node_count += graph.nodes.len();
                    merge_graph(&mut result.graph, graph);
                }
                Err(error) => failures.push(format!("{label}: {error}")),
            },
            Err(message) => {
                if message.contains("not found") || message.contains("No such file") {
                    unavailable = true;
                }
                failures.push(format!("{label}: {message}"));
            }
        }
    }

    if node_count > 0 {
        result.reports.push(ProbeReport {
            adapter: "lsscsi".to_string(),
            status: if failures.is_empty() {
                ProbeStatus::Available
            } else {
                ProbeStatus::Partial
            },
            message: Some(format!(
                "normalized {node_count} graph nodes from lsscsi output{}",
                if failures.is_empty() {
                    String::new()
                } else {
                    format!("; {} lsscsi probes failed", failures.len())
                }
            )),
        });
    } else if !failures.is_empty() {
        result.reports.push(ProbeReport {
            adapter: "lsscsi".to_string(),
            status: if unavailable {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Partial
            },
            message: Some(format!("lsscsi probes failed: {}", failures.join("; "))),
        });
    }
}

fn collect_smartctl(result: &mut ProbeResult) {
    let disk_paths: Vec<String> = result
        .graph
        .nodes
        .iter()
        .filter(|node| node.kind == disk_nix_model::NodeKind::PhysicalDisk)
        .filter_map(|node| node.path.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if disk_paths.is_empty() {
        return;
    }

    let mut node_count = 0_usize;
    let mut unavailable = false;
    let mut failures = Vec::new();
    for path in disk_paths {
        match run_report("smartctl", &["-a", "-j", path.as_str()]) {
            Ok(output) => match smartctl::normalize_smartctl_json(&path, &output) {
                Ok(graph) => {
                    node_count += graph.nodes.len();
                    merge_graph(&mut result.graph, graph);
                }
                Err(error) => failures.push(format!("{path}: {error}")),
            },
            Err(message) => {
                if message.contains("not found") || message.contains("No such file") {
                    unavailable = true;
                }
                failures.push(format!("{path}: {message}"));
            }
        }
    }

    if node_count > 0 {
        result.reports.push(ProbeReport {
            adapter: "smartctl".to_string(),
            status: if failures.is_empty() {
                ProbeStatus::Available
            } else {
                ProbeStatus::Partial
            },
            message: Some(format!(
                "normalized {node_count} graph nodes from smartctl JSON{}",
                if failures.is_empty() {
                    String::new()
                } else {
                    format!("; {} smartctl probes failed", failures.len())
                }
            )),
        });
    } else if !failures.is_empty() {
        result.reports.push(ProbeReport {
            adapter: "smartctl".to_string(),
            status: if unavailable {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Partial
            },
            message: Some(format!("smartctl probes failed: {}", failures.join("; "))),
        });
    }
}

fn collect_lsblk(result: &mut ProbeResult) {
    match Command::new("lsblk")
        .args(["--json", "--bytes", "--output-all"])
        .output()
    {
        Ok(output) if output.status.success() => {
            match lsblk::normalize_lsblk_json(&output.stdout) {
                Ok(graph) => {
                    let node_count = graph.nodes.len();
                    merge_graph(&mut result.graph, graph);
                    result.reports.push(lsblk::available_report(node_count));
                }
                Err(error) => result.reports.push(ProbeReport {
                    adapter: "lsblk".to_string(),
                    status: ProbeStatus::Failed,
                    message: Some(error.to_string()),
                }),
            }
        }
        Ok(output) => result.reports.push(ProbeReport {
            adapter: "lsblk".to_string(),
            status: ProbeStatus::Failed,
            message: Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
        }),
        Err(error) => result.reports.push(ProbeReport {
            adapter: "lsblk".to_string(),
            status: ProbeStatus::Unavailable,
            message: Some(error.to_string()),
        }),
    }
}

fn collect_blkid(result: &mut ProbeResult) {
    match run_report("blkid", &["-o", "export"]) {
        Ok(output) => match blkid::normalize_blkid_export(&output) {
            Ok(graph) if graph.nodes.is_empty() => result.reports.push(ProbeReport {
                adapter: "blkid".to_string(),
                status: ProbeStatus::Available,
                message: Some("no block signatures discovered".to_string()),
            }),
            Ok(graph) => {
                let node_count = graph.nodes.len();
                merge_graph(&mut result.graph, graph);
                result.reports.push(ProbeReport {
                    adapter: "blkid".to_string(),
                    status: ProbeStatus::Available,
                    message: Some(format!(
                        "normalized {node_count} graph nodes from blkid export"
                    )),
                });
            }
            Err(error) => result.reports.push(ProbeReport {
                adapter: "blkid".to_string(),
                status: ProbeStatus::Failed,
                message: Some(error.to_string()),
            }),
        },
        Err(message) => result.reports.push(ProbeReport {
            adapter: "blkid".to_string(),
            status: if message.contains("not found") || message.contains("No such file") {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Partial
            },
            message: Some(message),
        }),
    }
}

fn collect_parted(result: &mut ProbeResult) {
    match run_report("parted", &["-lm"]) {
        Ok(output) => match parted::normalize_parted_machine(&output) {
            Ok(graph) => {
                let node_count = graph.nodes.len();
                merge_graph(&mut result.graph, graph);
                result.reports.push(ProbeReport {
                    adapter: "parted".to_string(),
                    status: ProbeStatus::Available,
                    message: Some(format!(
                        "normalized {node_count} graph nodes from parted machine output"
                    )),
                });
            }
            Err(error) => result.reports.push(ProbeReport {
                adapter: "parted".to_string(),
                status: ProbeStatus::Failed,
                message: Some(error.to_string()),
            }),
        },
        Err(message) => result.reports.push(ProbeReport {
            adapter: "parted".to_string(),
            status: if message.contains("not found") || message.contains("No such file") {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Partial
            },
            message: Some(message),
        }),
    }
}

fn collect_udev(result: &mut ProbeResult) {
    match run_report("udevadm", &["info", "--export-db"]) {
        Ok(output) => match udev::normalize_udev_export_db(&output) {
            Ok(graph) if graph.nodes.is_empty() => result.reports.push(ProbeReport {
                adapter: "udev".to_string(),
                status: ProbeStatus::Available,
                message: Some("no block device metadata discovered".to_string()),
            }),
            Ok(graph) => {
                let node_count = graph.nodes.len();
                merge_graph(&mut result.graph, graph);
                result.reports.push(ProbeReport {
                    adapter: "udev".to_string(),
                    status: ProbeStatus::Available,
                    message: Some(format!(
                        "normalized {node_count} graph nodes from udev export database"
                    )),
                });
            }
            Err(error) => result.reports.push(ProbeReport {
                adapter: "udev".to_string(),
                status: ProbeStatus::Failed,
                message: Some(error.to_string()),
            }),
        },
        Err(message) => result.reports.push(ProbeReport {
            adapter: "udev".to_string(),
            status: if message.contains("not found") || message.contains("No such file") {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Partial
            },
            message: Some(message),
        }),
    }
}

fn collect_ext(result: &mut ProbeResult) {
    let targets = ext_targets(&result.graph);
    if targets.is_empty() {
        result.reports.push(ProbeReport {
            adapter: "ext".to_string(),
            status: if command_exists("tune2fs") {
                ProbeStatus::Available
            } else {
                ProbeStatus::Unavailable
            },
            message: Some("no ext filesystems discovered".to_string()),
        });
        return;
    }

    let mut collected = 0usize;
    let mut failures = Vec::new();
    for target in targets {
        match run_report("tune2fs", &["-l", &target]) {
            Ok(output) => match ext::normalize_tune2fs(&target, &output) {
                Ok(graph) => {
                    collected += graph.nodes.len();
                    merge_graph(&mut result.graph, graph);
                }
                Err(error) => {
                    failures.push(format!("{target}: {error}"));
                }
            },
            Err(message) => {
                failures.push(format!("{target}: {message}"));
            }
        }
    }

    match (collected, failures.is_empty()) {
        (0, false) => result.reports.push(ProbeReport {
            adapter: "ext".to_string(),
            status: if failures
                .iter()
                .any(|message| message.contains("not found") || message.contains("No such file"))
            {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Partial
            },
            message: Some(failures.join("; ")),
        }),
        (_, false) => result.reports.push(ProbeReport {
            adapter: "ext".to_string(),
            status: ProbeStatus::Partial,
            message: Some(format!(
                "normalized {collected} graph nodes from tune2fs; failed targets: {}",
                failures.join("; ")
            )),
        }),
        _ => result.reports.push(ProbeReport {
            adapter: "ext".to_string(),
            status: ProbeStatus::Available,
            message: Some(format!("normalized {collected} graph nodes from tune2fs")),
        }),
    }
}

fn collect_exfat(result: &mut ProbeResult) {
    let targets = filesystem_targets(&result.graph, |filesystem_type| filesystem_type == "exfat");
    if targets.is_empty() {
        result.reports.push(ProbeReport {
            adapter: "exfat".to_string(),
            status: if command_exists("tune.exfat") || command_exists("dump.exfat") {
                ProbeStatus::Available
            } else {
                ProbeStatus::Unavailable
            },
            message: Some("no exFAT filesystems discovered".to_string()),
        });
        return;
    }

    let mut collected = 0usize;
    let mut failures = Vec::new();
    for target in targets {
        let label = run_report("tune.exfat", &["-l", &target])
            .map_err(|message| failures.push(format!("{target} label: {message}")))
            .ok();
        let guid = run_report("tune.exfat", &["-u", &target])
            .map_err(|message| failures.push(format!("{target} GUID: {message}")))
            .ok();
        let serial = run_report("tune.exfat", &["-i", &target])
            .map_err(|message| failures.push(format!("{target} serial: {message}")))
            .ok();
        let dump = run_report_accept_stdout_without_stderr("dump.exfat", &[&target])
            .map_err(|message| failures.push(format!("{target} dump: {message}")))
            .ok();

        if label.is_none() && guid.is_none() && serial.is_none() && dump.is_none() {
            continue;
        }

        match exfat::normalize_exfat_metadata(
            &target,
            label.as_deref(),
            guid.as_deref(),
            serial.as_deref(),
            dump.as_deref(),
        ) {
            Ok(graph) => {
                collected += graph.nodes.len();
                merge_graph(&mut result.graph, graph);
            }
            Err(error) => failures.push(format!("{target}: {error}")),
        }
    }

    match (collected, failures.is_empty()) {
        (0, false) => result.reports.push(ProbeReport {
            adapter: "exfat".to_string(),
            status: if failures
                .iter()
                .any(|message| message.contains("not found") || message.contains("No such file"))
            {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Partial
            },
            message: Some(failures.join("; ")),
        }),
        (_, false) => result.reports.push(ProbeReport {
            adapter: "exfat".to_string(),
            status: ProbeStatus::Partial,
            message: Some(format!(
                "normalized {collected} graph nodes from exfatprogs; failed targets: {}",
                failures.join("; ")
            )),
        }),
        _ => result.reports.push(ProbeReport {
            adapter: "exfat".to_string(),
            status: ProbeStatus::Available,
            message: Some(format!(
                "normalized {collected} graph nodes from exfatprogs"
            )),
        }),
    }
}

fn collect_ntfs(result: &mut ProbeResult) {
    let targets = filesystem_targets(&result.graph, |filesystem_type| {
        matches!(filesystem_type, "ntfs" | "ntfs3")
    });
    if targets.is_empty() {
        result.reports.push(ProbeReport {
            adapter: "ntfs".to_string(),
            status: if command_exists("ntfsinfo") {
                ProbeStatus::Available
            } else {
                ProbeStatus::Unavailable
            },
            message: Some("no NTFS filesystems discovered".to_string()),
        });
        return;
    }

    let mut collected = 0usize;
    let mut failures = Vec::new();
    for target in targets {
        match run_report("ntfsinfo", &["-m", &target]) {
            Ok(output) => match ntfs::normalize_ntfsinfo(&target, &output) {
                Ok(graph) => {
                    collected += graph.nodes.len();
                    merge_graph(&mut result.graph, graph);
                }
                Err(error) => failures.push(format!("{target}: {error}")),
            },
            Err(message) => failures.push(format!("{target}: {message}")),
        }
    }

    match (collected, failures.is_empty()) {
        (0, false) => result.reports.push(ProbeReport {
            adapter: "ntfs".to_string(),
            status: if failures
                .iter()
                .any(|message| message.contains("not found") || message.contains("No such file"))
            {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Partial
            },
            message: Some(failures.join("; ")),
        }),
        (_, false) => result.reports.push(ProbeReport {
            adapter: "ntfs".to_string(),
            status: ProbeStatus::Partial,
            message: Some(format!(
                "normalized {collected} graph nodes from ntfsinfo; failed targets: {}",
                failures.join("; ")
            )),
        }),
        _ => result.reports.push(ProbeReport {
            adapter: "ntfs".to_string(),
            status: ProbeStatus::Available,
            message: Some(format!("normalized {collected} graph nodes from ntfsinfo")),
        }),
    }
}

fn collect_f2fs(result: &mut ProbeResult) {
    let targets = filesystem_targets(&result.graph, |filesystem_type| filesystem_type == "f2fs");
    if targets.is_empty() {
        result.reports.push(ProbeReport {
            adapter: "f2fs".to_string(),
            status: if command_exists("dump.f2fs") {
                ProbeStatus::Available
            } else {
                ProbeStatus::Unavailable
            },
            message: Some("no F2FS filesystems discovered".to_string()),
        });
        return;
    }

    let mut collected = 0usize;
    let mut failures = Vec::new();
    for target in targets {
        match run_report("dump.f2fs", &[&target]) {
            Ok(output) => match f2fs::normalize_dump_f2fs(&target, &output) {
                Ok(graph) => {
                    collected += graph.nodes.len();
                    merge_graph(&mut result.graph, graph);
                }
                Err(error) => failures.push(format!("{target}: {error}")),
            },
            Err(message) => failures.push(format!("{target}: {message}")),
        }
    }

    match (collected, failures.is_empty()) {
        (0, false) => result.reports.push(ProbeReport {
            adapter: "f2fs".to_string(),
            status: if failures
                .iter()
                .any(|message| message.contains("not found") || message.contains("No such file"))
            {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Partial
            },
            message: Some(failures.join("; ")),
        }),
        (_, false) => result.reports.push(ProbeReport {
            adapter: "f2fs".to_string(),
            status: ProbeStatus::Partial,
            message: Some(format!(
                "normalized {collected} graph nodes from dump.f2fs; failed targets: {}",
                failures.join("; ")
            )),
        }),
        _ => result.reports.push(ProbeReport {
            adapter: "f2fs".to_string(),
            status: ProbeStatus::Available,
            message: Some(format!("normalized {collected} graph nodes from dump.f2fs")),
        }),
    }
}

fn collect_bcachefs(result: &mut ProbeResult) {
    let device_targets = filesystem_targets(&result.graph, |filesystem_type| {
        filesystem_type == "bcachefs"
    });
    let mount_targets = run_findmnt_targets("bcachefs");

    if device_targets.is_empty() && matches!(&mount_targets, Ok(targets) if targets.is_empty()) {
        result.reports.push(ProbeReport {
            adapter: "bcachefs".to_string(),
            status: if command_exists("bcachefs") {
                ProbeStatus::Available
            } else {
                ProbeStatus::Unavailable
            },
            message: Some("no bcachefs filesystems discovered".to_string()),
        });
        return;
    }

    let mut collected = 0usize;
    let mut failures = Vec::new();
    for target in device_targets {
        match run_report("bcachefs", &["show-super", &target]) {
            Ok(output) => match bcachefs::normalize_show_super(&target, &output) {
                Ok(graph) => {
                    collected += graph.nodes.len();
                    merge_graph(&mut result.graph, graph);
                }
                Err(error) => failures.push(format!("{target} show-super: {error}")),
            },
            Err(message) => failures.push(format!("{target} show-super: {message}")),
        }
    }

    match mount_targets {
        Ok(targets) => {
            for target in targets {
                match run_report("bcachefs", &["fs", "usage", &target]) {
                    Ok(output) => match bcachefs::normalize_fs_usage(&target, &output) {
                        Ok(graph) => {
                            collected += graph.nodes.len();
                            merge_graph(&mut result.graph, graph);
                        }
                        Err(error) => failures.push(format!("{target} fs usage: {error}")),
                    },
                    Err(message) => failures.push(format!("{target} fs usage: {message}")),
                }
            }
        }
        Err(message) => failures.push(format!("findmnt bcachefs targets: {message}")),
    }

    match (collected, failures.is_empty()) {
        (0, false) => result.reports.push(ProbeReport {
            adapter: "bcachefs".to_string(),
            status: if failures
                .iter()
                .any(|message| message.contains("not found") || message.contains("No such file"))
            {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Partial
            },
            message: Some(failures.join("; ")),
        }),
        (_, false) => result.reports.push(ProbeReport {
            adapter: "bcachefs".to_string(),
            status: ProbeStatus::Partial,
            message: Some(format!(
                "normalized {collected} graph nodes from bcachefs tools; failed targets: {}",
                failures.join("; ")
            )),
        }),
        _ => result.reports.push(ProbeReport {
            adapter: "bcachefs".to_string(),
            status: ProbeStatus::Available,
            message: Some(format!(
                "normalized {collected} graph nodes from bcachefs tools"
            )),
        }),
    }
}

fn ext_targets(graph: &StorageGraph) -> Vec<String> {
    filesystem_targets(graph, |filesystem_type| {
        matches!(filesystem_type, "ext2" | "ext3" | "ext4")
    })
}

fn filesystem_targets(
    graph: &StorageGraph,
    filesystem_type_matches: impl Fn(&str) -> bool,
) -> Vec<String> {
    let mut filesystem_ids = BTreeSet::new();
    for node in &graph.nodes {
        let is_matching_filesystem = node.properties.iter().any(|property| {
            property.key == "filesystem.type" && filesystem_type_matches(&property.value)
        });
        if !is_matching_filesystem {
            continue;
        }

        if let Some(path) = &node.path {
            if path.starts_with("/dev/") && !path.contains('[') {
                filesystem_ids.insert(path.clone());
            }
        }

        filesystem_ids.insert(node.id.0.clone());
    }

    let mut targets = BTreeSet::new();
    for candidate in filesystem_ids {
        if candidate.starts_with("/dev/") {
            targets.insert(candidate);
            continue;
        }

        for edge in graph.edges.iter().filter(|edge| {
            edge.to.0 == candidate && edge.relationship == disk_nix_model::Relationship::Backs
        }) {
            if let Some(node) = graph.nodes.iter().find(|node| node.id == edge.from) {
                if let Some(path) = &node.path {
                    if path.starts_with("/dev/") && !path.contains('[') {
                        targets.insert(path.clone());
                    }
                }
            }
        }
    }

    targets.into_iter().collect()
}
