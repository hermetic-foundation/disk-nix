use disk_nix_model::{Node, NodeKind, StorageGraph};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RiskClass {
    Safe,
    Online,
    OfflineRequired,
    Reversible,
    PotentialDataLoss,
    Destructive,
    Irreversible,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Advice {
    pub summary: String,
    pub alternatives: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capability {
    pub node_kind: NodeKind,
    pub operation: Operation,
    pub risk: RiskClass,
    pub advice: Option<Advice>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Operation {
    Create,
    Format,
    Grow,
    Shrink,
    Check,
    Repair,
    Scrub,
    Trim,
    Rescan,
    ReplaceDevice,
    AddDevice,
    RemoveDevice,
    AddKey,
    RemoveKey,
    ImportToken,
    RemoveToken,
    SetProperty,
    Snapshot,
    Clone,
    Promote,
    Import,
    Export,
    Unexport,
    Attach,
    Detach,
    Activate,
    Deactivate,
    Assemble,
    Start,
    Stop,
    Login,
    Logout,
    Open,
    Close,
    Mount,
    Unmount,
    Remount,
    Rename,
    Rebalance,
    Rollback,
    Destroy,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub summary: PlanSummary,
    pub actions: Vec<PlannedAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topology_comparison: Option<TopologyComparison>,
}

impl Plan {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSummary {
    pub action_count: usize,
    pub offline_required_count: usize,
    pub destructive_count: usize,
    pub potential_data_loss_count: usize,
    pub unsupported_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopologyComparison {
    pub summary: TopologyComparisonSummary,
    pub diagnostics: Vec<TopologyDiagnostic>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopologyComparisonSummary {
    pub action_count: usize,
    pub matched_count: usize,
    pub missing_count: usize,
    pub size_diagnostic_count: usize,
    pub type_conflict_count: usize,
    pub already_satisfied_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopologyDiagnostic {
    pub action_id: String,
    pub level: TopologyDiagnosticLevel,
    pub kind: TopologyDiagnosticKind,
    pub query: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<CurrentNodeSummary>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TopologyDiagnosticLevel {
    Info,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TopologyDiagnosticKind {
    Matched,
    Missing,
    SizeBelowDesired,
    SizeAlreadySatisfied,
    SizeConflict,
    FilesystemTypeConflict,
    PropertyAlreadySatisfied,
    PropertyDiffers,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentNodeSummary {
    pub id: String,
    pub kind: NodeKind,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedAction {
    pub id: String,
    pub description: String,
    pub operation: Operation,
    pub risk: RiskClass,
    pub destructive: bool,
    #[serde(default, skip_serializing_if = "ActionContext::is_empty")]
    pub context: ActionContext,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advice: Option<Advice>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub devices: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rename_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property_value: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub property_assignments: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fs_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mountpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desired_size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub controllers: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_slot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_key_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recursive_rollback: Option<bool>,
}

impl ActionContext {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.collection.is_none()
            && self.name.is_none()
            && self.target.is_none()
            && self.snapshot_path.is_none()
            && self.device.is_none()
            && self.devices.is_empty()
            && self.replacement.is_none()
            && self.rename_to.is_none()
            && self.property.is_none()
            && self.property_value.is_none()
            && self.property_assignments.is_empty()
            && self.fs_type.is_none()
            && self.mountpoint.is_none()
            && self.desired_size.is_none()
            && self.start.is_none()
            && self.end.is_none()
            && self.partition_number.is_none()
            && self.partition_type.is_none()
            && self.level.is_none()
            && self.client.is_none()
            && self.portal.is_none()
            && self.options.is_none()
            && self.namespace_id.is_none()
            && self.controllers.is_none()
            && self.key_slot.is_none()
            && self.key_file.is_none()
            && self.new_key_file.is_none()
            && self.token_id.is_none()
            && self.token_file.is_none()
            && self.read_only.is_none()
            && self.recursive_rollback.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct ApplyPolicy {
    pub mode: ApplyMode,
    pub allow_destructive: bool,
    pub allow_format: bool,
    pub allow_shrink: bool,
    pub allow_potential_data_loss: bool,
    #[serde(default = "default_true")]
    pub allow_grow: bool,
    pub allow_offline: bool,
    #[serde(default = "default_true")]
    pub allow_property_changes: bool,
    #[serde(default = "default_true")]
    pub allow_device_replacement: bool,
    #[serde(default = "default_true")]
    pub allow_rebalance: bool,
    pub require_backup: bool,
    pub backup_verified: bool,
    pub require_confirmation: bool,
    pub confirmation: bool,
    pub require_confirmation_file: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for ApplyPolicy {
    fn default() -> Self {
        Self {
            mode: ApplyMode::Manual,
            allow_destructive: false,
            allow_format: false,
            allow_shrink: false,
            allow_potential_data_loss: false,
            allow_grow: true,
            allow_offline: false,
            allow_property_changes: true,
            allow_device_replacement: true,
            allow_rebalance: true,
            require_backup: false,
            backup_verified: false,
            require_confirmation: false,
            confirmation: false,
            require_confirmation_file: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApplyMode {
    Manual,
    Activation,
    Boot,
    Install,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyReport {
    pub policy: ApplyPolicy,
    pub allowed_count: usize,
    pub blocked_count: usize,
    pub blocked_summary: BlockedSummary,
    pub blocked: Vec<BlockedAction>,
}

impl ApplyReport {
    #[must_use]
    pub fn can_execute(&self) -> bool {
        self.blocked.is_empty()
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockedAction {
    pub id: String,
    pub operation: Operation,
    pub risk: RiskClass,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockedSummary {
    pub offline_required_count: usize,
    pub destructive_count: usize,
    pub potential_data_loss_count: usize,
    pub unsupported_count: usize,
}

pub fn plan_from_json_bytes(bytes: &[u8]) -> Result<Plan, serde_json::Error> {
    let value: Value = serde_json::from_slice(bytes)?;
    Ok(plan_from_value(&value))
}

pub fn plan_and_policy_from_json_bytes(
    bytes: &[u8],
) -> Result<(Plan, ApplyPolicy), serde_json::Error> {
    let value: Value = serde_json::from_slice(bytes)?;
    let plan = plan_from_value(&value);
    let policy = apply_policy_from_value(&value)?;
    Ok((plan, policy))
}

pub fn apply_policy_from_value(value: &Value) -> Result<ApplyPolicy, serde_json::Error> {
    match value.get("apply") {
        Some(apply) => serde_json::from_value(apply.clone()),
        None => Ok(ApplyPolicy::default()),
    }
}

#[must_use]
pub fn evaluate_apply_policy(plan: &Plan, policy: ApplyPolicy) -> ApplyReport {
    let blocked: Vec<BlockedAction> = plan
        .actions
        .iter()
        .filter_map(|action| blocked_action(action, &policy))
        .collect();
    let blocked_count = blocked.len();
    let blocked_summary = blocked_summary(&blocked);

    ApplyReport {
        policy,
        allowed_count: plan.actions.len().saturating_sub(blocked_count),
        blocked_count,
        blocked_summary,
        blocked,
    }
}

fn blocked_summary(blocked: &[BlockedAction]) -> BlockedSummary {
    BlockedSummary {
        offline_required_count: blocked
            .iter()
            .filter(|action| action.risk == RiskClass::OfflineRequired)
            .count(),
        destructive_count: blocked
            .iter()
            .filter(|action| action.risk == RiskClass::Destructive)
            .count(),
        potential_data_loss_count: blocked
            .iter()
            .filter(|action| action.risk == RiskClass::PotentialDataLoss)
            .count(),
        unsupported_count: blocked
            .iter()
            .filter(|action| action.risk == RiskClass::Unsupported)
            .count(),
    }
}

#[must_use]
pub fn plan_from_value(value: &Value) -> Plan {
    let spec = value.get("spec").unwrap_or(value);
    let mut actions = Vec::new();

    if let Some(filesystems) = spec.get("filesystems").and_then(Value::as_object) {
        for (name, filesystem) in filesystems {
            add_filesystem_actions(&mut actions, name, filesystem);
        }
    }
    if let Some(nfs_mounts) = spec
        .get("nfs")
        .and_then(|nfs| nfs.get("mounts"))
        .and_then(Value::as_object)
    {
        for (name, mount) in nfs_mounts {
            add_lifecycle_actions(&mut actions, "nfs.mounts", name, mount);
        }
    }
    if let Some(swaps) = spec.get("swaps").and_then(Value::as_object) {
        for (name, swap) in swaps {
            add_swap_actions(&mut actions, name, swap);
        }
    }
    if let Some(luks) = spec
        .get("luks")
        .and_then(|luks| luks.get("devices"))
        .and_then(Value::as_object)
    {
        for (name, luks) in luks {
            add_luks_actions(&mut actions, name, luks);
        }
    }
    for collection in [
        "disks",
        "partitions",
        "btrfsSubvolumes",
        "btrfsQgroups",
        "vdoVolumes",
        "physicalVolumes",
        "luksKeyslots",
        "luksTokens",
        "volumes",
        "volumeGroups",
        "thinPools",
        "lvmSnapshots",
        "lvmCaches",
        "loopDevices",
        "mdRaids",
        "multipathMaps",
        "pools",
        "datasets",
        "zvols",
        "luns",
        "nvmeNamespaces",
        "iscsiSessions",
        "exports",
        "caches",
    ] {
        if let Some(objects) = spec.get(collection).and_then(Value::as_object) {
            for (name, object) in objects {
                add_lifecycle_actions(&mut actions, collection, name, object);
            }
        }
    }
    if let Some(snapshots) = spec.get("snapshots").and_then(Value::as_object) {
        for (name, snapshot) in snapshots {
            add_snapshot_actions(&mut actions, name, snapshot);
        }
    }

    let summary = PlanSummary {
        action_count: actions.len(),
        offline_required_count: actions
            .iter()
            .filter(|action| action.risk == RiskClass::OfflineRequired)
            .count(),
        destructive_count: actions
            .iter()
            .filter(|action| action.risk == RiskClass::Destructive || action.destructive)
            .count(),
        potential_data_loss_count: actions
            .iter()
            .filter(|action| action.risk == RiskClass::PotentialDataLoss)
            .count(),
        unsupported_count: actions
            .iter()
            .filter(|action| action.risk == RiskClass::Unsupported)
            .count(),
    };

    Plan {
        summary,
        actions,
        topology_comparison: None,
    }
}

#[must_use]
pub fn compare_plan_with_topology(mut plan: Plan, graph: &StorageGraph) -> Plan {
    let diagnostics: Vec<TopologyDiagnostic> = plan
        .actions
        .iter()
        .flat_map(|action| topology_diagnostics_for_action(action, graph))
        .collect();

    let summary = TopologyComparisonSummary {
        action_count: plan.actions.len(),
        matched_count: diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.kind == TopologyDiagnosticKind::Matched)
            .count(),
        missing_count: diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.kind == TopologyDiagnosticKind::Missing)
            .count(),
        size_diagnostic_count: diagnostics
            .iter()
            .filter(|diagnostic| {
                matches!(
                    diagnostic.kind,
                    TopologyDiagnosticKind::SizeConflict
                        | TopologyDiagnosticKind::SizeBelowDesired
                        | TopologyDiagnosticKind::SizeAlreadySatisfied
                )
            })
            .count(),
        type_conflict_count: diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.kind == TopologyDiagnosticKind::FilesystemTypeConflict)
            .count(),
        already_satisfied_count: diagnostics
            .iter()
            .filter(|diagnostic| {
                matches!(
                    diagnostic.kind,
                    TopologyDiagnosticKind::SizeAlreadySatisfied
                        | TopologyDiagnosticKind::PropertyAlreadySatisfied
                )
            })
            .count(),
    };

    plan.topology_comparison = Some(TopologyComparison {
        summary,
        diagnostics,
    });
    plan
}

fn topology_diagnostics_for_action(
    action: &PlannedAction,
    graph: &StorageGraph,
) -> Vec<TopologyDiagnostic> {
    let Some(query) = topology_query(action) else {
        return Vec::new();
    };

    let matches = graph.find_nodes(&query);
    if matches.is_empty() {
        return vec![TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::Missing,
            query,
            message: "no current topology node matched this planned action target".to_string(),
            current: None,
        }];
    }

    let node = matches[0];
    let mut diagnostics = vec![TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::Matched,
        query: query.clone(),
        message: format!("matched current {} node {}", node.kind, node.name),
        current: Some(current_node_summary(node)),
    }];

    diagnostics.extend(size_diagnostic(action, node, &query));
    diagnostics.extend(filesystem_type_diagnostic(action, node, &query));
    diagnostics.extend(property_diagnostic(action, node, &query));
    diagnostics
}

fn topology_query(action: &PlannedAction) -> Option<String> {
    action
        .context
        .target
        .clone()
        .or_else(|| action.context.name.clone())
        .or_else(|| action.context.device.clone())
}

fn current_node_summary(node: &Node) -> CurrentNodeSummary {
    CurrentNodeSummary {
        id: node.id.0.clone(),
        kind: node.kind,
        name: node.name.clone(),
        path: node.path.clone(),
        size_bytes: node.size_bytes,
    }
}

fn size_diagnostic(action: &PlannedAction, node: &Node, query: &str) -> Option<TopologyDiagnostic> {
    let desired = action.context.desired_size.as_deref()?;
    let desired_bytes = parse_size_bytes(desired)?;
    let current_bytes = node.size_bytes?;

    let (level, kind, message) = match action.operation {
        Operation::Grow if current_bytes >= desired_bytes => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::SizeAlreadySatisfied,
            format!("current size {current_bytes} bytes already satisfies desired size {desired}"),
        ),
        Operation::Grow => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::SizeBelowDesired,
            format!("current size {current_bytes} bytes is below desired size {desired}"),
        ),
        Operation::Shrink if current_bytes <= desired_bytes => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::SizeAlreadySatisfied,
            format!(
                "current size {current_bytes} bytes is already at or below desired size {desired}"
            ),
        ),
        Operation::Shrink => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::SizeConflict,
            format!("current size {current_bytes} bytes is above desired shrink target {desired}"),
        ),
        _ => return None,
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level,
        kind,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn filesystem_type_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    let desired = action.context.fs_type.as_deref()?;
    let current = property_value_from_node(node, "filesystem.type")?;
    if current == desired {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::FilesystemTypeConflict,
        query: query.to_string(),
        message: format!("desired filesystem type {desired} differs from current {current}"),
        current: Some(current_node_summary(node)),
    })
}

fn property_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::SetProperty {
        return None;
    }
    let property = action.context.property.as_deref()?;
    let desired = action.context.property_value.as_deref()?;
    let current = property_value_from_node(node, property)?;
    let (level, kind, message) = if current == desired {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::PropertyAlreadySatisfied,
            format!("property {property} already has desired value {desired}"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::PropertyDiffers,
            format!("property {property} is {current}, desired {desired}"),
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level,
        kind,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn property_value_from_node<'a>(node: &'a Node, key: &str) -> Option<&'a str> {
    node.properties
        .iter()
        .find(|property| property.key == key)
        .map(|property| property.value.as_str())
}

fn parse_size_bytes(value: &str) -> Option<u64> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.ends_with('%') {
        return None;
    }

    let number_end = trimmed
        .find(|character: char| !(character.is_ascii_digit() || character == '.'))
        .unwrap_or(trimmed.len());
    let number = trimmed[..number_end].parse::<f64>().ok()?;
    let unit = trimmed[number_end..].trim().to_ascii_lowercase();
    let multiplier = match unit.as_str() {
        "" | "b" => 1_f64,
        "k" | "kb" => 1_000_f64,
        "m" | "mb" => 1_000_000_f64,
        "g" | "gb" => 1_000_000_000_f64,
        "t" | "tb" => 1_000_000_000_000_f64,
        "p" | "pb" => 1_000_000_000_000_000_f64,
        "ki" | "kib" => 1024_f64,
        "mi" | "mib" => 1024_f64.powi(2),
        "gi" | "gib" => 1024_f64.powi(3),
        "ti" | "tib" => 1024_f64.powi(4),
        "pi" | "pib" => 1024_f64.powi(5),
        _ => return None,
    };

    let bytes = number * multiplier;
    bytes.is_finite().then_some(bytes.round() as u64)
}

fn blocked_action(action: &PlannedAction, policy: &ApplyPolicy) -> Option<BlockedAction> {
    let reason = if action.risk == RiskClass::Unsupported {
        Some("unsupported actions cannot be applied")
    } else if requires_backup(action) && policy.require_backup && !policy.backup_verified {
        Some("backup-required actions require backupVerified=true")
    } else if requires_confirmation(action) && policy.require_confirmation && !policy.confirmation {
        Some("confirmation-required actions require confirmation=true")
    } else if requires_confirmation(action)
        && policy.require_confirmation_file.is_some()
        && !policy.confirmation
    {
        Some(
            "confirmation-file policy requires confirmation=true after checking the configured file",
        )
    } else if action.risk == RiskClass::OfflineRequired && !policy.allow_offline {
        Some("offline-required actions require allowOffline=true")
    } else if action.operation == Operation::Format && !policy.allow_format {
        Some("format actions require allowFormat=true")
    } else if action.operation == Operation::Shrink && !policy.allow_shrink {
        Some("shrink actions require allowShrink=true")
    } else if action.risk == RiskClass::PotentialDataLoss && !policy.allow_potential_data_loss {
        Some("potential-data-loss actions require allowPotentialDataLoss=true")
    } else if action.operation == Operation::Grow && !policy.allow_grow {
        Some("grow actions require allowGrow=true")
    } else if matches!(
        action.operation,
        Operation::AddDevice | Operation::ReplaceDevice | Operation::RemoveDevice
    ) && !policy.allow_device_replacement
    {
        Some("device topology changes require allowDeviceReplacement=true")
    } else if action.operation == Operation::Rebalance && !policy.allow_rebalance {
        Some("rebalance actions require allowRebalance=true")
    } else if action.operation == Operation::SetProperty && !policy.allow_property_changes {
        Some("property changes require allowPropertyChanges=true")
    } else if action.operation == Operation::Format && !policy.allow_destructive {
        Some("format actions also require allowDestructive=true")
    } else if action.destructive
        || action.risk == RiskClass::Destructive
        || action.risk == RiskClass::Irreversible
    {
        (!policy.allow_destructive)
            .then_some("destructive or irreversible actions require allowDestructive=true")
    } else {
        None
    }?;

    Some(BlockedAction {
        id: action.id.clone(),
        operation: action.operation,
        risk: action.risk,
        reason: reason.to_string(),
    })
}

fn requires_backup(action: &PlannedAction) -> bool {
    action.destructive
        || matches!(
            action.risk,
            RiskClass::PotentialDataLoss | RiskClass::Destructive | RiskClass::Irreversible
        )
}

fn requires_confirmation(action: &PlannedAction) -> bool {
    requires_backup(action)
        || matches!(
            action.risk,
            RiskClass::OfflineRequired | RiskClass::Unsupported
        )
}

fn filesystem_context(
    name: &str,
    mountpoint: &str,
    fs_type: &str,
    device: Option<String>,
    desired_size: Option<String>,
) -> ActionContext {
    ActionContext {
        collection: Some("filesystems".to_string()),
        name: Some(name.to_string()),
        target: Some(mountpoint.to_string()),
        device,
        fs_type: Some(fs_type.to_string()),
        mountpoint: Some(mountpoint.to_string()),
        desired_size,
        ..ActionContext::default()
    }
}

fn lifecycle_context(collection: &str, name: &str, object: &Value) -> ActionContext {
    ActionContext {
        collection: Some(collection.to_string()),
        name: Some(name.to_string()),
        target: string_field(object, &["target", "path", "mountpoint"]).or(Some(name.to_string())),
        device: string_field(object, &["device", "disk", "source"]),
        devices: string_array_field(object, &["devices", "addDevices"]),
        rename_to: string_field(object, &["renameTo", "renameTarget", "newName"]),
        fs_type: string_field(object, &["fsType", "type"]),
        mountpoint: string_field(object, &["mountpoint", "path"])
            .or_else(|| name.starts_with('/').then(|| name.to_string())),
        desired_size: desired_size(object),
        start: string_field(object, &["start", "startOffset"]),
        end: string_field(object, &["end", "endOffset"]),
        partition_number: string_field(object, &["partitionNumber", "number"]),
        partition_type: string_field(object, &["partitionType", "type"]),
        level: string_field(object, &["level", "raidLevel"]),
        client: string_field(object, &["client"]),
        portal: lifecycle_portal(object),
        options: lifecycle_options(object),
        namespace_id: metadata_string_field(object, &["namespaceId", "nsid"]),
        controllers: metadata_string_field(object, &["controllers", "controllerId", "controller"]),
        key_slot: metadata_string_field(object, &["keySlot", "key-slot", "slot"]),
        key_file: metadata_string_field(object, &["keyFile", "key-file", "currentKeyFile"]),
        new_key_file: metadata_string_field(object, &["newKeyFile", "new-key-file"]),
        token_id: metadata_string_field(object, &["tokenId", "token-id", "token"]),
        token_file: metadata_string_field(object, &["tokenFile", "token-file", "jsonFile"]),
        read_only: object
            .get("readOnly")
            .or_else(|| object.get("readonly"))
            .and_then(Value::as_bool),
        property_assignments: property_assignments(object),
        ..ActionContext::default()
    }
}

fn string_field(object: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        object.get(*key).and_then(|value| match value {
            Value::String(value) => Some(value.clone()),
            Value::Number(value) => Some(value.to_string()),
            _ => None,
        })
    })
}

fn string_array_field(object: &Value, keys: &[&str]) -> Vec<String> {
    keys.iter()
        .find_map(|key| {
            object.get(*key).and_then(|value| {
                value.as_array().map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(ToString::to_string))
                        .collect::<Vec<_>>()
                })
            })
        })
        .unwrap_or_default()
}

fn desired_size(object: &Value) -> Option<String> {
    object
        .get("desiredSize")
        .or_else(|| object.get("targetSize"))
        .or_else(|| object.get("size"))
        .and_then(|value| match value {
            Value::String(size) => Some(size.clone()),
            Value::Number(size) => Some(size.to_string()),
            _ => None,
        })
}

fn lifecycle_options(object: &Value) -> Option<String> {
    string_field(object, &["options"])
        .or_else(|| {
            let options = string_array_field(object, &["options"]);
            if options.is_empty() {
                None
            } else {
                Some(options.join(","))
            }
        })
        .or_else(|| {
            object
                .get("properties")
                .and_then(|properties| string_field(properties, &["options"]))
        })
}

fn property_assignments(object: &Value) -> Vec<String> {
    object
        .get("properties")
        .and_then(Value::as_object)
        .map(|properties| {
            properties
                .iter()
                .map(|(property, value)| format!("{property}={}", property_value(value)))
                .collect()
        })
        .unwrap_or_default()
}

fn lifecycle_portal(object: &Value) -> Option<String> {
    string_field(object, &["portal"]).or_else(|| {
        object
            .get("metadata")
            .and_then(|metadata| string_field(metadata, &["portal"]))
    })
}

fn metadata_string_field(object: &Value, keys: &[&str]) -> Option<String> {
    string_field(object, keys).or_else(|| {
        object
            .get("metadata")
            .and_then(|metadata| string_field(metadata, keys))
    })
}

fn add_filesystem_actions(actions: &mut Vec<PlannedAction>, name: &str, filesystem: &Value) {
    let mountpoint = filesystem
        .get("mountpoint")
        .and_then(Value::as_str)
        .unwrap_or(name);
    let fs_type = filesystem
        .get("fsType")
        .or_else(|| filesystem.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let resize_policy = filesystem
        .get("resizePolicy")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let preserve_data = filesystem
        .get("preserveData")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let desired_size = desired_size(filesystem);
    let device = string_field(filesystem, &["device", "disk"]);

    match resize_policy {
        "grow-only" => actions.push(PlannedAction {
            id: format!("filesystem:{name}:grow"),
            description: format!(
                "allow non-destructive growth for {fs_type} filesystem at {mountpoint}"
            ),
            operation: Operation::Grow,
            risk: RiskClass::Online,
            destructive: false,
            context: filesystem_context(
                name,
                mountpoint,
                fs_type,
                device.clone(),
                desired_size.clone(),
            ),
            advice: None,
        }),
        "shrink-allowed" => actions.push(filesystem_shrink_action(
            name,
            mountpoint,
            fs_type,
            device.clone(),
            desired_size.clone(),
        )),
        _ => actions.push(PlannedAction {
            id: format!("filesystem:{name}:inspect"),
            description: format!("inspect {fs_type} filesystem declaration at {mountpoint}"),
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            destructive: false,
            context: filesystem_context(
                name,
                mountpoint,
                fs_type,
                device.clone(),
                desired_size.clone(),
            ),
            advice: None,
        }),
    }

    if !preserve_data {
        actions.push(PlannedAction {
            id: format!("filesystem:{name}:preserve-data-disabled"),
            description: format!(
                "preserveData=false permits destructive replacement for filesystem at {mountpoint}"
            ),
            operation: Operation::Format,
            risk: RiskClass::Destructive,
            destructive: true,
            context: filesystem_context(
                name,
                mountpoint,
                fs_type,
                device.clone(),
                desired_size.clone(),
            ),
            advice: Some(Advice {
                summary: "formatting or replacing a filesystem destroys existing data".to_string(),
                alternatives: vec![
                    "leave preserveData=true and request a grow or property-only update"
                        .to_string(),
                    "migrate data to a new filesystem before replacing this one".to_string(),
                    "require an explicit backup and confirmation policy before applying"
                        .to_string(),
                ],
            }),
        });
    }

    if let Some(
        operation @ (Operation::Check
        | Operation::Repair
        | Operation::Scrub
        | Operation::Trim
        | Operation::Rebalance
        | Operation::Mount
        | Operation::Unmount
        | Operation::Remount),
    ) = filesystem
        .get("operation")
        .or_else(|| filesystem.get("action"))
        .and_then(Value::as_str)
        .and_then(parse_operation)
    {
        let (risk, destructive, advice) = classify_operation("filesystems", operation, filesystem);
        actions.push(PlannedAction {
            id: format!("filesystems:{name}:{}", operation_id(operation)),
            description: format!(
                "plan {} operation for filesystem {name}",
                operation_id(operation)
            ),
            operation,
            risk,
            destructive,
            context: ActionContext {
                collection: Some("filesystems".to_string()),
                options: lifecycle_options(filesystem),
                property_assignments: property_assignments(filesystem),
                ..filesystem_context(
                    name,
                    mountpoint,
                    fs_type,
                    device.clone(),
                    desired_size.clone(),
                )
            },
            advice,
        });
    }

    add_device_membership_actions(actions, "filesystems", name, filesystem);
    add_filesystem_property_actions(actions, name, mountpoint, fs_type, filesystem);
}

fn add_filesystem_property_actions(
    actions: &mut Vec<PlannedAction>,
    name: &str,
    mountpoint: &str,
    fs_type: &str,
    filesystem: &Value,
) {
    let Some(properties) = filesystem.get("properties").and_then(Value::as_object) else {
        return;
    };
    let desired_size = desired_size(filesystem);

    for (property, value) in properties {
        if fs_type == "btrfs" && is_btrfs_balance_filter_property(property) {
            continue;
        }
        let (risk, advice) = classify_filesystem_property_change(fs_type, property, value);
        actions.push(PlannedAction {
            id: format!("filesystems:{name}:set-property:{property}"),
            description: format!("set property {property} on filesystem {name}"),
            operation: Operation::SetProperty,
            risk,
            destructive: false,
            context: ActionContext {
                property: Some(property.to_string()),
                property_value: Some(property_value(value)),
                property_assignments: property_assignments(filesystem),
                ..filesystem_context(
                    name,
                    mountpoint,
                    fs_type,
                    string_field(filesystem, &["device", "disk"]),
                    desired_size.clone(),
                )
            },
            advice,
        });
    }
}

fn classify_filesystem_property_change(
    fs_type: &str,
    property: &str,
    value: &Value,
) -> (RiskClass, Option<Advice>) {
    if is_fat_filesystem_uuid_property(fs_type, property)
        && !is_valid_fat_volume_id(&property_value(value))
    {
        return (
            RiskClass::Unsupported,
            Some(Advice {
                summary: format!(
                    "{fs_type} filesystem volume ID {} is not a valid FAT volume ID",
                    property_value(value)
                ),
                alternatives: vec![
                    "use an 8-hex-digit FAT volume ID such as A1B2-C3D4 or A1B2C3D4".to_string(),
                    "update NixOS fileSystems and boot references instead of changing the FAT volume ID when possible"
                        .to_string(),
                    "recreate the FAT filesystem only when data preservation is not required"
                        .to_string(),
                ],
            }),
        );
    }

    if is_ntfs_filesystem_uuid_property(fs_type, property)
        && !is_valid_ntfs_volume_serial(&property_value(value))
    {
        return (
            RiskClass::Unsupported,
            Some(Advice {
                summary: format!(
                    "{fs_type} filesystem volume serial {} is not a valid NTFS serial",
                    property_value(value)
                ),
                alternatives: vec![
                    "use a 16-hex-digit NTFS volume serial such as 01234567-89ABCDEF or 0123456789ABCDEF"
                        .to_string(),
                    "update NixOS fileSystems and dependent mount references instead of changing the NTFS serial when possible"
                        .to_string(),
                    "leave the NTFS serial unchanged unless consumers explicitly depend on it"
                        .to_string(),
                ],
            }),
        );
    }

    if is_exfat_filesystem_uuid_property(fs_type, property)
        && !is_valid_exfat_volume_serial(&property_value(value))
    {
        return (
            RiskClass::Unsupported,
            Some(Advice {
                summary: format!(
                    "{fs_type} filesystem volume serial {} is not a valid exFAT serial",
                    property_value(value)
                ),
                alternatives: vec![
                    "use an 8-hex-digit exFAT volume serial such as A1B2-C3D4 or A1B2C3D4"
                        .to_string(),
                    "update NixOS fileSystems and dependent mount references instead of changing the exFAT serial when possible"
                        .to_string(),
                    "leave the exFAT serial unchanged unless consumers explicitly depend on it"
                        .to_string(),
                ],
            }),
        );
    }

    if is_filesystem_uuid_property_supported(fs_type, property) {
        return (
            RiskClass::OfflineRequired,
            Some(Advice {
                summary: format!(
                    "{fs_type} filesystem UUID updates mutate filesystem identity metadata"
                ),
                alternatives: vec![
                    "prefer updating references to the current UUID when possible".to_string(),
                    "update NixOS fileSystems, initrd, bootloader, and dependent mount references before changing the UUID"
                        .to_string(),
                    "perform UUID changes with the filesystem unmounted and a recovery path available"
                        .to_string(),
                ],
            }),
        );
    }
    if is_filesystem_property_supported(fs_type, property) {
        return (RiskClass::Safe, None);
    }

    (
        RiskClass::Unsupported,
        Some(Advice {
            summary: format!(
                "{fs_type} filesystem property {property} is not mapped to a safe command"
            ),
            alternatives: vec![
                "use label, filesystem.label, btrfs.label, exfat.label, ext.label, f2fs.label, fat.label, ntfs.label, vfat.label, or xfs.label when changing filesystem labels"
                    .to_string(),
                "use uuid, filesystem.uuid, btrfs.uuid, exfat.uuid, ext.uuid, fat.uuid, ntfs.uuid, vfat.uuid, or xfs.uuid when changing supported filesystem UUIDs"
                    .to_string(),
                "use ZFS dataset declarations for arbitrary zfs set property updates".to_string(),
                "apply unsupported filesystem property changes manually after reviewing filesystem-specific tooling"
                    .to_string(),
            ],
        }),
    )
}

fn is_filesystem_property_supported(fs_type: &str, property: &str) -> bool {
    match fs_type {
        "btrfs" => matches!(
            property,
            "label"
                | "btrfs.label"
                | "filesystem.label"
                | "uuid"
                | "btrfs.uuid"
                | "filesystem.uuid"
        ),
        "ext2" | "ext3" | "ext4" => {
            matches!(
                property,
                "label"
                    | "ext.label"
                    | "filesystem.label"
                    | "uuid"
                    | "ext.uuid"
                    | "filesystem.uuid"
            )
        }
        "fat" | "fat12" | "fat16" | "fat32" | "msdos" | "vfat" => matches!(
            property,
            "label"
                | "fat.label"
                | "vfat.label"
                | "filesystem.label"
                | "uuid"
                | "fat.uuid"
                | "vfat.uuid"
                | "filesystem.uuid"
                | "volumeId"
                | "volume-id"
                | "fat.volume-id"
                | "vfat.volume-id"
        ),
        "ntfs" | "ntfs3" => matches!(
            property,
            "label"
                | "ntfs.label"
                | "filesystem.label"
                | "uuid"
                | "ntfs.uuid"
                | "filesystem.uuid"
                | "serial"
                | "volumeSerial"
                | "volume-serial"
                | "ntfs.serial"
                | "ntfs.volume-serial"
        ),
        "exfat" => matches!(
            property,
            "label"
                | "exfat.label"
                | "filesystem.label"
                | "uuid"
                | "exfat.uuid"
                | "filesystem.uuid"
                | "serial"
                | "volumeSerial"
                | "volume-serial"
                | "exfat.serial"
                | "exfat.volume-serial"
        ),
        "f2fs" => matches!(property, "label" | "f2fs.label" | "filesystem.label"),
        "xfs" => matches!(
            property,
            "label" | "xfs.label" | "filesystem.label" | "uuid" | "xfs.uuid" | "filesystem.uuid"
        ),
        "zfs" => true,
        _ => false,
    }
}

fn is_filesystem_uuid_property_supported(fs_type: &str, property: &str) -> bool {
    matches!(
        (fs_type, property),
        ("btrfs", "uuid" | "btrfs.uuid" | "filesystem.uuid")
            | (
                "ext2" | "ext3" | "ext4",
                "uuid" | "ext.uuid" | "filesystem.uuid"
            )
            | (
                "fat" | "fat12" | "fat16" | "fat32" | "msdos" | "vfat",
                "uuid"
                    | "fat.uuid"
                    | "vfat.uuid"
                    | "filesystem.uuid"
                    | "volumeId"
                    | "volume-id"
                    | "fat.volume-id"
                    | "vfat.volume-id"
            )
            | (
                "ntfs" | "ntfs3",
                "uuid"
                    | "ntfs.uuid"
                    | "filesystem.uuid"
                    | "serial"
                    | "volumeSerial"
                    | "volume-serial"
                    | "ntfs.serial"
                    | "ntfs.volume-serial"
            )
            | (
                "exfat",
                "uuid"
                    | "exfat.uuid"
                    | "filesystem.uuid"
                    | "serial"
                    | "volumeSerial"
                    | "volume-serial"
                    | "exfat.serial"
                    | "exfat.volume-serial"
            )
            | ("xfs", "uuid" | "xfs.uuid" | "filesystem.uuid")
    )
}

fn is_fat_filesystem_uuid_property(fs_type: &str, property: &str) -> bool {
    matches!(
        (fs_type, property),
        (
            "fat" | "fat12" | "fat16" | "fat32" | "msdos" | "vfat",
            "uuid"
                | "fat.uuid"
                | "vfat.uuid"
                | "filesystem.uuid"
                | "volumeId"
                | "volume-id"
                | "fat.volume-id"
                | "vfat.volume-id"
        )
    )
}

fn is_valid_fat_volume_id(value: &str) -> bool {
    fat_volume_id(value).is_some()
}

fn fat_volume_id(value: &str) -> Option<String> {
    let normalized: String = value
        .trim()
        .chars()
        .filter(|character| *character != '-')
        .collect();
    if normalized.len() == 8
        && normalized
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        Some(normalized.to_ascii_uppercase())
    } else {
        None
    }
}

fn is_ntfs_filesystem_uuid_property(fs_type: &str, property: &str) -> bool {
    matches!(
        (fs_type, property),
        (
            "ntfs" | "ntfs3",
            "uuid"
                | "ntfs.uuid"
                | "filesystem.uuid"
                | "serial"
                | "volumeSerial"
                | "volume-serial"
                | "ntfs.serial"
                | "ntfs.volume-serial"
        )
    )
}

fn is_valid_ntfs_volume_serial(value: &str) -> bool {
    ntfs_volume_serial(value).is_some()
}

fn ntfs_volume_serial(value: &str) -> Option<String> {
    let normalized: String = value
        .trim()
        .chars()
        .filter(|character| *character != '-')
        .collect();
    if normalized.len() == 16
        && normalized
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        Some(normalized.to_ascii_uppercase())
    } else {
        None
    }
}

fn is_exfat_filesystem_uuid_property(fs_type: &str, property: &str) -> bool {
    matches!(
        (fs_type, property),
        (
            "exfat",
            "uuid"
                | "exfat.uuid"
                | "filesystem.uuid"
                | "serial"
                | "volumeSerial"
                | "volume-serial"
                | "exfat.serial"
                | "exfat.volume-serial"
        )
    )
}

fn is_valid_exfat_volume_serial(value: &str) -> bool {
    exfat_volume_serial(value).is_some()
}

fn exfat_volume_serial(value: &str) -> Option<String> {
    let normalized: String = value
        .trim()
        .chars()
        .filter(|character| *character != '-')
        .collect();
    if normalized.len() == 8
        && normalized
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        Some(normalized.to_ascii_uppercase())
    } else {
        None
    }
}

fn is_btrfs_balance_filter_property(property: &str) -> bool {
    let property = property
        .strip_prefix("btrfs.balance.")
        .or_else(|| property.strip_prefix("balance."))
        .unwrap_or(property);
    matches!(
        property,
        "data" | "d" | "metadata" | "meta" | "m" | "system" | "s"
    )
}

fn add_swap_actions(actions: &mut Vec<PlannedAction>, name: &str, swap: &Value) {
    let device = string_field(swap, &["device"]).unwrap_or_else(|| name.to_string());
    let operation = swap
        .get("operation")
        .or_else(|| swap.get("action"))
        .and_then(Value::as_str)
        .and_then(parse_operation);
    let preserve_data = swap
        .get("preserveData")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let desired_size = desired_size(swap);
    let context = ActionContext {
        collection: Some("swaps".to_string()),
        name: Some(name.to_string()),
        target: Some(device.clone()),
        device: Some(device.clone()),
        desired_size: desired_size.clone(),
        ..ActionContext::default()
    };

    match operation {
        Some(Operation::Grow) => actions.push(PlannedAction {
            id: format!("swaps:{name}:grow"),
            description: format!("grow swap backing storage for {device}"),
            operation: Operation::Grow,
            risk: RiskClass::OfflineRequired,
            destructive: false,
            context: context.clone(),
            advice: Some(Advice {
                summary:
                    "swap growth requires disabling active swap before resizing backing storage"
                        .to_string(),
                alternatives: vec![
                    "add a second swap device before resizing this one".to_string(),
                    "disable swap, resize backing storage, recreate the signature, and re-enable"
                        .to_string(),
                    "verify memory pressure and hibernation dependencies before disabling swap"
                        .to_string(),
                ],
            }),
        }),
        Some(Operation::Rescan) => actions.push(PlannedAction {
            id: format!("swaps:{name}:rescan"),
            description: format!("refresh swap inventory for {device}"),
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            destructive: false,
            context: context.clone(),
            advice: Some(Advice {
                summary: "swap rescan refreshes signature, activation, and graph inventory"
                    .to_string(),
                alternatives: vec![
                    "use grow when backing swap capacity must change".to_string(),
                    "use format only when replacing the swap signature is intended".to_string(),
                    "verify resume and hibernation references before changing swap identity"
                        .to_string(),
                ],
            }),
        }),
        Some(Operation::Create | Operation::Format) => actions.push(swap_format_action(
            name,
            &device,
            desired_size,
            "create or refresh swap signature",
        )),
        _ if !preserve_data => actions.push(swap_format_action(
            name,
            &device,
            desired_size,
            "preserveData=false permits recreating the swap signature",
        )),
        _ => actions.push(PlannedAction {
            id: format!("swaps:{name}:inspect"),
            description: format!("inspect swap declaration for {device}"),
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            destructive: false,
            context: context.clone(),
            advice: None,
        }),
    }

    add_swap_property_actions(actions, name, swap, &context);
}

fn add_swap_property_actions(
    actions: &mut Vec<PlannedAction>,
    name: &str,
    swap: &Value,
    context: &ActionContext,
) {
    let Some(properties) = swap.get("properties").and_then(Value::as_object) else {
        return;
    };

    for (property, value) in properties {
        let (risk, advice) = classify_swap_property_change(property);
        actions.push(PlannedAction {
            id: format!("swaps:{name}:set-property:{property}"),
            description: format!("set swap property {property} on {name}"),
            operation: Operation::SetProperty,
            risk,
            destructive: false,
            context: ActionContext {
                property: Some(property.to_string()),
                property_value: Some(property_value(value)),
                ..context.clone()
            },
            advice,
        });
    }
}

fn classify_swap_property_change(property: &str) -> (RiskClass, Option<Advice>) {
    if is_swap_identity_property(property) {
        return (
            RiskClass::OfflineRequired,
            Some(Advice {
                summary: "swap label and UUID updates mutate swap signature identity".to_string(),
                alternatives: vec![
                    "prefer updating NixOS swapDevices references to the current identity when possible"
                        .to_string(),
                    "disable active swap and verify hibernation/resume references before changing identity"
                        .to_string(),
                    "use a stable device path instead of changing swap UUID when consumers allow it"
                        .to_string(),
                ],
            }),
        );
    }

    (
        RiskClass::Unsupported,
        Some(Advice {
            summary: format!("swap property {property} is not mapped to a safe command"),
            alternatives: vec![
                "use label, swap.label, uuid, or swap.uuid for swap identity changes".to_string(),
                "recreate the swap signature with preserveData=false only when overwriting metadata is intended"
                    .to_string(),
                "apply unsupported swap changes manually after reviewing util-linux swap tools"
                    .to_string(),
            ],
        }),
    )
}

fn is_swap_identity_property(property: &str) -> bool {
    matches!(property, "label" | "swap.label" | "uuid" | "swap.uuid")
}

fn swap_format_action(
    name: &str,
    device: &str,
    desired_size: Option<String>,
    description: &str,
) -> PlannedAction {
    PlannedAction {
        id: format!("swaps:{name}:format"),
        description: format!("{description} on {device}"),
        operation: Operation::Format,
        risk: RiskClass::Destructive,
        destructive: true,
        context: ActionContext {
            collection: Some("swaps".to_string()),
            name: Some(name.to_string()),
            target: Some(device.to_string()),
            device: Some(device.to_string()),
            desired_size,
            ..ActionContext::default()
        },
        advice: Some(Advice {
            summary: "creating a swap signature overwrites existing metadata on the target"
                .to_string(),
            alternatives: vec![
                "use an additional swap file or device instead of replacing this target"
                    .to_string(),
                "verify the target contains no filesystem or encrypted data before mkswap"
                    .to_string(),
                "set preserveData=true for inspection-only planning".to_string(),
            ],
        }),
    }
}

fn add_luks_actions(actions: &mut Vec<PlannedAction>, name: &str, luks: &Value) {
    let device = string_field(luks, &["device"]);
    let device_label = device.as_deref().unwrap_or("<device>");
    let mapper_name = string_field(luks, &["name"]).unwrap_or_else(|| name.to_string());
    let operation = luks
        .get("operation")
        .or_else(|| luks.get("action"))
        .and_then(Value::as_str)
        .and_then(parse_operation);
    let preserve_data = luks
        .get("preserveData")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let context = ActionContext {
        collection: Some("luks.devices".to_string()),
        name: Some(name.to_string()),
        target: Some(mapper_name.clone()),
        device: device.clone(),
        property_assignments: property_assignments(luks),
        ..ActionContext::default()
    };
    let has_properties = luks
        .get("properties")
        .and_then(Value::as_object)
        .is_some_and(|properties| !properties.is_empty());

    match operation {
        Some(Operation::Grow) => actions.push(PlannedAction {
            id: format!("luks.devices:{name}:grow"),
            description: format!("resize LUKS mapping {mapper_name} on {device_label}"),
            operation: Operation::Grow,
            risk: RiskClass::OfflineRequired,
            destructive: false,
            context,
            advice: Some(Advice {
                summary: "LUKS resize requires backing-device growth and mapper coordination"
                    .to_string(),
                alternatives: vec![
                    "grow the partition, LUN, or volume before resizing the LUKS mapper"
                        .to_string(),
                    "verify the mapping is open and dependent layers are paused or coordinated"
                        .to_string(),
                    "resize filesystems only after cryptsetup resize reports the new size"
                        .to_string(),
                ],
            }),
        }),
        Some(Operation::Destroy | Operation::Close) => {
            actions.push(luks_close_action(
                name,
                &mapper_name,
                device_label,
                operation.expect("operation already matched"),
                context,
            ));
        }
        Some(Operation::Open) => {
            actions.push(luks_open_action(
                name,
                &mapper_name,
                device_label,
                Operation::Open,
                context,
            ));
        }
        Some(Operation::Create) if preserve_data => {
            actions.push(luks_open_action(
                name,
                &mapper_name,
                device_label,
                Operation::Create,
                context,
            ));
        }
        Some(Operation::Create | Operation::Format) => actions.push(luks_format_action(
            name,
            device.clone(),
            &mapper_name,
            "create or replace LUKS container",
        )),
        _ if !preserve_data => actions.push(luks_format_action(
            name,
            device.clone(),
            &mapper_name,
            "preserveData=false permits replacing the LUKS container",
        )),
        _ if !has_properties => actions.push(PlannedAction {
            id: format!("luks.devices:{name}:inspect"),
            description: format!("inspect LUKS declaration {mapper_name} on {device_label}"),
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            destructive: false,
            context,
            advice: None,
        }),
        _ => {}
    }

    add_luks_property_actions(actions, name, &mapper_name, device, luks);
}

fn luks_format_action(
    name: &str,
    device: Option<String>,
    mapper_name: &str,
    description: &str,
) -> PlannedAction {
    let device_label = device.as_deref().unwrap_or("<device>");
    PlannedAction {
        id: format!("luks.devices:{name}:format"),
        description: format!("{description} on {device_label}"),
        operation: Operation::Format,
        risk: RiskClass::Destructive,
        destructive: true,
        context: ActionContext {
            collection: Some("luks.devices".to_string()),
            name: Some(name.to_string()),
            target: Some(mapper_name.to_string()),
            device,
            ..ActionContext::default()
        },
        advice: Some(Advice {
            summary: "formatting a LUKS container destroys access to existing encrypted data"
                .to_string(),
            alternatives: vec![
                "open and reuse the existing LUKS container when data must be preserved"
                    .to_string(),
                "back up headers with cryptsetup luksHeaderBackup before destructive work"
                    .to_string(),
                "create a new encrypted target and migrate data before switching mounts"
                    .to_string(),
            ],
        }),
    }
}

fn luks_open_action(
    name: &str,
    mapper_name: &str,
    device_label: &str,
    operation: Operation,
    context: ActionContext,
) -> PlannedAction {
    PlannedAction {
        id: format!("luks.devices:{name}:{}", operation_id(operation)),
        description: format!("open existing LUKS container {device_label} as {mapper_name}"),
        operation,
        risk: RiskClass::OfflineRequired,
        destructive: false,
        context,
        advice: Some(Advice {
            summary: "opening a LUKS mapper changes active device topology without formatting"
                .to_string(),
            alternatives: vec![
                "verify the backing device is the intended LUKS container before opening"
                    .to_string(),
                "use preserveData=false or operation=format only when replacing the header"
                    .to_string(),
                "create filesystems or LVM layers only after the mapper appears".to_string(),
            ],
        }),
    }
}

fn luks_close_action(
    name: &str,
    mapper_name: &str,
    device_label: &str,
    operation: Operation,
    context: ActionContext,
) -> PlannedAction {
    PlannedAction {
        id: format!("luks.devices:{name}:{}", operation_id(operation)),
        description: format!("close LUKS mapping {mapper_name} without formatting {device_label}"),
        operation,
        risk: RiskClass::OfflineRequired,
        destructive: false,
        context,
        advice: Some(Advice {
            summary: "closing a LUKS mapper requires dependent layers to be stopped".to_string(),
            alternatives: vec![
                "unmount filesystems and deactivate LVM volumes before closing the mapper"
                    .to_string(),
                "leave the LUKS header and backing device intact for later reopen".to_string(),
                "use preserveData=false only when reformatting is explicitly intended".to_string(),
            ],
        }),
    }
}

fn add_luks_property_actions(
    actions: &mut Vec<PlannedAction>,
    name: &str,
    mapper_name: &str,
    device: Option<String>,
    luks: &Value,
) {
    let Some(properties) = luks.get("properties").and_then(Value::as_object) else {
        return;
    };

    for (property, value) in properties {
        let (risk, advice) = classify_luks_device_property_change(property);
        actions.push(PlannedAction {
            id: format!("luks.devices:{name}:set-property:{property}"),
            description: format!("set LUKS header property {property} on {mapper_name}"),
            operation: Operation::SetProperty,
            risk,
            destructive: false,
            context: ActionContext {
                collection: Some("luks.devices".to_string()),
                name: Some(name.to_string()),
                target: Some(mapper_name.to_string()),
                device: device.clone(),
                property: Some(property.to_string()),
                property_value: Some(property_value(value)),
                property_assignments: property_assignments(luks),
                ..ActionContext::default()
            },
            advice,
        });
    }
}

fn classify_luks_device_property_change(property: &str) -> (RiskClass, Option<Advice>) {
    if is_luks_identity_property(property) {
        return (
            RiskClass::OfflineRequired,
            Some(Advice {
                summary: format!(
                    "LUKS header property {property} updates encrypted-container identity metadata"
                ),
                alternatives: vec![
                    "prefer updating consumers to stable by-id paths when possible".to_string(),
                    "back up the LUKS header before changing header identity metadata".to_string(),
                    "verify initrd, crypttab, and NixOS LUKS references after identity changes"
                        .to_string(),
                ],
            }),
        );
    }

    (
        RiskClass::Unsupported,
        Some(Advice {
            summary: format!("LUKS header property {property} is not mapped to a safe command"),
            alternatives: vec![
                "use label, luks.label, subsystem, luks.subsystem, uuid, or luks.uuid for supported LUKS identity changes"
                    .to_string(),
                "use luksKeyslots or luksTokens declarations for access-material changes"
                    .to_string(),
                "apply unsupported LUKS header changes manually after reviewing cryptsetup documentation"
                    .to_string(),
            ],
        }),
    )
}

fn is_luks_identity_property(property: &str) -> bool {
    matches!(
        property,
        "label"
            | "luks.label"
            | "cryptsetup.label"
            | "subsystem"
            | "luks.subsystem"
            | "cryptsetup.subsystem"
            | "uuid"
            | "luks.uuid"
            | "cryptsetup.uuid"
    )
}

fn filesystem_shrink_action(
    name: &str,
    mountpoint: &str,
    fs_type: &str,
    device: Option<String>,
    desired_size: Option<String>,
) -> PlannedAction {
    let (risk, advice) = match fs_type {
        "xfs" => (
            RiskClass::Unsupported,
            Advice {
                summary: "XFS does not support shrinking in place".to_string(),
                alternatives: vec![
                    "create a new smaller filesystem and migrate data".to_string(),
                    "snapshot or back up the current filesystem before migration".to_string(),
                    "switch the mount to the replacement filesystem after verification".to_string(),
                ],
            },
        ),
        "btrfs" => (
            RiskClass::PotentialDataLoss,
            Advice {
                summary:
                    "Btrfs shrink requires enough data and metadata slack before resizing"
                        .to_string(),
                alternatives: vec![
                    "run a balance to reduce allocated chunks before shrink".to_string(),
                    "remove or replace devices only after checking filesystem usage".to_string(),
                    "take a snapshot or backup before resizing".to_string(),
                ],
            },
        ),
        "ext2" | "ext3" | "ext4" => (
            RiskClass::PotentialDataLoss,
            Advice {
                summary: format!("{fs_type} shrink requires offline filesystem checks"),
                alternatives: vec![
                    "unmount the filesystem and run fsck before resize".to_string(),
                    "take and verify a backup before shrinking".to_string(),
                    "create a new smaller filesystem and migrate data when downtime is not acceptable"
                        .to_string(),
                ],
            },
        ),
        _ => (
            RiskClass::PotentialDataLoss,
            Advice {
                summary:
                    "shrinking can require offline checks and filesystem-specific migration paths"
                        .to_string(),
                alternatives: vec![
                    "prefer grow-only policies for live systems".to_string(),
                    "create a new smaller filesystem and migrate data when shrink support is absent"
                        .to_string(),
                    "take and verify a backup before any shrink attempt".to_string(),
                ],
            },
        ),
    };

    PlannedAction {
        id: format!("filesystem:{name}:shrink"),
        description: format!("allow shrink evaluation for {fs_type} filesystem at {mountpoint}"),
        operation: Operation::Shrink,
        risk,
        destructive: false,
        context: filesystem_context(name, mountpoint, fs_type, device, desired_size),
        advice: Some(advice),
    }
}

fn add_lifecycle_actions(
    actions: &mut Vec<PlannedAction>,
    collection: &str,
    name: &str,
    object: &Value,
) {
    add_requested_operation(actions, collection, name, object);
    add_device_membership_actions(actions, collection, name, object);
    add_property_actions(actions, collection, name, object);
    add_destroy_guard(actions, collection, name, object);
}

fn add_requested_operation(
    actions: &mut Vec<PlannedAction>,
    collection: &str,
    name: &str,
    object: &Value,
) {
    let Some(operation) = object
        .get("operation")
        .or_else(|| object.get("action"))
        .and_then(Value::as_str)
        .and_then(parse_operation)
    else {
        return;
    };
    let (risk, destructive, advice) = classify_operation(collection, operation, object);
    let operation_name = match operation {
        Operation::AddKey
        | Operation::RemoveKey
        | Operation::ImportToken
        | Operation::RemoveToken => operation_id(operation).to_string(),
        _ => format!("{operation:?}").to_ascii_lowercase(),
    };
    actions.push(PlannedAction {
        id: format!("{collection}:{name}:{operation_name}").to_ascii_lowercase(),
        description: format!(
            "plan {} operation for {collection} {name}",
            operation_label(operation)
        ),
        operation,
        risk,
        destructive,
        context: lifecycle_context(collection, name, object),
        advice,
    });
}

fn add_device_membership_actions(
    actions: &mut Vec<PlannedAction>,
    collection: &str,
    name: &str,
    object: &Value,
) {
    if let Some(devices) = object.get("addDevices").and_then(Value::as_array) {
        for device in devices.iter().filter_map(Value::as_str) {
            let (risk, advice) = classify_add_device(collection);
            actions.push(PlannedAction {
                id: format!("{collection}:{name}:add-device:{device}"),
                description: format!("add device {device} to {collection} {name}"),
                operation: Operation::AddDevice,
                risk,
                destructive: false,
                context: ActionContext {
                    device: Some(device.to_string()),
                    ..lifecycle_context(collection, name, object)
                },
                advice,
            });
        }
    }

    if let Some(devices) = object.get("removeDevices").and_then(Value::as_array) {
        for device in devices.iter().filter_map(Value::as_str) {
            let (risk, advice) = classify_remove_device(collection);
            actions.push(PlannedAction {
                id: format!("{collection}:{name}:remove-device:{device}"),
                description: format!("remove device {device} from {collection} {name}"),
                operation: Operation::RemoveDevice,
                risk,
                destructive: false,
                context: ActionContext {
                    device: Some(device.to_string()),
                    ..lifecycle_context(collection, name, object)
                },
                advice: Some(advice),
            });
        }
    }

    if let Some(replacements) = object.get("replaceDevices").and_then(Value::as_object) {
        for (from, to) in replacements
            .iter()
            .filter_map(|(from, to)| to.as_str().map(|replacement| (from.as_str(), replacement)))
        {
            let (risk, advice) = classify_replace_device(collection);
            actions.push(PlannedAction {
                id: format!("{collection}:{name}:replace-device:{from}"),
                description: format!("replace device {from} with {to} in {collection} {name}"),
                operation: Operation::ReplaceDevice,
                risk,
                destructive: false,
                context: ActionContext {
                    device: Some(from.to_string()),
                    replacement: Some(to.to_string()),
                    ..lifecycle_context(collection, name, object)
                },
                advice: Some(advice),
            });
        }
    }
}

fn add_property_actions(
    actions: &mut Vec<PlannedAction>,
    collection: &str,
    name: &str,
    object: &Value,
) {
    let Some(properties) = object.get("properties").and_then(Value::as_object) else {
        return;
    };

    for (property, value) in properties {
        let (risk, advice) = classify_property_change(collection, property, value);
        actions.push(PlannedAction {
            id: format!("{collection}:{name}:set-property:{property}"),
            description: format!("set property {property} on {collection} {name}"),
            operation: Operation::SetProperty,
            risk,
            destructive: false,
            context: ActionContext {
                property: Some(property.to_string()),
                property_value: Some(property_value(value)),
                ..lifecycle_context(collection, name, object)
            },
            advice,
        });
    }
}

fn classify_property_change(
    collection: &str,
    property: &str,
    value: &Value,
) -> (RiskClass, Option<Advice>) {
    if collection == "btrfsSubvolumes" && !is_btrfs_subvolume_property_supported(property) {
        return (
            RiskClass::Unsupported,
            Some(Advice {
                summary: format!("Btrfs subvolume property {property} is not mapped to a safe command"),
                alternatives: vec![
                    "use readOnly, readonly, ro, btrfs.readonly, or btrfs.ro for read-only toggles"
                        .to_string(),
                    "apply unsupported Btrfs subvolume property changes manually after reviewing btrfs property documentation"
                        .to_string(),
                ],
            }),
        );
    }

    if collection == "vdoVolumes" {
        return classify_vdo_property_change(property, value);
    }

    if collection == "lvmCaches" {
        return (
            RiskClass::Safe,
            Some(Advice {
                summary: format!(
                    "LVM cache property {property} changes cache behavior on the origin LV"
                ),
                alternatives: vec![
                    "prefer writethrough mode before cache detach or replacement".to_string(),
                    "verify dirty cache data is drained before disabling writeback".to_string(),
                    "review lvs cache fields after changing cache policy or mode".to_string(),
                ],
            }),
        );
    }

    if collection == "luksKeyslots" || collection == "luksTokens" {
        return (
            RiskClass::OfflineRequired,
            Some(Advice {
                summary: format!(
                    "LUKS access property {property} updates encrypted-container access material"
                ),
                alternatives: vec![
                    "verify at least one independent recovery key before changing key material"
                        .to_string(),
                    "add and test replacement access before removing the old keyslot or token"
                        .to_string(),
                    "back up the LUKS header before access changes".to_string(),
                ],
            }),
        );
    }

    (RiskClass::Safe, None)
}

fn classify_vdo_property_change(property: &str, value: &Value) -> (RiskClass, Option<Advice>) {
    let property_name = normalize_storage_property_name(property);
    let normalized_value = normalize_storage_property_name(&property_value(value));
    let safe_advice = || {
        Some(Advice {
            summary: format!("VDO property {property} updates an existing VDO volume in place"),
            alternatives: vec![
                "verify vdo status and vdostats before and after the property update".to_string(),
                "prefer changing the existing VDO volume over recreating it when preserving data"
                    .to_string(),
                "review dependent filesystems and mappings before changing write policy"
                    .to_string(),
            ],
        })
    };
    let unsupported_advice = |summary: String, alternatives: Vec<String>| {
        (
            RiskClass::Unsupported,
            Some(Advice {
                summary,
                alternatives,
            }),
        )
    };

    match property_name.as_str() {
        "writepolicy" | "write-policy" | "vdo-write-policy" => {
            if matches!(normalized_value.as_str(), "auto" | "sync" | "async") {
                (RiskClass::Safe, safe_advice())
            } else {
                unsupported_advice(
                    format!(
                        "VDO write policy value {} is not supported",
                        property_value(value)
                    ),
                    vec![
                        "use auto, sync, or async for VDO writePolicy updates".to_string(),
                        "inspect the backing storage cache behavior before choosing sync or async"
                            .to_string(),
                    ],
                )
            }
        }
        "compression" | "vdo-compression" => {
            if is_vdo_boolean_property_value(&normalized_value) {
                (RiskClass::Safe, safe_advice())
            } else {
                unsupported_advice(
                    format!(
                        "VDO compression value {} is not mapped to enable or disable",
                        property_value(value)
                    ),
                    vec![
                        "use enabled/disabled, true/false, yes/no, or on/off for compression"
                            .to_string(),
                        "leave compression unchanged until the intended boolean state is explicit"
                            .to_string(),
                    ],
                )
            }
        }
        "deduplication" | "dedupe" | "vdo-deduplication" | "vdo-dedupe" => {
            if is_vdo_boolean_property_value(&normalized_value) {
                (RiskClass::Safe, safe_advice())
            } else {
                unsupported_advice(
                    format!(
                        "VDO deduplication value {} is not mapped to enable or disable",
                        property_value(value)
                    ),
                    vec![
                        "use enabled/disabled, true/false, yes/no, or on/off for deduplication"
                            .to_string(),
                        "inspect VDO space savings before changing deduplication state".to_string(),
                    ],
                )
            }
        }
        _ => unsupported_advice(
            format!("VDO property {property} is not mapped to a safe command"),
            vec![
                "use writePolicy, compression, or deduplication for supported VDO updates"
                    .to_string(),
                "apply unsupported VDO property changes manually after reviewing VDO tooling"
                    .to_string(),
            ],
        ),
    }
}

fn is_vdo_boolean_property_value(value: &str) -> bool {
    matches!(
        value,
        "enabled"
            | "enable"
            | "true"
            | "yes"
            | "on"
            | "disabled"
            | "disable"
            | "false"
            | "no"
            | "off"
    )
}

fn normalize_storage_property_name(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("vdo.")
        .chars()
        .map(|character| match character {
            'A'..='Z' => character.to_ascii_lowercase(),
            'a'..='z' | '0'..='9' => character,
            _ => '-',
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn is_btrfs_subvolume_property_supported(property: &str) -> bool {
    matches!(
        property,
        "ro" | "readonly" | "readOnly" | "btrfs.readonly" | "btrfs.ro"
    )
}

fn property_value(value: &Value) -> String {
    value
        .as_str()
        .map(ToString::to_string)
        .unwrap_or_else(|| value.to_string())
}

fn add_destroy_guard(
    actions: &mut Vec<PlannedAction>,
    collection: &str,
    name: &str,
    object: &Value,
) {
    let destroy = object
        .get("destroy")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let preserve_data = object
        .get("preserveData")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    if destroy || !preserve_data {
        if collection == "exports" {
            actions.push(PlannedAction {
                id: format!("{collection}:{name}:destroy").to_ascii_lowercase(),
                description: format!("unexport NFS path {name}"),
                operation: Operation::Destroy,
                risk: RiskClass::OfflineRequired,
                destructive: false,
                context: lifecycle_context(collection, name, object),
                advice: Some(Advice {
                    summary: "unexporting NFS paths can interrupt active remote clients"
                        .to_string(),
                    alternatives: vec![
                        "remove or migrate clients before unexporting the path".to_string(),
                        "switch export options to read-only before final removal".to_string(),
                        "verify no active mounts depend on the export before reload".to_string(),
                    ],
                }),
            });
            return;
        }
        if collection == "iscsiSessions" {
            actions.push(PlannedAction {
                id: format!("{collection}:{name}:destroy").to_ascii_lowercase(),
                description: format!("log out iSCSI session {name}"),
                operation: Operation::Destroy,
                risk: RiskClass::OfflineRequired,
                destructive: false,
                context: lifecycle_context(collection, name, object),
                advice: Some(Advice {
                    summary: "iSCSI logout detaches remote LUN paths from the host".to_string(),
                    alternatives: vec![
                        "unmount filesystems and deactivate mappings before logout".to_string(),
                        "verify multipath, LVM, and filesystem consumers have migrated away"
                            .to_string(),
                        "disable automatic login only after dependent services no longer need the LUN"
                            .to_string(),
                    ],
                }),
            });
            return;
        }
        if collection == "nfs.mounts" {
            actions.push(PlannedAction {
                id: format!("{collection}:{name}:destroy").to_ascii_lowercase(),
                description: format!("unmount NFS client mount {name}"),
                operation: Operation::Destroy,
                risk: RiskClass::OfflineRequired,
                destructive: false,
                context: lifecycle_context(collection, name, object),
                advice: Some(Advice {
                    summary: "unmounting an NFS client path can interrupt local services"
                        .to_string(),
                    alternatives: vec![
                        "stop local services and automount units before unmounting".to_string(),
                        "switch the mount to read-only or noauto before final removal".to_string(),
                        "verify no open files or bind mounts still depend on the mountpoint"
                            .to_string(),
                    ],
                }),
            });
            return;
        }
        if collection == "luns" {
            actions.push(PlannedAction {
                id: format!("{collection}:{name}:destroy").to_ascii_lowercase(),
                description: format!("detach LUN paths for {name}"),
                operation: Operation::Destroy,
                risk: RiskClass::OfflineRequired,
                destructive: false,
                context: lifecycle_context(collection, name, object),
                advice: Some(Advice {
                    summary: "LUN host detach removes reviewed SCSI paths from this host"
                        .to_string(),
                    alternatives: vec![
                        "unmount filesystems and deactivate LVM, multipath, or dm consumers before detach"
                            .to_string(),
                        "remove a single path only after redundancy or alternate paths are healthy"
                            .to_string(),
                        "disable automatic session login only after dependent services no longer need the LUN"
                            .to_string(),
                    ],
                }),
            });
            return;
        }
        if collection == "physicalVolumes" {
            actions.push(PlannedAction {
                id: format!("{collection}:{name}:destroy").to_ascii_lowercase(),
                description: format!("remove LVM physical volume metadata from {name}"),
                operation: Operation::Destroy,
                risk: RiskClass::Destructive,
                destructive: true,
                context: lifecycle_context(collection, name, object),
                advice: Some(Advice {
                    summary: "LVM physical volume removal erases PV metadata from the device"
                        .to_string(),
                    alternatives: vec![
                        "pvmove allocated extents and vgreduce the PV before pvremove".to_string(),
                        "verify no volume group still depends on the PV".to_string(),
                        "preserve the device for recovery until backups are verified".to_string(),
                    ],
                }),
            });
            return;
        }
        if collection == "lvmCaches" {
            actions.push(PlannedAction {
                id: format!("{collection}:{name}:destroy").to_ascii_lowercase(),
                description: format!("detach LVM cache from {name}"),
                operation: Operation::Destroy,
                risk: RiskClass::OfflineRequired,
                destructive: false,
                context: lifecycle_context(collection, name, object),
                advice: Some(Advice {
                    summary: "LVM cache removal must flush dirty cache state before uncaching"
                        .to_string(),
                    alternatives: vec![
                        "switch to writethrough and wait for dirty blocks to drain before lvconvert --uncache"
                            .to_string(),
                        "verify the origin LV is readable without the cache before removing cache media"
                            .to_string(),
                        "keep the cache pool intact until post-uncache verification passes".to_string(),
                    ],
                }),
            });
            return;
        }
        if collection == "luksKeyslots" {
            actions.push(PlannedAction {
                id: format!("{collection}:{name}:destroy").to_ascii_lowercase(),
                description: format!("remove LUKS keyslot {name}"),
                operation: Operation::Destroy,
                risk: RiskClass::PotentialDataLoss,
                destructive: false,
                context: lifecycle_context(collection, name, object),
                advice: Some(Advice {
                    summary:
                        "removing a LUKS keyslot can lock out encrypted data if no other key works"
                            .to_string(),
                    alternatives: vec![
                        "verify another passphrase, key file, or token unlocks the device first"
                            .to_string(),
                        "take a LUKS header backup before keyslot removal".to_string(),
                        "add and test a replacement keyslot before killing the old slot"
                            .to_string(),
                    ],
                }),
            });
            return;
        }
        if collection == "luksTokens" {
            actions.push(PlannedAction {
                id: format!("{collection}:{name}:destroy").to_ascii_lowercase(),
                description: format!("remove LUKS token {name}"),
                operation: Operation::Destroy,
                risk: RiskClass::PotentialDataLoss,
                destructive: false,
                context: lifecycle_context(collection, name, object),
                advice: Some(Advice {
                    summary:
                        "removing a LUKS token can lock out automated unlock if no other path works"
                            .to_string(),
                    alternatives: vec![
                        "verify a passphrase, recovery key, or replacement token unlocks the device first"
                            .to_string(),
                        "take a LUKS header backup before token removal".to_string(),
                        "import and test a replacement token before removing the old token".to_string(),
                    ],
                }),
            });
            return;
        }

        let mut alternatives = destructive_alternatives(collection, object);
        alternatives.push("rename, detach, or unmount first when supported".to_string());
        actions.push(PlannedAction {
            id: format!("{collection}:{name}:destroy").to_ascii_lowercase(),
            description: format!("{collection} {name} may be destroyed or replaced"),
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            destructive: true,
            context: lifecycle_context(collection, name, object),
            advice: Some(Advice {
                summary: "destroying or replacing storage removes live data".to_string(),
                alternatives,
            }),
        });
    }
}

fn add_snapshot_actions(actions: &mut Vec<PlannedAction>, name: &str, snapshot: &Value) {
    let target = snapshot
        .get("target")
        .and_then(Value::as_str)
        .unwrap_or(name);
    let snapshot_path = string_field(snapshot, &["path", "snapshotPath", "snapshot-path"]);
    let hold = string_field(snapshot, &["hold", "holdTag"]);
    let release_hold = string_field(snapshot, &["releaseHold", "release-hold"]);
    let clone_to = string_field(snapshot, &["cloneTo", "cloneTarget", "clone"]);
    let rename_to = string_field(snapshot, &["renameTo", "renameTarget", "newName"]);
    let destroy = snapshot
        .get("destroy")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let rollback = snapshot
        .get("rollback")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let recursive_rollback = snapshot
        .get("recursiveRollback")
        .or_else(|| snapshot.get("recursive"))
        .or_else(|| snapshot.get("zfs.rollbackRecursive"))
        .and_then(Value::as_bool);
    let read_only = snapshot
        .get("readOnly")
        .or_else(|| snapshot.get("readonly"))
        .and_then(Value::as_bool);
    let requested_operation = snapshot
        .get("operation")
        .or_else(|| snapshot.get("action"))
        .and_then(Value::as_str)
        .and_then(parse_operation);

    if requested_operation == Some(Operation::Rescan) {
        actions.push(PlannedAction {
            id: format!("snapshot:{name}:rescan"),
            description: format!("rescan snapshot metadata for {name}"),
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            destructive: false,
            context: ActionContext {
                collection: Some("snapshots".to_string()),
                name: Some(name.to_string()),
                target: Some(target.to_string()),
                snapshot_path: snapshot_path.clone(),
                read_only,
                ..ActionContext::default()
            },
            advice: Some(Advice {
                summary: "snapshot rescan refreshes recovery-point metadata without mutating data"
                    .to_string(),
                alternatives: vec![
                    "use holds for retention changes instead of recreating snapshots".to_string(),
                    "clone a snapshot for inspection before rollback or destruction".to_string(),
                    "verify source dataset or subvolume relationships after metadata refresh"
                        .to_string(),
                ],
            }),
        });
    }

    if let Some(hold) = hold {
        actions.push(snapshot_hold_action(name, target, &hold, read_only, false));
    }
    if let Some(release_hold) = release_hold {
        actions.push(snapshot_hold_action(
            name,
            target,
            &release_hold,
            read_only,
            true,
        ));
    }
    if let Some(clone_to) = clone_to {
        actions.push(PlannedAction {
            id: format!("snapshot:{name}:clone:{clone_to}"),
            description: format!("clone snapshot {name} to {clone_to}"),
            operation: Operation::Clone,
            risk: RiskClass::Reversible,
            destructive: false,
            context: ActionContext {
                collection: Some("snapshots".to_string()),
                name: Some(name.to_string()),
                target: Some(clone_to),
                read_only,
                ..ActionContext::default()
            },
            advice: Some(Advice {
                summary: "ZFS snapshot clone creates a writable dataset from the snapshot"
                    .to_string(),
                alternatives: vec![
                    "inspect the clone before rollback or destructive changes".to_string(),
                    "destroy the clone after migration or validation if it is no longer needed"
                        .to_string(),
                ],
            }),
        });
    }
    if let Some(rename_to) = rename_to {
        actions.push(PlannedAction {
            id: format!("snapshot:{name}:rename:{rename_to}"),
            description: format!("rename snapshot {name} to {rename_to}"),
            operation: Operation::Rename,
            risk: RiskClass::OfflineRequired,
            destructive: false,
            context: ActionContext {
                collection: Some("snapshots".to_string()),
                name: Some(name.to_string()),
                target: Some(target.to_string()),
                rename_to: Some(rename_to),
                read_only,
                ..ActionContext::default()
            },
            advice: Some(Advice {
                summary:
                    "snapshot rename preserves the recovery point while changing its reference"
                        .to_string(),
                alternatives: vec![
                    "hold the snapshot before renaming when retention jobs may race".to_string(),
                    "update replication, rollback, and cleanup references after rename".to_string(),
                ],
            }),
        });
    }

    if destroy {
        actions.push(PlannedAction {
            id: format!("snapshot:{name}:destroy"),
            description: format!("destroy snapshot {name} for {target}"),
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            destructive: true,
            context: ActionContext {
                collection: Some("snapshots".to_string()),
                name: Some(name.to_string()),
                target: Some(target.to_string()),
                read_only,
                ..ActionContext::default()
            },
            advice: Some(Advice {
                summary: "snapshot destruction removes a recovery point".to_string(),
                alternatives: vec![
                    "keep the snapshot until replacement backups are verified".to_string(),
                    "rename or hold the snapshot before pruning".to_string(),
                ],
            }),
        });
    } else if rollback {
        actions.push(PlannedAction {
            id: format!("snapshot:{name}:rollback"),
            description: format!("roll back {target} to snapshot {name}"),
            operation: Operation::Rollback,
            risk: RiskClass::PotentialDataLoss,
            destructive: false,
            context: ActionContext {
                collection: Some("snapshots".to_string()),
                name: Some(name.to_string()),
                target: Some(target.to_string()),
                read_only,
                recursive_rollback,
                ..ActionContext::default()
            },
            advice: Some(Advice {
                summary: "rollback can discard changes newer than the snapshot".to_string(),
                alternatives: vec![
                    "clone the snapshot and inspect data before rollback".to_string(),
                    "take a pre-rollback snapshot of the current state".to_string(),
                ],
            }),
        });
    } else if actions
        .iter()
        .all(|action| !action.id.starts_with(&format!("snapshot:{name}:")))
    {
        actions.push(PlannedAction {
            id: format!("snapshot:{name}:create"),
            description: format!("create snapshot {name} for {target}"),
            operation: Operation::Snapshot,
            risk: RiskClass::Reversible,
            destructive: false,
            context: ActionContext {
                collection: Some("snapshots".to_string()),
                name: Some(name.to_string()),
                target: Some(target.to_string()),
                read_only,
                ..ActionContext::default()
            },
            advice: None,
        });
    }
}

fn snapshot_hold_action(
    name: &str,
    target: &str,
    tag: &str,
    read_only: Option<bool>,
    release: bool,
) -> PlannedAction {
    let (verb, property) = if release {
        ("release hold on", "zfs.releaseHold")
    } else {
        ("hold", "zfs.hold")
    };
    PlannedAction {
        id: format!(
            "snapshot:{name}:{}:{tag}",
            if release { "release-hold" } else { "hold" }
        ),
        description: format!("{verb} snapshot {name} for {target} with tag {tag}"),
        operation: Operation::SetProperty,
        risk: RiskClass::Safe,
        destructive: false,
        context: ActionContext {
            collection: Some("snapshots".to_string()),
            name: Some(name.to_string()),
            target: Some(target.to_string()),
            property: Some(property.to_string()),
            property_value: Some(tag.to_string()),
            read_only,
            ..ActionContext::default()
        },
        advice: Some(Advice {
            summary: if release {
                "releasing a snapshot hold allows later pruning by the same tag".to_string()
            } else {
                "holding a snapshot prevents accidental ZFS snapshot destruction by tag".to_string()
            },
            alternatives: if release {
                vec![
                    "keep the hold until replacement backups or replication are verified"
                        .to_string(),
                    "list active holds before releasing retention protection".to_string(),
                ]
            } else {
                vec![
                    "use a stable tag name that identifies the retention policy".to_string(),
                    "replicate or back up the snapshot before removing retention holds".to_string(),
                ]
            },
        }),
    }
}

fn parse_operation(value: &str) -> Option<Operation> {
    match value {
        "create" => Some(Operation::Create),
        "format" => Some(Operation::Format),
        "grow" => Some(Operation::Grow),
        "shrink" => Some(Operation::Shrink),
        "check" => Some(Operation::Check),
        "repair" => Some(Operation::Repair),
        "scrub" => Some(Operation::Scrub),
        "trim" => Some(Operation::Trim),
        "rescan" | "re-scan" => Some(Operation::Rescan),
        "replace-device" | "replaceDevice" => Some(Operation::ReplaceDevice),
        "add-device" | "addDevice" => Some(Operation::AddDevice),
        "remove-device" | "removeDevice" => Some(Operation::RemoveDevice),
        "add-key" | "addKey" | "add-keyslot" | "addKeyslot" => Some(Operation::AddKey),
        "remove-key" | "removeKey" | "remove-keyslot" | "removeKeyslot" | "kill-slot"
        | "killSlot" => Some(Operation::RemoveKey),
        "import-token" | "importToken" => Some(Operation::ImportToken),
        "remove-token" | "removeToken" => Some(Operation::RemoveToken),
        "set-property" | "setProperty" => Some(Operation::SetProperty),
        "snapshot" => Some(Operation::Snapshot),
        "clone" => Some(Operation::Clone),
        "promote" => Some(Operation::Promote),
        "import" => Some(Operation::Import),
        "export" => Some(Operation::Export),
        "unexport" | "un-export" => Some(Operation::Unexport),
        "attach" => Some(Operation::Attach),
        "detach" => Some(Operation::Detach),
        "activate" => Some(Operation::Activate),
        "deactivate" => Some(Operation::Deactivate),
        "assemble" => Some(Operation::Assemble),
        "start" => Some(Operation::Start),
        "stop" => Some(Operation::Stop),
        "login" | "log-in" | "logIn" => Some(Operation::Login),
        "logout" | "log-out" | "logOut" => Some(Operation::Logout),
        "open" => Some(Operation::Open),
        "close" => Some(Operation::Close),
        "mount" => Some(Operation::Mount),
        "unmount" | "un-mount" | "umount" => Some(Operation::Unmount),
        "remount" => Some(Operation::Remount),
        "rename" => Some(Operation::Rename),
        "rebalance" => Some(Operation::Rebalance),
        "rollback" => Some(Operation::Rollback),
        "destroy" => Some(Operation::Destroy),
        _ => None,
    }
}

fn operation_id(operation: Operation) -> &'static str {
    match operation {
        Operation::Create => "create",
        Operation::Format => "format",
        Operation::Grow => "grow",
        Operation::Shrink => "shrink",
        Operation::Check => "check",
        Operation::Repair => "repair",
        Operation::Scrub => "scrub",
        Operation::Trim => "trim",
        Operation::Rescan => "rescan",
        Operation::ReplaceDevice => "replace-device",
        Operation::AddDevice => "add-device",
        Operation::RemoveDevice => "remove-device",
        Operation::AddKey => "add-key",
        Operation::RemoveKey => "remove-key",
        Operation::ImportToken => "import-token",
        Operation::RemoveToken => "remove-token",
        Operation::SetProperty => "set-property",
        Operation::Snapshot => "snapshot",
        Operation::Clone => "clone",
        Operation::Promote => "promote",
        Operation::Import => "import",
        Operation::Export => "export",
        Operation::Unexport => "unexport",
        Operation::Attach => "attach",
        Operation::Detach => "detach",
        Operation::Activate => "activate",
        Operation::Deactivate => "deactivate",
        Operation::Assemble => "assemble",
        Operation::Start => "start",
        Operation::Stop => "stop",
        Operation::Login => "login",
        Operation::Logout => "logout",
        Operation::Open => "open",
        Operation::Close => "close",
        Operation::Mount => "mount",
        Operation::Unmount => "unmount",
        Operation::Remount => "remount",
        Operation::Rename => "rename",
        Operation::Rebalance => "rebalance",
        Operation::Rollback => "rollback",
        Operation::Destroy => "destroy",
    }
}

fn classify_operation(
    collection: &str,
    operation: Operation,
    object: &Value,
) -> (RiskClass, bool, Option<Advice>) {
    match operation {
        Operation::Create if collection == "disks" => (
            RiskClass::Destructive,
            true,
            Some(Advice {
                summary: "creating or replacing a disk partition table can hide existing data"
                    .to_string(),
                alternatives: destructive_alternatives(collection, object),
            }),
        ),
        Operation::Check if collection == "filesystems" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "filesystem consistency checks require a stable source device"
                    .to_string(),
                alternatives: vec![
                    "prefer read-only checks before any repair attempt".to_string(),
                    "unmount or quiesce the filesystem when the checker requires it".to_string(),
                    "capture current topology and recent backups before maintenance".to_string(),
                ],
            }),
        ),
        Operation::Repair if collection == "filesystems" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "filesystem repair mutates metadata and must be reviewed offline"
                    .to_string(),
                alternatives: vec![
                    "run a read-only check first and review the reported damage".to_string(),
                    "restore from backup or snapshot when repair risk is unacceptable".to_string(),
                    "repair a cloned block device before touching the production source"
                        .to_string(),
                ],
            }),
        ),
        Operation::Scrub if collection == "filesystems" => {
            if string_field(object, &["fsType", "type"]).as_deref() == Some("btrfs") {
                (
                    RiskClass::Online,
                    false,
                    Some(Advice {
                        summary: "Btrfs scrub verifies checksums and repairs redundant data online"
                            .to_string(),
                        alternatives: vec![
                            "run a read-only filesystem check when metadata corruption is suspected"
                                .to_string(),
                            "verify device health and backups before scrubbing degraded filesystems"
                                .to_string(),
                            "monitor scrub status until completion".to_string(),
                        ],
                    }),
                )
            } else {
                (
                    RiskClass::Unsupported,
                    false,
                    Some(Advice {
                        summary:
                            "filesystem scrub command mapping is currently available for Btrfs"
                                .to_string(),
                        alternatives: vec![
                            "use filesystem check for ext or XFS consistency validation"
                                .to_string(),
                            "model ZFS scrubs through pool lifecycle declarations".to_string(),
                            "run filesystem-specific scrub tooling manually after review"
                                .to_string(),
                        ],
                    }),
                )
            }
        }
        Operation::Trim if collection == "filesystems" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "filesystem trim discards unused blocks on the mounted filesystem"
                    .to_string(),
                alternatives: vec![
                    "verify discard passthrough on encrypted or virtual block layers first"
                        .to_string(),
                    "prefer scheduled fstrim for steady-state maintenance".to_string(),
                    "run trim outside latency-sensitive windows on thin or remote storage"
                        .to_string(),
                ],
            }),
        ),
        Operation::Remount if collection == "filesystems" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary:
                    "filesystem remount updates local mount options without rewriting data"
                        .to_string(),
                alternatives: vec![
                    "prefer remounting with reviewed options before unmounting a busy path"
                        .to_string(),
                    "persist long-lived option changes through NixOS fileSystems".to_string(),
                    "verify active services tolerate option changes such as ro, rw, or discard"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "partitions" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary:
                    "partition creation changes on-disk metadata and requires kernel reread coordination"
                        .to_string(),
                alternatives: vec![
                    "verify the target disk, free region, and partition table before applying"
                        .to_string(),
                    "prefer stable /dev/disk/by-id paths for disk selection".to_string(),
                    "run partprobe or reboot if the kernel cannot reread the table".to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "disks" || collection == "partitions" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary:
                    "partition-table rescan refreshes kernel disk and partition inventory"
                        .to_string(),
                alternatives: vec![
                    "use grow when partition geometry must change before the reread".to_string(),
                    "pause dependent consumers when the kernel cannot reread an active table"
                        .to_string(),
                    "verify stable by-id and by-partuuid paths after the rescan".to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "btrfsSubvolumes" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "Btrfs subvolume creation is reversible but changes namespace layout"
                    .to_string(),
                alternatives: vec![
                    "create the subvolume at an empty reviewed path".to_string(),
                    "prefer read-only snapshots or clones for migrations".to_string(),
                    "verify parent mount and qgroup policy before creation".to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "btrfsSubvolumes" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary:
                    "Btrfs subvolume rescan refreshes subvolume metadata and read-only state"
                        .to_string(),
                alternatives: vec![
                    "use property updates only when read-only enforcement must change"
                        .to_string(),
                    "inspect qgroup and snapshot relationships before destructive cleanup"
                        .to_string(),
                    "verify consumers still mount the intended subvolume path".to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "btrfsQgroups" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary:
                    "Btrfs qgroup rescan refreshes quota hierarchy, limits, and usage"
                        .to_string(),
                alternatives: vec![
                    "use limit property updates only when quota enforcement must change"
                        .to_string(),
                    "inspect qgroup usage before tightening referenced or exclusive limits"
                        .to_string(),
                    "verify quota accounting and subvolume relationships before qgroup removal"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "zvols" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "zvol creation allocates a block volume inside an existing ZFS pool"
                    .to_string(),
                alternatives: vec![
                    "verify pool free space and refreservation policy before creation".to_string(),
                    "use sparse volumes only when overcommit is intentional".to_string(),
                    "create consumers only after the zvol appears by stable /dev/zvol path"
                        .to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "zvols" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "zvol rescan refreshes ZFS volume properties and block graph state"
                    .to_string(),
                alternatives: vec![
                    "use grow only when volsize must change".to_string(),
                    "inspect dependent guests, LUNs, and filesystems before changing consumers"
                        .to_string(),
                    "snapshot or clone the zvol before destructive cleanup".to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "pools" => (
            RiskClass::Destructive,
            true,
            Some(Advice {
                summary: "ZFS pool creation writes pool labels to every selected device"
                    .to_string(),
                alternatives: vec![
                    "verify every vdev device is empty or fully backed up before creation"
                        .to_string(),
                    "import an existing pool instead of recreating it".to_string(),
                    "use stable /dev/disk/by-id paths and review redundancy layout before zpool create"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "datasets" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "ZFS dataset creation adds a filesystem namespace inside an existing pool"
                    .to_string(),
                alternatives: vec![
                    "verify parent dataset properties before creating children".to_string(),
                    "set mountpoint, quota, reservation, and encryption policy before use"
                        .to_string(),
                    "create snapshots or consumers only after the dataset appears in zfs list"
                        .to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "datasets" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "ZFS dataset rescan refreshes dataset properties, mounts, and graph state"
                    .to_string(),
                alternatives: vec![
                    "use property updates only when mountpoint, quota, or reservation policy must change"
                        .to_string(),
                    "inspect snapshots and clones before promote, rollback, or destroy work"
                        .to_string(),
                    "verify consumers still use the intended mounted dataset".to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "volumes" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "logical volume creation allocates a new volume inside an existing volume group"
                    .to_string(),
                alternatives: vec![
                    "verify volume group free extents before creating the logical volume"
                        .to_string(),
                    "use an explicit desired size and stable LV name".to_string(),
                    "create filesystems or mappings only after the LV path appears".to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "volumes" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "logical volume rescan refreshes LV attributes, size, and graph relationships"
                    .to_string(),
                alternatives: vec![
                    "use grow only when logical volume capacity must change".to_string(),
                    "use activate or deactivate only when LV visibility must change".to_string(),
                    "verify dependent filesystems or mappings after status refresh".to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "physicalVolumes" => (
            RiskClass::Destructive,
            true,
            Some(Advice {
                summary: "LVM physical volume creation writes PV metadata to the selected device"
                    .to_string(),
                alternatives: vec![
                    "inspect signatures and backups before pvcreate".to_string(),
                    "reuse an existing PV when preserving volume-group data".to_string(),
                    "add a new device to the VG instead of reinitializing an existing PV"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create | Operation::AddKey if collection == "luksKeyslots" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "adding a LUKS keyslot changes access to the encrypted container"
                    .to_string(),
                alternatives: vec![
                    "back up the LUKS header before enrolling new key material".to_string(),
                    "test the new key before removing any existing recovery key".to_string(),
                    "use an explicit keyslot only when site policy requires stable slot assignment"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create | Operation::ImportToken if collection == "luksTokens" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "importing a LUKS token changes automated unlock access".to_string(),
                alternatives: vec![
                    "back up the LUKS header before importing token metadata".to_string(),
                    "verify a recovery key or passphrase works before relying on the token"
                        .to_string(),
                    "test the token unlock path before removing older tokens".to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "thinPools" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "thin pool creation allocates a new LVM thin-pool data volume inside an existing volume group"
                    .to_string(),
                alternatives: vec![
                    "verify volume group free extents before creating the thin pool".to_string(),
                    "choose explicit pool size and monitor metadata utilization from first use"
                        .to_string(),
                    "review thin-volume overcommit policy before exposing consumers".to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "thinPools" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "thin pool rescan refreshes data, metadata, monitoring, and graph status"
                    .to_string(),
                alternatives: vec![
                    "use grow only when data or metadata capacity must change".to_string(),
                    "verify data and metadata utilization before creating more thin volumes"
                        .to_string(),
                    "review autoextend and monitoring policy before pool exhaustion".to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "lvmCaches" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary:
                    "LVM cache attachment converts an origin LV to use a reviewed cache pool"
                        .to_string(),
                alternatives: vec![
                    "attach cache only after the cache pool LV and origin LV are both verified"
                        .to_string(),
                    "use writethrough mode first when data safety is more important than write latency"
                        .to_string(),
                    "snapshot or back up the origin LV before enabling writeback cache".to_string(),
                ],
            }),
        ),
        Operation::AddDevice if collection == "lvmCaches" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary:
                    "LVM cache attachment changes origin LV write paths through a cache pool"
                        .to_string(),
                alternatives: vec![
                    "verify the cache pool LV belongs to the same volume group as the origin"
                        .to_string(),
                    "start in writethrough mode when rollback safety matters".to_string(),
                    "keep the origin LV snapshot or backup until cache verification passes"
                        .to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "lvmCaches" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "LVM cache rescan refreshes origin, cache mode, policy, and dirty-data reports"
                    .to_string(),
                alternatives: vec![
                    "use property updates when cache mode or cache policy must change".to_string(),
                    "use remove-device only after dirty cache data has drained".to_string(),
                    "verify origin LV readability before any later cache detach or replacement"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "volumeGroups" => (
            RiskClass::Destructive,
            true,
            Some(Advice {
                summary: "volume group creation writes LVM metadata to the selected physical volume"
                    .to_string(),
                alternatives: vec![
                    "verify the physical volume contains no data that must be preserved"
                        .to_string(),
                    "extend an existing volume group when preserving consumers is possible"
                        .to_string(),
                    "use stable /dev/disk/by-id paths and inspect pvs before vgcreate".to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "vdoVolumes" => (
            RiskClass::Destructive,
            true,
            Some(Advice {
                summary: "VDO volume creation writes VDO metadata to the selected backing device"
                    .to_string(),
                alternatives: vec![
                    "verify the backing device identity and existing signatures before creation"
                        .to_string(),
                    "grow or migrate an existing VDO volume when preserving data is required"
                        .to_string(),
                    "choose logical size, compression, and deduplication policy before use"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "mdRaids" => (
            RiskClass::Destructive,
            true,
            Some(Advice {
                summary: "creating an MD RAID array writes array metadata to every member device"
                    .to_string(),
                alternatives: vec![
                    "verify every member device is empty or fully backed up before creation"
                        .to_string(),
                    "assemble and inspect an existing array instead of recreating it".to_string(),
                    "add replacement members to an existing redundant array when preserving data"
                        .to_string(),
                ],
            }),
        ),
        Operation::Assemble if collection == "mdRaids" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "MD RAID assemble activates an existing array from reviewed member devices"
                    .to_string(),
                alternatives: vec![
                    "assemble existing arrays instead of recreating them when metadata already exists"
                        .to_string(),
                    "verify member identities and event counts with mdadm --examine before assemble"
                        .to_string(),
                    "mount or activate consumers only after mdadm reports the array clean or recovering"
                        .to_string(),
                ],
            }),
        ),
        Operation::Stop if collection == "mdRaids" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "MD RAID stop makes the array unavailable without removing member metadata"
                    .to_string(),
                alternatives: vec![
                    "unmount filesystems and deactivate mappings before stopping the array"
                        .to_string(),
                    "prefer stop over destroy when preserving member metadata for later assembly"
                        .to_string(),
                    "verify no open consumers remain with lsblk, findmnt, and dmsetup before stopping"
                        .to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "mdRaids" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary:
                    "MD RAID rescan refreshes array metadata inventory without assembling arrays"
                        .to_string(),
                alternatives: vec![
                    "use assemble when existing member metadata should activate an array"
                        .to_string(),
                    "inspect member event counts with mdadm --examine before assembly or replacement"
                        .to_string(),
                    "verify /proc/mdstat and dependent consumers after devices reappear"
                        .to_string(),
                ],
            }),
        ),
        Operation::Start if collection == "vdoVolumes" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "VDO start activates an existing VDO volume without rewriting metadata"
                    .to_string(),
                alternatives: vec![
                    "verify the VDO backing device is present before starting".to_string(),
                    "start dependent filesystems, LVM layers, or mounts only after VDO status is healthy"
                        .to_string(),
                    "use create only when intentionally initializing new VDO metadata".to_string(),
                ],
            }),
        ),
        Operation::Stop if collection == "vdoVolumes" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "VDO stop deactivates the volume while preserving VDO metadata".to_string(),
                alternatives: vec![
                    "unmount filesystems and deactivate mappings before stopping VDO".to_string(),
                    "prefer stop over remove when the VDO volume should be started again later"
                        .to_string(),
                    "verify no open consumers remain with lsblk, findmnt, and dmsetup before stopping"
                        .to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "vdoVolumes" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "VDO rescan refreshes status, utilization, and graph inventory"
                    .to_string(),
                alternatives: vec![
                    "use grow when logical or physical VDO capacity must change".to_string(),
                    "use start or stop only when intentionally changing activation state"
                        .to_string(),
                    "verify vdostats before growing filesystems or dependent volumes"
                    .to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "caches" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "bcache rescan refreshes cache state, dirty-data, and graph inventory"
                    .to_string(),
                alternatives: vec![
                    "use add-device or remove-device only when cache-set attachment must change"
                        .to_string(),
                    "verify dirty data is zero before any later detach or replacement".to_string(),
                    "use cache property updates when changing cache mode or writeback behavior"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "loopDevices" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "loop device creation maps an existing backing file or block device"
                    .to_string(),
                alternatives: vec![
                    "use a stable backing file path and explicit loop device name when needed"
                        .to_string(),
                    "verify the backing file is not concurrently managed elsewhere".to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "loopDevices" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "loop device rescan refreshes mapping inventory without changing size"
                    .to_string(),
                alternatives: vec![
                    "use grow only after the backing file or block device size has changed"
                        .to_string(),
                    "inspect dependent filesystems and mappings before detach".to_string(),
                    "keep stable /dev/loop* targets for executable refresh plans".to_string(),
                ],
            }),
        ),
        Operation::Create | Operation::Export if collection == "exports" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "NFS export creation publishes an existing path to selected clients"
                    .to_string(),
                alternatives: vec![
                    "export read-only first when client behavior is unknown".to_string(),
                    "restrict clients and options before enabling write access".to_string(),
                    "verify the source path and ownership before reloading exports".to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "exports" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "NFS export rescan refreshes exported path and client visibility"
                    .to_string(),
                alternatives: vec![
                    "use option property updates only when client access semantics must change"
                        .to_string(),
                    "verify active clients before unexporting or tightening access".to_string(),
                    "persist long-lived exports through NixOS services.nfs.server.exports"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create | Operation::Login if collection == "iscsiSessions" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "iSCSI session login discovers target portals and attaches remote LUNs"
                    .to_string(),
                alternatives: vec![
                    "verify the portal and target IQN before logging in".to_string(),
                    "prefer stable multipath and by-id consumers before resizing filesystems"
                        .to_string(),
                    "keep NixOS open-iscsi session declarations aligned with imperative login"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create | Operation::Mount if collection == "nfs.mounts" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "mounting an NFS client path changes host namespace state".to_string(),
                alternatives: vec![
                    "use x-systemd.automount or nofail when the server may be unavailable"
                        .to_string(),
                    "verify DNS, routing, firewall, and export permissions before mounting"
                        .to_string(),
                    "prefer declarative NixOS fileSystems for steady-state client mounts"
                        .to_string(),
                ],
            }),
        ),
        Operation::Mount if collection == "filesystems" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "mounting a filesystem changes local namespace state without formatting storage"
                    .to_string(),
                alternatives: vec![
                    "verify the source device, filesystem type, and mountpoint before mounting"
                        .to_string(),
                    "prefer x-systemd.automount, nofail, or service ordering when dependencies may be unavailable"
                        .to_string(),
                    "persist long-lived mounts through the matching NixOS fileSystems entry"
                        .to_string(),
                ],
            }),
        ),
        Operation::Remount if collection == "nfs.mounts" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "remounting an NFS client path updates local mount options without deleting remote data"
                    .to_string(),
                alternatives: vec![
                    "prefer remounting with reviewed options before unmounting a busy path"
                        .to_string(),
                    "use NixOS fileSystems for the steady-state mount options".to_string(),
                    "verify active services tolerate option changes such as ro, rw, or timeouts"
                        .to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "nfs.mounts" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "NFS mount rescan refreshes local mount source, options, and client stats"
                    .to_string(),
                alternatives: vec![
                    "use remount only when local mount options must change".to_string(),
                    "verify server reachability before unmounting busy client paths".to_string(),
                    "persist long-lived mounts through NixOS fileSystems".to_string(),
                ],
            }),
        ),
        Operation::Create | Operation::Attach if collection == "luns" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "LUN host attach makes an existing target-side LUN visible to this host"
                    .to_string(),
                alternatives: vec![
                    "create or grow the target-side LUN before host attach".to_string(),
                    "declare stable by-path devices so apply can verify every expected path"
                        .to_string(),
                    "keep multipath and filesystem consumers disabled until paths are verified"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "nvmeNamespaces" => (
            RiskClass::Destructive,
            true,
            Some(Advice {
                summary: "NVMe namespace creation allocates controller-managed namespace capacity"
                    .to_string(),
                alternatives: vec![
                    "attach an existing namespace when preserving data is required".to_string(),
                    "verify controller namespace inventory before create-ns".to_string(),
                    "declare namespaceId and controllers before attaching the created namespace"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create | Operation::SetProperty => (RiskClass::Safe, false, None),
        Operation::Clone => (
            RiskClass::Reversible,
            false,
            Some(Advice {
                summary: format!("{collection} clone creates a dependent writable copy"),
                alternatives: vec![
                    "inspect the clone before using it for rollback or migration".to_string(),
                    "destroy temporary clones after validation".to_string(),
                ],
            }),
        ),
        Operation::Promote => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: format!("{collection} promote makes a clone independent of its origin"),
                alternatives: vec![
                    "inspect origin and dependent snapshots before promoting".to_string(),
                    "validate mounts, shares, LUN mappings, and services against the promoted clone"
                        .to_string(),
                    "keep the original dataset until the promoted clone is verified".to_string(),
                ],
            }),
        ),
        Operation::Import if collection == "pools" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "ZFS pool import makes an existing pool active on this host".to_string(),
                alternatives: vec![
                    "import read-only first when validating a moved or recovered pool".to_string(),
                    "verify hostid, cachefile, mountpoints, and encryption keys before import"
                        .to_string(),
                    "prefer import over recreating a pool when preserving data".to_string(),
                ],
            }),
        ),
        Operation::Export if collection == "pools" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "ZFS pool export cleanly detaches a pool without deleting data".to_string(),
                alternatives: vec![
                    "export a pool instead of destroying it when moving hosts".to_string(),
                    "stop mounts, shares, LUN exports, and services before export".to_string(),
                    "verify all writes are complete and pool health is reviewed first".to_string(),
                ],
            }),
        ),
        Operation::Import if collection == "volumeGroups" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "LVM volume group import reactivates an exported VG on this host"
                    .to_string(),
                alternatives: vec![
                    "inspect PV identities and VG UUIDs before vgimport".to_string(),
                    "prefer vgimport over vgcreate when preserving existing logical volumes"
                        .to_string(),
                    "activate and mount consumers only after the imported VG is verified"
                        .to_string(),
                ],
            }),
        ),
        Operation::Export if collection == "volumeGroups" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "LVM volume group export marks a VG inactive for movement without deleting data"
                    .to_string(),
                alternatives: vec![
                    "export a VG instead of removing it when moving disks between hosts"
                        .to_string(),
                    "deactivate logical volumes and stop mounts or services before vgexport"
                        .to_string(),
                    "verify metadata backups before changing VG activation state".to_string(),
                ],
            }),
        ),
        Operation::Activate
            if collection == "volumes"
                || collection == "thinPools"
                || collection == "lvmSnapshots"
                || collection == "volumeGroups" =>
        {
            (
                RiskClass::OfflineRequired,
                false,
                Some(Advice {
                    summary: format!(
                        "{collection} activation makes an existing LVM object available without creating it"
                    ),
                    alternatives: vec![
                        "inspect LVM metadata and dependent mappings before activation"
                            .to_string(),
                        "activate only the reviewed VG or LV needed for consumers".to_string(),
                        "verify filesystems, mounts, and services after activation".to_string(),
                    ],
                }),
            )
        }
        Operation::Deactivate
            if collection == "volumes"
                || collection == "thinPools"
                || collection == "lvmSnapshots"
                || collection == "volumeGroups" =>
        {
            (
                RiskClass::OfflineRequired,
                false,
                Some(Advice {
                    summary: format!(
                        "{collection} deactivation makes an existing LVM object unavailable without deleting it"
                    ),
                    alternatives: vec![
                        "unmount filesystems and stop services before deactivation".to_string(),
                        "deactivate instead of removing storage when preserving data".to_string(),
                        "verify no dm, filesystem, LUN, or service consumers remain active"
                            .to_string(),
                    ],
                }),
            )
        }
        Operation::Rescan if collection == "lvmSnapshots" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "LVM snapshot rescan refreshes origin, COW usage, and graph status"
                    .to_string(),
                alternatives: vec![
                    "merge only after inspecting the snapshot contents and origin state"
                        .to_string(),
                    "activate the snapshot for recovery inspection instead of removing it"
                        .to_string(),
                    "verify snapshot fullness before relying on it as a recovery point"
                        .to_string(),
                ],
            }),
        ),
        Operation::Rename => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: format!("{collection} rename retargets a storage object without deleting it"),
                alternatives: vec![
                    "rename first and validate consumers before destroying old paths".to_string(),
                    "update mounts, exports, LUN mappings, and services before applying".to_string(),
                    "keep snapshots or backups until consumers use the renamed object".to_string(),
                ],
            }),
        ),
        Operation::Grow if collection == "mdRaids" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "MD RAID grow or reshape requires redundancy, bitmap, and resync coordination"
                    .to_string(),
                alternatives: vec![
                    "add replacement members and wait for sync before reshaping".to_string(),
                    "verify backups and array health before changing size or member count"
                        .to_string(),
                    "monitor /proc/mdstat until reshape and filesystem growth are complete"
                        .to_string(),
                ],
            }),
        ),
        Operation::Grow if collection == "multipathMaps" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "multipath map growth requires path rescan and map resize coordination"
                    .to_string(),
                alternatives: vec![
                    "rescan every backing SCSI path before resizing the map".to_string(),
                    "verify all expected paths are active before growing consumers".to_string(),
                    "reload multipath maps and confirm no stale path reports the old size".to_string(),
                ],
            }),
        ),
        Operation::Grow if collection == "thinPools" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary:
                    "LVM thin pool growth must account for both data and metadata usage"
                        .to_string(),
                alternatives: vec![
                    "extend thin pool metadata before data exhaustion".to_string(),
                    "verify autoextend thresholds and monitored status before growth".to_string(),
                    "review thin volume overcommit before adding more virtual capacity"
                        .to_string(),
                ],
            }),
        ),
        Operation::Scrub if collection == "pools" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "pool scrub verifies data and repairs redundant copies online".to_string(),
                alternatives: vec![
                    "review pool health before starting a scrub".to_string(),
                    "schedule scrubs outside latency-sensitive windows".to_string(),
                    "monitor scrub, resilver, or repair status until completion".to_string(),
                ],
            }),
        ),
        Operation::Grow if collection == "zvols" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "zvol growth updates volsize and requires consumer capacity verification"
                    .to_string(),
                alternatives: vec![
                    "verify pool free space before increasing volsize".to_string(),
                    "rescan dependent guests, LUNs, or filesystems after growth".to_string(),
                    "grow dependent partitions and filesystems only after the zvol reports the new size"
                        .to_string(),
                ],
            }),
        ),
        Operation::Rescan
            if collection == "luns"
                || collection == "iscsiSessions"
                || collection == "nvmeNamespaces"
                || collection == "multipathMaps" =>
        {
            (
                RiskClass::Online,
                false,
                Some(Advice {
                    summary:
                        "host rescan refreshes existing storage paths without deleting target data"
                            .to_string(),
                    alternatives: vec![
                        "use grow when the target-side capacity changed and consumers must be resized"
                            .to_string(),
                        "declare stable path devices so apply can verify each refreshed path"
                            .to_string(),
                        "verify multipath and dependent volumes after the rescan".to_string(),
                    ],
                }),
            )
        }
        Operation::Rescan if collection == "physicalVolumes" || collection == "volumeGroups" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary:
                    "LVM rescan refreshes PV cache and VG metadata without deleting data"
                        .to_string(),
                alternatives: vec![
                    "use grow when backing device capacity changed and PV or LV sizes must be updated"
                        .to_string(),
                    "rescan block paths before refreshing LVM metadata on newly visible devices"
                        .to_string(),
                    "verify VG free extents and LV activation state after the metadata refresh"
                        .to_string(),
                ],
            }),
        ),
        Operation::Grow if collection == "luns" || collection == "iscsiSessions" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary:
                    "network LUN growth must be coordinated with the storage target and host rescan"
                        .to_string(),
                alternatives: vec![
                    "grow the target LUN before resizing consumers".to_string(),
                    "rescan SCSI paths and verify multipath before filesystem growth".to_string(),
                    "confirm every dependent filesystem or volume sees the new capacity"
                        .to_string(),
                ],
            }),
        ),
        Operation::Grow if collection == "partitions" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "partition growth may require inactive consumers and a kernel partition table reread"
                    .to_string(),
                alternatives: vec![
                    "grow the backing disk or LUN before resizing the partition".to_string(),
                    "verify dependent LUKS, LVM, and filesystem layers before resizing consumers"
                        .to_string(),
                    "schedule a reboot when active consumers prevent partition table reread"
                        .to_string(),
                ],
            }),
        ),
        Operation::Grow if collection == "vdoVolumes" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "VDO growth must distinguish logical size from backing physical capacity"
                    .to_string(),
                alternatives: vec![
                    "grow physical backing storage before VDO physical growth".to_string(),
                    "grow logical size only after confirming pool utilization and slab health"
                        .to_string(),
                    "verify vdostats and dependent filesystems after the grow".to_string(),
                ],
            }),
        ),
        Operation::Grow if collection == "physicalVolumes" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "LVM physical volume growth refreshes PV size after backing storage grows"
                    .to_string(),
                alternatives: vec![
                    "grow the backing partition, LUN, or disk before pvresize".to_string(),
                    "verify VG free extents before extending logical volumes".to_string(),
                    "coordinate dependent LV and filesystem growth after pvresize".to_string(),
                ],
            }),
        ),
        Operation::Grow if collection == "loopDevices" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "loop device growth refreshes the mapping after backing size changes"
                    .to_string(),
                alternatives: vec![
                    "grow the backing file or block device before refreshing the loop mapping"
                        .to_string(),
                    "resize dependent partitions or filesystems only after losetup reports the new size"
                        .to_string(),
                ],
            }),
        ),
        Operation::Grow if collection == "nvmeNamespaces" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "NVMe namespace growth is represented as host rescan after controller-side changes"
                    .to_string(),
                alternatives: vec![
                    "resize or recreate the namespace on the controller before host rescan"
                        .to_string(),
                    "rescan the controller and verify namespace capacity before growing consumers"
                        .to_string(),
                    "prefer replacement namespace migration when controller resize is unsupported"
                        .to_string(),
                ],
            }),
        ),
        Operation::Grow | Operation::AddDevice | Operation::Rebalance => {
            (RiskClass::Online, false, None)
        }
        Operation::ReplaceDevice => {
            let (risk, advice) = classify_replace_device(collection);
            (risk, false, Some(advice))
        }
        Operation::Snapshot => (RiskClass::Reversible, false, None),
        Operation::Destroy if collection == "loopDevices" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "detaching a loop device requires consumers to be unmounted or stopped"
                    .to_string(),
                alternatives: vec![
                    "unmount filesystems and deactivate mappings before detach".to_string(),
                    "keep the backing file intact and recreate the loop mapping after validation"
                        .to_string(),
                ],
            }),
        ),
        Operation::Destroy | Operation::Unexport if collection == "exports" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "unexporting NFS paths can interrupt active remote clients".to_string(),
                alternatives: vec![
                    "remove or migrate clients before unexporting the path".to_string(),
                    "switch export options to read-only before final removal".to_string(),
                    "verify no active mounts depend on the export before reload".to_string(),
                ],
            }),
        ),
        Operation::Destroy | Operation::Unmount if collection == "nfs.mounts" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "unmounting an NFS client path can interrupt local services".to_string(),
                alternatives: vec![
                    "stop local services and automount units before unmounting".to_string(),
                    "switch the mount to read-only or noauto before final removal".to_string(),
                    "verify no open files or bind mounts still depend on the mountpoint"
                        .to_string(),
                ],
            }),
        ),
        Operation::Unmount if collection == "filesystems" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "unmounting a filesystem can interrupt local services without deleting data"
                    .to_string(),
                alternatives: vec![
                    "stop dependent services, automount units, user sessions, and bind mounts before unmounting"
                        .to_string(),
                    "switch the mount to read-only or noauto first when a staged removal is safer"
                        .to_string(),
                    "verify no open files still reference the mountpoint before applying"
                        .to_string(),
                ],
            }),
        ),
        Operation::Destroy | Operation::Logout if collection == "iscsiSessions" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "iSCSI logout detaches remote LUN paths from the host".to_string(),
                alternatives: vec![
                    "unmount filesystems and deactivate mappings before logout".to_string(),
                    "verify multipath, LVM, and filesystem consumers have migrated away"
                        .to_string(),
                    "disable automatic login only after dependent services no longer need the LUN"
                        .to_string(),
                ],
            }),
        ),
        Operation::Destroy | Operation::Detach if collection == "luns" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "LUN host detach removes reviewed SCSI paths from this host".to_string(),
                alternatives: vec![
                    "unmount filesystems and deactivate LVM, multipath, or dm consumers before detach"
                        .to_string(),
                    "remove a single path only after redundancy or alternate paths are healthy"
                        .to_string(),
                    "disable automatic session login only after dependent services no longer need the LUN"
                        .to_string(),
                ],
            }),
        ),
        Operation::Destroy if collection == "lvmCaches" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "LVM cache removal must flush dirty cache state before uncaching"
                    .to_string(),
                alternatives: vec![
                    "switch to writethrough and wait for dirty blocks to drain before lvconvert --uncache"
                        .to_string(),
                    "verify the origin LV is readable without the cache before removing cache media"
                        .to_string(),
                    "keep the cache pool intact until post-uncache verification passes".to_string(),
                ],
            }),
        ),
        Operation::Destroy | Operation::RemoveKey if collection == "luksKeyslots" => (
            RiskClass::PotentialDataLoss,
            false,
            Some(Advice {
                summary: "removing a LUKS keyslot can lock out encrypted data if no other key works"
                    .to_string(),
                alternatives: vec![
                    "verify another passphrase, key file, or token unlocks the device first".to_string(),
                    "take a LUKS header backup before keyslot removal".to_string(),
                    "add and test a replacement keyslot before killing the old slot".to_string(),
                ],
            }),
        ),
        Operation::Destroy | Operation::RemoveToken if collection == "luksTokens" => (
            RiskClass::PotentialDataLoss,
            false,
            Some(Advice {
                summary: "removing a LUKS token can lock out automated unlock".to_string(),
                alternatives: vec![
                    "verify another token, keyslot, or passphrase unlocks the device first"
                        .to_string(),
                    "take a LUKS header backup before token removal".to_string(),
                    "import and test a replacement token before removing the old token".to_string(),
                ],
            }),
        ),
        Operation::Destroy if collection == "physicalVolumes" => (
            RiskClass::Destructive,
            true,
            Some(Advice {
                summary: "LVM physical volume removal erases PV metadata from the device"
                    .to_string(),
                alternatives: vec![
                    "pvmove allocated extents and vgreduce the PV before pvremove".to_string(),
                    "verify no volume group still depends on the PV".to_string(),
                    "preserve the device for recovery until backups are verified".to_string(),
                ],
            }),
        ),
        Operation::Destroy if collection == "nvmeNamespaces" => (
            RiskClass::Destructive,
            true,
            Some(Advice {
                summary: "NVMe namespace deletion removes the namespace from the controller"
                    .to_string(),
                alternatives: vec![
                    "detach the namespace from selected controllers before deletion".to_string(),
                    "migrate or snapshot data before deleting the namespace".to_string(),
                    "use host detach or rescan workflows when target-side data should remain"
                        .to_string(),
                ],
            }),
        ),
        Operation::Rollback if collection == "lvmSnapshots" => (
            RiskClass::PotentialDataLoss,
            false,
            Some(Advice {
                summary: "merging an LVM snapshot rolls the origin back to older contents"
                    .to_string(),
                alternatives: vec![
                    "take a fresh snapshot of the current origin before merge".to_string(),
                    "mount or clone the snapshot for inspection before rollback".to_string(),
                    "schedule downtime when the origin must be deactivated for merge".to_string(),
                ],
            }),
        ),
        Operation::Import | Operation::Export => (
            RiskClass::Unsupported,
            false,
            Some(Advice {
                summary: format!(
                    "{} is currently only supported for ZFS pools, LVM volume groups, and NFS exports",
                    operation_label(operation)
                ),
                alternatives: vec![
                    "use pools.<name>.operation for ZFS pool import or export".to_string(),
                    "use volumeGroups.<name>.operation for LVM VG import or export".to_string(),
                    "use exports.<path>.operation = \"export\" for NFS export publication"
                        .to_string(),
                    "use domain-specific attach, detach, mount, or unmount operations where available"
                        .to_string(),
                ],
            }),
        ),
        Operation::Unexport => (
            RiskClass::Unsupported,
            false,
            Some(Advice {
                summary: "unexport operations are currently only supported for exports".to_string(),
                alternatives: vec![
                    "use operation = \"unexport\" on exports declarations for NFS server export lifecycle"
                        .to_string(),
                    "use operation = \"unmount\" on nfs.mounts declarations for NFS client mounts"
                        .to_string(),
                    "use destroy only where a storage domain has not yet gained explicit lifecycle verbs"
                        .to_string(),
                ],
            }),
        ),
        Operation::Attach | Operation::Detach => (
            RiskClass::Unsupported,
            false,
            Some(Advice {
                summary: format!(
                    "{} operations are currently only supported for luns",
                    operation_label(operation)
                ),
                alternatives: vec![
                    "use operation = \"attach\" or \"detach\" on luns declarations for host-side LUN path lifecycle"
                        .to_string(),
                    "use operation = \"login\" or \"logout\" on iscsiSessions declarations for target session lifecycle"
                        .to_string(),
                    "use domain-specific add-device, remove-device, mount, unmount, import, or export operations where available"
                        .to_string(),
                ],
            }),
        ),
        Operation::Activate | Operation::Deactivate => (
            RiskClass::Unsupported,
            false,
            Some(Advice {
                summary: format!(
                    "{} is currently only supported for LVM volumes, thin pools, snapshots, and volume groups",
                    operation_label(operation)
                ),
                alternatives: vec![
                    "use volumes, thinPools, lvmSnapshots, or volumeGroups for LVM activation lifecycle"
                        .to_string(),
                    "use mount, login, attach, or import operations for non-LVM domains where available"
                        .to_string(),
                ],
            }),
        ),
        Operation::Assemble | Operation::Start | Operation::Stop => (
            RiskClass::Unsupported,
            false,
            Some(Advice {
                summary: format!(
                    "{} operations are not implemented for {collection}",
                    operation_label(operation)
                ),
                alternatives: vec![
                    "use operation = \"assemble\" only on mdRaids declarations for now".to_string(),
                    "use operation = \"start\" or \"stop\" on vdoVolumes declarations for VDO activation lifecycle"
                        .to_string(),
                    "use subsystem-specific import, export, activate, or deactivate operations where supported"
                        .to_string(),
                ],
            }),
        ),
        Operation::Login | Operation::Logout => (
            RiskClass::Unsupported,
            false,
            Some(Advice {
                summary: format!(
                    "{} operations are currently only supported for iscsiSessions",
                    operation_label(operation)
                ),
                alternatives: vec![
                    "use operation = \"login\" or \"logout\" on iscsiSessions declarations for iSCSI session lifecycle"
                        .to_string(),
                    "use create/destroy only where a storage domain has not yet gained explicit lifecycle verbs"
                        .to_string(),
                ],
            }),
        ),
        Operation::Mount | Operation::Unmount => (
            RiskClass::Unsupported,
            false,
            Some(Advice {
                summary: format!(
                    "{} operations are currently only supported for filesystems and nfs.mounts",
                    operation_label(operation)
                ),
                alternatives: vec![
                    "use operation = \"mount\" or \"unmount\" on filesystems declarations for local filesystem mount lifecycle"
                        .to_string(),
                    "use operation = \"mount\" or \"unmount\" on nfs.mounts declarations for NFS client mount lifecycle"
                        .to_string(),
                    "use service or automount-specific workflows for domains outside the modeled mount collections"
                        .to_string(),
                ],
            }),
        ),
        Operation::Open | Operation::Close => (
            RiskClass::Unsupported,
            false,
            Some(Advice {
                summary: format!(
                    "{} operations are currently only supported for luks.devices",
                    operation_label(operation)
                ),
                alternatives: vec![
                    "use luks.devices.<name>.operation for encrypted mapper open or close"
                        .to_string(),
                    "use activate, deactivate, import, export, mount, or remount for other storage domains"
                        .to_string(),
                ],
            }),
        ),
        Operation::Rescan => (
            RiskClass::Unsupported,
            false,
            Some(Advice {
                summary: "rescan operations are currently supported for disks, partitions, snapshots, LUNs, iSCSI sessions, NFS exports/mounts, NVMe namespaces, multipath maps, loop devices, ZFS datasets/zvols, Btrfs subvolumes/qgroups, LVM PV/VG/LV/snapshot/cache/thin-pool metadata, MD RAID metadata, VDO status, and bcache status"
                    .to_string(),
                alternatives: vec![
                    "use disks.<path>.operation = \"rescan\" to reread a partition table"
                        .to_string(),
                    "use partitions.<name>.operation = \"rescan\" to refresh a reviewed backing disk"
                        .to_string(),
                    "use luns.<name>.operation = \"rescan\" to refresh reviewed SCSI paths"
                        .to_string(),
                    "use iscsiSessions.<target>.operation = \"rescan\" to refresh existing target sessions"
                        .to_string(),
                    "use exports.<path>.operation = \"rescan\" to refresh NFS export inventory"
                        .to_string(),
                    "use nfs.mounts.<mountpoint>.operation = \"rescan\" to refresh NFS client mount state"
                        .to_string(),
                    "use nvmeNamespaces.<controller>.operation = \"rescan\" to refresh namespace inventory"
                        .to_string(),
                    "use multipathMaps.<name>.operation = \"rescan\" to reload reviewed path maps"
                        .to_string(),
                    "use loopDevices.<path>.operation = \"rescan\" to refresh loop mapping inventory"
                        .to_string(),
                    "use physicalVolumes or volumeGroups operation = \"rescan\" to refresh LVM metadata"
                        .to_string(),
                    "use volumes.<vg/lv>.operation = \"rescan\" to refresh LVM logical volume status"
                        .to_string(),
                    "use lvmCaches.<origin>.operation = \"rescan\" to refresh LVM cache status and utilization"
                        .to_string(),
                    "use thinPools.<pool>.operation = \"rescan\" to refresh LVM thin-pool utilization"
                        .to_string(),
                    "use lvmSnapshots.<vg/lv>.operation = \"rescan\" to refresh LVM snapshot status"
                        .to_string(),
                    "use snapshots.<name>.operation = \"rescan\" to refresh snapshot metadata and holds"
                        .to_string(),
                    "use btrfsSubvolumes.<path>.operation = \"rescan\" to refresh subvolume metadata and read-only state"
                        .to_string(),
                    "use datasets.<name>.operation = \"rescan\" to refresh ZFS dataset properties and graph state"
                        .to_string(),
                    "use zvols.<name>.operation = \"rescan\" to refresh ZFS volume properties and block graph state"
                        .to_string(),
                    "use mdRaids.<name>.operation = \"rescan\" to refresh MD RAID metadata inventory"
                        .to_string(),
                    "use vdoVolumes.<name>.operation = \"rescan\" to refresh VDO status and utilization"
                        .to_string(),
                    "use caches.<device>.operation = \"rescan\" to refresh bcache state and dirty-data counters"
                        .to_string(),
                    "use btrfsQgroups.<id>.operation = \"rescan\" with target = <mountpoint> to refresh quota hierarchy and usage"
                        .to_string(),
                ],
            }),
        ),
        Operation::AddKey | Operation::RemoveKey => (
            RiskClass::Unsupported,
            false,
            Some(Advice {
                summary: format!(
                    "{} operations are currently only supported for luksKeyslots",
                    operation_label(operation)
                ),
                alternatives: vec![
                    "use luksKeyslots.<name>.operation for LUKS keyslot add or remove lifecycle"
                        .to_string(),
                    "use luks.devices.<name>.operation for encrypted mapper open or close"
                        .to_string(),
                    "use set-property for LUKS label, UUID, or key rotation updates".to_string(),
                ],
            }),
        ),
        Operation::ImportToken | Operation::RemoveToken => (
            RiskClass::Unsupported,
            false,
            Some(Advice {
                summary: format!(
                    "{} operations are currently only supported for luksTokens",
                    operation_label(operation)
                ),
                alternatives: vec![
                    "use luksTokens.<name>.operation for LUKS token import or remove lifecycle"
                        .to_string(),
                    "verify a fallback keyslot or recovery passphrase before changing tokens"
                        .to_string(),
                    "use luksKeyslots declarations when changing passphrase/key-file access"
                        .to_string(),
                ],
            }),
        ),
        Operation::Remount => (
            RiskClass::Unsupported,
            false,
            Some(Advice {
                summary: format!(
                    "{} operations are not implemented for {collection}",
                    operation_label(operation)
                ),
                alternatives: vec![
                    "use operation = \"remount\" on filesystems or nfs.mounts declarations"
                        .to_string(),
                    "use a filesystem-specific mount or service restart workflow for other remount needs"
                        .to_string(),
                ],
            }),
        ),
        Operation::Shrink
        | Operation::Check
        | Operation::Repair
        | Operation::Scrub
        | Operation::Trim
        | Operation::RemoveDevice
        | Operation::Rollback => (
            RiskClass::PotentialDataLoss,
            false,
            Some(Advice {
                summary: format!(
                    "{} can require evacuation, rollback, or offline validation",
                    operation_label(operation)
                ),
                alternatives: vec![
                    "prefer grow, add, replace, or clone operations where possible".to_string(),
                    "verify backups and health before applying".to_string(),
                    "stage the change against a clone or replacement target first".to_string(),
                ],
            }),
        ),
        Operation::Format | Operation::Destroy => (
            RiskClass::Destructive,
            true,
            Some(Advice {
                summary: format!(
                    "{} on {collection} removes or overwrites existing storage",
                    operation_label(operation)
                ),
                alternatives: destructive_alternatives(collection, object),
            }),
        ),
    }
}

fn classify_replace_device(collection: &str) -> (RiskClass, Advice) {
    if collection == "caches" || collection == "lvmCaches" {
        (
            RiskClass::OfflineRequired,
            Advice {
                summary: "cache replacement must account for dirty or writeback data".to_string(),
                alternatives: vec![
                    "flush dirty data before replacing the cache device".to_string(),
                    "detach or disable writeback caching before removing the source".to_string(),
                    "verify the origin or backing volume before re-enabling the cache".to_string(),
                ],
            },
        )
    } else if collection == "mdRaids" {
        (
            RiskClass::OfflineRequired,
            Advice {
                summary:
                    "MD RAID replacement must preserve redundancy through fail, add, and resync"
                        .to_string(),
                alternatives: vec![
                    "add a spare and wait for sync before failing the old member".to_string(),
                    "replace one member at a time while the array is healthy".to_string(),
                    "verify /proc/mdstat and mdadm --detail before removing the old device"
                        .to_string(),
                ],
            },
        )
    } else if collection == "multipathMaps" {
        (
            RiskClass::OfflineRequired,
            Advice {
                summary: "multipath path replacement must preserve live path redundancy"
                    .to_string(),
                alternatives: vec![
                    "add and verify the replacement path before deleting the old path".to_string(),
                    "fail or disable one path at a time while other paths remain active"
                        .to_string(),
                    "reload maps only after every expected path is visible".to_string(),
                ],
            },
        )
    } else if collection == "volumeGroups" {
        (
            RiskClass::OfflineRequired,
            Advice {
                summary: "LVM physical volume replacement must migrate extents before vgreduce"
                    .to_string(),
                alternatives: vec![
                    "vgextend the replacement PV before running pvmove".to_string(),
                    "keep the old PV available until pvmove completes and LVs are verified"
                        .to_string(),
                    "use pvs and vgs reports to confirm no allocated extents remain before vgreduce"
                        .to_string(),
                ],
            },
        )
    } else {
        (
            RiskClass::Reversible,
            Advice {
                summary: "replacement should preserve data when the source remains available"
                    .to_string(),
                alternatives: vec![
                    "attach the replacement and resilver or rebalance before detaching the source"
                        .to_string(),
                    "keep the original device untouched until post-apply verification passes"
                        .to_string(),
                ],
            },
        )
    }
}

fn classify_add_device(collection: &str) -> (RiskClass, Option<Advice>) {
    if collection == "lvmCaches" {
        (
            RiskClass::OfflineRequired,
            Some(Advice {
                summary: "LVM cache attachment changes origin LV I/O through cache media"
                    .to_string(),
                alternatives: vec![
                    "verify the cache pool LV belongs to the same volume group as the origin"
                        .to_string(),
                    "start in writethrough mode when rollback safety matters".to_string(),
                    "keep the origin LV snapshot or backup until cache verification passes"
                        .to_string(),
                ],
            }),
        )
    } else {
        (RiskClass::Online, None)
    }
}

fn classify_remove_device(collection: &str) -> (RiskClass, Advice) {
    if collection == "caches" || collection == "lvmCaches" {
        (
            RiskClass::OfflineRequired,
            Advice {
                summary: "cache detach must flush dirty data before removing cache media"
                    .to_string(),
                alternatives: vec![
                    "switch writeback caches to writethrough before detach".to_string(),
                    "wait for dirty data to drain before removing the cache device".to_string(),
                    "keep backing storage online and verify it remains readable after detach"
                        .to_string(),
                ],
            },
        )
    } else {
        (
            RiskClass::PotentialDataLoss,
            Advice {
                summary: "device removal requires enough remaining data and metadata capacity"
                    .to_string(),
                alternatives: vec![
                    "add replacement capacity before removing the old device".to_string(),
                    "rebalance or evacuate data before removal".to_string(),
                    "verify redundancy and current health before applying".to_string(),
                ],
            },
        )
    }
}

fn destructive_alternatives(collection: &str, object: &Value) -> Vec<String> {
    let mut alternatives = vec![
        "take and verify a backup before destructive changes".to_string(),
        "migrate data to replacement storage first".to_string(),
    ];

    match collection {
        "pools" | "datasets" | "zvols" => {
            alternatives.push("take a recursive snapshot before destroy or rollback".to_string());
            alternatives
                .push("rename or unmount the dataset while validating consumers".to_string());
        }
        "btrfsSubvolumes" => {
            alternatives
                .push("take a read-only snapshot before deleting the subvolume".to_string());
            alternatives
                .push("rename the subvolume and validate consumers before removal".to_string());
        }
        "volumes" | "volumeGroups" | "thinPools" | "luns" | "mdRaids" | "multipathMaps" => {
            alternatives
                .push("grow or attach replacement capacity instead of reformatting".to_string());
        }
        "loopDevices" => {
            alternatives
                .push("detach the loop device without deleting its backing file".to_string());
            alternatives.push("unmount consumers before changing the backing file".to_string());
        }
        "lvmSnapshots" => {
            alternatives.push("merge or mount the snapshot before deleting it".to_string());
            alternatives.push(
                "create a replacement snapshot before pruning old recovery points".to_string(),
            );
        }
        "vdoVolumes" => {
            alternatives
                .push("grow the VDO logical or physical size instead of recreating it".to_string());
            alternatives
                .push("migrate data to a replacement VDO volume before removal".to_string());
        }
        "disks" | "partitions" => {
            alternatives.push(
                "preserve the existing partition table and add capacity elsewhere".to_string(),
            );
            alternatives.push("clone the disk before changing partition metadata".to_string());
        }
        "exports" => {
            alternatives
                .push("disable clients or switch exports before removing the source".to_string());
        }
        _ => {}
    }

    if object
        .get("preserveData")
        .and_then(Value::as_bool)
        .is_some_and(|preserve| !preserve)
    {
        alternatives.push("set preserveData=true for non-destructive planning".to_string());
    }

    alternatives
}

fn operation_label(operation: Operation) -> &'static str {
    match operation {
        Operation::Create => "create",
        Operation::Format => "format",
        Operation::Grow => "grow",
        Operation::Shrink => "shrink",
        Operation::Check => "check",
        Operation::Repair => "repair",
        Operation::Scrub => "scrub",
        Operation::Trim => "trim",
        Operation::Rescan => "rescan",
        Operation::ReplaceDevice => "replace device",
        Operation::AddDevice => "add device",
        Operation::RemoveDevice => "remove device",
        Operation::AddKey => "add key",
        Operation::RemoveKey => "remove key",
        Operation::ImportToken => "import token",
        Operation::RemoveToken => "remove token",
        Operation::SetProperty => "set property",
        Operation::Snapshot => "snapshot",
        Operation::Clone => "clone",
        Operation::Promote => "promote",
        Operation::Import => "import",
        Operation::Export => "export",
        Operation::Unexport => "unexport",
        Operation::Attach => "attach",
        Operation::Detach => "detach",
        Operation::Activate => "activate",
        Operation::Deactivate => "deactivate",
        Operation::Assemble => "assemble",
        Operation::Start => "start",
        Operation::Stop => "stop",
        Operation::Login => "login",
        Operation::Logout => "logout",
        Operation::Open => "open",
        Operation::Close => "close",
        Operation::Mount => "mount",
        Operation::Unmount => "unmount",
        Operation::Remount => "remount",
        Operation::Rename => "rename",
        Operation::Rebalance => "rebalance",
        Operation::Rollback => "rollback",
        Operation::Destroy => "destroy",
    }
}

#[must_use]
pub fn default_capabilities() -> Vec<Capability> {
    vec![
        Capability {
            node_kind: NodeKind::PhysicalDisk,
            operation: Operation::Format,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "creating a new partition table can hide existing data".to_string(),
                alternatives: vec![
                    "clone the disk before replacing partition metadata".to_string(),
                    "prefer adding partitions in known-free regions".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::PhysicalDisk,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "disk rescan rereads the partition table without editing layout"
                    .to_string(),
                alternatives: vec![
                    "use grow or create when partition geometry must change first".to_string(),
                    "verify stable disk identity before refreshing kernel state".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Partition,
            operation: Operation::Create,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "partition creation requires partition table reread coordination"
                    .to_string(),
                alternatives: vec![
                    "verify disk identity and free regions before applying".to_string(),
                    "schedule reboot when active consumers block table reread".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Partition,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "partition rescan refreshes kernel partition inventory".to_string(),
                alternatives: vec![
                    "rescan after target-side disk, LUN, or table changes are complete"
                        .to_string(),
                    "verify dependent filesystems and mappings after kernel reread".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Partition,
            operation: Operation::Grow,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "partition growth may require inactive consumers".to_string(),
                alternatives: vec![
                    "grow backing LUNs or disks before the partition".to_string(),
                    "resize LUKS, LVM, and filesystems only after kernel reread succeeds"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Swap,
            operation: Operation::Format,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "creating a swap signature overwrites target metadata".to_string(),
                alternatives: vec![
                    "add another swap device or file instead of replacing this target".to_string(),
                    "verify the target has no filesystem or encrypted data before mkswap"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Swap,
            operation: Operation::Grow,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "swap growth requires disabling active swap and resizing backing storage"
                    .to_string(),
                alternatives: vec![
                    "add replacement swap capacity before disabling this swap".to_string(),
                    "recreate the swap signature only after backing storage resize".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Swap,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "swap inventory refresh reads activation, size, label, and UUID state"
                    .to_string(),
                alternatives: vec![
                    "use grow when backing capacity changed".to_string(),
                    "use swaplabel property updates only when identity must change".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LuksContainer,
            operation: Operation::Format,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "formatting LUKS destroys access to existing encrypted data".to_string(),
                alternatives: vec![
                    "reuse the existing LUKS container when preserving data".to_string(),
                    "back up LUKS headers before destructive changes".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LuksContainer,
            operation: Operation::Grow,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "LUKS resize requires backing-device growth and mapper coordination"
                    .to_string(),
                alternatives: vec![
                    "grow the backing device before cryptsetup resize".to_string(),
                    "resize consumers only after the mapper reports the new capacity".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LuksContainer,
            operation: Operation::Open,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "LUKS open activates an existing encrypted container as a mapper"
                    .to_string(),
                alternatives: vec![
                    "verify backing device identity before entering credentials".to_string(),
                    "keep formatting as a separate explicit destructive operation".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LuksContainer,
            operation: Operation::Close,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "LUKS close tears down an active mapper without removing the header"
                    .to_string(),
                alternatives: vec![
                    "unmount filesystems and deactivate dependent mappings before close".to_string(),
                    "leave the backing LUKS header intact for later reopen".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LuksContainer,
            operation: Operation::Create,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "LUKS keyslot or token enrollment changes encrypted-container access"
                    .to_string(),
                alternatives: vec![
                    "back up the LUKS header before adding key or token material".to_string(),
                    "test the new unlock path before removing any old keyslot or token".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LuksContainer,
            operation: Operation::AddKey,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "LUKS keyslot enrollment changes encrypted-container access".to_string(),
                alternatives: vec![
                    "back up the LUKS header before adding key material".to_string(),
                    "test the new keyslot before removing any old recovery key".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LuksContainer,
            operation: Operation::ImportToken,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "LUKS token import changes automated unlock access".to_string(),
                alternatives: vec![
                    "back up the LUKS header before importing token metadata".to_string(),
                    "test the token unlock path before removing any older token".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LuksContainer,
            operation: Operation::SetProperty,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "LUKS property changes update header identity metadata or access material"
                    .to_string(),
                alternatives: vec![
                    "back up the LUKS header before changing label, subsystem, UUID, keys, or tokens"
                        .to_string(),
                    "verify a recovery key still unlocks the container".to_string(),
                    "review initrd, crypttab, and stable device references after identity changes"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LuksContainer,
            operation: Operation::Destroy,
            risk: RiskClass::PotentialDataLoss,
            advice: Some(Advice {
                summary: "LUKS keyslot or token removal can lock out encrypted data".to_string(),
                alternatives: vec![
                    "verify another key or token unlocks the device first".to_string(),
                    "take a LUKS header backup before removing access material".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LuksContainer,
            operation: Operation::RemoveKey,
            risk: RiskClass::PotentialDataLoss,
            advice: Some(Advice {
                summary: "LUKS keyslot removal can lock out encrypted data".to_string(),
                alternatives: vec![
                    "verify another key or token unlocks the device first".to_string(),
                    "take a LUKS header backup before removing the keyslot".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LuksContainer,
            operation: Operation::RemoveToken,
            risk: RiskClass::PotentialDataLoss,
            advice: Some(Advice {
                summary: "LUKS token removal can lock out automated unlock".to_string(),
                alternatives: vec![
                    "verify another token, keyslot, or passphrase unlocks first".to_string(),
                    "take a LUKS header backup before removing the token".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::VdoVolume,
            operation: Operation::Grow,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "VDO growth separates logical size from physical backing capacity"
                    .to_string(),
                alternatives: vec![
                    "confirm vdostats utilization before increasing logical size".to_string(),
                    "grow backing storage before physical VDO growth".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::VdoVolume,
            operation: Operation::Start,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "starting a VDO volume activates existing VDO metadata".to_string(),
                alternatives: vec![
                    "verify backing storage and consumers before activation".to_string(),
                    "use create only when initializing new VDO metadata".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::VdoVolume,
            operation: Operation::Stop,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "stopping a VDO volume deactivates it without removing metadata".to_string(),
                alternatives: vec![
                    "unmount and deactivate all consumers before stopping".to_string(),
                    "use remove only when destroying the VDO volume metadata".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::VdoVolume,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "VDO rescan refreshes status and utilization reporting".to_string(),
                alternatives: vec![
                    "use grow when logical or physical capacity must change".to_string(),
                    "review vdostats before resizing dependent filesystems".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Filesystem,
            operation: Operation::Grow,
            risk: RiskClass::Online,
            advice: None,
        },
        Capability {
            node_kind: NodeKind::Filesystem,
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            advice: Some(Advice {
                summary: "supported filesystem property updates reconcile labels, selected UUIDs, and ZFS filesystem properties"
                    .to_string(),
                alternatives: vec![
                    "use filesystem label aliases for Btrfs, ext, and XFS filesystems"
                        .to_string(),
                    "treat ext and XFS UUID changes as offline identity changes".to_string(),
                    "model arbitrary ZFS filesystem properties through ZFS dataset declarations"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Filesystem,
            operation: Operation::Shrink,
            risk: RiskClass::PotentialDataLoss,
            advice: Some(Advice {
                summary: "filesystem shrink support depends on filesystem type".to_string(),
                alternatives: vec![
                    "create a new smaller filesystem and migrate data".to_string(),
                    "grow consumers around the existing filesystem instead".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Filesystem,
            operation: Operation::Check,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "filesystem checks inspect metadata before risky maintenance".to_string(),
                alternatives: vec![
                    "run read-only checks before repair".to_string(),
                    "quiesce or unmount filesystems before tools that require offline access"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Filesystem,
            operation: Operation::Repair,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "filesystem repair mutates metadata and requires review".to_string(),
                alternatives: vec![
                    "restore from backup or snapshot instead of repairing in place".to_string(),
                    "repair a cloned device first when production risk is high".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Filesystem,
            operation: Operation::Scrub,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "Btrfs scrub verifies checksummed filesystem data online".to_string(),
                alternatives: vec![
                    "use filesystem check when metadata corruption is suspected".to_string(),
                    "monitor scrub status until completion".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Filesystem,
            operation: Operation::Trim,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "filesystem trim returns unused blocks to lower storage layers"
                    .to_string(),
                alternatives: vec![
                    "verify discard propagation through LUKS, LVM, thin, and virtual layers"
                        .to_string(),
                    "schedule regular fstrim instead of ad hoc discard on busy systems"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Filesystem,
            operation: Operation::Remount,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "filesystem remount updates live mount options without deleting data"
                    .to_string(),
                alternatives: vec![
                    "remount with reviewed options before unmounting a busy path".to_string(),
                    "persist steady-state options through NixOS fileSystems".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Filesystem,
            operation: Operation::AddDevice,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "multi-device filesystem growth attaches reviewed member devices"
                    .to_string(),
                alternatives: vec![
                    "verify stable by-id paths before adding devices".to_string(),
                    "prefer replacement workflows when removing old media after adding capacity"
                        .to_string(),
                    "rebalance or rereplicate data after changing member topology".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Filesystem,
            operation: Operation::ReplaceDevice,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "filesystem device replacement preserves data while changing members"
                    .to_string(),
                alternatives: vec![
                    "add replacement capacity before evacuating old media".to_string(),
                    "review filesystem-specific replacement status until convergence"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Filesystem,
            operation: Operation::RemoveDevice,
            risk: RiskClass::PotentialDataLoss,
            advice: Some(Advice {
                summary: "filesystem device removal requires enough remaining replicas and capacity"
                    .to_string(),
                alternatives: vec![
                    "add replacement capacity before removing old media".to_string(),
                    "take a backup or snapshot before topology contraction".to_string(),
                    "rebalance or rereplicate data before final member removal".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsPool,
            operation: Operation::Scrub,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "ZFS pool scrub verifies data and repairs redundant copies".to_string(),
                alternatives: vec![
                    "review pool health before starting a scrub".to_string(),
                    "schedule scrubs outside latency-sensitive windows".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmThinPool,
            operation: Operation::Create,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "thin pool creation consumes free extents in a volume group".to_string(),
                alternatives: vec![
                    "verify VG free extents before allocation".to_string(),
                    "choose thin-pool size and overcommit policy before creating thin volumes"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmThinPool,
            operation: Operation::Grow,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "thin pool growth must monitor data and metadata utilization".to_string(),
                alternatives: vec![
                    "extend metadata before it approaches exhaustion".to_string(),
                    "verify autoextend policy and overcommit before growth".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmThinPool,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "thin pool status refresh reads data and metadata utilization"
                    .to_string(),
                alternatives: vec![
                    "grow data or metadata only after reviewing utilization".to_string(),
                    "verify monitoring and autoextend before overcommitting thin volumes"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmThinPool,
            operation: Operation::Activate,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "thin pool activation makes existing thin volumes available".to_string(),
                alternatives: vec![
                    "activate only after VG metadata and dependent consumers are reviewed"
                        .to_string(),
                    "verify thin metadata health before exposing consumers".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmThinPool,
            operation: Operation::Deactivate,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "thin pool deactivation makes contained thin volumes unavailable without deleting them"
                    .to_string(),
                alternatives: vec![
                    "stop consumers before deactivation".to_string(),
                    "deactivate instead of removing a thin pool when preserving data".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmThinPool,
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "removing a thin pool destroys all thin volumes stored in it".to_string(),
                alternatives: vec![
                    "migrate or snapshot thin volumes before removing the pool".to_string(),
                    "deactivate dependent thin volumes and filesystems before lvremove".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmSnapshot,
            operation: Operation::Snapshot,
            risk: RiskClass::Reversible,
            advice: Some(Advice {
                summary: "LVM snapshot creation preserves an origin recovery point".to_string(),
                alternatives: vec![
                    "size the snapshot for expected changed blocks".to_string(),
                    "monitor snapshot fullness while it exists".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmSnapshot,
            operation: Operation::Rollback,
            risk: RiskClass::PotentialDataLoss,
            advice: Some(Advice {
                summary: "LVM snapshot merge rolls the origin back".to_string(),
                alternatives: vec![
                    "take a fresh snapshot before merge".to_string(),
                    "inspect the snapshot before rollback".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmSnapshot,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "LVM snapshot rescan refreshes origin and COW usage metadata"
                    .to_string(),
                alternatives: vec![
                    "activate snapshots for read-only recovery inspection".to_string(),
                    "merge only after reviewing origin and snapshot state".to_string(),
                    "verify snapshot fullness before depending on rollback".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmSnapshot,
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "removing an LVM snapshot deletes a recovery point".to_string(),
                alternatives: vec![
                    "keep the snapshot until backups are verified".to_string(),
                    "merge or clone the snapshot before deletion".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmSnapshot,
            operation: Operation::Activate,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "LVM snapshot activation exposes an existing recovery volume"
                    .to_string(),
                alternatives: vec![
                    "activate snapshots only for reviewed inspection or recovery".to_string(),
                    "mount read-only where possible before data validation".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmSnapshot,
            operation: Operation::Deactivate,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "LVM snapshot deactivation hides the recovery volume without deleting it"
                    .to_string(),
                alternatives: vec![
                    "unmount any snapshot filesystem before deactivation".to_string(),
                    "keep the snapshot until recovery needs are resolved".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsSnapshot,
            operation: Operation::Snapshot,
            risk: RiskClass::Reversible,
            advice: Some(Advice {
                summary: "ZFS snapshot creation preserves a point-in-time recovery point"
                    .to_string(),
                alternatives: vec![
                    "use recursive snapshots when descendants must be captured together"
                        .to_string(),
                    "add holds for snapshots that retention jobs must not prune".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsSnapshot,
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            advice: Some(Advice {
                summary: "ZFS snapshot holds and releases update retention references"
                    .to_string(),
                alternatives: vec![
                    "hold snapshots before risky migrations or destructive changes".to_string(),
                    "release only after replacement backups or snapshots are verified"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsSnapshot,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "ZFS snapshot rescan refreshes metadata, holds, and graph relationships"
                    .to_string(),
                alternatives: vec![
                    "use holds or releases when retention state must change".to_string(),
                    "clone snapshots for inspection before rollback or destruction".to_string(),
                    "verify source dataset relationships after snapshot metadata refresh"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsSnapshot,
            operation: Operation::Clone,
            risk: RiskClass::Reversible,
            advice: Some(Advice {
                summary: "ZFS snapshot clone creates a writable dataset from a recovery point"
                    .to_string(),
                alternatives: vec![
                    "clone a snapshot for inspection before rollback".to_string(),
                    "destroy temporary clones after migration or validation".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsSnapshot,
            operation: Operation::Rename,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "ZFS snapshot rename preserves a recovery point under a new name"
                    .to_string(),
                alternatives: vec![
                    "hold snapshots before renaming when retention jobs may race".to_string(),
                    "update replication and rollback references after rename".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsSnapshot,
            operation: Operation::Rollback,
            risk: RiskClass::PotentialDataLoss,
            advice: Some(Advice {
                summary: "ZFS rollback can discard changes newer than the snapshot".to_string(),
                alternatives: vec![
                    "clone the snapshot and inspect data before rollback".to_string(),
                    "take a pre-rollback snapshot of the current state".to_string(),
                    "use recursive rollback only after reviewing newer snapshots and clones"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsSnapshot,
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "destroying a ZFS snapshot removes a recovery point".to_string(),
                alternatives: vec![
                    "keep or hold the snapshot until replacement backups are verified"
                        .to_string(),
                    "clone the snapshot before pruning if data may still be needed".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsSnapshot,
            operation: Operation::Snapshot,
            risk: RiskClass::Reversible,
            advice: Some(Advice {
                summary: "Btrfs snapshot creation preserves a subvolume recovery point"
                    .to_string(),
                alternatives: vec![
                    "prefer read-only snapshots for backup or migration checkpoints"
                        .to_string(),
                    "verify qgroup policy before creating many snapshots".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsSnapshot,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "Btrfs snapshot rescan refreshes subvolume metadata and relationships"
                    .to_string(),
                alternatives: vec![
                    "use read-only snapshots for recovery points before risky updates".to_string(),
                    "verify qgroup usage before pruning or creating many snapshots".to_string(),
                    "clone or rename snapshots when retention intent is uncertain".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsSnapshot,
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "deleting a Btrfs snapshot removes its recovery tree".to_string(),
                alternatives: vec![
                    "keep a read-only snapshot until replacement backups are verified"
                        .to_string(),
                    "rename the snapshot before final deletion when consumers are uncertain"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LoopDevice,
            operation: Operation::Create,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "loop device creation maps a backing file or block device".to_string(),
                alternatives: vec![
                    "verify backing path identity before mapping".to_string(),
                    "use stable loop configuration when the mapping must survive reboot"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LoopDevice,
            operation: Operation::Grow,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "loop device growth refreshes backing size visibility".to_string(),
                alternatives: vec![
                    "grow the backing file first".to_string(),
                    "refresh dependent consumers after losetup -c".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LoopDevice,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "loop device rescan refreshes mapping inventory without mutation"
                    .to_string(),
                alternatives: vec![
                    "use grow only when backing size changed".to_string(),
                    "detach only after consumers are stopped".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LoopDevice,
            operation: Operation::Destroy,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "loop detach requires inactive consumers".to_string(),
                alternatives: vec![
                    "unmount filesystems before detach".to_string(),
                    "preserve the backing file for remapping".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsPool,
            operation: Operation::Create,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "creating a ZFS pool writes labels to member devices".to_string(),
                alternatives: vec![
                    "verify devices are empty before zpool create".to_string(),
                    "import an existing pool instead of recreating it".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsPool,
            operation: Operation::AddDevice,
            risk: RiskClass::Online,
            advice: None,
        },
        Capability {
            node_kind: NodeKind::ZfsPool,
            operation: Operation::Import,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "ZFS pool import activates an existing pool on this host".to_string(),
                alternatives: vec![
                    "import read-only first when validating moved storage".to_string(),
                    "verify hostid, cachefile, mountpoints, and encryption keys".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsPool,
            operation: Operation::Export,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "ZFS pool export cleanly detaches a pool without deleting it".to_string(),
                alternatives: vec![
                    "export instead of destroying a pool that will be moved".to_string(),
                    "stop services, shares, and LUN mappings before export".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsPool,
            operation: Operation::ReplaceDevice,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "ZFS pool device replacement must preserve pool health through resilver"
                    .to_string(),
                alternatives: vec![
                    "attach or add replacement capacity before removing a weak vdev".to_string(),
                    "monitor zpool status until resilver completes".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsPool,
            operation: Operation::RemoveDevice,
            risk: RiskClass::PotentialDataLoss,
            advice: Some(Advice {
                summary: "ZFS pool device removal depends on pool topology and evacuation support"
                    .to_string(),
                alternatives: vec![
                    "replace the device instead when removal is not supported".to_string(),
                    "verify pool free space and health before starting evacuation".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsPool,
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            advice: Some(Advice {
                summary: "ZFS pool property updates use zpool set on the reviewed pool"
                    .to_string(),
                alternatives: vec![
                    "inspect current pool properties before changing behavior".to_string(),
                    "prefer reversible property changes before topology changes".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsPool,
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "destroying a ZFS pool removes the pool and all contained datasets"
                    .to_string(),
                alternatives: vec![
                    "export the pool when moving it between systems".to_string(),
                    "take recursive snapshots and verify backups before destruction".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsFilesystem,
            operation: Operation::AddDevice,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "adding a Btrfs device expands the mounted filesystem device set"
                    .to_string(),
                alternatives: vec![
                    "verify the new block device identity before adding it".to_string(),
                    "run a filtered balance after adding capacity when profiles need reshaping"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsFilesystem,
            operation: Operation::ReplaceDevice,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "Btrfs device replacement must preserve live filesystem availability"
                    .to_string(),
                alternatives: vec![
                    "add replacement capacity before removing a failing device".to_string(),
                    "monitor btrfs replace status until the operation completes".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsFilesystem,
            operation: Operation::RemoveDevice,
            risk: RiskClass::PotentialDataLoss,
            advice: Some(Advice {
                summary:
                    "removing a Btrfs device requires enough remaining data and metadata space"
                        .to_string(),
                alternatives: vec![
                    "run a filtered balance before removal".to_string(),
                    "add replacement capacity before removing the old device".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsSubvolume,
            operation: Operation::Create,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "Btrfs subvolume creation changes namespace layout".to_string(),
                alternatives: vec![
                    "create at an empty reviewed path".to_string(),
                    "verify qgroup policy before creation".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsSubvolume,
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "deleting a Btrfs subvolume removes its live tree".to_string(),
                alternatives: vec![
                    "take a read-only snapshot before deletion".to_string(),
                    "rename the subvolume before final removal".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsSubvolume,
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            advice: Some(Advice {
                summary: "Btrfs subvolume property updates support read-only toggles".to_string(),
                alternatives: vec![
                    "use readOnly, readonly, ro, btrfs.readonly, or btrfs.ro".to_string(),
                    "review unsupported subvolume properties manually before changing them"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsSubvolume,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "Btrfs subvolume rescan refreshes metadata without changing data"
                    .to_string(),
                alternatives: vec![
                    "use read-only property updates only when enforcement must change"
                        .to_string(),
                    "inspect qgroups and snapshots before cleanup".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsSubvolume,
            operation: Operation::Rename,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "Btrfs subvolume rename stages a path move before deletion".to_string(),
                alternatives: vec![
                    "update mounts and qgroups before moving the path".to_string(),
                    "validate consumers on the renamed subvolume before deleting old paths"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsQgroup,
            operation: Operation::Create,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "Btrfs qgroup creation changes quota hierarchy for a mounted filesystem"
                    .to_string(),
                alternatives: vec![
                    "enable quota accounting and inspect existing qgroups before creation"
                        .to_string(),
                    "create qgroups before assigning subvolume limits".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsQgroup,
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            advice: Some(Advice {
                summary: "Btrfs qgroup limit changes alter referenced or exclusive quota enforcement"
                    .to_string(),
                alternatives: vec![
                    "inspect current referenced and exclusive usage before tightening limits"
                        .to_string(),
                    "raise limits temporarily before migrations or balance operations".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsQgroup,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "Btrfs qgroup rescan refreshes quota usage and hierarchy"
                    .to_string(),
                alternatives: vec![
                    "inspect referenced and exclusive usage before tightening limits"
                        .to_string(),
                    "use property updates only when quota enforcement must change".to_string(),
                    "verify quota accounting before deleting or replacing qgroups".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsQgroup,
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "destroying a Btrfs qgroup removes quota policy for that group"
                    .to_string(),
                alternatives: vec![
                    "clear limits or move subvolumes to a replacement qgroup first".to_string(),
                    "verify quota hierarchy and usage after removing the qgroup".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Zvol,
            operation: Operation::Create,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "zvol creation consumes ZFS pool capacity".to_string(),
                alternatives: vec![
                    "verify free pool capacity before creation".to_string(),
                    "decide sparse versus reserved allocation before exposing the block device"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Zvol,
            operation: Operation::Grow,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "zvol growth changes volsize for downstream block consumers".to_string(),
                alternatives: vec![
                    "verify pool free space before changing volsize".to_string(),
                    "rescan dependent block consumers after growth".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Zvol,
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            advice: Some(Advice {
                summary: "zvol property updates use zfs set on the reviewed block volume"
                    .to_string(),
                alternatives: vec![
                    "verify dependent guests or LUN exports before changing zvol behavior"
                        .to_string(),
                    "snapshot or clone the zvol before risky property changes".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Zvol,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "zvol rescan refreshes volume properties and block graph state"
                    .to_string(),
                alternatives: vec![
                    "use grow only when volsize must change".to_string(),
                    "review dependent guests and LUN exports before changing consumers"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Zvol,
            operation: Operation::Rename,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "zvol rename preserves block data while changing the ZFS volume name"
                    .to_string(),
                alternatives: vec![
                    "detach or rescan downstream LUN, VM, and filesystem consumers first"
                        .to_string(),
                    "validate consumers on the renamed zvol before removing old references"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Zvol,
            operation: Operation::Promote,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "zvol clone promotion changes clone dependency ownership".to_string(),
                alternatives: vec![
                    "inspect origin and consumers before promoting the clone".to_string(),
                    "validate downstream LUN, VM, and filesystem consumers after promotion"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Zvol,
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "destroying a zvol removes the block volume and its data".to_string(),
                alternatives: vec![
                    "snapshot or clone the zvol before destruction".to_string(),
                    "detach downstream LUN, VM, or filesystem consumers first".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsDataset,
            operation: Operation::Create,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "ZFS dataset creation consumes pool namespace and inherits parent policy"
                    .to_string(),
                alternatives: vec![
                    "review inherited mountpoint, quota, reservation, and encryption properties"
                        .to_string(),
                    "create under the intended parent dataset before exposing consumers"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsDataset,
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            advice: Some(Advice {
                summary: "ZFS dataset property updates use zfs set on the reviewed dataset"
                    .to_string(),
                alternatives: vec![
                    "review inherited quota, reservation, mountpoint, and encryption policy first"
                        .to_string(),
                    "snapshot datasets before property changes that affect consumers".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsDataset,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "ZFS dataset rescan refreshes properties, mounts, and graph state"
                    .to_string(),
                alternatives: vec![
                    "use property updates only when dataset policy must change".to_string(),
                    "inspect snapshots and clones before destructive cleanup".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsDataset,
            operation: Operation::Rename,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "ZFS dataset rename preserves data while changing its dataset name"
                    .to_string(),
                alternatives: vec![
                    "update mountpoints, shares, and services before rename".to_string(),
                    "validate consumers on the renamed dataset before destroying old references"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsDataset,
            operation: Operation::Promote,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "ZFS dataset clone promotion changes clone dependency ownership"
                    .to_string(),
                alternatives: vec![
                    "inspect clone origin and dependent snapshots before promotion".to_string(),
                    "validate mounts, shares, and services against the promoted dataset"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::VdoVolume,
            operation: Operation::Create,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "creating a VDO volume writes metadata to the backing device".to_string(),
                alternatives: vec![
                    "inspect existing signatures before creation".to_string(),
                    "migrate data or grow an existing VDO volume instead of recreating it"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::VdoVolume,
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "removing a VDO volume destroys the deduplicated block layer".to_string(),
                alternatives: vec![
                    "migrate data away from the VDO device before removal".to_string(),
                    "deactivate dependent filesystems and mappings before vdo remove".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmPhysicalVolume,
            operation: Operation::Create,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "physical volume creation writes LVM metadata to the device".to_string(),
                alternatives: vec![
                    "inspect existing signatures before pvcreate".to_string(),
                    "reuse the existing PV when preserving VG data".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmPhysicalVolume,
            operation: Operation::Grow,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "physical volume growth refreshes LVM capacity after backing growth"
                    .to_string(),
                alternatives: vec![
                    "grow backing storage before pvresize".to_string(),
                    "verify VG free extents after pvresize".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmPhysicalVolume,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "physical volume rescan refreshes the LVM device cache".to_string(),
                alternatives: vec![
                    "rescan the underlying block path before refreshing LVM metadata".to_string(),
                    "use grow when pvresize is required after backing capacity changes".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmPhysicalVolume,
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "physical volume removal erases LVM metadata from the device".to_string(),
                alternatives: vec![
                    "pvmove and vgreduce before pvremove".to_string(),
                    "verify no volume group still uses the PV".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmLogicalVolume,
            operation: Operation::Create,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "logical volume creation consumes free extents in a volume group"
                    .to_string(),
                alternatives: vec![
                    "verify VG free space before allocation".to_string(),
                    "choose explicit LV size and naming before formatting consumers".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmLogicalVolume,
            operation: Operation::Grow,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "logical volume growth consumes free extents from the volume group"
                    .to_string(),
                alternatives: vec![
                    "verify volume group free space before lvextend".to_string(),
                    "grow the filesystem only after the LV reports the new size".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmLogicalVolume,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "logical volume status refresh reads LV attributes and graph relationships"
                    .to_string(),
                alternatives: vec![
                    "grow only when capacity must change".to_string(),
                    "activate or deactivate only when availability must change".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmLogicalVolume,
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "removing a logical volume destroys its contents".to_string(),
                alternatives: vec![
                    "snapshot the LV before removal".to_string(),
                    "rename or deactivate the LV while validating consumers".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmLogicalVolume,
            operation: Operation::Activate,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "logical volume activation exposes an existing LV without creating it"
                    .to_string(),
                alternatives: vec![
                    "inspect LV metadata and dependent mappings before activation".to_string(),
                    "activate only the reviewed LV needed by consumers".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmLogicalVolume,
            operation: Operation::Deactivate,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "logical volume deactivation hides an LV without deleting data".to_string(),
                alternatives: vec![
                    "unmount filesystems and stop services before deactivation".to_string(),
                    "deactivate instead of removing an LV when preserving data".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmLogicalVolume,
            operation: Operation::Rename,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "logical volume rename changes the LV path without deleting data"
                    .to_string(),
                alternatives: vec![
                    "update crypttab, fileSystems, LUN exports, and services before rename"
                        .to_string(),
                    "validate consumers with the renamed LV before removing old declarations"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmVolumeGroup,
            operation: Operation::Create,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "creating an LVM volume group writes metadata to member physical volumes"
                    .to_string(),
                alternatives: vec![
                    "inspect pvs and block identity before creation".to_string(),
                    "extend an existing volume group instead of recreating it".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmVolumeGroup,
            operation: Operation::Grow,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "volume group growth adds reviewed physical volumes to the VG"
                    .to_string(),
                alternatives: vec![
                    "inspect the candidate PV before vgextend".to_string(),
                    "extend the existing VG instead of recreating it".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmVolumeGroup,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "volume group rescan refreshes LVM metadata and active LV tables"
                    .to_string(),
                alternatives: vec![
                    "run block and PV rescans first when storage paths changed".to_string(),
                    "verify LV activation state and VG free extents after refresh".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmVolumeGroup,
            operation: Operation::Import,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "volume group import reactivates an exported VG without recreating it"
                    .to_string(),
                alternatives: vec![
                    "inspect PV identities and VG UUIDs before vgimport".to_string(),
                    "activate consumers only after imported metadata is verified".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmVolumeGroup,
            operation: Operation::Export,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "volume group export prepares a VG for movement without deleting data"
                    .to_string(),
                alternatives: vec![
                    "deactivate logical volumes before vgexport".to_string(),
                    "export instead of removing a VG that will be moved".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmVolumeGroup,
            operation: Operation::Activate,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "volume group activation makes contained LVs available".to_string(),
                alternatives: vec![
                    "inspect PV membership and VG metadata before vgchange -ay".to_string(),
                    "activate only reviewed VGs needed by the host".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmVolumeGroup,
            operation: Operation::Deactivate,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "volume group deactivation makes contained LVs unavailable without deletion"
                    .to_string(),
                alternatives: vec![
                    "stop mounts, mappings, and services before vgchange -an".to_string(),
                    "deactivate instead of removing a VG when preserving storage".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmVolumeGroup,
            operation: Operation::RemoveDevice,
            risk: RiskClass::PotentialDataLoss,
            advice: Some(Advice {
                summary: "volume group device removal must evacuate allocated extents first"
                    .to_string(),
                alternatives: vec![
                    "run pvmove to drain the physical volume before vgreduce".to_string(),
                    "add replacement capacity before reducing a full or constrained VG"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmVolumeGroup,
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "removing an LVM volume group removes the grouping layer for all contained volumes"
                    .to_string(),
                alternatives: vec![
                    "remove or migrate logical volumes before vgremove".to_string(),
                    "deactivate or rename the volume group while validating consumers".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmVolumeGroup,
            operation: Operation::Rename,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "volume group rename changes every contained LV path".to_string(),
                alternatives: vec![
                    "update initrd, mount, crypttab, and service references before reboot"
                        .to_string(),
                    "validate activation with the renamed VG before cleanup".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsDataset,
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "destroying a dataset removes its live data".to_string(),
                alternatives: vec![
                    "take a recursive snapshot before destruction".to_string(),
                    "rename or unmount the dataset while validating consumers".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsFilesystem,
            operation: Operation::Rebalance,
            risk: RiskClass::Online,
            advice: None,
        },
        Capability {
            node_kind: NodeKind::MdRaid,
            operation: Operation::Create,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "MD RAID creation writes array metadata to member devices".to_string(),
                alternatives: vec![
                    "inspect member signatures before mdadm --create".to_string(),
                    "assemble an existing array instead of recreating it".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::MdRaid,
            operation: Operation::Assemble,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "MD RAID assemble activates an existing array from known members"
                    .to_string(),
                alternatives: vec![
                    "assemble existing metadata instead of recreating arrays".to_string(),
                    "inspect member event counts before starting consumers".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::MdRaid,
            operation: Operation::Stop,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "MD RAID stop deactivates the array without removing member metadata"
                    .to_string(),
                alternatives: vec![
                    "unmount and deactivate all consumers before stopping".to_string(),
                    "use stop instead of destroy when preserving later assembly".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::MdRaid,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "MD RAID rescan refreshes array and member metadata inventory"
                    .to_string(),
                alternatives: vec![
                    "use assemble only after member identities and event counts are reviewed"
                        .to_string(),
                    "verify /proc/mdstat before starting dependent consumers".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::MdRaid,
            operation: Operation::Grow,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "MD RAID growth and reshape require redundancy and resync coordination"
                    .to_string(),
                alternatives: vec![
                    "add replacement capacity before increasing array size".to_string(),
                    "monitor /proc/mdstat until reshape completes".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::MdRaid,
            operation: Operation::AddDevice,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "adding an MD RAID member starts array resync or spare activation"
                    .to_string(),
                alternatives: vec![
                    "verify member identity before adding it".to_string(),
                    "monitor /proc/mdstat until sync completes".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::MdRaid,
            operation: Operation::ReplaceDevice,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "MD RAID replacement must maintain redundancy through resync".to_string(),
                alternatives: vec![
                    "replace one member at a time".to_string(),
                    "keep the old member available until mdadm reports the array clean".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::MdRaid,
            operation: Operation::RemoveDevice,
            risk: RiskClass::PotentialDataLoss,
            advice: Some(Advice {
                summary: "removing an MD RAID member can degrade or break redundancy".to_string(),
                alternatives: vec![
                    "add replacement capacity before removal".to_string(),
                    "verify the array remains redundant after removal".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::MdRaid,
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "stopping and removing an MD RAID array can make member data inaccessible"
                    .to_string(),
                alternatives: vec![
                    "deactivate consumers and preserve member devices for later assembly"
                        .to_string(),
                    "verify backups before zeroing or reusing member metadata".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::MultipathDevice,
            operation: Operation::Grow,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "multipath map growth requires path rescan and map resize".to_string(),
                alternatives: vec![
                    "rescan all backing paths before resizing the map".to_string(),
                    "verify every active path reports the new size".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::MultipathDevice,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "multipath map rescan reloads existing path maps without deleting data"
                    .to_string(),
                alternatives: vec![
                    "rescan backing SCSI or iSCSI paths before reloading the map".to_string(),
                    "verify map WWID, path state, and dependent consumers after reload".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::MultipathDevice,
            operation: Operation::RemoveDevice,
            risk: RiskClass::PotentialDataLoss,
            advice: Some(Advice {
                summary: "multipath path removal can reduce or break path redundancy".to_string(),
                alternatives: vec![
                    "remove a path only after alternate paths are active".to_string(),
                    "verify the path WWID before deleting it from the map".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::MultipathDevice,
            operation: Operation::AddDevice,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "adding a multipath path should preserve active path redundancy"
                    .to_string(),
                alternatives: vec![
                    "verify the path WWID matches the intended map".to_string(),
                    "reload maps after adding the path".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::MultipathDevice,
            operation: Operation::ReplaceDevice,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "multipath path replacement needs live-path coordination".to_string(),
                alternatives: vec![
                    "add replacement paths before deleting old paths".to_string(),
                    "keep at least one healthy path active during replacement".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::NfsExport,
            operation: Operation::Create,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "NFS export creation exposes a local path to selected clients".to_string(),
                alternatives: vec![
                    "start with restrictive client selectors and read-only options".to_string(),
                    "verify ownership, permissions, and firewall policy before exporting"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::NfsExport,
            operation: Operation::Export,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "NFS export publication exposes a local path to selected clients"
                    .to_string(),
                alternatives: vec![
                    "start with restrictive client selectors and read-only options".to_string(),
                    "verify ownership, permissions, and firewall policy before exporting"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::NfsExport,
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            advice: Some(Advice {
                summary: "NFS export option changes alter client access semantics".to_string(),
                alternatives: vec![
                    "switch writable exports to read-only before removal".to_string(),
                    "review active clients before changing root squash or sync policy".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::NfsExport,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "NFS export rescan refreshes export inventory without reloading exports"
                    .to_string(),
                alternatives: vec![
                    "use option updates only when access policy must change".to_string(),
                    "verify active clients before unexporting".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::NfsExport,
            operation: Operation::Destroy,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "unexporting NFS paths can interrupt active remote clients".to_string(),
                alternatives: vec![
                    "drain or migrate clients before unexporting the path".to_string(),
                    "verify no active mounts still depend on the export".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::NfsExport,
            operation: Operation::Unexport,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "unexporting NFS paths can interrupt active remote clients".to_string(),
                alternatives: vec![
                    "drain or migrate clients before unexporting the path".to_string(),
                    "verify no active mounts still depend on the export".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::NfsMount,
            operation: Operation::Create,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "mounting an NFS source changes local namespace state".to_string(),
                alternatives: vec![
                    "use x-systemd.automount or nofail for unreliable networks".to_string(),
                    "verify server reachability and export permissions before mounting"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::NfsMount,
            operation: Operation::Mount,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "mounting an NFS source changes local namespace state".to_string(),
                alternatives: vec![
                    "use x-systemd.automount or nofail for unreliable networks".to_string(),
                    "verify server reachability and export permissions before mounting"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::NfsMount,
            operation: Operation::Remount,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "NFS remount updates local mount options without deleting remote data"
                    .to_string(),
                alternatives: vec![
                    "remount with reviewed options before unmounting a busy path".to_string(),
                    "persist long-lived option changes through NixOS fileSystems".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::NfsMount,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "NFS mount rescan refreshes mounted source and options without remounting"
                    .to_string(),
                alternatives: vec![
                    "use remount only when local options must change".to_string(),
                    "verify open files before unmounting busy paths".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::NfsMount,
            operation: Operation::Destroy,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "unmounting an NFS client path can interrupt local services".to_string(),
                alternatives: vec![
                    "stop services and automount units before unmounting".to_string(),
                    "verify no open files or bind mounts still depend on the mountpoint"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::NfsMount,
            operation: Operation::Unmount,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "unmounting an NFS client path can interrupt local services".to_string(),
                alternatives: vec![
                    "stop services and automount units before unmounting".to_string(),
                    "verify no open files or bind mounts still depend on the mountpoint"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Lun,
            operation: Operation::Create,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "LUN attach discovers existing target-side storage on this host"
                    .to_string(),
                alternatives: vec![
                    "verify target-side LUN identity before rescanning sessions".to_string(),
                    "use stable by-path devices before provisioning downstream consumers"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Lun,
            operation: Operation::Attach,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "LUN attach discovers existing target-side storage on this host"
                    .to_string(),
                alternatives: vec![
                    "verify target-side LUN identity before rescanning sessions".to_string(),
                    "use stable by-path devices before provisioning downstream consumers"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Lun,
            operation: Operation::Grow,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "LUN growth must be coordinated with the storage target and kernel rescan"
                    .to_string(),
                alternatives: vec![
                    "grow the target LUN before resizing consumers".to_string(),
                    "rescan paths and verify multipath before filesystem growth".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Lun,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "LUN rescan refreshes existing host paths without deleting target data"
                    .to_string(),
                alternatives: vec![
                    "declare stable by-path devices before depending on refreshed paths"
                        .to_string(),
                    "use grow when target capacity changed and consumers must be resized"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Lun,
            operation: Operation::Destroy,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "LUN detach removes selected host paths without deleting target-side data"
                    .to_string(),
                alternatives: vec![
                    "unmount filesystems and deactivate mappings before deleting paths"
                        .to_string(),
                    "detach one redundant path at a time after alternate paths are healthy"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Lun,
            operation: Operation::Detach,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "LUN detach removes selected host paths without deleting target-side data"
                    .to_string(),
                alternatives: vec![
                    "unmount filesystems and deactivate mappings before deleting paths"
                        .to_string(),
                    "detach one redundant path at a time after alternate paths are healthy"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::NvmeNamespace,
            operation: Operation::Create,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "NVMe namespace creation allocates capacity on the controller"
                    .to_string(),
                alternatives: vec![
                    "attach an existing namespace when data must be preserved".to_string(),
                    "review nvme list-ns output before creating a namespace".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::NvmeNamespace,
            operation: Operation::Grow,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "NVMe namespace growth requires controller-side change and host rescan"
                    .to_string(),
                alternatives: vec![
                    "perform controller-side resize before running host namespace rescan"
                        .to_string(),
                    "migrate to a replacement namespace when resize is unsupported".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::NvmeNamespace,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "NVMe namespace rescan refreshes controller namespace inventory"
                    .to_string(),
                alternatives: vec![
                    "use grow when controller-side namespace capacity changed".to_string(),
                    "verify consumers after namespace inventory changes".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::NvmeNamespace,
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "NVMe namespace deletion removes controller-managed storage".to_string(),
                alternatives: vec![
                    "detach the namespace without deleting it when preserving data".to_string(),
                    "migrate consumers before delete-ns".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::IscsiSession,
            operation: Operation::Create,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "iSCSI login attaches remote targets and may expose new LUN paths"
                    .to_string(),
                alternatives: vec![
                    "verify portal and target IQN before login".to_string(),
                    "prefer stable by-path devices before layering filesystems or mappings"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::IscsiSession,
            operation: Operation::Login,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "iSCSI login attaches remote targets and may expose new LUN paths"
                    .to_string(),
                alternatives: vec![
                    "verify portal and target IQN before login".to_string(),
                    "prefer stable by-path devices before layering filesystems or mappings"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::IscsiSession,
            operation: Operation::Grow,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "iSCSI-backed growth requires target coordination and host rescan"
                    .to_string(),
                alternatives: vec![
                    "grow the target LUN before resizing consumers".to_string(),
                    "rescan the iSCSI session and verify every path before filesystem growth"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::IscsiSession,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "iSCSI session rescan refreshes existing target paths".to_string(),
                alternatives: vec![
                    "use login for new target sessions and logout for removal".to_string(),
                    "declare LUN path devices when individual SCSI paths need verification"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::IscsiSession,
            operation: Operation::Destroy,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "iSCSI logout detaches remote LUN paths from this host".to_string(),
                alternatives: vec![
                    "drain filesystems, multipath maps, and LVM consumers before logout"
                        .to_string(),
                    "disable automatic login only after dependent services are migrated"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::IscsiSession,
            operation: Operation::Logout,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "iSCSI logout detaches remote LUN paths from this host".to_string(),
                alternatives: vec![
                    "drain filesystems, multipath maps, and LVM consumers before logout"
                        .to_string(),
                    "disable automatic login only after dependent services are migrated"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmCache,
            operation: Operation::Create,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "LVM cache creation attaches a cache pool to an origin LV".to_string(),
                alternatives: vec![
                    "verify the origin LV and cache pool with lvs before lvconvert".to_string(),
                    "use writethrough mode before moving to writeback".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmCache,
            operation: Operation::AddDevice,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "LVM cache attachment changes origin LV I/O through cache media"
                    .to_string(),
                alternatives: vec![
                    "attach a reviewed cache pool LV from the same VG".to_string(),
                    "verify dirty data and cache mode after conversion".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmCache,
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            advice: Some(Advice {
                summary: "LVM cache property changes tune cache mode or policy".to_string(),
                alternatives: vec![
                    "switch toward writethrough before detach or replacement".to_string(),
                    "review lvs cache fields after every mode change".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmCache,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "LVM cache status refresh reads cache mode, policy, and utilization"
                    .to_string(),
                alternatives: vec![
                    "review lvs cache fields before detach or replacement".to_string(),
                    "use property updates only when cache mode or policy must change".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmCache,
            operation: Operation::RemoveDevice,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "LVM cache detach must flush dirty cache state before uncaching"
                    .to_string(),
                alternatives: vec![
                    "wait for dirty data to drain before lvconvert --uncache".to_string(),
                    "keep cache media available until the origin LV is verified".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmCache,
            operation: Operation::Destroy,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "LVM cache removal detaches cache state from the origin LV".to_string(),
                alternatives: vec![
                    "set cache mode to writethrough before uncaching".to_string(),
                    "verify origin LV consistency after cache removal".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::CacheDevice,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "cache status refresh reads bcache sysfs state without changing attachment"
                    .to_string(),
                alternatives: vec![
                    "check dirty data before later cache detach or replacement".to_string(),
                    "use attach, detach, or property updates only when cache state must change"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::CacheDevice,
            operation: Operation::AddDevice,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "cache attachment must preserve backing data and cache identity"
                    .to_string(),
                alternatives: vec![
                    "attach an existing clean cache set instead of formatting a cache device"
                        .to_string(),
                    "verify backing and cache-set identity before enabling writeback".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::CacheDevice,
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            advice: Some(Advice {
                summary: "cache property changes should be staged toward safer modes first"
                    .to_string(),
                alternatives: vec![
                    "switch to writethrough or writearound before detaching cache media"
                        .to_string(),
                    "verify dirty data is zero before disabling a writeback cache".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::CacheDevice,
            operation: Operation::ReplaceDevice,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "cache replacement must flush or detach dirty cache state".to_string(),
                alternatives: vec![
                    "flush dirty data before replacing the cache device".to_string(),
                    "disable writeback before removing the source cache".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::CacheDevice,
            operation: Operation::RemoveDevice,
            risk: RiskClass::PotentialDataLoss,
            advice: Some(Advice {
                summary: "cache detachment must account for dirty data and backing-device safety"
                    .to_string(),
                alternatives: vec![
                    "switch writeback caches to writethrough before detach".to_string(),
                    "wait for dirty data to drain before removing cache media".to_string(),
                ],
            }),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destructive_zfs_dataset_destroy_has_advice() {
        let capabilities = default_capabilities();
        let capability = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::ZfsDataset
                    && capability.operation == Operation::Destroy
            })
            .expect("zfs dataset destroy capability should exist");

        assert_eq!(capability.risk, RiskClass::Destructive);
        assert!(capability.advice.is_some());
    }

    #[test]
    fn cache_device_capabilities_describe_safe_lifecycle_paths() {
        let capabilities = default_capabilities();
        let add = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::CacheDevice
                    && capability.operation == Operation::AddDevice
            })
            .expect("cache add capability should exist");
        let replace = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::CacheDevice
                    && capability.operation == Operation::ReplaceDevice
            })
            .expect("cache replace capability should exist");
        let remove = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::CacheDevice
                    && capability.operation == Operation::RemoveDevice
            })
            .expect("cache remove capability should exist");
        let rescan = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::CacheDevice
                    && capability.operation == Operation::Rescan
            })
            .expect("cache rescan capability should exist");

        assert_eq!(add.risk, RiskClass::Online);
        assert_eq!(replace.risk, RiskClass::OfflineRequired);
        assert_eq!(remove.risk, RiskClass::PotentialDataLoss);
        assert_eq!(rescan.risk, RiskClass::Online);
        assert!(replace.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("flush dirty data"))
        }));
    }

    #[test]
    fn lvm_cache_capabilities_describe_lifecycle_paths() {
        let capabilities = default_capabilities();
        let create = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::LvmCache
                    && capability.operation == Operation::Create
            })
            .expect("LVM cache create capability should exist");
        let add = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::LvmCache
                    && capability.operation == Operation::AddDevice
            })
            .expect("LVM cache add-device capability should exist");
        let set_property = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::LvmCache
                    && capability.operation == Operation::SetProperty
            })
            .expect("LVM cache property capability should exist");
        let rescan = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::LvmCache
                    && capability.operation == Operation::Rescan
            })
            .expect("LVM cache rescan capability should exist");
        let remove = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::LvmCache
                    && capability.operation == Operation::RemoveDevice
            })
            .expect("LVM cache remove-device capability should exist");

        assert_eq!(create.risk, RiskClass::OfflineRequired);
        assert_eq!(add.risk, RiskClass::OfflineRequired);
        assert_eq!(set_property.risk, RiskClass::Safe);
        assert_eq!(rescan.risk, RiskClass::Online);
        assert_eq!(remove.risk, RiskClass::OfflineRequired);
    }

    #[test]
    fn nfs_capabilities_describe_export_and_mount_lifecycle() {
        let capabilities = default_capabilities();
        let export_create = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::NfsExport
                    && capability.operation == Operation::Create
            })
            .expect("NFS export create capability should exist");
        let export = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::NfsExport
                    && capability.operation == Operation::Export
            })
            .expect("NFS export capability should exist");
        let export_destroy = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::NfsExport
                    && capability.operation == Operation::Destroy
            })
            .expect("NFS export destroy capability should exist");
        let unexport = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::NfsExport
                    && capability.operation == Operation::Unexport
            })
            .expect("NFS unexport capability should exist");
        let export_rescan = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::NfsExport
                    && capability.operation == Operation::Rescan
            })
            .expect("NFS export rescan capability should exist");
        let mount_create = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::NfsMount
                    && capability.operation == Operation::Create
            })
            .expect("NFS mount create capability should exist");
        let mount = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::NfsMount
                    && capability.operation == Operation::Mount
            })
            .expect("NFS mount capability should exist");
        let mount_destroy = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::NfsMount
                    && capability.operation == Operation::Destroy
            })
            .expect("NFS mount destroy capability should exist");
        let unmount = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::NfsMount
                    && capability.operation == Operation::Unmount
            })
            .expect("NFS unmount capability should exist");
        let mount_rescan = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::NfsMount
                    && capability.operation == Operation::Rescan
            })
            .expect("NFS mount rescan capability should exist");

        assert_eq!(export_create.risk, RiskClass::Online);
        assert_eq!(export.risk, RiskClass::Online);
        assert_eq!(export_destroy.risk, RiskClass::OfflineRequired);
        assert_eq!(unexport.risk, RiskClass::OfflineRequired);
        assert_eq!(export_rescan.risk, RiskClass::Online);
        assert_eq!(mount_create.risk, RiskClass::Online);
        assert_eq!(mount_rescan.risk, RiskClass::Online);
        assert_eq!(mount.risk, RiskClass::Online);
        assert_eq!(mount_destroy.risk, RiskClass::OfflineRequired);
        assert_eq!(unmount.risk, RiskClass::OfflineRequired);
        assert!(mount_destroy.advice.is_some());
        assert!(unmount.advice.is_some());
    }

    #[test]
    fn btrfs_qgroup_capabilities_describe_limit_lifecycle() {
        let capabilities = default_capabilities();
        let create = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::BtrfsQgroup
                    && capability.operation == Operation::Create
            })
            .expect("Btrfs qgroup create capability should exist");
        let update_limit = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::BtrfsQgroup
                    && capability.operation == Operation::SetProperty
            })
            .expect("Btrfs qgroup property capability should exist");
        let rescan = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::BtrfsQgroup
                    && capability.operation == Operation::Rescan
            })
            .expect("Btrfs qgroup rescan capability should exist");
        let destroy = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::BtrfsQgroup
                    && capability.operation == Operation::Destroy
            })
            .expect("Btrfs qgroup destroy capability should exist");

        assert_eq!(create.risk, RiskClass::Online);
        assert_eq!(update_limit.risk, RiskClass::Safe);
        assert_eq!(rescan.risk, RiskClass::Online);
        assert_eq!(destroy.risk, RiskClass::Destructive);
        assert!(destroy.advice.is_some());
    }

    #[test]
    fn lvm_physical_volume_capabilities_describe_lifecycle() {
        let capabilities = default_capabilities();
        let create = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::LvmPhysicalVolume
                    && capability.operation == Operation::Create
            })
            .expect("LVM physical volume create capability should exist");
        let grow = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::LvmPhysicalVolume
                    && capability.operation == Operation::Grow
            })
            .expect("LVM physical volume grow capability should exist");
        let rescan = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::LvmPhysicalVolume
                    && capability.operation == Operation::Rescan
            })
            .expect("LVM physical volume rescan capability should exist");
        let destroy = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::LvmPhysicalVolume
                    && capability.operation == Operation::Destroy
            })
            .expect("LVM physical volume destroy capability should exist");

        assert_eq!(create.risk, RiskClass::Destructive);
        assert_eq!(grow.risk, RiskClass::Online);
        assert_eq!(rescan.risk, RiskClass::Online);
        assert_eq!(destroy.risk, RiskClass::Destructive);
        assert!(destroy.advice.is_some());
    }

    #[test]
    fn luks_keyslot_capabilities_describe_header_lifecycle() {
        let capabilities = default_capabilities();
        let create = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::LuksContainer
                    && capability.operation == Operation::Create
            })
            .expect("LUKS keyslot create capability should exist");
        let add_key = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::LuksContainer
                    && capability.operation == Operation::AddKey
            })
            .expect("LUKS add-key capability should exist");
        let import_token = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::LuksContainer
                    && capability.operation == Operation::ImportToken
            })
            .expect("LUKS import-token capability should exist");
        let change = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::LuksContainer
                    && capability.operation == Operation::SetProperty
            })
            .expect("LUKS keyslot change capability should exist");
        let destroy = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::LuksContainer
                    && capability.operation == Operation::Destroy
            })
            .expect("LUKS keyslot destroy capability should exist");
        let remove_key = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::LuksContainer
                    && capability.operation == Operation::RemoveKey
            })
            .expect("LUKS remove-key capability should exist");
        let remove_token = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::LuksContainer
                    && capability.operation == Operation::RemoveToken
            })
            .expect("LUKS remove-token capability should exist");

        assert_eq!(create.risk, RiskClass::OfflineRequired);
        assert_eq!(add_key.risk, RiskClass::OfflineRequired);
        assert_eq!(import_token.risk, RiskClass::OfflineRequired);
        assert_eq!(change.risk, RiskClass::OfflineRequired);
        assert_eq!(destroy.risk, RiskClass::PotentialDataLoss);
        assert_eq!(remove_key.risk, RiskClass::PotentialDataLoss);
        assert_eq!(remove_token.risk, RiskClass::PotentialDataLoss);
    }

    #[test]
    fn iscsi_and_lun_capabilities_describe_host_lifecycle() {
        let capabilities = default_capabilities();
        let lun_create = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::Lun && capability.operation == Operation::Create
            })
            .expect("LUN create capability should exist");
        let lun_attach = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::Lun && capability.operation == Operation::Attach
            })
            .expect("LUN attach capability should exist");
        let lun_rescan = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::Lun && capability.operation == Operation::Rescan
            })
            .expect("LUN rescan capability should exist");
        let lun_destroy = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::Lun && capability.operation == Operation::Destroy
            })
            .expect("LUN destroy capability should exist");
        let lun_detach = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::Lun && capability.operation == Operation::Detach
            })
            .expect("LUN detach capability should exist");
        let session_create = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::IscsiSession
                    && capability.operation == Operation::Create
            })
            .expect("iSCSI session create capability should exist");
        let session_login = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::IscsiSession
                    && capability.operation == Operation::Login
            })
            .expect("iSCSI session login capability should exist");
        let session_rescan = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::IscsiSession
                    && capability.operation == Operation::Rescan
            })
            .expect("iSCSI session rescan capability should exist");
        let session_destroy = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::IscsiSession
                    && capability.operation == Operation::Destroy
            })
            .expect("iSCSI session destroy capability should exist");
        let session_logout = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::IscsiSession
                    && capability.operation == Operation::Logout
            })
            .expect("iSCSI session logout capability should exist");

        assert_eq!(lun_create.risk, RiskClass::Online);
        assert_eq!(lun_attach.risk, RiskClass::Online);
        assert_eq!(lun_rescan.risk, RiskClass::Online);
        assert_eq!(lun_destroy.risk, RiskClass::OfflineRequired);
        assert_eq!(lun_detach.risk, RiskClass::OfflineRequired);
        assert_eq!(session_create.risk, RiskClass::Online);
        assert_eq!(session_login.risk, RiskClass::Online);
        assert_eq!(session_rescan.risk, RiskClass::Online);
        assert_eq!(session_destroy.risk, RiskClass::OfflineRequired);
        assert_eq!(session_logout.risk, RiskClass::OfflineRequired);
        assert!(lun_destroy.advice.as_ref().is_some_and(|advice| {
            advice.summary.contains("without deleting target-side data")
        }));
    }

    #[test]
    fn nvme_namespace_capabilities_describe_controller_lifecycle() {
        let capabilities = default_capabilities();
        let create = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::NvmeNamespace
                    && capability.operation == Operation::Create
            })
            .expect("NVMe namespace create capability should exist");
        let grow = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::NvmeNamespace
                    && capability.operation == Operation::Grow
            })
            .expect("NVMe namespace grow capability should exist");
        let rescan = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::NvmeNamespace
                    && capability.operation == Operation::Rescan
            })
            .expect("NVMe namespace rescan capability should exist");
        let destroy = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::NvmeNamespace
                    && capability.operation == Operation::Destroy
            })
            .expect("NVMe namespace destroy capability should exist");

        assert_eq!(create.risk, RiskClass::Destructive);
        assert_eq!(grow.risk, RiskClass::OfflineRequired);
        assert_eq!(rescan.risk, RiskClass::Online);
        assert_eq!(destroy.risk, RiskClass::Destructive);
        assert!(create.advice.is_some());
    }

    #[test]
    fn zfs_clone_promotion_capabilities_are_advertised() {
        let capabilities = default_capabilities();
        for node_kind in [NodeKind::ZfsDataset, NodeKind::Zvol] {
            let capability = capabilities
                .iter()
                .find(|capability| {
                    capability.node_kind == node_kind && capability.operation == Operation::Promote
                })
                .unwrap_or_else(|| panic!("{node_kind} promote capability should exist"));

            assert_eq!(capability.risk, RiskClass::OfflineRequired);
            assert!(
                capability
                    .advice
                    .as_ref()
                    .is_some_and(|advice| { advice.summary.contains("promotion") })
            );
        }
    }

    #[test]
    fn snapshot_capabilities_cover_zfs_and_btrfs_lifecycle() {
        let capabilities = default_capabilities();
        let zfs_snapshot = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::ZfsSnapshot
                    && capability.operation == Operation::Snapshot
            })
            .expect("ZFS snapshot create capability should exist");
        let zfs_hold = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::ZfsSnapshot
                    && capability.operation == Operation::SetProperty
            })
            .expect("ZFS snapshot hold capability should exist");
        let zfs_rollback = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::ZfsSnapshot
                    && capability.operation == Operation::Rollback
            })
            .expect("ZFS snapshot rollback capability should exist");
        let zfs_clone = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::ZfsSnapshot
                    && capability.operation == Operation::Clone
            })
            .expect("ZFS snapshot clone capability should exist");
        let zfs_rescan = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::ZfsSnapshot
                    && capability.operation == Operation::Rescan
            })
            .expect("ZFS snapshot rescan capability should exist");
        let btrfs_snapshot = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::BtrfsSnapshot
                    && capability.operation == Operation::Snapshot
            })
            .expect("Btrfs snapshot create capability should exist");
        let btrfs_rescan = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::BtrfsSnapshot
                    && capability.operation == Operation::Rescan
            })
            .expect("Btrfs snapshot rescan capability should exist");
        let btrfs_destroy = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::BtrfsSnapshot
                    && capability.operation == Operation::Destroy
            })
            .expect("Btrfs snapshot destroy capability should exist");

        assert_eq!(zfs_snapshot.risk, RiskClass::Reversible);
        assert_eq!(zfs_hold.risk, RiskClass::Safe);
        assert_eq!(zfs_clone.risk, RiskClass::Reversible);
        assert_eq!(zfs_rescan.risk, RiskClass::Online);
        assert_eq!(zfs_rollback.risk, RiskClass::PotentialDataLoss);
        assert!(zfs_rollback.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("recursive rollback"))
        }));
        assert_eq!(btrfs_snapshot.risk, RiskClass::Reversible);
        assert_eq!(btrfs_rescan.risk, RiskClass::Online);
        assert_eq!(btrfs_destroy.risk, RiskClass::Destructive);
    }

    #[test]
    fn property_capabilities_cover_supported_update_domains() {
        let capabilities = default_capabilities();
        for node_kind in [
            NodeKind::Filesystem,
            NodeKind::BtrfsSubvolume,
            NodeKind::ZfsPool,
            NodeKind::ZfsDataset,
            NodeKind::Zvol,
        ] {
            let capability = capabilities
                .iter()
                .find(|capability| {
                    capability.node_kind == node_kind
                        && capability.operation == Operation::SetProperty
                })
                .unwrap_or_else(|| panic!("{node_kind} set-property capability should exist"));

            assert_eq!(capability.risk, RiskClass::Safe);
            assert!(capability.advice.is_some());
        }
    }

    #[test]
    fn btrfs_filesystem_capabilities_cover_device_topology_updates() {
        let capabilities = default_capabilities();
        for (operation, risk) in [
            (Operation::AddDevice, RiskClass::Online),
            (Operation::ReplaceDevice, RiskClass::OfflineRequired),
            (Operation::RemoveDevice, RiskClass::PotentialDataLoss),
        ] {
            let capability = capabilities
                .iter()
                .find(|capability| {
                    capability.node_kind == NodeKind::Filesystem
                        && capability.operation == operation
                })
                .unwrap_or_else(|| {
                    panic!("generic filesystem {operation:?} capability should exist")
                });
            assert_eq!(capability.risk, risk);
            assert!(capability.advice.is_some());
        }

        let add = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::BtrfsFilesystem
                    && capability.operation == Operation::AddDevice
            })
            .expect("Btrfs filesystem add-device capability should exist");
        let replace = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::BtrfsFilesystem
                    && capability.operation == Operation::ReplaceDevice
            })
            .expect("Btrfs filesystem replace-device capability should exist");
        let remove = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::BtrfsFilesystem
                    && capability.operation == Operation::RemoveDevice
            })
            .expect("Btrfs filesystem remove-device capability should exist");

        assert_eq!(add.risk, RiskClass::Online);
        assert_eq!(replace.risk, RiskClass::OfflineRequired);
        assert_eq!(remove.risk, RiskClass::PotentialDataLoss);
        assert!(replace.advice.is_some());
    }

    #[test]
    fn capability_inventory_covers_rendered_topology_updates() {
        let capabilities = default_capabilities();
        for (node_kind, operation, risk) in [
            (
                NodeKind::ZfsPool,
                Operation::ReplaceDevice,
                RiskClass::OfflineRequired,
            ),
            (
                NodeKind::ZfsPool,
                Operation::RemoveDevice,
                RiskClass::PotentialDataLoss,
            ),
            (
                NodeKind::LvmLogicalVolume,
                Operation::Grow,
                RiskClass::Online,
            ),
            (
                NodeKind::LvmLogicalVolume,
                Operation::Rescan,
                RiskClass::Online,
            ),
            (NodeKind::Swap, Operation::Rescan, RiskClass::Online),
            (
                NodeKind::BtrfsSubvolume,
                Operation::Rescan,
                RiskClass::Online,
            ),
            (NodeKind::ZfsDataset, Operation::Rescan, RiskClass::Online),
            (NodeKind::Zvol, Operation::Rescan, RiskClass::Online),
            (NodeKind::BtrfsQgroup, Operation::Rescan, RiskClass::Online),
            (NodeKind::LvmVolumeGroup, Operation::Grow, RiskClass::Online),
            (
                NodeKind::LvmVolumeGroup,
                Operation::Rescan,
                RiskClass::Online,
            ),
            (
                NodeKind::LvmVolumeGroup,
                Operation::RemoveDevice,
                RiskClass::PotentialDataLoss,
            ),
            (NodeKind::LvmThinPool, Operation::Rescan, RiskClass::Online),
            (NodeKind::LvmSnapshot, Operation::Rescan, RiskClass::Online),
            (NodeKind::LoopDevice, Operation::Rescan, RiskClass::Online),
            (NodeKind::CacheDevice, Operation::Rescan, RiskClass::Online),
            (NodeKind::VdoVolume, Operation::Rescan, RiskClass::Online),
            (NodeKind::ZfsSnapshot, Operation::Rescan, RiskClass::Online),
            (
                NodeKind::BtrfsSnapshot,
                Operation::Rescan,
                RiskClass::Online,
            ),
            (NodeKind::MdRaid, Operation::Create, RiskClass::Destructive),
            (
                NodeKind::MdRaid,
                Operation::Grow,
                RiskClass::OfflineRequired,
            ),
            (NodeKind::MdRaid, Operation::Rescan, RiskClass::Online),
            (NodeKind::MdRaid, Operation::Destroy, RiskClass::Destructive),
            (
                NodeKind::MultipathDevice,
                Operation::RemoveDevice,
                RiskClass::PotentialDataLoss,
            ),
        ] {
            let capability = capabilities
                .iter()
                .find(|capability| {
                    capability.node_kind == node_kind && capability.operation == operation
                })
                .unwrap_or_else(|| panic!("{node_kind} {operation:?} capability should exist"));

            assert_eq!(capability.risk, risk);
            assert!(capability.advice.is_some());
        }
    }

    #[test]
    fn plan_warns_for_shrink_and_disabled_preservation() {
        let plan = plan_from_json_bytes(
            br#"{
              "spec": {
                "filesystems": {
                  "home": {
                    "mountpoint": "/home",
                    "fsType": "xfs",
                    "resizePolicy": "shrink-allowed",
                    "preserveData": false
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 2);
        assert_eq!(plan.summary.destructive_count, 1);
        assert_eq!(plan.summary.potential_data_loss_count, 0);
        assert_eq!(plan.summary.unsupported_count, 1);
        assert!(plan.actions.iter().any(|action| {
            action.operation == Operation::Shrink
                && action.risk == RiskClass::Unsupported
                && action
                    .advice
                    .as_ref()
                    .is_some_and(|advice| advice.summary.contains("XFS"))
        }));
    }

    #[test]
    fn plan_keeps_ext_shrink_as_potential_data_loss() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "home": {
                  "mountpoint": "/home",
                  "fsType": "ext4",
                  "resizePolicy": "shrink-allowed"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 1);
        assert_eq!(plan.summary.potential_data_loss_count, 1);
        assert_eq!(plan.summary.unsupported_count, 0);
        assert_eq!(plan.actions[0].risk, RiskClass::PotentialDataLoss);
        assert_eq!(plan.actions[0].context.fs_type.as_deref(), Some("ext4"));
        assert_eq!(plan.actions[0].context.mountpoint.as_deref(), Some("/home"));
    }

    #[test]
    fn plan_carries_filesystem_device_for_lifecycle_actions() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "home": {
                  "mountpoint": "/home",
                  "device": "/dev/disk/by-label/home",
                  "fsType": "ext4",
                  "resizePolicy": "shrink-allowed",
                  "desiredSize": "100G"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 1);
        assert_eq!(plan.actions[0].operation, Operation::Shrink);
        assert_eq!(
            plan.actions[0].context.device.as_deref(),
            Some("/dev/disk/by-label/home")
        );
        assert_eq!(plan.actions[0].context.target.as_deref(), Some("/home"));
        assert_eq!(
            plan.actions[0].context.desired_size.as_deref(),
            Some("100G")
        );
    }

    #[test]
    fn plan_filesystem_properties_keep_filesystem_context() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "data": {
                  "mountpoint": "/data",
                  "device": "/dev/disk/by-label/data",
                  "fsType": "btrfs",
                  "properties": {
                    "label": "bulk-data",
                    "compression": "zstd"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 3);
        assert_eq!(plan.summary.unsupported_count, 1);
        let action = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:data:set-property:label")
            .expect("filesystem label property action should exist");

        assert_eq!(action.operation, Operation::SetProperty);
        assert_eq!(action.risk, RiskClass::Safe);
        assert_eq!(action.context.target.as_deref(), Some("/data"));
        assert_eq!(
            action.context.device.as_deref(),
            Some("/dev/disk/by-label/data")
        );
        assert_eq!(action.context.fs_type.as_deref(), Some("btrfs"));
        assert_eq!(action.context.mountpoint.as_deref(), Some("/data"));
        assert_eq!(action.context.property.as_deref(), Some("label"));
        assert_eq!(action.context.property_value.as_deref(), Some("bulk-data"));

        let unsupported = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:data:set-property:compression")
            .expect("unsupported filesystem property action should exist");
        assert_eq!(unsupported.operation, Operation::SetProperty);
        assert_eq!(unsupported.risk, RiskClass::Unsupported);
        assert!(unsupported.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("ZFS dataset"))
        }));
    }

    #[test]
    fn plan_accepts_xfs_filesystem_label_property() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "scratch": {
                  "mountpoint": "/scratch",
                  "device": "/dev/disk/by-label/scratch",
                  "fsType": "xfs",
                  "properties": {
                    "xfs.label": "scratch-new",
                    "xfs.reflink": "1"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 3);
        assert_eq!(plan.summary.unsupported_count, 1);
        let label = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:scratch:set-property:xfs.label")
            .expect("XFS label property action should exist");

        assert_eq!(label.operation, Operation::SetProperty);
        assert_eq!(label.risk, RiskClass::Safe);
        assert_eq!(label.context.target.as_deref(), Some("/scratch"));
        assert_eq!(
            label.context.device.as_deref(),
            Some("/dev/disk/by-label/scratch")
        );
        assert_eq!(label.context.fs_type.as_deref(), Some("xfs"));
        assert_eq!(label.context.property_value.as_deref(), Some("scratch-new"));

        let unsupported = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:scratch:set-property:xfs.reflink")
            .expect("unsupported XFS property action should exist");
        assert_eq!(unsupported.risk, RiskClass::Unsupported);
    }

    #[test]
    fn plan_accepts_fat_label_and_volume_id_properties() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "efi": {
                  "mountpoint": "/boot",
                  "device": "/dev/disk/by-partlabel/EFI",
                  "fsType": "vfat",
                  "properties": {
                    "vfat.label": "NIXBOOT",
                    "vfat.uuid": "A1B2-C3D4",
                    "fat.volume-id": "not-a-fat-id"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 4);
        assert_eq!(plan.summary.offline_required_count, 1);
        assert_eq!(plan.summary.unsupported_count, 1);

        let label = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:efi:set-property:vfat.label")
            .expect("FAT label property action should exist");
        assert_eq!(label.operation, Operation::SetProperty);
        assert_eq!(label.risk, RiskClass::Safe);
        assert_eq!(label.context.fs_type.as_deref(), Some("vfat"));
        assert_eq!(label.context.property_value.as_deref(), Some("NIXBOOT"));

        let volume_id = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:efi:set-property:vfat.uuid")
            .expect("FAT volume ID property action should exist");
        assert_eq!(volume_id.risk, RiskClass::OfflineRequired);
        assert!(volume_id.advice.as_ref().is_some_and(|advice| {
            advice.summary.contains("UUID")
                && advice
                    .alternatives
                    .iter()
                    .any(|alternative| alternative.contains("NixOS fileSystems"))
        }));

        let invalid_volume_id = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:efi:set-property:fat.volume-id")
            .expect("invalid FAT volume ID property action should exist");
        assert_eq!(invalid_volume_id.risk, RiskClass::Unsupported);
        assert!(invalid_volume_id.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("8-hex-digit FAT volume ID"))
        }));
    }

    #[test]
    fn plan_accepts_ntfs_label_and_volume_serial_properties() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "windows": {
                  "mountpoint": "/mnt/windows",
                  "device": "/dev/disk/by-label/Windows",
                  "fsType": "ntfs",
                  "properties": {
                    "ntfs.label": "Windows",
                    "ntfs.uuid": "01234567-89abcdef",
                    "ntfs.volume-serial": "not-a-serial"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 4);
        assert_eq!(plan.summary.offline_required_count, 1);
        assert_eq!(plan.summary.unsupported_count, 1);

        let label = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:windows:set-property:ntfs.label")
            .expect("NTFS label property action should exist");
        assert_eq!(label.operation, Operation::SetProperty);
        assert_eq!(label.risk, RiskClass::Safe);
        assert_eq!(label.context.fs_type.as_deref(), Some("ntfs"));
        assert_eq!(label.context.property_value.as_deref(), Some("Windows"));

        let serial = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:windows:set-property:ntfs.uuid")
            .expect("NTFS serial property action should exist");
        assert_eq!(serial.risk, RiskClass::OfflineRequired);
        assert!(serial.advice.as_ref().is_some_and(|advice| {
            advice.summary.contains("UUID")
                && advice
                    .alternatives
                    .iter()
                    .any(|alternative| alternative.contains("NixOS fileSystems"))
        }));

        let invalid_serial = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:windows:set-property:ntfs.volume-serial")
            .expect("invalid NTFS serial property action should exist");
        assert_eq!(invalid_serial.risk, RiskClass::Unsupported);
        assert!(invalid_serial.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("16-hex-digit NTFS volume serial"))
        }));
    }

    #[test]
    fn plan_accepts_exfat_label_and_volume_serial_properties() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "shared": {
                  "mountpoint": "/mnt/shared",
                  "device": "/dev/disk/by-label/Shared",
                  "fsType": "exfat",
                  "properties": {
                    "exfat.label": "Shared",
                    "exfat.uuid": "A1B2-C3D4",
                    "exfat.volume-serial": "not-a-serial"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 4);
        assert_eq!(plan.summary.offline_required_count, 1);
        assert_eq!(plan.summary.unsupported_count, 1);

        let label = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:shared:set-property:exfat.label")
            .expect("exFAT label property action should exist");
        assert_eq!(label.operation, Operation::SetProperty);
        assert_eq!(label.risk, RiskClass::Safe);
        assert_eq!(label.context.fs_type.as_deref(), Some("exfat"));
        assert_eq!(label.context.property_value.as_deref(), Some("Shared"));

        let serial = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:shared:set-property:exfat.uuid")
            .expect("exFAT serial property action should exist");
        assert_eq!(serial.risk, RiskClass::OfflineRequired);
        assert!(serial.advice.as_ref().is_some_and(|advice| {
            advice.summary.contains("UUID")
                && advice
                    .alternatives
                    .iter()
                    .any(|alternative| alternative.contains("NixOS fileSystems"))
        }));

        let invalid_serial = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:shared:set-property:exfat.volume-serial")
            .expect("invalid exFAT serial property action should exist");
        assert_eq!(invalid_serial.risk, RiskClass::Unsupported);
        assert!(invalid_serial.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("8-hex-digit exFAT volume serial"))
        }));
    }

    #[test]
    fn plan_accepts_f2fs_label_property() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "mobile": {
                  "mountpoint": "/mnt/mobile",
                  "device": "/dev/disk/by-label/mobile",
                  "fsType": "f2fs",
                  "properties": {
                    "f2fs.label": "mobile-new",
                    "f2fs.uuid": "11111111-2222-3333-4444-555555555555"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 3);
        assert_eq!(plan.summary.unsupported_count, 1);

        let label = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:mobile:set-property:f2fs.label")
            .expect("F2FS label property action should exist");
        assert_eq!(label.operation, Operation::SetProperty);
        assert_eq!(label.risk, RiskClass::Safe);
        assert_eq!(label.context.fs_type.as_deref(), Some("f2fs"));
        assert_eq!(label.context.property_value.as_deref(), Some("mobile-new"));

        let unsupported = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:mobile:set-property:f2fs.uuid")
            .expect("unsupported F2FS property action should exist");
        assert_eq!(unsupported.risk, RiskClass::Unsupported);
        assert!(unsupported.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("f2fs.label"))
        }));
    }

    #[test]
    fn plan_classifies_filesystem_uuid_updates_as_offline_required() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "home": {
                  "mountpoint": "/home",
                  "device": "/dev/disk/by-label/home",
                  "fsType": "ext4",
                  "properties": {
                    "ext.uuid": "11111111-2222-3333-4444-555555555555"
                  }
                },
                "scratch": {
                  "mountpoint": "/scratch",
                  "device": "/dev/disk/by-label/scratch",
                  "fsType": "xfs",
                  "properties": {
                    "filesystem.uuid": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
                  }
                },
                "data": {
                  "mountpoint": "/data",
                  "device": "/dev/disk/by-label/data",
                  "fsType": "btrfs",
                  "properties": {
                    "btrfs.uuid": "bbbbbbbb-1111-2222-3333-cccccccccccc"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 6);
        assert_eq!(plan.summary.offline_required_count, 3);
        assert_eq!(plan.summary.unsupported_count, 0);
        let ext_uuid = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:home:set-property:ext.uuid")
            .expect("Ext UUID property action should exist");
        assert_eq!(ext_uuid.operation, Operation::SetProperty);
        assert_eq!(ext_uuid.risk, RiskClass::OfflineRequired);
        assert_eq!(
            ext_uuid.context.property_value.as_deref(),
            Some("11111111-2222-3333-4444-555555555555")
        );
        assert!(ext_uuid.advice.as_ref().is_some_and(|advice| {
            advice.summary.contains("UUID")
                && advice
                    .alternatives
                    .iter()
                    .any(|alternative| alternative.contains("NixOS fileSystems"))
        }));

        let xfs_uuid = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:scratch:set-property:filesystem.uuid")
            .expect("XFS UUID property action should exist");
        assert_eq!(xfs_uuid.risk, RiskClass::OfflineRequired);
        assert_eq!(xfs_uuid.context.fs_type.as_deref(), Some("xfs"));

        let btrfs_uuid = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:data:set-property:btrfs.uuid")
            .expect("Btrfs UUID property action should exist");
        assert_eq!(btrfs_uuid.risk, RiskClass::OfflineRequired);
        assert_eq!(btrfs_uuid.context.fs_type.as_deref(), Some("btrfs"));
        assert_eq!(
            btrfs_uuid.context.property_value.as_deref(),
            Some("bbbbbbbb-1111-2222-3333-cccccccccccc")
        );
    }

    #[test]
    fn plan_warns_for_filesystem_device_removal() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "data": {
                  "mountpoint": "/data",
                  "fsType": "btrfs",
                  "removeDevices": ["/dev/disk/by-id/old-btrfs-device"]
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 2);
        assert_eq!(plan.summary.potential_data_loss_count, 1);
        assert!(plan.actions.iter().any(|action| {
            action.id == "filesystems:data:remove-device:/dev/disk/by-id/old-btrfs-device"
                && action.operation == Operation::RemoveDevice
                && action.risk == RiskClass::PotentialDataLoss
                && action.context.collection.as_deref() == Some("filesystems")
                && action.context.target.as_deref() == Some("/data")
                && action.context.device.as_deref() == Some("/dev/disk/by-id/old-btrfs-device")
                && action.advice.is_some()
        }));
    }

    #[test]
    fn plan_accepts_filesystem_rebalance_with_filters() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "data": {
                  "mountpoint": "/data",
                  "fsType": "btrfs",
                  "operation": "rebalance",
                  "properties": {
                    "balance.data": "usage=50",
                    "balance.metadata": "usage=75"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 2);
        assert_eq!(plan.summary.unsupported_count, 0);
        let action = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:data:rebalance")
            .expect("filesystem rebalance action exists");

        assert_eq!(action.operation, Operation::Rebalance);
        assert_eq!(action.risk, RiskClass::Online);
        assert_eq!(action.context.collection.as_deref(), Some("filesystems"));
        assert_eq!(action.context.target.as_deref(), Some("/data"));
        assert_eq!(
            action.context.property_assignments,
            vec![
                "balance.data=usage=50".to_string(),
                "balance.metadata=usage=75".to_string()
            ]
        );
    }

    #[test]
    fn plan_accepts_filesystem_check_and_repair_operations() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "home": {
                  "mountpoint": "/home",
                  "device": "/dev/disk/by-label/home",
                  "fsType": "ext4",
                  "operation": "check"
                },
                "data": {
                  "mountpoint": "/data",
                  "device": "/dev/disk/by-label/data",
                  "fsType": "btrfs",
                  "operation": "repair"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 4);
        assert_eq!(plan.summary.offline_required_count, 2);
        assert_eq!(plan.summary.unsupported_count, 0);
        let check = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:home:check")
            .expect("filesystem check action exists");
        assert_eq!(check.operation, Operation::Check);
        assert_eq!(check.risk, RiskClass::OfflineRequired);
        assert!(!check.destructive);
        assert_eq!(check.context.fs_type.as_deref(), Some("ext4"));
        assert_eq!(
            check.context.device.as_deref(),
            Some("/dev/disk/by-label/home")
        );

        let repair = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:data:repair")
            .expect("filesystem repair action exists");
        assert_eq!(repair.operation, Operation::Repair);
        assert_eq!(repair.risk, RiskClass::OfflineRequired);
        assert!(!repair.destructive);
        assert_eq!(repair.context.fs_type.as_deref(), Some("btrfs"));
        assert!(
            repair
                .advice
                .as_ref()
                .is_some_and(|advice| { advice.summary.contains("repair mutates metadata") })
        );
    }

    #[test]
    fn plan_accepts_scrub_lifecycle_for_btrfs_and_pools() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "data": {
                  "mountpoint": "/data",
                  "fsType": "btrfs",
                  "operation": "scrub"
                },
                "archive": {
                  "mountpoint": "/archive",
                  "fsType": "ext4",
                  "operation": "scrub"
                }
              },
              "pools": {
                "tank": {
                  "operation": "scrub"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 5);
        assert_eq!(plan.summary.unsupported_count, 1);
        let btrfs = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:data:scrub")
            .expect("Btrfs scrub action exists");
        assert_eq!(btrfs.operation, Operation::Scrub);
        assert_eq!(btrfs.risk, RiskClass::Online);
        assert_eq!(btrfs.context.target.as_deref(), Some("/data"));

        let pool = plan
            .actions
            .iter()
            .find(|action| action.id == "pools:tank:scrub")
            .expect("pool scrub action exists");
        assert_eq!(pool.operation, Operation::Scrub);
        assert_eq!(pool.risk, RiskClass::Online);

        let unsupported = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:archive:scrub")
            .expect("unsupported filesystem scrub action exists");
        assert_eq!(unsupported.risk, RiskClass::Unsupported);
        assert!(
            unsupported
                .advice
                .as_ref()
                .is_some_and(|advice| { advice.summary.contains("available for Btrfs") })
        );
    }

    #[test]
    fn plan_accepts_filesystem_trim_operation() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "scratch": {
                  "mountpoint": "/scratch",
                  "fsType": "xfs",
                  "operation": "trim"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 2);
        assert_eq!(plan.summary.offline_required_count, 0);
        assert_eq!(plan.summary.unsupported_count, 0);
        let trim = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:scratch:trim")
            .expect("filesystem trim action exists");
        assert_eq!(trim.operation, Operation::Trim);
        assert_eq!(trim.risk, RiskClass::Online);
        assert_eq!(trim.context.target.as_deref(), Some("/scratch"));
        assert!(
            trim.advice
                .as_ref()
                .is_some_and(|advice| { advice.summary.contains("discards unused blocks") })
        );
    }

    #[test]
    fn plan_accepts_filesystem_remount_operation() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "scratch": {
                  "mountpoint": "/scratch",
                  "fsType": "xfs",
                  "operation": "remount",
                  "options": ["rw", "noatime", "discard=async"]
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 2);
        assert_eq!(plan.summary.offline_required_count, 0);
        assert_eq!(plan.summary.unsupported_count, 0);
        let remount = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:scratch:remount")
            .expect("filesystem remount action exists");
        assert_eq!(remount.operation, Operation::Remount);
        assert_eq!(remount.risk, RiskClass::Online);
        assert_eq!(remount.context.target.as_deref(), Some("/scratch"));
        assert_eq!(
            remount.context.options.as_deref(),
            Some("rw,noatime,discard=async")
        );
        assert!(
            remount
                .advice
                .as_ref()
                .is_some_and(|advice| advice.summary.contains("updates local mount options"))
        );
    }

    #[test]
    fn plan_accepts_filesystem_mount_lifecycle() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "backup": {
                  "mountpoint": "/backup",
                  "device": "/dev/disk/by-label/backup",
                  "fsType": "xfs",
                  "operation": "mount",
                  "options": ["rw", "noatime"]
                },
                "archive": {
                  "mountpoint": "/archive",
                  "device": "/dev/disk/by-label/archive",
                  "fsType": "ext4",
                  "operation": "unmount"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 4);
        assert_eq!(plan.summary.offline_required_count, 1);
        assert_eq!(plan.summary.unsupported_count, 0);
        assert_eq!(plan.summary.destructive_count, 0);
        let mount = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:backup:mount")
            .expect("filesystem mount action exists");
        assert_eq!(mount.operation, Operation::Mount);
        assert_eq!(mount.risk, RiskClass::Online);
        assert_eq!(
            mount.context.device.as_deref(),
            Some("/dev/disk/by-label/backup")
        );
        assert_eq!(mount.context.mountpoint.as_deref(), Some("/backup"));
        assert_eq!(mount.context.fs_type.as_deref(), Some("xfs"));
        assert_eq!(mount.context.options.as_deref(), Some("rw,noatime"));

        let unmount = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:archive:unmount")
            .expect("filesystem unmount action exists");
        assert_eq!(unmount.operation, Operation::Unmount);
        assert_eq!(unmount.risk, RiskClass::OfflineRequired);
        assert!(!unmount.destructive);
        assert_eq!(unmount.context.mountpoint.as_deref(), Some("/archive"));
    }

    #[test]
    fn plan_carries_desired_size_context_for_resize_actions() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "home": {
                  "mountpoint": "/home",
                  "fsType": "btrfs",
                  "resizePolicy": "grow-only",
                  "desiredSize": "750GiB"
                }
              },
              "volumes": {
                "vg/home": {
                  "operation": "grow",
                  "size": "800GiB"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        let filesystem = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystem:home:grow")
            .expect("filesystem grow action exists");
        assert_eq!(filesystem.context.desired_size.as_deref(), Some("750GiB"));

        let volume = plan
            .actions
            .iter()
            .find(|action| action.id == "volumes:vg/home:grow")
            .expect("volume grow action exists");
        assert_eq!(volume.context.desired_size.as_deref(), Some("800GiB"));
    }

    #[test]
    fn plan_classifies_lvm_logical_volume_lifecycle() {
        let plan = plan_from_json_bytes(
            br#"{
              "volumes": {
                "vg0/scratch": {
                  "operation": "create",
                  "desiredSize": "10GiB"
                },
                "vg0/home": {
                  "operation": "activate"
                },
                "vg0/archive": {
                  "operation": "deactivate"
                },
                "vg0/reporting": {
                  "operation": "rescan"
                },
                "vg0/old": {
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 5);
        assert_eq!(plan.summary.offline_required_count, 2);
        assert_eq!(plan.summary.destructive_count, 1);
        let create = plan
            .actions
            .iter()
            .find(|action| action.id == "volumes:vg0/scratch:create")
            .expect("LV create action exists");
        assert_eq!(create.risk, RiskClass::Online);
        assert_eq!(create.context.desired_size.as_deref(), Some("10GiB"));
        let destroy = plan
            .actions
            .iter()
            .find(|action| action.id == "volumes:vg0/old:destroy")
            .expect("LV destroy action exists");
        assert_eq!(destroy.risk, RiskClass::Destructive);
        assert!(destroy.destructive);
        let activate = plan
            .actions
            .iter()
            .find(|action| action.id == "volumes:vg0/home:activate")
            .expect("LV activate action exists");
        assert_eq!(activate.risk, RiskClass::OfflineRequired);
        assert!(!activate.destructive);
        let deactivate = plan
            .actions
            .iter()
            .find(|action| action.id == "volumes:vg0/archive:deactivate")
            .expect("LV deactivate action exists");
        assert_eq!(deactivate.risk, RiskClass::OfflineRequired);
        assert!(!deactivate.destructive);
        let rescan = plan
            .actions
            .iter()
            .find(|action| action.id == "volumes:vg0/reporting:rescan")
            .expect("LV rescan action exists");
        assert_eq!(rescan.operation, Operation::Rescan);
        assert_eq!(rescan.risk, RiskClass::Online);
        assert!(!rescan.destructive);
    }

    #[test]
    fn plan_classifies_lvm_physical_volume_lifecycle() {
        let plan = plan_from_json_bytes(
            br#"{
              "physicalVolumes": {
                "/dev/disk/by-id/nvme-pv-new": {
                  "operation": "create"
                },
                "/dev/disk/by-id/nvme-pv-grow": {
                  "operation": "grow"
                },
                "/dev/disk/by-id/nvme-pv-old": {
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 3);
        assert_eq!(plan.summary.destructive_count, 2);
        let create = plan
            .actions
            .iter()
            .find(|action| action.id == "physicalvolumes:/dev/disk/by-id/nvme-pv-new:create")
            .expect("PV create action exists");
        assert_eq!(create.risk, RiskClass::Destructive);
        assert!(create.destructive);
        let grow = plan
            .actions
            .iter()
            .find(|action| action.id == "physicalvolumes:/dev/disk/by-id/nvme-pv-grow:grow")
            .expect("PV grow action exists");
        assert_eq!(grow.risk, RiskClass::Online);
        let destroy = plan
            .actions
            .iter()
            .find(|action| action.id == "physicalvolumes:/dev/disk/by-id/nvme-pv-old:destroy")
            .expect("PV destroy action exists");
        assert_eq!(destroy.risk, RiskClass::Destructive);
        assert!(destroy.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("pvmove"))
        }));
    }

    #[test]
    fn plan_classifies_lvm_volume_group_lifecycle() {
        let plan = plan_from_json_bytes(
            br#"{
              "volumeGroups": {
                "vg0": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/nvme-vg0"
                },
                "vgdata": {
                  "replaceDevices": {
                    "/dev/disk/by-id/old-pv": "/dev/disk/by-id/new-pv"
                  }
                },
                "importvg": {
                  "operation": "import"
                },
                "exportvg": {
                  "operation": "export"
                },
                "activevg": {
                  "operation": "activate"
                },
                "coldvg": {
                  "operation": "deactivate"
                },
                "oldvg": {
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 7);
        assert_eq!(plan.summary.offline_required_count, 5);
        assert_eq!(plan.summary.destructive_count, 2);
        let create = plan
            .actions
            .iter()
            .find(|action| action.id == "volumegroups:vg0:create")
            .expect("volume group create action exists");
        assert_eq!(create.risk, RiskClass::Destructive);
        assert!(create.destructive);
        assert_eq!(
            create.context.device.as_deref(),
            Some("/dev/disk/by-id/nvme-vg0")
        );
        assert!(create.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("pvs"))
        }));
        let destroy = plan
            .actions
            .iter()
            .find(|action| action.id == "volumegroups:oldvg:destroy")
            .expect("volume group destroy action exists");
        assert_eq!(destroy.risk, RiskClass::Destructive);

        let replace = plan
            .actions
            .iter()
            .find(|action| action.id == "volumeGroups:vgdata:replace-device:/dev/disk/by-id/old-pv")
            .expect("volume group replacement action exists");
        assert_eq!(replace.risk, RiskClass::OfflineRequired);
        assert_eq!(
            replace.context.device.as_deref(),
            Some("/dev/disk/by-id/old-pv")
        );
        assert_eq!(
            replace.context.replacement.as_deref(),
            Some("/dev/disk/by-id/new-pv")
        );
        assert!(
            replace.advice.as_ref().is_some_and(|advice| {
                advice.summary.contains("migrate extents before vgreduce")
            })
        );
        let import = plan
            .actions
            .iter()
            .find(|action| action.id == "volumegroups:importvg:import")
            .expect("volume group import action exists");
        assert_eq!(import.risk, RiskClass::OfflineRequired);
        assert!(!import.destructive);
        assert!(import.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("vgimport"))
        }));
        let export = plan
            .actions
            .iter()
            .find(|action| action.id == "volumegroups:exportvg:export")
            .expect("volume group export action exists");
        assert_eq!(export.risk, RiskClass::OfflineRequired);
        assert!(!export.destructive);
        let activate = plan
            .actions
            .iter()
            .find(|action| action.id == "volumegroups:activevg:activate")
            .expect("volume group activate action exists");
        assert_eq!(activate.risk, RiskClass::OfflineRequired);
        assert!(!activate.destructive);
        let deactivate = plan
            .actions
            .iter()
            .find(|action| action.id == "volumegroups:coldvg:deactivate")
            .expect("volume group deactivate action exists");
        assert_eq!(deactivate.risk, RiskClass::OfflineRequired);
        assert!(!deactivate.destructive);
    }

    #[test]
    fn plan_classifies_disk_and_partition_lifecycle_safely() {
        let plan = plan_from_json_bytes(
            br#"{
              "disks": {
                "/dev/disk/by-id/nvme-root": {
                  "operation": "create",
                  "partitionType": "gpt"
                },
                "/dev/disk/by-id/nvme-data": {
                  "operation": "rescan"
                }
              },
              "partitions": {
                "root": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/nvme-root",
                  "start": "1MiB",
                  "end": "100%",
                  "partitionType": "linux"
                },
                "home": {
                  "operation": "grow",
                  "device": "/dev/disk/by-id/nvme-root",
                  "partitionNumber": 2,
                  "end": "100%"
                },
                "data-table": {
                  "operation": "rescan",
                  "device": "/dev/disk/by-id/nvme-data"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 5);
        assert_eq!(plan.summary.offline_required_count, 2);
        assert_eq!(plan.summary.destructive_count, 1);

        let root = plan
            .actions
            .iter()
            .find(|action| action.id == "partitions:root:create")
            .expect("partition create action exists");
        assert_eq!(root.risk, RiskClass::OfflineRequired);
        assert_eq!(
            root.context.device.as_deref(),
            Some("/dev/disk/by-id/nvme-root")
        );
        assert_eq!(root.context.start.as_deref(), Some("1MiB"));
        assert_eq!(root.context.end.as_deref(), Some("100%"));
        assert_eq!(root.context.partition_type.as_deref(), Some("linux"));

        let home = plan
            .actions
            .iter()
            .find(|action| action.id == "partitions:home:grow")
            .expect("partition grow action exists");
        assert_eq!(home.risk, RiskClass::OfflineRequired);
        assert_eq!(
            home.context.device.as_deref(),
            Some("/dev/disk/by-id/nvme-root")
        );
        assert_eq!(home.context.partition_number.as_deref(), Some("2"));
        assert_eq!(home.context.end.as_deref(), Some("100%"));

        let disk = plan
            .actions
            .iter()
            .find(|action| action.id == "disks:/dev/disk/by-id/nvme-root:create")
            .expect("disk create action exists");
        assert_eq!(disk.risk, RiskClass::Destructive);
        assert!(disk.destructive);

        let disk_rescan = plan
            .actions
            .iter()
            .find(|action| action.id == "disks:/dev/disk/by-id/nvme-data:rescan")
            .expect("disk rescan action exists");
        assert_eq!(disk_rescan.risk, RiskClass::Online);
        assert!(!disk_rescan.destructive);

        let partition_rescan = plan
            .actions
            .iter()
            .find(|action| action.id == "partitions:data-table:rescan")
            .expect("partition rescan action exists");
        assert_eq!(partition_rescan.risk, RiskClass::Online);
        assert_eq!(
            partition_rescan.context.device.as_deref(),
            Some("/dev/disk/by-id/nvme-data")
        );
    }

    #[test]
    fn plan_classifies_swap_and_luks_lifecycle_safely() {
        let plan = plan_from_json_bytes(
            br#"{
              "swaps": {
                "primary": {
                  "device": "/dev/disk/by-label/swap",
                  "preserveData": false
                },
                "scratch": {
                  "device": "/swapfile",
                  "operation": "grow",
                  "desiredSize": "16GiB"
                },
                "inventory": {
                  "device": "/dev/disk/by-label/swap-inventory",
                  "operation": "rescan"
                }
              },
              "luks": {
                "devices": {
                  "cryptroot": {
                    "name": "cryptroot",
                    "device": "/dev/disk/by-partuuid/root",
                    "operation": "grow"
                  },
                  "cryptdata": {
                    "name": "cryptdata",
                    "device": "/dev/disk/by-id/data-luks",
                    "operation": "create"
                  },
                  "cryptarchive": {
                    "name": "cryptarchive",
                    "device": "/dev/disk/by-id/archive-luks",
                    "operation": "open",
                    "preserveData": false
                  },
                  "cryptmissing": {
                    "name": "cryptmissing",
                    "operation": "create"
                  },
                  "cryptscratch": {
                    "name": "cryptscratch",
                    "device": "/dev/disk/by-id/scratch",
                    "preserveData": false
                  },
                  "cryptold": {
                    "name": "cryptold",
                    "device": "/dev/disk/by-id/old-luks",
                    "operation": "destroy"
                  },
                  "cryptclosed": {
                    "name": "cryptclosed",
                    "device": "/dev/disk/by-id/closed-luks",
                    "operation": "close"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 10);
        assert_eq!(plan.summary.offline_required_count, 7);
        assert_eq!(plan.summary.destructive_count, 2);

        let swap = plan
            .actions
            .iter()
            .find(|action| action.id == "swaps:primary:format")
            .expect("swap format action exists");
        assert_eq!(swap.risk, RiskClass::Destructive);
        assert_eq!(
            swap.context.device.as_deref(),
            Some("/dev/disk/by-label/swap")
        );

        let swap_rescan = plan
            .actions
            .iter()
            .find(|action| action.id == "swaps:inventory:rescan")
            .expect("swap rescan action exists");
        assert_eq!(swap_rescan.operation, Operation::Rescan);
        assert_eq!(swap_rescan.risk, RiskClass::Online);
        assert!(!swap_rescan.destructive);
        assert_eq!(
            swap_rescan.context.device.as_deref(),
            Some("/dev/disk/by-label/swap-inventory")
        );

        let luks = plan
            .actions
            .iter()
            .find(|action| action.id == "luks.devices:cryptroot:grow")
            .expect("luks grow action exists");
        assert_eq!(luks.risk, RiskClass::OfflineRequired);
        assert_eq!(luks.context.target.as_deref(), Some("cryptroot"));
        assert_eq!(
            luks.context.device.as_deref(),
            Some("/dev/disk/by-partuuid/root")
        );

        let open = plan
            .actions
            .iter()
            .find(|action| action.id == "luks.devices:cryptdata:create")
            .expect("luks open action exists");
        assert_eq!(open.risk, RiskClass::OfflineRequired);
        assert!(!open.destructive);
        assert_eq!(open.context.target.as_deref(), Some("cryptdata"));
        assert_eq!(
            open.context.device.as_deref(),
            Some("/dev/disk/by-id/data-luks")
        );

        let explicit_open = plan
            .actions
            .iter()
            .find(|action| action.id == "luks.devices:cryptarchive:open")
            .expect("explicit luks open action exists");
        assert_eq!(explicit_open.operation, Operation::Open);
        assert_eq!(explicit_open.risk, RiskClass::OfflineRequired);
        assert!(!explicit_open.destructive);
        assert_eq!(
            explicit_open.context.device.as_deref(),
            Some("/dev/disk/by-id/archive-luks")
        );

        let missing = plan
            .actions
            .iter()
            .find(|action| action.id == "luks.devices:cryptmissing:create")
            .expect("underspecified luks open action exists");
        assert_eq!(missing.risk, RiskClass::OfflineRequired);
        assert_eq!(missing.context.target.as_deref(), Some("cryptmissing"));
        assert_eq!(missing.context.device, None);

        let close = plan
            .actions
            .iter()
            .find(|action| action.id == "luks.devices:cryptold:destroy")
            .expect("luks close action exists");
        assert_eq!(close.risk, RiskClass::OfflineRequired);
        assert!(!close.destructive);
        assert_eq!(close.context.target.as_deref(), Some("cryptold"));
        assert_eq!(
            close.context.device.as_deref(),
            Some("/dev/disk/by-id/old-luks")
        );

        let explicit_close = plan
            .actions
            .iter()
            .find(|action| action.id == "luks.devices:cryptclosed:close")
            .expect("explicit luks close action exists");
        assert_eq!(explicit_close.operation, Operation::Close);
        assert_eq!(explicit_close.risk, RiskClass::OfflineRequired);
        assert!(!explicit_close.destructive);
        assert_eq!(
            explicit_close.context.target.as_deref(),
            Some("cryptclosed")
        );
    }

    #[test]
    fn plan_accepts_swap_label_and_uuid_properties() {
        let plan = plan_from_json_bytes(
            br#"{
              "swaps": {
                "primary": {
                  "device": "/dev/disk/by-label/swap-old",
                  "properties": {
                    "label": "swap-new",
                    "swap.uuid": "01234567-89ab-cdef-0123-456789abcdef",
                    "priority": "10"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 4);
        assert_eq!(plan.summary.offline_required_count, 2);
        assert_eq!(plan.summary.unsupported_count, 1);

        let label = plan
            .actions
            .iter()
            .find(|action| action.id == "swaps:primary:set-property:label")
            .expect("swap label action exists");
        assert_eq!(label.operation, Operation::SetProperty);
        assert_eq!(label.risk, RiskClass::OfflineRequired);
        assert_eq!(label.context.property_value.as_deref(), Some("swap-new"));
        assert!(label.advice.as_ref().is_some_and(|advice| {
            advice
                .summary
                .contains("swap label and UUID updates mutate swap signature identity")
        }));

        let uuid = plan
            .actions
            .iter()
            .find(|action| action.id == "swaps:primary:set-property:swap.uuid")
            .expect("swap UUID action exists");
        assert_eq!(uuid.risk, RiskClass::OfflineRequired);
        assert_eq!(
            uuid.context.property_value.as_deref(),
            Some("01234567-89ab-cdef-0123-456789abcdef")
        );

        let unsupported = plan
            .actions
            .iter()
            .find(|action| action.id == "swaps:primary:set-property:priority")
            .expect("unsupported swap property action exists");
        assert_eq!(unsupported.risk, RiskClass::Unsupported);
        assert!(unsupported.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("swap.label"))
        }));
    }

    #[test]
    fn plan_accepts_luks_header_identity_properties() {
        let plan = plan_from_json_bytes(
            br#"{
              "luks": {
                "devices": {
                  "cryptroot": {
                    "name": "cryptroot",
                    "device": "/dev/disk/by-id/root-luks",
                    "properties": {
                      "label": "root",
                      "luks.subsystem": "nixos",
                      "luks.uuid": "01234567-89ab-cdef-0123-456789abcdef",
                      "priority": "prefer"
                    }
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.unsupported_count, 1);

        let label = plan
            .actions
            .iter()
            .find(|action| action.id == "luks.devices:cryptroot:set-property:label")
            .expect("LUKS label property action exists");
        assert_eq!(label.operation, Operation::SetProperty);
        assert_eq!(label.risk, RiskClass::OfflineRequired);
        assert_eq!(label.context.target.as_deref(), Some("cryptroot"));
        assert_eq!(
            label.context.device.as_deref(),
            Some("/dev/disk/by-id/root-luks")
        );
        assert_eq!(label.context.property_value.as_deref(), Some("root"));

        let subsystem = plan
            .actions
            .iter()
            .find(|action| action.id == "luks.devices:cryptroot:set-property:luks.subsystem")
            .expect("LUKS subsystem property action exists");
        assert_eq!(subsystem.risk, RiskClass::OfflineRequired);
        assert_eq!(subsystem.context.property_value.as_deref(), Some("nixos"));

        let uuid = plan
            .actions
            .iter()
            .find(|action| action.id == "luks.devices:cryptroot:set-property:luks.uuid")
            .expect("LUKS UUID property action exists");
        assert_eq!(uuid.risk, RiskClass::OfflineRequired);
        assert_eq!(
            uuid.context.property_value.as_deref(),
            Some("01234567-89ab-cdef-0123-456789abcdef")
        );

        let unsupported = plan
            .actions
            .iter()
            .find(|action| action.id == "luks.devices:cryptroot:set-property:priority")
            .expect("unsupported LUKS property action exists");
        assert_eq!(unsupported.risk, RiskClass::Unsupported);
        assert!(unsupported.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("luksKeyslots or luksTokens"))
        }));
    }

    #[test]
    fn plan_classifies_luks_keyslot_lifecycle() {
        let plan = plan_from_json_bytes(
            br#"{
              "luksKeyslots": {
                "cryptroot:1": {
                  "operation": "add-key",
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "keySlot": "1",
                    "newKeyFile": "/run/keys/root-new"
                  }
                },
                "cryptroot:2": {
                  "operation": "remove-key",
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "keySlot": "2"
                  }
                },
                "cryptroot:3": {
                  "properties": {
                    "keyFile": "/run/keys/root-rotated"
                  },
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "keySlot": "3",
                    "keyFile": "/run/keys/root-old"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 3);
        assert_eq!(plan.summary.offline_required_count, 2);
        assert_eq!(plan.summary.potential_data_loss_count, 1);
        let create = plan
            .actions
            .iter()
            .find(|action| action.id == "lukskeyslots:cryptroot:1:add-key")
            .expect("LUKS keyslot add-key action exists");
        assert_eq!(create.risk, RiskClass::OfflineRequired);
        assert_eq!(
            create.context.device.as_deref(),
            Some("/dev/disk/by-id/root-luks")
        );
        assert_eq!(create.context.key_slot.as_deref(), Some("1"));
        assert_eq!(
            create.context.new_key_file.as_deref(),
            Some("/run/keys/root-new")
        );

        let destroy = plan
            .actions
            .iter()
            .find(|action| action.id == "lukskeyslots:cryptroot:2:remove-key")
            .expect("LUKS keyslot remove-key action exists");
        assert_eq!(destroy.risk, RiskClass::PotentialDataLoss);
        assert!(!destroy.destructive);

        let change = plan
            .actions
            .iter()
            .find(|action| action.id == "luksKeyslots:cryptroot:3:set-property:keyFile")
            .expect("LUKS keyslot change action exists");
        assert_eq!(change.risk, RiskClass::OfflineRequired);
        assert_eq!(change.context.key_slot.as_deref(), Some("3"));
        assert_eq!(
            change.context.key_file.as_deref(),
            Some("/run/keys/root-old")
        );
        assert_eq!(
            change.context.property_value.as_deref(),
            Some("/run/keys/root-rotated")
        );
    }

    #[test]
    fn plan_classifies_luks_token_lifecycle() {
        let plan = plan_from_json_bytes(
            br#"{
              "luksTokens": {
                "cryptroot:0": {
                  "operation": "import-token",
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "tokenId": "0",
                    "tokenFile": "/run/keys/root-token.json"
                  }
                },
                "cryptroot:1": {
                  "operation": "remove-token",
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "tokenId": "1"
                  }
                },
                "cryptroot:2": {
                  "properties": {
                    "tokenFile": "/run/keys/root-token-new.json"
                  },
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "tokenId": "2"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 3);
        assert_eq!(plan.summary.offline_required_count, 2);
        assert_eq!(plan.summary.potential_data_loss_count, 1);
        let create = plan
            .actions
            .iter()
            .find(|action| action.id == "lukstokens:cryptroot:0:import-token")
            .expect("LUKS token import-token action exists");
        assert_eq!(create.risk, RiskClass::OfflineRequired);
        assert_eq!(
            create.context.device.as_deref(),
            Some("/dev/disk/by-id/root-luks")
        );
        assert_eq!(create.context.token_id.as_deref(), Some("0"));
        assert_eq!(
            create.context.token_file.as_deref(),
            Some("/run/keys/root-token.json")
        );

        let destroy = plan
            .actions
            .iter()
            .find(|action| action.id == "lukstokens:cryptroot:1:remove-token")
            .expect("LUKS token remove-token action exists");
        assert_eq!(destroy.risk, RiskClass::PotentialDataLoss);
        assert!(!destroy.destructive);

        let change = plan
            .actions
            .iter()
            .find(|action| action.id == "luksTokens:cryptroot:2:set-property:tokenFile")
            .expect("LUKS token change action exists");
        assert_eq!(change.risk, RiskClass::OfflineRequired);
        assert_eq!(change.context.token_id.as_deref(), Some("2"));
        assert_eq!(
            change.context.property_value.as_deref(),
            Some("/run/keys/root-token-new.json")
        );
    }

    #[test]
    fn plan_classifies_vdo_lifecycle_with_vdo_advice() {
        let plan = plan_from_json_bytes(
            br#"{
              "vdoVolumes": {
                "new-cache": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/vdo-backing",
                  "desiredSize": "2TiB"
                },
                "archive": {
                  "operation": "grow",
                  "desiredSize": "4TiB",
                  "properties": {
                    "writePolicy": "sync",
                    "compression": "enabled",
                    "deduplication": "disabled"
                  }
                },
                "warmArchive": {
                  "operation": "start"
                },
                "coldArchive": {
                  "operation": "stop"
                },
                "refreshArchive": {
                  "operation": "rescan"
                },
                "old-cache": {
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 9);
        assert_eq!(plan.summary.offline_required_count, 2);
        assert_eq!(plan.summary.destructive_count, 2);
        let create = plan
            .actions
            .iter()
            .find(|action| action.id == "vdovolumes:new-cache:create")
            .expect("VDO create action exists");
        assert_eq!(create.risk, RiskClass::Destructive);
        assert!(create.destructive);
        assert_eq!(
            create.context.device.as_deref(),
            Some("/dev/disk/by-id/vdo-backing")
        );
        assert_eq!(create.context.desired_size.as_deref(), Some("2TiB"));
        let grow = plan
            .actions
            .iter()
            .find(|action| action.id == "vdovolumes:archive:grow")
            .expect("VDO grow action exists");
        assert_eq!(grow.risk, RiskClass::Online);
        assert!(grow.advice.as_ref().is_some_and(|advice| {
            advice.summary.contains("logical size")
                && advice
                    .alternatives
                    .iter()
                    .any(|alternative| alternative.contains("vdostats"))
        }));
        let write_policy = plan
            .actions
            .iter()
            .find(|action| action.id == "vdoVolumes:archive:set-property:writePolicy")
            .expect("VDO write policy property action exists");
        assert_eq!(write_policy.risk, RiskClass::Safe);
        assert_eq!(write_policy.context.property_value.as_deref(), Some("sync"));
        assert!(plan.actions.iter().any(|action| {
            action.id == "vdoVolumes:archive:set-property:compression"
                && action.risk == RiskClass::Safe
        }));
        assert!(plan.actions.iter().any(|action| {
            action.id == "vdoVolumes:archive:set-property:deduplication"
                && action.risk == RiskClass::Safe
        }));
        let start = plan
            .actions
            .iter()
            .find(|action| action.id == "vdovolumes:warmarchive:start")
            .expect("VDO start action exists");
        assert_eq!(start.operation, Operation::Start);
        assert_eq!(start.risk, RiskClass::OfflineRequired);
        assert!(!start.destructive);
        assert!(start.advice.as_ref().is_some_and(|advice| {
            advice.summary.contains("activates")
                && advice
                    .alternatives
                    .iter()
                    .any(|alternative| alternative.contains("backing device"))
        }));
        let stop = plan
            .actions
            .iter()
            .find(|action| action.id == "vdovolumes:coldarchive:stop")
            .expect("VDO stop action exists");
        assert_eq!(stop.operation, Operation::Stop);
        assert_eq!(stop.risk, RiskClass::OfflineRequired);
        assert!(!stop.destructive);
        assert!(stop.advice.as_ref().is_some_and(|advice| {
            advice.summary.contains("preserving VDO metadata")
                && advice
                    .alternatives
                    .iter()
                    .any(|alternative| alternative.contains("stop over remove"))
        }));
        let rescan = plan
            .actions
            .iter()
            .find(|action| action.id == "vdovolumes:refresharchive:rescan")
            .expect("VDO rescan action exists");
        assert_eq!(rescan.operation, Operation::Rescan);
        assert_eq!(rescan.risk, RiskClass::Online);
        assert!(!rescan.destructive);
        let destroy = plan
            .actions
            .iter()
            .find(|action| action.id == "vdovolumes:old-cache:destroy")
            .expect("VDO destroy action exists");
        assert_eq!(destroy.risk, RiskClass::Destructive);
    }

    #[test]
    fn plan_rejects_unsupported_vdo_property_updates() {
        let plan = plan_from_json_bytes(
            br#"{
              "vdoVolumes": {
                "archive": {
                  "properties": {
                    "writePolicy": "eventual",
                    "compression": "maybe",
                    "deduplication": "off",
                    "indexMemory": "0.5"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 4);
        assert_eq!(plan.summary.unsupported_count, 3);
        assert!(plan.actions.iter().any(|action| {
            action.id == "vdoVolumes:archive:set-property:deduplication"
                && action.risk == RiskClass::Safe
        }));

        let write_policy = plan
            .actions
            .iter()
            .find(|action| action.id == "vdoVolumes:archive:set-property:writePolicy")
            .expect("VDO write policy property action exists");
        assert_eq!(write_policy.risk, RiskClass::Unsupported);
        assert!(write_policy.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("auto, sync, or async"))
        }));

        assert!(plan.actions.iter().any(|action| {
            action.id == "vdoVolumes:archive:set-property:compression"
                && action.risk == RiskClass::Unsupported
        }));
        assert!(plan.actions.iter().any(|action| {
            action.id == "vdoVolumes:archive:set-property:indexMemory"
                && action.risk == RiskClass::Unsupported
        }));
    }

    #[test]
    fn plan_accepts_btrfs_subvolume_lifecycle_with_target_path() {
        let plan = plan_from_json_bytes(
            br#"{
              "btrfsSubvolumes": {
                "/mnt/persist/@home": {
                  "operation": "create",
                  "path": "/mnt/persist/@home"
                },
                "/mnt/persist/@inventory": {
                  "operation": "rescan",
                  "path": "/mnt/persist/@inventory"
                },
                "/mnt/persist/@old": {
                  "destroy": true,
                  "preserveData": false
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 3);
        let create = plan
            .actions
            .iter()
            .find(|action| {
                action.id == "btrfsSubvolumes:/mnt/persist/@home:create".to_ascii_lowercase()
            })
            .expect("create action exists");
        assert_eq!(create.risk, RiskClass::Online);
        assert_eq!(create.context.target.as_deref(), Some("/mnt/persist/@home"));
        let rescan = plan
            .actions
            .iter()
            .find(|action| {
                action.id == "btrfsSubvolumes:/mnt/persist/@inventory:rescan".to_ascii_lowercase()
            })
            .expect("rescan action exists");
        assert_eq!(rescan.operation, Operation::Rescan);
        assert_eq!(rescan.risk, RiskClass::Online);
        assert!(!rescan.destructive);
        assert_eq!(
            rescan.context.target.as_deref(),
            Some("/mnt/persist/@inventory")
        );
        let destroy = plan
            .actions
            .iter()
            .find(|action| {
                action.id == "btrfsSubvolumes:/mnt/persist/@old:destroy".to_ascii_lowercase()
            })
            .expect("destroy action exists");
        assert_eq!(destroy.risk, RiskClass::Destructive);
        assert!(destroy.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("read-only snapshot"))
        }));
    }

    #[test]
    fn plan_accepts_btrfs_qgroup_rescan_as_online_refresh() {
        let plan = plan_from_json_bytes(
            br#"{
              "btrfsQgroups": {
                "0/257": {
                  "operation": "rescan",
                  "target": "/mnt/persist"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 1);
        assert_eq!(plan.summary.offline_required_count, 0);
        assert_eq!(plan.summary.destructive_count, 0);
        let rescan = plan
            .actions
            .iter()
            .find(|action| action.id == "btrfsqgroups:0/257:rescan")
            .expect("Btrfs qgroup rescan action exists");
        assert_eq!(rescan.operation, Operation::Rescan);
        assert_eq!(rescan.risk, RiskClass::Online);
        assert!(!rescan.destructive);
        assert_eq!(rescan.context.target.as_deref(), Some("/mnt/persist"));
        assert!(
            rescan
                .advice
                .as_ref()
                .is_some_and(|advice| { advice.summary.contains("Btrfs qgroup rescan refreshes") })
        );
    }

    #[test]
    fn plan_classifies_btrfs_subvolume_property_support() {
        let plan = plan_from_json_bytes(
            br#"{
              "btrfsSubvolumes": {
                "/mnt/persist/@home": {
                  "path": "/mnt/persist/@home",
                  "properties": {
                    "readonly": true,
                    "compression": "zstd"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 2);
        assert_eq!(plan.summary.unsupported_count, 1);
        let readonly = plan
            .actions
            .iter()
            .find(|action| action.id == "btrfsSubvolumes:/mnt/persist/@home:set-property:readonly")
            .expect("readonly property action exists");
        assert_eq!(readonly.risk, RiskClass::Safe);

        let compression = plan
            .actions
            .iter()
            .find(|action| {
                action.id == "btrfsSubvolumes:/mnt/persist/@home:set-property:compression"
            })
            .expect("unsupported property action exists");
        assert_eq!(compression.risk, RiskClass::Unsupported);
        assert!(compression.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("readOnly"))
        }));
    }

    #[test]
    fn plan_accepts_zvol_lifecycle_with_zfs_advice() {
        let plan = plan_from_json_bytes(
            br#"{
              "zvols": {
                "tank/vm/root": {
                  "operation": "grow",
                  "desiredSize": "80GiB"
                },
                "tank/vm/tmp": {
                  "operation": "create",
                  "desiredSize": "20GiB"
                },
                "tank/vm/inventory": {
                  "operation": "rescan"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 3);
        assert_eq!(plan.summary.offline_required_count, 0);
        let grow = plan
            .actions
            .iter()
            .find(|action| action.id == "zvols:tank/vm/root:grow")
            .expect("zvol grow action exists");
        assert_eq!(grow.risk, RiskClass::Online);
        assert_eq!(grow.context.desired_size.as_deref(), Some("80GiB"));
        assert!(grow.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("rescan dependent"))
        }));
        let create = plan
            .actions
            .iter()
            .find(|action| action.id == "zvols:tank/vm/tmp:create")
            .expect("zvol create action exists");
        assert_eq!(create.risk, RiskClass::Online);
        let rescan = plan
            .actions
            .iter()
            .find(|action| action.id == "zvols:tank/vm/inventory:rescan")
            .expect("zvol rescan action exists");
        assert_eq!(rescan.operation, Operation::Rescan);
        assert_eq!(rescan.risk, RiskClass::Online);
        assert!(!rescan.destructive);
    }

    #[test]
    fn plan_classifies_zfs_dataset_lifecycle() {
        let plan = plan_from_json_bytes(
            br#"{
              "datasets": {
                "tank/home": {
                  "operation": "create",
                  "properties": {
                    "compression": "zstd",
                    "mountpoint": "/home"
                  }
                },
                "tank/inventory": {
                  "operation": "rescan"
                },
                "tank/archive": {
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 5);
        assert_eq!(plan.summary.destructive_count, 1);
        let create = plan
            .actions
            .iter()
            .find(|action| action.id == "datasets:tank/home:create")
            .expect("dataset create action exists");
        assert_eq!(create.risk, RiskClass::Online);
        assert_eq!(
            create.context.property_assignments,
            vec![
                "compression=zstd".to_string(),
                "mountpoint=/home".to_string()
            ]
        );
        assert!(create.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("mountpoint"))
        }));
        let rescan = plan
            .actions
            .iter()
            .find(|action| action.id == "datasets:tank/inventory:rescan")
            .expect("dataset rescan action exists");
        assert_eq!(rescan.operation, Operation::Rescan);
        assert_eq!(rescan.risk, RiskClass::Online);
        assert!(!rescan.destructive);
        let destroy = plan
            .actions
            .iter()
            .find(|action| action.id == "datasets:tank/archive:destroy")
            .expect("dataset destroy action exists");
        assert_eq!(destroy.risk, RiskClass::Destructive);
        assert!(destroy.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("recursive snapshot"))
        }));
    }

    #[test]
    fn plan_classifies_md_raid_lifecycle_with_redundancy_advice() {
        let plan = plan_from_json_bytes(
            br#"{
              "mdRaids": {
                "newroot": {
                  "target": "/dev/md/newroot",
                  "operation": "create",
                  "level": "1",
                  "devices": [
                    "/dev/disk/by-id/nvme-a",
                    "/dev/disk/by-id/nvme-b"
                  ]
                },
                "existing": {
                  "target": "/dev/md/existing",
                  "operation": "assemble",
                  "devices": [
                    "/dev/disk/by-id/existing-a",
                    "/dev/disk/by-id/existing-b"
                  ]
                },
                "oldroot": {
                  "target": "/dev/md/oldroot",
                  "operation": "stop"
                },
                "inventory": {
                  "operation": "rescan"
                },
                "root": {
                  "target": "/dev/md/root",
                  "operation": "grow",
                  "desiredSize": "max",
                  "addDevices": ["/dev/disk/by-id/nvme-spare"],
                  "replaceDevices": {
                    "/dev/disk/by-id/old-md-member": "/dev/disk/by-id/new-md-member"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 7);
        assert_eq!(plan.summary.destructive_count, 1);
        assert_eq!(plan.summary.offline_required_count, 4);
        let create = plan
            .actions
            .iter()
            .find(|action| action.id == "mdraids:newroot:create")
            .expect("md create action exists");
        assert_eq!(create.risk, RiskClass::Destructive);
        assert_eq!(create.context.level.as_deref(), Some("1"));
        assert_eq!(
            create.context.devices,
            vec![
                "/dev/disk/by-id/nvme-a".to_string(),
                "/dev/disk/by-id/nvme-b".to_string(),
            ]
        );
        let assemble = plan
            .actions
            .iter()
            .find(|action| action.id == "mdraids:existing:assemble")
            .expect("md assemble action exists");
        assert_eq!(assemble.operation, Operation::Assemble);
        assert_eq!(assemble.risk, RiskClass::OfflineRequired);
        assert!(!assemble.destructive);
        assert_eq!(assemble.context.target.as_deref(), Some("/dev/md/existing"));
        assert_eq!(
            assemble.context.devices,
            vec![
                "/dev/disk/by-id/existing-a".to_string(),
                "/dev/disk/by-id/existing-b".to_string(),
            ]
        );
        let stop = plan
            .actions
            .iter()
            .find(|action| action.id == "mdraids:oldroot:stop")
            .expect("md stop action exists");
        assert_eq!(stop.operation, Operation::Stop);
        assert_eq!(stop.risk, RiskClass::OfflineRequired);
        assert!(!stop.destructive);
        let rescan = plan
            .actions
            .iter()
            .find(|action| action.id == "mdraids:inventory:rescan")
            .expect("md rescan action exists");
        assert_eq!(rescan.operation, Operation::Rescan);
        assert_eq!(rescan.risk, RiskClass::Online);
        assert!(!rescan.destructive);
        let grow = plan
            .actions
            .iter()
            .find(|action| action.id == "mdraids:root:grow")
            .expect("md grow action exists");
        assert_eq!(grow.risk, RiskClass::OfflineRequired);
        assert_eq!(grow.context.target.as_deref(), Some("/dev/md/root"));
        assert!(grow.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("/proc/mdstat"))
        }));
        let add = plan
            .actions
            .iter()
            .find(|action| action.id == "mdRaids:root:add-device:/dev/disk/by-id/nvme-spare")
            .expect("md add action exists");
        assert_eq!(add.risk, RiskClass::Online);
        let replace = plan
            .actions
            .iter()
            .find(|action| action.id == "mdRaids:root:replace-device:/dev/disk/by-id/old-md-member")
            .expect("md replace action exists");
        assert_eq!(replace.risk, RiskClass::OfflineRequired);
    }

    #[test]
    fn plan_classifies_multipath_map_lifecycle_with_path_advice() {
        let plan = plan_from_json_bytes(
            br#"{
              "multipathMaps": {
                "mpatha": {
                  "target": "mpatha",
                  "operation": "grow",
                  "addDevices": ["/dev/sdb"],
                  "replaceDevices": {
                    "/dev/sdc": "/dev/sdd"
                  }
                },
                "mpathb": {
                  "target": "mpathb",
                  "operation": "rescan"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 4);
        assert_eq!(plan.summary.offline_required_count, 1);
        let grow = plan
            .actions
            .iter()
            .find(|action| action.id == "multipathmaps:mpatha:grow")
            .expect("multipath grow action exists");
        assert_eq!(grow.risk, RiskClass::Online);
        assert!(grow.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("rescan"))
        }));
        let add = plan
            .actions
            .iter()
            .find(|action| action.id == "multipathMaps:mpatha:add-device:/dev/sdb")
            .expect("multipath add action exists");
        assert_eq!(add.risk, RiskClass::Online);
        let replace = plan
            .actions
            .iter()
            .find(|action| action.id == "multipathMaps:mpatha:replace-device:/dev/sdc")
            .expect("multipath replace action exists");
        assert_eq!(replace.risk, RiskClass::OfflineRequired);
        let rescan = plan
            .actions
            .iter()
            .find(|action| action.id == "multipathmaps:mpathb:rescan")
            .expect("multipath rescan action exists");
        assert_eq!(rescan.risk, RiskClass::Online);
        assert!(rescan.advice.as_ref().is_some_and(|advice| {
            advice
                .summary
                .contains("refreshes existing storage paths without deleting target data")
        }));
    }

    #[test]
    fn plan_classifies_thin_pool_lifecycle_with_metadata_advice() {
        let plan = plan_from_json_bytes(
            br#"{
              "thinPools": {
                "vg0/newpool": {
                  "operation": "create",
                  "desiredSize": "100GiB"
                },
                "vg0/pool": {
                  "operation": "grow",
                  "desiredSize": "500GiB"
                },
                "vg0/reporting": {
                  "operation": "rescan"
                },
                "vg0/oldpool": {
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 4);
        assert_eq!(plan.summary.offline_required_count, 0);
        assert_eq!(plan.summary.destructive_count, 1);
        let create = plan
            .actions
            .iter()
            .find(|action| action.id == "thinpools:vg0/newpool:create")
            .expect("thin pool create action exists");
        assert_eq!(create.risk, RiskClass::Online);
        assert_eq!(create.context.desired_size.as_deref(), Some("100GiB"));
        let grow = plan
            .actions
            .iter()
            .find(|action| action.id == "thinpools:vg0/pool:grow")
            .expect("thin pool grow action exists");
        assert_eq!(grow.id, "thinpools:vg0/pool:grow");
        assert_eq!(grow.risk, RiskClass::Online);
        assert_eq!(grow.context.desired_size.as_deref(), Some("500GiB"));
        assert!(grow.advice.as_ref().is_some_and(|advice| {
            advice.summary.contains("metadata")
                && advice
                    .alternatives
                    .iter()
                    .any(|alternative| alternative.contains("overcommit"))
        }));
        let rescan = plan
            .actions
            .iter()
            .find(|action| action.id == "thinpools:vg0/reporting:rescan")
            .expect("thin pool rescan action exists");
        assert_eq!(rescan.operation, Operation::Rescan);
        assert_eq!(rescan.risk, RiskClass::Online);
        assert!(!rescan.destructive);
        assert!(
            rescan
                .advice
                .as_ref()
                .is_some_and(|advice| { advice.summary.contains("thin pool rescan refreshes") })
        );
        let destroy = plan
            .actions
            .iter()
            .find(|action| action.id == "thinpools:vg0/oldpool:destroy")
            .expect("thin pool destroy action exists");
        assert_eq!(destroy.risk, RiskClass::Destructive);
        assert!(destroy.destructive);
    }

    #[test]
    fn plan_classifies_lvm_snapshot_lifecycle() {
        let plan = plan_from_json_bytes(
            br#"{
              "lvmSnapshots": {
                "vg0/root-snap": {
                  "operation": "snapshot",
                  "target": "vg0/root",
                  "desiredSize": "20GiB"
                },
                "vg0/root-rollback": {
                  "operation": "rollback"
                },
                "vg0/root-inspect": {
                  "operation": "rescan"
                },
                "vg0/old-snap": {
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 4);
        assert_eq!(plan.summary.potential_data_loss_count, 1);
        assert_eq!(plan.summary.destructive_count, 1);
        let snapshot = plan
            .actions
            .iter()
            .find(|action| action.id == "lvmsnapshots:vg0/root-snap:snapshot")
            .expect("snapshot action exists");
        assert_eq!(snapshot.risk, RiskClass::Reversible);
        assert_eq!(snapshot.context.target.as_deref(), Some("vg0/root"));
        assert_eq!(snapshot.context.desired_size.as_deref(), Some("20GiB"));
        let rollback = plan
            .actions
            .iter()
            .find(|action| action.id == "lvmsnapshots:vg0/root-rollback:rollback")
            .expect("rollback action exists");
        assert_eq!(rollback.risk, RiskClass::PotentialDataLoss);
        assert!(
            rollback
                .advice
                .as_ref()
                .is_some_and(|advice| advice.summary.contains("rolls the origin back"))
        );
        let rescan = plan
            .actions
            .iter()
            .find(|action| action.id == "lvmsnapshots:vg0/root-inspect:rescan")
            .expect("rescan action exists");
        assert_eq!(rescan.operation, Operation::Rescan);
        assert_eq!(rescan.risk, RiskClass::Online);
        assert!(!rescan.destructive);
        assert!(
            rescan
                .advice
                .as_ref()
                .is_some_and(|advice| { advice.summary.contains("LVM snapshot rescan refreshes") })
        );
    }

    #[test]
    fn plan_classifies_loop_device_lifecycle() {
        let plan = plan_from_json_bytes(
            br#"{
              "loopDevices": {
                "/dev/loop7": {
                  "operation": "create",
                  "device": "/var/lib/images/root.img"
                },
                "/dev/loop8": {
                  "operation": "grow"
                },
                "/dev/loop10": {
                  "operation": "rescan"
                },
                "/dev/loop9": {
                  "operation": "destroy"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 4);
        assert_eq!(plan.summary.offline_required_count, 1);
        assert_eq!(plan.summary.destructive_count, 0);
        let create = plan
            .actions
            .iter()
            .find(|action| action.id == "loopdevices:/dev/loop7:create")
            .expect("loop create action exists");
        assert_eq!(create.risk, RiskClass::Online);
        assert_eq!(
            create.context.device.as_deref(),
            Some("/var/lib/images/root.img")
        );
        let rescan = plan
            .actions
            .iter()
            .find(|action| action.id == "loopdevices:/dev/loop10:rescan")
            .expect("loop rescan action exists");
        assert_eq!(rescan.operation, Operation::Rescan);
        assert_eq!(rescan.risk, RiskClass::Online);
        assert!(!rescan.destructive);
        let destroy = plan
            .actions
            .iter()
            .find(|action| action.id == "loopdevices:/dev/loop9:destroy")
            .expect("loop destroy action exists");
        assert_eq!(destroy.risk, RiskClass::OfflineRequired);
        assert!(!destroy.destructive);
    }

    #[test]
    fn topology_comparison_reports_current_state_diagnostics() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "home": {
                  "mountpoint": "/home",
                  "fsType": "btrfs",
                  "resizePolicy": "grow-only",
                  "desiredSize": "750GiB"
                }
              },
              "datasets": {
                "tank/home": {
                  "properties": {
                    "compression": "zstd"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("filesystem:/home", NodeKind::Filesystem, "/home")
                .with_path("/home")
                .with_size_bytes(500 * 1024 * 1024 * 1024)
                .with_property("filesystem.type", "ext4"),
        );
        graph.add_node(
            Node::new("zfs:dataset:tank/home", NodeKind::ZfsDataset, "tank/home")
                .with_property("compression", "zstd"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 2);
        assert_eq!(comparison.summary.matched_count, 2);
        assert_eq!(comparison.summary.missing_count, 0);
        assert_eq!(comparison.summary.type_conflict_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.kind == TopologyDiagnosticKind::SizeBelowDesired
                && diagnostic.action_id == "filesystem:home:grow"
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.kind == TopologyDiagnosticKind::FilesystemTypeConflict
                && diagnostic.action_id == "filesystem:home:grow"
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.kind == TopologyDiagnosticKind::PropertyAlreadySatisfied
                && diagnostic.action_id == "datasets:tank/home:set-property:compression"
        }));
    }

    #[test]
    fn topology_comparison_reports_missing_targets() {
        let plan = plan_from_json_bytes(
            br#"{
              "volumes": {
                "vg/missing": {
                  "operation": "grow",
                  "desiredSize": "50GiB"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        let plan = compare_plan_with_topology(plan, &StorageGraph::empty());
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.missing_count, 1);
        assert_eq!(
            comparison.diagnostics[0].kind,
            TopologyDiagnosticKind::Missing
        );
    }

    #[test]
    fn plan_warns_for_pool_device_removal_and_dataset_destroy() {
        let plan = plan_from_json_bytes(
            br#"{
              "spec": {
                "pools": {
                  "tank": {
                    "removeDevices": ["/dev/sdb"],
                    "properties": {
                      "autotrim": "on"
                    }
                  }
                },
                "datasets": {
                  "tank/old": {
                    "destroy": true
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 3);
        assert_eq!(plan.summary.destructive_count, 1);
        assert_eq!(plan.summary.potential_data_loss_count, 1);
        assert!(plan.actions.iter().any(|action| {
            action.operation == Operation::RemoveDevice
                && action.risk == RiskClass::PotentialDataLoss
                && action.context.device.as_deref() == Some("/dev/sdb")
                && action.advice.is_some()
        }));
    }

    #[test]
    fn plan_classifies_zfs_pool_lifecycle() {
        let plan = plan_from_json_bytes(
            br#"{
              "pools": {
                "tank": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/pool-vdev0"
                },
                "oldtank": {
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 2);
        assert_eq!(plan.summary.destructive_count, 2);
        let create = plan
            .actions
            .iter()
            .find(|action| action.id == "pools:tank:create")
            .expect("pool create action exists");
        assert_eq!(create.risk, RiskClass::Destructive);
        assert!(create.destructive);
        assert_eq!(
            create.context.device.as_deref(),
            Some("/dev/disk/by-id/pool-vdev0")
        );
        assert!(create.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("zpool create"))
        }));
        let destroy = plan
            .actions
            .iter()
            .find(|action| action.id == "pools:oldtank:destroy")
            .expect("pool destroy action exists");
        assert_eq!(destroy.risk, RiskClass::Destructive);
    }

    #[test]
    fn plan_accepts_zfs_pool_import_export_lifecycle() {
        let plan = plan_from_json_bytes(
            br#"{
              "pools": {
                "tank": {
                  "operation": "export"
                },
                "vault": {
                  "operation": "import",
                  "readOnly": true
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 2);
        assert_eq!(plan.summary.offline_required_count, 2);
        assert_eq!(plan.summary.destructive_count, 0);
        let export = plan
            .actions
            .iter()
            .find(|action| action.id == "pools:tank:export")
            .expect("pool export action exists");
        assert_eq!(export.risk, RiskClass::OfflineRequired);
        assert!(!export.destructive);
        assert!(export.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("instead of destroying"))
        }));
        let import = plan
            .actions
            .iter()
            .find(|action| action.id == "pools:vault:import")
            .expect("pool import action exists");
        assert_eq!(import.risk, RiskClass::OfflineRequired);
        assert_eq!(import.context.read_only, Some(true));
        assert!(import.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("read-only"))
        }));
    }

    #[test]
    fn zfs_pool_import_export_capabilities_are_advertised() {
        let capabilities = default_capabilities();

        assert!(capabilities.iter().any(|capability| {
            capability.node_kind == NodeKind::ZfsPool
                && capability.operation == Operation::Import
                && capability.risk == RiskClass::OfflineRequired
        }));
        assert!(capabilities.iter().any(|capability| {
            capability.node_kind == NodeKind::ZfsPool
                && capability.operation == Operation::Export
                && capability.risk == RiskClass::OfflineRequired
        }));
    }

    #[test]
    fn plan_classifies_snapshot_rollback_as_potential_data_loss() {
        let plan = plan_from_json_bytes(
            br#"{
              "snapshots": {
                "tank/home@before-upgrade": {
                  "target": "tank/home",
                  "rollback": true,
                  "recursiveRollback": true
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 1);
        assert_eq!(plan.summary.potential_data_loss_count, 1);
        assert_eq!(plan.actions[0].operation, Operation::Rollback);
        assert_eq!(plan.actions[0].context.recursive_rollback, Some(true));
    }

    #[test]
    fn plan_accepts_zfs_snapshot_clone_as_reversible() {
        let plan = plan_from_json_bytes(
            br#"{
              "snapshots": {
                "tank/home@before-upgrade": {
                  "target": "tank/home",
                  "cloneTo": "tank/home-review"
                }
              }
            }"#,
        )
        .expect("document should parse");

        assert_eq!(plan.summary.action_count, 1);
        assert_eq!(plan.actions[0].operation, Operation::Clone);
        assert_eq!(plan.actions[0].risk, RiskClass::Reversible);
        assert_eq!(
            plan.actions[0].context.name.as_deref(),
            Some("tank/home@before-upgrade")
        );
        assert_eq!(
            plan.actions[0].context.target.as_deref(),
            Some("tank/home-review")
        );
    }

    #[test]
    fn plan_accepts_storage_rename_as_offline_non_destructive() {
        let plan = plan_from_json_bytes(
            br#"{
              "spec": {
                "datasets": {
                  "tank/home": {
                    "operation": "rename",
                    "renameTo": "tank/home-staged"
                  }
                },
                "volumes": {
                  "vg0/old": {
                    "operation": "rename",
                    "renameTo": "vg0/new"
                  }
                },
                "snapshots": {
                  "tank/home@before-prune": {
                    "target": "tank/home",
                    "renameTo": "tank/home@retained"
                  }
                }
              }
            }"#,
        )
        .expect("document should parse");

        assert_eq!(plan.summary.action_count, 3);
        assert_eq!(plan.summary.offline_required_count, 3);
        assert_eq!(plan.summary.destructive_count, 0);
        assert!(plan.actions.iter().all(|action| {
            action.operation == Operation::Rename
                && action.risk == RiskClass::OfflineRequired
                && !action.destructive
        }));
        assert!(plan.actions.iter().any(|action| {
            action.id == "datasets:tank/home:rename"
                && action.context.rename_to.as_deref() == Some("tank/home-staged")
        }));
        assert!(plan.actions.iter().any(|action| {
            action.id == "snapshot:tank/home@before-prune:rename:tank/home@retained"
                && action.context.rename_to.as_deref() == Some("tank/home@retained")
        }));
    }

    #[test]
    fn plan_accepts_zfs_clone_promotion_as_offline_non_destructive() {
        let plan = plan_from_json_bytes(
            br#"{
              "spec": {
                "datasets": {
                  "tank/home-review": {
                    "operation": "promote"
                  }
                },
                "zvols": {
                  "tank/vm/root-review": {
                    "operation": "promote"
                  }
                }
              }
            }"#,
        )
        .expect("document should parse");

        assert_eq!(plan.summary.action_count, 2);
        assert_eq!(plan.summary.offline_required_count, 2);
        assert_eq!(plan.summary.destructive_count, 0);
        assert!(plan.actions.iter().all(|action| {
            action.operation == Operation::Promote
                && action.risk == RiskClass::OfflineRequired
                && !action.destructive
        }));
        assert!(plan.actions.iter().any(|action| {
            action.id == "datasets:tank/home-review:promote"
                && action.context.target.as_deref() == Some("tank/home-review")
        }));
        assert!(plan.actions.iter().any(|action| {
            action.id == "zvols:tank/vm/root-review:promote"
                && action.context.target.as_deref() == Some("tank/vm/root-review")
        }));
    }

    #[test]
    fn plan_accepts_zfs_snapshot_holds_as_safe_property_actions() {
        let plan = plan_from_json_bytes(
            br#"{
              "snapshots": {
                "tank/home@before-upgrade": {
                  "target": "tank/home",
                  "hold": "disk-nix-retain"
                },
                "tank/home@old": {
                  "target": "tank/home",
                  "releaseHold": "old-retention"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 2);
        assert_eq!(plan.summary.destructive_count, 0);
        assert_eq!(plan.summary.potential_data_loss_count, 0);
        let hold = plan
            .actions
            .iter()
            .find(|action| action.id == "snapshot:tank/home@before-upgrade:hold:disk-nix-retain")
            .expect("snapshot hold action exists");
        assert_eq!(hold.operation, Operation::SetProperty);
        assert_eq!(hold.risk, RiskClass::Safe);
        assert_eq!(hold.context.property.as_deref(), Some("zfs.hold"));
        assert_eq!(
            hold.context.property_value.as_deref(),
            Some("disk-nix-retain")
        );
        let release = plan
            .actions
            .iter()
            .find(|action| action.id == "snapshot:tank/home@old:release-hold:old-retention")
            .expect("snapshot hold release action exists");
        assert_eq!(release.operation, Operation::SetProperty);
        assert_eq!(release.risk, RiskClass::Safe);
        assert_eq!(release.context.property.as_deref(), Some("zfs.releaseHold"));
        assert_eq!(
            release.context.property_value.as_deref(),
            Some("old-retention")
        );
    }

    #[test]
    fn plan_preserves_btrfs_read_only_snapshot_context() {
        let plan = plan_from_json_bytes(
            br#"{
              "snapshots": {
                "/mnt/persist/@home-before": {
                  "target": "/mnt/persist/@home",
                  "readOnly": true
                }
              }
            }"#,
        )
        .expect("plan should parse");

        let action = plan
            .actions
            .iter()
            .find(|action| action.id == "snapshot:/mnt/persist/@home-before:create")
            .expect("snapshot action exists");

        assert_eq!(action.operation, Operation::Snapshot);
        assert_eq!(action.context.target.as_deref(), Some("/mnt/persist/@home"));
        assert_eq!(action.context.read_only, Some(true));
    }

    #[test]
    fn plan_classifies_snapshot_rescan_as_online_refresh() {
        let plan = plan_from_json_bytes(
            br#"{
              "snapshots": {
                "tank/home@before-upgrade": {
                  "operation": "rescan",
                  "target": "tank/home"
                },
                "/mnt/persist/@home-before-upgrade": {
                  "operation": "rescan",
                  "target": "/mnt/persist/@home",
                  "readOnly": true
                },
                "home-before-friendly": {
                  "operation": "rescan",
                  "target": "/mnt/persist/@home",
                  "snapshotPath": "/mnt/persist/@home-before-friendly"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 3);
        assert_eq!(plan.summary.destructive_count, 0);
        assert_eq!(plan.summary.offline_required_count, 0);
        let zfs_rescan = plan
            .actions
            .iter()
            .find(|action| action.id == "snapshot:tank/home@before-upgrade:rescan")
            .expect("ZFS snapshot rescan action exists");
        assert_eq!(zfs_rescan.operation, Operation::Rescan);
        assert_eq!(zfs_rescan.risk, RiskClass::Online);
        assert!(!zfs_rescan.destructive);
        assert_eq!(zfs_rescan.context.target.as_deref(), Some("tank/home"));
        let btrfs_rescan = plan
            .actions
            .iter()
            .find(|action| action.id == "snapshot:/mnt/persist/@home-before-upgrade:rescan")
            .expect("Btrfs snapshot rescan action exists");
        assert_eq!(btrfs_rescan.operation, Operation::Rescan);
        assert_eq!(btrfs_rescan.context.read_only, Some(true));
        assert!(
            btrfs_rescan
                .advice
                .as_ref()
                .is_some_and(|advice| { advice.summary.contains("without mutating data") })
        );
        let friendly_btrfs_rescan = plan
            .actions
            .iter()
            .find(|action| action.id == "snapshot:home-before-friendly:rescan")
            .expect("friendly-key Btrfs snapshot rescan action exists");
        assert_eq!(
            friendly_btrfs_rescan.context.target.as_deref(),
            Some("/mnt/persist/@home")
        );
        assert_eq!(
            friendly_btrfs_rescan.context.snapshot_path.as_deref(),
            Some("/mnt/persist/@home-before-friendly")
        );
    }

    #[test]
    fn plan_classifies_lun_growth_as_offline_required() {
        let plan = plan_from_json_bytes(
            br#"{
              "luns": {
                "iqn.2026-06.example:storage/root:0": {
                  "operation": "grow",
                  "desiredSize": "2TiB",
                  "device": "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0",
                  "devices": [
                    "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-0"
                  ]
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 1);
        assert_eq!(plan.summary.offline_required_count, 1);
        assert_eq!(plan.actions[0].operation, Operation::Grow);
        assert_eq!(plan.actions[0].risk, RiskClass::OfflineRequired);
        assert_eq!(
            plan.actions[0].context.desired_size.as_deref(),
            Some("2TiB")
        );
        assert_eq!(
            plan.actions[0].context.device.as_deref(),
            Some("/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0")
        );
        assert_eq!(
            plan.actions[0].context.devices,
            vec![
                "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-0"
                    .to_string()
            ]
        );
        assert!(plan.actions[0].advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("multipath"))
        }));
    }

    #[test]
    fn plan_classifies_lun_attach_and_detach() {
        let plan = plan_from_json_bytes(
            br#"{
              "luns": {
                "iqn.2026-06.example:storage/root:0": {
                  "operation": "attach",
                  "device": "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0"
                },
                "iqn.2026-06.example:storage/old:1": {
                  "operation": "detach",
                  "devices": [
                    "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-1"
                  ]
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 2);
        assert_eq!(plan.summary.offline_required_count, 1);
        assert_eq!(plan.summary.destructive_count, 0);

        let attach = plan
            .actions
            .iter()
            .find(|action| action.id == "luns:iqn.2026-06.example:storage/root:0:attach")
            .expect("LUN attach action exists");
        assert_eq!(attach.operation, Operation::Attach);
        assert_eq!(attach.risk, RiskClass::Online);
        assert!(attach.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("stable by-path"))
        }));

        let detach = plan
            .actions
            .iter()
            .find(|action| action.id == "luns:iqn.2026-06.example:storage/old:1:detach")
            .expect("LUN detach action exists");
        assert_eq!(detach.operation, Operation::Detach);
        assert_eq!(detach.risk, RiskClass::OfflineRequired);
        assert!(!detach.destructive);
        assert!(detach.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("deactivate"))
        }));
    }

    #[test]
    fn plan_classifies_nvme_namespace_lifecycle() {
        let plan = plan_from_json_bytes(
            br#"{
              "nvmeNamespaces": {
                "/dev/nvme0": {
                  "operation": "create",
                  "desiredSize": "100G",
                  "namespaceId": "4",
                  "controllers": "0x1"
                },
                "/dev/nvme1": {
                  "operation": "grow"
                },
                "/dev/nvme2": {
                  "destroy": true,
                  "namespaceId": "7",
                  "controllers": "0x2"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 3);
        assert_eq!(plan.summary.destructive_count, 2);
        assert_eq!(plan.summary.offline_required_count, 1);

        let create = plan
            .actions
            .iter()
            .find(|action| action.id == "nvmenamespaces:/dev/nvme0:create")
            .expect("NVMe namespace create action exists");
        assert_eq!(create.operation, Operation::Create);
        assert_eq!(create.risk, RiskClass::Destructive);
        assert_eq!(create.context.namespace_id.as_deref(), Some("4"));
        assert_eq!(create.context.controllers.as_deref(), Some("0x1"));

        let grow = plan
            .actions
            .iter()
            .find(|action| action.id == "nvmenamespaces:/dev/nvme1:grow")
            .expect("NVMe namespace grow action exists");
        assert_eq!(grow.operation, Operation::Grow);
        assert_eq!(grow.risk, RiskClass::OfflineRequired);

        let destroy = plan
            .actions
            .iter()
            .find(|action| action.id == "nvmenamespaces:/dev/nvme2:destroy")
            .expect("NVMe namespace destroy action exists");
        assert_eq!(destroy.operation, Operation::Destroy);
        assert_eq!(destroy.risk, RiskClass::Destructive);
    }

    #[test]
    fn plan_classifies_iscsi_session_growth_as_offline_required() {
        let plan = plan_from_json_bytes(
            br#"{
              "iscsiSessions": {
                "iqn.2026-06.example:storage.root": {
                  "operation": "grow"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 1);
        assert_eq!(plan.summary.offline_required_count, 1);
        assert_eq!(plan.actions[0].operation, Operation::Grow);
        assert_eq!(plan.actions[0].risk, RiskClass::OfflineRequired);
    }

    #[test]
    fn plan_classifies_host_storage_rescans_as_online() {
        let plan = plan_from_json_bytes(
            br#"{
              "iscsiSessions": {
                "iqn.2026-06.example:storage.root": {
                  "operation": "rescan"
                }
              },
              "luns": {
                "iqn.2026-06.example:storage/root:0": {
                  "operation": "rescan",
                  "devices": [
                    "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0"
                  ]
                }
              },
              "nvmeNamespaces": {
                "/dev/nvme0": {
                  "operation": "rescan"
                }
              },
              "physicalVolumes": {
                "/dev/disk/by-id/nvme-pv-refresh": {
                  "operation": "rescan"
                }
              },
              "volumeGroups": {
                "vgrefresh": {
                  "operation": "rescan"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 5);
        assert_eq!(plan.summary.offline_required_count, 0);
        assert!(plan.actions.iter().all(|action| {
            action.operation == Operation::Rescan && action.risk == RiskClass::Online
        }));
    }

    #[test]
    fn plan_classifies_iscsi_session_login_and_logout() {
        let plan = plan_from_json_bytes(
            br#"{
              "iscsiSessions": {
                "iqn.2026-06.example:storage.root": {
                  "operation": "login",
                  "metadata": {
                    "portal": "192.0.2.10:3260"
                  }
                },
                "iqn.2026-06.example:storage.old": {
                  "operation": "logout",
                  "portal": "192.0.2.11:3260"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 2);
        assert_eq!(plan.summary.offline_required_count, 1);
        assert_eq!(plan.summary.destructive_count, 0);
        let create = plan
            .actions
            .iter()
            .find(|action| action.id == "iscsisessions:iqn.2026-06.example:storage.root:login")
            .expect("iSCSI login action exists");
        assert_eq!(create.operation, Operation::Login);
        assert_eq!(create.risk, RiskClass::Online);
        assert_eq!(create.context.portal.as_deref(), Some("192.0.2.10:3260"));

        let destroy = plan
            .actions
            .iter()
            .find(|action| action.id == "iscsisessions:iqn.2026-06.example:storage.old:logout")
            .expect("iSCSI logout action exists");
        assert_eq!(destroy.operation, Operation::Logout);
        assert_eq!(destroy.risk, RiskClass::OfflineRequired);
        assert_eq!(destroy.context.portal.as_deref(), Some("192.0.2.11:3260"));
    }

    #[test]
    fn plan_classifies_nfs_export_lifecycle_without_data_destruction() {
        let plan = plan_from_json_bytes(
            br#"{
              "exports": {
                "/srv/share": {
                  "operation": "export",
                  "client": "192.0.2.0/24",
                  "options": "rw,sync,no_subtree_check"
                },
                "/srv/inventory": {
                  "operation": "rescan"
                },
                "/srv/old": {
                  "operation": "unexport",
                  "client": "192.0.2.55"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 3);
        assert_eq!(plan.summary.offline_required_count, 1);
        assert_eq!(plan.summary.destructive_count, 0);
        let create = plan
            .actions
            .iter()
            .find(|action| action.id == "exports:/srv/share:export")
            .expect("export action exists");
        assert_eq!(create.operation, Operation::Export);
        assert_eq!(create.risk, RiskClass::Online);
        assert_eq!(create.context.client.as_deref(), Some("192.0.2.0/24"));
        assert_eq!(
            create.context.options.as_deref(),
            Some("rw,sync,no_subtree_check")
        );
        let rescan = plan
            .actions
            .iter()
            .find(|action| action.id == "exports:/srv/inventory:rescan")
            .expect("export rescan action exists");
        assert_eq!(rescan.operation, Operation::Rescan);
        assert_eq!(rescan.risk, RiskClass::Online);
        assert_eq!(rescan.context.target.as_deref(), Some("/srv/inventory"));
        let destroy = plan
            .actions
            .iter()
            .find(|action| action.id == "exports:/srv/old:unexport")
            .expect("unexport action exists");
        assert_eq!(destroy.operation, Operation::Unexport);
        assert_eq!(destroy.risk, RiskClass::OfflineRequired);
        assert!(!destroy.destructive);
    }

    #[test]
    fn plan_classifies_nfs_mount_lifecycle_without_remote_data_destruction() {
        let plan = plan_from_json_bytes(
            br#"{
              "nfs": {
                "mounts": {
                  "/srv/shared": {
                    "operation": "mount",
                    "source": "nas.example.com:/srv/shared",
                    "fsType": "nfs4",
                    "options": ["_netdev", "vers=4.2"]
                  },
                  "/srv/old": {
                    "operation": "unmount",
                    "source": "nas.example.com:/srv/old"
                  },
                  "/srv/tuned": {
                    "operation": "remount",
                    "source": "nas.example.com:/srv/tuned",
                    "options": ["_netdev", "ro", "vers=4.2"]
                  },
                  "/srv/inventory": {
                    "operation": "rescan",
                    "source": "nas.example.com:/srv/inventory"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 4);
        assert_eq!(plan.summary.offline_required_count, 1);
        assert_eq!(plan.summary.destructive_count, 0);
        let create = plan
            .actions
            .iter()
            .find(|action| action.id == "nfs.mounts:/srv/shared:mount")
            .expect("NFS mount action exists");
        assert_eq!(create.operation, Operation::Mount);
        assert_eq!(create.risk, RiskClass::Online);
        assert_eq!(
            create.context.device.as_deref(),
            Some("nas.example.com:/srv/shared")
        );
        assert_eq!(create.context.mountpoint.as_deref(), Some("/srv/shared"));
        assert_eq!(create.context.fs_type.as_deref(), Some("nfs4"));
        assert_eq!(create.context.options.as_deref(), Some("_netdev,vers=4.2"));

        let destroy = plan
            .actions
            .iter()
            .find(|action| action.id == "nfs.mounts:/srv/old:unmount")
            .expect("NFS unmount action exists");
        assert_eq!(destroy.operation, Operation::Unmount);
        assert_eq!(destroy.risk, RiskClass::OfflineRequired);
        assert!(!destroy.destructive);
        assert_eq!(destroy.context.mountpoint.as_deref(), Some("/srv/old"));

        let remount = plan
            .actions
            .iter()
            .find(|action| action.id == "nfs.mounts:/srv/tuned:remount")
            .expect("NFS mount remount exists");
        assert_eq!(remount.operation, Operation::Remount);
        assert_eq!(remount.risk, RiskClass::Online);
        assert_eq!(remount.context.mountpoint.as_deref(), Some("/srv/tuned"));
        assert_eq!(
            remount.context.options.as_deref(),
            Some("_netdev,ro,vers=4.2")
        );

        let rescan = plan
            .actions
            .iter()
            .find(|action| action.id == "nfs.mounts:/srv/inventory:rescan")
            .expect("NFS mount rescan exists");
        assert_eq!(rescan.operation, Operation::Rescan);
        assert_eq!(rescan.risk, RiskClass::Online);
        assert_eq!(rescan.context.mountpoint.as_deref(), Some("/srv/inventory"));
    }

    #[test]
    fn plan_classifies_cache_replacement_as_offline_required() {
        let plan = plan_from_json_bytes(
            br#"{
              "caches": {
                "vg0/root-cache": {
                  "operation": "replace-device",
                  "removeDevices": ["/dev/sdd"],
                  "replaceDevices": {
                    "/dev/sdb": "/dev/sdc"
                  }
                },
                "/dev/bcache0": {
                  "operation": "rescan"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 4);
        assert_eq!(plan.summary.offline_required_count, 3);
        assert!(
            plan.actions
                .iter()
                .filter(|action| action.operation == Operation::ReplaceDevice)
                .all(|action| {
                    action.operation == Operation::ReplaceDevice
                        && action.risk == RiskClass::OfflineRequired
                        && action.advice.as_ref().is_some_and(|advice| {
                            advice
                                .alternatives
                                .iter()
                                .any(|alternative| alternative.contains("flush dirty data"))
                        })
                })
        );
        let detach = plan
            .actions
            .iter()
            .find(|action| action.id == "caches:vg0/root-cache:remove-device:/dev/sdd")
            .expect("cache detach action exists");
        assert_eq!(detach.operation, Operation::RemoveDevice);
        assert_eq!(detach.risk, RiskClass::OfflineRequired);
        assert!(detach.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("dirty data"))
        }));
        let rescan = plan
            .actions
            .iter()
            .find(|action| action.id == "caches:/dev/bcache0:rescan")
            .expect("cache rescan action exists");
        assert_eq!(rescan.operation, Operation::Rescan);
        assert_eq!(rescan.risk, RiskClass::Online);
        assert!(!rescan.destructive);
        assert!(rescan.advice.as_ref().is_some_and(|advice| {
            advice
                .summary
                .contains("bcache rescan refreshes cache state")
        }));
    }

    #[test]
    fn plan_classifies_lvm_cache_attach_and_detach() {
        let plan = plan_from_json_bytes(
            br#"{
              "lvmCaches": {
                "vg0/root": {
                  "operation": "create",
                  "device": "vg0/root-cache",
                  "addDevices": ["vg0/root-cache"],
                  "removeDevices": ["vg0/root-cache"],
                  "properties": {
                    "lvm.cache-mode": "writethrough"
                  }
                },
                "vg0/archive": {
                  "operation": "rescan"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        let create = plan
            .actions
            .iter()
            .find(|action| action.id == "lvmcaches:vg0/root:create")
            .expect("LVM cache create action exists");
        let add = plan
            .actions
            .iter()
            .find(|action| action.id == "lvmCaches:vg0/root:add-device:vg0/root-cache")
            .expect("LVM cache add action exists");
        let remove = plan
            .actions
            .iter()
            .find(|action| action.id == "lvmCaches:vg0/root:remove-device:vg0/root-cache")
            .expect("LVM cache remove action exists");
        let property = plan
            .actions
            .iter()
            .find(|action| action.id == "lvmCaches:vg0/root:set-property:lvm.cache-mode")
            .expect("LVM cache property action exists");
        let rescan = plan
            .actions
            .iter()
            .find(|action| action.id == "lvmcaches:vg0/archive:rescan")
            .expect("LVM cache rescan action exists");

        assert_eq!(plan.summary.action_count, 5);
        assert_eq!(plan.summary.offline_required_count, 3);
        assert_eq!(create.risk, RiskClass::OfflineRequired);
        assert_eq!(add.risk, RiskClass::OfflineRequired);
        assert_eq!(remove.risk, RiskClass::OfflineRequired);
        assert_eq!(property.risk, RiskClass::Safe);
        assert_eq!(rescan.operation, Operation::Rescan);
        assert_eq!(rescan.risk, RiskClass::Online);
        assert!(!rescan.destructive);
        assert!(remove.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("dirty data"))
        }));
    }

    #[test]
    fn apply_policy_blocks_destructive_and_potential_data_loss_actions() {
        let (plan, mut policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "datasets": {
                  "tank/old": { "destroy": true }
                },
                "pools": {
                  "tank": { "removeDevices": ["/dev/sdb"] }
                }
              },
              "apply": {
                "mode": "manual",
                "allowDestructive": false,
                "allowFormat": false,
                "allowShrink": false,
                "allowPotentialDataLoss": false,
                "allowGrow": true,
                "allowOffline": false,
                "allowPropertyChanges": true
              }
            }"#,
        )
        .expect("document should parse");

        let report = evaluate_apply_policy(&plan, policy.clone());

        assert_eq!(report.blocked_count, 2);
        assert_eq!(report.blocked_summary.destructive_count, 1);
        assert_eq!(report.blocked_summary.potential_data_loss_count, 1);
        assert!(report.blocked.iter().any(|blocked| {
            blocked.reason == "potential-data-loss actions require allowPotentialDataLoss=true"
        }));
        assert!(!report.can_execute());

        policy.allow_destructive = true;
        policy.allow_potential_data_loss = true;
        let report = evaluate_apply_policy(&plan, policy);
        assert_eq!(report.blocked_count, 0);
        assert!(report.can_execute());
    }

    #[test]
    fn apply_policy_requires_backup_and_confirmation_for_allowed_potential_data_loss() {
        let (plan, mut policy) = plan_and_policy_from_json_bytes(
            br#"{
              "filesystems": {
                "home": {
                  "mountpoint": "/home",
                  "fsType": "ext4",
                  "resizePolicy": "shrink-allowed"
                }
              },
              "apply": {
                "allowShrink": true,
                "allowPotentialDataLoss": true,
                "requireBackup": true,
                "backupVerified": false,
                "requireConfirmation": true,
                "confirmation": false
              }
            }"#,
        )
        .expect("document should parse");

        let report = evaluate_apply_policy(&plan, policy.clone());
        assert_eq!(report.blocked_count, 1);
        assert_eq!(
            report.blocked[0].reason,
            "backup-required actions require backupVerified=true"
        );

        policy.backup_verified = true;
        let report = evaluate_apply_policy(&plan, policy.clone());
        assert_eq!(report.blocked_count, 1);
        assert_eq!(
            report.blocked[0].reason,
            "confirmation-required actions require confirmation=true"
        );

        policy.confirmation = true;
        let report = evaluate_apply_policy(&plan, policy);
        assert_eq!(report.blocked_count, 0);
        assert!(report.can_execute());
    }

    #[test]
    fn apply_policy_blocks_unsupported_actions_even_when_permissive() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "filesystems": {
                "archive": {
                  "mountpoint": "/archive",
                  "fsType": "xfs",
                  "resizePolicy": "shrink-allowed"
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowFormat": true,
                "allowShrink": true,
                "allowGrow": true,
                "allowOffline": true,
                "allowPropertyChanges": true
              }
            }"#,
        )
        .expect("document should parse");

        let report = evaluate_apply_policy(&plan, policy);

        assert_eq!(report.blocked_count, 1);
        assert_eq!(report.blocked_summary.unsupported_count, 1);
        assert_eq!(report.blocked[0].risk, RiskClass::Unsupported);
        assert_eq!(
            report.blocked[0].reason,
            "unsupported actions cannot be applied"
        );
    }

    #[test]
    fn apply_policy_allows_grow_when_enabled() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "volumes": {
                  "vg/root": { "operation": "grow" }
                }
              },
              "apply": {
                "allowGrow": true
              }
            }"#,
        )
        .expect("document should parse");

        let report = evaluate_apply_policy(&plan, policy);

        assert_eq!(report.blocked_count, 0);
        assert!(report.can_execute());
    }

    #[test]
    fn apply_policy_requires_offline_permission_for_offline_required_actions() {
        let (plan, mut policy) = plan_and_policy_from_json_bytes(
            br#"{
              "luns": {
                "iqn.2026-06.example:storage/root:0": {
                  "operation": "grow"
                }
              },
              "apply": {
                "allowGrow": true,
                "allowOffline": false
              }
            }"#,
        )
        .expect("document should parse");

        let report = evaluate_apply_policy(&plan, policy.clone());
        assert_eq!(report.blocked_count, 1);
        assert_eq!(report.blocked_summary.offline_required_count, 1);
        assert_eq!(report.blocked[0].risk, RiskClass::OfflineRequired);
        assert_eq!(
            report.blocked[0].reason,
            "offline-required actions require allowOffline=true"
        );

        policy.allow_offline = true;
        let report = evaluate_apply_policy(&plan, policy);
        assert_eq!(report.blocked_count, 0);
        assert!(report.can_execute());
    }

    #[test]
    fn apply_policy_requires_format_and_destructive_permission_for_format() {
        let (plan, mut policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "filesystems": {
                  "scratch": {
                    "mountpoint": "/scratch",
                    "fsType": "xfs",
                    "preserveData": false
                  }
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowFormat": false
              }
            }"#,
        )
        .expect("document should parse");

        let report = evaluate_apply_policy(&plan, policy.clone());
        assert!(
            report
                .blocked
                .iter()
                .any(|blocked| blocked.reason == "format actions require allowFormat=true")
        );

        policy.allow_format = true;
        let report = evaluate_apply_policy(&plan, policy);
        assert_eq!(report.blocked_count, 0);
    }

    #[test]
    fn apply_policy_can_require_verified_backup_for_high_risk_actions() {
        let (plan, mut policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "datasets": {
                  "tank/old": { "destroy": true }
                }
              },
              "apply": {
                "allowDestructive": true,
                "requireBackup": true,
                "backupVerified": false
              }
            }"#,
        )
        .expect("document should parse");

        let report = evaluate_apply_policy(&plan, policy.clone());
        assert_eq!(report.blocked_count, 1);
        assert_eq!(
            report.blocked[0].reason,
            "backup-required actions require backupVerified=true"
        );

        policy.backup_verified = true;
        let report = evaluate_apply_policy(&plan, policy);
        assert_eq!(report.blocked_count, 0);
    }

    #[test]
    fn apply_policy_can_require_confirmation_for_offline_actions() {
        let (plan, mut policy) = plan_and_policy_from_json_bytes(
            br#"{
              "luns": {
                "iqn.2026-06.example:storage/root:0": {
                  "operation": "grow"
                }
              },
              "apply": {
                "allowGrow": true,
                "allowOffline": true,
                "requireConfirmation": true,
                "confirmation": false
              }
            }"#,
        )
        .expect("document should parse");

        let report = evaluate_apply_policy(&plan, policy.clone());
        assert_eq!(report.blocked_count, 1);
        assert_eq!(
            report.blocked[0].reason,
            "confirmation-required actions require confirmation=true"
        );

        policy.confirmation = true;
        let report = evaluate_apply_policy(&plan, policy);
        assert_eq!(report.blocked_count, 0);
    }

    #[test]
    fn apply_policy_can_require_confirmation_file_for_offline_actions() {
        let (plan, mut policy) = plan_and_policy_from_json_bytes(
            br#"{
              "luns": {
                "iqn.2026-06.example:storage/root:0": {
                  "operation": "grow"
                }
              },
              "apply": {
                "allowGrow": true,
                "allowOffline": true,
                "requireConfirmationFile": "/run/disk-nix/confirm"
              }
            }"#,
        )
        .expect("document should parse");

        let report = evaluate_apply_policy(&plan, policy.clone());
        assert_eq!(report.blocked_count, 1);
        assert_eq!(
            report.blocked[0].reason,
            "confirmation-file policy requires confirmation=true after checking the configured file"
        );

        policy.confirmation = true;
        let report = evaluate_apply_policy(&plan, policy);
        assert_eq!(report.blocked_count, 0);
    }

    #[test]
    fn apply_policy_can_disable_device_topology_changes_and_rebalance() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "pools": {
                "tank": {
                  "operation": "rebalance",
                  "addDevices": ["/dev/disk/by-id/new"],
                  "replaceDevices": {
                    "/dev/disk/by-id/old": "/dev/disk/by-id/new"
                  }
                }
              },
              "apply": {
                "allowGrow": true,
                "allowDeviceReplacement": false,
                "allowRebalance": false
              }
            }"#,
        )
        .expect("document should parse");

        let report = evaluate_apply_policy(&plan, policy);

        assert_eq!(report.blocked_count, 3);
        assert!(report.blocked.iter().any(|blocked| {
            blocked.reason == "device topology changes require allowDeviceReplacement=true"
        }));
        assert!(
            report
                .blocked
                .iter()
                .any(|blocked| blocked.reason == "rebalance actions require allowRebalance=true")
        );
    }
}
