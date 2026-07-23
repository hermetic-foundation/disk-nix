fn collect_xfs(result: &mut ProbeResult) {
    match run_findmnt_targets("xfs") {
        Ok(targets) if targets.is_empty() => result.reports.push(ProbeReport {
            adapter: "xfs".to_string(),
            status: if command_exists("xfs_info") {
                ProbeStatus::Available
            } else {
                ProbeStatus::Unavailable
            },
            message: Some("no mounted XFS filesystems discovered".to_string()),
        }),
        Ok(targets) => {
            let mut collected = 0usize;
            let mut failures = Vec::new();
            for target in targets {
                match run_report("xfs_info", &[&target]) {
                    Ok(output) => match xfs::normalize_xfs_info(&target, &output) {
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
                    adapter: "xfs".to_string(),
                    status: if failures.iter().any(|message| {
                        message.contains("not found") || message.contains("No such file")
                    }) {
                        ProbeStatus::Unavailable
                    } else {
                        ProbeStatus::Partial
                    },
                    message: Some(failures.join("; ")),
                }),
                (_, false) => result.reports.push(ProbeReport {
                    adapter: "xfs".to_string(),
                    status: ProbeStatus::Partial,
                    message: Some(format!(
                        "normalized {collected} graph nodes from xfs_info; failed targets: {}",
                        failures.join("; ")
                    )),
                }),
                _ => result.reports.push(ProbeReport {
                    adapter: "xfs".to_string(),
                    status: ProbeStatus::Available,
                    message: Some(format!("normalized {collected} graph nodes from xfs_info")),
                }),
            }
        }
        Err(message) => result.reports.push(ProbeReport {
            adapter: "xfs".to_string(),
            status: if message.contains("not found") || message.contains("No such file") {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Partial
            },
            message: Some(message),
        }),
    }
}

fn collect_swaps(result: &mut ProbeResult) {
    match std::fs::read("/proc/swaps") {
        Ok(output) => match swaps::normalize_proc_swaps(&output) {
            Ok(graph) if graph.nodes.is_empty() => result.reports.push(ProbeReport {
                adapter: "swaps".to_string(),
                status: ProbeStatus::Available,
                message: Some("no active swap devices discovered".to_string()),
            }),
            Ok(graph) => {
                let node_count = graph.nodes.len();
                merge_graph(&mut result.graph, graph);
                result.reports.push(ProbeReport {
                    adapter: "swaps".to_string(),
                    status: ProbeStatus::Available,
                    message: Some(format!(
                        "normalized {node_count} graph nodes from /proc/swaps"
                    )),
                });
            }
            Err(error) => result.reports.push(ProbeReport {
                adapter: "swaps".to_string(),
                status: ProbeStatus::Failed,
                message: Some(error.to_string()),
            }),
        },
        Err(error) => result.reports.push(ProbeReport {
            adapter: "swaps".to_string(),
            status: ProbeStatus::Unavailable,
            message: Some(error.to_string()),
        }),
    }
}

fn collect_zram(result: &mut ProbeResult) {
    match run_report(
        "zramctl",
        &["--bytes", "--raw", "--noheadings", "--output-all"],
    ) {
        Ok(output) => match zram::normalize_zramctl_output(&output) {
            Ok(graph) if graph.nodes.is_empty() => result.reports.push(ProbeReport {
                adapter: "zramctl".to_string(),
                status: ProbeStatus::Available,
                message: Some("no zram devices discovered".to_string()),
            }),
            Ok(graph) => {
                let node_count = graph.nodes.len();
                merge_graph(&mut result.graph, graph);
                result.reports.push(ProbeReport {
                    adapter: "zramctl".to_string(),
                    status: ProbeStatus::Available,
                    message: Some(format!("normalized {node_count} graph nodes from zramctl")),
                });
            }
            Err(error) => result.reports.push(ProbeReport {
                adapter: "zramctl".to_string(),
                status: ProbeStatus::Failed,
                message: Some(error.to_string()),
            }),
        },
        Err(message) => result.reports.push(ProbeReport {
            adapter: "zramctl".to_string(),
            status: if message.contains("not found") || message.contains("No such file") {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Partial
            },
            message: Some(message),
        }),
    }
}

fn collect_loopdev(result: &mut ProbeResult) {
    match run_report("losetup", &["--json", "--list"]) {
        Ok(output) => match loopdev::normalize_losetup_json(&output) {
            Ok(graph) if graph.nodes.is_empty() => result.reports.push(ProbeReport {
                adapter: "loop".to_string(),
                status: ProbeStatus::Available,
                message: Some("no loop devices discovered".to_string()),
            }),
            Ok(graph) => {
                let node_count = graph.nodes.len();
                merge_graph(&mut result.graph, graph);
                result.reports.push(ProbeReport {
                    adapter: "loop".to_string(),
                    status: ProbeStatus::Available,
                    message: Some(format!(
                        "normalized {node_count} graph nodes from losetup JSON"
                    )),
                });
            }
            Err(error) => result.reports.push(ProbeReport {
                adapter: "loop".to_string(),
                status: ProbeStatus::Failed,
                message: Some(error.to_string()),
            }),
        },
        Err(message) => result.reports.push(ProbeReport {
            adapter: "loop".to_string(),
            status: if message.contains("not found") || message.contains("No such file") {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Partial
            },
            message: Some(message),
        }),
    }
}

fn collect_cryptsetup(result: &mut ProbeResult) {
    let containers: Vec<(String, bool)> = result
        .graph
        .nodes
        .iter()
        .filter(|node| node.kind == disk_nix_model::NodeKind::LuksContainer)
        .map(|node| {
            (
                node.path.clone().unwrap_or_else(|| node.name.clone()),
                node.properties.iter().any(|property| {
                    property.key == "blkid.type" && property.value == "crypto_LUKS"
                }),
            )
        })
        .collect();

    if containers.is_empty() {
        result.reports.push(ProbeReport {
            adapter: "cryptsetup".to_string(),
            status: if command_exists("cryptsetup") {
                ProbeStatus::Available
            } else {
                ProbeStatus::Unavailable
            },
            message: Some("no LUKS containers discovered".to_string()),
        });
        return;
    }

    let mut collected = 0usize;
    let mut partials = Vec::new();
    for (container, is_luks_device) in containers {
        if is_luks_device {
            match run_report("cryptsetup", &["luksDump", &container]) {
                Ok(output) => match cryptsetup::normalize_luks_dump(&container, &output) {
                    Ok(graph) => {
                        collected += graph.nodes.len();
                        merge_graph(&mut result.graph, graph);
                    }
                    Err(error) => partials.push(error.to_string()),
                },
                Err(message) => partials.push(message),
            }
        }

        if !container.starts_with("/dev/mapper/") {
            continue;
        }

        let status_arg = cryptsetup_status_arg(&container);
        match run_report("cryptsetup", &["status", &status_arg]) {
            Ok(output) => match cryptsetup::normalize_cryptsetup_status(&container, &output) {
                Ok(graph) => {
                    collected += graph.nodes.len();
                    merge_graph(&mut result.graph, graph);
                }
                Err(error) => partials.push(error.to_string()),
            },
            Err(message) => partials.push(message),
        }
    }

    let status = if collected == 0
        && partials
            .iter()
            .any(|message| message.contains("not found") || message.contains("No such file"))
    {
        ProbeStatus::Unavailable
    } else if partials.is_empty() {
        ProbeStatus::Available
    } else {
        ProbeStatus::Partial
    };
    let message = if partials.is_empty() {
        format!("normalized {collected} graph nodes from cryptsetup status and luksDump")
    } else {
        format!(
            "normalized {collected} graph nodes from cryptsetup status and luksDump; partial errors: {}",
            partials.join("; ")
        )
    };

    result.reports.push(ProbeReport {
        adapter: "cryptsetup".to_string(),
        status,
        message: Some(message),
    });
}

fn cryptsetup_status_arg(container: &str) -> String {
    container
        .strip_prefix("/dev/mapper/")
        .unwrap_or(container)
        .to_string()
}

fn collect_dmsetup(result: &mut ProbeResult) {
    let info = run_report(
        "dmsetup",
        &[
            "info",
            "-c",
            "--noheadings",
            "--separator",
            "|",
            "-o",
            "name,uuid,major,minor,open,segments,events",
        ],
    );
    let deps = run_report("dmsetup", &["deps", "-o", "devname"]);
    let table = run_report("dmsetup", &["table"]);
    let status = run_report("dmsetup", &["status"]);

    match (info, deps) {
        (Ok(info), Ok(deps)) if info.is_empty() && deps.is_empty() => {
            result.reports.push(ProbeReport {
                adapter: "dmsetup".to_string(),
                status: ProbeStatus::Available,
                message: Some("no device-mapper devices discovered".to_string()),
            });
        }
        (Ok(info), Ok(deps)) => match dmsetup::normalize_dmsetup(
            &info,
            &deps,
            table.as_deref().ok(),
            status.as_deref().ok(),
        ) {
            Ok(graph) => {
                let node_count = graph.nodes.len();
                merge_graph(&mut result.graph, graph);
                result.reports.push(ProbeReport {
                    adapter: "dmsetup".to_string(),
                    status: if table.is_ok() && status.is_ok() {
                        ProbeStatus::Available
                    } else {
                        ProbeStatus::Partial
                    },
                    message: Some(format!(
                        "normalized {node_count} graph nodes from dmsetup{}",
                        dmsetup_partial_message(&table, &status)
                    )),
                });
            }
            Err(error) => result.reports.push(ProbeReport {
                adapter: "dmsetup".to_string(),
                status: ProbeStatus::Failed,
                message: Some(error.to_string()),
            }),
        },
        (Err(message), _) | (_, Err(message)) => result.reports.push(ProbeReport {
            adapter: "dmsetup".to_string(),
            status: if message.contains("not found") || message.contains("No such file") {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Partial
            },
            message: Some(message),
        }),
    }
}

fn dmsetup_partial_message(
    table: &Result<Vec<u8>, String>,
    status: &Result<Vec<u8>, String>,
) -> String {
    let mut failures = Vec::new();
    if let Err(message) = table {
        failures.push(format!("table: {message}"));
    }
    if let Err(message) = status {
        failures.push(format!("status: {message}"));
    }
    if failures.is_empty() {
        String::new()
    } else {
        format!("; partial errors: {}", failures.join("; "))
    }
}
