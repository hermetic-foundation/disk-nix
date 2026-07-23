#[test]
fn lun_detach_without_stable_path_reports_unresolved_input() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "luns": {
                "iqn.2026-06.example:storage/old:1": {
                  "destroy": true
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
        step.action_id == "luns:iqn.2026-06.example:storage/old:1:destroy"
            && step.commands.iter().any(|command| {
                command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["stable LUN device path"]
            })
    }));
    assert_eq!(report.command_summary.needs_domain_implementation_count, 1);
}

#[test]
fn iscsi_session_lifecycle_reports_login_and_logout_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "iscsiSessions": {
                "iqn.2026-06.example:storage.root": {
                  "operation": "login",
                  "portal": "192.0.2.10:3260"
                },
                "iqn.2026-06.example:storage.old": {
                  "operation": "logout",
                  "metadata": {
                    "portal": "192.0.2.11:3260"
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
        step.commands.iter().any(|command| {
            command.argv
                == [
                    "iscsiadm",
                    "--mode",
                    "discovery",
                    "--type",
                    "sendtargets",
                    "--portal",
                    "192.0.2.10:3260",
                ]
                && command.mutates
        })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv
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
                && command.mutates
                && command.readiness == CommandReadiness::Ready
        })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv
                == [
                    "iscsiadm",
                    "--mode",
                    "node",
                    "--targetname",
                    "iqn.2026-06.example:storage.old",
                    "--portal",
                    "192.0.2.11:3260",
                    "--logout",
                ]
                && command.mutates
        })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "iscsisessions:iqn.2026-06.example:storage.root:login"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["iscsiadm", "--mode", "session"])
    }));
}

#[test]
fn iscsi_session_login_without_portal_reports_unresolved_input() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "iscsiSessions": {
                "iqn.2026-06.example:storage.root": {
                  "operation": "create"
                }
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv
                == [
                    "iscsiadm",
                    "--mode",
                    "node",
                    "--targetname",
                    "iqn.2026-06.example:storage.root",
                    "--portal",
                    "<portal>",
                    "--login",
                ]
                && command.readiness == CommandReadiness::NeedsDomainImplementation
                && command.unresolved_inputs == ["iSCSI portal"]
        })
    }));
    assert!(!report.command_summary.all_commands_ready());
}

#[test]
fn pool_actions_report_domain_specific_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "pools": {
                "newtank": {
                  "operation": "create",
                  "device": "/dev/disk/by-id/new-pool-vdev"
                },
                "mirrorpool": {
                  "operation": "create",
                  "devices": [
                    "mirror",
                    "/dev/disk/by-id/mirror-a",
                    "/dev/disk/by-id/mirror-b"
                  ],
                  "properties": {
                    "ashift": "12",
                    "autotrim": "on",
                    "compression": "zstd",
                    "com.sun:auto-snapshot": "false"
                  }
                },
                "tank": {
                  "operation": "rebalance",
                  "mountpoint": "/mnt/tank",
                  "addDevices": ["/dev/disk/by-id/new"],
                  "properties": {
                    "autotrim": "on",
                    "compression": "zstd",
                    "com.sun:auto-snapshot": "false"
                  }
                },
                "vault": {
                  "operation": "import",
                  "readOnly": true
                },
                "moveme": {
                  "operation": "export"
                },
                "oldtank": {
                  "destroy": true
                }
              },
              "snapshots": {
                "tank/home@before": {
                  "target": "tank/home"
                },
                "/mnt/persist/@home-before": {
                  "target": "/mnt/persist/@home",
                  "readOnly": true
                }
              },
              "apply": {
                "allowGrow": true,
                "allowDestructive": true,
                "allowOffline": true,
                "allowPropertyChanges": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv
                == [
                    "zpool",
                    "create",
                    "newtank",
                    "/dev/disk/by-id/new-pool-vdev",
                ]
                && command.readiness == CommandReadiness::Ready
        })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv
                == [
                    "zpool",
                    "create",
                    "-o",
                    "ashift=12",
                    "-o",
                    "autotrim=on",
                    "-O",
                    "com.sun:auto-snapshot=false",
                    "-O",
                    "compression=zstd",
                    "mirrorpool",
                    "mirror",
                    "/dev/disk/by-id/mirror-a",
                    "/dev/disk/by-id/mirror-b",
                ]
                && command.readiness == CommandReadiness::Ready
        })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "pools:mirrorpool:create"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "/dev/disk/by-id/mirror-a"])
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "/dev/disk/by-id/mirror-b"])
            && !step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "mirror"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv == ["zpool", "add", "tank", "/dev/disk/by-id/new"]
                && command.readiness == CommandReadiness::Ready
        })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["zpool", "set", "autotrim=on", "tank"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["zfs", "set", "compression=zstd", "tank"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["zfs", "set", "com.sun:auto-snapshot=false", "tank"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "pools:vault:import"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["zpool", "import"])
            && step.commands.iter().any(|command| {
                command.argv == ["zpool", "import", "-o", "readonly=on", "vault"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "pools:moveme:export"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["zpool", "status", "-P", "moveme"])
            && step.commands.iter().any(|command| {
                command.argv == ["zpool", "export", "moveme"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["zfs", "snapshot", "tank/home@before"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv
                == [
                    "btrfs",
                    "subvolume",
                    "snapshot",
                    "-r",
                    "/mnt/persist/@home",
                    "/mnt/persist/@home-before",
                ]
        })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["zpool", "destroy", "oldtank"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "pools:newtank:create"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["zpool", "list", "-H", "-p", "newtank"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["zpool", "status", "-P", "tank"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "pools:vault:import"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["zpool", "status", "-P", "vault"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "pools:moveme:export"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "moveme", "--json"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "pools:oldtank:destroy"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "topology", "--json"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
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
        })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv == ["btrfs", "subvolume", "show", "/mnt/persist/@home-before"]
        })
    }));
    assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
    assert!(report.command_summary.all_commands_ready());
}

#[test]
fn cache_lifecycle_reports_bcache_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "caches": {
                  "/dev/bcache0": {
                    "operation": "rescan",
                    "addDevices": ["cache-set-uuid"],
                    "removeDevices": ["cache-set-uuid"],
                    "replaceDevices": {
                      "/dev/disk/by-id/old-cache": "/dev/disk/by-id/new-cache"
                    },
                    "cacheSetUuid": "11111111-2222-3333-4444-555555555555",
                    "properties": {
                      "bcache.cache-mode": "writethrough",
                      "bcache.set-journal-delay-ms": "100"
                    }
                  }
                }
              },
              "apply": {
                "allowDeviceReplacement": true,
                "allowOffline": true,
                "allowPropertyChanges": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv
                == [
                    "sh",
                    "-c",
                    "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/attach\"",
                    "disk-nix-bcache-attach",
                    "/dev/bcache0",
                    "cache-set-uuid",
                ]
        })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "caches:/dev/bcache0:rescan"
            && step
                .commands
                .iter()
                .all(|command| command.readiness == CommandReadiness::Ready && !command.mutates)
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "/dev/bcache0"])
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "sh",
                        "-c",
                        "cat \"/sys/block/${1#/dev/}/bcache/$2\"",
                        "disk-nix-bcache-read",
                        "/dev/bcache0",
                        "state",
                    ]
            })
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "sh",
                        "-c",
                        "cat \"/sys/block/${1#/dev/}/bcache/$2\"",
                        "disk-nix-bcache-read",
                        "/dev/bcache0",
                        "cache_mode",
                    ]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv
                == [
                    "sh",
                    "-c",
                    "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/$3\"",
                    "disk-nix-bcache-property",
                    "/dev/bcache0",
                    "writethrough",
                    "cache_mode",
                ]
        })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "caches:/dev/bcache0:set-property:bcache.set-journal-delay-ms"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "sh",
                        "-c",
                        "printf '%s\\n' \"$2\" > \"/sys/fs/bcache/$1/$3\"",
                        "disk-nix-bcache-set-property",
                        "11111111-2222-3333-4444-555555555555",
                        "100",
                        "journal_delay_ms",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv
                    == [
                        "sh",
                        "-c",
                        "make-bcache -C \"$2\" --cset-uuid \"$3\" --writeback && printf '1\\n' > \"/sys/block/${1#/dev/}/bcache/detach\" && printf '%s\\n' \"$3\" > \"/sys/block/${1#/dev/}/bcache/attach\"",
                        "disk-nix-bcache-replace",
                        "/dev/bcache0",
                        "/dev/disk/by-id/new-cache",
                        "11111111-2222-3333-4444-555555555555",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
        }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "caches:/dev/bcache0:remove-device:cache-set-uuid"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "sh",
                        "-c",
                        "printf '1\\n' > \"/sys/block/${1#/dev/}/bcache/detach\"",
                        "disk-nix-bcache-detach",
                        "/dev/bcache0",
                    ]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv
                == [
                    "sh",
                    "-c",
                    "cat \"/sys/block/${1#/dev/}/bcache/$2\"",
                    "disk-nix-bcache-read",
                    "/dev/bcache0",
                    "dirty_data",
                ]
        })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "caches:/dev/bcache0:rescan"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "/dev/bcache0", "--json"])
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "sh",
                        "-c",
                        "cat \"/sys/block/${1#/dev/}/bcache/$2\"",
                        "disk-nix-bcache-read",
                        "/dev/bcache0",
                        "dirty_data",
                    ]
            })
    }));
}

#[test]
fn cache_replacement_requires_declared_cache_set_uuid() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "caches": {
                  "/dev/bcache0": {
                    "operation": "replace-device",
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
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(!report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
            step.action_id == "caches:/dev/bcache0:replace-device:/dev/disk/by-id/old-cache"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "sh",
                            "-c",
                            "make-bcache -C \"$2\" --cset-uuid \"$3\" --writeback && printf '1\\n' > \"/sys/block/${1#/dev/}/bcache/detach\" && printf '%s\\n' \"$3\" > \"/sys/block/${1#/dev/}/bcache/attach\"",
                            "disk-nix-bcache-replace",
                            "/dev/bcache0",
                            "/dev/disk/by-id/new-cache",
                            "<new-cache-set-uuid>",
                        ]
                        && command.readiness == CommandReadiness::NeedsDomainImplementation
                        && command.unresolved_inputs == ["new cache-set UUID"]
                })
        }));
}

#[test]
fn cache_lifecycle_uses_declared_bcache_target_for_logical_names() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "caches": {
                  "cache0": {
                    "device": "/dev/bcache0",
                    "operation": "rescan",
                    "addDevices": ["cache-set-uuid"],
                    "removeDevices": ["cache-set-uuid"],
                    "cacheSetUuid": "cache-set-uuid",
                    "properties": {
                      "bcache.cache-mode": "writethrough",
                      "bcache.set-journal-delay-ms": "100"
                    }
                  }
                }
              },
              "apply": {
                "allowDeviceReplacement": true,
                "allowOffline": true,
                "allowPropertyChanges": true,
                "allowPotentialDataLoss": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "caches:cache0:add-device:cache-set-uuid"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "sh",
                        "-c",
                        "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/attach\"",
                        "disk-nix-bcache-attach",
                        "/dev/bcache0",
                        "cache-set-uuid",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "caches:cache0:set-property:bcache.cache-mode"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "sh",
                        "-c",
                        "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/$3\"",
                        "disk-nix-bcache-property",
                        "/dev/bcache0",
                        "writethrough",
                        "cache_mode",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "caches:cache0:set-property:bcache.set-journal-delay-ms"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "sh",
                        "-c",
                        "printf '%s\\n' \"$2\" > \"/sys/fs/bcache/$1/$3\"",
                        "disk-nix-bcache-set-property",
                        "cache-set-uuid",
                        "100",
                        "journal_delay_ms",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "caches:cache0:remove-device:cache-set-uuid"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "sh",
                        "-c",
                        "printf '1\\n' > \"/sys/block/${1#/dev/}/bcache/detach\"",
                        "disk-nix-bcache-detach",
                        "/dev/bcache0",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "caches:cache0:rescan"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "/dev/bcache0"])
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "sh",
                        "-c",
                        "cat \"/sys/block/${1#/dev/}/bcache/$2\"",
                        "disk-nix-bcache-read",
                        "/dev/bcache0",
                        "dirty_data",
                    ]
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "caches:cache0:add-device:cache-set-uuid"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "/dev/bcache0", "--json"])
    }));
}

#[test]
fn cache_lifecycle_accepts_path_aliases_for_logical_names() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "caches": {
                  "writeback-cache": {
                    "path": "/dev/bcache1",
                    "operation": "rescan",
                    "addDevices": ["cache-set-uuid"],
                    "removeDevices": ["cache-set-uuid"],
                    "properties": {
                      "bcache.cache-mode": "writearound"
                    }
                  }
                }
              },
              "apply": {
                "allowDeviceReplacement": true,
                "allowOffline": true,
                "allowPropertyChanges": true,
                "allowPotentialDataLoss": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "caches:writeback-cache:add-device:cache-set-uuid"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "sh",
                        "-c",
                        "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/attach\"",
                        "disk-nix-bcache-attach",
                        "/dev/bcache1",
                        "cache-set-uuid",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "caches:writeback-cache:set-property:bcache.cache-mode"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "sh",
                        "-c",
                        "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/$3\"",
                        "disk-nix-bcache-property",
                        "/dev/bcache1",
                        "writearound",
                        "cache_mode",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "caches:writeback-cache:remove-device:cache-set-uuid"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "sh",
                        "-c",
                        "printf '1\\n' > \"/sys/block/${1#/dev/}/bcache/detach\"",
                        "disk-nix-bcache-detach",
                        "/dev/bcache1",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "caches:writeback-cache:rescan"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "/dev/bcache1"])
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "sh",
                        "-c",
                        "cat \"/sys/block/${1#/dev/}/bcache/$2\"",
                        "disk-nix-bcache-read",
                        "/dev/bcache1",
                        "dirty_data",
                    ]
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "caches:writeback-cache:set-property:bcache.cache-mode"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "sh",
                        "-c",
                        "cat \"/sys/block/${1#/dev/}/bcache/$2\"",
                        "disk-nix-bcache-read",
                        "/dev/bcache1",
                        "dirty_data",
                    ]
            })
    }));
}

#[test]
fn lvm_cache_lifecycle_reports_lvm_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "lvmCaches": {
                  "vg0/root": {
                    "operation": "create",
                    "device": "vg0/root-cache",
                    "addDevices": ["vg0/root-cache"],
                    "removeDevices": ["vg0/root-cache"],
                    "replaceDevices": {
                      "vg0/root-cache": "vg0/root-cache-new"
                    },
                    "properties": {
                      "lvm.cache-mode": "writethrough"
                    },
                    "destroy": true
                  },
                  "vg0/archive": {
                    "operation": "rescan"
                  }
                }
              },
              "apply": {
                "allowOffline": true,
                "allowDeviceReplacement": true,
                "confirmation": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "lvmcaches:vg0/root:create"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "lvconvert",
                        "--type",
                        "cache",
                        "--cachepool",
                        "vg0/root-cache",
                        "vg0/root",
                    ]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "lvmCaches:vg0/root:add-device:vg0/root-cache"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "lvconvert",
                        "--type",
                        "cache",
                        "--cachepool",
                        "vg0/root-cache",
                        "vg0/root",
                    ]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "lvmCaches:vg0/root:set-property:lvm.cache-mode"
            && step.commands.iter().any(|command| {
                command.argv == ["lvchange", "--cachemode", "writethrough", "vg0/root"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "lvmCaches:vg0/root:remove-device:vg0/root-cache"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["lvconvert", "--uncache", "vg0/root"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "lvmcaches:vg0/root:destroy"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["lvconvert", "--uncache", "vg0/root"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "lvmcaches:vg0/archive:rescan"
            && step
                .commands
                .iter()
                .all(|command| command.readiness == CommandReadiness::Ready && !command.mutates)
            && step.commands.iter().any(|command| {
                command.argv.len() >= 6
                    && command.argv[0] == "lvs"
                    && command.argv[1] == "--reportformat"
                    && command.argv[2] == "json"
                    && command.argv[3] == "-o"
                    && command.argv[5] == "vg0/archive"
            })
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "vg0/archive"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "lvmCaches:vg0/root:set-property:lvm.cache-mode"
            && step.commands.iter().any(|command| {
                command.argv.len() >= 4
                    && command.argv[0] == "lvs"
                    && command.argv[1] == "--reportformat"
                    && command.argv[2] == "json"
                    && command.argv[3] == "-o"
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "lvmcaches:vg0/archive:rescan"
            && step.commands.iter().any(|command| {
                command.argv.len() >= 6
                    && command.argv[0] == "lvs"
                    && command.argv[1] == "--reportformat"
                    && command.argv[2] == "json"
                    && command.argv[3] == "-o"
                    && command.argv[5] == "vg0/archive"
            })
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "vg0/archive", "--json"])
    }));
}
