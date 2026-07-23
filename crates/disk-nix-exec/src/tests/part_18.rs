#[test]
fn failed_target_lun_tgt_property_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:tgt.root": {
                    "provider": "tgt",
                    "targetId": 42,
                    "source": "/dev/zvol/tank/root",
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

    let failed_property = [
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
    ];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_property,
            status_code: Some(if argv == failed_property { 89 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_property {
                "tgt property failed".to_string()
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
        "targetLuns:iqn.2026-06.example:tgt.root:set-property:tgt.writeCache"
    );
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("tgt target-side LUN property domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("SetProperty"));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["targetcli", "/iscsi", "ls"] && !command.mutates }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["targetcli", "/iscsi/iqn.2026-06.example:tgt.root", "ls"]
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
        .expect("tgt target-side LUN property roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show", "--tid", "42",
            ]
            && !command.mutates
    }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("tgt target-side LUN property rollback recovery review is reported");
    assert!(rollback.commands.iter().all(|command| !command.mutates));
}

#[test]
fn target_lun_tgt_provider_requires_reviewed_target_id_and_lun_inputs() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:tgt.root": {
                    "operation": "create",
                    "provider": "tgt"
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
    assert!(!report.command_summary.all_commands_ready());

    let step = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "targetluns:iqn.2026-06.example:tgt.root:create")
        .expect("Linux tgt target-side LUN create command plan exists");
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
                "<tid>",
                "--targetname",
                "iqn.2026-06.example:tgt.root",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command
                .unresolved_inputs
                .contains(&"Linux tgt numeric target id (targetId or tid)".to_string())
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
                "<tid>",
                "--lun",
                "<lun>",
                "--backing-store",
                "<backing-block-device-or-file>",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command
                .unresolved_inputs
                .contains(&"Linux tgt LUN number".to_string())
            && command
                .unresolved_inputs
                .contains(&"Linux tgt backing store path".to_string())
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
                "<tid>",
                "--initiator-address",
                "<initiator-address-or-ALL>",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command
                .unresolved_inputs
                .contains(&"Linux tgt initiator address or ALL ACL value".to_string())
    }));
}

#[test]
fn target_lun_scst_provider_renders_concrete_scstadmin_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:scst.root": {
                    "operation": "create",
                    "provider": "scst",
                    "source": "/dev/zvol/tank/root",
                    "lun": 9,
                    "group": "hosts",
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
        .find(|step| step.action_id == "targetluns:iqn.2026-06.example:scst.root:create")
        .expect("SCST target-side LUN create command plan exists");
    assert!(step.requires_manual_review);
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "scstadmin",
                "-open_dev",
                "_dev_zvol_tank_root",
                "-handler",
                "vdisk_blockio",
                "-attributes",
                "filename=/dev/zvol/tank/root",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "scstadmin",
                "-add_target",
                "iqn.2026-06.example:scst.root",
                "-driver",
                "iscsi",
            ]
            && command.mutates
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "scstadmin",
                "-add_group",
                "hosts",
                "-driver",
                "iscsi",
                "-target",
                "iqn.2026-06.example:scst.root",
            ]
            && command.mutates
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "scstadmin",
                "-add_init",
                "iqn.2026-06.example:host.primary",
                "-driver",
                "iscsi",
                "-target",
                "iqn.2026-06.example:scst.root",
                "-group",
                "hosts",
            ]
            && command.mutates
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "scstadmin",
                "-add_lun",
                "9",
                "-driver",
                "iscsi",
                "-target",
                "iqn.2026-06.example:scst.root",
                "-group",
                "hosts",
                "-device",
                "_dev_zvol_tank_root",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "scstadmin",
                "-enable_target",
                "iqn.2026-06.example:scst.root",
                "-driver",
                "iscsi",
            ]
            && command.mutates
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv == ["scstadmin", "-write_config", "/etc/scst.conf"] && command.mutates
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "targetluns:iqn.2026-06.example:scst.root:create"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "scstadmin",
                        "-list_target",
                        "iqn.2026-06.example:scst.root",
                        "-driver",
                        "iscsi",
                    ]
                    && !command.mutates
            })
    }));

    let scstadmin = report
        .tool_requirements
        .iter()
        .find(|requirement| requirement.tool == "scstadmin")
        .expect("scstadmin tool requirement exists");
    assert!(scstadmin
        .remediation
        .iter()
        .any(|hint| hint.contains("provides scstadmin")));
}

#[test]
fn target_lun_scst_grow_and_property_use_native_scstadmin_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:scst.root": {
                    "operation": "grow",
                    "provider": "scst",
                    "source": "/dev/zvol/tank/root",
                    "desiredSize": "4TiB",
                    "lun": 9,
                    "group": "hosts",
                    "properties": {
                      "read_only": "0"
                    }
                  }
                }
              },
              "apply": {
                "allowOffline": true,
                "allowGrow": true,
                "allowPropertyChanges": true
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
        .find(|step| step.action_id == "targetluns:iqn.2026-06.example:scst.root:grow")
        .expect("SCST target-side LUN grow command plan exists");
    assert!(grow.commands.iter().any(|command| {
        command.argv == ["scstadmin", "-list_dev_attr", "_dev_zvol_tank_root"]
            && !command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(grow.commands.iter().any(|command| {
        command.argv == ["scstadmin", "-resync_dev", "_dev_zvol_tank_root"]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));

    let property = report
        .command_plan
        .iter()
        .find(|step| {
            step.action_id == "targetLuns:iqn.2026-06.example:scst.root:set-property:read_only"
        })
        .expect("SCST target-side LUN property command plan exists");
    assert!(property.commands.iter().any(|command| {
        command.argv
            == [
                "scstadmin",
                "-set_lun_attr",
                "9",
                "-driver",
                "iscsi",
                "-target",
                "iqn.2026-06.example:scst.root",
                "-group",
                "hosts",
                "-attributes",
                "read_only=0",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
}

#[test]
fn target_lun_scst_provider_requires_reviewed_lun_and_backing_inputs() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:scst.root": {
                    "operation": "create",
                    "provider": "scst"
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
    assert!(!report.command_summary.all_commands_ready());

    let step = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "targetluns:iqn.2026-06.example:scst.root:create")
        .expect("SCST target-side LUN create command plan exists");
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "scstadmin",
                "-open_dev",
                "iqn.2026-06.example_scst.root",
                "-handler",
                "vdisk_blockio",
                "-attributes",
                "filename=<backing-block-device-or-file>",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command
                .unresolved_inputs
                .contains(&"SCST backing block device or file".to_string())
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "scstadmin",
                "-add_lun",
                "<lun>",
                "-driver",
                "iscsi",
                "-target",
                "iqn.2026-06.example:scst.root",
                "-device",
                "iqn.2026-06.example_scst.root",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command
                .unresolved_inputs
                .contains(&"SCST LUN number".to_string())
    }));
}

#[test]
fn nvme_namespace_lifecycle_reports_nvme_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "nvmeNamespaces": {
                "/dev/nvme0": {
                  "operation": "create",
                  "desiredSize": "100G",
                  "namespaceId": "4",
                  "controllers": "0x1"
                },
                "/dev/nvme1": {
                  "operation": "grow"
                },
                "/dev/nvme2": {
                  "operation": "attach",
                  "namespaceId": "7",
                  "controllers": "0x2"
                },
                "/dev/nvme3": {
                  "operation": "detach",
                  "namespaceId": "8",
                  "controllers": "0x3"
                },
                "/dev/nvme4": {
                  "destroy": true,
                  "namespaceId": "9",
                  "controllers": "0x4"
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowGrow": true,
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nvmenamespaces:/dev/nvme0:create"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "nvme",
                        "create-ns",
                        "/dev/nvme0",
                        "--nsze-si",
                        "100G",
                        "--ncap-si",
                        "100G",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "nvme",
                        "attach-ns",
                        "/dev/nvme0",
                        "--namespace-id",
                        "4",
                        "--controllers",
                        "0x1",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nvmenamespaces:/dev/nvme1:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["nvme", "list-subsys", "--output-format=json"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["nvme", "ns-rescan", "/dev/nvme1"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nvmenamespaces:/dev/nvme2:attach"
            && step.commands.iter().any(|command| {
                command.argv == ["nvme", "list-subsys", "--output-format=json"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "nvme",
                        "attach-ns",
                        "/dev/nvme2",
                        "--namespace-id",
                        "7",
                        "--controllers",
                        "0x2",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["nvme", "ns-rescan", "/dev/nvme2"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nvmenamespaces:/dev/nvme3:detach"
            && step.commands.iter().any(|command| {
                command.argv == ["nvme", "list-subsys", "--output-format=json"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "nvme",
                        "detach-ns",
                        "/dev/nvme3",
                        "--namespace-id",
                        "8",
                        "--controllers",
                        "0x3",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["nvme", "ns-rescan", "/dev/nvme3"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nvmenamespaces:/dev/nvme4:destroy"
            && step.commands.iter().any(|command| {
                command.argv == ["nvme", "list-subsys", "--output-format=json"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "nvme",
                        "detach-ns",
                        "/dev/nvme4",
                        "--namespace-id",
                        "9",
                        "--controllers",
                        "0x4",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["nvme", "delete-ns", "/dev/nvme4", "--namespace-id", "9"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "nvmenamespaces:/dev/nvme0:create"
            && step.commands.iter().any(|command| {
                command.argv == ["nvme", "list", "--output-format=json"] && !command.mutates
            })
    }));
}

#[test]
fn nvme_namespace_lifecycle_requires_explicit_namespace_inputs() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "nvmeNamespaces": {
                "logical-ns": {
                  "operation": "create"
                }
              },
              "apply": {
                "allowDestructive": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    let step = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "nvmenamespaces:logical-ns:create")
        .expect("NVMe namespace create command plan exists");
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "nvme",
                "create-ns",
                "<nvme-controller>",
                "--nsze-si",
                "<size>",
                "--ncap-si",
                "<size>",
            ]
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command
                .unresolved_inputs
                .contains(&"NVMe controller path such as /dev/nvme0".to_string())
            && command
                .unresolved_inputs
                .contains(&"desired namespace size".to_string())
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "nvme",
                "attach-ns",
                "<nvme-controller>",
                "--namespace-id",
                "<namespace-id>",
                "--controllers",
                "<controller-id-list>",
            ]
            && command.readiness == CommandReadiness::NeedsDomainImplementation
    }));
}

#[test]
fn nvme_namespace_lifecycle_accepts_controller_path_aliases() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "nvmeNamespaces": {
                  "logical-create": {
                    "operation": "create",
                    "path": "/dev/nvme0",
                    "desiredSize": "100G",
                    "namespaceId": "4",
                    "controllers": "0x1"
                  },
                  "logical-grow": {
                    "operation": "grow",
                    "device": "/dev/nvme1"
                  },
                  "logical-attach": {
                    "operation": "attach",
                    "target": "/dev/nvme2",
                    "device": "/dev/nvme2n1",
                    "namespaceId": "7",
                    "controllers": "0x2"
                  },
                  "logical-detach": {
                    "operation": "detach",
                    "target": "/dev/nvme3",
                    "device": "/dev/nvme3n1",
                    "namespaceId": "8",
                    "controllers": "0x3"
                  },
                  "logical-destroy": {
                    "destroy": true,
                    "target": "/dev/nvme4",
                    "namespaceId": "9",
                    "controllers": "0x4"
                  }
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowGrow": true,
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nvmenamespaces:logical-create:create"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "nvme",
                        "create-ns",
                        "/dev/nvme0",
                        "--nsze-si",
                        "100G",
                        "--ncap-si",
                        "100G",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nvmenamespaces:logical-grow:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["nvme", "ns-rescan", "/dev/nvme1"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nvmenamespaces:logical-attach:attach"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "nvme",
                        "attach-ns",
                        "/dev/nvme2",
                        "--namespace-id",
                        "7",
                        "--controllers",
                        "0x2",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nvmenamespaces:logical-detach:detach"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "nvme",
                        "detach-ns",
                        "/dev/nvme3",
                        "--namespace-id",
                        "8",
                        "--controllers",
                        "0x3",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nvmenamespaces:logical-destroy:destroy"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "nvme",
                        "detach-ns",
                        "/dev/nvme4",
                        "--namespace-id",
                        "9",
                        "--controllers",
                        "0x4",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
}

#[test]
fn lun_attach_and_grow_without_stable_path_reports_unresolved_input() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "luns": {
                "iqn.2026-06.example:storage/new:0": {
                  "operation": "create"
                },
                "iqn.2026-06.example:storage/grow:1": {
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

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "luns:iqn.2026-06.example:storage/new:0:create"
            && step.commands.iter().any(|command| {
                command.argv == ["<scsi-rescan-device>", "<lun-path>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["stable LUN device path"]
            })
            && step.commands.iter().any(|command| {
                command.argv == ["blockdev", "--getsize64", "<lun-path>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["stable LUN device path"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "luns:iqn.2026-06.example:storage/grow:1:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["<scsi-rescan-device>", "<lun-path>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["stable LUN device path"]
            })
    }));
    assert_eq!(report.command_summary.needs_domain_implementation_count, 3);
    assert!(!report.command_summary.all_commands_ready());
}
