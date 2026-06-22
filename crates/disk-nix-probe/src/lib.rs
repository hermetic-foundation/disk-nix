use std::process::Command;

use disk_nix_model::StorageGraph;
use thiserror::Error;

mod btrfs;
mod cryptsetup;
mod dmsetup;
mod findmnt;
mod iscsi;
mod lsblk;
mod lvm;
mod mdraid;
mod multipath;
mod nfs;
mod nvme;
mod vdo;
mod zfs;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeStatus {
    Available,
    Unavailable,
    Partial,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeReport {
    pub adapter: String,
    pub status: ProbeStatus,
    pub message: Option<String>,
}

pub trait ProbeAdapter {
    fn name(&self) -> &'static str;
    fn collect(&self) -> Result<ProbeResult, ProbeError>;
}

#[derive(Debug, Error)]
pub enum ProbeError {
    #[error("probe adapter failed: {0}")]
    Adapter(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeResult {
    pub graph: StorageGraph,
    pub reports: Vec<ProbeReport>,
}

impl ProbeResult {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            graph: StorageGraph::empty(),
            reports: Vec::new(),
        }
    }
}

#[derive(Debug, Default)]
pub struct LinuxProbe;

impl LinuxProbe {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl ProbeAdapter for LinuxProbe {
    fn name(&self) -> &'static str {
        "linux"
    }

    fn collect(&self) -> Result<ProbeResult, ProbeError> {
        let mut result = ProbeResult::empty();

        collect_lsblk(&mut result);
        collect_findmnt(&mut result);
        collect_cryptsetup(&mut result);
        collect_dmsetup(&mut result);
        collect_lvm(&mut result);
        collect_vdo(&mut result);
        collect_zfs(&mut result);
        collect_btrfs(&mut result);
        collect_iscsi(&mut result);
        collect_nfs(&mut result);
        collect_mdraid(&mut result);
        collect_multipath(&mut result);
        collect_nvme(&mut result);
        collect_optional_tools(&mut result);

        Ok(result)
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

fn collect_cryptsetup(result: &mut ProbeResult) {
    let containers: Vec<String> = result
        .graph
        .nodes
        .iter()
        .filter(|node| node.kind == disk_nix_model::NodeKind::LuksContainer)
        .map(|node| node.path.clone().unwrap_or_else(|| node.name.clone()))
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
    for container in containers {
        let status_arg = cryptsetup_status_arg(&container);
        match run_report("cryptsetup", &["status", &status_arg]) {
            Ok(output) => match cryptsetup::normalize_cryptsetup_status(&container, &output) {
                Ok(graph) => {
                    collected += graph.nodes.len();
                    merge_graph(&mut result.graph, graph);
                }
                Err(error) => {
                    result.reports.push(ProbeReport {
                        adapter: "cryptsetup".to_string(),
                        status: ProbeStatus::Failed,
                        message: Some(error.to_string()),
                    });
                    return;
                }
            },
            Err(message) => {
                result.reports.push(ProbeReport {
                    adapter: "cryptsetup".to_string(),
                    status: if message.contains("not found") || message.contains("No such file") {
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

    result.reports.push(ProbeReport {
        adapter: "cryptsetup".to_string(),
        status: ProbeStatus::Available,
        message: Some(format!(
            "normalized {collected} graph nodes from cryptsetup status"
        )),
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

    match (info, deps) {
        (Ok(info), Ok(deps)) if info.is_empty() && deps.is_empty() => {
            result.reports.push(ProbeReport {
                adapter: "dmsetup".to_string(),
                status: ProbeStatus::Available,
                message: Some("no device-mapper devices discovered".to_string()),
            });
        }
        (Ok(info), Ok(deps)) => match dmsetup::normalize_dmsetup(&info, &deps) {
            Ok(graph) => {
                let node_count = graph.nodes.len();
                merge_graph(&mut result.graph, graph);
                result.reports.push(ProbeReport {
                    adapter: "dmsetup".to_string(),
                    status: ProbeStatus::Available,
                    message: Some(format!("normalized {node_count} graph nodes from dmsetup")),
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

fn collect_nvme(result: &mut ProbeResult) {
    match run_report("nvme", &["list", "-o", "json"]) {
        Ok(output) => match nvme::normalize_nvme_list_json(&output) {
            Ok(graph) => {
                let node_count = graph.nodes.len();
                merge_graph(&mut result.graph, graph);
                result.reports.push(ProbeReport {
                    adapter: "nvme".to_string(),
                    status: ProbeStatus::Available,
                    message: Some(format!(
                        "normalized {node_count} graph nodes from NVMe JSON"
                    )),
                });
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
        let subvolumes = run_report("btrfs", &["subvolume", "list", "-u", &target]);

        match (show, usage, subvolumes) {
            (Ok(show), Ok(usage), Ok(subvolumes)) => reports.push(btrfs::BtrfsReport {
                target,
                show,
                usage,
                subvolumes,
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
        &["list", "-H", "-p", "-o", "name,size,alloc,free,health"],
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
            "name,type,used,available,referenced,mountpoint,origin",
        ],
    );

    match (zpool_list, zfs_list) {
        (Ok(zpool_list), Ok(zfs_list)) => match zfs::normalize_zfs(&zpool_list, &zfs_list) {
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
        },
        (Err(message), _) | (_, Err(message)) => {
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

fn collect_lvm(result: &mut ProbeResult) {
    let pvs = run_report(
        "pvs",
        &[
            "--reportformat",
            "json",
            "-o",
            "pv_name,vg_name,pv_uuid,pv_size,pv_free,pv_used",
        ],
    );
    let vgs = run_report(
        "vgs",
        &[
            "--reportformat",
            "json",
            "-o",
            "vg_name,vg_uuid,vg_size,vg_free,vg_extent_size,pv_count,lv_count",
        ],
    );
    let lvs = run_report(
        "lvs",
        &[
            "--reportformat",
            "json",
            "-o",
            "lv_name,vg_name,lv_uuid,lv_path,lv_size,lv_attr,origin,pool_lv,data_percent,metadata_percent",
        ],
    );

    match (pvs, vgs, lvs) {
        (Ok(pvs), Ok(vgs), Ok(lvs)) => match lvm::normalize_lvm_json(&pvs, &vgs, &lvs) {
            Ok(graph) => {
                let node_count = graph.nodes.len();
                merge_graph(&mut result.graph, graph);
                result.reports.push(ProbeReport {
                    adapter: "lvm".to_string(),
                    status: ProbeStatus::Available,
                    message: Some(format!("normalized {node_count} graph nodes from LVM JSON")),
                });
            }
            Err(error) => result.reports.push(ProbeReport {
                adapter: "lvm".to_string(),
                status: ProbeStatus::Failed,
                message: Some(error.to_string()),
            }),
        },
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

fn collect_findmnt(result: &mut ProbeResult) {
    match Command::new("findmnt").args(["--json", "--bytes"]).output() {
        Ok(output) if output.status.success() => {
            match findmnt::normalize_findmnt_json(&output.stdout) {
                Ok(graph) => {
                    let node_count = graph.nodes.len();
                    merge_graph(&mut result.graph, graph);
                    result.reports.push(findmnt::available_report(node_count));
                }
                Err(error) => result.reports.push(ProbeReport {
                    adapter: "findmnt".to_string(),
                    status: ProbeStatus::Failed,
                    message: Some(error.to_string()),
                }),
            }
        }
        Ok(output) => result.reports.push(ProbeReport {
            adapter: "findmnt".to_string(),
            status: ProbeStatus::Failed,
            message: Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
        }),
        Err(error) => result.reports.push(ProbeReport {
            adapter: "findmnt".to_string(),
            status: ProbeStatus::Unavailable,
            message: Some(error.to_string()),
        }),
    }
}

fn parse_lines(bytes: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn collect_optional_tools(result: &mut ProbeResult) {
    for tool in ["vdostats"] {
        let status = if command_exists(tool) {
            ProbeStatus::Available
        } else {
            ProbeStatus::Unavailable
        };

        result.reports.push(ProbeReport {
            adapter: tool.to_string(),
            status,
            message: None,
        });
    }
}

fn command_exists(tool: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v -- {tool} >/dev/null 2>&1"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_probe_result_has_empty_graph_and_reports() {
        let result = ProbeResult::empty();
        assert!(result.graph.nodes.is_empty());
        assert!(result.reports.is_empty());
    }
}
