#[test]
fn filesystem_unmount_lifecycle_reports_umount_command_when_offline_allowed() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "archive": {
                    "mountpoint": "/archive",
                    "device": "/dev/disk/by-label/archive",
                    "fsType": "ext4",
                    "operation": "unmount"
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
    assert!(report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:archive:unmount"
            && step.commands.iter().any(|command| {
                command.argv == ["umount", "/archive"]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "filesystems:archive:unmount"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "topology", "--json"])
    }));
}

#[test]
fn filesystem_mount_lifecycle_requires_source_and_mountpoint_for_execute_readiness() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "scratch": {
                    "fsType": "xfs",
                    "operation": "mount",
                    "options": ["ro"]
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
        step.action_id == "filesystems:scratch:mount"
            && step.commands.iter().any(|command| {
                command.argv == ["mount", "-t", "xfs", "-o", "ro", "<device>", "<mountpoint>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["filesystem source device", "mountpoint path"]
            })
    }));
}

#[test]
fn filesystem_unmount_lifecycle_is_blocked_by_default_offline_policy() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "archive": {
                    "mountpoint": "/archive",
                    "operation": "unmount"
                  }
                }
              },
              "apply": {}
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::Blocked);
    assert_eq!(report.apply.blocked_count, 1);
    assert_eq!(
        report.messages,
        ["apply policy blocked 1 action(s)".to_string()]
    );
    assert!(!report
        .command_plan
        .iter()
        .any(|step| step.action_id == "filesystems:archive:unmount"));
}

#[test]
fn filesystem_remount_requires_mountpoint_for_execute_readiness() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "scratch": {
                    "fsType": "xfs",
                    "operation": "remount",
                    "options": ["ro"]
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
        step.action_id == "filesystems:scratch:remount"
            && step.commands.iter().any(|command| {
                command.argv == ["mount", "-o", "remount,ro", "<mountpoint>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["mountpoint path"]
            })
    }));
}

#[test]
fn remove_device_renderer_uses_pool_and_lvm_commands() {
    let pool_action = PlannedAction {
        id: "pools:tank:remove-device:/dev/disk/by-id/old-vdev".to_string(),
        description: "remove old pool device".to_string(),
        operation: Operation::RemoveDevice,
        risk: RiskClass::PotentialDataLoss,
        destructive: false,
        context: ActionContext {
            collection: Some("pools".to_string()),
            name: Some("tank".to_string()),
            target: Some("tank".to_string()),
            device: Some("/dev/disk/by-id/old-vdev".to_string()),
            ..ActionContext::default()
        },
        advice: None,
    };
    let vg_action = PlannedAction {
        id: "volumeGroups:vg0:remove-device:/dev/disk/by-id/old-pv".to_string(),
        description: "remove old physical volume".to_string(),
        operation: Operation::RemoveDevice,
        risk: RiskClass::PotentialDataLoss,
        destructive: false,
        context: ActionContext {
            collection: Some("volumeGroups".to_string()),
            name: Some("vg0".to_string()),
            target: Some("vg0".to_string()),
            device: Some("/dev/disk/by-id/old-pv".to_string()),
            ..ActionContext::default()
        },
        advice: None,
    };
    let missing_pool_action = PlannedAction {
        id: "pools:tank:removedevice".to_string(),
        description: "remove unspecified pool device".to_string(),
        operation: Operation::RemoveDevice,
        risk: RiskClass::PotentialDataLoss,
        destructive: false,
        context: ActionContext {
            collection: Some("pools".to_string()),
            name: Some("tank".to_string()),
            target: Some("tank".to_string()),
            ..ActionContext::default()
        },
        advice: None,
    };
    let missing_vg_action = PlannedAction {
        id: "volumeGroups:vg0:removedevice".to_string(),
        description: "remove unspecified physical volume".to_string(),
        operation: Operation::RemoveDevice,
        risk: RiskClass::PotentialDataLoss,
        destructive: false,
        context: ActionContext {
            collection: Some("volumeGroups".to_string()),
            name: Some("vg0".to_string()),
            target: Some("vg0".to_string()),
            ..ActionContext::default()
        },
        advice: None,
    };

    let (pool_commands, pool_notes, pool_manual_review) = commands_for_action(&pool_action);
    let (vg_commands, vg_notes, vg_manual_review) = commands_for_action(&vg_action);
    let (missing_pool_commands, _, _) = commands_for_action(&missing_pool_action);
    let (missing_vg_commands, _, _) = commands_for_action(&missing_vg_action);

    assert!(pool_manual_review);
    assert!(pool_commands
        .iter()
        .any(|command| { command.argv == ["zpool", "status", "-P", "tank"] && !command.mutates }));
    assert!(pool_commands.iter().any(|command| {
        command.argv == ["zpool", "remove", "tank", "/dev/disk/by-id/old-vdev"]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(pool_notes
        .iter()
        .any(|note| note.contains("supports device removal")));

    assert!(vg_manual_review);
    assert!(vg_commands.iter().any(|command| {
        command.argv == ["pvs", "--reportformat", "json", "/dev/disk/by-id/old-pv"]
            && !command.mutates
    }));
    assert!(vg_commands.iter().any(|command| {
        command.argv == ["pvmove", "/dev/disk/by-id/old-pv"]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(vg_commands.iter().any(|command| {
        command.argv == ["vgreduce", "vg0", "/dev/disk/by-id/old-pv"]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(vg_notes
        .iter()
        .any(|note| note.contains("pvmove or add replacement capacity")));
    assert!(missing_pool_commands.iter().any(|command| {
        command.argv == ["zpool", "remove", "tank", "<device>"]
            && command.mutates
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command.unresolved_inputs == ["device to remove"]
    }));
    assert!(missing_vg_commands.iter().any(|command| {
        command.argv == ["pvmove", "<physical-volume>"]
            && command.mutates
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command.unresolved_inputs == ["physical volume to remove"]
    }));
    assert!(missing_vg_commands.iter().any(|command| {
        command.argv == ["vgreduce", "vg0", "<physical-volume>"]
            && command.mutates
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command.unresolved_inputs == ["physical volume to remove"]
    }));
}

#[test]
fn volume_group_replacement_renders_lvm_migration_commands() {
    let action = PlannedAction {
        id: "volumeGroups:vg0:replace-device:/dev/disk/by-id/old-pv".to_string(),
        description: "replace old physical volume".to_string(),
        operation: Operation::ReplaceDevice,
        risk: RiskClass::OfflineRequired,
        destructive: false,
        context: ActionContext {
            collection: Some("volumeGroups".to_string()),
            name: Some("vg0".to_string()),
            target: Some("vg0".to_string()),
            device: Some("/dev/disk/by-id/old-pv".to_string()),
            replacement: Some("/dev/disk/by-id/new-pv".to_string()),
            ..ActionContext::default()
        },
        advice: None,
    };
    let missing_action = PlannedAction {
        id: "volumeGroups:vg0:replacedevice".to_string(),
        description: "replace unspecified physical volume".to_string(),
        operation: Operation::ReplaceDevice,
        risk: RiskClass::OfflineRequired,
        destructive: false,
        context: ActionContext {
            collection: Some("volumeGroups".to_string()),
            name: Some("vg0".to_string()),
            target: Some("vg0".to_string()),
            ..ActionContext::default()
        },
        advice: None,
    };

    let (commands, notes, manual_review) = commands_for_action(&action);
    let (missing_commands, _, _) = commands_for_action(&missing_action);

    assert!(manual_review);
    assert!(commands.iter().any(|command| {
        command.argv == ["pvs", "--reportformat", "json", "/dev/disk/by-id/old-pv"]
            && !command.mutates
    }));
    assert!(commands.iter().any(|command| {
        command.argv == ["pvs", "--reportformat", "json", "/dev/disk/by-id/new-pv"]
            && !command.mutates
    }));
    assert!(commands.iter().any(|command| {
        command.argv == ["vgextend", "vg0", "/dev/disk/by-id/new-pv"]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(commands.iter().any(|command| {
        command.argv == ["pvmove", "/dev/disk/by-id/old-pv", "/dev/disk/by-id/new-pv"]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(commands.iter().any(|command| {
        command.argv == ["vgreduce", "vg0", "/dev/disk/by-id/old-pv"]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(notes
        .iter()
        .any(|note| note.contains("replacement physical volume")));

    assert!(missing_commands.iter().any(|command| {
        command.argv == ["vgextend", "vg0", "<replacement-physical-volume>"]
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command.unresolved_inputs == ["replacement physical volume"]
    }));
    assert!(missing_commands.iter().any(|command| {
        command.argv
            == [
                "pvmove",
                "<physical-volume>",
                "<replacement-physical-volume>",
            ]
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command.unresolved_inputs
                == ["physical volume to replace", "replacement physical volume"]
    }));
    assert!(missing_commands.iter().any(|command| {
        command.argv == ["vgreduce", "vg0", "<physical-volume>"]
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command.unresolved_inputs == ["physical volume to remove"]
    }));
}

#[test]
fn zfs_snapshot_rollback_renderer_reports_reviewable_commands() {
    let action = PlannedAction {
        id: "snapshot:tank/home@before:rollback".to_string(),
        description: "roll back tank/home to snapshot tank/home@before".to_string(),
        operation: Operation::Rollback,
        risk: RiskClass::PotentialDataLoss,
        destructive: false,
        context: ActionContext {
            collection: Some("snapshots".to_string()),
            name: Some("tank/home@before".to_string()),
            target: Some("tank/home@before".to_string()),
            recursive_rollback: Some(true),
            ..ActionContext::default()
        },
        advice: None,
    };

    let (commands, notes, requires_manual_review) = commands_for_action(&action);
    let (verification_commands, verification_checks) = verification_for_action(&action);

    assert!(requires_manual_review);
    assert!(notes.iter().any(|note| note.contains("fresh snapshot")));
    assert!(notes.iter().any(|note| note.contains("recursive rollback")));
    assert!(commands.iter().any(|command| {
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
    assert!(commands.iter().any(|command| {
        command.argv == ["zfs", "rollback", "-r", "tank/home@before"]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(verification_commands.iter().any(|command| {
        command.argv == ["zfs", "list", "-H", "-p", "tank/home"] && !command.mutates
    }));
    assert!(verification_checks
        .iter()
        .any(|check| check.contains("rollback point")));
}

#[test]
fn zfs_snapshot_rollback_stays_blocked_by_apply_policy() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "snapshots": {
                "tank/home@before": {
                  "target": "tank/home",
                  "rollback": true
                }
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::Blocked);
    assert!(report.command_plan.is_empty());
    assert_eq!(report.command_summary.step_count, 0);

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

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["zfs", "rollback", "tank/home@before"])
    }));
}

#[test]
fn zfs_snapshot_holds_render_safe_property_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "snapshots": {
                "tank/home@before": {
                  "target": "tank/home",
                  "hold": "disk-nix-retain"
                },
                "tank/home@old": {
                  "target": "tank/home",
                  "releaseHold": "old-retention"
                }
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert_eq!(report.command_plan.len(), 2);
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "snapshot:tank/home@before:hold:disk-nix-retain"
            && step.commands.iter().any(|command| {
                command.argv == ["zfs", "hold", "disk-nix-retain", "tank/home@before"]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "snapshot:tank/home@old:release-hold:old-retention"
            && step.commands.iter().any(|command| {
                command.argv == ["zfs", "release", "old-retention", "tank/home@old"]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "snapshot:tank/home@before:hold:disk-nix-retain"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["zfs", "holds", "tank/home@before"])
    }));
    assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
    assert!(report.command_summary.all_commands_ready());
}

#[test]
fn snapshot_lifecycle_accepts_names_for_logical_keys() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "snapshots": {
                "before-hold": {
                  "name": "tank/home@before",
                  "target": "tank/home",
                  "hold": "keep"
                },
                "old-release": {
                  "snapshotName": "tank/home@old",
                  "target": "tank/home",
                  "releaseHold": "expired"
                },
                "before-rescan": {
                  "snapshot-name": "tank/home@before",
                  "target": "tank/home",
                  "operation": "rescan"
                },
                "before-clone": {
                  "name": "tank/home@before",
                  "target": "tank/home",
                  "cloneTo": "tank/home-review"
                },
                "before-rename": {
                  "name": "tank/home@before-rename",
                  "target": "tank/home",
                  "renameTo": "tank/home@retained"
                },
                "before-rollback": {
                  "name": "tank/home@before",
                  "target": "tank/home",
                  "rollback": true,
                  "recursiveRollback": true
                },
                "old-destroy": {
                  "name": "tank/home@old",
                  "target": "tank/home",
                  "destroy": true
                }
              },
              "apply": {
                "allowOffline": true,
                "allowDestructive": true,
                "allowPotentialDataLoss": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "snapshot:before-hold:hold:keep"
            && step.commands.iter().any(|command| {
                command.argv == ["zfs", "hold", "keep", "tank/home@before"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "snapshot:old-release:release-hold:expired"
            && step.commands.iter().any(|command| {
                command.argv == ["zfs", "release", "expired", "tank/home@old"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "snapshot:before-rescan:rescan"
            && step.commands.iter().any(|command| {
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
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "snapshot:before-clone:clone:tank/home-review"
            && step.commands.iter().any(|command| {
                command.argv == ["zfs", "clone", "tank/home@before", "tank/home-review"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "snapshot:before-rename:rename:tank/home@retained"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "zfs",
                        "rename",
                        "tank/home@before-rename",
                        "tank/home@retained",
                    ]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "snapshot:before-rollback:rollback"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["zfs", "rollback", "-r", "tank/home@before"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "snapshot:old-destroy:destroy"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["zfs", "destroy", "tank/home@old"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "snapshot:before-hold:hold:keep"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["zfs", "holds", "tank/home@before"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "snapshot:old-destroy:destroy"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "tank/home", "--json"])
    }));
    assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
    assert!(report.command_summary.all_commands_ready());
}

#[test]
fn snapshot_rescan_reports_read_only_metadata_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "snapshots": {
                "tank/home@before": {
                  "operation": "rescan",
                  "target": "tank/home"
                },
                "/mnt/persist/@home-before": {
                  "operation": "rescan",
                  "target": "/mnt/persist/@home",
                  "readOnly": true
                },
                "home-before-friendly": {
                  "operation": "rescan",
                  "target": "/mnt/persist/@home",
                  "snapshotPath": "/mnt/persist/@home-before-friendly"
                }
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert_eq!(report.command_plan.len(), 3);
    let zfs_step = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "snapshot:tank/home@before:rescan")
        .expect("ZFS snapshot rescan step exists");
    assert!(zfs_step
        .commands
        .iter()
        .all(|command| command.readiness == CommandReadiness::Ready && !command.mutates));
    assert!(zfs_step.commands.iter().any(|command| {
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
    }));
    assert!(zfs_step
        .commands
        .iter()
        .any(|command| command.argv == ["zfs", "holds", "tank/home@before"]));

    let btrfs_step = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "snapshot:/mnt/persist/@home-before:rescan")
        .expect("Btrfs snapshot rescan step exists");
    assert!(btrfs_step
        .commands
        .iter()
        .all(|command| command.readiness == CommandReadiness::Ready && !command.mutates));
    assert!(btrfs_step.commands.iter().any(|command| {
        command.argv == ["btrfs", "subvolume", "show", "/mnt/persist/@home-before"]
    }));
    assert!(btrfs_step.commands.iter().any(|command| {
        command.argv
            == [
                "btrfs",
                "property",
                "get",
                "-ts",
                "/mnt/persist/@home-before",
                "ro",
            ]
    }));

    let friendly_btrfs_step = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "snapshot:home-before-friendly:rescan")
        .expect("friendly-key Btrfs snapshot rescan step exists");
    assert!(friendly_btrfs_step
        .commands
        .iter()
        .all(|command| command.readiness == CommandReadiness::Ready && !command.mutates));
    assert!(friendly_btrfs_step.commands.iter().any(|command| {
        command.argv
            == [
                "btrfs",
                "subvolume",
                "show",
                "/mnt/persist/@home-before-friendly",
            ]
    }));
    assert!(friendly_btrfs_step.commands.iter().any(|command| {
        command.argv
            == [
                "btrfs",
                "property",
                "get",
                "-ts",
                "/mnt/persist/@home-before-friendly",
                "ro",
            ]
    }));

    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "snapshot:tank/home@before:rescan"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["zfs", "holds", "tank/home@before"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "snapshot:/mnt/persist/@home-before:rescan"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "btrfs",
                        "property",
                        "get",
                        "-ts",
                        "/mnt/persist/@home-before",
                        "ro",
                    ]
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "snapshot:home-before-friendly:rescan"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "btrfs",
                        "property",
                        "get",
                        "-ts",
                        "/mnt/persist/@home-before-friendly",
                        "ro",
                    ]
            })
    }));
    assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
    assert!(report.command_summary.all_commands_ready());
}

#[test]
fn snapshot_destroy_reports_domain_specific_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "snapshots": {
                "tank/home@old": {
                  "target": "tank/home",
                  "destroy": true
                },
                "/mnt/persist/@home-old": {
                  "target": "/mnt/persist/@home",
                  "destroy": true
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
    assert_eq!(report.apply.blocked.len(), 0);
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "snapshot:tank/home@old:destroy"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["zfs", "destroy", "tank/home@old"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "snapshot:/mnt/persist/@home-old:destroy"
            && step.commands.iter().any(|command| {
                command.argv == ["btrfs", "subvolume", "delete", "/mnt/persist/@home-old"]
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "snapshot:tank/home@old:destroy"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "tank/home", "--json"])
    }));
    assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
    assert!(report.command_summary.all_commands_ready());
}

#[test]
fn shell_script_includes_commands_and_verification() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "home": {
                    "mountpoint": "/home",
                    "fsType": "btrfs",
                    "resizePolicy": "grow-only",
                    "desiredSize": "750GiB"
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
    let script = report.to_shell_script().expect("script can render");

    assert!(script.starts_with("#!/usr/bin/env bash"));
    assert!(script.contains("btrfs filesystem resize 750GiB /home"));
    assert!(script.contains("# Post-apply verification commands"));
    assert!(script.contains("disk-nix inspect /home --json"));
}

#[test]
fn shell_script_comments_non_ready_commands() {
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

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);
    let script = report.to_shell_script().expect("script can render");

    assert!(script.contains("# NOT READY: lvextend --resizefs --size '+<size>' vg/root"));
    assert!(script.contains("# Unresolved inputs: desired size delta"));
}
