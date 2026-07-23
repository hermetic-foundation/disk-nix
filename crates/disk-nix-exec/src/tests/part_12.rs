#[test]
fn rollback_replay_refuses_review_only_recipes_without_running_commands() {
    let report = failed_report_for_rollback_replay();
    let mut calls = Vec::new();

    let replay = replay_proven_safe_rollback_recipe_with_runner(
        &report,
        0,
        "receipt:apply-123".to_string(),
        "topology:fresh-456".to_string(),
        &mut |argv| {
            calls.push(argv.to_vec());
            CommandRunResult {
                success: true,
                status_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            }
        },
    );

    assert_eq!(replay.status, RollbackExecutionStatus::Refused);
    assert!(calls.is_empty());
    assert!(replay.validation_results.is_empty());
    assert!(replay.rollback_results.is_empty());
    assert!(replay
        .refusal_reasons
        .iter()
        .any(|reason| reason.contains("not marked proven-safe")));
}

#[test]
fn rollback_replay_requires_fresh_topology_binding_before_running_commands() {
    let mut report = failed_report_for_rollback_replay();
    report.rollback_recipes = vec![proven_safe_rollback_recipe()];
    let mut calls = Vec::new();

    let replay = replay_proven_safe_rollback_recipe_with_runner(
        &report,
        0,
        "receipt:apply-123".to_string(),
        String::new(),
        &mut |argv| {
            calls.push(argv.to_vec());
            CommandRunResult {
                success: true,
                status_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            }
        },
    );

    assert_eq!(replay.status, RollbackExecutionStatus::Refused);
    assert!(calls.is_empty());
    assert!(replay
        .refusal_reasons
        .iter()
        .any(|reason| reason.contains("fresh post-failure topology probe binding")));
}

#[test]
fn rollback_replay_requires_original_receipt_binding_before_running_commands() {
    let mut report = failed_report_for_rollback_replay();
    report.rollback_recipes = vec![proven_safe_rollback_recipe()];
    let mut calls = Vec::new();

    let replay = replay_proven_safe_rollback_recipe_with_runner(
        &report,
        0,
        String::new(),
        "topology:fresh-456".to_string(),
        &mut |argv| {
            calls.push(argv.to_vec());
            CommandRunResult {
                success: true,
                status_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            }
        },
    );

    assert_eq!(replay.status, RollbackExecutionStatus::Refused);
    assert!(calls.is_empty());
    assert!(replay.validation_results.is_empty());
    assert!(replay.rollback_results.is_empty());
    assert!(replay
        .refusal_reasons
        .iter()
        .any(|reason| reason.contains("original apply receipt binding")));
}

#[test]
fn rollback_replay_allows_clean_topology_comparison() {
    let mut report = failed_report_for_rollback_replay();
    report.rollback_recipes = vec![proven_safe_rollback_recipe()];
    report.topology_comparison = Some(clean_topology_comparison());
    let mut calls = Vec::new();

    let replay = replay_proven_safe_rollback_recipe_with_runner(
        &report,
        0,
        "receipt:apply-123".to_string(),
        "topology:fresh-456".to_string(),
        &mut |argv| {
            calls.push(argv.to_vec());
            CommandRunResult {
                success: true,
                status_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            }
        },
    );

    assert_eq!(replay.status, RollbackExecutionStatus::Succeeded);
    assert_eq!(calls.len(), 2);
}

#[test]
fn rollback_replay_refuses_divergent_topology_comparison_before_running_commands() {
    let mut report = failed_report_for_rollback_replay();
    report.rollback_recipes = vec![proven_safe_rollback_recipe()];
    let mut comparison = clean_topology_comparison();
    comparison.summary.missing_count = 1;
    comparison.summary.size_diagnostic_count = 1;
    comparison.summary.type_conflict_count = 1;
    comparison.summary.graph_dependency_conflict_count = 1;
    comparison.summary.partially_suppressed_group_count = 1;
    report.topology_comparison = Some(comparison);
    let mut calls = Vec::new();

    let replay = replay_proven_safe_rollback_recipe_with_runner(
        &report,
        0,
        "receipt:apply-123".to_string(),
        "topology:fresh-456".to_string(),
        &mut |argv| {
            calls.push(argv.to_vec());
            CommandRunResult {
                success: true,
                status_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            }
        },
    );

    assert_eq!(replay.status, RollbackExecutionStatus::Refused);
    assert!(calls.is_empty());
    assert!(replay.validation_results.is_empty());
    assert!(replay.rollback_results.is_empty());
    assert!(replay.refusal_reasons.iter().any(|reason| {
        reason.contains("divergent topology comparison")
            && reason.contains("1 missing target")
            && reason.contains("1 size diagnostic")
            && reason.contains("1 type conflict")
            && reason.contains("1 graph dependency conflict")
            && reason.contains("1 partially suppressed reconciliation group")
    }));
}

#[test]
fn rollback_replay_refuses_risky_topology_diagnostics_before_running_commands() {
    let cases = [
        (
            "topology-live-use-mount",
            disk_nix_plan::TopologyDiagnosticKind::UnmountRequired,
            "topology diagnostic live-use blocker",
        ),
        (
            "topology-stale-rollback-point",
            disk_nix_plan::TopologyDiagnosticKind::SnapshotRollbackPointMissing,
            "topology diagnostic stale identity or ambiguous rollback point",
        ),
        (
            "topology-already-rolled-back",
            disk_nix_plan::TopologyDiagnosticKind::SnapshotRollbackPointAvailable,
            "topology diagnostic rollback idempotency blocker",
        ),
        (
            "topology-data-loss-destroy",
            disk_nix_plan::TopologyDiagnosticKind::ZfsObjectDestroyRequired,
            "topology diagnostic plausible data-loss path",
        ),
    ];

    for (action_id, diagnostic_kind, expected_reason) in cases {
        let mut report = failed_report_for_rollback_replay();
        report.rollback_recipes = vec![proven_safe_rollback_recipe()];
        let mut comparison = clean_topology_comparison();
        comparison
            .diagnostics
            .push(rollback_topology_diagnostic(action_id, diagnostic_kind));
        report.topology_comparison = Some(comparison);
        let mut calls = Vec::new();

        let replay = replay_proven_safe_rollback_recipe_with_runner(
            &report,
            0,
            "receipt:apply-123".to_string(),
            "topology:fresh-456".to_string(),
            &mut |argv| {
                calls.push(argv.to_vec());
                CommandRunResult {
                    success: true,
                    status_code: Some(0),
                    stdout: String::new(),
                    stderr: String::new(),
                }
            },
        );

        assert_eq!(replay.status, RollbackExecutionStatus::Refused);
        assert!(calls.is_empty());
        assert!(replay.validation_results.is_empty());
        assert!(replay.rollback_results.is_empty());
        assert!(replay.refusal_reasons.iter().any(|reason| {
            reason.contains("divergent topology comparison")
                && reason.contains(expected_reason)
                && reason.contains(action_id)
        }));
    }
}

#[test]
fn rollback_replay_refuses_missing_required_topology_evidence_before_running_commands() {
    let mut report = failed_report_for_rollback_replay();
    report.rollback_recipes = vec![proven_safe_rollback_recipe()];
    let mut topology_evidence = complete_rollback_topology_evidence();
    topology_evidence.remove("preApply");
    let mut calls = Vec::new();

    let replay = replay_proven_safe_rollback_recipe_with_runner_and_tool_checker(
        &report,
        0,
        RollbackReplayBindings {
            original_receipt_id: "receipt:apply-123".to_string(),
            fresh_topology_probe_id: "topology:fresh-456".to_string(),
            topology_evidence,
            topology_payloads: BTreeMap::new(),
        },
        &mut |argv| {
            calls.push(argv.to_vec());
            CommandRunResult {
                success: true,
                status_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            }
        },
        |_| true,
    );

    assert_eq!(replay.status, RollbackExecutionStatus::Refused);
    assert!(calls.is_empty());
    assert!(replay.validation_results.is_empty());
    assert!(replay.rollback_results.is_empty());
    assert!(replay
        .refusal_reasons
        .iter()
        .any(|reason| reason.contains("missing topology evidence binding(s): preApply")));
}

#[test]
fn rollback_replay_refuses_missing_tools_before_running_commands() {
    let mut report = failed_report_for_rollback_replay();
    report.rollback_recipes = vec![proven_safe_rollback_recipe()];
    let mut calls = Vec::new();

    let replay = replay_proven_safe_rollback_recipe_with_runner_and_tool_checker(
        &report,
        0,
        RollbackReplayBindings {
            original_receipt_id: "receipt:apply-123".to_string(),
            fresh_topology_probe_id: "topology:fresh-456".to_string(),
            topology_evidence: complete_rollback_topology_evidence(),
            topology_payloads: BTreeMap::new(),
        },
        &mut |argv| {
            calls.push(argv.to_vec());
            CommandRunResult {
                success: true,
                status_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            }
        },
        |tool| tool != "disk-nix-test-rollback",
    );

    assert_eq!(replay.status, RollbackExecutionStatus::Refused);
    assert!(calls.is_empty());
    assert!(replay.validation_results.is_empty());
    assert!(replay.rollback_results.is_empty());
    assert!(replay
        .refusal_reasons
        .iter()
        .any(|reason| reason.contains("missing required tool(s): disk-nix-test-rollback")));
}

#[test]
fn rollback_replay_stops_before_mutation_when_validation_fails() {
    let mut report = failed_report_for_rollback_replay();
    report.rollback_recipes = vec![proven_safe_rollback_recipe()];
    let mut calls = Vec::new();

    let replay = replay_proven_safe_rollback_recipe_with_runner(
        &report,
        0,
        "receipt:apply-123".to_string(),
        "topology:fresh-456".to_string(),
        &mut |argv| {
            calls.push(argv.to_vec());
            CommandRunResult {
                success: argv != ["disk-nix-test-probe", "topology"],
                status_code: Some(if argv == ["disk-nix-test-probe", "topology"] {
                    1
                } else {
                    0
                }),
                stdout: String::new(),
                stderr: if argv == ["disk-nix-test-probe", "topology"] {
                    "topology changed".to_string()
                } else {
                    String::new()
                },
            }
        },
    );

    assert_eq!(replay.status, RollbackExecutionStatus::Failed);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], ["disk-nix-test-probe", "topology"]);
    assert_eq!(replay.validation_results.len(), 1);
    assert!(replay.rollback_results.is_empty());
}

#[test]
fn rollback_replay_refuses_unsafe_sections_and_not_ready_commands() {
    let cases = [
        (
            "destructive",
            {
                let mut recipe = proven_safe_rollback_recipe();
                recipe.destructive_mutations.commands = vec![rollback_replay_command(
                    &["disk-nix-test-destroy", "rollback-point"],
                    true,
                )];
                recipe
            },
            "destructive mutation steps",
        ),
        (
            "operator-only",
            {
                let mut recipe = proven_safe_rollback_recipe();
                recipe.operator_only_handoff.commands = vec![rollback_replay_command(
                    &["disk-nix-test-operator", "handoff"],
                    false,
                )];
                recipe
            },
            "operator-only handoff steps",
        ),
        (
            "validation-not-ready",
            {
                let mut recipe = proven_safe_rollback_recipe();
                recipe.read_only_validation.commands[0].readiness = CommandReadiness::ManualOnly;
                recipe
            },
            "read-only validation command is not ready",
        ),
        (
            "rollback-not-ready",
            {
                let mut recipe = proven_safe_rollback_recipe();
                recipe.reversible_mutations.commands[0].readiness =
                    CommandReadiness::NeedsDomainImplementation;
                recipe
            },
            "reversible rollback command is not ready",
        ),
        (
            "data-loss-argv",
            {
                let mut recipe = proven_safe_rollback_recipe();
                recipe.reversible_mutations.commands = vec![rollback_replay_command(
                    &["zfs", "rollback", "tank/home@before"],
                    true,
                )];
                recipe
            },
            "plausible data-loss command",
        ),
        (
            "data-loss-metadata",
            {
                let mut recipe = proven_safe_rollback_recipe();
                recipe.reversible_mutations.commands[0]
                    .provider_capabilities
                    .push("risk.potential-data-loss".to_string());
                recipe
            },
            "plausible data-loss command metadata",
        ),
        (
            "live-use-blocker-metadata",
            {
                let mut recipe = proven_safe_rollback_recipe();
                recipe.reversible_mutations.commands[0]
                    .provider_capabilities
                    .push("rollback.blocker.active-consumers".to_string());
                recipe.reversible_mutations.commands[0]
                    .unresolved_inputs
                    .push("mounted filesystem state".to_string());
                recipe
            },
            "live-use blocker metadata",
        ),
        (
            "ambiguous-stale-identity-metadata",
            {
                let mut recipe = proven_safe_rollback_recipe();
                recipe.reversible_mutations.commands[0]
                    .provider_capabilities
                    .push("rollback.blocker.ambiguous rollback point".to_string());
                recipe.reversible_mutations.commands[0]
                    .unresolved_inputs
                    .push("stale identity data".to_string());
                recipe
            },
            "ambiguous or stale identity metadata",
        ),
        (
            "idempotency-already-rolled-back-metadata",
            {
                let mut recipe = proven_safe_rollback_recipe();
                recipe.reversible_mutations.commands[0]
                    .provider_capabilities
                    .push("rollback.state.already rolled back".to_string());
                recipe
            },
            "idempotency blocker metadata",
        ),
        (
            "idempotency-partially-rolled-back-metadata",
            {
                let mut recipe = proven_safe_rollback_recipe();
                recipe.reversible_mutations.commands[0]
                    .unresolved_inputs
                    .push("rollback partially applied".to_string());
                recipe
            },
            "idempotency blocker metadata",
        ),
        (
            "idempotency-externally-modified-metadata",
            {
                let mut recipe = proven_safe_rollback_recipe();
                recipe.reversible_mutations.commands[0].note =
                    "topology externally modified after failed apply".to_string();
                recipe
            },
            "idempotency blocker metadata",
        ),
    ];

    for (case_name, recipe, expected_reason) in cases {
        let mut report = failed_report_for_rollback_replay();
        report.rollback_recipes = vec![recipe];
        let mut calls = Vec::new();

        let replay = replay_proven_safe_rollback_recipe_with_runner(
            &report,
            0,
            "receipt:apply-123".to_string(),
            "topology:fresh-456".to_string(),
            &mut |argv| {
                calls.push(argv.to_vec());
                CommandRunResult {
                    success: true,
                    status_code: Some(0),
                    stdout: String::new(),
                    stderr: String::new(),
                }
            },
        );

        assert_eq!(
            replay.status,
            RollbackExecutionStatus::Refused,
            "{case_name} should be refused"
        );
        assert!(calls.is_empty(), "{case_name} should not run commands");
        assert!(
            replay
                .refusal_reasons
                .iter()
                .any(|reason| reason.contains(expected_reason)),
            "{case_name} should report {expected_reason}: {:?}",
            replay.refusal_reasons
        );
    }
}

#[test]
fn failed_zfs_snapshot_clone_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "snapshots": {
                "before-clone": {
                  "name": "tank/home@before",
                  "target": "tank/home",
                  "cloneTo": "tank/home-review"
                }
              }
            }"#,
    )
    .expect("document parses");

    let failed_clone = ["zfs", "clone", "tank/home@before", "tank/home-review"];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_clone,
            status_code: Some(if argv == failed_clone { 1 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_clone {
                "clone failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report
        .execution_results
        .iter()
        .any(|result| !result.success && result.argv == failed_clone));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("ZFS snapshot domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Clone"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv
            == [
                "zfs",
                "list",
                "-t",
                "snapshot",
                "-H",
                "-p",
                "tank/home@before",
            ]
            && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["zfs", "holds", "tank/home@before"] && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv
            == [
                "zfs",
                "list",
                "-t",
                "snapshot",
                "-H",
                "-p",
                "-o",
                "name,creation,used,referenced,userrefs",
                "-r",
                "tank/home",
            ]
            && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["disk-nix", "inspect", "tank/home@before", "--json"] && !command.mutates
    }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| { note.contains("snapshot lifecycle") && note.contains("hold tags") }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("ZFS snapshot rollback recovery review is reported");
    assert!(rollback.commands.iter().any(|command| {
        command.argv == ["zfs", "holds", "tank/home@before"] && !command.mutates
    }));
}

#[test]
fn failed_btrfs_snapshot_clone_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "snapshots": {
                "/mnt/persist/@home-before": {
                  "target": "/mnt/persist/@home",
                  "cloneTo": "/mnt/persist/@home-review",
                  "readOnly": true
                }
              }
            }"#,
    )
    .expect("document parses");

    let failed_clone = [
        "btrfs",
        "subvolume",
        "snapshot",
        "-r",
        "/mnt/persist/@home-before",
        "/mnt/persist/@home-review",
    ];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_clone,
            status_code: Some(if argv == failed_clone { 1 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_clone {
                "clone failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report
        .execution_results
        .iter()
        .any(|result| !result.success && result.argv == failed_clone));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("Btrfs snapshot domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Clone"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["btrfs", "subvolume", "show", "/mnt/persist/@home-before"]
            && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv
            == [
                "btrfs",
                "property",
                "get",
                "-ts",
                "/mnt/persist/@home-before",
                "ro",
            ]
            && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["disk-nix", "inspect", "/mnt/persist/@home-before", "--json"]
            && !command.mutates
    }));
}

#[test]
fn failed_md_member_replacement_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "mdRaids": {
                "root": {
                  "target": "/dev/md/root",
                  "replaceDevices": {
                    "/dev/disk/by-id/old-md-member": "/dev/disk/by-id/new-md-member"
                  }
                }
              },
              "apply": {
                "allowDeviceReplacement": true,
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_replace = [
        "mdadm",
        "/dev/md/root",
        "--replace",
        "/dev/disk/by-id/old-md-member",
        "--with",
        "/dev/disk/by-id/new-md-member",
    ];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_replace,
            status_code: Some(if argv == failed_replace { 16 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_replace {
                "replacement failed".to_string()
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
                    "mdadm",
                    "/dev/md/root",
                    "--replace",
                    "/dev/disk/by-id/old-md-member",
                    "--with",
                    "/dev/disk/by-id/new-md-member",
                ]
    }));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("MD domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("ReplaceDevice"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["mdadm", "--detail", "/dev/md/root"] && !command.mutates
    }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["cat", "/proc/mdstat"] && !command.mutates }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| { note.contains("MD RAID member changes") && note.contains("/proc/mdstat") }));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("MD roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv == ["mdadm", "--detail", "/dev/md/root"] && !command.mutates
    }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("MD rollback recovery review is reported");
    assert!(rollback
        .commands
        .iter()
        .any(|command| { command.argv == ["cat", "/proc/mdstat"] && !command.mutates }));
    assert!(report
        .recovery_actions
        .iter()
        .any(|action| action.kind == RecoveryActionKind::PreserveRecoveryPoints));
}

#[test]
fn failed_nvme_namespace_delete_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "nvmeNamespaces": {
                "logical-destroy": {
                  "target": "/dev/nvme4",
                  "destroy": true,
                  "namespaceId": "9",
                  "controllers": "0x4"
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_delete = ["nvme", "delete-ns", "/dev/nvme4", "--namespace-id", "9"];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_delete,
            status_code: Some(if argv == failed_delete { 16 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_delete {
                "namespace delete failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report.execution_results.iter().any(|result| {
        !result.success && result.argv == ["nvme", "delete-ns", "/dev/nvme4", "--namespace-id", "9"]
    }));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("NVMe namespace domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Destroy"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv
            == [
                "nvme",
                "list-ns",
                "/dev/nvme4",
                "--all",
                "--output-format=json",
            ]
            && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["nvme", "list-subsys", "--output-format=json"] && !command.mutates
    }));
    assert!(domain_recovery.notes.iter().any(|note| {
        note.contains("NVMe namespace changes")
            && note.contains("create, grow/rescan, attach, detach, or delete")
    }));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("NVMe roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv
            == [
                "nvme",
                "list-ns",
                "/dev/nvme4",
                "--all",
                "--output-format=json",
            ]
            && !command.mutates
    }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("NVMe rollback recovery review is reported");
    assert!(rollback.commands.iter().any(|command| {
        command.argv == ["nvme", "list-subsys", "--output-format=json"] && !command.mutates
    }));
}

#[test]
fn failed_nvme_namespace_grow_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "nvmeNamespaces": {
                "logical-grow": {
                  "target": "/dev/nvme1",
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

    let failed_rescan = ["nvme", "ns-rescan", "/dev/nvme1"];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_rescan,
            status_code: Some(if argv == failed_rescan { 84 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_rescan {
                "namespace grow rescan failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report
        .execution_results
        .iter()
        .any(|result| { !result.success && result.argv == ["nvme", "ns-rescan", "/dev/nvme1"] }));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("NVMe namespace grow domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Grow"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv
            == [
                "nvme",
                "list-ns",
                "/dev/nvme1",
                "--all",
                "--output-format=json",
            ]
            && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["nvme", "list-subsys", "--output-format=json"] && !command.mutates
    }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| { note.contains("NVMe namespace changes") && note.contains("grow/rescan") }));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("NVMe grow roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv
            == [
                "nvme",
                "list-ns",
                "/dev/nvme1",
                "--all",
                "--output-format=json",
            ]
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
        .expect("NVMe grow rollback recovery review is reported");
    assert!(rollback.commands.iter().all(|command| !command.mutates));
    assert!(rollback.commands.iter().any(|command| {
        command.argv == ["nvme", "list-subsys", "--output-format=json"] && !command.mutates
    }));
    assert!(report
        .recovery_actions
        .iter()
        .any(|action| action.kind == RecoveryActionKind::PreserveRecoveryPoints));
}
