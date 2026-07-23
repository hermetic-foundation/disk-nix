#[test]
fn dry_run_reports_no_mutation_when_policy_allows_plan() {
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

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.can_apply());
    assert_eq!(report.command_plan.len(), 1);
    assert_eq!(report.command_summary.step_count, 1);
    assert_eq!(report.command_summary.command_count, 2);
    assert_eq!(report.command_summary.ready_count, 1);
    assert_eq!(report.command_summary.needs_desired_size_count, 1);
    assert!(!report.command_summary.all_commands_ready());
    assert!(report.command_plan[0].requires_manual_review);
    assert_eq!(report.verification_summary.step_count, 1);
    assert!(report.verification_summary.command_count >= 1);
    assert_eq!(report.verification_plan.len(), 1);
    assert!(report.verification_plan[0]
        .commands
        .iter()
        .all(|command| { !command.mutates && command.readiness == CommandReadiness::Ready }));
    assert!(report.command_plan[0].commands.iter().any(|command| {
        command
            .argv
            .first()
            .is_some_and(|program| program == "lvextend")
            && command.argv.contains(&"vg/root".to_string())
            && command.readiness == CommandReadiness::NeedsDesiredSize
            && command.unresolved_inputs == ["desired size delta"]
    }));
}

#[test]
fn filesystem_format_renders_mkfs_commands_when_explicitly_allowed() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "root": {
                    "mountpoint": "/",
                    "device": "/dev/disk/by-label/root",
                    "fsType": "ext4",
                    "preserveData": false
                  },
                  "data": {
                    "mountpoint": "/data",
                    "device": "/dev/disk/by-label/data",
                    "fsType": "xfs",
                    "preserveData": false
                  },
                  "bulk": {
                    "mountpoint": "/bulk",
                    "device": "/dev/disk/by-label/bulk",
                    "fsType": "btrfs",
                    "preserveData": false
                  },
                  "missing": {
                    "mountpoint": "/missing",
                    "fsType": "ext4",
                    "preserveData": false
                  }
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowFormat": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(!report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystem:root:preserve-data-disabled"
            && step.commands.iter().any(|command| {
                command.argv == ["mkfs.ext4", "-F", "/dev/disk/by-label/root"]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystem:data:preserve-data-disabled"
            && step.commands.iter().any(|command| {
                command.argv == ["mkfs.xfs", "-f", "/dev/disk/by-label/data"]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystem:bulk:preserve-data-disabled"
            && step.commands.iter().any(|command| {
                command.argv == ["mkfs.btrfs", "-f", "/dev/disk/by-label/bulk"]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystem:missing:preserve-data-disabled"
            && step.commands.iter().any(|command| {
                command.argv == ["mkfs", "-t", "ext4", "<filesystem-device>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["filesystem source device"]
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "filesystem:root:preserve-data-disabled"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["blkid", "/dev/disk/by-label/root"])
    }));
}

#[test]
fn filesystem_check_and_repair_render_domain_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "home": {
                    "mountpoint": "/home",
                    "device": "/dev/disk/by-label/home",
                    "fsType": "ext4",
                    "operation": "check"
                  },
                  "data": {
                    "mountpoint": "/data",
                    "device": "/dev/disk/by-label/data",
                    "fsType": "btrfs",
                    "operation": "repair"
                  },
                  "scratch": {
                    "mountpoint": "/scratch",
                    "device": "/dev/disk/by-label/scratch",
                    "fsType": "xfs",
                    "operation": "check"
                  },
                  "efi": {
                    "mountpoint": "/boot",
                    "device": "/dev/disk/by-partlabel/EFI",
                    "fsType": "vfat",
                    "operation": "check"
                  },
                  "shared": {
                    "mountpoint": "/mnt/shared",
                    "device": "/dev/disk/by-label/Shared",
                    "fsType": "exfat",
                    "operation": "repair"
                  },
                  "windows": {
                    "mountpoint": "/mnt/windows",
                    "device": "/dev/disk/by-label/Windows",
                    "fsType": "ntfs",
                    "operation": "repair"
                  },
                  "mobile": {
                    "mountpoint": "/mnt/mobile",
                    "device": "/dev/disk/by-label/Mobile",
                    "fsType": "f2fs",
                    "operation": "check"
                  },
                  "bulk": {
                    "mountpoint": "/bulk",
                    "device": "/dev/disk/by-label/Bulk",
                    "fsType": "bcachefs",
                    "operation": "repair"
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
        step.action_id == "filesystems:home:check"
            && step.commands.iter().any(|command| {
                command.argv == ["e2fsck", "-n", "/dev/disk/by-label/home"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:data:repair"
            && step.commands.iter().any(|command| {
                command.argv == ["btrfs", "check", "--repair", "/dev/disk/by-label/data"]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:scratch:check"
            && step.commands.iter().any(|command| {
                command.argv == ["xfs_repair", "-n", "/dev/disk/by-label/scratch"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:efi:check"
            && step.commands.iter().any(|command| {
                command.argv == ["fsck.fat", "-n", "/dev/disk/by-partlabel/EFI"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:shared:repair"
            && step.commands.iter().any(|command| {
                command.argv == ["fsck.exfat", "-p", "/dev/disk/by-label/Shared"]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:windows:repair"
            && step.commands.iter().any(|command| {
                command.argv == ["ntfsfix", "/dev/disk/by-label/Windows"]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:mobile:check"
            && step.commands.iter().any(|command| {
                command.argv == ["fsck.f2fs", "--dry-run", "/dev/disk/by-label/Mobile"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:bulk:repair"
            && step.commands.iter().any(|command| {
                command.argv == ["bcachefs", "fsck", "-y", "/dev/disk/by-label/Bulk"]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "filesystems:home:check"
            && step
                .checks
                .iter()
                .any(|check| check.contains("read-only check completed"))
    }));
}

#[test]
fn filesystem_check_and_repair_require_source_device_for_readiness() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "home": {
                    "mountpoint": "/home",
                    "fsType": "ext4",
                    "operation": "check"
                  },
                  "data": {
                    "mountpoint": "/data",
                    "fsType": "btrfs",
                    "operation": "repair"
                  },
                  "shared": {
                    "mountpoint": "/mnt/shared",
                    "fsType": "exfat",
                    "operation": "check"
                  },
                  "mobile": {
                    "mountpoint": "/mnt/mobile",
                    "fsType": "f2fs",
                    "operation": "check"
                  },
                  "bulk": {
                    "mountpoint": "/bulk",
                    "fsType": "bcachefs",
                    "operation": "repair"
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
        step.action_id == "filesystems:home:check"
            && step.commands.iter().any(|command| {
                command.argv == ["e2fsck", "-n", "<filesystem-device>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["filesystem source device"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:data:repair"
            && step.commands.iter().any(|command| {
                command.argv == ["btrfs", "check", "--repair", "<filesystem-device>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["filesystem source device"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:shared:check"
            && step.commands.iter().any(|command| {
                command.argv == ["fsck.exfat", "-n", "<filesystem-device>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["filesystem source device"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:mobile:check"
            && step.commands.iter().any(|command| {
                command.argv == ["fsck.f2fs", "--dry-run", "<filesystem-device>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["filesystem source device"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:bulk:repair"
            && step.commands.iter().any(|command| {
                command.argv == ["bcachefs", "fsck", "-y", "<filesystem-device>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["filesystem source device"]
            })
    }));
}

#[test]
fn desired_sizes_and_devices_drive_resize_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "home": {
                    "mountpoint": "/home",
                    "fsType": "btrfs",
                    "resizePolicy": "grow-only",
                    "desiredSize": "750GiB"
                  },
                  "srv": {
                    "mountpoint": "/srv",
                    "device": "/dev/disk/by-label/srv",
                    "fsType": "ext4",
                    "resizePolicy": "grow-only",
                    "desiredSize": "100G"
                  },
                  "var": {
                    "mountpoint": "/var",
                    "fsType": "ext4",
                    "resizePolicy": "grow-only",
                    "desiredSize": "50G"
                  },
                  "mobile": {
                    "mountpoint": "/mnt/mobile",
                    "device": "/dev/disk/by-label/mobile",
                    "fsType": "f2fs",
                    "resizePolicy": "grow-only",
                    "desiredSize": "409600"
                  },
                  "cache": {
                    "mountpoint": "/cache",
                    "fsType": "f2fs",
                    "resizePolicy": "grow-only"
                  }
                },
                "volumes": {
                  "vg/home": {
                    "operation": "grow",
                    "desiredSize": "800GiB"
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
    assert_eq!(report.command_summary.needs_desired_size_count, 0);
    assert_eq!(report.command_summary.needs_domain_implementation_count, 2);
    assert!(report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv == ["btrfs", "filesystem", "resize", "750GiB", "/home"]
                && command.readiness == CommandReadiness::Ready
        })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystem:srv:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["resize2fs", "/dev/disk/by-label/srv", "100G"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystem:var:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["resize2fs", "<filesystem-device>", "50G"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["filesystem source device"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystem:mobile:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["resize.f2fs", "-t", "409600", "/dev/disk/by-label/mobile"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystem:cache:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["resize.f2fs", "<filesystem-device>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["filesystem source device"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv == ["lvextend", "--resizefs", "--size", "800GiB", "vg/home"]
                && command.readiness == CommandReadiness::Ready
                && command.unresolved_inputs.is_empty()
        })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.checks
            .iter()
            .any(|check| check.contains("750GiB") || check.contains("800GiB"))
    }));
}

#[test]
fn filesystem_shrink_renderer_uses_domain_commands() {
    let btrfs_action = PlannedAction {
        id: "filesystem:data:shrink".to_string(),
        description: "shrink btrfs data".to_string(),
        operation: Operation::Shrink,
        risk: RiskClass::PotentialDataLoss,
        destructive: false,
        context: ActionContext {
            collection: Some("filesystems".to_string()),
            name: Some("data".to_string()),
            target: Some("/data".to_string()),
            fs_type: Some("btrfs".to_string()),
            desired_size: Some("750GiB".to_string()),
            ..ActionContext::default()
        },
        advice: None,
    };
    let ext_action = PlannedAction {
        id: "filesystem:home:shrink".to_string(),
        description: "shrink ext home".to_string(),
        operation: Operation::Shrink,
        risk: RiskClass::PotentialDataLoss,
        destructive: false,
        context: ActionContext {
            collection: Some("filesystems".to_string()),
            name: Some("home".to_string()),
            target: Some("/home".to_string()),
            device: Some("/dev/disk/by-label/home".to_string()),
            fs_type: Some("ext4".to_string()),
            desired_size: Some("100G".to_string()),
            ..ActionContext::default()
        },
        advice: None,
    };
    let ext_mountpoint_action = PlannedAction {
        id: "filesystem:srv:shrink".to_string(),
        description: "shrink ext srv".to_string(),
        operation: Operation::Shrink,
        risk: RiskClass::PotentialDataLoss,
        destructive: false,
        context: ActionContext {
            collection: Some("filesystems".to_string()),
            name: Some("srv".to_string()),
            target: Some("/srv".to_string()),
            fs_type: Some("ext4".to_string()),
            desired_size: Some("50G".to_string()),
            ..ActionContext::default()
        },
        advice: None,
    };
    let xfs_action = PlannedAction {
        id: "filesystem:scratch:shrink".to_string(),
        description: "shrink xfs scratch".to_string(),
        operation: Operation::Shrink,
        risk: RiskClass::Unsupported,
        destructive: false,
        context: ActionContext {
            collection: Some("filesystems".to_string()),
            name: Some("scratch".to_string()),
            target: Some("/scratch".to_string()),
            fs_type: Some("xfs".to_string()),
            desired_size: Some("500G".to_string()),
            ..ActionContext::default()
        },
        advice: None,
    };

    let (btrfs_commands, btrfs_notes, btrfs_manual_review) = commands_for_action(&btrfs_action);
    let (ext_commands, ext_notes, ext_manual_review) = commands_for_action(&ext_action);
    let (ext_mountpoint_commands, _, _) = commands_for_action(&ext_mountpoint_action);
    let (xfs_commands, _, xfs_manual_review) = commands_for_action(&xfs_action);

    assert!(btrfs_manual_review);
    assert!(btrfs_commands.iter().any(|command| {
        command.argv == ["btrfs", "filesystem", "usage", "-b", "/data"] && !command.mutates
    }));
    assert!(btrfs_commands.iter().any(|command| {
        command.argv == ["btrfs", "filesystem", "resize", "750GiB", "/data"]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(btrfs_notes
        .iter()
        .any(|note| note.contains("backups or snapshots")));

    assert!(ext_manual_review);
    assert!(ext_commands.iter().any(|command| {
        command.argv
            == [
                "findmnt",
                "--noheadings",
                "--output",
                "SOURCE,FSTYPE,SIZE,USED,AVAIL",
                "--target",
                "/home",
            ]
            && !command.mutates
    }));
    assert!(ext_commands.iter().any(|command| {
        command.argv == ["umount", "/home"]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(ext_commands.iter().any(|command| {
        command.argv == ["e2fsck", "-f", "/dev/disk/by-label/home"]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(ext_commands.iter().any(|command| {
        command.argv == ["resize2fs", "/dev/disk/by-label/home", "100G"]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(ext_mountpoint_commands.iter().any(|command| {
        command.argv == ["resize2fs", "<filesystem-device>", "50G"]
            && command.mutates
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command.unresolved_inputs == ["filesystem source device"]
    }));
    assert!(ext_notes
        .iter()
        .any(|note| note.contains("migrate-to-smaller-filesystem")));

    assert!(xfs_manual_review);
    assert!(xfs_commands.iter().any(|command| {
        command.argv == ["<migrate-to-smaller-filesystem>", "/scratch"]
            && command.readiness == CommandReadiness::ManualOnly
            && command.unresolved_inputs == ["replacement filesystem", "migration plan"]
    }));
}

#[test]
fn btrfs_filesystem_device_removal_stays_blocked_by_apply_policy() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "data": {
                    "mountpoint": "/data",
                    "fsType": "btrfs",
                    "removeDevices": ["/dev/disk/by-id/old-btrfs-device"]
                  }
                }
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::Blocked);
    assert_eq!(report.command_summary.step_count, 1);
    assert!(
        !report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv
                    == [
                        "btrfs",
                        "device",
                        "remove",
                        "/dev/disk/by-id/old-btrfs-device",
                        "/data",
                    ]
            })
        }),
        "potential-data-loss Btrfs device removal remains blocked by apply policy"
    );
    assert!(report.verification_plan.iter().all(|step| {
        step.action_id != "filesystems:data:remove-device:/dev/disk/by-id/old-btrfs-device"
    }));
}

#[test]
fn btrfs_filesystem_device_removal_renderer_uses_btrfs_commands() {
    let action = PlannedAction {
        id: "filesystems:data:remove-device:/dev/disk/by-id/old-btrfs-device".to_string(),
        description: "remove old Btrfs device".to_string(),
        operation: Operation::RemoveDevice,
        risk: RiskClass::PotentialDataLoss,
        destructive: false,
        context: ActionContext {
            collection: Some("filesystems".to_string()),
            name: Some("data".to_string()),
            target: Some("/data".to_string()),
            device: Some("/dev/disk/by-id/old-btrfs-device".to_string()),
            fs_type: Some("btrfs".to_string()),
            ..ActionContext::default()
        },
        advice: None,
    };

    let (commands, notes, requires_manual_review) = commands_for_action(&action);
    let (verification_commands, verification_checks) = verification_for_action(&action);

    assert!(requires_manual_review);
    assert!(notes
        .iter()
        .any(|note| note.contains("remaining data and metadata space are sufficient")));
    assert!(commands.iter().any(|command| {
        command.argv == ["btrfs", "filesystem", "usage", "-b", "/data"] && !command.mutates
    }));
    assert!(commands.iter().any(|command| {
        command.argv
            == [
                "btrfs",
                "device",
                "remove",
                "/dev/disk/by-id/old-btrfs-device",
                "/data",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(verification_commands.iter().any(|command| {
        command.argv == ["btrfs", "filesystem", "usage", "-b", "/data"] && !command.mutates
    }));
    assert!(verification_checks
        .iter()
        .any(|check| check.contains("Btrfs device list matches desired topology")));
}

#[test]
fn bcachefs_filesystem_lifecycle_reports_domain_commands() {
    let grow = PlannedAction {
        id: "filesystem:bulk:grow".to_string(),
        description: "grow bcachefs member".to_string(),
        operation: Operation::Grow,
        risk: RiskClass::Online,
        destructive: false,
        context: ActionContext {
            collection: Some("filesystems".to_string()),
            name: Some("bulk".to_string()),
            target: Some("/bulk".to_string()),
            device: Some("/dev/disk/by-id/bcachefs-member".to_string()),
            fs_type: Some("bcachefs".to_string()),
            desired_size: Some("4TiB".to_string()),
            ..ActionContext::default()
        },
        advice: None,
    };
    let add = PlannedAction {
        id: "filesystems:bulk:add-device:/dev/disk/by-id/new-bcachefs-device".to_string(),
        description: "add bcachefs member".to_string(),
        operation: Operation::AddDevice,
        risk: RiskClass::Online,
        destructive: false,
        context: ActionContext {
            collection: Some("filesystems".to_string()),
            target: Some("/bulk".to_string()),
            device: Some("/dev/disk/by-id/new-bcachefs-device".to_string()),
            fs_type: Some("bcachefs".to_string()),
            ..ActionContext::default()
        },
        advice: None,
    };
    let remove = PlannedAction {
        id: "filesystems:bulk:remove-device:/dev/disk/by-id/old-bcachefs-device".to_string(),
        description: "remove bcachefs member".to_string(),
        operation: Operation::RemoveDevice,
        risk: RiskClass::PotentialDataLoss,
        destructive: false,
        context: ActionContext {
            collection: Some("filesystems".to_string()),
            target: Some("/bulk".to_string()),
            device: Some("/dev/disk/by-id/old-bcachefs-device".to_string()),
            fs_type: Some("bcachefs".to_string()),
            ..ActionContext::default()
        },
        advice: None,
    };
    let replace = PlannedAction {
        id: "filesystems:bulk:replace-device:/dev/disk/by-id/old-bcachefs-device".to_string(),
        description: "replace bcachefs member".to_string(),
        operation: Operation::ReplaceDevice,
        risk: RiskClass::OfflineRequired,
        destructive: false,
        context: ActionContext {
            collection: Some("filesystems".to_string()),
            target: Some("/bulk".to_string()),
            device: Some("/dev/disk/by-id/old-bcachefs-device".to_string()),
            replacement: Some("/dev/disk/by-id/new-bcachefs-device".to_string()),
            fs_type: Some("bcachefs".to_string()),
            ..ActionContext::default()
        },
        advice: None,
    };
    let rebalance = PlannedAction {
        id: "filesystems:bulk:rebalance".to_string(),
        description: "rereplicate bcachefs data".to_string(),
        operation: Operation::Rebalance,
        risk: RiskClass::Online,
        destructive: false,
        context: ActionContext {
            collection: Some("filesystems".to_string()),
            target: Some("/bulk".to_string()),
            fs_type: Some("bcachefs".to_string()),
            ..ActionContext::default()
        },
        advice: None,
    };
    let scrub = PlannedAction {
        id: "filesystems:bulk:scrub".to_string(),
        description: "scrub bcachefs".to_string(),
        operation: Operation::Scrub,
        risk: RiskClass::Online,
        destructive: false,
        context: ActionContext {
            collection: Some("filesystems".to_string()),
            target: Some("/bulk".to_string()),
            fs_type: Some("bcachefs".to_string()),
            ..ActionContext::default()
        },
        advice: None,
    };

    let (grow_commands, _, _) = commands_for_action(&grow);
    let (add_commands, _, _) = commands_for_action(&add);
    let (remove_commands, remove_notes, _) = commands_for_action(&remove);
    let (replace_commands, replace_notes, _) = commands_for_action(&replace);
    let (rebalance_commands, _, _) = commands_for_action(&rebalance);
    let (scrub_commands, _, _) = commands_for_action(&scrub);
    let (add_verification_commands, add_verification_checks) = verification_for_action(&add);

    assert!(grow_commands.iter().any(|command| {
        command.argv
            == [
                "bcachefs",
                "device",
                "resize",
                "/dev/disk/by-id/bcachefs-member",
                "4TiB",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(add_commands.iter().any(|command| {
        command.argv
            == [
                "bcachefs",
                "device",
                "add",
                "/bulk",
                "/dev/disk/by-id/new-bcachefs-device",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(add_verification_commands
        .iter()
        .any(|command| { command.argv == ["bcachefs", "fs", "usage", "/bulk"] }));
    assert!(!add_verification_commands
        .iter()
        .any(|command| command.argv == ["btrfs", "filesystem", "usage", "-b", "/bulk"]));
    assert!(add_verification_checks
        .iter()
        .any(|check| check.contains("bcachefs member list")));
    assert!(remove_commands.iter().any(|command| {
        command.argv == ["bcachefs", "fs", "usage", "/bulk"] && !command.mutates
    }));
    assert!(remove_commands.iter().any(|command| {
        command.argv == ["bcachefs", "data", "rereplicate", "/bulk"] && command.mutates
    }));
    assert!(remove_commands.iter().any(|command| {
        command.argv
            == [
                "bcachefs",
                "device",
                "remove",
                "/bulk",
                "/dev/disk/by-id/old-bcachefs-device",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(remove_notes
        .iter()
        .any(|note| note.contains("remaining replicas")));
    assert!(replace_commands.iter().any(|command| {
        command.argv
            == [
                "bcachefs",
                "device",
                "add",
                "/bulk",
                "/dev/disk/by-id/new-bcachefs-device",
            ]
    }));
    assert!(replace_commands.iter().any(|command| {
        command.argv
            == [
                "bcachefs",
                "device",
                "remove",
                "/bulk",
                "/dev/disk/by-id/old-bcachefs-device",
            ]
    }));
    assert!(replace_notes
        .iter()
        .any(|note| note.contains("replacement capacity")));
    assert!(rebalance_commands.iter().any(|command| {
        command.argv == ["bcachefs", "data", "rereplicate", "/bulk"] && command.mutates
    }));
    assert!(scrub_commands
        .iter()
        .any(|command| { command.argv == ["bcachefs", "scrub", "/bulk"] && command.mutates }));
}
