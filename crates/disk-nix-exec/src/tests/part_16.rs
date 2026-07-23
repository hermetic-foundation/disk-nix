#[test]
fn failed_partition_growth_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "partitions": {
                "root": {
                  "operation": "grow",
                  "target": "/dev/disk/by-id/nvme-root-part2",
                  "device": "/dev/disk/by-id/nvme-root",
                  "partitionNumber": 2,
                  "end": "100%"
                }
              },
              "apply": {
                "allowOffline": true,
                "allowGrow": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_resize = [
        "parted",
        "-s",
        "/dev/disk/by-id/nvme-root",
        "resizepart",
        "2",
        "100%",
    ];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_resize,
            status_code: Some(if argv == failed_resize { 1 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_resize {
                "resizepart failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report
        .execution_results
        .iter()
        .any(|result| !result.success && result.argv == failed_resize));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("partition domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Grow"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["parted", "-lm", "/dev/disk/by-id/nvme-root"] && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv
            == [
                "lsblk",
                "--json",
                "--bytes",
                "--output-all",
                "/dev/disk/by-id/nvme-root",
            ]
            && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["disk-nix", "inspect", "/dev/disk/by-id/nvme-root", "--json"]
            && !command.mutates
    }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| note.contains("partition-table changes") && note.contains("kernel")));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("partition roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv == ["parted", "-lm", "/dev/disk/by-id/nvme-root"] && !command.mutates
    }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("partition rollback recovery review is reported");
    assert!(rollback.commands.iter().any(|command| {
        command.argv
            == [
                "lsblk",
                "--json",
                "--bytes",
                "--output-all",
                "/dev/disk/by-id/nvme-root",
            ]
            && !command.mutates
    }));
}

#[test]
fn failed_dm_map_rename_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "dmMaps": {
                "cryptswap": {
                  "operation": "rename",
                  "target": "/dev/mapper/cryptswap",
                  "renameTo": "/dev/mapper/cryptswap-retired"
                }
              },
              "apply": {
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_rename = [
        "dmsetup",
        "rename",
        "/dev/mapper/cryptswap",
        "cryptswap-retired",
    ];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_rename,
            status_code: Some(if argv == failed_rename { 1 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_rename {
                "dm rename failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report
        .execution_results
        .iter()
        .any(|result| !result.success && result.argv == failed_rename));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("device-mapper domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Rename"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv
            == [
                "dmsetup",
                "info",
                "-c",
                "--noheadings",
                "-o",
                "name,uuid,major,minor,open,segments,events",
                "/dev/mapper/cryptswap",
            ]
            && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["dmsetup", "deps", "-o", "devname", "/dev/mapper/cryptswap"]
            && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["dmsetup", "table", "/dev/mapper/cryptswap"] && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["dmsetup", "status", "/dev/mapper/cryptswap"] && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["disk-nix", "inspect", "/dev/mapper/cryptswap", "--json"]
            && !command.mutates
    }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| note.contains("local mapping changes") && note.contains("dependencies")));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("device-mapper roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv == ["dmsetup", "status", "/dev/mapper/cryptswap"] && !command.mutates
    }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("device-mapper rollback recovery review is reported");
    assert!(rollback.commands.iter().any(|command| {
        command.argv == ["dmsetup", "table", "/dev/mapper/cryptswap"] && !command.mutates
    }));
}

#[test]
fn failed_nfs_mount_remount_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "nfs": {
                "mounts": {
                  "/srv/tuned": {
                    "operation": "remount",
                    "source": "nas.example.com:/srv/tuned",
                    "options": ["_netdev", "ro", "vers=4.2"]
                  }
                }
              },
              "apply": {
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_remount = ["mount", "-o", "remount,_netdev,ro,vers=4.2", "/srv/tuned"];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_remount,
            status_code: Some(if argv == failed_remount { 32 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_remount {
                "remount failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report
        .execution_results
        .iter()
        .any(|result| !result.success && result.argv == failed_remount));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("NFS domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Remount"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["findmnt", "--json", "/srv/tuned"] && !command.mutates
    }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["nfsstat", "-m", "/srv/tuned"] && !command.mutates }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["disk-nix", "inspect", "/srv/tuned", "--json"] && !command.mutates
    }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| note.contains("NFS changes") && note.contains("negotiated mount options")));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("NFS roll-forward recovery review is reported");
    assert!(roll_forward
        .commands
        .iter()
        .any(|command| { command.argv == ["nfsstat", "-m", "/srv/tuned"] && !command.mutates }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("NFS rollback recovery review is reported");
    assert!(rollback.commands.iter().any(|command| {
        command.argv == ["findmnt", "--json", "/srv/tuned"] && !command.mutates
    }));
}

#[test]
fn blocked_policy_reports_blocked_status() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "datasets": {
                  "tank/old": { "destroy": true }
                }
              },
              "apply": {
                "allowDestructive": false
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::Blocked);
    assert_eq!(report.apply.blocked_count, 1);
    assert_eq!(report.command_summary.command_count, 0);
    assert!(report.command_summary.all_commands_ready());
    assert!(report.command_plan.is_empty());
    assert_eq!(report.verification_summary.step_count, 0);
    assert!(report.verification_plan.is_empty());
    assert!(report.recovery_actions.iter().any(|action| {
        action.kind == RecoveryActionKind::ReviewPolicy
            && action.summary.contains("Review blocked actions")
    }));
    assert!(report.recovery_actions.iter().any(|action| {
        action.kind == RecoveryActionKind::InspectCurrentState
            && action
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "topology", "--json"])
    }));
}

#[test]
fn filesystem_growth_reports_read_only_verification_steps() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "filesystems": {
                "root": {
                  "mountpoint": "/",
                  "fsType": "xfs",
                  "resizePolicy": "grow-only"
                }
              },
              "apply": {
                "allowGrow": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert_eq!(report.verification_plan.len(), 1);
    assert!(report.verification_plan[0].commands.iter().any(|command| {
        command.argv == ["findmnt", "--json", "--bytes", "/"] && !command.mutates
    }));
    assert!(report.verification_plan[0]
        .checks
        .iter()
        .any(|check| check.contains("filesystem size")));
}

#[test]
fn allowed_lun_growth_reports_rescan_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "luns": {
                "iqn.2026-06.example:storage/root:0": {
                  "operation": "grow",
                  "device": "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0",
                  "devices": [
                    "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-0"
                  ]
                }
              },
              "apply": {
                "allowGrow": true,
                "allowOffline": true
              }
            }"#,
        )
        .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert_eq!(report.command_plan.len(), 1);
    assert!(report.command_plan[0].commands.iter().any(|command| {
        command.argv == ["iscsiadm", "--mode", "session", "--rescan"] && command.mutates
    }));
    assert!(report.command_plan[0].commands.iter().any(|command| {
        command.argv == ["lsscsi", "-t", "-s"]
            && !command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(report.command_plan[0].commands.iter().any(|command| {
            command.argv
                == [
                    "sh",
                    "-c",
                    "block=$(basename \"$(readlink -f \"$1\")\"); printf '1\\n' > \"/sys/class/block/${block}/device/rescan\"",
                    "disk-nix-scsi-rescan",
                    "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0",
                ]
                && command.mutates
                && command.readiness == CommandReadiness::Ready
        }));
    assert!(report.command_plan[0].commands.iter().any(|command| {
            command.argv
                == [
                    "sh",
                    "-c",
                    "block=$(basename \"$(readlink -f \"$1\")\"); printf '1\\n' > \"/sys/class/block/${block}/device/rescan\"",
                    "disk-nix-scsi-rescan",
                    "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-0",
                ]
                && command.mutates
                && command.readiness == CommandReadiness::Ready
        }));
    assert!(report.verification_plan[0].commands.iter().any(|command| {
        command.argv == ["lsscsi", "-t", "-s"]
            && !command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(report.verification_plan[0].commands.iter().any(|command| {
        command.argv
            == [
                "blockdev",
                "--getsize64",
                "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0",
            ]
            && !command.mutates
    }));
}

#[test]
fn host_storage_rescan_reports_online_refresh_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "luns": {
                  "iqn.2026-06.example:storage/root:0": {
                    "operation": "rescan",
                    "devices": [
                      "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0"
                    ]
                  }
                },
                "iscsiSessions": {
                  "iqn.2026-06.example:storage.root": {
                    "operation": "rescan"
                  }
                },
                "nvmeNamespaces": {
                  "/dev/nvme2": {
                    "operation": "rescan"
                  }
                }
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.apply.allowed_count >= 3);
    assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luns:iqn.2026-06.example:storage/root:0:rescan"
                && step.commands.iter().any(|command| {
                    command.argv == ["iscsiadm", "--mode", "session", "--rescan"]
                        && command.mutates
                })
                && step.commands.iter().any(|command| {
                    command.argv == ["lsscsi", "-t", "-s"] && !command.mutates
                })
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "sh",
                            "-c",
                            "block=$(basename \"$(readlink -f \"$1\")\"); printf '1\\n' > \"/sys/class/block/${block}/device/rescan\"",
                            "disk-nix-scsi-rescan",
                            "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["multipath", "-r"])
        }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "iscsisessions:iqn.2026-06.example:storage.root:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["iscsiadm", "--mode", "session", "--rescan"] && command.mutates
            })
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["lsscsi", "-t", "-s"] && !command.mutates)
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nvmenamespaces:/dev/nvme2:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["nvme", "list-subsys", "--output-format=json"] && !command.mutates
            })
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["nvme", "ns-rescan", "/dev/nvme2"])
    }));
    assert!(report.command_summary.all_commands_ready());
}

#[test]
fn lun_attach_and_detach_reports_host_path_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "luns": {
                "iqn.2026-06.example:storage/root:0": {
                  "operation": "attach",
                  "device": "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0"
                },
                "iqn.2026-06.example:storage/old:1": {
                  "operation": "detach",
                  "devices": [
                    "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-1"
                  ]
                }
              },
              "apply": {
                "allowOffline": true
              }
            }"#,
        )
        .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "luns:iqn.2026-06.example:storage/root:0:attach"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["iscsiadm", "--mode", "session", "--rescan"])
    }));
    assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luns:iqn.2026-06.example:storage/root:0:attach"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "sh",
                            "-c",
                            "block=$(basename \"$(readlink -f \"$1\")\"); printf '1\\n' > \"/sys/class/block/${block}/device/rescan\"",
                            "disk-nix-scsi-rescan",
                            "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0",
                        ]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
    assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luns:iqn.2026-06.example:storage/root:0:attach"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "blockdev",
                            "--getsize64",
                            "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0",
                        ]
                        && !command.mutates
                })
        }));
    assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luns:iqn.2026-06.example:storage/old:1:detach"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "sh",
                            "-c",
                            "block=$(basename \"$(readlink -f \"$1\")\"); printf '1\\n' > \"/sys/class/block/${block}/device/delete\"",
                            "disk-nix-scsi-delete",
                            "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-1",
                        ]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
    assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "luns:iqn.2026-06.example:storage/old:1:detach"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "test",
                            "!",
                            "-e",
                            "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-1",
                        ]
                        && !command.mutates
                })
        }));
}

#[test]
fn lun_lifecycle_accepts_stable_path_aliases() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "luns": {
                  "iqn.2026-06.example:storage/path:0": {
                    "operation": "attach",
                    "path": "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0"
                  },
                  "iqn.2026-06.example:storage/paths:1": {
                    "operation": "rescan",
                    "paths": [
                      "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-1"
                    ]
                  },
                  "iqn.2026-06.example:storage/device-paths:2": {
                    "operation": "detach",
                    "devicePaths": [
                      "/dev/disk/by-path/ip-192.0.2.12:3260-iscsi-iqn.2026-06.example:storage-lun-2"
                    ]
                  }
                }
              },
              "apply": {
                "allowOffline": true
              }
            }"#,
        )
        .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luns:iqn.2026-06.example:storage/path:0:attach"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["lsscsi", "-t", "-s"] && !command.mutates)
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "blockdev",
                            "--getsize64",
                            "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
    assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luns:iqn.2026-06.example:storage/paths:1:rescan"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "sh",
                            "-c",
                            "block=$(basename \"$(readlink -f \"$1\")\"); printf '1\\n' > \"/sys/class/block/${block}/device/rescan\"",
                            "disk-nix-scsi-rescan",
                            "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-1",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
    assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luns:iqn.2026-06.example:storage/device-paths:2:detach"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["lsscsi", "-t", "-s"] && !command.mutates)
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "sh",
                            "-c",
                            "block=$(basename \"$(readlink -f \"$1\")\"); printf '1\\n' > \"/sys/class/block/${block}/device/delete\"",
                            "disk-nix-scsi-delete",
                            "/dev/disk/by-path/ip-192.0.2.12:3260-iscsi-iqn.2026-06.example:storage-lun-2",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
}

#[test]
fn target_lun_lifecycle_renders_provider_handoff_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "array-a/root": {
                    "operation": "create",
                    "desiredSize": "2TiB",
                    "source": "pool-a/volumes/root",
                    "provider": "netapp-ontap",
                    "vendor": "netapp",
                    "arrayId": "ontap-cluster-a",
                    "storagePool": "aggr1",
                    "volumeId": "vol-root",
                    "snapshotId": "snap-before",
                    "cloneSource": "vol-root@snap-before",
                    "maskingGroup": "linux-hosts",
                    "portal": "192.0.2.10:3260",
                    "client": "iqn.2026-06.example:host.primary",
                    "initiators": [
                      "iqn.2026-06.example:host.secondary"
                    ]
                  },
                  "array-a/root-grow": {
                    "operation": "grow",
                    "target": "array-a/root",
                    "desiredSize": "3TiB"
                  }
                }
              },
              "apply": {
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert_eq!(report.command_summary.needs_domain_implementation_count, 6);
    assert!(!report.command_summary.all_commands_ready());

    let create = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "targetluns:array-a/root:create")
        .expect("target-side LUN create command plan exists");
    assert!(create.requires_manual_review);
    assert!(create.commands.iter().any(|command| {
        command.argv
            == [
                "<target-lun-provider:netapp-ontap>",
                "create-lun",
                "--target",
                "array-a/root",
                "--provider",
                "netapp-ontap",
                "--vendor",
                "netapp",
                "--array-id",
                "ontap-cluster-a",
                "--storage-pool",
                "aggr1",
                "--volume-id",
                "vol-root",
                "--snapshot-id",
                "snap-before",
                "--clone-source",
                "vol-root@snap-before",
                "--masking-group",
                "linux-hosts",
                "--size",
                "2TiB",
                "--backing",
                "pool-a/volumes/root",
                "--portal",
                "192.0.2.10:3260",
                "--initiator",
                "iqn.2026-06.example:host.primary",
                "--initiator",
                "iqn.2026-06.example:host.secondary",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command
                .unresolved_inputs
                .contains(&"netapp-ontap target LUN provider implementation".to_string())
            && command
                .provider_capabilities
                .contains(&"target-lun.create".to_string())
            && command
                .provider_capabilities
                .contains(&"target-lun.persistence".to_string())
            && command
                .provider_capabilities
                .contains(&"target-lun.refusal".to_string())
            && command
                .provider_capabilities
                .contains(&"target-lun.initiator-scope.declared".to_string())
            && command
                .provider_capabilities
                .contains(&"target-lun.array-id.declared".to_string())
            && command
                .provider_capabilities
                .contains(&"target-lun.volume-id.declared".to_string())
            && command
                .provider_capabilities
                .contains(&"target-lun.snapshot-id.declared".to_string())
            && command
                .provider_capabilities
                .contains(&"target-lun.clone-source.declared".to_string())
            && command
                .provider_capabilities
                .contains(&"target-lun.masking-group.declared".to_string())
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "targetluns:array-a/root:create"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "<target-lun-provider:netapp-ontap>",
                        "show-mapping",
                        "--portal",
                        "192.0.2.10:3260",
                        "--target",
                        "array-a/root",
                    ]
                    && !command.mutates
                    && command
                        .provider_capabilities
                        .contains(&"target-lun.verification".to_string())
                    && command
                        .provider_capabilities
                        .contains(&"target-lun.portal.declared".to_string())
            })
            && step.commands.iter().any(|command| {
                command.argv == ["lsscsi", "-t", "-s"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["multipath", "-ll"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "array-a/root", "--json"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));

    let grow = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "targetluns:array-a/root-grow:grow")
        .expect("target-side LUN grow command plan exists");
    assert!(grow.commands.iter().any(|command| {
        command.argv
            == [
                "<target-lun-provider>",
                "grow-lun",
                "--target",
                "array-a/root",
                "--size",
                "3TiB",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command
                .provider_capabilities
                .contains(&"target-lun.grow".to_string())
            && command
                .provider_capabilities
                .contains(&"target-lun.capacity.expand".to_string())
            && command
                .provider_capabilities
                .contains(&"target-lun.consumer-refresh.handoff".to_string())
    }));

    let script = report
        .to_shell_script()
        .expect("not-ready plans still render a review script");
    assert!(script.contains("# Provider capabilities: target-lun.identity, target-lun.inventory"));
    assert!(script.contains("target-lun.capacity.expand"));
    assert!(script.contains("target-lun.refusal"));
}
