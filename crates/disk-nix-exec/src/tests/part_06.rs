#[test]
fn vdo_lifecycle_reports_vdo_commands_and_verification() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "vdoVolumes": {
                  "new-cache": {
                    "operation": "create",
                    "device": "/dev/disk/by-id/vdo-backing",
                    "desiredSize": "2TiB"
                  },
                  "archive": {
                    "operation": "grow",
                    "desiredSize": "4TiB",
                    "properties": {
                      "writePolicy": "sync",
                      "compression": "enabled",
                      "deduplication": "disabled"
                    }
                  },
                  "archive-physical": {
                    "operation": "grow",
                    "physicalSize": "6TiB"
                  },
                  "warmArchive": {
                    "operation": "start"
                  },
                  "coldArchive": {
                    "operation": "stop"
                  },
                  "refreshArchive": {
                    "operation": "rescan"
                  },
                  "missing-backing": {
                    "operation": "create",
                    "desiredSize": "1TiB"
                  },
                  "old-cache": {
                    "destroy": true
                  }
                }
              },
              "apply": {
                "allowGrow": true,
                "allowOffline": true,
                "allowDestructive": true
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
                    "vdo",
                    "create",
                    "--name",
                    "new-cache",
                    "--device",
                    "/dev/disk/by-id/vdo-backing",
                    "--vdoLogicalSize",
                    "2TiB",
                ]
                && command.readiness == CommandReadiness::Ready
        })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "vdovolumes:missing-backing:create"
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "<backing-device>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["backing device"]
            })
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "vdo",
                        "create",
                        "--name",
                        "missing-backing",
                        "--device",
                        "<backing-device>",
                        "--vdoLogicalSize",
                        "1TiB",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["backing device"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv
                == [
                    "vdo",
                    "growLogical",
                    "--name",
                    "archive",
                    "--vdoLogicalSize",
                    "4TiB",
                ]
                && command.readiness == CommandReadiness::Ready
        })
    }));
    assert!(
        !report.command_plan.iter().any(|step| {
            step.action_id == "vdovolumes:archive:grow"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["vdo", "growPhysical", "--name", "archive"])
        }),
        "logical-only VDO growth must not grow physical capacity implicitly"
    );
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "vdovolumes:archive-physical:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["vdo", "growPhysical", "--name", "archive-physical"]
                    && command.readiness == CommandReadiness::Ready
            })
            && !step.commands.iter().any(|command| {
                command
                    .argv
                    .starts_with(&["vdo".to_string(), "growLogical".to_string()])
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "vdoVolumes:archive:set-property:writePolicy"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "vdo",
                        "changeWritePolicy",
                        "--name",
                        "archive",
                        "--writePolicy",
                        "sync",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "vdoVolumes:archive:set-property:compression"
            && step.commands.iter().any(|command| {
                command.argv == ["vdo", "enableCompression", "--name", "archive"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "vdoVolumes:archive:set-property:deduplication"
            && step.commands.iter().any(|command| {
                command.argv == ["vdo", "disableDeduplication", "--name", "archive"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv == ["vdo", "start", "--name", "warmArchive"]
                && command.readiness == CommandReadiness::Ready
        })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv == ["vdo", "stop", "--name", "coldArchive"]
                && command.readiness == CommandReadiness::Ready
        })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "vdovolumes:refresharchive:rescan"
            && step
                .commands
                .iter()
                .all(|command| command.readiness == CommandReadiness::Ready && !command.mutates)
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["vdo", "status", "--name", "refreshArchive"])
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["vdostats", "--human-readable", "refreshArchive"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv == ["vdo", "remove", "--name", "old-cache"]
                && command.readiness == CommandReadiness::Ready
        })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "vdovolumes:new-cache:create"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["vdo", "status", "--name", "new-cache"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["vdostats", "--human-readable", "archive"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "vdovolumes:warmarchive:start"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["vdo", "status", "--name", "warmArchive"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "vdovolumes:coldarchive:stop"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["vdo", "status"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "vdovolumes:refresharchive:rescan"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "refreshArchive", "--json"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "vdoVolumes:archive:set-property:writePolicy"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["vdostats", "--verbose", "archive"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "vdovolumes:old-cache:destroy"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["vdo", "status"])
    }));
}

#[test]
fn vdo_lifecycle_accepts_targets_for_logical_names() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "vdoVolumes": {
                  "newArchive": {
                    "target": "archive-vdo",
                    "operation": "create",
                    "device": "/dev/disk/by-id/vdo-backing",
                    "desiredSize": "2TiB"
                  },
                  "archiveGrow": {
                    "target": "archive-vdo",
                    "operation": "grow",
                    "desiredSize": "4TiB",
                    "physicalSize": "6TiB",
                    "properties": {
                      "writePolicy": "sync",
                      "compression": "disabled",
                      "deduplication": "enabled"
                    }
                  },
                  "archiveStart": {
                    "target": "archive-vdo",
                    "operation": "start"
                  },
                  "archiveStop": {
                    "target": "archive-vdo",
                    "operation": "stop"
                  },
                  "archiveRefresh": {
                    "target": "archive-vdo",
                    "operation": "rescan"
                  },
                  "archiveRemove": {
                    "target": "archive-vdo",
                    "operation": "destroy"
                  }
                }
              },
              "apply": {
                "allowGrow": true,
                "allowOffline": true,
                "allowDestructive": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "vdovolumes:newarchive:create"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "vdo",
                        "create",
                        "--name",
                        "archive-vdo",
                        "--device",
                        "/dev/disk/by-id/vdo-backing",
                        "--vdoLogicalSize",
                        "2TiB",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "vdovolumes:archivegrow:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["vdo", "growPhysical", "--name", "archive-vdo"]
                    && command.readiness == CommandReadiness::Ready
                    && command.note.contains("6TiB")
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "vdovolumes:archivegrow:grow"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "vdo",
                        "growLogical",
                        "--name",
                        "archive-vdo",
                        "--vdoLogicalSize",
                        "4TiB",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "vdoVolumes:archiveGrow:set-property:writePolicy"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "vdo",
                        "changeWritePolicy",
                        "--name",
                        "archive-vdo",
                        "--writePolicy",
                        "sync",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "vdoVolumes:archiveGrow:set-property:compression"
            && step.commands.iter().any(|command| {
                command.argv == ["vdo", "disableCompression", "--name", "archive-vdo"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "vdoVolumes:archiveGrow:set-property:deduplication"
            && step.commands.iter().any(|command| {
                command.argv == ["vdo", "enableDeduplication", "--name", "archive-vdo"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "vdovolumes:archivestart:start"
            && step.commands.iter().any(|command| {
                command.argv == ["vdo", "start", "--name", "archive-vdo"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "vdovolumes:archivestop:stop"
            && step.commands.iter().any(|command| {
                command.argv == ["vdo", "stop", "--name", "archive-vdo"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "vdovolumes:archiverefresh:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["vdo", "status", "--name", "archive-vdo"]
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["vdostats", "--human-readable", "archive-vdo"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "vdovolumes:archiveremove:destroy"
            && step.commands.iter().any(|command| {
                command.argv == ["vdo", "remove", "--name", "archive-vdo"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
}

#[test]
fn vdo_property_lifecycle_blocks_unsupported_properties_and_values() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "vdoVolumes": {
                  "archive": {
                    "properties": {
                      "writePolicy": "eventual",
                      "compression": "maybe",
                      "indexMemory": "0.5"
                    }
                  }
                }
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::Blocked);
    assert_eq!(report.apply.blocked_summary.unsupported_count, 3);
    assert!(report.command_plan.is_empty());
    assert!(report.apply.blocked.iter().any(|blocked| {
        blocked.id == "vdoVolumes:archive:set-property:writePolicy"
            && blocked.risk == RiskClass::Unsupported
    }));
    assert!(report.apply.blocked.iter().any(|blocked| {
        blocked.id == "vdoVolumes:archive:set-property:compression"
            && blocked.risk == RiskClass::Unsupported
    }));
    assert!(report.apply.blocked.iter().any(|blocked| {
        blocked.id == "vdoVolumes:archive:set-property:indexMemory"
            && blocked.risk == RiskClass::Unsupported
    }));
}

#[test]
fn zfs_snapshot_clone_renderer_reports_reviewable_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "snapshots": {
                "tank/home@before": {
                  "target": "tank/home",
                  "cloneTo": "tank/home-review"
                }
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "snapshot:tank/home@before:clone:tank/home-review"
            && step.commands.iter().any(|command| {
                command.argv == ["zfs", "clone", "tank/home@before", "tank/home-review"]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "snapshot:tank/home@before:clone:tank/home-review"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["zfs", "list", "-H", "-p", "tank/home-review"])
    }));
}

#[test]
fn btrfs_snapshot_clone_renderer_reports_reviewable_commands() {
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

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "snapshot:/mnt/persist/@home-before:clone:/mnt/persist/@home-review"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "btrfs",
                        "subvolume",
                        "snapshot",
                        "-r",
                        "/mnt/persist/@home-before",
                        "/mnt/persist/@home-review",
                    ]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "snapshot:/mnt/persist/@home-before:clone:/mnt/persist/@home-review"
            && step.commands.iter().any(|command| {
                command.argv == ["btrfs", "subvolume", "show", "/mnt/persist/@home-review"]
            })
    }));
}

#[test]
fn rename_lifecycle_reports_domain_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "datasets": {
                  "tank/home": {
                    "operation": "rename",
                    "renameTo": "tank/home-staged"
                  }
                },
                "volumes": {
                  "vg0/old": {
                    "operation": "rename",
                    "renameTo": "vg0/new"
                  }
                },
                "btrfsSubvolumes": {
                  "/mnt/persist/@old": {
                    "operation": "rename",
                    "renameTo": "/mnt/persist/@new"
                  }
                },
                "snapshots": {
                  "tank/home@before-prune": {
                    "target": "tank/home",
                    "renameTo": "tank/home@retained"
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
        step.action_id == "datasets:tank/home:rename"
            && step.commands.iter().any(|command| {
                command.argv == ["zfs", "rename", "tank/home", "tank/home-staged"]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "volumes:vg0/old:rename"
            && step.commands.iter().any(|command| {
                command.argv == ["lvrename", "vg0/old", "vg0/new"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "btrfssubvolumes:/mnt/persist/@old:rename"
            && step.commands.iter().any(|command| {
                command.argv == ["mv", "--", "/mnt/persist/@old", "/mnt/persist/@new"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "snapshot:tank/home@before-prune:rename:tank/home@retained"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "zfs",
                        "rename",
                        "tank/home@before-prune",
                        "tank/home@retained",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
}

#[test]
fn ambiguous_snapshot_rename_reports_unresolved_domain() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "snapshots": {
                  "before-upgrade": {
                    "target": "tank/home",
                    "renameTo": "retained-before-upgrade"
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
        step.action_id == "snapshot:before-upgrade:rename:retained-before-upgrade"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "<snapshot-rename-tool>",
                        "before-upgrade",
                        "retained-before-upgrade",
                    ]
                    && command.mutates
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["ZFS snapshot name or Btrfs snapshot path"]
            })
    }));
    assert_eq!(report.command_summary.needs_domain_implementation_count, 1);
    assert!(!report.command_summary.all_commands_ready());
}

#[test]
fn ambiguous_snapshot_clone_and_rollback_report_unresolved_domain() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "snapshots": {
                  "before-clone": {
                    "target": "tank/home",
                    "cloneTo": "tank/home-review"
                  },
                  "before-rollback": {
                    "target": "tank/home",
                    "rollback": true
                  }
                }
              },
              "apply": {
                "allowPotentialDataLoss": true,
                "confirmed": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "snapshot:before-clone:clone:tank/home-review"
            && step.commands.iter().any(|command| {
                command.argv == ["<snapshot-clone-tool>", "before-clone", "tank/home-review"]
                    && command.mutates
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["ZFS snapshot name or Btrfs snapshot path"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "snapshot:before-rollback:rollback"
            && step.commands.iter().any(|command| {
                command.argv == ["<snapshot-rollback-tool>", "before-rollback"]
                    && command.mutates
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["ZFS snapshot name"]
            })
    }));
    assert_eq!(report.command_summary.needs_domain_implementation_count, 2);
    assert!(!report.command_summary.all_commands_ready());
}

#[test]
fn zfs_clone_promotion_reports_reviewable_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "datasets": {
                  "tank/home-review": {
                    "operation": "promote"
                  }
                },
                "zvols": {
                  "tank/vm/root-review": {
                    "operation": "promote"
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
        step.action_id == "datasets:tank/home-review:promote"
            && step.commands.iter().any(|command| {
                command.argv == ["zfs", "promote", "tank/home-review"]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "zvols:tank/vm/root-review:promote"
            && step.commands.iter().any(|command| {
                command.argv == ["zfs", "promote", "tank/vm/root-review"]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "datasets:tank/home-review:promote"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "zfs",
                        "get",
                        "-H",
                        "-o",
                        "value",
                        "origin",
                        "tank/home-review",
                    ]
            })
    }));
}

#[test]
fn btrfs_subvolume_lifecycle_reports_domain_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "btrfsSubvolumes": {
                  "/mnt/persist/@home": {
                    "operation": "create",
                    "path": "/mnt/persist/@home",
                    "properties": {
                      "readonly": true
                    }
                  },
                  "/mnt/persist/@inventory": {
                    "operation": "rescan",
                    "path": "/mnt/persist/@inventory"
                  },
                  "/mnt/persist/@old-name": {
                    "operation": "rename",
                    "renameTo": "/mnt/persist/@new-name"
                  },
                  "/mnt/persist/@old": {
                    "destroy": true,
                    "preserveData": false
                  }
                }
              },
              "apply": {
                "allowOffline": true,
                "allowDestructive": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["btrfs", "subvolume", "create", "/mnt/persist/@home"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "btrfsSubvolumes:/mnt/persist/@home:set-property:readonly"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "btrfs",
                        "property",
                        "set",
                        "-ts",
                        "/mnt/persist/@home",
                        "ro",
                        "true",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "btrfssubvolumes:/mnt/persist/@inventory:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["btrfs", "subvolume", "show", "/mnt/persist/@inventory"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "btrfs",
                        "property",
                        "get",
                        "-ts",
                        "/mnt/persist/@inventory",
                        "ro",
                    ]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "btrfssubvolumes:/mnt/persist/@inventory:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "/mnt/persist/@inventory", "--json"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "btrfssubvolumes:/mnt/persist/@old-name:rename"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "mv",
                        "--",
                        "/mnt/persist/@old-name",
                        "/mnt/persist/@new-name",
                    ]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["btrfs", "subvolume", "delete", "/mnt/persist/@old"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "btrfssubvolumes:/mnt/persist/@old:destroy"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "topology", "--json"])
    }));
}
