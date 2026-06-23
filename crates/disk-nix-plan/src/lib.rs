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
    ReplaceDevice,
    AddDevice,
    RemoveDevice,
    SetProperty,
    Snapshot,
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
    pub device: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub devices: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
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
    pub read_only: Option<bool>,
}

impl ActionContext {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.collection.is_none()
            && self.name.is_none()
            && self.target.is_none()
            && self.device.is_none()
            && self.devices.is_empty()
            && self.replacement.is_none()
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
            && self.read_only.is_none()
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
    } else if action.risk == RiskClass::PotentialDataLoss {
        Some("potential data loss actions require a safer workflow or future explicit policy")
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

    if let Some(Operation::Rebalance) = filesystem
        .get("operation")
        .or_else(|| filesystem.get("action"))
        .and_then(Value::as_str)
        .and_then(parse_operation)
    {
        let (risk, destructive, advice) =
            classify_operation("filesystems", Operation::Rebalance, filesystem);
        actions.push(PlannedAction {
            id: format!("filesystems:{name}:rebalance"),
            description: format!("plan rebalance operation for filesystem {name}"),
            operation: Operation::Rebalance,
            risk,
            destructive,
            context: ActionContext {
                collection: Some("filesystems".to_string()),
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
        let (risk, advice) = classify_filesystem_property_change(fs_type, property);
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
) -> (RiskClass, Option<Advice>) {
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
                "use label, filesystem.label, btrfs.label, or ext.label when changing filesystem labels"
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
        "btrfs" => matches!(property, "label" | "btrfs.label" | "filesystem.label"),
        "ext2" | "ext3" | "ext4" => {
            matches!(property, "label" | "ext.label" | "filesystem.label")
        }
        "zfs" => true,
        _ => false,
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
            context,
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
            context,
            advice: None,
        }),
    }
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
        ..ActionContext::default()
    };

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
        Some(Operation::Destroy) => actions.push(PlannedAction {
            id: format!("luks.devices:{name}:destroy"),
            description: format!(
                "close LUKS mapping {mapper_name} without formatting {device_label}"
            ),
            operation: Operation::Destroy,
            risk: RiskClass::OfflineRequired,
            destructive: false,
            context,
            advice: Some(Advice {
                summary: "closing a LUKS mapper requires dependent layers to be stopped"
                    .to_string(),
                alternatives: vec![
                    "unmount filesystems and deactivate LVM volumes before closing the mapper"
                        .to_string(),
                    "leave the LUKS header and backing device intact for later reopen".to_string(),
                    "use preserveData=false only when reformatting is explicitly intended"
                        .to_string(),
                ],
            }),
        }),
        Some(Operation::Create) if preserve_data => actions.push(PlannedAction {
            id: format!("luks.devices:{name}:create"),
            description: format!("open existing LUKS container {device_label} as {mapper_name}"),
            operation: Operation::Create,
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
        }),
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
        _ => actions.push(PlannedAction {
            id: format!("luks.devices:{name}:inspect"),
            description: format!("inspect LUKS declaration {mapper_name} on {device_label}"),
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            destructive: false,
            context,
            advice: None,
        }),
    }
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
    actions.push(PlannedAction {
        id: format!("{collection}:{name}:{operation:?}").to_ascii_lowercase(),
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
        let (risk, advice) = classify_property_change(collection, property);
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

fn classify_property_change(collection: &str, property: &str) -> (RiskClass, Option<Advice>) {
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

    if collection == "luksKeyslots" {
        return (
            RiskClass::OfflineRequired,
            Some(Advice {
                summary: format!(
                    "LUKS keyslot property {property} updates encrypted-container access material"
                ),
                alternatives: vec![
                    "verify at least one independent recovery key before changing key material"
                        .to_string(),
                    "add and test a replacement key before removing the old keyslot".to_string(),
                    "back up the LUKS header before keyslot changes".to_string(),
                ],
            }),
        );
    }

    (RiskClass::Safe, None)
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
    let hold = string_field(snapshot, &["hold", "holdTag"]);
    let release_hold = string_field(snapshot, &["releaseHold", "release-hold"]);
    let destroy = snapshot
        .get("destroy")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let rollback = snapshot
        .get("rollback")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let read_only = snapshot
        .get("readOnly")
        .or_else(|| snapshot.get("readonly"))
        .and_then(Value::as_bool);

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
        "replace-device" | "replaceDevice" => Some(Operation::ReplaceDevice),
        "add-device" | "addDevice" => Some(Operation::AddDevice),
        "remove-device" | "removeDevice" => Some(Operation::RemoveDevice),
        "set-property" | "setProperty" => Some(Operation::SetProperty),
        "snapshot" => Some(Operation::Snapshot),
        "rebalance" => Some(Operation::Rebalance),
        "rollback" => Some(Operation::Rollback),
        "destroy" => Some(Operation::Destroy),
        _ => None,
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
        Operation::Create if collection == "luksKeyslots" => (
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
        Operation::Create if collection == "exports" => (
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
        Operation::Create if collection == "iscsiSessions" => (
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
        Operation::Create if collection == "nfs.mounts" => (
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
        Operation::Create if collection == "luns" => (
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
        Operation::Destroy if collection == "exports" => (
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
        Operation::Destroy if collection == "nfs.mounts" => (
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
        Operation::Destroy if collection == "iscsiSessions" => (
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
        Operation::Destroy if collection == "luns" => (
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
        Operation::Destroy if collection == "luksKeyslots" => (
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
        Operation::Shrink | Operation::RemoveDevice | Operation::Rollback => (
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
        Operation::ReplaceDevice => "replace device",
        Operation::AddDevice => "add device",
        Operation::RemoveDevice => "remove device",
        Operation::SetProperty => "set property",
        Operation::Snapshot => "snapshot",
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
            operation: Operation::Create,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "LUKS keyslot enrollment changes encrypted-container access".to_string(),
                alternatives: vec![
                    "back up the LUKS header before adding key material".to_string(),
                    "test the new key before removing any old keyslot".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LuksContainer,
            operation: Operation::SetProperty,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "LUKS key changes update header access material".to_string(),
                alternatives: vec![
                    "verify a recovery key still unlocks the container".to_string(),
                    "stage replacement key material before deleting old access".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LuksContainer,
            operation: Operation::Destroy,
            risk: RiskClass::PotentialDataLoss,
            advice: Some(Advice {
                summary: "LUKS keyslot removal can lock out encrypted data".to_string(),
                alternatives: vec![
                    "verify another key or token unlocks the device first".to_string(),
                    "take a LUKS header backup before killing a slot".to_string(),
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
                summary: "supported filesystem property updates reconcile labels and ZFS filesystem properties"
                    .to_string(),
                alternatives: vec![
                    "use filesystem label aliases for Btrfs and ext filesystems".to_string(),
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
            operation: Operation::Rollback,
            risk: RiskClass::PotentialDataLoss,
            advice: Some(Advice {
                summary: "ZFS rollback can discard changes newer than the snapshot".to_string(),
                alternatives: vec![
                    "clone the snapshot and inspect data before rollback".to_string(),
                    "take a pre-rollback snapshot of the current state".to_string(),
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

        assert_eq!(add.risk, RiskClass::Online);
        assert_eq!(replace.risk, RiskClass::OfflineRequired);
        assert_eq!(remove.risk, RiskClass::PotentialDataLoss);
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
        let export_destroy = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::NfsExport
                    && capability.operation == Operation::Destroy
            })
            .expect("NFS export destroy capability should exist");
        let mount_create = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::NfsMount
                    && capability.operation == Operation::Create
            })
            .expect("NFS mount create capability should exist");
        let mount_destroy = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::NfsMount
                    && capability.operation == Operation::Destroy
            })
            .expect("NFS mount destroy capability should exist");

        assert_eq!(export_create.risk, RiskClass::Online);
        assert_eq!(export_destroy.risk, RiskClass::OfflineRequired);
        assert_eq!(mount_create.risk, RiskClass::Online);
        assert_eq!(mount_destroy.risk, RiskClass::OfflineRequired);
        assert!(mount_destroy.advice.is_some());
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
        let destroy = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::BtrfsQgroup
                    && capability.operation == Operation::Destroy
            })
            .expect("Btrfs qgroup destroy capability should exist");

        assert_eq!(create.risk, RiskClass::Online);
        assert_eq!(update_limit.risk, RiskClass::Safe);
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
        let destroy = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::LvmPhysicalVolume
                    && capability.operation == Operation::Destroy
            })
            .expect("LVM physical volume destroy capability should exist");

        assert_eq!(create.risk, RiskClass::Destructive);
        assert_eq!(grow.risk, RiskClass::Online);
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

        assert_eq!(create.risk, RiskClass::OfflineRequired);
        assert_eq!(change.risk, RiskClass::OfflineRequired);
        assert_eq!(destroy.risk, RiskClass::PotentialDataLoss);
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
        let lun_destroy = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::Lun && capability.operation == Operation::Destroy
            })
            .expect("LUN destroy capability should exist");
        let session_create = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::IscsiSession
                    && capability.operation == Operation::Create
            })
            .expect("iSCSI session create capability should exist");
        let session_destroy = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::IscsiSession
                    && capability.operation == Operation::Destroy
            })
            .expect("iSCSI session destroy capability should exist");

        assert_eq!(lun_create.risk, RiskClass::Online);
        assert_eq!(lun_destroy.risk, RiskClass::OfflineRequired);
        assert_eq!(session_create.risk, RiskClass::Online);
        assert_eq!(session_destroy.risk, RiskClass::OfflineRequired);
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
        let destroy = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::NvmeNamespace
                    && capability.operation == Operation::Destroy
            })
            .expect("NVMe namespace destroy capability should exist");

        assert_eq!(create.risk, RiskClass::Destructive);
        assert_eq!(grow.risk, RiskClass::OfflineRequired);
        assert_eq!(destroy.risk, RiskClass::Destructive);
        assert!(create.advice.is_some());
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
        let btrfs_snapshot = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::BtrfsSnapshot
                    && capability.operation == Operation::Snapshot
            })
            .expect("Btrfs snapshot create capability should exist");
        let btrfs_destroy = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::BtrfsSnapshot
                    && capability.operation == Operation::Destroy
            })
            .expect("Btrfs snapshot destroy capability should exist");

        assert_eq!(zfs_snapshot.risk, RiskClass::Reversible);
        assert_eq!(zfs_hold.risk, RiskClass::Safe);
        assert_eq!(zfs_rollback.risk, RiskClass::PotentialDataLoss);
        assert_eq!(btrfs_snapshot.risk, RiskClass::Reversible);
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
            (NodeKind::LvmVolumeGroup, Operation::Grow, RiskClass::Online),
            (
                NodeKind::LvmVolumeGroup,
                Operation::RemoveDevice,
                RiskClass::PotentialDataLoss,
            ),
            (NodeKind::MdRaid, Operation::Create, RiskClass::Destructive),
            (
                NodeKind::MdRaid,
                Operation::Grow,
                RiskClass::OfflineRequired,
            ),
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
                "vg0/old": {
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 2);
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
                "oldvg": {
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
    }

    #[test]
    fn plan_classifies_disk_and_partition_lifecycle_safely() {
        let plan = plan_from_json_bytes(
            br#"{
              "disks": {
                "/dev/disk/by-id/nvme-root": {
                  "operation": "create",
                  "partitionType": "gpt"
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
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 3);
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
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 7);
        assert_eq!(plan.summary.offline_required_count, 5);
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
    }

    #[test]
    fn plan_classifies_luks_keyslot_lifecycle() {
        let plan = plan_from_json_bytes(
            br#"{
              "luksKeyslots": {
                "cryptroot:1": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "keySlot": "1",
                    "newKeyFile": "/run/keys/root-new"
                  }
                },
                "cryptroot:2": {
                  "destroy": true,
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
            .find(|action| action.id == "lukskeyslots:cryptroot:1:create")
            .expect("LUKS keyslot create action exists");
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
            .find(|action| action.id == "lukskeyslots:cryptroot:2:destroy")
            .expect("LUKS keyslot destroy action exists");
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
                  "desiredSize": "4TiB"
                },
                "old-cache": {
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 3);
        assert_eq!(plan.summary.offline_required_count, 0);
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
        let destroy = plan
            .actions
            .iter()
            .find(|action| action.id == "vdovolumes:old-cache:destroy")
            .expect("VDO destroy action exists");
        assert_eq!(destroy.risk, RiskClass::Destructive);
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
                "/mnt/persist/@old": {
                  "destroy": true,
                  "preserveData": false
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 2);
        let create = plan
            .actions
            .iter()
            .find(|action| {
                action.id == "btrfsSubvolumes:/mnt/persist/@home:create".to_ascii_lowercase()
            })
            .expect("create action exists");
        assert_eq!(create.risk, RiskClass::Online);
        assert_eq!(create.context.target.as_deref(), Some("/mnt/persist/@home"));
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
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 2);
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
                "tank/archive": {
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 4);
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

        assert_eq!(plan.summary.action_count, 4);
        assert_eq!(plan.summary.destructive_count, 1);
        assert_eq!(plan.summary.offline_required_count, 2);
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
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 3);
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
                "vg0/oldpool": {
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 3);
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
                "vg0/old-snap": {
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 3);
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
                "/dev/loop9": {
                  "operation": "destroy"
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
            .find(|action| action.id == "loopdevices:/dev/loop7:create")
            .expect("loop create action exists");
        assert_eq!(create.risk, RiskClass::Online);
        assert_eq!(
            create.context.device.as_deref(),
            Some("/var/lib/images/root.img")
        );
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
    fn plan_classifies_snapshot_rollback_as_potential_data_loss() {
        let plan = plan_from_json_bytes(
            br#"{
              "snapshots": {
                "tank/home@before-upgrade": {
                  "target": "tank/home",
                  "rollback": true
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 1);
        assert_eq!(plan.summary.potential_data_loss_count, 1);
        assert_eq!(plan.actions[0].operation, Operation::Rollback);
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
                  "operation": "create",
                  "device": "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0"
                },
                "iqn.2026-06.example:storage/old:1": {
                  "destroy": true,
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

        let create = plan
            .actions
            .iter()
            .find(|action| action.id == "luns:iqn.2026-06.example:storage/root:0:create")
            .expect("LUN create action exists");
        assert_eq!(create.operation, Operation::Create);
        assert_eq!(create.risk, RiskClass::Online);
        assert!(create.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("stable by-path"))
        }));

        let destroy = plan
            .actions
            .iter()
            .find(|action| action.id == "luns:iqn.2026-06.example:storage/old:1:destroy")
            .expect("LUN destroy action exists");
        assert_eq!(destroy.operation, Operation::Destroy);
        assert_eq!(destroy.risk, RiskClass::OfflineRequired);
        assert!(!destroy.destructive);
        assert!(destroy.advice.as_ref().is_some_and(|advice| {
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
    fn plan_classifies_iscsi_session_login_and_logout() {
        let plan = plan_from_json_bytes(
            br#"{
              "iscsiSessions": {
                "iqn.2026-06.example:storage.root": {
                  "operation": "create",
                  "metadata": {
                    "portal": "192.0.2.10:3260"
                  }
                },
                "iqn.2026-06.example:storage.old": {
                  "destroy": true,
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
            .find(|action| action.id == "iscsisessions:iqn.2026-06.example:storage.root:create")
            .expect("iSCSI create action exists");
        assert_eq!(create.operation, Operation::Create);
        assert_eq!(create.risk, RiskClass::Online);
        assert_eq!(create.context.portal.as_deref(), Some("192.0.2.10:3260"));

        let destroy = plan
            .actions
            .iter()
            .find(|action| action.id == "iscsisessions:iqn.2026-06.example:storage.old:destroy")
            .expect("iSCSI destroy action exists");
        assert_eq!(destroy.operation, Operation::Destroy);
        assert_eq!(destroy.risk, RiskClass::OfflineRequired);
        assert_eq!(destroy.context.portal.as_deref(), Some("192.0.2.11:3260"));
    }

    #[test]
    fn plan_classifies_nfs_export_lifecycle_without_data_destruction() {
        let plan = plan_from_json_bytes(
            br#"{
              "exports": {
                "/srv/share": {
                  "operation": "create",
                  "client": "192.0.2.0/24",
                  "options": "rw,sync,no_subtree_check"
                },
                "/srv/old": {
                  "destroy": true,
                  "client": "192.0.2.55"
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
            .find(|action| action.id == "exports:/srv/share:create")
            .expect("export create exists");
        assert_eq!(create.risk, RiskClass::Online);
        assert_eq!(create.context.client.as_deref(), Some("192.0.2.0/24"));
        assert_eq!(
            create.context.options.as_deref(),
            Some("rw,sync,no_subtree_check")
        );
        let destroy = plan
            .actions
            .iter()
            .find(|action| action.id == "exports:/srv/old:destroy")
            .expect("export destroy exists");
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
                    "operation": "create",
                    "source": "nas.example.com:/srv/shared",
                    "fsType": "nfs4",
                    "options": ["_netdev", "vers=4.2"]
                  },
                  "/srv/old": {
                    "destroy": true,
                    "source": "nas.example.com:/srv/old"
                  }
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
            .find(|action| action.id == "nfs.mounts:/srv/shared:create")
            .expect("NFS mount create exists");
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
            .find(|action| action.id == "nfs.mounts:/srv/old:destroy")
            .expect("NFS mount destroy exists");
        assert_eq!(destroy.risk, RiskClass::OfflineRequired);
        assert!(!destroy.destructive);
        assert_eq!(destroy.context.mountpoint.as_deref(), Some("/srv/old"));
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
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 3);
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

        assert_eq!(plan.summary.action_count, 4);
        assert_eq!(plan.summary.offline_required_count, 3);
        assert_eq!(create.risk, RiskClass::OfflineRequired);
        assert_eq!(add.risk, RiskClass::OfflineRequired);
        assert_eq!(remove.risk, RiskClass::OfflineRequired);
        assert_eq!(property.risk, RiskClass::Safe);
        assert!(remove.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("dirty data"))
        }));
    }

    #[test]
    fn apply_policy_blocks_destructive_and_potential_data_loss_actions() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
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
                "allowGrow": true,
                "allowOffline": false,
                "allowPropertyChanges": true
              }
            }"#,
        )
        .expect("document should parse");

        let report = evaluate_apply_policy(&plan, policy);

        assert_eq!(report.blocked_count, 2);
        assert_eq!(report.blocked_summary.destructive_count, 1);
        assert_eq!(report.blocked_summary.potential_data_loss_count, 1);
        assert!(!report.can_execute());
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
