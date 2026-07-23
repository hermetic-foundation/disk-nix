
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recovery_depends_on: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recovery_unblocks: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DependencyPhase {
    BuildLowerLayers,
    MutateInPlace,
    TearDownUpperLayers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reconciliation_groups: Vec<TopologyReconciliationGroup>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lifecycle_groups: Vec<TopologyLifecycleGroup>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub graph_dependency_conflict_resolutions: Vec<GraphDependencyConflictResolution>,
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
    #[serde(default)]
    pub graph_dependency_conflict_count: usize,
    #[serde(default)]
    pub reconciliation_group_count: usize,
    #[serde(default)]
    pub partially_suppressed_group_count: usize,
    #[serde(default)]
    pub lifecycle_group_count: usize,
    #[serde(default)]
    pub graph_derived_lifecycle_group_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopologyLifecycleGroup {
    pub group_id: String,
    pub action_ids: Vec<String>,
    pub action_count: usize,
    pub edge_count: usize,
    pub graph_derived_edge_count: usize,
    pub phases: Vec<DependencyPhase>,
    pub directions: Vec<DependencyDirection>,
    pub recommendation: String,
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopologyReconciliationGroup {
    pub identity: String,
    pub action_ids: Vec<String>,
    pub planned_action_ids: Vec<String>,
    pub suppressed_action_ids: Vec<String>,
    pub action_count: usize,
    pub planned_count: usize,
    pub suppressed_count: usize,
    pub partially_suppressed: bool,
    pub recommendation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphDependencyConflictResolution {
    pub path: String,
    pub lower_action_id: String,
    pub upper_action_id: String,
    pub lower_direction: DependencyDirection,
    pub upper_direction: DependencyDirection,
    pub build_or_update_pass: Vec<String>,
    pub teardown_or_recovery_pass: Vec<String>,
    pub recommendation: String,
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
    DmMapRenameAlreadySatisfied,
    DmMapRenameRequired,
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
    LvmRenameAlreadySatisfied,
    LvmRenameRequired,
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
    MountRequired,
    MountSourceConflict,
    MountOptionsAlreadySatisfied,
    MountOptionsDiffer,
    UnmountAlreadySatisfied,
    UnmountRequired,
    NfsExportAlreadySatisfied,
    NfsExportDiffers,
    NfsExportRequired,
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
    ZfsObjectPromoteAlreadySatisfied,
    ZfsObjectPromoteRequired,
    ZfsObjectRenameAlreadySatisfied,
    ZfsObjectRenameRequired,
    ZfsPoolCreateAlreadySatisfied,
    ZfsPoolCreateRequired,
    ZfsPoolImportAlreadySatisfied,
    ZfsPoolImportRequired,
    GraphDependencyOrder,
    GraphDependencyConflict,
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
    pub rollback_value: Option<String>,
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
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backstore_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub array_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_pool: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clone_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub masking_group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lun: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rollback_options: Option<String>,
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
            && self.rollback_value.is_none()
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
            && self.provider.is_none()
            && self.backstore_type.is_none()
            && self.vendor.is_none()
            && self.array_id.is_none()
            && self.storage_pool.is_none()
            && self.volume_id.is_none()
            && self.snapshot_id.is_none()
            && self.clone_source.is_none()
            && self.masking_group.is_none()
            && self.target_id.is_none()
            && self.group.is_none()
            && self.lun.is_none()
            && self.options.is_none()
            && self.rollback_options.is_none()
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
