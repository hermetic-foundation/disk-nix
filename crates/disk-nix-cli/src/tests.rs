
use disk_nix_exec::{prepare_execution, ExecutionMode};
use disk_nix_model::{Edge, Identity, Node, NodeKind, Relationship, StorageGraph, Usage};
use disk_nix_plan::{compare_plan_with_topology, plan_and_policy_from_json_bytes};
use disk_nix_probe::{ProbeReport, ProbeStatus};
use serde_json::Value;

use super::{
    apply_receipt, command_stdout_first_line, confirmation_file_accepts, consumer_count,
    is_backing_file_node, is_bcachefs_node, is_btrfs_node, is_cache_node,
    is_complex_filesystem_node, is_device_node, is_dm_node, is_encryption_node, is_filesystem_node,
    is_iscsi_node, is_loop_node, is_lun_node, is_lvm_node, is_mapping_node, is_multipath_node,
    is_network_storage_node, is_nfs_node, is_nvme_node, is_partition_node, is_pool_node,
    is_raid_node, is_snapshot_node, is_swap_node, is_vdo_node, is_volume_node, is_zfs_node,
    is_zram_node, iscsi_lun_count, member_count, migration_report_from_json_bytes, mount_details,
    nfs_mount_count, parse_os_release, print_backing_files, print_bcachefs, print_btrfs,
    print_cache, print_complex_filesystems, print_devices, print_dm, print_encryption,
    print_filesystems, print_filtered_json, print_inspect, print_inspect_json, print_iscsi,
    print_loop, print_luns, print_lvm, print_mappings, print_migration_report, print_mounts,
    print_multipath, print_network_storage, print_nfs, print_nvme, print_partitions, print_pools,
    print_probe_preflight_checks, print_probe_preflight_environment, print_probe_reports,
    print_raid, print_snapshots, print_swap, print_usage, print_vdo, print_volumes, print_zfs,
    print_zram, probe_preflight_checks, script_refusal_message, snapshot_source,
    storage_tool_version_report, usage_details, usage_percent, zfs_child_count,
    ProbePreflightEnvironment, ProbeStatusPreflightReport, ToolVersionReport, ToolVersionStatus,
};

fn assert_mapping(
    mappings: &[super::LegacyMigrationMapping],
    source: &str,
    target: &str,
    scope: &str,
) {
    assert!(
        mappings.iter().any(|mapping| {
            mapping.source == source && mapping.target == target && mapping.scope == scope
        }),
        "missing mapping {source} -> {target} ({scope}) in {mappings:?}"
    );
}

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

#[test]
fn snapshot_source_follows_snapshot_relationships() {
    let mut graph = StorageGraph::empty();
    graph.add_node(Node::new(
        "dataset:tank/home",
        NodeKind::ZfsDataset,
        "tank/home",
    ));
    graph.add_node(Node::new(
        "snapshot:tank/home@before",
        NodeKind::ZfsSnapshot,
        "tank/home@before",
    ));
    graph.add_edge(Edge::new(
        "snapshot:tank/home@before",
        "dataset:tank/home",
        Relationship::SnapshotOf,
    ));

    let snapshot = graph
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::ZfsSnapshot)
        .expect("snapshot exists");
    assert_eq!(snapshot_source(&graph, snapshot), Some("tank/home"));
}

#[test]
fn focused_json_includes_direct_relationship_neighbors() {
    let mut graph = StorageGraph::empty();
    graph.add_node(Node::new(
        "filesystem:root",
        NodeKind::Filesystem,
        "/dev/mapper/vg-root",
    ));
    graph.add_node(Node::new("mount:/", NodeKind::Mountpoint, "/"));
    graph.add_node(Node::new(
        "block:/dev/nvme0n1",
        NodeKind::PhysicalDisk,
        "/dev/nvme0n1",
    ));
    graph.add_edge(Edge::new(
        "filesystem:root",
        "mount:/",
        Relationship::MountedAt,
    ));

    let mut output = Vec::new();
    print_filtered_json(&mut output, &graph, is_filesystem_node).expect("filtered graph renders");
    let output = String::from_utf8(output).expect("json is utf8");
    let graph: StorageGraph = serde_json::from_str(&output).expect("valid storage graph json");

    assert_eq!(graph.nodes.len(), 2);
    assert!(graph
        .nodes
        .iter()
        .any(|node| node.id.0 == "filesystem:root"));
    assert!(graph.nodes.iter().any(|node| node.id.0 == "mount:/"));
    assert!(graph
        .nodes
        .iter()
        .all(|node| node.id.0 != "block:/dev/nvme0n1"));
    assert_eq!(
        graph.edges,
        vec![Edge::new(
            "filesystem:root",
            "mount:/",
            Relationship::MountedAt
        )]
    );
}

#[test]
fn devices_table_includes_probe_metadata_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("block:/dev/nvme0n1", NodeKind::PhysicalDisk, "/dev/nvme0n1")
            .with_path("/dev/nvme0n1")
            .with_size_bytes(1_000_000_000_000)
            .with_property("model", "FastDisk")
            .with_property("vendor", "Acme")
            .with_property("transport", "nvme")
            .with_property("rotational", "false")
            .with_property("nvme.model", "Example NVMe")
            .with_property("nvme.product", "Example Controller")
            .with_property("nvme.firmware", "1.0")
            .with_property("nvme.index", "0")
            .with_property("nvme.namespace", "1")
            .with_property("nvme.namespace-id", "1")
            .with_property(
                "nvme.namespace-uuid",
                "12345678-1234-1234-1234-123456789abc",
            )
            .with_property("nvme.eui64", "0011223344556677")
            .with_property("nvme.nguid", "00112233445566778899aabbccddeeff")
            .with_property("nvme.subsystem", "nvme-subsys0")
            .with_property("nvme.controller", "nvme0")
            .with_property("nvme.transport", "pcie")
            .with_property("nvme.controller-id", "1")
            .with_property("nvme.namespace-capacity", "900000000000")
            .with_property("nvme.lba-format", "512 B + 0 B")
            .with_property("nvme.maximum-lba", "1953125")
            .with_property("nvme.sector-size", "512")
            .with_property("nvme.ana-state", "optimized")
            .with_property("lsblk.logical-sector-size", "512")
            .with_property("lsblk.physical-sector-size", "4096")
            .with_property("lsblk.minimum-io-size", "4096")
            .with_property("lsblk.optimal-io-size", "1048576")
            .with_property("lsblk.discard-alignment", "0")
            .with_property("lsblk.discard-granularity", "4096")
            .with_property("lsblk.discard-max", "2147483648")
            .with_property("lsblk.discard-zeroes-data", "false")
            .with_property("lsblk.scheduler", "none")
            .with_property("lsblk.request-queue-size", "1023")
            .with_property("lsblk.write-same-max", "0")
            .with_property("lsblk.zoned", "host-managed")
            .with_property("lsblk.zone-size", "268435456")
            .with_property("lsblk.zone-write-granularity", "4096")
            .with_property("lsblk.zone-append-max", "65536")
            .with_property("lsblk.zone-count", "64")
            .with_property("lsblk.zone-open-max", "32")
            .with_property("lsblk.zone-active-max", "48")
            .with_property("lsblk.dax", "false")
            .with_property("lsblk.hotplug", "false")
            .with_property("partition.table", "gpt")
            .with_property("udev.symlink", "disk/by-id/nvme-Acme_FastDisk")
            .with_property("udev.devname", "/dev/nvme0n1")
            .with_property("udev.devtype", "disk")
            .with_property("udev.id-bus", "nvme")
            .with_property("udev.id-model", "FastDisk")
            .with_property("udev.id-model-id", "a808")
            .with_property("udev.id-vendor", "Acme")
            .with_property("udev.id-vendor-id", "144d")
            .with_property("udev.id-revision", "1.0")
            .with_property("udev.id-serial", "Acme_FastDisk_SERIAL")
            .with_property("udev.id-serial-short", "SERIAL")
            .with_property("udev.id-wwn", "eui.1234")
            .with_property("udev.id-path", "pci-0000:01:00.0-nvme-1")
            .with_property("udev.id-path-tag", "pci-0000_01_00_0-nvme-1")
            .with_property("udev.major", "259")
            .with_property("udev.minor", "0")
            .with_property("udev.subsystem", "block"),
    );
    graph.add_node(
        Node::new(
            "block:/dev/nvme0n1p1",
            NodeKind::Partition,
            "/dev/nvme0n1p1",
        )
        .with_path("/dev/nvme0n1p1")
        .with_property("lsblk.type", "part")
        .with_property("filesystem.type", "vfat")
        .with_property("partition.number", "1")
        .with_property("udev.id-fs-type", "vfat")
        .with_property("udev.id-fs-version", "FAT32")
        .with_property("udev.id-fs-usage", "filesystem")
        .with_property("udev.id-fs-uuid", "AAAA-BBBB")
        .with_property("udev.id-fs-uuid-enc", "AAAA-BBBB")
        .with_property("udev.id-fs-uuid-sub", "CCCC-DDDD")
        .with_property("udev.id-fs-label", "EFI")
        .with_property("udev.id-fs-label-enc", "EFI")
        .with_property("udev.id-fs-label-safe", "EFI")
        .with_property("udev.id-fs-block-size", "512")
        .with_property("udev.id-fs-lastblock", "1048575")
        .with_property("udev.id-part-entry-disk", "259:0")
        .with_property("udev.id-part-entry-number", "1")
        .with_property("udev.id-part-entry-offset", "2048")
        .with_property("udev.id-part-entry-size", "1048576")
        .with_property("udev.id-part-entry-scheme", "gpt")
        .with_property("udev.id-part-entry-type", "uefi")
        .with_property("udev.id-part-entry-name", "EFI System Partition")
        .with_property("udev.id-part-entry-uuid", "part-uuid")
        .with_property("udev.id-part-entry-flags", "0x1")
        .with_property("udev.id-part-table-type", "gpt")
        .with_property("udev.id-part-table-uuid", "table-uuid"),
    );
    graph.add_node(
        Node::new("block:/dev/loop0", NodeKind::LoopDevice, "/dev/loop0")
            .with_path("/dev/loop0")
            .with_property("lsblk.type", "loop")
            .with_property("loop.back-file", "/var/lib/images/root.img")
            .with_property("loop.backing-inode", "12345")
            .with_property("loop.backing-major-minor", "0:45")
            .with_property("loop.offset", "1048576")
            .with_property("loop.autoclear", "true")
            .with_property("loop.partscan", "true")
            .with_property("loop.direct-io", "true"),
    );
    graph.add_node(
        Node::new("block:/dev/dm-0", NodeKind::DeviceMapper, "/dev/dm-0")
            .with_path("/dev/dm-0")
            .with_property("udev.dm-name", "cryptroot")
            .with_property("udev.dm-uuid", "CRYPT-LUKS2-luks-uuid-cryptroot")
            .with_property("udev.dm-vg-name", "vg0")
            .with_property("udev.dm-lv-name", "root")
            .with_property("udev.dm-udev-rules-vsn", "3")
            .with_property("udev.dm-udev-primary-source-flag", "1")
            .with_property("udev.dm-udev-disable-other-rules-flag", "0")
            .with_property("udev.dm-subsystem-udev-flag0", "1")
            .with_property("udev.dm-subsystem-udev-flag1", "0"),
    );
    graph.add_node(
        Node::new(
            "file:/var/lib/images/root.img",
            NodeKind::BackingFile,
            "/var/lib/images/root.img",
        )
        .with_path("/var/lib/images/root.img")
        .with_property("loop.backing", "true"),
    );
    graph.add_node(
        Node::new("swap:/dev/zram0", NodeKind::Swap, "/dev/zram0")
            .with_path("/dev/zram0")
            .with_property("swap.active", "true")
            .with_property("swap.type", "partition")
            .with_property("swap.priority", "100"),
    );
    graph.add_node(
        Node::new("block:/dev/sda1", NodeKind::Partition, "/dev/sda1")
            .with_path("/dev/sda1")
            .with_property("md.member-state", "active sync"),
    );
    graph.add_node(
        Node::new("block:/dev/sdb", NodeKind::PhysicalDisk, "/dev/sdb")
            .with_path("/dev/sdb")
            .with_property("smartctl.svn-revision", "5530")
            .with_property("smartctl.platform", "x86_64-linux")
            .with_property("smartctl.exit-status", "0")
            .with_property("smartctl.device-name", "/dev/sdb")
            .with_property("smartctl.health.passed", "true")
            .with_property("smartctl.device-type", "sat")
            .with_property("smartctl.protocol", "ATA")
            .with_property("smartctl.model", "Example SSD")
            .with_property("smartctl.model-family", "Example SSDs")
            .with_property("smartctl.serial", "SATA123")
            .with_property("smartctl.revision", "A1")
            .with_property("smartctl.firmware-version", "1.2.3")
            .with_property("smartctl.wwn-naa", "5")
            .with_property("smartctl.wwn-oui", "12345")
            .with_property("smartctl.wwn-id", "67890")
            .with_property("smartctl.user-capacity-bytes", "1000204886016")
            .with_property("smartctl.logical-block-size", "512")
            .with_property("smartctl.physical-block-size", "4096")
            .with_property("smartctl.rotation-rate-rpm", "0")
            .with_property("smartctl.form-factor", "2.5 inches")
            .with_property("smartctl.sata-version", "SATA 3.3")
            .with_property("smartctl.interface-speed-current", "6.0")
            .with_property("smartctl.interface-speed-max", "6.0")
            .with_property("smartctl.power-on-hours", "4242")
            .with_property("smartctl.power-cycle-count", "12")
            .with_property("smartctl.temperature-current-celsius", "31")
            .with_property("smartctl.temperature-highest-celsius", "44")
            .with_property("smartctl.temperature-lowest-celsius", "20")
            .with_property(
                "smartctl.offline-data-collection-status",
                "was completed without error",
            )
            .with_property("smartctl.self-test-status", "completed without error")
            .with_property("smartctl.error-log-summary-count", "3")
            .with_property("smartctl.self-test-log-count", "2")
            .with_property("smartctl.error-logging-supported", "true")
            .with_property("smartctl.gp-logging-supported", "true")
            .with_property("smartctl.sct-capabilities", "61")
            .with_property("smartctl.scsi-grown-defect-list", "0")
            .with_property("smartctl.attribute.reallocated-sector-ct.raw", "0")
            .with_property("smartctl.attribute.reallocated-sector-ct.value", "100")
            .with_property("smartctl.attribute.reallocated-sector-ct.worst", "100")
            .with_property("smartctl.attribute.reallocated-sector-ct.threshold", "10")
            .with_property(
                "smartctl.attribute.reallocated-sector-ct.when-failed",
                "never",
            )
            .with_property("smartctl.attribute.current-pending-sector.raw", "1")
            .with_property("smartctl.attribute.current-pending-sector.value", "99")
            .with_property("smartctl.attribute.current-pending-sector.worst", "98")
            .with_property("smartctl.attribute.current-pending-sector.threshold", "0")
            .with_property(
                "smartctl.attribute.current-pending-sector.when-failed",
                "past",
            )
            .with_property("smartctl.attribute.offline-uncorrectable.raw", "2")
            .with_property("smartctl.attribute.offline-uncorrectable.value", "97")
            .with_property("smartctl.attribute.offline-uncorrectable.worst", "96")
            .with_property("smartctl.attribute.offline-uncorrectable.threshold", "0")
            .with_property(
                "smartctl.attribute.offline-uncorrectable.when-failed",
                "past",
            )
            .with_property("scsi.address", "1:0:0:0")
            .with_property("scsi.generic-device", "/dev/sg1")
            .with_property("scsi.transport", "sata:5000c500a5a461dc")
            .with_property("scsi.unit-name", "5000c500a5a461dc")
            .with_property("scsi.queue-depth", "32")
            .with_property("multipath.host-path", "2:0:0:1")
            .with_property("major-minor", "8:16")
            .with_property("multipath.path-flags", "ghost")
            .with_property("multipath.path-state", "active ready running ghost"),
    );

    let mut output = Vec::new();
    print_devices(&mut output, &graph).expect("devices table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("DETAILS"));
    assert!(output.contains("model=FastDisk vendor=Acme transport=nvme rotational=false"));
    assert!(output.contains("nvme-model=Example NVMe product=Example Controller firmware=1.0"));
    assert!(output
        .contains("ns-index=0 namespace=1 nsid=1 ns-uuid=12345678-1234-1234-1234-123456789abc"));
    assert!(output.contains(
            "eui64=0011223344556677 nguid=00112233445566778899aabbccddeeff subsystem=nvme-subsys0 controller=nvme0"
        ));
    assert!(output.contains(
        "transport=pcie controller-id=1 namespace-capacity=900000000000 lba-format=512 B + 0 B"
    ));
    assert!(output.contains("max-lba=1953125 sector-size=512 ana-state=optimized"));
    assert!(output
        .contains("logical-sector=512 physical-sector=4096 minimum-io=4096 optimal-io=1048576"));
    assert!(output.contains(
        "discard-alignment=0 discard-granularity=4096 discard-max=2147483648 discard-zeroes=false"
    ));
    assert!(output.contains("scheduler=none rq-size=1023 write-same-max=0 zoned=host-managed"));
    assert!(output.contains(
        "zone-size=268435456 zone-write-granularity=4096 zone-append-max=65536 zone-count=64"
    ));
    assert!(output.contains("zone-open-max=32 zone-active-max=48 dax=false hotplug=false"));
    assert!(output.contains("ptable=gpt"));
    assert!(output.contains("udev-link=disk/by-id/nvme-Acme_FastDisk"));
    assert!(output.contains("udev-devname=/dev/nvme0n1 udev-devtype=disk"));
    assert!(output.contains("udev-bus=nvme udev-model=FastDisk udev-model-id=a808"));
    assert!(output.contains("udev-vendor=Acme udev-vendor-id=144d udev-revision=1.0"));
    assert!(output.contains("udev-serial=Acme_FastDisk_SERIAL udev-serial-short=SERIAL"));
    assert!(output.contains("udev-wwn=eui.1234 udev-path=pci-0000:01:00.0-nvme-1"));
    assert!(output.contains("udev-path-tag=pci-0000_01_00_0-nvme-1"));
    assert!(output.contains("major=259 minor=0 subsystem=block"));
    assert!(output.contains("lsblk-type=part fstype=vfat partno=1 udev-fstype=vfat"));
    assert!(output.contains("udev-fs-version=FAT32 udev-fs-usage=filesystem"));
    assert!(output.contains("udev-fs-uuid=AAAA-BBBB udev-fs-uuid-enc=AAAA-BBBB"));
    assert!(output.contains("udev-fs-uuid-sub=CCCC-DDDD"));
    assert!(output.contains("udev-label=EFI udev-label-enc=EFI udev-label-safe=EFI"));
    assert!(output.contains("udev-fs-block-size=512 udev-fs-lastblock=1048575"));
    assert!(output.contains("udev-part-disk=259:0 udev-part-number=1"));
    assert!(output.contains("udev-part-offset=2048 udev-part-size=1048576"));
    assert!(output.contains("udev-part-scheme=gpt udev-part-type=uefi"));
    assert!(output.contains("udev-part-name=EFI System Partition udev-part-uuid=part-uuid"));
    assert!(output.contains("udev-part-flags=0x1 udev-table-type=gpt"));
    assert!(output.contains("udev-table-uuid=table-uuid"));
    assert!(output.contains("dm-name=cryptroot dm-uuid=CRYPT-LUKS2-luks-uuid-cryptroot"));
    assert!(output.contains("dm-vg=vg0 dm-lv=root dm-rules=3"));
    assert!(output.contains("dm-primary-source=1 dm-disable-other-rules=0"));
    assert!(output.contains("dm-subsystem-flag0=1 dm-subsystem-flag1=0"));
    assert!(output.contains(
            "lsblk-type=loop back-file=/var/lib/images/root.img back-ino=12345 back-major-minor=0:45 offset=1048576 autoclear=true partscan=true dio=true"
        ));
    assert!(output.contains("loop-backing=true"));
    assert!(output.contains("swap-active=true swap-type=partition swap-priority=100"));
    assert!(output.contains("member-state=active sync"));
    assert!(output.contains(
        "smart-svn=5530 smart-platform=x86_64-linux smart-exit-status=0 smart-device-name=/dev/sdb"
    ));
    assert!(output.contains(
        "smart-health-passed=true smart-device-type=sat smart-protocol=ATA smart-model=Example SSD"
    ));
    assert!(output.contains("smart-family=Example SSDs"));
    assert!(output.contains("smart-revision=A1 smart-firmware=1.2.3"));
    assert!(output
        .contains("smart-serial=SATA123 smart-wwn-naa=5 smart-wwn-oui=12345 smart-wwn-id=67890"));
    assert!(output.contains("smart-capacity=1000204886016 smart-logical-block=512"));
    assert!(output.contains(
        "smart-physical-block=4096 smart-rpm=0 smart-form-factor=2.5 inches sata-version=SATA 3.3"
    ));
    assert!(output.contains("interface-speed-current=6.0 interface-speed-max=6.0"));
    assert!(output.contains("smart-power-on-hours=4242"));
    assert!(
            output.contains(
                "smart-power-cycles=12 smart-temperature-c=31 smart-temperature-highest-c=44 smart-temperature-lowest-c=20"
            )
        );
    assert!(output.contains(
        "smart-offline-status=was completed without error smart-self-test=completed without error"
    ));
    assert!(output.contains(
            "smart-error-log-count=3 smart-self-test-count=2 smart-error-logging=true smart-gp-logging=true"
        ));
    assert!(output.contains("smart-sct-capabilities=61 smart-scsi-grown-defects=0"));
    assert!(output.contains(
            "reallocated-sectors=0 reallocated-value=100 reallocated-worst=100 reallocated-threshold=10 reallocated-failed=never"
        ));
    assert!(output.contains(
            "pending-sectors=1 pending-value=99 pending-worst=98 pending-threshold=0 pending-failed=past"
        ));
    assert!(
            output.contains(
                "offline-uncorrectable=2 offline-uncorrectable-value=97 offline-uncorrectable-worst=96 offline-uncorrectable-threshold=0 offline-uncorrectable-failed=past"
            )
        );
    assert!(output.contains(
        "scsi-address=1:0:0:0 scsi-generic=/dev/sg1 scsi-transport=sata:5000c500a5a461dc"
    ));
    assert!(output.contains("scsi-unit=5000c500a5a461dc scsi-queue-depth=32"));
    assert!(output.contains(
        "host-path=2:0:0:1 major-minor=8:16 path-flags=ghost path-state=active ready running ghost"
    ));
    assert!(output.contains("/var/lib/images/root.img"));
}

#[test]
fn partitions_table_includes_geometry_metadata_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "block:/dev/nvme0n1p1",
            NodeKind::Partition,
            "/dev/nvme0n1p1",
        )
        .with_path("/dev/nvme0n1p1")
        .with_size_bytes(536_870_912)
        .with_identity(Identity {
            partuuid: Some("1111-2222".to_string()),
            ..Default::default()
        })
        .with_property("partition.number", "1")
        .with_property("partition.start", "1049kB")
        .with_property("partition.start-bytes", "1049000")
        .with_property("partition.end", "538MB")
        .with_property("partition.end-bytes", "538000000")
        .with_property("partition.type", "fat32")
        .with_property("partition.name", "ESP")
        .with_property("partition.flags", "boot, esp")
        .with_property("filesystem.type", "vfat")
        .with_property("blkid.type", "vfat")
        .with_property("blkid.version", "FAT32")
        .with_property("blkid.block-size", "512")
        .with_property("blkid.usage", "filesystem")
        .with_property("blkid.partlabel", "EFI System Partition"),
    );

    let mut output = Vec::new();
    print_partitions(&mut output, &graph).expect("partitions table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("DETAILS"));
    assert!(output.contains("1111-2222"));
    assert!(output.contains(
            "fstype=vfat blkid-type=vfat version=FAT32 blkid-block-size=512 usage=filesystem partlabel=EFI System Partition partno=1 start=1049kB start-bytes=1049000 end=538MB end-bytes=538000000 type=fat32 part-name=ESP flags=boot, esp"
        ));
}

#[test]
fn usage_percent_prefers_size_then_allocated_then_used_plus_free() {
    let sized = Node::new("filesystem:root", NodeKind::Filesystem, "/")
        .with_size_bytes(100)
        .with_usage(Usage {
            used_bytes: Some(25),
            free_bytes: Some(75),
            allocated_bytes: Some(50),
        });
    assert_eq!(usage_percent(&sized), "25.0%");

    let allocated = Node::new("btrfs:data", NodeKind::BtrfsFilesystem, "data").with_usage(Usage {
        used_bytes: Some(25),
        free_bytes: None,
        allocated_bytes: Some(50),
    });
    assert_eq!(usage_percent(&allocated), "50.0%");

    let used_free = Node::new("swap:/dev/sda3", NodeKind::Swap, "/dev/sda3").with_usage(Usage {
        used_bytes: Some(25),
        free_bytes: Some(75),
        allocated_bytes: None,
    });
    assert_eq!(usage_percent(&used_free), "25.0%");
}

#[test]
fn usage_details_surfaces_storage_metadata() {
    let lv = Node::new("lv:vg/thin", NodeKind::LvmLogicalVolume, "vg/thin")
        .with_size_bytes(100)
        .with_usage(Usage {
            used_bytes: Some(25),
            free_bytes: Some(75),
            allocated_bytes: None,
        })
        .with_property("lvm.data-percent", "12.50")
        .with_property("lvm.metadata-percent", "3.00")
        .with_property("lvm.snap-percent", "4.00")
        .with_property("lvm.copy-percent", "99.00")
        .with_property("lvm.active", "active")
        .with_property("lvm.layout", "thin")
        .with_property("lvm.health", "ok")
        .with_property("lvm.when-full", "queue")
        .with_property("lvm.metadata-size", "128.00m")
        .with_property("lvm.role", "public")
        .with_property("lvm.cache-mode", "writeback")
        .with_property("lvm.cache-policy", "smq")
        .with_property("lvm.kernel-discards", "passdown")
        .with_property("lvm.writecache-writeback-blocks", "16");
    assert_eq!(
            usage_details(&lv),
            "data=12.50 metadata=3.00 snap=4.00 copy=99.00 layout=thin active=active health=ok when-full=queue metadata-size=128.00m role=public cache-mode=writeback cache-policy=smq kernel-discards=passdown writecache-writeback=16"
        );

    let pool = Node::new("zpool:tank", NodeKind::ZfsPool, "tank")
        .with_size_bytes(100)
        .with_property("zfs.health", "ONLINE");
    assert_eq!(usage_details(&pool), "health=ONLINE");

    let snapshot = Node::new(
        "zfs-snapshot:tank/home@daily",
        NodeKind::ZfsSnapshot,
        "tank/home@daily",
    )
    .with_property("zfs.userrefs", "2");
    let snapshot = snapshot.with_property("zfs.holds", "disk-nix-retain");
    assert_eq!(usage_details(&snapshot), "userrefs=2 holds=disk-nix-retain");

    let dataset = Node::new("zfs-dataset:tank/home", NodeKind::ZfsDataset, "tank/home")
        .with_property("zfs.compression", "zstd")
        .with_property("zfs.encryption", "aes-256-gcm")
        .with_property("zfs.keystatus", "available");
    assert_eq!(
        usage_details(&dataset),
        "compression=zstd encryption=aes-256-gcm keystatus=available"
    );

    let xfs = Node::new("mount:/", NodeKind::Mountpoint, "/")
        .with_property("xfs.meta-data.meta-data", "/dev/mapper/vg-root")
        .with_property("xfs.meta-data.isize", "512")
        .with_property("xfs.meta-data.agcount", "4")
        .with_property("xfs.meta-data.agsize", "65536")
        .with_property("xfs.meta-data.sectsz", "512")
        .with_property("xfs.meta-data.attr", "2")
        .with_property("xfs.meta-data.projid32bit", "1")
        .with_property("xfs.meta-data.crc", "1")
        .with_property("xfs.meta-data.finobt", "1")
        .with_property("xfs.meta-data.sparse", "1")
        .with_property("xfs.meta-data.rmapbt", "0")
        .with_property("xfs.data.blocks", "262144")
        .with_property("xfs.data.bsize", "4096")
        .with_property("xfs.data.imaxpct", "25")
        .with_property("xfs.data.sunit", "0")
        .with_property("xfs.data.swidth", "0")
        .with_property("xfs.meta-data.reflink", "1")
        .with_property("xfs.meta-data.bigtime", "1")
        .with_property("xfs.meta-data.inobtcount", "1")
        .with_property("xfs.meta-data.nrext64", "0")
        .with_property("xfs.naming.version", "2")
        .with_property("xfs.naming.bsize", "4096")
        .with_property("xfs.naming.ascii-ci", "0")
        .with_property("xfs.naming.ftype", "1")
        .with_property("xfs.log.type", "internal log")
        .with_property("xfs.log.bsize", "4096")
        .with_property("xfs.log.blocks", "2560")
        .with_property("xfs.log.version", "2")
        .with_property("xfs.log.sectsz", "512")
        .with_property("xfs.log.sunit", "0")
        .with_property("xfs.log.lazy-count", "1")
        .with_property("xfs.realtime.type", "none")
        .with_property("xfs.realtime.extsz", "4096")
        .with_property("xfs.realtime.blocks", "0")
        .with_property("xfs.realtime.rtextents", "0");
    assert_eq!(
            usage_details(&xfs),
            "xfs-source=/dev/mapper/vg-root xfs-isize=512 xfs-agcount=4 xfs-agsize=65536 xfs-sectsz=512 xfs-attr=2 xfs-projid32bit=1 xfs-crc=1 xfs-finobt=1 xfs-sparse=1 xfs-rmapbt=0 xfs-blocks=262144 xfs-bsize=4096 xfs-imaxpct=25 xfs-sunit=0 xfs-swidth=0 reflink=1 bigtime=1 xfs-inobtcount=1 xfs-nrext64=0 xfs-naming-version=2 xfs-naming-bsize=4096 xfs-ascii-ci=0 xfs-ftype=1 xfs-log-type=internal log xfs-log-bsize=4096 log-blocks=2560 xfs-log-version=2 xfs-log-sectsz=512 xfs-log-sunit=0 xfs-log-lazy-count=1 xfs-realtime-type=none xfs-realtime-extsz=4096 xfs-realtime-blocks=0 xfs-realtime-rtextents=0"
        );

    let ext = Node::new("fs:/dev/sda2", NodeKind::Filesystem, "ext4")
        .with_property("filesystem.type", "ext4")
        .with_property("blkid.version", "1.0")
        .with_property("blkid.block-size", "4096")
        .with_property("blkid.usage", "filesystem")
        .with_property("blkid.uuid-sub", "subvol-uuid")
        .with_property("ext.state", "clean")
        .with_property("ext.magic-number", "0xEF53")
        .with_property("ext.revision", "1 (dynamic)")
        .with_property("ext.errors-behavior", "Continue")
        .with_property("ext.fs-error-count", "2")
        .with_property("ext.os-type", "Linux")
        .with_property("ext.block-count", "122096646")
        .with_property("ext.reserved-block-count", "6104832")
        .with_property("ext.overhead-clusters", "123456")
        .with_property("ext.free-blocks", "73328197")
        .with_property("ext.first-block", "0")
        .with_property("ext.block-size", "4096")
        .with_property("ext.fragment-size", "4096")
        .with_property("ext.blocks-per-group", "32768")
        .with_property("ext.fragments-per-group", "32768")
        .with_property("ext.inode-count", "30531584")
        .with_property("ext.free-inodes", "27187554")
        .with_property("ext.inodes-per-group", "8192")
        .with_property("ext.raid-stride", "128")
        .with_property("ext.raid-stripe-width", "256")
        .with_property("ext.features", "has_journal extent metadata_csum")
        .with_property("ext.flags", "signed_directory_hash")
        .with_property("ext.default-directory-hash", "half_md4")
        .with_property(
            "ext.directory-hash-seed",
            "11111111-2222-3333-4444-555555555555",
        )
        .with_property("ext.default-mount-options", "user_xattr acl")
        .with_property("ext.created", "Mon Jan 01 00:00:00 2024")
        .with_property("ext.last-mount-time", "Mon Jun 22 12:00:00 2026")
        .with_property("ext.last-write-time", "Mon Jun 22 12:00:00 2026")
        .with_property("ext.mount-count", "12")
        .with_property("ext.maximum-mount-count", "-1")
        .with_property("ext.last-checked", "Mon Jan 01 00:00:00 2024")
        .with_property("ext.check-interval", "0 (<none>)")
        .with_property("ext.lifetime-writes", "189 GB")
        .with_property("ext.reserved-blocks-uid", "0 (user root)")
        .with_property("ext.reserved-blocks-gid", "0 (group root)")
        .with_property("ext.first-inode", "11")
        .with_property("ext.inode-size", "256")
        .with_property("ext.journal-inode", "8")
        .with_property("ext.journal-uuid", "99999999-aaaa-bbbb-cccc-dddddddddddd")
        .with_property("ext.journal-backup", "inode blocks")
        .with_property("ext.journal-features", "journal_incompat_revoke")
        .with_property("ext.journal-size", "1024M")
        .with_property("ext.first-error-time", "Mon Jun 22 12:30:00 2026")
        .with_property("ext.first-error-function", "ext4_lookup")
        .with_property("ext.first-error-line", "1234")
        .with_property("ext.first-error-inode", "42")
        .with_property("ext.first-error-block", "9001")
        .with_property("ext.last-error-time", "Mon Jun 22 12:45:00 2026")
        .with_property("ext.last-error-function", "ext4_journal_check_start")
        .with_property("ext.last-error-line", "5678")
        .with_property("ext.last-error-inode", "43")
        .with_property("ext.last-error-block", "9002")
        .with_property("ext.checksum-type", "crc32c")
        .with_property("ext.checksum", "0x12345678");
    assert_eq!(
            usage_details(&ext),
            "fstype=ext4 version=1.0 blkid-block-size=4096 usage=filesystem uuid-sub=subvol-uuid ext-state=clean ext-magic=0xEF53 ext-revision=1 (dynamic) errors=Continue fs-error-count=2 os=Linux blocks=122096646 reserved-blocks=6104832 overhead-clusters=123456 free-blocks=73328197 first-block=0 block-size=4096 fragment-size=4096 blocks-per-group=32768 fragments-per-group=32768 inodes=30531584 free-inodes=27187554 inodes-per-group=8192 raid-stride=128 raid-stripe-width=256 features=has_journal extent metadata_csum flags=signed_directory_hash dir-hash=half_md4 dir-hash-seed=11111111-2222-3333-4444-555555555555 default-mount=user_xattr acl created=Mon Jan 01 00:00:00 2024 last-mounted=Mon Jun 22 12:00:00 2026 last-written=Mon Jun 22 12:00:00 2026 mount-count=12 max-mount-count=-1 last-checked=Mon Jan 01 00:00:00 2024 check-interval=0 (<none>) lifetime-writes=189 GB reserved-uid=0 (user root) reserved-gid=0 (group root) first-inode=11 inode-size=256 journal-inode=8 journal-uuid=99999999-aaaa-bbbb-cccc-dddddddddddd journal-backup=inode blocks journal-features=journal_incompat_revoke journal-size=1024M first-error-time=Mon Jun 22 12:30:00 2026 first-error-function=ext4_lookup first-error-line=1234 first-error-inode=42 first-error-block=9001 last-error-time=Mon Jun 22 12:45:00 2026 last-error-function=ext4_journal_check_start last-error-line=5678 last-error-inode=43 last-error-block=9002 checksum-type=crc32c checksum=0x12345678"
        );

    let exfat = Node::new("fs:/dev/sdb1", NodeKind::Filesystem, "exfat")
        .with_property("exfat.guid", "01234567-89ab-cdef-0123-456789abcdef")
        .with_property("exfat.volume-label", "SHARED")
        .with_property("exfat.exfatprogs-version", "1.2.4")
        .with_property("exfat.volume-serial", "0x6eef953b")
        .with_property("exfat.volume-length-sectors", "3203072")
        .with_property("exfat.fat-offset-sector-offset", "2048")
        .with_property("exfat.fat-length-sectors", "448")
        .with_property("exfat.cluster-heap-offset-sector-offset", "4096")
        .with_property("exfat.cluster-count", "49984")
        .with_property("exfat.used-clusters", "48960")
        .with_property("exfat.free-clusters", "1024")
        .with_property("exfat.root-cluster-cluster-offset", "4")
        .with_property("exfat.bytes-per-sector", "512")
        .with_property("exfat.sectors-per-cluster", "64")
        .with_property("exfat.bytes-per-cluster", "32768");
    assert_eq!(
            usage_details(&exfat),
            "guid=01234567-89ab-cdef-0123-456789abcdef exfat-label=SHARED exfatprogs=1.2.4 serial=0x6eef953b sectors=3203072 fat-offset=2048 fat-length=448 cluster-heap-offset=4096 clusters=49984 used-clusters=48960 free-clusters=1024 root-cluster=4 sector-bytes=512 sectors-per-cluster=64 cluster-bytes=32768"
        );

    let ntfs = Node::new("fs:/dev/sda1", NodeKind::Filesystem, "ntfs")
        .with_property("ntfs.device-name", "/dev/sda1")
        .with_property("ntfs.device-state", "11")
        .with_property("ntfs.volume-name", "Windows")
        .with_property("ntfs.volume-serial", "01234567-89abcdef")
        .with_property("ntfs.version", "3.1")
        .with_property("ntfs.sector-size", "512")
        .with_property("ntfs.cluster-size", "4096")
        .with_property("ntfs.volume-size-clusters", "262144")
        .with_property("ntfs.mft-record-size", "1024")
        .with_property("ntfs.mft-zone-multiplier", "0")
        .with_property("ntfs.mft-zone-start", "786432")
        .with_property("ntfs.mft-zone-end", "819200")
        .with_property("ntfs.mft-data-position", "786944")
        .with_property("ntfs.mft-lcn", "4");
    assert_eq!(
            usage_details(&ntfs),
            "ntfs-device=/dev/sda1 ntfs-device-state=11 ntfs-name=Windows ntfs-serial=01234567-89abcdef ntfs-version=3.1 ntfs-sector=512 ntfs-cluster=4096 ntfs-clusters=262144 ntfs-mft-record=1024 ntfs-mft-zone-multiplier=0 ntfs-mft-zone-start=786432 ntfs-mft-zone-end=819200 ntfs-mft-data-position=786944 ntfs-mft-lcn=4"
        );

    let f2fs = Node::new("fs:/dev/sdb2", NodeKind::Filesystem, "f2fs")
        .with_property("f2fs.filesystem-volume-name", "mobile")
        .with_property(
            "f2fs.filesystem-uuid",
            "01234567-89ab-cdef-0123-456789abcdef",
        )
        .with_property("f2fs.block-size", "4096")
        .with_property("f2fs.block-count", "262144")
        .with_property("f2fs.user-block-count", "245760")
        .with_property("f2fs.valid-block-count", "65536")
        .with_property("f2fs.total-valid-block-count", "65540")
        .with_property("f2fs.valid-node-count", "4096")
        .with_property("f2fs.valid-inode-count", "2048")
        .with_property("f2fs.segment-count", "2048")
        .with_property("f2fs.segment-count-main", "1984")
        .with_property("f2fs.segment-count-ckpt", "2")
        .with_property("f2fs.segment-count-sit", "2")
        .with_property("f2fs.segment-count-nat", "4")
        .with_property("f2fs.segment-count-ssa", "1")
        .with_property("f2fs.overprov-segment-count", "64")
        .with_property("f2fs.section-count", "1984")
        .with_property("f2fs.segs-per-sec", "1")
        .with_property("f2fs.secs-per-zone", "1")
        .with_property("f2fs.log-sectorsize", "9")
        .with_property("f2fs.log-sectors-per-block", "3")
        .with_property("f2fs.log-blocksize", "12")
        .with_property("f2fs.log-blocks-per-seg", "9")
        .with_property("f2fs.cp-payload", "0")
        .with_property("f2fs.version", "Linux version 6.12")
        .with_property("f2fs.init-version", "Linux version 6.1")
        .with_property("f2fs.extension-count", "29")
        .with_property("f2fs.hot-ext-count", "5");
    assert_eq!(
            usage_details(&f2fs),
            "f2fs-uuid=01234567-89ab-cdef-0123-456789abcdef f2fs-name=mobile f2fs-block-size=4096 f2fs-blocks=262144 f2fs-user-blocks=245760 f2fs-valid-blocks=65536 f2fs-total-valid-blocks=65540 f2fs-valid-nodes=4096 f2fs-valid-inodes=2048 f2fs-segments=2048 f2fs-main-segments=1984 f2fs-ckpt-segments=2 f2fs-sit-segments=2 f2fs-nat-segments=4 f2fs-ssa-segments=1 f2fs-overprov=64 f2fs-sections=1984 f2fs-segs-per-sec=1 f2fs-secs-per-zone=1 f2fs-log-sector=9 f2fs-log-sectors-block=3 f2fs-log-block=12 f2fs-log-blocks-seg=9 f2fs-cp-payload=0 f2fs-version=Linux version 6.12 f2fs-init-version=Linux version 6.1 f2fs-extensions=29 f2fs-hot-extensions=5"
        );

    let bcachefs = Node::new(
        "bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3",
        NodeKind::BcachefsFilesystem,
        "archive",
    )
    .with_property(
        "bcachefs.external-uuid",
        "a2d6fc04-efd0-4e36-aece-2475941d09a3",
    )
    .with_property(
        "bcachefs.internal-uuid",
        "55083d1e-27cf-4929-ada4-3fe6e45cf02c",
    )
    .with_property(
        "bcachefs.magic-number",
        "c68573f6-66ce-90a9-d96a-60cf803df7ef",
    )
    .with_property("bcachefs.device", "ST12000NM001G-2M")
    .with_property("bcachefs.member-device", "/dev/sdc")
    .with_property("bcachefs.mount-target", "/mnt/archive")
    .with_property("bcachefs.device-index", "6")
    .with_property("bcachefs.version", "1.20: (unknown version)")
    .with_property(
        "bcachefs.version-upgrade-complete",
        "1.20: (unknown version)",
    )
    .with_property("bcachefs.online-reserved", "507957248")
    .with_property("bcachefs.device-count", "2")
    .with_property("bcachefs.data-sb", "3149824")
    .with_property("bcachefs.data-journal", "4294967296")
    .with_property("bcachefs.data-btree", "1048576")
    .with_property("bcachefs.data-user", "2147483648");
    assert_eq!(
            usage_details(&bcachefs),
            "bcachefs-uuid=a2d6fc04-efd0-4e36-aece-2475941d09a3 bcachefs-internal=55083d1e-27cf-4929-ada4-3fe6e45cf02c bcachefs-magic=c68573f6-66ce-90a9-d96a-60cf803df7ef bcachefs-super-device=ST12000NM001G-2M bcachefs-member=/dev/sdc bcachefs-mount=/mnt/archive bcachefs-device=6 bcachefs-version=1.20: (unknown version) bcachefs-upgrade-complete=1.20: (unknown version) bcachefs-reserved=507957248 bcachefs-devices=2 bcachefs-sb=3149824 bcachefs-journal=4294967296 bcachefs-btree=1048576 bcachefs-user=2147483648"
        );

    let bcachefs_device = Node::new(
        "bcachefs-device:a2d6fc04-efd0-4e36-aece-2475941d09a3:6",
        NodeKind::BcachefsDevice,
        "sdc",
    )
    .with_property("bcachefs.device-label", "hdd.archive")
    .with_property("bcachefs.device-state", "rw")
    .with_property("bcachefs.device-free", "1649975230464")
    .with_property("bcachefs.device-capacity", "16000900661248")
    .with_property("bcachefs.device-data-sb", "3149824")
    .with_property("bcachefs.device-data-journal", "4294967296")
    .with_property("bcachefs.device-data-btree", "890241024")
    .with_property("bcachefs.device-data-user", "0");
    assert_eq!(
            usage_details(&bcachefs_device),
            "bcachefs-label=hdd.archive bcachefs-state=rw bcachefs-device-free=1649975230464 bcachefs-device-capacity=16000900661248 bcachefs-device-sb=3149824 bcachefs-device-journal=4294967296 bcachefs-device-btree=890241024 bcachefs-device-user=0"
        );

    let bcache = Node::new("block:/dev/bcache0", NodeKind::CacheDevice, "bcache0")
        .with_property("bcache.role", "backing")
        .with_property("bcache.kind", "cache-set")
        .with_property("bcache.backing-device", "/dev/sdb1")
        .with_property("bcache.set-uuid", "cache-set-uuid")
        .with_property("bcache.label", "fast-cache")
        .with_property("bcache.state", "clean")
        .with_property("bcache.running", "1")
        .with_property("bcache.cache-available-percent", "78")
        .with_property("bcache.cache-mode", "writeback")
        .with_property("bcache.cache-replacement-policy", "lru")
        .with_property("bcache.congested-read-threshold-us", "2000")
        .with_property("bcache.congested-write-threshold-us", "20000")
        .with_property("bcache.discard", "true")
        .with_property("bcache.dirty-data", "64.0M")
        .with_property("bcache.io-errors", "0")
        .with_property("bcache.metadata-written", "128.0M")
        .with_property("bcache.priority-stats", "Unused: 0% Metadata: 1%")
        .with_property("bcache.readahead", "0")
        .with_property("bcache.sequential-cutoff", "4.0M")
        .with_property("bcache.written", "512.0M")
        .with_property("bcache.writeback-delay", "30")
        .with_property("bcache.writeback-metadata", "true")
        .with_property("bcache.writeback-percent", "10")
        .with_property("bcache.writeback-rate", "1.0M/sec")
        .with_property("bcache.writeback-rate-debug", "rate=1024")
        .with_property("bcache.writeback-rate-d-term", "30")
        .with_property("bcache.writeback-rate-i-term-inverse", "10000")
        .with_property("bcache.writeback-rate-minimum", "4.0k")
        .with_property("bcache.writeback-rate-p-term-inverse", "40")
        .with_property("bcache.writeback-rate-update-seconds", "5")
        .with_property("bcache.writeback-running", "1");
    assert_eq!(
            usage_details(&bcache),
            "role=backing kind=cache-set backing-device=/dev/sdb1 set-uuid=cache-set-uuid label=fast-cache state=clean running=1 available-percent=78 cache-mode=writeback replacement=lru congested-read-us=2000 congested-write-us=20000 discard=true dirty=64.0M io-errors=0 metadata-written=128.0M priority-stats=Unused: 0% Metadata: 1% readahead=0 sequential-cutoff=4.0M written=512.0M writeback-delay=30 writeback-metadata=true writeback-percent=10 writeback-rate=1.0M/sec writeback-rate-debug=rate=1024 writeback-rate-d-term=30 writeback-rate-i-inverse=10000 writeback-rate-min=4.0k writeback-rate-p-inverse=40 writeback-rate-update=5 writeback-running=1"
        );

    let swap = Node::new("swap:/dev/zram0", NodeKind::Swap, "/dev/zram0")
        .with_property("swap.active", "true")
        .with_property("swap.type", "partition")
        .with_property("swap.priority", "100");
    assert_eq!(
        usage_details(&swap),
        "swap-active=true swap-type=partition swap-priority=100"
    );

    let loop_device = Node::new("block:/dev/loop0", NodeKind::LoopDevice, "/dev/loop0")
        .with_property("loop.back-file", "/var/lib/images/root.img")
        .with_property("loop.backing-inode", "12345")
        .with_property("loop.backing-major-minor", "0:45")
        .with_property("loop.major-minor", "7:0")
        .with_property("loop.offset", "1048576")
        .with_property("loop.sizelimit", "1073741824")
        .with_property("loop.logical-sector-size", "512")
        .with_property("loop.autoclear", "true")
        .with_property("loop.partscan", "true")
        .with_property("loop.read-only", "false")
        .with_property("loop.direct-io", "true");
    assert_eq!(
            usage_details(&loop_device),
            "back-file=/var/lib/images/root.img back-ino=12345 back-major-minor=0:45 major-minor=7:0 offset=1048576 sizelimit=1073741824 logical-sector=512 autoclear=true partscan=true ro=false dio=true"
        );

    let nvme = Node::new(
        "block:/dev/nvme0n1",
        NodeKind::NvmeNamespace,
        "/dev/nvme0n1",
    )
    .with_property("nvme.generic-path", "/dev/ng0n1")
    .with_property("nvme.model", "Example NVMe")
    .with_property("nvme.product", "Example Controller")
    .with_property("nvme.firmware", "1.0")
    .with_property("nvme.index", "0")
    .with_property("nvme.namespace", "1")
    .with_property("nvme.namespace-id", "1")
    .with_property(
        "nvme.namespace-uuid",
        "12345678-1234-1234-1234-123456789abc",
    )
    .with_property("nvme.eui64", "0011223344556677")
    .with_property("nvme.nguid", "00112233445566778899aabbccddeeff")
    .with_property("nvme.subsystem", "nvme-subsys0")
    .with_property("nvme.controller", "nvme0")
    .with_property("nvme.address", "0000:01:00.0")
    .with_property("nvme.transport", "pcie")
    .with_property("nvme.controller-id", "1")
    .with_property("nvme.namespace-capacity", "900000000000")
    .with_property("nvme.lba-format", "512 B + 0 B")
    .with_property("nvme.maximum-lba", "1953125")
    .with_property("nvme.sector-size", "512")
    .with_property("nvme.ana-state", "optimized");
    assert_eq!(
            usage_details(&nvme),
            "generic=/dev/ng0n1 nvme-model=Example NVMe product=Example Controller firmware=1.0 ns-index=0 namespace=1 nsid=1 ns-uuid=12345678-1234-1234-1234-123456789abc eui64=0011223344556677 nguid=00112233445566778899aabbccddeeff subsystem=nvme-subsys0 controller=nvme0 address=0000:01:00.0 transport=pcie controller-id=1 namespace-capacity=900000000000 lba-format=512 B + 0 B max-lba=1953125 sector-size=512 ana-state=optimized"
        );
}

#[test]
fn usage_table_includes_details_column() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("vdo:archive", NodeKind::VdoVolume, "archive")
            .with_size_bytes(100)
            .with_usage(Usage {
                used_bytes: Some(50),
                free_bytes: Some(50),
                allocated_bytes: None,
            })
            .with_property("vdo.storage-device", "/dev/sdb")
            .with_property("vdo.logical-size", "100G")
            .with_property("vdo.physical-size", "50G")
            .with_property("vdo.use-percent", "50%")
            .with_property("vdo.space-saving-percent", "20%")
            .with_property("vdo.operating-mode", "normal")
            .with_property("vdo.write-policy", "sync")
            .with_property("vdo.configured-write-policy", "auto")
            .with_property("vdo.block-map-cache-size", "128M")
            .with_property("vdo.data-blocks-used", "65536")
            .with_property("vdo.logical-blocks-used", "262144"),
    );

    let mut output = Vec::new();
    print_usage(&mut output, &graph).expect("usage table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("DETAILS"));
    assert!(output.contains("backing=/dev/sdb logical=100G physical=50G"));
    assert!(output.contains(
        "vdo-use=50% saving=20% mode=normal write-policy=sync configured-write-policy=auto"
    ));
    assert!(output.contains("block-map-cache=128M data-blocks=65536 logical-blocks=262144"));
}

#[test]
fn inspect_includes_capacity_usage_identity_properties_and_relationships() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "filesystem:/srv/archive",
            NodeKind::Filesystem,
            "/srv/archive",
        )
        .with_path("/srv/archive")
        .with_size_bytes(1024)
        .with_usage(Usage {
            used_bytes: Some(256),
            free_bytes: Some(768),
            allocated_bytes: Some(512),
        })
        .with_identity(Identity {
            uuid: Some("fs-uuid".to_string()),
            partuuid: None,
            label: Some("archive".to_string()),
            serial: None,
            wwn: None,
        })
        .with_property("filesystem.type", "xfs")
        .with_property("mount.source", "/dev/mapper/archive"),
    );
    graph.add_node(Node::new(
        "block:/dev/mapper/archive",
        NodeKind::DeviceMapper,
        "/dev/mapper/archive",
    ));
    graph.add_edge(Edge::new(
        "block:/dev/mapper/archive",
        "filesystem:/srv/archive",
        Relationship::Backs,
    ));

    let mut output = Vec::new();
    print_inspect(&mut output, &graph, "archive", 1).expect("inspect renders");
    let output = String::from_utf8(output).expect("inspect output is utf8");

    assert!(output.contains("filesystem /srv/archive"));
    assert!(output.contains("  path: /srv/archive"));
    assert!(output.contains("  size: 1.0 KiB"));
    assert!(output.contains("  usage: used=256 B free=768 B allocated=512 B use=25.0%"));
    assert!(output.contains("    uuid: fs-uuid"));
    assert!(output.contains("    label: archive"));
    assert!(output.contains("    filesystem.type: xfs"));
    assert!(output.contains("    mount.source: /dev/mapper/archive"));
    assert!(output.contains("    in backs block:/dev/mapper/archive (/dev/mapper/archive)"));
}

#[test]
fn inspect_json_depth_walks_layered_relationships() {
    let mut graph = StorageGraph::empty();
    graph.add_node(Node::new(
        "block:/dev/nvme0n1p2",
        NodeKind::Partition,
        "/dev/nvme0n1p2",
    ));
    graph.add_node(Node::new(
        "block:/dev/mapper/cryptroot",
        NodeKind::LuksContainer,
        "cryptroot",
    ));
    graph.add_node(Node::new(
        "lvm-lv:vg/root",
        NodeKind::LvmLogicalVolume,
        "vg/root",
    ));
    graph.add_node(Node::new("filesystem:/", NodeKind::Filesystem, "/"));
    graph.add_edge(Edge::new(
        "block:/dev/nvme0n1p2",
        "block:/dev/mapper/cryptroot",
        Relationship::Backs,
    ));
    graph.add_edge(Edge::new(
        "block:/dev/mapper/cryptroot",
        "lvm-lv:vg/root",
        Relationship::Backs,
    ));
    graph.add_edge(Edge::new(
        "lvm-lv:vg/root",
        "filesystem:/",
        Relationship::Backs,
    ));

    let mut output = Vec::new();
    print_inspect_json(&mut output, &graph, "/", 2).expect("inspect json renders");
    let output = String::from_utf8(output).expect("json is utf8");
    let graph: StorageGraph = serde_json::from_str(&output).expect("valid storage graph json");

    assert_eq!(graph.nodes.len(), 3);
    assert!(graph.nodes.iter().any(|node| node.id.0 == "filesystem:/"));
    assert!(graph.nodes.iter().any(|node| node.id.0 == "lvm-lv:vg/root"));
    assert!(graph
        .nodes
        .iter()
        .any(|node| node.id.0 == "block:/dev/mapper/cryptroot"));
    assert!(graph
        .nodes
        .iter()
        .all(|node| node.id.0 != "block:/dev/nvme0n1p2"));
    assert_eq!(graph.edges.len(), 2);
}

#[test]
fn filesystems_table_includes_metadata_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("fs-source:/dev/mapper/vg-root", NodeKind::Filesystem, "xfs")
            .with_usage(Usage {
                used_bytes: Some(512),
                free_bytes: Some(512),
                allocated_bytes: None,
            })
            .with_property("xfs.meta-data.meta-data", "/dev/mapper/vg-root")
            .with_property("xfs.meta-data.isize", "512")
            .with_property("xfs.meta-data.agcount", "4")
            .with_property("xfs.meta-data.crc", "1")
            .with_property("xfs.data.blocks", "262144")
            .with_property("xfs.data.bsize", "4096")
            .with_property("xfs.data.imaxpct", "25")
            .with_property("xfs.meta-data.reflink", "1")
            .with_property("xfs.meta-data.bigtime", "1")
            .with_property("xfs.naming.version", "2")
            .with_property("xfs.naming.ftype", "1")
            .with_property("xfs.log.type", "internal log")
            .with_property("xfs.log.blocks", "2560")
            .with_property("xfs.realtime.type", "none")
            .with_property("xfs.realtime.blocks", "0"),
    );
    graph.add_node(
        Node::new("fs:/dev/sda2", NodeKind::Filesystem, "ext4")
            .with_property("filesystem.type", "ext4")
            .with_property("ext.state", "clean")
            .with_property("ext.magic-number", "0xEF53")
            .with_property("ext.revision", "1 (dynamic)")
            .with_property("ext.errors-behavior", "Continue")
            .with_property("ext.fs-error-count", "2")
            .with_property("ext.os-type", "Linux")
            .with_property("ext.block-count", "122096646")
            .with_property("ext.reserved-block-count", "6104832")
            .with_property("ext.overhead-clusters", "123456")
            .with_property("ext.free-blocks", "73328197")
            .with_property("ext.first-block", "0")
            .with_property("ext.block-size", "4096")
            .with_property("ext.blocks-per-group", "32768")
            .with_property("ext.inode-count", "30531584")
            .with_property("ext.free-inodes", "27187554")
            .with_property("ext.inodes-per-group", "8192")
            .with_property("ext.raid-stride", "128")
            .with_property("ext.raid-stripe-width", "256")
            .with_property("ext.features", "has_journal extent metadata_csum")
            .with_property("ext.flags", "signed_directory_hash")
            .with_property("ext.default-directory-hash", "half_md4")
            .with_property(
                "ext.directory-hash-seed",
                "11111111-2222-3333-4444-555555555555",
            )
            .with_property("ext.default-mount-options", "user_xattr acl")
            .with_property("ext.mount-count", "12")
            .with_property("ext.maximum-mount-count", "-1")
            .with_property("ext.check-interval", "0 (<none>)")
            .with_property("ext.inode-size", "256")
            .with_property("ext.journal-inode", "8")
            .with_property("ext.journal-uuid", "99999999-aaaa-bbbb-cccc-dddddddddddd")
            .with_property("ext.journal-size", "1024M")
            .with_property("ext.first-error-function", "ext4_lookup")
            .with_property("ext.first-error-block", "9001")
            .with_property("ext.last-error-function", "ext4_journal_check_start")
            .with_property("ext.last-error-block", "9002")
            .with_property("ext.checksum-type", "crc32c")
            .with_property("ext.checksum", "0x12345678"),
    );
    graph.add_node(
        Node::new("fs:/dev/sdb1", NodeKind::Filesystem, "exfat")
            .with_property("exfat.guid", "01234567-89ab-cdef-0123-456789abcdef")
            .with_property("exfat.volume-label", "SHARED")
            .with_property("exfat.exfatprogs-version", "1.2.4")
            .with_property("exfat.volume-serial", "0x6eef953b")
            .with_property("exfat.volume-length-sectors", "3203072")
            .with_property("exfat.fat-offset-sector-offset", "2048")
            .with_property("exfat.fat-length-sectors", "448")
            .with_property("exfat.cluster-heap-offset-sector-offset", "4096")
            .with_property("exfat.cluster-count", "49984")
            .with_property("exfat.used-clusters", "48960")
            .with_property("exfat.free-clusters", "1024")
            .with_property("exfat.root-cluster-cluster-offset", "4")
            .with_property("exfat.bytes-per-sector", "512")
            .with_property("exfat.sectors-per-cluster", "64")
            .with_property("exfat.bytes-per-cluster", "32768"),
    );
    graph.add_node(
        Node::new("btrfs:fs-uuid", NodeKind::BtrfsFilesystem, "data")
            .with_property("btrfs.mount-target", "/data")
            .with_property("btrfs.data-profile", "single")
            .with_property("btrfs.data-size", "512")
            .with_property("btrfs.data-used", "400")
            .with_property("btrfs.metadata-profile", "DUP")
            .with_property("btrfs.metadata-size", "128")
            .with_property("btrfs.metadata-used", "64")
            .with_property("btrfs.system-profile", "DUP")
            .with_property("btrfs.system-size", "64")
            .with_property("btrfs.system-used", "32"),
    );
    graph.add_node(
        Node::new("fs:/dev/sda1", NodeKind::Filesystem, "ntfs")
            .with_property("ntfs.device-name", "/dev/sda1")
            .with_property("ntfs.device-state", "11")
            .with_property("ntfs.volume-name", "Windows")
            .with_property("ntfs.volume-serial", "01234567-89abcdef")
            .with_property("ntfs.version", "3.1")
            .with_property("ntfs.cluster-size", "4096")
            .with_property("ntfs.mft-record-size", "1024")
            .with_property("ntfs.mft-zone-multiplier", "0")
            .with_property("ntfs.mft-zone-start", "786432")
            .with_property("ntfs.mft-zone-end", "819200")
            .with_property("ntfs.mft-data-position", "786944")
            .with_property("ntfs.mft-lcn", "4"),
    );
    graph.add_node(
        Node::new("fs:/dev/sdb2", NodeKind::Filesystem, "f2fs")
            .with_property("f2fs.filesystem-volume-name", "mobile")
            .with_property("f2fs.block-size", "4096")
            .with_property("f2fs.block-count", "262144")
            .with_property("f2fs.user-block-count", "245760")
            .with_property("f2fs.valid-block-count", "65536")
            .with_property("f2fs.segment-count", "2048")
            .with_property("f2fs.segment-count-main", "1984")
            .with_property("f2fs.segment-count-ckpt", "2")
            .with_property("f2fs.segment-count-sit", "2")
            .with_property("f2fs.segment-count-nat", "4")
            .with_property("f2fs.segment-count-ssa", "1")
            .with_property("f2fs.overprov-segment-count", "64")
            .with_property("f2fs.section-count", "1984")
            .with_property("f2fs.segs-per-sec", "1")
            .with_property("f2fs.secs-per-zone", "1")
            .with_property("f2fs.version", "Linux version 6.12"),
    );
    graph.add_node(
        Node::new(
            "bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3",
            NodeKind::BcachefsFilesystem,
            "archive",
        )
        .with_property(
            "bcachefs.external-uuid",
            "a2d6fc04-efd0-4e36-aece-2475941d09a3",
        )
        .with_property("bcachefs.member-device", "/dev/sdc")
        .with_property("bcachefs.mount-target", "/mnt/archive")
        .with_property("bcachefs.device-index", "6")
        .with_property(
            "bcachefs.magic-number",
            "c68573f6-66ce-90a9-d96a-60cf803df7ef",
        )
        .with_property(
            "bcachefs.version-upgrade-complete",
            "1.20: (unknown version)",
        )
        .with_property("bcachefs.data-sb", "3149824")
        .with_property("bcachefs.data-journal", "4294967296")
        .with_property("bcachefs.data-user", "2147483648"),
    );

    let mut output = Vec::new();
    print_filesystems(&mut output, &graph).expect("filesystems table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("DETAILS"));
    assert!(output.contains("xfs-source=/dev/mapper/vg-root xfs-isize=512 xfs-agcount=4"));
    assert!(output.contains("xfs-crc=1 xfs-blocks=262144 xfs-bsize=4096"));
    assert!(output.contains("xfs-imaxpct=25 reflink=1 bigtime=1"));
    assert!(output
        .contains("xfs-naming-version=2 xfs-ftype=1 xfs-log-type=internal log log-blocks=2560"));
    assert!(output.contains("xfs-realtime-type=none xfs-realtime-blocks=0"));
    assert!(output.contains(
            "fstype=ext4 ext-state=clean ext-magic=0xEF53 ext-revision=1 (dynamic) errors=Continue fs-error-count=2 os=Linux blocks=122096646 reserved-blocks=6104832 overhead-clusters=123456 free-blocks=73328197 first-block=0"
        ));
    assert!(output.contains(
            "first-error-function=ext4_lookup first-error-block=9001 last-error-function=ext4_journal_check_start last-error-block=9002"
        ));
    assert!(output.contains(
            "block-size=4096 blocks-per-group=32768 inodes=30531584 free-inodes=27187554 inodes-per-group=8192 raid-stride=128 raid-stripe-width=256"
        ));
    assert!(output.contains(
            "features=has_journal extent metadata_csum flags=signed_directory_hash dir-hash=half_md4 dir-hash-seed=11111111-2222-3333-4444-555555555555"
        ));
    assert!(output.contains("default-mount=user_xattr acl"));
    assert!(output.contains(
            "mount-count=12 max-mount-count=-1 check-interval=0 (<none>) inode-size=256 journal-inode=8 journal-uuid=99999999-aaaa-bbbb-cccc-dddddddddddd"
        ));
    assert!(output.contains("journal-size=1024M"));
    assert!(output.contains("checksum-type=crc32c checksum=0x12345678"));
    assert!(output.contains(
            "guid=01234567-89ab-cdef-0123-456789abcdef exfat-label=SHARED exfatprogs=1.2.4 serial=0x6eef953b sectors=3203072"
        ));
    assert!(output.contains("fat-offset=2048 fat-length=448 cluster-heap-offset=4096"));
    assert!(output.contains("clusters=49984 used-clusters=48960 free-clusters=1024 root-cluster=4"));
    assert!(output.contains("sector-bytes=512 sectors-per-cluster=64 cluster-bytes=32768"));
    assert!(output.contains(
        "mount-target=/data data-profile=single data-size=512 data-used=400 metadata-profile=DUP"
    ));
    assert!(output.contains(
        "metadata-size=128 metadata-used=64 system-profile=DUP system-size=64 system-used=32"
    ));
    assert!(
            output.contains(
                "ntfs-device=/dev/sda1 ntfs-device-state=11 ntfs-name=Windows ntfs-serial=01234567-89abcdef ntfs-version=3.1 ntfs-cluster=4096 ntfs-mft-record=1024"
            )
        );
    assert!(output.contains(
            "ntfs-mft-zone-multiplier=0 ntfs-mft-zone-start=786432 ntfs-mft-zone-end=819200 ntfs-mft-data-position=786944 ntfs-mft-lcn=4"
        ));
    assert!(output.contains(
            "f2fs-name=mobile f2fs-block-size=4096 f2fs-blocks=262144 f2fs-user-blocks=245760 f2fs-valid-blocks=65536"
        ));
    assert!(output.contains(
            "f2fs-segments=2048 f2fs-main-segments=1984 f2fs-ckpt-segments=2 f2fs-sit-segments=2 f2fs-nat-segments=4 f2fs-ssa-segments=1"
        ));
    assert!(output.contains(
            "f2fs-overprov=64 f2fs-sections=1984 f2fs-segs-per-sec=1 f2fs-secs-per-zone=1 f2fs-version=Linux version 6.12"
        ));
    assert!(output.contains(
            "bcachefs-uuid=a2d6fc04-efd0-4e36-aece-2475941d09a3 bcachefs-magic=c68573f6-66ce-90a9-d96a-60cf803df7ef bcachefs-member=/dev/sdc bcachefs-mount=/mnt/archive"
        ));
    assert!(output.contains(
            "bcachefs-device=6 bcachefs-upgrade-complete=1.20: (unknown version) bcachefs-sb=3149824 bcachefs-journal=4294967296 bcachefs-user=2147483648"
        ));
}

#[test]
fn complex_filesystems_table_includes_topology_and_domain_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(Node::new(
        "block:/dev/nvme0n1p2",
        NodeKind::Partition,
        "/dev/nvme0n1p2",
    ));
    graph.add_node(
        Node::new("btrfs:fs-uuid", NodeKind::BtrfsFilesystem, "/mnt/persist")
            .with_size_bytes(536_870_912_000)
            .with_usage(Usage {
                used_bytes: Some(214_748_364_800),
                free_bytes: Some(322_122_547_200),
                allocated_bytes: None,
            })
            .with_property("btrfs.mount-target", "/mnt/persist")
            .with_property("btrfs.data-profile", "single")
            .with_property("btrfs.metadata-profile", "DUP"),
    );
    graph.add_node(
        Node::new(
            "bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3",
            NodeKind::BcachefsFilesystem,
            "archive",
        )
        .with_usage(Usage {
            used_bytes: Some(2_147_483_648),
            free_bytes: Some(8_589_934_592),
            allocated_bytes: Some(10_737_418_240),
        })
        .with_property("bcachefs.mount-target", "/mnt/archive")
        .with_property("bcachefs.device-count", "2")
        .with_property("bcachefs.data-user", "2147483648"),
    );
    graph.add_node(
        Node::new("zpool:tank", NodeKind::ZfsPool, "tank")
            .with_size_bytes(1_099_511_627_776)
            .with_usage(Usage {
                used_bytes: Some(274_877_906_944),
                free_bytes: Some(824_633_720_832),
                allocated_bytes: None,
            })
            .with_property("zfs.health", "ONLINE"),
    );
    graph.add_node(
        Node::new("zfs-dataset:tank/home", NodeKind::ZfsDataset, "tank/home")
            .with_property("zfs.compression", "zstd")
            .with_property("zfs.encryption", "aes-256-gcm")
            .with_property("zfs.keystatus", "available")
            .with_property("zfs.recordsize", "1048576")
            .with_property("zfs.dedup", "off")
            .with_property("zfs.checksum", "sha512")
            .with_property("zfs.primarycache", "metadata"),
    );
    graph.add_node(
        Node::new(
            "bcachefs-device:a2d6fc04-efd0-4e36-aece-2475941d09a3:0",
            NodeKind::BcachefsDevice,
            "/dev/sdc",
        )
        .with_property("bcachefs.device-state", "rw")
        .with_property("bcachefs.device-free", "8589934592"),
    );
    graph.add_edge(Edge::new(
        "block:/dev/nvme0n1p2",
        "btrfs:fs-uuid",
        Relationship::Backs,
    ));
    graph.add_edge(Edge::new(
        "bcachefs-device:a2d6fc04-efd0-4e36-aece-2475941d09a3:0",
        "bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3",
        Relationship::MemberOf,
    ));

    let mut output = Vec::new();
    print_complex_filesystems(&mut output, &graph).expect("complex filesystems table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("BACKING"));
    assert!(output.contains("/mnt/persist"));
    assert!(output.contains("500.0 GiB"));
    assert!(output.contains("40.0%"));
    assert!(output.contains("data-profile=single metadata-profile=DUP"));
    assert!(output.contains("archive"));
    assert!(output.contains("20.0%"));
    assert!(output.contains("bcachefs-mount=/mnt/archive bcachefs-devices=2"));
    assert!(output.contains("tank"));
    assert!(output.contains("health=ONLINE"));
    assert!(output.contains("tank/home"));
    assert!(output.contains(
        "compression=zstd encryption=aes-256-gcm keystatus=available recordsize=1048576"
    ));
    assert!(output.contains("dedup=off checksum=sha512 primarycache=metadata"));
    assert!(output.contains("bcachefs-state=rw bcachefs-device-free=8589934592"));
}

#[test]
fn btrfs_table_includes_subvolume_qgroup_and_json_neighbors() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "block:/dev/nvme0n1p2",
            NodeKind::Partition,
            "/dev/nvme0n1p2",
        )
        .with_property("btrfs.device-id", "1")
        .with_property("btrfs.device-stat-write-io-errs", "1")
        .with_property("btrfs.device-stat-read-io-errs", "2")
        .with_property("btrfs.device-stat-flush-io-errs", "3")
        .with_property("btrfs.device-stat-corruption-errs", "4")
        .with_property("btrfs.device-stat-generation-errs", "5"),
    );
    graph.add_node(
        Node::new("btrfs:fs-uuid", NodeKind::BtrfsFilesystem, "/mnt/persist")
            .with_size_bytes(536_870_912_000)
            .with_usage(Usage {
                used_bytes: Some(214_748_364_800),
                free_bytes: Some(322_122_547_200),
                allocated_bytes: None,
            })
            .with_property("btrfs.mount-target", "/mnt/persist")
            .with_property("btrfs.data-profile", "single")
            .with_property("btrfs.metadata-profile", "DUP"),
    );
    graph.add_node(
        Node::new(
            "btrfs-subvolume:fs:@home",
            NodeKind::BtrfsSubvolume,
            "@home",
        )
        .with_property("btrfs.id", "257")
        .with_property("btrfs.parent-id", "5")
        .with_property("btrfs.top-level", "5")
        .with_property("btrfs.mount-target", "/mnt/persist/@home"),
    );
    graph.add_node(
        Node::new(
            "btrfs-snapshot:fs:@home-before",
            NodeKind::BtrfsSnapshot,
            "@home-before",
        )
        .with_property("btrfs.id", "258")
        .with_property("btrfs.parent-uuid", "home-subvol")
        .with_property("btrfs.received-uuid", "received-home"),
    );
    graph.add_node(
        Node::new("btrfs-qgroup:0/257", NodeKind::BtrfsQgroup, "0/257")
            .with_property("btrfs.qgroup-id", "0/257")
            .with_property("btrfs.qgroup-parents", "0/5")
            .with_property("btrfs.max-referenced", "25GiB")
            .with_property("btrfs.max-exclusive", "10GiB"),
    );
    graph.add_edge(Edge::new(
        "block:/dev/nvme0n1p2",
        "btrfs:fs-uuid",
        Relationship::Backs,
    ));
    graph.add_edge(Edge::new(
        "btrfs:fs-uuid",
        "btrfs-subvolume:fs:@home",
        Relationship::Contains,
    ));
    graph.add_edge(Edge::new(
        "btrfs-subvolume:fs:@home",
        "btrfs-snapshot:fs:@home-before",
        Relationship::SnapshotOf,
    ));

    let mut output = Vec::new();
    print_btrfs(&mut output, &graph).expect("Btrfs table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("MOUNT"));
    assert!(output.contains("/mnt/persist"));
    assert!(output.contains("500.0 GiB"));
    assert!(output.contains("40.0%"));
    assert!(output.contains("/dev/nvme0n1p2"));
    assert!(output.contains("device-id=1 write-io-errs=1 read-io-errs=2"));
    assert!(output.contains("flush-io-errs=3 corruption-errs=4 generation-errs=5"));
    assert!(output.contains("data-profile=single metadata-profile=DUP"));
    assert!(output.contains("@home"));
    assert!(output.contains("subvol-id=257 parent-id=5 top-level=5"));
    assert!(output.contains("@home-before"));
    assert!(output.contains("parent-uuid=home-subvol received-uuid=received-home"));
    assert!(output.contains("qgroup=0/257 qgroup-parents=0/5"));
    assert!(output.contains("max-rfer=25GiB max-excl=10GiB"));

    let mut json = Vec::new();
    print_filtered_json(&mut json, &graph, is_btrfs_node).expect("Btrfs json renders");
    let json = String::from_utf8(json).expect("json is utf8");
    assert!(json.contains("btrfs:fs-uuid"));
    assert!(json.contains("btrfs-subvolume:fs:@home"));
    assert!(json.contains("btrfs-snapshot:fs:@home-before"));
    assert!(json.contains("block:/dev/nvme0n1p2"));
}

#[test]
fn bcachefs_table_includes_member_usage_and_json_neighbors() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3",
            NodeKind::BcachefsFilesystem,
            "archive",
        )
        .with_size_bytes(10_737_418_240)
        .with_usage(Usage {
            used_bytes: Some(2_147_483_648),
            free_bytes: Some(8_589_934_592),
            allocated_bytes: Some(10_737_418_240),
        })
        .with_property(
            "bcachefs.external-uuid",
            "a2d6fc04-efd0-4e36-aece-2475941d09a3",
        )
        .with_property(
            "bcachefs.internal-uuid",
            "55083d1e-27cf-4929-ada4-3fe6e45cf02c",
        )
        .with_property("bcachefs.mount-target", "/mnt/archive")
        .with_property("bcachefs.device-count", "2")
        .with_property("bcachefs.version", "1.20: (unknown version)")
        .with_property("bcachefs.data-user", "2147483648")
        .with_property("bcachefs.data-cached", "1048576"),
    );
    graph.add_node(
        Node::new(
            "bcachefs-device:a2d6fc04-efd0-4e36-aece-2475941d09a3:0",
            NodeKind::BcachefsDevice,
            "/dev/sdc",
        )
        .with_size_bytes(16_000_900_661_248)
        .with_property("bcachefs.device-label", "hdd.archive")
        .with_property("bcachefs.device-state", "rw")
        .with_property("bcachefs.device-free", "1649975230464")
        .with_property("bcachefs.device-capacity", "16000900661248")
        .with_property("bcachefs.device-data-user", "2147483648"),
    );
    graph.add_edge(Edge::new(
        "bcachefs-device:a2d6fc04-efd0-4e36-aece-2475941d09a3:0",
        "bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3",
        Relationship::MemberOf,
    ));

    let filesystem = graph
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::BcachefsFilesystem)
        .expect("bcachefs filesystem exists");
    assert_eq!(member_count(&graph, filesystem), 1);

    let mut output = Vec::new();
    print_bcachefs(&mut output, &graph).expect("bcachefs table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("MEMBERS"));
    assert!(output.contains("archive"));
    assert!(output.contains("10.0 GiB"));
    assert!(output.contains("20.0%"));
    assert!(output.contains("/mnt/archive"));
    assert!(output.contains("bcachefs-uuid=a2d6fc04-efd0-4e36-aece-2475941d09a3"));
    assert!(output.contains("bcachefs-internal=55083d1e-27cf-4929-ada4-3fe6e45cf02c"));
    assert!(output.contains("bcachefs-version=1.20: (unknown version)"));
    assert!(output.contains("bcachefs-user=2147483648 bcachefs-cached=1048576"));
    assert!(output.contains("hdd.archive"));
    assert!(output.contains("14.6 TiB"));
    assert!(output.contains("bcachefs-label=hdd.archive bcachefs-state=rw"));
    assert!(output.contains("bcachefs-device-free=1649975230464"));
    assert!(output.contains("bcachefs-device-user=2147483648"));

    let mut json = Vec::new();
    print_filtered_json(&mut json, &graph, is_bcachefs_node).expect("bcachefs json renders");
    let json = String::from_utf8(json).expect("json is utf8");
    assert!(json.contains("bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3"));
    assert!(json.contains("bcachefs-device:a2d6fc04-efd0-4e36-aece-2475941d09a3:0"));
}

#[test]
fn zfs_table_includes_pool_vdev_dataset_snapshot_and_zvol_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("zfs-pool:tank", NodeKind::ZfsPool, "tank")
            .with_size_bytes(1_099_511_627_776)
            .with_usage(Usage {
                used_bytes: Some(274_877_906_944),
                free_bytes: Some(824_633_720_832),
                allocated_bytes: Some(274_877_906_944),
            })
            .with_property("zfs.health", "ONLINE")
            .with_property("zfs.state", "ONLINE")
            .with_property("zfs.pool-ashift", "12")
            .with_property("zfs.pool-autotrim", "on")
            .with_property("zfs.pool-autoexpand", "off")
            .with_property("zfs.pool-cachefile", "/etc/zfs/zpool.cache")
            .with_property("zfs.pool-failmode", "wait")
            .with_property("zfs.status", "some devices need attention")
            .with_property("zfs.action", "replace the faulted device")
            .with_property("zfs.scan", "scrub repaired 0B")
            .with_property("zfs.errors", "No known data errors")
            .with_property("zfs.pool-read-errors", "3")
            .with_property("zfs.pool-write-errors", "4")
            .with_property("zfs.pool-checksum-errors", "5"),
    );
    graph.add_node(
        Node::new(
            "zfs-vdev:tank:/dev/disk/by-id/nvme-tank-a",
            NodeKind::ZfsVdev,
            "/dev/disk/by-id/nvme-tank-a",
        )
        .with_path("/dev/disk/by-id/nvme-tank-a")
        .with_property("zfs.vdev-role", "data")
        .with_property("zfs.vdev-state", "ONLINE")
        .with_property("zfs.read-errors", "0")
        .with_property("zfs.write-errors", "1")
        .with_property("zfs.checksum-errors", "2"),
    );
    graph.add_node(
        Node::new("zfs-dataset:tank/home", NodeKind::ZfsDataset, "tank/home")
            .with_usage(Usage {
                used_bytes: Some(107_374_182_400),
                free_bytes: Some(805_306_368_000),
                allocated_bytes: Some(107_374_182_400),
            })
            .with_property("zfs.compression", "zstd")
            .with_property("zfs.quota", "500G")
            .with_property("zfs.reservation", "10G")
            .with_property("zfs.encryption", "aes-256-gcm")
            .with_property("zfs.keystatus", "available")
            .with_property("zfs.recordsize", "1048576")
            .with_property("zfs.dedup", "off")
            .with_property("zfs.checksum", "sha512")
            .with_property("zfs.copies", "2")
            .with_property("zfs.sync", "disabled")
            .with_property("zfs.primarycache", "metadata")
            .with_property("zfs.secondarycache", "all")
            .with_property("zfs.atime", "off")
            .with_property("zfs.relatime", "on")
            .with_property("zfs.snapdir", "visible")
            .with_property("zfs.acltype", "posixacl")
            .with_property("zfs.xattr", "sa"),
    );
    graph.add_node(
        Node::new(
            "zfs-snapshot:tank/home@daily",
            NodeKind::ZfsSnapshot,
            "tank/home@daily",
        )
        .with_property("zfs.userrefs", "2")
        .with_property("zfs.compression", "zstd"),
    );
    graph.add_node(
        Node::new("zvol:tank/vm/root", NodeKind::Zvol, "tank/vm/root")
            .with_size_bytes(85_899_345_920)
            .with_property("zfs.origin", "tank/vm/base@clean")
            .with_property("zfs.volsize", "80G"),
    );
    graph.add_edge(Edge::new(
        "zfs-pool:tank",
        "zfs-vdev:tank:/dev/disk/by-id/nvme-tank-a",
        Relationship::Contains,
    ));
    graph.add_edge(Edge::new(
        "zfs-pool:tank",
        "zfs-dataset:tank/home",
        Relationship::Contains,
    ));
    graph.add_edge(Edge::new(
        "zfs-pool:tank",
        "zvol:tank/vm/root",
        Relationship::Contains,
    ));
    graph.add_edge(Edge::new(
        "zfs-snapshot:tank/home@daily",
        "zfs-dataset:tank/home",
        Relationship::SnapshotOf,
    ));

    let pool = graph
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::ZfsPool)
        .expect("pool fixture exists");
    let snapshot = graph
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::ZfsSnapshot)
        .expect("snapshot fixture exists");
    assert_eq!(zfs_child_count(&graph, pool), 3);
    assert_eq!(zfs_child_count(&graph, snapshot), 1);

    let mut output = Vec::new();
    print_zfs(&mut output, &graph).expect("zfs table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("HEALTH"));
    assert!(output.contains("ORIGIN"));
    assert!(output.contains("CHILDREN"));
    assert!(output.contains("tank"));
    assert!(output.contains("ONLINE"));
    assert!(output.contains(
            "pool-ashift=12 pool-autotrim=on pool-autoexpand=off pool-cachefile=/etc/zfs/zpool.cache pool-failmode=wait"
        ));
    assert!(output.contains(
            "status=some devices need attention action=replace the faulted device scan=scrub repaired 0B errors=No known data errors pool-read-errors=3 pool-write-errors=4 pool-checksum-errors=5"
        ));
    assert!(
        output.contains("data vdev-state=ONLINE read-errors=0 write-errors=1 checksum-errors=2")
    );
    assert!(output.contains("tank/home"));
    assert!(output.contains(
        "compression=zstd quota=500G reservation=10G encryption=aes-256-gcm keystatus=available"
    ));
    assert!(output.contains("recordsize=1048576 dedup=off checksum=sha512 copies=2"));
    assert!(output.contains("sync=disabled primarycache=metadata secondarycache=all"));
    assert!(output.contains("atime=off relatime=on snapdir=visible acltype=posixacl xattr=sa"));
    assert!(output.contains("tank/home@daily"));
    assert!(output.contains("userrefs=2 compression=zstd"));
    assert!(output.contains("tank/vm/root"));
    assert!(output.contains("tank/vm/base@clean"));
    assert!(output.contains("volsize=80G"));
}

#[test]
fn volumes_table_includes_domain_metadata_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("lvm-lv:vg/root-snap", NodeKind::LvmSnapshot, "vg/root-snap")
            .with_property("lvm.origin", "root")
            .with_property("lvm.pool", "thinpool")
            .with_property("lvm.data-percent", "12.50")
            .with_property("lvm.active", "active")
            .with_property("lvm.layout", "snapshot")
            .with_property("lvm.health", "partial")
            .with_property("lvm.tags", "backup,snapshot")
            .with_property("lvm.cache-mode", "writeback")
            .with_property("lvm.cache-policy", "smq"),
    );
    graph.add_node(
        Node::new("md:/dev/md/root", NodeKind::MdRaid, "/dev/md/root")
            .with_property("md.level", "raid1")
            .with_property("md.state", "clean")
            .with_property("md.raid-devices", "2"),
    );
    graph.add_node(
        Node::new("iscsi-lun:iqn.example:0", NodeKind::Lun, "0")
            .with_property("iscsi.attached-disk", "sdb"),
    );
    graph.add_node(
        Node::new(
            "nfs-export:storage.example:/export/home",
            NodeKind::NfsExport,
            "storage.example:/export/home",
        )
        .with_property("nfs.server", "storage.example")
        .with_property("nfs.export", "/export/home"),
    );

    let mut output = Vec::new();
    print_volumes(&mut output, &graph).expect("volumes table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("DETAILS"));
    assert!(output.contains(
            "data=12.50 layout=snapshot origin=root pool=thinpool active=active health=partial tags=backup,snapshot cache-mode=writeback cache-policy=smq"
        ));
    assert!(output.contains("level=raid1 state=clean raid-devices=2"));
    assert!(output.contains("attached-disk=sdb"));
    assert!(output.contains("server=storage.example export=/export/home"));
}

#[test]
fn lvm_table_includes_volume_group_and_segment_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "lvm-pv:/dev/nvme0n1p3",
            NodeKind::LvmPhysicalVolume,
            "/dev/nvme0n1p3",
        )
        .with_path("/dev/nvme0n1p3")
        .with_size_bytes(536_870_912_000)
        .with_property("lvm.active", "active")
        .with_property("lvm.pv-format", "lvm2")
        .with_property("lvm.dev-size", "500.00g")
        .with_property("lvm.pe-start", "1.00m")
        .with_property("lvm.pv-missing", "missing")
        .with_property("lvm.pv-pe-count", "128000")
        .with_property("lvm.pv-pe-allocated", "102400")
        .with_property("lvm.pv-mda-free", "1020.00k")
        .with_property("lvm.pv-device-id", "wwn-0x1234")
        .with_property("lvm.tags", "ssd,system"),
    );
    graph.add_node(
        Node::new("lvm-vg:vg0", NodeKind::LvmVolumeGroup, "vg0")
            .with_size_bytes(1_099_511_627_776)
            .with_property("lvm.vg-format", "lvm2")
            .with_property("lvm.permissions", "writeable")
            .with_property("lvm.vg-autoactivation", "enabled")
            .with_property("lvm.allocation-policy", "normal")
            .with_property("lvm.vg-system-id", "host-a")
            .with_property("lvm.vg-lock-type", "none")
            .with_property("lvm.extent-size", "4.00m")
            .with_property("lvm.extent-count", "262144")
            .with_property("lvm.free-count", "5120")
            .with_property("lvm.pv-count", "2")
            .with_property("lvm.missing-pv-count", "1")
            .with_property("lvm.lv-count", "5")
            .with_property("lvm.snapshot-count", "1")
            .with_property("lvm.vg-seqno", "17")
            .with_property("lvm.vg-mda-free", "1020.00k")
            .with_property("lvm.vg-mda-copies", "unmanaged"),
    );
    graph.add_node(
        Node::new("lvm-thin-pool:vg0/pool", NodeKind::LvmThinPool, "vg0/pool")
            .with_size_bytes(858_993_459_200)
            .with_property("lvm.data-percent", "42.00")
            .with_property("lvm.metadata-percent", "7.50")
            .with_property("lvm.active", "active")
            .with_property("lvm.when-full", "queue")
            .with_property("lvm.metadata-size", "8.00g"),
    );
    graph.add_node(
        Node::new("lvm-lv:vg0/root", NodeKind::LvmLogicalVolume, "vg0/root")
            .with_size_bytes(214_748_364_800)
            .with_property("lvm.active", "active")
            .with_property("lvm.active-locally", "active locally")
            .with_property("lvm.active-exclusively", "active exclusively")
            .with_property("lvm.layout", "thin")
            .with_property("lvm.pool", "pool")
            .with_property("lvm.dm-path", "/dev/mapper/vg0-root")
            .with_property("lvm.read-ahead", "auto")
            .with_property("lvm.kernel-read-ahead", "256")
            .with_property("lvm.suspended", "not suspended")
            .with_property("lvm.live-table", "live")
            .with_property("lvm.modules", "thin")
            .with_property("lvm.host", "host-a")
            .with_property("lvm.health", "ok"),
    );
    graph.add_node(
        Node::new(
            "lvm-snapshot:vg0/root-snap",
            NodeKind::LvmSnapshot,
            "vg0/root-snap",
        )
        .with_property("lvm.origin", "root")
        .with_property("lvm.snap-percent", "12.50")
        .with_property("lvm.active", "active"),
    );
    graph.add_node(
        Node::new("lvm-cache:vg0/root", NodeKind::LvmCache, "vg0/root")
            .with_property("lvm.cache-mode", "writeback")
            .with_property("lvm.cache-policy", "smq")
            .with_property("lvm.raid-mismatch-count", "2")
            .with_property("lvm.raid-sync-action", "repair")
            .with_property("lvm.raid-write-behind", "256")
            .with_property("lvm.raid-min-recovery-rate", "1024")
            .with_property("lvm.raid-max-recovery-rate", "8192")
            .with_property("lvm.raid-integrity-mode", "journal")
            .with_property("lvm.raid-integrity-block-size", "4096")
            .with_property("lvm.raid-integrity-mismatches", "1")
            .with_property("lvm.writecache-block-size", "4096")
            .with_property("lvm.writecache-writeback-blocks", "16"),
    );
    graph.add_node(
        Node::new("lvm-segment:vg0/root:0", NodeKind::LvmSegment, "vg0/root:0")
            .with_property("lvm.segment-type", "thin")
            .with_property("lvm.segment-stripes", "2")
            .with_property("lvm.segment-data-stripes", "2")
            .with_property("lvm.reshape-length", "128.00m")
            .with_property("lvm.data-copies", "2")
            .with_property("lvm.stripe-size", "64.00k")
            .with_property("lvm.segment-start", "0")
            .with_property("lvm.segment-size", "200.00g")
            .with_property("lvm.segment-size-extents", "51200")
            .with_property("lvm.devices", "pool(0)")
            .with_property("lvm.segment-le-ranges", "0-51199")
            .with_property("lvm.segment-metadata-le-ranges", "pool_tmeta:0-31")
            .with_property("lvm.integrity-settings", "journal_sectors=2048")
            .with_property("lvm.vdo-block-map-cache-size", "128.00m")
            .with_property("lvm.vdo-use-sparse-index", "enabled")
            .with_property("lvm.vdo-bio-threads", "4")
            .with_property("lvm.vdo-max-discard", "4.00m"),
    );
    graph.add_edge(Edge::new(
        "lvm-pv:/dev/nvme0n1p3",
        "lvm-vg:vg0",
        Relationship::MemberOf,
    ));
    graph.add_edge(Edge::new(
        "lvm-thin-pool:vg0/pool",
        "lvm-lv:vg0/root",
        Relationship::Backs,
    ));

    let mut output = Vec::new();
    print_lvm(&mut output, &graph).expect("lvm table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("DATA%"));
    assert!(output.contains("META%"));
    assert!(output.contains("/dev/nvme0n1p3"));
    assert!(output.contains("active"));
    assert!(output.contains("tags=ssd,system"));
    assert!(output.contains("pv-format=lvm2 dev-size=500.00g"));
    assert!(output.contains("pe-start=1.00m pv-missing=missing pv-extents=128000"));
    assert!(output.contains("pv-extents-used=102400 pv-mda-free=1020.00k"));
    assert!(output.contains("pv-device-id=wwn-0x1234"));
    assert!(output.contains("vg-format=lvm2"));
    assert!(output.contains("permissions=writeable"));
    assert!(output.contains("vg-autoactivation=enabled allocation=normal"));
    assert!(output.contains("system-id=host-a lock-type=none"));
    assert!(output.contains("extent=4.00m extents=262144 free-extents=5120"));
    assert!(output.contains("pvs=2 missing-pvs=1 lvs=5 snapshots=1 seqno=17"));
    assert!(output.contains("vg-mda-free=1020.00k vg-mda-copies=unmanaged"));
    assert!(output.contains("42.00"));
    assert!(output.contains("7.50"));
    assert!(output.contains("when-full=queue metadata-size=8.00g"));
    assert!(output.contains("layout=thin pool=pool active=active active-local=active locally"));
    assert!(output.contains("active-exclusive=active exclusively"));
    assert!(output.contains("dm-path=/dev/mapper/vg0-root read-ahead=auto"));
    assert!(output.contains("kernel-read-ahead=256 suspended=not suspended"));
    assert!(output.contains("live-table=live modules=thin host=host-a"));
    assert!(output.contains("health=ok"));
    assert!(output.contains("snap=12.50 origin=root active=active"));
    assert!(output.contains("raid-mismatches=2 raid-sync=repair"));
    assert!(output.contains("raid-write-behind=256 raid-min-recovery=1024"));
    assert!(output.contains("raid-max-recovery=8192 raid-integrity=journal"));
    assert!(output.contains("raid-integrity-block=4096 raid-integrity-mismatches=1"));
    assert!(output.contains("cache-mode=writeback cache-policy=smq"));
    assert!(output.contains("writecache-writeback=16 writecache-block-size=4096"));
    assert!(output.contains("segment-type=thin stripes=2 data-stripes=2"));
    assert!(output.contains("reshape-length=128.00m data-copies=2"));
    assert!(output.contains("stripe-size=64.00k segment-start=0 segment-size=200.00g"));
    assert!(output.contains("segment-size-pe=51200 devices=pool(0) le-ranges=0-51199"));
    assert!(output.contains("metadata-le-ranges=pool_tmeta:0-31"));
    assert!(output.contains("integrity-settings=journal_sectors=2048"));
    assert!(output.contains("vdo-block-map-cache=128.00m vdo-sparse-index=enabled"));
    assert!(output.contains("vdo-bio-threads=4 vdo-max-discard=4.00m"));
}

#[test]
fn iscsi_table_includes_session_target_lun_and_disk_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "iscsi-session:12",
            NodeKind::IscsiSession,
            "iscsi-session:12",
        )
        .with_property("iscsi.portal", "10.0.0.10:3260,1")
        .with_property("iscsi.target", "iqn.2026-06.example:storage")
        .with_property("iscsi.portal-address", "10.0.0.10")
        .with_property("iscsi.portal-port", "3260")
        .with_property("iscsi.portal-tpgt", "1")
        .with_property("iscsi.persistent-portal", "10.0.0.11:3260,1")
        .with_property("iscsi.persistent-portal-address", "10.0.0.11")
        .with_property("iscsi.persistent-portal-port", "3260")
        .with_property("iscsi.persistent-portal-tpgt", "1")
        .with_property("iscsi.target-portal-group-tag", "1")
        .with_property("iscsi.connection-state", "LOGGED IN")
        .with_property("iscsi.connection-cid", "0")
        .with_property("iscsi.connection-detail-state", "LOGGED IN")
        .with_property("iscsi.connection-local-address", "10.0.0.20")
        .with_property("iscsi.connection-peer-address", "10.0.0.10"),
    );
    graph.add_node(
        Node::new(
            "iscsi-target:iqn.2026-06.example:storage",
            NodeKind::IscsiTarget,
            "iqn.2026-06.example:storage",
        )
        .with_property("iscsi.node-configured", "true")
        .with_property("iscsi.node-portal", "10.0.0.10:3260,1")
        .with_property("iscsi.node-portal-address", "10.0.0.10")
        .with_property("iscsi.node-portal-port", "3260")
        .with_property("iscsi.node-portal-tpgt", "1")
        .with_property("iscsi.node-startup", "automatic")
        .with_property("iscsi.node-iface-name", "default")
        .with_property("iscsi.node-auth-method", "CHAP"),
    );
    graph.add_node(
        Node::new(
            "iscsi-lun:iqn.2026-06.example:storage:0",
            NodeKind::Lun,
            "0",
        )
        .with_path("/dev/sdb")
        .with_size_bytes(1_073_741_824)
        .with_property("iscsi.attached-disk", "sdb")
        .with_property("scsi.address", "4:0:0:0")
        .with_property("scsi.transport", "iscsi")
        .with_property("scsi.generic-device", "/dev/sg2")
        .with_property("scsi.state", "running")
        .with_property("scsi.queue-depth", "64"),
    );
    graph.add_node(
        Node::new("block:/dev/sdb", NodeKind::PhysicalDisk, "/dev/sdb")
            .with_path("/dev/sdb")
            .with_property("iscsi.attached-disk", "sdb"),
    );
    graph.add_edge(Edge::new(
        "iscsi-session:12",
        "iscsi-target:iqn.2026-06.example:storage",
        Relationship::ImportedFrom,
    ));
    graph.add_edge(Edge::new(
        "iscsi-target:iqn.2026-06.example:storage",
        "iscsi-lun:iqn.2026-06.example:storage:0",
        Relationship::Contains,
    ));
    graph.add_edge(Edge::new(
        "iscsi-lun:iqn.2026-06.example:storage:0",
        "block:/dev/sdb",
        Relationship::Backs,
    ));

    let session = graph
        .nodes
        .iter()
        .find(|node| node.id.0 == "iscsi-session:12")
        .expect("session fixture exists");
    let target = graph
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::IscsiTarget)
        .expect("target fixture exists");
    assert_eq!(iscsi_lun_count(&graph, session), 1);
    assert_eq!(iscsi_lun_count(&graph, target), 1);

    let mut output = Vec::new();
    print_iscsi(&mut output, &graph).expect("iscsi table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("PORTAL"));
    assert!(output.contains("STATE"));
    assert!(output.contains("LUNS"));
    assert!(output.contains("PATH"));
    assert!(output.contains("iscsi-session:12"));
    assert!(output.contains("10.0.0.10:3260,1"));
    assert!(output.contains("LOGGED IN"));
    assert!(output
        .lines()
        .any(|line| { line.contains("lun") && line.contains("0") && line.contains("/dev/sdb") }));
    assert!(output.contains("target=iqn.2026-06.example:storage"));
    assert!(output.contains("portal-address=10.0.0.10 portal-port=3260 portal-tpgt=1"));
    assert!(
        output.contains("persistent-portal=10.0.0.11:3260,1 persistent-portal-address=10.0.0.11")
    );
    assert!(output.contains("persistent-portal-port=3260 persistent-portal-tpgt=1"));
    assert!(output.contains("tpgt=1 connection-state=LOGGED IN"));
    assert!(output.contains("cid=0 connection-detail-state=LOGGED IN"));
    assert!(output.contains("local-address=10.0.0.20 peer-address=10.0.0.10"));
    assert!(output.contains("iqn.2026-06.example:storage"));
    assert!(output.contains("configured=true node-portal=10.0.0.10:3260,1"));
    assert!(output.contains("node-portal-address=10.0.0.10 node-portal-port=3260"));
    assert!(output.contains("node-portal-tpgt=1 node-iface=default startup=automatic"));
    assert!(output.contains("auth-method=CHAP"));
    assert!(output.contains("1.0 GiB"));
    assert!(output.contains("attached-disk=sdb"));
    assert!(output.contains("scsi-address=4:0:0:0 scsi-generic=/dev/sg2"));
    assert!(output.contains("scsi-transport=iscsi scsi-state=running scsi-queue-depth=64"));
}

#[test]
fn luns_table_includes_scsi_path_and_json_neighbors() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "iscsi-target:iqn.2026-06.example:storage",
            NodeKind::IscsiTarget,
            "iqn.2026-06.example:storage",
        )
        .with_property("iscsi.node-portal", "10.0.0.10:3260,1"),
    );
    graph.add_node(
        Node::new(
            "iscsi-lun:iqn.2026-06.example:storage:0",
            NodeKind::Lun,
            "0",
        )
        .with_path("/dev/sdb")
        .with_size_bytes(1_073_741_824)
        .with_property("iscsi.attached-disk", "sdb")
        .with_property("iscsi.attached-disk-state", "running")
        .with_property("scsi.address", "4:0:0:0")
        .with_property("scsi.host", "4")
        .with_property("scsi.channel", "0")
        .with_property("scsi.target", "0")
        .with_property("scsi.lun", "0")
        .with_property("scsi.transport", "iscsi")
        .with_property("scsi.generic-device", "/dev/sg2")
        .with_property("scsi.state", "running")
        .with_property("scsi.queue-depth", "64"),
    );
    graph.add_node(Node::new(
        "block:/dev/sdb",
        NodeKind::PhysicalDisk,
        "/dev/sdb",
    ));
    graph.add_edge(Edge::new(
        "iscsi-target:iqn.2026-06.example:storage",
        "iscsi-lun:iqn.2026-06.example:storage:0",
        Relationship::Contains,
    ));
    graph.add_edge(Edge::new(
        "iscsi-lun:iqn.2026-06.example:storage:0",
        "block:/dev/sdb",
        Relationship::Backs,
    ));

    let mut output = Vec::new();
    print_luns(&mut output, &graph).expect("LUN table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("TRANSPORT"));
    assert!(output.contains("GENERIC"));
    assert!(output.contains("1.0 GiB"));
    assert!(output.contains("/dev/sdb"));
    assert!(output.contains("iscsi"));
    assert!(output.contains("/dev/sg2"));
    assert!(output.contains("scsi-address=4:0:0:0 scsi-host=4 scsi-channel=0"));
    assert!(output.contains("scsi-target=0 scsi-lun=0"));
    assert!(output.contains("attached-disk=sdb attached-disk-state=running"));

    let mut json = Vec::new();
    print_filtered_json(&mut json, &graph, is_lun_node).expect("LUN json renders");
    let json = String::from_utf8(json).expect("json is utf8");
    assert!(json.contains("iscsi-lun:iqn.2026-06.example:storage:0"));
    assert!(json.contains("iscsi-target:iqn.2026-06.example:storage"));
    assert!(json.contains("block:/dev/sdb"));
}

#[test]
fn nfs_table_includes_exports_mounts_and_transport_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "nfs-export:storage.example:/export/home",
            NodeKind::NfsExport,
            "storage.example:/export/home",
        )
        .with_property("nfs.server", "storage.example")
        .with_property("nfs.export", "/export/home"),
    );
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
        .with_property("nfs.export-option-no-subtree-check", "true")
        .with_property("nfs.export-option-sec", "sys")
        .with_property("nfs.export-option-root-squash", "true"),
    );
    graph.add_node(
        Node::new("mount:/home", NodeKind::NfsMount, "/home")
            .with_size_bytes(1_099_511_627_776)
            .with_usage(Usage {
                used_bytes: Some(274_877_906_944),
                free_bytes: Some(824_633_720_832),
                allocated_bytes: None,
            })
            .with_property("nfs.source", "storage.example:/export/home")
            .with_property("nfs.server", "storage.example")
            .with_property("nfs.export", "/export/home")
            .with_property("nfs.vers", "4.2")
            .with_property("nfs.proto", "tcp")
            .with_property("nfs.sec", "sys")
            .with_property("nfs.clientaddr", "10.0.0.20")
            .with_property("nfs.addr", "10.0.0.10")
            .with_property("nfs.port", "2049")
            .with_property("nfs.mountaddr", "10.0.0.10")
            .with_property("nfs.mountvers", "3")
            .with_property("nfs.mountproto", "tcp")
            .with_property("nfs.rsize", "1048576")
            .with_property("nfs.wsize", "1048576")
            .with_property("nfs.timeo", "600")
            .with_property("nfs.retrans", "2")
            .with_property("nfs.local-lock", "none")
            .with_property("nfs.lookupcache", "positive")
            .with_property("nfs.fsc", "true")
            .with_property("nfs.caps", "0x3fffdf")
            .with_property("nfs.wtmult", "512")
            .with_property("nfs.dtsize", "32768")
            .with_property("nfs.bsize", "0")
            .with_property("nfs.flavor", "1")
            .with_property("nfs.pseudoflavor", "1")
            .with_property("nfs.age", "123"),
    );
    graph.add_edge(Edge::new(
        "nfs-export:storage.example:/export/home",
        "mount:/home",
        Relationship::MountedAt,
    ));

    let export = graph
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::NfsExport)
        .expect("export fixture exists");
    let mount = graph
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::NfsMount)
        .expect("mount fixture exists");
    assert_eq!(nfs_mount_count(&graph, export), 1);
    assert_eq!(nfs_mount_count(&graph, mount), 0);

    let mut output = Vec::new();
    print_nfs(&mut output, &graph).expect("nfs table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("SOURCE"));
    assert!(output.contains("SERVER"));
    assert!(output.contains("EXPORT"));
    assert!(output.contains("MOUNTS"));
    assert!(output.contains("storage.example:/export/home"));
    assert!(output.contains("storage.example"));
    assert!(output.contains("/export/home"));
    assert!(output.contains("/home"));
    assert!(output.contains("source=storage.example:/export/home"));
    assert!(output.contains("vers=4.2 proto=tcp sec=sys"));
    assert!(output.contains("clientaddr=10.0.0.20 addr=10.0.0.10 port=2049"));
    assert!(output.contains("mountaddr=10.0.0.10 mountvers=3 mountproto=tcp"));
    assert!(output.contains("rsize=1048576 wsize=1048576 timeo=600 retrans=2"));
    assert!(output.contains("local-lock=none lookupcache=positive fsc=true age=123"));
    assert!(output.contains("caps=0x3fffdf wtmult=512 dtsize=32768 bsize=0"));
    assert!(output.contains("flavor=1 pseudoflavor=1"));
    assert!(output.contains("/srv/share"));
    assert!(output.contains("export-client=192.0.2.0/24 exportfs=true"));
    assert!(output.contains("export-rw=true export-sync=true"));
    assert!(output.contains("export-no-subtree-check=true export-sec=sys"));
    assert!(output.contains("export-root-squash=true"));
}

#[test]
fn network_storage_table_includes_domain_metadata_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("iscsi-session:1", NodeKind::IscsiSession, "iscsi-session:1")
            .with_property("iscsi.portal", "10.0.0.10:3260,1")
            .with_property("iscsi.portal-address", "10.0.0.10")
            .with_property("iscsi.portal-port", "3260")
            .with_property("iscsi.portal-tpgt", "1")
            .with_property("iscsi.persistent-portal", "10.0.0.11:3260,1")
            .with_property("iscsi.persistent-portal-address", "10.0.0.11")
            .with_property("iscsi.persistent-portal-port", "3260")
            .with_property("iscsi.persistent-portal-tpgt", "1")
            .with_property("iscsi.connection-state", "LOGGED IN")
            .with_property("iscsi.session-state", "LOGGED_IN")
            .with_property("iscsi.internal-session-state", "NO CHANGE")
            .with_property("iscsi.iface-name", "default")
            .with_property("iscsi.iface-transport", "tcp")
            .with_property("iscsi.iface-initiator-name", "iqn.2026-06.client:node1")
            .with_property("iscsi.iface-ip-address", "10.0.0.20")
            .with_property("iscsi.iface-netdev", "eno1")
            .with_property("iscsi.host-number", "4")
            .with_property("iscsi.host-state", "running")
            .with_property("iscsi.headerdigest", "None")
            .with_property("iscsi.datadigest", "None")
            .with_property("iscsi.maxrecvdatasegmentlength", "262144")
            .with_property("iscsi.maxburstlength", "262144"),
    );
    graph.add_node(Node::new(
        "iscsi-target:iqn.2026-06.example:storage",
        NodeKind::IscsiTarget,
        "iqn.2026-06.example:storage",
    ));
    graph.add_node(
        Node::new(
            "iscsi-lun:iqn.2026-06.example:storage:0",
            NodeKind::Lun,
            "0",
        )
        .with_size_bytes(1_073_741_824)
        .with_property("iscsi.host-number", "4")
        .with_property("iscsi.scsi-channel", "00")
        .with_property("iscsi.scsi-id", "0")
        .with_property("iscsi.attached-disk", "sdb")
        .with_property("iscsi.attached-disk-state", "running"),
    );
    graph.add_node(
        Node::new(
            "nfs-export:storage.example:/export/home",
            NodeKind::NfsExport,
            "storage.example:/export/home",
        )
        .with_property("nfs.server", "storage.example")
        .with_property("nfs.export", "/export/home"),
    );
    graph.add_node(
        Node::new("mount:/home", NodeKind::NfsMount, "/home")
            .with_property("nfs.source", "storage.example:/export/home")
            .with_property("nfs.server", "storage.example")
            .with_property("nfs.export", "/export/home")
            .with_property("nfs.vers", "4.2")
            .with_property("nfs.proto", "tcp")
            .with_property("nfs.sec", "sys")
            .with_property("nfs.clientaddr", "10.0.0.20")
            .with_property("nfs.addr", "10.0.0.10")
            .with_property("nfs.port", "2049")
            .with_property("nfs.mountaddr", "10.0.0.10")
            .with_property("nfs.mountvers", "3")
            .with_property("nfs.mountproto", "tcp")
            .with_property("nfs.rsize", "1048576")
            .with_property("nfs.wsize", "1048576")
            .with_property("nfs.timeo", "600")
            .with_property("nfs.retrans", "2")
            .with_property("nfs.local-lock", "none")
            .with_property("nfs.lookupcache", "positive")
            .with_property("nfs.fsc", "true")
            .with_property("nfs.caps", "0x3fffdf")
            .with_property("nfs.wtmult", "512")
            .with_property("nfs.dtsize", "32768")
            .with_property("nfs.bsize", "0")
            .with_property("nfs.flavor", "1")
            .with_property("nfs.pseudoflavor", "1")
            .with_property("nfs.age", "123"),
    );

    let mut output = Vec::new();
    print_network_storage(&mut output, &graph).expect("network storage table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("DETAILS"));
    assert!(output.contains("portal=10.0.0.10:3260,1"));
    assert!(output.contains("portal-address=10.0.0.10 portal-port=3260 portal-tpgt=1"));
    assert!(output.contains("persistent-portal=10.0.0.11:3260,1"));
    assert!(output.contains(
        "persistent-portal-address=10.0.0.11 persistent-portal-port=3260 persistent-portal-tpgt=1"
    ));
    assert!(output.contains("connection-state=LOGGED IN"));
    assert!(output.contains("session-state=LOGGED_IN"));
    assert!(output.contains("internal-session-state=NO CHANGE"));
    assert!(output.contains("iface=default transport=tcp"));
    assert!(output.contains("initiator=iqn.2026-06.client:node1"));
    assert!(output.contains("iface-ip=10.0.0.20 netdev=eno1"));
    assert!(output.contains("host=4 host-state=running"));
    assert!(output.contains("header-digest=None data-digest=None"));
    assert!(output.contains("max-recv-data-segment=262144"));
    assert!(output.contains("max-burst=262144"));
    assert!(output.contains("scsi-channel=00 scsi-id=0"));
    assert!(output.contains("attached-disk=sdb attached-disk-state=running"));
    assert!(output.contains("server=storage.example export=/export/home"));
    assert!(output.contains(
        "source=storage.example:/export/home server=storage.example export=/export/home vers=4.2"
    ));
    assert!(output.contains("proto=tcp sec=sys clientaddr=10.0.0.20 addr=10.0.0.10"));
    assert!(output.contains("mountaddr=10.0.0.10 mountvers=3 mountproto=tcp"));
    assert!(output.contains("rsize=1048576 wsize=1048576 timeo=600 retrans=2"));
    assert!(output.contains("local-lock=none lookupcache=positive fsc=true age=123"));
    assert!(output.contains("caps=0x3fffdf wtmult=512 dtsize=32768 bsize=0"));
    assert!(output.contains("flavor=1 pseudoflavor=1"));
}

#[test]
fn snapshots_table_includes_domain_metadata_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(Node::new(
        "dataset:tank/home",
        NodeKind::ZfsDataset,
        "tank/home",
    ));
    graph.add_node(
        Node::new(
            "zfs-snapshot:tank/home@daily",
            NodeKind::ZfsSnapshot,
            "tank/home@daily",
        )
        .with_size_bytes(1_073_741_824)
        .with_property("zfs.userrefs", "2")
        .with_property("zfs.holds", "disk-nix-retain")
        .with_property("zfs.compression", "zstd")
        .with_property("zfs.encryption", "aes-256-gcm")
        .with_property("zfs.keystatus", "available")
        .with_property("zfs.checksum", "sha512")
        .with_property("zfs.copies", "2"),
    );
    graph.add_edge(Edge::new(
        "zfs-snapshot:tank/home@daily",
        "dataset:tank/home",
        Relationship::SnapshotOf,
    ));
    graph.add_node(
        Node::new("lvm-lv:vg/root-snap", NodeKind::LvmSnapshot, "vg/root-snap")
            .with_property("lvm.origin", "root")
            .with_property("lvm.pool", "thinpool")
            .with_property("lvm.data-percent", "12.50"),
    );
    graph.add_node(
        Node::new(
            "btrfs-subvolume:fs:@/.snapshots/1/snapshot",
            NodeKind::BtrfsSnapshot,
            "@/.snapshots/1/snapshot",
        )
        .with_property("btrfs.id", "257")
        .with_property("btrfs.generation", "11")
        .with_property("btrfs.created-generation", "8")
        .with_property("btrfs.parent-id", "256")
        .with_property("btrfs.top-level", "5")
        .with_property("btrfs.parent-uuid", "subvol-root")
        .with_property("btrfs.received-uuid", "received-snap"),
    );

    let mut output = Vec::new();
    print_snapshots(&mut output, &graph).expect("snapshots table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("DETAILS"));
    assert!(output.contains("tank/home"));
    assert!(
            output
                .contains("userrefs=2 holds=disk-nix-retain compression=zstd encryption=aes-256-gcm keystatus=available")
        );
    assert!(output.contains("checksum=sha512 copies=2"));
    assert!(output.contains("data=12.50 origin=root pool=thinpool"));
    assert!(output.contains("subvol-id=257 generation=11 created-generation=8 parent-id=256"));
    assert!(output.contains("top-level=5 parent-uuid=subvol-root received-uuid=received-snap"));
}

#[test]
fn pools_table_includes_domain_metadata_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("zpool:tank", NodeKind::ZfsPool, "tank")
            .with_property("zfs.health", "ONLINE")
            .with_property("zfs.state", "ONLINE"),
    );
    graph.add_node(
        Node::new(
            "zfs-vdev:tank:cache0",
            NodeKind::ZfsVdev,
            "/dev/disk/by-id/cache0",
        )
        .with_property("zfs.vdev-role", "cache")
        .with_property("zfs.vdev-state", "ONLINE")
        .with_property("zfs.read-errors", "0")
        .with_property("zfs.write-errors", "1")
        .with_property("zfs.checksum-errors", "2"),
    );
    graph.add_node(
        Node::new("lvm-vg:vg0", NodeKind::LvmVolumeGroup, "vg0")
            .with_property("lvm.extent-size", "4.00m")
            .with_property("lvm.pv-count", "2")
            .with_property("lvm.lv-count", "8"),
    );
    graph.add_node(
        Node::new("btrfs-qgroup:0/257", NodeKind::BtrfsQgroup, "0/257")
            .with_property("btrfs.qgroup-id", "0/257")
            .with_property("btrfs.qgroup-parents", "0/5")
            .with_property("btrfs.qgroup-children", "1/257")
            .with_property("btrfs.max-referenced", "25GiB"),
    );
    graph.add_node(
        Node::new("md:/dev/md/root", NodeKind::MdRaid, "/dev/md/root")
            .with_property("md.version", "1.2")
            .with_property("md.uuid", "aaaa:bbbb:cccc:dddd")
            .with_property("md.level", "raid1")
            .with_property("md.state", "clean")
            .with_property("md.raid-devices", "2")
            .with_property("md.total-devices", "2")
            .with_property("md.name", "host:root")
            .with_property("md.events", "17"),
    );

    let mut output = Vec::new();
    print_pools(&mut output, &graph).expect("pools table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("DETAILS"));
    assert!(output.contains("health=ONLINE state=ONLINE"));
    assert!(output.contains(
        "vdev-role=cache vdev-state=ONLINE read-errors=0 write-errors=1 checksum-errors=2"
    ));
    assert!(output.contains("extent=4.00m pvs=2 lvs=8"));
    assert!(output.contains("qgroup=0/257 qgroup-parents=0/5 qgroup-children=1/257"));
    assert!(output.contains("max-rfer=25GiB"));
    assert!(output.contains(
            "md-version=1.2 level=raid1 state=clean raid-devices=2 total-devices=2 md-name=host:root events=17"
        ));
}

#[test]
fn encryption_table_includes_luks_header_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "block:/dev/mapper/cryptroot",
            NodeKind::LuksContainer,
            "cryptroot",
        )
        .with_path("/dev/mapper/cryptroot")
        .with_property("cryptsetup.active", "true")
        .with_property("cryptsetup.in-use", "true")
        .with_property("cryptsetup.cipher", "aes-xts-plain64")
        .with_property("cryptsetup.luks-version", "2")
        .with_property("cryptsetup.luks-epoch", "7")
        .with_property("cryptsetup.luks-metadata-area", "16384 [bytes]")
        .with_property("cryptsetup.luks-keyslots-area", "16744448 [bytes]")
        .with_property("cryptsetup.luks-keyslot-count", "2")
        .with_property("cryptsetup.luks-token-count", "1")
        .with_property("cryptsetup.luks-keyslots", "0,1")
        .with_property("cryptsetup.luks-tokens", "0")
        .with_property("cryptsetup.luks-keyslot-0-type", "luks2")
        .with_property("cryptsetup.luks-keyslot-0-priority", "normal")
        .with_property("cryptsetup.luks-keyslot-0-cipher", "aes-xts-plain64")
        .with_property("cryptsetup.luks-keyslot-0-cipher-key", "512 bits")
        .with_property("cryptsetup.luks-keyslot-0-pbkdf", "argon2id")
        .with_property("cryptsetup.luks-keyslot-0-time-cost", "4")
        .with_property("cryptsetup.luks-keyslot-0-memory", "1048576")
        .with_property("cryptsetup.luks-keyslot-0-threads", "4")
        .with_property("cryptsetup.luks-keyslot-0-salt", "00 11 22 33")
        .with_property("cryptsetup.luks-keyslot-0-af-stripes", "4000")
        .with_property("cryptsetup.luks-keyslot-0-area-offset", "32768 [bytes]")
        .with_property("cryptsetup.luks-keyslot-0-area-length", "258048 [bytes]")
        .with_property("cryptsetup.luks-keyslot-0-digest-id", "0")
        .with_property("cryptsetup.luks-keyslot-1-type", "luks2")
        .with_property("cryptsetup.luks-keyslot-1-priority", "ignored")
        .with_property("cryptsetup.luks-token-0-type", "systemd-tpm2")
        .with_property("cryptsetup.luks-token-0-keyslot", "0")
        .with_property("cryptsetup.luks-token-0-keyslots", "0")
        .with_property("cryptsetup.luks-token-0-tpm2-pcrs", "0+7")
        .with_property("cryptsetup.luks-token-0-tpm2-hash", "sha256")
        .with_property("cryptsetup.luks-digest-count", "1")
        .with_property("cryptsetup.luks-digests", "0")
        .with_property("cryptsetup.luks-digest-0-type", "pbkdf2")
        .with_property("cryptsetup.luks-digest-0-hash", "sha256")
        .with_property("cryptsetup.luks-digest-0-iterations", "1000")
        .with_property("cryptsetup.luks-digest-0-salt", "aa bb cc dd")
        .with_property("cryptsetup.luks-digest-0-digest", "ee ff 00 11"),
    );

    let mut output = Vec::new();
    print_encryption(&mut output, &graph).expect("encryption table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("CIPHER"));
    assert!(output.contains("KEYSLOTS"));
    assert!(output.contains("TOKENS"));
    assert!(output.contains("cryptroot"));
    assert!(output.contains("aes-xts-plain64"));
    assert!(output.contains(" 2         "));
    assert!(output.contains(" 1         "));
    assert!(output.contains("active=true in-use=true cipher=aes-xts-plain64"));
    assert!(output.contains("luks=2 epoch=7 metadata-area=16384 [bytes]"));
    assert!(output.contains("keyslot-ids=0,1 token-ids=0"));
    assert!(output
        .contains("keyslot-0=luks2 keyslot-0-priority=normal keyslot-0-cipher=aes-xts-plain64"));
    assert!(output.contains(
            "keyslot-0-cipher-key=512 bits keyslot-0-pbkdf=argon2id keyslot-0-time=4 keyslot-0-memory=1048576 keyslot-0-threads=4"
        ));
    assert!(output.contains("keyslot-0-salt=00 11 22 33 keyslot-0-af-stripes=4000"));
    assert!(output.contains(
            "keyslot-0-area-offset=32768 [bytes] keyslot-0-area-length=258048 [bytes] keyslot-0-digest=0"
        ));
    assert!(output.contains(
        "keyslot-1=luks2 keyslot-1-priority=ignored token-0=systemd-tpm2 token-0-keyslot=0"
    ));
    assert!(output.contains("token-0-keyslots=0 token-0-tpm2-pcrs=0+7 token-0-tpm2-hash=sha256"));
    assert!(output.contains("digests=1 digest-ids=0 digest-0=pbkdf2"));
    assert!(output.contains("digest-0-hash=sha256 digest-0-iterations=1000"));
    assert!(output.contains("digest-0-salt=aa bb cc dd digest-0-digest=ee ff 00 11"));
}

#[test]
fn cache_table_includes_cache_layer_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("block:/dev/bcache0", NodeKind::CacheDevice, "bcache0")
            .with_path("/dev/bcache0")
            .with_property("bcache.role", "backing")
            .with_property("bcache.kind", "cache-set")
            .with_property("bcache.backing-device", "/dev/sdb1")
            .with_property("bcache.set-uuid", "cache-set-uuid")
            .with_property("bcache.set-average-key-size", "16.0k")
            .with_property("bcache.set-root-usage-percent", "3")
            .with_property("bcache.state", "clean")
            .with_property("bcache.running", "1")
            .with_property("bcache.cache-available-percent", "78")
            .with_property("bcache.cache-mode", "writeback")
            .with_property("bcache.cache-replacement-policy", "lru")
            .with_property("bcache.dirty-data", "64.0M")
            .with_property("bcache.io-errors", "0")
            .with_property("bcache.metadata-written", "128.0M")
            .with_property("bcache.writeback-delay", "30")
            .with_property("bcache.writeback-running", "1"),
    );
    graph.add_node(
        Node::new("lvm-lv:vg/root", NodeKind::LvmLogicalVolume, "vg/root")
            .with_property("lvm.cache-mode", "writethrough")
            .with_property("lvm.cache-policy", "smq")
            .with_property("lvm.cache-total-blocks", "4096")
            .with_property("lvm.cache-used-blocks", "1024")
            .with_property("lvm.cache-dirty-blocks", "64")
            .with_property("lvm.cache-read-hits", "1000")
            .with_property("lvm.cache-read-misses", "25")
            .with_property("lvm.cache-write-hits", "900")
            .with_property("lvm.cache-write-misses", "30")
            .with_property("lvm.cache-promotions", "128")
            .with_property("lvm.cache-demotions", "32")
            .with_property("lvm.kernel-cache-settings", "migration_threshold=2048")
            .with_property("lvm.kernel-metadata-format", "2")
            .with_property("lvm.writecache-total-blocks", "1024")
            .with_property("lvm.writecache-free-blocks", "512")
            .with_property("lvm.writecache-writeback-blocks", "16")
            .with_property("lvm.writecache-error", "0"),
    );
    graph.add_node(
        Node::new(
            "zfs-vdev:tank:cache0",
            NodeKind::ZfsVdev,
            "/dev/disk/by-id/cache0",
        )
        .with_property("zfs.vdev-role", "cache")
        .with_property("zfs.vdev-state", "ONLINE"),
    );

    let mut output = Vec::new();
    print_cache(&mut output, &graph).expect("cache table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("MODE"));
    assert!(output.contains("POLICY"));
    assert!(output.contains("DIRTY"));
    assert!(output.contains("bcache0"));
    assert!(output.contains("writeback"));
    assert!(output.contains("lru"));
    assert!(output.contains("backing-device=/dev/sdb1"));
    assert!(output.contains("set-average-key-size=16.0k set-root-usage-percent=3"));
    assert!(output.contains("dirty=64.0M"));
    assert!(output.contains("running=1 available-percent=78"));
    assert!(output.contains("io-errors=0 metadata-written=128.0M"));
    assert!(output.contains("writeback-delay=30"));
    assert!(output.contains("writeback-running=1"));
    assert!(output.contains("vg/root"));
    assert!(output.contains("writethrough"));
    assert!(output.contains("cache-policy=smq"));
    assert!(output.contains("cache-total=4096"));
    assert!(output.contains("cache-used=1024"));
    assert!(output.contains("cache-dirty=64"));
    assert!(output.contains("cache-read-hits=1000"));
    assert!(output.contains("cache-read-misses=25"));
    assert!(output.contains("cache-write-hits=900"));
    assert!(output.contains("cache-write-misses=30"));
    assert!(output.contains("cache-promotions=128"));
    assert!(output.contains("cache-demotions=32"));
    assert!(output.contains("kernel-cache-settings=migration_threshold=2048"));
    assert!(output.contains("kernel-metadata-format=2"));
    assert!(output.contains("writecache-total=1024"));
    assert!(output.contains("writecache-free=512"));
    assert!(output.contains("writecache-writeback=16"));
    assert!(output.contains("writecache-error=0"));
    assert!(output.contains("/dev/disk/by-id/cache0"));
    assert!(output.contains("vdev-role=cache vdev-state=ONLINE"));
}

#[test]
fn vdo_table_includes_vdo_reduction_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("vdo:archive", NodeKind::VdoVolume, "archive")
            .with_size_bytes(1_099_511_627_776)
            .with_usage(Usage {
                used_bytes: Some(268_435_456_000),
                free_bytes: Some(805_306_368_000),
                allocated_bytes: Some(1_073_741_824_000),
            })
            .with_property("vdo.storage-device", "/dev/sdb")
            .with_property("vdo.logical-size", "1T")
            .with_property("vdo.physical-size", "250G")
            .with_property("vdo.stats-size", "268435456")
            .with_property("vdo.stats-used", "134217728")
            .with_property("vdo.stats-available", "134217728")
            .with_property("vdo.use-percent", "50%")
            .with_property("vdo.space-saving-percent", "75%")
            .with_property("vdo.operating-mode", "normal")
            .with_property("vdo.recovery-percentage", "100%")
            .with_property("vdo.write-policy", "sync")
            .with_property("vdo.configured-write-policy", "auto")
            .with_property("vdo.index-memory-setting", "0.25")
            .with_property("vdo.block-map-cache-size", "128M")
            .with_property("vdo.compression", "enabled")
            .with_property("vdo.deduplication", "enabled")
            .with_property("vdo.version", "47")
            .with_property("vdo.release-version", "133524")
            .with_property("vdo.data-blocks-used", "65536")
            .with_property("vdo.data-blocks-used-bytes", "268435456")
            .with_property("vdo.overhead-blocks-used", "4096")
            .with_property("vdo.overhead-blocks-used-bytes", "16777216")
            .with_property("vdo.logical-blocks-used", "262144")
            .with_property("vdo.logical-blocks-used-bytes", "1073741824"),
    );
    graph.add_node(
        Node::new(
            "lvm-seg:vg0/archive:0",
            NodeKind::LvmSegment,
            "vg0/archive:0",
        )
        .with_size_bytes(10 * 1024 * 1024 * 1024)
        .with_usage(Usage {
            used_bytes: Some(8 * 1024 * 1024 * 1024),
            free_bytes: None,
            allocated_bytes: None,
        })
        .with_property("lvm.segment-type", "vdo")
        .with_property("lvm.vdo-operating-mode", "normal")
        .with_property("lvm.vdo-compression", "enabled")
        .with_property("lvm.vdo-compression-state", "online")
        .with_property("lvm.vdo-deduplication", "disabled")
        .with_property("lvm.vdo-index-state", "online")
        .with_property("lvm.vdo-used-size", "8.00g")
        .with_property("lvm.vdo-saving-percent", "42.00")
        .with_property("lvm.vdo-write-policy", "auto"),
    );

    let mut output = Vec::new();
    print_vdo(&mut output, &graph).expect("vdo table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("LOGICAL"));
    assert!(output.contains("PHYSICAL"));
    assert!(output.contains("USED"));
    assert!(output.contains("FREE"));
    assert!(output.contains("USE%"));
    assert!(output.contains("WRITE"));
    assert!(output.contains("archive"));
    assert!(output.contains("          1T"));
    assert!(output.contains("        250G"));
    assert!(output.contains("   250.0 GiB"));
    assert!(output.contains("   750.0 GiB"));
    assert!(output.contains("  24.4%"));
    assert!(output.contains("normal"));
    assert!(output.contains("sync"));
    assert!(output.contains("backing=/dev/sdb logical=1T physical=250G"));
    assert!(output.contains("stats-size=268435456 stats-used=134217728"));
    assert!(output.contains("vdo-use=50% saving=75%"));
    assert!(output.contains("recovery=100% write-policy=sync configured-write-policy=auto"));
    assert!(output.contains("index-memory=0.25 block-map-cache=128M"));
    assert!(output.contains("compression=enabled deduplication=enabled"));
    assert!(output.contains("vdo-version=47 vdo-release=133524"));
    assert!(output.contains("data-blocks=65536 data-bytes=268435456"));
    assert!(output.contains("overhead-blocks=4096 overhead-bytes=16777216"));
    assert!(output.contains("logical-blocks=262144 logical-bytes=1073741824"));
    assert!(output.contains("vg0/archive:0"));
    assert!(output.contains("    10.0 GiB      8.0 GiB      8.0 GiB"));
    assert!(output.contains("vdo-mode=normal"));
    assert!(output.contains("vdo-compression-state=online"));
    assert!(output.contains("vdo-index-state=online"));
    assert!(output.contains("vdo-used=8.00g"));
    assert!(output.contains("vdo-saving=42.00"));
    assert!(output.contains("vdo-write-policy=auto"));
}

#[test]
fn multipath_table_includes_map_and_path_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("multipath:mpatha", NodeKind::MultipathDevice, "mpatha")
            .with_path("/dev/mapper/mpatha")
            .with_property("multipath.dm", "dm-2")
            .with_property("multipath.wwid", "3600508b400105e210000900000490000")
            .with_property("multipath.vendor-product", "IBM,2145")
            .with_property("multipath.size", "100G")
            .with_property("multipath.features", "1 queue_if_no_path")
            .with_property("multipath.hwhandler", "1 alua")
            .with_property("multipath.write-protect", "rw"),
    );
    graph.add_node(
        Node::new("block:/dev/sdb", NodeKind::PhysicalDisk, "/dev/sdb")
            .with_path("/dev/sdb")
            .with_property("multipath.host-path", "2:0:0:1")
            .with_property("multipath.scsi-host", "2")
            .with_property("multipath.scsi-channel", "0")
            .with_property("multipath.scsi-id", "0")
            .with_property("multipath.scsi-lun", "1")
            .with_property("major-minor", "8:16")
            .with_property("multipath.group-policy", "service-time 0")
            .with_property("multipath.group-prio", "50")
            .with_property("multipath.group-status", "active")
            .with_property("multipath.dm-state", "active")
            .with_property("multipath.checker-state", "ready")
            .with_property("multipath.online-state", "running")
            .with_property("multipath.path-flags", "ghost")
            .with_property("multipath.path-state", "active ready running ghost"),
    );
    graph.add_node(
        Node::new("block:/dev/sdc", NodeKind::PhysicalDisk, "/dev/sdc")
            .with_path("/dev/sdc")
            .with_property("multipath.host-path", "3:0:0:1")
            .with_property("multipath.scsi-host", "3")
            .with_property("multipath.scsi-channel", "0")
            .with_property("multipath.scsi-id", "0")
            .with_property("multipath.scsi-lun", "1")
            .with_property("major-minor", "8:32")
            .with_property("multipath.group-policy", "service-time 0")
            .with_property("multipath.group-prio", "10")
            .with_property("multipath.group-status", "enabled")
            .with_property("multipath.dm-state", "active")
            .with_property("multipath.checker-state", "ready")
            .with_property("multipath.online-state", "running")
            .with_property("multipath.path-flags", "faulty shaky")
            .with_property("multipath.path-state", "active ready running faulty shaky"),
    );
    graph.add_edge(Edge::new(
        "block:/dev/sdb",
        "multipath:mpatha",
        Relationship::Backs,
    ));
    graph.add_edge(Edge::new(
        "block:/dev/sdc",
        "multipath:mpatha",
        Relationship::Backs,
    ));

    let mut output = Vec::new();
    print_multipath(&mut output, &graph).expect("multipath table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("WWID"));
    assert!(output.contains("PATHS"));
    assert!(output.contains("GROUP"));
    assert!(output.contains("PATH-STATE"));
    assert!(output.contains("mpatha"));
    assert!(output.contains("3600508b400105e210000900000490000"));
    assert!(output.contains("dm=dm-2 wwid=3600508b400105e210000900000490000"));
    assert!(output.contains("vendor=IBM,2145 size=100G"));
    assert!(output.contains("features=1 queue_if_no_path handler=1 alua wp=rw"));
    assert!(output.contains("/dev/sdb"));
    assert!(output.contains("host-path=2:0:0:1 scsi-host=2"));
    assert!(output.contains("scsi-host=2 scsi-channel=0 scsi-id=0 scsi-lun=1"));
    assert!(output.contains("scsi-lun=1 major-minor=8:16"));
    assert!(output.contains("group-policy=service-time 0 group-prio=50 group-status=active"));
    assert!(output
        .contains("dm-state=active checker-state=ready online-state=running path-flags=ghost"));
    assert!(output.contains("path-state=active ready running ghost"));
    assert!(output.contains("path-flags=faulty shaky"));
    assert!(output.contains("path-state=active ready running faulty shaky"));
    assert!(output.contains("/dev/sdc"));
    assert!(output.contains("host-path=3:0:0:1 scsi-host=3"));
    assert!(output.contains("scsi-host=3 scsi-channel=0 scsi-id=0 scsi-lun=1"));
    assert!(output.contains("scsi-lun=1 major-minor=8:32"));
    assert!(output.contains("group-policy=service-time 0 group-prio=10 group-status=enabled"));
}

#[test]
fn nvme_table_includes_namespace_identity_and_geometry() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("nvme-controller:nvme0", NodeKind::NvmeController, "nvme0")
            .with_path("/dev/nvme0")
            .with_identity(Identity {
                serial: Some("SERIAL123".to_string()),
                ..Identity::default()
            })
            .with_property("nvme.controller", "nvme0")
            .with_property("nvme.model", "Example NVMe")
            .with_property("nvme.firmware", "1.0")
            .with_property("nvme.subsystem", "nqn.2014-08.org.nvmexpress:uuid:12345678")
            .with_property("nvme.controller-id", "1")
            .with_property("nvme.id-ctrl.vid", "5197")
            .with_property("nvme.id-ctrl.ssvid", "5197")
            .with_property("nvme.id-ctrl.mdts", "9")
            .with_property("nvme.id-ctrl.controller-type", "1")
            .with_property("nvme.id-ctrl.oacs", "31")
            .with_property("nvme.id-ctrl.fuses", "1")
            .with_property("nvme.id-ctrl.fna", "4")
            .with_property("nvme.id-ctrl.awun", "255")
            .with_property("nvme.id-ctrl.awupf", "0")
            .with_property("nvme.id-ctrl.acwu", "0")
            .with_property("nvme.id-ctrl.sgls", "131073")
            .with_property("nvme.id-ctrl.namespace-set-id-max", "32")
            .with_property("nvme.id-ctrl.endurance-group-id-max", "8")
            .with_property("nvme.id-ctrl.ana-transition-time", "10")
            .with_property("nvme.id-ctrl.ana-group-max", "4")
            .with_property("nvme.id-ctrl.persistent-event-log-size", "4096")
            .with_property("nvme.id-ctrl.domain-id", "2")
            .with_property("nvme.id-ctrl.warning-composite-temp", "343")
            .with_property("nvme.id-ctrl.critical-composite-temp", "353")
            .with_property("nvme.id-ctrl.minimum-thermal-management-temp", "273")
            .with_property("nvme.id-ctrl.maximum-thermal-management-temp", "358")
            .with_property("nvme.id-ctrl.total-nvm-capacity", "1000000000")
            .with_property("nvme.id-ctrl.unallocated-nvm-capacity", "500000000")
            .with_property("nvme.id-ctrl.namespace-count", "16")
            .with_property("nvme.id-ctrl.oncs", "95")
            .with_property("nvme.id-ctrl.volatile-write-cache", "1")
            .with_property("nvme.id-ctrl.sanitize-capabilities", "7")
            .with_property("nvme.id-ctrl.ana-capabilities", "3")
            .with_property("nvme.smart.critical-warning", "0")
            .with_property("nvme.smart.temperature-kelvin", "301")
            .with_property("nvme.smart.available-spare-percent", "100")
            .with_property("nvme.smart.percent-used", "2")
            .with_property("nvme.smart.data-units-read", "123456")
            .with_property("nvme.smart.data-units-written", "654321")
            .with_property("nvme.smart.power-on-hours", "1200")
            .with_property("nvme.smart.unsafe-shutdowns", "3")
            .with_property("nvme.smart.media-errors", "0")
            .with_property("nvme.smart.error-log-entries", "4")
            .with_property("nvme.smart.temperature-sensor-1-kelvin", "300")
            .with_property("nvme.smart.temperature-sensor-2-kelvin", "302")
            .with_property("nvme.smart.temperature-sensor-3-kelvin", "303")
            .with_property("nvme.smart.temperature-sensor-4-kelvin", "304")
            .with_property("nvme.smart.thermal-temp1-transition-count", "5")
            .with_property("nvme.smart.thermal-temp2-transition-count", "6")
            .with_property("nvme.smart.thermal-temp1-total-time", "70")
            .with_property("nvme.smart.thermal-temp2-total-time", "80"),
    );
    graph.add_node(
        Node::new(
            "block:/dev/nvme0n1",
            NodeKind::NvmeNamespace,
            "/dev/nvme0n1",
        )
        .with_path("/dev/nvme0n1")
        .with_size_bytes(1_000_000_000_000)
        .with_usage(Usage {
            used_bytes: Some(400_000_000_000),
            free_bytes: Some(600_000_000_000),
            allocated_bytes: Some(400_000_000_000),
        })
        .with_identity(Identity {
            serial: Some("SERIAL123".to_string()),
            ..Identity::default()
        })
        .with_property("nvme.generic-path", "/dev/ng0n1")
        .with_property("nvme.model", "Example NVMe")
        .with_property("nvme.product", "Example Controller")
        .with_property("nvme.firmware", "1.0")
        .with_property("nvme.index", "0")
        .with_property("nvme.namespace", "1")
        .with_property("nvme.namespace-id", "1")
        .with_property(
            "nvme.namespace-uuid",
            "12345678-1234-1234-1234-123456789abc",
        )
        .with_property("nvme.eui64", "0011223344556677")
        .with_property("nvme.nguid", "00112233445566778899aabbccddeeff")
        .with_property("nvme.subsystem", "nvme-subsys0")
        .with_property("nvme.controller", "nvme0")
        .with_property("nvme.address", "0000:01:00.0")
        .with_property("nvme.transport", "pcie")
        .with_property("nvme.controller-id", "1")
        .with_property("nvme.namespace-capacity", "900000000000")
        .with_property("nvme.lba-format", "512 B + 0 B")
        .with_property("nvme.maximum-lba", "1953125")
        .with_property("nvme.sector-size", "512")
        .with_property("nvme.ana-state", "optimized")
        .with_property("nvme.formatted-lba-index", "0")
        .with_property("nvme.formatted-lba-data-size", "512")
        .with_property("nvme.formatted-lba-metadata-size", "0")
        .with_property("nvme.formatted-lba-relative-performance", "0")
        .with_property("nvme.id-ns.nsze", "1953125")
        .with_property("nvme.id-ns.ncap", "1800000")
        .with_property("nvme.id-ns.nuse", "900000")
        .with_property("nvme.id-ns.nsfeat", "0")
        .with_property("nvme.id-ns.nlbaf", "1")
        .with_property("nvme.id-ns.flbas", "0")
        .with_property("nvme.id-ns.nmic", "1")
        .with_property("nvme.id-ns.nvmcap", "1000000000"),
    );

    let mut output = Vec::new();
    print_nvme(&mut output, &graph).expect("nvme table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("SERIAL"));
    assert!(output.contains("CONTROLLER"));
    assert!(output.contains("USE%"));
    assert!(output.contains("nvme-controller"));
    assert!(output.contains("nvme0"));
    assert!(output.contains("nqn.2014-08.org.nvmexpress:uuid:12345678"));
    assert!(output.contains("vid=5197 ssvid=5197 mdts=9 controller-type=1"));
    assert!(
        output.contains("optional-admin-commands=31 fused-operations=1 format-nvm-attributes=4")
    );
    assert!(output.contains(
            "atomic-write-unit-normal=255 atomic-write-unit-powerfail=0 atomic-compare-write-unit=0 sgl-support=131073"
        ));
    assert!(output.contains(
        "namespace-set-id-max=32 endurance-group-id-max=8 ana-transition-time=10 ana-group-max=4"
    ));
    assert!(output.contains("persistent-event-log-size=4096 domain-id=2"));
    assert!(output.contains(
            "warning-composite-temp=343 critical-composite-temp=353 min-thermal-management-temp=273 max-thermal-management-temp=358"
        ));
    assert!(output.contains("total-nvm-capacity=1000000000 unallocated-nvm-capacity=500000000"));
    assert!(output.contains("namespace-count=16 oncs=95 volatile-write-cache=1"));
    assert!(output.contains("sanitize-capabilities=7 ana-capabilities=3"));
    assert!(output.contains("critical-warning=0 temperature-k=301 available-spare-percent=100"));
    assert!(output.contains("percent-used=2 data-units-read=123456"));
    assert!(output.contains("data-units-written=654321"));
    assert!(output.contains("power-on-hours=1200 unsafe-shutdowns=3 media-errors=0"));
    assert!(output.contains("error-log-entries=4 temp-sensor-1-k=300 temp-sensor-2-k=302"));
    assert!(output.contains("temp-sensor-3-k=303 temp-sensor-4-k=304"));
    assert!(output.contains(
            "thermal-temp1-transitions=5 thermal-temp2-transitions=6 thermal-temp1-total-time=70 thermal-temp2-total-time=80"
        ));
    assert!(output.contains("/dev/nvme0n1"));
    assert!(output.contains("SERIAL123"));
    assert!(output.contains("nvme0"));
    assert!(output.contains("40.0%"));
    assert!(output.contains("generic=/dev/ng0n1 nvme-model=Example NVMe"));
    assert!(output.contains("product=Example Controller firmware=1.0"));
    assert!(output
        .contains("ns-index=0 namespace=1 nsid=1 ns-uuid=12345678-1234-1234-1234-123456789abc"));
    assert!(output.contains(
        "eui64=0011223344556677 nguid=00112233445566778899aabbccddeeff subsystem=nvme-subsys0"
    ));
    assert!(output.contains("controller=nvme0 address=0000:01:00.0"));
    assert!(output.contains(
        "transport=pcie controller-id=1 namespace-capacity=900000000000 lba-format=512 B + 0 B"
    ));
    assert!(output.contains("max-lba=1953125 sector-size=512 ana-state=optimized"));
    assert!(
        output.contains("flba-index=0 flba-data=512 flba-metadata=0 flba-relative-performance=0")
    );
    assert!(output.contains("nsze=1953125 ncap=1800000 nuse=900000 nsfeat=0"));
    assert!(output.contains("nlbaf=1 flbas=0 nmic=1 nvmcap=1000000000"));
}

#[test]
fn raid_table_includes_array_and_member_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("md:/dev/md0", NodeKind::MdRaid, "/dev/md0")
            .with_path("/dev/md0")
            .with_size_bytes(1_071_644_672)
            .with_identity(Identity {
                uuid: Some("aaaa:bbbb:cccc:dddd".to_string()),
                ..Identity::default()
            })
            .with_property("md.version", "1.2")
            .with_property("md.uuid", "aaaa:bbbb:cccc:dddd")
            .with_property("md.level", "raid1")
            .with_property("md.state", "clean")
            .with_property("md.raid-devices", "2")
            .with_property("md.total-devices", "2")
            .with_property("md.array-devices", "2")
            .with_property("md.active-devices", "1")
            .with_property("md.working-devices", "2")
            .with_property("md.failed-devices", "1")
            .with_property("md.spare-devices", "1")
            .with_property("md.degraded-devices", "1")
            .with_property("md.name", "host:0")
            .with_property("md.creation-time", "Tue Jun 23 10:15:00 2026")
            .with_property("md.update-time", "Tue Jun 23 10:16:00 2026")
            .with_property("md.events", "17")
            .with_property("md.chunk-size", "512K")
            .with_property("md.layout", "near=2")
            .with_property("md.consistency-policy", "bitmap")
            .with_property("md.rebuild-status", "42% complete")
            .with_property("md.resync-status", "delayed")
            .with_property("md.check-status", "10% complete")
            .with_property("md.intent-bitmap", "Internal")
            .with_property("md.persistence", "Superblock is persistent")
            .with_property("md.bitmap", "0/8 pages [0KB], 65536KB chunk")
            .with_property("md.mdstat-state", "active")
            .with_property("md.mdstat-level", "raid1")
            .with_property("md.mdstat-devices", "2/1")
            .with_property("md.mdstat-health", "U_")
            .with_property("md.mdstat-progress", "recovery")
            .with_property("md.mdstat-progress-percent", "20.0%")
            .with_property("md.mdstat-progress-blocks", "209305/1046528")
            .with_property("md.mdstat-finish", "1.2min")
            .with_property("md.mdstat-speed", "12345K/sec")
            .with_property("md.mdstat-bitmap", "0/8 pages [0KB], 65536KB chunk"),
    );
    graph.add_node(
        Node::new("md:/dev/md/root", NodeKind::MdRaid, "/dev/md/root")
            .with_path("/dev/md/root")
            .with_identity(Identity {
                uuid: Some("eeee:ffff:1111:2222".to_string()),
                ..Identity::default()
            })
            .with_property("md.scan-metadata", "1.2")
            .with_property("md.uuid", "eeee:ffff:1111:2222")
            .with_property("md.scan-name", "host:root")
            .with_property("md.scan-spares", "1")
            .with_property("md.scan-devices", "/dev/sdc1,/dev/sdd1"),
    );
    graph.add_node(
        Node::new("block:/dev/sda1", NodeKind::Partition, "/dev/sda1")
            .with_path("/dev/sda1")
            .with_property("md.member-number", "0")
            .with_property("md.member-major", "8")
            .with_property("md.member-minor", "1")
            .with_property("md.member-raid-device", "0")
            .with_property("md.member-state", "active sync"),
    );
    graph.add_node(
        Node::new("block:/dev/sdb1", NodeKind::Partition, "/dev/sdb1")
            .with_path("/dev/sdb1")
            .with_property("md.member-number", "1")
            .with_property("md.member-major", "8")
            .with_property("md.member-minor", "17")
            .with_property("md.member-raid-device", "1")
            .with_property("md.member-state", "active sync")
            .with_property("md.mdstat-member-slot", "1")
            .with_property("md.mdstat-member-flags", "F"),
    );
    graph.add_edge(Edge::new(
        "block:/dev/sda1",
        "md:/dev/md0",
        Relationship::MemberOf,
    ));
    graph.add_edge(Edge::new(
        "block:/dev/sdb1",
        "md:/dev/md0",
        Relationship::MemberOf,
    ));

    let mut output = Vec::new();
    print_raid(&mut output, &graph).expect("raid table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("LEVEL"));
    assert!(output.contains("STATE"));
    assert!(output.contains("ACTIVE"));
    assert!(output.contains("FAILED"));
    assert!(output.contains("SPARE"));
    assert!(output.contains("MEMBERS"));
    assert!(output.contains("/dev/md0"));
    assert!(output.contains("raid1"));
    assert!(output.contains("clean"));
    assert!(output.contains("md-uuid=aaaa:bbbb:cccc:dddd"));
    assert!(output.contains("md-version=1.2 level=raid1 state=clean"));
    assert!(output.contains("raid-devices=2 total-devices=2 array-devices=2"));
    assert!(output.contains("active-devices=1 working-devices=2 failed-devices=1"));
    assert!(output.contains("spare-devices=1 degraded-devices=1"));
    assert!(output.contains("md-name=host:0"));
    assert!(output.contains("created=Tue Jun 23 10:15:00 2026"));
    assert!(output.contains("updated=Tue Jun 23 10:16:00 2026"));
    assert!(output.contains("events=17"));
    assert!(output.contains("chunk=512K layout=near=2"));
    assert!(output.contains("consistency=bitmap rebuild=42% complete"));
    assert!(output.contains("resync=delayed check=10% complete bitmap=Internal"));
    assert!(output.contains(
        "persistence=Superblock is persistent bitmap-detail=0/8 pages [0KB], 65536KB chunk"
    ));
    assert!(output.contains("mdstat-state=active mdstat-level=raid1"));
    assert!(output.contains("mdstat-devices=2/1 mdstat-health=U_"));
    assert!(output.contains("mdstat-progress=recovery mdstat-progress-percent=20.0%"));
    assert!(output.contains("mdstat-progress-blocks=209305/1046528"));
    assert!(output.contains("mdstat-finish=1.2min mdstat-speed=12345K/sec"));
    assert!(output.contains("mdstat-bitmap=0/8 pages [0KB], 65536KB chunk"));
    assert!(output.contains("/dev/md/root"));
    assert!(output.contains("md-uuid=eeee:ffff:1111:2222"));
    assert!(output.contains("scan-metadata=1.2 scan-name=host:root"));
    assert!(output.contains("scan-spares=1 scan-devices=/dev/sdc1,/dev/sdd1"));
    assert!(output.contains("/dev/sda1"));
    assert!(output.contains("active sync"));
    assert!(output.contains("member-number=0 member-major=8 member-minor=1 member-raid-device=0"));
    assert!(output.contains("member-state=active sync"));
    assert!(output.contains("/dev/sdb1"));
    assert!(output.contains("member-number=1 member-major=8 member-minor=17 member-raid-device=1"));
    assert!(output.contains("mdstat-member-slot=1 mdstat-member-flags=F"));
}

#[test]
fn loop_table_includes_mapping_and_backing_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("block:/dev/loop0", NodeKind::LoopDevice, "/dev/loop0")
            .with_path("/dev/loop0")
            .with_property("loop.back-file", "/var/lib/images/root.img")
            .with_property("loop.backing-inode", "12345")
            .with_property("loop.backing-major-minor", "0:45")
            .with_property("loop.major-minor", "7:0")
            .with_property("loop.offset", "1048576")
            .with_property("loop.sizelimit", "0")
            .with_property("loop.logical-sector-size", "512")
            .with_property("loop.autoclear", "true")
            .with_property("loop.partscan", "true")
            .with_property("loop.read-only", "false")
            .with_property("loop.direct-io", "true"),
    );
    graph.add_node(
        Node::new(
            "file:/var/lib/images/root.img",
            NodeKind::BackingFile,
            "/var/lib/images/root.img",
        )
        .with_path("/var/lib/images/root.img")
        .with_property("loop.backing", "true"),
    );
    graph.add_node(
        Node::new("block:/dev/loop1", NodeKind::LoopDevice, "/dev/loop1")
            .with_path("/dev/loop1")
            .with_size_bytes(1_073_741_824)
            .with_property("loop.back-file", "/dev/disk/by-id/nvme-loop-backing")
            .with_property("loop.offset", "0")
            .with_property("loop.sizelimit", "1073741824")
            .with_property("loop.read-only", "true"),
    );
    graph.add_edge(Edge::new(
        "file:/var/lib/images/root.img",
        "block:/dev/loop0",
        Relationship::Backs,
    ));

    let mut output = Vec::new();
    print_loop(&mut output, &graph).expect("loop table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("BACKING"));
    assert!(output.contains("OFFSET"));
    assert!(output.contains("/dev/loop0"));
    assert!(output.contains("/var/lib/images/root.img"));
    assert!(output.contains("1048576"));
    assert!(output.contains("ro=false"));
    assert!(output.contains(
        "back-file=/var/lib/images/root.img back-ino=12345 back-major-minor=0:45 major-minor=7:0"
    ));
    assert!(output.contains("logical-sector=512 autoclear=true partscan=true ro=false dio=true"));
    assert!(output.contains("loop-backing=true"));
    assert!(output.contains("/dev/loop1"));
    assert!(output.contains("1.0 GiB"));
    assert!(output.contains("/dev/disk/by-id/nvme-loop-backing"));
    assert!(output.contains("sizelimit=1073741824"));
}

#[test]
fn backing_files_table_includes_consumers_and_json_neighbors() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new(
            "file:/var/lib/images/root.img",
            NodeKind::BackingFile,
            "/var/lib/images/root.img",
        )
        .with_path("/var/lib/images/root.img")
        .with_size_bytes(4_294_967_296)
        .with_usage(Usage {
            used_bytes: Some(1_073_741_824),
            free_bytes: Some(3_221_225_472),
            allocated_bytes: Some(4_294_967_296),
        })
        .with_property("loop.backing", "true"),
    );
    graph.add_node(
        Node::new("block:/dev/loop0", NodeKind::LoopDevice, "/dev/loop0")
            .with_path("/dev/loop0")
            .with_property("loop.back-file", "/var/lib/images/root.img")
            .with_property("loop.offset", "0")
            .with_property("loop.read-only", "false"),
    );
    graph.add_edge(Edge::new(
        "file:/var/lib/images/root.img",
        "block:/dev/loop0",
        Relationship::Backs,
    ));

    let file = graph
        .nodes
        .iter()
        .find(|node| node.id.0 == "file:/var/lib/images/root.img")
        .expect("backing file exists");
    assert_eq!(consumer_count(&graph, file), 1);

    let mut output = Vec::new();
    print_backing_files(&mut output, &graph).expect("backing files table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("CONSUMERS"));
    assert!(output.contains("/var/lib/images/root.img"));
    assert!(output.contains("4.0 GiB"));
    assert!(output.contains("25.0%"));
    assert!(output.contains("loop-backing=true"));
    assert!(!output.contains("/dev/loop0"));

    let mut json = Vec::new();
    print_filtered_json(&mut json, &graph, is_backing_file_node)
        .expect("backing files json renders");
    let json = String::from_utf8(json).expect("json is utf8");
    assert!(json.contains("file:/var/lib/images/root.img"));
    assert!(json.contains("block:/dev/loop0"));
    assert!(json.contains("\"relationship\":\"backs\""));
}

#[test]
fn swap_table_includes_active_swap_usage_and_priority() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("block:/dev/sda3", NodeKind::Swap, "/dev/sda3")
            .with_path("/dev/sda3")
            .with_size_bytes(9_448_955_904)
            .with_usage(Usage {
                used_bytes: Some(53_592_064),
                free_bytes: Some(9_395_363_840),
                allocated_bytes: Some(9_448_955_904),
            })
            .with_property("swap.active", "true")
            .with_property("swap.type", "partition")
            .with_property("swap.priority", "-2"),
    );
    graph.add_node(
        Node::new("swap:/dev/sda3", NodeKind::Swap, "/dev/sda3")
            .with_path("/dev/sda3")
            .with_size_bytes(9_448_955_904)
            .with_usage(Usage {
                used_bytes: Some(53_592_064),
                free_bytes: Some(9_395_363_840),
                allocated_bytes: Some(9_448_955_904),
            })
            .with_property("swap.active", "true")
            .with_property("swap.type", "partition")
            .with_property("swap.priority", "-2"),
    );
    graph.add_node(
        Node::new("swap:/swapfile", NodeKind::Swap, "/swapfile")
            .with_path("/swapfile")
            .with_size_bytes(1_073_741_824)
            .with_usage(Usage {
                used_bytes: Some(0),
                free_bytes: Some(1_073_741_824),
                allocated_bytes: Some(1_073_741_824),
            })
            .with_property("swap.active", "true")
            .with_property("swap.type", "file")
            .with_property("swap.priority", "10"),
    );
    graph.add_node(
        Node::new("block:/dev/zram0", NodeKind::ZramDevice, "/dev/zram0")
            .with_path("/dev/zram0")
            .with_size_bytes(8_589_934_592)
            .with_usage(Usage {
                used_bytes: Some(2_147_483_648),
                free_bytes: Some(6_442_450_944),
                allocated_bytes: Some(805_306_368),
            })
            .with_property("zram.algorithm", "zstd")
            .with_property("zram.streams", "8")
            .with_property("zram.compressed", "715827882")
            .with_property("zram.total", "805306368")
            .with_property("zram.memory-used", "900000000")
            .with_property("zram.memory-peak", "900000000")
            .with_property("zram.compression-ratio", "2.67")
            .with_property("zram.mountpoint", "[SWAP]")
            .with_property("zram.swap", "true"),
    );
    graph.add_edge(Edge::new(
        "block:/dev/sda3",
        "swap:/dev/sda3",
        Relationship::Backs,
    ));

    let mut output = Vec::new();
    print_swap(&mut output, &graph).expect("swap table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("TYPE"));
    assert!(output.contains("PRIO"));
    assert!(output.contains("/dev/sda3"));
    assert!(output.contains("partition"));
    assert!(output.contains("-2"));
    assert!(output.contains("swap-active=true swap-type=partition swap-priority=-2"));
    assert!(output.contains("/swapfile"));
    assert!(output.contains("file"));
    assert!(output.contains("10"));
    assert!(output.contains("swap-active=true swap-type=file swap-priority=10"));
    assert!(output.contains("/dev/zram0"));
    assert!(output.contains("zram-algorithm=zstd zram-streams=8 zram-compressed=715827882"));
    assert!(output
        .contains("zram-total=805306368 zram-memory-used=900000000 zram-memory-peak=900000000"));
    assert!(output.contains("zram-ratio=2.67 zram-mountpoint=[SWAP] zram-swap=true"));
    assert!(output.contains("0.0%"));
}

#[test]
fn zram_table_includes_compressed_swap_memory_accounting() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("block:/dev/zram0", NodeKind::ZramDevice, "/dev/zram0")
            .with_path("/dev/zram0")
            .with_size_bytes(8_589_934_592)
            .with_usage(Usage {
                used_bytes: Some(2_147_483_648),
                free_bytes: Some(6_442_450_944),
                allocated_bytes: Some(805_306_368),
            })
            .with_property("zram.algorithm", "zstd")
            .with_property("zram.streams", "8")
            .with_property("zram.compressed", "715827882")
            .with_property("zram.data", "2147483648")
            .with_property("zram.total", "805306368")
            .with_property("zram.memory-limit", "0")
            .with_property("zram.memory-used", "900000000")
            .with_property("zram.memory-peak", "900000000")
            .with_property("zram.compression-ratio", "2.67")
            .with_property("zram.mountpoint", "[SWAP]")
            .with_property("zram.swap", "true"),
    );
    graph.add_node(
        Node::new("swap:/dev/sda3", NodeKind::Swap, "/dev/sda3")
            .with_path("/dev/sda3")
            .with_property("swap.type", "partition"),
    );

    let mut output = Vec::new();
    print_zram(&mut output, &graph).expect("zram table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("ALGO"));
    assert!(output.contains("RATIO"));
    assert!(output.contains("MEM-PEAK"));
    assert!(output.contains("/dev/zram0"));
    assert!(output.contains("8.0 GiB"));
    assert!(output.contains("2.0 GiB"));
    assert!(output.contains("768.0 MiB"));
    assert!(output.contains("zstd"));
    assert!(output.contains("2.67"));
    assert!(output.contains("900000000"));
    assert!(output.contains("[SWAP]"));
    assert!(output.contains("zram-compressed=715827882"));
    assert!(output.contains("zram-memory-limit=0"));
    assert!(output.contains("zram-memory-peak=900000000"));
    assert!(!output.contains("/dev/sda3"));

    let mut json = Vec::new();
    print_filtered_json(&mut json, &graph, is_zram_node).expect("zram json renders");
    let json = String::from_utf8(json).expect("json is utf8");
    assert!(json.contains("block:/dev/zram0"));
    assert!(!json.contains("swap:/dev/sda3"));
}

#[test]
fn mappings_table_includes_domain_metadata_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(Node::new(
        "block:/dev/nvme0n1p2",
        NodeKind::Partition,
        "/dev/nvme0n1p2",
    ));
    graph.add_node(
        Node::new(
            "block:/dev/mapper/cryptroot",
            NodeKind::LuksContainer,
            "cryptroot",
        )
        .with_path("/dev/mapper/cryptroot")
        .with_property("dm.name", "cryptroot")
        .with_property("dm.uuid", "CRYPT-LUKS2-crypt-uuid-cryptroot")
        .with_property("dm.major", "253")
        .with_property("dm.minor", "0")
        .with_property("dm.open-count", "1")
        .with_property("dm.segments", "1")
        .with_property("dm.events", "0")
        .with_property("dm.table.targets", "crypt")
        .with_property("dm.table.segment-count", "1")
        .with_property("dm.table.segment.0.start", "0")
        .with_property("dm.table.segment.0.length", "2097152")
        .with_property("dm.table.segment.0.target", "crypt")
        .with_property("dm.table.segment.0.crypt.cipher", "aes-xts-plain64")
        .with_property("dm.table.segment.0.crypt.device", "259:2")
        .with_property("dm.table.segment.0.crypt.offset", "4096")
        .with_property("dm.status.targets", "crypt")
        .with_property("dm.status.segment-count", "1")
        .with_property("dm.status.segment.0.target", "crypt")
        .with_property("dm.status.segment.0.payload", "0 2097152")
        .with_property("cryptsetup.active", "true")
        .with_property("cryptsetup.in-use", "true")
        .with_property("cryptsetup.cipher", "aes-xts-plain64")
        .with_property("cryptsetup.luks-version", "2")
        .with_property("cryptsetup.luks-epoch", "7")
        .with_property("cryptsetup.luks-metadata-area", "16384 [bytes]")
        .with_property("cryptsetup.luks-keyslots-area", "16744448 [bytes]")
        .with_property("cryptsetup.luks-subsystem", "(no subsystem)")
        .with_property("cryptsetup.luks-flags", "allow-discards")
        .with_property("cryptsetup.luks-keyslot-count", "2")
        .with_property("cryptsetup.luks-token-count", "1")
        .with_property("cryptsetup.luks-keyslots", "0,1")
        .with_property("cryptsetup.luks-tokens", "0")
        .with_property("cryptsetup.luks-keyslot-0-type", "luks2")
        .with_property("cryptsetup.luks-keyslot-0-priority", "normal")
        .with_property("cryptsetup.luks-keyslot-0-cipher", "aes-xts-plain64")
        .with_property("cryptsetup.luks-keyslot-0-cipher-key", "512 bits")
        .with_property("cryptsetup.luks-keyslot-0-pbkdf", "argon2id")
        .with_property("cryptsetup.luks-keyslot-0-time-cost", "4")
        .with_property("cryptsetup.luks-keyslot-0-memory", "1048576")
        .with_property("cryptsetup.luks-keyslot-0-threads", "4")
        .with_property("cryptsetup.luks-keyslot-1-type", "luks2")
        .with_property("cryptsetup.luks-keyslot-1-priority", "ignored")
        .with_property("cryptsetup.luks-token-0-type", "systemd-tpm2")
        .with_property("cryptsetup.luks-token-0-keyslot", "0")
        .with_property("cryptsetup.luks-data-cipher", "aes-xts-plain64")
        .with_property("cryptsetup.luks-data-offset", "32768 [bytes]")
        .with_property("cryptsetup.luks-data-length", "(whole device)")
        .with_property("cryptsetup.luks-data-sector", "4096 [bytes]"),
    );
    graph.add_edge(Edge::new(
        "block:/dev/nvme0n1p2",
        "block:/dev/mapper/cryptroot",
        Relationship::Backs,
    ));
    graph.add_node(
        Node::new(
            "block:/dev/mapper/cachevol",
            NodeKind::DeviceMapper,
            "cachevol",
        )
        .with_path("/dev/mapper/cachevol")
        .with_property("dm.name", "cachevol")
        .with_property("dm.table.targets", "cache")
        .with_property("dm.table.segment-count", "1")
        .with_property("dm.table.segment.0.target", "cache")
        .with_property("dm.table.segment.0.metadata-device", "253:10")
        .with_property("dm.table.segment.0.cache-device", "253:11")
        .with_property("dm.table.segment.0.origin-device", "253:12")
        .with_property("dm.table.segment.0.block-size", "128")
        .with_property("dm.status.targets", "cache")
        .with_property("dm.status.segment-count", "1")
        .with_property("dm.status.segment.0.target", "cache")
        .with_property("dm.status.segment.0.metadata-used-blocks", "64")
        .with_property("dm.status.segment.0.metadata-total-blocks", "256")
        .with_property("dm.status.segment.0.cache-used-blocks", "32")
        .with_property("dm.status.segment.0.cache-total-blocks", "1024")
        .with_property("dm.status.segment.0.read-hits", "900")
        .with_property("dm.status.segment.0.read-misses", "100")
        .with_property("dm.status.segment.0.write-hits", "700")
        .with_property("dm.status.segment.0.write-misses", "50")
        .with_property("dm.status.segment.0.dirty-blocks", "4"),
    );
    graph.add_node(
        Node::new("multipath:mpatha", NodeKind::MultipathDevice, "mpatha")
            .with_path("/dev/mapper/mpatha")
            .with_property("multipath.dm", "dm-2")
            .with_property("multipath.wwid", "3600508b400105e210000900000490000")
            .with_property("multipath.vendor-product", "IBM,2145")
            .with_property("multipath.size", "100G")
            .with_property("multipath.features", "'1 queue_if_no_path'")
            .with_property("multipath.write-protect", "rw"),
    );
    graph.add_node(
        Node::new("vdo:archive", NodeKind::VdoVolume, "archive")
            .with_property("vdo.storage-device", "/dev/sdb")
            .with_property("vdo.logical-size", "1T")
            .with_property("vdo.physical-size", "250G")
            .with_property("vdo.operating-mode", "normal")
            .with_property("vdo.write-policy", "sync")
            .with_property("vdo.compression", "enabled")
            .with_property("vdo.deduplication", "disabled"),
    );
    let segment = Node::new(
        "lvm-seg:vg0/thinpool:0",
        NodeKind::LvmSegment,
        "vg0/thinpool:0",
    )
    .with_property("lvm.segment-type", "thin-pool")
    .with_property("lvm.segment-start", "0")
    .with_property("lvm.segment-size", "100.00g")
    .with_property("lvm.chunk-size", "64.00k")
    .with_property("lvm.thin-count", "3")
    .with_property("lvm.discards", "passdown")
    .with_property("lvm.zero", "zero")
    .with_property("lvm.transaction-id", "42")
    .with_property("lvm.devices", "thinpool_tdata(0)")
    .with_property("lvm.metadata-devices", "thinpool_tmeta(0)")
    .with_property("lvm.segment-monitor", "monitored")
    .with_property("lvm.cache-metadata-format", "2")
    .with_property("lvm.segment-cache-mode", "writeback")
    .with_property("lvm.segment-cache-policy", "smq")
    .with_property("lvm.cache-settings", "migration_threshold=2048")
    .with_property("lvm.vdo-compression", "enabled")
    .with_property("lvm.vdo-deduplication", "enabled")
    .with_property("lvm.vdo-write-policy", "auto");
    let segment_details = usage_details(&segment);
    assert!(segment_details.contains("segment-type=thin-pool"));
    assert!(segment_details.contains("metadata-devices=thinpool_tmeta(0)"));
    assert!(segment_details.contains("segment-cache-policy=smq"));
    assert!(segment_details.contains("vdo-write-policy=auto"));
    graph.add_node(segment);
    graph.add_node(
        Node::new("block:/dev/bcache0", NodeKind::CacheDevice, "bcache0")
            .with_path("/dev/bcache0")
            .with_property("bcache.role", "backing")
            .with_property("bcache.kind", "cache-set")
            .with_property("bcache.label", "fast-cache")
            .with_property("bcache.state", "clean")
            .with_property("bcache.running", "1")
            .with_property("bcache.cache-available-percent", "78")
            .with_property("bcache.cache-mode", "writeback")
            .with_property("bcache.discard", "true")
            .with_property("bcache.io-errors", "0")
            .with_property("bcache.readahead", "0")
            .with_property("bcache.sequential-cutoff", "4.0M")
            .with_property("bcache.written", "512.0M")
            .with_property("bcache.writeback-rate", "1.0M/sec"),
    );

    let mut output = Vec::new();
    print_mappings(&mut output, &graph).expect("mappings table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("DETAILS"));
    assert!(output.contains("cryptroot"));
    assert!(output.contains(
            "dm-name=cryptroot dm-uuid=CRYPT-LUKS2-crypt-uuid-cryptroot dm-major=253 dm-minor=0 open=1 segments=1 events=0"
        ));
    assert!(
            output.contains(
                "active=true in-use=true cipher=aes-xts-plain64 luks=2 epoch=7 metadata-area=16384 [bytes] keyslots-area=16744448 [bytes] subsystem=(no subsystem) flags=allow-discards keyslots=2 tokens=1 keyslot-ids=0,1 token-ids=0 keyslot-0=luks2 keyslot-0-priority=normal"
            )
        );
    assert!(output.contains(
            "keyslot-0-cipher=aes-xts-plain64 keyslot-0-cipher-key=512 bits keyslot-0-pbkdf=argon2id keyslot-0-time=4 keyslot-0-memory=1048576 keyslot-0-threads=4"
        ));
    assert!(output.contains(
            "keyslot-1=luks2 keyslot-1-priority=ignored token-0=systemd-tpm2 token-0-keyslot=0 data-cipher=aes-xts-plain64"
        ));
    assert!(output.contains(
            "dm-table-targets=crypt dm-table-segments=1 dm-table-start=0 dm-table-length=2097152 dm-table-target=crypt"
        ));
    assert!(output
        .contains("dm-crypt-cipher=aes-xts-plain64 dm-crypt-device=259:2 dm-crypt-offset=4096"));
    assert!(output.contains(
            "dm-status-targets=crypt dm-status-segments=1 dm-status-target=crypt dm-status-payload=0 2097152"
        ));
    assert!(output.contains("cachevol"));
    assert!(output.contains(
        "dm-name=cachevol dm-table-targets=cache dm-table-segments=1 dm-table-target=cache"
    ));
    assert!(output.contains(
            "dm-table-metadata-device=253:10 dm-table-cache-device=253:11 dm-table-origin-device=253:12 dm-table-block-size=128"
        ));
    assert!(output.contains("dm-status-targets=cache dm-status-segments=1 dm-status-target=cache"));
    assert!(output.contains(
            "dm-status-metadata-used=64 dm-status-metadata-total=256 dm-status-cache-used=32 dm-status-cache-total=1024"
        ));
    assert!(output.contains(
            "dm-status-read-hits=900 dm-status-read-misses=100 dm-status-write-hits=700 dm-status-write-misses=50 dm-status-dirty=4"
        ));
    assert!(
        output.contains("dm=dm-2 wwid=3600508b400105e210000900000490000 vendor=IBM,2145 size=100G")
    );
    assert!(
            output.contains(
                "backing=/dev/sdb logical=1T physical=250G mode=normal write-policy=sync compression=enabled deduplication=disabled"
            )
        );
    assert!(output.contains("vg0/thinpool:0"));
    assert!(output.contains("segment-type=thin-pool"));
    assert!(output.contains("metadata-devices=thinpool_tmeta(0)"));
    assert!(output.contains("segment-cache-policy=smq"));
    assert!(output.contains("vdo-write-policy=auto"));
    assert!(output.contains(
            "role=backing kind=cache-set label=fast-cache state=clean running=1 available-percent=78 cache-mode=writeback discard=true io-errors=0 readahead=0 sequential-cutoff=4.0M written=512.0M writeback-rate=1.0M/sec"
        ));
}

#[test]
fn dm_table_includes_table_status_and_json_neighbors() {
    let mut graph = StorageGraph::empty();
    graph.add_node(Node::new(
        "block:/dev/nvme0n1p2",
        NodeKind::Partition,
        "/dev/nvme0n1p2",
    ));
    graph.add_node(
        Node::new(
            "block:/dev/mapper/cryptroot",
            NodeKind::DeviceMapper,
            "cryptroot",
        )
        .with_path("/dev/mapper/cryptroot")
        .with_property("dm.name", "cryptroot")
        .with_property("dm.uuid", "CRYPT-LUKS2-crypt-uuid-cryptroot")
        .with_property("dm.major", "253")
        .with_property("dm.minor", "0")
        .with_property("dm.open-count", "1")
        .with_property("dm.segments", "1")
        .with_property("dm.events", "0")
        .with_property("dm.table.targets", "crypt")
        .with_property("dm.table.segment-count", "1")
        .with_property("dm.table.segment.0.start", "0")
        .with_property("dm.table.segment.0.length", "2097152")
        .with_property("dm.table.segment.0.target", "crypt")
        .with_property("dm.table.segment.0.crypt.cipher", "aes-xts-plain64")
        .with_property("dm.table.segment.0.crypt.device", "259:2")
        .with_property("dm.table.segment.0.crypt.offset", "4096")
        .with_property("dm.status.targets", "crypt")
        .with_property("dm.status.segment-count", "1")
        .with_property("dm.status.segment.0.target", "crypt")
        .with_property("dm.status.segment.0.payload", "0 2097152"),
    );
    graph.add_edge(Edge::new(
        "block:/dev/nvme0n1p2",
        "block:/dev/mapper/cryptroot",
        Relationship::Backs,
    ));
    graph.add_node(
        Node::new(
            "block:/dev/mapper/cachevol",
            NodeKind::DeviceMapper,
            "cachevol",
        )
        .with_path("/dev/mapper/cachevol")
        .with_property("dm.name", "cachevol")
        .with_property("dm.table.targets", "cache")
        .with_property("dm.table.segment-count", "1")
        .with_property("dm.table.segment.0.target", "cache")
        .with_property("dm.table.segment.0.metadata-device", "253:10")
        .with_property("dm.table.segment.0.cache-device", "253:11")
        .with_property("dm.table.segment.0.origin-device", "253:12")
        .with_property("dm.table.segment.0.block-size", "128")
        .with_property("dm.status.targets", "cache")
        .with_property("dm.status.segment-count", "1")
        .with_property("dm.status.segment.0.target", "cache")
        .with_property("dm.status.segment.0.metadata-used-blocks", "64")
        .with_property("dm.status.segment.0.metadata-total-blocks", "256")
        .with_property("dm.status.segment.0.cache-used-blocks", "32")
        .with_property("dm.status.segment.0.cache-total-blocks", "1024")
        .with_property("dm.status.segment.0.read-hits", "900")
        .with_property("dm.status.segment.0.read-misses", "100")
        .with_property("dm.status.segment.0.write-hits", "700")
        .with_property("dm.status.segment.0.write-misses", "50")
        .with_property("dm.status.segment.0.dirty-blocks", "4"),
    );

    let mut output = Vec::new();
    print_dm(&mut output, &graph).expect("dm table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("TARGETS"));
    assert!(output.contains("STATUS"));
    assert!(output.contains("MAJOR:MINOR"));
    assert!(output.contains("cryptroot"));
    assert!(output.contains("crypt"));
    assert!(output.contains("253:0"));
    assert!(output.contains(
            "dm-name=cryptroot dm-uuid=CRYPT-LUKS2-crypt-uuid-cryptroot dm-major=253 dm-minor=0 open=1 segments=1 events=0"
        ));
    assert!(output.contains(
            "dm-table-targets=crypt dm-table-segments=1 dm-table-start=0 dm-table-length=2097152 dm-table-target=crypt"
        ));
    assert!(output
        .contains("dm-crypt-cipher=aes-xts-plain64 dm-crypt-device=259:2 dm-crypt-offset=4096"));
    assert!(output.contains(
            "dm-status-targets=crypt dm-status-segments=1 dm-status-target=crypt dm-status-payload=0 2097152"
        ));
    assert!(output.contains("cachevol"));
    assert!(output.contains("cache"));
    assert!(output.contains(
            "dm-table-metadata-device=253:10 dm-table-cache-device=253:11 dm-table-origin-device=253:12 dm-table-block-size=128"
        ));
    assert!(output.contains(
            "dm-status-read-hits=900 dm-status-read-misses=100 dm-status-write-hits=700 dm-status-write-misses=50 dm-status-dirty=4"
        ));

    let mut json = Vec::new();
    print_filtered_json(&mut json, &graph, is_dm_node).expect("dm json renders");
    let json = String::from_utf8(json).expect("json is utf8");
    assert!(json.contains("block:/dev/mapper/cryptroot"));
    assert!(json.contains("block:/dev/nvme0n1p2"));
    assert!(json.contains("\"relationship\":\"backs\""));
}

#[test]
fn mounts_table_includes_source_and_pseudo_mount_details() {
    let mut graph = StorageGraph::empty();
    graph.add_node(
        Node::new("mount:/run", NodeKind::Mountpoint, "/run")
            .with_property("filesystem.type", "tmpfs")
            .with_property("mount.source", "tmpfs")
            .with_property("mount.read-write", "true")
            .with_property("tmpfs.size", "64M")
            .with_property("tmpfs.mode", "0755"),
    );
    graph.add_node(
        Node::new("mount:/srv/cache", NodeKind::Mountpoint, "/srv/cache")
            .with_property("filesystem.type", "none")
            .with_property("mount.source", "/var/cache/disk-nix")
            .with_property("mount.bind", "true"),
    );
    graph.add_node(
        Node::new("mount:/merged", NodeKind::Mountpoint, "/merged")
            .with_property("filesystem.type", "overlay")
            .with_property("mount.source", "overlay")
            .with_property("overlay.lowerdir", "/lower")
            .with_property("overlay.upperdir", "/upper")
            .with_property("overlay.workdir", "/work")
            .with_property("overlay.index", "off"),
    );

    assert_eq!(
        mount_details(
            graph
                .nodes
                .iter()
                .find(|node| node.name == "/run")
                .expect("tmpfs mount fixture should exist")
        ),
        "source=tmpfs rw=true tmpfs-size=64M mode=0755"
    );

    let mut output = Vec::new();
    print_mounts(&mut output, &graph).expect("mount table renders");
    let output = String::from_utf8(output).expect("table is utf8");

    assert!(output.contains("DETAILS"));
    assert!(output.contains("source=/var/cache/disk-nix bind=true"));
    assert!(
        output.contains("source=overlay lowerdir=/lower upperdir=/upper workdir=/work index=off")
    );
}
