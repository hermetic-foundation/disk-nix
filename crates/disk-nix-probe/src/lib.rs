use std::{collections::BTreeSet, fs, process::Command};

use disk_nix_model::StorageGraph;
use serde::{Serialize, ser::SerializeStruct};
use thiserror::Error;

mod bcache;
mod bcachefs;
mod blkid;
mod btrfs;
mod cryptsetup;
mod dmsetup;
mod exfat;
mod ext;
mod f2fs;
mod findmnt;
mod iscsi;
mod loopdev;
mod lsblk;
mod lsscsi;
mod lvm;
mod mdraid;
mod multipath;
mod nfs;
mod ntfs;
mod nvme;
mod parted;
mod smartctl;
mod swaps;
mod udev;
mod vdo;
mod xfs;
mod zfs;
mod zram;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProbeStatus {
    Available,
    Unavailable,
    Partial,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProbeIssueCategory {
    None,
    MissingTool,
    PermissionDenied,
    CommandFailed,
    ParseFailed,
    InaccessibleData,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeReport {
    pub adapter: String,
    pub status: ProbeStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeAdapterRemediation {
    pub adapter: String,
    pub canonical_adapter: String,
    pub tools: Vec<String>,
    pub nix_packages: Vec<String>,
    pub privilege_hint: String,
    pub data_hint: String,
    pub parse_hint: String,
    pub command_hint: String,
}

impl ProbeReport {
    #[must_use]
    pub fn category(&self) -> ProbeIssueCategory {
        match self.status {
            ProbeStatus::Available => ProbeIssueCategory::None,
            ProbeStatus::Unavailable | ProbeStatus::Partial | ProbeStatus::Failed => self
                .message
                .as_deref()
                .map(|message| probe_category_for_status(&self.status, message))
                .unwrap_or(ProbeIssueCategory::InaccessibleData),
        }
    }

    #[must_use]
    pub fn remediation(&self) -> Vec<String> {
        remediation_for_category(&self.adapter, self.category())
    }
}

#[must_use]
pub fn adapter_remediation(adapter: &str) -> ProbeAdapterRemediation {
    let canonical_adapter = canonical_adapter(adapter);
    ProbeAdapterRemediation {
        adapter: adapter.to_string(),
        canonical_adapter: canonical_adapter.to_string(),
        tools: adapter_tools(adapter)
            .into_iter()
            .map(ToString::to_string)
            .collect(),
        nix_packages: adapter_nix_packages(adapter)
            .into_iter()
            .map(ToString::to_string)
            .collect(),
        privilege_hint: adapter_privilege_hint(adapter),
        data_hint: adapter_data_hint(adapter),
        parse_hint: adapter_parse_hint(adapter),
        command_hint: adapter_command_hint(adapter),
    }
}

impl Serialize for ProbeReport {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let remediation = self.remediation();
        let mut state = serializer.serialize_struct("ProbeReport", 5)?;
        state.serialize_field("adapter", &self.adapter)?;
        state.serialize_field("status", &self.status)?;
        state.serialize_field("category", &self.category())?;
        state.serialize_field("remediation", &remediation)?;
        state.serialize_field("message", &self.message)?;
        state.end()
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
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
        collect_lsscsi(&mut result);
        collect_smartctl(&mut result);
        collect_blkid(&mut result);
        collect_parted(&mut result);
        collect_udev(&mut result);
        collect_findmnt(&mut result);
        collect_ext(&mut result);
        collect_exfat(&mut result);
        collect_ntfs(&mut result);
        collect_f2fs(&mut result);
        collect_bcachefs(&mut result);
        collect_xfs(&mut result);
        collect_swaps(&mut result);
        collect_zram(&mut result);
        collect_loopdev(&mut result);
        collect_cryptsetup(&mut result);
        collect_dmsetup(&mut result);
        collect_lvm(&mut result);
        collect_vdo(&mut result);
        collect_vdostats(&mut result);
        collect_vdostats_verbose(&mut result);
        collect_zfs(&mut result);
        collect_btrfs(&mut result);
        collect_bcache(&mut result);
        collect_iscsi_nodes(&mut result);
        collect_iscsi(&mut result);
        collect_nfs(&mut result);
        collect_nfs_exports(&mut result);
        collect_mdraid(&mut result);
        collect_multipath(&mut result);
        collect_nvme(&mut result);

        Ok(result)
    }
}

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
    use disk_nix_model::{NodeKind, Relationship, StorageGraph};

    use super::*;

    #[test]
    fn empty_probe_result_has_empty_graph_and_reports() {
        let result = ProbeResult::empty();
        assert!(result.graph.nodes.is_empty());
        assert!(result.reports.is_empty());
    }

    const SHARED_ISCSI_SESSION: &[u8] = br#"
Target: iqn.2026-06.example:storage.shared
    Current Portal: 10.0.0.10:3260,1
    Persistent Portal: 10.0.0.10:3260,1
    Target Portal Group Tag: 1
    **********
    Interface:
    **********
    Iface Name: default
    Iface Transport: tcp
    Iface Initiatorname: iqn.2026-06.client:node1
    Iface IPaddress: 10.0.0.20
    Iface Netdev: eno1
    SID: 42
    iSCSI Connection State: LOGGED IN
    iSCSI Session State: LOGGED_IN
    Internal iscsid Session State: NO CHANGE
    HeaderDigest: None
    DataDigest: None
    MaxRecvDataSegmentLength: 262144
    CID: 0
    Connection State: LOGGED IN
    Local Address: 10.0.0.20
    Peer Address: 10.0.0.10
    Host Number: 2  State: running
    scsi2 Channel 00 Id 0 Lun: 1
        Attached scsi disk sdb          State: running
"#;

    const SHARED_ISCSI_NODE: &[u8] = br#"
Target: iqn.2026-06.example:storage.shared
    Portal: 10.0.0.10:3260,1
    Persistent Portal: 10.0.0.11:3260,1
    TPGT: 1
    Iface Name: default
    Startup: automatic
    Leading Login: Yes
    AuthMethod: CHAP
    Username: node-user
    Password: outbound-secret
    Username_in: target-user
    Password_in: inbound-secret
"#;

    const SHARED_LSSCSI_LIST: &[u8] = br#"
[2:0:0:1]    disk    LIO-ORG  shared-lun      4.0   /dev/sdb   /dev/sg2   100G
  device_blocked=0
  queue_depth=128
  queue_type=simple
  state=running
  timeout=60
[3:0:0:1]    disk    LIO-ORG  shared-lun      4.0   /dev/sdc   /dev/sg3   100G
  device_blocked=0
  queue_depth=128
  queue_type=simple
  state=running
  timeout=60
"#;

    const SHARED_LSSCSI_TRANSPORT: &[u8] = br#"
[2:0:0:1]    disk    iscsi:iqn.2026-06.example:storage.shared,t,0x1  /dev/sdb   /dev/sg2  /dev/disk/by-id/scsi-3600508b400105e210000900000490000  /dev/disk/by-id/wwn-0x600508b400105e210000900000490000  100G
[3:0:0:1]    disk    iscsi:iqn.2026-06.example:storage.shared,t,0x2  /dev/sdc   /dev/sg3  /dev/disk/by-id/scsi-3600508b400105e210000900000490000  /dev/disk/by-id/wwn-0x600508b400105e210000900000490000  100G
"#;

    const SHARED_LSSCSI_UNIT: &[u8] = br#"
[2:0:0:1]    disk    3600508b400105e210000900000490000  /dev/sdb   /dev/sg2  /dev/disk/by-id/scsi-3600508b400105e210000900000490000  /dev/disk/by-id/wwn-0x600508b400105e210000900000490000  100G
[3:0:0:1]    disk    3600508b400105e210000900000490000  /dev/sdc   /dev/sg3  /dev/disk/by-id/scsi-3600508b400105e210000900000490000  /dev/disk/by-id/wwn-0x600508b400105e210000900000490000  100G
"#;

    const SHARED_MULTIPATH: &[u8] = br#"
mpatha (3600508b400105e210000900000490000) dm-2 LIO-ORG,shared-lun
size=100G features='1 queue_if_no_path' hwhandler='1 alua' wp=rw
|-+- policy='service-time 0' prio=50 status=active
| `- 2:0:0:1 sdb 8:16 active ready running ghost
`-+- policy='service-time 0' prio=10 status=enabled
  `- 3:0:0:1 sdc 8:32 active ready running faulty shaky
"#;

    const ENCRYPTED_DEGRADED_MDSTAT: &[u8] = br#"
Personalities : [raid1]
md127 : active raid1 nvme1n1p2[1](F) nvme0n1p2[0]
      2097152 blocks super 1.2 [2/1] [U_]
      [=>...................]  recovery = 8.5% (178257/2097152) finish=3.5min speed=15360K/sec
      bitmap: 1/16 pages [4KB], 65536KB chunk

unused devices: <none>
"#;

    const ENCRYPTED_DEGRADED_CRYPT_STATUS: &[u8] =
        br#"/dev/mapper/cryptraid is active and is in use.
  type:    LUKS2
  cipher:  aes-xts-plain64
  keysize: 512 bits
  key location: keyring
  device:  /dev/md127
  sector size: 4096
  offset:  32768 sectors
  size:    4186112 sectors
  mode:    read/write
  UUID:    luks-raid-uuid
"#;

    const ENCRYPTED_DEGRADED_LUKS_DUMP: &[u8] = br#"
LUKS header information
Version:        2
Epoch:          5
Metadata area:  16384 [bytes]
Keyslots area:  16744448 [bytes]
UUID:           luks-raid-uuid
Label:          encrypted-md-root
Subsystem:      disk-nix-fixture
Flags:          allow-discards

Data segments:
  0: crypt
        offset: 32768 [bytes]
        length: (whole device)
        cipher: aes-xts-plain64
        sector: 4096 [bytes]

Keyslots:
  0: luks2
        Key:        512 bits
        Priority:   normal
        Cipher:     aes-xts-plain64
        Cipher key: 512 bits
        PBKDF:      argon2id
        AF stripes: 4000
        Area offset:32768 [bytes]
        Area length:258048 [bytes]
        Digest ID:  0

Tokens:
  0: systemd-tpm2
        Keyslot:    0
        Keyslots:   0
        TPM2 PCRs:  0+7
        TPM2 Hash:  sha256

Digests:
  0: pbkdf2
        Hash:       sha256
        Iterations: 1000
"#;

    #[test]
    fn shared_storage_fabric_fixture_links_iscsi_luns_and_multipath_paths() {
        let mut graph = StorageGraph::empty();
        merge_graph(
            &mut graph,
            iscsi::normalize_iscsi_session_output(SHARED_ISCSI_SESSION)
                .expect("iSCSI session fixture should parse"),
        );
        merge_graph(
            &mut graph,
            iscsi::normalize_iscsi_node_output(SHARED_ISCSI_NODE)
                .expect("iSCSI node fixture should parse"),
        );
        merge_graph(
            &mut graph,
            lsscsi::normalize_lsscsi_list_output(SHARED_LSSCSI_LIST)
                .expect("lsscsi list fixture should parse"),
        );
        merge_graph(
            &mut graph,
            lsscsi::normalize_lsscsi_transport_output(SHARED_LSSCSI_TRANSPORT)
                .expect("lsscsi transport fixture should parse"),
        );
        merge_graph(
            &mut graph,
            lsscsi::normalize_lsscsi_unit_output(SHARED_LSSCSI_UNIT)
                .expect("lsscsi unit fixture should parse"),
        );
        merge_graph(
            &mut graph,
            multipath::normalize_multipath_output(SHARED_MULTIPATH)
                .expect("multipath fixture should parse"),
        );

        let session = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "iscsi-session:42")
            .expect("logged-in iSCSI session should exist");
        assert_eq!(session.kind, NodeKind::IscsiSession);
        assert_has_property(session, "iscsi.session-state", "LOGGED_IN");
        assert_has_property(session, "iscsi.portal-address", "10.0.0.10");
        assert_has_property(session, "iscsi.host-number", "2");

        let target = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "iscsi-target:iqn.2026-06.example:storage.shared")
            .expect("configured iSCSI target should exist");
        assert_eq!(target.kind, NodeKind::IscsiTarget);
        assert_has_property(target, "iscsi.node-startup", "automatic");
        assert_has_property(target, "iscsi.node-auth-password-configured", "true");
        assert_has_property(target, "iscsi.node-auth-password-in-configured", "true");
        assert!(
            !target.properties.iter().any(|property| {
                property.value == "outbound-secret" || property.value == "inbound-secret"
            }),
            "configured iSCSI node normalization must not leak CHAP secrets"
        );

        let scsi_lun = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "scsi-lun:2:0:0:1")
            .expect("host-visible SCSI LUN should exist");
        assert_eq!(scsi_lun.kind, NodeKind::Lun);
        assert_eq!(scsi_lun.size_bytes, Some(100_000_000_000));
        assert_has_property(
            scsi_lun,
            "scsi.transport",
            "iscsi:iqn.2026-06.example:storage.shared,t,0x1",
        );
        assert_has_property(scsi_lun, "scsi.queue-depth", "128");
        assert_eq!(
            scsi_lun.identity.wwn.as_deref(),
            Some("/dev/disk/by-id/wwn-0x600508b400105e210000900000490000")
        );

        let map = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "multipath:mpatha")
            .expect("multipath map should exist");
        assert_eq!(map.kind, NodeKind::MultipathDevice);
        assert_eq!(map.size_bytes, Some(100_000_000_000));
        assert_has_property(map, "multipath.wwid", "3600508b400105e210000900000490000");
        assert_has_property(map, "multipath.features", "1 queue_if_no_path");

        let path_sdb = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/sdb")
            .expect("first shared-storage path should exist");
        assert_eq!(path_sdb.kind, NodeKind::PhysicalDisk);
        assert_has_property(path_sdb, "scsi.address", "2:0:0:1");
        assert_has_property(path_sdb, "multipath.group-status", "active");
        assert_has_property(path_sdb, "multipath.path-flags", "ghost");

        let path_sdc = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/sdc")
            .expect("second shared-storage path should exist");
        assert_has_property(path_sdc, "scsi.address", "3:0:0:1");
        assert_has_property(path_sdc, "multipath.group-status", "enabled");
        assert_has_property(path_sdc, "multipath.path-flags", "faulty shaky");

        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "iscsi-session:42"
                && edge.to.0 == "iscsi-target:iqn.2026-06.example:storage.shared"
                && edge.relationship == Relationship::ImportedFrom
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "iscsi-lun:iqn.2026-06.example:storage.shared:1"
                && edge.to.0 == "block:/dev/sdb"
                && edge.relationship == Relationship::Backs
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "scsi-lun:2:0:0:1"
                && edge.to.0 == "block:/dev/sdb"
                && edge.relationship == Relationship::Backs
        }));
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|edge| {
                    edge.to.0 == "multipath:mpatha" && edge.relationship == Relationship::Backs
                })
                .count(),
            2
        );
    }

    #[test]
    fn encrypted_degraded_array_fixture_links_mdraid_and_luks_metadata() {
        let mut graph = StorageGraph::empty();
        merge_graph(
            &mut graph,
            mdraid::normalize_mdstat(ENCRYPTED_DEGRADED_MDSTAT)
                .expect("degraded mdstat fixture should parse"),
        );
        merge_graph(
            &mut graph,
            cryptsetup::normalize_cryptsetup_status(
                "/dev/mapper/cryptraid",
                ENCRYPTED_DEGRADED_CRYPT_STATUS,
            )
            .expect("cryptsetup status fixture should parse"),
        );
        merge_graph(
            &mut graph,
            cryptsetup::normalize_luks_dump(
                "/dev/disk/by-uuid/luks-raid-uuid",
                ENCRYPTED_DEGRADED_LUKS_DUMP,
            )
            .expect("LUKS header fixture should parse"),
        );

        let array = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "md:/dev/md127")
            .expect("degraded MD array should exist");
        assert_eq!(array.kind, NodeKind::MdRaid);
        assert_eq!(array.size_bytes, Some(2_147_483_648));
        assert_has_property(array, "md.mdstat-level", "raid1");
        assert_has_property(array, "md.mdstat-devices", "2/1");
        assert_has_property(array, "md.mdstat-health", "U_");
        assert_has_property(array, "md.mdstat-progress", "recovery");
        assert_has_property(array, "md.mdstat-progress-percent", "8.5%");

        let failed_member = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/nvme1n1p2")
            .expect("failed MD member should exist");
        assert_eq!(failed_member.kind, NodeKind::Partition);
        assert_has_property(failed_member, "md.mdstat-member-flags", "F");

        let mapper = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/mapper/cryptraid")
            .expect("active LUKS mapper should exist");
        assert_eq!(mapper.kind, NodeKind::LuksContainer);
        assert_eq!(mapper.path.as_deref(), Some("/dev/mapper/cryptraid"));
        assert_eq!(mapper.identity.uuid.as_deref(), Some("luks-raid-uuid"));
        assert_eq!(mapper.size_bytes, Some(17_146_314_752));
        assert_has_property(mapper, "cryptsetup.active", "true");
        assert_has_property(mapper, "cryptsetup.in-use", "true");
        assert_has_property(mapper, "cryptsetup.cipher", "aes-xts-plain64");

        let header = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/disk/by-uuid/luks-raid-uuid")
            .expect("LUKS header node on MD array should exist");
        assert_eq!(header.kind, NodeKind::LuksContainer);
        assert_eq!(header.identity.label.as_deref(), Some("encrypted-md-root"));
        assert_has_property(header, "cryptsetup.luks-version", "2");
        assert_has_property(header, "cryptsetup.luks-subsystem", "disk-nix-fixture");
        assert_has_property(header, "cryptsetup.luks-keyslot-count", "1");
        assert_has_property(header, "cryptsetup.luks-token-0-type", "systemd-tpm2");

        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "block:/dev/md127"
                && edge.to.0 == "block:/dev/mapper/cryptraid"
                && edge.relationship == Relationship::Backs
        }));
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|edge| {
                    edge.to.0 == "md:/dev/md127" && edge.relationship == Relationship::MemberOf
                })
                .count(),
            2
        );
    }

    fn merge_graph(target: &mut StorageGraph, source: StorageGraph) {
        for node in source.nodes {
            target.add_node(node);
        }
        for edge in source.edges {
            target.add_edge(edge);
        }
    }

    fn assert_has_property(node: &disk_nix_model::Node, key: &str, value: &str) {
        assert!(
            node.properties
                .iter()
                .any(|property| property.key == key && property.value == value),
            "{} should have property {key}={value}",
            node.id.0
        );
    }

    #[test]
    fn probe_reports_expose_structured_issue_categories() {
        let reports = vec![
            ProbeReport {
                adapter: "zfs".to_string(),
                status: ProbeStatus::Unavailable,
                message: Some("zpool not found or failed to run: No such file".to_string()),
            },
            ProbeReport {
                adapter: "lvm".to_string(),
                status: ProbeStatus::Partial,
                message: Some(
                    "must be root or have sufficient privileges to read device mapper state"
                        .to_string(),
                ),
            },
            ProbeReport {
                adapter: "lsblk".to_string(),
                status: ProbeStatus::Failed,
                message: Some("expected field blockdevices".to_string()),
            },
            ProbeReport {
                adapter: "findmnt".to_string(),
                status: ProbeStatus::Failed,
                message: Some("findmnt returned exit status 1".to_string()),
            },
            ProbeReport {
                adapter: "findmnt".to_string(),
                status: ProbeStatus::Available,
                message: Some("normalized 3 graph nodes".to_string()),
            },
            ProbeReport {
                adapter: "iscsi".to_string(),
                status: ProbeStatus::Partial,
                message: Some("configured node database is inaccessible".to_string()),
            },
            ProbeReport {
                adapter: "nvme".to_string(),
                status: ProbeStatus::Failed,
                message: Some("invalid JSON from nvme list".to_string()),
            },
        ];

        assert_eq!(reports[0].category(), ProbeIssueCategory::MissingTool);
        assert_eq!(reports[1].category(), ProbeIssueCategory::PermissionDenied);
        assert_eq!(reports[2].category(), ProbeIssueCategory::ParseFailed);
        assert_eq!(reports[3].category(), ProbeIssueCategory::CommandFailed);
        assert_eq!(reports[4].category(), ProbeIssueCategory::None);
        assert_eq!(reports[5].category(), ProbeIssueCategory::InaccessibleData);
        assert_eq!(reports[6].category(), ProbeIssueCategory::ParseFailed);
        assert!(
            reports[0]
                .remediation()
                .iter()
                .any(|item| { item.contains("pkgs.zfs") })
        );
        assert!(
            reports[1]
                .remediation()
                .iter()
                .any(|item| { item.contains("device-mapper state") })
        );
        assert!(
            reports[2]
                .remediation()
                .iter()
                .any(|item| { item.contains("fixture coverage") })
        );
        assert!(
            reports[3]
                .remediation()
                .iter()
                .any(|item| { item.contains("exit status") })
        );
        assert!(reports[4].remediation().is_empty());
        assert!(
            reports[5]
                .remediation()
                .iter()
                .any(|item| { item.contains("iscsid") || item.contains("open-iscsi") })
        );
        assert!(
            reports[6]
                .remediation()
                .iter()
                .any(|item| { item.contains("nvme-cli") })
        );

        let json = serde_json::to_string(&reports).expect("reports should serialize");
        assert!(json.contains(r#""category":"missing-tool""#));
        assert!(json.contains(r#""category":"permission-denied""#));
        assert!(json.contains(r#""category":"parse-failed""#));
        assert!(json.contains(r#""category":"command-failed""#));
        assert!(json.contains(r#""category":"inaccessible-data""#));
        assert!(json.contains(r#""category":"none""#));
        assert!(json.contains(r#""remediation":["#));
        assert!(json.contains("pkgs.zfs"));
        assert!(json.contains("device-mapper state"));
        assert!(json.contains("open-iscsi"));
        assert!(json.contains("nvme-cli"));
    }

    #[test]
    fn sub_adapters_inherit_domain_specific_remediation() {
        let cases = [
            ("nvme-id-ns", "nvme", "pkgs.nvme-cli", "nvme-cli JSON"),
            ("mdadm-scan", "mdraid", "pkgs.mdadm", "/proc/mdstat"),
            ("vdostats-verbose", "vdo", "pkgs.vdo", "VDO services"),
            ("zramctl", "zram", "pkgs.util-linux", "zram devices"),
            ("nfs-exports", "nfs", "pkgs.nfs-utils", "NFS mounts"),
        ];

        for (adapter, canonical, package, domain_hint) in cases {
            let metadata = adapter_remediation(adapter);
            assert_eq!(metadata.adapter, adapter);
            assert_eq!(metadata.canonical_adapter, canonical);
            assert!(
                metadata.nix_packages.iter().any(|item| item == package),
                "{adapter} should include package {package}"
            );
            assert!(
                metadata.data_hint.contains(domain_hint)
                    || metadata.parse_hint.contains(domain_hint)
                    || metadata.privilege_hint.contains(domain_hint),
                "{adapter} should include domain hint {domain_hint}"
            );

            let report = ProbeReport {
                adapter: adapter.to_string(),
                status: ProbeStatus::Unavailable,
                message: Some(format!("{adapter} not found or failed to run")),
            };
            let remediation = report.remediation();
            assert!(
                remediation.iter().any(|item| item.contains(package)),
                "{adapter} missing-tool remediation should include package {package}"
            );
        }
    }

    #[test]
    fn probe_issue_classifier_handles_common_real_world_messages() {
        for message in [
            "sh: zpool: command not found",
            "executable file not found in $PATH",
            "failed to run lvs: ENOENT",
            "No such file or directory (os error 2)",
        ] {
            assert_eq!(
                probe_category_for_message(message),
                ProbeIssueCategory::MissingTool
            );
        }

        for message in [
            "only root can use this command",
            "requires superuser privileges",
            "are you root?",
            "cannot open /dev/mapper/control: Operation not permitted",
        ] {
            assert_eq!(
                probe_category_for_message(message),
                ProbeIssueCategory::PermissionDenied
            );
        }
    }
}
