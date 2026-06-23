use disk_nix_model::NodeKind;
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fs_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mountpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desired_size: Option<String>,
}

impl ActionContext {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.collection.is_none()
            && self.name.is_none()
            && self.target.is_none()
            && self.device.is_none()
            && self.replacement.is_none()
            && self.property.is_none()
            && self.property_value.is_none()
            && self.fs_type.is_none()
            && self.mountpoint.is_none()
            && self.desired_size.is_none()
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
    pub allow_grow: bool,
    pub allow_offline: bool,
    pub allow_property_changes: bool,
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
    for collection in [
        "volumes",
        "volumeGroups",
        "pools",
        "datasets",
        "luns",
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

    Plan { summary, actions }
}

fn blocked_action(action: &PlannedAction, policy: &ApplyPolicy) -> Option<BlockedAction> {
    let reason = if action.risk == RiskClass::Unsupported {
        Some("unsupported actions cannot be applied")
    } else if action.risk == RiskClass::OfflineRequired && !policy.allow_offline {
        Some("offline-required actions require allowOffline=true")
    } else if action.operation == Operation::Format && !policy.allow_format {
        Some("format actions require allowFormat=true")
    } else if action.operation == Operation::Shrink {
        (!policy.allow_shrink).then_some("shrink actions require allowShrink=true")
    } else if action.operation == Operation::Grow {
        (!policy.allow_grow).then_some("grow actions require allowGrow=true")
    } else if action.operation == Operation::SetProperty {
        (!policy.allow_property_changes)
            .then_some("property changes require allowPropertyChanges=true")
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

fn filesystem_context(
    name: &str,
    mountpoint: &str,
    fs_type: &str,
    desired_size: Option<String>,
) -> ActionContext {
    ActionContext {
        collection: Some("filesystems".to_string()),
        name: Some(name.to_string()),
        target: Some(mountpoint.to_string()),
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
        target: Some(name.to_string()),
        desired_size: desired_size(object),
        ..ActionContext::default()
    }
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

    match resize_policy {
        "grow-only" => actions.push(PlannedAction {
            id: format!("filesystem:{name}:grow"),
            description: format!(
                "allow non-destructive growth for {fs_type} filesystem at {mountpoint}"
            ),
            operation: Operation::Grow,
            risk: RiskClass::Online,
            destructive: false,
            context: filesystem_context(name, mountpoint, fs_type, desired_size.clone()),
            advice: None,
        }),
        "shrink-allowed" => actions.push(filesystem_shrink_action(
            name,
            mountpoint,
            fs_type,
            desired_size.clone(),
        )),
        _ => actions.push(PlannedAction {
            id: format!("filesystem:{name}:inspect"),
            description: format!("inspect {fs_type} filesystem declaration at {mountpoint}"),
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            destructive: false,
            context: filesystem_context(name, mountpoint, fs_type, desired_size.clone()),
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
            context: filesystem_context(name, mountpoint, fs_type, desired_size),
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
}

fn filesystem_shrink_action(
    name: &str,
    mountpoint: &str,
    fs_type: &str,
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
        context: filesystem_context(name, mountpoint, fs_type, desired_size),
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
            actions.push(PlannedAction {
                id: format!("{collection}:{name}:add-device:{device}"),
                description: format!("add device {device} to {collection} {name}"),
                operation: Operation::AddDevice,
                risk: RiskClass::Online,
                destructive: false,
                context: ActionContext {
                    device: Some(device.to_string()),
                    ..lifecycle_context(collection, name, object)
                },
                advice: None,
            });
        }
    }

    if let Some(devices) = object.get("removeDevices").and_then(Value::as_array) {
        for device in devices.iter().filter_map(Value::as_str) {
            actions.push(PlannedAction {
                id: format!("{collection}:{name}:remove-device:{device}"),
                description: format!("remove device {device} from {collection} {name}"),
                operation: Operation::RemoveDevice,
                risk: RiskClass::PotentialDataLoss,
                destructive: false,
                context: ActionContext {
                    device: Some(device.to_string()),
                    ..lifecycle_context(collection, name, object)
                },
                advice: Some(Advice {
                    summary: "device removal requires enough remaining data and metadata capacity"
                        .to_string(),
                    alternatives: vec![
                        "add replacement capacity before removing the old device".to_string(),
                        "rebalance or evacuate data before removal".to_string(),
                        "verify redundancy and current health before applying".to_string(),
                    ],
                }),
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
        actions.push(PlannedAction {
            id: format!("{collection}:{name}:set-property:{property}"),
            description: format!("set property {property} on {collection} {name}"),
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            destructive: false,
            context: ActionContext {
                property: Some(property.to_string()),
                property_value: Some(property_value(value)),
                ..lifecycle_context(collection, name, object)
            },
            advice: None,
        });
    }
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
        actions.push(PlannedAction {
            id: format!("{collection}:{name}:destroy"),
            description: format!("{collection} {name} may be destroyed or replaced"),
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            destructive: true,
            context: lifecycle_context(collection, name, object),
            advice: Some(Advice {
                summary: "destroying or replacing storage removes live data".to_string(),
                alternatives: vec![
                    "take and verify a backup before destructive changes".to_string(),
                    "rename, detach, or unmount first when supported".to_string(),
                    "migrate data to replacement storage before removal".to_string(),
                ],
            }),
        });
    }
}

fn add_snapshot_actions(actions: &mut Vec<PlannedAction>, name: &str, snapshot: &Value) {
    let target = snapshot
        .get("target")
        .and_then(Value::as_str)
        .unwrap_or(name);
    let destroy = snapshot
        .get("destroy")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let rollback = snapshot
        .get("rollback")
        .and_then(Value::as_bool)
        .unwrap_or(false);

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
    } else {
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
                ..ActionContext::default()
            },
            advice: None,
        });
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
        Operation::Create | Operation::SetProperty => (RiskClass::Safe, false, None),
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
        Operation::Grow | Operation::AddDevice | Operation::Rebalance => {
            (RiskClass::Online, false, None)
        }
        Operation::ReplaceDevice => {
            let (risk, advice) = classify_replace_device(collection);
            (risk, false, Some(advice))
        }
        Operation::Snapshot => (RiskClass::Reversible, false, None),
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
    if collection == "caches" {
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

fn destructive_alternatives(collection: &str, object: &Value) -> Vec<String> {
    let mut alternatives = vec![
        "take and verify a backup before destructive changes".to_string(),
        "migrate data to replacement storage first".to_string(),
    ];

    match collection {
        "pools" | "datasets" => {
            alternatives.push("take a recursive snapshot before destroy or rollback".to_string());
            alternatives
                .push("rename or unmount the dataset while validating consumers".to_string());
        }
        "volumes" | "volumeGroups" | "luns" => {
            alternatives
                .push("grow or attach replacement capacity instead of reformatting".to_string());
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
            node_kind: NodeKind::Filesystem,
            operation: Operation::Grow,
            risk: RiskClass::Online,
            advice: None,
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
            node_kind: NodeKind::ZfsPool,
            operation: Operation::AddDevice,
            risk: RiskClass::Online,
            advice: None,
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
            node_kind: NodeKind::VdoVolume,
            operation: Operation::Grow,
            risk: RiskClass::Online,
            advice: None,
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
    fn plan_classifies_lun_growth_as_offline_required() {
        let plan = plan_from_json_bytes(
            br#"{
              "luns": {
                "iqn.2026-06.example:storage/root:0": {
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
        assert!(plan.actions[0].advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("multipath"))
        }));
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
    fn plan_classifies_cache_replacement_as_offline_required() {
        let plan = plan_from_json_bytes(
            br#"{
              "caches": {
                "vg0/root-cache": {
                  "operation": "replace-device",
                  "replaceDevices": {
                    "/dev/sdb": "/dev/sdc"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 2);
        assert_eq!(plan.summary.offline_required_count, 2);
        assert!(plan.actions.iter().all(|action| {
            action.operation == Operation::ReplaceDevice
                && action.risk == RiskClass::OfflineRequired
                && action.advice.as_ref().is_some_and(|advice| {
                    advice
                        .alternatives
                        .iter()
                        .any(|alternative| alternative.contains("flush dirty data"))
                })
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
}
