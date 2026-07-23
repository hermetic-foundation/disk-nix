    #[test]
    fn nfs_server_client_fixture_merges_mount_usage_and_export_policy() {
        let mut graph = StorageGraph::empty();
        merge_graph(
            &mut graph,
            findmnt::normalize_findmnt_json(NFS_SERVER_CLIENT_FINDMNT)
                .expect("NFS findmnt fixture should parse"),
        );
        merge_graph(
            &mut graph,
            nfs::normalize_nfsstat_mounts(NFS_SERVER_CLIENT_NFSSTAT)
                .expect("NFS mount fixture should parse"),
        );
        merge_graph(
            &mut graph,
            nfs::normalize_exportfs_verbose(NFS_SERVER_CLIENT_EXPORTFS)
                .expect("NFS export fixture should parse"),
        );

        let mount = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "mount:/mnt/projects")
            .expect("NFS client mount should exist");
        assert_eq!(mount.kind, NodeKind::NfsMount);
        assert_eq!(mount.path.as_deref(), Some("/mnt/projects"));
        assert_eq!(mount.size_bytes, Some(1_099_511_627_776));
        assert_eq!(
            mount.usage.as_ref().and_then(|usage| usage.used_bytes),
            Some(274_877_906_944)
        );
        assert_eq!(
            mount.usage.as_ref().and_then(|usage| usage.free_bytes),
            Some(824_633_720_832)
        );
        assert_has_property(mount, "mount.source", "nas01.example:/exports/projects");
        assert_has_property(mount, "mount.read-write", "true");
        assert_has_property(mount, "nfs.source", "nas01.example:/exports/projects");
        assert_has_property(mount, "nfs.server", "nas01.example");
        assert_has_property(mount, "nfs.export", "/exports/projects");
        assert_has_property(mount, "nfs.vers", "4.2");
        assert_has_property(mount, "nfs.sec", "krb5p");
        assert_has_property(mount, "nfs.clientaddr", "10.20.30.40");
        assert_has_property(mount, "nfs.local-lock", "none");
        assert_has_property(mount, "nfs.age", "456");

        let mounted_export = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "nfs-export:nas01.example:/exports/projects")
            .expect("NFS source export should exist");
        assert_eq!(mounted_export.kind, NodeKind::NfsExport);
        assert_has_property(mounted_export, "nfs.server", "nas01.example");
        assert_has_property(mounted_export, "nfs.export", "/exports/projects");

        let subnet_export = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "nfs-export:/exports/projects:10.20.0.0/16")
            .expect("NFS exportfs subnet export should exist");
        assert_eq!(subnet_export.kind, NodeKind::NfsExport);
        assert_eq!(subnet_export.path.as_deref(), Some("/exports/projects"));
        assert_has_property(subnet_export, "nfs.export-client", "10.20.0.0/16");
        assert_has_property(subnet_export, "nfs.export-option-rw", "true");
        assert_has_property(subnet_export, "nfs.export-option-sec", "krb5p");
        assert_has_property(subnet_export, "nfs.export-option-root-squash", "true");

        let ipv6_export = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "nfs-export:/exports/projects:[2001:db8:120::]/64")
            .expect("NFS exportfs IPv6 export should exist");
        assert_has_property(ipv6_export, "nfs.export-client", "[2001:db8:120::]/64");
        assert_has_property(ipv6_export, "nfs.export-option-ro", "true");

        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "nfs-export:nas01.example:/exports/projects"
                && edge.to.0 == "mount:/mnt/projects"
                && edge.relationship == Relationship::MountedAt
        }));
    }

    #[test]
    fn shared_storage_fabric_fixture_links_iscsi_luns_and_multipath_paths() {
        let mut graph = StorageGraph::empty();
        merge_graph(
            &mut graph,
            iscsi::normalize_iscsi_session_output(SHARED_ISCSI_SESSION)
                .expect("iSCSI session fixture should parse"),
        );
        merge_graph(
            &mut graph,
            iscsi::normalize_iscsi_node_output(SHARED_ISCSI_NODE)
                .expect("iSCSI node fixture should parse"),
        );
        merge_graph(
            &mut graph,
            lsscsi::normalize_lsscsi_list_output(SHARED_LSSCSI_LIST)
                .expect("lsscsi list fixture should parse"),
        );
        merge_graph(
            &mut graph,
            lsscsi::normalize_lsscsi_transport_output(SHARED_LSSCSI_TRANSPORT)
                .expect("lsscsi transport fixture should parse"),
        );
        merge_graph(
            &mut graph,
            lsscsi::normalize_lsscsi_unit_output(SHARED_LSSCSI_UNIT)
                .expect("lsscsi unit fixture should parse"),
        );
        merge_graph(
            &mut graph,
            multipath::normalize_multipath_output(SHARED_MULTIPATH)
                .expect("multipath fixture should parse"),
        );

        let session = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "iscsi-session:42")
            .expect("logged-in iSCSI session should exist");
        assert_eq!(session.kind, NodeKind::IscsiSession);
        assert_has_property(session, "iscsi.session-state", "LOGGED_IN");
        assert_has_property(session, "iscsi.portal-address", "10.0.0.10");
        assert_has_property(session, "iscsi.host-number", "2");

        let target = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "iscsi-target:iqn.2026-06.example:storage.shared")
            .expect("configured iSCSI target should exist");
        assert_eq!(target.kind, NodeKind::IscsiTarget);
        assert_has_property(target, "iscsi.node-startup", "automatic");
        assert_has_property(target, "iscsi.node-auth-password-configured", "true");
        assert_has_property(target, "iscsi.node-auth-password-in-configured", "true");
        assert!(
            !target.properties.iter().any(|property| {
                property.value == "outbound-secret" || property.value == "inbound-secret"
            }),
            "configured iSCSI node normalization must not leak CHAP secrets"
        );

        let scsi_lun = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "scsi-lun:2:0:0:1")
            .expect("host-visible SCSI LUN should exist");
        assert_eq!(scsi_lun.kind, NodeKind::Lun);
        assert_eq!(scsi_lun.size_bytes, Some(100_000_000_000));
        assert_has_property(
            scsi_lun,
            "scsi.transport",
            "iscsi:iqn.2026-06.example:storage.shared,t,0x1",
        );
        assert_has_property(scsi_lun, "scsi.queue-depth", "128");
        assert_eq!(
            scsi_lun.identity.wwn.as_deref(),
            Some("/dev/disk/by-id/wwn-0x600508b400105e210000900000490000")
        );

        let map = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "multipath:mpatha")
            .expect("multipath map should exist");
        assert_eq!(map.kind, NodeKind::MultipathDevice);
        assert_eq!(map.size_bytes, Some(100_000_000_000));
        assert_has_property(map, "multipath.wwid", "3600508b400105e210000900000490000");
        assert_has_property(map, "multipath.features", "1 queue_if_no_path");

        let path_sdb = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/sdb")
            .expect("first shared-storage path should exist");
        assert_eq!(path_sdb.kind, NodeKind::PhysicalDisk);
        assert_has_property(path_sdb, "scsi.address", "2:0:0:1");
        assert_has_property(path_sdb, "multipath.group-status", "active");
        assert_has_property(path_sdb, "multipath.path-flags", "ghost");

        let path_sdc = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/sdc")
            .expect("second shared-storage path should exist");
        assert_has_property(path_sdc, "scsi.address", "3:0:0:1");
        assert_has_property(path_sdc, "multipath.group-status", "enabled");
        assert_has_property(path_sdc, "multipath.path-flags", "faulty shaky");

        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "iscsi-session:42"
                && edge.to.0 == "iscsi-target:iqn.2026-06.example:storage.shared"
                && edge.relationship == Relationship::ImportedFrom
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "iscsi-lun:iqn.2026-06.example:storage.shared:1"
                && edge.to.0 == "block:/dev/sdb"
                && edge.relationship == Relationship::Backs
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "scsi-lun:2:0:0:1"
                && edge.to.0 == "block:/dev/sdb"
                && edge.relationship == Relationship::Backs
        }));
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|edge| {
                    edge.to.0 == "multipath:mpatha" && edge.relationship == Relationship::Backs
                })
                .count(),
            2
        );
    }

    #[test]
    fn fibre_channel_multipath_fixture_preserves_transport_and_path_state() {
        let mut graph = StorageGraph::empty();
        merge_graph(
            &mut graph,
            lsscsi::normalize_lsscsi_list_output(FC_LSSCSI_LIST)
                .expect("FC lsscsi list fixture should parse"),
        );
        merge_graph(
            &mut graph,
            lsscsi::normalize_lsscsi_transport_output(FC_LSSCSI_TRANSPORT)
                .expect("FC lsscsi transport fixture should parse"),
        );
        merge_graph(
            &mut graph,
            lsscsi::normalize_lsscsi_unit_output(FC_LSSCSI_UNIT)
                .expect("FC lsscsi unit fixture should parse"),
        );
        merge_graph(
            &mut graph,
            multipath::normalize_multipath_output(FC_MULTIPATH)
                .expect("FC multipath fixture should parse"),
        );

        let primary_lun = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "scsi-lun:6:0:2:12")
            .expect("primary FC LUN should exist");
        assert_eq!(primary_lun.kind, NodeKind::Lun);
        assert_eq!(primary_lun.size_bytes, Some(2_000_000_000_000));
        assert_has_property(
            primary_lun,
            "scsi.transport",
            "fc:0x5006016841e0abcd,0x5006016041e0abcd",
        );
        assert_has_property(
            primary_lun,
            "scsi.unit-name",
            "36006016041e05d00c8b7f0a0d7a4ee11",
        );
        assert_eq!(
            primary_lun.identity.wwn.as_deref(),
            Some("/dev/disk/by-id/wwn-0x6006016041e05d00c8b7f0a0d7a4ee11")
        );

        let standby_lun = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "scsi-lun:7:0:3:12")
            .expect("standby FC LUN should exist");
        assert_has_property(standby_lun, "scsi.state", "blocked");
        assert_has_property(
            standby_lun,
            "scsi.transport",
            "fc:0x5006016841e0abce,0x5006016041e0abce",
        );

        let map = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "multipath:mpathfc")
            .expect("FC multipath map should exist");
        assert_eq!(map.kind, NodeKind::MultipathDevice);
        assert_eq!(map.size_bytes, Some(2_000_000_000_000));
        assert_has_property(map, "multipath.wwid", "36006016041e05d00c8b7f0a0d7a4ee11");
        assert_has_property(map, "multipath.vendor-product", "DGC,VRAID");
        assert_has_property(map, "multipath.hwhandler", "1 alua");
        assert_has_property(
            map,
            "multipath.features",
            "2 queue_if_no_path pg_init_retries 50",
        );

        let active_path = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/sdd")
            .expect("active FC path should exist");
        assert_eq!(active_path.kind, NodeKind::PhysicalDisk);
        assert_has_property(
            active_path,
            "scsi.transport",
            "fc:0x5006016841e0abcd,0x5006016041e0abcd",
        );
        assert_has_property(active_path, "multipath.group-status", "active");
        assert_has_property(active_path, "multipath.checker-state", "ready");
        assert_has_property(active_path, "multipath.online-state", "running");

        let standby_path = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/sde")
            .expect("standby FC path should exist");
        assert_has_property(
            standby_path,
            "scsi.transport",
            "fc:0x5006016841e0abce,0x5006016041e0abce",
        );
        assert_has_property(standby_path, "multipath.group-status", "enabled");
        assert_has_property(standby_path, "multipath.dm-state", "failed");
        assert_has_property(standby_path, "multipath.checker-state", "faulty");
        assert_has_property(standby_path, "multipath.online-state", "offline");
        assert_has_property(standby_path, "multipath.path-flags", "standby");

        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "scsi-lun:6:0:2:12"
                && edge.to.0 == "block:/dev/sdd"
                && edge.relationship == Relationship::Backs
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "scsi-lun:7:0:3:12"
                && edge.to.0 == "block:/dev/sde"
                && edge.relationship == Relationship::Backs
        }));
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|edge| {
                    edge.to.0 == "multipath:mpathfc" && edge.relationship == Relationship::Backs
                })
                .count(),
            2
        );
    }

    #[test]
    fn fibre_channel_zoned_fixture_preserves_adapter_alua_and_failed_paths() {
        let mut graph = StorageGraph::empty();
        merge_graph(
            &mut graph,
            lsscsi::normalize_lsscsi_list_output(FC_ZONED_LSSCSI_LIST)
                .expect("zoned FC lsscsi list fixture should parse"),
        );
        merge_graph(
            &mut graph,
            lsscsi::normalize_lsscsi_transport_output(FC_ZONED_LSSCSI_TRANSPORT)
                .expect("zoned FC lsscsi transport fixture should parse"),
        );
        merge_graph(
            &mut graph,
            lsscsi::normalize_lsscsi_unit_output(FC_ZONED_LSSCSI_UNIT)
                .expect("zoned FC lsscsi unit fixture should parse"),
        );
        merge_graph(
            &mut graph,
            multipath::normalize_multipath_output(FC_ZONED_MULTIPATH)
                .expect("zoned FC multipath fixture should parse"),
        );

        let map = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "multipath:mpathfczone")
            .expect("zoned FC multipath map should exist");
        assert_eq!(map.kind, NodeKind::MultipathDevice);
        assert_eq!(map.size_bytes, Some(4_000_000_000_000));
        assert_has_property(map, "multipath.wwid", "3600a098038314f6f2b5d514d43594c33");
        assert_has_property(map, "multipath.hwhandler", "1 alua");
        assert_has_property(
            map,
            "multipath.features",
            "2 queue_if_no_path retain_attached_hw_handler",
        );

        let fabric_a_optimized = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/sdf")
            .expect("fabric A optimized path should exist");
        assert_has_property(
            fabric_a_optimized,
            "scsi.fc-initiator-wwpn",
            "0x100000109babcdef",
        );
        assert_has_property(
            fabric_a_optimized,
            "scsi.fc-target-wwpn",
            "0x500a098299aabb01",
        );
        assert_has_property(fabric_a_optimized, "multipath.group-status", "active");
        assert_has_property(fabric_a_optimized, "multipath.path-flags", "optimized");
        let fabric_a_lun = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "scsi-lun:8:0:1:23")
            .expect("fabric A optimized LUN should exist");
        assert_has_property(fabric_a_lun, "scsi.fabric-name", "0x1000000533fedcba");

        let fabric_b_optimized = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/sdg")
            .expect("fabric B optimized path should exist");
        assert_has_property(
            fabric_b_optimized,
            "scsi.fc-initiator-wwpn",
            "0x100000109babcd00",
        );
        assert_has_property(
            fabric_b_optimized,
            "scsi.fc-target-wwpn",
            "0x500a098399aabb01",
        );
        assert_has_property(fabric_b_optimized, "multipath.path-flags", "optimized");
        let fabric_b_lun = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "scsi-lun:9:0:2:23")
            .expect("fabric B optimized LUN should exist");
        assert_has_property(fabric_b_lun, "scsi.fabric-name", "0x1000000533fedcbb");

        let nonoptimized = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/sdh")
            .expect("non-optimized ALUA path should exist");
        assert_has_property(nonoptimized, "multipath.group-status", "enabled");
        assert_has_property(nonoptimized, "multipath.path-flags", "nonoptimized");
        assert_has_property(nonoptimized, "scsi.fc-target-wwpn", "0x500a098299aabb02");

        let failed = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/sdi")
            .expect("failed standby FC path should exist");
        assert_has_property(failed, "multipath.dm-state", "failed");
        assert_has_property(failed, "multipath.checker-state", "faulty");
        assert_has_property(failed, "multipath.online-state", "offline");
        assert_has_property(failed, "multipath.path-flags", "standby");
        assert_has_property(failed, "scsi.fc-target-wwpn", "0x500a098399aabb02");
        let failed_lun = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "scsi-lun:11:0:4:23")
            .expect("failed standby FC LUN should exist");
        assert_has_property(failed_lun, "scsi.device-blocked", "1");
        assert_has_property(failed_lun, "scsi.state", "blocked");

        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|edge| {
                    edge.to.0 == "multipath:mpathfczone" && edge.relationship == Relationship::Backs
                })
                .count(),
            4
        );
    }

    #[test]
    fn hardware_array_fixture_preserves_ses_failures_and_identity_drift() {
        let mut graph = StorageGraph::empty();
        merge_graph(
            &mut graph,
            lsscsi::normalize_lsscsi_list_output(HARDWARE_ARRAY_LSSCSI_LIST)
                .expect("hardware array lsscsi list fixture should parse"),
        );
        merge_graph(
            &mut graph,
            lsscsi::normalize_lsscsi_transport_output(HARDWARE_ARRAY_LSSCSI_TRANSPORT)
                .expect("hardware array lsscsi transport fixture should parse"),
        );
        merge_graph(
            &mut graph,
            lsscsi::normalize_lsscsi_unit_output(HARDWARE_ARRAY_LSSCSI_UNIT)
                .expect("hardware array lsscsi unit fixture should parse"),
        );
        merge_graph(
            &mut graph,
            multipath::normalize_multipath_output(HARDWARE_ARRAY_MULTIPATH)
                .expect("hardware array multipath fixture should parse"),
        );

        let healthy_enclosure = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "scsi-lun:12:0:0:0")
            .expect("healthy SES enclosure should exist");
        assert_eq!(healthy_enclosure.kind, NodeKind::Lun);
        assert_has_property(healthy_enclosure, "scsi.peripheral-type", "enclosu");
        assert_has_property(
            healthy_enclosure,
            "scsi.enclosure-identifier",
            "0x5000c500dead0001",
        );
        assert_has_property(healthy_enclosure, "scsi.ses-status", "ok");

        let failed_enclosure = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "scsi-lun:12:0:1:0")
            .expect("failed SES enclosure should exist");
        assert_eq!(failed_enclosure.kind, NodeKind::Lun);
        assert_has_property(failed_enclosure, "scsi.device-blocked", "1");
        assert_has_property(failed_enclosure, "scsi.element-status", "critical");
        assert_has_property(failed_enclosure, "scsi.fault-code", "over_temperature");
        assert_has_property(failed_enclosure, "scsi.ses-status", "failed");
        assert_has_property(failed_enclosure, "scsi.state", "blocked");

        assert!(
            !graph
                .nodes
                .iter()
                .any(|node| node.id.0 == "block:-" || node.path.as_deref() == Some("-")),
            "SES enclosure records must not create placeholder block devices"
        );

        let preferred_lun = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "scsi-lun:12:0:2:7")
            .expect("preferred vendor LUN should exist");
        assert_eq!(preferred_lun.kind, NodeKind::Lun);
        assert_eq!(preferred_lun.size_bytes, Some(8_000_000_000_000));
        assert_has_property(preferred_lun, "scsi.array-serial", "ME5SN12345");
        assert_has_property(preferred_lun, "scsi.storage-pool", "pool-a");
        assert_has_property(preferred_lun, "scsi.volume-id", "vol-prod");
        assert_has_property(preferred_lun, "scsi.vendor-lun-id", "vdisk-prod-77");
        assert_has_property(preferred_lun, "scsi.target-port-group", "preferred-a");
        assert_has_property(
            preferred_lun,
            "scsi.logical-unit-id",
            "600c0ff0005a4bcd0000000000000077",
        );

        let replacement_lun = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "scsi-lun:13:0:2:7")
            .expect("replacement vendor LUN path should exist");
        assert_has_property(
            replacement_lun,
            "scsi.vendor-lun-id",
            "vdisk-prod-77-replaced",
        );
        assert_has_property(replacement_lun, "scsi.target-port-group", "nonpreferred-b");
        assert_has_property(
            replacement_lun,
            "scsi.logical-unit-id",
            "600c0ff0005a4bcd0000000000000088",
        );

        let preferred_path = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/sdj")
            .expect("preferred array path should exist");
        assert_has_property(preferred_path, "scsi.transport", "sas:0x5000c500dead0177");
        assert_has_property(
            preferred_path,
            "scsi.unit-name",
            "3600c0ff0005a4bcd0000000000000077",
        );
        assert_has_property(preferred_path, "multipath.path-flags", "preferred");

        let drifted_path = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/sdk")
            .expect("drifted array path should exist");
        assert_has_property(drifted_path, "scsi.transport", "sas:0x5000c500dead0277");
        assert_has_property(
            drifted_path,
            "scsi.unit-name",
            "3600c0ff0005a4bcd0000000000000088",
        );
        assert_has_property(
            drifted_path,
            "multipath.path-flags",
            "nonpreferred identity-drift",
        );

        let map = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "multipath:mpatharray")
            .expect("array-backed multipath map should exist");
        assert_eq!(map.kind, NodeKind::MultipathDevice);
        assert_has_property(map, "multipath.vendor-product", "DELL,ME5 VirtualDisk");
        assert_has_property(map, "multipath.wwid", "3600c0ff0005a4bcd0000000000000099");
        assert_ne!(
            map.properties
                .iter()
                .find(|property| property.key == "multipath.wwid")
                .map(|property| property.value.as_str()),
            drifted_path
                .identity
                .wwn
                .as_deref()
                .map(|value| value.trim_start_matches("/dev/disk/by-id/scsi-"))
        );

        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "scsi-lun:12:0:2:7"
                && edge.to.0 == "block:/dev/sdj"
                && edge.relationship == Relationship::Backs
        }));
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|edge| {
                    edge.to.0 == "multipath:mpatharray" && edge.relationship == Relationship::Backs
                })
                .count(),
            2
        );
    }

    #[test]
    fn nvme_tcp_multipath_fixture_preserves_native_path_state() {
        let mut graph = StorageGraph::empty();
        merge_graph(
            &mut graph,
            nvme::normalize_nvme_list_json(NVME_TCP_MULTIPATH_LIST)
                .expect("NVMe/TCP list fixture should parse"),
        );
        merge_graph(
            &mut graph,
            nvme::normalize_nvme_subsystems_json(NVME_TCP_MULTIPATH_SUBSYSTEMS)
                .expect("NVMe/TCP subsystem fixture should parse"),
        );
        merge_graph(
            &mut graph,
            multipath::normalize_multipath_output(NVME_TCP_MULTIPATH)
                .expect("NVMe/TCP multipath fixture should parse"),
        );

        let namespace = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/nvme4n1")
            .expect("primary NVMe/TCP namespace should exist");
        assert_eq!(namespace.kind, NodeKind::NvmeNamespace);
        assert_eq!(namespace.size_bytes, Some(800_000_000_000));
        assert_eq!(
            namespace.usage.as_ref().and_then(|usage| usage.used_bytes),
            Some(640_000_000_000)
        );
        assert_has_property(namespace, "nvme.transport", "tcp");
        assert_has_property(namespace, "nvme.ana-state", "optimized");
        assert_has_property(namespace, "multipath.group-status", "active");
        assert_has_property(namespace, "multipath.path-flags", "optimized");

        let failed_namespace = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/nvme5n1")
            .expect("failed NVMe/TCP namespace path should exist");
        assert_eq!(failed_namespace.kind, NodeKind::NvmeNamespace);
        assert_has_property(failed_namespace, "nvme.controller", "nvme5");
        assert_has_property(failed_namespace, "multipath.group-status", "enabled");
        assert_has_property(failed_namespace, "multipath.dm-state", "failed");
        assert_has_property(failed_namespace, "multipath.checker-state", "faulty");
        assert_has_property(failed_namespace, "multipath.online-state", "offline");
        assert_has_property(failed_namespace, "multipath.path-flags", "inaccessible");

        let controller = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "nvme-controller:nvme4")
            .expect("live NVMe/TCP controller should exist");
        assert_eq!(controller.kind, NodeKind::NvmeController);
        assert_has_property(controller, "nvme.transport", "tcp");
        assert_has_property(controller, "nvme.host-iface", "ens5f0");
        assert_has_property(controller, "nvme.path-state", "live");
        assert_has_property(controller, "nvme.ana-state", "optimized");

        let failed_controller = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "nvme-controller:nvme5")
            .expect("failed NVMe/TCP controller should exist");
        assert_has_property(failed_controller, "nvme.host-iface", "ens5f1");
        assert_has_property(failed_controller, "nvme.path-state", "reconnecting");
        assert_has_property(failed_controller, "nvme.ana-state", "inaccessible");

        let map = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "multipath:mpathnvme")
            .expect("native NVMe multipath map should exist");
        assert_eq!(map.kind, NodeKind::MultipathDevice);
        assert_eq!(map.size_bytes, Some(800_000_000_000));
        assert_has_property(
            map,
            "multipath.wwid",
            "uuid.aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
        );
        assert_has_property(map, "multipath.vendor-product", "NVME,Array");

        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "nvme-subsystem:nvme-subsys4"
                && edge.to.0 == "nvme-controller:nvme4"
                && edge.relationship == Relationship::Contains
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "nvme-controller:nvme5"
                && edge.to.0 == "block:/dev/nvme5n1"
                && edge.relationship == Relationship::Contains
        }));
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|edge| {
                    edge.to.0 == "multipath:mpathnvme" && edge.relationship == Relationship::Backs
                })
                .count(),
            2
        );
    }

    #[test]
    fn nvme_of_mixed_fabric_fixture_preserves_sharing_and_path_churn() {
        let mut graph = StorageGraph::empty();
        merge_graph(
            &mut graph,
            nvme::normalize_nvme_list_json(NVME_OF_MIXED_LIST)
                .expect("mixed NVMe-oF list fixture should parse"),
        );
        merge_graph(
            &mut graph,
            nvme::normalize_nvme_subsystems_json(NVME_OF_MIXED_SUBSYSTEMS)
                .expect("mixed NVMe-oF subsystem fixture should parse"),
        );
        merge_graph(
            &mut graph,
            multipath::normalize_multipath_output(NVME_OF_MIXED_MULTIPATH)
                .expect("mixed NVMe-oF multipath fixture should parse"),
        );

        let rdma_namespace = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/nvme6n1")
            .expect("RDMA shared namespace path should exist");
        assert_eq!(rdma_namespace.kind, NodeKind::NvmeNamespace);
        assert_eq!(rdma_namespace.size_bytes, Some(1_200_000_000_000));
        assert_eq!(
            rdma_namespace.identity.uuid.as_deref(),
            Some("bbbbbbbb-cccc-dddd-eeee-ffffffffffff")
        );
        assert_eq!(
            rdma_namespace.identity.wwn.as_deref(),
            Some("bbbbbbbb11111111cccccccc22222222")
        );
        assert_has_property(rdma_namespace, "nvme.transport", "rdma");
        assert_has_property(rdma_namespace, "nvme.ana-state", "optimized");
        assert_has_property(rdma_namespace, "multipath.path-flags", "optimized");

        let fc_namespace = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/nvme7n1")
            .expect("FC shared namespace path should exist");
        assert_eq!(
            fc_namespace.identity.uuid.as_deref(),
            Some("bbbbbbbb-cccc-dddd-eeee-ffffffffffff")
        );
        assert_eq!(
            fc_namespace.identity.wwn.as_deref(),
            Some("bbbbbbbb11111111cccccccc22222222")
        );
        assert_has_property(fc_namespace, "nvme.transport", "fc");
        assert_has_property(fc_namespace, "nvme.ana-state", "non-optimized");
        assert_has_property(fc_namespace, "multipath.path-flags", "nonoptimized");

        let rdma_controller = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "nvme-controller:nvme6")
            .expect("RDMA/RoCE controller should exist");
        assert_has_property(rdma_controller, "nvme.transport", "rdma");
        assert_has_property(rdma_controller, "nvme.host-iface", "roce0");
        assert_has_property(rdma_controller, "nvme.path-state", "live");
        assert_has_property(rdma_controller, "nvme.ana-state", "optimized");

        let fc_controller = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "nvme-controller:nvme7")
            .expect("NVMe/FC controller should exist");
        assert_has_property(fc_controller, "nvme.transport", "fc");
        assert_has_property(fc_controller, "nvme.host-iface", "fc0");
        assert_has_property(fc_controller, "nvme.path-state", "reconnecting");
        assert_has_property(fc_controller, "nvme.ana-state", "non-optimized");

        let transitioning_controller = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "nvme-controller:nvme8")
            .expect("ANA transition controller should exist");
        assert_has_property(transitioning_controller, "nvme.host-iface", "roce1");
        assert_has_property(transitioning_controller, "nvme.path-state", "connecting");
        assert_has_property(transitioning_controller, "nvme.ana-state", "change");

        let lost_controller = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "nvme-controller:nvme9")
            .expect("lost NVMe/FC controller should exist");
        assert_has_property(lost_controller, "nvme.host-iface", "fc1");
        assert_has_property(lost_controller, "nvme.path-state", "lost");
        assert_has_property(lost_controller, "nvme.ana-state", "inaccessible");

        let inaccessible_namespace = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "block:/dev/nvme9n1")
            .expect("lost namespace path should exist");
        assert_has_property(inaccessible_namespace, "multipath.dm-state", "failed");
        assert_has_property(inaccessible_namespace, "multipath.checker-state", "faulty");
        assert_has_property(inaccessible_namespace, "multipath.online-state", "offline");
        assert_has_property(
            inaccessible_namespace,
            "multipath.path-flags",
            "inaccessible",
        );

        let map = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "multipath:mpathnvmemixed")
            .expect("mixed NVMe-oF multipath map should exist");
        assert_eq!(map.kind, NodeKind::MultipathDevice);
        assert_has_property(
            map,
            "multipath.wwid",
            "uuid.bbbbbbbb-cccc-dddd-eeee-ffffffffffff",
        );

        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "nvme-subsystem:nvme-subsys-mixed"
                && edge.to.0 == "nvme-controller:nvme8"
                && edge.relationship == Relationship::Contains
        }));
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|edge| {
                    edge.to.0 == "multipath:mpathnvmemixed"
                        && edge.relationship == Relationship::Backs
                })
                .count(),
            4
        );
    }
