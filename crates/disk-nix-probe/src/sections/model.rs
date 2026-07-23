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
