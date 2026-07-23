#[test]
fn backing_file_property_reports_chmod_command() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "backingFiles": {
                  "/var/lib/images/root.img": {
                    "properties": {
                      "mode": "0600"
                    }
                  }
                }
              },
              "apply": {
                "allowPropertyChanges": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "backingFiles:/var/lib/images/root.img:set-property:mode"
            && step.commands.iter().any(|command| {
                command.argv == ["chmod", "0600", "/var/lib/images/root.img"]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
}

#[test]
fn backing_file_create_and_growth_require_path_and_size_for_execute_readiness() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "backingFiles": {
                  "new-image": {
                    "operation": "create"
                  },
                  "root-image": {
                    "operation": "grow"
                  }
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
    assert!(!report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "backingfiles:new-image:create"
            && step.commands.iter().any(|command| {
                command.argv == ["test", "!", "-e", "<backing-file>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["backing file path"]
            })
            && step.commands.iter().any(|command| {
                command.argv == ["truncate", "--size", "<size>", "<backing-file>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs
                        == ["backing file path", "desired backing file size"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "backingfiles:root-image:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["truncate", "--size", "<size>", "<backing-file>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs
                        == ["backing file path", "desired backing file size"]
            })
    }));
}

#[test]
fn dm_map_rescan_reports_dmsetup_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "dmMaps": {
                  "cryptroot": {
                    "operation": "rescan",
                    "target": "/dev/mapper/cryptroot"
                  }
                }
              },
              "apply": {}
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "dmmaps:cryptroot:rescan"
            && step.commands.iter().all(|command| !command.mutates)
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "dmsetup",
                        "info",
                        "-c",
                        "--noheadings",
                        "-o",
                        "name,uuid,major,minor,open,segments,events",
                        "/dev/mapper/cryptroot",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["dmsetup", "deps", "-o", "devname", "/dev/mapper/cryptroot"]
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["dmsetup", "table", "/dev/mapper/cryptroot"]
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["dmsetup", "status", "/dev/mapper/cryptroot"]
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "/dev/mapper/cryptroot"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "dmmaps:cryptroot:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "/dev/mapper/cryptroot", "--json"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
}

#[test]
fn dm_map_rescan_requires_concrete_mapper_path_for_execute_readiness() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "dmMaps": {
                  "cryptroot": {
                    "operation": "rescan"
                  }
                }
              },
              "apply": {}
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(!report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "dmmaps:cryptroot:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["dmsetup", "table", "<dm-map>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["device-mapper path"]
            })
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "<dm-map>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["device-mapper path"]
            })
    }));
}

#[test]
fn dm_map_rename_reports_dmsetup_command() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "dmMaps": {
                  "cryptswap": {
                    "operation": "rename",
                    "target": "/dev/mapper/cryptswap",
                    "renameTo": "/dev/mapper/cryptswap-retired"
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
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "dmmaps:cryptswap:rename"
            && step.commands.iter().any(|command| {
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
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "dmsetup",
                        "rename",
                        "/dev/mapper/cryptswap",
                        "cryptswap-retired",
                    ]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "dmmaps:cryptswap:rename"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "dmsetup",
                        "info",
                        "-c",
                        "--noheadings",
                        "-o",
                        "name,uuid,major,minor,open,segments,events",
                        "/dev/mapper/cryptswap-retired",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "disk-nix",
                        "inspect",
                        "/dev/mapper/cryptswap-retired",
                        "--json",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
}

#[test]
fn dm_map_rename_requires_concrete_path_and_new_name_for_execute_readiness() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "dmMaps": {
                  "cryptswap": {
                    "operation": "rename",
                    "renameTo": "/tmp/not-a-mapper-name"
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
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "dmmaps:cryptswap:rename"
            && step.commands.iter().any(|command| {
                command.argv == ["dmsetup", "rename", "<dm-map>", "<new-dm-map-name>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["device-mapper path", "new device-mapper name"]
            })
    }));
}

#[test]
fn dm_map_destroy_reports_dmsetup_remove_command() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "dmMaps": {
                  "oldmap": {
                    "operation": "destroy",
                    "target": "/dev/mapper/oldmap"
                  }
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
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "dmmaps:oldmap:destroy"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "dmsetup",
                        "info",
                        "-c",
                        "--noheadings",
                        "-o",
                        "name,uuid,major,minor,open,segments,events",
                        "/dev/mapper/oldmap",
                    ]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["dmsetup", "deps", "-o", "devname", "/dev/mapper/oldmap"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["dmsetup", "status", "/dev/mapper/oldmap"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["dmsetup", "remove", "/dev/mapper/oldmap"]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "dmmaps:oldmap:destroy"
            && step.commands.iter().any(|command| {
                command.argv == ["dmsetup", "ls", "--tree"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
}

#[test]
fn dm_map_destroy_requires_concrete_mapper_path_for_execute_readiness() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "dmMaps": {
                  "oldmap": {
                    "operation": "destroy"
                  }
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
    assert!(!report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "dmmaps:oldmap:destroy"
            && step.commands.iter().any(|command| {
                command.argv == ["dmsetup", "remove", "<dm-map>"]
                    && command.mutates
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["device-mapper path"]
            })
    }));
}

#[test]
fn loop_device_lifecycle_accepts_path_aliases_for_logical_names() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "loopDevices": {
                  "root-image": {
                    "operation": "create",
                    "path": "/dev/loop7",
                    "device": "/var/lib/images/root.img"
                  },
                  "grown-image": {
                    "operation": "grow",
                    "target": "/dev/loop8"
                  },
                  "inventory-image": {
                    "operation": "rescan",
                    "path": "/dev/loop10"
                  },
                  "old-image": {
                    "operation": "destroy",
                    "target": "/dev/loop9"
                  }
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
    assert!(report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "loopdevices:root-image:create"
            && step.commands.iter().any(|command| {
                command.argv == ["losetup", "/dev/loop7", "/var/lib/images/root.img"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "loopdevices:grown-image:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["losetup", "-c", "/dev/loop8"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "loopdevices:inventory-image:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["losetup", "--json", "--list", "/dev/loop10"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "loopdevices:old-image:destroy"
            && step.commands.iter().any(|command| {
                command.argv == ["losetup", "--detach", "/dev/loop9"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
}

#[test]
fn blocked_reports_do_not_render_scripts() {
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

    assert!(report.to_shell_script().is_none());
}

#[test]
fn execute_refuses_non_ready_command_plans() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "volumes": {
                  "vg/root": { "operation": "grow" }
                }
              },
              "apply": {
                "allowGrow": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::Execute);

    assert_eq!(report.status, ExecutionStatus::NotReady);
    assert!(!report.can_apply());
    assert_eq!(report.command_plan.len(), 1);
    assert!(report.execution_results.is_empty());
    assert!(report
        .messages
        .iter()
        .any(|message| message.contains("every planned command must be ready")));
    assert!(report.recovery_actions.iter().any(|action| {
        action.kind == RecoveryActionKind::ResolveInputs
            && action
                .notes
                .iter()
                .any(|note| note.contains("need desired size"))
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
fn execute_refuses_unimplemented_domain_action_placeholders() {
    let plan = Plan {
        summary: PlanSummary {
            action_count: 1,
            offline_required_count: 0,
            destructive_count: 0,
            potential_data_loss_count: 0,
            unsupported_count: 0,
        },
        dependency_order: Vec::new(),
        actions: vec![PlannedAction {
            id: "mysteryVolumes:alpha:check".to_string(),
            description: "check a storage domain without a command renderer".to_string(),
            operation: Operation::Check,
            risk: RiskClass::Safe,
            destructive: false,
            context: ActionContext {
                collection: Some("mysteryVolumes".to_string()),
                target: Some("/dev/mystery-alpha".to_string()),
                ..ActionContext::default()
            },
            advice: None,
        }],
        topology_comparison: None,
    };
    let mut ran_commands = false;

    let report = prepare_execution_with_runner(
        &plan,
        ApplyPolicy::default(),
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
    );

    assert_eq!(report.status, ExecutionStatus::NotReady);
    assert!(!ran_commands);
    assert_eq!(report.command_summary.command_count, 1);
    assert_eq!(report.command_summary.needs_domain_implementation_count, 1);
    assert!(!report.command_summary.all_commands_ready());
    assert!(report.execution_results.is_empty());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "mysteryVolumes:alpha:check"
            && step.requires_manual_review
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "disk-nix",
                        "storage-action",
                        "check",
                        "--collection",
                        "mysteryVolumes",
                        "--target",
                        "/dev/mystery-alpha",
                    ]
                    && command.mutates
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["storage-domain command renderer"]
            })
    }));
}

#[test]
fn execute_refuses_graph_dependency_conflicts_before_running_commands() {
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
    .expect("document parses");
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
    graph.add_edge(disk_nix_model::Edge::new(
        "luks:cryptroot",
        "filesystem:/",
        Relationship::Backs,
    ));
    let plan = compare_plan_with_topology(plan, &graph);
    let dry_run = prepare_execution(&plan, policy.clone(), ExecutionMode::DryRun);

    assert_eq!(dry_run.status, ExecutionStatus::DryRun);
    assert_eq!(
        dry_run
            .topology_comparison
            .as_ref()
            .map(|comparison| comparison.summary.graph_dependency_conflict_count),
        Some(1)
    );
    assert!(!dry_run.can_apply());
    assert!(dry_run.to_shell_script().is_none());

    let mut ran_commands = false;
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |_| {
        ran_commands = true;
        CommandRunResult {
            success: true,
            status_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
        }
    });

    assert_eq!(report.status, ExecutionStatus::NotReady);
    assert!(!ran_commands);
    assert!(report.execution_results.is_empty());
    assert!(report.command_summary.all_commands_ready());
    assert!(report.messages.iter().any(|message| {
        message.contains("graph dependency conflict") && message.contains("execute refused")
    }));
    assert!(report.recovery_actions.iter().any(|action| {
        action.kind == RecoveryActionKind::ResolveInputs
            && action
                .notes
                .iter()
                .any(|note| note.contains("1 graph dependency conflict"))
    }));
}

#[test]
fn execute_refuses_partially_suppressed_reconciliation_groups_before_running_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "exports": {
                  "/srv/share": {
                    "operation": "export",
                    "client": "192.0.2.0/24",
                    "options": "rw,sync,no_subtree_check"
                  }
                },
                "nfs": {
                  "mounts": {
                    "/mnt/share": {
                      "operation": "mount",
                      "source": "nas.example.com:/srv/share",
                      "fsType": "nfs4"
                    }
                  }
                }
              },
              "apply": {}
            }"#,
    )
    .expect("document parses");
    let mut graph = StorageGraph::empty();
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
        .with_property("nfs.export-option-no-subtree-check", "true"),
    );
    let plan = compare_plan_with_topology(plan, &graph);
    let dry_run = prepare_execution(&plan, policy.clone(), ExecutionMode::DryRun);

    assert_eq!(dry_run.status, ExecutionStatus::DryRun);
    assert_eq!(
        dry_run
            .topology_comparison
            .as_ref()
            .map(|comparison| comparison.summary.partially_suppressed_group_count),
        Some(1)
    );
    assert!(!dry_run.can_apply());
    assert!(dry_run.to_shell_script().is_none());
    assert!(dry_run.messages.iter().any(|message| {
        message.contains("partially suppressed reconciliation group")
            && message.contains("fresh-topology review")
    }));

    let mut ran_commands = false;
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |_| {
        ran_commands = true;
        CommandRunResult {
            success: true,
            status_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
        }
    });

    assert_eq!(report.status, ExecutionStatus::NotReady);
    assert!(!ran_commands);
    assert!(report.execution_results.is_empty());
    assert!(report.command_summary.all_commands_ready());
    assert!(report.messages.iter().any(|message| {
        message.contains("partially suppressed reconciliation group")
            && message.contains("execute refused")
    }));
    assert!(report.recovery_actions.iter().any(|action| {
        action.kind == RecoveryActionKind::ResolveInputs
            && action
                .notes
                .iter()
                .any(|note| note.contains("1 partially suppressed reconciliation group"))
    }));
}

#[test]
fn execute_runs_ready_commands_and_verification_with_runner() {
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

    let mut seen = Vec::new();
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        seen.push(argv.to_vec());
        CommandRunResult {
            success: true,
            status_code: Some(0),
            stdout: "ok\n".to_string(),
            stderr: String::new(),
        }
    });

    assert_eq!(report.status, ExecutionStatus::Succeeded);
    assert_eq!(report.execution_results.len(), seen.len());
    assert!(report.execution_results.iter().all(|result| result.success));
    let exportfs_requirement = report
        .tool_requirements
        .iter()
        .find(|requirement| requirement.tool == "exportfs")
        .expect("exportfs tool requirement is reported");
    assert_eq!(exportfs_requirement.command_count, 2);
    assert_eq!(exportfs_requirement.mutating_count, 1);
    assert_eq!(exportfs_requirement.verification_count, 1);
    assert_eq!(
        exportfs_requirement.phases,
        [ExecutionPhase::Command, ExecutionPhase::Verification]
    );
    assert_eq!(
        exportfs_requirement.availability,
        ToolAvailability::Available
    );
    assert!(exportfs_requirement.message.contains("available"));
    assert!(exportfs_requirement
        .remediation
        .iter()
        .any(|hint| hint.contains("pkgs.nfs-utils")));
    assert!(seen.iter().any(|argv| {
        argv == &[
            "exportfs".to_string(),
            "-i".to_string(),
            "-o".to_string(),
            "ro,sync".to_string(),
            "192.0.2.0/24:/srv/share".to_string(),
        ]
    }));
    assert!(report.execution_results.iter().any(|result| {
        result.phase == ExecutionPhase::Verification && result.argv == ["exportfs", "-v"]
    }));
}

#[test]
fn execute_treats_inactive_luks_close_verification_as_success() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "luks": {
                "devices": {
                  "closedMapping": {
                    "device": "/dev/disk/by-id/closed-luks",
                    "target": "cryptclosed",
                    "operation": "close"
                  }
                }
              },
              "apply": {
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let mut status_calls = 0;
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        let status_code = if argv == ["cryptsetup", "status", "cryptclosed"] {
            status_calls += 1;
            if status_calls == 1 {
                0
            } else {
                4
            }
        } else {
            0
        };
        CommandRunResult {
            success: status_code == 0,
            status_code: Some(status_code),
            stdout: if status_code == 4 {
                "/dev/mapper/cryptclosed is inactive.\n".to_string()
            } else {
                String::new()
            },
            stderr: String::new(),
        }
    });

    assert_eq!(report.status, ExecutionStatus::Succeeded);
    assert!(report.execution_results.iter().any(|result| {
        result.phase == ExecutionPhase::Command
            && result.argv == ["cryptsetup", "close", "cryptclosed"]
            && result.success
    }));
    let verification = report
        .execution_results
        .iter()
        .find(|result| {
            result.phase == ExecutionPhase::Verification
                && result.argv == ["cryptsetup", "status", "cryptclosed"]
        })
        .expect("cryptsetup close verification ran");
    assert!(verification.success);
    assert_eq!(verification.status_code, Some(4));
    assert!(verification.stdout.contains("inactive"));
}
