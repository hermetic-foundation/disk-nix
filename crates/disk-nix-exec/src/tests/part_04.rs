#[test]
fn disk_initialization_requires_destructive_policy_and_renders_mklabel() {
    let (blocked_plan, blocked_policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "disks": {
                  "/dev/disk/by-id/nvme-root": {
                    "operation": "create",
                    "partitionType": "gpt"
                  }
                }
              }
            }"#,
    )
    .expect("document parses");

    let blocked = prepare_execution(&blocked_plan, blocked_policy, ExecutionMode::DryRun);

    assert_eq!(blocked.status, ExecutionStatus::Blocked);
    assert!(blocked.command_plan.is_empty());

    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "disks": {
                  "/dev/disk/by-id/nvme-root": {
                    "operation": "create",
                    "partitionType": "gpt"
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
    assert_eq!(report.command_plan.len(), 1);
    assert!(report.command_plan[0].requires_manual_review);
    assert!(report.command_plan[0].commands.iter().any(|command| {
        command.argv
            == [
                "parted",
                "-s",
                "/dev/disk/by-id/nvme-root",
                "mklabel",
                "gpt",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(report.command_plan[0].commands.iter().any(|command| {
        command.argv == ["partprobe", "/dev/disk/by-id/nvme-root"] && command.mutates
    }));
    assert!(report.verification_plan[0].commands.iter().any(|command| {
        command.argv == ["parted", "-lm", "/dev/disk/by-id/nvme-root"] && !command.mutates
    }));

    let (raw_zfs_plan, raw_zfs_policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "disks": {
                  "/dev/disk/by-id/raw-zfs": {
                    "operation": "create",
                    "partitionType": "zfs"
                  }
                }
              },
              "apply": {
                "allowDestructive": true
              }
            }"#,
    )
    .expect("document parses");

    let raw_zfs = prepare_execution(&raw_zfs_plan, raw_zfs_policy, ExecutionMode::DryRun);

    assert_eq!(raw_zfs.status, ExecutionStatus::DryRun);
    assert!(raw_zfs.command_plan[0].commands.iter().any(|command| {
        command.argv == ["wipefs", "--all", "--force", "/dev/disk/by-id/raw-zfs"]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(!raw_zfs.command_plan[0].commands.iter().any(|command| {
        command.argv == ["parted", "-s", "/dev/disk/by-id/raw-zfs", "mklabel", "zfs"]
    }));
}

#[test]
fn disk_initialization_requires_stable_disk_path_for_execute_readiness() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "disks": {
                  "root": {
                    "operation": "create",
                    "partitionType": "gpt"
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
    assert!(report.command_plan[0].commands.iter().any(|command| {
        command.argv == ["disk-nix", "inspect", "<disk>"]
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command.unresolved_inputs == ["disk path"]
    }));
    assert!(report.command_plan[0].commands.iter().any(|command| {
        command.argv == ["parted", "-s", "<disk>", "mklabel", "gpt"]
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command.unresolved_inputs == ["disk path"]
    }));
    assert!(report.command_plan[0].commands.iter().any(|command| {
        command.argv == ["partprobe", "<disk>"]
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command.unresolved_inputs == ["disk path"]
    }));
    assert!(report.command_plan[0].commands.iter().any(|command| {
        command.argv == ["parted", "-lm", "<disk>"]
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command.unresolved_inputs == ["disk path"]
    }));
}

#[test]
fn partition_creation_reports_reviewable_commands_when_offline_allowed() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "partitions": {
                  "root": {
                    "operation": "create",
                    "device": "/dev/disk/by-id/nvme-root",
                    "start": "1MiB",
                    "end": "100%",
                    "partitionType": "linux"
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
    assert_eq!(report.command_plan.len(), 1);
    assert!(report.command_plan[0].commands.iter().any(|command| {
        command.argv
            == [
                "parted",
                "-s",
                "/dev/disk/by-id/nvme-root",
                "mkpart",
                "linux",
                "1MiB",
                "100%",
            ]
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(report.command_plan[0].commands.iter().any(|command| {
        command.argv == ["blockdev", "--rereadpt", "/dev/disk/by-id/nvme-root"]
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(report.verification_plan[0]
        .commands
        .iter()
        .any(|command| command.argv == ["parted", "-lm"]));
}

#[test]
fn partition_creation_requires_disk_and_stable_partition_path_for_execute_readiness() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "partitions": {
                  "root": {
                    "operation": "create",
                    "start": "1MiB",
                    "end": "100%",
                    "partitionType": "linux"
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
    assert!(report.command_plan[0].commands.iter().any(|command| {
        command.argv == ["disk-nix", "inspect", "<disk>"]
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command.unresolved_inputs == ["disk path"]
    }));
    assert!(report.command_plan[0].commands.iter().any(|command| {
        command.argv == ["parted", "-s", "<disk>", "mkpart", "linux", "1MiB", "100%"]
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command.unresolved_inputs == ["disk path"]
    }));
    assert!(report.command_plan[0].commands.iter().any(|command| {
        command.argv == ["disk-nix", "inspect", "<partition>"]
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command.unresolved_inputs == ["partition path"]
    }));
}

#[test]
fn partition_growth_uses_partition_number_for_resizepart() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "partitions": {
                  "root": {
                    "operation": "grow",
                    "device": "/dev/disk/by-id/nvme-root",
                    "partitionNumber": 2,
                    "end": "100%"
                  }
                }
              },
              "apply": {
                "allowOffline": true,
                "allowGrow": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert_eq!(report.command_plan.len(), 1);
    assert!(report.command_plan[0].commands.iter().any(|command| {
        command.argv
            == [
                "parted",
                "-s",
                "/dev/disk/by-id/nvme-root",
                "resizepart",
                "2",
                "100%",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(report.command_plan[0].commands.iter().any(|command| {
        command.argv == ["blockdev", "--rereadpt", "/dev/disk/by-id/nvme-root"]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
}

#[test]
fn partition_table_rescan_reports_partprobe_and_rereadpt_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "disks": {
                  "/dev/disk/by-id/nvme-data": {
                    "operation": "rescan"
                  }
                },
                "partitions": {
                  "data-table": {
                    "operation": "rescan",
                    "device": "/dev/disk/by-id/nvme-data"
                  }
                }
              },
              "apply": {}
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert_eq!(report.command_plan.len(), 2);
    assert!(report.command_summary.all_commands_ready());
    for action_id in [
        "disks:/dev/disk/by-id/nvme-data:rescan",
        "partitions:data-table:rescan",
    ] {
        assert!(report.command_plan.iter().any(|step| {
            step.action_id == action_id
                && step.commands.iter().any(|command| {
                    command.argv == ["partprobe", "/dev/disk/by-id/nvme-data"]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
                && step.commands.iter().any(|command| {
                    command.argv == ["blockdev", "--rereadpt", "/dev/disk/by-id/nvme-data"]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
    }
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "partitions:data-table:rescan"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["parted", "-lm", "/dev/disk/by-id/nvme-data"])
    }));
}

#[test]
fn partition_table_rescan_requires_disk_path_for_execute_readiness() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "partitions": {
                  "data-table": {
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
    assert!(report.command_plan[0].commands.iter().any(|command| {
        command.argv == ["partprobe", "<disk>"]
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command.unresolved_inputs == ["disk path"]
    }));
    assert!(report.command_plan[0].commands.iter().any(|command| {
        command.argv == ["blockdev", "--rereadpt", "<disk>"]
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command.unresolved_inputs == ["disk path"]
    }));
}

#[test]
fn luks_keyslot_lifecycle_reports_cryptsetup_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "luksKeyslots": {
                  "cryptroot:1": {
                    "operation": "add-key",
                    "device": "/dev/disk/by-id/root-luks",
                    "metadata": {
                      "keySlot": "1",
                      "newKeyFile": "/run/keys/root-new"
                    }
                  },
                  "cryptroot:3": {
                    "properties": {
                      "keyFile": "/run/keys/root-rotated"
                    },
                    "device": "/dev/disk/by-id/root-luks",
                    "metadata": {
                      "keySlot": "3",
                      "keyFile": "/run/keys/root-old"
                    }
                  },
                  "cryptroot:4": {
                    "properties": {
                      "priority": "prefer"
                    },
                    "device": "/dev/disk/by-id/root-luks",
                    "metadata": {
                      "keySlot": "4"
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
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "lukskeyslots:cryptroot:1:add-key"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "cryptsetup",
                        "luksAddKey",
                        "--key-slot",
                        "1",
                        "/dev/disk/by-id/root-luks",
                        "/run/keys/root-new",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "luksKeyslots:cryptroot:3:set-property:keyFile"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "cryptsetup",
                        "luksChangeKey",
                        "--key-slot",
                        "3",
                        "--key-file",
                        "/run/keys/root-old",
                        "/dev/disk/by-id/root-luks",
                        "/run/keys/root-rotated",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "luksKeyslots:cryptroot:4:set-property:priority"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "cryptsetup",
                        "config",
                        "/dev/disk/by-id/root-luks",
                        "--key-slot",
                        "4",
                        "--priority",
                        "prefer",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "lukskeyslots:cryptroot:1:add-key"
            && step.commands.iter().any(|command| {
                command.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/root-luks"]
            })
    }));
}

#[test]
fn luks_keyslot_priority_reports_missing_inputs_for_execute_readiness() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "luksKeyslots": {
                  "root-priority": {
                    "properties": {
                      "priority": "normal"
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
    assert!(!report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "luksKeyslots:root-priority:set-property:priority"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "cryptsetup",
                        "config",
                        "<luks-device>",
                        "--key-slot",
                        "<key-slot>",
                        "--priority",
                        "normal",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["LUKS backing device", "LUKS keyslot number"]
            })
    }));
}

#[test]
fn luks_keyslot_lifecycle_reports_missing_inputs_for_execute_readiness() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "luksKeyslots": {
                  "root-add": {
                    "operation": "add-key"
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
        step.action_id == "lukskeyslots:root-add:add-key"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "cryptsetup",
                        "luksAddKey",
                        "<luks-device>",
                        "<new-key-file>",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["LUKS backing device", "new key file"]
            })
    }));
}

#[test]
fn luks_keyslot_destroy_renderer_uses_cryptsetup_kill_slot() {
    let action = PlannedAction {
        id: "lukskeyslots:cryptroot:2:destroy".to_string(),
        description: "remove LUKS keyslot".to_string(),
        operation: Operation::Destroy,
        risk: RiskClass::PotentialDataLoss,
        destructive: false,
        context: ActionContext {
            collection: Some("luksKeyslots".to_string()),
            name: Some("cryptroot:2".to_string()),
            device: Some("/dev/disk/by-id/root-luks".to_string()),
            key_slot: Some("2".to_string()),
            ..ActionContext::default()
        },
        advice: None,
    };

    let (commands, _, requires_manual_review) = commands_for_action(&action);

    assert!(requires_manual_review);
    assert!(commands.iter().any(|command| {
        command.argv
            == [
                "cryptsetup",
                "luksKillSlot",
                "/dev/disk/by-id/root-luks",
                "2",
            ]
            && command.readiness == CommandReadiness::Ready
    }));
}

#[test]
fn luks_token_lifecycle_reports_cryptsetup_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "luksTokens": {
                  "cryptroot:0": {
                    "operation": "import-token",
                    "device": "/dev/disk/by-id/root-luks",
                    "metadata": {
                      "tokenId": "0",
                      "tokenFile": "/run/keys/root-token.json"
                    }
                  },
                  "cryptroot:2": {
                    "properties": {
                      "tokenFile": "/run/keys/root-token-new.json"
                    },
                    "device": "/dev/disk/by-id/root-luks",
                    "metadata": {
                      "tokenId": "2"
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
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "lukstokens:cryptroot:0:import-token"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "cryptsetup",
                        "token",
                        "import",
                        "--token-id",
                        "0",
                        "--json-file",
                        "/run/keys/root-token.json",
                        "/dev/disk/by-id/root-luks",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "luksTokens:cryptroot:2:set-property:tokenFile"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "cryptsetup",
                        "token",
                        "import",
                        "--token-id",
                        "2",
                        "--json-file",
                        "/run/keys/root-token-new.json",
                        "/dev/disk/by-id/root-luks",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "lukstokens:cryptroot:0:import-token"
            && step.commands.iter().any(|command| {
                command.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/root-luks"]
            })
    }));
}

#[test]
fn luks_token_lifecycle_reports_missing_inputs_for_execute_readiness() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "luksTokens": {
                  "root-token": {
                    "operation": "import-token"
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
        step.action_id == "lukstokens:root-token:import-token"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "cryptsetup",
                        "token",
                        "import",
                        "--json-file",
                        "<token-json-file>",
                        "<luks-device>",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["LUKS backing device", "token JSON file"]
            })
    }));
}

#[test]
fn luks_token_destroy_renderer_uses_cryptsetup_token_remove() {
    let action = PlannedAction {
        id: "lukstokens:cryptroot:1:destroy".to_string(),
        description: "remove LUKS token".to_string(),
        operation: Operation::Destroy,
        risk: RiskClass::PotentialDataLoss,
        destructive: false,
        context: ActionContext {
            collection: Some("luksTokens".to_string()),
            name: Some("cryptroot:1".to_string()),
            device: Some("/dev/disk/by-id/root-luks".to_string()),
            token_id: Some("1".to_string()),
            ..ActionContext::default()
        },
        advice: None,
    };

    let (commands, _, requires_manual_review) = commands_for_action(&action);

    assert!(requires_manual_review);
    assert!(commands.iter().any(|command| {
        command.argv
            == [
                "cryptsetup",
                "token",
                "remove",
                "--token-id",
                "1",
                "/dev/disk/by-id/root-luks",
            ]
            && command.readiness == CommandReadiness::Ready
    }));
}

#[test]
fn luks_keyslot_and_token_lifecycle_accept_metadata_for_logical_names() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "luksKeyslots": {
                  "rootAdd": {
                    "operation": "add-key",
                    "device": "/dev/disk/by-id/root-luks",
                    "keySlot": "4",
                    "newKeyFile": "/run/keys/root-new"
                  },
                  "rootRotate": {
                    "device": "/dev/disk/by-id/root-luks",
                    "key-slot": "5",
                    "key-file": "/run/keys/root-old",
                    "properties": {
                      "keyFile": "/run/keys/root-rotated"
                    }
                  },
                  "rootRemove": {
                    "operation": "remove-key",
                    "device": "/dev/disk/by-id/root-luks",
                    "slot": "6"
                  }
                },
                "luksTokens": {
                  "rootImport": {
                    "operation": "import-token",
                    "device": "/dev/disk/by-id/root-luks",
                    "tokenId": "7",
                    "tokenFile": "/run/keys/root-token.json"
                  },
                  "rootRotate": {
                    "device": "/dev/disk/by-id/root-luks",
                    "token-id": "8",
                    "properties": {
                      "tokenFile": "/run/keys/root-token-rotated.json"
                    }
                  },
                  "rootRemove": {
                    "operation": "remove-token",
                    "device": "/dev/disk/by-id/root-luks",
                    "token": "9"
                  }
                }
              },
              "apply": {
                "allowOffline": true,
                "allowPotentialDataLoss": true,
                "requireBackup": false,
                "requireConfirmation": false
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "lukskeyslots:rootadd:add-key"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "cryptsetup",
                        "luksAddKey",
                        "--key-slot",
                        "4",
                        "/dev/disk/by-id/root-luks",
                        "/run/keys/root-new",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "luksKeyslots:rootRotate:set-property:keyFile"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "cryptsetup",
                        "luksChangeKey",
                        "--key-slot",
                        "5",
                        "--key-file",
                        "/run/keys/root-old",
                        "/dev/disk/by-id/root-luks",
                        "/run/keys/root-rotated",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "lukskeyslots:rootremove:remove-key"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "cryptsetup",
                        "luksKillSlot",
                        "/dev/disk/by-id/root-luks",
                        "6",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "lukstokens:rootimport:import-token"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "cryptsetup",
                        "token",
                        "import",
                        "--token-id",
                        "7",
                        "--json-file",
                        "/run/keys/root-token.json",
                        "/dev/disk/by-id/root-luks",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "luksTokens:rootRotate:set-property:tokenFile"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "cryptsetup",
                        "token",
                        "import",
                        "--token-id",
                        "8",
                        "--json-file",
                        "/run/keys/root-token-rotated.json",
                        "/dev/disk/by-id/root-luks",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "lukstokens:rootremove:remove-token"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "cryptsetup",
                        "token",
                        "remove",
                        "--token-id",
                        "9",
                        "/dev/disk/by-id/root-luks",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "lukskeyslots:rootremove:remove-key"
            && step.commands.iter().any(|command| {
                command.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/root-luks"]
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "lukstokens:rootremove:remove-token"
            && step.commands.iter().any(|command| {
                command.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/root-luks"]
            })
    }));
}
