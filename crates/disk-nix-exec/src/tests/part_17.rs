#[test]
fn target_lun_lio_provider_renders_concrete_targetcli_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:storage.root": {
                    "operation": "create",
                    "provider": "lio",
                    "source": "/dev/zvol/tank/root",
                    "lun": 7,
                    "portal": "192.0.2.10:3260",
                    "client": "iqn.2026-06.example:host.primary",
                    "initiators": [
                      "iqn.2026-06.example:host.secondary"
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
    assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
    assert!(report.command_summary.all_commands_ready());

    let step = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "targetluns:iqn.2026-06.example:storage.root:create")
        .expect("LIO target-side LUN create command plan exists");
    assert!(step.requires_manual_review);
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "targetcli",
                "/backstores/block",
                "create",
                "name=_dev_zvol_tank_root",
                "dev=/dev/zvol/tank/root",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "targetcli",
                "/iscsi",
                "create",
                "iqn.2026-06.example:storage.root",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "targetcli",
                "/iscsi/iqn.2026-06.example:storage.root/tpg1/luns",
                "create",
                "/backstores/block/_dev_zvol_tank_root",
                "lun=7",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "targetcli",
                "/iscsi/iqn.2026-06.example:storage.root/tpg1/acls",
                "create",
                "iqn.2026-06.example:host.primary",
            ]
            && command.mutates
    }));
    assert!(step
        .commands
        .iter()
        .any(|command| { command.argv == ["targetcli", "saveconfig"] && command.mutates }));

    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "targetluns:iqn.2026-06.example:storage.root:create"
            && step.commands.iter().any(|command| {
                command.argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));

    let targetcli = report
        .tool_requirements
        .iter()
        .find(|requirement| requirement.tool == "targetcli")
        .expect("targetcli tool requirement exists");
    assert!(targetcli
        .remediation
        .iter()
        .any(|hint| hint.contains("pkgs.targetcli-fb")));
}

#[test]
fn target_lun_lio_grow_and_property_use_native_inventory_and_capacity_validation() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:storage.root": {
                    "operation": "grow",
                    "provider": "lio",
                    "source": "/dev/zvol/tank/root",
                    "desiredSize": "4TiB",
                    "lun": 7,
                    "properties": {
                      "lio.writeCache": "off"
                    }
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
    assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
    assert!(report.command_summary.all_commands_ready());

    let grow = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "targetluns:iqn.2026-06.example:storage.root:grow")
        .expect("LIO target-side LUN grow command plan exists");
    assert!(grow.commands.iter().any(|command| {
        command.argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]
            && !command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(grow.commands.iter().any(|command| {
        command.argv == ["targetcli", "/backstores/block/_dev_zvol_tank_root", "ls"]
            && !command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(grow.commands.iter().any(|command| {
        command.argv == ["blockdev", "--getsize64", "/dev/zvol/tank/root"]
            && !command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(grow.commands.iter().any(|command| {
        command.argv
            == [
                "targetcli",
                "/iscsi/iqn.2026-06.example:storage.root/tpg1/luns",
                "ls",
            ]
            && !command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(grow.commands.iter().any(|command| {
        command.argv == ["targetcli", "saveconfig"]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    let grow_verification = report
        .verification_plan
        .iter()
        .find(|step| step.action_id == "targetluns:iqn.2026-06.example:storage.root:grow")
        .expect("LIO target-side LUN grow verification plan exists");
    assert!(grow_verification.commands.iter().any(|command| {
        command.argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]
            && !command.mutates
    }));
    assert!(grow_verification
        .commands
        .iter()
        .any(|command| { command.argv == ["lsscsi", "-t", "-s"] && !command.mutates }));
    assert!(grow_verification
        .commands
        .iter()
        .any(|command| { command.argv == ["multipath", "-ll"] && !command.mutates }));
    assert!(grow_verification.commands.iter().any(|command| {
        command.argv
            == [
                "disk-nix",
                "inspect",
                "iqn.2026-06.example:storage.root",
                "--json",
            ]
            && !command.mutates
    }));

    let property = report
        .command_plan
        .iter()
        .find(|step| {
            step.action_id
                == "targetLuns:iqn.2026-06.example:storage.root:set-property:lio.writeCache"
        })
        .expect("LIO target-side LUN property command plan exists");
    assert!(property.commands.iter().any(|command| {
        command.argv == ["targetcli", "/backstores/block/_dev_zvol_tank_root", "ls"]
            && !command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(property.commands.iter().any(|command| {
        command.argv
            == [
                "targetcli",
                "/backstores/block/_dev_zvol_tank_root",
                "set",
                "attribute",
                "emulate_write_cache=0",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(property.commands.iter().any(|command| {
        command.argv == ["targetcli", "saveconfig"]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
}

#[test]
fn target_lun_lio_fileio_grow_forces_backstore_resize_before_refresh() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:storage.file": {
                    "operation": "grow",
                    "provider": "lio",
                    "backstoreType": "fileio",
                    "source": "/var/lib/iscsi/root.img",
                    "desiredSize": "4TiB",
                    "lun": 3
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
    assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
    assert!(report.command_summary.all_commands_ready());

    let grow = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "targetluns:iqn.2026-06.example:storage.file:grow")
        .expect("LIO fileio target-side LUN grow command plan exists");
    assert!(grow.commands.iter().any(|command| {
        command.argv
            == [
                "targetcli",
                "/backstores/fileio/_var_lib_iscsi_root.img",
                "ls",
            ]
            && !command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(grow.commands.iter().any(|command| {
        command.argv == ["truncate", "--size", "4TiB", "/var/lib/iscsi/root.img"]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
            && command.note.contains("fileio backstore")
    }));
    assert!(grow.commands.iter().any(|command| {
        command.argv == ["stat", "--format=%s", "/var/lib/iscsi/root.img"]
            && !command.mutates
            && command.readiness == CommandReadiness::Ready
    }));

    let truncate_requirement = report
        .tool_requirements
        .iter()
        .find(|requirement| requirement.tool == "truncate")
        .expect("truncate tool requirement exists");
    assert!(truncate_requirement
        .remediation
        .iter()
        .any(|hint| hint.contains("pkgs.coreutils")));
}

#[test]
fn failed_target_lun_lio_create_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:storage.root": {
                    "operation": "create",
                    "provider": "lio",
                    "source": "/dev/zvol/tank/root",
                    "lun": 7,
                    "portal": "192.0.2.10:3260",
                    "client": "iqn.2026-06.example:host.primary"
                  }
                }
              },
              "apply": {
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_lun_create = [
        "targetcli",
        "/iscsi/iqn.2026-06.example:storage.root/tpg1/luns",
        "create",
        "/backstores/block/_dev_zvol_tank_root",
        "lun=7",
    ];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_lun_create,
            status_code: Some(if argv == failed_lun_create { 85 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_lun_create {
                "target LUN mapping failed".to_string()
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
                    "targetcli",
                    "/iscsi/iqn.2026-06.example:storage.root/tpg1/luns",
                    "create",
                    "/backstores/block/_dev_zvol_tank_root",
                    "lun=7",
                ]
    }));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("target-side LUN domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Create"));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["targetcli", "/iscsi", "ls"] && !command.mutates }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]
            && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show",
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
        note.contains("target-side LUN changes") && note.contains("provider inventory")
    }));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("target-side LUN roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]
            && !command.mutates
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
        .expect("target-side LUN rollback recovery review is reported");
    assert!(rollback.commands.iter().all(|command| !command.mutates));
    assert!(rollback.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show",
            ]
            && !command.mutates
    }));
    assert!(report
        .recovery_actions
        .iter()
        .any(|action| action.kind == RecoveryActionKind::PreserveRecoveryPoints));
}

#[test]
fn failed_target_lun_lio_property_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:storage.root": {
                    "provider": "lio",
                    "source": "/dev/zvol/tank/root",
                    "lun": 7,
                    "properties": {
                      "lio.writeCache": "off"
                    }
                  }
                }
              },
              "apply": {
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_property = [
        "targetcli",
        "/backstores/block/_dev_zvol_tank_root",
        "set",
        "attribute",
        "emulate_write_cache=0",
    ];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_property,
            status_code: Some(if argv == failed_property { 88 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_property {
                "target LUN property failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert_eq!(
        report
            .partial_execution_recovery
            .as_ref()
            .expect("partial execution recovery is reported")
            .failed_action_id,
        "targetLuns:iqn.2026-06.example:storage.root:set-property:lio.writeCache"
    );
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("target-side LUN property domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("SetProperty"));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["targetcli", "/iscsi", "ls"] && !command.mutates }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]
            && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show",
            ]
            && !command.mutates
    }));
    assert!(domain_recovery.notes.iter().any(|note| {
        note.contains("target-side LUN changes") && note.contains("provider inventory")
    }));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("target-side LUN property roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]
            && !command.mutates
    }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("target-side LUN property rollback recovery review is reported");
    assert!(rollback.commands.iter().all(|command| !command.mutates));
}

#[test]
fn target_lun_lio_destroy_renders_concrete_targetcli_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:storage.root": {
                    "destroy": true,
                    "provider": "lio",
                    "source": "/dev/zvol/tank/root",
                    "lun": 7,
                    "client": "iqn.2026-06.example:host.primary",
                    "initiators": [
                      "iqn.2026-06.example:host.secondary"
                    ]
                  }
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowOffline": true,
                "backupVerified": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
    assert!(report.command_summary.all_commands_ready());

    let step = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "targetluns:iqn.2026-06.example:storage.root:destroy")
        .expect("LIO target-side LUN destroy command plan exists");
    assert!(step.requires_manual_review);
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "targetcli",
                "/iscsi/iqn.2026-06.example:storage.root/tpg1/acls",
                "delete",
                "iqn.2026-06.example:host.primary",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "targetcli",
                "/iscsi/iqn.2026-06.example:storage.root/tpg1/luns",
                "delete",
                "7",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "targetcli",
                "/iscsi",
                "delete",
                "iqn.2026-06.example:storage.root",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "targetcli",
                "/backstores/block",
                "delete",
                "_dev_zvol_tank_root",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(step
        .commands
        .iter()
        .any(|command| command.argv == ["targetcli", "saveconfig"] && command.mutates));
}

#[test]
fn target_lun_lio_destroy_requires_backstore_identity_for_backstore_removal() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:storage.root": {
                    "destroy": true,
                    "provider": "lio",
                    "lun": 7
                  }
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowOffline": true,
                "backupVerified": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert_eq!(report.command_summary.needs_domain_implementation_count, 2);
    assert!(!report.command_summary.all_commands_ready());

    let step = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "targetluns:iqn.2026-06.example:storage.root:destroy")
        .expect("LIO target-side LUN destroy command plan exists");
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "targetcli",
                "/backstores/block",
                "delete",
                "<backstore-name>",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command
                .unresolved_inputs
                .contains(&"LIO backstore name or backing device for removal".to_string())
    }));
}

#[test]
fn target_lun_tgt_provider_renders_concrete_tgtadm_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:tgt.root": {
                    "operation": "create",
                    "provider": "tgt",
                    "targetId": 42,
                    "source": "/dev/zvol/tank/root",
                    "lun": 8,
                    "client": "ALL"
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
    assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
    assert!(report.command_summary.all_commands_ready());

    let step = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "targetluns:iqn.2026-06.example:tgt.root:create")
        .expect("Linux tgt target-side LUN create command plan exists");
    assert!(step.requires_manual_review);
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm",
                "--lld",
                "iscsi",
                "--mode",
                "target",
                "--op",
                "new",
                "--tid",
                "42",
                "--targetname",
                "iqn.2026-06.example:tgt.root",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm",
                "--lld",
                "iscsi",
                "--mode",
                "logicalunit",
                "--op",
                "new",
                "--tid",
                "42",
                "--lun",
                "8",
                "--backing-store",
                "/dev/zvol/tank/root",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm",
                "--lld",
                "iscsi",
                "--mode",
                "target",
                "--op",
                "bind",
                "--tid",
                "42",
                "--initiator-address",
                "ALL",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "targetluns:iqn.2026-06.example:tgt.root:create"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show", "--tid",
                        "42",
                    ]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));

    let tgtadm = report
        .tool_requirements
        .iter()
        .find(|requirement| requirement.tool == "tgtadm")
        .expect("tgtadm tool requirement exists");
    assert!(tgtadm
        .remediation
        .iter()
        .any(|hint| hint.contains("pkgs.tgt")));
}

#[test]
fn target_lun_tgt_grow_and_property_use_native_refresh_and_capacity_validation() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:tgt.root": {
                    "operation": "grow",
                    "provider": "tgt",
                    "targetId": 42,
                    "source": "/dev/zvol/tank/root",
                    "desiredSize": "4TiB",
                    "lun": 8,
                    "properties": {
                      "tgt.writeCache": "off"
                    }
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
    assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
    assert!(report.command_summary.all_commands_ready());

    let grow = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "targetluns:iqn.2026-06.example:tgt.root:grow")
        .expect("Linux tgt target-side LUN grow command plan exists");
    assert!(grow.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show", "--tid", "42",
            ]
            && !command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(grow.commands.iter().any(|command| {
        command.argv == ["blockdev", "--getsize64", "/dev/zvol/tank/root"]
            && !command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(grow.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm",
                "--lld",
                "iscsi",
                "--mode",
                "logicalunit",
                "--op",
                "update",
                "--tid",
                "42",
                "--lun",
                "8",
                "--params",
                "online=1",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(grow.commands.iter().any(|command| {
        command.argv == ["tgt-admin", "--dump"]
            && !command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    let grow_verification = report
        .verification_plan
        .iter()
        .find(|step| step.action_id == "targetluns:iqn.2026-06.example:tgt.root:grow")
        .expect("Linux tgt target-side LUN grow verification plan exists");
    assert!(grow_verification.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show", "--tid", "42",
            ]
            && !command.mutates
    }));
    assert!(grow_verification
        .commands
        .iter()
        .any(|command| { command.argv == ["lsscsi", "-t", "-s"] && !command.mutates }));
    assert!(grow_verification
        .commands
        .iter()
        .any(|command| { command.argv == ["multipath", "-ll"] && !command.mutates }));
    assert!(grow_verification.commands.iter().any(|command| {
        command.argv
            == [
                "disk-nix",
                "inspect",
                "iqn.2026-06.example:tgt.root",
                "--json",
            ]
            && !command.mutates
    }));

    let property = report
        .command_plan
        .iter()
        .find(|step| {
            step.action_id == "targetLuns:iqn.2026-06.example:tgt.root:set-property:tgt.writeCache"
        })
        .expect("Linux tgt target-side LUN property command plan exists");
    assert!(property.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show", "--tid", "42",
            ]
            && !command.mutates
    }));
    assert!(property.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm",
                "--lld",
                "iscsi",
                "--mode",
                "logicalunit",
                "--op",
                "update",
                "--tid",
                "42",
                "--lun",
                "8",
                "--name",
                "tgt.writeCache",
                "--value",
                "off",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
}
