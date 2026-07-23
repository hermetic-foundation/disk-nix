#[test]
fn failed_iscsi_session_login_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "iscsiSessions": {
                "iqn.2026-06.example:storage.root": {
                  "operation": "login",
                  "portal": "192.0.2.10:3260"
                }
              },
              "apply": {
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_login = [
        "iscsiadm",
        "--mode",
        "node",
        "--targetname",
        "iqn.2026-06.example:storage.root",
        "--portal",
        "192.0.2.10:3260",
        "--login",
    ];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_login,
            status_code: Some(if argv == failed_login { 15 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_login {
                "login failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report.execution_results.iter().any(|result| {
        !result.success
            && result.argv
                == [
                    "iscsiadm",
                    "--mode",
                    "node",
                    "--targetname",
                    "iqn.2026-06.example:storage.root",
                    "--portal",
                    "192.0.2.10:3260",
                    "--login",
                ]
    }));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("iSCSI session domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Login"));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["iscsiadm", "--mode", "session"] && !command.mutates }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv
            == [
                "iscsiadm",
                "--mode",
                "node",
                "--targetname",
                "iqn.2026-06.example:storage.root",
            ]
            && !command.mutates
    }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["lsscsi", "-t", "-s"] && !command.mutates }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["multipath", "-ll"] && !command.mutates }));
    assert!(domain_recovery.notes.iter().any(|note| {
        note.contains("iSCSI session changes") && note.contains("login or logout")
    }));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("iSCSI roll-forward recovery review is reported");
    assert!(roll_forward
        .commands
        .iter()
        .any(|command| { command.argv == ["iscsiadm", "--mode", "session"] && !command.mutates }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("iSCSI rollback recovery review is reported");
    assert!(rollback
        .commands
        .iter()
        .any(|command| { command.argv == ["multipath", "-ll"] && !command.mutates }));
}

#[test]
fn failed_vdo_growth_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "vdoVolumes": {
                "archive": {
                  "operation": "grow",
                  "desiredSize": "4TiB"
                }
              },
              "apply": {
                "allowGrow": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_grow = [
        "vdo",
        "growLogical",
        "--name",
        "archive",
        "--vdoLogicalSize",
        "4TiB",
    ];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_grow,
            status_code: Some(if argv == failed_grow { 1 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_grow {
                "growth failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report.execution_results.iter().any(|result| {
        !result.success
            && result.argv
                == [
                    "vdo",
                    "growLogical",
                    "--name",
                    "archive",
                    "--vdoLogicalSize",
                    "4TiB",
                ]
    }));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("VDO domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Grow"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["vdo", "status", "--name", "archive"] && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["vdostats", "--human-readable", "archive"] && !command.mutates
    }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["disk-nix", "vdo", "--json"] && !command.mutates }));
    assert!(domain_recovery.notes.iter().any(|note| {
        note.contains("VDO lifecycle changes") && note.contains("create, grow, start, stop")
    }));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("VDO roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv == ["vdo", "status", "--name", "archive"] && !command.mutates
    }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("VDO rollback recovery review is reported");
    assert!(rollback.commands.iter().any(|command| {
        command.argv == ["vdostats", "--human-readable", "archive"] && !command.mutates
    }));
}

#[test]
fn failed_multipath_resize_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "multipathMaps": {
                "root-map": {
                  "device": "/dev/mapper/mpatha",
                  "operation": "grow"
                }
              },
              "apply": {
                "allowGrow": true,
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_resize = ["multipathd", "resize", "map", "/dev/mapper/mpatha"];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_resize,
            status_code: Some(if argv == failed_resize { 1 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_resize {
                "resize failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report.execution_results.iter().any(|result| {
        !result.success && result.argv == ["multipathd", "resize", "map", "/dev/mapper/mpatha"]
    }));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("multipath domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Grow"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["multipath", "-ll", "/dev/mapper/mpatha"] && !command.mutates
    }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["lsscsi", "-t", "-s"] && !command.mutates }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["disk-nix", "multipath", "--json"] && !command.mutates
    }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| { note.contains("multipath changes") && note.contains("reload, resize") }));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("multipath roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv == ["multipath", "-ll", "/dev/mapper/mpatha"] && !command.mutates
    }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("multipath rollback recovery review is reported");
    assert!(rollback.commands.iter().any(|command| {
        command.argv == ["disk-nix", "multipath", "--json"] && !command.mutates
    }));
}

#[test]
fn failed_multipath_replace_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "multipathMaps": {
                "root-map": {
                  "device": "/dev/mapper/mpatha",
                  "replaceDevices": {
                    "/dev/sdc": "/dev/sdd"
                  }
                }
              },
              "apply": {
                "allowOffline": true,
                "allowDeviceReplacement": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_delete = ["multipathd", "del", "path", "/dev/sdc"];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_delete,
            status_code: Some(if argv == failed_delete { 87 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_delete {
                "multipath replacement delete failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report.execution_results.iter().any(|result| {
        result.success && result.argv == ["multipathd", "add", "path", "/dev/sdd"]
    }));
    assert!(report.execution_results.iter().any(|result| {
        !result.success && result.argv == ["multipathd", "del", "path", "/dev/sdc"]
    }));
    let partial = report
        .partial_execution_recovery
        .as_ref()
        .expect("partial execution recovery is reported");
    assert_eq!(
        partial.failed_action_id,
        "multipathMaps:root-map:replace-device:/dev/sdc"
    );
    assert_eq!(
        partial.failed_command,
        vec!["multipathd", "del", "path", "/dev/sdc"]
    );
    assert_eq!(partial.completed_mutating_command_count, 1);
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("multipath replacement domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("ReplaceDevice"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["multipath", "-ll", "/dev/mapper/mpatha"] && !command.mutates
    }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["lsscsi", "-t", "-s"] && !command.mutates }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["disk-nix", "multipath", "--json"] && !command.mutates
    }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| { note.contains("multipath changes") && note.contains("path removal") }));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("multipath replacement roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv
            == [
                "disk-nix",
                "apply",
                "--spec",
                "<spec>",
                "--probe-current",
                "--json",
            ]
            && command.readiness == CommandReadiness::ManualOnly
    }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("multipath replacement rollback recovery review is reported");
    assert!(rollback.commands.iter().all(|command| !command.mutates));
    assert!(rollback.commands.iter().any(|command| {
        command.argv == ["disk-nix", "multipath", "--json"] && !command.mutates
    }));
}

#[test]
fn failed_luks_open_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "luks": {
                "devices": {
                  "cryptarchive": {
                    "name": "cryptarchive",
                    "device": "/dev/disk/by-id/archive-luks",
                    "operation": "open"
                  }
                }
              },
              "apply": {
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_open = [
        "cryptsetup",
        "open",
        "/dev/disk/by-id/archive-luks",
        "cryptarchive",
    ];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_open,
            status_code: Some(if argv == failed_open { 2 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_open {
                "open failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report.execution_results.iter().any(|result| {
        !result.success
            && result.argv
                == [
                    "cryptsetup",
                    "open",
                    "/dev/disk/by-id/archive-luks",
                    "cryptarchive",
                ]
    }));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("LUKS domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Open"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/archive-luks"]
            && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["cryptsetup", "status", "cryptarchive"] && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv
            == [
                "disk-nix",
                "inspect",
                "/dev/disk/by-id/archive-luks",
                "--json",
            ]
            && !command.mutates
    }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| note.contains("LUKS changes")));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| note.contains("alternate unlock paths")));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("LUKS roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv == ["cryptsetup", "status", "cryptarchive"] && !command.mutates
    }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("LUKS rollback recovery review is reported");
    assert!(rollback.commands.iter().any(|command| {
        command.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/archive-luks"]
            && !command.mutates
    }));
}

#[test]
fn failed_lvm_volume_growth_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "volumes": {
                "root": {
                  "target": "vg0/root",
                  "operation": "grow",
                  "desiredSize": "50GiB"
                }
              },
              "apply": {
                "allowGrow": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_extend = ["lvextend", "--resizefs", "--size", "50GiB", "vg0/root"];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_extend,
            status_code: Some(if argv == failed_extend { 5 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_extend {
                "extend failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report.execution_results.iter().any(|result| {
        !result.success && result.argv == ["lvextend", "--resizefs", "--size", "50GiB", "vg0/root"]
    }));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("LVM domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Grow"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["lvs", "--reportformat", "json", "vg0/root"] && !command.mutates
    }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["vgs", "--reportformat", "json"] && !command.mutates }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["pvs", "--reportformat", "json"] && !command.mutates }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["disk-nix", "inspect", "vg0/root", "--json"] && !command.mutates
    }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| { note.contains("LVM changes") && note.contains("activation, resize") }));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("LVM roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv == ["lvs", "--reportformat", "json", "vg0/root"] && !command.mutates
    }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("LVM rollback recovery review is reported");
    assert!(rollback.commands.iter().any(|command| {
        command.argv == ["disk-nix", "inspect", "vg0/root", "--json"] && !command.mutates
    }));
}

#[test]
fn failed_lvm_volume_group_rename_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "volumeGroups": {
                "vg-old": {
                  "operation": "rename",
                  "renameTo": "vg-new"
                }
              },
              "apply": {
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_rename = ["vgrename", "vg-old", "vg-new"];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_rename,
            status_code: Some(if argv == failed_rename { 5 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_rename {
                "rename failed".to_string()
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
        .expect("LVM VG domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Rename"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["vgs", "--reportformat", "json", "vg-old"] && !command.mutates
    }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["pvs", "--reportformat", "json"] && !command.mutates }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["lvs", "--reportformat", "json", "-a"] && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["disk-nix", "inspect", "vg-old", "--json"] && !command.mutates
    }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| note.contains("LVM changes") && note.contains("import, export")));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("LVM VG roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv == ["vgs", "--reportformat", "json", "vg-old"] && !command.mutates
    }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("LVM VG rollback recovery review is reported");
    assert!(rollback.commands.iter().any(|command| {
        command.argv == ["disk-nix", "inspect", "vg-old", "--json"] && !command.mutates
    }));
}

#[test]
fn failed_bcache_property_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "caches": {
                "writeback-cache": {
                  "path": "/dev/bcache1",
                  "properties": {
                    "bcache.cache-mode": "writearound"
                  }
                }
              },
              "apply": {
                "allowPropertyChanges": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_property = [
        "sh",
        "-c",
        "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/$3\"",
        "disk-nix-bcache-property",
        "/dev/bcache1",
        "writearound",
        "cache_mode",
    ];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_property,
            status_code: Some(if argv == failed_property { 5 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_property {
                "cache mode failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report
        .execution_results
        .iter()
        .any(|result| !result.success && result.argv == failed_property));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("bcache domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("SetProperty"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv
            == [
                "sh",
                "-c",
                "cat \"/sys/block/${1#/dev/}/bcache/$2\"",
                "disk-nix-bcache-read",
                "/dev/bcache1",
                "state",
            ]
            && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv
            == [
                "sh",
                "-c",
                "cat \"/sys/block/${1#/dev/}/bcache/$2\"",
                "disk-nix-bcache-read",
                "/dev/bcache1",
                "dirty_data",
            ]
            && !command.mutates
    }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["disk-nix", "cache", "--json"] && !command.mutates }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["disk-nix", "inspect", "/dev/bcache1", "--json"] && !command.mutates
    }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| note.contains("cache changes") && note.contains("dirty-data")));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("bcache rollback recovery review is reported");
    assert!(rollback.commands.iter().any(|command| {
        command.argv
            == [
                "sh",
                "-c",
                "cat \"/sys/block/${1#/dev/}/bcache/$2\"",
                "disk-nix-bcache-read",
                "/dev/bcache1",
                "cache_mode",
            ]
            && !command.mutates
    }));
}

#[test]
fn failed_lvm_cache_property_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "lvmCaches": {
                "vg0/root": {
                  "properties": {
                    "lvm.cache-mode": "writethrough"
                  }
                }
              },
              "apply": {
                "allowPropertyChanges": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_property = ["lvchange", "--cachemode", "writethrough", "vg0/root"];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_property,
            status_code: Some(if argv == failed_property { 5 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_property {
                "cache mode failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report
        .execution_results
        .iter()
        .any(|result| !result.success && result.argv == failed_property));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("LVM cache domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("SetProperty"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv
            == [
                "lvs",
                "--reportformat",
                "json",
                "-a",
                "-o",
                "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent",
                "vg0/root",
            ]
            && !command.mutates
    }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["vgs", "--reportformat", "json"] && !command.mutates }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["pvs", "--reportformat", "json"] && !command.mutates }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["disk-nix", "inspect", "vg0/root", "--json"] && !command.mutates
    }));
}

#[test]
fn failed_vdo_property_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "vdoVolumes": {
                "archive": {
                  "properties": {
                    "writePolicy": "sync"
                  }
                }
              },
              "apply": {
                "allowPropertyChanges": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_property = [
        "vdo",
        "changeWritePolicy",
        "--name",
        "archive",
        "--writePolicy",
        "sync",
    ];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_property,
            status_code: Some(if argv == failed_property { 86 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_property {
                "VDO write policy failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report
        .execution_results
        .iter()
        .any(|result| !result.success && result.argv == failed_property));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("VDO domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("SetProperty"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["vdo", "status", "--name", "archive"] && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["vdostats", "--human-readable", "archive"] && !command.mutates
    }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["disk-nix", "vdo", "--json"] && !command.mutates }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| { note.contains("VDO lifecycle changes") && note.contains("operating mode") }));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("VDO roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv == ["vdo", "status", "--name", "archive"] && !command.mutates
    }));
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv
            == [
                "disk-nix",
                "apply",
                "--spec",
                "<spec>",
                "--probe-current",
                "--json",
            ]
            && command.readiness == CommandReadiness::ManualOnly
    }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("VDO rollback recovery review is reported");
    assert!(rollback.commands.iter().all(|command| !command.mutates));
    assert!(rollback.commands.iter().any(|command| {
        command.argv == ["vdostats", "--human-readable", "archive"] && !command.mutates
    }));
    assert!(report
        .recovery_actions
        .iter()
        .any(|action| action.kind == RecoveryActionKind::PreserveRecoveryPoints));
}
