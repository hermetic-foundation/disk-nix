#[test]
fn execute_refuses_missing_required_tools_before_running_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "exports": {
                "/srv/share": {
                  "operation": "create",
                  "client": "192.0.2.0/24",
                  "options": "ro,sync"
                }
              }
            }"#,
    )
    .expect("document parses");

    let mut ran_commands = false;
    let report = prepare_execution_with_runner_and_tool_checker(
        &plan,
        policy,
        ExecutionMode::Execute,
        |_| {
            ran_commands = true;
            CommandRunResult {
                success: true,
                status_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            }
        },
        |tool| tool != "exportfs",
    );

    assert_eq!(report.status, ExecutionStatus::NotReady);
    assert!(!ran_commands);
    assert!(report.execution_results.is_empty());
    assert!(report.messages.iter().any(|message| {
        message.contains("required tool(s) are not available") && message.contains("exportfs")
    }));
    let exportfs_requirement = report
        .tool_requirements
        .iter()
        .find(|requirement| requirement.tool == "exportfs")
        .expect("exportfs tool requirement is reported");
    assert_eq!(exportfs_requirement.availability, ToolAvailability::Missing);
    assert!(exportfs_requirement.message.contains("missing"));
    assert!(exportfs_requirement
        .remediation
        .iter()
        .any(|hint| hint.contains("pkgs.nfs-utils")));
    assert!(exportfs_requirement
        .remediation
        .iter()
        .any(|hint| hint.contains("services.disk-nix.toolPackages")));
    assert!(report
        .recovery_actions
        .iter()
        .any(|action| { action.kind == RecoveryActionKind::ResolveInputs }));
}

#[test]
fn tool_requirements_map_multipathd_to_multipath_tools() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "multipathMaps": {
                "mpatha": {
                  "target": "mpatha",
                  "addDevices": ["/dev/sdb"]
                }
              }
            }"#,
    )
    .expect("document parses");

    let mut ran_commands = false;
    let report = prepare_execution_with_runner_and_tool_checker(
        &plan,
        policy,
        ExecutionMode::Execute,
        |_| {
            ran_commands = true;
            CommandRunResult {
                success: true,
                status_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            }
        },
        |tool| tool != "multipathd",
    );

    assert_eq!(report.status, ExecutionStatus::NotReady);
    assert!(!ran_commands);
    assert!(report.messages.iter().any(|message| {
        message.contains("required tool(s) are not available") && message.contains("multipathd")
    }));
    let multipathd_requirement = report
        .tool_requirements
        .iter()
        .find(|requirement| requirement.tool == "multipathd")
        .expect("multipathd tool requirement is reported");
    assert_eq!(
        multipathd_requirement.availability,
        ToolAvailability::Missing
    );
    assert_eq!(multipathd_requirement.command_count, 1);
    assert_eq!(multipathd_requirement.mutating_count, 1);
    assert!(multipathd_requirement
        .remediation
        .iter()
        .any(|hint| hint.contains("pkgs.multipath-tools")));
    assert!(multipathd_requirement
        .remediation
        .iter()
        .any(|hint| hint.contains("services.disk-nix.toolPackages")));
}

#[test]
fn tool_requirements_map_shell_wrappers_to_bash() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "swaps": {
                  "primary": {
                    "device": "/dev/disk/by-label/swap-old",
                    "properties": {
                      "priority": "10"
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

    let mut ran_commands = false;
    let report = prepare_execution_with_runner_and_tool_checker(
        &plan,
        policy,
        ExecutionMode::Execute,
        |_| {
            ran_commands = true;
            CommandRunResult {
                success: true,
                status_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            }
        },
        |tool| tool != "sh",
    );

    assert_eq!(report.status, ExecutionStatus::NotReady);
    assert!(!ran_commands);
    assert!(report.messages.iter().any(|message| {
        message.contains("required tool(s) are not available") && message.contains("sh")
    }));
    let shell_requirement = report
        .tool_requirements
        .iter()
        .find(|requirement| requirement.tool == "sh")
        .expect("sh tool requirement is reported");
    assert_eq!(shell_requirement.availability, ToolAvailability::Missing);
    assert_eq!(shell_requirement.command_count, 1);
    assert_eq!(shell_requirement.mutating_count, 1);
    assert!(shell_requirement
        .remediation
        .iter()
        .any(|hint| hint.contains("pkgs.bash")));
    assert!(shell_requirement
        .remediation
        .iter()
        .any(|hint| hint.contains("services.disk-nix.toolPackages")));
}

#[test]
fn tool_requirements_map_coreutils_commands_to_coreutils() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "backingFiles": {
                  "/var/lib/images/root.img": {
                    "operation": "grow",
                    "desiredSize": "16GiB"
                  }
                }
              },
              "apply": {
                "allowGrow": true
              }
            }"#,
    )
    .expect("document parses");

    let mut ran_commands = false;
    let report = prepare_execution_with_runner_and_tool_checker(
        &plan,
        policy,
        ExecutionMode::Execute,
        |_| {
            ran_commands = true;
            CommandRunResult {
                success: true,
                status_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            }
        },
        |tool| tool != "stat",
    );

    assert_eq!(report.status, ExecutionStatus::NotReady);
    assert!(!ran_commands);
    assert!(report.messages.iter().any(|message| {
        message.contains("required tool(s) are not available") && message.contains("stat")
    }));
    let stat_requirement = report
        .tool_requirements
        .iter()
        .find(|requirement| requirement.tool == "stat")
        .expect("stat tool requirement is reported");
    assert_eq!(stat_requirement.availability, ToolAvailability::Missing);
    assert_eq!(stat_requirement.command_count, 2);
    assert_eq!(stat_requirement.mutating_count, 0);
    assert!(stat_requirement
        .remediation
        .iter()
        .any(|hint| hint.contains("pkgs.coreutils")));
    assert!(stat_requirement
        .remediation
        .iter()
        .any(|hint| hint.contains("services.disk-nix.toolPackages")));
}

#[test]
fn tool_requirements_map_util_linux_storage_helpers() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "scratch": {
                    "mountpoint": "/scratch",
                    "fsType": "xfs",
                    "operation": "trim"
                  }
                },
                "swaps": {
                  "primary": {
                    "device": "/dev/disk/by-label/swap",
                    "preserveData": false
                  },
                  "scratch": {
                    "device": "/swapfile",
                    "operation": "grow",
                    "desiredSize": "16GiB"
                  }
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowFormat": true,
                "allowOffline": true,
                "allowGrow": true
              }
            }"#,
    )
    .expect("document parses");

    let mut ran_commands = false;
    let report = prepare_execution_with_runner_and_tool_checker(
        &plan,
        policy,
        ExecutionMode::Execute,
        |_| {
            ran_commands = true;
            CommandRunResult {
                success: true,
                status_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            }
        },
        |tool| !matches!(tool, "fallocate" | "fstrim" | "mkswap"),
    );

    assert_eq!(report.status, ExecutionStatus::NotReady);
    assert!(!ran_commands);
    for tool in ["fallocate", "fstrim", "mkswap"] {
        assert!(report.messages.iter().any(|message| {
            message.contains("required tool(s) are not available") && message.contains(tool)
        }));
        let requirement = report
            .tool_requirements
            .iter()
            .find(|requirement| requirement.tool == tool)
            .unwrap_or_else(|| panic!("{tool} tool requirement is reported"));
        assert_eq!(requirement.availability, ToolAvailability::Missing);
        assert!(
            requirement
                .remediation
                .iter()
                .any(|hint| hint.contains("pkgs.util-linux")),
            "{tool} should suggest pkgs.util-linux"
        );
        assert!(
            requirement
                .remediation
                .iter()
                .any(|hint| hint.contains("services.disk-nix.toolPackages")),
            "{tool} should include the NixOS module toolPackages hint"
        );
    }
}

#[test]
fn tool_requirements_map_inventory_and_lvm_helpers() {
    for (tool, package) in [
        ("btrfstune", "btrfs-progs"),
        ("growpart", "cloud-utils"),
        ("lsblk", "util-linux"),
        ("mkfs", "util-linux"),
        ("pvmove", "lvm2"),
        ("vgchange", "lvm2"),
        ("vgexport", "lvm2"),
        ("vgimport", "lvm2"),
        ("vgreduce", "lvm2"),
        ("vgrename", "lvm2"),
        ("vgscan", "lvm2"),
    ] {
        assert_eq!(nix_package_for_tool(tool), Some(package));
        assert!(
            disk_nix_default_tool_package(package),
            "{package} should be recognized as a NixOS module default tool package"
        );
    }
}

#[test]
fn execute_stops_after_first_failed_command() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "exports": {
                "/srv/share": {
                  "operation": "create",
                  "client": "192.0.2.0/24",
                  "options": "ro,sync"
                }
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |_argv| {
        CommandRunResult {
            success: false,
            status_code: Some(32),
            stdout: String::new(),
            stderr: "export failed".to_string(),
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert_eq!(report.execution_results.len(), 1);
    assert_eq!(report.execution_results[0].status_code, Some(32));
    assert_eq!(report.execution_results[0].stderr, "export failed");
    assert!(report.recovery_actions.iter().any(|action| {
        action.kind == RecoveryActionKind::ReviewExecutionFailure
            && action
                .notes
                .iter()
                .any(|note| note.contains("export failed"))
    }));
    assert!(report.recovery_actions.iter().any(|action| {
        action.kind == RecoveryActionKind::InspectCurrentState
            && action.commands.iter().any(|command| {
                command.argv == ["disk-nix", "probe-status", "--json"] && !command.mutates
            })
    }));
    assert!(report
        .recovery_actions
        .iter()
        .any(|action| { action.kind == RecoveryActionKind::PreserveRecoveryPoints }));
}

#[test]
fn failed_snapshot_rollback_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "snapshots": {
                "tank/home@before": {
                  "target": "tank/home",
                  "rollback": true
                }
              },
              "apply": {
                "allowPotentialDataLoss": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != ["zfs", "rollback", "tank/home@before"],
            status_code: Some(if argv == ["zfs", "rollback", "tank/home@before"] {
                1
            } else {
                0
            }),
            stdout: String::new(),
            stderr: if argv == ["zfs", "rollback", "tank/home@before"] {
                "rollback failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report.execution_results.iter().any(|result| {
        !result.success && result.argv == ["zfs", "rollback", "tank/home@before"]
    }));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Rollback"));
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
        command.argv == ["zfs", "list", "-H", "-p", "tank/home"] && !command.mutates
    }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| note.contains("do not retry")));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| note.contains("cloning the snapshot")));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("roll-forward recovery review is reported");
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
            && command.unresolved_inputs == ["original spec path"]
    }));
    assert!(roll_forward.commands.iter().any(|command| {
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
    assert!(roll_forward
        .notes
        .iter()
        .any(|note| note.contains("fresh topology")));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("rollback recovery review is reported");
    assert!(rollback.commands.iter().any(|command| {
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
    assert!(rollback
        .notes
        .iter()
        .any(|note| note.contains("read-only checks")));
    assert_eq!(report.rollback_recipes.len(), 1);
    let recipe = &report.rollback_recipes[0];
    assert_eq!(recipe.recipe_version, 1);
    assert_eq!(
        recipe.source_action_id,
        "snapshot:tank/home@before:rollback"
    );
    assert_eq!(
        recipe.failed_command,
        ["zfs", "rollback", "tank/home@before"]
    );
    assert_eq!(recipe.status, RollbackRecipeStatus::Refused);
    assert!(recipe.receipt_binding_required);
    assert!(recipe.fresh_topology_probe_required);
    assert!(!recipe.read_only_validation.commands.is_empty());
    assert!(recipe
        .read_only_validation
        .commands
        .iter()
        .all(|command| !command.mutates));
    assert!(recipe.reversible_mutations.commands.is_empty());
    assert!(recipe.destructive_mutations.commands.is_empty());
    assert!(recipe
        .operator_only_handoff
        .notes
        .iter()
        .any(|note| note.contains("operator review")));
    assert!(recipe
        .safety_gates
        .iter()
        .any(|gate| gate.contains("original apply receipt")));
    for expected_gate in [
        "filesystem rollback gates",
        "block-stack rollback gates",
        "advanced-storage rollback gates",
        "network-storage rollback gates",
    ] {
        assert!(
            recipe
                .safety_gates
                .iter()
                .any(|gate| gate.contains(expected_gate)),
            "{expected_gate} should be emitted in rollback recipe safety gates"
        );
    }
    assert!(recipe
        .refusal_reasons
        .iter()
        .any(|reason| reason.contains("snapshot rollback is refused")));
    let value = serde_json::to_value(&report).expect("report should serialize");
    assert_eq!(
        value["rollbackRecipes"][0]["readOnlyValidation"]["commands"][0]["mutates"],
        false
    );
    assert_eq!(
        value["rollbackRecipes"][0]["reversibleMutations"]["commands"]
            .as_array()
            .expect("reversible mutation command section is an array")
            .len(),
        0
    );
}

fn failed_report_for_rollback_replay() -> ExecutionReport {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "snapshots": {
                "tank/home@before": {
                  "target": "tank/home",
                  "rollback": true
                }
              },
              "apply": {
                "allowPotentialDataLoss": true
              }
            }"#,
    )
    .expect("document parses");

    prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != ["zfs", "rollback", "tank/home@before"],
            status_code: Some(if argv == ["zfs", "rollback", "tank/home@before"] {
                1
            } else {
                0
            }),
            stdout: String::new(),
            stderr: if argv == ["zfs", "rollback", "tank/home@before"] {
                "rollback failed".to_string()
            } else {
                String::new()
            },
        }
    })
}

fn rollback_replay_command(argv: &[&str], mutates: bool) -> ExecutionCommand {
    ExecutionCommand {
        argv: argv.iter().map(|part| (*part).to_string()).collect(),
        mutates,
        readiness: CommandReadiness::Ready,
        unresolved_inputs: Vec::new(),
        provider_capabilities: Vec::new(),
        note: "test rollback replay command".to_string(),
    }
}

fn complete_rollback_topology_evidence() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("expected".to_string(), "topology:expected-123".to_string()),
        ("preApply".to_string(), "topology:pre-apply-123".to_string()),
        (
            "failedApply".to_string(),
            "topology:failed-apply-123".to_string(),
        ),
        ("current".to_string(), "topology:fresh-456".to_string()),
    ])
}

fn clean_topology_comparison() -> TopologyComparison {
    TopologyComparison {
        summary: disk_nix_plan::TopologyComparisonSummary {
            action_count: 1,
            matched_count: 1,
            missing_count: 0,
            size_diagnostic_count: 0,
            type_conflict_count: 0,
            already_satisfied_count: 0,
            suppressed_action_count: 0,
            graph_dependency_edge_count: 0,
            graph_dependency_conflict_count: 0,
            reconciliation_group_count: 0,
            partially_suppressed_group_count: 0,
            lifecycle_group_count: 0,
            graph_derived_lifecycle_group_count: 0,
        },
        diagnostics: Vec::new(),
        reconciliation_groups: Vec::new(),
        lifecycle_groups: Vec::new(),
        graph_dependency_conflict_resolutions: Vec::new(),
    }
}

fn rollback_topology_diagnostic(
    action_id: &str,
    kind: disk_nix_plan::TopologyDiagnosticKind,
) -> disk_nix_plan::TopologyDiagnostic {
    disk_nix_plan::TopologyDiagnostic {
        action_id: action_id.to_string(),
        level: disk_nix_plan::TopologyDiagnosticLevel::Warning,
        kind,
        query: "test topology query".to_string(),
        message: "test topology diagnostic".to_string(),
        current: None,
    }
}

fn proven_safe_rollback_recipe() -> RollbackRecipe {
    RollbackRecipe {
        recipe_version: 1,
        source_action_id: "snapshot:tank/home@before:rollback".to_string(),
        failed_command: vec![
            "zfs".to_string(),
            "rollback".to_string(),
            "tank/home@before".to_string(),
        ],
        status: RollbackRecipeStatus::ProvenSafe,
        receipt_binding_required: true,
        fresh_topology_probe_required: true,
        read_only_validation: RollbackRecipeSection {
            commands: vec![rollback_replay_command(
                &["disk-nix-test-probe", "topology"],
                false,
            )],
            notes: vec!["validate current topology before mutation".to_string()],
        },
        reversible_mutations: RollbackRecipeSection {
            commands: vec![rollback_replay_command(
                &["disk-nix-test-rollback", "restore"],
                true,
            )],
            notes: vec!["restore the recorded rollback point".to_string()],
        },
        destructive_mutations: RollbackRecipeSection {
            commands: Vec::new(),
            notes: Vec::new(),
        },
        operator_only_handoff: RollbackRecipeSection {
            commands: Vec::new(),
            notes: Vec::new(),
        },
        safety_gates: vec![
            "original apply receipt must match this failed apply report".to_string(),
            "fresh topology probe must be captured after the failure".to_string(),
        ],
        required_topology_evidence: vec![
            "expected".to_string(),
            "preApply".to_string(),
            "failedApply".to_string(),
            "current".to_string(),
        ],
        refusal_reasons: Vec::new(),
        notes: Vec::new(),
    }
}

#[test]
fn rollback_replay_runs_only_proven_safe_reversible_steps_with_receipt_binding() {
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
                success: true,
                status_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            }
        },
    );

    assert_eq!(replay.status, RollbackExecutionStatus::Succeeded);
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0], ["disk-nix-test-probe", "topology"]);
    assert_eq!(calls[1], ["disk-nix-test-rollback", "restore"]);
    assert_eq!(replay.validation_results.len(), 1);
    assert_eq!(
        replay.validation_results[0].phase,
        ExecutionPhase::Verification
    );
    assert_eq!(replay.rollback_results.len(), 1);
    assert_eq!(replay.rollback_results[0].phase, ExecutionPhase::Command);
    assert_eq!(
        replay.receipt_binding.original_receipt_id,
        "receipt:apply-123"
    );
    assert_eq!(
        replay.receipt_binding.fresh_topology_probe_id,
        "topology:fresh-456"
    );
    assert_eq!(
        replay.receipt_binding.topology_evidence,
        complete_rollback_topology_evidence()
    );
    assert_eq!(
        replay.receipt_binding.failed_command,
        ["zfs", "rollback", "tank/home@before"]
    );

    let value = serde_json::to_value(&replay).expect("rollback replay report serializes");
    assert_eq!(value["status"], "succeeded");
    assert_eq!(
        value["receiptBinding"]["originalReceiptId"],
        "receipt:apply-123"
    );
    assert_eq!(
        value["receiptBinding"]["freshTopologyProbeId"],
        "topology:fresh-456"
    );
    assert_eq!(
        value["receiptBinding"]["topologyEvidence"]["failedApply"],
        "topology:failed-apply-123"
    );
}

#[test]
fn rollback_topology_evidence_materializes_from_failed_report_and_fresh_probe() {
    let mut report = failed_report_for_rollback_replay();
    let topology_evidence = materialize_rollback_topology_evidence(&report, "topology:fresh-456");

    assert_eq!(
        topology_evidence.get("current").map(String::as_str),
        Some("topology:fresh-456")
    );
    for label in ["expected", "preApply", "failedApply"] {
        let evidence_id = topology_evidence
            .get(label)
            .unwrap_or_else(|| panic!("{label} evidence should exist"));
        assert!(
            evidence_id.starts_with("topology:"),
            "{label} evidence should be a topology evidence id: {evidence_id}"
        );
    }

    let original_failed_apply = topology_evidence
        .get("failedApply")
        .expect("failed apply evidence exists")
        .clone();
    report.execution_results[0].stderr = "different failure".to_string();
    let changed_evidence = materialize_rollback_topology_evidence(&report, "topology:fresh-456");
    assert_ne!(
        changed_evidence.get("failedApply"),
        Some(&original_failed_apply)
    );
}

#[test]
fn rollback_replay_binds_full_topology_payloads_to_receipt() {
    let mut report = failed_report_for_rollback_replay();
    report.rollback_recipes = vec![proven_safe_rollback_recipe()];
    let current_payload = serde_json::json!({
        "nodes": [
            {
                "id": "node:current",
                "kind": "disk",
                "name": "current",
                "identity": {},
                "properties": []
            }
        ],
        "edges": []
    });
    let topology_payloads =
        materialize_rollback_topology_payloads(&report, current_payload.clone());
    let mut calls = Vec::new();

    let replay = replay_proven_safe_rollback_recipe_with_runner_and_tool_checker(
        &report,
        0,
        RollbackReplayBindings {
            original_receipt_id: "receipt:apply-123".to_string(),
            fresh_topology_probe_id: "topology:fresh-456".to_string(),
            topology_evidence: complete_rollback_topology_evidence(),
            topology_payloads,
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

    assert_eq!(replay.status, RollbackExecutionStatus::Succeeded);
    assert_eq!(calls.len(), 2);
    assert_eq!(
        replay.receipt_binding.topology_payloads.get("current"),
        Some(&current_payload)
    );
    for label in ["expected", "preApply", "failedApply", "current"] {
        assert!(
            replay.receipt_binding.topology_payloads.contains_key(label),
            "{label} topology payload should be bound to rollback receipt"
        );
    }

    let value = serde_json::to_value(&replay).expect("rollback replay report serializes");
    assert_eq!(
        value["receiptBinding"]["topologyPayloads"]["current"]["nodes"][0]["id"],
        "node:current"
    );
}
