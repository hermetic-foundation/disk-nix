use std::process::Command;

use disk_nix_model::{Node, NodeKind, StorageGraph};
use thiserror::Error;

mod findmnt;
mod lsblk;
mod lvm;

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
        collect_lvm(&mut result);
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

fn merge_graph(target: &mut StorageGraph, source: StorageGraph) {
    for node in source.nodes {
        target.add_node(node);
    }
    for edge in source.edges {
        target.add_edge(edge);
    }
}

fn collect_lvm(result: &mut ProbeResult) {
    let pvs = run_lvm_report(
        "pvs",
        &[
            "--reportformat",
            "json",
            "-o",
            "pv_name,vg_name,pv_uuid,pv_size,pv_free,pv_used",
        ],
    );
    let vgs = run_lvm_report(
        "vgs",
        &[
            "--reportformat",
            "json",
            "-o",
            "vg_name,vg_uuid,vg_size,vg_free,vg_extent_size,pv_count,lv_count",
        ],
    );
    let lvs = run_lvm_report(
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

fn run_lvm_report(command: &str, args: &[&str]) -> Result<Vec<u8>, String> {
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

fn collect_optional_tools(result: &mut ProbeResult) {
    for tool in [
        "cryptsetup",
        "dmsetup",
        "mdadm",
        "btrfs",
        "zfs",
        "zpool",
        "iscsiadm",
        "nfsstat",
        "multipath",
        "nvme",
        "vdo",
        "vdostats",
    ] {
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
