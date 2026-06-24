use std::{collections::BTreeSet, process::Command};

use disk_nix_model::StorageGraph;
use serde::Serialize;
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
mod lvm;
mod mdraid;
mod multipath;
mod nfs;
mod ntfs;
mod nvme;
mod parted;
mod swaps;
mod udev;
mod vdo;
mod xfs;
mod zfs;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProbeStatus {
    Available,
    Unavailable,
    Partial,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
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
                collect_nvme_namespace_details(result, namespace_paths);
                collect_nvme_controller_details(result, controllers);
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

        match (show, usage, subvolumes) {
            (Ok(show), Ok(usage), Ok(subvolumes)) => reports.push(btrfs::BtrfsReport {
                target,
                show,
                usage,
                subvolumes,
                qgroups,
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
            "name,type,used,available,referenced,mountpoint,origin,userrefs,compression,quota,reservation,encryption,keystatus,volsize,recordsize,dedup,checksum,copies,sync,primarycache,secondarycache,atime,relatime,snapdir,acltype,xattr",
        ],
    );
    let zpool_status = run_report("zpool", &["status", "-P"]);

    match (zpool_list, zfs_list, zpool_status) {
        (Ok(zpool_list), Ok(zfs_list), Ok(zpool_status)) => {
            match zfs::normalize_zfs(&zpool_list, &zfs_list, &zpool_status) {
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
        (Err(message), _, _) | (_, Err(message), _) | (_, _, Err(message)) => {
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
    use super::*;

    #[test]
    fn empty_probe_result_has_empty_graph_and_reports() {
        let result = ProbeResult::empty();
        assert!(result.graph.nodes.is_empty());
        assert!(result.reports.is_empty());
    }
}
