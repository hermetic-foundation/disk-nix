fn capability_group_core() -> Vec<Capability> {
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
    ]
}
