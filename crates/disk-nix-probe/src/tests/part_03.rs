    #[test]
    fn clustered_lvm_over_nvme_fabric_fixture_preserves_shared_locking_metadata() {
        let mut graph = StorageGraph::empty();
        merge_graph(
            &mut graph,
            nvme::normalize_nvme_list_json(CLUSTERED_NVME_LIST)
                .expect("NVMe list fixture should parse"),
        );
        merge_graph(
            &mut graph,
            nvme::normalize_nvme_subsystems_json(CLUSTERED_NVME_SUBSYSTEMS)
                .expect("NVMe subsystem fixture should parse"),
        );
        merge_graph(
            &mut graph,
            lvm::normalize_lvm_json(
                CLUSTERED_LVM_PVS,
                CLUSTERED_LVM_VGS,
                CLUSTERED_LVM_LVS,
                None,
            )
            .expect("clustered LVM fixture should parse"),
        );

        let namespace = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/nvme2n1")
            .expect("NVMe-oF namespace should exist");
        assert_eq!(namespace.kind, NodeKind::NvmeNamespace);
        assert_eq!(namespace.size_bytes, Some(500_000_000_000));
        assert_eq!(
            namespace.usage.as_ref().and_then(|usage| usage.used_bytes),
            Some(300_000_000_000)
        );
        assert_has_property(namespace, "nvme.transport", "tcp");
        assert_has_property(namespace, "nvme.ana-state", "optimized");
        assert_has_property(
            namespace,
            "nvme.subsystem-nqn",
            "nqn.2014-08.org.nvmexpress:uuid:clustered-array",
        );

        let controller = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "nvme-controller:nvme2")
            .expect("primary fabric controller should exist");
        assert_eq!(controller.kind, NodeKind::NvmeController);
        assert_has_property(controller, "nvme.transport", "tcp");
        assert_has_property(controller, "nvme.path-state", "live");
        assert_has_property(controller, "nvme.host-iface", "ens3f0");
        assert_has_property(controller, "nvme.ana-state", "optimized");

        let secondary_controller = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "nvme-controller:nvme3")
            .expect("secondary fabric controller should exist");
        assert_has_property(secondary_controller, "nvme.path-state", "connecting");
        assert_has_property(secondary_controller, "nvme.ana-state", "non-optimized");

        let pv = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "lvm-pv:/dev/nvme2n1")
            .expect("clustered LVM PV should exist");
        assert_eq!(pv.kind, NodeKind::LvmPhysicalVolume);
        assert_eq!(pv.identity.uuid.as_deref(), Some("cluster-pv-uuid"));
        assert_has_property(pv, "lvm.pv-device-id-type", "sys_wwid");
        assert_has_property(
            pv,
            "lvm.pv-device-id",
            "nvme.0123456789abcdef0123456789abcdef",
        );
        assert_has_property(pv, "lvm.pv-tags", "fabric,shared");

        let vg = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "lvm-vg:vgcluster")
            .expect("clustered LVM VG should exist");
        assert_eq!(vg.kind, NodeKind::LvmVolumeGroup);
        assert_eq!(vg.identity.uuid.as_deref(), Some("cluster-vg-uuid"));
        assert_has_property(vg, "lvm.vg-clustered", "clustered");
        assert_has_property(vg, "lvm.vg-shared", "shared");
        assert_has_property(vg, "lvm.vg-lock-type", "sanlock");
        assert_has_property(vg, "lvm.vg-lock-args", "host_id=1");
        assert_has_property(vg, "lvm.vg-system-id", "node-a");

        let lv = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "lvm-lv:vgcluster/shareddata")
            .expect("clustered shared LV should exist");
        assert_eq!(lv.kind, NodeKind::LvmLogicalVolume);
        assert_eq!(lv.path.as_deref(), Some("/dev/vgcluster/shareddata"));
        assert_has_property(lv, "lvm.active-locally", "active locally");
        assert_has_property(lv, "lvm.active-remotely", "active remotely");
        assert_has_property(lv, "lvm.host", "node-a");
        assert_has_property(lv, "lvm.tags", "clustered,fabric");

        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "nvme-subsystem:nvme-subsys2"
                && edge.to.0 == "nvme-controller:nvme2"
                && edge.relationship == Relationship::Contains
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "nvme-controller:nvme2"
                && edge.to.0 == "block:/dev/nvme2n1"
                && edge.relationship == Relationship::Contains
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "lvm-pv:/dev/nvme2n1"
                && edge.to.0 == "lvm-vg:vgcluster"
                && edge.relationship == Relationship::MemberOf
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "lvm-vg:vgcluster"
                && edge.to.0 == "lvm-lv:vgcluster/shareddata"
                && edge.relationship == Relationship::Contains
        }));
    }

    #[test]
    fn clustered_lvm_failure_fixture_preserves_lock_manager_and_split_brain_state() {
        let graph = lvm::normalize_lvm_json(
            CLUSTERED_FAILURE_PVS,
            CLUSTERED_FAILURE_VGS,
            CLUSTERED_FAILURE_LVS,
            None,
        )
        .expect("clustered failure LVM fixture should parse");

        let vg = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "lvm-vg:vgshared")
            .expect("shared VG should exist");
        assert_eq!(vg.kind, NodeKind::LvmVolumeGroup);
        assert_eq!(vg.identity.uuid.as_deref(), Some("shared-vg-uuid"));
        assert_has_property(vg, "lvm.vg-clustered", "clustered");
        assert_has_property(vg, "lvm.vg-shared", "shared");
        assert_has_property(vg, "lvm.vg-lock-type", "dlm");
        assert_has_property(vg, "lvm.vg-lock-args", "lockspace=vgshared host_id=2");
        assert_has_property(vg, "lvm.vg-lock-status", "partial");
        assert_has_property(vg, "lvm.vg-lock-failure", "lvmlockd unavailable");
        assert_has_property(
            vg,
            "lvm.vg-lock-reason",
            "quorum lost after fabric partition",
        );
        assert_has_property(vg, "lvm.vg-split-brain", "suspected");
        assert_has_property(vg, "lvm.missing-pv-count", "1");
        assert_has_property(vg, "lvm.tags", "clustered,split-brain,lock-failure");

        let fabric_a_pv = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "lvm-pv:/dev/mapper/mpath-cluster-a")
            .expect("fabric A PV should exist");
        assert_has_property(fabric_a_pv, "lvm.pv-device-id-type", "sys_wwid");
        assert_has_property(
            fabric_a_pv,
            "lvm.pv-device-id",
            "dm.uuid.mpath-3600a098038314f6f2b5d514d43594c33",
        );
        assert_has_property(fabric_a_pv, "lvm.pv-tags", "fabric-a,lockspace");

        let fabric_b_pv = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "lvm-pv:/dev/mapper/mpath-cluster-b")
            .expect("fabric B PV should exist");
        assert_has_property(
            fabric_b_pv,
            "lvm.pv-device-id",
            "dm.uuid.mpath-3600a098038314f6f2b5d514d43594c44",
        );
        assert_has_property(fabric_b_pv, "lvm.pv-tags", "fabric-b,lockspace");

        let remote_lv = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "lvm-lv:vgshared/remoteactive")
            .expect("remote-active LV should exist");
        assert_has_property(remote_lv, "lvm.active-remotely", "active remotely");
        assert_has_property(remote_lv, "lvm.host", "node-a");
        assert_has_property(remote_lv, "lvm.lock-status", "remote");
        assert_has_property(remote_lv, "lvm.lock-args", "dlm remote-holder=node-a");

        let blocked_lv = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "lvm-lv:vgshared/blocked")
            .expect("blocked LV should exist");
        assert_has_property(blocked_lv, "lvm.active", "inactive");
        assert_has_property(blocked_lv, "lvm.health", "lock-failed");
        assert_has_property(blocked_lv, "lvm.suspended", "suspended");
        assert_has_property(blocked_lv, "lvm.lock-status", "failed");
        assert_has_property(blocked_lv, "lvm.lock-failure", "resource busy");
        assert_has_property(
            blocked_lv,
            "lvm.lock-reason",
            "split-brain protection refused activation",
        );
        assert_has_property(blocked_lv, "lvm.tags", "blocked,split-brain");

        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|edge| {
                    edge.to.0 == "lvm-vg:vgshared" && edge.relationship == Relationship::MemberOf
                })
                .count(),
            2
        );
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|edge| {
                    edge.from.0 == "lvm-vg:vgshared" && edge.relationship == Relationship::Contains
                })
                .count(),
            2
        );
    }

    #[test]
    fn lvm_backed_vdo_fixture_merges_runtime_stats_and_lvm_metadata() {
        let mut graph = StorageGraph::empty();
        merge_graph(
            &mut graph,
            vdo::normalize_vdo_status(LVM_BACKED_VDO_STATUS)
                .expect("LVM-backed VDO status fixture should parse"),
        );
        merge_graph(
            &mut graph,
            vdo::normalize_vdostats_table(LVM_BACKED_VDOSTATS)
                .expect("LVM-backed vdostats fixture should parse"),
        );
        merge_graph(
            &mut graph,
            vdo::normalize_vdostats_verbose(LVM_BACKED_VDOSTATS_VERBOSE)
                .expect("LVM-backed verbose vdostats fixture should parse"),
        );
        merge_graph(
            &mut graph,
            lvm::normalize_lvm_json(
                LVM_BACKED_VDO_PVS,
                LVM_BACKED_VDO_VGS,
                LVM_BACKED_VDO_LVS,
                Some(LVM_BACKED_VDO_SEGMENTS),
            )
            .expect("LVM-backed VDO LVM fixture should parse"),
        );

        let runtime_vdo = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "vdo:vgvdo-vdoarchive")
            .expect("native VDO runtime node should exist");
        assert_eq!(runtime_vdo.kind, NodeKind::VdoVolume);
        assert_eq!(
            runtime_vdo.path.as_deref(),
            Some("/dev/mapper/vgvdo-vdoarchive")
        );
        assert_eq!(runtime_vdo.size_bytes, Some(2_199_023_255_552));
        assert_has_property(
            runtime_vdo,
            "vdo.storage-device",
            "/dev/mapper/vgvdo-vdopool",
        );
        assert_has_property(runtime_vdo, "vdo.write-policy", "async");
        assert_has_property(runtime_vdo, "vdo.use-percent", "25");
        assert_has_property(runtime_vdo, "vdo.space-saving-percent", "68");
        assert_has_property(runtime_vdo, "vdo.operating-mode", "normal");
        assert_has_property(runtime_vdo, "vdo.data-blocks-used-bytes", "402653184");
        assert_has_property(runtime_vdo, "vdo.overhead-blocks-used-bytes", "67108864");
        assert_has_property(runtime_vdo, "vdo.logical-blocks-used-bytes", "2147483648");

        let lvm_vdo = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "lvm-lv:vgvdo/vdoarchive")
            .expect("LVM VDO logical volume should exist");
        assert_eq!(lvm_vdo.kind, NodeKind::VdoVolume);
        assert_eq!(
            lvm_vdo.path.as_deref(),
            Some("/dev/mapper/vgvdo-vdoarchive")
        );
        assert_eq!(lvm_vdo.identity.uuid.as_deref(), Some("vdo-lv-uuid"));
        assert_has_property(lvm_vdo, "lvm.layout", "vdo");
        assert_has_property(lvm_vdo, "lvm.vdo-operating-mode", "normal");
        assert_has_property(lvm_vdo, "lvm.vdo-compression-state", "online");
        assert_has_property(lvm_vdo, "lvm.vdo-index-state", "online");
        assert_has_property(lvm_vdo, "lvm.vdo-saving-percent", "68.00");

        let pool = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "lvm-lv:vgvdo/vdopool")
            .expect("LVM VDO pool backing LV should exist");
        assert_eq!(pool.kind, NodeKind::LvmLogicalVolume);
        assert_has_property(pool, "lvm.role", "private");

        let segment = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "lvm-seg:vgvdo/vdoarchive:0")
            .expect("LVM VDO segment should exist");
        assert_eq!(segment.kind, NodeKind::LvmSegment);
        assert_has_property(segment, "lvm.segment-type", "vdo");
        assert_has_property(segment, "lvm.vdo-compression", "enabled");
        assert_has_property(segment, "lvm.vdo-deduplication", "enabled");
        assert_has_property(segment, "lvm.vdo-write-policy", "auto");
        assert_has_property(segment, "lvm.vdo-use-metadata-hints", "enabled");

        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "block:/dev/mapper/vgvdo-vdopool"
                && edge.to.0 == "vdo:vgvdo-vdoarchive"
                && edge.relationship == Relationship::Backs
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "lvm-lv:vgvdo/vdoarchive"
                && edge.to.0 == "lvm-lv:vgvdo/vdopool"
                && edge.relationship == Relationship::DependsOn
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "lvm-seg:vgvdo/vdoarchive:0"
                && edge.to.0 == "lvm-lv:vgvdo/vdopool"
                && edge.relationship == Relationship::DependsOn
        }));
    }

    #[test]
    fn vdo_pressure_fixture_preserves_rebuild_policy_and_failure_state() {
        let mut graph = StorageGraph::empty();
        merge_graph(
            &mut graph,
            vdo::normalize_vdo_status(VDO_PRESSURE_STATUS)
                .expect("VDO pressure status fixture should parse"),
        );
        merge_graph(
            &mut graph,
            vdo::normalize_vdostats_table(VDO_PRESSURE_STATS)
                .expect("VDO pressure stats fixture should parse"),
        );
        merge_graph(
            &mut graph,
            vdo::normalize_vdostats_verbose(VDO_PRESSURE_VERBOSE)
                .expect("VDO pressure verbose fixture should parse"),
        );

        let pressure = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "vdo:archive-pressure")
            .expect("pressure VDO runtime node should exist");
        assert_eq!(pressure.kind, NodeKind::VdoVolume);
        assert_eq!(
            pressure.path.as_deref(),
            Some("/dev/mapper/archive-pressure")
        );
        assert_eq!(pressure.size_bytes, Some(8_796_093_022_208));
        assert_has_property(
            pressure,
            "vdo.storage-device",
            "/dev/disk/by-id/scsi-vdo-pressure",
        );
        assert_has_property(pressure, "vdo.physical-space-status", "near-full");
        assert_has_property(pressure, "vdo.operating-mode", "recovering");
        assert_has_property(pressure, "vdo.index-state", "rebuilding");
        assert_has_property(pressure, "vdo.index-rebuild-progress", "42%");
        assert_has_property(pressure, "vdo.compression", "disabled");
        assert_has_property(pressure, "vdo.compression-state", "offline");
        assert_has_property(pressure, "vdo.deduplication-state", "online");
        assert_has_property(pressure, "vdo.configured-write-policy", "sync");
        assert_has_property(pressure, "vdo.write-policy", "async");
        assert_has_property(pressure, "vdo.last-start-result", "failed");
        assert_has_property(pressure, "vdo.last-stop-result", "timeout");
        assert_has_property(pressure, "vdo.use-percent", "95");
        assert_has_property(pressure, "vdo.space-saving-percent", "12");
        assert_has_property(pressure, "vdo.data-blocks-used-bytes", "8160436224");
        assert_has_property(pressure, "vdo.overhead-blocks-used-bytes", "838860800");

        let stopped = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "vdo:archive-stopped")
            .expect("stopped VDO runtime node should exist");
        assert_eq!(stopped.kind, NodeKind::VdoVolume);
        assert_has_property(stopped, "vdo.operating-mode", "read-only");
        assert_has_property(stopped, "vdo.vdo-service-state", "stopped");
        assert_has_property(stopped, "vdo.deduplication", "disabled");
        assert_has_property(stopped, "vdo.deduplication-state", "offline");
        assert_has_property(stopped, "vdo.physical-space-status", "full");
        assert_has_property(stopped, "vdo.last-start-result", "device busy");
        assert_has_property(stopped, "vdo.last-stop-result", "failed");
        assert_has_property(stopped, "vdo.space-saving-percent", "0");

        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "block:/dev/disk/by-id/scsi-vdo-pressure"
                && edge.to.0 == "vdo:archive-pressure"
                && edge.relationship == Relationship::Backs
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "block:/dev/disk/by-id/scsi-vdo-stopped"
                && edge.to.0 == "vdo:archive-stopped"
                && edge.relationship == Relationship::Backs
        }));
    }

    #[test]
    fn encrypted_degraded_array_fixture_links_mdraid_and_luks_metadata() {
        let mut graph = StorageGraph::empty();
        merge_graph(
            &mut graph,
            mdraid::normalize_mdstat(ENCRYPTED_DEGRADED_MDSTAT)
                .expect("degraded mdstat fixture should parse"),
        );
        merge_graph(
            &mut graph,
            cryptsetup::normalize_cryptsetup_status(
                "/dev/mapper/cryptraid",
                ENCRYPTED_DEGRADED_CRYPT_STATUS,
            )
            .expect("cryptsetup status fixture should parse"),
        );
        merge_graph(
            &mut graph,
            cryptsetup::normalize_luks_dump(
                "/dev/disk/by-uuid/luks-raid-uuid",
                ENCRYPTED_DEGRADED_LUKS_DUMP,
            )
            .expect("LUKS header fixture should parse"),
        );

        let array = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "md:/dev/md127")
            .expect("degraded MD array should exist");
        assert_eq!(array.kind, NodeKind::MdRaid);
        assert_eq!(array.size_bytes, Some(2_147_483_648));
        assert_has_property(array, "md.mdstat-level", "raid1");
        assert_has_property(array, "md.mdstat-devices", "2/1");
        assert_has_property(array, "md.mdstat-health", "U_");
        assert_has_property(array, "md.mdstat-progress", "recovery");
        assert_has_property(array, "md.mdstat-progress-percent", "8.5%");

        let failed_member = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/nvme1n1p2")
            .expect("failed MD member should exist");
        assert_eq!(failed_member.kind, NodeKind::Partition);
        assert_has_property(failed_member, "md.mdstat-member-flags", "F");

        let mapper = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/mapper/cryptraid")
            .expect("active LUKS mapper should exist");
        assert_eq!(mapper.kind, NodeKind::LuksContainer);
        assert_eq!(mapper.path.as_deref(), Some("/dev/mapper/cryptraid"));
        assert_eq!(mapper.identity.uuid.as_deref(), Some("luks-raid-uuid"));
        assert_eq!(mapper.size_bytes, Some(17_146_314_752));
        assert_has_property(mapper, "cryptsetup.active", "true");
        assert_has_property(mapper, "cryptsetup.in-use", "true");
        assert_has_property(mapper, "cryptsetup.cipher", "aes-xts-plain64");

        let header = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/disk/by-uuid/luks-raid-uuid")
            .expect("LUKS header node on MD array should exist");
        assert_eq!(header.kind, NodeKind::LuksContainer);
        assert_eq!(header.identity.label.as_deref(), Some("encrypted-md-root"));
        assert_has_property(header, "cryptsetup.luks-version", "2");
        assert_has_property(header, "cryptsetup.luks-subsystem", "disk-nix-fixture");
        assert_has_property(header, "cryptsetup.luks-keyslot-count", "1");
        assert_has_property(header, "cryptsetup.luks-token-0-type", "systemd-tpm2");

        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "block:/dev/md127"
                && edge.to.0 == "block:/dev/mapper/cryptraid"
                && edge.relationship == Relationship::Backs
        }));
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|edge| {
                    edge.to.0 == "md:/dev/md127" && edge.relationship == Relationship::MemberOf
                })
                .count(),
            2
        );
    }

    fn merge_graph(target: &mut StorageGraph, source: StorageGraph) {
        for node in source.nodes {
            target.add_node(node);
        }
        for edge in source.edges {
            target.add_edge(edge);
        }
    }

    fn assert_has_property(node: &disk_nix_model::Node, key: &str, value: &str) {
        assert!(
            node.properties
                .iter()
                .any(|property| property.key == key && property.value == value),
            "{} should have property {key}={value}",
            node.id.0
        );
    }

    #[test]
    fn probe_reports_expose_structured_issue_categories() {
        let reports = vec![
            ProbeReport {
                adapter: "zfs".to_string(),
                status: ProbeStatus::Unavailable,
                message: Some("zpool not found or failed to run: No such file".to_string()),
            },
            ProbeReport {
                adapter: "lvm".to_string(),
                status: ProbeStatus::Partial,
                message: Some(
                    "must be root or have sufficient privileges to read device mapper state"
                        .to_string(),
                ),
            },
            ProbeReport {
                adapter: "lsblk".to_string(),
                status: ProbeStatus::Failed,
                message: Some("expected field blockdevices".to_string()),
            },
            ProbeReport {
                adapter: "findmnt".to_string(),
                status: ProbeStatus::Failed,
                message: Some("findmnt returned exit status 1".to_string()),
            },
            ProbeReport {
                adapter: "findmnt".to_string(),
                status: ProbeStatus::Available,
                message: Some("normalized 3 graph nodes".to_string()),
            },
            ProbeReport {
                adapter: "iscsi".to_string(),
                status: ProbeStatus::Partial,
                message: Some("configured node database is inaccessible".to_string()),
            },
            ProbeReport {
                adapter: "nvme".to_string(),
                status: ProbeStatus::Failed,
                message: Some("invalid JSON from nvme list".to_string()),
            },
        ];

        assert_eq!(reports[0].category(), ProbeIssueCategory::MissingTool);
        assert_eq!(reports[1].category(), ProbeIssueCategory::PermissionDenied);
        assert_eq!(reports[2].category(), ProbeIssueCategory::ParseFailed);
        assert_eq!(reports[3].category(), ProbeIssueCategory::CommandFailed);
        assert_eq!(reports[4].category(), ProbeIssueCategory::None);
        assert_eq!(reports[5].category(), ProbeIssueCategory::InaccessibleData);
        assert_eq!(reports[6].category(), ProbeIssueCategory::ParseFailed);
        assert!(
            reports[0]
                .remediation()
                .iter()
                .any(|item| { item.contains("pkgs.zfs") })
        );
        assert!(
            reports[1]
                .remediation()
                .iter()
                .any(|item| { item.contains("device-mapper state") })
        );
        assert!(
            reports[2]
                .remediation()
                .iter()
                .any(|item| { item.contains("fixture coverage") })
        );
        assert!(
            reports[3]
                .remediation()
                .iter()
                .any(|item| { item.contains("exit status") })
        );
        assert!(reports[4].remediation().is_empty());
        assert!(
            reports[5]
                .remediation()
                .iter()
                .any(|item| { item.contains("iscsid") || item.contains("open-iscsi") })
        );
        assert!(
            reports[6]
                .remediation()
                .iter()
                .any(|item| { item.contains("nvme-cli") })
        );

        let json = serde_json::to_string(&reports).expect("reports should serialize");
        assert!(json.contains(r#""category":"missing-tool""#));
        assert!(json.contains(r#""category":"permission-denied""#));
        assert!(json.contains(r#""category":"parse-failed""#));
        assert!(json.contains(r#""category":"command-failed""#));
        assert!(json.contains(r#""category":"inaccessible-data""#));
        assert!(json.contains(r#""category":"none""#));
        assert!(json.contains(r#""remediation":["#));
        assert!(json.contains("pkgs.zfs"));
        assert!(json.contains("device-mapper state"));
        assert!(json.contains("open-iscsi"));
        assert!(json.contains("nvme-cli"));
    }

    #[test]
    fn sub_adapters_inherit_domain_specific_remediation() {
        let cases = [
            ("nvme-id-ns", "nvme", "pkgs.nvme-cli", "nvme-cli JSON"),
            ("mdadm-scan", "mdraid", "pkgs.mdadm", "/proc/mdstat"),
            ("vdostats-verbose", "vdo", "pkgs.vdo", "VDO services"),
            ("zramctl", "zram", "pkgs.util-linux", "zram devices"),
            ("nfs-exports", "nfs", "pkgs.nfs-utils", "NFS mounts"),
        ];

        for (adapter, canonical, package, domain_hint) in cases {
            let metadata = adapter_remediation(adapter);
            assert_eq!(metadata.adapter, adapter);
            assert_eq!(metadata.canonical_adapter, canonical);
            assert!(
                metadata.nix_packages.iter().any(|item| item == package),
                "{adapter} should include package {package}"
            );
            assert!(
                metadata.data_hint.contains(domain_hint)
                    || metadata.parse_hint.contains(domain_hint)
                    || metadata.privilege_hint.contains(domain_hint),
                "{adapter} should include domain hint {domain_hint}"
            );

            let report = ProbeReport {
                adapter: adapter.to_string(),
                status: ProbeStatus::Unavailable,
                message: Some(format!("{adapter} not found or failed to run")),
            };
            let remediation = report.remediation();
            assert!(
                remediation.iter().any(|item| item.contains(package)),
                "{adapter} missing-tool remediation should include package {package}"
            );
        }
    }

    #[test]
    fn probe_issue_classifier_handles_common_real_world_messages() {
        for message in [
            "sh: zpool: command not found",
            "executable file not found in $PATH",
            "failed to run lvs: ENOENT",
            "No such file or directory (os error 2)",
        ] {
            assert_eq!(
                probe_category_for_message(message),
                ProbeIssueCategory::MissingTool
            );
        }

        for message in [
            "only root can use this command",
            "requires superuser privileges",
            "are you root?",
            "cannot open /dev/mapper/control: Operation not permitted",
        ] {
            assert_eq!(
                probe_category_for_message(message),
                ProbeIssueCategory::PermissionDenied
            );
        }
    }
