#![recursion_limit = "512"]

use std::{
    collections::{BTreeSet, VecDeque},
    fmt, fs,
    io::{self, Write},
    os::unix::fs::PermissionsExt,
    process::Command as ProcessCommand,
    process::ExitCode,
    time::{SystemTime, UNIX_EPOCH},
};

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use clap_mangen::Man;
use disk_nix_exec::{prepare_execution, ExecutionMode, ExecutionReport, ExecutionStatus};
use disk_nix_model::{Node, NodeKind, StorageGraph};
use disk_nix_plan::{
    compare_plan_with_topology, default_capabilities, plan_and_policy_from_json_bytes,
    plan_from_json_bytes, ApplyPolicy, Plan, TopologyComparison, TopologyDiagnosticLevel,
    SUPPORTED_SPEC_VERSION,
};
use disk_nix_probe::{
    adapter_remediation, LinuxProbe, ProbeAdapter, ProbeAdapterRemediation, ProbeIssueCategory,
    ProbeStatus,
};
use serde::Serialize;
use serde_json::Value;

fn main() -> ExitCode {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    match run(Cli::parse(), &mut stdout) {
        Ok(()) => ExitCode::SUCCESS,
        Err(AppError::Io(error)) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "disk-nix",
    version,
    about = "NixOS-native storage topology and lifecycle manager"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Inspect storage topology.
    Topology {
        /// Emit the canonical JSON graph.
        #[arg(long)]
        json: bool,
    },
    /// Show probe adapter availability and degradation status.
    ProbeStatus {
        /// Emit JSON probe reports.
        #[arg(long)]
        json: bool,
        /// Include OS, kernel, and storage tool version preflight context.
        #[arg(long)]
        preflight: bool,
    },
    /// Show modeled storage operation capabilities and risk classes.
    Capabilities {
        /// Emit JSON capability records.
        #[arg(long)]
        json: bool,
    },
    /// List block-like storage devices.
    Devices {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered partition nodes.
    Partitions {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered filesystems.
    Filesystems {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List complex filesystem objects across Btrfs, bcachefs, and ZFS.
    ComplexFilesystems {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List Btrfs filesystems, subvolumes, snapshots, qgroups, and members.
    Btrfs {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List bcachefs filesystems, member devices, and usage accounting.
    Bcachefs {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List ZFS pools, vdevs, datasets, snapshots, and zvols.
    Zfs {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered volumes, pools, datasets, LUNs, and exports.
    Volumes {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered storage pools and volume groups.
    Pools {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered snapshots across LVM, Btrfs, and ZFS.
    Snapshots {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered mapping layers such as LUKS, dm, LVM, VDO, and multipath.
    Mappings {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List device-mapper maps and dmsetup table/status metadata.
    Dm {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered disk encryption mappings and header metadata.
    Encryption {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered cache layers and cache device metadata.
    Cache {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List LVM physical volumes, volume groups, logical volumes, and segments.
    Lvm {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered VDO volumes and LVM VDO segment metadata.
    Vdo {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered multipath maps and path metadata.
    Multipath {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered NVMe subsystems, controllers, namespaces, and path metadata.
    Nvme {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered MD RAID arrays and member metadata.
    Raid {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered loop devices and backing file mappings.
    Loop {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered file-backed storage origins.
    BackingFiles {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered active swap devices and files.
    Swap {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered zram compressed swap devices.
    Zram {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered iSCSI sessions, targets, and LUNs.
    Iscsi {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered host-visible LUNs and SCSI path metadata.
    Luns {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered NFS exports and client mounts.
    Nfs {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered mountpoints.
    Mounts {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered iSCSI, LUN, and NFS nodes.
    NetworkStorage {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List storage identity fields such as UUIDs, labels, serials, and WWNs.
    Ids {
        /// Emit JSON for identity-bearing graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// Summarize capacity, free space, allocation, and utilization.
    Usage {
        /// Emit JSON for graph nodes with size or usage information.
        #[arg(long)]
        json: bool,
    },
    /// Inspect a graph node by id, path, name, UUID, label, serial, or property.
    Inspect {
        /// Query value to inspect.
        query: String,
        /// Relationship depth to include around matched nodes.
        #[arg(long, default_value_t = 1)]
        depth: usize,
        /// Emit JSON for matched nodes and relationship context.
        #[arg(long)]
        json: bool,
    },
    /// Plan desired storage changes from a JSON spec.
    Plan {
        /// Desired storage specification path.
        #[arg(long)]
        spec: String,
        /// Probe current topology and compare planned actions against it.
        #[arg(long)]
        probe_current: bool,
        /// Emit JSON plan output.
        #[arg(long)]
        json: bool,
    },
    /// Evaluate apply policy for a desired storage spec.
    Apply {
        /// Desired storage specification path.
        #[arg(long)]
        spec: String,
        /// Probe current topology and compare planned actions against it.
        #[arg(long)]
        probe_current: bool,
        /// Attempt execution after policy validation.
        #[arg(long)]
        execute: bool,
        /// Write a reviewable shell script for the allowed command and verification plan.
        #[arg(long)]
        script_out: Option<String>,
        /// Write the JSON apply report to a file before exit handling.
        #[arg(long)]
        report_out: Option<String>,
        /// Write a JSON apply receipt with invocation metadata and the report.
        #[arg(long)]
        receipt_out: Option<String>,
        /// Emit JSON apply report.
        #[arg(long)]
        json: bool,
    },
    /// Validate a desired storage spec and policy without treating policy blocks as command failure.
    Validate {
        /// Desired storage specification path.
        #[arg(long)]
        spec: String,
        /// Probe current topology and compare planned actions against it.
        #[arg(long)]
        probe_current: bool,
        /// Write a reviewable shell script when every planned action is policy-allowed.
        #[arg(long)]
        script_out: Option<String>,
        /// Write the JSON validation report to a file.
        #[arg(long)]
        report_out: Option<String>,
        /// Write a JSON validation receipt with invocation metadata and the report.
        #[arg(long)]
        receipt_out: Option<String>,
        /// Emit JSON validation report.
        #[arg(long)]
        json: bool,
    },
    /// Normalize a desired storage spec to the current supported contract version.
    Migrate {
        /// Desired storage specification path.
        #[arg(long)]
        spec: String,
        /// Emit JSON migration report with the migrated spec.
        #[arg(long)]
        json: bool,
    },
    /// Emit the supported desired-spec JSON contract.
    Schema,
    /// Generate shell completions.
    Completions {
        /// Shell completion format to emit.
        shell: Shell,
    },
    /// Generate a roff manpage.
    Manpage,
}

#[derive(Debug)]
enum AppError {
    Message(String),
    Io(io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct MigrationReport {
    source_version: Option<u64>,
    target_version: u64,
    migrated: bool,
    version_migrations: Vec<VersionMigrationContract>,
    legacy_mappings: Vec<LegacyMigrationMapping>,
    applied_mappings: Vec<LegacyMigrationMapping>,
    changes: Vec<String>,
    warnings: Vec<String>,
    spec: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct VersionMigrationContract {
    source_version: Option<u64>,
    target_version: u64,
    status: String,
    mapping_scope: String,
    field_mappings: Vec<LegacyMigrationMapping>,
    safety_notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct LegacyMigrationMapping {
    source: String,
    target: String,
    scope: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProbeStatusPreflightReport {
    environment: ProbePreflightEnvironment,
    preflight_checks: ProbePreflightChecks,
    reports: Vec<disk_nix_probe::ProbeReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProbePreflightEnvironment {
    os_id: Option<String>,
    os_version_id: Option<String>,
    os_pretty_name: Option<String>,
    kernel_release: Option<String>,
    effective_uid: Option<String>,
    tool_versions: Vec<ToolVersionReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProbePreflightChecks {
    status: ProbePreflightCheckStatus,
    root: bool,
    unavailable_tool_count: usize,
    failed_tool_count: usize,
    missing_tools: Vec<String>,
    failed_tools: Vec<String>,
    adapter_remediation: Vec<ProbeAdapterRemediation>,
    remediation: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ProbePreflightCheckStatus {
    Ready,
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolVersionReport {
    tool: String,
    status: ToolVersionStatus,
    version: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ToolVersionStatus {
    Available,
    Unavailable,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApplyReceipt {
    receipt_version: u64,
    command: String,
    spec_path: String,
    probe_current: bool,
    execute_requested: bool,
    generated_at_unix_seconds: u64,
    report: ExecutionReport,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Message(message) => f.write_str(message),
            Self::Io(error) => error.fmt(f),
        }
    }
}

impl From<io::Error> for AppError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

fn run(cli: Cli, output: &mut impl Write) -> Result<(), AppError> {
    match cli.command {
        Command::Topology { json: false } => {
            let probe = LinuxProbe::new();
            let result = probe
                .collect()
                .map_err(|error| AppError::Message(error.to_string()))?;
            print_topology_summary(output, &result)?;
            Ok(())
        }
        Command::Topology { json: true } => {
            let graph = collect_graph()?;
            writeln!(
                output,
                "{}",
                graph
                    .to_json()
                    .map_err(|error| AppError::Message(error.to_string()))?
            )?;
            Ok(())
        }
        Command::ProbeStatus { json, preflight } => {
            let probe = LinuxProbe::new();
            let result = probe
                .collect()
                .map_err(|error| AppError::Message(error.to_string()))?;
            if json {
                if preflight {
                    let environment = collect_probe_preflight_environment();
                    let preflight_checks = probe_preflight_checks(&environment);
                    let report = ProbeStatusPreflightReport {
                        environment,
                        preflight_checks,
                        reports: result.reports,
                    };
                    writeln!(
                        output,
                        "{}",
                        serde_json::to_string_pretty(&report)
                            .map_err(|error| AppError::Message(error.to_string()))?
                    )?;
                } else {
                    writeln!(
                        output,
                        "{}",
                        serde_json::to_string_pretty(&result.reports)
                            .map_err(|error| AppError::Message(error.to_string()))?
                    )?;
                }
            } else if preflight {
                let environment = collect_probe_preflight_environment();
                let preflight_checks = probe_preflight_checks(&environment);
                print_probe_preflight_environment(output, &environment)?;
                print_probe_preflight_checks(output, &preflight_checks)?;
                print_probe_reports(output, &result.reports)?;
            } else {
                print_probe_reports(output, &result.reports)?;
            }
            Ok(())
        }
        Command::Capabilities { json } => {
            let capabilities = default_capabilities();
            if json {
                writeln!(
                    output,
                    "{}",
                    serde_json::to_string_pretty(&capabilities)
                        .map_err(|error| AppError::Message(error.to_string()))?
                )?;
            } else {
                for capability in capabilities {
                    writeln!(
                        output,
                        "{:?} {:?} {:?}",
                        capability.node_kind, capability.operation, capability.risk
                    )?;
                }
            }
            Ok(())
        }
        Command::Devices { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_device_node)?;
            } else {
                print_devices(output, &graph)?;
            }
            Ok(())
        }
        Command::Partitions { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_partition_node)?;
            } else {
                print_partitions(output, &graph)?;
            }
            Ok(())
        }
        Command::Filesystems { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_filesystem_node)?;
            } else {
                print_filesystems(output, &graph)?;
            }
            Ok(())
        }
        Command::ComplexFilesystems { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_complex_filesystem_node)?;
            } else {
                print_complex_filesystems(output, &graph)?;
            }
            Ok(())
        }
        Command::Btrfs { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_btrfs_node)?;
            } else {
                print_btrfs(output, &graph)?;
            }
            Ok(())
        }
        Command::Bcachefs { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_bcachefs_node)?;
            } else {
                print_bcachefs(output, &graph)?;
            }
            Ok(())
        }
        Command::Zfs { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_zfs_node)?;
            } else {
                print_zfs(output, &graph)?;
            }
            Ok(())
        }
        Command::Volumes { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_volume_node)?;
            } else {
                print_volumes(output, &graph)?;
            }
            Ok(())
        }
        Command::Pools { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_pool_node)?;
            } else {
                print_pools(output, &graph)?;
            }
            Ok(())
        }
        Command::Snapshots { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_snapshot_node)?;
            } else {
                print_snapshots(output, &graph)?;
            }
            Ok(())
        }
        Command::Mappings { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_mapping_node)?;
            } else {
                print_mappings(output, &graph)?;
            }
            Ok(())
        }
        Command::Dm { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_dm_node)?;
            } else {
                print_dm(output, &graph)?;
            }
            Ok(())
        }
        Command::Encryption { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_encryption_node)?;
            } else {
                print_encryption(output, &graph)?;
            }
            Ok(())
        }
        Command::Cache { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_cache_node)?;
            } else {
                print_cache(output, &graph)?;
            }
            Ok(())
        }
        Command::Lvm { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_lvm_node)?;
            } else {
                print_lvm(output, &graph)?;
            }
            Ok(())
        }
        Command::Vdo { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_vdo_node)?;
            } else {
                print_vdo(output, &graph)?;
            }
            Ok(())
        }
        Command::Multipath { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_multipath_node)?;
            } else {
                print_multipath(output, &graph)?;
            }
            Ok(())
        }
        Command::Nvme { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_nvme_node)?;
            } else {
                print_nvme(output, &graph)?;
            }
            Ok(())
        }
        Command::Raid { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_raid_node)?;
            } else {
                print_raid(output, &graph)?;
            }
            Ok(())
        }
        Command::Loop { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_loop_node)?;
            } else {
                print_loop(output, &graph)?;
            }
            Ok(())
        }
        Command::BackingFiles { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_backing_file_node)?;
            } else {
                print_backing_files(output, &graph)?;
            }
            Ok(())
        }
        Command::Swap { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_swap_node)?;
            } else {
                print_swap(output, &graph)?;
            }
            Ok(())
        }
        Command::Zram { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_zram_node)?;
            } else {
                print_zram(output, &graph)?;
            }
            Ok(())
        }
        Command::Iscsi { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_iscsi_node)?;
            } else {
                print_iscsi(output, &graph)?;
            }
            Ok(())
        }
        Command::Luns { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_lun_node)?;
            } else {
                print_luns(output, &graph)?;
            }
            Ok(())
        }
        Command::Nfs { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_nfs_node)?;
            } else {
                print_nfs(output, &graph)?;
            }
            Ok(())
        }
        Command::Mounts { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_mount_node)?;
            } else {
                print_mounts(output, &graph)?;
            }
            Ok(())
        }
        Command::NetworkStorage { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_network_storage_node)?;
            } else {
                print_network_storage(output, &graph)?;
            }
            Ok(())
        }
        Command::Ids { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, has_identity)?;
            } else {
                print_ids(output, &graph)?;
            }
            Ok(())
        }
        Command::Usage { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, has_capacity_or_usage)?;
            } else {
                print_usage(output, &graph)?;
            }
            Ok(())
        }
        Command::Inspect { query, depth, json } => {
            let graph = collect_graph()?;
            if json {
                print_inspect_json(output, &graph, &query, depth)?;
            } else {
                print_inspect(output, &graph, &query, depth)?;
            }
            Ok(())
        }
        Command::Plan {
            spec,
            probe_current,
            json,
        } => {
            let bytes = std::fs::read(&spec)?;
            let mut plan = plan_from_json_bytes(&bytes)
                .map_err(|error| AppError::Message(format!("failed to parse {spec}: {error}")))?;
            if probe_current {
                plan = compare_plan_with_topology(plan, &collect_graph()?);
            }
            if json {
                writeln!(
                    output,
                    "{}",
                    plan.to_json()
                        .map_err(|error| AppError::Message(error.to_string()))?
                )?;
            } else {
                print_plan(output, &plan)?;
            }
            Ok(())
        }
        Command::Apply {
            spec,
            probe_current,
            execute,
            script_out,
            report_out,
            receipt_out,
            json,
        } => {
            let mode = if execute {
                ExecutionMode::Execute
            } else {
                ExecutionMode::DryRun
            };
            let report = prepare_apply_report(&spec, probe_current, mode)?;
            if let Some(report_out) = report_out.as_deref() {
                write_execution_report(report_out, &report)?;
            }
            if let Some(receipt_out) = receipt_out.as_deref() {
                write_apply_receipt(
                    receipt_out,
                    apply_receipt(
                        "apply",
                        &spec,
                        probe_current,
                        execute,
                        current_unix_seconds()?,
                        &report,
                    ),
                )?;
            }
            if let Some(script_out) = script_out.as_deref() {
                write_execution_script(script_out, &report)?;
            }

            if json {
                writeln!(
                    output,
                    "{}",
                    report
                        .to_json()
                        .map_err(|error| AppError::Message(error.to_string()))?
                )?;
            } else {
                print_execution_report(output, &report, execute)?;
            }

            if report.status == ExecutionStatus::Blocked {
                return Err(AppError::Message(format!(
                    "apply policy blocked {} action(s)",
                    report.apply.blocked_count
                )));
            }
            if matches!(
                report.status,
                ExecutionStatus::NotReady | ExecutionStatus::Failed
            ) {
                return Err(AppError::Message(report.messages.join("; ")));
            }

            Ok(())
        }
        Command::Validate {
            spec,
            probe_current,
            script_out,
            report_out,
            receipt_out,
            json,
        } => {
            let report = prepare_apply_report(&spec, probe_current, ExecutionMode::DryRun)?;
            if let Some(report_out) = report_out.as_deref() {
                write_execution_report(report_out, &report)?;
            }
            if let Some(receipt_out) = receipt_out.as_deref() {
                write_apply_receipt(
                    receipt_out,
                    apply_receipt(
                        "validate",
                        &spec,
                        probe_current,
                        false,
                        current_unix_seconds()?,
                        &report,
                    ),
                )?;
            }
            if let Some(script_out) = script_out.as_deref() {
                write_execution_script(script_out, &report)?;
            }

            if json {
                writeln!(
                    output,
                    "{}",
                    report
                        .to_json()
                        .map_err(|error| AppError::Message(error.to_string()))?
                )?;
            } else {
                print_execution_report(output, &report, false)?;
            }

            Ok(())
        }
        Command::Migrate { spec, json } => {
            let bytes = std::fs::read(&spec)?;
            let report = migration_report_from_json_bytes(&bytes)
                .map_err(|error| AppError::Message(format!("failed to migrate {spec}: {error}")))?;
            if json {
                writeln!(
                    output,
                    "{}",
                    serde_json::to_string_pretty(&report)
                        .map_err(|error| AppError::Message(error.to_string()))?
                )?;
            } else {
                print_migration_report(output, &report)?;
            }
            Ok(())
        }
        Command::Schema => {
            writeln!(
                output,
                "{}",
                serde_json::to_string_pretty(&spec_schema())
                    .map_err(|error| AppError::Message(error.to_string()))?
            )?;
            Ok(())
        }
        Command::Completions { shell } => {
            let mut command = Cli::command();
            generate(shell, &mut command, "disk-nix", output);
            Ok(())
        }
        Command::Manpage => {
            let command = Cli::command();
            Man::new(command).render(output)?;
            Ok(())
        }
    }
}

fn spec_schema() -> serde_json::Value {
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://github.com/midischwarz12/disk-nix/schema/disk-nix-spec.schema.json",
        "title": "disk-nix desired storage spec",
        "description": "Desired storage declaration accepted by disk-nix plan, apply, and validate. The CLI accepts either this direct shape or a wrapper with { spec, apply } as produced by the NixOS module.",
        "type": "object",
        "additionalProperties": true,
        "properties": {
            "version": {
                "type": "integer",
                "const": SUPPORTED_SPEC_VERSION,
                "description": "Optional disk-nix spec contract version. Version 1 is the current supported contract."
            },
            "spec": {
                "$ref": "#/$defs/specBody",
                "description": "NixOS module wrapper body. When present, planner lifecycle inputs are read from this object."
            },
            "apply": {
                "$ref": "#/$defs/applyPolicy"
            },
            "filesystems": {
                "$ref": "#/$defs/filesystemMap"
            },
            "swaps": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "zram": {
                "$ref": "#/$defs/zramSpec"
            },
            "luks": {
                "$ref": "#/$defs/luksSpec"
            },
            "nfs": {
                "$ref": "#/$defs/nfsSpec"
            },
            "iscsi": {
                "$ref": "#/$defs/iscsiSpec"
            },
            "disks": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "partitions": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "btrfsSubvolumes": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "btrfsQgroups": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "vdoVolumes": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "physicalVolumes": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "luksKeyslots": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "luksTokens": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "volumes": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "volumeGroups": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "thinPools": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "lvmSnapshots": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "lvmCaches": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "loopDevices": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "backingFiles": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "dmMaps": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "mdRaids": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "multipathMaps": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "pools": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "datasets": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "zvols": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "luns": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "targetLuns": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "nvmeNamespaces": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "iscsiSessions": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "exports": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "caches": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "snapshots": {
                "$ref": "#/$defs/snapshotMap"
            }
        },
        "$defs": {
            "specBody": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "version": {
                        "type": "integer",
                        "const": SUPPORTED_SPEC_VERSION,
                        "description": "Optional disk-nix spec contract version. Version 1 is the current supported contract."
                    },
                    "filesystems": { "$ref": "#/$defs/filesystemMap" },
                    "swaps": { "$ref": "#/$defs/lifecycleMap" },
                    "zram": { "$ref": "#/$defs/zramSpec" },
                    "luks": { "$ref": "#/$defs/luksSpec" },
                    "nfs": { "$ref": "#/$defs/nfsSpec" },
                    "iscsi": { "$ref": "#/$defs/iscsiSpec" },
                    "disks": { "$ref": "#/$defs/lifecycleMap" },
                    "partitions": { "$ref": "#/$defs/lifecycleMap" },
                    "btrfsSubvolumes": { "$ref": "#/$defs/lifecycleMap" },
                    "btrfsQgroups": { "$ref": "#/$defs/lifecycleMap" },
                    "vdoVolumes": { "$ref": "#/$defs/lifecycleMap" },
                    "physicalVolumes": { "$ref": "#/$defs/lifecycleMap" },
                    "luksKeyslots": { "$ref": "#/$defs/lifecycleMap" },
                    "luksTokens": { "$ref": "#/$defs/lifecycleMap" },
                    "volumes": { "$ref": "#/$defs/lifecycleMap" },
                    "volumeGroups": { "$ref": "#/$defs/lifecycleMap" },
                    "thinPools": { "$ref": "#/$defs/lifecycleMap" },
                    "lvmSnapshots": { "$ref": "#/$defs/lifecycleMap" },
                    "lvmCaches": { "$ref": "#/$defs/lifecycleMap" },
                    "loopDevices": { "$ref": "#/$defs/lifecycleMap" },
                    "backingFiles": { "$ref": "#/$defs/lifecycleMap" },
                    "dmMaps": { "$ref": "#/$defs/lifecycleMap" },
                    "mdRaids": { "$ref": "#/$defs/lifecycleMap" },
                    "multipathMaps": { "$ref": "#/$defs/lifecycleMap" },
                    "pools": { "$ref": "#/$defs/lifecycleMap" },
                    "datasets": { "$ref": "#/$defs/lifecycleMap" },
                    "zvols": { "$ref": "#/$defs/lifecycleMap" },
                    "luns": { "$ref": "#/$defs/lifecycleMap" },
                    "targetLuns": { "$ref": "#/$defs/lifecycleMap" },
                    "nvmeNamespaces": { "$ref": "#/$defs/lifecycleMap" },
                    "iscsiSessions": { "$ref": "#/$defs/lifecycleMap" },
                    "exports": { "$ref": "#/$defs/lifecycleMap" },
                    "caches": { "$ref": "#/$defs/lifecycleMap" },
                    "snapshots": { "$ref": "#/$defs/snapshotMap" }
                }
            },
            "filesystemMap": {
                "type": "object",
                "additionalProperties": { "$ref": "#/$defs/filesystem" }
            },
            "filesystem": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "mountpoint": { "type": "string" },
                    "device": { "type": "string" },
                    "fsType": { "type": "string" },
                    "type": { "type": "string" },
                    "operation": { "$ref": "#/$defs/operation" },
                    "action": { "$ref": "#/$defs/operation" },
                    "neededForBoot": { "type": "boolean" },
                    "destroy": { "type": "boolean" },
                    "resizePolicy": {
                        "type": "string",
                        "enum": ["none", "grow-only", "shrink-allowed"]
                    },
                    "desiredSize": { "type": ["string", "number"] },
                    "targetSize": { "type": ["string", "number"] },
                    "size": { "type": ["string", "number"] },
                    "options": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "addDevices": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "removeDevices": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "replaceDevices": {
                        "type": "object",
                        "additionalProperties": { "type": "string" }
                    },
                    "renameTo": { "type": "string" },
                    "renameTarget": { "type": "string" },
                    "newName": { "type": "string" },
                    "properties": {
                        "type": "object",
                        "additionalProperties": true
                    },
                    "metadata": {
                        "type": "object",
                        "additionalProperties": true
                    },
                    "preserveData": { "type": "boolean", "default": true },
                    "readOnly": { "type": "boolean" },
                    "readonly": { "type": "boolean" }
                }
            },
            "lifecycleMap": {
                "type": "object",
                "additionalProperties": { "$ref": "#/$defs/lifecycleObject" }
            },
            "zramSpec": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "enable": { "type": "boolean" },
                    "operation": { "$ref": "#/$defs/operation" },
                    "action": { "$ref": "#/$defs/operation" },
                    "swapDevices": { "type": "integer", "minimum": 1 },
                    "memoryPercent": { "type": "integer", "minimum": 1 },
                    "memoryMax": { "type": ["integer", "null"] },
                    "priority": { "type": "integer" },
                    "algorithm": { "type": "string" },
                    "writebackDevice": { "type": ["string", "null"] },
                    "preserveData": { "type": "boolean", "default": true },
                    "properties": {
                        "type": "object",
                        "additionalProperties": true
                    }
                }
            },
            "luksSpec": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "devices": { "$ref": "#/$defs/lifecycleMap" }
                }
            },
            "nfsSpec": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "mounts": { "$ref": "#/$defs/nfsMountMap" }
                }
            },
            "nfsMountMap": {
                "type": "object",
                "additionalProperties": { "$ref": "#/$defs/nfsMount" }
            },
            "nfsMount": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "source": { "type": "string" },
                    "device": { "type": "string" },
                    "fsType": {
                        "type": "string",
                        "enum": ["nfs", "nfs4"]
                    },
                    "operation": { "$ref": "#/$defs/operation" },
                    "action": { "$ref": "#/$defs/operation" },
                    "mountpoint": { "type": "string" },
                    "options": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "neededForBoot": { "type": "boolean" },
                    "destroy": { "type": "boolean" },
                    "metadata": {
                        "type": "object",
                        "additionalProperties": true
                    },
                    "preserveData": { "type": "boolean", "default": true }
                }
            },
            "iscsiSpec": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "initiatorName": { "type": ["string", "null"] },
                    "discoverPortal": { "type": ["string", "null"] },
                    "enableAutoLoginOut": { "type": "boolean" },
                    "extraConfig": { "type": "string" },
                    "sessions": { "$ref": "#/$defs/lifecycleMap" },
                    "boot": { "$ref": "#/$defs/iscsiBoot" }
                }
            },
            "iscsiBoot": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "enable": { "type": "boolean" },
                    "discoverPortal": { "type": ["string", "null"] },
                    "target": { "type": ["string", "null"] },
                    "loginAll": { "type": "boolean" },
                    "logLevel": { "type": "integer" },
                    "extraIscsiCommands": { "type": "string" },
                    "extraConfig": { "type": ["string", "null"] }
                }
            },
            "lifecycleObject": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "operation": { "$ref": "#/$defs/operation" },
                    "action": { "$ref": "#/$defs/operation" },
                    "addDevices": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "devices": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "paths": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "devicePaths": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "removeDevices": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "replaceDevices": {
                        "type": "object",
                        "additionalProperties": { "type": "string" }
                    },
                    "cacheSetUuid": { "type": "string" },
                    "cacheSetUUID": { "type": "string" },
                    "cache-set-uuid": { "type": "string" },
                    "cache_set_uuid": { "type": "string" },
                    "properties": {
                        "type": "object",
                        "additionalProperties": true
                    },
                    "desiredSize": { "type": ["string", "number"] },
                    "targetSize": { "type": ["string", "number"] },
                    "size": { "type": ["string", "number"] },
                    "physicalSize": { "type": ["string", "number"] },
                    "vdoPhysicalSize": { "type": ["string", "number"] },
                    "physical-size": { "type": ["string", "number"] },
                    "renameTo": { "type": "string" },
                    "renameTarget": { "type": "string" },
                    "newName": { "type": "string" },
                    "name": { "type": "string" },
                    "target": { "type": "string" },
                    "path": { "type": "string" },
                    "mountpoint": { "type": "string" },
                    "device": { "type": "string" },
                    "disk": { "type": "string" },
                    "client": { "type": "string" },
                    "initiators": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "initiatorIqns": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "clients": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "portal": { "type": "string" },
                    "provider": { "type": "string" },
                    "storageProvider": { "type": "string" },
                    "storage-provider": { "type": "string" },
                    "arrayProvider": { "type": "string" },
                    "array-provider": { "type": "string" },
                    "vendor": { "type": "string" },
                    "arrayVendor": { "type": "string" },
                    "array-vendor": { "type": "string" },
                    "arrayId": { "type": "string" },
                    "arrayID": { "type": "string" },
                    "array-id": { "type": "string" },
                    "array_id": { "type": "string" },
                    "systemId": { "type": "string" },
                    "system-id": { "type": "string" },
                    "storagePool": { "type": "string" },
                    "storage-pool": { "type": "string" },
                    "poolName": { "type": "string" },
                    "pool-name": { "type": "string" },
                    "aggregate": { "type": "string" },
                    "volumeId": { "type": "string" },
                    "volumeID": { "type": "string" },
                    "volume-id": { "type": "string" },
                    "volume_id": { "type": "string" },
                    "volumeName": { "type": "string" },
                    "snapshotId": { "type": "string" },
                    "snapshotID": { "type": "string" },
                    "snapshot-id": { "type": "string" },
                    "snapshot_id": { "type": "string" },
                    "snapshotName": { "type": "string" },
                    "cloneSource": { "type": "string" },
                    "clone-source": { "type": "string" },
                    "sourceSnapshot": { "type": "string" },
                    "source-snapshot": { "type": "string" },
                    "sourceVolume": { "type": "string" },
                    "source-volume": { "type": "string" },
                    "maskingGroup": { "type": "string" },
                    "masking-group": { "type": "string" },
                    "hostGroup": { "type": "string" },
                    "host-group": { "type": "string" },
                    "igroup": { "type": "string" },
                    "lun": { "type": ["string", "number"] },
                    "lunId": { "type": ["string", "number"] },
                    "lun-id": { "type": ["string", "number"] },
                    "lunNumber": { "type": ["string", "number"] },
                    "lun-number": { "type": ["string", "number"] },
                    "namespaceId": { "type": ["string", "number"] },
                    "nsid": { "type": ["string", "number"] },
                    "controllers": { "type": "string" },
                    "controllerId": { "type": ["string", "number"] },
                    "controller": { "type": ["string", "number"] },
                    "keySlot": { "type": ["string", "number"] },
                    "key-slot": { "type": ["string", "number"] },
                    "slot": { "type": ["string", "number"] },
                    "keyFile": { "type": "string" },
                    "key-file": { "type": "string" },
                    "currentKeyFile": { "type": "string" },
                    "newKeyFile": { "type": "string" },
                    "new-key-file": { "type": "string" },
                    "tokenId": { "type": ["string", "number"] },
                    "token-id": { "type": ["string", "number"] },
                    "token": { "type": ["string", "number"] },
                    "tokenFile": { "type": "string" },
                    "token-file": { "type": "string" },
                    "jsonFile": { "type": "string" },
                    "options": { "type": "string" },
                    "priority": { "type": "integer" },
                    "randomEncryption": { "type": "boolean" },
                    "allowDiscards": { "type": "boolean" },
                    "bypassWorkqueues": { "type": "boolean" },
                    "preLVM": { "type": "boolean" },
                    "start": { "type": ["string", "number"] },
                    "startOffset": { "type": ["string", "number"] },
                    "end": { "type": ["string", "number"] },
                    "endOffset": { "type": ["string", "number"] },
                    "partitionNumber": { "type": ["string", "number"] },
                    "number": { "type": ["string", "number"] },
                    "partitionType": { "type": "string" },
                    "level": { "type": "string" },
                    "raidLevel": { "type": "string" },
                    "type": { "type": "string" },
                    "destroy": { "type": "boolean" },
                    "readOnly": { "type": "boolean" },
                    "readonly": { "type": "boolean" },
                    "preserveData": { "type": "boolean", "default": true },
                    "metadata": {
                        "type": "object",
                        "additionalProperties": true
                    }
                }
            },
            "snapshotMap": {
                "type": "object",
                "additionalProperties": { "$ref": "#/$defs/snapshot" }
            },
            "snapshot": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "target": { "type": "string" },
                    "path": { "type": "string" },
                    "snapshotPath": { "type": "string" },
                    "snapshot-path": { "type": "string" },
                    "operation": { "$ref": "#/$defs/operation" },
                    "action": { "$ref": "#/$defs/operation" },
                    "destroy": { "type": "boolean" },
                    "rollback": { "type": "boolean" },
                    "cloneTo": { "type": "string" },
                    "cloneTarget": { "type": "string" },
                    "clone": { "type": "string" },
                    "renameTo": { "type": "string" },
                    "renameTarget": { "type": "string" },
                    "newName": { "type": "string" },
                    "recursiveRollback": { "type": "boolean" },
                    "recursive": { "type": "boolean" },
                    "zfs.rollbackRecursive": { "type": "boolean" },
                    "hold": { "type": "string" },
                    "holdTag": { "type": "string" },
                    "releaseHold": { "type": "string" },
                    "readOnly": { "type": "boolean" },
                    "readonly": { "type": "boolean" },
                    "preserveData": { "type": "boolean", "default": true },
                    "metadata": {
                        "type": "object",
                        "additionalProperties": true
                    }
                }
            },
            "operation": {
                "type": "string",
                "enum": [
                    "create",
                    "format",
                    "grow",
                    "shrink",
                    "check",
                    "repair",
                    "scrub",
                    "trim",
                    "rescan",
                    "replace-device",
                    "add-device",
                    "remove-device",
                    "add-key",
                    "remove-key",
                    "import-token",
                    "remove-token",
                    "set-property",
                    "snapshot",
                    "clone",
                    "promote",
                    "import",
                    "export",
                    "unexport",
                    "attach",
                    "detach",
                    "activate",
                    "deactivate",
                    "assemble",
                    "start",
                    "stop",
                    "login",
                    "logout",
                    "open",
                    "close",
                    "mount",
                    "unmount",
                    "remount",
                    "rename",
                    "rebalance",
                    "rollback",
                    "destroy"
                ]
            },
            "applyPolicy": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "mode": {
                        "type": "string",
                        "enum": ["manual", "activation", "boot", "install"],
                        "default": "manual"
                    },
                    "allowDestructive": { "type": "boolean", "default": false },
                    "allowFormat": { "type": "boolean", "default": false },
                    "allowShrink": { "type": "boolean", "default": false },
                    "allowPotentialDataLoss": { "type": "boolean", "default": false },
                    "allowGrow": { "type": "boolean", "default": true },
                    "allowOffline": { "type": "boolean", "default": false },
                    "allowPropertyChanges": { "type": "boolean", "default": true },
                    "allowDeviceReplacement": { "type": "boolean", "default": true },
                    "allowRebalance": { "type": "boolean", "default": true },
                    "requireBackup": { "type": "boolean", "default": false },
                    "backupVerified": { "type": "boolean", "default": false },
                    "requireConfirmation": { "type": "boolean", "default": false },
                    "confirmation": { "type": "boolean", "default": false },
                    "requireConfirmationFile": { "type": ["string", "null"] },
                    "probeCurrent": {
                        "type": "boolean",
                        "description": "NixOS module helper that controls whether activation validation passes --probe-current."
                    },
                    "failOnBlocked": {
                        "type": "boolean",
                        "default": true,
                        "description": "NixOS module helper that controls whether activation uses apply and fails on blocked policy, or validate and reports blocked policy without failing the unit."
                    },
                    "scriptOut": {
                        "type": ["string", "null"],
                        "description": "NixOS module helper that controls activation --script-out."
                    },
                    "reportOut": {
                        "type": ["string", "null"],
                        "description": "NixOS module helper that controls activation --report-out."
                    },
                    "receiptOut": {
                        "type": ["string", "null"],
                        "description": "NixOS module helper that controls activation --receipt-out."
                    }
                }
            }
        }
    })
}

fn migration_report_from_json_bytes(bytes: &[u8]) -> Result<MigrationReport, AppError> {
    let mut value: Value =
        serde_json::from_slice(bytes).map_err(|error| AppError::Message(error.to_string()))?;
    let source_version = migration_source_version(&value)?;
    let target_version = SUPPORTED_SPEC_VERSION;
    if source_version.is_some_and(|version| version != target_version) {
        return Err(AppError::Message(format!(
            "unsupported disk-nix spec version {}; supported migration target is {target_version}",
            source_version.expect("checked")
        )));
    }

    let mut changes = Vec::new();
    let mut warnings = Vec::new();
    let mut applied_mappings = Vec::new();
    apply_legacy_pre_version_mappings(
        &mut value,
        source_version,
        &mut changes,
        &mut applied_mappings,
    )?;
    ensure_object_version(&mut value, "version", target_version, &mut changes)?;
    if let Some(spec) = value.get_mut("spec") {
        ensure_object_version(spec, "spec.version", target_version, &mut changes)?;
    }
    if changes.is_empty() {
        changes.push("spec already declares the current supported contract version".to_string());
    }
    warnings.push(
        "migration does not apply storage mutations; run plan or apply separately after review"
            .to_string(),
    );
    warnings.push(
        "version 1 migration only normalizes metadata and documented legacy pre-versioned field names"
            .to_string(),
    );

    let serialized =
        serde_json::to_vec(&value).map_err(|error| AppError::Message(error.to_string()))?;
    plan_from_json_bytes(&serialized)
        .map_err(|error| AppError::Message(format!("migrated spec is invalid: {error}")))?;

    Ok(MigrationReport {
        source_version,
        target_version,
        migrated: !changes
            .iter()
            .any(|change| change == "spec already declares the current supported contract version"),
        version_migrations: version_migration_contracts(),
        legacy_mappings: legacy_pre_version_mappings(),
        applied_mappings,
        changes,
        warnings,
        spec: value,
    })
}

fn apply_legacy_pre_version_mappings(
    value: &mut Value,
    source_version: Option<u64>,
    changes: &mut Vec<String>,
    applied_mappings: &mut Vec<LegacyMigrationMapping>,
) -> Result<(), AppError> {
    if source_version.is_some() {
        return Ok(());
    }
    normalize_legacy_pre_version_container(value, "", changes, applied_mappings)?;
    if let Some(spec) = value.get_mut("spec") {
        normalize_legacy_pre_version_container(spec, "spec.", changes, applied_mappings)?;
    }
    Ok(())
}

fn normalize_legacy_pre_version_container(
    value: &mut Value,
    prefix: &str,
    changes: &mut Vec<String>,
    applied_mappings: &mut Vec<LegacyMigrationMapping>,
) -> Result<(), AppError> {
    rename_legacy_field(
        value,
        prefix,
        "fileSystems",
        "filesystems",
        changes,
        applied_mappings,
    )?;
    rename_legacy_field(
        value,
        prefix,
        "swapDevices",
        "swaps",
        changes,
        applied_mappings,
    )?;
    move_legacy_nested_field(
        value,
        prefix,
        "luksDevices",
        "luks",
        "devices",
        changes,
        applied_mappings,
    )?;
    move_legacy_nested_field(
        value,
        prefix,
        "nfsMounts",
        "nfs",
        "mounts",
        changes,
        applied_mappings,
    )?;
    move_legacy_nested_field(
        value,
        prefix,
        "iscsiSessions",
        "iscsi",
        "sessions",
        changes,
        applied_mappings,
    )?;
    Ok(())
}

fn rename_legacy_field(
    value: &mut Value,
    prefix: &str,
    legacy: &str,
    current: &str,
    changes: &mut Vec<String>,
    applied_mappings: &mut Vec<LegacyMigrationMapping>,
) -> Result<(), AppError> {
    let Value::Object(object) = value else {
        return Ok(());
    };
    if !object.contains_key(legacy) {
        return Ok(());
    }
    if object.contains_key(current) {
        return Err(AppError::Message(format!(
            "legacy field {prefix}{legacy} conflicts with current field {prefix}{current}"
        )));
    }
    let Some(mapped) = object.remove(legacy) else {
        return Ok(());
    };
    object.insert(current.to_string(), mapped);
    changes.push(format!(
        "mapped legacy field {prefix}{legacy} to {prefix}{current}"
    ));
    applied_mappings.push(legacy_mapping(prefix, legacy, current));
    Ok(())
}

fn move_legacy_nested_field(
    value: &mut Value,
    prefix: &str,
    legacy: &str,
    parent: &str,
    child: &str,
    changes: &mut Vec<String>,
    applied_mappings: &mut Vec<LegacyMigrationMapping>,
) -> Result<(), AppError> {
    let Value::Object(object) = value else {
        return Ok(());
    };
    if !object.contains_key(legacy) {
        return Ok(());
    }

    if object
        .get(parent)
        .and_then(Value::as_object)
        .is_some_and(|parent| parent.contains_key(child))
    {
        return Err(AppError::Message(format!(
            "legacy field {prefix}{legacy} conflicts with current field {prefix}{parent}.{child}"
        )));
    }

    let Some(mapped) = object.remove(legacy) else {
        return Ok(());
    };
    let parent_value = object
        .entry(parent.to_string())
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    let Value::Object(parent_object) = parent_value else {
        return Err(AppError::Message(format!(
            "legacy field {prefix}{legacy} cannot be mapped because {prefix}{parent} is not an object"
        )));
    };
    parent_object.insert(child.to_string(), mapped);
    changes.push(format!(
        "mapped legacy field {prefix}{legacy} to {prefix}{parent}.{child}"
    ));
    applied_mappings.push(legacy_mapping(prefix, legacy, &format!("{parent}.{child}")));
    Ok(())
}

fn legacy_mapping(prefix: &str, source: &str, target: &str) -> LegacyMigrationMapping {
    LegacyMigrationMapping {
        source: format!("{prefix}{source}"),
        target: format!("{prefix}{target}"),
        scope: if prefix.is_empty() {
            "top-level".to_string()
        } else {
            prefix.trim_end_matches('.').to_string()
        },
    }
}

fn legacy_pre_version_mappings() -> Vec<LegacyMigrationMapping> {
    ["", "spec."]
        .into_iter()
        .flat_map(|prefix| {
            [
                ("fileSystems", "filesystems"),
                ("swapDevices", "swaps"),
                ("luksDevices", "luks.devices"),
                ("nfsMounts", "nfs.mounts"),
                ("iscsiSessions", "iscsi.sessions"),
            ]
            .into_iter()
            .map(move |(source, target)| legacy_mapping(prefix, source, target))
        })
        .collect()
}

fn version_migration_contracts() -> Vec<VersionMigrationContract> {
    vec![
        VersionMigrationContract {
            source_version: None,
            target_version: SUPPORTED_SPEC_VERSION,
            status: "supported".to_string(),
            mapping_scope: "pre-version legacy aliases to version 1".to_string(),
            field_mappings: legacy_pre_version_mappings(),
            safety_notes: vec![
                "applies only to unversioned documents".to_string(),
                "does not apply storage mutations".to_string(),
                "conflicting legacy and current fields are rejected".to_string(),
            ],
        },
        VersionMigrationContract {
            source_version: Some(SUPPORTED_SPEC_VERSION),
            target_version: SUPPORTED_SPEC_VERSION,
            status: "supported".to_string(),
            mapping_scope: "version 1 metadata normalization".to_string(),
            field_mappings: Vec::new(),
            safety_notes: vec![
                "explicit version 1 documents are validated without legacy alias rewrites"
                    .to_string(),
                "does not apply storage mutations".to_string(),
            ],
        },
    ]
}

fn migration_source_version(value: &Value) -> Result<Option<u64>, AppError> {
    let top_level = optional_version_field(value, "version")?;
    let spec = value
        .get("spec")
        .map(|spec| optional_version_field(spec, "spec.version"))
        .transpose()?
        .flatten();
    if let (Some(top_level), Some(spec)) = (top_level, spec) {
        if top_level != spec {
            return Err(AppError::Message(format!(
                "conflicting disk-nix spec versions: top-level version {top_level}, spec.version {spec}"
            )));
        }
    }
    Ok(top_level.or(spec))
}

fn optional_version_field(value: &Value, location: &str) -> Result<Option<u64>, AppError> {
    let Some(version) = value.get("version") else {
        return Ok(None);
    };
    version.as_u64().map(Some).ok_or_else(|| {
        AppError::Message(format!(
            "disk-nix spec version at {location} must be an integer"
        ))
    })
}

fn ensure_object_version(
    value: &mut Value,
    location: &str,
    target_version: u64,
    changes: &mut Vec<String>,
) -> Result<(), AppError> {
    let Value::Object(object) = value else {
        return Err(AppError::Message(format!(
            "disk-nix spec at {location} must be an object to add version metadata"
        )));
    };
    match object.get("version").and_then(Value::as_u64) {
        Some(version) if version == target_version => Ok(()),
        Some(version) => Err(AppError::Message(format!(
            "unsupported disk-nix spec version {version}; supported migration target is {target_version}"
        ))),
        None => {
            object.insert("version".to_string(), Value::from(target_version));
            changes.push(format!("set {location} to {target_version}"));
            Ok(())
        }
    }
}

fn print_migration_report(output: &mut impl Write, report: &MigrationReport) -> io::Result<()> {
    writeln!(
        output,
        "Migration: {:?} -> {}",
        report.source_version, report.target_version
    )?;
    writeln!(output, "migrated: {}", report.migrated)?;
    writeln!(output, "Changes:")?;
    for change in &report.changes {
        writeln!(output, "- {change}")?;
    }
    writeln!(output, "Version migration contracts:")?;
    for contract in &report.version_migrations {
        writeln!(
            output,
            "- {:?} -> {}: {} ({})",
            contract.source_version,
            contract.target_version,
            contract.status,
            contract.mapping_scope
        )?;
        if contract.field_mappings.is_empty() {
            writeln!(output, "  field mappings: none")?;
        } else {
            writeln!(output, "  field mappings:")?;
            for mapping in &contract.field_mappings {
                writeln!(
                    output,
                    "  - {} -> {} ({})",
                    mapping.source, mapping.target, mapping.scope
                )?;
            }
        }
    }
    writeln!(output, "Legacy mappings:")?;
    for mapping in &report.legacy_mappings {
        writeln!(
            output,
            "- {} -> {} ({})",
            mapping.source, mapping.target, mapping.scope
        )?;
    }
    writeln!(output, "Applied mappings:")?;
    if report.applied_mappings.is_empty() {
        writeln!(output, "- none")?;
    } else {
        for mapping in &report.applied_mappings {
            writeln!(
                output,
                "- {} -> {} ({})",
                mapping.source, mapping.target, mapping.scope
            )?;
        }
    }
    writeln!(output, "Warnings:")?;
    for warning in &report.warnings {
        writeln!(output, "- {warning}")?;
    }
    writeln!(output, "Migrated spec:")?;
    writeln!(
        output,
        "{}",
        serde_json::to_string_pretty(&report.spec).map_err(io::Error::other)?
    )
}

fn prepare_apply_report(
    spec: &str,
    probe_current: bool,
    mode: ExecutionMode,
) -> Result<ExecutionReport, AppError> {
    let bytes = std::fs::read(spec)?;
    let (mut plan, mut policy) = plan_and_policy_from_json_bytes(&bytes)
        .map_err(|error| AppError::Message(format!("failed to parse {spec}: {error}")))?;
    if probe_current {
        plan = compare_plan_with_topology(plan, &collect_graph()?);
    }
    apply_confirmation_file(&mut policy)?;
    Ok(prepare_execution(&plan, policy, mode))
}

fn apply_confirmation_file(policy: &mut ApplyPolicy) -> Result<(), AppError> {
    let Some(path) = policy.require_confirmation_file.as_deref() else {
        return Ok(());
    };

    match std::fs::read_to_string(path) {
        Ok(content) if confirmation_file_accepts(&content) => {
            policy.confirmation = true;
            Ok(())
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::Io(error)),
    }
}

fn confirmation_file_accepts(content: &str) -> bool {
    content
        .lines()
        .any(|line| line.trim() == "disk-nix confirm")
}

fn write_execution_script(path: &str, report: &ExecutionReport) -> Result<(), AppError> {
    let script = report
        .to_shell_script()
        .ok_or_else(|| AppError::Message(script_refusal_message(report)))?;
    std::fs::write(path, script)?;
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

fn script_refusal_message(report: &ExecutionReport) -> String {
    let graph_dependency_conflict_count =
        report.topology_comparison.as_ref().map_or(0, |comparison| {
            comparison.summary.graph_dependency_conflict_count
        });
    let partially_suppressed_group_count =
        report.topology_comparison.as_ref().map_or(0, |comparison| {
            comparison.summary.partially_suppressed_group_count
        });
    let mut reasons = Vec::new();
    if !report.apply.can_execute() {
        reasons.push(format!(
            "apply policy blocks {} action(s)",
            report.apply.blocked_count
        ));
    }
    if graph_dependency_conflict_count > 0 {
        reasons.push(format!(
            "{graph_dependency_conflict_count} graph dependency conflict(s) require plan splitting or ordering review"
        ));
    }
    if partially_suppressed_group_count > 0 {
        reasons.push(format!(
            "{partially_suppressed_group_count} partially suppressed reconciliation group(s) require fresh-topology review or plan splitting"
        ));
    }
    if !report.command_summary.all_commands_ready() {
        reasons.push(format!(
            "{} command(s) need desired size, {} need domain command implementation, {} are manual-only",
            report.command_summary.needs_desired_size_count,
            report.command_summary.needs_domain_implementation_count,
            report.command_summary.manual_only_count
        ));
    }
    if reasons.is_empty() {
        reasons.push("report is not in a scriptable dry-run state".to_string());
    }
    format!(
        "script generation requires a policy-allowed, conflict-free command plan: {}",
        reasons.join("; ")
    )
}

fn write_execution_report(path: &str, report: &ExecutionReport) -> Result<(), AppError> {
    let mut report_json = report
        .to_json()
        .map_err(|error| AppError::Message(error.to_string()))?;
    report_json.push('\n');
    std::fs::write(path, report_json)?;
    Ok(())
}

fn apply_receipt(
    command: &str,
    spec_path: &str,
    probe_current: bool,
    execute_requested: bool,
    generated_at_unix_seconds: u64,
    report: &ExecutionReport,
) -> ApplyReceipt {
    ApplyReceipt {
        receipt_version: 1,
        command: command.to_string(),
        spec_path: spec_path.to_string(),
        probe_current,
        execute_requested,
        generated_at_unix_seconds,
        report: report.clone(),
    }
}

fn write_apply_receipt(path: &str, receipt: ApplyReceipt) -> Result<(), AppError> {
    let mut receipt_json = serde_json::to_string_pretty(&receipt)
        .map_err(|error| AppError::Message(error.to_string()))?;
    receipt_json.push('\n');
    std::fs::write(path, receipt_json)?;
    Ok(())
}

fn current_unix_seconds() -> Result<u64, AppError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| {
            AppError::Message(format!("system clock is before the Unix epoch: {error}"))
        })
}

fn collect_graph() -> Result<StorageGraph, AppError> {
    let probe = LinuxProbe::new();
    Ok(probe
        .collect()
        .map_err(|error| AppError::Message(error.to_string()))?
        .graph)
}

fn print_filtered_json(
    output: &mut impl Write,
    graph: &StorageGraph,
    predicate: fn(&Node) -> bool,
) -> Result<(), AppError> {
    let matched_ids: BTreeSet<String> = graph
        .nodes
        .iter()
        .filter(|node| predicate(node))
        .map(|node| node.id.0.clone())
        .collect();

    let mut node_ids = matched_ids.clone();
    let edges = graph
        .edges
        .iter()
        .filter(|edge| {
            matched_ids.contains(edge.from.0.as_str()) || matched_ids.contains(edge.to.0.as_str())
        })
        .inspect(|edge| {
            node_ids.insert(edge.from.0.clone());
            node_ids.insert(edge.to.0.clone());
        })
        .cloned()
        .collect();
    let nodes = graph
        .nodes
        .iter()
        .filter(|node| node_ids.contains(node.id.0.as_str()))
        .cloned()
        .collect();
    let filtered = StorageGraph { nodes, edges };

    writeln!(
        output,
        "{}",
        filtered
            .to_json()
            .map_err(|error| AppError::Message(error.to_string()))?
    )?;
    Ok(())
}

fn print_inspect_json(
    output: &mut impl Write,
    graph: &StorageGraph,
    query: &str,
    depth: usize,
) -> Result<(), AppError> {
    let matched_ids: BTreeSet<String> = graph
        .find_nodes(query)
        .into_iter()
        .map(|node| node.id.0.clone())
        .collect();

    let subgraph = relationship_subgraph(graph, &matched_ids, depth);
    writeln!(
        output,
        "{}",
        subgraph
            .to_json()
            .map_err(|error| AppError::Message(error.to_string()))?
    )?;
    Ok(())
}

fn relationship_subgraph(
    graph: &StorageGraph,
    initial_ids: &BTreeSet<String>,
    depth: usize,
) -> StorageGraph {
    let mut node_ids = initial_ids.clone();
    let mut edge_indexes = BTreeSet::new();
    let mut queue = initial_ids
        .iter()
        .map(|id| (id.clone(), 0_usize))
        .collect::<VecDeque<_>>();

    while let Some((node_id, distance)) = queue.pop_front() {
        if distance >= depth {
            continue;
        }

        for (index, edge) in graph.edges.iter().enumerate() {
            let neighbor = if edge.from.0 == node_id {
                Some(edge.to.0.as_str())
            } else if edge.to.0 == node_id {
                Some(edge.from.0.as_str())
            } else {
                None
            };

            let Some(neighbor) = neighbor else {
                continue;
            };

            edge_indexes.insert(index);
            if node_ids.insert(neighbor.to_string()) {
                queue.push_back((neighbor.to_string(), distance + 1));
            }
        }
    }

    let nodes = graph
        .nodes
        .iter()
        .filter(|node| node_ids.contains(node.id.0.as_str()))
        .cloned()
        .collect();
    let edges = graph
        .edges
        .iter()
        .enumerate()
        .filter(|(index, _)| edge_indexes.contains(index))
        .map(|(_, edge)| edge.clone())
        .collect();

    StorageGraph { nodes, edges }
}

fn print_topology_summary(
    output: &mut impl Write,
    result: &disk_nix_probe::ProbeResult,
) -> io::Result<()> {
    writeln!(output, "Storage topology probe")?;
    writeln!(output, "nodes: {}", result.graph.nodes.len())?;
    writeln!(output, "edges: {}", result.graph.edges.len())?;
    writeln!(output)?;
    print_probe_reports(output, &result.reports)?;

    Ok(())
}

fn collect_probe_preflight_environment() -> ProbePreflightEnvironment {
    let os_release = fs::read_to_string("/etc/os-release")
        .ok()
        .map(|contents| parse_os_release(&contents))
        .unwrap_or_default();
    ProbePreflightEnvironment {
        os_id: os_release
            .iter()
            .find(|(key, _)| key == "ID")
            .map(|(_, value)| value.clone()),
        os_version_id: os_release
            .iter()
            .find(|(key, _)| key == "VERSION_ID")
            .map(|(_, value)| value.clone()),
        os_pretty_name: os_release
            .iter()
            .find(|(key, _)| key == "PRETTY_NAME")
            .map(|(_, value)| value.clone()),
        kernel_release: command_stdout_first_line("uname", &["-r"]).ok(),
        effective_uid: command_stdout_first_line("id", &["-u"]).ok(),
        tool_versions: storage_tool_version_reports(),
    }
}

fn parse_os_release(contents: &str) -> Vec<(String, String)> {
    contents
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            Some((key.to_string(), unquote_os_release_value(value)))
        })
        .collect()
}

fn unquote_os_release_value(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        value[1..value.len() - 1]
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
    } else {
        value.to_string()
    }
}

fn storage_tool_version_reports() -> Vec<ToolVersionReport> {
    [
        ("lsblk", &["--version"][..]),
        ("blkid", &["--version"][..]),
        ("findmnt", &["--version"][..]),
        ("parted", &["--version"][..]),
        ("smartctl", &["--version"][..]),
        ("cryptsetup", &["--version"][..]),
        ("dmsetup", &["version"][..]),
        ("lvm", &["version"][..]),
        ("vdo", &["--version"][..]),
        ("zpool", &["--version"][..]),
        ("zfs", &["--version"][..]),
        ("btrfs", &["--version"][..]),
        ("bcachefs", &["version"][..]),
        ("lsscsi", &["--version"][..]),
        ("iscsiadm", &["--version"][..]),
        ("exportfs", &["--version"][..]),
        ("nfsstat", &["--version"][..]),
        ("mdadm", &["--version"][..]),
        ("multipath", &["-h"][..]),
        ("nvme", &["version"][..]),
    ]
    .into_iter()
    .map(|(tool, args)| storage_tool_version_report(tool, args))
    .collect()
}

fn storage_tool_version_report(tool: &str, args: &[&str]) -> ToolVersionReport {
    match command_stdout_first_line(tool, args) {
        Ok(version) => ToolVersionReport {
            tool: tool.to_string(),
            status: ToolVersionStatus::Available,
            version: Some(version),
            message: None,
        },
        Err(message) if message.contains("not found") || message.contains("No such file") => {
            ToolVersionReport {
                tool: tool.to_string(),
                status: ToolVersionStatus::Unavailable,
                version: None,
                message: Some(message),
            }
        }
        Err(message) => ToolVersionReport {
            tool: tool.to_string(),
            status: ToolVersionStatus::Failed,
            version: None,
            message: Some(message),
        },
    }
}

fn probe_preflight_checks(environment: &ProbePreflightEnvironment) -> ProbePreflightChecks {
    let root = environment.effective_uid.as_deref() == Some("0");
    let missing_tools = environment
        .tool_versions
        .iter()
        .filter(|tool| tool.status == ToolVersionStatus::Unavailable)
        .map(|tool| tool.tool.clone())
        .collect::<Vec<_>>();
    let failed_tools = environment
        .tool_versions
        .iter()
        .filter(|tool| tool.status == ToolVersionStatus::Failed)
        .map(|tool| tool.tool.clone())
        .collect::<Vec<_>>();
    let mut remediation = Vec::new();
    if !root {
        remediation.push(
            "run probe-status with privileges when adapter metadata requires root-only kernel or device access"
                .to_string(),
        );
    }
    if !missing_tools.is_empty() {
        remediation.push(format!(
            "install or expose missing storage tool(s): {}",
            missing_tools.join(", ")
        ));
        remediation.push(
            "on NixOS, add the required storage packages to environment.systemPackages or services.disk-nix.toolPackages"
                .to_string(),
        );
    }
    if !failed_tools.is_empty() {
        remediation.push(format!(
            "rerun failed storage tool version probe(s) manually with stderr captured: {}",
            failed_tools.join(", ")
        ));
    }
    let status = if root && missing_tools.is_empty() && failed_tools.is_empty() {
        ProbePreflightCheckStatus::Ready
    } else {
        ProbePreflightCheckStatus::Degraded
    };

    ProbePreflightChecks {
        status,
        root,
        unavailable_tool_count: missing_tools.len(),
        failed_tool_count: failed_tools.len(),
        missing_tools,
        failed_tools,
        adapter_remediation: preflight_adapter_remediation(),
        remediation,
    }
}

fn preflight_adapter_remediation() -> Vec<ProbeAdapterRemediation> {
    [
        "lsblk",
        "blkid",
        "findmnt",
        "udev",
        "parted",
        "smartctl",
        "ext",
        "xfs",
        "btrfs",
        "bcachefs",
        "bcache",
        "cryptsetup",
        "dmsetup",
        "lvm",
        "vdo",
        "vdostats",
        "vdostats-verbose",
        "mdraid",
        "mdadm-scan",
        "mdadm-examine",
        "multipath",
        "nfs",
        "nfs-exports",
        "iscsi",
        "iscsi-nodes",
        "lsscsi",
        "nvme",
        "nvme-list-subsys",
        "nvme-smart-log",
        "nvme-id-ctrl",
        "nvme-id-ns",
        "loop",
        "swaps",
        "zramctl",
        "zfs",
    ]
    .into_iter()
    .map(adapter_remediation)
    .collect()
}

fn command_stdout_first_line(command: &str, args: &[&str]) -> Result<String, String> {
    match ProcessCommand::new(command).args(args).output() {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let line = stdout
                .lines()
                .chain(stderr.lines())
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or("");
            if line.is_empty() {
                Err(format!("{command} {:?} returned no version text", args))
            } else {
                Ok(line.to_string())
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let detail = stderr
                .lines()
                .chain(stdout.lines())
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or("command returned a non-zero status");
            Err(format!(
                "{command} {:?} failed with status {}: {detail}",
                args, output.status
            ))
        }
        Err(error) => Err(format!("{command} not found or failed to run: {error}")),
    }
}

fn print_probe_preflight_environment(
    output: &mut impl Write,
    environment: &ProbePreflightEnvironment,
) -> io::Result<()> {
    writeln!(output, "Preflight environment:")?;
    writeln!(
        output,
        "  os: {}",
        environment.os_pretty_name.as_deref().unwrap_or("-")
    )?;
    writeln!(
        output,
        "  os-id: {}",
        environment.os_id.as_deref().unwrap_or("-")
    )?;
    writeln!(
        output,
        "  os-version-id: {}",
        environment.os_version_id.as_deref().unwrap_or("-")
    )?;
    writeln!(
        output,
        "  kernel: {}",
        environment.kernel_release.as_deref().unwrap_or("-")
    )?;
    writeln!(
        output,
        "  effective-uid: {}",
        environment.effective_uid.as_deref().unwrap_or("-")
    )?;
    writeln!(output, "  storage tools:")?;
    for tool in &environment.tool_versions {
        let status = match tool.status {
            ToolVersionStatus::Available => "available",
            ToolVersionStatus::Unavailable => "unavailable",
            ToolVersionStatus::Failed => "failed",
        };
        let detail = tool
            .version
            .as_deref()
            .or(tool.message.as_deref())
            .unwrap_or("-");
        writeln!(output, "    {:<12} {:<12} {}", tool.tool, status, detail)?;
    }
    writeln!(output)?;
    Ok(())
}

fn print_probe_preflight_checks(
    output: &mut impl Write,
    checks: &ProbePreflightChecks,
) -> io::Result<()> {
    let status = match checks.status {
        ProbePreflightCheckStatus::Ready => "ready",
        ProbePreflightCheckStatus::Degraded => "degraded",
    };
    writeln!(output, "Preflight checks:")?;
    writeln!(output, "  status: {status}")?;
    writeln!(output, "  root: {}", checks.root)?;
    writeln!(
        output,
        "  unavailable-tools: {}",
        checks.unavailable_tool_count
    )?;
    writeln!(output, "  failed-tools: {}", checks.failed_tool_count)?;
    if !checks.missing_tools.is_empty() {
        writeln!(
            output,
            "  missing-tools: {}",
            checks.missing_tools.join(", ")
        )?;
    }
    if !checks.failed_tools.is_empty() {
        writeln!(
            output,
            "  failed-tool-names: {}",
            checks.failed_tools.join(", ")
        )?;
    }
    for remediation in &checks.remediation {
        writeln!(output, "    remediation: {remediation}")?;
    }
    writeln!(output)?;
    Ok(())
}

fn print_probe_reports(
    output: &mut impl Write,
    reports: &[disk_nix_probe::ProbeReport],
) -> io::Result<()> {
    writeln!(output, "Adapters:")?;

    for report in reports {
        let status = match report.status {
            ProbeStatus::Available => "available",
            ProbeStatus::Unavailable => "unavailable",
            ProbeStatus::Partial => "partial",
            ProbeStatus::Failed => "failed",
        };
        let category = match report.category() {
            ProbeIssueCategory::None => "none",
            ProbeIssueCategory::MissingTool => "missing-tool",
            ProbeIssueCategory::PermissionDenied => "permission-denied",
            ProbeIssueCategory::CommandFailed => "command-failed",
            ProbeIssueCategory::ParseFailed => "parse-failed",
            ProbeIssueCategory::InaccessibleData => "inaccessible-data",
        };

        if let Some(message) = &report.message {
            writeln!(
                output,
                "  {:<12} {:<12} {:<18} {}",
                report.adapter, status, category, message
            )?;
        } else {
            writeln!(
                output,
                "  {:<12} {:<12} {}",
                report.adapter, status, category
            )?;
        }
        for remediation in report.remediation() {
            writeln!(output, "    remediation: {remediation}")?;
        }
    }

    Ok(())
}

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

fn print_inspect(
    output: &mut impl Write,
    graph: &StorageGraph,
    query: &str,
    depth: usize,
) -> io::Result<()> {
    let matches = graph.find_nodes(query);

    if matches.is_empty() {
        writeln!(output, "No storage graph nodes matched '{query}'.")?;
        return Ok(());
    }

    for (index, node) in matches.iter().enumerate() {
        if index > 0 {
            writeln!(output)?;
        }

        writeln!(output, "{} {}", node.kind, node.name)?;
        writeln!(output, "  id: {}", node.id.0)?;
        if let Some(path) = &node.path {
            writeln!(output, "  path: {path}")?;
        }
        if let Some(size_bytes) = node.size_bytes {
            writeln!(output, "  size: {}", human_bytes(Some(size_bytes)))?;
        }
        if let Some(usage) = &node.usage {
            if usage.used_bytes.is_some()
                || usage.free_bytes.is_some()
                || usage.allocated_bytes.is_some()
            {
                writeln!(
                    output,
                    "  usage: used={} free={} allocated={} use={}",
                    human_bytes(usage.used_bytes),
                    human_bytes(usage.free_bytes),
                    human_bytes(usage.allocated_bytes),
                    usage_percent(node)
                )?;
            }
        }

        print_identity(output, node)?;
        print_properties(output, node)?;
        print_relationships(output, graph, node, depth)?;
    }

    Ok(())
}

fn print_identity(output: &mut impl Write, node: &Node) -> io::Result<()> {
    if node.identity.is_empty() {
        return Ok(());
    }

    writeln!(output, "  identity:")?;
    for (key, value) in [
        ("uuid", node.identity.uuid.as_deref()),
        ("partuuid", node.identity.partuuid.as_deref()),
        ("label", node.identity.label.as_deref()),
        ("serial", node.identity.serial.as_deref()),
        ("wwn", node.identity.wwn.as_deref()),
    ] {
        if let Some(value) = value {
            writeln!(output, "    {key}: {value}")?;
        }
    }
    Ok(())
}

fn print_properties(output: &mut impl Write, node: &Node) -> io::Result<()> {
    if node.properties.is_empty() {
        return Ok(());
    }

    writeln!(output, "  properties:")?;
    for property in &node.properties {
        writeln!(output, "    {}: {}", property.key, property.value)?;
    }
    Ok(())
}

fn print_relationships(
    output: &mut impl Write,
    graph: &StorageGraph,
    node: &Node,
    depth: usize,
) -> io::Result<()> {
    let mut initial_ids = BTreeSet::new();
    initial_ids.insert(node.id.0.clone());
    let subgraph = relationship_subgraph(graph, &initial_ids, depth);
    let edges = subgraph.edges.iter().collect::<Vec<_>>();
    if edges.is_empty() {
        return Ok(());
    }

    writeln!(output, "  relationships:")?;
    for edge in edges {
        if depth <= 1 {
            let direction = if edge.from == node.id { "out" } else { "in" };
            let other_id = if edge.from == node.id {
                &edge.to
            } else {
                &edge.from
            };
            let other_name = graph
                .nodes
                .iter()
                .find(|candidate| &candidate.id == other_id)
                .map(|candidate| candidate.name.as_str())
                .unwrap_or(other_id.0.as_str());

            writeln!(
                output,
                "    {direction} {} {} ({})",
                edge.relationship, other_id.0, other_name
            )?;
        } else {
            let from_name = graph
                .nodes
                .iter()
                .find(|candidate| candidate.id == edge.from)
                .map(|candidate| candidate.name.as_str())
                .unwrap_or(edge.from.0.as_str());
            let to_name = graph
                .nodes
                .iter()
                .find(|candidate| candidate.id == edge.to)
                .map(|candidate| candidate.name.as_str())
                .unwrap_or(edge.to.0.as_str());

            writeln!(
                output,
                "    {} ({}) {} {} ({})",
                edge.from.0, from_name, edge.relationship, edge.to.0, to_name
            )?;
        }
    }

    Ok(())
}

fn print_plan(output: &mut impl Write, plan: &Plan) -> io::Result<()> {
    writeln!(
        output,
        "Plan: {} actions, {} offline required, {} destructive, {} potential data loss, {} unsupported",
        plan.summary.action_count,
        plan.summary.offline_required_count,
        plan.summary.destructive_count,
        plan.summary.potential_data_loss_count,
        plan.summary.unsupported_count
    )?;

    for action in &plan.actions {
        writeln!(
            output,
            "- {:?} {:?}: {}",
            action.risk, action.operation, action.description
        )?;

        if let Some(advice) = &action.advice {
            writeln!(output, "  advice: {}", advice.summary)?;
            for alternative in &advice.alternatives {
                writeln!(output, "  alternative: {alternative}")?;
            }
        }
    }

    if let Some(comparison) = &plan.topology_comparison {
        print_topology_comparison(output, comparison)?;
    }

    Ok(())
}

fn print_topology_comparison(
    output: &mut impl Write,
    comparison: &TopologyComparison,
) -> io::Result<()> {
    writeln!(
        output,
        "Topology comparison: {} actions, {} matched, {} missing, {} size notes, {} type conflicts, {} already satisfied, {} suppressed, {} graph dependency conflicts",
        comparison.summary.action_count,
        comparison.summary.matched_count,
        comparison.summary.missing_count,
        comparison.summary.size_diagnostic_count,
        comparison.summary.type_conflict_count,
        comparison.summary.already_satisfied_count,
        comparison.summary.suppressed_action_count,
        comparison.summary.graph_dependency_conflict_count
    )?;

    for diagnostic in &comparison.diagnostics {
        let level = match diagnostic.level {
            TopologyDiagnosticLevel::Info => "info",
            TopologyDiagnosticLevel::Warning => "warning",
        };
        writeln!(
            output,
            "  {level}: {:?} {}: {}",
            diagnostic.kind, diagnostic.action_id, diagnostic.message
        )?;
    }

    Ok(())
}

fn print_execution_report(
    output: &mut impl Write,
    report: &ExecutionReport,
    execute: bool,
) -> io::Result<()> {
    writeln!(
        output,
        "Apply policy: {} allowed, {} blocked",
        report.apply.allowed_count, report.apply.blocked_count
    )?;
    writeln!(output, "mode: {:?}", report.apply.policy.mode)?;
    writeln!(output, "status: {:?}", report.status)?;
    writeln!(output, "execute requested: {execute}")?;
    if let Some(comparison) = &report.topology_comparison {
        print_topology_comparison(output, comparison)?;
    }

    if report.apply.blocked.is_empty() {
        writeln!(output, "No policy blocks detected.")?;
        for message in &report.messages {
            writeln!(output, "{message}")?;
        }
        if !report.command_plan.is_empty() {
            writeln!(
                output,
                "Command summary: {} steps, {} commands, {} mutating, {} manual review, {} ready, {} need size, {} need implementation, {} manual only",
                report.command_summary.step_count,
                report.command_summary.command_count,
                report.command_summary.mutating_count,
                report.command_summary.manual_review_count,
                report.command_summary.ready_count,
                report.command_summary.needs_desired_size_count,
                report.command_summary.needs_domain_implementation_count,
                report.command_summary.manual_only_count
            )?;
            writeln!(output, "Command plan:")?;
            if !report.tool_requirements.is_empty() {
                writeln!(output, "Tool requirements:")?;
                for requirement in &report.tool_requirements {
                    writeln!(
                        output,
                        "- {}: {} commands, {} mutating, {} verification, phases {:?}, availability {:?}",
                        requirement.tool,
                        requirement.command_count,
                        requirement.mutating_count,
                        requirement.verification_count,
                        requirement.phases,
                        requirement.availability
                    )?;
                    writeln!(output, "  {}", requirement.message)?;
                    for remediation in &requirement.remediation {
                        writeln!(output, "  - {remediation}")?;
                    }
                }
            }
            for step in &report.command_plan {
                writeln!(
                    output,
                    "- {:?} {:?} {}",
                    step.risk, step.operation, step.action_id
                )?;
                if step.requires_manual_review {
                    writeln!(output, "  manual review required")?;
                }
                for command in &step.commands {
                    let mutation = if command.mutates {
                        "mutating"
                    } else {
                        "read-only"
                    };
                    writeln!(output, "  {mutation}: {}", command.argv.join(" "))?;
                    writeln!(output, "    readiness: {:?}", command.readiness)?;
                    if !command.unresolved_inputs.is_empty() {
                        writeln!(
                            output,
                            "    unresolved: {}",
                            command.unresolved_inputs.join(", ")
                        )?;
                    }
                    writeln!(output, "    {}", command.note)?;
                }
                for note in &step.notes {
                    writeln!(output, "  note: {note}")?;
                }
            }
        }
        if !report.execution_results.is_empty() {
            writeln!(
                output,
                "Execution results: {} command(s)",
                report.execution_results.len()
            )?;
            for result in &report.execution_results {
                let status = if result.success { "ok" } else { "failed" };
                writeln!(
                    output,
                    "- {:?} {} {}",
                    result.phase,
                    status,
                    result.argv.join(" ")
                )?;
                if let Some(status_code) = result.status_code {
                    writeln!(output, "  exit: {status_code}")?;
                }
                if !result.stdout.is_empty() {
                    writeln!(output, "  stdout: {}", result.stdout.trim_end())?;
                }
                if !result.stderr.is_empty() {
                    writeln!(output, "  stderr: {}", result.stderr.trim_end())?;
                }
            }
        }
        if !report.verification_plan.is_empty() {
            writeln!(
                output,
                "Verification summary: {} steps, {} read-only commands, {} checks",
                report.verification_summary.step_count,
                report.verification_summary.command_count,
                report.verification_summary.check_count
            )?;
            writeln!(output, "Verification plan:")?;
            for step in &report.verification_plan {
                writeln!(
                    output,
                    "- {:?} {:?} {}",
                    step.risk, step.operation, step.action_id
                )?;
                for command in &step.commands {
                    writeln!(output, "  read-only: {}", command.argv.join(" "))?;
                    writeln!(output, "    {}", command.note)?;
                }
                for check in &step.checks {
                    writeln!(output, "  check: {check}")?;
                }
            }
        }
    } else {
        writeln!(
            output,
            "Blocked summary: {} offline required, {} destructive, {} potential data loss, {} unsupported",
            report.apply.blocked_summary.offline_required_count,
            report.apply.blocked_summary.destructive_count,
            report.apply.blocked_summary.potential_data_loss_count,
            report.apply.blocked_summary.unsupported_count
        )?;
        writeln!(output, "Blocked actions:")?;
        for blocked in &report.apply.blocked {
            writeln!(
                output,
                "- {:?} {:?} {}: {}",
                blocked.risk, blocked.operation, blocked.id, blocked.reason
            )?;
        }
    }

    if !report.recovery_actions.is_empty() {
        writeln!(output, "Recovery actions:")?;
        for action in &report.recovery_actions {
            writeln!(output, "- {:?}: {}", action.kind, action.summary)?;
            for command in &action.commands {
                let mutation = if command.mutates {
                    "mutating"
                } else {
                    "read-only"
                };
                writeln!(output, "  {mutation}: {}", command.argv.join(" "))?;
                writeln!(output, "    readiness: {:?}", command.readiness)?;
                writeln!(output, "    {}", command.note)?;
            }
            for note in &action.notes {
                writeln!(output, "  note: {note}")?;
            }
        }
    }

    Ok(())
}

fn is_device_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::PhysicalDisk
            | NodeKind::Partition
            | NodeKind::LuksContainer
            | NodeKind::DeviceMapper
            | NodeKind::LvmLogicalVolume
            | NodeKind::LvmPhysicalVolume
            | NodeKind::LvmVolumeGroup
            | NodeKind::LvmThinPool
            | NodeKind::LvmSnapshot
            | NodeKind::LvmCache
            | NodeKind::VdoVolume
            | NodeKind::MdRaid
            | NodeKind::Zvol
            | NodeKind::CacheDevice
            | NodeKind::MultipathDevice
            | NodeKind::NvmeSubsystem
            | NodeKind::NvmeNamespace
            | NodeKind::LoopDevice
            | NodeKind::BcachefsDevice
            | NodeKind::BackingFile
            | NodeKind::ZramDevice
            | NodeKind::Swap
    )
}

fn is_partition_node(node: &Node) -> bool {
    node.kind == NodeKind::Partition
}

fn is_filesystem_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::Filesystem
            | NodeKind::BtrfsFilesystem
            | NodeKind::BtrfsSubvolume
            | NodeKind::BtrfsSnapshot
            | NodeKind::BtrfsQgroup
            | NodeKind::BcachefsFilesystem
            | NodeKind::ZfsDataset
            | NodeKind::ZfsSnapshot
            | NodeKind::NfsExport
    )
}

fn is_complex_filesystem_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::BtrfsFilesystem
            | NodeKind::BtrfsSubvolume
            | NodeKind::BtrfsSnapshot
            | NodeKind::BtrfsQgroup
            | NodeKind::BcachefsFilesystem
            | NodeKind::BcachefsDevice
            | NodeKind::ZfsPool
            | NodeKind::ZfsVdev
            | NodeKind::ZfsDataset
            | NodeKind::ZfsSnapshot
            | NodeKind::Zvol
    ) || node.properties.iter().any(|property| {
        property.key.starts_with("btrfs.")
            || property.key.starts_with("bcachefs.")
            || property.key.starts_with("zfs.")
    })
}

fn is_btrfs_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::BtrfsFilesystem
            | NodeKind::BtrfsSubvolume
            | NodeKind::BtrfsSnapshot
            | NodeKind::BtrfsQgroup
    ) || node
        .properties
        .iter()
        .any(|property| property.key.starts_with("btrfs."))
}

fn is_bcachefs_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::BcachefsFilesystem | NodeKind::BcachefsDevice
    ) || node
        .properties
        .iter()
        .any(|property| property.key.starts_with("bcachefs."))
}

fn is_zfs_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::ZfsPool
            | NodeKind::ZfsVdev
            | NodeKind::ZfsDataset
            | NodeKind::ZfsSnapshot
            | NodeKind::Zvol
    ) || node
        .properties
        .iter()
        .any(|property| property.key.starts_with("zfs."))
}

fn is_volume_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::LvmVolumeGroup
            | NodeKind::LvmLogicalVolume
            | NodeKind::LvmSegment
            | NodeKind::LvmThinPool
            | NodeKind::LvmSnapshot
            | NodeKind::LvmCache
            | NodeKind::VdoVolume
            | NodeKind::MdRaid
            | NodeKind::BtrfsFilesystem
            | NodeKind::BtrfsSubvolume
            | NodeKind::BtrfsSnapshot
            | NodeKind::BtrfsQgroup
            | NodeKind::BcachefsFilesystem
            | NodeKind::ZfsPool
            | NodeKind::ZfsDataset
            | NodeKind::ZfsSnapshot
            | NodeKind::Zvol
            | NodeKind::Lun
            | NodeKind::NfsExport
    )
}

fn is_pool_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::LvmVolumeGroup
            | NodeKind::LvmThinPool
            | NodeKind::BtrfsFilesystem
            | NodeKind::BtrfsQgroup
            | NodeKind::BcachefsFilesystem
            | NodeKind::ZfsPool
            | NodeKind::ZfsVdev
            | NodeKind::MdRaid
    )
}

fn is_snapshot_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::LvmSnapshot | NodeKind::BtrfsSnapshot | NodeKind::ZfsSnapshot
    )
}

fn is_mapping_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::LuksContainer
            | NodeKind::DeviceMapper
            | NodeKind::LvmLogicalVolume
            | NodeKind::LvmSegment
            | NodeKind::LvmThinPool
            | NodeKind::LvmSnapshot
            | NodeKind::LvmCache
            | NodeKind::VdoVolume
            | NodeKind::MdRaid
            | NodeKind::MultipathDevice
            | NodeKind::LoopDevice
            | NodeKind::CacheDevice
            | NodeKind::BcachefsDevice
    )
}

fn is_dm_node(node: &Node) -> bool {
    node.kind == NodeKind::DeviceMapper
        || node
            .properties
            .iter()
            .any(|property| property.key.starts_with("dm."))
}

fn is_encryption_node(node: &Node) -> bool {
    node.kind == NodeKind::LuksContainer
        || node
            .properties
            .iter()
            .any(|property| property.key.starts_with("cryptsetup."))
}

fn is_cache_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::LvmCache | NodeKind::CacheDevice | NodeKind::BcachefsDevice
    ) || node.properties.iter().any(|property| {
        property.key.starts_with("bcache.")
            || property.key.starts_with("bcachefs.device-")
            || property.key == "lvm.cache-mode"
            || property.key == "lvm.cache-policy"
            || property.key == "lvm.kernel-cache-mode"
            || property.key == "lvm.kernel-cache-policy"
            || property.key == "lvm.cache-metadata-format"
            || property.key == "lvm.segment-cache-mode"
            || property.key == "lvm.segment-cache-policy"
            || property.key == "lvm.cache-settings"
            || property.key.starts_with("lvm.writecache-")
            || (property.key == "zfs.vdev-role" && property.value == "cache")
    })
}

fn is_lvm_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::LvmPhysicalVolume
            | NodeKind::LvmVolumeGroup
            | NodeKind::LvmLogicalVolume
            | NodeKind::LvmSegment
            | NodeKind::LvmThinPool
            | NodeKind::LvmSnapshot
            | NodeKind::LvmCache
    ) || node
        .properties
        .iter()
        .any(|property| property.key.starts_with("lvm."))
}

fn is_vdo_node(node: &Node) -> bool {
    node.kind == NodeKind::VdoVolume
        || node.properties.iter().any(|property| {
            property.key.starts_with("vdo.") || property.key.starts_with("lvm.vdo-")
        })
}

fn is_multipath_node(node: &Node) -> bool {
    node.kind == NodeKind::MultipathDevice
        || node
            .properties
            .iter()
            .any(|property| property.key.starts_with("multipath."))
}

fn is_nvme_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::NvmeSubsystem | NodeKind::NvmeController | NodeKind::NvmeNamespace
    ) || node
        .properties
        .iter()
        .any(|property| property.key.starts_with("nvme."))
}

fn is_raid_node(node: &Node) -> bool {
    node.kind == NodeKind::MdRaid
        || node
            .properties
            .iter()
            .any(|property| property.key.starts_with("md."))
}

fn is_loop_node(node: &Node) -> bool {
    matches!(node.kind, NodeKind::LoopDevice | NodeKind::BackingFile)
        || node
            .properties
            .iter()
            .any(|property| property.key.starts_with("loop."))
}

fn is_backing_file_node(node: &Node) -> bool {
    node.kind == NodeKind::BackingFile
}

fn is_swap_node(node: &Node) -> bool {
    node.kind == NodeKind::Swap
        || node.kind == NodeKind::ZramDevice
        || property_value(node, "zram.swap") == Some("true")
        || node
            .properties
            .iter()
            .any(|property| property.key.starts_with("swap."))
}

fn is_zram_node(node: &Node) -> bool {
    node.kind == NodeKind::ZramDevice
        || node
            .properties
            .iter()
            .any(|property| property.key.starts_with("zram."))
}

fn is_iscsi_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::IscsiSession | NodeKind::IscsiTarget | NodeKind::Lun
    ) || node
        .properties
        .iter()
        .any(|property| property.key.starts_with("iscsi."))
}

fn is_lun_node(node: &Node) -> bool {
    node.kind == NodeKind::Lun
}

fn is_nfs_node(node: &Node) -> bool {
    matches!(node.kind, NodeKind::NfsExport | NodeKind::NfsMount)
        || node
            .properties
            .iter()
            .any(|property| property.key.starts_with("nfs."))
}

fn is_mount_node(node: &Node) -> bool {
    matches!(node.kind, NodeKind::Mountpoint | NodeKind::NfsMount)
}

fn is_network_storage_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::IscsiSession
            | NodeKind::IscsiTarget
            | NodeKind::Lun
            | NodeKind::NfsExport
            | NodeKind::NfsMount
    )
}

fn has_capacity_or_usage(node: &Node) -> bool {
    node.size_bytes.is_some()
        || node.usage.as_ref().is_some_and(|usage| {
            usage.used_bytes.is_some()
                || usage.free_bytes.is_some()
                || usage.allocated_bytes.is_some()
        })
}

fn usage_details(node: &Node) -> String {
    const DETAIL_KEYS: &[(&str, &str)] = &[
        ("model", "model"),
        ("vendor", "vendor"),
        ("transport", "transport"),
        ("rotational", "rotational"),
        ("md.uuid", "md-uuid"),
        ("scsi.address", "scsi-address"),
        ("scsi.host", "scsi-host"),
        ("scsi.channel", "scsi-channel"),
        ("scsi.target", "scsi-target"),
        ("scsi.lun", "scsi-lun"),
        ("scsi.peripheral-type", "scsi-type"),
        ("scsi.vendor", "scsi-vendor"),
        ("scsi.model", "scsi-model"),
        ("scsi.revision", "scsi-revision"),
        ("scsi.block-device", "scsi-block"),
        ("scsi.generic-device", "scsi-generic"),
        ("scsi.size", "scsi-size"),
        ("scsi.transport", "scsi-transport"),
        ("scsi.unit-name", "scsi-unit"),
        ("scsi.by-id", "scsi-by-id"),
        ("scsi.wwn", "scsi-wwn"),
        ("scsi.state", "scsi-state"),
        ("scsi.queue-depth", "scsi-queue-depth"),
        ("scsi.queue-type", "scsi-queue-type"),
        ("scsi.scsi-level", "scsi-level"),
        ("scsi.timeout", "scsi-timeout"),
        ("smartctl.svn-revision", "smart-svn"),
        ("smartctl.platform", "smart-platform"),
        ("smartctl.exit-status", "smart-exit-status"),
        ("smartctl.device-name", "smart-device-name"),
        ("smartctl.health.passed", "smart-health-passed"),
        ("smartctl.device-type", "smart-device-type"),
        ("smartctl.protocol", "smart-protocol"),
        ("smartctl.model", "smart-model"),
        ("smartctl.model-family", "smart-family"),
        ("smartctl.vendor", "smart-vendor"),
        ("smartctl.product", "smart-product"),
        ("smartctl.revision", "smart-revision"),
        ("smartctl.firmware-version", "smart-firmware"),
        ("smartctl.serial", "smart-serial"),
        ("smartctl.wwn-naa", "smart-wwn-naa"),
        ("smartctl.wwn-oui", "smart-wwn-oui"),
        ("smartctl.wwn-id", "smart-wwn-id"),
        ("smartctl.user-capacity-bytes", "smart-capacity"),
        ("smartctl.logical-block-size", "smart-logical-block"),
        ("smartctl.physical-block-size", "smart-physical-block"),
        ("smartctl.rotation-rate-rpm", "smart-rpm"),
        ("smartctl.form-factor", "smart-form-factor"),
        ("smartctl.sata-version", "sata-version"),
        (
            "smartctl.interface-speed-current",
            "interface-speed-current",
        ),
        ("smartctl.interface-speed-max", "interface-speed-max"),
        ("smartctl.power-on-hours", "smart-power-on-hours"),
        ("smartctl.power-cycle-count", "smart-power-cycles"),
        (
            "smartctl.temperature-current-celsius",
            "smart-temperature-c",
        ),
        (
            "smartctl.temperature-highest-celsius",
            "smart-temperature-highest-c",
        ),
        (
            "smartctl.temperature-lowest-celsius",
            "smart-temperature-lowest-c",
        ),
        (
            "smartctl.offline-data-collection-status",
            "smart-offline-status",
        ),
        ("smartctl.self-test-status", "smart-self-test"),
        ("smartctl.error-log-summary-count", "smart-error-log-count"),
        ("smartctl.self-test-log-count", "smart-self-test-count"),
        ("smartctl.error-logging-supported", "smart-error-logging"),
        ("smartctl.gp-logging-supported", "smart-gp-logging"),
        ("smartctl.sct-capabilities", "smart-sct-capabilities"),
        (
            "smartctl.scsi-grown-defect-list",
            "smart-scsi-grown-defects",
        ),
        (
            "smartctl.attribute.reallocated-sector-ct.raw",
            "reallocated-sectors",
        ),
        (
            "smartctl.attribute.reallocated-sector-ct.value",
            "reallocated-value",
        ),
        (
            "smartctl.attribute.reallocated-sector-ct.worst",
            "reallocated-worst",
        ),
        (
            "smartctl.attribute.reallocated-sector-ct.threshold",
            "reallocated-threshold",
        ),
        (
            "smartctl.attribute.reallocated-sector-ct.when-failed",
            "reallocated-failed",
        ),
        (
            "smartctl.attribute.current-pending-sector.raw",
            "pending-sectors",
        ),
        (
            "smartctl.attribute.current-pending-sector.value",
            "pending-value",
        ),
        (
            "smartctl.attribute.current-pending-sector.worst",
            "pending-worst",
        ),
        (
            "smartctl.attribute.current-pending-sector.threshold",
            "pending-threshold",
        ),
        (
            "smartctl.attribute.current-pending-sector.when-failed",
            "pending-failed",
        ),
        (
            "smartctl.attribute.offline-uncorrectable.raw",
            "offline-uncorrectable",
        ),
        (
            "smartctl.attribute.offline-uncorrectable.value",
            "offline-uncorrectable-value",
        ),
        (
            "smartctl.attribute.offline-uncorrectable.worst",
            "offline-uncorrectable-worst",
        ),
        (
            "smartctl.attribute.offline-uncorrectable.threshold",
            "offline-uncorrectable-threshold",
        ),
        (
            "smartctl.attribute.offline-uncorrectable.when-failed",
            "offline-uncorrectable-failed",
        ),
        ("nvme.generic-path", "generic"),
        ("nvme.model", "nvme-model"),
        ("nvme.product", "product"),
        ("nvme.firmware", "firmware"),
        ("nvme.index", "ns-index"),
        ("nvme.namespace", "namespace"),
        ("nvme.namespace-id", "nsid"),
        ("nvme.namespace-uuid", "ns-uuid"),
        ("nvme.eui64", "eui64"),
        ("nvme.nguid", "nguid"),
        ("nvme.subsystem", "subsystem"),
        ("nvme.subsystem-name", "subsystem-name"),
        ("nvme.subsystem-nqn", "subsystem-nqn"),
        ("nvme.hostnqn", "hostnqn"),
        ("nvme.controller", "controller"),
        ("nvme.address", "address"),
        ("nvme.transport", "transport"),
        ("nvme.traddr", "traddr"),
        ("nvme.trsvcid", "trsvcid"),
        ("nvme.host-traddr", "host-traddr"),
        ("nvme.host-iface", "host-iface"),
        ("nvme.path-state", "path-state"),
        ("nvme.controller-id", "controller-id"),
        ("nvme.namespace-capacity", "namespace-capacity"),
        ("nvme.lba-format", "lba-format"),
        ("nvme.maximum-lba", "max-lba"),
        ("nvme.sector-size", "sector-size"),
        ("nvme.ana-state", "ana-state"),
        ("nvme.formatted-lba-index", "flba-index"),
        ("nvme.formatted-lba-data-size", "flba-data"),
        ("nvme.formatted-lba-metadata-size", "flba-metadata"),
        (
            "nvme.formatted-lba-relative-performance",
            "flba-relative-performance",
        ),
        ("nvme.id-ns.nsze", "nsze"),
        ("nvme.id-ns.ncap", "ncap"),
        ("nvme.id-ns.nuse", "nuse"),
        ("nvme.id-ns.nsfeat", "nsfeat"),
        ("nvme.id-ns.nlbaf", "nlbaf"),
        ("nvme.id-ns.flbas", "flbas"),
        ("nvme.id-ns.mc", "mc"),
        ("nvme.id-ns.dpc", "dpc"),
        ("nvme.id-ns.dps", "dps"),
        ("nvme.id-ns.nmic", "nmic"),
        ("nvme.id-ns.rescap", "rescap"),
        ("nvme.id-ns.fpi", "fpi"),
        ("nvme.id-ns.dlfeat", "dlfeat"),
        ("nvme.id-ns.nawun", "nawun"),
        ("nvme.id-ns.nawupf", "nawupf"),
        ("nvme.id-ns.nacwu", "nacwu"),
        ("nvme.id-ns.nabsn", "nabsn"),
        ("nvme.id-ns.nabo", "nabo"),
        ("nvme.id-ns.nabspf", "nabspf"),
        ("nvme.id-ns.noiob", "noiob"),
        ("nvme.id-ns.nvmcap", "nvmcap"),
        ("nvme.id-ctrl.vid", "vid"),
        ("nvme.id-ctrl.ssvid", "ssvid"),
        ("nvme.id-ctrl.rab", "rab"),
        ("nvme.id-ctrl.ieee", "ieee"),
        ("nvme.id-ctrl.cmic", "cmic"),
        ("nvme.id-ctrl.mdts", "mdts"),
        ("nvme.id-ctrl.version", "version"),
        ("nvme.id-ctrl.controller-type", "controller-type"),
        ("nvme.id-ctrl.oacs", "optional-admin-commands"),
        ("nvme.id-ctrl.fuses", "fused-operations"),
        ("nvme.id-ctrl.fna", "format-nvm-attributes"),
        ("nvme.id-ctrl.awun", "atomic-write-unit-normal"),
        ("nvme.id-ctrl.awupf", "atomic-write-unit-powerfail"),
        ("nvme.id-ctrl.acwu", "atomic-compare-write-unit"),
        ("nvme.id-ctrl.sgls", "sgl-support"),
        ("nvme.id-ctrl.namespace-set-id-max", "namespace-set-id-max"),
        (
            "nvme.id-ctrl.endurance-group-id-max",
            "endurance-group-id-max",
        ),
        ("nvme.id-ctrl.ana-transition-time", "ana-transition-time"),
        ("nvme.id-ctrl.ana-group-max", "ana-group-max"),
        (
            "nvme.id-ctrl.persistent-event-log-size",
            "persistent-event-log-size",
        ),
        ("nvme.id-ctrl.domain-id", "domain-id"),
        (
            "nvme.id-ctrl.warning-composite-temp",
            "warning-composite-temp",
        ),
        (
            "nvme.id-ctrl.critical-composite-temp",
            "critical-composite-temp",
        ),
        (
            "nvme.id-ctrl.minimum-thermal-management-temp",
            "min-thermal-management-temp",
        ),
        (
            "nvme.id-ctrl.maximum-thermal-management-temp",
            "max-thermal-management-temp",
        ),
        ("nvme.id-ctrl.total-nvm-capacity", "total-nvm-capacity"),
        (
            "nvme.id-ctrl.unallocated-nvm-capacity",
            "unallocated-nvm-capacity",
        ),
        ("nvme.id-ctrl.namespace-count", "namespace-count"),
        ("nvme.id-ctrl.oncs", "oncs"),
        ("nvme.id-ctrl.volatile-write-cache", "volatile-write-cache"),
        (
            "nvme.id-ctrl.sanitize-capabilities",
            "sanitize-capabilities",
        ),
        ("nvme.id-ctrl.ana-capabilities", "ana-capabilities"),
        ("nvme.smart.critical-warning", "critical-warning"),
        ("nvme.smart.temperature-kelvin", "temperature-k"),
        (
            "nvme.smart.available-spare-percent",
            "available-spare-percent",
        ),
        (
            "nvme.smart.spare-threshold-percent",
            "spare-threshold-percent",
        ),
        ("nvme.smart.percent-used", "percent-used"),
        ("nvme.smart.data-units-read", "data-units-read"),
        ("nvme.smart.data-units-written", "data-units-written"),
        ("nvme.smart.host-read-commands", "host-read-commands"),
        ("nvme.smart.host-write-commands", "host-write-commands"),
        ("nvme.smart.controller-busy-time", "controller-busy-time"),
        ("nvme.smart.power-cycles", "power-cycles"),
        ("nvme.smart.power-on-hours", "power-on-hours"),
        ("nvme.smart.unsafe-shutdowns", "unsafe-shutdowns"),
        ("nvme.smart.media-errors", "media-errors"),
        ("nvme.smart.error-log-entries", "error-log-entries"),
        ("nvme.smart.warning-temperature-time", "warning-temp-time"),
        (
            "nvme.smart.critical-composite-temperature-time",
            "critical-temp-time",
        ),
        ("nvme.smart.temperature-sensor-1-kelvin", "temp-sensor-1-k"),
        ("nvme.smart.temperature-sensor-2-kelvin", "temp-sensor-2-k"),
        ("nvme.smart.temperature-sensor-3-kelvin", "temp-sensor-3-k"),
        ("nvme.smart.temperature-sensor-4-kelvin", "temp-sensor-4-k"),
        ("nvme.smart.temperature-sensor-5-kelvin", "temp-sensor-5-k"),
        ("nvme.smart.temperature-sensor-6-kelvin", "temp-sensor-6-k"),
        ("nvme.smart.temperature-sensor-7-kelvin", "temp-sensor-7-k"),
        ("nvme.smart.temperature-sensor-8-kelvin", "temp-sensor-8-k"),
        (
            "nvme.smart.thermal-temp1-transition-count",
            "thermal-temp1-transitions",
        ),
        (
            "nvme.smart.thermal-temp2-transition-count",
            "thermal-temp2-transitions",
        ),
        (
            "nvme.smart.thermal-temp1-total-time",
            "thermal-temp1-total-time",
        ),
        (
            "nvme.smart.thermal-temp2-total-time",
            "thermal-temp2-total-time",
        ),
        ("lsblk.type", "lsblk-type"),
        ("lsblk.logical-sector-size", "logical-sector"),
        ("lsblk.physical-sector-size", "physical-sector"),
        ("lsblk.minimum-io-size", "minimum-io"),
        ("lsblk.optimal-io-size", "optimal-io"),
        ("lsblk.discard-alignment", "discard-alignment"),
        ("lsblk.discard-granularity", "discard-granularity"),
        ("lsblk.discard-max", "discard-max"),
        ("lsblk.discard-zeroes-data", "discard-zeroes"),
        ("lsblk.scheduler", "scheduler"),
        ("lsblk.request-queue-size", "rq-size"),
        ("lsblk.write-same-max", "write-same-max"),
        ("lsblk.zoned", "zoned"),
        ("lsblk.zone-size", "zone-size"),
        ("lsblk.zone-write-granularity", "zone-write-granularity"),
        ("lsblk.zone-append-max", "zone-append-max"),
        ("lsblk.zone-count", "zone-count"),
        ("lsblk.zone-open-max", "zone-open-max"),
        ("lsblk.zone-active-max", "zone-active-max"),
        ("lsblk.dax", "dax"),
        ("lsblk.hotplug", "hotplug"),
        ("filesystem.type", "fstype"),
        ("blkid.type", "blkid-type"),
        ("blkid.version", "version"),
        ("blkid.block-size", "blkid-block-size"),
        ("blkid.usage", "usage"),
        ("blkid.uuid-sub", "uuid-sub"),
        ("blkid.partlabel", "partlabel"),
        ("partition.table", "ptable"),
        ("partition.number", "partno"),
        ("partition.start", "start"),
        ("partition.start-bytes", "start-bytes"),
        ("partition.end", "end"),
        ("partition.end-bytes", "end-bytes"),
        ("partition.type", "type"),
        ("partition.name", "part-name"),
        ("partition.flags", "flags"),
        ("partition.disk-flags", "disk-flags"),
        ("parted.transport", "parted-transport"),
        ("parted.logical-sector-size", "logical-sector"),
        ("parted.physical-sector-size", "physical-sector"),
        ("swap.active", "swap-active"),
        ("swap.type", "swap-type"),
        ("swap.priority", "swap-priority"),
        ("zram.algorithm", "zram-algorithm"),
        ("zram.streams", "zram-streams"),
        ("zram.disksize", "zram-disksize"),
        ("zram.data", "zram-data"),
        ("zram.compressed", "zram-compressed"),
        ("zram.total", "zram-total"),
        ("zram.memory-limit", "zram-memory-limit"),
        ("zram.memory-used", "zram-memory-used"),
        ("zram.memory-peak", "zram-memory-peak"),
        ("zram.zero-pages", "zram-zero-pages"),
        ("zram.migrated", "zram-migrated"),
        ("zram.compression-ratio", "zram-ratio"),
        ("zram.mountpoint", "zram-mountpoint"),
        ("zram.swap", "zram-swap"),
        ("loop.backing", "loop-backing"),
        ("loop.back-file", "back-file"),
        ("loop.backing-inode", "back-ino"),
        ("loop.backing-major-minor", "back-major-minor"),
        ("loop.major-minor", "major-minor"),
        ("loop.offset", "offset"),
        ("loop.sizelimit", "sizelimit"),
        ("loop.logical-sector-size", "logical-sector"),
        ("loop.autoclear", "autoclear"),
        ("loop.partscan", "partscan"),
        ("loop.read-only", "ro"),
        ("loop.direct-io", "dio"),
        ("udev.symlink", "udev-link"),
        ("udev.devpath", "udev-devpath"),
        ("udev.devname", "udev-devname"),
        ("udev.devtype", "udev-devtype"),
        ("udev.id-fs-type", "udev-fstype"),
        ("udev.id-fs-version", "udev-fs-version"),
        ("udev.id-fs-usage", "udev-fs-usage"),
        ("udev.id-fs-uuid", "udev-fs-uuid"),
        ("udev.id-fs-uuid-enc", "udev-fs-uuid-enc"),
        ("udev.id-fs-uuid-sub", "udev-fs-uuid-sub"),
        ("udev.id-fs-label", "udev-label"),
        ("udev.id-fs-label-enc", "udev-label-enc"),
        ("udev.id-fs-label-safe", "udev-label-safe"),
        ("udev.id-fs-block-size", "udev-fs-block-size"),
        ("udev.id-fs-lastblock", "udev-fs-lastblock"),
        ("udev.id-bus", "udev-bus"),
        ("udev.id-type", "udev-type"),
        ("udev.id-model", "udev-model"),
        ("udev.id-model-id", "udev-model-id"),
        ("udev.id-vendor", "udev-vendor"),
        ("udev.id-vendor-id", "udev-vendor-id"),
        ("udev.id-revision", "udev-revision"),
        ("udev.id-serial", "udev-serial"),
        ("udev.id-serial-short", "udev-serial-short"),
        ("udev.id-wwn", "udev-wwn"),
        ("udev.id-path", "udev-path"),
        ("udev.id-path-tag", "udev-path-tag"),
        ("udev.id-part-entry-disk", "udev-part-disk"),
        ("udev.id-part-entry-number", "udev-part-number"),
        ("udev.id-part-entry-offset", "udev-part-offset"),
        ("udev.id-part-entry-size", "udev-part-size"),
        ("udev.id-part-entry-scheme", "udev-part-scheme"),
        ("udev.id-part-entry-type", "udev-part-type"),
        ("udev.id-part-entry-name", "udev-part-name"),
        ("udev.id-part-entry-uuid", "udev-part-uuid"),
        ("udev.id-part-entry-flags", "udev-part-flags"),
        ("udev.id-part-table-type", "udev-table-type"),
        ("udev.id-part-table-uuid", "udev-table-uuid"),
        ("udev.dm-name", "dm-name"),
        ("udev.dm-uuid", "dm-uuid"),
        ("udev.dm-vg-name", "dm-vg"),
        ("udev.dm-lv-name", "dm-lv"),
        ("udev.dm-udev-rules-vsn", "dm-rules"),
        ("udev.dm-udev-primary-source-flag", "dm-primary-source"),
        (
            "udev.dm-udev-disable-other-rules-flag",
            "dm-disable-other-rules",
        ),
        ("udev.dm-subsystem-udev-flag0", "dm-subsystem-flag0"),
        ("udev.dm-subsystem-udev-flag1", "dm-subsystem-flag1"),
        ("udev.major", "major"),
        ("udev.minor", "minor"),
        ("udev.subsystem", "subsystem"),
        ("lvm.data-percent", "data"),
        ("lvm.metadata-percent", "metadata"),
        ("lvm.snap-percent", "snap"),
        ("lvm.copy-percent", "copy"),
        ("lvm.sync-percent", "sync"),
        ("lvm.attr", "attr"),
        ("lvm.layout", "layout"),
        ("lvm.segment-type", "segment-type"),
        ("lvm.segment-stripes", "stripes"),
        ("lvm.segment-data-stripes", "data-stripes"),
        ("lvm.reshape-length", "reshape-length"),
        ("lvm.reshape-length-extents", "reshape-extents"),
        ("lvm.data-copies", "data-copies"),
        ("lvm.data-offset", "data-offset"),
        ("lvm.new-data-offset", "new-data-offset"),
        ("lvm.parity-chunks", "parity-chunks"),
        ("lvm.stripe-size", "stripe-size"),
        ("lvm.region-size", "region-size"),
        ("lvm.segment-start", "segment-start"),
        ("lvm.segment-start-extent", "segment-start-pe"),
        ("lvm.segment-size", "segment-size"),
        ("lvm.segment-size-extents", "segment-size-pe"),
        ("lvm.segment-tags", "segment-tags"),
        ("lvm.chunk-size", "chunk-size"),
        ("lvm.thin-count", "thin-count"),
        ("lvm.discards", "discards"),
        ("lvm.zero", "zero"),
        ("lvm.transaction-id", "transaction-id"),
        ("lvm.thin-id", "thin-id"),
        ("lvm.devices", "devices"),
        ("lvm.metadata-devices", "metadata-devices"),
        ("lvm.segment-pe-ranges", "pe-ranges"),
        ("lvm.segment-le-ranges", "le-ranges"),
        ("lvm.segment-metadata-le-ranges", "metadata-le-ranges"),
        ("lvm.segment-monitor", "segment-monitor"),
        ("lvm.cache-metadata-format", "cache-metadata-format"),
        ("lvm.segment-cache-mode", "segment-cache-mode"),
        ("lvm.segment-cache-policy", "segment-cache-policy"),
        ("lvm.cache-settings", "cache-settings"),
        ("lvm.integrity-settings", "integrity-settings"),
        ("lvm.vdo-compression", "vdo-compression"),
        ("lvm.vdo-deduplication", "vdo-deduplication"),
        ("lvm.vdo-minimum-io-size", "vdo-min-io"),
        ("lvm.vdo-block-map-cache-size", "vdo-block-map-cache"),
        ("lvm.vdo-block-map-era-length", "vdo-block-map-era"),
        ("lvm.vdo-use-sparse-index", "vdo-sparse-index"),
        ("lvm.vdo-index-memory-size", "vdo-index-memory"),
        ("lvm.vdo-slab-size", "vdo-slab"),
        ("lvm.vdo-ack-threads", "vdo-ack-threads"),
        ("lvm.vdo-bio-threads", "vdo-bio-threads"),
        ("lvm.vdo-bio-rotation", "vdo-bio-rotation"),
        ("lvm.vdo-cpu-threads", "vdo-cpu-threads"),
        ("lvm.vdo-hash-zone-threads", "vdo-hash-zone-threads"),
        ("lvm.vdo-logical-threads", "vdo-logical-threads"),
        ("lvm.vdo-physical-threads", "vdo-physical-threads"),
        ("lvm.vdo-max-discard", "vdo-max-discard"),
        ("lvm.vdo-header-size", "vdo-header"),
        ("lvm.vdo-use-metadata-hints", "vdo-metadata-hints"),
        ("lvm.vdo-write-policy", "vdo-write-policy"),
        ("lvm.vdo-operating-mode", "vdo-mode"),
        ("lvm.vdo-compression-state", "vdo-compression-state"),
        ("lvm.vdo-index-state", "vdo-index-state"),
        ("lvm.vdo-used-size", "vdo-used"),
        ("lvm.vdo-saving-percent", "vdo-saving"),
        ("lvm.origin", "origin"),
        ("lvm.pool", "pool"),
        ("lvm.pv-format", "pv-format"),
        ("lvm.dev-size", "dev-size"),
        ("lvm.pv-major", "pv-major"),
        ("lvm.pv-minor", "pv-minor"),
        ("lvm.pe-start", "pe-start"),
        ("lvm.pv-attr", "pv-attr"),
        ("lvm.pv-allocatable", "pv-allocatable"),
        ("lvm.pv-exported", "pv-exported"),
        ("lvm.pv-missing", "pv-missing"),
        ("lvm.pv-pe-count", "pv-extents"),
        ("lvm.pv-pe-allocated", "pv-extents-used"),
        ("lvm.pv-tags", "pv-tags"),
        ("lvm.pv-mda-count", "pv-mda-count"),
        ("lvm.pv-mda-used-count", "pv-mda-used"),
        ("lvm.pv-mda-free", "pv-mda-free"),
        ("lvm.pv-mda-size", "pv-mda-size"),
        ("lvm.pv-bootloader-area-start", "pv-ba-start"),
        ("lvm.pv-bootloader-area-size", "pv-ba-size"),
        ("lvm.pv-in-use", "pv-in-use"),
        ("lvm.pv-duplicate", "pv-duplicate"),
        ("lvm.pv-device-id", "pv-device-id"),
        ("lvm.pv-device-id-type", "pv-device-id-type"),
        ("lvm.vg-format", "vg-format"),
        ("lvm.vg-attr", "vg-attr"),
        ("lvm.vg-extendable", "vg-extendable"),
        ("lvm.vg-exported", "vg-exported"),
        ("lvm.vg-autoactivation", "vg-autoactivation"),
        ("lvm.vg-partial", "vg-partial"),
        ("lvm.allocation-policy", "allocation"),
        ("lvm.vg-clustered", "vg-clustered"),
        ("lvm.vg-shared", "vg-shared"),
        ("lvm.vg-system-id", "system-id"),
        ("lvm.vg-lock-type", "lock-type"),
        ("lvm.vg-lock-args", "lock-args"),
        ("lvm.extent-size", "extent"),
        ("lvm.extent-count", "extents"),
        ("lvm.free-count", "free-extents"),
        ("lvm.max-lvs", "max-lvs"),
        ("lvm.max-pvs", "max-pvs"),
        ("lvm.pv-count", "pvs"),
        ("lvm.missing-pv-count", "missing-pvs"),
        ("lvm.lv-count", "lvs"),
        ("lvm.snapshot-count", "snapshots"),
        ("lvm.vg-seqno", "seqno"),
        ("lvm.vg-profile", "vg-profile"),
        ("lvm.vg-mda-count", "vg-mda-count"),
        ("lvm.vg-mda-used-count", "vg-mda-used"),
        ("lvm.vg-mda-free", "vg-mda-free"),
        ("lvm.vg-mda-size", "vg-mda-size"),
        ("lvm.vg-mda-copies", "vg-mda-copies"),
        ("lvm.active", "active"),
        ("lvm.active-locally", "active-local"),
        ("lvm.active-remotely", "active-remote"),
        ("lvm.active-exclusively", "active-exclusive"),
        ("lvm.permissions", "permissions"),
        ("lvm.health", "health"),
        ("lvm.when-full", "when-full"),
        ("lvm.metadata-size", "metadata-size"),
        ("lvm.tags", "tags"),
        ("lvm.dm-path", "dm-path"),
        ("lvm.parent", "parent"),
        ("lvm.read-ahead", "read-ahead"),
        ("lvm.kernel-read-ahead", "kernel-read-ahead"),
        ("lvm.suspended", "suspended"),
        ("lvm.live-table", "live-table"),
        ("lvm.inactive-table", "inactive-table"),
        ("lvm.modules", "modules"),
        ("lvm.host", "host"),
        ("lvm.historical", "historical"),
        ("lvm.kernel-major", "kernel-major"),
        ("lvm.kernel-minor", "kernel-minor"),
        ("lvm.device-open", "device-open"),
        ("lvm.check-needed", "check-needed"),
        ("lvm.role", "role"),
        ("lvm.time", "time"),
        ("lvm.raid-mismatch-count", "raid-mismatches"),
        ("lvm.raid-sync-action", "raid-sync"),
        ("lvm.raid-write-behind", "raid-write-behind"),
        ("lvm.raid-min-recovery-rate", "raid-min-recovery"),
        ("lvm.raid-max-recovery-rate", "raid-max-recovery"),
        ("lvm.raid-integrity-mode", "raid-integrity"),
        ("lvm.raid-integrity-block-size", "raid-integrity-block"),
        ("lvm.raid-integrity-mismatches", "raid-integrity-mismatches"),
        ("lvm.cache-total-blocks", "cache-total"),
        ("lvm.cache-used-blocks", "cache-used"),
        ("lvm.cache-dirty-blocks", "cache-dirty"),
        ("lvm.cache-read-hits", "cache-read-hits"),
        ("lvm.cache-read-misses", "cache-read-misses"),
        ("lvm.cache-write-hits", "cache-write-hits"),
        ("lvm.cache-write-misses", "cache-write-misses"),
        ("lvm.cache-promotions", "cache-promotions"),
        ("lvm.cache-demotions", "cache-demotions"),
        ("lvm.cache-mode", "cache-mode"),
        ("lvm.cache-policy", "cache-policy"),
        ("lvm.kernel-cache-settings", "kernel-cache-settings"),
        ("lvm.kernel-cache-mode", "kernel-cache-mode"),
        ("lvm.kernel-cache-policy", "kernel-cache-policy"),
        ("lvm.kernel-metadata-format", "kernel-metadata-format"),
        ("lvm.kernel-discards", "kernel-discards"),
        ("lvm.writecache-total-blocks", "writecache-total"),
        ("lvm.writecache-free-blocks", "writecache-free"),
        ("lvm.writecache-writeback-blocks", "writecache-writeback"),
        ("lvm.writecache-block-size", "writecache-block-size"),
        ("lvm.writecache-error", "writecache-error"),
        ("btrfs.qgroup-id", "qgroup"),
        ("btrfs.qgroup-parents", "qgroup-parents"),
        ("btrfs.qgroup-children", "qgroup-children"),
        ("btrfs.mount-target", "mount-target"),
        ("btrfs.device-id", "device-id"),
        ("btrfs.device-stat-write-io-errs", "write-io-errs"),
        ("btrfs.device-stat-read-io-errs", "read-io-errs"),
        ("btrfs.device-stat-flush-io-errs", "flush-io-errs"),
        ("btrfs.device-stat-corruption-errs", "corruption-errs"),
        ("btrfs.device-stat-generation-errs", "generation-errs"),
        ("btrfs.id", "subvol-id"),
        ("btrfs.generation", "generation"),
        ("btrfs.created-generation", "created-generation"),
        ("btrfs.parent-id", "parent-id"),
        ("btrfs.top-level", "top-level"),
        ("btrfs.parent-uuid", "parent-uuid"),
        ("btrfs.received-uuid", "received-uuid"),
        ("btrfs.data-profile", "data-profile"),
        ("btrfs.data-size", "data-size"),
        ("btrfs.data-used", "data-used"),
        ("btrfs.metadata-profile", "metadata-profile"),
        ("btrfs.metadata-size", "metadata-size"),
        ("btrfs.metadata-used", "metadata-used"),
        ("btrfs.system-profile", "system-profile"),
        ("btrfs.system-size", "system-size"),
        ("btrfs.system-used", "system-used"),
        ("btrfs.max-referenced", "max-rfer"),
        ("btrfs.max-exclusive", "max-excl"),
        ("vdo.storage-device", "backing"),
        ("vdo.logical-size", "logical"),
        ("vdo.physical-size", "physical"),
        ("vdo.stats-size", "stats-size"),
        ("vdo.stats-used", "stats-used"),
        ("vdo.stats-available", "stats-free"),
        ("vdo.use-percent", "vdo-use"),
        ("vdo.space-saving-percent", "saving"),
        ("vdo.operating-mode", "mode"),
        ("vdo.recovery-percentage", "recovery"),
        ("vdo.write-policy", "write-policy"),
        ("vdo.configured-write-policy", "configured-write-policy"),
        ("vdo.index-memory-setting", "index-memory"),
        ("vdo.block-map-cache-size", "block-map-cache"),
        ("vdo.compression", "compression"),
        ("vdo.deduplication", "deduplication"),
        ("vdo.version", "vdo-version"),
        ("vdo.release-version", "vdo-release"),
        ("vdo.data-blocks-used", "data-blocks"),
        ("vdo.data-blocks-used-bytes", "data-bytes"),
        ("vdo.overhead-blocks-used", "overhead-blocks"),
        ("vdo.overhead-blocks-used-bytes", "overhead-bytes"),
        ("vdo.logical-blocks-used", "logical-blocks"),
        ("vdo.logical-blocks-used-bytes", "logical-bytes"),
        ("dm.name", "dm-name"),
        ("dm.uuid", "dm-uuid"),
        ("dm.major", "dm-major"),
        ("dm.minor", "dm-minor"),
        ("dm.open-count", "open"),
        ("dm.segments", "segments"),
        ("dm.events", "events"),
        ("dm.table.targets", "dm-table-targets"),
        ("dm.table.segment-count", "dm-table-segments"),
        ("dm.table.segment.0.start", "dm-table-start"),
        ("dm.table.segment.0.length", "dm-table-length"),
        ("dm.table.segment.0.target", "dm-table-target"),
        ("dm.table.segment.0.payload", "dm-table-payload"),
        ("dm.table.segment.0.device", "dm-table-device"),
        ("dm.table.segment.0.offset", "dm-table-offset"),
        (
            "dm.table.segment.0.metadata-device",
            "dm-table-metadata-device",
        ),
        ("dm.table.segment.0.data-device", "dm-table-data-device"),
        (
            "dm.table.segment.0.data-block-size",
            "dm-table-data-block-size",
        ),
        (
            "dm.table.segment.0.low-water-mark",
            "dm-table-low-water-mark",
        ),
        ("dm.table.segment.0.pool-device", "dm-table-pool-device"),
        ("dm.table.segment.0.thin-device-id", "dm-table-thin-id"),
        (
            "dm.table.segment.0.external-origin-device",
            "dm-table-external-origin",
        ),
        ("dm.table.segment.0.cache-device", "dm-table-cache-device"),
        ("dm.table.segment.0.origin-device", "dm-table-origin-device"),
        ("dm.table.segment.0.block-size", "dm-table-block-size"),
        ("dm.table.segment.0.cow-device", "dm-table-cow-device"),
        ("dm.table.segment.0.persistence", "dm-table-persistence"),
        ("dm.table.segment.0.chunk-size", "dm-table-chunk-size"),
        ("dm.table.segment.0.stripe-count", "dm-table-stripes"),
        (
            "dm.table.segment.0.stripe.0.device",
            "dm-table-stripe0-device",
        ),
        (
            "dm.table.segment.0.stripe.0.offset",
            "dm-table-stripe0-offset",
        ),
        (
            "dm.table.segment.0.stripe.1.device",
            "dm-table-stripe1-device",
        ),
        (
            "dm.table.segment.0.stripe.1.offset",
            "dm-table-stripe1-offset",
        ),
        ("dm.table.segment.0.crypt.cipher", "dm-crypt-cipher"),
        ("dm.table.segment.0.crypt.iv-offset", "dm-crypt-iv-offset"),
        ("dm.table.segment.0.crypt.device", "dm-crypt-device"),
        ("dm.table.segment.0.crypt.offset", "dm-crypt-offset"),
        ("dm.status.targets", "dm-status-targets"),
        ("dm.status.segment-count", "dm-status-segments"),
        ("dm.status.segment.0.start", "dm-status-start"),
        ("dm.status.segment.0.length", "dm-status-length"),
        ("dm.status.segment.0.target", "dm-status-target"),
        ("dm.status.segment.0.payload", "dm-status-payload"),
        (
            "dm.status.segment.0.metadata-used-blocks",
            "dm-status-metadata-used",
        ),
        (
            "dm.status.segment.0.metadata-total-blocks",
            "dm-status-metadata-total",
        ),
        (
            "dm.status.segment.0.cache-used-blocks",
            "dm-status-cache-used",
        ),
        (
            "dm.status.segment.0.cache-total-blocks",
            "dm-status-cache-total",
        ),
        (
            "dm.status.segment.0.data-used-blocks",
            "dm-status-data-used",
        ),
        (
            "dm.status.segment.0.data-total-blocks",
            "dm-status-data-total",
        ),
        ("dm.status.segment.0.read-hits", "dm-status-read-hits"),
        ("dm.status.segment.0.read-misses", "dm-status-read-misses"),
        ("dm.status.segment.0.write-hits", "dm-status-write-hits"),
        ("dm.status.segment.0.write-misses", "dm-status-write-misses"),
        ("dm.status.segment.0.dirty-blocks", "dm-status-dirty"),
        ("dm.status.segment.0.mode", "dm-status-mode"),
        ("dm.status.segment.0.used-sectors", "dm-status-used-sectors"),
        (
            "dm.status.segment.0.total-sectors",
            "dm-status-total-sectors",
        ),
        ("cryptsetup.active", "active"),
        ("cryptsetup.in-use", "in-use"),
        ("cryptsetup.cipher", "cipher"),
        ("cryptsetup.mode", "mode"),
        ("cryptsetup.sector-size", "sector-size"),
        ("cryptsetup.sector-count", "sectors"),
        ("cryptsetup.luks-version", "luks"),
        ("cryptsetup.luks-epoch", "epoch"),
        ("cryptsetup.luks-metadata-area", "metadata-area"),
        ("cryptsetup.luks-keyslots-area", "keyslots-area"),
        ("cryptsetup.luks-subsystem", "subsystem"),
        ("cryptsetup.luks-flags", "flags"),
        ("cryptsetup.luks-keyslot-count", "keyslots"),
        ("cryptsetup.luks-token-count", "tokens"),
        ("cryptsetup.luks-keyslots", "keyslot-ids"),
        ("cryptsetup.luks-tokens", "token-ids"),
        ("cryptsetup.luks-keyslot-0-type", "keyslot-0"),
        ("cryptsetup.luks-keyslot-0-priority", "keyslot-0-priority"),
        ("cryptsetup.luks-keyslot-0-cipher", "keyslot-0-cipher"),
        (
            "cryptsetup.luks-keyslot-0-cipher-key",
            "keyslot-0-cipher-key",
        ),
        ("cryptsetup.luks-keyslot-0-pbkdf", "keyslot-0-pbkdf"),
        ("cryptsetup.luks-keyslot-0-time-cost", "keyslot-0-time"),
        ("cryptsetup.luks-keyslot-0-memory", "keyslot-0-memory"),
        ("cryptsetup.luks-keyslot-0-threads", "keyslot-0-threads"),
        ("cryptsetup.luks-keyslot-0-salt", "keyslot-0-salt"),
        (
            "cryptsetup.luks-keyslot-0-af-stripes",
            "keyslot-0-af-stripes",
        ),
        (
            "cryptsetup.luks-keyslot-0-area-offset",
            "keyslot-0-area-offset",
        ),
        (
            "cryptsetup.luks-keyslot-0-area-length",
            "keyslot-0-area-length",
        ),
        ("cryptsetup.luks-keyslot-0-digest-id", "keyslot-0-digest"),
        ("cryptsetup.luks-keyslot-1-type", "keyslot-1"),
        ("cryptsetup.luks-keyslot-1-priority", "keyslot-1-priority"),
        ("cryptsetup.luks-keyslot-1-cipher", "keyslot-1-cipher"),
        (
            "cryptsetup.luks-keyslot-1-cipher-key",
            "keyslot-1-cipher-key",
        ),
        ("cryptsetup.luks-keyslot-1-pbkdf", "keyslot-1-pbkdf"),
        ("cryptsetup.luks-keyslot-1-time-cost", "keyslot-1-time"),
        ("cryptsetup.luks-keyslot-1-memory", "keyslot-1-memory"),
        ("cryptsetup.luks-keyslot-1-threads", "keyslot-1-threads"),
        ("cryptsetup.luks-keyslot-1-salt", "keyslot-1-salt"),
        (
            "cryptsetup.luks-keyslot-1-af-stripes",
            "keyslot-1-af-stripes",
        ),
        (
            "cryptsetup.luks-keyslot-1-area-offset",
            "keyslot-1-area-offset",
        ),
        (
            "cryptsetup.luks-keyslot-1-area-length",
            "keyslot-1-area-length",
        ),
        ("cryptsetup.luks-keyslot-1-digest-id", "keyslot-1-digest"),
        ("cryptsetup.luks-token-0-type", "token-0"),
        ("cryptsetup.luks-token-0-keyslot", "token-0-keyslot"),
        ("cryptsetup.luks-token-0-keyslots", "token-0-keyslots"),
        ("cryptsetup.luks-token-0-tpm2-pcrs", "token-0-tpm2-pcrs"),
        ("cryptsetup.luks-token-0-tpm2-hash", "token-0-tpm2-hash"),
        ("cryptsetup.luks-token-1-type", "token-1"),
        ("cryptsetup.luks-token-1-keyslot", "token-1-keyslot"),
        ("cryptsetup.luks-token-1-keyslots", "token-1-keyslots"),
        ("cryptsetup.luks-token-1-tpm2-pcrs", "token-1-tpm2-pcrs"),
        ("cryptsetup.luks-token-1-tpm2-hash", "token-1-tpm2-hash"),
        ("cryptsetup.luks-digest-count", "digests"),
        ("cryptsetup.luks-digests", "digest-ids"),
        ("cryptsetup.luks-digest-0-type", "digest-0"),
        ("cryptsetup.luks-digest-0-hash", "digest-0-hash"),
        ("cryptsetup.luks-digest-0-iterations", "digest-0-iterations"),
        ("cryptsetup.luks-digest-0-salt", "digest-0-salt"),
        ("cryptsetup.luks-digest-0-digest", "digest-0-digest"),
        ("cryptsetup.luks-digest-1-type", "digest-1"),
        ("cryptsetup.luks-digest-1-hash", "digest-1-hash"),
        ("cryptsetup.luks-digest-1-iterations", "digest-1-iterations"),
        ("cryptsetup.luks-digest-1-salt", "digest-1-salt"),
        ("cryptsetup.luks-digest-1-digest", "digest-1-digest"),
        ("cryptsetup.luks-data-cipher", "data-cipher"),
        ("cryptsetup.luks-data-offset", "data-offset"),
        ("cryptsetup.luks-data-length", "data-length"),
        ("cryptsetup.luks-data-sector", "data-sector"),
        ("multipath.dm", "dm"),
        ("multipath.wwid", "wwid"),
        ("multipath.vendor-product", "vendor"),
        ("multipath.size", "size"),
        ("multipath.features", "features"),
        ("multipath.hwhandler", "handler"),
        ("multipath.write-protect", "wp"),
        ("multipath.host-path", "host-path"),
        ("multipath.scsi-host", "scsi-host"),
        ("multipath.scsi-channel", "scsi-channel"),
        ("multipath.scsi-id", "scsi-id"),
        ("multipath.scsi-lun", "scsi-lun"),
        ("major-minor", "major-minor"),
        ("multipath.group-policy", "group-policy"),
        ("multipath.group-prio", "group-prio"),
        ("multipath.group-status", "group-status"),
        ("multipath.dm-state", "dm-state"),
        ("multipath.checker-state", "checker-state"),
        ("multipath.online-state", "online-state"),
        ("multipath.path-flags", "path-flags"),
        ("multipath.path-state", "path-state"),
        ("md.version", "md-version"),
        ("md.level", "level"),
        ("md.state", "state"),
        ("md.raid-devices", "raid-devices"),
        ("md.total-devices", "total-devices"),
        ("md.array-devices", "array-devices"),
        ("md.active-devices", "active-devices"),
        ("md.working-devices", "working-devices"),
        ("md.failed-devices", "failed-devices"),
        ("md.spare-devices", "spare-devices"),
        ("md.degraded-devices", "degraded-devices"),
        ("md.name", "md-name"),
        ("md.events", "events"),
        ("md.chunk-size", "chunk"),
        ("md.layout", "layout"),
        ("md.consistency-policy", "consistency"),
        ("md.rebuild-status", "rebuild"),
        ("md.resync-status", "resync"),
        ("md.check-status", "check"),
        ("md.intent-bitmap", "bitmap"),
        ("md.persistence", "persistence"),
        ("md.bitmap", "bitmap-detail"),
        ("md.mdstat-state", "mdstat-state"),
        ("md.mdstat-level", "mdstat-level"),
        ("md.mdstat-blocks", "mdstat-blocks"),
        ("md.mdstat-superblock", "mdstat-superblock"),
        ("md.mdstat-layout", "mdstat-layout"),
        ("md.mdstat-chunk-size", "mdstat-chunk"),
        ("md.mdstat-devices", "mdstat-devices"),
        ("md.mdstat-health", "mdstat-health"),
        ("md.mdstat-progress", "mdstat-progress"),
        ("md.mdstat-progress-percent", "mdstat-progress-percent"),
        ("md.mdstat-progress-blocks", "mdstat-progress-blocks"),
        ("md.mdstat-finish", "mdstat-finish"),
        ("md.mdstat-speed", "mdstat-speed"),
        ("md.mdstat-bitmap", "mdstat-bitmap"),
        ("md.creation-time", "created"),
        ("md.update-time", "updated"),
        ("md.scan-metadata", "scan-metadata"),
        ("md.scan-name", "scan-name"),
        ("md.scan-spares", "scan-spares"),
        ("md.scan-devices", "scan-devices"),
        ("md.member-number", "member-number"),
        ("md.member-major", "member-major"),
        ("md.member-minor", "member-minor"),
        ("md.member-raid-device", "member-raid-device"),
        ("md.member-state", "member-state"),
        ("md.mdstat-member-slot", "mdstat-member-slot"),
        ("md.mdstat-member-flags", "mdstat-member-flags"),
        ("md.uuid", "md-uuid"),
        ("iscsi.target", "target"),
        ("iscsi.portal", "portal"),
        ("iscsi.portal-address", "portal-address"),
        ("iscsi.portal-port", "portal-port"),
        ("iscsi.portal-tpgt", "portal-tpgt"),
        ("iscsi.persistent-portal", "persistent-portal"),
        (
            "iscsi.persistent-portal-address",
            "persistent-portal-address",
        ),
        ("iscsi.persistent-portal-port", "persistent-portal-port"),
        ("iscsi.persistent-portal-tpgt", "persistent-portal-tpgt"),
        ("iscsi.target-portal-group-tag", "tpgt"),
        ("iscsi.connection-state", "connection-state"),
        ("iscsi.session-state", "session-state"),
        ("iscsi.internal-session-state", "internal-session-state"),
        ("iscsi.iface-name", "iface"),
        ("iscsi.iface-transport", "transport"),
        ("iscsi.iface-initiator-name", "initiator"),
        ("iscsi.iface-ip-address", "iface-ip"),
        ("iscsi.iface-netdev", "netdev"),
        ("iscsi.host-number", "host"),
        ("iscsi.host-state", "host-state"),
        ("iscsi.connection-cid", "cid"),
        ("iscsi.connection-detail-state", "connection-detail-state"),
        ("iscsi.connection-local-address", "local-address"),
        ("iscsi.connection-peer-address", "peer-address"),
        ("iscsi.headerdigest", "header-digest"),
        ("iscsi.datadigest", "data-digest"),
        ("iscsi.maxrecvdatasegmentlength", "max-recv-data-segment"),
        ("iscsi.maxxmitdatasegmentlength", "max-xmit-data-segment"),
        ("iscsi.firstburstlength", "first-burst"),
        ("iscsi.maxburstlength", "max-burst"),
        ("iscsi.immediatedata", "immediate-data"),
        ("iscsi.initialr2t", "initial-r2t"),
        ("iscsi.maxoutstandingr2t", "max-outstanding-r2t"),
        ("iscsi.scsi-channel", "scsi-channel"),
        ("iscsi.scsi-id", "scsi-id"),
        ("iscsi.attached-disk", "attached-disk"),
        ("iscsi.attached-disk-state", "attached-disk-state"),
        ("iscsi.node-configured", "configured"),
        ("iscsi.node-portal", "node-portal"),
        ("iscsi.node-portal-address", "node-portal-address"),
        ("iscsi.node-portal-port", "node-portal-port"),
        ("iscsi.node-portal-tpgt", "node-portal-tpgt"),
        ("iscsi.node-persistent-portal", "node-persistent-portal"),
        (
            "iscsi.node-persistent-portal-address",
            "node-persistent-portal-address",
        ),
        (
            "iscsi.node-persistent-portal-port",
            "node-persistent-portal-port",
        ),
        (
            "iscsi.node-persistent-portal-tpgt",
            "node-persistent-portal-tpgt",
        ),
        ("iscsi.node-tpgt", "node-tpgt"),
        ("iscsi.node-iface-name", "node-iface"),
        ("iscsi.node-startup", "startup"),
        ("iscsi.node-leading-login", "leading-login"),
        ("iscsi.node-auth-method", "auth-method"),
        ("iscsi.node-auth-username", "auth-username"),
        ("nfs.source", "source"),
        ("nfs.server", "server"),
        ("nfs.export", "export"),
        ("nfs.export-client", "export-client"),
        ("nfs.exportfs", "exportfs"),
        ("nfs.export-option-rw", "export-rw"),
        ("nfs.export-option-ro", "export-ro"),
        ("nfs.export-option-sync", "export-sync"),
        ("nfs.export-option-async", "export-async"),
        ("nfs.export-option-wdelay", "export-wdelay"),
        ("nfs.export-option-no-wdelay", "export-no-wdelay"),
        ("nfs.export-option-hide", "export-hide"),
        ("nfs.export-option-nohide", "export-nohide"),
        (
            "nfs.export-option-no-subtree-check",
            "export-no-subtree-check",
        ),
        ("nfs.export-option-subtree-check", "export-subtree-check"),
        ("nfs.export-option-sec", "export-sec"),
        ("nfs.export-option-secure", "export-secure"),
        ("nfs.export-option-insecure", "export-insecure"),
        ("nfs.export-option-root-squash", "export-root-squash"),
        ("nfs.export-option-no-root-squash", "export-no-root-squash"),
        ("nfs.export-option-all-squash", "export-all-squash"),
        ("nfs.export-option-no-all-squash", "export-no-all-squash"),
        ("nfs.export-option-fsid", "export-fsid"),
        ("nfs.vers", "vers"),
        ("nfs.proto", "proto"),
        ("nfs.sec", "sec"),
        ("nfs.clientaddr", "clientaddr"),
        ("nfs.addr", "addr"),
        ("nfs.port", "port"),
        ("nfs.mountaddr", "mountaddr"),
        ("nfs.mountvers", "mountvers"),
        ("nfs.mountproto", "mountproto"),
        ("nfs.rsize", "rsize"),
        ("nfs.wsize", "wsize"),
        ("nfs.timeo", "timeo"),
        ("nfs.retrans", "retrans"),
        ("nfs.local-lock", "local-lock"),
        ("nfs.lookupcache", "lookupcache"),
        ("nfs.fsc", "fsc"),
        ("nfs.age", "age"),
        ("nfs.namlen", "namlen"),
        ("nfs.caps", "caps"),
        ("nfs.wtmult", "wtmult"),
        ("nfs.dtsize", "dtsize"),
        ("nfs.bsize", "bsize"),
        ("nfs.flavor", "flavor"),
        ("nfs.pseudoflavor", "pseudoflavor"),
        ("nfs.hard", "hard"),
        ("nfs.soft", "soft"),
        ("nfs.noresvport", "noresvport"),
        ("ext.state", "ext-state"),
        ("ext.magic-number", "ext-magic"),
        ("ext.revision", "ext-revision"),
        ("ext.errors-behavior", "errors"),
        ("ext.fs-error-count", "fs-error-count"),
        ("ext.os-type", "os"),
        ("ext.block-count", "blocks"),
        ("ext.reserved-block-count", "reserved-blocks"),
        ("ext.overhead-clusters", "overhead-clusters"),
        ("ext.free-blocks", "free-blocks"),
        ("ext.first-block", "first-block"),
        ("ext.block-size", "block-size"),
        ("ext.fragment-size", "fragment-size"),
        ("ext.blocks-per-group", "blocks-per-group"),
        ("ext.fragments-per-group", "fragments-per-group"),
        ("ext.inode-count", "inodes"),
        ("ext.free-inodes", "free-inodes"),
        ("ext.inodes-per-group", "inodes-per-group"),
        ("ext.raid-stride", "raid-stride"),
        ("ext.raid-stripe-width", "raid-stripe-width"),
        ("ext.features", "features"),
        ("ext.flags", "flags"),
        ("ext.default-directory-hash", "dir-hash"),
        ("ext.directory-hash-seed", "dir-hash-seed"),
        ("ext.default-mount-options", "default-mount"),
        ("ext.created", "created"),
        ("ext.last-mount-time", "last-mounted"),
        ("ext.last-write-time", "last-written"),
        ("ext.mount-count", "mount-count"),
        ("ext.maximum-mount-count", "max-mount-count"),
        ("ext.last-checked", "last-checked"),
        ("ext.check-interval", "check-interval"),
        ("ext.lifetime-writes", "lifetime-writes"),
        ("ext.reserved-blocks-uid", "reserved-uid"),
        ("ext.reserved-blocks-gid", "reserved-gid"),
        ("ext.first-inode", "first-inode"),
        ("ext.inode-size", "inode-size"),
        ("ext.journal-inode", "journal-inode"),
        ("ext.journal-uuid", "journal-uuid"),
        ("ext.journal-backup", "journal-backup"),
        ("ext.journal-features", "journal-features"),
        ("ext.journal-size", "journal-size"),
        ("ext.first-error-time", "first-error-time"),
        ("ext.first-error-function", "first-error-function"),
        ("ext.first-error-line", "first-error-line"),
        ("ext.first-error-inode", "first-error-inode"),
        ("ext.first-error-block", "first-error-block"),
        ("ext.last-error-time", "last-error-time"),
        ("ext.last-error-function", "last-error-function"),
        ("ext.last-error-line", "last-error-line"),
        ("ext.last-error-inode", "last-error-inode"),
        ("ext.last-error-block", "last-error-block"),
        ("ext.checksum-type", "checksum-type"),
        ("ext.checksum", "checksum"),
        ("exfat.guid", "guid"),
        ("exfat.volume-label", "exfat-label"),
        ("exfat.exfatprogs-version", "exfatprogs"),
        ("exfat.volume-serial", "serial"),
        ("exfat.volume-length-sectors", "sectors"),
        ("exfat.fat-offset-sector-offset", "fat-offset"),
        ("exfat.fat-length-sectors", "fat-length"),
        (
            "exfat.cluster-heap-offset-sector-offset",
            "cluster-heap-offset",
        ),
        ("exfat.cluster-count", "clusters"),
        ("exfat.used-clusters", "used-clusters"),
        ("exfat.free-clusters", "free-clusters"),
        ("exfat.root-cluster-cluster-offset", "root-cluster"),
        ("exfat.bytes-per-sector", "sector-bytes"),
        ("exfat.sectors-per-cluster", "sectors-per-cluster"),
        ("exfat.bytes-per-cluster", "cluster-bytes"),
        ("exfat.sector-size-bits", "sector-bits"),
        ("exfat.sector-per-cluster-bits", "cluster-bits"),
        ("ntfs.device-name", "ntfs-device"),
        ("ntfs.device-state", "ntfs-device-state"),
        ("ntfs.volume-name", "ntfs-name"),
        ("ntfs.volume-state", "ntfs-state"),
        ("ntfs.volume-serial", "ntfs-serial"),
        ("ntfs.volume-flags", "ntfs-flags"),
        ("ntfs.version", "ntfs-version"),
        ("ntfs.sector-size", "ntfs-sector"),
        ("ntfs.cluster-size", "ntfs-cluster"),
        ("ntfs.volume-size-clusters", "ntfs-clusters"),
        ("ntfs.index-block-size", "ntfs-index-block"),
        ("ntfs.mft-record-size", "ntfs-mft-record"),
        ("ntfs.mft-zone-multiplier", "ntfs-mft-zone-multiplier"),
        ("ntfs.mft-zone-start", "ntfs-mft-zone-start"),
        ("ntfs.mft-zone-end", "ntfs-mft-zone-end"),
        ("ntfs.mft-data-position", "ntfs-mft-data-position"),
        ("ntfs.mft-lcn", "ntfs-mft-lcn"),
        ("f2fs.filesystem-uuid", "f2fs-uuid"),
        ("f2fs.filesystem-volume-name", "f2fs-name"),
        ("f2fs.block-size", "f2fs-block-size"),
        ("f2fs.block-count", "f2fs-blocks"),
        ("f2fs.user-block-count", "f2fs-user-blocks"),
        ("f2fs.valid-block-count", "f2fs-valid-blocks"),
        ("f2fs.total-valid-block-count", "f2fs-total-valid-blocks"),
        ("f2fs.valid-node-count", "f2fs-valid-nodes"),
        ("f2fs.valid-inode-count", "f2fs-valid-inodes"),
        ("f2fs.segment-count", "f2fs-segments"),
        ("f2fs.segment-count-main", "f2fs-main-segments"),
        ("f2fs.segment-count-ckpt", "f2fs-ckpt-segments"),
        ("f2fs.segment-count-sit", "f2fs-sit-segments"),
        ("f2fs.segment-count-nat", "f2fs-nat-segments"),
        ("f2fs.segment-count-ssa", "f2fs-ssa-segments"),
        ("f2fs.overprov-segment-count", "f2fs-overprov"),
        ("f2fs.section-count", "f2fs-sections"),
        ("f2fs.segs-per-sec", "f2fs-segs-per-sec"),
        ("f2fs.secs-per-zone", "f2fs-secs-per-zone"),
        ("f2fs.log-sectorsize", "f2fs-log-sector"),
        ("f2fs.log-sectors-per-block", "f2fs-log-sectors-block"),
        ("f2fs.log-blocksize", "f2fs-log-block"),
        ("f2fs.log-blocks-per-seg", "f2fs-log-blocks-seg"),
        ("f2fs.cp-payload", "f2fs-cp-payload"),
        ("f2fs.version", "f2fs-version"),
        ("f2fs.init-version", "f2fs-init-version"),
        ("f2fs.extension-count", "f2fs-extensions"),
        ("f2fs.hot-ext-count", "f2fs-hot-extensions"),
        ("scsi.address", "scsi-address"),
        ("scsi.host", "scsi-host"),
        ("scsi.channel", "scsi-channel"),
        ("scsi.target", "scsi-target"),
        ("scsi.lun", "scsi-lun"),
        ("scsi.peripheral-type", "scsi-type"),
        ("scsi.vendor", "scsi-vendor"),
        ("scsi.model", "scsi-model"),
        ("scsi.revision", "scsi-revision"),
        ("scsi.block-device", "scsi-block"),
        ("scsi.generic-device", "scsi-generic"),
        ("scsi.size", "scsi-size"),
        ("scsi.transport", "scsi-transport"),
        ("scsi.unit-name", "scsi-unit"),
        ("scsi.by-id", "scsi-by-id"),
        ("scsi.wwn", "scsi-wwn"),
        ("scsi.state", "scsi-state"),
        ("scsi.queue-depth", "scsi-queue-depth"),
        ("scsi.queue-type", "scsi-queue-type"),
        ("scsi.scsi-level", "scsi-level"),
        ("scsi.timeout", "scsi-timeout"),
        ("bcachefs.external-uuid", "bcachefs-uuid"),
        ("bcachefs.internal-uuid", "bcachefs-internal"),
        ("bcachefs.magic-number", "bcachefs-magic"),
        ("bcachefs.device", "bcachefs-super-device"),
        ("bcachefs.member-device", "bcachefs-member"),
        ("bcachefs.mount-target", "bcachefs-mount"),
        ("bcachefs.device-index", "bcachefs-device"),
        ("bcachefs.version", "bcachefs-version"),
        (
            "bcachefs.version-upgrade-complete",
            "bcachefs-upgrade-complete",
        ),
        ("bcachefs.online-reserved", "bcachefs-reserved"),
        ("bcachefs.device-count", "bcachefs-devices"),
        ("bcachefs.data-sb", "bcachefs-sb"),
        ("bcachefs.data-journal", "bcachefs-journal"),
        ("bcachefs.data-btree", "bcachefs-btree"),
        ("bcachefs.data-user", "bcachefs-user"),
        ("bcachefs.data-cached", "bcachefs-cached"),
        ("bcachefs.data-parity", "bcachefs-parity"),
        ("bcachefs.device-label", "bcachefs-label"),
        ("bcachefs.device-state", "bcachefs-state"),
        ("bcachefs.device-free", "bcachefs-device-free"),
        ("bcachefs.device-capacity", "bcachefs-device-capacity"),
        ("bcachefs.device-data-sb", "bcachefs-device-sb"),
        ("bcachefs.device-data-journal", "bcachefs-device-journal"),
        ("bcachefs.device-data-btree", "bcachefs-device-btree"),
        ("bcachefs.device-data-user", "bcachefs-device-user"),
        ("bcachefs.device-data-cached", "bcachefs-device-cached"),
        ("bcache.role", "role"),
        ("bcache.kind", "kind"),
        ("bcache.backing-device", "backing-device"),
        ("bcache.set-uuid", "set-uuid"),
        ("bcache.set-average-key-size", "set-average-key-size"),
        ("bcache.set-btree-cache-size", "set-btree-cache-size"),
        (
            "bcache.set-cache-available-percent",
            "set-available-percent",
        ),
        ("bcache.set-congested", "set-congested"),
        (
            "bcache.set-congested-read-threshold-us",
            "set-congested-read-us",
        ),
        (
            "bcache.set-congested-write-threshold-us",
            "set-congested-write-us",
        ),
        ("bcache.set-io-error-halflife", "set-io-error-halflife"),
        ("bcache.set-io-error-limit", "set-io-error-limit"),
        ("bcache.set-journal-delay-ms", "set-journal-delay-ms"),
        ("bcache.set-root-usage-percent", "set-root-usage-percent"),
        ("bcache.label", "label"),
        ("bcache.state", "state"),
        ("bcache.running", "running"),
        ("bcache.cache-available-percent", "available-percent"),
        ("bcache.cache-mode", "cache-mode"),
        ("bcache.cache-replacement-policy", "replacement"),
        ("bcache.congested-read-threshold-us", "congested-read-us"),
        ("bcache.congested-write-threshold-us", "congested-write-us"),
        ("bcache.discard", "discard"),
        ("bcache.dirty-data", "dirty"),
        ("bcache.io-errors", "io-errors"),
        ("bcache.metadata-written", "metadata-written"),
        ("bcache.priority-stats", "priority-stats"),
        ("bcache.readahead", "readahead"),
        ("bcache.sequential-cutoff", "sequential-cutoff"),
        ("bcache.written", "written"),
        ("bcache.writeback-delay", "writeback-delay"),
        ("bcache.writeback-metadata", "writeback-metadata"),
        ("bcache.writeback-percent", "writeback-percent"),
        ("bcache.writeback-rate", "writeback-rate"),
        ("bcache.writeback-rate-debug", "writeback-rate-debug"),
        ("bcache.writeback-rate-d-term", "writeback-rate-d-term"),
        (
            "bcache.writeback-rate-i-term-inverse",
            "writeback-rate-i-inverse",
        ),
        ("bcache.writeback-rate-minimum", "writeback-rate-min"),
        (
            "bcache.writeback-rate-p-term-inverse",
            "writeback-rate-p-inverse",
        ),
        (
            "bcache.writeback-rate-update-seconds",
            "writeback-rate-update",
        ),
        ("bcache.writeback-running", "writeback-running"),
        ("zfs.health", "health"),
        ("zfs.pool-capacity", "pool-capacity"),
        ("zfs.pool-dedupratio", "pool-dedupratio"),
        ("zfs.pool-fragmentation", "pool-fragmentation"),
        ("zfs.pool-altroot", "pool-altroot"),
        ("zfs.pool-ashift", "pool-ashift"),
        ("zfs.pool-autotrim", "pool-autotrim"),
        ("zfs.pool-autoexpand", "pool-autoexpand"),
        ("zfs.pool-autoreplace", "pool-autoreplace"),
        ("zfs.pool-bootfs", "pool-bootfs"),
        ("zfs.pool-cachefile", "pool-cachefile"),
        ("zfs.pool-comment", "pool-comment"),
        ("zfs.pool-delegation", "pool-delegation"),
        ("zfs.pool-failmode", "pool-failmode"),
        ("zfs.pool-listsnapshots", "pool-listsnapshots"),
        ("zfs.pool-multihost", "pool-multihost"),
        ("zfs.state", "state"),
        ("zfs.status", "status"),
        ("zfs.action", "action"),
        ("zfs.scan", "scan"),
        ("zfs.errors", "errors"),
        ("zfs.pool-read-errors", "pool-read-errors"),
        ("zfs.pool-write-errors", "pool-write-errors"),
        ("zfs.pool-checksum-errors", "pool-checksum-errors"),
        ("zfs.vdev-role", "vdev-role"),
        ("zfs.vdev-state", "vdev-state"),
        ("zfs.read-errors", "read-errors"),
        ("zfs.write-errors", "write-errors"),
        ("zfs.checksum-errors", "checksum-errors"),
        ("zfs.origin", "origin"),
        ("zfs.userrefs", "userrefs"),
        ("zfs.holds", "holds"),
        ("zfs.compression", "compression"),
        ("zfs.quota", "quota"),
        ("zfs.reservation", "reservation"),
        ("zfs.encryption", "encryption"),
        ("zfs.keystatus", "keystatus"),
        ("zfs.volsize", "volsize"),
        ("zfs.recordsize", "recordsize"),
        ("zfs.dedup", "dedup"),
        ("zfs.checksum", "checksum"),
        ("zfs.copies", "copies"),
        ("zfs.sync", "sync"),
        ("zfs.primarycache", "primarycache"),
        ("zfs.secondarycache", "secondarycache"),
        ("zfs.atime", "atime"),
        ("zfs.relatime", "relatime"),
        ("zfs.snapdir", "snapdir"),
        ("zfs.acltype", "acltype"),
        ("zfs.xattr", "xattr"),
        ("xfs.meta-data.meta-data", "xfs-source"),
        ("xfs.meta-data.isize", "xfs-isize"),
        ("xfs.meta-data.agcount", "xfs-agcount"),
        ("xfs.meta-data.agsize", "xfs-agsize"),
        ("xfs.meta-data.sectsz", "xfs-sectsz"),
        ("xfs.meta-data.attr", "xfs-attr"),
        ("xfs.meta-data.projid32bit", "xfs-projid32bit"),
        ("xfs.meta-data.crc", "xfs-crc"),
        ("xfs.meta-data.finobt", "xfs-finobt"),
        ("xfs.meta-data.sparse", "xfs-sparse"),
        ("xfs.meta-data.rmapbt", "xfs-rmapbt"),
        ("xfs.data.blocks", "xfs-blocks"),
        ("xfs.data.bsize", "xfs-bsize"),
        ("xfs.data.imaxpct", "xfs-imaxpct"),
        ("xfs.data.sunit", "xfs-sunit"),
        ("xfs.data.swidth", "xfs-swidth"),
        ("xfs.meta-data.reflink", "reflink"),
        ("xfs.meta-data.bigtime", "bigtime"),
        ("xfs.meta-data.inobtcount", "xfs-inobtcount"),
        ("xfs.meta-data.nrext64", "xfs-nrext64"),
        ("xfs.naming.version", "xfs-naming-version"),
        ("xfs.naming.bsize", "xfs-naming-bsize"),
        ("xfs.naming.ascii-ci", "xfs-ascii-ci"),
        ("xfs.naming.ftype", "xfs-ftype"),
        ("xfs.log.type", "xfs-log-type"),
        ("xfs.log.bsize", "xfs-log-bsize"),
        ("xfs.log.blocks", "log-blocks"),
        ("xfs.log.version", "xfs-log-version"),
        ("xfs.log.sectsz", "xfs-log-sectsz"),
        ("xfs.log.sunit", "xfs-log-sunit"),
        ("xfs.log.lazy-count", "xfs-log-lazy-count"),
        ("xfs.realtime.type", "xfs-realtime-type"),
        ("xfs.realtime.extsz", "xfs-realtime-extsz"),
        ("xfs.realtime.blocks", "xfs-realtime-blocks"),
        ("xfs.realtime.rtextents", "xfs-realtime-rtextents"),
    ];

    let details = DETAIL_KEYS
        .iter()
        .filter_map(|(key, label)| {
            property_value(node, key).map(|value| format!("{label}={value}"))
        })
        .collect::<Vec<_>>();

    if details.is_empty() {
        "-".to_string()
    } else {
        details.join(" ")
    }
}

fn mount_details(node: &Node) -> String {
    const DETAIL_KEYS: &[(&str, &str)] = &[
        ("mount.source", "source"),
        ("mount.read-only", "ro"),
        ("mount.read-write", "rw"),
        ("mount.bind", "bind"),
        ("mount.propagation", "propagation"),
        ("mount.propagation.id", "propagation-id"),
        ("tmpfs.size", "tmpfs-size"),
        ("tmpfs.mode", "mode"),
        ("tmpfs.uid", "uid"),
        ("tmpfs.gid", "gid"),
        ("tmpfs.nr-inodes", "nr-inodes"),
        ("overlay.lowerdir", "lowerdir"),
        ("overlay.upperdir", "upperdir"),
        ("overlay.workdir", "workdir"),
        ("overlay.index", "index"),
    ];

    let details = DETAIL_KEYS
        .iter()
        .filter_map(|(key, label)| {
            property_value(node, key).map(|value| format!("{label}={value}"))
        })
        .collect::<Vec<_>>();

    if details.is_empty() {
        "-".to_string()
    } else {
        details.join(" ")
    }
}

fn backing_count(graph: &StorageGraph, node: &Node) -> usize {
    graph
        .edges
        .iter()
        .filter(|edge| {
            edge.to == node.id
                && matches!(
                    edge.relationship,
                    disk_nix_model::Relationship::Backs
                        | disk_nix_model::Relationship::DependsOn
                        | disk_nix_model::Relationship::MemberOf
                )
        })
        .count()
}

fn consumer_count(graph: &StorageGraph, node: &Node) -> usize {
    graph
        .edges
        .iter()
        .filter(|edge| {
            edge.from == node.id
                && matches!(
                    edge.relationship,
                    disk_nix_model::Relationship::Backs
                        | disk_nix_model::Relationship::DependsOn
                        | disk_nix_model::Relationship::MemberOf
                )
        })
        .count()
}

fn member_count(graph: &StorageGraph, node: &Node) -> usize {
    graph
        .edges
        .iter()
        .filter(|edge| {
            edge.to == node.id && edge.relationship == disk_nix_model::Relationship::MemberOf
        })
        .count()
}

fn iscsi_lun_count(graph: &StorageGraph, node: &Node) -> usize {
    match node.kind {
        NodeKind::IscsiSession => {
            let direct_luns = graph
                .edges
                .iter()
                .filter(|edge| {
                    edge.from == node.id
                        && edge.relationship == disk_nix_model::Relationship::Contains
                        && graph.nodes.iter().any(|candidate| {
                            candidate.id == edge.to && candidate.kind == NodeKind::Lun
                        })
                })
                .count();
            let target_luns = graph
                .edges
                .iter()
                .filter(|edge| {
                    edge.from == node.id
                        && edge.relationship == disk_nix_model::Relationship::ImportedFrom
                })
                .map(|edge| {
                    graph
                        .edges
                        .iter()
                        .filter(|candidate_edge| {
                            candidate_edge.from == edge.to
                                && candidate_edge.relationship
                                    == disk_nix_model::Relationship::Contains
                                && graph.nodes.iter().any(|candidate_node| {
                                    candidate_node.id == candidate_edge.to
                                        && candidate_node.kind == NodeKind::Lun
                                })
                        })
                        .count()
                })
                .sum::<usize>();
            direct_luns + target_luns
        }
        NodeKind::IscsiTarget => graph
            .edges
            .iter()
            .filter(|edge| {
                edge.from == node.id
                    && edge.relationship == disk_nix_model::Relationship::Contains
                    && graph
                        .nodes
                        .iter()
                        .any(|candidate| candidate.id == edge.to && candidate.kind == NodeKind::Lun)
            })
            .count(),
        _ => 0,
    }
}

fn nfs_mount_count(graph: &StorageGraph, node: &Node) -> usize {
    if node.kind != NodeKind::NfsExport {
        return 0;
    }

    graph
        .edges
        .iter()
        .filter(|edge| {
            edge.from == node.id
                && edge.relationship == disk_nix_model::Relationship::MountedAt
                && graph.nodes.iter().any(|candidate| {
                    candidate.id == edge.to && candidate.kind == NodeKind::NfsMount
                })
        })
        .count()
}

fn zfs_child_count(graph: &StorageGraph, node: &Node) -> usize {
    graph
        .edges
        .iter()
        .filter(|edge| {
            edge.from == node.id
                && matches!(
                    edge.relationship,
                    disk_nix_model::Relationship::Contains
                        | disk_nix_model::Relationship::MountedAt
                        | disk_nix_model::Relationship::SnapshotOf
                )
        })
        .count()
}

fn snapshot_source<'a>(graph: &'a StorageGraph, node: &Node) -> Option<&'a str> {
    graph
        .edges
        .iter()
        .find(|edge| {
            edge.from == node.id && edge.relationship == disk_nix_model::Relationship::SnapshotOf
        })
        .and_then(|edge| graph.nodes.iter().find(|candidate| candidate.id == edge.to))
        .map(|source| source.name.as_str())
}

fn property_value<'a>(node: &'a Node, key: &str) -> Option<&'a str> {
    node.properties
        .iter()
        .find(|property| property.key == key)
        .map(|property| property.value.as_str())
}

fn vdo_logical_display(node: &Node) -> String {
    property_value(node, "vdo.logical-size")
        .or_else(|| property_value(node, "lvm.vdo-logical-size"))
        .map(str::to_string)
        .unwrap_or_else(|| human_bytes(node.size_bytes))
}

fn vdo_physical_display(node: &Node) -> String {
    property_value(node, "vdo.physical-size")
        .or_else(|| property_value(node, "lvm.vdo-physical-size"))
        .map(str::to_string)
        .unwrap_or_else(|| human_bytes(node.usage.as_ref().and_then(|usage| usage.used_bytes)))
}

fn usage_percent(node: &Node) -> String {
    let Some(usage) = &node.usage else {
        return "-".to_string();
    };
    let Some(used) = usage.used_bytes else {
        return "-".to_string();
    };
    let capacity = node
        .size_bytes
        .or(usage.allocated_bytes)
        .or_else(|| usage.free_bytes.map(|free| used.saturating_add(free)));
    let Some(capacity) = capacity else {
        return "-".to_string();
    };
    if capacity == 0 {
        return "-".to_string();
    }

    format!("{:.1}%", (used as f64 / capacity as f64) * 100.0)
}

fn human_bytes(value: Option<u64>) -> String {
    let Some(bytes) = value else {
        return "-".to_string();
    };

    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut size = bytes as f64;
    let mut unit = UNITS[0];
    for next_unit in UNITS.iter().skip(1) {
        if size < 1024.0 {
            break;
        }
        size /= 1024.0;
        unit = next_unit;
    }

    if unit == "B" {
        format!("{bytes} B")
    } else {
        format!("{size:.1} {unit}")
    }
}

#[cfg(test)]
mod tests;
