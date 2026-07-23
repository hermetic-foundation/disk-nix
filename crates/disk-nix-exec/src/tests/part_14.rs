#[test]
fn failed_swap_label_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "swaps": {
                "primary": {
                  "device": "/dev/disk/by-label/swap-old",
                  "properties": {
                    "label": "swap-new"
                  }
                }
              },
              "apply": {
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_label = [
        "swaplabel",
        "--label",
        "swap-new",
        "/dev/disk/by-label/swap-old",
    ];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_label,
            status_code: Some(if argv == failed_label { 5 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_label {
                "swap label failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report
        .execution_results
        .iter()
        .any(|result| !result.success && result.argv == failed_label));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("swap domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("SetProperty"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["swapon", "--show", "--bytes", "--raw"] && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["blkid", "/dev/disk/by-label/swap-old"] && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv
            == [
                "disk-nix",
                "inspect",
                "/dev/disk/by-label/swap-old",
                "--json",
            ]
            && !command.mutates
    }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| note.contains("swap changes") && note.contains("resume")));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("swap rollback recovery review is reported");
    assert!(rollback.commands.iter().any(|command| {
        command.argv == ["blkid", "/dev/disk/by-label/swap-old"] && !command.mutates
    }));
}

#[test]
fn failed_zfs_dataset_rename_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "datasets": {
                "tank/home": {
                  "operation": "rename",
                  "renameTo": "tank/home-staged"
                }
              },
              "apply": {
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_rename = ["zfs", "rename", "tank/home", "tank/home-staged"];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_rename,
            status_code: Some(if argv == failed_rename { 1 } else { 0 }),
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
        .expect("ZFS dataset domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Rename"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["zfs", "list", "-H", "-p", "-t", "filesystem", "tank/home"]
            && !command.mutates
    }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["zfs", "get", "all", "tank/home"] && !command.mutates }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["disk-nix", "inspect", "tank/home", "--json"] && !command.mutates
    }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| note.contains("ZFS changes") && note.contains("LUN consumers")));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("ZFS dataset roll-forward recovery review is reported");
    assert!(roll_forward
        .commands
        .iter()
        .any(|command| { command.argv == ["zfs", "get", "all", "tank/home"] && !command.mutates }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("ZFS dataset rollback recovery review is reported");
    assert!(rollback.commands.iter().any(|command| {
        command.argv == ["zfs", "list", "-H", "-p", "-t", "filesystem", "tank/home"]
            && !command.mutates
    }));
}

#[test]
fn failed_filesystem_grow_reports_domain_recovery_guidance() {
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

    let failed_grow = ["xfs_growfs", "/"];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_grow,
            status_code: Some(if argv == failed_grow { 1 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_grow {
                "grow failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report
        .execution_results
        .iter()
        .any(|result| !result.success && result.argv == failed_grow));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("filesystem domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Grow"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["findmnt", "--json", "--target", "/"] && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["disk-nix", "inspect", "/", "--json"] && !command.mutates
    }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| note.contains("filesystem changes") && note.contains("UUIDs")));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("filesystem roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv == ["findmnt", "--json", "--target", "/"] && !command.mutates
    }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("filesystem rollback recovery review is reported");
    assert!(rollback.commands.iter().any(|command| {
        command.argv == ["disk-nix", "inspect", "/", "--json"] && !command.mutates
    }));
    let recipe = report
        .rollback_recipes
        .first()
        .expect("filesystem grow rollback recipe is reported");
    assert_eq!(recipe.status, RollbackRecipeStatus::Refused);
    assert!(recipe.reversible_mutations.commands.is_empty());
    assert!(recipe.refusal_reasons.iter().any(|reason| {
        reason.contains("filesystem grow rollback is refused")
            && reason.contains("not data-preserving")
    }));
    assert!(recipe
        .operator_only_handoff
        .notes
        .iter()
        .any(|note| { note.contains("grow") && note.contains("operator review") }));
}

#[test]
fn filesystem_remount_failure_emits_proven_safe_rollback_recipe() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "scratch": {
                    "mountpoint": "/scratch",
                    "fsType": "xfs",
                    "operation": "remount",
                    "options": ["rw", "noatime"],
                    "rollbackOptions": "ro,noatime"
                  }
                }
              },
              "apply": {
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_remount = ["mount", "-o", "remount,rw,noatime", "/scratch"];
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
    let recipe = report
        .rollback_recipes
        .first()
        .expect("filesystem remount rollback recipe is reported");
    assert_eq!(recipe.status, RollbackRecipeStatus::ProvenSafe);
    assert!(recipe.refusal_reasons.is_empty());
    assert!(recipe
        .read_only_validation
        .commands
        .iter()
        .all(|command| { !command.mutates && command.readiness == CommandReadiness::Ready }));
    assert_eq!(recipe.reversible_mutations.commands.len(), 1);
    assert_eq!(
        recipe.reversible_mutations.commands[0].argv,
        ["mount", "-o", "remount,ro,noatime", "/scratch"]
    );

    let mut ran = Vec::new();
    let replay = replay_proven_safe_rollback_recipe_with_runner(
        &report,
        0,
        "receipt:remount".to_string(),
        "topology:current".to_string(),
        &mut |argv| {
            ran.push(argv.to_vec());
            CommandRunResult {
                success: true,
                status_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            }
        },
    );
    assert_eq!(replay.status, RollbackExecutionStatus::Succeeded);
    assert!(ran.iter().any(|argv| {
        argv == &[
            "mount".to_string(),
            "-o".to_string(),
            "remount,ro,noatime".to_string(),
            "/scratch".to_string(),
        ]
    }));
}

#[test]
fn filesystem_property_failure_emits_proven_safe_rollback_recipe() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "home": {
                    "mountpoint": "/home",
                    "device": "/dev/disk/by-label/home-old",
                    "fsType": "ext4",
                    "properties": {
                      "label": "home-new"
                    },
                    "rollbackValue": "home-old"
                  }
                }
              },
              "apply": {
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_label = ["e2label", "/dev/disk/by-label/home-old", "home-new"];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_label,
            status_code: Some(if argv == failed_label { 1 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_label {
                "label failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    let recipe = report
        .rollback_recipes
        .first()
        .expect("filesystem property rollback recipe is reported");
    assert_eq!(recipe.status, RollbackRecipeStatus::ProvenSafe);
    assert_eq!(
        recipe.reversible_mutations.commands[0].argv,
        ["e2label", "/dev/disk/by-label/home-old", "home-old"]
    );

    let replay = replay_proven_safe_rollback_recipe_with_runner(
        &report,
        0,
        "receipt:property".to_string(),
        "topology:current".to_string(),
        &mut |_| CommandRunResult {
            success: true,
            status_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
        },
    );
    assert_eq!(replay.status, RollbackExecutionStatus::Succeeded);
}

#[test]
fn filesystem_mount_verification_failure_emits_proven_safe_unmount_recipe() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "media": {
                    "mountpoint": "/media",
                    "device": "/dev/disk/by-label/media",
                    "fsType": "ext4",
                    "operation": "mount",
                    "options": ["rw", "nodev"]
                  }
                }
              },
              "apply": {
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_verification = ["findmnt", "--json", "/media"];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_verification,
            status_code: Some(if argv == failed_verification { 1 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_verification {
                "mount verification failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report.execution_results.iter().any(|result| {
        result.success
            && result.phase == ExecutionPhase::Command
            && result.argv
                == [
                    "mount",
                    "-t",
                    "ext4",
                    "-o",
                    "rw,nodev",
                    "/dev/disk/by-label/media",
                    "/media",
                ]
    }));
    let recipe = report
        .rollback_recipes
        .first()
        .expect("filesystem mount rollback recipe is reported");
    assert_eq!(recipe.status, RollbackRecipeStatus::ProvenSafe);
    assert_eq!(
        recipe.reversible_mutations.commands[0].argv,
        ["umount", "/media"]
    );
}

#[test]
fn filesystem_check_scrub_and_repair_failures_emit_refused_rollback_recipes() {
    let cases: &[(&[u8], &[&str], &str)] = &[
        (
            br#"{
                  "spec": {
                    "filesystems": {
                      "home": {
                        "operation": "check",
                        "mountpoint": "/home",
                        "device": "/dev/disk/by-label/home",
                        "fsType": "ext4"
                      }
                    }
                  },
                  "apply": {
                    "allowOffline": true
                  }
                }"#,
            &["e2fsck", "-n", "/dev/disk/by-label/home"],
            "failed-check",
        ),
        (
            br#"{
                  "spec": {
                    "filesystems": {
                      "data": {
                        "operation": "scrub",
                        "mountpoint": "/data",
                        "fsType": "btrfs"
                      }
                    }
                  },
                  "apply": {
                    "allowOffline": true
                  }
                }"#,
            &["btrfs", "scrub", "start", "-B", "/data"],
            "scrub",
        ),
        (
            br#"{
                  "spec": {
                    "filesystems": {
                      "bulk": {
                        "operation": "repair",
                        "mountpoint": "/bulk",
                        "device": "/dev/disk/by-label/bulk",
                        "fsType": "xfs"
                      }
                    }
                  },
                  "apply": {
                    "allowOffline": true
                  }
                }"#,
            &["xfs_repair", "/dev/disk/by-label/bulk"],
            "repair",
        ),
    ];

    for (document, failed_command, boundary) in cases {
        let (plan, policy) = plan_and_policy_from_json_bytes(document).expect("document parses");
        let failed_command: Vec<String> = failed_command
            .iter()
            .map(|part| (*part).to_string())
            .collect();
        let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
            CommandRunResult {
                success: argv != failed_command,
                status_code: Some(if argv == failed_command { 1 } else { 0 }),
                stdout: String::new(),
                stderr: if argv == failed_command {
                    format!("{boundary} failed")
                } else {
                    String::new()
                },
            }
        });

        assert_eq!(report.status, ExecutionStatus::Failed);
        let recipe = report
            .rollback_recipes
            .first()
            .unwrap_or_else(|| panic!("{boundary} rollback recipe should be reported"));
        assert_eq!(recipe.status, RollbackRecipeStatus::Refused);
        assert!(recipe.reversible_mutations.commands.is_empty());
        assert!(recipe.notes.iter().any(|note| note.contains(boundary)));
        assert!(recipe
            .operator_only_handoff
            .notes
            .iter()
            .any(|note| note.contains("operator review")));
    }
}

#[test]
fn block_stack_property_failures_emit_proven_safe_rollback_recipes() {
    for (case, spec, failed_command, expected_rollback) in [
        (
            "swap label",
            br#"{
                  "spec": {
                    "swaps": {
                      "primary": {
                        "device": "/dev/disk/by-label/swap-old",
                        "rollbackValue": "swap-old",
                        "properties": {
                          "label": "swap-new"
                        }
                      }
                    }
                  },
                  "apply": {
                    "allowOffline": true
                  }
                }"# as &[u8],
            vec![
                "swaplabel".to_string(),
                "--label".to_string(),
                "swap-new".to_string(),
                "/dev/disk/by-label/swap-old".to_string(),
            ],
            vec![
                "swaplabel".to_string(),
                "--label".to_string(),
                "swap-old".to_string(),
                "/dev/disk/by-label/swap-old".to_string(),
            ],
        ),
        (
            "LUKS label",
            br#"{
                  "spec": {
                    "luks": {
                      "devices": {
                        "cryptroot": {
                          "device": "/dev/disk/by-id/root-luks",
                          "target": "cryptroot",
                          "rollbackValue": "root-old",
                          "properties": {
                            "label": "root-new"
                          }
                        }
                      }
                    }
                  },
                  "apply": {
                    "allowOffline": true,
                    "allowPropertyChanges": true
                  }
                }"# as &[u8],
            vec![
                "cryptsetup".to_string(),
                "config".to_string(),
                "/dev/disk/by-id/root-luks".to_string(),
                "--label".to_string(),
                "root-new".to_string(),
            ],
            vec![
                "cryptsetup".to_string(),
                "config".to_string(),
                "/dev/disk/by-id/root-luks".to_string(),
                "--label".to_string(),
                "root-old".to_string(),
            ],
        ),
    ] {
        let (plan, policy) = plan_and_policy_from_json_bytes(spec).expect("document parses");
        let report = prepare_execution_with_runner_and_tool_checker(
            &plan,
            policy,
            ExecutionMode::Execute,
            |argv| CommandRunResult {
                success: argv != failed_command.as_slice(),
                status_code: Some(if argv == failed_command.as_slice() {
                    12
                } else {
                    0
                }),
                stdout: String::new(),
                stderr: if argv == failed_command.as_slice() {
                    format!("{case} failed")
                } else {
                    String::new()
                },
            },
            |_| true,
        );

        assert_eq!(report.status, ExecutionStatus::Failed, "{case}");
        let recipe = report
            .rollback_recipes
            .first()
            .unwrap_or_else(|| panic!("{case} rollback recipe is reported"));
        assert_eq!(recipe.status, RollbackRecipeStatus::ProvenSafe, "{case}");
        assert_eq!(
            recipe.reversible_mutations.commands[0].argv, expected_rollback,
            "{case}"
        );

        let replay = replay_proven_safe_rollback_recipe_with_runner(
            &report,
            0,
            "receipt:block-stack-property".to_string(),
            "topology:block-stack-property".to_string(),
            &mut |_| CommandRunResult {
                success: true,
                status_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            },
        );
        assert_eq!(
            replay.status,
            RollbackExecutionStatus::Succeeded,
            "{case}: {:?}",
            replay.refusal_reasons
        );
    }
}

#[test]
fn block_stack_verification_failures_emit_proven_safe_rollback_recipes() {
    for (case, spec, failed_verification, expected_rollback) in [
        (
            "device-mapper rename",
            br#"{
                  "spec": {
                    "dmMaps": {
                      "crypt-old": {
                        "operation": "rename",
                        "target": "/dev/mapper/crypt-old",
                        "renameTo": "crypt-new",
                        "rollbackValue": "crypt-old"
                      }
                    }
                  },
                  "apply": {
                    "allowOffline": true
                  }
                }"# as &[u8],
            vec![
                "dmsetup".to_string(),
                "info".to_string(),
                "-c".to_string(),
                "--noheadings".to_string(),
                "-o".to_string(),
                "name,uuid,major,minor,open,segments,events".to_string(),
                "/dev/mapper/crypt-new".to_string(),
            ],
            vec![
                "dmsetup".to_string(),
                "rename".to_string(),
                "crypt-new".to_string(),
                "crypt-old".to_string(),
            ],
        ),
        (
            "LUKS open",
            br#"{
                  "spec": {
                    "luks": {
                      "devices": {
                        "cryptroot": {
                          "operation": "open",
                          "device": "/dev/disk/by-id/root-luks",
                          "target": "cryptroot"
                        }
                      }
                    }
                  },
                  "apply": {
                    "allowOffline": true
                  }
                }"# as &[u8],
            vec![
                "cryptsetup".to_string(),
                "status".to_string(),
                "cryptroot".to_string(),
            ],
            vec![
                "cryptsetup".to_string(),
                "close".to_string(),
                "cryptroot".to_string(),
            ],
        ),
    ] {
        let (plan, policy) = plan_and_policy_from_json_bytes(spec).expect("document parses");
        let report = prepare_execution_with_runner_and_tool_checker(
            &plan,
            policy,
            ExecutionMode::Execute,
            |argv| CommandRunResult {
                success: argv != failed_verification.as_slice(),
                status_code: Some(if argv == failed_verification.as_slice() {
                    13
                } else {
                    0
                }),
                stdout: String::new(),
                stderr: if argv == failed_verification.as_slice() {
                    format!("{case} verification failed")
                } else {
                    String::new()
                },
            },
            |_| true,
        );

        assert_eq!(report.status, ExecutionStatus::Failed, "{case}");
        let recipe = report
            .rollback_recipes
            .first()
            .unwrap_or_else(|| panic!("{case} rollback recipe is reported"));
        assert_eq!(recipe.status, RollbackRecipeStatus::ProvenSafe, "{case}");
        assert_eq!(
            recipe.reversible_mutations.commands[0].argv, expected_rollback,
            "{case}"
        );

        let replay = replay_proven_safe_rollback_recipe_with_runner(
            &report,
            0,
            "receipt:block-stack-verification".to_string(),
            "topology:block-stack-verification".to_string(),
            &mut |_| CommandRunResult {
                success: true,
                status_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            },
        );
        assert_eq!(
            replay.status,
            RollbackExecutionStatus::Succeeded,
            "{case}: {:?}",
            replay.refusal_reasons
        );
    }
}

#[test]
fn block_stack_refused_boundaries_emit_operator_only_rollback_recipes() {
    for (boundary, spec, failed_command, reason_fragment) in [
        (
            "partition grow",
            br#"{
                  "spec": {
                    "partitions": {
                      "root": {
                        "operation": "grow",
                        "device": "/dev/disk/by-id/nvme-root",
                        "target": "/dev/disk/by-id/nvme-root-part2",
                        "partitionNumber": 2,
                        "end": "100%"
                      }
                    }
                  },
                  "apply": {
                    "allowOffline": true,
                    "allowGrow": true
                  }
                }"# as &[u8],
            vec![
                "parted".to_string(),
                "-s".to_string(),
                "/dev/disk/by-id/nvme-root".to_string(),
                "resizepart".to_string(),
                "2".to_string(),
                "100%".to_string(),
            ],
            "partition rollback is refused",
        ),
        (
            "LVM grow",
            br#"{
                  "spec": {
                    "volumes": {
                      "vg0/root": {
                        "operation": "grow",
                        "target": "vg0/root",
                        "desiredSize": "64GiB"
                      }
                    }
                  },
                  "apply": {
                    "allowGrow": true
                  }
                }"# as &[u8],
            vec![
                "lvextend".to_string(),
                "--resizefs".to_string(),
                "--size".to_string(),
                "64GiB".to_string(),
                "vg0/root".to_string(),
            ],
            "LVM rollback is refused",
        ),
        (
            "MD RAID replace",
            br#"{
                  "spec": {
                    "mdRaids": {
                      "root": {
                        "target": "/dev/md/root",
                        "replaceDevices": {
                          "/dev/disk/by-id/old-md-member": "/dev/disk/by-id/new-md-member"
                        }
                      }
                    }
                  },
                  "apply": {
                    "allowOffline": true
                  }
                }"# as &[u8],
            vec![
                "mdadm".to_string(),
                "/dev/md/root".to_string(),
                "--replace".to_string(),
                "/dev/disk/by-id/old-md-member".to_string(),
                "--with".to_string(),
                "/dev/disk/by-id/new-md-member".to_string(),
            ],
            "MD RAID rollback is refused",
        ),
        (
            "swap deactivate",
            br#"{
                  "spec": {
                    "swaps": {
                      "retired": {
                        "operation": "deactivate",
                        "device": "/dev/disk/by-label/swap-retired"
                      }
                    }
                  },
                  "apply": {
                    "allowOffline": true
                  }
                }"# as &[u8],
            vec![
                "swapoff".to_string(),
                "/dev/disk/by-label/swap-retired".to_string(),
            ],
            "swap deactivation rollback",
        ),
        (
            "loop create",
            br#"{
                  "spec": {
                    "loopDevices": {
                      "loop0": {
                        "operation": "create",
                        "target": "/dev/loop10",
                        "source": "/var/lib/images/root.img"
                      }
                    }
                  },
                  "apply": {
                    "allowOffline": true
                  }
                }"# as &[u8],
            vec![
                "losetup".to_string(),
                "/dev/loop10".to_string(),
                "/var/lib/images/root.img".to_string(),
            ],
            "loop attach rollback",
        ),
        (
            "backing-file grow",
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
                }"# as &[u8],
            vec![
                "truncate".to_string(),
                "--size".to_string(),
                "16GiB".to_string(),
                "/var/lib/images/root.img".to_string(),
            ],
            "backing-file rollback is refused",
        ),
    ] {
        let (plan, policy) = plan_and_policy_from_json_bytes(spec).expect("document parses");
        let report = prepare_execution_with_runner_and_tool_checker(
            &plan,
            policy,
            ExecutionMode::Execute,
            |argv| CommandRunResult {
                success: argv != failed_command.as_slice(),
                status_code: Some(if argv == failed_command.as_slice() {
                    14
                } else {
                    0
                }),
                stdout: String::new(),
                stderr: if argv == failed_command.as_slice() {
                    format!("{boundary} failed")
                } else {
                    String::new()
                },
            },
            |_| true,
        );

        assert_eq!(report.status, ExecutionStatus::Failed, "{boundary}");
        let recipe = report
            .rollback_recipes
            .first()
            .unwrap_or_else(|| panic!("{boundary} rollback recipe should be reported"));
        assert_eq!(recipe.status, RollbackRecipeStatus::Refused, "{boundary}");
        assert!(
            recipe.reversible_mutations.commands.is_empty(),
            "{boundary} should not emit automatic rollback mutation"
        );
        assert!(
            recipe
                .operator_only_handoff
                .notes
                .iter()
                .any(|note| note.contains("operator review")),
            "{boundary} should hand off to operator review"
        );
        assert!(
            recipe
                .refusal_reasons
                .iter()
                .any(|reason| reason.contains(reason_fragment)),
            "{boundary} should explain refusal"
        );
    }
}
