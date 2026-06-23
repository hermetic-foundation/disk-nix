use std::{
    collections::BTreeSet,
    fmt,
    io::{self, Write},
    process::ExitCode,
};

use clap::{Parser, Subcommand};
use disk_nix_exec::{ExecutionMode, ExecutionReport, ExecutionStatus, prepare_execution};
use disk_nix_model::{Node, NodeKind, StorageGraph};
use disk_nix_plan::{
    Plan, default_capabilities, plan_and_policy_from_json_bytes, plan_from_json_bytes,
};
use disk_nix_probe::{LinuxProbe, ProbeAdapter, ProbeStatus};

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
#[command(version, about = "NixOS-native storage topology and lifecycle manager")]
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
    /// List discovered filesystems.
    Filesystems {
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
    /// List discovered mapping layers such as LUKS, dm, LVM, VDO, and multipath.
    Mappings {
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
    /// List storage identity fields such as UUIDs, labels, serials, and WWNs.
    Ids {
        /// Emit JSON for identity-bearing graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// Inspect a graph node by id, path, name, UUID, label, serial, or property.
    Inspect {
        /// Query value to inspect.
        query: String,
        /// Emit JSON for matched nodes and direct relationships.
        #[arg(long)]
        json: bool,
    },
    /// Plan desired storage changes from a JSON spec.
    Plan {
        /// Desired storage specification path.
        #[arg(long)]
        spec: String,
        /// Emit JSON plan output.
        #[arg(long)]
        json: bool,
    },
    /// Evaluate apply policy for a desired storage spec.
    Apply {
        /// Desired storage specification path.
        #[arg(long)]
        spec: String,
        /// Attempt execution after policy validation.
        #[arg(long)]
        execute: bool,
        /// Emit JSON apply report.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug)]
enum AppError {
    Message(String),
    Io(io::Error),
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
        Command::ProbeStatus { json } => {
            let probe = LinuxProbe::new();
            let result = probe
                .collect()
                .map_err(|error| AppError::Message(error.to_string()))?;
            if json {
                writeln!(
                    output,
                    "{}",
                    serde_json::to_string_pretty(&result.reports)
                        .map_err(|error| AppError::Message(error.to_string()))?
                )?;
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
        Command::Filesystems { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_filesystem_node)?;
            } else {
                print_filesystems(output, &graph)?;
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
        Command::Mappings { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_mapping_node)?;
            } else {
                print_mappings(output, &graph)?;
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
        Command::Ids { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, has_identity)?;
            } else {
                print_ids(output, &graph)?;
            }
            Ok(())
        }
        Command::Inspect { query, json } => {
            let graph = collect_graph()?;
            if json {
                print_inspect_json(output, &graph, &query)?;
            } else {
                print_inspect(output, &graph, &query)?;
            }
            Ok(())
        }
        Command::Plan { spec, json } => {
            let bytes = std::fs::read(&spec)?;
            let plan = plan_from_json_bytes(&bytes)
                .map_err(|error| AppError::Message(format!("failed to parse {spec}: {error}")))?;
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
            execute,
            json,
        } => {
            let bytes = std::fs::read(&spec)?;
            let (plan, policy) = plan_and_policy_from_json_bytes(&bytes)
                .map_err(|error| AppError::Message(format!("failed to parse {spec}: {error}")))?;
            let mode = if execute {
                ExecutionMode::Execute
            } else {
                ExecutionMode::DryRun
            };
            let report = prepare_execution(&plan, policy, mode);

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
            if report.status == ExecutionStatus::ExecutorUnavailable {
                return Err(AppError::Message(report.messages.join("; ")));
            }

            Ok(())
        }
    }
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
    let nodes: Vec<Node> = graph
        .nodes
        .iter()
        .filter(|node| predicate(node))
        .cloned()
        .collect();
    let node_ids: BTreeSet<String> = nodes.iter().map(|node| node.id.0.clone()).collect();
    let filtered = StorageGraph {
        nodes,
        edges: graph
            .edges
            .iter()
            .filter(|edge| {
                node_ids.contains(edge.from.0.as_str()) && node_ids.contains(edge.to.0.as_str())
            })
            .cloned()
            .collect(),
    };

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
) -> Result<(), AppError> {
    let matched_ids: BTreeSet<String> = graph
        .find_nodes(query)
        .into_iter()
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

    let subgraph = StorageGraph { nodes, edges };
    writeln!(
        output,
        "{}",
        subgraph
            .to_json()
            .map_err(|error| AppError::Message(error.to_string()))?
    )?;
    Ok(())
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

        if let Some(message) = &report.message {
            writeln!(
                output,
                "  {:<12} {:<12} {}",
                report.adapter, status, message
            )?;
        } else {
            writeln!(output, "  {:<12} {}", report.adapter, status)?;
        }
    }

    Ok(())
}

fn print_devices(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(output, "{:<22} {:<38} {:>12} PATH", "KIND", "NAME", "SIZE")?;
    for node in graph.nodes.iter().filter(|node| is_device_node(node)) {
        writeln!(
            output,
            "{:<22} {:<38} {:>12} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            node.path.as_deref().unwrap_or("-")
        )?;
    }
    Ok(())
}

fn print_filesystems(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<32} {:>12} {:>12} {:<24}",
        "KIND", "NAME", "USED", "FREE", "UUID"
    )?;
    for node in graph.nodes.iter().filter(|node| is_filesystem_node(node)) {
        writeln!(
            output,
            "{:<22} {:<32} {:>12} {:>12} {:<24}",
            node.kind,
            node.name,
            human_bytes(node.usage.as_ref().and_then(|usage| usage.used_bytes)),
            human_bytes(node.usage.as_ref().and_then(|usage| usage.free_bytes)),
            node.identity.uuid.as_deref().unwrap_or("-")
        )?;
    }
    Ok(())
}

fn print_volumes(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>12} {:>12} {:>12}",
        "KIND", "NAME", "SIZE", "USED", "FREE"
    )?;
    for node in graph.nodes.iter().filter(|node| is_volume_node(node)) {
        writeln!(
            output,
            "{:<22} {:<38} {:>12} {:>12} {:>12}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            human_bytes(node.usage.as_ref().and_then(|usage| usage.used_bytes)),
            human_bytes(node.usage.as_ref().and_then(|usage| usage.free_bytes))
        )?;
    }
    Ok(())
}

fn print_mappings(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>8} PATH",
        "KIND", "NAME", "BACKING"
    )?;
    for node in graph.nodes.iter().filter(|node| is_mapping_node(node)) {
        writeln!(
            output,
            "{:<22} {:<38} {:>8} {}",
            node.kind,
            node.name,
            backing_count(graph, node),
            node.path.as_deref().unwrap_or("-")
        )?;
    }
    Ok(())
}

fn print_mounts(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(output, "{:<22} {:<48} FSTYPE", "KIND", "TARGET")?;
    for node in graph.nodes.iter().filter(|node| is_mount_node(node)) {
        writeln!(
            output,
            "{:<22} {:<48} {}",
            node.kind,
            node.name,
            property_value(node, "filesystem.type").unwrap_or("-")
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

fn has_identity(node: &Node) -> bool {
    !node.identity.is_empty()
}

fn print_inspect(output: &mut impl Write, graph: &StorageGraph, query: &str) -> io::Result<()> {
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
            if usage.used_bytes.is_some() || usage.free_bytes.is_some() {
                writeln!(
                    output,
                    "  usage: used={} free={}",
                    human_bytes(usage.used_bytes),
                    human_bytes(usage.free_bytes)
                )?;
            }
        }

        print_identity(output, node)?;
        print_properties(output, node)?;
        print_relationships(output, graph, node)?;
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
) -> io::Result<()> {
    let edges = graph.related_edges(&node.id);
    if edges.is_empty() {
        return Ok(());
    }

    writeln!(output, "  relationships:")?;
    for edge in edges {
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
            | NodeKind::NvmeNamespace
            | NodeKind::LoopDevice
            | NodeKind::Swap
    )
}

fn is_filesystem_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::Filesystem
            | NodeKind::BtrfsFilesystem
            | NodeKind::BtrfsSubvolume
            | NodeKind::BtrfsSnapshot
            | NodeKind::BtrfsQgroup
            | NodeKind::ZfsDataset
            | NodeKind::ZfsSnapshot
            | NodeKind::NfsExport
    )
}

fn is_volume_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::LvmVolumeGroup
            | NodeKind::LvmLogicalVolume
            | NodeKind::LvmThinPool
            | NodeKind::LvmSnapshot
            | NodeKind::LvmCache
            | NodeKind::VdoVolume
            | NodeKind::MdRaid
            | NodeKind::BtrfsFilesystem
            | NodeKind::BtrfsSubvolume
            | NodeKind::BtrfsSnapshot
            | NodeKind::BtrfsQgroup
            | NodeKind::ZfsPool
            | NodeKind::ZfsDataset
            | NodeKind::ZfsSnapshot
            | NodeKind::Zvol
            | NodeKind::Lun
            | NodeKind::NfsExport
    )
}

fn is_mapping_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::LuksContainer
            | NodeKind::DeviceMapper
            | NodeKind::LvmLogicalVolume
            | NodeKind::LvmThinPool
            | NodeKind::LvmSnapshot
            | NodeKind::LvmCache
            | NodeKind::VdoVolume
            | NodeKind::MdRaid
            | NodeKind::MultipathDevice
            | NodeKind::CacheDevice
    )
}

fn is_mount_node(node: &Node) -> bool {
    matches!(node.kind, NodeKind::Mountpoint | NodeKind::NfsMount)
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

fn property_value<'a>(node: &'a Node, key: &str) -> Option<&'a str> {
    node.properties
        .iter()
        .find(|property| property.key == key)
        .map(|property| property.value.as_str())
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
