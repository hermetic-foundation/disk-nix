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
    /// Prepare and install NixOS systems with disk-nix storage specs.
    Install {
        #[command(subcommand)]
        command: InstallCommand,
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

#[derive(Debug, Subcommand)]
enum InstallCommand {
    /// Render a reusable install storage spec template.
    Template {
        #[command(subcommand)]
        command: InstallTemplateCommand,
    },
    /// Emit or run mount commands for an install spec.
    Mount {
        /// Desired storage specification path with disk-nix install metadata.
        #[arg(long)]
        spec: String,
        /// Mount target for nixos-install.
        #[arg(long, default_value = "/mnt")]
        target: String,
        /// Write a reviewable mount script.
        #[arg(long)]
        script_out: Option<String>,
        /// Execute the generated mount script.
        #[arg(long)]
        execute: bool,
    },
    /// Emit or run mount commands followed by nixos-install.
    Nixos {
        /// Desired storage specification path with disk-nix install metadata.
        #[arg(long)]
        spec: String,
        /// NixOS flake reference, such as .#hostname.
        #[arg(long)]
        flake: String,
        /// Mount target for nixos-install.
        #[arg(long, default_value = "/mnt")]
        target: String,
        /// Write a reviewable install script.
        #[arg(long)]
        script_out: Option<String>,
        /// Execute the generated install script.
        #[arg(long)]
        execute: bool,
    },
}

#[derive(Debug, Subcommand)]
enum InstallTemplateCommand {
    /// Render an encrypted or unencrypted ZFS root spec for NixOS installs.
    ZfsRoot {
        /// Stable install disk path, preferably /dev/disk/by-id/...
        #[arg(long)]
        disk: String,
        /// Output JSON spec path.
        #[arg(long, default_value = "disk-nix-install.json")]
        out: String,
        /// ZFS pool name.
        #[arg(long, default_value = "zroot")]
        pool: String,
        /// Root dataset name. Defaults to <pool>/root.
        #[arg(long)]
        root_dataset: Option<String>,
        /// EFI partition label.
        #[arg(long, default_value = "BOOT")]
        boot_label: String,
        /// Swap partition label.
        #[arg(long, default_value = "swap")]
        swap_label: String,
        /// EFI partition start.
        #[arg(long, default_value = "1MiB")]
        efi_start: String,
        /// EFI partition end.
        #[arg(long, default_value = "1025MiB")]
        efi_end: String,
        /// Swap partition start.
        #[arg(long, default_value = "1025MiB")]
        swap_start: String,
        /// Swap partition end.
        #[arg(long, default_value = "65GiB")]
        swap_end: String,
        /// ZFS partition start.
        #[arg(long, default_value = "65GiB")]
        zfs_start: String,
        /// Partition path prefix. Defaults to <disk>-part.
        #[arg(long)]
        part_prefix: Option<String>,
        /// Enable native ZFS encryption on the root dataset.
        #[arg(long)]
        encrypt: bool,
    },
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
