fn capability_group_logical_volumes() -> Vec<Capability> {
    vec![
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
