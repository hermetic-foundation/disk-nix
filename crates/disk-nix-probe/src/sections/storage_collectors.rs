fn merge_graph(target: &mut StorageGraph, source: StorageGraph) {
    for node in source.nodes {
        target.add_node(node);
    }
    for edge in source.edges {
        target.add_edge(edge);
    }
}

fn collect_vdo(result: &mut ProbeResult) {
    match run_report("vdo", &["status"]) {
        Ok(output) if output.is_empty() => result.reports.push(ProbeReport {
            adapter: "vdo".to_string(),
            status: ProbeStatus::Available,
            message: Some("no VDO volumes discovered".to_string()),
        }),
        Ok(output) => match vdo::normalize_vdo_status(&output) {
            Ok(graph) => {
                let node_count = graph.nodes.len();
                merge_graph(&mut result.graph, graph);
                result.reports.push(ProbeReport {
                    adapter: "vdo".to_string(),
                    status: ProbeStatus::Available,
                    message: Some(format!(
                        "normalized {node_count} graph nodes from VDO status"
                    )),
                });
            }
            Err(error) => result.reports.push(ProbeReport {
                adapter: "vdo".to_string(),
                status: ProbeStatus::Failed,
                message: Some(error.to_string()),
            }),
        },
        Err(message) => result.reports.push(ProbeReport {
            adapter: "vdo".to_string(),
            status: if message.contains("not found") || message.contains("No such file") {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Partial
            },
            message: Some(message),
        }),
    }
}

fn collect_vdostats(result: &mut ProbeResult) {
    match run_report("vdostats", &["--human-readable"]) {
        Ok(output) => match vdo::normalize_vdostats_table(&output) {
            Ok(graph) if graph.nodes.is_empty() => result.reports.push(ProbeReport {
                adapter: "vdostats".to_string(),
                status: ProbeStatus::Available,
                message: Some("no VDO statistics discovered".to_string()),
            }),
            Ok(graph) => {
                let node_count = graph.nodes.len();
                merge_graph(&mut result.graph, graph);
                result.reports.push(ProbeReport {
                    adapter: "vdostats".to_string(),
                    status: ProbeStatus::Available,
                    message: Some(format!(
                        "normalized {node_count} graph nodes from VDO statistics"
                    )),
                });
            }
            Err(error) => result.reports.push(ProbeReport {
                adapter: "vdostats".to_string(),
                status: ProbeStatus::Failed,
                message: Some(error.to_string()),
            }),
        },
        Err(message) => result.reports.push(ProbeReport {
            adapter: "vdostats".to_string(),
            status: if message.contains("not found") || message.contains("No such file") {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Partial
            },
            message: Some(message),
        }),
    }
}

fn collect_vdostats_verbose(result: &mut ProbeResult) {
    match run_report("vdostats", &["--verbose"]) {
        Ok(output) => match vdo::normalize_vdostats_verbose(&output) {
            Ok(graph) if graph.nodes.is_empty() => result.reports.push(ProbeReport {
                adapter: "vdostats-verbose".to_string(),
                status: ProbeStatus::Available,
                message: Some("no verbose VDO statistics discovered".to_string()),
            }),
            Ok(graph) => {
                let node_count = graph.nodes.len();
                merge_graph(&mut result.graph, graph);
                result.reports.push(ProbeReport {
                    adapter: "vdostats-verbose".to_string(),
                    status: ProbeStatus::Available,
                    message: Some(format!(
                        "normalized {node_count} graph nodes from verbose VDO statistics"
                    )),
                });
            }
            Err(error) => result.reports.push(ProbeReport {
                adapter: "vdostats-verbose".to_string(),
                status: ProbeStatus::Failed,
                message: Some(error.to_string()),
            }),
        },
        Err(message) => result.reports.push(ProbeReport {
            adapter: "vdostats-verbose".to_string(),
            status: if message.contains("not found") || message.contains("No such file") {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Partial
            },
            message: Some(message),
        }),
    }
}

fn collect_nvme(result: &mut ProbeResult) {
    match run_report("nvme", &["list", "-o", "json"]) {
        Ok(output) => match nvme::normalize_nvme_list_json(&output) {
            Ok(graph) => {
                let namespace_paths: Vec<String> = graph
                    .nodes
                    .iter()
                    .filter(|node| node.kind == disk_nix_model::NodeKind::NvmeNamespace)
                    .filter_map(|node| node.path.clone())
                    .collect();
                let controllers: Vec<String> = graph
                    .nodes
                    .iter()
                    .flat_map(|node| node.properties.iter())
                    .filter(|property| property.key == "nvme.controller")
                    .map(|property| property.value.clone())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect();
                let node_count = graph.nodes.len();
                merge_graph(&mut result.graph, graph);
                result.reports.push(ProbeReport {
                    adapter: "nvme".to_string(),
                    status: ProbeStatus::Available,
                    message: Some(format!(
                        "normalized {node_count} graph nodes from NVMe JSON"
                    )),
                });
                collect_nvme_subsystems(result);
                collect_nvme_namespace_details(result, namespace_paths);
                collect_nvme_controller_details(result, controllers.clone());
                collect_nvme_smart_logs(result, controllers);
            }
            Err(error) => result.reports.push(ProbeReport {
                adapter: "nvme".to_string(),
                status: ProbeStatus::Failed,
                message: Some(error.to_string()),
            }),
        },
        Err(message) => result.reports.push(ProbeReport {
            adapter: "nvme".to_string(),
            status: if message.contains("not found") || message.contains("No such file") {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Partial
            },
            message: Some(message),
        }),
    }
}

fn collect_nvme_subsystems(result: &mut ProbeResult) {
    match run_report("nvme", &["list-subsys", "-o", "json"]) {
        Ok(output) => match nvme::normalize_nvme_subsystems_json(&output) {
            Ok(graph) => {
                let node_count = graph.nodes.len();
                merge_graph(&mut result.graph, graph);
                result.reports.push(ProbeReport {
                    adapter: "nvme-list-subsys".to_string(),
                    status: ProbeStatus::Available,
                    message: Some(format!(
                        "normalized {node_count} graph nodes from NVMe subsystem JSON"
                    )),
                });
            }
            Err(error) => result.reports.push(ProbeReport {
                adapter: "nvme-list-subsys".to_string(),
                status: ProbeStatus::Failed,
                message: Some(error.to_string()),
            }),
        },
        Err(message) => result.reports.push(ProbeReport {
            adapter: "nvme-list-subsys".to_string(),
            status: if message.contains("not found") || message.contains("No such file") {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Partial
            },
            message: Some(message),
        }),
    }
}

fn collect_nvme_smart_logs(result: &mut ProbeResult, controllers: Vec<String>) {
    let mut node_count = 0_usize;
    let mut failures = Vec::new();
    for controller in controllers {
        let controller_path = if controller.starts_with("/dev/") {
            controller.clone()
        } else {
            format!("/dev/{controller}")
        };
        match run_report(
            "nvme",
            &["smart-log", controller_path.as_str(), "-o", "json"],
        ) {
            Ok(output) => match nvme::normalize_nvme_smart_log_json(&controller_path, &output) {
                Ok(graph) => {
                    node_count += graph.nodes.len();
                    merge_graph(&mut result.graph, graph);
                }
                Err(error) => failures.push(format!("{controller_path}: {error}")),
            },
            Err(message) => failures.push(format!("{controller_path}: {message}")),
        }
    }

    if node_count > 0 {
        result.reports.push(ProbeReport {
            adapter: "nvme-smart-log".to_string(),
            status: if failures.is_empty() {
                ProbeStatus::Available
            } else {
                ProbeStatus::Partial
            },
            message: Some(format!(
                "normalized {node_count} graph nodes from NVMe SMART log JSON{}",
                if failures.is_empty() {
                    String::new()
                } else {
                    format!("; {} SMART probes failed", failures.len())
                }
            )),
        });
    } else if !failures.is_empty() {
        result.reports.push(ProbeReport {
            adapter: "nvme-smart-log".to_string(),
            status: ProbeStatus::Partial,
            message: Some(format!(
                "NVMe SMART log probes failed: {}",
                failures.join("; ")
            )),
        });
    }
}

fn collect_nvme_controller_details(result: &mut ProbeResult, controllers: Vec<String>) {
    let mut node_count = 0_usize;
    let mut failures = Vec::new();
    for controller in controllers {
        let controller_path = if controller.starts_with("/dev/") {
            controller.clone()
        } else {
            format!("/dev/{controller}")
        };
        match run_report("nvme", &["id-ctrl", controller_path.as_str(), "-o", "json"]) {
            Ok(output) => match nvme::normalize_nvme_id_ctrl_json(&controller_path, &output) {
                Ok(graph) => {
                    node_count += graph.nodes.len();
                    merge_graph(&mut result.graph, graph);
                }
                Err(error) => failures.push(format!("{controller_path}: {error}")),
            },
            Err(message) => failures.push(format!("{controller_path}: {message}")),
        }
    }

    if node_count > 0 {
        result.reports.push(ProbeReport {
            adapter: "nvme-id-ctrl".to_string(),
            status: if failures.is_empty() {
                ProbeStatus::Available
            } else {
                ProbeStatus::Partial
            },
            message: Some(format!(
                "normalized {node_count} graph nodes from NVMe controller identity JSON{}",
                if failures.is_empty() {
                    String::new()
                } else {
                    format!("; {} controller probes failed", failures.len())
                }
            )),
        });
    } else if !failures.is_empty() {
        result.reports.push(ProbeReport {
            adapter: "nvme-id-ctrl".to_string(),
            status: ProbeStatus::Partial,
            message: Some(format!(
                "NVMe controller identity probes failed: {}",
                failures.join("; ")
            )),
        });
    }
}

fn collect_nvme_namespace_details(result: &mut ProbeResult, namespace_paths: Vec<String>) {
    let mut node_count = 0_usize;
    let mut failures = Vec::new();
    for path in namespace_paths {
        match run_report("nvme", &["id-ns", path.as_str(), "-o", "json"]) {
            Ok(output) => match nvme::normalize_nvme_id_ns_json(&path, &output) {
                Ok(graph) => {
                    node_count += graph.nodes.len();
                    merge_graph(&mut result.graph, graph);
                }
                Err(error) => failures.push(format!("{path}: {error}")),
            },
            Err(message) => failures.push(format!("{path}: {message}")),
        }
    }

    if node_count > 0 {
        result.reports.push(ProbeReport {
            adapter: "nvme-id-ns".to_string(),
            status: if failures.is_empty() {
                ProbeStatus::Available
            } else {
                ProbeStatus::Partial
            },
            message: Some(format!(
                "normalized {node_count} graph nodes from NVMe namespace identity JSON{}",
                if failures.is_empty() {
                    String::new()
                } else {
                    format!("; {} namespace probes failed", failures.len())
                }
            )),
        });
    } else if !failures.is_empty() {
        result.reports.push(ProbeReport {
            adapter: "nvme-id-ns".to_string(),
            status: ProbeStatus::Partial,
            message: Some(format!(
                "NVMe namespace identity probes failed: {}",
                failures.join("; ")
            )),
        });
    }
}

fn collect_multipath(result: &mut ProbeResult) {
    match run_report("multipath", &["-ll"]) {
        Ok(output) if output.is_empty() => result.reports.push(ProbeReport {
            adapter: "multipath".to_string(),
            status: ProbeStatus::Available,
            message: Some("no multipath maps discovered".to_string()),
        }),
        Ok(output) => match multipath::normalize_multipath_output(&output) {
            Ok(graph) => {
                let node_count = graph.nodes.len();
                merge_graph(&mut result.graph, graph);
                result.reports.push(ProbeReport {
                    adapter: "multipath".to_string(),
                    status: ProbeStatus::Available,
                    message: Some(format!(
                        "normalized {node_count} graph nodes from multipath maps"
                    )),
                });
            }
            Err(error) => result.reports.push(ProbeReport {
                adapter: "multipath".to_string(),
                status: ProbeStatus::Failed,
                message: Some(error.to_string()),
            }),
        },
        Err(message) => result.reports.push(ProbeReport {
            adapter: "multipath".to_string(),
            status: if message.contains("not found") || message.contains("No such file") {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Partial
            },
            message: Some(message),
        }),
    }
}

fn collect_mdraid(result: &mut ProbeResult) {
    match fs::read("/proc/mdstat") {
        Ok(mdstat) => match mdraid::normalize_mdstat(&mdstat) {
            Ok(graph) if !graph.nodes.is_empty() => {
                let node_count = graph.nodes.len();
                merge_graph(&mut result.graph, graph);
                result.reports.push(ProbeReport {
                    adapter: "mdstat".to_string(),
                    status: ProbeStatus::Available,
                    message: Some(format!(
                        "normalized {node_count} graph nodes from /proc/mdstat"
                    )),
                });
            }
            Ok(_) => result.reports.push(ProbeReport {
                adapter: "mdstat".to_string(),
                status: ProbeStatus::Available,
                message: Some("no MD RAID arrays reported by /proc/mdstat".to_string()),
            }),
            Err(error) => result.reports.push(ProbeReport {
                adapter: "mdstat".to_string(),
                status: ProbeStatus::Failed,
                message: Some(error.to_string()),
            }),
        },
        Err(error) => result.reports.push(ProbeReport {
            adapter: "mdstat".to_string(),
            status: ProbeStatus::Partial,
            message: Some(format!("failed to read /proc/mdstat: {error}")),
        }),
    }

    let scan = match run_report("mdadm", &["--detail", "--scan"]) {
        Ok(scan) => scan,
        Err(message) => {
            result.reports.push(ProbeReport {
                adapter: "mdadm".to_string(),
                status: if message.contains("not found") || message.contains("No such file") {
                    ProbeStatus::Unavailable
                } else {
                    ProbeStatus::Partial
                },
                message: Some(message),
            });
            return;
        }
    };

    match mdraid::normalize_md_scan(&scan) {
        Ok(graph) if !graph.nodes.is_empty() => {
            let node_count = graph.nodes.len();
            merge_graph(&mut result.graph, graph);
            result.reports.push(ProbeReport {
                adapter: "mdadm-scan".to_string(),
                status: ProbeStatus::Available,
                message: Some(format!(
                    "normalized {node_count} graph nodes from MD RAID detail scan"
                )),
            });
        }
        Ok(_) => {}
        Err(error) => result.reports.push(ProbeReport {
            adapter: "mdadm-scan".to_string(),
            status: ProbeStatus::Failed,
            message: Some(error.to_string()),
        }),
    }

    match run_report("mdadm", &["--examine", "--scan"]) {
        Ok(examine_scan) => match mdraid::normalize_md_scan(&examine_scan) {
            Ok(graph) if !graph.nodes.is_empty() => {
                let node_count = graph.nodes.len();
                merge_graph(&mut result.graph, graph);
                result.reports.push(ProbeReport {
                    adapter: "mdadm-examine".to_string(),
                    status: ProbeStatus::Available,
                    message: Some(format!(
                        "normalized {node_count} graph nodes from MD RAID examine scan"
                    )),
                });
            }
            Ok(_) => {}
            Err(error) => result.reports.push(ProbeReport {
                adapter: "mdadm-examine".to_string(),
                status: ProbeStatus::Failed,
                message: Some(error.to_string()),
            }),
        },
        Err(message) => result.reports.push(ProbeReport {
            adapter: "mdadm-examine".to_string(),
            status: if message.contains("not found") || message.contains("No such file") {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Partial
            },
            message: Some(message),
        }),
    }

    let arrays = match mdraid::arrays_from_scan(&scan) {
        Ok(arrays) => arrays,
        Err(error) => {
            result.reports.push(ProbeReport {
                adapter: "mdadm".to_string(),
                status: ProbeStatus::Failed,
                message: Some(error.to_string()),
            });
            return;
        }
    };

    if arrays.is_empty() {
        result.reports.push(ProbeReport {
            adapter: "mdadm".to_string(),
            status: ProbeStatus::Available,
            message: Some("no MD RAID arrays discovered".to_string()),
        });
        return;
    }

    let mut reports = Vec::new();
    for array in arrays {
        match run_report("mdadm", &["--detail", &array]) {
            Ok(detail) => reports.push(mdraid::MdArrayReport {
                name: array,
                detail,
            }),
            Err(message) => {
                result.reports.push(ProbeReport {
                    adapter: "mdadm".to_string(),
                    status: ProbeStatus::Partial,
                    message: Some(message),
                });
                return;
            }
        }
    }

    match mdraid::normalize_md_arrays(&reports) {
        Ok(graph) => {
            let node_count = graph.nodes.len();
            merge_graph(&mut result.graph, graph);
            result.reports.push(ProbeReport {
                adapter: "mdadm".to_string(),
                status: ProbeStatus::Available,
                message: Some(format!(
                    "normalized {node_count} graph nodes from MD RAID arrays"
                )),
            });
        }
        Err(error) => result.reports.push(ProbeReport {
            adapter: "mdadm".to_string(),
            status: ProbeStatus::Failed,
            message: Some(error.to_string()),
        }),
    }
}
