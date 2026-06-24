use disk_nix_model::{Node, NodeKind, Relationship, StorageGraph};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

pub const SUPPORTED_SPEC_VERSION: u64 = 1;

#[derive(Debug)]
pub enum PlanDocumentError {
    Json(serde_json::Error),
    UnsupportedVersion { found: u64, supported: u64 },
    InvalidVersion { location: &'static str },
    ConflictingVersions { top_level: u64, spec: u64 },
}

impl fmt::Display for PlanDocumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => error.fmt(formatter),
            Self::UnsupportedVersion { found, supported } => write!(
                formatter,
                "unsupported disk-nix spec version {found}; supported version is {supported}"
            ),
            Self::InvalidVersion { location } => {
                write!(
                    formatter,
                    "disk-nix spec version at {location} must be an integer"
                )
            }
            Self::ConflictingVersions { top_level, spec } => write!(
                formatter,
                "conflicting disk-nix spec versions: top-level version {top_level}, spec.version {spec}"
            ),
        }
    }
}

impl Error for PlanDocumentError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            Self::UnsupportedVersion { .. }
            | Self::InvalidVersion { .. }
            | Self::ConflictingVersions { .. } => None,
        }
    }
}

impl From<serde_json::Error> for PlanDocumentError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependency_order: Vec<ActionDependencyOrder>,
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
pub struct ActionDependencyOrder {
    pub action_id: String,
    pub phase: DependencyPhase,
    pub direction: DependencyDirection,
    pub layer_rank: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unblocks: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DependencyPhase {
    BuildLowerLayers,
    MutateInPlace,
    TearDownUpperLayers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DependencyDirection {
    LowerLayersFirst,
    UpperLayersFirst,
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
    pub suppressed_action_count: usize,
    #[serde(default)]
    pub graph_dependency_edge_count: usize,
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
    FilesystemFormatAlreadySatisfied,
    FilesystemTypeConflict,
    DiskCreateAlreadySatisfied,
    DiskCreateRequired,
    BtrfsSubvolumeCreateAlreadySatisfied,
    BtrfsSubvolumeCreateRequired,
    BtrfsSubvolumeDestroyAlreadySatisfied,
    BtrfsSubvolumeDestroyRequired,
    BtrfsQgroupCreateAlreadySatisfied,
    BtrfsQgroupCreateRequired,
    BtrfsQgroupDestroyAlreadySatisfied,
    BtrfsQgroupDestroyRequired,
    BcacheDetachAlreadySatisfied,
    BcacheDetachRequired,
    BackingFileCreateAlreadySatisfied,
    BackingFileCreateRequired,
    PartitionCreateAlreadySatisfied,
    PartitionCreateRequired,
    LvmPvCreateAlreadySatisfied,
    LvmPvCreateRequired,
    LvmVolumeCreateAlreadySatisfied,
    LvmVolumeCreateRequired,
    LvmVgCreateAlreadySatisfied,
    LvmVgCreateRequired,
    IscsiLoginAlreadySatisfied,
    IscsiLoginRequired,
    IscsiLogoutAlreadySatisfied,
    IscsiLogoutRequired,
    LunAttachAlreadySatisfied,
    LunAttachRequired,
    LunDetachAlreadySatisfied,
    LunDetachRequired,
    DmMapDestroyAlreadySatisfied,
    DmMapDestroyRequired,
    NvmeNamespaceAttachAlreadySatisfied,
    NvmeNamespaceAttachRequired,
    NvmeNamespaceDetachAlreadySatisfied,
    NvmeNamespaceDetachRequired,
    LvmActivateAlreadySatisfied,
    LvmActivateRequired,
    LvmDeactivateAlreadySatisfied,
    LvmDeactivateRequired,
    LvmVgExportAlreadySatisfied,
    LvmVgExportRequired,
    LvmVgImportAlreadySatisfied,
    LvmVgImportRequired,
    LvmCacheDetachAlreadySatisfied,
    LvmCacheDetachRequired,
    LuksCloseAlreadySatisfied,
    LuksCloseRequired,
    LuksFormatTargetPresent,
    LuksOpenAlreadySatisfied,
    LuksOpenRequired,
    LuksKeyslotRemoveAlreadySatisfied,
    LuksKeyslotRemoveRequired,
    LuksTokenRemoveAlreadySatisfied,
    LuksTokenRemoveRequired,
    MultipathDestroyAlreadySatisfied,
    MultipathDestroyRequired,
    MultipathPathAddAlreadySatisfied,
    MultipathPathAddRequired,
    MultipathPathRemoveAlreadySatisfied,
    MultipathPathRemoveRequired,
    SwapDeactivateAlreadySatisfied,
    SwapDeactivateRequired,
    SwapDestroyAlreadySatisfied,
    SwapDestroyRequired,
    SwapFormatTargetPresent,
    LoopCreateAlreadySatisfied,
    LoopCreateConflict,
    LoopCreateRequired,
    LoopDetachAlreadySatisfied,
    LoopDetachRequired,
    MdCreateAlreadySatisfied,
    MdCreateRequired,
    MdAssembleAlreadySatisfied,
    MdAssembleRequired,
    MdStopAlreadySatisfied,
    MdStopRequired,
    MdMemberAddAlreadySatisfied,
    MdMemberAddRequired,
    MdMemberRemoveAlreadySatisfied,
    MdMemberRemoveRequired,
    MdMemberReplaceAlreadySatisfied,
    MdMemberReplaceRequired,
    MountAlreadySatisfied,
    MountSourceConflict,
    MountOptionsAlreadySatisfied,
    MountOptionsDiffer,
    UnmountAlreadySatisfied,
    UnmountRequired,
    NfsExportAlreadySatisfied,
    NfsExportDiffers,
    NfsUnexportAlreadySatisfied,
    NfsUnexportRequired,
    PropertyAlreadySatisfied,
    PropertyDiffers,
    SnapshotCloneSourceAvailable,
    SnapshotCloneSourceMissing,
    SnapshotDestroyAlreadySatisfied,
    SnapshotDestroyRequired,
    SnapshotRenameRequired,
    SnapshotRenameSourceMissing,
    SnapshotRollbackPointAvailable,
    SnapshotRollbackPointMissing,
    VdoCreateTargetPresent,
    VdoDestroyAlreadySatisfied,
    VdoDestroyRequired,
    VdoGrowRequired,
    VdoStartAlreadySatisfied,
    VdoStartRequired,
    VdoStopAlreadySatisfied,
    VdoStopRequired,
    ZfsObjectCreateAlreadySatisfied,
    ZfsObjectCreateRequired,
    ZfsObjectDestroyAlreadySatisfied,
    ZfsObjectDestroyRequired,
    ZfsPoolCreateAlreadySatisfied,
    ZfsPoolCreateRequired,
    ZfsPoolImportAlreadySatisfied,
    ZfsPoolImportRequired,
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
    pub cache_set_uuid: Option<String>,
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
    pub physical_size: Option<String>,
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
            && self.cache_set_uuid.is_none()
            && self.rename_to.is_none()
            && self.property.is_none()
            && self.property_value.is_none()
            && self.property_assignments.is_empty()
            && self.fs_type.is_none()
            && self.mountpoint.is_none()
            && self.desired_size.is_none()
            && self.physical_size.is_none()
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

pub fn plan_from_json_bytes(bytes: &[u8]) -> Result<Plan, PlanDocumentError> {
    let value: Value = serde_json::from_slice(bytes)?;
    validate_spec_version(&value)?;
    Ok(plan_from_value(&value))
}

pub fn plan_and_policy_from_json_bytes(
    bytes: &[u8],
) -> Result<(Plan, ApplyPolicy), PlanDocumentError> {
    let value: Value = serde_json::from_slice(bytes)?;
    validate_spec_version(&value)?;
    let plan = plan_from_value(&value);
    let policy = apply_policy_from_value(&value)?;
    Ok((plan, policy))
}

fn validate_spec_version(value: &Value) -> Result<(), PlanDocumentError> {
    let top_level = read_spec_version(value.get("version"), "version")?;
    let spec = value
        .get("spec")
        .and_then(Value::as_object)
        .and_then(|spec| spec.get("version"))
        .map_or(Ok(None), |version| {
            read_spec_version(Some(version), "spec.version")
        })?;

    if let (Some(top_level), Some(spec)) = (top_level, spec) {
        if top_level != spec {
            return Err(PlanDocumentError::ConflictingVersions { top_level, spec });
        }
    }

    let version = top_level.or(spec).unwrap_or(SUPPORTED_SPEC_VERSION);
    if version != SUPPORTED_SPEC_VERSION {
        return Err(PlanDocumentError::UnsupportedVersion {
            found: version,
            supported: SUPPORTED_SPEC_VERSION,
        });
    }

    Ok(())
}

fn read_spec_version(
    version: Option<&Value>,
    location: &'static str,
) -> Result<Option<u64>, PlanDocumentError> {
    match version {
        Some(Value::Number(number)) => number
            .as_u64()
            .map(Some)
            .ok_or(PlanDocumentError::InvalidVersion { location }),
        Some(_) => Err(PlanDocumentError::InvalidVersion { location }),
        None => Ok(None),
    }
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
    if let Some(zram) = spec.get("zram").and_then(Value::as_object) {
        if !zram.is_empty() {
            add_zram_actions(&mut actions, zram);
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
        "backingFiles",
        "dmMaps",
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
    order_plan_actions(&mut actions);

    Plan {
        summary: plan_summary(&actions),
        dependency_order: dependency_order_for_actions(&actions),
        actions,
        topology_comparison: None,
    }
}

#[must_use]
pub fn compare_plan_with_topology(mut plan: Plan, graph: &StorageGraph) -> Plan {
    let original_action_count = plan.actions.len();
    let diagnostics: Vec<TopologyDiagnostic> = plan
        .actions
        .iter()
        .flat_map(|action| topology_diagnostics_for_action(action, graph))
        .collect();

    let suppressed_action_ids = already_satisfied_action_ids(&plan.actions, &diagnostics);
    let suppressed_action_count = suppressed_action_ids.len();

    if suppressed_action_count > 0 {
        plan.actions
            .retain(|action| !suppressed_action_ids.contains(&action.id));
        plan.summary = plan_summary(&plan.actions);
    }
    let graph_edges = graph_dependency_edges_for_actions(&plan.actions, graph);
    let graph_dependency_edge_count = graph_edges.graph_edges.len();
    plan.dependency_order = dependency_order_for_actions_with_edges(&plan.actions, graph_edges);

    let summary = TopologyComparisonSummary {
        action_count: original_action_count,
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
                        | TopologyDiagnosticKind::FilesystemFormatAlreadySatisfied
                        | TopologyDiagnosticKind::DiskCreateAlreadySatisfied
                        | TopologyDiagnosticKind::BtrfsSubvolumeCreateAlreadySatisfied
                        | TopologyDiagnosticKind::BtrfsQgroupCreateAlreadySatisfied
                        | TopologyDiagnosticKind::BackingFileCreateAlreadySatisfied
                        | TopologyDiagnosticKind::PartitionCreateAlreadySatisfied
                        | TopologyDiagnosticKind::LvmPvCreateAlreadySatisfied
                        | TopologyDiagnosticKind::LvmVolumeCreateAlreadySatisfied
                        | TopologyDiagnosticKind::LvmVgCreateAlreadySatisfied
                        | TopologyDiagnosticKind::BtrfsSubvolumeDestroyAlreadySatisfied
                        | TopologyDiagnosticKind::BtrfsQgroupDestroyAlreadySatisfied
                        | TopologyDiagnosticKind::BcacheDetachAlreadySatisfied
                        | TopologyDiagnosticKind::DmMapDestroyAlreadySatisfied
                        | TopologyDiagnosticKind::IscsiLoginAlreadySatisfied
                        | TopologyDiagnosticKind::IscsiLogoutAlreadySatisfied
                        | TopologyDiagnosticKind::LunAttachAlreadySatisfied
                        | TopologyDiagnosticKind::LunDetachAlreadySatisfied
                        | TopologyDiagnosticKind::NvmeNamespaceAttachAlreadySatisfied
                        | TopologyDiagnosticKind::NvmeNamespaceDetachAlreadySatisfied
                        | TopologyDiagnosticKind::LvmActivateAlreadySatisfied
                        | TopologyDiagnosticKind::LvmDeactivateAlreadySatisfied
                        | TopologyDiagnosticKind::LvmVgExportAlreadySatisfied
                        | TopologyDiagnosticKind::LvmVgImportAlreadySatisfied
                        | TopologyDiagnosticKind::LvmCacheDetachAlreadySatisfied
                        | TopologyDiagnosticKind::LuksCloseAlreadySatisfied
                        | TopologyDiagnosticKind::LuksOpenAlreadySatisfied
                        | TopologyDiagnosticKind::LuksKeyslotRemoveAlreadySatisfied
                        | TopologyDiagnosticKind::LuksTokenRemoveAlreadySatisfied
                        | TopologyDiagnosticKind::MultipathDestroyAlreadySatisfied
                        | TopologyDiagnosticKind::MultipathPathAddAlreadySatisfied
                        | TopologyDiagnosticKind::MultipathPathRemoveAlreadySatisfied
                        | TopologyDiagnosticKind::SwapDeactivateAlreadySatisfied
                        | TopologyDiagnosticKind::SwapDestroyAlreadySatisfied
                        | TopologyDiagnosticKind::LoopCreateAlreadySatisfied
                        | TopologyDiagnosticKind::LoopDetachAlreadySatisfied
                        | TopologyDiagnosticKind::MdCreateAlreadySatisfied
                        | TopologyDiagnosticKind::MdAssembleAlreadySatisfied
                        | TopologyDiagnosticKind::MdStopAlreadySatisfied
                        | TopologyDiagnosticKind::MdMemberAddAlreadySatisfied
                        | TopologyDiagnosticKind::MdMemberRemoveAlreadySatisfied
                        | TopologyDiagnosticKind::MdMemberReplaceAlreadySatisfied
                        | TopologyDiagnosticKind::MountAlreadySatisfied
                        | TopologyDiagnosticKind::MountOptionsAlreadySatisfied
                        | TopologyDiagnosticKind::UnmountAlreadySatisfied
                        | TopologyDiagnosticKind::NfsExportAlreadySatisfied
                        | TopologyDiagnosticKind::NfsUnexportAlreadySatisfied
                        | TopologyDiagnosticKind::PropertyAlreadySatisfied
                        | TopologyDiagnosticKind::SnapshotDestroyAlreadySatisfied
                        | TopologyDiagnosticKind::VdoDestroyAlreadySatisfied
                        | TopologyDiagnosticKind::VdoStartAlreadySatisfied
                        | TopologyDiagnosticKind::VdoStopAlreadySatisfied
                        | TopologyDiagnosticKind::ZfsObjectCreateAlreadySatisfied
                        | TopologyDiagnosticKind::ZfsObjectDestroyAlreadySatisfied
                        | TopologyDiagnosticKind::ZfsPoolCreateAlreadySatisfied
                        | TopologyDiagnosticKind::ZfsPoolImportAlreadySatisfied
                )
            })
            .count(),
        suppressed_action_count,
        graph_dependency_edge_count,
    };

    plan.topology_comparison = Some(TopologyComparison {
        summary,
        diagnostics,
    });
    plan
}

fn order_plan_actions(actions: &mut [PlannedAction]) {
    actions.sort_by_key(action_order_key);
}

fn action_order_key(action: &PlannedAction) -> (u16, u16) {
    let rank = collection_dependency_rank(action.context.collection.as_deref());
    let layer = if operation_runs_upper_layers_first(action.operation) {
        u16::MAX - rank
    } else {
        rank
    };

    (operation_dependency_phase(action.operation), layer)
}

fn dependency_order_for_actions(actions: &[PlannedAction]) -> Vec<ActionDependencyOrder> {
    let edges = dependency_edges_for_actions(actions);
    dependency_order_for_actions_with_edges(actions, edges)
}

fn dependency_order_for_actions_with_edges(
    actions: &[PlannedAction],
    edges: DependencyEdges,
) -> Vec<ActionDependencyOrder> {
    actions
        .iter()
        .map(|action| {
            let collection = action.context.collection.clone();
            let layer_rank = collection_dependency_rank(collection.as_deref());
            let direction = if operation_runs_upper_layers_first(action.operation) {
                DependencyDirection::UpperLayersFirst
            } else {
                DependencyDirection::LowerLayersFirst
            };
            ActionDependencyOrder {
                action_id: action.id.clone(),
                phase: operation_dependency_phase_kind(action.operation),
                direction,
                layer_rank,
                collection,
                depends_on: edges
                    .depends_on
                    .get(&action.id)
                    .cloned()
                    .unwrap_or_default(),
                unblocks: edges.unblocks.get(&action.id).cloned().unwrap_or_default(),
                notes: dependency_order_notes(action, direction, layer_rank, &edges),
            }
        })
        .collect()
}

#[derive(Debug, Default)]
struct DependencyEdges {
    depends_on: BTreeMap<String, Vec<String>>,
    unblocks: BTreeMap<String, Vec<String>>,
    graph_edges: BTreeSet<(String, String)>,
}

fn dependency_edges_for_actions(actions: &[PlannedAction]) -> DependencyEdges {
    let mut edges = DependencyEdges::default();
    for consumer in actions {
        let consumer_inputs = action_dependency_inputs(consumer);
        if consumer_inputs.is_empty() {
            continue;
        }
        let consumer_rank = collection_dependency_rank(consumer.context.collection.as_deref());
        let consumer_direction = if operation_runs_upper_layers_first(consumer.operation) {
            DependencyDirection::UpperLayersFirst
        } else {
            DependencyDirection::LowerLayersFirst
        };

        for provider in actions {
            if provider.id == consumer.id {
                continue;
            }
            let provider_identities = action_dependency_identities(provider);
            if provider_identities.is_empty()
                || !consumer_inputs
                    .iter()
                    .any(|input| provider_identities.contains(input))
            {
                continue;
            }

            let provider_rank = collection_dependency_rank(provider.context.collection.as_deref());
            let provider_direction = if operation_runs_upper_layers_first(provider.operation) {
                DependencyDirection::UpperLayersFirst
            } else {
                DependencyDirection::LowerLayersFirst
            };
            let edge = match consumer_direction {
                DependencyDirection::LowerLayersFirst
                    if provider_direction == DependencyDirection::LowerLayersFirst
                        && provider_rank < consumer_rank =>
                {
                    Some((provider.id.as_str(), consumer.id.as_str()))
                }
                DependencyDirection::UpperLayersFirst
                    if provider_direction == DependencyDirection::UpperLayersFirst
                        && provider_rank > consumer_rank =>
                {
                    Some((provider.id.as_str(), consumer.id.as_str()))
                }
                _ => None,
            };

            if let Some((depends_on, action_id)) = edge {
                insert_unique_sorted(&mut edges.depends_on, action_id, depends_on);
                insert_unique_sorted(&mut edges.unblocks, depends_on, action_id);
            }
        }
    }
    edges
}

fn graph_dependency_edges_for_actions(
    actions: &[PlannedAction],
    graph: &StorageGraph,
) -> DependencyEdges {
    let mut edges = dependency_edges_for_actions(actions);
    let matches = graph_action_matches(actions, graph);
    let reachability = graph_storage_reachability(graph);
    for (lower_id, upper_ids) in reachability {
        for upper_id in upper_ids {
            for lower_action in actions_for_node(&matches, &lower_id) {
                for upper_action in actions_for_node(&matches, &upper_id) {
                    if lower_action.id == upper_action.id {
                        continue;
                    }
                    let lower_direction = dependency_direction(lower_action.operation);
                    let upper_direction = dependency_direction(upper_action.operation);
                    if lower_direction != upper_direction {
                        continue;
                    }
                    let (depends_on, action_id) = match lower_direction {
                        DependencyDirection::LowerLayersFirst => {
                            (lower_action.id.as_str(), upper_action.id.as_str())
                        }
                        DependencyDirection::UpperLayersFirst => {
                            (upper_action.id.as_str(), lower_action.id.as_str())
                        }
                    };
                    insert_unique_sorted(&mut edges.depends_on, action_id, depends_on);
                    insert_unique_sorted(&mut edges.unblocks, depends_on, action_id);
                    edges
                        .graph_edges
                        .insert((action_id.to_string(), depends_on.to_string()));
                }
            }
        }
    }
    edges
}

fn graph_storage_reachability(graph: &StorageGraph) -> BTreeMap<String, BTreeSet<String>> {
    let mut adjacency: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for edge in &graph.edges {
        if let Some((lower_id, upper_id)) = normalized_storage_edge(edge) {
            adjacency
                .entry(lower_id.to_string())
                .or_default()
                .insert(upper_id.to_string());
        }
    }

    let mut reachability = BTreeMap::new();
    for lower_id in adjacency.keys() {
        let mut visited = BTreeSet::new();
        let mut pending: Vec<String> = adjacency
            .get(lower_id)
            .into_iter()
            .flat_map(|upper_ids| upper_ids.iter().cloned())
            .collect();
        while let Some(upper_id) = pending.pop() {
            if !visited.insert(upper_id.clone()) {
                continue;
            }
            if let Some(next_ids) = adjacency.get(&upper_id) {
                pending.extend(next_ids.iter().cloned());
            }
        }
        reachability.insert(lower_id.clone(), visited);
    }
    reachability
}

fn graph_action_matches<'a>(
    actions: &'a [PlannedAction],
    graph: &StorageGraph,
) -> BTreeMap<String, Vec<&'a PlannedAction>> {
    let mut matches: BTreeMap<String, Vec<&PlannedAction>> = BTreeMap::new();
    for action in actions {
        let Some(query) = topology_query(action) else {
            continue;
        };
        for node in graph.find_nodes(&query) {
            matches.entry(node.id.0.clone()).or_default().push(action);
        }
    }
    matches
}

fn actions_for_node<'a>(
    matches: &'a BTreeMap<String, Vec<&'a PlannedAction>>,
    node_id: &str,
) -> &'a [&'a PlannedAction] {
    matches.get(node_id).map(Vec::as_slice).unwrap_or(&[])
}

fn normalized_storage_edge(edge: &disk_nix_model::Edge) -> Option<(&str, &str)> {
    match edge.relationship {
        Relationship::Contains
        | Relationship::Backs
        | Relationship::MapsTo
        | Relationship::MemberOf
        | Relationship::MountedAt
        | Relationship::CacheFor
        | Relationship::ImportedFrom
        | Relationship::Exports => Some((edge.from.0.as_str(), edge.to.0.as_str())),
        Relationship::SnapshotOf | Relationship::DependsOn => {
            Some((edge.to.0.as_str(), edge.from.0.as_str()))
        }
    }
}

fn dependency_direction(operation: Operation) -> DependencyDirection {
    if operation_runs_upper_layers_first(operation) {
        DependencyDirection::UpperLayersFirst
    } else {
        DependencyDirection::LowerLayersFirst
    }
}

fn action_dependency_inputs(action: &PlannedAction) -> BTreeSet<String> {
    let mut inputs = BTreeSet::new();
    insert_identity(&mut inputs, action.context.device.as_deref());
    for device in &action.context.devices {
        insert_identity(&mut inputs, Some(device));
    }
    match action.context.collection.as_deref() {
        Some("loopDevices") => insert_identity(&mut inputs, action.context.device.as_deref()),
        Some("filesystems") | Some("swaps") => {
            insert_identity(&mut inputs, action.context.target.as_deref());
            insert_identity(&mut inputs, action.context.device.as_deref());
        }
        Some("luks.devices")
        | Some("physicalVolumes")
        | Some("vdoVolumes")
        | Some("partitions")
        | Some("multipathMaps")
        | Some("mdRaids")
        | Some("caches") => {
            insert_identity(&mut inputs, action.context.target.as_deref());
            insert_identity(&mut inputs, action.context.device.as_deref());
        }
        Some("luns") => {
            insert_identity(&mut inputs, action.context.portal.as_deref());
            insert_identity(&mut inputs, action.context.target.as_deref());
        }
        Some("volumes") | Some("thinPools") | Some("lvmCaches") | Some("lvmSnapshots") => {
            insert_lvm_parent_identities(&mut inputs, action.context.target.as_deref());
            insert_lvm_parent_identities(&mut inputs, action.context.name.as_deref());
        }
        Some("datasets") | Some("zvols") => {
            insert_zfs_parent_identities(&mut inputs, action.context.target.as_deref());
            insert_zfs_parent_identities(&mut inputs, action.context.name.as_deref());
        }
        Some("snapshots") => {
            insert_identity(&mut inputs, action.context.target.as_deref());
            insert_snapshot_source_identity(&mut inputs, action.context.name.as_deref());
        }
        Some("btrfsSubvolumes") | Some("btrfsQgroups") | Some("nfs.mounts") | Some("exports") => {
            insert_identity(&mut inputs, action.context.target.as_deref());
            insert_identity(&mut inputs, action.context.mountpoint.as_deref());
        }
        _ => {}
    }
    inputs
}

fn action_dependency_identities(action: &PlannedAction) -> BTreeSet<String> {
    let mut identities = BTreeSet::new();
    insert_identity(&mut identities, action.context.name.as_deref());
    insert_identity(&mut identities, action.context.target.as_deref());
    insert_identity(&mut identities, action.context.device.as_deref());
    insert_identity(&mut identities, action.context.mountpoint.as_deref());
    for device in &action.context.devices {
        insert_identity(&mut identities, Some(device));
    }
    if action.context.collection.as_deref() == Some("iscsiSessions") {
        insert_identity(&mut identities, action.context.portal.as_deref());
    }
    identities
}

fn insert_lvm_parent_identities(identities: &mut BTreeSet<String>, value: Option<&str>) {
    let Some(value) = value else {
        return;
    };
    if let Some((vg, _lv)) = value.split_once('/') {
        insert_identity(identities, Some(vg));
    }
}

fn insert_zfs_parent_identities(identities: &mut BTreeSet<String>, value: Option<&str>) {
    let Some(value) = value else {
        return;
    };
    if let Some((pool, _rest)) = value.split_once('/') {
        insert_identity(identities, Some(pool));
    }
    if let Some((dataset, _snapshot)) = value.split_once('@') {
        insert_identity(identities, Some(dataset));
        insert_zfs_parent_identities(identities, Some(dataset));
    }
}

fn insert_snapshot_source_identity(identities: &mut BTreeSet<String>, value: Option<&str>) {
    let Some(value) = value else {
        return;
    };
    if let Some((dataset, _snapshot)) = value.split_once('@') {
        insert_identity(identities, Some(dataset));
    }
}

fn insert_identity(identities: &mut BTreeSet<String>, value: Option<&str>) {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    identities.insert(value.to_string());
}

fn insert_unique_sorted(map: &mut BTreeMap<String, Vec<String>>, key: &str, value: &str) {
    let values = map.entry(key.to_string()).or_default();
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
        values.sort();
    }
}

fn dependency_order_notes(
    action: &PlannedAction,
    direction: DependencyDirection,
    layer_rank: u16,
    edges: &DependencyEdges,
) -> Vec<String> {
    let mut notes = vec![format!(
        "collection layer rank {layer_rank} orders {} actions",
        action
            .context
            .collection
            .as_deref()
            .unwrap_or("unclassified")
    )];
    match direction {
        DependencyDirection::LowerLayersFirst => notes.push(
            "lower storage layers are planned before consumers for build, grow, rescan, and property work"
                .to_string(),
        ),
        DependencyDirection::UpperLayersFirst => notes.push(
            "consumer layers are planned before backing layers for teardown, shrink, rollback, detach, and destroy work"
                .to_string(),
        ),
    }
    if let Some(depends_on) = edges.depends_on.get(&action.id) {
        notes.push(format!(
            "explicit dependency edge requires {} before this action",
            depends_on.join(", ")
        ));
        let graph_depends_on: Vec<&str> = depends_on
            .iter()
            .filter(|depends_on| {
                edges
                    .graph_edges
                    .contains(&(action.id.clone(), (*depends_on).clone()))
            })
            .map(String::as_str)
            .collect();
        if !graph_depends_on.is_empty() {
            notes.push(format!(
                "current topology graph path requires {} before this action",
                graph_depends_on.join(", ")
            ));
        }
    }
    if let Some(unblocks) = edges.unblocks.get(&action.id) {
        notes.push(format!(
            "this action unblocks explicit dependent action(s): {}",
            unblocks.join(", ")
        ));
        let graph_unblocks: Vec<&str> = unblocks
            .iter()
            .filter(|unblocks| {
                edges
                    .graph_edges
                    .contains(&((*unblocks).clone(), action.id.clone()))
            })
            .map(String::as_str)
            .collect();
        if !graph_unblocks.is_empty() {
            notes.push(format!(
                "current topology graph path shows this action unblocks {}",
                graph_unblocks.join(", ")
            ));
        }
    }
    notes
}

fn operation_dependency_phase_kind(operation: Operation) -> DependencyPhase {
    match operation {
        Operation::Create
        | Operation::Import
        | Operation::Login
        | Operation::Attach
        | Operation::Open
        | Operation::Activate
        | Operation::Assemble
        | Operation::Start => DependencyPhase::BuildLowerLayers,
        Operation::Format
        | Operation::Grow
        | Operation::AddDevice
        | Operation::ReplaceDevice
        | Operation::AddKey
        | Operation::ImportToken
        | Operation::SetProperty
        | Operation::Snapshot
        | Operation::Clone
        | Operation::Promote
        | Operation::Mount
        | Operation::Remount
        | Operation::Check
        | Operation::Repair
        | Operation::Scrub
        | Operation::Trim
        | Operation::Rescan
        | Operation::Rename
        | Operation::Rebalance => DependencyPhase::MutateInPlace,
        Operation::Shrink
        | Operation::RemoveDevice
        | Operation::RemoveKey
        | Operation::RemoveToken
        | Operation::Rollback
        | Operation::Unmount
        | Operation::Close
        | Operation::Logout
        | Operation::Deactivate
        | Operation::Stop
        | Operation::Detach
        | Operation::Export
        | Operation::Unexport
        | Operation::Destroy => DependencyPhase::TearDownUpperLayers,
    }
}

fn operation_dependency_phase(operation: Operation) -> u16 {
    match operation_dependency_phase_kind(operation) {
        DependencyPhase::BuildLowerLayers => 10,
        DependencyPhase::MutateInPlace => 20,
        DependencyPhase::TearDownUpperLayers => 30,
    }
}

fn operation_runs_upper_layers_first(operation: Operation) -> bool {
    matches!(
        operation,
        Operation::Shrink
            | Operation::RemoveDevice
            | Operation::RemoveKey
            | Operation::RemoveToken
            | Operation::Rollback
            | Operation::Unmount
            | Operation::Close
            | Operation::Logout
            | Operation::Deactivate
            | Operation::Stop
            | Operation::Detach
            | Operation::Export
            | Operation::Unexport
            | Operation::Destroy
    )
}

fn collection_dependency_rank(collection: Option<&str>) -> u16 {
    match collection {
        Some("backingFiles") => 10,
        Some("loopDevices") => 15,
        Some("disks") => 20,
        Some("iscsiSessions") => 25,
        Some("nvmeNamespaces") => 30,
        Some("luns") => 35,
        Some("partitions") => 40,
        Some("mdRaids") | Some("multipathMaps") => 45,
        Some("luks.devices") | Some("dmMaps") => 50,
        Some("physicalVolumes") => 55,
        Some("volumeGroups") => 60,
        Some("thinPools") | Some("volumes") | Some("lvmCaches") | Some("lvmSnapshots") => 65,
        Some("vdoVolumes") | Some("caches") => 70,
        Some("pools") => 75,
        Some("datasets") | Some("zvols") => 80,
        Some("btrfsSubvolumes") | Some("btrfsQgroups") => 85,
        Some("filesystems") | Some("swaps") | Some("zram") | Some("nfs.mounts") => 90,
        Some("snapshots") | Some("exports") => 95,
        Some(_) | None => 100,
    }
}

fn plan_summary(actions: &[PlannedAction]) -> PlanSummary {
    PlanSummary {
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
    }
}

fn already_satisfied_action_ids(
    actions: &[PlannedAction],
    diagnostics: &[TopologyDiagnostic],
) -> Vec<String> {
    let mut ids = Vec::new();
    for action in actions {
        if !matches!(
            action.operation,
            Operation::Create
                | Operation::Grow
                | Operation::Shrink
                | Operation::AddDevice
                | Operation::ReplaceDevice
                | Operation::Attach
                | Operation::Detach
                | Operation::Assemble
                | Operation::Import
                | Operation::Activate
                | Operation::Deactivate
                | Operation::Close
                | Operation::Login
                | Operation::Logout
                | Operation::Open
                | Operation::Mount
                | Operation::Unmount
                | Operation::Remount
                | Operation::Export
                | Operation::Unexport
                | Operation::Start
                | Operation::Stop
                | Operation::Destroy
                | Operation::RemoveDevice
                | Operation::RemoveKey
                | Operation::RemoveToken
                | Operation::SetProperty
        ) {
            continue;
        }
        let action_diagnostics = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.action_id == action.id);
        let mut already_satisfied = false;
        let mut has_warning = false;
        for diagnostic in action_diagnostics {
            already_satisfied |= matches!(
                diagnostic.kind,
                TopologyDiagnosticKind::SizeAlreadySatisfied
                    | TopologyDiagnosticKind::FilesystemFormatAlreadySatisfied
                    | TopologyDiagnosticKind::DiskCreateAlreadySatisfied
                    | TopologyDiagnosticKind::BtrfsSubvolumeCreateAlreadySatisfied
                    | TopologyDiagnosticKind::BtrfsQgroupCreateAlreadySatisfied
                    | TopologyDiagnosticKind::BackingFileCreateAlreadySatisfied
                    | TopologyDiagnosticKind::PartitionCreateAlreadySatisfied
                    | TopologyDiagnosticKind::LvmPvCreateAlreadySatisfied
                    | TopologyDiagnosticKind::LvmVolumeCreateAlreadySatisfied
                    | TopologyDiagnosticKind::LvmVgCreateAlreadySatisfied
                    | TopologyDiagnosticKind::BtrfsSubvolumeDestroyAlreadySatisfied
                    | TopologyDiagnosticKind::BtrfsQgroupDestroyAlreadySatisfied
                    | TopologyDiagnosticKind::BcacheDetachAlreadySatisfied
                    | TopologyDiagnosticKind::DmMapDestroyAlreadySatisfied
                    | TopologyDiagnosticKind::IscsiLoginAlreadySatisfied
                    | TopologyDiagnosticKind::IscsiLogoutAlreadySatisfied
                    | TopologyDiagnosticKind::LunAttachAlreadySatisfied
                    | TopologyDiagnosticKind::LunDetachAlreadySatisfied
                    | TopologyDiagnosticKind::NvmeNamespaceAttachAlreadySatisfied
                    | TopologyDiagnosticKind::NvmeNamespaceDetachAlreadySatisfied
                    | TopologyDiagnosticKind::LvmActivateAlreadySatisfied
                    | TopologyDiagnosticKind::LvmDeactivateAlreadySatisfied
                    | TopologyDiagnosticKind::LvmVgExportAlreadySatisfied
                    | TopologyDiagnosticKind::LvmVgImportAlreadySatisfied
                    | TopologyDiagnosticKind::LvmCacheDetachAlreadySatisfied
                    | TopologyDiagnosticKind::LuksCloseAlreadySatisfied
                    | TopologyDiagnosticKind::LuksOpenAlreadySatisfied
                    | TopologyDiagnosticKind::LuksKeyslotRemoveAlreadySatisfied
                    | TopologyDiagnosticKind::LuksTokenRemoveAlreadySatisfied
                    | TopologyDiagnosticKind::MultipathDestroyAlreadySatisfied
                    | TopologyDiagnosticKind::MultipathPathAddAlreadySatisfied
                    | TopologyDiagnosticKind::MultipathPathRemoveAlreadySatisfied
                    | TopologyDiagnosticKind::SwapDeactivateAlreadySatisfied
                    | TopologyDiagnosticKind::SwapDestroyAlreadySatisfied
                    | TopologyDiagnosticKind::LoopCreateAlreadySatisfied
                    | TopologyDiagnosticKind::LoopDetachAlreadySatisfied
                    | TopologyDiagnosticKind::MdCreateAlreadySatisfied
                    | TopologyDiagnosticKind::MdAssembleAlreadySatisfied
                    | TopologyDiagnosticKind::MdStopAlreadySatisfied
                    | TopologyDiagnosticKind::MdMemberAddAlreadySatisfied
                    | TopologyDiagnosticKind::MdMemberRemoveAlreadySatisfied
                    | TopologyDiagnosticKind::MdMemberReplaceAlreadySatisfied
                    | TopologyDiagnosticKind::MountAlreadySatisfied
                    | TopologyDiagnosticKind::MountOptionsAlreadySatisfied
                    | TopologyDiagnosticKind::UnmountAlreadySatisfied
                    | TopologyDiagnosticKind::NfsExportAlreadySatisfied
                    | TopologyDiagnosticKind::NfsUnexportAlreadySatisfied
                    | TopologyDiagnosticKind::PropertyAlreadySatisfied
                    | TopologyDiagnosticKind::SnapshotDestroyAlreadySatisfied
                    | TopologyDiagnosticKind::VdoDestroyAlreadySatisfied
                    | TopologyDiagnosticKind::VdoStartAlreadySatisfied
                    | TopologyDiagnosticKind::VdoStopAlreadySatisfied
                    | TopologyDiagnosticKind::ZfsObjectCreateAlreadySatisfied
                    | TopologyDiagnosticKind::ZfsObjectDestroyAlreadySatisfied
                    | TopologyDiagnosticKind::ZfsPoolCreateAlreadySatisfied
                    | TopologyDiagnosticKind::ZfsPoolImportAlreadySatisfied
            );
            has_warning |= diagnostic.level == TopologyDiagnosticLevel::Warning;
        }
        if already_satisfied && !has_warning {
            ids.push(action.id.clone());
        }
    }
    ids
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
        if let Some(diagnostic) = bcache_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = snapshot_clone_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = snapshot_destroy_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = snapshot_rename_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = snapshot_rollback_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = btrfs_subvolume_destroy_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = btrfs_qgroup_destroy_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = vdo_destroy_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = zfs_object_destroy_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = dm_map_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = multipath_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = multipath_path_remove_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = loop_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = md_stop_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = md_member_remove_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = nvme_namespace_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = lun_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = nfs_unexport_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = swap_inactive_diagnostic(action, &query) {
            return vec![diagnostic];
        }
        if let Some(diagnostic) = unmount_absent_diagnostic(action, &query) {
            return vec![diagnostic];
        }
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
    diagnostics.extend(disk_create_diagnostic(action, node, &query));
    diagnostics.extend(iscsi_login_diagnostic(action, &matches, &query));
    diagnostics.extend(iscsi_logout_diagnostic(action, &matches, &query));
    diagnostics.extend(nvme_namespace_present_diagnostic(action, node, &query));
    diagnostics.extend(lun_present_diagnostic(action, node, &query));
    diagnostics.extend(lvm_volume_create_diagnostic(action, node, &query));
    diagnostics.extend(lvm_activate_diagnostic(action, node, &query));
    diagnostics.extend(lvm_deactivate_diagnostic(action, node, &query));
    diagnostics.extend(lvm_pv_create_diagnostic(action, &matches, &query));
    diagnostics.extend(lvm_vg_create_diagnostic(action, node, &query));
    diagnostics.extend(lvm_vg_export_diagnostic(action, node, &query));
    diagnostics.extend(lvm_vg_import_diagnostic(action, node, &query));
    diagnostics.extend(lvm_cache_detach_diagnostic(action, node, &query));
    diagnostics.extend(luks_close_diagnostic(action, node, &query));
    diagnostics.extend(luks_open_diagnostic(action, node, &query));
    diagnostics.extend(luks_keyslot_remove_diagnostic(action, node, &query));
    diagnostics.extend(luks_token_remove_diagnostic(action, node, &query));
    diagnostics.extend(bcache_present_diagnostic(action, node, &query));
    diagnostics.extend(snapshot_clone_present_diagnostic(action, node, &query));
    diagnostics.extend(snapshot_destroy_present_diagnostic(action, node, &query));
    diagnostics.extend(snapshot_rename_present_diagnostic(action, node, &query));
    diagnostics.extend(snapshot_rollback_present_diagnostic(action, node, &query));
    diagnostics.extend(btrfs_subvolume_create_present_diagnostic(
        action, node, &query,
    ));
    diagnostics.extend(btrfs_subvolume_destroy_present_diagnostic(
        action, node, &query,
    ));
    diagnostics.extend(btrfs_qgroup_destroy_present_diagnostic(
        action, node, &query,
    ));
    diagnostics.extend(btrfs_qgroup_create_present_diagnostic(action, node, &query));
    diagnostics.extend(dm_map_present_diagnostic(action, node, &query));
    diagnostics.extend(multipath_present_diagnostic(action, node, &query));
    diagnostics.extend(multipath_path_add_diagnostic(action, node, graph, &query));
    diagnostics.extend(multipath_path_remove_diagnostic(
        action, node, graph, &query,
    ));
    diagnostics.extend(loop_present_diagnostic(action, node, &query));
    diagnostics.extend(partition_create_diagnostic(action, node, &query));
    diagnostics.extend(backing_file_create_diagnostic(action, node, &query));
    diagnostics.extend(md_create_diagnostic(action, node, &query));
    diagnostics.extend(md_assemble_diagnostic(action, node, &query));
    diagnostics.extend(md_stop_diagnostic(action, node, &query));
    diagnostics.extend(md_member_add_diagnostic(action, node, graph, &query));
    diagnostics.extend(md_member_remove_diagnostic(action, node, graph, &query));
    diagnostics.extend(md_member_replace_diagnostic(action, node, graph, &query));
    diagnostics.extend(mount_diagnostic(action, node, &query));
    diagnostics.extend(mount_options_diagnostic(action, node, &query));
    diagnostics.extend(unmount_diagnostic(action, node, &query));
    diagnostics.extend(nfs_export_diagnostic(action, node, &query));
    diagnostics.extend(nfs_unexport_diagnostic(action, node, &query));
    diagnostics.extend(swap_active_diagnostic(action, node, &query));
    diagnostics.extend(swap_format_present_diagnostic(action, node, &query));
    diagnostics.extend(luks_format_present_diagnostic(action, node, &query));
    diagnostics.extend(property_diagnostic(action, node, &query));
    diagnostics.extend(vdo_create_present_diagnostic(action, node, &query));
    diagnostics.extend(vdo_destroy_present_diagnostic(action, node, &query));
    diagnostics.extend(vdo_grow_diagnostic(action, node, &query));
    diagnostics.extend(vdo_start_diagnostic(action, node, &query));
    diagnostics.extend(vdo_stop_diagnostic(action, node, &query));
    diagnostics.extend(zfs_object_create_present_diagnostic(action, node, &query));
    diagnostics.extend(zfs_object_destroy_present_diagnostic(action, node, &query));
    diagnostics.extend(zfs_pool_create_diagnostic(action, node, &query));
    diagnostics.extend(zfs_pool_import_diagnostic(action, node, &query));
    diagnostics
}

fn topology_query(action: &PlannedAction) -> Option<String> {
    if matches!(
        action.context.collection.as_deref(),
        Some("luns" | "nvmeNamespaces")
    ) {
        return action
            .context
            .device
            .clone()
            .or_else(|| action.context.target.clone())
            .or_else(|| action.context.name.clone());
    }

    if action.context.collection.as_deref() == Some("btrfsQgroups") {
        return action
            .context
            .name
            .clone()
            .or_else(|| action.context.target.clone())
            .or_else(|| action.context.device.clone());
    }

    if action.context.collection.as_deref() == Some("snapshots")
        && matches!(
            action.operation,
            Operation::Clone | Operation::Destroy | Operation::Rename | Operation::Rollback
        )
    {
        return action
            .context
            .snapshot_path
            .clone()
            .or_else(|| action.context.name.clone())
            .or_else(|| action.context.target.clone())
            .or_else(|| action.context.device.clone());
    }

    if matches!(
        action.context.collection.as_deref(),
        Some("luksKeyslots" | "luksTokens")
    ) {
        return action
            .context
            .device
            .clone()
            .or_else(|| action.context.target.clone())
            .or_else(|| action.context.name.clone());
    }

    if action.context.collection.as_deref() == Some("luks.devices")
        && action.operation == Operation::Format
    {
        return action
            .context
            .device
            .clone()
            .or_else(|| action.context.target.clone())
            .or_else(|| action.context.name.clone());
    }

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
    let desired = size_diagnostic_desired_size(action)?;
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

fn size_diagnostic_desired_size(action: &PlannedAction) -> Option<&str> {
    action.context.desired_size.as_deref().or_else(|| {
        if action.operation == Operation::Grow
            && action.context.collection.as_deref() == Some("partitions")
        {
            action.context.end.as_deref()
        } else {
            None
        }
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
        if action.operation == Operation::Format
            && action.context.collection.as_deref() == Some("filesystems")
        {
            return Some(TopologyDiagnostic {
                action_id: action.id.clone(),
                level: TopologyDiagnosticLevel::Info,
                kind: TopologyDiagnosticKind::FilesystemFormatAlreadySatisfied,
                query: query.to_string(),
                message: format!("filesystem {query} already reports type {current}"),
                current: Some(current_node_summary(node)),
            });
        }
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

fn disk_create_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Create
        || action.context.collection.as_deref() != Some("disks")
    {
        return None;
    }

    let desired_table = action.context.partition_type.as_deref().unwrap_or("gpt");

    if node.kind != NodeKind::PhysicalDisk {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::DiskCreateRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not a physical disk; partition table initialization remains actionable only after target review",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    let Some(current_table) = property_value_from_node(node, "partition.table") else {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::DiskCreateRequired,
            query: query.to_string(),
            message: format!(
                "disk {query} current partition table is unknown; desired {desired_table} remains actionable after disk identity review"
            ),
            current: Some(current_node_summary(node)),
        });
    };

    let (level, kind, message) = if current_table.eq_ignore_ascii_case(desired_table) {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::DiskCreateAlreadySatisfied,
            format!("disk {query} already has partition table {current_table}"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::DiskCreateRequired,
            format!(
                "disk {query} has partition table {current_table}, desired {desired_table}; mklabel remains destructive and requires review"
            ),
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

fn partition_create_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Create
        || action.context.collection.as_deref() != Some("partitions")
    {
        return None;
    }

    if node.kind != NodeKind::Partition {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::PartitionCreateRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not a partition; parted mkpart remains actionable only after target review",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    let desired = action.context.desired_size.as_deref();
    let desired_bytes = desired.and_then(parse_size_bytes);
    let (level, kind, message) = match (desired, desired_bytes, node.size_bytes) {
        (Some(desired), Some(desired_bytes), Some(current_bytes))
            if current_bytes == desired_bytes =>
        {
            (
                TopologyDiagnosticLevel::Info,
                TopologyDiagnosticKind::PartitionCreateAlreadySatisfied,
                format!(
                    "partition {query} already exists with desired size {desired} ({current_bytes} bytes)"
                ),
            )
        }
        (None, _, _) => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::PartitionCreateAlreadySatisfied,
            format!("partition {query} already exists"),
        ),
        (Some(desired), Some(_), Some(current_bytes)) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::PartitionCreateRequired,
            format!(
                "partition {query} already exists with size {current_bytes} bytes, not desired size {desired}; use grow or recreate only after data-preservation review"
            ),
        ),
        (Some(desired), None, _) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::PartitionCreateRequired,
            format!(
                "partition {query} already exists, but desired size {desired} could not be compared"
            ),
        ),
        (Some(desired), Some(_), None) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::PartitionCreateRequired,
            format!(
                "partition {query} already exists, but current size is unknown; desired size is {desired}"
            ),
        ),
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

fn mount_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Mount {
        return None;
    }
    let desired_source = action.context.device.as_deref()?;
    let current_source = property_value_from_node(node, "mount.source")?;
    let (level, kind, message) = if current_source == desired_source {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::MountAlreadySatisfied,
            format!("mountpoint {query} already uses source {desired_source}"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::MountSourceConflict,
            format!("mountpoint {query} uses source {current_source}, desired {desired_source}"),
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

fn lvm_activate_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Activate || !is_lvm_activation_collection(action) {
        return None;
    }
    let active = lvm_node_is_active(node)?;
    let (level, kind, message) = if active {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LvmActivateAlreadySatisfied,
            format!("LVM object {query} is already active"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LvmActivateRequired,
            format!("LVM object {query} is known but not active"),
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

fn lvm_deactivate_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Deactivate || !is_lvm_activation_collection(action) {
        return None;
    }
    let active = lvm_node_is_active(node)?;
    let (level, kind, message) = if active {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LvmDeactivateRequired,
            format!("LVM object {query} is still active"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LvmDeactivateAlreadySatisfied,
            format!("LVM object {query} is already inactive"),
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

fn lvm_volume_create_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Create
        || !matches!(
            action.context.collection.as_deref(),
            Some("volumes" | "thinPools")
        )
    {
        return None;
    }

    let (expected_kind, label, command) = match action.context.collection.as_deref() {
        Some("volumes") => (NodeKind::LvmLogicalVolume, "logical volume", "lvcreate"),
        Some("thinPools") => (
            NodeKind::LvmThinPool,
            "thin pool",
            "lvcreate --type thin-pool",
        ),
        _ => return None,
    };

    if node.kind != expected_kind {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::LvmVolumeCreateRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not the expected LVM {label}; {command} would create a new object",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    let desired = action.context.desired_size.as_deref();
    let desired_bytes = desired.and_then(parse_size_bytes);
    let (level, kind, message) = match (desired, desired_bytes, node.size_bytes) {
        (Some(desired), Some(desired_bytes), Some(current_bytes))
            if current_bytes == desired_bytes =>
        {
            (
                TopologyDiagnosticLevel::Info,
                TopologyDiagnosticKind::LvmVolumeCreateAlreadySatisfied,
                format!(
                    "LVM {label} {query} already exists with desired size {desired} ({current_bytes} bytes)"
                ),
            )
        }
        (None, _, _) => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LvmVolumeCreateAlreadySatisfied,
            format!("LVM {label} {query} already exists"),
        ),
        (Some(desired), Some(_), Some(current_bytes)) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LvmVolumeCreateRequired,
            format!(
                "LVM {label} {query} already exists with size {current_bytes} bytes, not desired size {desired}; use grow or shrink lifecycle instead of create when preserving data"
            ),
        ),
        (Some(desired), None, _) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LvmVolumeCreateRequired,
            format!(
                "LVM {label} {query} already exists, but desired size {desired} could not be compared"
            ),
        ),
        (Some(desired), Some(_), None) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LvmVolumeCreateRequired,
            format!(
                "LVM {label} {query} already exists, but current size is unknown; desired size is {desired}"
            ),
        ),
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

fn lvm_pv_create_diagnostic(
    action: &PlannedAction,
    matches: &[&Node],
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Create
        || action.context.collection.as_deref() != Some("physicalVolumes")
    {
        return None;
    }

    if let Some(pv_node) = matches
        .iter()
        .copied()
        .find(|node| node.kind == NodeKind::LvmPhysicalVolume)
    {
        let review_reasons = lvm_pv_review_reasons(pv_node);
        let (level, kind, message) = if review_reasons.is_empty() {
            (
                TopologyDiagnosticLevel::Info,
                TopologyDiagnosticKind::LvmPvCreateAlreadySatisfied,
                format!("physical volume {query} already has LVM PV metadata"),
            )
        } else {
            (
                TopologyDiagnosticLevel::Warning,
                TopologyDiagnosticKind::LvmPvCreateRequired,
                format!(
                    "physical volume {query} already exists, but metadata needs review: {}",
                    review_reasons.join(", ")
                ),
            )
        };

        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level,
            kind,
            query: query.to_string(),
            message,
            current: Some(current_node_summary(pv_node)),
        });
    }

    let node = matches[0];
    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::LvmPvCreateRequired,
        query: query.to_string(),
        message: format!(
            "matched current {} node {}, but it is not an LVM physical volume; pvcreate would write PV metadata",
            node.kind, node.name
        ),
        current: Some(current_node_summary(node)),
    })
}

fn lvm_vg_import_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Import
        || action.context.collection.as_deref() != Some("volumeGroups")
        || node.kind != NodeKind::LvmVolumeGroup
    {
        return None;
    }
    let exported = lvm_vg_is_exported(node);
    let (level, kind, message) = if exported {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LvmVgImportRequired,
            format!("LVM volume group {query} is visible but still exported"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LvmVgImportAlreadySatisfied,
            format!("LVM volume group {query} is already imported"),
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

fn lvm_vg_create_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Create
        || action.context.collection.as_deref() != Some("volumeGroups")
    {
        return None;
    }

    if node.kind != NodeKind::LvmVolumeGroup {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::LvmVgCreateRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not an LVM volume group; vgcreate would write VG metadata",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    let review_reasons = lvm_vg_review_reasons(node);
    let (level, kind, message) = if review_reasons.is_empty() {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LvmVgCreateAlreadySatisfied,
            format!("volume group {query} already exists"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LvmVgCreateRequired,
            format!(
                "volume group {query} already exists, but metadata needs review before treating create as satisfied: {}",
                review_reasons.join(", ")
            ),
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

fn lvm_vg_export_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Export
        || action.context.collection.as_deref() != Some("volumeGroups")
        || node.kind != NodeKind::LvmVolumeGroup
    {
        return None;
    }
    let exported = lvm_vg_is_exported(node);
    let (level, kind, message) = if exported {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LvmVgExportAlreadySatisfied,
            format!("LVM volume group {query} is already exported"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LvmVgExportRequired,
            format!("LVM volume group {query} is visible but not exported"),
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

fn lvm_cache_detach_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("lvmCaches")
        || !matches!(
            action.operation,
            Operation::Destroy | Operation::RemoveDevice
        )
        || !matches!(node.kind, NodeKind::LvmCache | NodeKind::LvmLogicalVolume)
    {
        return None;
    }

    let attached = lvm_cache_is_attached(node);
    let (level, kind, message) = if attached {
        let details = lvm_cache_detach_details(node);
        let message = if details.is_empty() {
            format!("LVM cache remains attached to origin {query}")
        } else {
            format!(
                "LVM cache remains attached to origin {query} with {}",
                details.join(", ")
            )
        };
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LvmCacheDetachRequired,
            message,
        )
    } else {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LvmCacheDetachAlreadySatisfied,
            format!("LVM cache is already detached from origin {query}"),
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

fn lvm_cache_is_attached(node: &Node) -> bool {
    node.kind == NodeKind::LvmCache
        || [
            "lvm.pool",
            "lvm.cache-mode",
            "lvm.cache-policy",
            "lvm.cache-dirty-blocks",
            "lvm.cache-total-blocks",
            "lvm.writecache-writeback-blocks",
            "lvm.writecache-total-blocks",
        ]
        .iter()
        .any(|property| property_value_from_node(node, property).is_some())
}

fn lvm_cache_detach_details(node: &Node) -> Vec<String> {
    [
        ("lvm.pool", "cache pool"),
        ("lvm.cache-mode", "cache mode"),
        ("lvm.cache-policy", "cache policy"),
        ("lvm.cache-dirty-blocks", "dirty blocks"),
        ("lvm.cache-total-blocks", "cache blocks"),
        ("lvm.cache-used-blocks", "used cache blocks"),
        ("lvm.writecache-writeback-blocks", "writeback blocks"),
        ("lvm.writecache-total-blocks", "writecache blocks"),
        ("lvm.writecache-free-blocks", "free writecache blocks"),
        ("lvm.data-percent", "data percent"),
        ("lvm.metadata-percent", "metadata percent"),
        ("lvm.health", "health"),
        ("lvm.attr", "LV attributes"),
    ]
    .into_iter()
    .filter_map(|(property, label)| {
        property_value_from_node(node, property).map(|value| format!("{label} {value}"))
    })
    .collect()
}

fn luks_open_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Open
        || action.context.collection.as_deref() != Some("luks.devices")
    {
        return None;
    }
    let active = luks_node_is_active(node)?;
    let (level, kind, message) = if active {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LuksOpenAlreadySatisfied,
            format!("LUKS mapper {query} is already active"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LuksOpenRequired,
            format!("LUKS mapper {query} is known but not active"),
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

fn luks_format_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Format
        || action.context.collection.as_deref() != Some("luks.devices")
    {
        return None;
    }

    let message = if node.kind == NodeKind::LuksContainer {
        let details = luks_format_present_details(node);
        if details.is_empty() {
            format!(
                "LUKS format target {query} already contains a LUKS container; format remains destructive and requires review"
            )
        } else {
            format!(
                "LUKS format target {query} already contains a LUKS container with {}; format remains destructive and requires review",
                details.join(", ")
            )
        }
    } else {
        format!(
            "LUKS format target {query} matched current {} node {}; format remains destructive and requires review",
            node.kind, node.name
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::LuksFormatTargetPresent,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn luks_format_present_details(node: &Node) -> Vec<String> {
    [
        ("cryptsetup.luks-version", "version"),
        ("cryptsetup.uuid", "UUID"),
        ("cryptsetup.luks-uuid", "UUID"),
        ("cryptsetup.label", "label"),
        ("cryptsetup.luks-label", "label"),
        ("cryptsetup.luks-keyslot-count", "keyslots"),
        ("cryptsetup.luks-token-count", "tokens"),
        ("cryptsetup.active", "active"),
    ]
    .into_iter()
    .filter_map(|(property, label)| {
        property_value_from_node(node, property).map(|value| format!("{label} {value}"))
    })
    .collect()
}

fn luks_close_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Close
        || action.context.collection.as_deref() != Some("luks.devices")
    {
        return None;
    }
    let active = luks_node_is_active(node)?;
    let (level, kind, message) = if active {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LuksCloseRequired,
            format!("LUKS mapper {query} is known and still active"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LuksCloseAlreadySatisfied,
            format!("LUKS mapper {query} is already inactive"),
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

fn luks_keyslot_remove_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("luksKeyslots")
        || !matches!(action.operation, Operation::Destroy | Operation::RemoveKey)
        || node.kind != NodeKind::LuksContainer
    {
        return None;
    }
    let key_slot = action.context.key_slot.as_deref()?;
    let present = property_list_contains(
        property_value_from_node(node, "cryptsetup.luks-keyslots"),
        key_slot,
    );

    let (level, kind, message) = if present {
        let details = luks_keyslot_remove_details(node, key_slot);
        let message = if details.is_empty() {
            format!("LUKS keyslot {key_slot} is still present on {query}")
        } else {
            format!(
                "LUKS keyslot {key_slot} is still present on {query} with {}",
                details.join(", ")
            )
        };
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LuksKeyslotRemoveRequired,
            message,
        )
    } else {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LuksKeyslotRemoveAlreadySatisfied,
            format!("LUKS keyslot {key_slot} is already absent from {query}"),
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

fn luks_token_remove_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("luksTokens")
        || !matches!(
            action.operation,
            Operation::Destroy | Operation::RemoveToken
        )
        || node.kind != NodeKind::LuksContainer
    {
        return None;
    }
    let token_id = action.context.token_id.as_deref()?;
    let present = property_list_contains(
        property_value_from_node(node, "cryptsetup.luks-tokens"),
        token_id,
    );

    let (level, kind, message) = if present {
        let details = luks_token_remove_details(node, token_id);
        let message = if details.is_empty() {
            format!("LUKS token {token_id} is still present on {query}")
        } else {
            format!(
                "LUKS token {token_id} is still present on {query} with {}",
                details.join(", ")
            )
        };
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LuksTokenRemoveRequired,
            message,
        )
    } else {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LuksTokenRemoveAlreadySatisfied,
            format!("LUKS token {token_id} is already absent from {query}"),
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

fn luks_keyslot_remove_details(node: &Node, key_slot: &str) -> Vec<String> {
    let prefix = format!("cryptsetup.luks-keyslot-{key_slot}-");
    [
        ("type", "type"),
        ("priority", "priority"),
        ("cipher", "cipher"),
        ("cipher-key", "cipher key"),
        ("pbkdf", "PBKDF"),
        ("time-cost", "time cost"),
        ("memory", "memory"),
        ("threads", "threads"),
    ]
    .into_iter()
    .filter_map(|(suffix, label)| {
        property_value_from_node(node, &format!("{prefix}{suffix}"))
            .map(|value| format!("{label} {value}"))
    })
    .collect()
}

fn luks_token_remove_details(node: &Node, token_id: &str) -> Vec<String> {
    let prefix = format!("cryptsetup.luks-token-{token_id}-");
    [("type", "type"), ("keyslot", "keyslot")]
        .into_iter()
        .filter_map(|(suffix, label)| {
            property_value_from_node(node, &format!("{prefix}{suffix}"))
                .map(|value| format!("{label} {value}"))
        })
        .collect()
}

fn property_list_contains(values: Option<&str>, needle: &str) -> bool {
    values
        .into_iter()
        .flat_map(|values| values.split(','))
        .map(str::trim)
        .any(|value| value == needle)
}

fn bcache_absent_diagnostic(action: &PlannedAction, query: &str) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("caches")
        || action.operation != Operation::RemoveDevice
        || !is_concrete_bcache_target(query)
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::BcacheDetachAlreadySatisfied,
        query: query.to_string(),
        message: format!("bcache device {query} is already absent from current topology"),
        current: None,
    })
}

fn bcache_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("caches")
        || action.operation != Operation::RemoveDevice
    {
        return None;
    }

    let details = bcache_detach_details(node);
    let message = if details.is_empty() {
        format!("bcache device {query} is still present")
    } else {
        format!(
            "bcache device {query} is still present with {}",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::BcacheDetachRequired,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn bcache_detach_details(node: &Node) -> Vec<String> {
    [
        ("bcache.dirty-data", "dirty data"),
        ("bcache.cache-mode", "cache mode"),
        ("bcache.set-uuid", "cache set"),
    ]
    .into_iter()
    .filter_map(|(property, label)| {
        property_value_from_node(node, property).map(|value| format!("{label} {value}"))
    })
    .collect()
}

fn is_concrete_bcache_target(query: &str) -> bool {
    query.starts_with("/dev/bcache") || query.starts_with("block:/dev/bcache")
}

fn snapshot_clone_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("snapshots")
        || action.operation != Operation::Clone
        || !is_concrete_snapshot_target(query)
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::SnapshotCloneSourceMissing,
        query: query.to_string(),
        message: format!("snapshot clone source {query} is missing from current topology"),
        current: None,
    })
}

fn snapshot_clone_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("snapshots")
        || action.operation != Operation::Clone
    {
        return None;
    }

    let (label, details) = match node.kind {
        NodeKind::ZfsSnapshot => ("ZFS snapshot", zfs_snapshot_destroy_details(node)),
        NodeKind::BtrfsSnapshot => ("Btrfs snapshot", btrfs_subvolume_destroy_details(node)),
        _ => return None,
    };
    let destination = action.context.target.as_deref().unwrap_or("<clone-target>");
    let message = if details.is_empty() {
        format!("{label} clone source {query} is available for clone to {destination}")
    } else {
        format!(
            "{label} clone source {query} is available with {}; clone target {destination}",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::SnapshotCloneSourceAvailable,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn snapshot_destroy_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("snapshots")
        || action.operation != Operation::Destroy
        || !is_concrete_snapshot_target(query)
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::SnapshotDestroyAlreadySatisfied,
        query: query.to_string(),
        message: format!("snapshot {query} is already absent from current topology"),
        current: None,
    })
}

fn snapshot_destroy_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("snapshots")
        || action.operation != Operation::Destroy
    {
        return None;
    }

    let (label, details) = match node.kind {
        NodeKind::ZfsSnapshot => ("ZFS snapshot", zfs_snapshot_destroy_details(node)),
        NodeKind::BtrfsSnapshot => ("Btrfs snapshot", btrfs_subvolume_destroy_details(node)),
        _ => return None,
    };
    let message = if details.is_empty() {
        format!("{label} {query} is still present")
    } else {
        format!(
            "{label} {query} is still present with {}",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::SnapshotDestroyRequired,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn snapshot_rename_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("snapshots")
        || action.operation != Operation::Rename
        || !is_concrete_snapshot_target(query)
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::SnapshotRenameSourceMissing,
        query: query.to_string(),
        message: format!("snapshot rename source {query} is missing from current topology"),
        current: None,
    })
}

fn snapshot_rename_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("snapshots")
        || action.operation != Operation::Rename
    {
        return None;
    }

    let (label, details) = match node.kind {
        NodeKind::ZfsSnapshot => ("ZFS snapshot", zfs_snapshot_destroy_details(node)),
        NodeKind::BtrfsSnapshot => ("Btrfs snapshot", btrfs_subvolume_destroy_details(node)),
        _ => return None,
    };
    let destination = action
        .context
        .rename_to
        .as_deref()
        .unwrap_or("<rename-target>");
    let message = if details.is_empty() {
        format!(
            "{label} rename source {query} is present; rename to {destination} remains offline-required"
        )
    } else {
        format!(
            "{label} rename source {query} is present with {}; rename to {destination} remains offline-required",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::SnapshotRenameRequired,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn snapshot_rollback_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("snapshots")
        || action.operation != Operation::Rollback
        || !is_concrete_zfs_snapshot_target(query)
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::SnapshotRollbackPointMissing,
        query: query.to_string(),
        message: format!("ZFS rollback snapshot {query} is missing from current topology"),
        current: None,
    })
}

fn snapshot_rollback_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("snapshots")
        || action.operation != Operation::Rollback
        || node.kind != NodeKind::ZfsSnapshot
    {
        return None;
    }

    let mut details = zfs_snapshot_destroy_details(node);
    if action.context.recursive_rollback == Some(true) {
        details.push("recursive rollback requested".to_string());
    }
    let message = if details.is_empty() {
        format!("ZFS rollback snapshot {query} is available; rollback remains potential data loss")
    } else {
        format!(
            "ZFS rollback snapshot {query} is available with {}; rollback remains potential data loss",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::SnapshotRollbackPointAvailable,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn is_concrete_snapshot_target(query: &str) -> bool {
    is_concrete_zfs_snapshot_target(query) || is_concrete_btrfs_snapshot_target(query)
}

fn is_concrete_zfs_snapshot_target(query: &str) -> bool {
    query.contains('@') && query.contains('/') && !query.starts_with('/')
}

fn is_concrete_btrfs_snapshot_target(query: &str) -> bool {
    query.starts_with('/')
}

fn btrfs_subvolume_destroy_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("btrfsSubvolumes")
        || action.operation != Operation::Destroy
        || !is_concrete_btrfs_subvolume_target(query)
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::BtrfsSubvolumeDestroyAlreadySatisfied,
        query: query.to_string(),
        message: format!("Btrfs subvolume {query} is already absent from current topology"),
        current: None,
    })
}

fn btrfs_subvolume_create_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("btrfsSubvolumes")
        || action.operation != Operation::Create
    {
        return None;
    }

    if node.kind != NodeKind::BtrfsSubvolume {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::BtrfsSubvolumeCreateRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not a Btrfs subvolume; btrfs subvolume create remains actionable",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    let details = btrfs_subvolume_destroy_details(node);
    let message = if details.is_empty() {
        format!("Btrfs subvolume {query} already exists")
    } else {
        format!(
            "Btrfs subvolume {query} already exists with {}",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::BtrfsSubvolumeCreateAlreadySatisfied,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn btrfs_subvolume_destroy_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("btrfsSubvolumes")
        || action.operation != Operation::Destroy
        || node.kind != NodeKind::BtrfsSubvolume
    {
        return None;
    }

    let details = btrfs_subvolume_destroy_details(node);
    let message = if details.is_empty() {
        format!("Btrfs subvolume {query} is still present")
    } else {
        format!(
            "Btrfs subvolume {query} is still present with {}",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::BtrfsSubvolumeDestroyRequired,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn btrfs_subvolume_destroy_details(node: &Node) -> Vec<String> {
    let mut details = [
        ("btrfs.id", "subvolume id"),
        ("btrfs.generation", "generation"),
        ("btrfs.created-generation", "created generation"),
        ("btrfs.parent-id", "parent id"),
        ("btrfs.top-level", "top level"),
        ("btrfs.received-uuid", "received UUID"),
        ("btrfs.parent-uuid", "parent UUID"),
    ]
    .into_iter()
    .filter_map(|(property, label)| {
        property_value_from_node(node, property).map(|value| format!("{label} {value}"))
    })
    .collect::<Vec<_>>();

    if let Some(uuid) = node.identity.uuid.as_deref() {
        details.push(format!("UUID {uuid}"));
    }

    details
}

fn is_concrete_btrfs_subvolume_target(query: &str) -> bool {
    query.starts_with('/')
}

fn btrfs_qgroup_destroy_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("btrfsQgroups")
        || action.operation != Operation::Destroy
        || !is_concrete_btrfs_qgroup_target(query)
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::BtrfsQgroupDestroyAlreadySatisfied,
        query: query.to_string(),
        message: format!("Btrfs qgroup {query} is already absent from current topology"),
        current: None,
    })
}

fn btrfs_qgroup_create_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("btrfsQgroups")
        || action.operation != Operation::Create
    {
        return None;
    }

    if node.kind != NodeKind::BtrfsQgroup {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::BtrfsQgroupCreateRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not a Btrfs qgroup; btrfs qgroup create remains actionable",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    let details = btrfs_qgroup_destroy_details(node);
    let message = if details.is_empty() {
        format!("Btrfs qgroup {query} already exists")
    } else {
        format!(
            "Btrfs qgroup {query} already exists with {}",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::BtrfsQgroupCreateAlreadySatisfied,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn btrfs_qgroup_destroy_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("btrfsQgroups")
        || action.operation != Operation::Destroy
        || node.kind != NodeKind::BtrfsQgroup
    {
        return None;
    }

    let details = btrfs_qgroup_destroy_details(node);
    let message = if details.is_empty() {
        format!("Btrfs qgroup {query} is still present")
    } else {
        format!(
            "Btrfs qgroup {query} is still present with {}",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::BtrfsQgroupDestroyRequired,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn btrfs_qgroup_destroy_details(node: &Node) -> Vec<String> {
    let mut details = [
        ("btrfs.qgroup-id", "qgroup id"),
        ("btrfs.max-referenced", "max referenced"),
        ("btrfs.max-exclusive", "max exclusive"),
        ("btrfs.qgroup-parents", "parents"),
        ("btrfs.qgroup-children", "children"),
    ]
    .into_iter()
    .filter_map(|(property, label)| {
        property_value_from_node(node, property).map(|value| format!("{label} {value}"))
    })
    .collect::<Vec<_>>();

    if let Some(used_bytes) = node.usage.as_ref().and_then(|usage| usage.used_bytes) {
        details.push(format!("referenced {used_bytes} bytes"));
    }
    if let Some(allocated_bytes) = node.usage.as_ref().and_then(|usage| usage.allocated_bytes) {
        details.push(format!("exclusive {allocated_bytes} bytes"));
    }

    details
}

fn is_concrete_btrfs_qgroup_target(query: &str) -> bool {
    let Some((level, id)) = query.split_once('/') else {
        return false;
    };

    !level.is_empty()
        && !id.is_empty()
        && level.chars().all(|character| character.is_ascii_digit())
        && id.chars().all(|character| character.is_ascii_digit())
}

fn dm_map_absent_diagnostic(action: &PlannedAction, query: &str) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("dmMaps")
        || action.operation != Operation::Destroy
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::DmMapDestroyAlreadySatisfied,
        query: query.to_string(),
        message: format!("device-mapper map {query} is already absent from current topology"),
        current: None,
    })
}

fn dm_map_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("dmMaps")
        || action.operation != Operation::Destroy
    {
        return None;
    }

    let message = match property_value_from_node(node, "dm.open-count") {
        Some(open_count) => {
            format!("device-mapper map {query} is still present with open count {open_count}")
        }
        None => format!("device-mapper map {query} is still present"),
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::DmMapDestroyRequired,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn multipath_absent_diagnostic(action: &PlannedAction, query: &str) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("multipathMaps")
        || action.operation != Operation::Destroy
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::MultipathDestroyAlreadySatisfied,
        query: query.to_string(),
        message: format!("multipath map {query} is already absent from current topology"),
        current: None,
    })
}

fn multipath_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("multipathMaps")
        || action.operation != Operation::Destroy
    {
        return None;
    }

    let message = multipath_identity_detail(node)
        .map(|detail| format!("multipath map {query} is still present with {detail}"))
        .unwrap_or_else(|| format!("multipath map {query} is still present"));

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::MultipathDestroyRequired,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn multipath_identity_detail(node: &Node) -> Option<String> {
    if let Some(wwid) = property_value_from_node(node, "multipath.wwid") {
        return Some(format!("WWID {wwid}"));
    }
    property_value_from_node(node, "multipath.dm").map(|dm_name| format!("dm map {dm_name}"))
}

fn multipath_path_remove_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::RemoveDevice
        || action.context.collection.as_deref() != Some("multipathMaps")
    {
        return None;
    }

    let device = action.context.device.as_deref().unwrap_or("<unknown-path>");
    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::MultipathPathRemoveAlreadySatisfied,
        query: query.to_string(),
        message: format!("multipath map {query} is absent, so path {device} is already removed"),
        current: None,
    })
}

fn multipath_path_add_diagnostic(
    action: &PlannedAction,
    node: &Node,
    graph: &StorageGraph,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::AddDevice
        || action.context.collection.as_deref() != Some("multipathMaps")
    {
        return None;
    }
    let device = action.context.device.as_deref()?;

    if node.kind != NodeKind::MultipathDevice {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MultipathPathAddRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not a multipath map; path add remains actionable only after target review",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    if multipath_map_has_path(graph, node, device) {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Info,
            kind: TopologyDiagnosticKind::MultipathPathAddAlreadySatisfied,
            query: query.to_string(),
            message: format!("multipath map {query} already includes path {device}"),
            current: Some(current_node_summary(node)),
        });
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::MultipathPathAddRequired,
        query: query.to_string(),
        message: format!("multipath map {query} does not currently include path {device}"),
        current: Some(current_node_summary(node)),
    })
}

fn multipath_path_remove_diagnostic(
    action: &PlannedAction,
    node: &Node,
    graph: &StorageGraph,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::RemoveDevice
        || action.context.collection.as_deref() != Some("multipathMaps")
    {
        return None;
    }
    let device = action.context.device.as_deref()?;

    if node.kind != NodeKind::MultipathDevice {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MultipathPathRemoveRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not a multipath map; path removal remains actionable only after target review",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    if multipath_map_has_path(graph, node, device) {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MultipathPathRemoveRequired,
            query: query.to_string(),
            message: format!("multipath map {query} still includes path {device}"),
            current: Some(current_node_summary(node)),
        });
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::MultipathPathRemoveAlreadySatisfied,
        query: query.to_string(),
        message: format!("multipath map {query} no longer includes path {device}"),
        current: Some(current_node_summary(node)),
    })
}

fn multipath_map_has_path(graph: &StorageGraph, map: &Node, device: &str) -> bool {
    graph.edges.iter().any(|edge| {
        edge.relationship == Relationship::Backs
            && edge.to == map.id
            && graph
                .nodes
                .iter()
                .find(|node| node.id == edge.from)
                .is_some_and(|path| path.matches(device))
    })
}

fn loop_absent_diagnostic(action: &PlannedAction, query: &str) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("loopDevices") {
        return None;
    }

    let (level, kind, message) = match action.operation {
        Operation::Create => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LoopCreateRequired,
            format!("loop device {query} is not currently mapped"),
        ),
        Operation::Destroy | Operation::Detach => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LoopDetachAlreadySatisfied,
            format!("loop device {query} is already absent from current topology"),
        ),
        _ => return None,
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level,
        kind,
        query: query.to_string(),
        message,
        current: None,
    })
}

fn loop_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("loopDevices") {
        return None;
    }

    match action.operation {
        Operation::Create => loop_create_diagnostic(action, node, query),
        Operation::Destroy | Operation::Detach => Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::LoopDetachRequired,
            query: query.to_string(),
            message: format!("loop device {query} is still mapped"),
            current: Some(current_node_summary(node)),
        }),
        _ => None,
    }
}

fn loop_create_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    let desired_backing = action.context.device.as_deref();
    let current_backing = property_value_from_node(node, "loop.back-file");
    let (level, kind, message) = match (desired_backing, current_backing) {
        (Some(desired), Some(current)) if desired == current => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LoopCreateAlreadySatisfied,
            format!("loop device {query} already maps backing file {desired}"),
        ),
        (Some(desired), Some(current)) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LoopCreateConflict,
            format!("loop device {query} maps backing file {current}, desired {desired}"),
        ),
        (Some(desired), None) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LoopCreateConflict,
            format!("loop device {query} is present but does not report backing file {desired}"),
        ),
        (None, Some(current)) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LoopCreateConflict,
            format!("loop device {query} is already mapped to backing file {current}"),
        ),
        (None, None) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LoopCreateConflict,
            format!("loop device {query} is already present with unknown backing file"),
        ),
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

fn backing_file_create_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Create
        || action.context.collection.as_deref() != Some("backingFiles")
        || node.kind != NodeKind::BackingFile
    {
        return None;
    }

    let desired = action.context.desired_size.as_deref();
    let desired_bytes = desired.and_then(parse_size_bytes);
    let (level, kind, message) = match (desired, desired_bytes, node.size_bytes) {
        (Some(desired), Some(desired_bytes), Some(current_bytes))
            if current_bytes == desired_bytes =>
        {
            (
                TopologyDiagnosticLevel::Info,
                TopologyDiagnosticKind::BackingFileCreateAlreadySatisfied,
                format!(
                    "backing file {query} already exists with desired size {desired} ({current_bytes} bytes)"
                ),
            )
        }
        (Some(desired), Some(_), Some(current_bytes)) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::BackingFileCreateRequired,
            format!(
                "backing file {query} already exists with size {current_bytes} bytes, not desired size {desired}; create would refuse to overwrite it"
            ),
        ),
        (Some(desired), None, _) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::BackingFileCreateRequired,
            format!(
                "backing file {query} already exists, but desired size {desired} could not be compared"
            ),
        ),
        (None, _, Some(current_bytes)) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::BackingFileCreateRequired,
            format!(
                "backing file {query} already exists with size {current_bytes} bytes, but create has no desired size to compare"
            ),
        ),
        (None, _, None) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::BackingFileCreateRequired,
            format!(
                "backing file {query} already exists, but create has no desired size to compare"
            ),
        ),
        (Some(desired), Some(_), None) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::BackingFileCreateRequired,
            format!(
                "backing file {query} already exists, but current size is unknown; desired size is {desired}"
            ),
        ),
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

fn md_create_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Create
        || action.context.collection.as_deref() != Some("mdRaids")
    {
        return None;
    }

    if node.kind != NodeKind::MdRaid {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MdCreateRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not an MD RAID array; mdadm --create remains actionable",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    let Some(status) = md_array_status(node) else {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MdCreateRequired,
            query: query.to_string(),
            message: format!(
                "MD RAID array {query} already exists, but current state is unknown; rescan before treating create as satisfied"
            ),
            current: Some(current_node_summary(node)),
        });
    };

    if !status.cleanly_active {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MdCreateRequired,
            query: query.to_string(),
            message: format!(
                "MD RAID array {query} already exists, but state needs review before treating create as satisfied: state={}, degradedDevices={}, failedDevices={}",
                status.state, status.degraded_devices, status.failed_devices
            ),
            current: Some(current_node_summary(node)),
        });
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::MdCreateAlreadySatisfied,
        query: query.to_string(),
        message: format!(
            "MD RAID array {query} already exists and is cleanly active: state={}, degradedDevices=0, failedDevices=0",
            status.state
        ),
        current: Some(current_node_summary(node)),
    })
}

fn md_assemble_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Assemble
        || action.context.collection.as_deref() != Some("mdRaids")
    {
        return None;
    }
    let status = md_array_status(node)?;
    let (level, kind, message) = if status.cleanly_active {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::MdAssembleAlreadySatisfied,
            format!("MD RAID array {query} is already cleanly assembled"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::MdAssembleRequired,
            format!(
                "MD RAID array {query} is not cleanly assembled: state={}, degradedDevices={}, failedDevices={}",
                status.state, status.degraded_devices, status.failed_devices
            ),
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

fn md_stop_absent_diagnostic(action: &PlannedAction, query: &str) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Stop
        || action.context.collection.as_deref() != Some("mdRaids")
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::MdStopAlreadySatisfied,
        query: query.to_string(),
        message: format!("MD RAID array {query} is already absent from current topology"),
        current: None,
    })
}

fn md_stop_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Stop
        || action.context.collection.as_deref() != Some("mdRaids")
    {
        return None;
    }

    if node.kind != NodeKind::MdRaid {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MdStopRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not an MD RAID array; mdadm --stop remains actionable only after target review",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    let Some(status) = md_array_status(node) else {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MdStopRequired,
            query: query.to_string(),
            message: format!(
                "MD RAID array {query} is present, but current state is unknown; rescan before treating stop as satisfied"
            ),
            current: Some(current_node_summary(node)),
        });
    };

    if status.active {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MdStopRequired,
            query: query.to_string(),
            message: format!(
                "MD RAID array {query} is still active: state={}, degradedDevices={}, failedDevices={}",
                status.state, status.degraded_devices, status.failed_devices
            ),
            current: Some(current_node_summary(node)),
        });
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::MdStopAlreadySatisfied,
        query: query.to_string(),
        message: format!(
            "MD RAID array {query} is already inactive: state={}",
            status.state
        ),
        current: Some(current_node_summary(node)),
    })
}

fn md_member_remove_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::RemoveDevice
        || action.context.collection.as_deref() != Some("mdRaids")
    {
        return None;
    }

    let device = action
        .context
        .device
        .as_deref()
        .unwrap_or("<unknown-member>");
    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::MdMemberRemoveAlreadySatisfied,
        query: query.to_string(),
        message: format!("MD RAID array {query} is absent, so member {device} is already removed"),
        current: None,
    })
}

fn md_member_add_diagnostic(
    action: &PlannedAction,
    node: &Node,
    graph: &StorageGraph,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::AddDevice
        || action.context.collection.as_deref() != Some("mdRaids")
    {
        return None;
    }
    let device = action.context.device.as_deref()?;

    if node.kind != NodeKind::MdRaid {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MdMemberAddRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not an MD RAID array; mdadm --add remains actionable only after target review",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    if md_array_has_member(graph, node, device) {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Info,
            kind: TopologyDiagnosticKind::MdMemberAddAlreadySatisfied,
            query: query.to_string(),
            message: format!("MD RAID array {query} already includes member {device}"),
            current: Some(current_node_summary(node)),
        });
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::MdMemberAddRequired,
        query: query.to_string(),
        message: format!("MD RAID array {query} does not currently include member {device}"),
        current: Some(current_node_summary(node)),
    })
}

fn md_member_remove_diagnostic(
    action: &PlannedAction,
    node: &Node,
    graph: &StorageGraph,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::RemoveDevice
        || action.context.collection.as_deref() != Some("mdRaids")
    {
        return None;
    }
    let device = action.context.device.as_deref()?;

    if node.kind != NodeKind::MdRaid {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MdMemberRemoveRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not an MD RAID array; mdadm --remove remains actionable only after target review",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    if md_array_has_member(graph, node, device) {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MdMemberRemoveRequired,
            query: query.to_string(),
            message: format!("MD RAID array {query} still includes member {device}"),
            current: Some(current_node_summary(node)),
        });
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::MdMemberRemoveAlreadySatisfied,
        query: query.to_string(),
        message: format!("MD RAID array {query} no longer includes member {device}"),
        current: Some(current_node_summary(node)),
    })
}

fn md_member_replace_diagnostic(
    action: &PlannedAction,
    node: &Node,
    graph: &StorageGraph,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::ReplaceDevice
        || action.context.collection.as_deref() != Some("mdRaids")
    {
        return None;
    }
    let old_device = action.context.device.as_deref()?;
    let new_device = action.context.replacement.as_deref()?;

    if node.kind != NodeKind::MdRaid {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MdMemberReplaceRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not an MD RAID array; mdadm replacement remains actionable only after target review",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    let old_present = md_array_has_member(graph, node, old_device);
    let new_present = md_array_has_member(graph, node, new_device);
    match (old_present, new_present) {
        (false, true) => Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Info,
            kind: TopologyDiagnosticKind::MdMemberReplaceAlreadySatisfied,
            query: query.to_string(),
            message: format!(
                "MD RAID array {query} already replaced member {old_device} with {new_device}"
            ),
            current: Some(current_node_summary(node)),
        }),
        (true, true) => Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MdMemberReplaceRequired,
            query: query.to_string(),
            message: format!(
                "MD RAID array {query} still includes old member {old_device} and already includes replacement {new_device}; review before removing the old member"
            ),
            current: Some(current_node_summary(node)),
        }),
        (true, false) => Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MdMemberReplaceRequired,
            query: query.to_string(),
            message: format!(
                "MD RAID array {query} still includes old member {old_device} and does not include replacement {new_device}"
            ),
            current: Some(current_node_summary(node)),
        }),
        (false, false) => Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::MdMemberReplaceRequired,
            query: query.to_string(),
            message: format!(
                "MD RAID array {query} no longer includes old member {old_device}, but replacement {new_device} is not attached"
            ),
            current: Some(current_node_summary(node)),
        }),
    }
}

fn md_array_has_member(graph: &StorageGraph, array: &Node, device: &str) -> bool {
    graph.edges.iter().any(|edge| {
        edge.relationship == Relationship::MemberOf
            && edge.to == array.id
            && graph
                .nodes
                .iter()
                .find(|node| node.id == edge.from)
                .is_some_and(|member| member.matches(device))
    })
}

struct MdArrayStatus<'a> {
    state: &'a str,
    degraded_devices: u64,
    failed_devices: u64,
    active: bool,
    cleanly_active: bool,
}

fn md_array_status(node: &Node) -> Option<MdArrayStatus<'_>> {
    let state = property_value_from_node(node, "md.state")?;
    let degraded_devices = md_device_count_property(node, "md.degraded-devices")?;
    let failed_devices = md_device_count_property(node, "md.failed-devices")?;
    let state_indicates_active = md_state_indicates_active(state);
    Some(MdArrayStatus {
        state,
        degraded_devices,
        failed_devices,
        active: state_indicates_active,
        cleanly_active: state_indicates_active && degraded_devices == 0 && failed_devices == 0,
    })
}

fn mount_options_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Remount {
        return None;
    }
    let desired_options = parse_mount_option_map(action.context.options.as_deref()?);
    if desired_options.is_empty() {
        return None;
    }
    let current_options = current_mount_option_map(node);
    if current_options.is_empty() {
        return None;
    }

    let missing_or_different = option_differences(&desired_options, &current_options);

    let (level, kind, message) = if missing_or_different.is_empty() {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::MountOptionsAlreadySatisfied,
            format!("mountpoint {query} already includes desired remount options"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::MountOptionsDiffer,
            format!(
                "mountpoint {query} is missing or differs on desired options: {}",
                missing_or_different.join(",")
            ),
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

fn unmount_absent_diagnostic(action: &PlannedAction, query: &str) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Unmount || !is_mount_collection(action) {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::UnmountAlreadySatisfied,
        query: query.to_string(),
        message: format!("mountpoint {query} is already absent from current topology"),
        current: None,
    })
}

fn unmount_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Unmount || !is_mount_collection(action) {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::UnmountRequired,
        query: query.to_string(),
        message: format!("mountpoint {query} is currently mounted"),
        current: Some(current_node_summary(node)),
    })
}

fn nfs_export_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Export
        || action.context.collection.as_deref() != Some("exports")
    {
        return None;
    }
    let desired_client = action.context.client.as_deref()?;
    let desired_options = parse_mount_option_map(action.context.options.as_deref()?);
    if desired_options.is_empty() {
        return None;
    }
    let current_client = property_value_from_node(node, "nfs.export-client")?;
    let current_options = current_nfs_export_option_map(node);
    if current_options.is_empty() {
        return None;
    }

    let mut differences = Vec::new();
    if current_client != desired_client {
        differences.push(format!("client={desired_client}"));
    }
    differences.extend(option_differences(&desired_options, &current_options));

    let (level, kind, message) = if differences.is_empty() {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::NfsExportAlreadySatisfied,
            format!("NFS export {query} already grants {desired_client} desired options"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::NfsExportDiffers,
            format!(
                "NFS export {query} differs from desired client/options: {}",
                differences.join(",")
            ),
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

fn nfs_unexport_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Unexport
        || action.context.collection.as_deref() != Some("exports")
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::NfsUnexportAlreadySatisfied,
        query: query.to_string(),
        message: format!("NFS export {query} is already absent from current topology"),
        current: None,
    })
}

fn nfs_unexport_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Unexport
        || action.context.collection.as_deref() != Some("exports")
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::NfsUnexportRequired,
        query: query.to_string(),
        message: format!("NFS export {query} is currently published"),
        current: Some(current_node_summary(node)),
    })
}

fn swap_inactive_diagnostic(action: &PlannedAction, query: &str) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("swaps")
        || !matches!(action.operation, Operation::Deactivate | Operation::Destroy)
    {
        return None;
    }

    let (kind, message) = match action.operation {
        Operation::Deactivate => (
            TopologyDiagnosticKind::SwapDeactivateAlreadySatisfied,
            format!("swap target {query} is already inactive or absent from current topology"),
        ),
        Operation::Destroy => (
            TopologyDiagnosticKind::SwapDestroyAlreadySatisfied,
            format!("swap target {query} is already inactive or absent from current topology"),
        ),
        _ => unreachable!("operation checked above"),
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind,
        query: query.to_string(),
        message,
        current: None,
    })
}

fn swap_active_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("swaps")
        || !matches!(action.operation, Operation::Deactivate | Operation::Destroy)
    {
        return None;
    }

    let details = swap_active_details(node);
    let detail_suffix = if details.is_empty() {
        String::new()
    } else {
        format!(" with {}", details.join(", "))
    };
    let (kind, message) = match action.operation {
        Operation::Deactivate => (
            TopologyDiagnosticKind::SwapDeactivateRequired,
            format!("swap target {query} is active{detail_suffix}"),
        ),
        Operation::Destroy => (
            TopologyDiagnosticKind::SwapDestroyRequired,
            format!("swap target {query} is active{detail_suffix}"),
        ),
        _ => unreachable!("operation checked above"),
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn swap_format_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Format
        || action.context.collection.as_deref() != Some("swaps")
    {
        return None;
    }

    let message = if node.kind == NodeKind::Swap {
        let details = swap_active_details(node);
        if details.is_empty() {
            format!(
                "swap format target {query} already has swap metadata; mkswap remains destructive and requires review"
            )
        } else {
            format!(
                "swap format target {query} already has swap metadata with {}; mkswap remains destructive and requires review",
                details.join(", ")
            )
        }
    } else {
        format!(
            "swap format target {query} matched current {} node {}; mkswap remains destructive and requires review",
            node.kind, node.name
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::SwapFormatTargetPresent,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn swap_active_details(node: &Node) -> Vec<String> {
    let mut details = Vec::new();
    if let Some(size) = node.size_bytes {
        details.push(format!("size {size} bytes"));
    }
    if let Some(used) = node.usage.as_ref().and_then(|usage| usage.used_bytes) {
        details.push(format!("used {used} bytes"));
    }
    if let Some(priority) = property_value_from_node(node, "swap.priority") {
        details.push(format!("priority {priority}"));
    }
    if let Some(swap_type) = property_value_from_node(node, "swap.type") {
        details.push(format!("type {swap_type}"));
    }
    details
}

fn vdo_destroy_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Destroy
        || action.context.collection.as_deref() != Some("vdoVolumes")
    {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::VdoDestroyAlreadySatisfied,
        query: query.to_string(),
        message: format!("VDO volume {query} is already absent from current topology"),
        current: None,
    })
}

fn vdo_destroy_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Destroy
        || action.context.collection.as_deref() != Some("vdoVolumes")
    {
        return None;
    }

    let details = vdo_destroy_details(node);
    let message = if details.is_empty() {
        format!("VDO volume {query} is still present")
    } else {
        format!(
            "VDO volume {query} is still present with {}",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::VdoDestroyRequired,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn vdo_create_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Create
        || action.context.collection.as_deref() != Some("vdoVolumes")
    {
        return None;
    }

    let message = if node.kind == NodeKind::VdoVolume {
        let details = vdo_destroy_details(node);
        if details.is_empty() {
            format!(
                "VDO create target {query} already has VDO metadata; create remains destructive and requires review"
            )
        } else {
            format!(
                "VDO create target {query} already has VDO metadata with {}; create remains destructive and requires review",
                details.join(", ")
            )
        }
    } else {
        format!(
            "VDO create target {query} matched current {} node {}; create remains destructive and requires review",
            node.kind, node.name
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::VdoCreateTargetPresent,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn vdo_destroy_details(node: &Node) -> Vec<String> {
    [
        ("vdo.operating-mode", "operating mode"),
        ("vdo.logical-size", "logical size"),
        ("vdo.physical-size", "physical size"),
        ("vdo.storage-device", "backing device"),
        ("vdo.backing-device", "backing device"),
        ("vdo.write-policy", "write policy"),
        ("lvm.vdo-operating-mode", "operating mode"),
        ("lvm.vdo-logical-size", "logical size"),
        ("lvm.vdo-physical-size", "physical size"),
        ("lvm.vdo-used-size", "used"),
        ("lvm.vdo-used", "used"),
        ("lvm.vdo-saving-percent", "saving"),
        ("lvm.vdo-write-policy", "write policy"),
    ]
    .into_iter()
    .filter_map(|(property, label)| {
        property_value_from_node(node, property).map(|value| format!("{label} {value}"))
    })
    .collect()
}

fn vdo_grow_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Grow
        || action.context.collection.as_deref() != Some("vdoVolumes")
        || node.size_bytes.is_some()
    {
        return None;
    }

    let desired = action.context.desired_size.as_deref()?;
    let desired_bytes = parse_size_bytes(desired);
    let current = vdo_logical_size(node);

    let (level, kind, message) = match (desired_bytes, current) {
        (Some(desired_bytes), Some((current, current_bytes))) if current_bytes >= desired_bytes => {
            (
                TopologyDiagnosticLevel::Info,
                TopologyDiagnosticKind::SizeAlreadySatisfied,
                format!(
                    "VDO volume {query} logical size {current} already satisfies desired size {desired}"
                ),
            )
        }
        (Some(_), Some((current, _))) => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::SizeBelowDesired,
            format!("VDO volume {query} logical size {current} is below desired size {desired}"),
        ),
        (None, _) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::VdoGrowRequired,
            format!(
                "VDO volume {query} desired size {desired} could not be parsed; grow remains actionable"
            ),
        ),
        (Some(_), None) => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::VdoGrowRequired,
            format!(
                "VDO volume {query} current logical size is unknown; grow to {desired} remains actionable"
            ),
        ),
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

fn vdo_logical_size(node: &Node) -> Option<(&str, u64)> {
    ["vdo.logical-size", "lvm.vdo-logical-size"]
        .into_iter()
        .find_map(|property| {
            let value = property_value_from_node(node, property)?;
            parse_size_bytes(value).map(|bytes| (value, bytes))
        })
}

fn vdo_start_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Start
        || action.context.collection.as_deref() != Some("vdoVolumes")
    {
        return None;
    }
    let operating_mode = vdo_operating_mode(node)?;
    let normal = operating_mode.eq_ignore_ascii_case("normal");
    let (level, kind, message) = if normal {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::VdoStartAlreadySatisfied,
            format!("VDO volume {query} is already running in normal mode"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::VdoStartRequired,
            format!("VDO volume {query} operating mode is {operating_mode}, desired normal"),
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

fn vdo_stop_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Stop
        || action.context.collection.as_deref() != Some("vdoVolumes")
    {
        return None;
    }
    let operating_mode = vdo_operating_mode(node)?;
    let stopped = vdo_operating_mode_is_stopped(operating_mode);
    let (level, kind, message) = if stopped {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::VdoStopAlreadySatisfied,
            format!("VDO volume {query} is already stopped"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::VdoStopRequired,
            format!("VDO volume {query} operating mode is {operating_mode}, desired stopped"),
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

fn vdo_operating_mode(node: &Node) -> Option<&str> {
    property_value_from_node(node, "vdo.operating-mode")
        .or_else(|| property_value_from_node(node, "lvm.vdo-operating-mode"))
}

fn zfs_object_destroy_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    let object_label = zfs_object_destroy_label(action)?;
    if action.operation != Operation::Destroy || !is_concrete_zfs_object_target(query) {
        return None;
    }

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::ZfsObjectDestroyAlreadySatisfied,
        query: query.to_string(),
        message: format!("ZFS {object_label} {query} is already absent from current topology"),
        current: None,
    })
}

fn zfs_object_create_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    let object_label = zfs_object_destroy_label(action)?;
    let expected_kind = zfs_object_expected_kind(action)?;
    if action.operation != Operation::Create {
        return None;
    }

    if node.kind != expected_kind {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::ZfsObjectCreateRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not a ZFS {object_label}; zfs create remains actionable",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    if expected_kind == NodeKind::Zvol {
        if let Some(desired) = action.context.desired_size.as_deref() {
            match (parse_size_bytes(desired), node.size_bytes) {
                (Some(desired_bytes), Some(current_bytes)) if current_bytes >= desired_bytes => {}
                (Some(_), Some(current_bytes)) => {
                    return Some(TopologyDiagnostic {
                        action_id: action.id.clone(),
                        level: TopologyDiagnosticLevel::Warning,
                        kind: TopologyDiagnosticKind::ZfsObjectCreateRequired,
                        query: query.to_string(),
                        message: format!(
                            "ZFS zvol {query} already exists with size {current_bytes} bytes, not desired size {desired}; use grow or shrink lifecycle instead of create when preserving data"
                        ),
                        current: Some(current_node_summary(node)),
                    });
                }
                (Some(_), None) => {
                    return Some(TopologyDiagnostic {
                        action_id: action.id.clone(),
                        level: TopologyDiagnosticLevel::Warning,
                        kind: TopologyDiagnosticKind::ZfsObjectCreateRequired,
                        query: query.to_string(),
                        message: format!(
                            "ZFS zvol {query} already exists, but current size is unknown; use rescan or grow/shrink lifecycle instead of create when preserving data"
                        ),
                        current: Some(current_node_summary(node)),
                    });
                }
                (None, _) => {
                    return Some(TopologyDiagnostic {
                        action_id: action.id.clone(),
                        level: TopologyDiagnosticLevel::Warning,
                        kind: TopologyDiagnosticKind::ZfsObjectCreateRequired,
                        query: query.to_string(),
                        message: format!(
                            "ZFS zvol {query} already exists, but desired size {desired} could not be parsed; review before treating create as satisfied"
                        ),
                        current: Some(current_node_summary(node)),
                    });
                }
            }
        }
    }

    let details = zfs_object_destroy_details(node);
    let message = if details.is_empty() {
        format!("ZFS {object_label} {query} already exists")
    } else {
        format!(
            "ZFS {object_label} {query} already exists with {}",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::ZfsObjectCreateAlreadySatisfied,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn zfs_object_destroy_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    let object_label = zfs_object_destroy_label(action)?;
    if action.operation != Operation::Destroy {
        return None;
    }

    match (action.context.collection.as_deref(), node.kind) {
        (Some("datasets"), NodeKind::ZfsDataset) | (Some("zvols"), NodeKind::Zvol) => {}
        _ => return None,
    }

    let details = zfs_object_destroy_details(node);
    let message = if details.is_empty() {
        format!("ZFS {object_label} {query} is still present")
    } else {
        format!(
            "ZFS {object_label} {query} is still present with {}",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Warning,
        kind: TopologyDiagnosticKind::ZfsObjectDestroyRequired,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn zfs_object_destroy_label(action: &PlannedAction) -> Option<&'static str> {
    match action.context.collection.as_deref() {
        Some("datasets") => Some("dataset"),
        Some("zvols") => Some("zvol"),
        _ => None,
    }
}

fn zfs_object_expected_kind(action: &PlannedAction) -> Option<NodeKind> {
    match action.context.collection.as_deref() {
        Some("datasets") => Some(NodeKind::ZfsDataset),
        Some("zvols") => Some(NodeKind::Zvol),
        _ => None,
    }
}

fn is_concrete_zfs_object_target(query: &str) -> bool {
    query.contains('/') && !query.starts_with('/')
}

fn zfs_object_destroy_details(node: &Node) -> Vec<String> {
    [
        ("zfs.type", "type"),
        ("zfs.mountpoint", "mountpoint"),
        ("zfs.origin", "origin"),
        ("zfs.used", "used"),
        ("zfs.available", "available"),
        ("zfs.referenced", "referenced"),
        ("zfs.quota", "quota"),
        ("zfs.reservation", "reservation"),
        ("zfs.volsize", "volsize"),
        ("zfs.encryption", "encryption"),
        ("zfs.keystatus", "key status"),
        ("zfs.compression", "compression"),
    ]
    .into_iter()
    .filter_map(|(property, label)| {
        property_value_from_node(node, property).map(|value| format!("{label} {value}"))
    })
    .collect()
}

fn zfs_snapshot_destroy_details(node: &Node) -> Vec<String> {
    let mut details = zfs_object_destroy_details(node);
    if let Some(userrefs) = property_value_from_node(node, "zfs.userrefs") {
        details.push(format!("user references {userrefs}"));
    }
    details
}

fn zfs_pool_create_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Create
        || action.context.collection.as_deref() != Some("pools")
    {
        return None;
    }

    if node.kind != NodeKind::ZfsPool {
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::ZfsPoolCreateRequired,
            query: query.to_string(),
            message: format!(
                "matched current {} node {}, but it is not a ZFS pool; zpool create remains actionable",
                node.kind, node.name
            ),
            current: Some(current_node_summary(node)),
        });
    }

    let state = property_value_from_node(node, "zfs.state");
    let health = property_value_from_node(node, "zfs.health");
    let online =
        state.is_some_and(zfs_status_is_online) && health.is_some_and(zfs_status_is_online);
    if !online {
        let state = state.unwrap_or("unknown");
        let health = health.unwrap_or("unknown");
        return Some(TopologyDiagnostic {
            action_id: action.id.clone(),
            level: TopologyDiagnosticLevel::Warning,
            kind: TopologyDiagnosticKind::ZfsPoolCreateRequired,
            query: query.to_string(),
            message: format!(
                "ZFS pool {query} already exists, but pool state needs review before treating create as satisfied: state={state}, health={health}"
            ),
            current: Some(current_node_summary(node)),
        });
    }

    let details = zfs_pool_details(node);
    let message = if details.is_empty() {
        format!("ZFS pool {query} already exists and is online")
    } else {
        format!(
            "ZFS pool {query} already exists and is online with {}",
            details.join(", ")
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level: TopologyDiagnosticLevel::Info,
        kind: TopologyDiagnosticKind::ZfsPoolCreateAlreadySatisfied,
        query: query.to_string(),
        message,
        current: Some(current_node_summary(node)),
    })
}

fn zfs_pool_details(node: &Node) -> Vec<String> {
    [
        ("zfs.state", "state"),
        ("zfs.health", "health"),
        ("zfs.pool-capacity", "capacity"),
        ("zfs.pool-dedupratio", "dedup ratio"),
        ("zfs.pool-fragmentation", "fragmentation"),
        ("zfs.pool-altroot", "altroot"),
    ]
    .into_iter()
    .filter_map(|(property, label)| {
        property_value_from_node(node, property).map(|value| format!("{label} {value}"))
    })
    .collect()
}

fn zfs_pool_import_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Import
        || action.context.collection.as_deref() != Some("pools")
    {
        return None;
    }
    let state = property_value_from_node(node, "zfs.state")?;
    let health = property_value_from_node(node, "zfs.health")?;
    let online = zfs_status_is_online(state) && zfs_status_is_online(health);
    let (level, kind, message) = if online {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::ZfsPoolImportAlreadySatisfied,
            format!("ZFS pool {query} is already imported and online"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::ZfsPoolImportRequired,
            format!("ZFS pool {query} is visible but not online: state={state}, health={health}"),
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

fn nvme_namespace_absent_diagnostic(
    action: &PlannedAction,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("nvmeNamespaces") {
        return None;
    }

    let (level, kind, message) = match action.operation {
        Operation::Attach => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::NvmeNamespaceAttachRequired,
            format!("NVMe namespace path {query} is not currently visible on this host"),
        ),
        Operation::Detach => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::NvmeNamespaceDetachAlreadySatisfied,
            format!("NVMe namespace path {query} is already absent from current topology"),
        ),
        _ => return None,
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level,
        kind,
        query: query.to_string(),
        message,
        current: None,
    })
}

fn nvme_namespace_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("nvmeNamespaces") {
        return None;
    }

    let (level, kind, message) = match action.operation {
        Operation::Attach => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::NvmeNamespaceAttachAlreadySatisfied,
            format!("NVMe namespace path {query} is already visible on this host"),
        ),
        Operation::Detach => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::NvmeNamespaceDetachRequired,
            format!("NVMe namespace path {query} is still visible on this host"),
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

fn lun_absent_diagnostic(action: &PlannedAction, query: &str) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("luns") {
        return None;
    }

    let (level, kind, message) = match action.operation {
        Operation::Attach => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LunAttachRequired,
            format!("LUN path {query} is not currently visible on this host"),
        ),
        Operation::Detach => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LunDetachAlreadySatisfied,
            format!("LUN path {query} is already absent from current topology"),
        ),
        _ => return None,
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level,
        kind,
        query: query.to_string(),
        message,
        current: None,
    })
}

fn lun_present_diagnostic(
    action: &PlannedAction,
    node: &Node,
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.context.collection.as_deref() != Some("luns") {
        return None;
    }

    let (level, kind, message) = match action.operation {
        Operation::Attach => (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::LunAttachAlreadySatisfied,
            format!("LUN path {query} is already visible on this host"),
        ),
        Operation::Detach => (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::LunDetachRequired,
            format!("LUN path {query} is still visible on this host"),
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

fn iscsi_login_diagnostic(
    action: &PlannedAction,
    matches: &[&Node],
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Login
        || action.context.collection.as_deref() != Some("iscsiSessions")
    {
        return None;
    }

    let logged_in = matches
        .iter()
        .copied()
        .find(|node| iscsi_node_is_logged_in(node));
    let current = logged_in
        .or_else(|| matches.first().copied())
        .map(current_node_summary);
    let (level, kind, message) = if logged_in.is_some() {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::IscsiLoginAlreadySatisfied,
            format!("iSCSI target {query} already has a logged-in session"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::IscsiLoginRequired,
            format!("iSCSI target {query} is known but no logged-in session was matched"),
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level,
        kind,
        query: query.to_string(),
        message,
        current,
    })
}

fn iscsi_logout_diagnostic(
    action: &PlannedAction,
    matches: &[&Node],
    query: &str,
) -> Option<TopologyDiagnostic> {
    if action.operation != Operation::Logout
        || action.context.collection.as_deref() != Some("iscsiSessions")
    {
        return None;
    }

    let logged_in = matches
        .iter()
        .copied()
        .find(|node| iscsi_node_is_logged_in(node));
    let current = logged_in
        .or_else(|| matches.first().copied())
        .map(current_node_summary);
    let (level, kind, message) = if logged_in.is_some() {
        (
            TopologyDiagnosticLevel::Warning,
            TopologyDiagnosticKind::IscsiLogoutRequired,
            format!("iSCSI target {query} still has a logged-in session"),
        )
    } else {
        (
            TopologyDiagnosticLevel::Info,
            TopologyDiagnosticKind::IscsiLogoutAlreadySatisfied,
            format!("iSCSI target {query} has no logged-in session"),
        )
    };

    Some(TopologyDiagnostic {
        action_id: action.id.clone(),
        level,
        kind,
        query: query.to_string(),
        message,
        current,
    })
}

fn iscsi_node_is_logged_in(node: &Node) -> bool {
    property_value_from_node(node, "iscsi.connection-state")
        .or_else(|| property_value_from_node(node, "iscsi.session-state"))
        .is_some_and(is_logged_in_iscsi_state)
}

fn is_logged_in_iscsi_state(value: &str) -> bool {
    let normalized = value
        .trim()
        .chars()
        .filter(|character| {
            !character.is_ascii_whitespace() && *character != '-' && *character != '_'
        })
        .collect::<String>()
        .to_ascii_lowercase();
    normalized == "loggedin"
}

fn luks_node_is_active(node: &Node) -> Option<bool> {
    property_value_from_node(node, "cryptsetup.active").map(|value| value == "true")
}

fn is_lvm_activation_collection(action: &PlannedAction) -> bool {
    matches!(
        action.context.collection.as_deref(),
        Some("volumes" | "thinPools" | "lvmSnapshots")
    )
}

fn is_mount_collection(action: &PlannedAction) -> bool {
    matches!(
        action.context.collection.as_deref(),
        Some("filesystems" | "nfs.mounts")
    )
}

fn lvm_node_is_active(node: &Node) -> Option<bool> {
    property_value_from_node(node, "lvm.active").map(|value| {
        value
            .split_whitespace()
            .next()
            .is_some_and(|state| state.eq_ignore_ascii_case("active"))
    })
}

fn lvm_vg_is_exported(node: &Node) -> bool {
    property_value_from_node(node, "lvm.vg-exported").is_some_and(|value| {
        let normalized = value.trim();
        normalized.eq_ignore_ascii_case("exported")
            || normalized.eq_ignore_ascii_case("true")
            || normalized.eq_ignore_ascii_case("yes")
            || normalized == "1"
    })
}

fn lvm_pv_review_reasons(node: &Node) -> Vec<String> {
    [
        ("lvm.pv-missing", "PV is marked missing"),
        ("lvm.pv-duplicate", "PV is marked duplicate"),
    ]
    .iter()
    .filter_map(|(property, reason)| {
        property_value_from_node(node, property)
            .filter(|value| lvm_truthy_or_named_state(value, reason))
            .map(|value| format!("{reason} ({property}={value})"))
    })
    .collect()
}

fn lvm_vg_review_reasons(node: &Node) -> Vec<String> {
    let mut reasons = Vec::new();

    if let Some(value) = property_value_from_node(node, "lvm.vg-exported")
        .filter(|value| lvm_truthy_or_named_state(value, "VG is marked exported"))
    {
        reasons.push(format!("VG is marked exported (lvm.vg-exported={value})"));
    }

    if let Some(value) = property_value_from_node(node, "lvm.vg-partial")
        .filter(|value| lvm_truthy_or_named_state(value, "VG is marked partial"))
    {
        reasons.push(format!("VG is marked partial (lvm.vg-partial={value})"));
    }

    if let Some(count) = property_value_from_node(node, "lvm.missing-pv-count")
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|count| *count > 0)
    {
        reasons.push(format!("VG reports {count} missing physical volume(s)"));
    }

    reasons
}

fn lvm_truthy_or_named_state(value: &str, reason: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    let named_state = reason
        .trim_start_matches("PV is marked ")
        .trim_start_matches("VG is marked ")
        .to_ascii_lowercase();
    normalized == "1" || normalized == "true" || normalized == "yes" || normalized == named_state
}

fn md_state_indicates_active(value: &str) -> bool {
    value
        .split(|character: char| !character.is_ascii_alphanumeric())
        .any(|token| matches!(token.to_ascii_lowercase().as_str(), "clean" | "active"))
}

fn md_device_count_property(node: &Node, key: &str) -> Option<u64> {
    property_value_from_node(node, key).and_then(|value| value.trim().parse().ok())
}

fn zfs_status_is_online(value: &str) -> bool {
    value.trim().eq_ignore_ascii_case("online")
}

fn vdo_operating_mode_is_stopped(value: &str) -> bool {
    let normalized = value
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_whitespace() || character == '_' {
                '-'
            } else {
                character
            }
        })
        .collect::<String>()
        .to_ascii_lowercase();
    matches!(normalized.as_str(), "stopped" | "not-running" | "inactive")
}

fn property_value_from_node<'a>(node: &'a Node, key: &str) -> Option<&'a str> {
    node.properties
        .iter()
        .find(|property| property.key == key)
        .map(|property| property.value.as_str())
}

fn current_mount_option_map(node: &Node) -> BTreeMap<String, String> {
    let mut options = property_value_from_node(node, "mount.options")
        .map(parse_mount_option_map)
        .unwrap_or_default();

    for property in &node.properties {
        if let Some(option) = property.key.strip_prefix("nfs.") {
            options
                .entry(normalize_mount_option_name(option))
                .or_insert_with(|| property.value.clone());
        }
    }
    if property_value_from_node(node, "mount.read-only") == Some("true") {
        options
            .entry("ro".to_string())
            .or_insert("true".to_string());
    }
    if property_value_from_node(node, "mount.read-write") == Some("true") {
        options
            .entry("rw".to_string())
            .or_insert("true".to_string());
    }
    if property_value_from_node(node, "mount.bind") == Some("true") {
        options
            .entry("bind".to_string())
            .or_insert("true".to_string());
    }

    options
}

fn current_nfs_export_option_map(node: &Node) -> BTreeMap<String, String> {
    node.properties
        .iter()
        .filter_map(|property| {
            property
                .key
                .strip_prefix("nfs.export-option-")
                .map(|option| (normalize_mount_option_name(option), property.value.clone()))
        })
        .filter(|(option, _)| !option.is_empty())
        .collect()
}

fn option_differences(
    desired_options: &BTreeMap<String, String>,
    current_options: &BTreeMap<String, String>,
) -> Vec<String> {
    desired_options
        .iter()
        .filter_map(|(option, desired)| match current_options.get(option) {
            Some(current) if current == desired => None,
            _ => Some(format!("{option}={desired}")),
        })
        .collect()
}

fn parse_mount_option_map(options: &str) -> BTreeMap<String, String> {
    options
        .split(',')
        .filter_map(|option| {
            let option = option.trim();
            if option.is_empty() {
                return None;
            }
            Some(option.split_once('=').map_or_else(
                || (normalize_mount_option_name(option), "true".to_string()),
                |(key, value)| (normalize_mount_option_name(key), value.trim().to_string()),
            ))
        })
        .filter(|(key, _)| !key.is_empty())
        .collect()
}

fn normalize_mount_option_name(option: &str) -> String {
    option
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|character| match character {
            'a'..='z' | '0'..='9' => character,
            _ => '-',
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
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
        target: lifecycle_target(collection, name, object),
        device: lifecycle_device(collection, object),
        devices: lifecycle_devices(collection, object),
        cache_set_uuid: metadata_string_field(
            object,
            &[
                "cacheSetUuid",
                "cacheSetUUID",
                "cache-set-uuid",
                "cache_set_uuid",
                "newCacheSetUuid",
                "newCacheSetUUID",
                "new-cache-set-uuid",
            ],
        ),
        rename_to: string_field(object, &["renameTo", "renameTarget", "newName"]),
        fs_type: string_field(object, &["fsType", "type"]),
        mountpoint: string_field(object, &["mountpoint", "path"])
            .or_else(|| name.starts_with('/').then(|| name.to_string())),
        desired_size: desired_size(object),
        physical_size: metadata_string_field(
            object,
            &[
                "physicalSize",
                "physical-size",
                "physical_size",
                "vdoPhysicalSize",
                "vdo-physical-size",
                "vdo_physical_size",
            ],
        ),
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

fn lifecycle_device(collection: &str, object: &Value) -> Option<String> {
    let keys: &[&str] = if collection == "luns" {
        &["device", "disk", "source", "path"]
    } else {
        &["device", "disk", "source"]
    };
    string_field(object, keys)
}

fn lifecycle_devices(collection: &str, object: &Value) -> Vec<String> {
    let keys: &[&str] = if collection == "luns" {
        &["devices", "devicePaths", "paths", "addDevices"]
    } else {
        &["devices", "addDevices"]
    };
    string_array_field(object, keys)
}

fn lifecycle_target(collection: &str, name: &str, object: &Value) -> Option<String> {
    if let Some(target) = string_field(object, &["target", "path", "mountpoint"]) {
        return Some(target);
    }
    if collection == "caches" || collection == "mdRaids" || collection == "multipathMaps" {
        if let Some(device) = string_field(object, &["device", "disk", "source"])
            .filter(|target| lifecycle_device_can_be_target(collection, target))
        {
            return Some(device);
        }
    }
    Some(name.to_string())
}

fn lifecycle_device_can_be_target(collection: &str, target: &str) -> bool {
    matches!(
        (collection, target),
        ("caches", target) if target.starts_with("/dev/bcache")
    ) || matches!(
        (collection, target),
        ("mdRaids", target) if target.starts_with("/dev/md")
    ) || matches!(
        (collection, target),
        ("multipathMaps", target)
            if target.starts_with("mpath") || target.starts_with("/dev/mapper/")
    )
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
        | Operation::Rescan
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
    let device =
        string_field(swap, &["target", "path", "device"]).unwrap_or_else(|| name.to_string());
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
        Some(Operation::Deactivate | Operation::Stop) => actions.push(PlannedAction {
            id: format!("swaps:{name}:deactivate"),
            description: format!("disable active swap on {device}"),
            operation: Operation::Deactivate,
            risk: RiskClass::OfflineRequired,
            destructive: false,
            context: context.clone(),
            advice: Some(Advice {
                summary: "swap deactivation runs swapoff without removing the swap signature"
                    .to_string(),
                alternatives: vec![
                    "add replacement swap capacity before disabling active swap".to_string(),
                    "use destroy only when the swap signature should be removed".to_string(),
                    "verify resume and hibernation references before disabling swap".to_string(),
                ],
            }),
        }),
        Some(Operation::Destroy) => actions.push(PlannedAction {
            id: format!("swaps:{name}:destroy"),
            description: format!("disable swap and remove swap signature from {device}"),
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            destructive: true,
            context: context.clone(),
            advice: Some(Advice {
                summary:
                    "swap destruction disables active swap and removes swap signature metadata"
                        .to_string(),
                alternatives: vec![
                    "use operation = \"deactivate\" to run swapoff without removing the signature"
                        .to_string(),
                    "remove or update NixOS swapDevices before deleting the swap signature"
                        .to_string(),
                    "verify resume and hibernation references before wiping swap metadata"
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

fn add_zram_actions(actions: &mut Vec<PlannedAction>, zram: &Map<String, Value>) {
    let operation = zram
        .get("operation")
        .or_else(|| zram.get("action"))
        .and_then(Value::as_str)
        .and_then(parse_operation);
    let context = ActionContext {
        collection: Some("zram".to_string()),
        name: Some("zram".to_string()),
        target: Some("zram".to_string()),
        ..ActionContext::default()
    };

    match operation {
        Some(Operation::Rescan) => actions.push(PlannedAction {
            id: "zram:rescan".to_string(),
            description: "refresh zram compressed swap inventory".to_string(),
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            destructive: false,
            context: context.clone(),
            advice: Some(Advice {
                summary: "zram rescan refreshes generated compressed swap state".to_string(),
                alternatives: vec![
                    "review zramctl output before changing generated zramSwap settings".to_string(),
                    "coordinate swapoff and setup when active zram devices must be recreated"
                        .to_string(),
                ],
            }),
        }),
        _ => actions.push(PlannedAction {
            id: "zram:inspect".to_string(),
            description: "inspect zram compressed swap declaration".to_string(),
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            destructive: false,
            context: context.clone(),
            advice: None,
        }),
    }

    add_zram_property_actions(actions, zram, &context);
}

fn add_zram_property_actions(
    actions: &mut Vec<PlannedAction>,
    zram: &Map<String, Value>,
    context: &ActionContext,
) {
    let Some(properties) = zram.get("properties").and_then(Value::as_object) else {
        return;
    };

    for (property, value) in properties {
        actions.push(PlannedAction {
            id: format!("zram:set-property:{property}"),
            description: format!("set zram property {property}"),
            operation: Operation::SetProperty,
            risk: RiskClass::Unsupported,
            destructive: false,
            context: ActionContext {
                property: Some(property.to_string()),
                property_value: Some(property_value(value)),
                ..context.clone()
            },
            advice: Some(Advice {
                summary: format!("zram property {property} requires generator reconciliation"),
                alternatives: vec![
                    "use services.disk-nix.zram options to derive NixOS zramSwap".to_string(),
                    "run a zram rescan before recreating active compressed swap devices".to_string(),
                    "coordinate swapoff before changing live zram algorithm, priority, size, or writeback device"
                        .to_string(),
                ],
            }),
        });
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
    let mapper_name = string_field(
        luks,
        &["target", "mapperName", "mapper-name", "mapper", "name"],
    )
    .unwrap_or_else(|| name.to_string());
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
    let snapshot_name = string_field(snapshot, &["name", "snapshotName", "snapshot-name"])
        .unwrap_or_else(|| name.to_string());
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
                name: Some(snapshot_name.clone()),
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
        actions.push(snapshot_hold_action(
            name,
            &snapshot_name,
            target,
            &hold,
            read_only,
            false,
        ));
    }
    if let Some(release_hold) = release_hold {
        actions.push(snapshot_hold_action(
            name,
            &snapshot_name,
            target,
            &release_hold,
            read_only,
            true,
        ));
    }
    if let Some(clone_to) = clone_to {
        actions.push(PlannedAction {
            id: format!("snapshot:{name}:clone:{clone_to}"),
            description: format!("clone snapshot {snapshot_name} to {clone_to}"),
            operation: Operation::Clone,
            risk: RiskClass::Reversible,
            destructive: false,
            context: ActionContext {
                collection: Some("snapshots".to_string()),
                name: Some(snapshot_name.clone()),
                target: Some(clone_to),
                snapshot_path: snapshot_path.clone(),
                read_only,
                ..ActionContext::default()
            },
            advice: Some(Advice {
                summary: "snapshot clone creates a writable ZFS dataset or Btrfs subvolume copy"
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
            description: format!("rename snapshot {snapshot_name} to {rename_to}"),
            operation: Operation::Rename,
            risk: RiskClass::OfflineRequired,
            destructive: false,
            context: ActionContext {
                collection: Some("snapshots".to_string()),
                name: Some(snapshot_name.clone()),
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
            description: format!("destroy snapshot {snapshot_name} for {target}"),
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            destructive: true,
            context: ActionContext {
                collection: Some("snapshots".to_string()),
                name: Some(snapshot_name.clone()),
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
            description: format!("roll back {target} to snapshot {snapshot_name}"),
            operation: Operation::Rollback,
            risk: RiskClass::PotentialDataLoss,
            destructive: false,
            context: ActionContext {
                collection: Some("snapshots".to_string()),
                name: Some(snapshot_name.clone()),
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
            description: format!("create snapshot {snapshot_name} for {target}"),
            operation: Operation::Snapshot,
            risk: RiskClass::Reversible,
            destructive: false,
            context: ActionContext {
                collection: Some("snapshots".to_string()),
                name: Some(snapshot_name),
                target: Some(target.to_string()),
                read_only,
                ..ActionContext::default()
            },
            advice: None,
        });
    }
}

fn snapshot_hold_action(
    action_name: &str,
    snapshot_name: &str,
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
            "snapshot:{action_name}:{}:{tag}",
            if release { "release-hold" } else { "hold" }
        ),
        description: format!("{verb} snapshot {snapshot_name} for {target} with tag {tag}"),
        operation: Operation::SetProperty,
        risk: RiskClass::Safe,
        destructive: false,
        context: ActionContext {
            collection: Some("snapshots".to_string()),
            name: Some(snapshot_name.to_string()),
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
        Operation::Rescan if collection == "filesystems" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "filesystem rescan refreshes mount and graph inventory without changing data"
                    .to_string(),
                alternatives: vec![
                    "use rescan before mount, remount, trim, check, or repair planning when current state may be stale"
                        .to_string(),
                    "use filesystem-specific check or scrub operations when integrity validation is needed"
                        .to_string(),
                    "persist steady-state mount declarations through NixOS fileSystems"
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
        Operation::Create if collection == "backingFiles" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "backing file creation initializes a new sparse file-backed storage origin"
                    .to_string(),
                alternatives: vec![
                    "verify the parent filesystem has enough free space before creating sparse images"
                        .to_string(),
                    "use grow only when an existing backing file should be extended".to_string(),
                    "create loop, swap, or filesystem consumers only after the file identity is verified"
                        .to_string(),
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
        Operation::Grow if collection == "backingFiles" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "backing file growth extends a file-backed storage origin before consumer refresh"
                    .to_string(),
                alternatives: vec![
                    "grow file-backed storage before refreshing loop devices or swap signatures"
                        .to_string(),
                    "prefer adding a replacement image and migrating consumers when shrinking is needed"
                        .to_string(),
                    "verify sparse-file allocation and host filesystem free space before growth"
                        .to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "backingFiles" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "backing file rescan refreshes file size, allocation, and graph relationships"
                    .to_string(),
                alternatives: vec![
                    "use grow only when the file-backed storage origin must be extended"
                        .to_string(),
                    "refresh loop devices after backing file size changes".to_string(),
                    "inspect dependent swap, loop, filesystem, or mapping consumers before detach"
                        .to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "dmMaps" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "device-mapper rescan refreshes dmsetup map, dependency, table, and status metadata"
                    .to_string(),
                alternatives: vec![
                    "use dmMaps.<name>.operation = \"rescan\" before editing dependent LUKS, LVM, VDO, or multipath layers"
                        .to_string(),
                    "review dmsetup table and status output before any destructive mapper replacement"
                        .to_string(),
                    "use domain-specific LUKS, LVM, VDO, or multipath declarations for mutating mapper lifecycle"
                        .to_string(),
                ],
            }),
        ),
        Operation::Destroy if collection == "dmMaps" => (
            RiskClass::Destructive,
            true,
            Some(Advice {
                summary: "device-mapper removal deletes the live map and can make dependent data inaccessible"
                    .to_string(),
                alternatives: vec![
                    "prefer LUKS, LVM, VDO, multipath, or cache-specific close/deactivate/detach declarations when the map is owned by another domain"
                        .to_string(),
                    "run dmMaps.<name>.operation = \"rescan\" and review dmsetup status before removal"
                        .to_string(),
                    "unmount filesystems and stop services before removing the mapper".to_string(),
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
        Operation::Destroy if collection == "multipathMaps" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "multipath map removal flushes the host map without deleting target-side data"
                    .to_string(),
                alternatives: vec![
                    "unmount filesystems and deactivate LVM, dm, and service consumers before flushing the map"
                        .to_string(),
                    "remove or drain individual failed paths first when alternate paths must remain active"
                        .to_string(),
                    "use a rescan or reload when the map should stay present and only path metadata changed"
                        .to_string(),
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
        Operation::Attach if collection == "nvmeNamespaces" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "NVMe namespace attach exposes an existing namespace to selected controllers"
                    .to_string(),
                alternatives: vec![
                    "attach an existing namespace instead of creating one when preserving data"
                        .to_string(),
                    "verify namespace id and controller list with nvme list-ns before attach"
                        .to_string(),
                    "rescan the controller and verify dependent consumers after attachment"
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
        Operation::Detach if collection == "nvmeNamespaces" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "NVMe namespace detach removes host/controller access without deleting the namespace"
                    .to_string(),
                alternatives: vec![
                    "detach from selected controllers before deleting only when data removal is intended"
                        .to_string(),
                    "unmount filesystems and deactivate LVM, dm, or multipath consumers before detach"
                        .to_string(),
                    "use rescan when namespace visibility changed outside disk-nix".to_string(),
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
                    "{} operations are currently only supported for LUNs and NVMe namespaces",
                    operation_label(operation)
                ),
                alternatives: vec![
                    "use operation = \"attach\" or \"detach\" on luns declarations for host-side LUN path lifecycle"
                        .to_string(),
                    "use operation = \"attach\" or \"detach\" on nvmeNamespaces declarations for namespace/controller lifecycle"
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
                summary: "rescan operations are currently supported for filesystems, disks, partitions, snapshots, LUNs, iSCSI sessions, NFS exports/mounts, NVMe namespaces, multipath maps, loop devices, backing files, ZFS datasets/zvols, Btrfs subvolumes/qgroups, LVM PV/VG/LV/snapshot/cache/thin-pool metadata, MD RAID metadata, VDO status, and bcache status"
                    .to_string(),
                alternatives: vec![
                    "use filesystems.<name>.operation = \"rescan\" to refresh local mount and graph inventory"
                        .to_string(),
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
                    "use backingFiles.<path>.operation = \"rescan\" to refresh file-backed storage origin inventory"
                        .to_string(),
                    "use dmMaps.<name>.operation = \"rescan\" to refresh device-mapper table and status metadata"
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
            node_kind: NodeKind::Swap,
            operation: Operation::Deactivate,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "swap deactivation runs swapoff without removing the signature"
                    .to_string(),
                alternatives: vec![
                    "add replacement swap capacity before disabling this swap".to_string(),
                    "use destroy only when the swap signature should be removed".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Swap,
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "swap destruction disables swap and removes signature metadata".to_string(),
                alternatives: vec![
                    "use deactivate to run swapoff without wiping the signature".to_string(),
                    "remove NixOS swapDevices and resume references before metadata removal"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZramDevice,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "zram inventory refresh reads compressed swap state from zramctl"
                    .to_string(),
                alternatives: vec![
                    "derive steady-state zram devices through NixOS zramSwap".to_string(),
                    "coordinate swapoff before recreating active zram devices".to_string(),
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
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "filesystem rescan refreshes mount and modeled graph state".to_string(),
                alternatives: vec![
                    "use rescan before planning mount, remount, trim, check, or repair work"
                        .to_string(),
                    "use check, scrub, or repair when data or metadata integrity must be validated"
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
            operation: Operation::Clone,
            risk: RiskClass::Reversible,
            advice: Some(Advice {
                summary: "Btrfs snapshot clone creates a reviewed subvolume copy".to_string(),
                alternatives: vec![
                    "clone snapshots for inspection before rollback or pruning".to_string(),
                    "use read-only clones when the copy should remain a recovery checkpoint"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsSnapshot,
            operation: Operation::Rename,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "Btrfs snapshot rename preserves a recovery point at a new path"
                    .to_string(),
                alternatives: vec![
                    "update mounts, qgroups, send/receive jobs, and retention references after rename"
                        .to_string(),
                    "clone before renaming when consumers still need the old path".to_string(),
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
            node_kind: NodeKind::BackingFile,
            operation: Operation::Create,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "backing file creation initializes a new sparse file-backed storage origin"
                    .to_string(),
                alternatives: vec![
                    "verify the parent filesystem has enough free space first".to_string(),
                    "attach loop devices or swap only after the file is verified".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BackingFile,
            operation: Operation::Grow,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "backing file growth extends file-backed storage before consumer refresh"
                    .to_string(),
                alternatives: vec![
                    "verify host filesystem free space before extending sparse or preallocated images"
                        .to_string(),
                    "refresh loop, swap, filesystem, or mapping consumers after growth".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BackingFile,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "backing file rescan refreshes size, allocation, and consumer relationships"
                    .to_string(),
                alternatives: vec![
                    "use grow when the file-backed origin capacity must change".to_string(),
                    "inspect consumers before detaching loop devices or disabling swap".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::DeviceMapper,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "device-mapper rescan refreshes map identity, dependencies, table, and status metadata"
                    .to_string(),
                alternatives: vec![
                    "use LUKS, LVM, VDO, multipath, or cache declarations for domain-specific mutations"
                        .to_string(),
                    "review dmsetup status before changing dependent filesystems or volumes"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::DeviceMapper,
            operation: Operation::Rename,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "device-mapper rename changes the visible mapper path without deleting mapped data"
                    .to_string(),
                alternatives: vec![
                    "update dependent LUKS, LVM, VDO, multipath, filesystem, mount, and service declarations before applying"
                        .to_string(),
                    "prefer the owning LUKS, LVM, VDO, multipath, or cache declaration when the map is domain-managed"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::DeviceMapper,
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "device-mapper removal deletes the live map and can make dependent data inaccessible"
                    .to_string(),
                alternatives: vec![
                    "use domain-specific LUKS, LVM, VDO, multipath, or cache teardown when available"
                        .to_string(),
                    "review dmsetup status and dependent mounts before removal".to_string(),
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
            node_kind: NodeKind::BcachefsFilesystem,
            operation: Operation::Grow,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "bcachefs member growth expands a mounted filesystem after backing capacity changes"
                    .to_string(),
                alternatives: vec![
                    "verify the member device and target size before resizing".to_string(),
                    "refresh bcachefs usage after growth before resizing consumers".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BcachefsFilesystem,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "bcachefs rescan refreshes filesystem and member-device usage metadata"
                    .to_string(),
                alternatives: vec![
                    "run rescan before device replacement or removal planning".to_string(),
                    "review per-device free and data-type byte accounting".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BcachefsFilesystem,
            operation: Operation::AddDevice,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "adding a bcachefs member expands the mounted filesystem device set"
                    .to_string(),
                alternatives: vec![
                    "verify the new block device identity before adding it".to_string(),
                    "rereplicate data after topology changes when replicas or durability changed"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BcachefsFilesystem,
            operation: Operation::ReplaceDevice,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "bcachefs replacement should add new capacity, rereplicate data, then remove the old member"
                    .to_string(),
                alternatives: vec![
                    "add replacement capacity before evacuating old media".to_string(),
                    "inspect rereplication status before removing the old member".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BcachefsFilesystem,
            operation: Operation::RemoveDevice,
            risk: RiskClass::PotentialDataLoss,
            advice: Some(Advice {
                summary: "bcachefs member removal requires enough remaining capacity and replicas"
                    .to_string(),
                alternatives: vec![
                    "add replacement capacity before removal".to_string(),
                    "rereplicate data and verify free metadata capacity before final removal"
                        .to_string(),
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
            node_kind: NodeKind::BcachefsFilesystem,
            operation: Operation::Rebalance,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "bcachefs rereplication rebalances data across the current member set"
                    .to_string(),
                alternatives: vec![
                    "inspect bcachefs usage before and after rereplication".to_string(),
                    "prefer rereplication before removing or replacing a member".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BcachefsFilesystem,
            operation: Operation::Scrub,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "bcachefs scrub verifies filesystem data and metadata online".to_string(),
                alternatives: vec![
                    "review scrub output before repair or topology contraction".to_string(),
                    "use filesystem check when offline metadata validation is required"
                        .to_string(),
                ],
            }),
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
            operation: Operation::Destroy,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "multipath map removal flushes the host map without deleting target-side data"
                    .to_string(),
                alternatives: vec![
                    "deactivate filesystems, LVM, dm, and services before flushing the map"
                        .to_string(),
                    "prefer path removal or rescan when the map should remain available".to_string(),
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
    use disk_nix_model::Usage;

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
        let btrfs_clone = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::BtrfsSnapshot
                    && capability.operation == Operation::Clone
            })
            .expect("Btrfs snapshot clone capability should exist");
        let btrfs_rename = capabilities
            .iter()
            .find(|capability| {
                capability.node_kind == NodeKind::BtrfsSnapshot
                    && capability.operation == Operation::Rename
            })
            .expect("Btrfs snapshot rename capability should exist");
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
        assert_eq!(btrfs_clone.risk, RiskClass::Reversible);
        assert_eq!(btrfs_rename.risk, RiskClass::OfflineRequired);
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
            (Operation::Rescan, RiskClass::Online),
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

        for (operation, risk) in [
            (Operation::Grow, RiskClass::Online),
            (Operation::Rescan, RiskClass::Online),
            (Operation::AddDevice, RiskClass::Online),
            (Operation::ReplaceDevice, RiskClass::OfflineRequired),
            (Operation::RemoveDevice, RiskClass::PotentialDataLoss),
            (Operation::Rebalance, RiskClass::Online),
            (Operation::Scrub, RiskClass::Online),
        ] {
            let capability = capabilities
                .iter()
                .find(|capability| {
                    capability.node_kind == NodeKind::BcachefsFilesystem
                        && capability.operation == operation
                })
                .unwrap_or_else(|| {
                    panic!("bcachefs filesystem {operation:?} capability should exist")
                });
            assert_eq!(capability.risk, risk);
            assert!(capability.advice.is_some());
        }
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
            (NodeKind::ZramDevice, Operation::Rescan, RiskClass::Online),
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
            (NodeKind::BackingFile, Operation::Create, RiskClass::Online),
            (NodeKind::BackingFile, Operation::Rescan, RiskClass::Online),
            (NodeKind::BackingFile, Operation::Grow, RiskClass::Online),
            (NodeKind::DeviceMapper, Operation::Rescan, RiskClass::Online),
            (
                NodeKind::DeviceMapper,
                Operation::Rename,
                RiskClass::OfflineRequired,
            ),
            (
                NodeKind::DeviceMapper,
                Operation::Destroy,
                RiskClass::Destructive,
            ),
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
                Operation::Grow,
                RiskClass::Online,
            ),
            (
                NodeKind::MultipathDevice,
                Operation::Rescan,
                RiskClass::Online,
            ),
            (
                NodeKind::MultipathDevice,
                Operation::Destroy,
                RiskClass::OfflineRequired,
            ),
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
    fn plan_accepts_supported_spec_versions() {
        let direct = plan_from_json_bytes(
            br#"{
              "version": 1,
              "filesystems": {
                "root": {
                  "mountpoint": "/",
                  "fsType": "ext4",
                  "resizePolicy": "grow-only"
                }
              }
            }"#,
        )
        .expect("direct spec should parse");

        let wrapped = plan_from_json_bytes(
            br#"{
              "version": 1,
              "spec": {
                "version": 1,
                "filesystems": {
                  "root": {
                    "mountpoint": "/",
                    "fsType": "ext4",
                    "resizePolicy": "grow-only"
                  }
                }
              }
            }"#,
        )
        .expect("wrapped spec should parse");

        assert_eq!(direct.summary.action_count, 1);
        assert_eq!(wrapped.summary.action_count, 1);
    }

    #[test]
    fn plan_rejects_unsupported_spec_versions() {
        let error = plan_from_json_bytes(
            br#"{
              "version": 2,
              "filesystems": {}
            }"#,
        )
        .expect_err("future version should be rejected");

        assert_eq!(
            error.to_string(),
            "unsupported disk-nix spec version 2; supported version is 1"
        );
    }

    #[test]
    fn plan_rejects_invalid_and_conflicting_spec_versions() {
        let invalid = plan_from_json_bytes(
            br#"{
              "spec": {
                "version": "1"
              }
            }"#,
        )
        .expect_err("string version should be rejected");

        let conflicting = plan_from_json_bytes(
            br#"{
              "version": 1,
              "spec": {
                "version": 2
              }
            }"#,
        )
        .expect_err("conflicting versions should be rejected");

        assert_eq!(
            invalid.to_string(),
            "disk-nix spec version at spec.version must be an integer"
        );
        assert_eq!(
            conflicting.to_string(),
            "conflicting disk-nix spec versions: top-level version 1, spec.version 2"
        );
    }

    #[test]
    fn plan_orders_stacked_storage_actions_by_dependency_layer() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "root": {
                  "mountpoint": "/",
                  "fsType": "ext4",
                  "resizePolicy": "grow-only"
                }
              },
              "volumes": {
                "root": {
                  "operation": "create",
                  "device": "/dev/vg/root"
                }
              },
              "volumeGroups": {
                "vg": {
                  "operation": "create"
                }
              },
              "physicalVolumes": {
                "pv0": {
                  "operation": "create",
                  "device": "/dev/mapper/cryptroot"
                }
              },
              "luks": {
                "devices": {
                  "cryptroot": {
                    "operation": "open",
                    "device": "/dev/disk/by-partlabel/root"
                  }
                }
              },
              "partitions": {
                "root": {
                  "operation": "create",
                  "device": "/dev/disk/by-partlabel/root"
                }
              },
              "disks": {
                "system": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/nvme-system"
                }
              },
              "snapshots": {
                "old-root": {
                  "target": "tank/root@old",
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");

        let ids: Vec<&str> = plan
            .actions
            .iter()
            .map(|action| action.id.as_str())
            .collect();

        assert_eq!(
            ids,
            vec![
                "disks:system:create",
                "partitions:root:create",
                "luks.devices:cryptroot:open",
                "physicalvolumes:pv0:create",
                "volumegroups:vg:create",
                "volumes:root:create",
                "filesystem:root:grow",
                "snapshot:old-root:destroy",
            ]
        );
        let dependency_ids: Vec<&str> = plan
            .dependency_order
            .iter()
            .map(|order| order.action_id.as_str())
            .collect();
        assert_eq!(dependency_ids, ids);
        assert_eq!(
            plan.dependency_order.first().map(|order| (
                order.phase,
                order.direction,
                order.layer_rank,
                order.collection.as_deref()
            )),
            Some((
                DependencyPhase::BuildLowerLayers,
                DependencyDirection::LowerLayersFirst,
                20,
                Some("disks")
            ))
        );
        assert_eq!(
            plan.dependency_order.last().map(|order| (
                order.phase,
                order.direction,
                order.layer_rank,
                order.collection.as_deref()
            )),
            Some((
                DependencyPhase::TearDownUpperLayers,
                DependencyDirection::UpperLayersFirst,
                95,
                Some("snapshots")
            ))
        );
        assert!(plan.dependency_order.iter().all(|order| {
            !order.notes.is_empty()
                && order
                    .notes
                    .iter()
                    .any(|note| note.contains("collection layer rank"))
        }));
    }

    #[test]
    fn dependency_order_reports_explicit_edges_for_layered_block_growth() {
        let plan = plan_from_json_bytes(
            br#"{
              "backingFiles": {
                "/var/lib/images/root.img": {
                  "operation": "grow",
                  "desiredSize": "32GiB"
                }
              },
              "loopDevices": {
                "/dev/loop7": {
                  "operation": "grow",
                  "device": "/var/lib/images/root.img"
                }
              },
              "filesystems": {
                "root": {
                  "operation": "grow",
                  "device": "/dev/loop7",
                  "desiredSize": "100%"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        let backing = plan
            .dependency_order
            .iter()
            .find(|order| order.action_id == "backingfiles:/var/lib/images/root.img:grow")
            .expect("backing file dependency order entry exists");
        let loop_device = plan
            .dependency_order
            .iter()
            .find(|order| order.action_id == "loopdevices:/dev/loop7:grow")
            .expect("loop device dependency order entry exists");
        let filesystem = plan
            .dependency_order
            .iter()
            .find(|order| order.action_id == "filesystem:root:inspect")
            .expect("filesystem dependency order entry exists");

        assert!(backing.depends_on.is_empty());
        assert_eq!(
            backing.unblocks,
            vec!["loopdevices:/dev/loop7:grow".to_string()]
        );
        assert_eq!(
            loop_device.depends_on,
            vec!["backingfiles:/var/lib/images/root.img:grow".to_string()]
        );
        assert_eq!(
            loop_device.unblocks,
            vec!["filesystem:root:inspect".to_string()]
        );
        assert_eq!(
            filesystem.depends_on,
            vec!["loopdevices:/dev/loop7:grow".to_string()]
        );
        assert!(filesystem.unblocks.is_empty());
        assert!(
            loop_device
                .notes
                .iter()
                .any(|note| note.contains("explicit dependency edge"))
        );
    }

    #[test]
    fn dependency_order_reports_explicit_edges_for_pool_dataset_snapshot_layers() {
        let plan = plan_from_json_bytes(
            br#"{
              "pools": {
                "tank": {
                  "operation": "import"
                }
              },
              "datasets": {
                "tank/home": {
                  "operation": "create"
                }
              },
              "snapshots": {
                "home-before": {
                  "target": "tank/home",
                  "name": "tank/home@before"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        let pool = plan
            .dependency_order
            .iter()
            .find(|order| order.action_id == "pools:tank:import")
            .expect("pool dependency order entry exists");
        let dataset = plan
            .dependency_order
            .iter()
            .find(|order| order.action_id == "datasets:tank/home:create")
            .expect("dataset dependency order entry exists");
        let snapshot = plan
            .dependency_order
            .iter()
            .find(|order| order.action_id == "snapshot:home-before:create")
            .expect("snapshot dependency order entry exists");

        assert_eq!(pool.unblocks, vec!["datasets:tank/home:create".to_string()]);
        assert_eq!(dataset.depends_on, vec!["pools:tank:import".to_string()]);
        assert_eq!(
            dataset.unblocks,
            vec!["snapshot:home-before:create".to_string()]
        );
        assert_eq!(
            snapshot.depends_on,
            vec!["datasets:tank/home:create".to_string()]
        );
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
    fn plan_accepts_filesystem_rescan_operation() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "scratch": {
                  "mountpoint": "/scratch",
                  "device": "/dev/disk/by-label/scratch",
                  "fsType": "xfs",
                  "operation": "rescan"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 2);
        assert_eq!(plan.summary.offline_required_count, 0);
        assert_eq!(plan.summary.unsupported_count, 0);
        let rescan = plan
            .actions
            .iter()
            .find(|action| action.id == "filesystems:scratch:rescan")
            .expect("filesystem rescan action exists");
        assert_eq!(rescan.operation, Operation::Rescan);
        assert_eq!(rescan.risk, RiskClass::Online);
        assert!(!rescan.destructive);
        assert_eq!(rescan.context.target.as_deref(), Some("/scratch"));
        assert_eq!(
            rescan.context.device.as_deref(),
            Some("/dev/disk/by-label/scratch")
        );
        assert!(
            rescan
                .advice
                .as_ref()
                .is_some_and(|advice| advice.summary.contains("refreshes mount"))
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
                },
                "retired": {
                  "device": "/dev/disk/by-label/old-swap",
                  "operation": "deactivate"
                },
                "remove": {
                  "device": "/dev/disk/by-label/remove-swap",
                  "operation": "destroy"
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

        assert_eq!(plan.summary.action_count, 12);
        assert_eq!(plan.summary.offline_required_count, 8);
        assert_eq!(plan.summary.destructive_count, 3);

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

        let swap_deactivate = plan
            .actions
            .iter()
            .find(|action| action.id == "swaps:retired:deactivate")
            .expect("swap deactivate action exists");
        assert_eq!(swap_deactivate.operation, Operation::Deactivate);
        assert_eq!(swap_deactivate.risk, RiskClass::OfflineRequired);
        assert!(!swap_deactivate.destructive);

        let swap_destroy = plan
            .actions
            .iter()
            .find(|action| action.id == "swaps:remove:destroy")
            .expect("swap destroy action exists");
        assert_eq!(swap_destroy.operation, Operation::Destroy);
        assert_eq!(swap_destroy.risk, RiskClass::Destructive);
        assert!(swap_destroy.destructive);
        assert!(swap_destroy.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("deactivate"))
        }));

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
    fn plan_accepts_luks_mapper_aliases_for_logical_keys() {
        let plan = plan_from_json_bytes(
            br#"{
              "luks": {
                "devices": {
                  "rootMapping": {
                    "target": "cryptroot",
                    "device": "/dev/disk/by-id/root-luks",
                    "operation": "grow"
                  },
                  "archiveMapping": {
                    "mapperName": "cryptarchive",
                    "device": "/dev/disk/by-id/archive-luks",
                    "operation": "open"
                  },
                  "backupMapping": {
                    "mapper": "cryptbackup",
                    "device": "/dev/disk/by-id/backup-luks",
                    "operation": "close"
                  },
                  "hyphenMapping": {
                    "mapper-name": "crypthyphen",
                    "device": "/dev/disk/by-id/hyphen-luks",
                    "operation": "open"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");

        let root = plan
            .actions
            .iter()
            .find(|action| action.id == "luks.devices:rootMapping:grow")
            .expect("target alias grow action exists");
        assert_eq!(root.context.target.as_deref(), Some("cryptroot"));

        let archive = plan
            .actions
            .iter()
            .find(|action| action.id == "luks.devices:archiveMapping:open")
            .expect("mapperName alias open action exists");
        assert_eq!(archive.context.target.as_deref(), Some("cryptarchive"));

        let backup = plan
            .actions
            .iter()
            .find(|action| action.id == "luks.devices:backupMapping:close")
            .expect("mapper alias close action exists");
        assert_eq!(backup.context.target.as_deref(), Some("cryptbackup"));

        let hyphen = plan
            .actions
            .iter()
            .find(|action| action.id == "luks.devices:hyphenMapping:open")
            .expect("hyphenated mapper alias open action exists");
        assert_eq!(hyphen.context.target.as_deref(), Some("crypthyphen"));
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
    fn plan_accepts_swap_path_aliases_for_logical_keys() {
        let plan = plan_from_json_bytes(
            br#"{
              "swaps": {
                "scratch": {
                  "path": "/swapfile",
                  "operation": "grow",
                  "desiredSize": "16GiB"
                },
                "inventory": {
                  "target": "/dev/disk/by-label/swap-inventory",
                  "operation": "rescan"
                },
                "primary": {
                  "path": "/dev/disk/by-label/swap",
                  "preserveData": false
                }
              }
            }"#,
        )
        .expect("plan should parse");

        let grow = plan
            .actions
            .iter()
            .find(|action| action.id == "swaps:scratch:grow")
            .expect("logical-key swap grow action exists");
        assert_eq!(grow.context.target.as_deref(), Some("/swapfile"));
        assert_eq!(grow.context.device.as_deref(), Some("/swapfile"));
        assert_eq!(grow.context.desired_size.as_deref(), Some("16GiB"));

        let rescan = plan
            .actions
            .iter()
            .find(|action| action.id == "swaps:inventory:rescan")
            .expect("logical-key swap rescan action exists");
        assert_eq!(
            rescan.context.target.as_deref(),
            Some("/dev/disk/by-label/swap-inventory")
        );

        let format = plan
            .actions
            .iter()
            .find(|action| action.id == "swaps:primary:format")
            .expect("logical-key swap format action exists");
        assert_eq!(
            format.context.target.as_deref(),
            Some("/dev/disk/by-label/swap")
        );
        assert_eq!(format.risk, RiskClass::Destructive);
    }

    #[test]
    fn plan_classifies_zram_rescan_and_properties() {
        let plan = plan_from_json_bytes(
            br#"{
              "zram": {
                "enable": true,
                "operation": "rescan",
                "swapDevices": 2,
                "memoryPercent": 40,
                "memoryMax": 8589934592,
                "priority": 20,
                "algorithm": "zstd",
                "properties": {
                  "zram.compression-ratio-target": "2.0"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 2);
        assert_eq!(plan.summary.unsupported_count, 1);

        let rescan = plan
            .actions
            .iter()
            .find(|action| action.id == "zram:rescan")
            .expect("zram rescan action exists");
        assert_eq!(rescan.operation, Operation::Rescan);
        assert_eq!(rescan.risk, RiskClass::Online);
        assert!(!rescan.destructive);
        assert_eq!(rescan.context.collection.as_deref(), Some("zram"));

        let property = plan
            .actions
            .iter()
            .find(|action| action.id == "zram:set-property:zram.compression-ratio-target")
            .expect("zram property action exists");
        assert_eq!(property.operation, Operation::SetProperty);
        assert_eq!(property.risk, RiskClass::Unsupported);
        assert_eq!(property.context.property_value.as_deref(), Some("2.0"));
        assert!(property.advice.as_ref().is_some_and(|advice| {
            advice
                .alternatives
                .iter()
                .any(|alternative| alternative.contains("zramSwap"))
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
                  "physicalSize": "6TiB",
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
        assert_eq!(grow.context.desired_size.as_deref(), Some("4TiB"));
        assert_eq!(grow.context.physical_size.as_deref(), Some("6TiB"));
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
                "/mnt/persist/@old-name": {
                  "operation": "rename",
                  "renameTo": "/mnt/persist/@new-name"
                },
                "/mnt/persist/@old": {
                  "destroy": true,
                  "preserveData": false
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 4);
        assert_eq!(plan.summary.offline_required_count, 1);
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
        let rename = plan
            .actions
            .iter()
            .find(|action| {
                action.id == "btrfsSubvolumes:/mnt/persist/@old-name:rename".to_ascii_lowercase()
            })
            .expect("rename action exists");
        assert_eq!(rename.operation, Operation::Rename);
        assert_eq!(rename.risk, RiskClass::OfflineRequired);
        assert_eq!(
            rename.context.rename_to.as_deref(),
            Some("/mnt/persist/@new-name")
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
                },
                "mpath-old": {
                  "target": "mpath-old",
                  "operation": "destroy"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 5);
        assert_eq!(plan.summary.offline_required_count, 2);
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
        let destroy = plan
            .actions
            .iter()
            .find(|action| action.id == "multipathmaps:mpath-old:destroy")
            .expect("multipath destroy action exists");
        assert_eq!(destroy.risk, RiskClass::OfflineRequired);
        assert!(!destroy.destructive);
        assert!(destroy.advice.as_ref().is_some_and(|advice| {
            advice
                .summary
                .contains("flushes the host map without deleting target-side data")
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
    fn plan_classifies_backing_file_lifecycle() {
        let plan = plan_from_json_bytes(
            br#"{
              "backingFiles": {
                "/var/lib/images/new.img": {
                  "operation": "create",
                  "desiredSize": "8GiB"
                },
                "/var/lib/images/root.img": {
                  "operation": "grow",
                  "desiredSize": "16GiB"
                },
                "inventory-image": {
                  "operation": "rescan",
                  "path": "/var/lib/images/inventory.img"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 3);
        assert_eq!(plan.summary.offline_required_count, 0);
        assert_eq!(plan.summary.destructive_count, 0);
        let create = plan
            .actions
            .iter()
            .find(|action| action.id == "backingfiles:/var/lib/images/new.img:create")
            .expect("backing file create action exists");
        assert_eq!(create.operation, Operation::Create);
        assert_eq!(create.risk, RiskClass::Online);
        assert_eq!(
            create.context.target.as_deref(),
            Some("/var/lib/images/new.img")
        );
        assert_eq!(create.context.desired_size.as_deref(), Some("8GiB"));
        assert!(create.advice.as_ref().is_some_and(|advice| {
            advice.summary.contains("backing file creation")
                && advice
                    .alternatives
                    .iter()
                    .any(|alternative| alternative.contains("existing backing file"))
        }));
        let grow = plan
            .actions
            .iter()
            .find(|action| action.id == "backingfiles:/var/lib/images/root.img:grow")
            .expect("backing file grow action exists");
        assert_eq!(grow.operation, Operation::Grow);
        assert_eq!(grow.risk, RiskClass::Online);
        assert_eq!(
            grow.context.target.as_deref(),
            Some("/var/lib/images/root.img")
        );
        assert_eq!(grow.context.desired_size.as_deref(), Some("16GiB"));
        assert!(
            grow.advice
                .as_ref()
                .is_some_and(|advice| advice.summary.contains("backing file growth"))
        );
        let rescan = plan
            .actions
            .iter()
            .find(|action| action.id == "backingfiles:inventory-image:rescan")
            .expect("backing file rescan action exists");
        assert_eq!(rescan.operation, Operation::Rescan);
        assert_eq!(rescan.risk, RiskClass::Online);
        assert_eq!(
            rescan.context.target.as_deref(),
            Some("/var/lib/images/inventory.img")
        );
        assert!(!rescan.destructive);
    }

    #[test]
    fn topology_comparison_reconciles_backing_file_create_and_grow() {
        let plan = plan_from_json_bytes(
            br#"{
              "backingFiles": {
                "/var/lib/images/new.img": {
                  "operation": "create",
                  "desiredSize": "8GiB"
                },
                "/var/lib/images/mismatch.img": {
                  "operation": "create",
                  "desiredSize": "8GiB"
                },
                "/var/lib/images/root.img": {
                  "operation": "grow",
                  "desiredSize": "8GiB"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "backing-file:/var/lib/images/new.img",
                NodeKind::BackingFile,
                "/var/lib/images/new.img",
            )
            .with_path("/var/lib/images/new.img")
            .with_size_bytes(8 * 1024 * 1024 * 1024),
        );
        graph.add_node(
            Node::new(
                "backing-file:/var/lib/images/mismatch.img",
                NodeKind::BackingFile,
                "/var/lib/images/mismatch.img",
            )
            .with_path("/var/lib/images/mismatch.img")
            .with_size_bytes(4 * 1024 * 1024 * 1024),
        );
        graph.add_node(
            Node::new(
                "backing-file:/var/lib/images/root.img",
                NodeKind::BackingFile,
                "/var/lib/images/root.img",
            )
            .with_path("/var/lib/images/root.img")
            .with_size_bytes(16 * 1024 * 1024 * 1024),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 3);
        assert_eq!(comparison.summary.matched_count, 3);
        assert_eq!(comparison.summary.already_satisfied_count, 2);
        assert_eq!(comparison.summary.suppressed_action_count, 2);
        assert_eq!(plan.summary.action_count, 1);
        assert!(
            plan.actions
                .iter()
                .all(|action| action.id == "backingfiles:/var/lib/images/mismatch.img:create")
        );
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "backingfiles:/var/lib/images/new.img:create"
                && diagnostic.kind == TopologyDiagnosticKind::BackingFileCreateAlreadySatisfied
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "backingfiles:/var/lib/images/mismatch.img:create"
                && diagnostic.kind == TopologyDiagnosticKind::BackingFileCreateRequired
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.message.contains("refuse to overwrite")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "backingfiles:/var/lib/images/root.img:grow"
                && diagnostic.kind == TopologyDiagnosticKind::SizeAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_reconciles_partition_grow_from_end_size() {
        let plan = plan_from_json_bytes(
            br#"{
              "partitions": {
                "root": {
                  "operation": "grow",
                  "target": "/dev/disk/by-partuuid/root",
                  "device": "/dev/disk/by-id/nvme-root",
                  "partitionNumber": 2,
                  "end": "64GiB"
                },
                "data": {
                  "operation": "grow",
                  "target": "/dev/disk/by-partuuid/data",
                  "device": "/dev/disk/by-id/nvme-root",
                  "partitionNumber": 3,
                  "end": "128GiB"
                },
                "max": {
                  "operation": "grow",
                  "target": "/dev/disk/by-partuuid/max",
                  "device": "/dev/disk/by-id/nvme-root",
                  "partitionNumber": 4,
                  "end": "100%"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "partition:/dev/disk/by-partuuid/root",
                NodeKind::Partition,
                "/dev/disk/by-partuuid/root",
            )
            .with_path("/dev/disk/by-partuuid/root")
            .with_size_bytes(80 * 1024 * 1024 * 1024),
        );
        graph.add_node(
            Node::new(
                "partition:/dev/disk/by-partuuid/data",
                NodeKind::Partition,
                "/dev/disk/by-partuuid/data",
            )
            .with_path("/dev/disk/by-partuuid/data")
            .with_size_bytes(64 * 1024 * 1024 * 1024),
        );
        graph.add_node(
            Node::new(
                "partition:/dev/disk/by-partuuid/max",
                NodeKind::Partition,
                "/dev/disk/by-partuuid/max",
            )
            .with_path("/dev/disk/by-partuuid/max")
            .with_size_bytes(64 * 1024 * 1024 * 1024),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 3);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert_eq!(plan.summary.action_count, 2);
        assert!(
            plan.actions
                .iter()
                .all(|action| action.id != "partitions:root:grow")
        );
        assert!(plan.actions.iter().any(|action| {
            action.id == "partitions:data:grow" && action.operation == Operation::Grow
        }));
        assert!(plan.actions.iter().any(|action| {
            action.id == "partitions:max:grow" && action.operation == Operation::Grow
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "partitions:root:grow"
                && diagnostic.kind == TopologyDiagnosticKind::SizeAlreadySatisfied
                && diagnostic.message.contains("desired size 64GiB")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "partitions:data:grow"
                && diagnostic.kind == TopologyDiagnosticKind::SizeBelowDesired
                && diagnostic.message.contains("desired size 128GiB")
        }));
        assert!(!comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "partitions:max:grow"
                && matches!(
                    diagnostic.kind,
                    TopologyDiagnosticKind::SizeAlreadySatisfied
                        | TopologyDiagnosticKind::SizeBelowDesired
                        | TopologyDiagnosticKind::SizeConflict
                )
        }));
    }

    #[test]
    fn topology_comparison_reconciles_partition_create_when_target_exists() {
        let plan = plan_from_json_bytes(
            br#"{
              "partitions": {
                "boot": {
                  "operation": "create",
                  "target": "/dev/disk/by-partuuid/boot",
                  "device": "/dev/disk/by-id/nvme-root",
                  "partitionNumber": 1,
                  "desiredSize": "1GiB"
                },
                "root": {
                  "operation": "create",
                  "target": "/dev/disk/by-partuuid/root",
                  "device": "/dev/disk/by-id/nvme-root",
                  "partitionNumber": 2,
                  "desiredSize": "64GiB"
                },
                "scratch": {
                  "operation": "create",
                  "target": "/dev/disk/by-partuuid/scratch",
                  "device": "/dev/disk/by-id/nvme-root",
                  "partitionNumber": 3
                },
                "wrong": {
                  "operation": "create",
                  "target": "/dev/disk/by-partuuid/wrong",
                  "device": "/dev/disk/by-id/nvme-root",
                  "partitionNumber": 4
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "partition:/dev/disk/by-partuuid/boot",
                NodeKind::Partition,
                "/dev/disk/by-partuuid/boot",
            )
            .with_path("/dev/disk/by-partuuid/boot")
            .with_size_bytes(1024 * 1024 * 1024),
        );
        graph.add_node(
            Node::new(
                "partition:/dev/disk/by-partuuid/root",
                NodeKind::Partition,
                "/dev/disk/by-partuuid/root",
            )
            .with_path("/dev/disk/by-partuuid/root")
            .with_size_bytes(32 * 1024 * 1024 * 1024),
        );
        graph.add_node(
            Node::new(
                "partition:/dev/disk/by-partuuid/scratch",
                NodeKind::Partition,
                "/dev/disk/by-partuuid/scratch",
            )
            .with_path("/dev/disk/by-partuuid/scratch"),
        );
        graph.add_node(
            Node::new(
                "block:/dev/disk/by-partuuid/wrong",
                NodeKind::PhysicalDisk,
                "/dev/disk/by-partuuid/wrong",
            )
            .with_path("/dev/disk/by-partuuid/wrong"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 4);
        assert_eq!(comparison.summary.already_satisfied_count, 2);
        assert_eq!(comparison.summary.suppressed_action_count, 2);
        assert_eq!(plan.summary.action_count, 2);
        assert!(
            plan.actions
                .iter()
                .all(|action| action.id != "partitions:boot:create")
        );
        assert!(
            plan.actions
                .iter()
                .all(|action| action.id != "partitions:scratch:create")
        );
        assert!(plan.actions.iter().any(|action| {
            action.id == "partitions:root:create" && action.operation == Operation::Create
        }));
        assert!(plan.actions.iter().any(|action| {
            action.id == "partitions:wrong:create" && action.operation == Operation::Create
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "partitions:boot:create"
                && diagnostic.kind == TopologyDiagnosticKind::PartitionCreateAlreadySatisfied
                && diagnostic.message.contains("desired size 1GiB")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "partitions:scratch:create"
                && diagnostic.kind == TopologyDiagnosticKind::PartitionCreateAlreadySatisfied
                && diagnostic.message.contains("already exists")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "partitions:root:create"
                && diagnostic.kind == TopologyDiagnosticKind::PartitionCreateRequired
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.message.contains("not desired size 64GiB")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "partitions:wrong:create"
                && diagnostic.kind == TopologyDiagnosticKind::PartitionCreateRequired
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.message.contains("not a partition")
        }));
    }

    #[test]
    fn topology_comparison_reconciles_disk_create_from_partition_table() {
        let plan = plan_from_json_bytes(
            br#"{
              "disks": {
                "/dev/disk/by-id/system": {
                  "operation": "create",
                  "partitionType": "gpt"
                },
                "/dev/disk/by-id/default-gpt": {
                  "operation": "create"
                },
                "/dev/disk/by-id/legacy": {
                  "operation": "create",
                  "partitionType": "gpt"
                },
                "/dev/disk/by-id/unknown": {
                  "operation": "create",
                  "partitionType": "gpt"
                },
                "/dev/disk/by-id/wrong": {
                  "operation": "create",
                  "partitionType": "gpt"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "block:/dev/disk/by-id/system",
                NodeKind::PhysicalDisk,
                "/dev/disk/by-id/system",
            )
            .with_path("/dev/disk/by-id/system")
            .with_property("partition.table", "gpt"),
        );
        graph.add_node(
            Node::new(
                "block:/dev/disk/by-id/default-gpt",
                NodeKind::PhysicalDisk,
                "/dev/disk/by-id/default-gpt",
            )
            .with_path("/dev/disk/by-id/default-gpt")
            .with_property("partition.table", "gpt"),
        );
        graph.add_node(
            Node::new(
                "block:/dev/disk/by-id/legacy",
                NodeKind::PhysicalDisk,
                "/dev/disk/by-id/legacy",
            )
            .with_path("/dev/disk/by-id/legacy")
            .with_property("partition.table", "msdos"),
        );
        graph.add_node(
            Node::new(
                "block:/dev/disk/by-id/unknown",
                NodeKind::PhysicalDisk,
                "/dev/disk/by-id/unknown",
            )
            .with_path("/dev/disk/by-id/unknown"),
        );
        graph.add_node(
            Node::new(
                "partition:/dev/disk/by-id/wrong",
                NodeKind::Partition,
                "/dev/disk/by-id/wrong",
            )
            .with_path("/dev/disk/by-id/wrong"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 5);
        assert_eq!(comparison.summary.already_satisfied_count, 2);
        assert_eq!(comparison.summary.suppressed_action_count, 2);
        assert_eq!(plan.summary.action_count, 3);
        assert!(
            plan.actions
                .iter()
                .all(|action| action.id != "disks:/dev/disk/by-id/system:create")
        );
        assert!(
            plan.actions
                .iter()
                .all(|action| action.id != "disks:/dev/disk/by-id/default-gpt:create")
        );
        assert!(plan.actions.iter().any(|action| {
            action.id == "disks:/dev/disk/by-id/legacy:create"
                && action.operation == Operation::Create
        }));
        assert!(plan.actions.iter().any(|action| {
            action.id == "disks:/dev/disk/by-id/unknown:create"
                && action.operation == Operation::Create
        }));
        assert!(plan.actions.iter().any(|action| {
            action.id == "disks:/dev/disk/by-id/wrong:create"
                && action.operation == Operation::Create
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "disks:/dev/disk/by-id/system:create"
                && diagnostic.kind == TopologyDiagnosticKind::DiskCreateAlreadySatisfied
                && diagnostic.message.contains("partition table gpt")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "disks:/dev/disk/by-id/default-gpt:create"
                && diagnostic.kind == TopologyDiagnosticKind::DiskCreateAlreadySatisfied
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "disks:/dev/disk/by-id/legacy:create"
                && diagnostic.kind == TopologyDiagnosticKind::DiskCreateRequired
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.message.contains("partition table msdos")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "disks:/dev/disk/by-id/unknown:create"
                && diagnostic.kind == TopologyDiagnosticKind::DiskCreateRequired
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic
                    .message
                    .contains("current partition table is unknown")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "disks:/dev/disk/by-id/wrong:create"
                && diagnostic.kind == TopologyDiagnosticKind::DiskCreateRequired
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.message.contains("not a physical disk")
        }));
    }

    #[test]
    fn topology_comparison_reconciles_lvm_physical_volume_create() {
        let plan = plan_from_json_bytes(
            br#"{
              "physicalVolumes": {
                "/dev/disk/by-id/pv-present": {
                  "operation": "create"
                },
                "/dev/disk/by-id/plain-device": {
                  "operation": "create"
                },
                "/dev/disk/by-id/duplicate-pv": {
                  "operation": "create"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "block:/dev/disk/by-id/pv-present",
                NodeKind::PhysicalDisk,
                "/dev/disk/by-id/pv-present",
            )
            .with_path("/dev/disk/by-id/pv-present"),
        );
        graph.add_node(
            Node::new(
                "lvm-pv:/dev/disk/by-id/pv-present",
                NodeKind::LvmPhysicalVolume,
                "/dev/disk/by-id/pv-present",
            )
            .with_path("/dev/disk/by-id/pv-present"),
        );
        graph.add_node(
            Node::new(
                "block:/dev/disk/by-id/plain-device",
                NodeKind::PhysicalDisk,
                "/dev/disk/by-id/plain-device",
            )
            .with_path("/dev/disk/by-id/plain-device"),
        );
        graph.add_node(
            Node::new(
                "lvm-pv:/dev/disk/by-id/duplicate-pv",
                NodeKind::LvmPhysicalVolume,
                "/dev/disk/by-id/duplicate-pv",
            )
            .with_path("/dev/disk/by-id/duplicate-pv")
            .with_property("lvm.pv-duplicate", "duplicate"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 3);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert_eq!(plan.summary.action_count, 2);
        assert!(
            plan.actions
                .iter()
                .all(|action| { action.id != "physicalvolumes:/dev/disk/by-id/pv-present:create" })
        );
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "physicalvolumes:/dev/disk/by-id/pv-present:create"
                && diagnostic.kind == TopologyDiagnosticKind::LvmPvCreateAlreadySatisfied
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "physicalvolumes:/dev/disk/by-id/plain-device:create"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::LvmPvCreateRequired
                && diagnostic.message.contains("not an LVM physical volume")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "physicalvolumes:/dev/disk/by-id/duplicate-pv:create"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::LvmPvCreateRequired
                && diagnostic.message.contains("lvm.pv-duplicate=duplicate")
        }));
    }

    #[test]
    fn plan_classifies_device_mapper_lifecycle() {
        let plan = plan_from_json_bytes(
            br#"{
              "dmMaps": {
                "cryptroot": {
                  "operation": "rescan",
                  "target": "/dev/mapper/cryptroot"
                },
                "cryptswap": {
                  "operation": "rename",
                  "target": "/dev/mapper/cryptswap",
                  "renameTo": "cryptswap-retired"
                },
                "oldmap": {
                  "operation": "destroy",
                  "target": "/dev/mapper/oldmap"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 3);
        assert_eq!(plan.summary.offline_required_count, 1);
        assert_eq!(plan.summary.destructive_count, 1);
        let rescan = plan
            .actions
            .iter()
            .find(|action| action.id == "dmmaps:cryptroot:rescan")
            .expect("device-mapper rescan action exists");
        assert_eq!(rescan.operation, Operation::Rescan);
        assert_eq!(rescan.risk, RiskClass::Online);
        assert_eq!(
            rescan.context.target.as_deref(),
            Some("/dev/mapper/cryptroot")
        );
        assert!(!rescan.destructive);
        assert!(
            rescan
                .advice
                .as_ref()
                .is_some_and(|advice| advice.summary.contains("device-mapper rescan"))
        );
        let rename = plan
            .actions
            .iter()
            .find(|action| action.id == "dmmaps:cryptswap:rename")
            .expect("device-mapper rename action exists");
        assert_eq!(rename.operation, Operation::Rename);
        assert_eq!(rename.risk, RiskClass::OfflineRequired);
        assert_eq!(
            rename.context.target.as_deref(),
            Some("/dev/mapper/cryptswap")
        );
        assert_eq!(
            rename.context.rename_to.as_deref(),
            Some("cryptswap-retired")
        );
        assert!(!rename.destructive);
        let destroy = plan
            .actions
            .iter()
            .find(|action| action.id == "dmmaps:oldmap:destroy")
            .expect("device-mapper destroy action exists");
        assert_eq!(destroy.operation, Operation::Destroy);
        assert_eq!(destroy.risk, RiskClass::Destructive);
        assert_eq!(
            destroy.context.target.as_deref(),
            Some("/dev/mapper/oldmap")
        );
        assert!(destroy.destructive);
        assert!(
            destroy
                .advice
                .as_ref()
                .is_some_and(|advice| advice.summary.contains("device-mapper removal"))
        );
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
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert_eq!(plan.summary.action_count, 1);
        assert!(
            plan.actions
                .iter()
                .all(|action| action.id != "datasets:tank/home:set-property:compression")
        );
        assert!(plan.actions.iter().any(|action| {
            action.id == "filesystem:home:grow" && action.operation == Operation::Grow
        }));
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
    fn topology_comparison_reports_matching_filesystem_format_type() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "data": {
                  "mountpoint": "/data",
                  "device": "/dev/disk/by-label/data",
                  "fsType": "ext4",
                  "preserveData": false
                },
                "legacy": {
                  "mountpoint": "/legacy",
                  "device": "/dev/disk/by-label/legacy",
                  "fsType": "xfs",
                  "preserveData": false
                },
                "small": {
                  "mountpoint": "/small",
                  "device": "/dev/disk/by-label/small",
                  "fsType": "ext4",
                  "desiredSize": "2GiB",
                  "preserveData": false
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("filesystem:/data", NodeKind::Filesystem, "/data")
                .with_path("/data")
                .with_property("filesystem.type", "ext4"),
        );
        graph.add_node(
            Node::new("filesystem:/legacy", NodeKind::Filesystem, "/legacy")
                .with_path("/legacy")
                .with_property("filesystem.type", "ext4"),
        );
        graph.add_node(
            Node::new("filesystem:/small", NodeKind::Filesystem, "/small")
                .with_path("/small")
                .with_size_bytes(1024 * 1024 * 1024)
                .with_property("filesystem.type", "ext4"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 6);
        assert_eq!(comparison.summary.matched_count, 6);
        assert_eq!(comparison.summary.type_conflict_count, 2);
        assert_eq!(comparison.summary.already_satisfied_count, 2);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.summary.action_count, 6);
        assert!(plan.actions.iter().any(|action| {
            action.id == "filesystem:data:preserve-data-disabled"
                && action.operation == Operation::Format
        }));
        assert!(plan.actions.iter().any(|action| {
            action.id == "filesystem:legacy:preserve-data-disabled"
                && action.operation == Operation::Format
        }));
        assert!(plan.actions.iter().any(|action| {
            action.id == "filesystem:small:preserve-data-disabled"
                && action.operation == Operation::Format
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "filesystem:data:preserve-data-disabled"
                && diagnostic.kind == TopologyDiagnosticKind::FilesystemFormatAlreadySatisfied
                && diagnostic.message.contains("type ext4")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "filesystem:legacy:preserve-data-disabled"
                && diagnostic.kind == TopologyDiagnosticKind::FilesystemTypeConflict
                && diagnostic.level == TopologyDiagnosticLevel::Warning
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "filesystem:small:preserve-data-disabled"
                && diagnostic.kind == TopologyDiagnosticKind::FilesystemFormatAlreadySatisfied
                && diagnostic.message.contains("type ext4")
        }));
    }

    #[test]
    fn topology_comparison_suppresses_already_mounted_sources() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "backup": {
                  "operation": "mount",
                  "mountpoint": "/backup",
                  "device": "/dev/disk/by-label/backup",
                  "fsType": "xfs"
                }
              },
              "nfs": {
                "mounts": {
                  "/srv/shared": {
                    "operation": "mount",
                    "source": "nas.example.com:/srv/shared",
                    "fsType": "nfs4"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("mount:/backup", NodeKind::Mountpoint, "/backup")
                .with_property("mount.source", "/dev/disk/by-label/backup"),
        );
        graph.add_node(
            Node::new("mount:/srv/shared", NodeKind::NfsMount, "/srv/shared")
                .with_property("mount.source", "nas.example.com:/srv/shared"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 3);
        assert_eq!(comparison.summary.already_satisfied_count, 2);
        assert_eq!(comparison.summary.suppressed_action_count, 2);
        assert!(
            plan.actions
                .iter()
                .all(|action| action.id != "filesystems:backup:mount")
        );
        assert!(
            plan.actions
                .iter()
                .all(|action| action.id != "nfs.mounts:/srv/shared:mount")
        );
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "filesystems:backup:mount"
                && diagnostic.kind == TopologyDiagnosticKind::MountAlreadySatisfied
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "nfs.mounts:/srv/shared:mount"
                && diagnostic.kind == TopologyDiagnosticKind::MountAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_mount_action_when_source_differs() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "backup": {
                  "operation": "mount",
                  "mountpoint": "/backup",
                  "device": "/dev/disk/by-label/backup",
                  "fsType": "xfs"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("mount:/backup", NodeKind::Mountpoint, "/backup")
                .with_property("mount.source", "/dev/disk/by-label/other"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 2);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert!(
            plan.actions
                .iter()
                .any(|action| action.id == "filesystems:backup:mount")
        );
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "filesystems:backup:mount"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::MountSourceConflict
        }));
    }

    #[test]
    fn topology_comparison_suppresses_unmount_when_mountpoint_absent() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "archive": {
                  "operation": "unmount",
                  "mountpoint": "/archive"
                }
              },
              "nfs": {
                "mounts": {
                  "/srv/old": {
                    "operation": "unmount",
                    "source": "nas.example.com:/srv/old"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let graph = StorageGraph::empty();

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 3);
        assert_eq!(comparison.summary.missing_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 2);
        assert_eq!(comparison.summary.suppressed_action_count, 2);
        assert!(
            plan.actions
                .iter()
                .all(|action| action.id != "filesystems:archive:unmount")
        );
        assert!(
            plan.actions
                .iter()
                .all(|action| action.id != "nfs.mounts:/srv/old:unmount")
        );
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "filesystems:archive:unmount"
                && diagnostic.current.is_none()
                && diagnostic.kind == TopologyDiagnosticKind::UnmountAlreadySatisfied
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "nfs.mounts:/srv/old:unmount"
                && diagnostic.current.is_none()
                && diagnostic.kind == TopologyDiagnosticKind::UnmountAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_unmount_when_mountpoint_exists() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "archive": {
                  "operation": "unmount",
                  "mountpoint": "/archive"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("mount:/archive", NodeKind::Mountpoint, "/archive")
                .with_property("mount.source", "/dev/disk/by-label/archive"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 2);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert!(
            plan.actions
                .iter()
                .any(|action| action.id == "filesystems:archive:unmount")
        );
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "filesystems:archive:unmount"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::UnmountRequired
        }));
    }

    #[test]
    fn topology_comparison_suppresses_inactive_swap_teardown() {
        let plan = plan_from_json_bytes(
            br#"{
              "swaps": {
                "old-file": {
                  "path": "/swapfile.old",
                  "operation": "deactivate"
                },
                "old-device": {
                  "device": "/dev/disk/by-label/old-swap",
                  "operation": "destroy"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let graph = StorageGraph::empty();

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 2);
        assert_eq!(comparison.summary.already_satisfied_count, 2);
        assert_eq!(comparison.summary.suppressed_action_count, 2);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "swaps:old-file:deactivate"
                && diagnostic.current.is_none()
                && diagnostic.kind == TopologyDiagnosticKind::SwapDeactivateAlreadySatisfied
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "swaps:old-device:destroy"
                && diagnostic.current.is_none()
                && diagnostic.kind == TopologyDiagnosticKind::SwapDestroyAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_active_swap_teardown() {
        let plan = plan_from_json_bytes(
            br#"{
              "swaps": {
                "scratch": {
                  "path": "/swapfile",
                  "operation": "deactivate"
                },
                "remove": {
                  "device": "/dev/disk/by-label/remove-swap",
                  "operation": "destroy"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("swap:/swapfile", NodeKind::Swap, "/swapfile")
                .with_path("/swapfile")
                .with_size_bytes(1_073_741_824)
                .with_usage(Usage {
                    used_bytes: Some(134_217_728),
                    free_bytes: Some(939_524_096),
                    allocated_bytes: Some(1_073_741_824),
                })
                .with_property("swap.active", "true")
                .with_property("swap.type", "file")
                .with_property("swap.priority", "10"),
        );
        graph.add_node(
            Node::new(
                "swap:/dev/disk/by-label/remove-swap",
                NodeKind::Swap,
                "/dev/disk/by-label/remove-swap",
            )
            .with_path("/dev/disk/by-label/remove-swap")
            .with_property("swap.active", "true")
            .with_property("swap.type", "partition"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 2);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 2);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "swaps:scratch:deactivate"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::SwapDeactivateRequired
                && diagnostic.message.contains("priority 10")
                && diagnostic.message.contains("type file")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "swaps:remove:destroy"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::SwapDestroyRequired
                && diagnostic.message.contains("type partition")
        }));
    }

    #[test]
    fn topology_comparison_reports_swap_format_target_metadata() {
        let plan = plan_from_json_bytes(
            br#"{
              "swaps": {
                "scratch": {
                  "path": "/swapfile",
                  "operation": "format"
                },
                "device": {
                  "device": "/dev/disk/by-label/swap",
                  "operation": "format"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("swap:/swapfile", NodeKind::Swap, "/swapfile")
                .with_path("/swapfile")
                .with_size_bytes(2_147_483_648)
                .with_usage(Usage {
                    used_bytes: Some(268_435_456),
                    free_bytes: Some(1_879_048_192),
                    allocated_bytes: Some(2_147_483_648),
                })
                .with_property("swap.active", "true")
                .with_property("swap.type", "file")
                .with_property("swap.priority", "5"),
        );
        graph.add_node(
            Node::new(
                "filesystem:/dev/disk/by-label/swap",
                NodeKind::Filesystem,
                "/dev/disk/by-label/swap",
            )
            .with_path("/dev/disk/by-label/swap")
            .with_property("filesystem.type", "ext4"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 2);
        assert_eq!(comparison.summary.matched_count, 2);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.summary.action_count, 2);
        assert!(plan.actions.iter().any(|action| {
            action.id == "swaps:scratch:format" && action.operation == Operation::Format
        }));
        assert!(plan.actions.iter().any(|action| {
            action.id == "swaps:device:format" && action.operation == Operation::Format
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "swaps:scratch:format"
                && diagnostic.query == "/swapfile"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::SwapFormatTargetPresent
                && diagnostic.message.contains("size 2147483648 bytes")
                && diagnostic.message.contains("used 268435456 bytes")
                && diagnostic.message.contains("priority 5")
                && diagnostic.message.contains("type file")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "swaps:device:format"
                && diagnostic.query == "/dev/disk/by-label/swap"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::SwapFormatTargetPresent
                && diagnostic.message.contains("filesystem")
        }));
    }

    #[test]
    fn topology_comparison_suppresses_remount_when_options_are_present() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "scratch": {
                  "operation": "remount",
                  "mountpoint": "/scratch",
                  "options": ["rw", "noatime", "discard=async"]
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("mount:/scratch", NodeKind::Mountpoint, "/scratch")
                .with_property("mount.options", "rw,relatime,noatime,discard=async"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 2);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(
            plan.actions
                .iter()
                .all(|action| action.id != "filesystems:scratch:remount")
        );
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "filesystems:scratch:remount"
                && diagnostic.kind == TopologyDiagnosticKind::MountOptionsAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_suppresses_nfs_remount_from_nfs_option_properties() {
        let plan = plan_from_json_bytes(
            br#"{
              "nfs": {
                "mounts": {
                  "/srv/shared": {
                    "operation": "remount",
                    "options": ["rw", "vers=4.2", "_netdev"]
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("mount:/srv/shared", NodeKind::NfsMount, "/srv/shared")
                .with_property("nfs.rw", "true")
                .with_property("nfs.vers", "4.2")
                .with_property("nfs.netdev", "true"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "nfs.mounts:/srv/shared:remount"
                && diagnostic.kind == TopologyDiagnosticKind::MountOptionsAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_remount_when_options_differ() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "scratch": {
                  "operation": "remount",
                  "mountpoint": "/scratch",
                  "options": ["ro", "noatime"]
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("mount:/scratch", NodeKind::Mountpoint, "/scratch")
                .with_property("mount.options", "rw,relatime"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 2);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert!(
            plan.actions
                .iter()
                .any(|action| action.id == "filesystems:scratch:remount")
        );
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "filesystems:scratch:remount"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::MountOptionsDiffer
        }));
    }

    #[test]
    fn topology_comparison_suppresses_already_exported_nfs_path() {
        let plan = plan_from_json_bytes(
            br#"{
              "exports": {
                "/srv/share": {
                  "operation": "export",
                  "client": "192.0.2.0/24",
                  "options": "rw,sync,no_subtree_check"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "nfs-export:/srv/share:192.0.2.0/24",
                NodeKind::NfsExport,
                "/srv/share",
            )
            .with_property("nfs.export", "/srv/share")
            .with_property("nfs.export-client", "192.0.2.0/24")
            .with_property("nfs.exportfs", "true")
            .with_property("nfs.export-option-rw", "true")
            .with_property("nfs.export-option-sync", "true")
            .with_property("nfs.export-option-no-subtree-check", "true"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "exports:/srv/share:export"
                && diagnostic.kind == TopologyDiagnosticKind::NfsExportAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_nfs_export_when_client_or_options_differ() {
        let plan = plan_from_json_bytes(
            br#"{
              "exports": {
                "/srv/share": {
                  "operation": "export",
                  "client": "192.0.2.0/24",
                  "options": "rw,sync,no_subtree_check"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "nfs-export:/srv/share:198.51.100.10",
                NodeKind::NfsExport,
                "/srv/share",
            )
            .with_property("nfs.export", "/srv/share")
            .with_property("nfs.export-client", "198.51.100.10")
            .with_property("nfs.exportfs", "true")
            .with_property("nfs.export-option-ro", "true")
            .with_property("nfs.export-option-sync", "true"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "exports:/srv/share:export"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::NfsExportDiffers
        }));
    }

    #[test]
    fn topology_comparison_suppresses_absent_nfs_unexport() {
        let plan = plan_from_json_bytes(
            br#"{
              "exports": {
                "/srv/old": {
                  "operation": "unexport",
                  "client": "192.0.2.0/24"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let graph = StorageGraph::empty();

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.missing_count, 0);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "exports:/srv/old:unexport"
                && diagnostic.current.is_none()
                && diagnostic.kind == TopologyDiagnosticKind::NfsUnexportAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_nfs_unexport_when_export_exists() {
        let plan = plan_from_json_bytes(
            br#"{
              "exports": {
                "/srv/old": {
                  "operation": "unexport",
                  "client": "192.0.2.0/24"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "nfs-export:/srv/old:192.0.2.0/24",
                NodeKind::NfsExport,
                "/srv/old",
            )
            .with_property("nfs.export", "/srv/old")
            .with_property("nfs.export-client", "192.0.2.0/24")
            .with_property("nfs.exportfs", "true"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "exports:/srv/old:unexport"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::NfsUnexportRequired
        }));
    }

    #[test]
    fn topology_comparison_reports_luks_format_target_metadata() {
        let plan = plan_from_json_bytes(
            br#"{
              "luks": {
                "devices": {
                  "cryptroot": {
                    "operation": "format",
                    "device": "/dev/disk/by-partuuid/root",
                    "target": "cryptroot"
                  },
                  "cryptdata": {
                    "operation": "format",
                    "device": "/dev/disk/by-id/data",
                    "target": "cryptdata"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "block:/dev/disk/by-partuuid/root",
                NodeKind::LuksContainer,
                "/dev/disk/by-partuuid/root",
            )
            .with_path("/dev/disk/by-partuuid/root")
            .with_property("cryptsetup.luks-version", "2")
            .with_property("cryptsetup.uuid", "11111111-2222-3333-4444-555555555555")
            .with_property("cryptsetup.luks-keyslot-count", "2")
            .with_property("cryptsetup.luks-token-count", "1")
            .with_property("cryptsetup.active", "false"),
        );
        graph.add_node(
            Node::new(
                "block:/dev/disk/by-id/data",
                NodeKind::Partition,
                "/dev/disk/by-id/data",
            )
            .with_path("/dev/disk/by-id/data"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 2);
        assert_eq!(comparison.summary.matched_count, 2);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.summary.action_count, 2);
        assert!(plan.actions.iter().any(|action| {
            action.id == "luks.devices:cryptroot:format" && action.operation == Operation::Format
        }));
        assert!(plan.actions.iter().any(|action| {
            action.id == "luks.devices:cryptdata:format" && action.operation == Operation::Format
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "luks.devices:cryptroot:format"
                && diagnostic.query == "/dev/disk/by-partuuid/root"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::LuksFormatTargetPresent
                && diagnostic.message.contains("version 2")
                && diagnostic.message.contains("keyslots 2")
                && diagnostic.message.contains("tokens 1")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "luks.devices:cryptdata:format"
                && diagnostic.query == "/dev/disk/by-id/data"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::LuksFormatTargetPresent
                && diagnostic.message.contains("partition")
        }));
    }

    #[test]
    fn topology_comparison_suppresses_open_luks_mapper_when_active() {
        let plan = plan_from_json_bytes(
            br#"{
              "luks": {
                "devices": {
                  "cryptroot": {
                    "operation": "open",
                    "device": "/dev/disk/by-partuuid/root",
                    "target": "cryptroot"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "block:/dev/mapper/cryptroot",
                NodeKind::LuksContainer,
                "cryptroot",
            )
            .with_path("/dev/mapper/cryptroot")
            .with_property("cryptsetup.active", "true")
            .with_property("cryptsetup.in-use", "true"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "luks.devices:cryptroot:open"
                && diagnostic.kind == TopologyDiagnosticKind::LuksOpenAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_open_luks_mapper_when_inactive() {
        let plan = plan_from_json_bytes(
            br#"{
              "luks": {
                "devices": {
                  "cryptroot": {
                    "operation": "open",
                    "device": "/dev/disk/by-partuuid/root",
                    "target": "cryptroot"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "block:/dev/mapper/cryptroot",
                NodeKind::LuksContainer,
                "cryptroot",
            )
            .with_path("/dev/mapper/cryptroot")
            .with_property("cryptsetup.active", "false"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "luks.devices:cryptroot:open"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::LuksOpenRequired
        }));
    }

    #[test]
    fn topology_comparison_suppresses_close_luks_mapper_when_inactive() {
        let plan = plan_from_json_bytes(
            br#"{
              "luks": {
                "devices": {
                  "cryptroot": {
                    "operation": "close",
                    "target": "cryptroot"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "block:/dev/mapper/cryptroot",
                NodeKind::LuksContainer,
                "cryptroot",
            )
            .with_path("/dev/mapper/cryptroot")
            .with_property("cryptsetup.active", "false"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "luks.devices:cryptroot:close"
                && diagnostic.kind == TopologyDiagnosticKind::LuksCloseAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_close_luks_mapper_when_active() {
        let plan = plan_from_json_bytes(
            br#"{
              "luks": {
                "devices": {
                  "cryptroot": {
                    "operation": "close",
                    "target": "cryptroot"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "block:/dev/mapper/cryptroot",
                NodeKind::LuksContainer,
                "cryptroot",
            )
            .with_path("/dev/mapper/cryptroot")
            .with_property("cryptsetup.active", "true"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "luks.devices:cryptroot:close"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::LuksCloseRequired
        }));
    }

    #[test]
    fn topology_comparison_suppresses_luks_keyslot_remove_when_slot_absent() {
        let plan = plan_from_json_bytes(
            br#"{
              "luksKeyslots": {
                "cryptroot:2": {
                  "operation": "remove-key",
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "keySlot": "2"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "block:/dev/disk/by-id/root-luks",
                NodeKind::LuksContainer,
                "root-luks",
            )
            .with_path("/dev/disk/by-id/root-luks")
            .with_property("cryptsetup.luks-keyslots", "0,1")
            .with_property("cryptsetup.luks-keyslot-count", "2"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "lukskeyslots:cryptroot:2:remove-key"
                && diagnostic.kind == TopologyDiagnosticKind::LuksKeyslotRemoveAlreadySatisfied
                && diagnostic.query == "/dev/disk/by-id/root-luks"
        }));
    }

    #[test]
    fn topology_comparison_keeps_luks_keyslot_remove_when_slot_present() {
        let plan = plan_from_json_bytes(
            br#"{
              "luksKeyslots": {
                "cryptroot:2": {
                  "operation": "remove-key",
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "keySlot": "2"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "block:/dev/disk/by-id/root-luks",
                NodeKind::LuksContainer,
                "root-luks",
            )
            .with_path("/dev/disk/by-id/root-luks")
            .with_property("cryptsetup.luks-keyslots", "0,2")
            .with_property("cryptsetup.luks-keyslot-2-type", "luks2")
            .with_property("cryptsetup.luks-keyslot-2-priority", "normal")
            .with_property("cryptsetup.luks-keyslot-2-pbkdf", "argon2id")
            .with_property("cryptsetup.luks-keyslot-2-time-cost", "4"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "lukskeyslots:cryptroot:2:remove-key"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::LuksKeyslotRemoveRequired
                && diagnostic.message.contains("type luks2")
                && diagnostic.message.contains("priority normal")
                && diagnostic.message.contains("PBKDF argon2id")
        }));
    }

    #[test]
    fn topology_comparison_suppresses_luks_token_remove_when_token_absent() {
        let plan = plan_from_json_bytes(
            br#"{
              "luksTokens": {
                "cryptroot:3": {
                  "operation": "remove-token",
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "tokenId": "3"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "block:/dev/disk/by-id/root-luks",
                NodeKind::LuksContainer,
                "root-luks",
            )
            .with_path("/dev/disk/by-id/root-luks")
            .with_property("cryptsetup.luks-tokens", "0,1")
            .with_property("cryptsetup.luks-token-count", "2"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "lukstokens:cryptroot:3:remove-token"
                && diagnostic.kind == TopologyDiagnosticKind::LuksTokenRemoveAlreadySatisfied
                && diagnostic.query == "/dev/disk/by-id/root-luks"
        }));
    }

    #[test]
    fn topology_comparison_keeps_luks_token_remove_when_token_present() {
        let plan = plan_from_json_bytes(
            br#"{
              "luksTokens": {
                "cryptroot:3": {
                  "operation": "remove-token",
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "tokenId": "3"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "block:/dev/disk/by-id/root-luks",
                NodeKind::LuksContainer,
                "root-luks",
            )
            .with_path("/dev/disk/by-id/root-luks")
            .with_property("cryptsetup.luks-tokens", "1,3")
            .with_property("cryptsetup.luks-token-3-type", "systemd-tpm2")
            .with_property("cryptsetup.luks-token-3-keyslot", "2"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "lukstokens:cryptroot:3:remove-token"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::LuksTokenRemoveRequired
                && diagnostic.message.contains("type systemd-tpm2")
                && diagnostic.message.contains("keyslot 2")
        }));
    }

    #[test]
    fn topology_comparison_keeps_luks_keyslot_remove_missing_without_container() {
        let plan = plan_from_json_bytes(
            br#"{
              "luksKeyslots": {
                "cryptroot:2": {
                  "operation": "remove-key",
                  "device": "/dev/disk/by-id/root-luks",
                  "metadata": {
                    "keySlot": "2"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let graph = StorageGraph::empty();

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(comparison.summary.missing_count, 1);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "lukskeyslots:cryptroot:2:remove-key"
                && diagnostic.kind == TopologyDiagnosticKind::Missing
                && diagnostic.query == "/dev/disk/by-id/root-luks"
        }));
    }

    #[test]
    fn topology_comparison_suppresses_active_lvm_activate_action() {
        let plan = plan_from_json_bytes(
            br#"{
              "volumes": {
                "vg0/home": {
                  "operation": "activate"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("lvm:lv:vg0/home", NodeKind::LvmLogicalVolume, "vg0/home")
                .with_path("/dev/vg0/home")
                .with_property("lvm.active", "active")
                .with_property("lvm.active-locally", "active locally"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "volumes:vg0/home:activate"
                && diagnostic.kind == TopologyDiagnosticKind::LvmActivateAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_lvm_activate_action_when_inactive() {
        let plan = plan_from_json_bytes(
            br#"{
              "volumes": {
                "vg0/home": {
                  "operation": "activate"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("lvm:lv:vg0/home", NodeKind::LvmLogicalVolume, "vg0/home")
                .with_path("/dev/vg0/home")
                .with_property("lvm.active", "inactive"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "volumes:vg0/home:activate"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::LvmActivateRequired
        }));
    }

    #[test]
    fn topology_comparison_suppresses_lvm_deactivate_action_when_inactive() {
        let plan = plan_from_json_bytes(
            br#"{
              "volumes": {
                "vg0/archive": {
                  "operation": "deactivate"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "lvm:lv:vg0/archive",
                NodeKind::LvmLogicalVolume,
                "vg0/archive",
            )
            .with_path("/dev/vg0/archive")
            .with_property("lvm.active", "inactive"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "volumes:vg0/archive:deactivate"
                && diagnostic.kind == TopologyDiagnosticKind::LvmDeactivateAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_lvm_deactivate_action_when_active() {
        let plan = plan_from_json_bytes(
            br#"{
              "volumes": {
                "vg0/archive": {
                  "operation": "deactivate"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "lvm:lv:vg0/archive",
                NodeKind::LvmLogicalVolume,
                "vg0/archive",
            )
            .with_path("/dev/vg0/archive")
            .with_property("lvm.active", "active")
            .with_property("lvm.active-locally", "active locally"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "volumes:vg0/archive:deactivate"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::LvmDeactivateRequired
        }));
    }

    #[test]
    fn topology_comparison_reconciles_lvm_volume_and_thin_pool_create() {
        let plan = plan_from_json_bytes(
            br#"{
              "volumes": {
                "vg0/home": {
                  "operation": "create",
                  "desiredSize": "8GiB"
                },
                "vg0/archive": {
                  "operation": "create",
                  "desiredSize": "8GiB"
                }
              },
              "thinPools": {
                "vg0/pool": {
                  "operation": "create",
                  "desiredSize": "16GiB"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("lvm-lv:vg0/home", NodeKind::LvmLogicalVolume, "vg0/home")
                .with_path("/dev/vg0/home")
                .with_size_bytes(8 * 1024 * 1024 * 1024),
        );
        graph.add_node(
            Node::new(
                "lvm-lv:vg0/archive",
                NodeKind::LvmLogicalVolume,
                "vg0/archive",
            )
            .with_path("/dev/vg0/archive")
            .with_size_bytes(4 * 1024 * 1024 * 1024),
        );
        graph.add_node(
            Node::new("lvm-thin-pool:vg0/pool", NodeKind::LvmThinPool, "vg0/pool")
                .with_path("/dev/vg0/pool")
                .with_size_bytes(16 * 1024 * 1024 * 1024),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 3);
        assert_eq!(comparison.summary.already_satisfied_count, 2);
        assert_eq!(comparison.summary.suppressed_action_count, 2);
        assert_eq!(plan.summary.action_count, 1);
        assert!(
            plan.actions
                .iter()
                .all(|action| action.id == "volumes:vg0/archive:create")
        );
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "volumes:vg0/home:create"
                && diagnostic.kind == TopologyDiagnosticKind::LvmVolumeCreateAlreadySatisfied
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "thinpools:vg0/pool:create"
                && diagnostic.kind == TopologyDiagnosticKind::LvmVolumeCreateAlreadySatisfied
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "volumes:vg0/archive:create"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::LvmVolumeCreateRequired
                && diagnostic.message.contains("not desired size 8GiB")
                && diagnostic.message.contains("grow or shrink")
        }));
    }

    #[test]
    fn topology_comparison_suppresses_imported_lvm_volume_group() {
        let plan = plan_from_json_bytes(
            br#"{
              "volumeGroups": {
                "vg0": {
                  "operation": "import"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(Node::new("lvm-vg:vg0", NodeKind::LvmVolumeGroup, "vg0"));

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "volumegroups:vg0:import"
                && diagnostic.kind == TopologyDiagnosticKind::LvmVgImportAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_reconciles_lvm_volume_group_create() {
        let plan = plan_from_json_bytes(
            br#"{
              "volumeGroups": {
                "vg-present": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/pv-present"
                },
                "vg-exported": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/pv-exported"
                },
                "vg-partial": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/pv-partial"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(Node::new(
            "lvm-vg:vg-present",
            NodeKind::LvmVolumeGroup,
            "vg-present",
        ));
        graph.add_node(
            Node::new(
                "lvm-vg:vg-exported",
                NodeKind::LvmVolumeGroup,
                "vg-exported",
            )
            .with_property("lvm.vg-exported", "exported"),
        );
        graph.add_node(
            Node::new("lvm-vg:vg-partial", NodeKind::LvmVolumeGroup, "vg-partial")
                .with_property("lvm.vg-partial", "partial")
                .with_property("lvm.missing-pv-count", "1"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 3);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert_eq!(plan.summary.action_count, 2);
        assert!(
            plan.actions
                .iter()
                .all(|action| { action.id != "volumegroups:vg-present:create" })
        );
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "volumegroups:vg-present:create"
                && diagnostic.kind == TopologyDiagnosticKind::LvmVgCreateAlreadySatisfied
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "volumegroups:vg-exported:create"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::LvmVgCreateRequired
                && diagnostic.message.contains("lvm.vg-exported=exported")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "volumegroups:vg-partial:create"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::LvmVgCreateRequired
                && diagnostic.message.contains("lvm.vg-partial=partial")
                && diagnostic.message.contains("1 missing physical volume")
        }));
    }

    #[test]
    fn topology_comparison_keeps_lvm_volume_group_import_when_exported() {
        let plan = plan_from_json_bytes(
            br#"{
              "volumeGroups": {
                "vg0": {
                  "operation": "import"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("lvm-vg:vg0", NodeKind::LvmVolumeGroup, "vg0")
                .with_property("lvm.vg-exported", "exported"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "volumegroups:vg0:import"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::LvmVgImportRequired
        }));
    }

    #[test]
    fn topology_comparison_suppresses_exported_lvm_volume_group() {
        let plan = plan_from_json_bytes(
            br#"{
              "volumeGroups": {
                "vg0": {
                  "operation": "export"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("lvm-vg:vg0", NodeKind::LvmVolumeGroup, "vg0")
                .with_property("lvm.vg-exported", "exported"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "volumegroups:vg0:export"
                && diagnostic.kind == TopologyDiagnosticKind::LvmVgExportAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_lvm_volume_group_export_when_imported() {
        let plan = plan_from_json_bytes(
            br#"{
              "volumeGroups": {
                "vg0": {
                  "operation": "export"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(Node::new("lvm-vg:vg0", NodeKind::LvmVolumeGroup, "vg0"));

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "volumegroups:vg0:export"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::LvmVgExportRequired
        }));
    }

    #[test]
    fn topology_comparison_suppresses_lvm_cache_detach_when_origin_uncached() {
        let plan = plan_from_json_bytes(
            br#"{
              "lvmCaches": {
                "vg0/root": {
                  "removeDevices": ["vg0/root-cache"]
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(Node::new(
            "lvm-lv:vg0/root",
            NodeKind::LvmLogicalVolume,
            "vg0/root",
        ));

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "lvmCaches:vg0/root:remove-device:vg0/root-cache"
                && diagnostic.kind == TopologyDiagnosticKind::LvmCacheDetachAlreadySatisfied
                && diagnostic.query == "vg0/root"
        }));
    }

    #[test]
    fn topology_comparison_keeps_lvm_cache_detach_when_origin_cached() {
        let plan = plan_from_json_bytes(
            br#"{
              "lvmCaches": {
                "vg0/root": {
                  "removeDevices": ["vg0/root-cache"]
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("lvm-lv:vg0/root", NodeKind::LvmCache, "vg0/root")
                .with_property("lvm.pool", "root-cache")
                .with_property("lvm.cache-mode", "writeback")
                .with_property("lvm.cache-policy", "smq")
                .with_property("lvm.cache-dirty-blocks", "64")
                .with_property("lvm.data-percent", "12.00"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "lvmCaches:vg0/root:remove-device:vg0/root-cache"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::LvmCacheDetachRequired
                && diagnostic.message.contains("cache pool root-cache")
                && diagnostic.message.contains("cache mode writeback")
                && diagnostic.message.contains("dirty blocks 64")
        }));
    }

    #[test]
    fn topology_comparison_keeps_lvm_cache_detach_missing_without_origin() {
        let plan = plan_from_json_bytes(
            br#"{
              "lvmCaches": {
                "vg0/root": {
                  "removeDevices": ["vg0/root-cache"]
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let graph = StorageGraph::empty();

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(comparison.summary.missing_count, 1);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "lvmCaches:vg0/root:remove-device:vg0/root-cache"
                && diagnostic.kind == TopologyDiagnosticKind::Missing
                && diagnostic.query == "vg0/root"
        }));
    }

    #[test]
    fn topology_comparison_suppresses_vdo_start_when_normal() {
        let plan = plan_from_json_bytes(
            br#"{
              "vdoVolumes": {
                "archive": {
                  "operation": "start"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("vdo:archive", NodeKind::VdoVolume, "archive")
                .with_path("/dev/mapper/archive")
                .with_property("vdo.operating-mode", "normal"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "vdovolumes:archive:start"
                && diagnostic.kind == TopologyDiagnosticKind::VdoStartAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_suppresses_lvm_vdo_start_when_normal() {
        let plan = plan_from_json_bytes(
            br#"{
              "vdoVolumes": {
                "vg0/archive": {
                  "operation": "start"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("lvm:vg0/archive", NodeKind::VdoVolume, "vg0/archive")
                .with_path("/dev/vg0/archive")
                .with_property("lvm.vdo-operating-mode", "normal"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "vdovolumes:vg0/archive:start"
                && diagnostic.kind == TopologyDiagnosticKind::VdoStartAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_vdo_start_when_not_normal() {
        let plan = plan_from_json_bytes(
            br#"{
              "vdoVolumes": {
                "archive": {
                  "operation": "start"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("vdo:archive", NodeKind::VdoVolume, "archive")
                .with_path("/dev/mapper/archive")
                .with_property("vdo.operating-mode", "recovering"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "vdovolumes:archive:start"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::VdoStartRequired
        }));
    }

    #[test]
    fn topology_comparison_suppresses_vdo_stop_when_stopped() {
        let plan = plan_from_json_bytes(
            br#"{
              "vdoVolumes": {
                "archive": {
                  "operation": "stop"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("vdo:archive", NodeKind::VdoVolume, "archive")
                .with_path("/dev/mapper/archive")
                .with_property("vdo.operating-mode", "stopped"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "vdovolumes:archive:stop"
                && diagnostic.kind == TopologyDiagnosticKind::VdoStopAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_suppresses_lvm_vdo_stop_when_not_running() {
        let plan = plan_from_json_bytes(
            br#"{
              "vdoVolumes": {
                "vg0/archive": {
                  "operation": "stop"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("lvm:vg0/archive", NodeKind::VdoVolume, "vg0/archive")
                .with_path("/dev/vg0/archive")
                .with_property("lvm.vdo-operating-mode", "not running"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "vdovolumes:vg0/archive:stop"
                && diagnostic.kind == TopologyDiagnosticKind::VdoStopAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_reports_vdo_create_target_metadata() {
        let plan = plan_from_json_bytes(
            br#"{
              "vdoVolumes": {
                "archive": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/vdo-backing",
                  "desiredSize": "2TiB"
                },
                "data": {
                  "operation": "create",
                  "target": "/dev/disk/by-label/data"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("vdo:archive", NodeKind::VdoVolume, "archive")
                .with_path("/dev/mapper/archive")
                .with_property("vdo.operating-mode", "normal")
                .with_property("vdo.storage-device", "/dev/disk/by-id/vdo-backing")
                .with_property("vdo.logical-size", "2TiB")
                .with_property("vdo.write-policy", "sync"),
        );
        graph.add_node(
            Node::new(
                "filesystem:/dev/disk/by-label/data",
                NodeKind::Filesystem,
                "/dev/disk/by-label/data",
            )
            .with_path("/dev/disk/by-label/data")
            .with_property("filesystem.type", "xfs"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 2);
        assert_eq!(comparison.summary.matched_count, 2);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 2);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "vdovolumes:archive:create"
                && diagnostic.query == "archive"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::VdoCreateTargetPresent
                && diagnostic.message.contains("operating mode normal")
                && diagnostic
                    .message
                    .contains("backing device /dev/disk/by-id/vdo-backing")
                && diagnostic.message.contains("logical size 2TiB")
                && diagnostic.message.contains("write policy sync")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "vdovolumes:data:create"
                && diagnostic.query == "/dev/disk/by-label/data"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::VdoCreateTargetPresent
                && diagnostic.message.contains("filesystem")
        }));
    }

    #[test]
    fn topology_comparison_keeps_vdo_stop_when_normal() {
        let plan = plan_from_json_bytes(
            br#"{
              "vdoVolumes": {
                "archive": {
                  "operation": "stop"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("vdo:archive", NodeKind::VdoVolume, "archive")
                .with_path("/dev/mapper/archive")
                .with_property("vdo.operating-mode", "normal"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "vdovolumes:archive:stop"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::VdoStopRequired
        }));
    }

    #[test]
    fn topology_comparison_suppresses_vdo_destroy_when_absent() {
        let plan = plan_from_json_bytes(
            br#"{
              "vdoVolumes": {
                "archive": {
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let graph = StorageGraph::empty();

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "vdovolumes:archive:destroy"
                && diagnostic.kind == TopologyDiagnosticKind::VdoDestroyAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_vdo_destroy_when_present() {
        let plan = plan_from_json_bytes(
            br#"{
              "vdoVolumes": {
                "archive": {
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("vdo:archive", NodeKind::VdoVolume, "archive")
                .with_path("/dev/mapper/archive")
                .with_property("vdo.operating-mode", "normal")
                .with_property("vdo.storage-device", "/dev/sdb")
                .with_property("vdo.logical-size", "4TiB")
                .with_property("vdo.physical-size", "1TiB")
                .with_property("vdo.write-policy", "sync"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "vdovolumes:archive:destroy"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::VdoDestroyRequired
                && diagnostic.message.contains("operating mode normal")
                && diagnostic.message.contains("backing device /dev/sdb")
                && diagnostic.message.contains("logical size 4TiB")
                && diagnostic.message.contains("write policy sync")
        }));
    }

    #[test]
    fn topology_comparison_reports_lvm_vdo_destroy_metadata() {
        let plan = plan_from_json_bytes(
            br#"{
              "vdoVolumes": {
                "vg0/archive": {
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("lvm:vg0/archive", NodeKind::VdoVolume, "vg0/archive")
                .with_property("lvm.vdo-operating-mode", "normal")
                .with_property("lvm.vdo-used-size", "128.00m")
                .with_property("lvm.vdo-saving-percent", "72.50"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "vdovolumes:vg0/archive:destroy"
                && diagnostic.kind == TopologyDiagnosticKind::VdoDestroyRequired
                && diagnostic.message.contains("used 128.00m")
                && diagnostic.message.contains("saving 72.50")
        }));
    }

    #[test]
    fn topology_comparison_reconciles_vdo_grow_from_logical_size_metadata() {
        let plan = plan_from_json_bytes(
            br#"{
              "vdoVolumes": {
                "archive": {
                  "operation": "grow",
                  "desiredSize": "2TiB"
                },
                "small": {
                  "operation": "grow",
                  "desiredSize": "4TiB"
                },
                "unknown": {
                  "operation": "grow",
                  "desiredSize": "4TiB"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("vdo:archive", NodeKind::VdoVolume, "archive")
                .with_path("/dev/mapper/archive")
                .with_property("vdo.logical-size", "4TiB"),
        );
        graph.add_node(
            Node::new("vdo:small", NodeKind::VdoVolume, "small")
                .with_path("/dev/mapper/small")
                .with_property("vdo.logical-size", "1TiB"),
        );
        graph.add_node(
            Node::new("vdo:unknown", NodeKind::VdoVolume, "unknown")
                .with_path("/dev/mapper/unknown"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 3);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert_eq!(plan.summary.action_count, 2);
        assert!(
            plan.actions
                .iter()
                .all(|action| action.id != "vdovolumes:archive:grow")
        );
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "vdovolumes:archive:grow"
                && diagnostic.level == TopologyDiagnosticLevel::Info
                && diagnostic.kind == TopologyDiagnosticKind::SizeAlreadySatisfied
                && diagnostic
                    .message
                    .contains("logical size 4TiB already satisfies desired size 2TiB")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "vdovolumes:small:grow"
                && diagnostic.level == TopologyDiagnosticLevel::Info
                && diagnostic.kind == TopologyDiagnosticKind::SizeBelowDesired
                && diagnostic
                    .message
                    .contains("logical size 1TiB is below desired size 4TiB")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "vdovolumes:unknown:grow"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::VdoGrowRequired
                && diagnostic
                    .message
                    .contains("current logical size is unknown")
        }));
    }

    #[test]
    fn topology_comparison_suppresses_md_assemble_when_clean() {
        let plan = plan_from_json_bytes(
            br#"{
              "mdRaids": {
                "existing": {
                  "operation": "assemble",
                  "target": "/dev/md/existing",
                  "devices": ["/dev/sdb1", "/dev/sdc1"]
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("md:/dev/md/existing", NodeKind::MdRaid, "/dev/md/existing")
                .with_path("/dev/md/existing")
                .with_property("md.state", "clean")
                .with_property("md.degraded-devices", "0")
                .with_property("md.failed-devices", "0"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "mdraids:existing:assemble"
                && diagnostic.kind == TopologyDiagnosticKind::MdAssembleAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_reconciles_md_create() {
        let plan = plan_from_json_bytes(
            br#"{
              "mdRaids": {
                "existing": {
                  "operation": "create",
                  "target": "/dev/md/existing",
                  "level": "1",
                  "devices": ["/dev/sdb1", "/dev/sdc1"]
                },
                "degraded": {
                  "operation": "create",
                  "target": "/dev/md/degraded",
                  "level": "1",
                  "devices": ["/dev/sdd1", "/dev/sde1"]
                },
                "wrong-kind": {
                  "operation": "create",
                  "target": "/dev/md/wrong-kind",
                  "level": "1",
                  "devices": ["/dev/sdf1", "/dev/sdg1"]
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("md:/dev/md/existing", NodeKind::MdRaid, "/dev/md/existing")
                .with_path("/dev/md/existing")
                .with_property("md.state", "clean")
                .with_property("md.degraded-devices", "0")
                .with_property("md.failed-devices", "0"),
        );
        graph.add_node(
            Node::new("md:/dev/md/degraded", NodeKind::MdRaid, "/dev/md/degraded")
                .with_path("/dev/md/degraded")
                .with_property("md.state", "clean, degraded")
                .with_property("md.degraded-devices", "1")
                .with_property("md.failed-devices", "0"),
        );
        graph.add_node(
            Node::new(
                "filesystem:/dev/md/wrong-kind",
                NodeKind::Filesystem,
                "/dev/md/wrong-kind",
            )
            .with_path("/dev/md/wrong-kind"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 3);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert_eq!(plan.summary.action_count, 2);
        assert!(
            plan.actions
                .iter()
                .all(|action| action.id != "mdraids:existing:create")
        );
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "mdraids:existing:create"
                && diagnostic.kind == TopologyDiagnosticKind::MdCreateAlreadySatisfied
                && diagnostic.message.contains("cleanly active")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "mdraids:degraded:create"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::MdCreateRequired
                && diagnostic.message.contains("state=clean, degraded")
                && diagnostic.message.contains("degradedDevices=1")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "mdraids:wrong-kind:create"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::MdCreateRequired
                && diagnostic.message.contains("not an MD RAID array")
        }));
    }

    #[test]
    fn topology_comparison_reconciles_md_stop() {
        let plan = plan_from_json_bytes(
            br#"{
              "mdRaids": {
                "absent": {
                  "operation": "stop",
                  "target": "/dev/md/absent"
                },
                "inactive": {
                  "operation": "stop",
                  "target": "/dev/md/inactive"
                },
                "active": {
                  "operation": "stop",
                  "target": "/dev/md/active"
                },
                "unknown": {
                  "operation": "stop",
                  "target": "/dev/md/unknown"
                },
                "wrong-kind": {
                  "operation": "stop",
                  "target": "/dev/md/wrong-kind"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("md:/dev/md/inactive", NodeKind::MdRaid, "/dev/md/inactive")
                .with_path("/dev/md/inactive")
                .with_property("md.state", "inactive")
                .with_property("md.degraded-devices", "0")
                .with_property("md.failed-devices", "0"),
        );
        graph.add_node(
            Node::new("md:/dev/md/active", NodeKind::MdRaid, "/dev/md/active")
                .with_path("/dev/md/active")
                .with_property("md.state", "clean")
                .with_property("md.degraded-devices", "0")
                .with_property("md.failed-devices", "0"),
        );
        graph.add_node(
            Node::new("md:/dev/md/unknown", NodeKind::MdRaid, "/dev/md/unknown")
                .with_path("/dev/md/unknown"),
        );
        graph.add_node(
            Node::new(
                "filesystem:/dev/md/wrong-kind",
                NodeKind::Filesystem,
                "/dev/md/wrong-kind",
            )
            .with_path("/dev/md/wrong-kind"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 5);
        assert_eq!(comparison.summary.already_satisfied_count, 2);
        assert_eq!(comparison.summary.suppressed_action_count, 2);
        assert_eq!(plan.summary.action_count, 3);
        assert!(
            plan.actions
                .iter()
                .all(|action| action.id != "mdraids:absent:stop")
        );
        assert!(
            plan.actions
                .iter()
                .all(|action| action.id != "mdraids:inactive:stop")
        );
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "mdraids:absent:stop"
                && diagnostic.kind == TopologyDiagnosticKind::MdStopAlreadySatisfied
                && diagnostic.message.contains("already absent")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "mdraids:inactive:stop"
                && diagnostic.kind == TopologyDiagnosticKind::MdStopAlreadySatisfied
                && diagnostic.message.contains("already inactive")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "mdraids:active:stop"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::MdStopRequired
                && diagnostic.message.contains("still active")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "mdraids:unknown:stop"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::MdStopRequired
                && diagnostic.message.contains("current state is unknown")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "mdraids:wrong-kind:stop"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::MdStopRequired
                && diagnostic.message.contains("not an MD RAID array")
        }));
    }

    #[test]
    fn topology_comparison_reconciles_md_membership_updates() {
        let plan = plan_from_json_bytes(
            br#"{
              "mdRaids": {
                "root": {
                  "target": "/dev/md/root",
                  "addDevices": ["/dev/sdb1", "/dev/sdd1"],
                  "removeDevices": ["/dev/sdc1", "/dev/sde1"]
                },
                "absent": {
                  "target": "/dev/md/absent",
                  "removeDevices": ["/dev/sdf1"]
                },
                "wrong-kind": {
                  "target": "/dev/md/wrong-kind",
                  "addDevices": ["/dev/sdg1"],
                  "removeDevices": ["/dev/sdh1"]
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("md:/dev/md/root", NodeKind::MdRaid, "/dev/md/root")
                .with_path("/dev/md/root")
                .with_property("md.state", "clean")
                .with_property("md.degraded-devices", "0")
                .with_property("md.failed-devices", "0"),
        );
        graph.add_node(
            Node::new("block:/dev/sdb1", NodeKind::Partition, "/dev/sdb1").with_path("/dev/sdb1"),
        );
        graph.add_node(
            Node::new("block:/dev/sdc1", NodeKind::Partition, "/dev/sdc1").with_path("/dev/sdc1"),
        );
        graph.add_node(
            Node::new(
                "filesystem:/dev/md/wrong-kind",
                NodeKind::Filesystem,
                "/dev/md/wrong-kind",
            )
            .with_path("/dev/md/wrong-kind"),
        );
        graph.add_edge(disk_nix_model::Edge::new(
            "block:/dev/sdb1",
            "md:/dev/md/root",
            Relationship::MemberOf,
        ));
        graph.add_edge(disk_nix_model::Edge::new(
            "block:/dev/sdc1",
            "md:/dev/md/root",
            Relationship::MemberOf,
        ));

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 7);
        assert_eq!(comparison.summary.already_satisfied_count, 3);
        assert_eq!(comparison.summary.suppressed_action_count, 3);
        assert_eq!(plan.summary.action_count, 4);
        for suppressed_id in [
            "mdRaids:root:add-device:/dev/sdb1",
            "mdRaids:root:remove-device:/dev/sde1",
            "mdRaids:absent:remove-device:/dev/sdf1",
        ] {
            assert!(plan.actions.iter().all(|action| action.id != suppressed_id));
        }
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "mdRaids:root:add-device:/dev/sdb1"
                && diagnostic.kind == TopologyDiagnosticKind::MdMemberAddAlreadySatisfied
                && diagnostic
                    .message
                    .contains("already includes member /dev/sdb1")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "mdRaids:root:add-device:/dev/sdd1"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::MdMemberAddRequired
                && diagnostic
                    .message
                    .contains("does not currently include member /dev/sdd1")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "mdRaids:root:remove-device:/dev/sdc1"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::MdMemberRemoveRequired
                && diagnostic
                    .message
                    .contains("still includes member /dev/sdc1")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "mdRaids:root:remove-device:/dev/sde1"
                && diagnostic.kind == TopologyDiagnosticKind::MdMemberRemoveAlreadySatisfied
                && diagnostic
                    .message
                    .contains("no longer includes member /dev/sde1")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "mdRaids:absent:remove-device:/dev/sdf1"
                && diagnostic.kind == TopologyDiagnosticKind::MdMemberRemoveAlreadySatisfied
                && diagnostic
                    .message
                    .contains("array /dev/md/absent is absent")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "mdRaids:wrong-kind:add-device:/dev/sdg1"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::MdMemberAddRequired
                && diagnostic.message.contains("not an MD RAID array")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "mdRaids:wrong-kind:remove-device:/dev/sdh1"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::MdMemberRemoveRequired
                && diagnostic.message.contains("not an MD RAID array")
        }));
    }

    #[test]
    fn topology_comparison_reconciles_md_member_replacement() {
        let plan = plan_from_json_bytes(
            br#"{
              "mdRaids": {
                "done": {
                  "target": "/dev/md/done",
                  "replaceDevices": {
                    "/dev/sdb1": "/dev/sdc1"
                  }
                },
                "pending": {
                  "target": "/dev/md/pending",
                  "replaceDevices": {
                    "/dev/sdd1": "/dev/sde1"
                  }
                },
                "both": {
                  "target": "/dev/md/both",
                  "replaceDevices": {
                    "/dev/sdf1": "/dev/sdg1"
                  }
                },
                "missing-new": {
                  "target": "/dev/md/missing-new",
                  "replaceDevices": {
                    "/dev/sdh1": "/dev/sdi1"
                  }
                },
                "wrong-kind": {
                  "target": "/dev/md/wrong-kind",
                  "replaceDevices": {
                    "/dev/sdj1": "/dev/sdk1"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        for target in [
            "/dev/md/done",
            "/dev/md/pending",
            "/dev/md/both",
            "/dev/md/missing-new",
        ] {
            graph.add_node(
                Node::new(format!("md:{target}"), NodeKind::MdRaid, target)
                    .with_path(target)
                    .with_property("md.state", "clean")
                    .with_property("md.degraded-devices", "0")
                    .with_property("md.failed-devices", "0"),
            );
        }
        graph.add_node(
            Node::new(
                "filesystem:/dev/md/wrong-kind",
                NodeKind::Filesystem,
                "/dev/md/wrong-kind",
            )
            .with_path("/dev/md/wrong-kind"),
        );

        for (device, target) in [
            ("/dev/sdc1", "/dev/md/done"),
            ("/dev/sdd1", "/dev/md/pending"),
            ("/dev/sdf1", "/dev/md/both"),
            ("/dev/sdg1", "/dev/md/both"),
        ] {
            graph.add_node(
                Node::new(format!("block:{device}"), NodeKind::Partition, device).with_path(device),
            );
            graph.add_edge(disk_nix_model::Edge::new(
                format!("block:{device}"),
                format!("md:{target}"),
                Relationship::MemberOf,
            ));
        }

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 5);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert_eq!(plan.summary.action_count, 4);
        assert!(
            plan.actions
                .iter()
                .all(|action| action.id != "mdRaids:done:replace-device:/dev/sdb1")
        );
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "mdRaids:done:replace-device:/dev/sdb1"
                && diagnostic.kind == TopologyDiagnosticKind::MdMemberReplaceAlreadySatisfied
                && diagnostic
                    .message
                    .contains("already replaced member /dev/sdb1 with /dev/sdc1")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "mdRaids:pending:replace-device:/dev/sdd1"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::MdMemberReplaceRequired
                && diagnostic
                    .message
                    .contains("still includes old member /dev/sdd1")
                && diagnostic
                    .message
                    .contains("does not include replacement /dev/sde1")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "mdRaids:both:replace-device:/dev/sdf1"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::MdMemberReplaceRequired
                && diagnostic
                    .message
                    .contains("still includes old member /dev/sdf1")
                && diagnostic
                    .message
                    .contains("already includes replacement /dev/sdg1")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "mdRaids:missing-new:replace-device:/dev/sdh1"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::MdMemberReplaceRequired
                && diagnostic
                    .message
                    .contains("no longer includes old member /dev/sdh1")
                && diagnostic
                    .message
                    .contains("replacement /dev/sdi1 is not attached")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "mdRaids:wrong-kind:replace-device:/dev/sdj1"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::MdMemberReplaceRequired
                && diagnostic.message.contains("not an MD RAID array")
        }));
    }

    #[test]
    fn topology_comparison_keeps_md_assemble_when_degraded() {
        let plan = plan_from_json_bytes(
            br#"{
              "mdRaids": {
                "existing": {
                  "operation": "assemble",
                  "target": "/dev/md/existing",
                  "devices": ["/dev/sdb1", "/dev/sdc1"]
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("md:/dev/md/existing", NodeKind::MdRaid, "/dev/md/existing")
                .with_path("/dev/md/existing")
                .with_property("md.state", "clean, degraded")
                .with_property("md.degraded-devices", "1")
                .with_property("md.failed-devices", "0"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "mdraids:existing:assemble"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::MdAssembleRequired
        }));
    }

    #[test]
    fn topology_comparison_keeps_md_assemble_when_inactive() {
        let plan = plan_from_json_bytes(
            br#"{
              "mdRaids": {
                "existing": {
                  "operation": "assemble",
                  "target": "/dev/md/existing",
                  "devices": ["/dev/sdb1", "/dev/sdc1"]
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("md:/dev/md/existing", NodeKind::MdRaid, "/dev/md/existing")
                .with_path("/dev/md/existing")
                .with_property("md.state", "inactive")
                .with_property("md.degraded-devices", "0")
                .with_property("md.failed-devices", "0"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "mdraids:existing:assemble"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::MdAssembleRequired
        }));
    }

    #[test]
    fn topology_comparison_reconciles_zfs_pool_create() {
        let plan = plan_from_json_bytes(
            br#"{
              "pools": {
                "tank": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/pool-vdev0"
                },
                "vault": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/pool-vdev1"
                },
                "archive": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/pool-vdev2"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("zfs-pool:tank", NodeKind::ZfsPool, "tank")
                .with_property("zfs.state", "ONLINE")
                .with_property("zfs.health", "ONLINE")
                .with_property("zfs.pool-capacity", "40%")
                .with_property("zfs.pool-fragmentation", "12%"),
        );
        graph.add_node(
            Node::new("zfs-pool:vault", NodeKind::ZfsPool, "vault")
                .with_property("zfs.state", "ONLINE")
                .with_property("zfs.health", "DEGRADED"),
        );
        graph.add_node(Node::new(
            "zfs-dataset:archive",
            NodeKind::ZfsDataset,
            "archive",
        ));

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 3);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert_eq!(plan.summary.action_count, 2);
        assert!(
            plan.actions
                .iter()
                .all(|action| action.id != "pools:tank:create")
        );
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "pools:tank:create"
                && diagnostic.kind == TopologyDiagnosticKind::ZfsPoolCreateAlreadySatisfied
                && diagnostic.message.contains("capacity 40%")
                && diagnostic.message.contains("fragmentation 12%")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "pools:vault:create"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::ZfsPoolCreateRequired
                && diagnostic.message.contains("state=ONLINE")
                && diagnostic.message.contains("health=DEGRADED")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "pools:archive:create"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::ZfsPoolCreateRequired
                && diagnostic.message.contains("not a ZFS pool")
        }));
    }

    #[test]
    fn topology_comparison_suppresses_imported_online_zfs_pool() {
        let plan = plan_from_json_bytes(
            br#"{
              "pools": {
                "tank": {
                  "operation": "import"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("zfs-pool:tank", NodeKind::ZfsPool, "tank")
                .with_property("zfs.state", "ONLINE")
                .with_property("zfs.health", "ONLINE"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "pools:tank:import"
                && diagnostic.kind == TopologyDiagnosticKind::ZfsPoolImportAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_zfs_pool_import_when_degraded() {
        let plan = plan_from_json_bytes(
            br#"{
              "pools": {
                "tank": {
                  "operation": "import"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("zfs-pool:tank", NodeKind::ZfsPool, "tank")
                .with_property("zfs.state", "DEGRADED")
                .with_property("zfs.health", "DEGRADED"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "pools:tank:import"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::ZfsPoolImportRequired
        }));
    }

    #[test]
    fn topology_comparison_reconciles_zfs_object_create() {
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
                "tank/conflict": {
                  "operation": "create"
                }
              },
              "zvols": {
                "tank/vm/root": {
                  "operation": "create",
                  "desiredSize": "20GiB"
                },
                "tank/vm/tmp": {
                  "operation": "create",
                  "desiredSize": "20GiB"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("zfs-dataset:tank/home", NodeKind::ZfsDataset, "tank/home")
                .with_property("zfs.type", "filesystem")
                .with_property("zfs.mountpoint", "/home")
                .with_property("zfs.compression", "zstd"),
        );
        graph.add_node(
            Node::new("zvol:tank/conflict", NodeKind::Zvol, "tank/conflict")
                .with_size_bytes(8 * 1024 * 1024 * 1024)
                .with_property("zfs.type", "volume")
                .with_property("zfs.volsize", "8G"),
        );
        graph.add_node(
            Node::new("zvol:tank/vm/root", NodeKind::Zvol, "tank/vm/root")
                .with_size_bytes(20 * 1024 * 1024 * 1024)
                .with_property("zfs.type", "volume")
                .with_property("zfs.volsize", "20G")
                .with_property("zfs.compression", "zstd"),
        );
        graph.add_node(
            Node::new("zvol:tank/vm/tmp", NodeKind::Zvol, "tank/vm/tmp")
                .with_size_bytes(10 * 1024 * 1024 * 1024)
                .with_property("zfs.type", "volume")
                .with_property("zfs.volsize", "10G"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 6);
        assert_eq!(comparison.summary.already_satisfied_count, 2);
        assert_eq!(comparison.summary.suppressed_action_count, 2);
        assert_eq!(plan.summary.action_count, 4);
        assert!(plan.actions.iter().any(|action| {
            action.id == "datasets:tank/conflict:create" && action.operation == Operation::Create
        }));
        assert!(plan.actions.iter().any(|action| {
            action.id == "zvols:tank/vm/tmp:create" && action.operation == Operation::Create
        }));
        assert!(plan.actions.iter().any(|action| {
            action.id == "datasets:tank/home:set-property:compression"
                && action.operation == Operation::SetProperty
        }));
        assert!(plan.actions.iter().any(|action| {
            action.id == "datasets:tank/home:set-property:mountpoint"
                && action.operation == Operation::SetProperty
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "datasets:tank/home:create"
                && diagnostic.kind == TopologyDiagnosticKind::ZfsObjectCreateAlreadySatisfied
                && diagnostic.message.contains("mountpoint /home")
                && diagnostic.message.contains("compression zstd")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "zvols:tank/vm/root:create"
                && diagnostic.kind == TopologyDiagnosticKind::ZfsObjectCreateAlreadySatisfied
                && diagnostic.message.contains("volsize 20G")
                && diagnostic.message.contains("compression zstd")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "datasets:tank/conflict:create"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::ZfsObjectCreateRequired
                && diagnostic.message.contains("not a ZFS dataset")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "zvols:tank/vm/tmp:create"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::ZfsObjectCreateRequired
                && diagnostic.message.contains("not desired size 20GiB")
        }));
    }

    #[test]
    fn topology_comparison_suppresses_zfs_dataset_destroy_when_absent() {
        let plan = plan_from_json_bytes(
            br#"{
              "datasets": {
                "tank/old": {
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let graph = StorageGraph::empty();

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "datasets:tank/old:destroy"
                && diagnostic.kind == TopologyDiagnosticKind::ZfsObjectDestroyAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_zfs_dataset_destroy_when_present() {
        let plan = plan_from_json_bytes(
            br#"{
              "datasets": {
                "tank/home": {
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("zfs-dataset:tank/home", NodeKind::ZfsDataset, "tank/home")
                .with_property("zfs.type", "filesystem")
                .with_property("zfs.mountpoint", "/home")
                .with_property("zfs.quota", "500G")
                .with_property("zfs.encryption", "aes-256-gcm")
                .with_property("zfs.keystatus", "available"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "datasets:tank/home:destroy"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::ZfsObjectDestroyRequired
                && diagnostic.message.contains("mountpoint /home")
                && diagnostic.message.contains("quota 500G")
                && diagnostic.message.contains("key status available")
        }));
    }

    #[test]
    fn topology_comparison_suppresses_zvol_destroy_when_absent() {
        let plan = plan_from_json_bytes(
            br#"{
              "zvols": {
                "tank/vm/old": {
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let graph = StorageGraph::empty();

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "zvols:tank/vm/old:destroy"
                && diagnostic.kind == TopologyDiagnosticKind::ZfsObjectDestroyAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_zvol_destroy_when_present() {
        let plan = plan_from_json_bytes(
            br#"{
              "zvols": {
                "tank/vm/root": {
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("zvol:tank/vm/root", NodeKind::Zvol, "tank/vm/root")
                .with_property("zfs.type", "volume")
                .with_property("zfs.volsize", "80G")
                .with_property("zfs.origin", "tank/vm/base@clean")
                .with_property("zfs.compression", "zstd"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "zvols:tank/vm/root:destroy"
                && diagnostic.kind == TopologyDiagnosticKind::ZfsObjectDestroyRequired
                && diagnostic.message.contains("volsize 80G")
                && diagnostic.message.contains("origin tank/vm/base@clean")
                && diagnostic.message.contains("compression zstd")
        }));
    }

    #[test]
    fn topology_comparison_suppresses_logged_in_iscsi_session() {
        let plan = plan_from_json_bytes(
            br#"{
              "iscsiSessions": {
                "iqn.2026-06.example:storage.root": {
                  "operation": "login",
                  "portal": "192.0.2.10:3260"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "iscsi-session:12",
                NodeKind::IscsiSession,
                "iscsi-session:12",
            )
            .with_property("iscsi.target", "iqn.2026-06.example:storage.root")
            .with_property("iscsi.session-state", "LOGGED_IN"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "iscsisessions:iqn.2026-06.example:storage.root:login"
                && diagnostic.kind == TopologyDiagnosticKind::IscsiLoginAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_prefers_logged_in_iscsi_session_over_configured_target() {
        let plan = plan_from_json_bytes(
            br#"{
              "iscsiSessions": {
                "iqn.2026-06.example:storage.root": {
                  "operation": "login",
                  "portal": "192.0.2.10:3260"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "iscsi-target:iqn.2026-06.example:storage.root",
                NodeKind::IscsiTarget,
                "iqn.2026-06.example:storage.root",
            )
            .with_property("iscsi.node-configured", "true"),
        );
        graph.add_node(
            Node::new(
                "iscsi-session:12",
                NodeKind::IscsiSession,
                "iscsi-session:12",
            )
            .with_property("iscsi.target", "iqn.2026-06.example:storage.root")
            .with_property("iscsi.connection-state", "LOGGED IN"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "iscsisessions:iqn.2026-06.example:storage.root:login"
                && diagnostic.current.as_ref().is_some_and(|current| {
                    current.kind == NodeKind::IscsiSession && current.id == "iscsi-session:12"
                })
                && diagnostic.kind == TopologyDiagnosticKind::IscsiLoginAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_iscsi_login_when_target_is_not_logged_in() {
        let plan = plan_from_json_bytes(
            br#"{
              "iscsiSessions": {
                "iqn.2026-06.example:storage.root": {
                  "operation": "login",
                  "portal": "192.0.2.10:3260"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "iscsi-target:iqn.2026-06.example:storage.root",
                NodeKind::IscsiTarget,
                "iqn.2026-06.example:storage.root",
            )
            .with_property("iscsi.node-configured", "true"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "iscsisessions:iqn.2026-06.example:storage.root:login"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::IscsiLoginRequired
        }));
    }

    #[test]
    fn topology_comparison_suppresses_iscsi_logout_when_session_absent() {
        let plan = plan_from_json_bytes(
            br#"{
              "iscsiSessions": {
                "iqn.2026-06.example:storage.old": {
                  "operation": "logout",
                  "portal": "192.0.2.10:3260"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "iscsi-target:iqn.2026-06.example:storage.old",
                NodeKind::IscsiTarget,
                "iqn.2026-06.example:storage.old",
            )
            .with_property("iscsi.node-configured", "true"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "iscsisessions:iqn.2026-06.example:storage.old:logout"
                && diagnostic.kind == TopologyDiagnosticKind::IscsiLogoutAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_iscsi_logout_when_session_is_logged_in() {
        let plan = plan_from_json_bytes(
            br#"{
              "iscsiSessions": {
                "iqn.2026-06.example:storage.old": {
                  "operation": "logout",
                  "portal": "192.0.2.10:3260"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "iscsi-session:19",
                NodeKind::IscsiSession,
                "iscsi-session:19",
            )
            .with_property("iscsi.target", "iqn.2026-06.example:storage.old")
            .with_property("iscsi.connection-state", "LOGGED_IN"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "iscsisessions:iqn.2026-06.example:storage.old:logout"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::IscsiLogoutRequired
        }));
    }

    #[test]
    fn topology_comparison_suppresses_bcache_detach_when_concrete_target_absent() {
        let plan = plan_from_json_bytes(
            br#"{
              "caches": {
                "/dev/bcache0": {
                  "removeDevices": ["cache-set-uuid"]
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let graph = StorageGraph::empty();

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert_eq!(comparison.summary.missing_count, 0);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "caches:/dev/bcache0:remove-device:cache-set-uuid"
                && diagnostic.current.is_none()
                && diagnostic.kind == TopologyDiagnosticKind::BcacheDetachAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_bcache_detach_when_target_exists() {
        let plan = plan_from_json_bytes(
            br#"{
              "caches": {
                "/dev/bcache0": {
                  "removeDevices": ["cache-set-uuid"]
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("block:/dev/bcache0", NodeKind::CacheDevice, "bcache0")
                .with_path("/dev/bcache0")
                .with_property("bcache.dirty-data", "64.0M")
                .with_property("bcache.cache-mode", "writeback")
                .with_property("bcache.set-uuid", "cache-set-uuid"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "caches:/dev/bcache0:remove-device:cache-set-uuid"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::BcacheDetachRequired
                && diagnostic.message.contains("dirty data 64.0M")
                && diagnostic.message.contains("cache mode writeback")
        }));
    }

    #[test]
    fn topology_comparison_keeps_logical_bcache_detach_missing() {
        let plan = plan_from_json_bytes(
            br#"{
              "caches": {
                "root-cache": {
                  "removeDevices": ["cache-set-uuid"]
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let graph = StorageGraph::empty();

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(comparison.summary.missing_count, 1);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "caches:root-cache:remove-device:cache-set-uuid"
                && diagnostic.kind == TopologyDiagnosticKind::Missing
        }));
    }

    #[test]
    fn topology_comparison_suppresses_btrfs_subvolume_destroy_when_absent() {
        let plan = plan_from_json_bytes(
            br#"{
              "btrfsSubvolumes": {
                "/mnt/persist/@old": {
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let graph = StorageGraph::empty();

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "btrfssubvolumes:/mnt/persist/@old:destroy"
                && diagnostic.kind == TopologyDiagnosticKind::BtrfsSubvolumeDestroyAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_reconciles_btrfs_subvolume_create() {
        let plan = plan_from_json_bytes(
            br#"{
              "btrfsSubvolumes": {
                "/mnt/persist/@home": {
                  "operation": "create"
                },
                "/mnt/persist/plain-dir": {
                  "operation": "create"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "btrfs-subvolume:fs-uuid:@home",
                NodeKind::BtrfsSubvolume,
                "@home",
            )
            .with_path("/mnt/persist/@home")
            .with_property("btrfs.id", "257")
            .with_property("btrfs.generation", "100")
            .with_property("btrfs.parent-id", "5")
            .with_property("btrfs.top-level", "5"),
        );
        graph.add_node(
            Node::new(
                "mount:/mnt/persist/plain-dir",
                NodeKind::Mountpoint,
                "/mnt/persist/plain-dir",
            )
            .with_path("/mnt/persist/plain-dir"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 2);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert_eq!(plan.summary.action_count, 1);
        assert!(
            plan.actions
                .iter()
                .all(|action| action.id == "btrfssubvolumes:/mnt/persist/plain-dir:create")
        );
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "btrfssubvolumes:/mnt/persist/@home:create"
                && diagnostic.kind == TopologyDiagnosticKind::BtrfsSubvolumeCreateAlreadySatisfied
                && diagnostic.message.contains("subvolume id 257")
                && diagnostic.message.contains("generation 100")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "btrfssubvolumes:/mnt/persist/plain-dir:create"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::BtrfsSubvolumeCreateRequired
                && diagnostic.message.contains("not a Btrfs subvolume")
        }));
    }

    #[test]
    fn topology_comparison_keeps_btrfs_subvolume_destroy_when_present() {
        let plan = plan_from_json_bytes(
            br#"{
              "btrfsSubvolumes": {
                "/mnt/persist/@home": {
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "btrfs-subvolume:fs-uuid:@home",
                NodeKind::BtrfsSubvolume,
                "@home",
            )
            .with_path("/mnt/persist/@home")
            .with_property("btrfs.id", "257")
            .with_property("btrfs.generation", "100")
            .with_property("btrfs.parent-id", "5")
            .with_property("btrfs.top-level", "5"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "btrfssubvolumes:/mnt/persist/@home:destroy"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::BtrfsSubvolumeDestroyRequired
                && diagnostic.message.contains("subvolume id 257")
                && diagnostic.message.contains("generation 100")
                && diagnostic.message.contains("parent id 5")
        }));
    }

    #[test]
    fn topology_comparison_keeps_logical_btrfs_subvolume_destroy_missing() {
        let plan = plan_from_json_bytes(
            br#"{
              "btrfsSubvolumes": {
                "old-home": {
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let graph = StorageGraph::empty();

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(comparison.summary.missing_count, 1);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "btrfssubvolumes:old-home:destroy"
                && diagnostic.kind == TopologyDiagnosticKind::Missing
        }));
    }

    #[test]
    fn topology_comparison_suppresses_zfs_snapshot_destroy_when_absent() {
        let plan = plan_from_json_bytes(
            br#"{
              "snapshots": {
                "tank/home@old": {
                  "target": "tank/home",
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let graph = StorageGraph::empty();

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "snapshot:tank/home@old:destroy"
                && diagnostic.kind == TopologyDiagnosticKind::SnapshotDestroyAlreadySatisfied
                && diagnostic.query == "tank/home@old"
        }));
    }

    #[test]
    fn topology_comparison_keeps_zfs_snapshot_destroy_when_present() {
        let plan = plan_from_json_bytes(
            br#"{
              "snapshots": {
                "tank/home@old": {
                  "target": "tank/home",
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "zfs-snapshot:tank/home@old",
                NodeKind::ZfsSnapshot,
                "tank/home@old",
            )
            .with_property("zfs.used", "10M")
            .with_property("zfs.referenced", "1G")
            .with_property("zfs.compression", "zstd")
            .with_property("zfs.userrefs", "2"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "snapshot:tank/home@old:destroy"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::SnapshotDestroyRequired
                && diagnostic.query == "tank/home@old"
                && diagnostic.message.contains("ZFS snapshot")
                && diagnostic.message.contains("used 10M")
                && diagnostic.message.contains("referenced 1G")
                && diagnostic.message.contains("user references 2")
        }));
    }

    #[test]
    fn topology_comparison_suppresses_btrfs_snapshot_destroy_when_absent() {
        let plan = plan_from_json_bytes(
            br#"{
              "snapshots": {
                "/mnt/persist/@home-old": {
                  "target": "/mnt/persist/@home",
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let graph = StorageGraph::empty();

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "snapshot:/mnt/persist/@home-old:destroy"
                && diagnostic.kind == TopologyDiagnosticKind::SnapshotDestroyAlreadySatisfied
                && diagnostic.query == "/mnt/persist/@home-old"
        }));
    }

    #[test]
    fn topology_comparison_keeps_btrfs_snapshot_destroy_when_present() {
        let plan = plan_from_json_bytes(
            br#"{
              "snapshots": {
                "/mnt/persist/@home-old": {
                  "target": "/mnt/persist/@home",
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "btrfs-snapshot:fs-uuid:@home-old",
                NodeKind::BtrfsSnapshot,
                "@home-old",
            )
            .with_path("/mnt/persist/@home-old")
            .with_property("btrfs.id", "258")
            .with_property("btrfs.generation", "120")
            .with_property("btrfs.parent-uuid", "source-uuid"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "snapshot:/mnt/persist/@home-old:destroy"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::SnapshotDestroyRequired
                && diagnostic.query == "/mnt/persist/@home-old"
                && diagnostic.message.contains("Btrfs snapshot")
                && diagnostic.message.contains("subvolume id 258")
                && diagnostic.message.contains("generation 120")
                && diagnostic.message.contains("parent UUID source-uuid")
        }));
    }

    #[test]
    fn topology_comparison_keeps_logical_snapshot_destroy_missing() {
        let plan = plan_from_json_bytes(
            br#"{
              "snapshots": {
                "old-home": {
                  "target": "tank/home",
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let graph = StorageGraph::empty();

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(comparison.summary.missing_count, 1);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "snapshot:old-home:destroy"
                && diagnostic.kind == TopologyDiagnosticKind::Missing
                && diagnostic.query == "old-home"
        }));
    }

    #[test]
    fn topology_comparison_warns_when_zfs_rollback_snapshot_absent() {
        let plan = plan_from_json_bytes(
            br#"{
              "snapshots": {
                "tank/home@before": {
                  "target": "tank/home",
                  "rollback": true
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(Node::new(
            "zfs-dataset:tank/home",
            NodeKind::ZfsDataset,
            "tank/home",
        ));

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(comparison.summary.missing_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "snapshot:tank/home@before:rollback"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::SnapshotRollbackPointMissing
                && diagnostic.query == "tank/home@before"
                && diagnostic.current.is_none()
        }));
    }

    #[test]
    fn topology_comparison_warns_when_zfs_rollback_snapshot_present() {
        let plan = plan_from_json_bytes(
            br#"{
              "snapshots": {
                "tank/home@before": {
                  "target": "tank/home",
                  "rollback": true,
                  "recursiveRollback": true
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "zfs-snapshot:tank/home@before",
                NodeKind::ZfsSnapshot,
                "tank/home@before",
            )
            .with_property("zfs.used", "64M")
            .with_property("zfs.referenced", "5G")
            .with_property("zfs.userrefs", "1")
            .with_property("zfs.compression", "lz4"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "snapshot:tank/home@before:rollback"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::SnapshotRollbackPointAvailable
                && diagnostic.query == "tank/home@before"
                && diagnostic.message.contains("used 64M")
                && diagnostic.message.contains("referenced 5G")
                && diagnostic.message.contains("user references 1")
                && diagnostic.message.contains("recursive rollback requested")
        }));
    }

    #[test]
    fn topology_comparison_keeps_logical_snapshot_rollback_missing() {
        let plan = plan_from_json_bytes(
            br#"{
              "snapshots": {
                "before": {
                  "target": "tank/home",
                  "rollback": true
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let graph = StorageGraph::empty();

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(comparison.summary.missing_count, 1);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "snapshot:before:rollback"
                && diagnostic.kind == TopologyDiagnosticKind::Missing
                && diagnostic.query == "before"
        }));
    }

    #[test]
    fn topology_comparison_warns_when_zfs_snapshot_clone_source_absent() {
        let plan = plan_from_json_bytes(
            br#"{
              "snapshots": {
                "tank/home@before": {
                  "target": "tank/home",
                  "cloneTo": "tank/home-review"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(Node::new(
            "zfs-dataset:tank/home-review",
            NodeKind::ZfsDataset,
            "tank/home-review",
        ));

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(comparison.summary.missing_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "snapshot:tank/home@before:clone:tank/home-review"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::SnapshotCloneSourceMissing
                && diagnostic.query == "tank/home@before"
                && diagnostic.current.is_none()
        }));
    }

    #[test]
    fn topology_comparison_reports_zfs_snapshot_clone_source_available() {
        let plan = plan_from_json_bytes(
            br#"{
              "snapshots": {
                "tank/home@before": {
                  "target": "tank/home",
                  "cloneTo": "tank/home-review"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "zfs-snapshot:tank/home@before",
                NodeKind::ZfsSnapshot,
                "tank/home@before",
            )
            .with_property("zfs.used", "8M")
            .with_property("zfs.referenced", "4G")
            .with_property("zfs.userrefs", "1"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "snapshot:tank/home@before:clone:tank/home-review"
                && diagnostic.level == TopologyDiagnosticLevel::Info
                && diagnostic.kind == TopologyDiagnosticKind::SnapshotCloneSourceAvailable
                && diagnostic.query == "tank/home@before"
                && diagnostic.message.contains("clone target tank/home-review")
                && diagnostic.message.contains("used 8M")
                && diagnostic.message.contains("user references 1")
        }));
    }

    #[test]
    fn topology_comparison_warns_when_btrfs_snapshot_clone_source_absent() {
        let plan = plan_from_json_bytes(
            br#"{
              "snapshots": {
                "/mnt/persist/@home-before": {
                  "target": "/mnt/persist/@home",
                  "cloneTo": "/mnt/persist/@home-review"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "btrfs-subvolume:fs-uuid:@home-review",
                NodeKind::BtrfsSubvolume,
                "@home-review",
            )
            .with_path("/mnt/persist/@home-review"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(comparison.summary.missing_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id
                == "snapshot:/mnt/persist/@home-before:clone:/mnt/persist/@home-review"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::SnapshotCloneSourceMissing
                && diagnostic.query == "/mnt/persist/@home-before"
                && diagnostic.current.is_none()
        }));
    }

    #[test]
    fn topology_comparison_reports_btrfs_snapshot_clone_source_available() {
        let plan = plan_from_json_bytes(
            br#"{
              "snapshots": {
                "/mnt/persist/@home-before": {
                  "target": "/mnt/persist/@home",
                  "cloneTo": "/mnt/persist/@home-review"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "btrfs-snapshot:fs-uuid:@home-before",
                NodeKind::BtrfsSnapshot,
                "@home-before",
            )
            .with_path("/mnt/persist/@home-before")
            .with_property("btrfs.id", "300")
            .with_property("btrfs.parent-uuid", "source-uuid"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id
                == "snapshot:/mnt/persist/@home-before:clone:/mnt/persist/@home-review"
                && diagnostic.level == TopologyDiagnosticLevel::Info
                && diagnostic.kind == TopologyDiagnosticKind::SnapshotCloneSourceAvailable
                && diagnostic.query == "/mnt/persist/@home-before"
                && diagnostic
                    .message
                    .contains("clone target /mnt/persist/@home-review")
                && diagnostic.message.contains("subvolume id 300")
                && diagnostic.message.contains("parent UUID source-uuid")
        }));
    }

    #[test]
    fn topology_comparison_uses_snapshot_path_for_friendly_btrfs_clone() {
        let plan = plan_from_json_bytes(
            br#"{
              "snapshots": {
                "before-home": {
                  "target": "/mnt/persist/@home",
                  "snapshotPath": "/mnt/persist/@home-before",
                  "cloneTo": "/mnt/persist/@home-review"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "btrfs-snapshot:fs-uuid:@home-before",
                NodeKind::BtrfsSnapshot,
                "@home-before",
            )
            .with_path("/mnt/persist/@home-before")
            .with_property("btrfs.id", "300"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        let action = plan
            .actions
            .iter()
            .find(|action| action.id == "snapshot:before-home:clone:/mnt/persist/@home-review")
            .expect("friendly clone action should remain actionable");
        assert_eq!(
            action.context.snapshot_path.as_deref(),
            Some("/mnt/persist/@home-before")
        );
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "snapshot:before-home:clone:/mnt/persist/@home-review"
                && diagnostic.kind == TopologyDiagnosticKind::SnapshotCloneSourceAvailable
                && diagnostic.query == "/mnt/persist/@home-before"
        }));
    }

    #[test]
    fn topology_comparison_warns_when_zfs_snapshot_rename_source_absent() {
        let plan = plan_from_json_bytes(
            br#"{
              "snapshots": {
                "tank/home@old": {
                  "target": "tank/home",
                  "renameTo": "tank/home@kept"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(Node::new(
            "zfs-dataset:tank/home",
            NodeKind::ZfsDataset,
            "tank/home",
        ));

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(comparison.summary.missing_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "snapshot:tank/home@old:rename:tank/home@kept"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::SnapshotRenameSourceMissing
                && diagnostic.query == "tank/home@old"
                && diagnostic.current.is_none()
        }));
    }

    #[test]
    fn topology_comparison_warns_when_zfs_snapshot_rename_source_present() {
        let plan = plan_from_json_bytes(
            br#"{
              "snapshots": {
                "tank/home@old": {
                  "target": "tank/home",
                  "renameTo": "tank/home@kept"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "zfs-snapshot:tank/home@old",
                NodeKind::ZfsSnapshot,
                "tank/home@old",
            )
            .with_property("zfs.used", "12M")
            .with_property("zfs.referenced", "2G")
            .with_property("zfs.userrefs", "3"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "snapshot:tank/home@old:rename:tank/home@kept"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::SnapshotRenameRequired
                && diagnostic.query == "tank/home@old"
                && diagnostic.message.contains("rename to tank/home@kept")
                && diagnostic.message.contains("used 12M")
                && diagnostic.message.contains("user references 3")
        }));
    }

    #[test]
    fn topology_comparison_warns_when_btrfs_snapshot_rename_source_absent() {
        let plan = plan_from_json_bytes(
            br#"{
              "snapshots": {
                "/mnt/persist/@home-old": {
                  "target": "/mnt/persist/@home",
                  "renameTo": "/mnt/persist/@home-kept"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "btrfs-subvolume:fs-uuid:@home",
                NodeKind::BtrfsSubvolume,
                "@home",
            )
            .with_path("/mnt/persist/@home"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(comparison.summary.missing_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "snapshot:/mnt/persist/@home-old:rename:/mnt/persist/@home-kept"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::SnapshotRenameSourceMissing
                && diagnostic.query == "/mnt/persist/@home-old"
                && diagnostic.current.is_none()
        }));
    }

    #[test]
    fn topology_comparison_warns_when_btrfs_snapshot_rename_source_present() {
        let plan = plan_from_json_bytes(
            br#"{
              "snapshots": {
                "/mnt/persist/@home-old": {
                  "target": "/mnt/persist/@home",
                  "renameTo": "/mnt/persist/@home-kept"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "btrfs-snapshot:fs-uuid:@home-old",
                NodeKind::BtrfsSnapshot,
                "@home-old",
            )
            .with_path("/mnt/persist/@home-old")
            .with_property("btrfs.id", "258")
            .with_property("btrfs.parent-uuid", "source-uuid"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "snapshot:/mnt/persist/@home-old:rename:/mnt/persist/@home-kept"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::SnapshotRenameRequired
                && diagnostic.query == "/mnt/persist/@home-old"
                && diagnostic
                    .message
                    .contains("rename to /mnt/persist/@home-kept")
                && diagnostic.message.contains("subvolume id 258")
                && diagnostic.message.contains("parent UUID source-uuid")
        }));
    }

    #[test]
    fn topology_comparison_suppresses_btrfs_qgroup_destroy_when_absent() {
        let plan = plan_from_json_bytes(
            br#"{
              "btrfsQgroups": {
                "0/257": {
                  "destroy": true,
                  "target": "/mnt/persist"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let graph = StorageGraph::empty();

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "btrfsqgroups:0/257:destroy"
                && diagnostic.kind == TopologyDiagnosticKind::BtrfsQgroupDestroyAlreadySatisfied
                && diagnostic.query == "0/257"
        }));
    }

    #[test]
    fn topology_comparison_reconciles_btrfs_qgroup_create() {
        let plan = plan_from_json_bytes(
            br#"{
              "btrfsQgroups": {
                "0/257": {
                  "operation": "create",
                  "target": "/mnt/persist"
                },
                "0/258": {
                  "operation": "create",
                  "target": "/mnt/persist"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("btrfs-qgroup:fs-uuid:0/257", NodeKind::BtrfsQgroup, "0/257")
                .with_property("btrfs.qgroup-id", "0/257")
                .with_property("btrfs.max-referenced", "21474836480")
                .with_property("btrfs.max-exclusive", "none")
                .with_property("btrfs.qgroup-parents", "1/0")
                .with_usage(disk_nix_model::Usage {
                    used_bytes: Some(10_737_418_240),
                    free_bytes: None,
                    allocated_bytes: Some(2_147_483_648),
                }),
        );
        graph.add_node(
            Node::new("mount:/mnt/persist/0/258", NodeKind::Mountpoint, "0/258")
                .with_path("/mnt/persist/0/258"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 2);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert_eq!(plan.summary.action_count, 1);
        assert!(
            plan.actions
                .iter()
                .all(|action| action.id == "btrfsqgroups:0/258:create")
        );
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "btrfsqgroups:0/257:create"
                && diagnostic.kind == TopologyDiagnosticKind::BtrfsQgroupCreateAlreadySatisfied
                && diagnostic.message.contains("qgroup id 0/257")
                && diagnostic.message.contains("max referenced 21474836480")
                && diagnostic.message.contains("referenced 10737418240 bytes")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "btrfsqgroups:0/258:create"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::BtrfsQgroupCreateRequired
                && diagnostic.message.contains("not a Btrfs qgroup")
        }));
    }

    #[test]
    fn topology_comparison_keeps_btrfs_qgroup_destroy_when_present() {
        let plan = plan_from_json_bytes(
            br#"{
              "btrfsQgroups": {
                "0/257": {
                  "destroy": true,
                  "target": "/mnt/persist"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("btrfs-qgroup:fs-uuid:0/257", NodeKind::BtrfsQgroup, "0/257")
                .with_property("btrfs.qgroup-id", "0/257")
                .with_property("btrfs.max-referenced", "21474836480")
                .with_property("btrfs.max-exclusive", "none")
                .with_property("btrfs.qgroup-parents", "1/0")
                .with_usage(disk_nix_model::Usage {
                    used_bytes: Some(10_737_418_240),
                    free_bytes: None,
                    allocated_bytes: Some(2_147_483_648),
                }),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "btrfsqgroups:0/257:destroy"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::BtrfsQgroupDestroyRequired
                && diagnostic.query == "0/257"
                && diagnostic.message.contains("qgroup id 0/257")
                && diagnostic.message.contains("max referenced 21474836480")
                && diagnostic.message.contains("parents 1/0")
                && diagnostic.message.contains("referenced 10737418240 bytes")
        }));
    }

    #[test]
    fn topology_comparison_keeps_logical_btrfs_qgroup_destroy_missing() {
        let plan = plan_from_json_bytes(
            br#"{
              "btrfsQgroups": {
                "old-qgroup": {
                  "destroy": true,
                  "target": "/mnt/persist"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let graph = StorageGraph::empty();

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(comparison.summary.missing_count, 1);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "btrfsqgroups:old-qgroup:destroy"
                && diagnostic.kind == TopologyDiagnosticKind::Missing
                && diagnostic.query == "old-qgroup"
        }));
    }

    #[test]
    fn topology_comparison_suppresses_dm_map_destroy_when_map_absent() {
        let plan = plan_from_json_bytes(
            br#"{
              "dmMaps": {
                "oldmap": {
                  "operation": "destroy",
                  "target": "/dev/mapper/oldmap"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let graph = StorageGraph::empty();

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert_eq!(comparison.summary.missing_count, 0);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "dmmaps:oldmap:destroy"
                && diagnostic.current.is_none()
                && diagnostic.kind == TopologyDiagnosticKind::DmMapDestroyAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_dm_map_destroy_when_map_exists() {
        let plan = plan_from_json_bytes(
            br#"{
              "dmMaps": {
                "oldmap": {
                  "operation": "destroy",
                  "target": "/dev/mapper/oldmap"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("dm:oldmap", NodeKind::DeviceMapper, "oldmap")
                .with_path("/dev/mapper/oldmap")
                .with_property("dm.open-count", "2"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "dmmaps:oldmap:destroy"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::DmMapDestroyRequired
                && diagnostic
                    .message
                    .contains("still present with open count 2")
        }));
    }

    #[test]
    fn topology_comparison_suppresses_multipath_destroy_when_map_absent() {
        let plan = plan_from_json_bytes(
            br#"{
              "multipathMaps": {
                "mpath-old": {
                  "operation": "destroy",
                  "target": "mpath-old"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let graph = StorageGraph::empty();

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert_eq!(comparison.summary.missing_count, 0);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "multipathmaps:mpath-old:destroy"
                && diagnostic.current.is_none()
                && diagnostic.kind == TopologyDiagnosticKind::MultipathDestroyAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_multipath_destroy_when_map_exists() {
        let plan = plan_from_json_bytes(
            br#"{
              "multipathMaps": {
                "mpatha": {
                  "operation": "destroy",
                  "target": "/dev/mapper/mpatha"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("multipath:mpatha", NodeKind::MultipathDevice, "mpatha")
                .with_path("/dev/mapper/mpatha")
                .with_property("multipath.wwid", "3600508b400105e210000900000490000")
                .with_property("multipath.dm", "dm-3"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "multipathmaps:mpatha:destroy"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::MultipathDestroyRequired
                && diagnostic
                    .message
                    .contains("WWID 3600508b400105e210000900000490000")
        }));
    }

    #[test]
    fn topology_comparison_reconciles_multipath_path_membership() {
        let plan = plan_from_json_bytes(
            br#"{
              "multipathMaps": {
                "mpatha": {
                  "target": "/dev/mapper/mpatha",
                  "addDevices": ["/dev/sdb", "/dev/sdd"],
                  "removeDevices": ["/dev/sdc", "/dev/sde"]
                },
                "absent": {
                  "target": "/dev/mapper/absent",
                  "removeDevices": ["/dev/sdf"]
                },
                "wrong-kind": {
                  "target": "/dev/mapper/wrong-kind",
                  "addDevices": ["/dev/sdg"],
                  "removeDevices": ["/dev/sdh"]
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("multipath:mpatha", NodeKind::MultipathDevice, "mpatha")
                .with_path("/dev/mapper/mpatha")
                .with_property("multipath.wwid", "3600508b400105e210000900000490000"),
        );
        graph.add_node(
            Node::new("block:/dev/sdb", NodeKind::PhysicalDisk, "/dev/sdb").with_path("/dev/sdb"),
        );
        graph.add_node(
            Node::new("block:/dev/sdc", NodeKind::PhysicalDisk, "/dev/sdc").with_path("/dev/sdc"),
        );
        graph.add_node(
            Node::new(
                "dm:/dev/mapper/wrong-kind",
                NodeKind::DeviceMapper,
                "/dev/mapper/wrong-kind",
            )
            .with_path("/dev/mapper/wrong-kind"),
        );
        graph.add_edge(disk_nix_model::Edge::new(
            "block:/dev/sdb",
            "multipath:mpatha",
            Relationship::Backs,
        ));
        graph.add_edge(disk_nix_model::Edge::new(
            "block:/dev/sdc",
            "multipath:mpatha",
            Relationship::Backs,
        ));

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 7);
        assert_eq!(comparison.summary.already_satisfied_count, 3);
        assert_eq!(comparison.summary.suppressed_action_count, 3);
        assert_eq!(plan.summary.action_count, 4);
        for suppressed_id in [
            "multipathMaps:mpatha:add-device:/dev/sdb",
            "multipathMaps:mpatha:remove-device:/dev/sde",
            "multipathMaps:absent:remove-device:/dev/sdf",
        ] {
            assert!(plan.actions.iter().all(|action| action.id != suppressed_id));
        }
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "multipathMaps:mpatha:add-device:/dev/sdb"
                && diagnostic.kind == TopologyDiagnosticKind::MultipathPathAddAlreadySatisfied
                && diagnostic
                    .message
                    .contains("already includes path /dev/sdb")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "multipathMaps:mpatha:add-device:/dev/sdd"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::MultipathPathAddRequired
                && diagnostic
                    .message
                    .contains("does not currently include path /dev/sdd")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "multipathMaps:mpatha:remove-device:/dev/sdc"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::MultipathPathRemoveRequired
                && diagnostic.message.contains("still includes path /dev/sdc")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "multipathMaps:mpatha:remove-device:/dev/sde"
                && diagnostic.kind == TopologyDiagnosticKind::MultipathPathRemoveAlreadySatisfied
                && diagnostic
                    .message
                    .contains("no longer includes path /dev/sde")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "multipathMaps:absent:remove-device:/dev/sdf"
                && diagnostic.kind == TopologyDiagnosticKind::MultipathPathRemoveAlreadySatisfied
                && diagnostic
                    .message
                    .contains("map /dev/mapper/absent is absent")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "multipathMaps:wrong-kind:add-device:/dev/sdg"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::MultipathPathAddRequired
                && diagnostic.message.contains("not a multipath map")
        }));
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "multipathMaps:wrong-kind:remove-device:/dev/sdh"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::MultipathPathRemoveRequired
                && diagnostic.message.contains("not a multipath map")
        }));
    }

    #[test]
    fn topology_comparison_suppresses_loop_create_when_mapping_matches() {
        let plan = plan_from_json_bytes(
            br#"{
              "loopDevices": {
                "/dev/loop7": {
                  "operation": "create",
                  "device": "/var/lib/images/root.img"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("block:/dev/loop7", NodeKind::LoopDevice, "/dev/loop7")
                .with_path("/dev/loop7")
                .with_property("loop.back-file", "/var/lib/images/root.img"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "loopdevices:/dev/loop7:create"
                && diagnostic.kind == TopologyDiagnosticKind::LoopCreateAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_loop_create_when_mapping_differs() {
        let plan = plan_from_json_bytes(
            br#"{
              "loopDevices": {
                "/dev/loop7": {
                  "operation": "create",
                  "device": "/var/lib/images/root.img"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("block:/dev/loop7", NodeKind::LoopDevice, "/dev/loop7")
                .with_path("/dev/loop7")
                .with_property("loop.back-file", "/var/lib/images/other.img"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "loopdevices:/dev/loop7:create"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::LoopCreateConflict
        }));
    }

    #[test]
    fn topology_comparison_keeps_loop_create_when_mapping_absent() {
        let plan = plan_from_json_bytes(
            br#"{
              "loopDevices": {
                "/dev/loop7": {
                  "operation": "create",
                  "device": "/var/lib/images/root.img"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let graph = StorageGraph::empty();

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(comparison.summary.missing_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "loopdevices:/dev/loop7:create"
                && diagnostic.level == TopologyDiagnosticLevel::Info
                && diagnostic.kind == TopologyDiagnosticKind::LoopCreateRequired
        }));
    }

    #[test]
    fn topology_comparison_suppresses_loop_destroy_when_mapping_absent() {
        let plan = plan_from_json_bytes(
            br#"{
              "loopDevices": {
                "/dev/loop9": {
                  "operation": "destroy"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let graph = StorageGraph::empty();

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert_eq!(comparison.summary.missing_count, 0);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "loopdevices:/dev/loop9:destroy"
                && diagnostic.current.is_none()
                && diagnostic.kind == TopologyDiagnosticKind::LoopDetachAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_loop_destroy_when_mapping_exists() {
        let plan = plan_from_json_bytes(
            br#"{
              "loopDevices": {
                "/dev/loop9": {
                  "operation": "destroy"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("block:/dev/loop9", NodeKind::LoopDevice, "/dev/loop9")
                .with_path("/dev/loop9")
                .with_property("loop.back-file", "/var/lib/images/old.img"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "loopdevices:/dev/loop9:destroy"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::LoopDetachRequired
        }));
    }

    #[test]
    fn topology_comparison_suppresses_nvme_namespace_attach_when_path_exists() {
        let plan = plan_from_json_bytes(
            br#"{
              "nvmeNamespaces": {
                "root-ns": {
                  "operation": "attach",
                  "target": "/dev/nvme0",
                  "device": "/dev/nvme0n1",
                  "namespaceId": "1",
                  "controllers": "0x1"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "block:/dev/nvme0n1",
                NodeKind::NvmeNamespace,
                "/dev/nvme0n1",
            )
            .with_path("/dev/nvme0n1")
            .with_property("nvme.namespace-id", "1"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "nvmenamespaces:root-ns:attach"
                && diagnostic.kind == TopologyDiagnosticKind::NvmeNamespaceAttachAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_nvme_namespace_attach_when_path_absent() {
        let plan = plan_from_json_bytes(
            br#"{
              "nvmeNamespaces": {
                "root-ns": {
                  "operation": "attach",
                  "target": "/dev/nvme0",
                  "device": "/dev/nvme0n1",
                  "namespaceId": "1",
                  "controllers": "0x1"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let graph = StorageGraph::empty();

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(comparison.summary.missing_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "nvmenamespaces:root-ns:attach"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::NvmeNamespaceAttachRequired
        }));
    }

    #[test]
    fn topology_comparison_suppresses_nvme_namespace_detach_when_path_absent() {
        let plan = plan_from_json_bytes(
            br#"{
              "nvmeNamespaces": {
                "old-ns": {
                  "operation": "detach",
                  "target": "/dev/nvme1",
                  "device": "/dev/nvme1n1",
                  "namespaceId": "2",
                  "controllers": "0x2"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let graph = StorageGraph::empty();

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert_eq!(comparison.summary.missing_count, 0);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "nvmenamespaces:old-ns:detach"
                && diagnostic.current.is_none()
                && diagnostic.kind == TopologyDiagnosticKind::NvmeNamespaceDetachAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_nvme_namespace_detach_when_path_exists() {
        let plan = plan_from_json_bytes(
            br#"{
              "nvmeNamespaces": {
                "old-ns": {
                  "operation": "detach",
                  "target": "/dev/nvme1",
                  "device": "/dev/nvme1n1",
                  "namespaceId": "2",
                  "controllers": "0x2"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "block:/dev/nvme1n1",
                NodeKind::NvmeNamespace,
                "/dev/nvme1n1",
            )
            .with_path("/dev/nvme1n1")
            .with_property("nvme.namespace-id", "2"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "nvmenamespaces:old-ns:detach"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::NvmeNamespaceDetachRequired
        }));
    }

    #[test]
    fn topology_comparison_suppresses_lun_attach_when_path_exists() {
        let plan = plan_from_json_bytes(
            br#"{
              "luns": {
                "iqn.2026-06.example:storage/root:0": {
                  "operation": "attach",
                  "device": "/dev/disk/by-path/ip-192.0.2.10-lun-0"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("lun:0", NodeKind::Lun, "0")
                .with_path("/dev/disk/by-path/ip-192.0.2.10-lun-0")
                .with_property("iscsi.attached-disk", "sdb"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "luns:iqn.2026-06.example:storage/root:0:attach"
                && diagnostic.kind == TopologyDiagnosticKind::LunAttachAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_lun_attach_when_path_absent() {
        let plan = plan_from_json_bytes(
            br#"{
              "luns": {
                "iqn.2026-06.example:storage/root:0": {
                  "operation": "attach",
                  "device": "/dev/disk/by-path/ip-192.0.2.10-lun-0"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let graph = StorageGraph::empty();

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(comparison.summary.missing_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "luns:iqn.2026-06.example:storage/root:0:attach"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::LunAttachRequired
        }));
    }

    #[test]
    fn topology_comparison_suppresses_lun_detach_when_path_absent() {
        let plan = plan_from_json_bytes(
            br#"{
              "luns": {
                "iqn.2026-06.example:storage/old:1": {
                  "operation": "detach",
                  "device": "/dev/disk/by-path/ip-192.0.2.10-lun-1"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let graph = StorageGraph::empty();

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert_eq!(comparison.summary.missing_count, 0);
        assert!(plan.actions.is_empty());
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "luns:iqn.2026-06.example:storage/old:1:detach"
                && diagnostic.current.is_none()
                && diagnostic.kind == TopologyDiagnosticKind::LunDetachAlreadySatisfied
        }));
    }

    #[test]
    fn topology_comparison_keeps_lun_detach_when_path_exists() {
        let plan = plan_from_json_bytes(
            br#"{
              "luns": {
                "iqn.2026-06.example:storage/old:1": {
                  "operation": "detach",
                  "device": "/dev/disk/by-path/ip-192.0.2.10-lun-1"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("lun:1", NodeKind::Lun, "1")
                .with_path("/dev/disk/by-path/ip-192.0.2.10-lun-1")
                .with_property("iscsi.attached-disk", "sdc"),
        );

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 0);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.actions.len(), 1);
        assert!(comparison.diagnostics.iter().any(|diagnostic| {
            diagnostic.action_id == "luns:iqn.2026-06.example:storage/old:1:detach"
                && diagnostic.level == TopologyDiagnosticLevel::Warning
                && diagnostic.kind == TopologyDiagnosticKind::LunDetachRequired
        }));
    }

    #[test]
    fn topology_comparison_adds_graph_dependency_edges_for_layered_growth() {
        let plan = plan_from_json_bytes(
            br#"{
              "luns": {
                "/dev/disk/by-path/ip-192.0.2.10-lun-0": {
                  "operation": "grow",
                  "desiredSize": "200GiB"
                }
              },
              "multipathMaps": {
                "mpatha": {
                  "operation": "grow",
                  "target": "/dev/mapper/mpatha",
                  "desiredSize": "200GiB"
                }
              },
              "partitions": {
                "root": {
                  "operation": "grow",
                  "target": "/dev/mapper/mpatha-part1",
                  "desiredSize": "200GiB"
                }
              },
              "luks": {
                "devices": {
                  "cryptroot": {
                    "operation": "grow",
                    "device": "/dev/mapper/mpatha-part1",
                    "target": "cryptroot",
                    "desiredSize": "200GiB"
                  }
                }
              },
              "volumes": {
                "vg0/root": {
                  "operation": "grow",
                  "desiredSize": "200GiB"
                }
              },
              "filesystems": {
                "root": {
                  "operation": "grow",
                  "device": "vg0/root",
                  "resizePolicy": "grow-only",
                  "desiredSize": "200GiB"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "lun:0",
                NodeKind::Lun,
                "/dev/disk/by-path/ip-192.0.2.10-lun-0",
            )
            .with_path("/dev/disk/by-path/ip-192.0.2.10-lun-0"),
        );
        graph.add_node(
            Node::new("multipath:mpatha", NodeKind::MultipathDevice, "mpatha")
                .with_path("/dev/mapper/mpatha"),
        );
        graph.add_node(
            Node::new(
                "partition:/dev/mapper/mpatha-part1",
                NodeKind::Partition,
                "/dev/mapper/mpatha-part1",
            )
            .with_path("/dev/mapper/mpatha-part1"),
        );
        graph.add_node(Node::new(
            "luks:cryptroot",
            NodeKind::LuksContainer,
            "cryptroot",
        ));
        graph.add_node(Node::new(
            "lvm:lv:vg0/root",
            NodeKind::LvmLogicalVolume,
            "vg0/root",
        ));
        graph.add_node(Node::new("filesystem:root", NodeKind::Filesystem, "root"));
        graph.add_edge(disk_nix_model::Edge::new(
            "lun:0",
            "multipath:mpatha",
            Relationship::Backs,
        ));
        graph.add_edge(disk_nix_model::Edge::new(
            "multipath:mpatha",
            "partition:/dev/mapper/mpatha-part1",
            Relationship::Contains,
        ));
        graph.add_edge(disk_nix_model::Edge::new(
            "partition:/dev/mapper/mpatha-part1",
            "luks:cryptroot",
            Relationship::Backs,
        ));
        graph.add_edge(disk_nix_model::Edge::new(
            "luks:cryptroot",
            "lvm:lv:vg0/root",
            Relationship::Backs,
        ));
        graph.add_edge(disk_nix_model::Edge::new(
            "lvm:lv:vg0/root",
            "filesystem:root",
            Relationship::Backs,
        ));

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.graph_dependency_edge_count, 15);
        let filesystem = plan
            .dependency_order
            .iter()
            .find(|order| order.action_id == "filesystem:root:grow")
            .expect("filesystem dependency order exists");
        assert!(
            filesystem
                .depends_on
                .contains(&"volumes:vg0/root:grow".to_string())
        );
        assert!(
            filesystem
                .depends_on
                .contains(&"luns:/dev/disk/by-path/ip-192.0.2.10-lun-0:grow".to_string())
        );
        assert!(
            filesystem
                .depends_on
                .contains(&"multipathmaps:mpatha:grow".to_string())
        );
        assert!(filesystem.notes.iter().any(|note| {
            note.contains("current topology graph path requires")
                && note.contains("volumes:vg0/root:grow")
        }));
        let lun = plan
            .dependency_order
            .iter()
            .find(|order| order.action_id == "luns:/dev/disk/by-path/ip-192.0.2.10-lun-0:grow")
            .expect("lun dependency order exists");
        assert_eq!(
            lun.unblocks,
            vec![
                "filesystem:root:grow".to_string(),
                "luks.devices:cryptroot:grow".to_string(),
                "multipathmaps:mpatha:grow".to_string(),
                "partitions:root:grow".to_string(),
                "volumes:vg0/root:grow".to_string(),
            ]
        );
        assert!(lun.notes.iter().any(|note| {
            note.contains("current topology graph path shows this action unblocks")
        }));
    }

    #[test]
    fn topology_comparison_reverses_graph_dependency_edges_for_teardown() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "root": {
                  "operation": "unmount",
                  "device": "/dev/mapper/cryptroot",
                  "mountpoint": "/"
                }
              },
              "luks": {
                "devices": {
                  "cryptroot": {
                    "operation": "close",
                    "device": "/dev/disk/by-partuuid/root",
                    "target": "cryptroot"
                  }
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("luks:cryptroot", NodeKind::LuksContainer, "cryptroot")
                .with_path("/dev/mapper/cryptroot"),
        );
        graph.add_node(
            Node::new("filesystem:/", NodeKind::Filesystem, "root")
                .with_path("/")
                .with_property("filesystem.type", "xfs"),
        );
        graph.add_edge(disk_nix_model::Edge::new(
            "luks:cryptroot",
            "filesystem:/",
            Relationship::Backs,
        ));

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");
        assert_eq!(comparison.summary.graph_dependency_edge_count, 1);

        let luks = plan
            .dependency_order
            .iter()
            .find(|order| order.action_id == "luks.devices:cryptroot:close")
            .expect("luks close dependency order exists");
        assert_eq!(
            luks.depends_on,
            vec!["filesystems:root:unmount".to_string()]
        );
        assert!(luks.notes.iter().any(|note| {
            note.contains("current topology graph path requires filesystems:root:unmount")
        }));
    }

    #[test]
    fn topology_comparison_ignores_suppressed_actions_for_graph_edges() {
        let plan = plan_from_json_bytes(
            br#"{
              "volumes": {
                "vg0/root": {
                  "operation": "grow",
                  "desiredSize": "100GiB"
                }
              },
              "filesystems": {
                "root": {
                  "operation": "grow",
                  "device": "vg0/root",
                  "desiredSize": "100GiB"
                }
              }
            }"#,
        )
        .expect("plan should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("lvm:lv:vg0/root", NodeKind::LvmLogicalVolume, "vg0/root")
                .with_size_bytes(200 * 1024 * 1024 * 1024),
        );
        graph.add_node(
            Node::new("filesystem:root", NodeKind::Filesystem, "root")
                .with_size_bytes(50 * 1024 * 1024 * 1024),
        );
        graph.add_edge(disk_nix_model::Edge::new(
            "lvm:lv:vg0/root",
            "filesystem:root",
            Relationship::Backs,
        ));

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");
        assert_eq!(comparison.summary.suppressed_action_count, 1);
        assert_eq!(comparison.summary.graph_dependency_edge_count, 0);
        assert!(
            plan.actions
                .iter()
                .all(|action| action.id != "volumes:vg0/root:grow")
        );
        assert!(
            plan.dependency_order
                .iter()
                .all(|order| order.depends_on.is_empty() && order.unblocks.is_empty())
        );
    }

    #[test]
    fn topology_comparison_keeps_satisfied_actions_with_warnings() {
        let plan = plan_from_json_bytes(
            br#"{
              "filesystems": {
                "home": {
                  "mountpoint": "/home",
                  "fsType": "btrfs",
                  "resizePolicy": "grow-only",
                  "desiredSize": "100GiB"
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

        let plan = compare_plan_with_topology(plan, &graph);
        let comparison = plan
            .topology_comparison
            .as_ref()
            .expect("comparison should be present");

        assert_eq!(comparison.summary.action_count, 1);
        assert_eq!(comparison.summary.already_satisfied_count, 1);
        assert_eq!(comparison.summary.type_conflict_count, 1);
        assert_eq!(comparison.summary.suppressed_action_count, 0);
        assert_eq!(plan.summary.action_count, 1);
        assert!(plan.actions.iter().any(|action| {
            action.id == "filesystem:home:grow" && action.operation == Operation::Grow
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
    fn plan_accepts_snapshot_clone_as_reversible() {
        let plan = plan_from_json_bytes(
            br#"{
              "snapshots": {
                "tank/home@before-upgrade": {
                  "target": "tank/home",
                  "cloneTo": "tank/home-review"
                },
                "/mnt/persist/@home-before": {
                  "target": "/mnt/persist/@home",
                  "cloneTo": "/mnt/persist/@home-review",
                  "readOnly": true
                }
              }
            }"#,
        )
        .expect("document should parse");

        assert_eq!(plan.summary.action_count, 2);
        let zfs_clone = plan
            .actions
            .iter()
            .find(|action| action.id == "snapshot:tank/home@before-upgrade:clone:tank/home-review")
            .expect("ZFS clone action exists");
        assert_eq!(zfs_clone.operation, Operation::Clone);
        assert_eq!(zfs_clone.risk, RiskClass::Reversible);
        assert_eq!(
            zfs_clone.context.name.as_deref(),
            Some("tank/home@before-upgrade")
        );
        assert_eq!(
            zfs_clone.context.target.as_deref(),
            Some("tank/home-review")
        );
        let btrfs_clone = plan
            .actions
            .iter()
            .find(|action| {
                action.id == "snapshot:/mnt/persist/@home-before:clone:/mnt/persist/@home-review"
            })
            .expect("Btrfs clone action exists");
        assert_eq!(btrfs_clone.operation, Operation::Clone);
        assert_eq!(btrfs_clone.risk, RiskClass::Reversible);
        assert_eq!(
            btrfs_clone.context.name.as_deref(),
            Some("/mnt/persist/@home-before")
        );
        assert_eq!(
            btrfs_clone.context.target.as_deref(),
            Some("/mnt/persist/@home-review")
        );
        assert_eq!(btrfs_clone.context.read_only, Some(true));
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
    fn plan_accepts_snapshot_name_aliases_for_logical_keys() {
        let plan = plan_from_json_bytes(
            br#"{
              "snapshots": {
                "before-hold": {
                  "name": "tank/home@before",
                  "target": "tank/home",
                  "hold": "keep"
                },
                "before-clone": {
                  "snapshotName": "tank/home@before",
                  "target": "tank/home",
                  "cloneTo": "tank/home-review"
                },
                "before-rescan": {
                  "snapshot-name": "tank/home@before",
                  "target": "tank/home",
                  "operation": "rescan"
                },
                "before-destroy": {
                  "name": "tank/home@old",
                  "target": "tank/home",
                  "destroy": true
                }
              }
            }"#,
        )
        .expect("plan should parse");

        let hold = plan
            .actions
            .iter()
            .find(|action| action.id == "snapshot:before-hold:hold:keep")
            .expect("logical-key hold action exists");
        assert_eq!(hold.context.name.as_deref(), Some("tank/home@before"));
        assert_eq!(hold.context.target.as_deref(), Some("tank/home"));

        let clone = plan
            .actions
            .iter()
            .find(|action| action.id == "snapshot:before-clone:clone:tank/home-review")
            .expect("logical-key clone action exists");
        assert_eq!(clone.context.name.as_deref(), Some("tank/home@before"));
        assert_eq!(clone.context.target.as_deref(), Some("tank/home-review"));

        let rescan = plan
            .actions
            .iter()
            .find(|action| action.id == "snapshot:before-rescan:rescan")
            .expect("logical-key rescan action exists");
        assert_eq!(rescan.context.name.as_deref(), Some("tank/home@before"));
        assert_eq!(rescan.context.target.as_deref(), Some("tank/home"));

        let destroy = plan
            .actions
            .iter()
            .find(|action| action.id == "snapshot:before-destroy:destroy")
            .expect("logical-key destroy action exists");
        assert_eq!(destroy.context.name.as_deref(), Some("tank/home@old"));
        assert_eq!(destroy.context.target.as_deref(), Some("tank/home"));
        assert_eq!(destroy.risk, RiskClass::Destructive);
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
                  "operation": "attach",
                  "namespaceId": "7",
                  "controllers": "0x2"
                },
                "/dev/nvme3": {
                  "operation": "detach",
                  "namespaceId": "8",
                  "controllers": "0x3"
                },
                "/dev/nvme4": {
                  "destroy": true,
                  "namespaceId": "9",
                  "controllers": "0x4"
                }
              }
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.summary.action_count, 5);
        assert_eq!(plan.summary.destructive_count, 2);
        assert_eq!(plan.summary.offline_required_count, 2);

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

        let attach = plan
            .actions
            .iter()
            .find(|action| action.id == "nvmenamespaces:/dev/nvme2:attach")
            .expect("NVMe namespace attach action exists");
        assert_eq!(attach.operation, Operation::Attach);
        assert_eq!(attach.risk, RiskClass::Online);
        assert_eq!(attach.context.namespace_id.as_deref(), Some("7"));
        assert_eq!(attach.context.controllers.as_deref(), Some("0x2"));

        let detach = plan
            .actions
            .iter()
            .find(|action| action.id == "nvmenamespaces:/dev/nvme3:detach")
            .expect("NVMe namespace detach action exists");
        assert_eq!(detach.operation, Operation::Detach);
        assert_eq!(detach.risk, RiskClass::OfflineRequired);
        assert!(!detach.destructive);
        assert_eq!(detach.context.namespace_id.as_deref(), Some("8"));
        assert_eq!(detach.context.controllers.as_deref(), Some("0x3"));

        let destroy = plan
            .actions
            .iter()
            .find(|action| action.id == "nvmenamespaces:/dev/nvme4:destroy")
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
