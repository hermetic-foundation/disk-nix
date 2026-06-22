use disk_nix_model::NodeKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Advice {
    pub summary: String,
    pub alternatives: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capability {
    pub node_kind: NodeKind,
    pub operation: Operation,
    pub risk: RiskClass,
    pub advice: Option<Advice>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    Rollback,
    Destroy,
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
}
