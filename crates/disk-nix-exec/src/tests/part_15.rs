#[test]
fn block_stack_zram_boundary_emits_refused_rollback_recipe() {
    let partial = PartialExecutionRecovery {
        completed_action_ids: Vec::new(),
        failed_action_id: "zram:set-property:algorithm".to_string(),
        failed_phase: ExecutionPhase::Command,
        failed_command: vec![
            "zramctl".to_string(),
            "--algorithm".to_string(),
            "zstd".to_string(),
            "/dev/zram0".to_string(),
        ],
        retry_review_action_ids: vec!["zram:set-property:algorithm".to_string()],
        remaining_action_ids: Vec::new(),
        completed_mutating_command_count: 0,
        notes: Vec::new(),
    };
    let rollback_review = RecoveryAction {
        kind: RecoveryActionKind::RollbackReview,
        summary: "review zram rollback preconditions".to_string(),
        commands: vec![command(
            ["disk-nix", "zram", "--json"],
            false,
            "inspect generated zram state before rollback review",
        )],
        notes: vec!["read-only zram rollback review".to_string()],
    };
    let step = ExecutionStep {
        action_id: "zram:set-property:algorithm".to_string(),
        operation: Operation::SetProperty,
        risk: RiskClass::OfflineRequired,
        requires_manual_review: false,
        commands: vec![command(
            ["zramctl", "--algorithm", "zstd", "/dev/zram0"],
            true,
            "test zram property mutation boundary",
        )],
        notes: Vec::new(),
    };

    let recipe = block_stack_rollback_recipe_for_step(&partial, &rollback_review, &step)
        .expect("zram boundary is handled by block-stack rollback recipes");

    assert_eq!(recipe.status, RollbackRecipeStatus::Refused);
    assert!(recipe.reversible_mutations.commands.is_empty());
    assert!(recipe
        .refusal_reasons
        .iter()
        .any(|reason| reason.contains("zram rollback is refused")));
    assert!(recipe
        .operator_only_handoff
        .notes
        .iter()
        .any(|note| note.contains("operator review")));
}

#[test]
fn advanced_storage_property_failures_emit_proven_safe_rollback_recipes() {
    for (case, spec, failed_command, expected_rollback) in [
        (
            "ZFS dataset property",
            br#"{
                  "spec": {
                    "datasets": {
                      "tank/home": {
                        "properties": {
                          "compression": "zstd"
                        },
                        "rollbackValue": "lz4"
                      }
                    }
                  },
                  "apply": {
                    "allowPropertyChanges": true
                  }
                }"# as &[u8],
            vec![
                "zfs".to_string(),
                "set".to_string(),
                "compression=zstd".to_string(),
                "tank/home".to_string(),
            ],
            vec![
                "zfs".to_string(),
                "set".to_string(),
                "compression=lz4".to_string(),
                "tank/home".to_string(),
            ],
        ),
        (
            "VDO write policy",
            br#"{
                  "spec": {
                    "vdoVolumes": {
                      "archive": {
                        "properties": {
                          "writePolicy": "sync"
                        },
                        "rollbackValue": "async"
                      }
                    }
                  },
                  "apply": {
                    "allowPropertyChanges": true
                  }
                }"# as &[u8],
            vec![
                "vdo".to_string(),
                "changeWritePolicy".to_string(),
                "--name".to_string(),
                "archive".to_string(),
                "--writePolicy".to_string(),
                "sync".to_string(),
            ],
            vec![
                "vdo".to_string(),
                "changeWritePolicy".to_string(),
                "--name".to_string(),
                "archive".to_string(),
                "--writePolicy".to_string(),
                "async".to_string(),
            ],
        ),
        (
            "bcache cache mode",
            br#"{
                  "spec": {
                    "caches": {
                      "/dev/bcache1": {
                        "path": "/dev/bcache1",
                        "rollbackValue": "writeback",
                        "properties": {
                          "bcache.cache-mode": "writearound"
                        }
                      }
                    }
                  },
                  "apply": {
                    "allowPropertyChanges": true
                  }
                }"# as &[u8],
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/$3\"".to_string(),
                "disk-nix-bcache-property".to_string(),
                "/dev/bcache1".to_string(),
                "writearound".to_string(),
                "cache_mode".to_string(),
            ],
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/$3\"".to_string(),
                "disk-nix-bcache-property".to_string(),
                "/dev/bcache1".to_string(),
                "writeback".to_string(),
                "cache_mode".to_string(),
            ],
        ),
        (
            "Btrfs subvolume readonly",
            br#"{
                  "spec": {
                    "btrfsSubvolumes": {
                      "/mnt/persist/@home": {
                        "path": "/mnt/persist/@home",
                        "rollbackValue": "false",
                        "properties": {
                          "readonly": true
                        }
                      }
                    }
                  },
                  "apply": {
                    "allowOffline": true
                  }
                }"# as &[u8],
            vec![
                "btrfs".to_string(),
                "property".to_string(),
                "set".to_string(),
                "-ts".to_string(),
                "/mnt/persist/@home".to_string(),
                "ro".to_string(),
                "true".to_string(),
            ],
            vec![
                "btrfs".to_string(),
                "property".to_string(),
                "set".to_string(),
                "-ts".to_string(),
                "/mnt/persist/@home".to_string(),
                "ro".to_string(),
                "false".to_string(),
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
                    22
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
            "receipt:advanced-property".to_string(),
            "topology:advanced-property".to_string(),
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
fn advanced_storage_refused_boundaries_emit_operator_only_rollback_recipes() {
    for (boundary, spec, failed_command, reason_fragment) in [
            (
                "ZFS snapshot rollback",
                br#"{
                  "spec": {
                    "snapshots": {
                      "tank/home@before": {
                        "target": "tank/home",
                        "rollback": true
                      }
                    }
                  },
                  "apply": {
                    "allowPotentialDataLoss": true
                  }
                }"# as &[u8],
                vec![
                    "zfs".to_string(),
                    "rollback".to_string(),
                    "tank/home@before".to_string(),
                ],
                "snapshot rollback is refused",
            ),
            (
                "ZFS snapshot clone",
                br#"{
                  "spec": {
                    "snapshots": {
                      "before-clone": {
                        "name": "tank/home@before",
                        "target": "tank/home",
                        "cloneTo": "tank/home-review"
                      }
                    }
                  },
                  "apply": {}
                }"# as &[u8],
                vec![
                    "zfs".to_string(),
                    "clone".to_string(),
                    "tank/home@before".to_string(),
                    "tank/home-review".to_string(),
                ],
                "snapshot rollback is refused",
            ),
            (
                "VDO grow",
                br#"{
                  "spec": {
                    "vdoVolumes": {
                      "archive": {
                        "operation": "grow",
                        "desiredSize": "4TiB"
                      }
                    }
                  },
                  "apply": {
                    "allowGrow": true
                  }
                }"# as &[u8],
                vec![
                    "vdo".to_string(),
                    "growLogical".to_string(),
                    "--name".to_string(),
                    "archive".to_string(),
                    "--vdoLogicalSize".to_string(),
                    "4TiB".to_string(),
                ],
                "VDO rollback is refused",
            ),
            (
                "bcache replacement",
                br#"{
                  "spec": {
                    "caches": {
                      "/dev/bcache0": {
                        "path": "/dev/bcache0",
                        "cacheSetUuid": "11111111-2222-3333-4444-555555555555",
                        "replaceDevices": {
                          "/dev/disk/by-id/old-cache": "/dev/disk/by-id/new-cache"
                        }
                      }
                    }
                  },
                  "apply": {
                    "allowDeviceReplacement": true,
                    "allowOffline": true
                  }
                }"# as &[u8],
                vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    "make-bcache -C \"$2\" --cset-uuid \"$3\" --writeback && printf '1\\n' > \"/sys/block/${1#/dev/}/bcache/detach\" && printf '%s\\n' \"$3\" > \"/sys/block/${1#/dev/}/bcache/attach\"".to_string(),
                    "disk-nix-bcache-replace".to_string(),
                    "/dev/bcache0".to_string(),
                    "/dev/disk/by-id/new-cache".to_string(),
                    "11111111-2222-3333-4444-555555555555".to_string(),
                ],
                "cache rollback is refused",
            ),
            (
                "LVM cache property",
                br#"{
                  "spec": {
                    "lvmCaches": {
                      "vg0/root": {
                        "properties": {
                          "lvm.cache-mode": "writethrough"
                        }
                      }
                    }
                  },
                  "apply": {
                    "allowPropertyChanges": true
                  }
                }"# as &[u8],
                vec![
                    "lvchange".to_string(),
                    "--cachemode".to_string(),
                    "writethrough".to_string(),
                    "vg0/root".to_string(),
                ],
                "cache rollback is refused",
            ),
            (
                "Btrfs qgroup limit",
                br#"{
                  "spec": {
                    "btrfsQgroups": {
                      "0/257": {
                        "target": "/mnt/persist",
                        "properties": {
                          "limit": "10GiB"
                        }
                      }
                    }
                  },
                  "apply": {
                    "allowOffline": true
                  }
                }"# as &[u8],
                vec![
                    "btrfs".to_string(),
                    "qgroup".to_string(),
                    "limit".to_string(),
                    "10GiB".to_string(),
                    "0/257".to_string(),
                    "/mnt/persist".to_string(),
                ],
                "Btrfs advanced rollback is refused",
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
                        23
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

#[test]
fn network_storage_failures_emit_proven_safe_rollback_recipes() {
    for (case, spec, failed_command, expected_rollback) in [
        (
            "NFS remount",
            br#"{
                  "spec": {
                    "nfs": {
                      "mounts": {
                        "/srv/tuned": {
                          "operation": "remount",
                          "source": "nas.example.com:/srv/tuned",
                          "options": ["_netdev", "rw", "vers=4.2"],
                          "rollbackValue": "_netdev,ro,vers=4.2"
                        }
                      }
                    }
                  },
                  "apply": {
                    "allowOffline": true
                  }
                }"# as &[u8],
            vec![
                "mount".to_string(),
                "-o".to_string(),
                "remount,_netdev,rw,vers=4.2".to_string(),
                "/srv/tuned".to_string(),
            ],
            vec![
                "mount".to_string(),
                "-o".to_string(),
                "remount,_netdev,ro,vers=4.2".to_string(),
                "/srv/tuned".to_string(),
            ],
        ),
        (
            "NFS mount verification",
            br#"{
                  "spec": {
                    "nfs": {
                      "mounts": {
                        "/srv/shared": {
                          "operation": "mount",
                          "source": "nas.example.com:/srv/shared",
                          "fsType": "nfs4",
                          "options": ["_netdev", "ro", "vers=4.2"]
                        }
                      }
                    }
                  },
                  "apply": {
                    "allowOffline": true
                  }
                }"# as &[u8],
            vec![
                "findmnt".to_string(),
                "--json".to_string(),
                "/srv/shared".to_string(),
            ],
            vec!["umount".to_string(), "/srv/shared".to_string()],
        ),
        (
            "NFS export options",
            br#"{
                  "spec": {
                    "exports": {
                      "/srv/share": {
                        "client": "192.0.2.0/24",
                        "properties": {
                          "options": "rw,sync,no_subtree_check"
                        },
                        "rollbackValue": "ro,sync,no_subtree_check"
                      }
                    }
                  },
                  "apply": {
                    "allowPropertyChanges": true
                  }
                }"# as &[u8],
            vec![
                "exportfs".to_string(),
                "-i".to_string(),
                "-o".to_string(),
                "rw,sync,no_subtree_check".to_string(),
                "192.0.2.0/24:/srv/share".to_string(),
            ],
            vec![
                "exportfs".to_string(),
                "-i".to_string(),
                "-o".to_string(),
                "ro,sync,no_subtree_check".to_string(),
                "192.0.2.0/24:/srv/share".to_string(),
            ],
        ),
        (
            "iSCSI login verification",
            br#"{
                  "spec": {
                    "iscsiSessions": {
                      "iqn.2026-06.example:storage.root": {
                        "operation": "login",
                        "portal": "192.0.2.10:3260"
                      }
                    }
                  },
                  "apply": {
                    "allowOffline": true
                  }
                }"# as &[u8],
            vec![
                "disk-nix".to_string(),
                "inspect".to_string(),
                "iqn.2026-06.example:storage.root".to_string(),
                "--json".to_string(),
            ],
            vec![
                "iscsiadm".to_string(),
                "--mode".to_string(),
                "node".to_string(),
                "--targetname".to_string(),
                "iqn.2026-06.example:storage.root".to_string(),
                "--portal".to_string(),
                "192.0.2.10:3260".to_string(),
                "--logout".to_string(),
            ],
        ),
        (
            "LIO target LUN property",
            br#"{
                  "spec": {
                    "targetLuns": {
                      "iqn.2026-06.example:storage.root": {
                        "provider": "lio",
                        "source": "/dev/zvol/tank/root",
                        "lun": 7,
                        "properties": {
                          "lio.writeCache": "off"
                        },
                        "rollbackValue": "1"
                      }
                    }
                  },
                  "apply": {
                    "allowPropertyChanges": true
                  }
                }"# as &[u8],
            vec![
                "targetcli".to_string(),
                "/backstores/block/_dev_zvol_tank_root".to_string(),
                "set".to_string(),
                "attribute".to_string(),
                "emulate_write_cache=0".to_string(),
            ],
            vec![
                "targetcli".to_string(),
                "/backstores/block/_dev_zvol_tank_root".to_string(),
                "set".to_string(),
                "attribute".to_string(),
                "emulate_write_cache=1".to_string(),
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
                    24
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
        assert!(recipe.refusal_reasons.is_empty(), "{case}");
        assert_eq!(
            recipe.reversible_mutations.commands[0].argv, expected_rollback,
            "{case}"
        );

        let replay = replay_proven_safe_rollback_recipe_with_runner(
            &report,
            0,
            "receipt:network".to_string(),
            "topology:network".to_string(),
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
fn network_storage_refused_boundaries_emit_operator_only_rollback_recipes() {
    for (boundary, spec, failed_command, reason_fragment) in [
            (
                "NFS unmount",
                br#"{
                  "spec": {
                    "nfs": {
                      "mounts": {
                        "/srv/old": {
                          "operation": "unmount",
                          "source": "nas.example.com:/srv/old"
                        }
                      }
                    }
                  },
                  "apply": {
                    "allowOffline": true
                  }
                }"# as &[u8],
                vec!["umount".to_string(), "/srv/old".to_string()],
                "network-storage rollback is refused",
            ),
            (
                "NFS unexport",
                br#"{
                  "spec": {
                    "exports": {
                      "/srv/share": {
                        "operation": "unexport",
                        "client": "192.0.2.0/24"
                      }
                    }
                  },
                  "apply": {
                    "allowOffline": true
                  }
                }"# as &[u8],
                vec![
                    "exportfs".to_string(),
                    "-u".to_string(),
                    "192.0.2.0/24:/srv/share".to_string(),
                ],
                "network-storage rollback is refused",
            ),
            (
                "iSCSI logout",
                br#"{
                  "spec": {
                    "iscsiSessions": {
                      "iqn.2026-06.example:storage.old": {
                        "operation": "logout",
                        "portal": "192.0.2.10:3260"
                      }
                    }
                  },
                  "apply": {
                    "allowOffline": true
                  }
                }"# as &[u8],
                vec![
                    "iscsiadm".to_string(),
                    "--mode".to_string(),
                    "node".to_string(),
                    "--targetname".to_string(),
                    "iqn.2026-06.example:storage.old".to_string(),
                    "--portal".to_string(),
                    "192.0.2.10:3260".to_string(),
                    "--logout".to_string(),
                ],
                "network-storage rollback is refused",
            ),
            (
                "host LUN grow",
                br#"{
                  "spec": {
                    "luns": {
                      "iqn.2026-06.example:storage/root:0": {
                        "operation": "grow",
                        "device": "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0",
                        "desiredSize": "2TiB"
                      }
                    }
                  },
                  "apply": {
                    "allowGrow": true,
                    "allowOffline": true
                  }
                }"# as &[u8],
                vec![
                    "iscsiadm".to_string(),
                    "--mode".to_string(),
                    "session".to_string(),
                    "--rescan".to_string(),
                ],
                "network-storage rollback is refused",
            ),
            (
                "target LUN grow",
                br#"{
                  "spec": {
                    "targetLuns": {
                      "iqn.2026-06.example:storage.root": {
                        "operation": "grow",
                        "provider": "lio",
                        "source": "/dev/zvol/tank/root",
                        "desiredSize": "4TiB",
                        "lun": 7
                      }
                    }
                  },
                  "apply": {
                    "allowGrow": true,
                    "allowOffline": true
                  }
                }"# as &[u8],
                vec!["targetcli".to_string(), "saveconfig".to_string()],
                "network-storage rollback is refused",
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
                        25
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

#[test]
fn failed_multi_layer_apply_reports_partial_execution_recovery_sequence() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "volumes": {
                "vg0/root": {
                  "operation": "grow",
                  "target": "vg0/root",
                  "desiredSize": "50GiB"
                }
              },
              "filesystems": {
                "root": {
                  "operation": "grow",
                  "device": "vg0/root",
                  "fsType": "ext4",
                  "desiredSize": "50GiB",
                  "resizePolicy": "grow-only"
                }
              },
              "apply": {
                "allowGrow": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        let failed = argv.first().is_some_and(|tool| tool == "resize2fs");
        CommandRunResult {
            success: !failed,
            status_code: Some(if failed { 1 } else { 0 }),
            stdout: String::new(),
            stderr: if failed {
                "filesystem resize failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    let partial = report
        .partial_execution_recovery
        .as_ref()
        .expect("partial execution recovery should be reported");
    assert_eq!(
        partial.completed_action_ids,
        vec!["volumes:vg0/root:grow".to_string()]
    );
    assert_eq!(partial.failed_action_id, "filesystem:root:grow");
    assert_eq!(partial.failed_phase, ExecutionPhase::Command);
    assert_eq!(partial.failed_command[0], "resize2fs");
    assert_eq!(
        partial.retry_review_action_ids,
        vec!["filesystem:root:grow".to_string()]
    );
    assert!(partial.remaining_action_ids.is_empty());
    assert_eq!(partial.completed_mutating_command_count, 1);
    assert!(partial
        .notes
        .iter()
        .any(|note| { note.contains("completed actions") && note.contains("fresh topology") }));

    let json = serde_json::to_value(&report).expect("report serializes");
    assert_eq!(
        json["partialExecutionRecovery"]["completedActionIds"][0],
        "volumes:vg0/root:grow"
    );
    assert_eq!(
        json["partialExecutionRecovery"]["failedActionId"],
        "filesystem:root:grow"
    );
    assert_eq!(
        json["partialExecutionRecovery"]["retryReviewActionIds"][0],
        "filesystem:root:grow"
    );
}
