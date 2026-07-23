#[test]
fn confirmation_file_accepts_exact_token_line() {
    assert!(confirmation_file_accepts("disk-nix confirm\n"));
    assert!(confirmation_file_accepts("# reviewed\ndisk-nix confirm\n"));
    assert!(confirmation_file_accepts("  disk-nix confirm  \n"));
}

#[test]
fn probe_status_output_includes_remediation_hints() {
    let reports = vec![ProbeReport {
        adapter: "lvm".to_string(),
        status: ProbeStatus::Partial,
        message: Some("permission denied while reading device mapper state".to_string()),
    }];
    let mut output = Vec::new();
    print_probe_reports(&mut output, &reports).expect("probe reports should render");
    let output = String::from_utf8(output).expect("probe status output is utf8");
    assert!(output.contains("permission-denied"));
    assert!(output.contains("remediation:"));
    assert!(output.contains("privileges"));
}

#[test]
fn probe_preflight_parses_os_release_fields() {
    let fields = parse_os_release(
        r#"
ID=nixos
VERSION_ID="26.05"
PRETTY_NAME="NixOS 26.05 (Hermetic)"
# ignored
"#,
    );

    assert_eq!(
        fields
            .iter()
            .find(|(key, _)| key == "ID")
            .map(|(_, value)| value.as_str()),
        Some("nixos")
    );
    assert_eq!(
        fields
            .iter()
            .find(|(key, _)| key == "VERSION_ID")
            .map(|(_, value)| value.as_str()),
        Some("26.05")
    );
    assert_eq!(
        fields
            .iter()
            .find(|(key, _)| key == "PRETTY_NAME")
            .map(|(_, value)| value.as_str()),
        Some("NixOS 26.05 (Hermetic)")
    );
}

#[test]
fn probe_preflight_tool_version_reports_missing_tools() {
    let report =
        storage_tool_version_report("disk-nix-definitely-missing-tool-for-test", &["--version"]);

    assert_eq!(
        report.tool,
        "disk-nix-definitely-missing-tool-for-test".to_string()
    );
    assert_eq!(report.status, ToolVersionStatus::Unavailable);
    assert!(report.version.is_none());
    assert!(report
        .message
        .as_deref()
        .is_some_and(|message| message.contains("not found")));
}

#[test]
fn probe_preflight_command_version_output_handles_common_variants() {
    let stdout = command_stdout_first_line("sh", &["-c", "printf 'tool 1.0\\n'"])
        .expect("stdout version text should parse");
    assert_eq!(stdout, "tool 1.0");

    let stderr = command_stdout_first_line("sh", &["-c", "printf 'tool 2.0\\n' >&2"])
        .expect("stderr version text should parse");
    assert_eq!(stderr, "tool 2.0");

    let empty = command_stdout_first_line("sh", &["-c", ":"])
        .expect_err("empty successful version output should fail preflight");
    assert!(empty.contains("returned no version text"));

    let nonzero = command_stdout_first_line("sh", &["-c", "printf 'bad version\\n' >&2; exit 2"])
        .expect_err("nonzero version command should fail preflight");
    assert!(nonzero.contains("failed with status"));
    assert!(nonzero.contains("bad version"));
}

#[test]
fn probe_preflight_human_output_includes_environment_and_tools() {
    let environment = ProbePreflightEnvironment {
        os_id: Some("nixos".to_string()),
        os_version_id: Some("26.05".to_string()),
        os_pretty_name: Some("NixOS 26.05".to_string()),
        kernel_release: Some("6.12.0".to_string()),
        effective_uid: Some("0".to_string()),
        tool_versions: vec![
            ToolVersionReport {
                tool: "lsblk".to_string(),
                status: ToolVersionStatus::Available,
                version: Some("lsblk from util-linux 2.41".to_string()),
                message: None,
            },
            ToolVersionReport {
                tool: "zpool".to_string(),
                status: ToolVersionStatus::Unavailable,
                version: None,
                message: Some("zpool not found or failed to run".to_string()),
            },
        ],
    };

    let mut output = Vec::new();
    print_probe_preflight_environment(&mut output, &environment)
        .expect("preflight environment renders");
    let output = String::from_utf8(output).expect("preflight output is utf8");
    assert!(output.contains("Preflight environment:"));
    assert!(output.contains("NixOS 26.05"));
    assert!(output.contains("effective-uid: 0"));
    assert!(output.contains("lsblk"));
    assert!(output.contains("zpool"));
    assert!(output.contains("unavailable"));

    let checks = probe_preflight_checks(&environment);
    let mut output = Vec::new();
    print_probe_preflight_checks(&mut output, &checks).expect("preflight checks render");
    let output = String::from_utf8(output).expect("preflight checks output is utf8");
    assert!(output.contains("Preflight checks:"));
    assert!(output.contains("status: degraded"));
    assert!(output.contains("missing-tools: zpool"));
    assert!(output.contains("remediation:"));
}

#[test]
fn probe_preflight_json_wraps_environment_and_reports() {
    let environment = ProbePreflightEnvironment {
        os_id: Some("nixos".to_string()),
        os_version_id: Some("26.05".to_string()),
        os_pretty_name: Some("NixOS 26.05".to_string()),
        kernel_release: Some("6.12.0".to_string()),
        effective_uid: Some("0".to_string()),
        tool_versions: vec![ToolVersionReport {
            tool: "lsblk".to_string(),
            status: ToolVersionStatus::Available,
            version: Some("lsblk from util-linux 2.41".to_string()),
            message: None,
        }],
    };
    let preflight_checks = probe_preflight_checks(&environment);
    let report = ProbeStatusPreflightReport {
        environment,
        preflight_checks,
        reports: vec![ProbeReport {
            adapter: "lsblk".to_string(),
            status: ProbeStatus::Available,
            message: Some("normalized graph nodes".to_string()),
        }],
    };

    let json = serde_json::to_value(&report).expect("preflight report serializes");
    assert_eq!(json["environment"]["osId"], "nixos");
    assert_eq!(json["environment"]["toolVersions"][0]["tool"], "lsblk");
    assert_eq!(json["preflightChecks"]["status"], "ready");
    assert_eq!(json["preflightChecks"]["root"], true);
    assert_eq!(json["preflightChecks"]["unavailableToolCount"], 0);
    assert!(json["preflightChecks"]["adapterRemediation"]
        .as_array()
        .is_some_and(|items| items.iter().any(|item| {
            item["adapter"] == "nvme-id-ns"
                && item["canonicalAdapter"] == "nvme"
                && item["nixPackages"].as_array().is_some_and(|packages| {
                    packages.iter().any(|package| package == "pkgs.nvme-cli")
                })
        })));
    assert_eq!(json["reports"][0]["adapter"], "lsblk");
    assert_eq!(json["reports"][0]["category"], "none");
}

#[test]
fn apply_receipt_wraps_report_with_invocation_metadata() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "root": {
                    "mountpoint": "/",
                    "fsType": "ext4",
                    "resizePolicy": "grow-only",
                    "desiredSize": "40G"
                  }
                }
              },
              "apply": {
                "mode": "manual"
              }
            }"#,
    )
    .expect("spec should parse");
    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);
    let receipt = apply_receipt("apply", "/etc/disk-nix/spec.json", true, false, 42, &report);
    let value: Value = serde_json::to_value(&receipt).expect("receipt should serialize to JSON");

    assert_eq!(value["receiptVersion"], 1);
    assert_eq!(value["command"], "apply");
    assert_eq!(value["specPath"], "/etc/disk-nix/spec.json");
    assert_eq!(value["probeCurrent"], true);
    assert_eq!(value["executeRequested"], false);
    assert_eq!(value["generatedAtUnixSeconds"], 42);
    assert_eq!(value["report"]["status"], "dry-run");
    assert!(value["report"]["commandSummary"].is_object());
}

#[test]
fn script_refusal_message_mentions_graph_dependency_conflicts() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "root": {
                    "operation": "grow",
                    "device": "/dev/mapper/cryptroot",
                    "mountpoint": "/",
                    "fsType": "xfs",
                    "resizePolicy": "grow-only",
                    "desiredSize": "200GiB"
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
              },
              "apply": {
                "allowGrow": true,
                "allowOffline": true
              }
            }"#,
    )
    .expect("spec should parse");
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("luks:cryptroot", NodeKind::LuksContainer, "cryptroot")
            .with_path("/dev/mapper/cryptroot"),
    );
    graph.add_node(
        Node::new("filesystem:/", NodeKind::Filesystem, "root")
            .with_path("/")
            .with_property("filesystem.type", "xfs")
            .with_size_bytes(100 * 1024 * 1024 * 1024),
    );
    graph.add_edge(Edge::new(
        "luks:cryptroot",
        "filesystem:/",
        Relationship::Backs,
    ));
    let plan = compare_plan_with_topology(plan, &graph);
    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert!(!report.can_apply());
    let message = script_refusal_message(&report);
    assert!(message.contains("conflict-free command plan"));
    assert!(message.contains("1 graph dependency conflict"));
    assert!(message.contains("plan splitting or ordering review"));
}

#[test]
fn migration_report_adds_current_version_to_direct_specs() {
    let report = migration_report_from_json_bytes(
        br#"{
              "filesystems": {
                "root": {
                  "mountpoint": "/",
                  "fsType": "ext4"
                }
              }
            }"#,
    )
    .expect("versionless spec should migrate");

    assert_eq!(report.source_version, None);
    assert_eq!(report.target_version, 1);
    assert!(report.migrated);
    assert_eq!(report.spec["version"], 1);
    assert!(report
        .changes
        .iter()
        .any(|change| change == "set version to 1"));
    assert!(report
        .warnings
        .iter()
        .any(|warning| warning.contains("does not apply storage mutations")));
    assert_eq!(report.version_migrations.len(), 2);
    let legacy_contract = report
        .version_migrations
        .iter()
        .find(|contract| contract.source_version.is_none())
        .expect("pre-version migration contract should exist");
    assert_eq!(legacy_contract.target_version, 1);
    assert_eq!(legacy_contract.status, "supported");
    assert_eq!(
        legacy_contract.mapping_scope,
        "pre-version legacy aliases to version 1"
    );
    assert_mapping(
        &legacy_contract.field_mappings,
        "fileSystems",
        "filesystems",
        "top-level",
    );
    assert!(legacy_contract
        .safety_notes
        .iter()
        .any(|note| note.contains("does not apply storage mutations")));
}

#[test]
fn migration_report_adds_wrapper_and_spec_versions() {
    let report = migration_report_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "root": {
                    "mountpoint": "/",
                    "fsType": "ext4"
                  }
                }
              },
              "apply": {
                "allowGrow": true
              }
            }"#,
    )
    .expect("wrapper spec should migrate");

    assert!(report.migrated);
    assert_eq!(report.spec["version"], 1);
    assert_eq!(report.spec["spec"]["version"], 1);
    assert!(report
        .changes
        .iter()
        .any(|change| change == "set version to 1"));
    assert!(report
        .changes
        .iter()
        .any(|change| change == "set spec.version to 1"));
}

#[test]
fn migration_report_maps_legacy_pre_version_aliases() {
    let report = migration_report_from_json_bytes(
        br#"{
              "fileSystems": {
                "root": {
                  "mountpoint": "/",
                  "fsType": "ext4"
                }
              },
              "swapDevices": {
                "swap": {
                  "device": "/dev/disk/by-label/swap",
                  "operation": "rescan"
                }
              },
              "luksDevices": {
                "cryptroot": {
                  "device": "/dev/disk/by-id/luks-root",
                  "operation": "open"
                }
              },
              "nfsMounts": {
                "/srv/shared": {
                  "source": "nas.example.com:/srv/shared",
                  "operation": "mount"
                }
              },
              "iscsiSessions": {
                "iqn.2026-06.example:storage.root": {
                  "portal": "192.0.2.10:3260",
                  "operation": "login"
                }
              }
            }"#,
    )
    .expect("legacy aliases should migrate");

    assert!(report.migrated);
    assert_eq!(report.spec["version"], 1);
    assert_eq!(report.legacy_mappings.len(), 10);
    assert_mapping(
        &report.legacy_mappings,
        "fileSystems",
        "filesystems",
        "top-level",
    );
    assert_mapping(
        &report.legacy_mappings,
        "spec.fileSystems",
        "spec.filesystems",
        "spec",
    );
    assert_mapping(
        &report.legacy_mappings,
        "iscsiSessions",
        "iscsi.sessions",
        "top-level",
    );
    assert_eq!(report.applied_mappings.len(), 5);
    assert_mapping(
        &report.applied_mappings,
        "fileSystems",
        "filesystems",
        "top-level",
    );
    assert_mapping(
        &report.applied_mappings,
        "swapDevices",
        "swaps",
        "top-level",
    );
    assert_mapping(
        &report.applied_mappings,
        "luksDevices",
        "luks.devices",
        "top-level",
    );
    assert_mapping(
        &report.applied_mappings,
        "nfsMounts",
        "nfs.mounts",
        "top-level",
    );
    assert_mapping(
        &report.applied_mappings,
        "iscsiSessions",
        "iscsi.sessions",
        "top-level",
    );
    assert!(report.spec.get("fileSystems").is_none());
    assert!(report.spec.get("swapDevices").is_none());
    assert!(report.spec.get("luksDevices").is_none());
    assert!(report.spec.get("nfsMounts").is_none());
    assert!(report.spec.get("iscsiSessions").is_none());
    assert_eq!(report.spec["filesystems"]["root"]["mountpoint"], "/");
    assert_eq!(report.spec["swaps"]["swap"]["operation"], "rescan");
    assert_eq!(
        report.spec["luks"]["devices"]["cryptroot"]["operation"],
        "open"
    );
    assert_eq!(
        report.spec["nfs"]["mounts"]["/srv/shared"]["source"],
        "nas.example.com:/srv/shared"
    );
    assert_eq!(
        report.spec["iscsi"]["sessions"]["iqn.2026-06.example:storage.root"]["operation"],
        "login"
    );
    assert!(report
        .changes
        .iter()
        .any(|change| { change == "mapped legacy field fileSystems to filesystems" }));
    assert!(report
        .changes
        .iter()
        .any(|change| { change == "mapped legacy field luksDevices to luks.devices" }));
}

#[test]
fn migration_report_maps_legacy_wrapper_aliases_inside_spec() {
    let report = migration_report_from_json_bytes(
        br#"{
              "spec": {
                "fileSystems": {
                  "root": {
                    "mountpoint": "/",
                    "fsType": "xfs"
                  }
                },
                "nfsMounts": {
                  "/srv/shared": {
                    "source": "nas.example.com:/srv/shared",
                    "operation": "mount"
                  }
                }
              },
              "apply": {
                "allowOffline": true
              }
            }"#,
    )
    .expect("legacy wrapper aliases should migrate");

    assert!(report.migrated);
    assert_eq!(report.spec["version"], 1);
    assert_eq!(report.spec["spec"]["version"], 1);
    assert_eq!(report.legacy_mappings.len(), 10);
    assert_eq!(report.applied_mappings.len(), 2);
    assert_mapping(
        &report.applied_mappings,
        "spec.fileSystems",
        "spec.filesystems",
        "spec",
    );
    assert_mapping(
        &report.applied_mappings,
        "spec.nfsMounts",
        "spec.nfs.mounts",
        "spec",
    );
    assert!(report.spec["spec"].get("fileSystems").is_none());
    assert!(report.spec["spec"].get("nfsMounts").is_none());
    assert_eq!(report.spec["spec"]["filesystems"]["root"]["fsType"], "xfs");
    assert_eq!(
        report.spec["spec"]["nfs"]["mounts"]["/srv/shared"]["source"],
        "nas.example.com:/srv/shared"
    );
    assert!(report
        .changes
        .iter()
        .any(|change| { change == "mapped legacy field spec.fileSystems to spec.filesystems" }));
    assert!(report
        .changes
        .iter()
        .any(|change| { change == "mapped legacy field spec.nfsMounts to spec.nfs.mounts" }));
}

#[test]
fn migration_report_rejects_conflicting_legacy_aliases() {
    let error = migration_report_from_json_bytes(
        br#"{
              "fileSystems": {
                "legacy": {
                  "mountpoint": "/legacy",
                  "fsType": "ext4"
                }
              },
              "filesystems": {
                "current": {
                  "mountpoint": "/current",
                  "fsType": "xfs"
                }
              }
            }"#,
    )
    .expect_err("conflicting aliases should be rejected");

    assert!(error
        .to_string()
        .contains("legacy field fileSystems conflicts with current field filesystems"));
}

#[test]
fn migration_report_does_not_rewrite_explicit_current_version_aliases() {
    let report = migration_report_from_json_bytes(
        br#"{
              "version": 1,
              "fileSystems": {
                "legacy": {
                  "mountpoint": "/legacy",
                  "fsType": "ext4"
                }
              }
            }"#,
    )
    .expect("explicit current-version spec should stay metadata-only");

    assert!(!report.migrated);
    assert_eq!(report.legacy_mappings.len(), 10);
    assert!(report.applied_mappings.is_empty());
    assert!(report.spec.get("fileSystems").is_some());
    assert!(report.spec.get("filesystems").is_none());
    assert!(report
        .changes
        .iter()
        .any(|change| change.contains("already declares")));
}

#[test]
fn migration_report_keeps_explicit_current_version() {
    let report = migration_report_from_json_bytes(
        br#"{
              "version": 1,
              "filesystems": {
                "root": {
                  "mountpoint": "/",
                  "fsType": "ext4"
                }
              }
            }"#,
    )
    .expect("current version should validate");

    assert_eq!(report.source_version, Some(1));
    assert!(!report.migrated);
    assert_eq!(report.legacy_mappings.len(), 10);
    assert!(report.applied_mappings.is_empty());
    let current_contract = report
        .version_migrations
        .iter()
        .find(|contract| contract.source_version == Some(1))
        .expect("version 1 migration contract should exist");
    assert_eq!(current_contract.target_version, 1);
    assert_eq!(current_contract.status, "supported");
    assert!(current_contract.field_mappings.is_empty());
    assert!(current_contract
        .safety_notes
        .iter()
        .any(|note| note.contains("validated without legacy alias rewrites")));
    assert!(report
        .changes
        .iter()
        .any(|change| change.contains("already declares")));
}

#[test]
fn migration_report_json_includes_version_migration_contracts() {
    let report = migration_report_from_json_bytes(
        br#"{
              "fileSystems": {
                "root": {
                  "mountpoint": "/",
                  "fsType": "ext4"
                }
              }
            }"#,
    )
    .expect("legacy spec should migrate");

    let json = serde_json::to_value(&report).expect("report should serialize");
    assert_eq!(json["versionMigrations"][0]["sourceVersion"], Value::Null);
    assert_eq!(json["versionMigrations"][0]["targetVersion"], 1);
    assert_eq!(json["versionMigrations"][0]["status"], "supported");
    assert_eq!(
        json["versionMigrations"][0]["mappingScope"],
        "pre-version legacy aliases to version 1"
    );
    assert!(json["versionMigrations"][0]["fieldMappings"]
        .as_array()
        .is_some_and(|mappings| mappings.iter().any(|mapping| {
            mapping["source"] == "fileSystems"
                && mapping["target"] == "filesystems"
                && mapping["scope"] == "top-level"
        })));
    assert_eq!(json["versionMigrations"][1]["sourceVersion"], 1);
    assert_eq!(json["versionMigrations"][1]["targetVersion"], 1);
    assert!(json["versionMigrations"][1]["fieldMappings"]
        .as_array()
        .is_some_and(Vec::is_empty));
}

#[test]
fn migration_report_rejects_future_and_conflicting_versions() {
    let future = migration_report_from_json_bytes(
        br#"{
              "version": 2,
              "filesystems": {
                "root": {
                  "mountpoint": "/",
                  "fsType": "ext4"
                }
              }
            }"#,
    )
    .expect_err("future version should not migrate implicitly");
    assert!(future
        .to_string()
        .contains("unsupported disk-nix spec version 2"));

    let conflict = migration_report_from_json_bytes(
        br#"{
              "version": 1,
              "spec": {
                "version": 2
              }
            }"#,
    )
    .expect_err("conflicting versions should be rejected");
    assert!(conflict
        .to_string()
        .contains("conflicting disk-nix spec versions"));
}

#[test]
fn migration_report_human_output_includes_migrated_spec() {
    let report = migration_report_from_json_bytes(
        br#"{
              "filesystems": {
                "root": {
                  "mountpoint": "/",
                  "fsType": "ext4"
                }
              }
            }"#,
    )
    .expect("spec should migrate");

    let mut output = Vec::new();
    print_migration_report(&mut output, &report).expect("migration report renders");
    let output = String::from_utf8(output).expect("migration output is utf8");
    assert!(output.contains("Migration: None -> 1"));
    assert!(output.contains("migrated: true"));
    assert!(output.contains("Version migration contracts:"));
    assert!(output.contains("- None -> 1: supported (pre-version legacy aliases to version 1)"));
    assert!(output.contains("- Some(1) -> 1: supported (version 1 metadata normalization)"));
    assert!(output.contains("Legacy mappings:"));
    assert!(output.contains("- fileSystems -> filesystems (top-level)"));
    assert!(output.contains("- spec.fileSystems -> spec.filesystems (spec)"));
    assert!(output.contains("Applied mappings:"));
    assert!(output.contains("- none"));
    assert!(output.contains("Migrated spec:"));
    assert!(output.contains(r#""version": 1"#));
}

#[test]
fn confirmation_file_rejects_partial_or_different_tokens() {
    assert!(!confirmation_file_accepts(""));
    assert!(!confirmation_file_accepts("disk-nix"));
    assert!(!confirmation_file_accepts("disk-nix confirm now"));
    assert!(!confirmation_file_accepts("prefix disk-nix confirm"));
}

#[test]
fn focused_view_predicates_cover_storage_domains() {
    assert!(is_partition_node(&Node::new(
        "partition:sda1",
        NodeKind::Partition,
        "sda1"
    )));
    assert!(is_pool_node(&Node::new(
        "zpool:tank",
        NodeKind::ZfsPool,
        "tank"
    )));
    assert!(is_pool_node(&Node::new(
        "vg:root",
        NodeKind::LvmVolumeGroup,
        "root"
    )));
    assert!(is_lvm_node(&Node::new(
        "lvm-vg:root",
        NodeKind::LvmVolumeGroup,
        "root"
    )));
    assert!(is_lvm_node(
        &Node::new(
            "block:/dev/mapper/vg-root",
            NodeKind::DeviceMapper,
            "vg-root"
        )
        .with_property("lvm.active", "active")
    ));
    assert!(is_dm_node(
        &Node::new(
            "block:/dev/mapper/cryptroot",
            NodeKind::DeviceMapper,
            "cryptroot"
        )
        .with_property("dm.name", "cryptroot")
    ));
    let bcachefs = Node::new(
        "bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3",
        NodeKind::BcachefsFilesystem,
        "archive",
    );
    assert!(is_filesystem_node(&bcachefs));
    assert!(is_complex_filesystem_node(&bcachefs));
    assert!(is_volume_node(&bcachefs));
    assert!(is_pool_node(&bcachefs));
    assert!(is_bcachefs_node(&bcachefs));
    assert!(is_complex_filesystem_node(&Node::new(
        "btrfs:/mnt/persist",
        NodeKind::BtrfsFilesystem,
        "/mnt/persist"
    )));
    assert!(is_btrfs_node(&Node::new(
        "btrfs:/mnt/persist",
        NodeKind::BtrfsFilesystem,
        "/mnt/persist"
    )));
    assert!(is_complex_filesystem_node(&Node::new(
        "zpool:tank",
        NodeKind::ZfsPool,
        "tank"
    )));
    assert!(is_zfs_node(&Node::new(
        "zpool:tank",
        NodeKind::ZfsPool,
        "tank"
    )));
    assert!(is_zfs_node(
        &Node::new("filesystem:tank/home", NodeKind::Filesystem, "tank/home")
            .with_property("zfs.compression", "zstd")
    ));
    assert!(is_complex_filesystem_node(
        &Node::new("filesystem:/data", NodeKind::Filesystem, "/data")
            .with_property("btrfs.data-profile", "single")
    ));
    assert!(is_device_node(&Node::new(
        "bcachefs-device:a2d6fc04-efd0-4e36-aece-2475941d09a3:6",
        NodeKind::BcachefsDevice,
        "sdc"
    )));
    assert!(is_snapshot_node(&Node::new(
        "snapshot:tank/home@before",
        NodeKind::ZfsSnapshot,
        "tank/home@before"
    )));
    assert!(is_network_storage_node(&Node::new(
        "lun:iqn.example:0",
        NodeKind::Lun,
        "iqn.example:0"
    )));
    assert!(is_lun_node(&Node::new(
        "lun:iqn.example:0",
        NodeKind::Lun,
        "iqn.example:0"
    )));
    assert!(is_iscsi_node(&Node::new(
        "iscsi-session:1",
        NodeKind::IscsiSession,
        "iscsi-session:1"
    )));
    assert!(is_iscsi_node(
        &Node::new("block:/dev/sdb", NodeKind::PhysicalDisk, "/dev/sdb")
            .with_property("iscsi.attached-disk", "sdb")
    ));
    assert!(is_network_storage_node(&Node::new(
        "nfs:server:/export",
        NodeKind::NfsExport,
        "server:/export"
    )));
    assert!(is_nfs_node(&Node::new(
        "nfs:server:/export",
        NodeKind::NfsExport,
        "server:/export"
    )));
    assert!(is_nfs_node(
        &Node::new("mount:/home", NodeKind::Mountpoint, "/home")
            .with_property("nfs.source", "server:/export")
    ));
    assert!(is_device_node(&Node::new(
        "file:/var/lib/images/root.img",
        NodeKind::BackingFile,
        "/var/lib/images/root.img"
    )));
    assert!(is_mapping_node(&Node::new(
        "block:/dev/loop0",
        NodeKind::LoopDevice,
        "/dev/loop0"
    )));
    assert!(is_encryption_node(&Node::new(
        "block:/dev/mapper/cryptroot",
        NodeKind::LuksContainer,
        "cryptroot"
    )));
    assert!(is_encryption_node(
        &Node::new("dm:cryptroot", NodeKind::DeviceMapper, "cryptroot")
            .with_property("cryptsetup.active", "true")
    ));
    assert!(is_cache_node(&Node::new(
        "block:/dev/bcache0",
        NodeKind::CacheDevice,
        "bcache0"
    )));
    assert!(is_cache_node(
        &Node::new("lvm-lv:vg/root", NodeKind::LvmLogicalVolume, "vg/root")
            .with_property("lvm.cache-mode", "writeback")
    ));
    assert!(is_cache_node(
        &Node::new(
            "zfs-vdev:tank:cache0",
            NodeKind::ZfsVdev,
            "/dev/disk/by-id/cache0"
        )
        .with_property("zfs.vdev-role", "cache")
    ));
    assert!(is_vdo_node(&Node::new(
        "vdo:archive",
        NodeKind::VdoVolume,
        "archive"
    )));
    assert!(is_vdo_node(
        &Node::new(
            "lvm-seg:vg0/archive:0",
            NodeKind::LvmSegment,
            "vg0/archive:0"
        )
        .with_property("lvm.vdo-write-policy", "auto")
    ));
    assert!(is_multipath_node(&Node::new(
        "multipath:mpatha",
        NodeKind::MultipathDevice,
        "mpatha"
    )));
    assert!(is_multipath_node(
        &Node::new("block:/dev/sdb", NodeKind::PhysicalDisk, "/dev/sdb")
            .with_property("multipath.path-state", "active ready running")
    ));
    assert!(is_multipath_node(
        &Node::new("block:/dev/sdc", NodeKind::PhysicalDisk, "/dev/sdc")
            .with_property("multipath.group-policy", "service-time 0")
    ));
    assert!(is_nvme_node(&Node::new(
        "block:/dev/nvme0n1",
        NodeKind::NvmeNamespace,
        "/dev/nvme0n1"
    )));
    assert!(is_nvme_node(
        &Node::new("block:/dev/nvme1n1", NodeKind::PhysicalDisk, "/dev/nvme1n1")
            .with_property("nvme.model", "Example NVMe")
    ));
    assert!(is_nvme_node(
        &Node::new("block:/dev/nvme2n1", NodeKind::PhysicalDisk, "/dev/nvme2n1")
            .with_property("nvme.subsystem", "nvme-subsys0")
    ));
    assert!(is_raid_node(&Node::new(
        "md:/dev/md0",
        NodeKind::MdRaid,
        "/dev/md0"
    )));
    assert!(is_raid_node(
        &Node::new("block:/dev/sda1", NodeKind::Partition, "/dev/sda1")
            .with_property("md.member-state", "active sync")
    ));
    assert!(is_loop_node(&Node::new(
        "block:/dev/loop0",
        NodeKind::LoopDevice,
        "/dev/loop0"
    )));
    assert!(is_loop_node(&Node::new(
        "file:/var/lib/images/root.img",
        NodeKind::BackingFile,
        "/var/lib/images/root.img"
    )));
    assert!(is_backing_file_node(&Node::new(
        "file:/var/lib/images/root.img",
        NodeKind::BackingFile,
        "/var/lib/images/root.img"
    )));
    assert!(is_swap_node(&Node::new(
        "swap:/dev/sda3",
        NodeKind::Swap,
        "/dev/sda3"
    )));
    assert!(is_swap_node(
        &Node::new("block:/swapfile", NodeKind::BackingFile, "/swapfile")
            .with_property("swap.active", "true")
    ));
    assert!(is_swap_node(
        &Node::new("block:/dev/zram0", NodeKind::ZramDevice, "/dev/zram0")
            .with_property("zram.swap", "true")
    ));
}
