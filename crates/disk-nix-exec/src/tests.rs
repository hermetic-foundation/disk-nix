
use disk_nix_model::{Node, NodeKind, Relationship, StorageGraph};
use disk_nix_plan::{
    compare_plan_with_topology, plan_and_policy_from_json_bytes, ActionContext, PlanSummary,
};

use super::*;

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

#[test]
fn bcachefs_filesystem_update_commands_report_missing_inputs() {
    let grow = PlannedAction {
        id: "filesystem:bulk:grow".to_string(),
        description: "grow bcachefs member".to_string(),
        operation: Operation::Grow,
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
    let add = PlannedAction {
        id: "filesystems:bulk:add-device".to_string(),
        description: "add bcachefs member".to_string(),
        operation: Operation::AddDevice,
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

    assert!(grow_commands.iter().any(|command| {
        command.argv
            == [
                "bcachefs",
                "device",
                "resize",
                "<bcachefs-device>",
                "<size>",
            ]
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command.unresolved_inputs
                == ["bcachefs member device", "desired bcachefs member size"]
    }));
    assert!(add_commands.iter().any(|command| {
        command.argv == ["bcachefs", "device", "add", "/bulk", "<device>"]
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command.unresolved_inputs == ["device to add"]
    }));
}

#[test]
fn btrfs_filesystem_label_property_is_ready() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "data": {
                  "mountpoint": "/data",
                  "fsType": "btrfs",
                  "properties": {
                    "label": "bulk-data"
                  }
                }
              }
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:data:set-property:label"
            && step.commands.iter().any(|command| {
                command.argv == ["btrfs", "filesystem", "label", "/data", "bulk-data"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_summary.ready_count >= 3);
    assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
}

#[test]
fn ext_filesystem_label_uses_declared_device() {
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
                    }
                  },
                  "missing-device": {
                    "mountpoint": "/srv",
                    "fsType": "ext4",
                    "properties": {
                      "label": "srv-new"
                    }
                  }
                }
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:home:set-property:label"
            && step.commands.iter().any(|command| {
                command.argv == ["e2label", "/dev/disk/by-label/home-old", "home-new"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:missing-device:set-property:label"
            && step.commands.iter().any(|command| {
                command.argv == ["e2label", "<filesystem-device>", "srv-new"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["filesystem source device"]
            })
    }));
}

#[test]
fn xfs_filesystem_label_uses_declared_device() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "scratch": {
                    "mountpoint": "/scratch",
                    "device": "/dev/disk/by-label/scratch-old",
                    "fsType": "xfs",
                    "properties": {
                      "label": "scratch-new"
                    }
                  },
                  "missing-device": {
                    "mountpoint": "/archive",
                    "fsType": "xfs",
                    "properties": {
                      "xfs.label": "archive-new"
                    }
                  }
                }
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:scratch:set-property:label"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "xfs_admin",
                        "-L",
                        "scratch-new",
                        "/dev/disk/by-label/scratch-old",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:missing-device:set-property:xfs.label"
            && step.commands.iter().any(|command| {
                command.argv == ["xfs_admin", "-L", "archive-new", "<filesystem-device>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["filesystem source device"]
            })
    }));
}

#[test]
fn fat_filesystem_properties_use_fatlabel() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "efi": {
                    "mountpoint": "/boot",
                    "device": "/dev/disk/by-partlabel/EFI",
                    "fsType": "vfat",
                    "properties": {
                      "label": "NIXBOOT",
                      "vfat.uuid": "a1b2-c3d4"
                    }
                  },
                  "missing-device": {
                    "mountpoint": "/firmware",
                    "fsType": "vfat",
                    "properties": {
                      "volume-id": "deadbeef"
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
        step.action_id == "filesystems:efi:set-property:label"
            && step.commands.iter().any(|command| {
                command.argv == ["fatlabel", "/dev/disk/by-partlabel/EFI", "NIXBOOT"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:efi:set-property:vfat.uuid"
            && step.commands.iter().any(|command| {
                command.argv == ["fatlabel", "-i", "/dev/disk/by-partlabel/EFI", "A1B2C3D4"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:missing-device:set-property:volume-id"
            && step.commands.iter().any(|command| {
                command.argv == ["fatlabel", "-i", "<filesystem-device>", "DEADBEEF"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["filesystem source device"]
            })
    }));
}

#[test]
fn ntfs_filesystem_properties_use_ntfslabel() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "windows": {
                    "mountpoint": "/mnt/windows",
                    "device": "/dev/disk/by-label/Windows",
                    "fsType": "ntfs",
                    "properties": {
                      "label": "Windows",
                      "ntfs.uuid": "01234567-89abcdef"
                    }
                  },
                  "missing-device": {
                    "mountpoint": "/mnt/media",
                    "fsType": "ntfs",
                    "properties": {
                      "volume-serial": "fedcba98-76543210"
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
        step.action_id == "filesystems:windows:set-property:label"
            && step.commands.iter().any(|command| {
                command.argv == ["ntfslabel", "/dev/disk/by-label/Windows", "Windows"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:windows:set-property:ntfs.uuid"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "ntfslabel",
                        "--new-serial=0123456789ABCDEF",
                        "/dev/disk/by-label/Windows",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:missing-device:set-property:volume-serial"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "ntfslabel",
                        "--new-serial=FEDCBA9876543210",
                        "<filesystem-device>",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["filesystem source device"]
            })
    }));
}

#[test]
fn exfat_filesystem_properties_use_exfatlabel() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "shared": {
                    "mountpoint": "/mnt/shared",
                    "device": "/dev/disk/by-label/Shared",
                    "fsType": "exfat",
                    "properties": {
                      "label": "Shared",
                      "exfat.uuid": "a1b2-c3d4"
                    }
                  },
                  "missing-device": {
                    "mountpoint": "/mnt/camera",
                    "fsType": "exfat",
                    "properties": {
                      "volume-serial": "deadbeef"
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
        step.action_id == "filesystems:shared:set-property:label"
            && step.commands.iter().any(|command| {
                command.argv == ["exfatlabel", "/dev/disk/by-label/Shared", "Shared"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:shared:set-property:exfat.uuid"
            && step.commands.iter().any(|command| {
                command.argv == ["exfatlabel", "-i", "/dev/disk/by-label/Shared", "A1B2C3D4"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:missing-device:set-property:volume-serial"
            && step.commands.iter().any(|command| {
                command.argv == ["exfatlabel", "-i", "<filesystem-device>", "DEADBEEF"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["filesystem source device"]
            })
    }));
}

#[test]
fn f2fs_filesystem_label_uses_f2fslabel() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "mobile": {
                    "mountpoint": "/mnt/mobile",
                    "device": "/dev/disk/by-label/mobile-old",
                    "fsType": "f2fs",
                    "properties": {
                      "label": "mobile-new"
                    }
                  },
                  "missing-device": {
                    "mountpoint": "/mnt/cache",
                    "fsType": "f2fs",
                    "properties": {
                      "f2fs.label": "cache-new"
                    }
                  }
                }
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:mobile:set-property:label"
            && step.commands.iter().any(|command| {
                command.argv == ["f2fslabel", "/dev/disk/by-label/mobile-old", "mobile-new"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:missing-device:set-property:f2fs.label"
            && step.commands.iter().any(|command| {
                command.argv == ["f2fslabel", "<filesystem-device>", "cache-new"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["filesystem source device"]
            })
    }));
}

#[test]
fn filesystem_uuid_updates_render_domain_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "home": {
                    "mountpoint": "/home",
                    "device": "/dev/disk/by-label/home",
                    "fsType": "ext4",
                    "properties": {
                      "ext.uuid": "11111111-2222-3333-4444-555555555555"
                    }
                  },
                  "scratch": {
                    "mountpoint": "/scratch",
                    "device": "/dev/disk/by-label/scratch",
                    "fsType": "xfs",
                    "properties": {
                      "filesystem.uuid": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
                    }
                  },
                  "data": {
                    "mountpoint": "/data",
                    "device": "/dev/disk/by-label/data",
                    "fsType": "btrfs",
                    "properties": {
                      "btrfs.uuid": "bbbbbbbb-1111-2222-3333-cccccccccccc"
                    }
                  },
                  "missing-device": {
                    "mountpoint": "/archive",
                    "fsType": "xfs",
                    "properties": {
                      "uuid": "ffffffff-1111-2222-3333-444444444444"
                    }
                  },
                  "missing-btrfs": {
                    "mountpoint": "/missing-btrfs",
                    "fsType": "btrfs",
                    "properties": {
                      "uuid": "cccccccc-1111-2222-3333-dddddddddddd"
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
        step.action_id == "filesystems:home:set-property:ext.uuid"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "tune2fs",
                        "-U",
                        "11111111-2222-3333-4444-555555555555",
                        "/dev/disk/by-label/home",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:scratch:set-property:filesystem.uuid"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "xfs_admin",
                        "-U",
                        "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                        "/dev/disk/by-label/scratch",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:data:set-property:btrfs.uuid"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "btrfstune",
                        "-U",
                        "bbbbbbbb-1111-2222-3333-cccccccccccc",
                        "/dev/disk/by-label/data",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:missing-device:set-property:uuid"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "xfs_admin",
                        "-U",
                        "ffffffff-1111-2222-3333-444444444444",
                        "<filesystem-device>",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["filesystem source device"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:missing-btrfs:set-property:uuid"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "btrfstune",
                        "-U",
                        "cccccccc-1111-2222-3333-dddddddddddd",
                        "<filesystem-device>",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["filesystem source device"]
            })
    }));
}

#[test]
fn btrfs_filesystem_rebalance_uses_declared_filters() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "filesystems": {
                "data": {
                  "mountpoint": "/data",
                  "fsType": "btrfs",
                  "operation": "rebalance",
                  "properties": {
                    "balance.data": "usage=50",
                    "balance.metadata": "usage=75"
                  }
                }
              },
              "apply": {
                "allowRebalance": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:data:rebalance"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "btrfs",
                        "balance",
                        "start",
                        "-dusage=50",
                        "-musage=75",
                        "/data",
                    ]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "filesystems:data:rebalance"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["btrfs", "filesystem", "usage", "-b", "/data"])
    }));
}

#[test]
fn scrub_lifecycle_reports_btrfs_and_zpool_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "data": {
                    "mountpoint": "/data",
                    "fsType": "btrfs",
                    "operation": "scrub"
                  }
                },
                "pools": {
                  "tank": {
                    "operation": "scrub"
                  }
                }
              },
              "apply": {}
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:data:scrub"
            && step.commands.iter().any(|command| {
                command.argv == ["btrfs", "scrub", "start", "-B", "/data"]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "pools:tank:scrub"
            && step.commands.iter().any(|command| {
                command.argv == ["zpool", "scrub", "tank"]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "pools:tank:scrub"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "tank", "--json"])
    }));
}

#[test]
fn filesystem_trim_lifecycle_reports_fstrim_command() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "scratch": {
                    "mountpoint": "/scratch",
                    "fsType": "xfs",
                    "operation": "trim"
                  }
                }
              },
              "apply": {}
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:scratch:trim"
            && step.commands.iter().any(|command| {
                command.argv == ["fstrim", "-v", "/scratch"]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "filesystems:scratch:trim"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "/scratch", "--json"])
    }));
}

#[test]
fn filesystem_rescan_lifecycle_reports_read_only_inventory_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "scratch": {
                    "mountpoint": "/scratch",
                    "device": "/dev/disk/by-label/scratch",
                    "fsType": "xfs",
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
    assert!(report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "filesystems:scratch:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["findmnt", "--json", "/scratch"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "/scratch"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "filesystems:scratch:rescan"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "/scratch", "--json"])
    }));
}

#[test]
fn filesystem_rescan_requires_mountpoint_for_execute_readiness() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "scratch": {
                    "fsType": "xfs",
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
        step.action_id == "filesystems:scratch:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["findmnt", "--json", "<mountpoint>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["mountpoint path"]
            })
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "<mountpoint>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["mountpoint path"]
            })
    }));
}

#[test]
fn filesystem_remount_lifecycle_reports_mount_command() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "scratch": {
                    "mountpoint": "/scratch",
                    "fsType": "xfs",
                    "operation": "remount",
                    "options": ["rw", "noatime", "discard=async"]
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
        step.action_id == "filesystems:scratch:remount"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "mount",
                        "-o",
                        "remount,rw,noatime,discard=async",
                        "/scratch",
                    ]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "filesystems:scratch:remount"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["findmnt", "--json", "/scratch"])
    }));
}

#[test]
fn filesystem_mount_lifecycle_reports_mount_command() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "backup": {
                    "mountpoint": "/backup",
                    "device": "/dev/disk/by-label/backup",
                    "fsType": "xfs",
                    "operation": "mount",
                    "options": ["rw", "noatime"]
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
        step.action_id == "filesystems:backup:mount"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "mount",
                        "-t",
                        "xfs",
                        "-o",
                        "rw,noatime",
                        "/dev/disk/by-label/backup",
                        "/backup",
                    ]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "filesystems:backup:mount"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["findmnt", "--json", "/backup"])
    }));
}

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

#[test]
fn swap_and_luks_commands_follow_policy_gates() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "swaps": {
                  "primary": {
                    "device": "/dev/disk/by-label/swap",
                    "preserveData": false
                  },
                  "scratch": {
                    "device": "/swapfile",
                    "operation": "grow",
                    "desiredSize": "16GiB"
                  },
                  "inventory": {
                    "device": "/dev/disk/by-label/swap-inventory",
                    "operation": "rescan"
                  },
                  "retired": {
                    "device": "/dev/disk/by-label/retired-swap",
                    "operation": "deactivate"
                  },
                  "remove": {
                    "device": "/dev/disk/by-label/remove-swap",
                    "operation": "destroy"
                  }
                },
                "luks": {
                  "devices": {
                    "cryptroot": {
                      "name": "cryptroot",
                      "device": "/dev/disk/by-partuuid/root",
                      "operation": "grow"
                    },
                    "cryptdata": {
                      "name": "cryptdata",
                      "device": "/dev/disk/by-id/data-luks",
                      "operation": "create"
                    },
                    "cryptarchive": {
                      "name": "cryptarchive",
                      "device": "/dev/disk/by-id/archive-luks",
                      "operation": "open"
                    },
                    "cryptmissing": {
                      "name": "cryptmissing",
                      "operation": "create"
                    },
                    "cryptold": {
                      "name": "cryptold",
                      "device": "/dev/disk/by-id/old-luks",
                      "operation": "destroy"
                    },
                    "cryptclosed": {
                      "name": "cryptclosed",
                      "device": "/dev/disk/by-id/closed-luks",
                      "operation": "close"
                    }
                  }
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowFormat": true,
                "allowOffline": true,
                "allowGrow": true,
                "allowPropertyChanges": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert_eq!(report.command_plan.len(), 11);
    assert!(report.command_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["mkswap", "/dev/disk/by-label/swap"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv == ["fallocate", "--length", "16GiB", "/swapfile"]
                && command.readiness == CommandReadiness::Ready
        })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "swaps:inventory:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["blkid", "/dev/disk/by-label/swap-inventory"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "/dev/disk/by-label/swap-inventory"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "swaps:retired:deactivate"
            && step.commands.iter().any(|command| {
                command.argv == ["swapoff", "/dev/disk/by-label/retired-swap"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "swaps:remove:destroy"
            && step.commands.iter().any(|command| {
                command.argv == ["swapoff", "/dev/disk/by-label/remove-swap"]
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["wipefs", "--all", "/dev/disk/by-label/remove-swap"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["cryptsetup", "resize", "cryptroot"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "luks.devices:cryptdata:create"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "cryptsetup",
                        "open",
                        "/dev/disk/by-id/data-luks",
                        "cryptdata",
                    ]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "luks.devices:cryptarchive:open"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "cryptsetup",
                        "open",
                        "/dev/disk/by-id/archive-luks",
                        "cryptarchive",
                    ]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "luks.devices:cryptmissing:create"
            && step.commands.iter().any(|command| {
                command.argv == ["cryptsetup", "isLuks", "<device>"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["LUKS backing device"]
            })
            && step.commands.iter().any(|command| {
                command.argv == ["cryptsetup", "open", "<device>", "cryptmissing"]
                    && command.mutates
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["LUKS backing device"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "luks.devices:cryptold:destroy"
            && step.commands.iter().any(|command| {
                command.argv == ["cryptsetup", "close", "cryptold"]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "luks.devices:cryptclosed:close"
            && step.commands.iter().any(|command| {
                command.argv == ["cryptsetup", "close", "cryptclosed"]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "swaps:scratch:grow"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["swapon", "--show", "--bytes", "--raw"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "swaps:inventory:rescan"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "disk-nix",
                        "inspect",
                        "/dev/disk/by-label/swap-inventory",
                        "--json",
                    ]
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "luks.devices:cryptold:destroy"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "topology", "--json"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "luks.devices:cryptdata:create"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["cryptsetup", "status", "cryptdata"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "luks.devices:cryptarchive:open"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["cryptsetup", "status", "cryptarchive"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "luks.devices:cryptclosed:close"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "topology", "--json"])
    }));
}

#[test]
fn luks_device_lifecycle_accepts_mapper_names_for_logical_keys() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "luks": {
                  "devices": {
                    "rootMapping": {
                      "target": "cryptroot",
                      "device": "/dev/disk/by-id/root-luks",
                      "operation": "grow"
                    },
                    "archiveMapping": {
                      "mapperName": "cryptarchive",
                      "device": "/dev/disk/by-id/archive-luks",
                      "operation": "open"
                    },
                    "headerIdentity": {
                      "mapper-name": "cryptroot",
                      "device": "/dev/disk/by-id/root-luks",
                      "properties": {
                        "label": "root",
                        "luks.subsystem": "nixos",
                        "luks.uuid": "01234567-89ab-cdef-0123-456789abcdef"
                      }
                    },
                    "closedMapping": {
                      "mapper": "cryptclosed",
                      "device": "/dev/disk/by-id/closed-luks",
                      "operation": "close"
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

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "luks.devices:rootMapping:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["cryptsetup", "resize", "cryptroot"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "luks.devices:archiveMapping:open"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "cryptsetup",
                        "open",
                        "/dev/disk/by-id/archive-luks",
                        "cryptarchive",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "luks.devices:headerIdentity:set-property:label"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "cryptsetup",
                        "config",
                        "/dev/disk/by-id/root-luks",
                        "--label",
                        "root",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "luks.devices:headerIdentity:set-property:luks.subsystem"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "cryptsetup",
                        "config",
                        "/dev/disk/by-id/root-luks",
                        "--subsystem",
                        "nixos",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "luks.devices:headerIdentity:set-property:luks.uuid"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "cryptsetup",
                        "luksUUID",
                        "/dev/disk/by-id/root-luks",
                        "--uuid",
                        "01234567-89ab-cdef-0123-456789abcdef",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "luks.devices:closedMapping:close"
            && step.commands.iter().any(|command| {
                command.argv == ["cryptsetup", "close", "cryptclosed"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "luks.devices:archiveMapping:open"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["cryptsetup", "status", "cryptarchive"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "luks.devices:closedMapping:close"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "topology", "--json"])
    }));
}

#[test]
fn swap_lifecycle_requires_target_path_for_execute_readiness() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "swaps": {
                  "scratch": {
                    "operation": "grow",
                    "desiredSize": "16GiB"
                  },
                  "inventory": {
                    "operation": "rescan"
                  },
                  "retired": {
                    "operation": "deactivate"
                  },
                  "remove": {
                    "operation": "destroy"
                  },
                  "primary": {
                    "preserveData": false
                  }
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowFormat": true,
                "allowOffline": true,
                "allowGrow": true,
                "allowPropertyChanges": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(!report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "swaps:scratch:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["swapoff", "<swap>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["swap target path"]
            })
            && step.commands.iter().any(|command| {
                command.argv == ["<resize-swap-backing-storage>", "<swap>", "16GiB"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["swap target path", "backing storage domain"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "swaps:inventory:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["blkid", "<swap>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["swap target path"]
            })
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "<swap>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["swap target path"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "swaps:retired:deactivate"
            && step.commands.iter().any(|command| {
                command.argv == ["swapoff", "<swap>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["swap target path"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "swaps:remove:destroy"
            && step.commands.iter().any(|command| {
                command.argv == ["swapoff", "<swap>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["swap target path"]
            })
            && step.commands.iter().any(|command| {
                command.argv == ["wipefs", "--all", "<swap>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["swap target path"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "swaps:primary:format"
            && step.commands.iter().any(|command| {
                command.argv == ["mkswap", "<swap>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["swap target path"]
            })
    }));
}

#[test]
fn swap_lifecycle_accepts_path_aliases_for_logical_names() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "swaps": {
                  "scratchSwap": {
                    "path": "/swapfile",
                    "operation": "grow",
                    "desiredSize": "16GiB"
                  },
                  "inventorySwap": {
                    "target": "/dev/disk/by-label/swap-inventory",
                    "operation": "rescan"
                  },
                  "primarySwap": {
                    "path": "/dev/disk/by-label/swap",
                    "preserveData": false,
                    "properties": {
                      "label": "swap",
                      "swap.uuid": "01234567-89ab-cdef-0123-456789abcdef"
                    }
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

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "swaps:scratchSwap:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["fallocate", "--length", "16GiB", "/swapfile"]
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["swapoff", "/swapfile"]
                    && command.readiness == CommandReadiness::Ready
            })
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["swapon", "/swapfile"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "swaps:inventorySwap:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["blkid", "/dev/disk/by-label/swap-inventory"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "/dev/disk/by-label/swap-inventory"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "swaps:primarySwap:format"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["mkswap", "/dev/disk/by-label/swap"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "swaps:primarySwap:set-property:label"
            && step.commands.iter().any(|command| {
                command.argv == ["swaplabel", "--label", "swap", "/dev/disk/by-label/swap"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "swaps:primarySwap:set-property:swap.uuid"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "swaplabel",
                        "--uuid",
                        "01234567-89ab-cdef-0123-456789abcdef",
                        "/dev/disk/by-label/swap",
                    ]
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "swaps:scratchSwap:grow"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "/swapfile", "--json"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "swaps:primarySwap:set-property:swap.uuid"
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "/dev/disk/by-label/swap", "--json"]
            })
    }));
}

#[test]
fn zram_rescan_reports_read_only_inventory_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "zram": {
                  "enable": true,
                  "operation": "rescan",
                  "swapDevices": 2,
                  "algorithm": "zstd"
                }
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert_eq!(report.command_plan.len(), 1);
    assert_eq!(report.command_plan[0].action_id, "zram:rescan");
    assert!(report.command_plan[0]
        .commands
        .iter()
        .all(|command| { !command.mutates && command.readiness == CommandReadiness::Ready }));
    assert!(report.command_plan[0].commands.iter().any(|command| {
        command.argv
            == [
                "zramctl",
                "--bytes",
                "--raw",
                "--noheadings",
                "--output-all",
            ]
    }));
    assert!(report.command_plan[0]
        .commands
        .iter()
        .any(|command| { command.argv == ["swapon", "--show", "--bytes", "--raw"] }));
    assert!(report.command_plan[0]
        .commands
        .iter()
        .any(|command| command.argv == ["disk-nix", "zram"]));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "zram:rescan"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "zramctl",
                        "--bytes",
                        "--raw",
                        "--noheadings",
                        "--output-all",
                    ]
            })
    }));
}

#[test]
fn zram_default_declaration_inspects_generated_inventory() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "zram": {
                  "enable": true,
                  "swapDevices": 2,
                  "memoryPercent": 40,
                  "priority": 20,
                  "algorithm": "zstd"
                }
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert_eq!(report.command_plan.len(), 1);
    assert_eq!(report.command_plan[0].action_id, "zram:inspect");
    assert!(!report.command_plan[0].requires_manual_review);
    assert!(report.command_plan[0]
        .commands
        .iter()
        .all(|command| !command.mutates && command.readiness == CommandReadiness::Ready));
    assert!(report.command_plan[0].commands.iter().any(|command| {
        command.argv
            == [
                "zramctl",
                "--bytes",
                "--raw",
                "--noheadings",
                "--output-all",
            ]
    }));
    assert!(report.command_plan[0]
        .commands
        .iter()
        .any(|command| command.argv == ["disk-nix", "zram"]));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "zram:inspect"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "zram"])
    }));
}

#[test]
fn luks_header_properties_use_cryptsetup_identity_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "luks": {
                  "devices": {
                    "cryptroot": {
                      "name": "cryptroot",
                      "device": "/dev/disk/by-id/root-luks",
                      "properties": {
                        "label": "root",
                        "luks.subsystem": "nixos",
                        "luks.uuid": "01234567-89ab-cdef-0123-456789abcdef"
                      }
                    },
                    "logical": {
                      "properties": {
                        "luks.label": "logical-root"
                      }
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
        step.action_id == "luks.devices:cryptroot:set-property:label"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "cryptsetup",
                        "config",
                        "/dev/disk/by-id/root-luks",
                        "--label",
                        "root",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "luks.devices:cryptroot:set-property:luks.subsystem"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "cryptsetup",
                        "config",
                        "/dev/disk/by-id/root-luks",
                        "--subsystem",
                        "nixos",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "luks.devices:cryptroot:set-property:luks.uuid"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "cryptsetup",
                        "luksUUID",
                        "/dev/disk/by-id/root-luks",
                        "--uuid",
                        "01234567-89ab-cdef-0123-456789abcdef",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "luks.devices:logical:set-property:luks.label"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "cryptsetup",
                        "config",
                        "<luks-device>",
                        "--label",
                        "logical-root",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["LUKS backing device"]
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "luks.devices:cryptroot:set-property:label"
            && step.commands.iter().any(|command| {
                command.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/root-luks"]
            })
    }));
}

#[test]
fn swap_properties_use_swaplabel() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "swaps": {
                  "primary": {
                    "device": "/dev/disk/by-label/swap-old",
                    "properties": {
                      "label": "swap-new",
                      "swap.uuid": "01234567-89ab-cdef-0123-456789abcdef",
                      "priority": "10"
                    }
                  },
                  "logical": {
                    "properties": {
                      "swap.label": "logical-swap",
                      "swap.priority": "20"
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
        step.action_id == "swaps:primary:set-property:label"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "swaplabel",
                        "--label",
                        "swap-new",
                        "/dev/disk/by-label/swap-old",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "swaps:primary:set-property:swap.uuid"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "swaplabel",
                        "--uuid",
                        "01234567-89ab-cdef-0123-456789abcdef",
                        "/dev/disk/by-label/swap-old",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "swaps:logical:set-property:swap.label"
            && step.commands.iter().any(|command| {
                command.argv == ["swaplabel", "--label", "logical-swap", "<swap>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["swap target path"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
            step.action_id == "swaps:primary:set-property:priority"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "sh",
                            "-c",
                            "swapoff /dev/disk/by-label/swap-old 2>/dev/null || true; swapon --priority 10 /dev/disk/by-label/swap-old",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "swaps:logical:set-property:swap.priority"
            && step.commands.iter().any(|command| {
                command.argv == ["swapon", "--priority", "20", "<swap>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["swap target path"]
            })
    }));
}

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

#[test]
fn btrfs_qgroup_lifecycle_reports_limit_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "btrfsQgroups": {
                  "0/258": {
                    "target": "/mnt/persist",
                    "operation": "create"
                  },
                  "0/257": {
                    "target": "/mnt/persist",
                    "properties": {
                      "limit": "25GiB",
                      "maxExclusive": "10GiB"
                    }
                  },
                  "0/263": {
                    "target": "/mnt/persist",
                    "operation": "rescan"
                  },
                  "0/259": {
                    "target": "/mnt/persist",
                    "destroy": true
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
        step.action_id == "btrfsqgroups:0/258:create"
            && step.commands.iter().any(|command| {
                command.argv == ["btrfs", "qgroup", "create", "0/258", "/mnt/persist"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "btrfsQgroups:0/257:set-property:limit"
            && step.commands.iter().any(|command| {
                command.argv == ["btrfs", "qgroup", "limit", "25GiB", "0/257", "/mnt/persist"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "btrfsQgroups:0/257:set-property:maxExclusive"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "btrfs",
                        "qgroup",
                        "limit",
                        "-e",
                        "10GiB",
                        "0/257",
                        "/mnt/persist",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "btrfsqgroups:0/259:destroy"
            && step.commands.iter().any(|command| {
                command.argv == ["btrfs", "qgroup", "destroy", "0/259", "/mnt/persist"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "btrfsqgroups:0/263:rescan"
            && step
                .commands
                .iter()
                .all(|command| command.readiness == CommandReadiness::Ready && !command.mutates)
            && step.commands.iter().any(|command| {
                command.argv == ["btrfs", "qgroup", "show", "--raw", "-reF", "/mnt/persist"]
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "btrfsQgroups:0/257:set-property:limit"
            && step.commands.iter().any(|command| {
                command.argv == ["btrfs", "qgroup", "show", "--raw", "-reF", "/mnt/persist"]
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "btrfsqgroups:0/263:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["btrfs", "qgroup", "show", "--raw", "-reF", "/mnt/persist"]
            })
    }));
    assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
    assert!(report.command_summary.all_commands_ready());
}

#[test]
fn btrfs_qgroup_lifecycle_accepts_path_aliases_for_mount_targets() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "btrfsQgroups": {
                  "0/257": {
                    "path": "/mnt/persist",
                    "properties": {
                      "limit": "25GiB"
                    }
                  },
                  "0/258": {
                    "mountpoint": "/mnt/archive",
                    "operation": "rescan"
                  }
                }
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "btrfsQgroups:0/257:set-property:limit"
            && step.commands.iter().any(|command| {
                command.argv == ["btrfs", "qgroup", "limit", "25GiB", "0/257", "/mnt/persist"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "btrfsqgroups:0/258:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["btrfs", "qgroup", "show", "--raw", "-reF", "/mnt/archive"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "btrfsqgroups:0/258:rescan"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "/mnt/archive", "--json"])
    }));
}

#[test]
fn btrfs_qgroup_lifecycle_without_target_reports_unresolved_path() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "btrfsQgroups": {
                  "0/260": {
                    "operation": "create"
                  },
                  "0/261": {
                    "properties": {
                      "limit": "5GiB"
                    }
                  },
                  "0/263": {
                    "operation": "rescan"
                  },
                  "0/262": {
                    "destroy": true
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
        step.action_id == "btrfsqgroups:0/260:create"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "btrfs",
                        "qgroup",
                        "create",
                        "0/260",
                        "<btrfs-filesystem-path>",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["mounted Btrfs filesystem path"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "btrfsQgroups:0/261:set-property:limit"
            && step.commands.iter().any(|command| {
                command.argv == ["btrfs", "qgroup", "limit", "5GiB", "0/261", "<path>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["mounted Btrfs filesystem path"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "btrfsqgroups:0/263:rescan"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "btrfs",
                        "qgroup",
                        "show",
                        "--raw",
                        "-reF",
                        "<btrfs-filesystem-path>",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["mounted Btrfs filesystem path"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "btrfsqgroups:0/262:destroy"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "btrfs",
                        "qgroup",
                        "destroy",
                        "0/262",
                        "<btrfs-filesystem-path>",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["mounted Btrfs filesystem path"]
            })
    }));
    assert_eq!(report.command_summary.needs_domain_implementation_count, 7);
    assert!(!report.command_summary.all_commands_ready());
}

#[test]
fn zvol_lifecycle_reports_zfs_volume_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "zvols": {
                  "tank/vm/root": {
                    "operation": "grow",
                    "desiredSize": "80GiB",
                    "properties": {
                      "compression": "zstd"
                    }
                  },
                  "tank/vm/tmp": {
                    "operation": "create",
                    "desiredSize": "20GiB",
                    "properties": {
                      "compression": "zstd",
                      "volblocksize": "16K"
                    }
                  },
                  "tank/vm/inventory": {
                    "operation": "rescan"
                  },
                  "tank/vm/old": {
                    "destroy": true
                  }
                }
              },
              "apply": {
                "allowGrow": true,
                "allowDestructive": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv == ["zfs", "set", "volsize=80GiB", "tank/vm/root"]
                && command.readiness == CommandReadiness::Ready
        })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "zvols:tank/vm/root:set-property:compression"
            && step.commands.iter().any(|command| {
                command.argv == ["zfs", "set", "compression=zstd", "tank/vm/root"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv.len() == 11
                && command.argv[0] == "bash"
                && command.argv[1] == "-c"
                && command.argv[2].contains("zfs list -H")
                && command.argv[2].contains("zfs create")
                && command.argv[2].contains("status=\"$?\"")
                && command.argv[3] == "disk-nix-zfs-create"
                && command.argv[4] == "tank/vm/tmp"
                && command.argv[5..]
                    == [
                        "-o",
                        "compression=zstd",
                        "-o",
                        "volblocksize=16K",
                        "-V",
                        "20GiB",
                    ]
                && command.readiness == CommandReadiness::Ready
        })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["zfs", "destroy", "tank/vm/old"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "zvols:tank/vm/inventory:rescan"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "zfs",
                        "list",
                        "-H",
                        "-p",
                        "-t",
                        "volume",
                        "tank/vm/inventory",
                    ]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "tank/vm/inventory"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv == ["zfs", "list", "-H", "-p", "-t", "volume", "tank/vm/root"]
        })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "zvols:tank/vm/inventory:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "tank/vm/inventory", "--json"]
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "zvols:tank/vm/root:set-property:compression"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["zfs", "get", "all", "tank/vm/root"])
    }));
}

#[test]
fn zfs_dataset_lifecycle_reports_zfs_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "datasets": {
                  "tank/home": {
                    "operation": "create",
                    "mountpoint": "/home",
                    "properties": {
                      "compression": "zstd",
                      "mountpoint": "/home"
                    }
                  },
                  "tank/inventory": {
                    "operation": "rescan"
                  },
                  "tank/archive": {
                    "destroy": true
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
        step.commands.iter().any(|command| {
            command.argv.len() == 9
                && command.argv[0] == "bash"
                && command.argv[1] == "-c"
                && command.argv[2].contains("zfs list -H")
                && command.argv[2].contains("zfs create")
                && command.argv[2].contains("status=\"$?\"")
                && command.argv[3] == "disk-nix-zfs-create"
                && command.argv[4] == "tank/home"
                && command.argv[5..] == ["-o", "compression=zstd", "-o", "mountpoint=/home"]
                && command.readiness == CommandReadiness::Ready
        })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["zfs", "destroy", "tank/archive"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "datasets:tank/inventory:rescan"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "zfs",
                        "list",
                        "-H",
                        "-p",
                        "-t",
                        "filesystem",
                        "tank/inventory",
                    ]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "tank/inventory"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "datasets:tank/home:create"
            && step.commands.iter().any(|command| {
                command.argv == ["zfs", "list", "-H", "-p", "-t", "filesystem", "tank/home"]
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "datasets:tank/inventory:rescan"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "tank/inventory", "--json"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "datasets:tank/archive:destroy"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["zfs", "list", "-H", "-p", "-t", "filesystem"])
    }));
}

#[test]
fn zfs_lifecycle_accepts_targets_for_logical_names() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "datasets": {
                  "homeCreate": {
                    "target": "tank/home",
                    "operation": "create",
                    "properties": {
                      "compression": "zstd",
                      "mountpoint": "/home"
                    }
                  },
                  "homeInventory": {
                    "target": "tank/home",
                    "operation": "rescan"
                  },
                  "homeRename": {
                    "target": "tank/home-old",
                    "operation": "rename",
                    "renameTo": "tank/home-staged"
                  },
                  "homeReview": {
                    "target": "tank/home-review",
                    "operation": "promote"
                  },
                  "oldDataset": {
                    "target": "tank/old",
                    "operation": "destroy"
                  }
                },
                "zvols": {
                  "rootCreate": {
                    "target": "tank/vm/root",
                    "operation": "create",
                    "desiredSize": "32GiB",
                    "properties": {
                      "compression": "zstd"
                    }
                  },
                  "rootGrow": {
                    "target": "tank/vm/root",
                    "operation": "grow",
                    "desiredSize": "64GiB",
                    "properties": {
                      "volblocksize": "16K"
                    }
                  },
                  "rootInventory": {
                    "target": "tank/vm/root",
                    "operation": "rescan"
                  },
                  "rootPromote": {
                    "target": "tank/vm/root-review",
                    "operation": "promote"
                  },
                  "oldRoot": {
                    "target": "tank/vm/old",
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
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "datasets:homecreate:create"
            && step.commands.iter().any(|command| {
                command.argv.len() == 9
                    && command.argv[0] == "bash"
                    && command.argv[1] == "-c"
                    && command.argv[2].contains("zfs list -H")
                    && command.argv[2].contains("zfs create")
                    && command.argv[2].contains("status=\"$?\"")
                    && command.argv[3] == "disk-nix-zfs-create"
                    && command.argv[4] == "tank/home"
                    && command.argv[5..] == ["-o", "compression=zstd", "-o", "mountpoint=/home"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "datasets:homeinventory:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["zfs", "list", "-H", "-p", "-t", "filesystem", "tank/home"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "zfs",
                        "get",
                        "-H",
                        "-p",
                        "-o",
                        "property,value,source",
                        "all",
                        "tank/home",
                    ]
                    && !command.mutates
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "datasets:homerename:rename"
            && step.commands.iter().any(|command| {
                command.argv == ["zfs", "rename", "tank/home-old", "tank/home-staged"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "datasets:homereview:promote"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["zfs", "promote", "tank/home-review"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "datasets:olddataset:destroy"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["zfs", "destroy", "tank/old"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "zvols:rootcreate:create"
            && step.commands.iter().any(|command| {
                command.argv.len() == 9
                    && command.argv[0] == "bash"
                    && command.argv[1] == "-c"
                    && command.argv[2].contains("zfs list -H")
                    && command.argv[2].contains("zfs create")
                    && command.argv[2].contains("status=\"$?\"")
                    && command.argv[3] == "disk-nix-zfs-create"
                    && command.argv[4] == "tank/vm/root"
                    && command.argv[5..] == ["-o", "compression=zstd", "-V", "32GiB"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "zvols:rootgrow:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["zfs", "set", "volsize=64GiB", "tank/vm/root"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "zvols:rootGrow:set-property:volblocksize"
            && step.commands.iter().any(|command| {
                command.argv.len() == 7
                    && command.argv[0] == "bash"
                    && command.argv[1] == "-c"
                    && command.argv[2].contains("zfs get -H -p -o value")
                    && command.argv[2].contains("exec zfs set")
                    && command.argv[3] == "disk-nix-zfs-set"
                    && command.argv[4] == "tank/vm/root"
                    && command.argv[5] == "volblocksize"
                    && command.argv[6] == "16K"
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "zvols:rootinventory:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["zfs", "list", "-H", "-p", "-t", "volume", "tank/vm/root"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "tank/vm/root"] && !command.mutates
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "zvols:rootpromote:promote"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["zfs", "promote", "tank/vm/root-review"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "zvols:oldroot:destroy"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["zfs", "destroy", "tank/vm/old"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "datasets:homereview:promote"
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "tank/home-review", "--json"]
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "zvols:rootGrow:set-property:volblocksize"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["zfs", "get", "all", "tank/vm/root"])
    }));
}

#[test]
fn md_raid_lifecycle_reports_mdadm_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "mdRaids": {
                  "existing": {
                    "target": "/dev/md/existing",
                    "operation": "assemble",
                    "devices": [
                      "/dev/disk/by-id/existing-a",
                      "/dev/disk/by-id/existing-b"
                    ]
                  },
                  "oldroot": {
                    "target": "/dev/md/oldroot",
                    "operation": "stop"
                  },
                  "inventory": {
                    "target": "/dev/md/root",
                    "operation": "rescan"
                  },
                  "root": {
                    "target": "/dev/md/root",
                    "operation": "grow",
                    "desiredSize": "max",
                    "addDevices": ["/dev/disk/by-id/nvme-spare"],
                    "replaceDevices": {
                      "/dev/disk/by-id/old-md-member": "/dev/disk/by-id/new-md-member"
                    },
                    "removeDevices": ["/dev/disk/by-id/failed-md-member"]
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

    assert_eq!(report.status, ExecutionStatus::Blocked);
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "mdraids:existing:assemble"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "mdadm",
                        "--assemble",
                        "/dev/md/existing",
                        "/dev/disk/by-id/existing-a",
                        "/dev/disk/by-id/existing-b",
                    ]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "mdraids:oldroot:stop"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["mdadm", "--stop", "/dev/md/oldroot"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "mdraids:inventory:rescan"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["mdadm", "--detail", "/dev/md/root"])
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["mdadm", "--examine", "--scan"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv
                == [
                    "mdadm",
                    "/dev/md/root",
                    "--add",
                    "/dev/disk/by-id/nvme-spare",
                ]
        })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["mdadm", "--grow", "/dev/md/root", "--size", "max"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv
                == [
                    "mdadm",
                    "/dev/md/root",
                    "--replace",
                    "/dev/disk/by-id/old-md-member",
                    "--with",
                    "/dev/disk/by-id/new-md-member",
                ]
        })
    }));
    assert!(
        !report.command_plan.iter().any(|step| {
            step.commands.iter().any(|command| {
                command.argv
                    == [
                        "mdadm",
                        "/dev/md/root",
                        "--remove",
                        "/dev/disk/by-id/failed-md-member",
                    ]
            })
        }),
        "potential-data-loss remove action remains blocked by apply policy"
    );
    assert!(report.verification_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["cat", "/proc/mdstat"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "mdraids:inventory:rescan"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "topology", "--json"])
    }));
}

#[test]
fn md_raid_lifecycle_uses_declared_device_as_array_target_for_logical_names() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "mdRaids": {
                  "root": {
                    "device": "/dev/md/root",
                    "operation": "grow",
                    "desiredSize": "max",
                    "addDevices": ["/dev/disk/by-id/nvme-spare"],
                    "replaceDevices": {
                      "/dev/disk/by-id/old-md-member": "/dev/disk/by-id/new-md-member"
                    }
                  },
                  "oldroot": {
                    "device": "/dev/md/oldroot",
                    "operation": "stop"
                  },
                  "inventory": {
                    "device": "/dev/md/root",
                    "operation": "rescan"
                  }
                }
              },
              "apply": {
                "allowGrow": true,
                "allowOffline": true,
                "allowDeviceReplacement": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "mdraids:root:grow"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["mdadm", "--grow", "/dev/md/root", "--size", "max"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "mdRaids:root:add-device:/dev/disk/by-id/nvme-spare"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "mdadm",
                        "/dev/md/root",
                        "--add",
                        "/dev/disk/by-id/nvme-spare",
                    ]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "mdRaids:root:replace-device:/dev/disk/by-id/old-md-member"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "mdadm",
                        "/dev/md/root",
                        "--replace",
                        "/dev/disk/by-id/old-md-member",
                        "--with",
                        "/dev/disk/by-id/new-md-member",
                    ]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "mdraids:oldroot:stop"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["mdadm", "--stop", "/dev/md/oldroot"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "mdraids:inventory:rescan"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["mdadm", "--detail", "/dev/md/root"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "mdraids:root:grow"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "/dev/md/root", "--json"])
    }));
}

#[test]
fn md_raid_create_requires_destructive_policy_and_renders_mdadm_create() {
    let (blocked_plan, blocked_policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "mdRaids": {
                  "newroot": {
                    "target": "/dev/md/newroot",
                    "operation": "create",
                    "level": "1",
                    "devices": [
                      "/dev/disk/by-id/nvme-a",
                      "/dev/disk/by-id/nvme-b"
                    ]
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
                "mdRaids": {
                  "newroot": {
                    "target": "/dev/md/newroot",
                    "operation": "create",
                    "level": "1",
                    "devices": [
                      "/dev/disk/by-id/nvme-a",
                      "/dev/disk/by-id/nvme-b"
                    ]
                  },
                  "missing-level": {
                    "target": "/dev/md/missing-level",
                    "operation": "create",
                    "devices": [
                      "/dev/disk/by-id/nvme-c",
                      "/dev/disk/by-id/nvme-d"
                    ]
                  },
                  "missing-members": {
                    "target": "/dev/md/missing-members",
                    "operation": "create",
                    "level": "10"
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
        step.commands.iter().any(|command| {
            command.argv
                == [
                    "mdadm",
                    "--create",
                    "/dev/md/newroot",
                    "--level",
                    "1",
                    "--raid-devices",
                    "2",
                    "--bitmap",
                    "none",
                    "--name",
                    "newroot",
                    "/dev/disk/by-id/nvme-a",
                    "/dev/disk/by-id/nvme-b",
                ]
                && command.readiness == CommandReadiness::Ready
                && command.mutates
        })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "mdraids:missing-level:create"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "mdadm",
                        "--create",
                        "/dev/md/missing-level",
                        "--level",
                        "<level>",
                        "--raid-devices",
                        "2",
                        "--bitmap",
                        "none",
                        "--name",
                        "missing-level",
                        "/dev/disk/by-id/nvme-c",
                        "/dev/disk/by-id/nvme-d",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["RAID level"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "mdraids:missing-members:create"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "mdadm",
                        "--create",
                        "/dev/md/missing-members",
                        "--level",
                        "10",
                        "--raid-devices",
                        "<member-count>",
                        "--bitmap",
                        "none",
                        "--name",
                        "missing-members",
                        "<member-device>",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["member devices"]
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "mdraids:newroot:create"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["mdadm", "--detail", "/dev/md/newroot"])
    }));
}

#[test]
fn md_raid_lifecycle_requires_array_path_for_execute_readiness() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "mdRaids": {
                  "newroot": {
                    "operation": "create",
                    "level": "1",
                    "devices": [
                      "/dev/disk/by-id/nvme-a",
                      "/dev/disk/by-id/nvme-b"
                    ]
                  },
                  "root": {
                    "operation": "grow",
                    "desiredSize": "max"
                  },
                  "existing": {
                    "operation": "assemble"
                  },
                  "oldroot": {
                    "operation": "stop"
                  },
                  "inventory": {
                    "operation": "rescan"
                  }
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowGrow": true,
                "allowOffline": true,
                "allowDeviceReplacement": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(!report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "mdraids:newroot:create"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "mdadm",
                        "--create",
                        "<md-array>",
                        "--level",
                        "1",
                        "--raid-devices",
                        "2",
                        "--bitmap",
                        "none",
                        "/dev/disk/by-id/nvme-a",
                        "/dev/disk/by-id/nvme-b",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["MD array path"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "mdraids:root:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["mdadm", "--grow", "<md-array>", "--size", "max"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["MD array path"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "mdraids:existing:assemble"
            && step.commands.iter().any(|command| {
                command.argv == ["mdadm", "--assemble", "<md-array>", "<member-device>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["MD array path", "member devices"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "mdraids:oldroot:stop"
            && step.commands.iter().any(|command| {
                command.argv == ["mdadm", "--stop", "<md-array>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["MD array path"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "mdraids:inventory:rescan"
            && step
                .commands
                .iter()
                .all(|command| command.readiness == CommandReadiness::Ready)
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["mdadm", "--detail", "--scan"])
    }));

    let remove_action = PlannedAction {
        id: "mdRaids:root:remove-device:/dev/disk/by-id/failed-md-member".to_string(),
        description: "remove failed MD member".to_string(),
        operation: Operation::RemoveDevice,
        risk: RiskClass::PotentialDataLoss,
        destructive: false,
        context: ActionContext {
            collection: Some("mdRaids".to_string()),
            name: Some("root".to_string()),
            target: Some("root".to_string()),
            device: Some("/dev/disk/by-id/failed-md-member".to_string()),
            ..ActionContext::default()
        },
        advice: None,
    };
    let (commands, _, _) = commands_for_action(&remove_action);
    assert!(commands.iter().any(|command| {
        command.argv
            == [
                "mdadm",
                "<md-array>",
                "--remove",
                "/dev/disk/by-id/failed-md-member",
            ]
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command.unresolved_inputs == ["MD array path"]
    }));

    let add_action = PlannedAction {
        id: "mdRaids:root:add-device:/dev/disk/by-id/nvme-spare".to_string(),
        description: "add MD member".to_string(),
        operation: Operation::AddDevice,
        risk: RiskClass::Online,
        destructive: false,
        context: ActionContext {
            collection: Some("mdRaids".to_string()),
            name: Some("root".to_string()),
            target: Some("root".to_string()),
            device: Some("/dev/disk/by-id/nvme-spare".to_string()),
            ..ActionContext::default()
        },
        advice: None,
    };
    let replace_action = PlannedAction {
        id: "mdRaids:root:replace-device:/dev/disk/by-id/old-md-member".to_string(),
        description: "replace MD member".to_string(),
        operation: Operation::ReplaceDevice,
        risk: RiskClass::OfflineRequired,
        destructive: false,
        context: ActionContext {
            collection: Some("mdRaids".to_string()),
            name: Some("root".to_string()),
            target: Some("root".to_string()),
            device: Some("/dev/disk/by-id/old-md-member".to_string()),
            replacement: Some("/dev/disk/by-id/new-md-member".to_string()),
            ..ActionContext::default()
        },
        advice: None,
    };
    let (add_commands, _, _) = commands_for_action(&add_action);
    let (replace_commands, _, _) = commands_for_action(&replace_action);
    assert!(add_commands.iter().any(|command| {
        command.argv == ["mdadm", "<md-array>", "--add", "/dev/disk/by-id/nvme-spare"]
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command.unresolved_inputs == ["MD array path"]
    }));
    assert!(replace_commands.iter().any(|command| {
        command.argv
            == [
                "mdadm",
                "<md-array>",
                "--replace",
                "/dev/disk/by-id/old-md-member",
                "--with",
                "/dev/disk/by-id/new-md-member",
            ]
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command.unresolved_inputs == ["MD array path"]
    }));
}

#[test]
fn multipath_map_lifecycle_reports_multipath_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "multipathMaps": {
                  "mpatha": {
                    "target": "mpatha",
                    "operation": "grow",
                    "addDevices": ["/dev/sdb"],
                    "replaceDevices": {
                      "/dev/sdc": "/dev/sdd"
                    },
                    "removeDevices": ["/dev/sde"]
                  },
                  "mpathb": {
                    "target": "mpathb",
                    "operation": "rescan"
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

    assert_eq!(report.status, ExecutionStatus::Blocked);
    assert!(report.command_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["multipathd", "resize", "map", "mpatha"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["multipathd", "add", "path", "/dev/sdb"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "multipathmaps:mpathb:rescan"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["lsscsi", "-t", "-s"] && !command.mutates)
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["multipath", "-r"] && command.mutates)
            && step
                .commands
                .iter()
                .filter(|command| command.argv == ["multipath", "-ll", "mpathb"])
                .count()
                == 2
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["multipathd", "add", "path", "/dev/sdd"])
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["multipathd", "del", "path", "/dev/sdc"])
    }));
    assert!(
        !report.command_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["multipathd", "del", "path", "/dev/sde"])
        }),
        "potential-data-loss path removal remains blocked by apply policy"
    );
    assert!(report.verification_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["multipath", "-ll", "mpatha"])
    }));
}

#[test]
fn multipath_map_lifecycle_uses_declared_device_as_map_target_for_logical_names() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "multipathMaps": {
                  "root-map": {
                    "device": "/dev/mapper/mpatha",
                    "operation": "grow",
                    "addDevices": ["/dev/sdb"],
                    "replaceDevices": {
                      "/dev/sdc": "/dev/sdd"
                    }
                  },
                  "inventory": {
                    "device": "/dev/mapper/mpatha",
                    "operation": "rescan"
                  }
                }
              },
              "apply": {
                "allowGrow": true,
                "allowOffline": true,
                "allowDeviceReplacement": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "multipathmaps:root-map:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["multipathd", "resize", "map", "/dev/mapper/mpatha"]
            })
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["lsscsi", "-t", "-s"] && !command.mutates)
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["multipath", "-ll", "/dev/mapper/mpatha"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "multipathMaps:root-map:add-device:/dev/sdb"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["multipathd", "add", "path", "/dev/sdb"])
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["multipath", "-ll", "/dev/mapper/mpatha"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "multipathMaps:root-map:replace-device:/dev/sdc"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["multipathd", "add", "path", "/dev/sdd"])
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["multipathd", "del", "path", "/dev/sdc"])
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["multipath", "-ll", "/dev/mapper/mpatha"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "multipathmaps:inventory:rescan"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["lsscsi", "-t", "-s"] && !command.mutates)
            && step
                .commands
                .iter()
                .filter(|command| command.argv == ["multipath", "-ll", "/dev/mapper/mpatha"])
                .count()
                == 2
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "multipathmaps:root-map:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "/dev/mapper/mpatha", "--json"]
            })
    }));
}

#[test]
fn multipath_map_destroy_reports_flush_command() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "multipathMaps": {
                  "mpath-old": {
                    "target": "mpath-old",
                    "operation": "destroy"
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
        step.action_id == "multipathmaps:mpath-old:destroy"
            && step.commands.iter().any(|command| {
                command.argv == ["multipath", "-ll", "mpath-old"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["multipath", "-f", "mpath-old"]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "multipathmaps:mpath-old:destroy"
            && step.commands.iter().any(|command| {
                command.argv == ["multipath", "-ll"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "multipath", "--json"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
}

#[test]
fn multipath_map_lifecycle_requires_explicit_map_target_for_execute_readiness() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "multipathMaps": {
                  "root-map": {
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
        step.action_id == "multipathmaps:root-map:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["multipathd", "resize", "map", "<multipath-map>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["multipath map target"]
            })
    }));

    let remove_action = PlannedAction {
        id: "multipathMaps:root-map:remove-device:/dev/sde".to_string(),
        description: "remove stale multipath path".to_string(),
        operation: Operation::RemoveDevice,
        risk: RiskClass::PotentialDataLoss,
        destructive: false,
        context: ActionContext {
            collection: Some("multipathMaps".to_string()),
            name: Some("root-map".to_string()),
            target: Some("root-map".to_string()),
            device: Some("/dev/sde".to_string()),
            ..ActionContext::default()
        },
        advice: None,
    };
    let (commands, _, _) = commands_for_action(&remove_action);
    assert!(commands.iter().any(|command| {
        command.argv == ["multipath", "-ll", "<multipath-map>"]
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command.unresolved_inputs == ["multipath map target"]
    }));

    let destroy_action = PlannedAction {
        id: "multipathmaps:root-map:destroy".to_string(),
        description: "remove multipath map".to_string(),
        operation: Operation::Destroy,
        risk: RiskClass::OfflineRequired,
        destructive: false,
        context: ActionContext {
            collection: Some("multipathMaps".to_string()),
            name: Some("root-map".to_string()),
            target: Some("root-map".to_string()),
            ..ActionContext::default()
        },
        advice: None,
    };
    let (commands, _, _) = commands_for_action(&destroy_action);
    assert!(commands.iter().any(|command| {
        command.argv == ["multipath", "-f", "<multipath-map>"]
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command.unresolved_inputs == ["multipath map target"]
    }));
}

#[test]
fn thin_pool_lifecycle_reports_lvm_pool_commands_and_verification() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "thinPools": {
                  "vg0/newpool": {
                    "operation": "create",
                    "desiredSize": "100GiB"
                  },
                  "vg0/pool": {
                    "operation": "grow",
                    "desiredSize": "500GiB"
                  },
                  "vg0/reporting": {
                    "operation": "rescan"
                  },
                  "badthin": {
                    "operation": "create"
                  },
                  "vg0/oldpool": {
                    "destroy": true
                  }
                }
              },
              "apply": {
                "allowGrow": true,
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
                    "lvcreate",
                    "--type",
                    "thin-pool",
                    "--size",
                    "100GiB",
                    "--name",
                    "newpool",
                    "vg0",
                ]
                && command.readiness == CommandReadiness::Ready
        })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv == ["lvextend", "--size", "500GiB", "vg0/pool"]
                && command.readiness == CommandReadiness::Ready
        })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "thinpools:vg0/reporting:rescan"
            && step
                .commands
                .iter()
                .all(|command| command.readiness == CommandReadiness::Ready && !command.mutates)
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "lvs",
                        "--reportformat",
                        "json",
                        "-o",
                        "lv_name,lv_size,data_percent,metadata_percent,seg_monitor",
                        "vg0/reporting",
                    ]
            })
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "vg0/reporting"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "thinpools:badthin:create"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "lvcreate",
                        "--type",
                        "thin-pool",
                        "--size",
                        "<size>",
                        "--name",
                        "<thin-pool>",
                        "<volume-group>",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs
                        == [
                            "target in volume-group/thin-pool form",
                            "desired thin pool size",
                        ]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["lvremove", "--yes", "vg0/oldpool"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "thinpools:vg0/newpool:create"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "lvs",
                        "--reportformat",
                        "json",
                        "-o",
                        "lv_name,lv_size,data_percent,metadata_percent,seg_monitor",
                        "vg0/newpool",
                    ]
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv
                == [
                    "lvs",
                    "--reportformat",
                    "json",
                    "-o",
                    "lv_name,lv_size,data_percent,metadata_percent,seg_monitor",
                    "vg0/pool",
                ]
        })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "thinpools:vg0/reporting:rescan"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "lvs",
                        "--reportformat",
                        "json",
                        "-o",
                        "lv_name,lv_size,data_percent,metadata_percent,seg_monitor",
                        "vg0/reporting",
                    ]
            })
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "vg0/reporting", "--json"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "thinpools:vg0/oldpool:destroy"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "topology", "--json"])
    }));
}

#[test]
fn lvm_logical_volume_lifecycle_reports_lvm_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "volumes": {
                  "vg0/scratch": {
                    "operation": "create",
                    "desiredSize": "10GiB"
                  },
                  "scratch": {
                    "operation": "create"
                  },
                  "vg0/home": {
                    "operation": "activate"
                  },
                  "vg0/archive": {
                    "operation": "deactivate"
                  },
                  "vg0/reporting": {
                    "operation": "rescan"
                  },
                  "vg0/old": {
                    "destroy": true
                  }
                }
              },
              "apply": {
                "allowDestructive": false,
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::Blocked);
    assert!(report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv
                == [
                    "lvcreate",
                    "--yes",
                    "--wipesignatures",
                    "y",
                    "--size",
                    "10GiB",
                    "--name",
                    "scratch",
                    "vg0",
                ]
        })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "volumes:scratch:create"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "lvcreate",
                        "--yes",
                        "--wipesignatures",
                        "y",
                        "--size",
                        "<size>",
                        "--name",
                        "<logical-volume>",
                        "<volume-group>",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs
                        == [
                            "target in volume-group/logical-volume form",
                            "desired logical volume size",
                        ]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "volumes:vg0/home:activate"
            && step.commands.iter().any(|command| {
                command.argv == ["lvchange", "--activate", "y", "vg0/home"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "volumes:vg0/archive:deactivate"
            && step.commands.iter().any(|command| {
                command.argv == ["lvchange", "--activate", "n", "vg0/archive"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "volumes:vg0/reporting:rescan"
            && step
                .commands
                .iter()
                .all(|command| command.readiness == CommandReadiness::Ready && !command.mutates)
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["lvs", "--reportformat", "json", "vg0/reporting"])
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "vg0/reporting"])
    }));
    assert!(
        !report.command_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["lvremove", "--yes", "vg0/old"])
        }),
        "destructive LV removal remains blocked by default policy"
    );
    assert!(report.verification_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["lvs", "--reportformat", "json", "vg0/scratch"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "volumes:vg0/home:activate"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["lvs", "--reportformat", "json", "vg0/home"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "volumes:vg0/reporting:rescan"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["lvs", "--reportformat", "json", "vg0/reporting"])
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "vg0/reporting", "--json"])
    }));
}

#[test]
fn lvm_volume_update_and_remove_require_canonical_targets_for_execute_readiness() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "volumes": {
                  "scratch": {
                    "operation": "grow",
                    "desiredSize": "20GiB"
                  },
                  "old": {
                    "destroy": true
                  }
                },
                "thinPools": {
                  "pool": {
                    "operation": "grow",
                    "desiredSize": "200GiB"
                  },
                  "oldpool": {
                    "destroy": true
                  }
                }
              },
              "apply": {
                "allowGrow": true,
                "allowDestructive": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(!report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "volumes:scratch:grow"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "lvextend",
                        "--resizefs",
                        "--size",
                        "20GiB",
                        "<logical-volume>",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["target in volume-group/logical-volume form"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "volumes:old:destroy"
            && step.commands.iter().any(|command| {
                command.argv == ["lvremove", "--yes", "<logical-volume>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["target in volume-group/logical-volume form"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "thinpools:pool:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["lvextend", "--size", "200GiB", "<thin-pool>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["target in volume-group/thin-pool form"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "thinpools:oldpool:destroy"
            && step.commands.iter().any(|command| {
                command.argv == ["lvremove", "--yes", "<thin-pool>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["target in volume-group/thin-pool form"]
            })
    }));
}

#[test]
fn lvm_volume_and_thin_pool_lifecycle_accept_target_aliases_for_logical_names() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "volumes": {
                  "scratch": {
                    "operation": "grow",
                    "target": "vg0/scratch",
                    "desiredSize": "20GiB"
                  },
                  "old": {
                    "destroy": true,
                    "path": "vg0/old"
                  }
                },
                "thinPools": {
                  "pool": {
                    "operation": "grow",
                    "target": "vg0/pool",
                    "desiredSize": "200GiB"
                  },
                  "oldpool": {
                    "destroy": true,
                    "path": "vg0/oldpool"
                  }
                }
              },
              "apply": {
                "allowGrow": true,
                "allowDestructive": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "volumes:scratch:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["lvextend", "--resizefs", "--size", "20GiB", "vg0/scratch"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "volumes:old:destroy"
            && step.commands.iter().any(|command| {
                command.argv == ["lvremove", "--yes", "vg0/old"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "thinpools:pool:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["lvextend", "--size", "200GiB", "vg0/pool"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "thinpools:oldpool:destroy"
            && step.commands.iter().any(|command| {
                command.argv == ["lvremove", "--yes", "vg0/oldpool"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
}

#[test]
fn lvm_volume_group_lifecycle_reports_lvm_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "volumeGroups": {
                  "vg0": {
                    "operation": "create",
                    "device": "/dev/disk/by-id/nvme-vg0"
                  },
                  "vgdata": {
                    "operation": "grow",
                    "device": "/dev/disk/by-id/nvme-data-pv"
                  },
                  "vgrefresh": {
                    "operation": "rescan"
                  },
                  "vgmissing": {
                    "operation": "grow"
                  },
                  "vgadd": {
                    "operation": "add-device"
                  },
                  "vgreplace": {
                    "operation": "replace-device",
                    "device": "/dev/disk/by-id/old-pv"
                  },
                  "importvg": {
                    "operation": "import"
                  },
                  "exportvg": {
                    "operation": "export"
                  },
                  "activevg": {
                    "operation": "activate"
                  },
                  "coldvg": {
                    "operation": "deactivate"
                  },
                  "oldvg": {
                    "destroy": true
                  }
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv == ["vgcreate", "vg0", "/dev/disk/by-id/nvme-vg0"]
                && command.readiness == CommandReadiness::Ready
        })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "volumegroups:vgdata:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["vgextend", "vgdata", "/dev/disk/by-id/nvme-data-pv"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "volumegroups:vgrefresh:rescan"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["pvscan", "--cache"])
            && step.commands.iter().any(|command| {
                command.argv == ["vgchange", "--refresh", "vgrefresh"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "volumegroups:vgmissing:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["vgextend", "vgmissing", "<physical-volume>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["physical volume device"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "volumegroups:vgadd:adddevice"
            && step.commands.iter().any(|command| {
                command.argv == ["vgextend", "vgadd", "<physical-volume>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["physical volume device"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "volumegroups:vgreplace:replacedevice"
            && step.commands.iter().any(|command| {
                command.argv == ["vgextend", "vgreplace", "<replacement-physical-volume>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["replacement physical volume"]
            })
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "pvmove",
                        "/dev/disk/by-id/old-pv",
                        "<replacement-physical-volume>",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["replacement physical volume"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["vgremove", "--yes", "oldvg"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "volumegroups:importvg:import"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["pvs", "--reportformat", "json"])
            && step.commands.iter().any(|command| {
                command.argv == ["vgimport", "importvg"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "volumegroups:exportvg:export"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["vgs", "--reportformat", "json", "exportvg"])
            && step.commands.iter().any(|command| {
                command.argv == ["vgexport", "exportvg"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "volumegroups:activevg:activate"
            && step.commands.iter().any(|command| {
                command.argv == ["vgchange", "--activate", "y", "activevg"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "volumegroups:coldvg:deactivate"
            && step.commands.iter().any(|command| {
                command.argv == ["vgchange", "--activate", "n", "coldvg"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "volumegroups:vg0:create"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["vgs", "--reportformat", "json", "vg0"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "volumegroups:vgdata:grow"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["pvs", "--reportformat", "json"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "volumegroups:vgrefresh:rescan"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["lvs", "--reportformat", "json", "vgrefresh"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "volumegroups:oldvg:destroy"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["pvs", "--reportformat", "json"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "volumegroups:importvg:import"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["vgs", "--reportformat", "json", "importvg"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "volumegroups:exportvg:export"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "exportvg", "--json"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "volumegroups:activevg:activate"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["lvs", "--reportformat", "json", "activevg"])
    }));
}

#[test]
fn lvm_physical_volume_lifecycle_reports_lvm_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "physicalVolumes": {
                  "/dev/disk/by-id/nvme-pv-new": {
                    "operation": "create"
                  },
                  "/dev/disk/by-id/nvme-pv-grow": {
                    "operation": "grow"
                  },
                  "/dev/disk/by-id/nvme-pv-refresh": {
                    "operation": "rescan"
                  },
                  "/dev/disk/by-id/nvme-pv-old": {
                    "destroy": true
                  }
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowGrow": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "physicalvolumes:/dev/disk/by-id/nvme-pv-new:create"
            && step.commands.iter().any(|command| {
                command.argv == ["pvcreate", "/dev/disk/by-id/nvme-pv-new"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "physicalvolumes:/dev/disk/by-id/nvme-pv-grow:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["pvresize", "/dev/disk/by-id/nvme-pv-grow"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "physicalvolumes:/dev/disk/by-id/nvme-pv-refresh:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["pvscan", "--cache", "/dev/disk/by-id/nvme-pv-refresh"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "physicalvolumes:/dev/disk/by-id/nvme-pv-old:destroy"
            && step.commands.iter().any(|command| {
                command.argv == ["pvremove", "--yes", "/dev/disk/by-id/nvme-pv-old"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "physicalvolumes:/dev/disk/by-id/nvme-pv-grow:grow"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["pvs", "--reportformat", "json"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "physicalvolumes:/dev/disk/by-id/nvme-pv-refresh:rescan"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "topology", "--json"])
    }));
}

#[test]
fn lvm_physical_volume_lifecycle_requires_device_path_for_execute_readiness() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "physicalVolumes": {
                  "logical-pv": {
                    "operation": "create"
                  },
                  "refresh-all": {
                    "operation": "rescan"
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
        step.action_id == "physicalvolumes:logical-pv:create"
            && step.commands.iter().any(|command| {
                command.argv == ["pvcreate", "<physical-volume>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["physical volume device"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "physicalvolumes:refresh-all:rescan"
            && step
                .commands
                .iter()
                .all(|command| command.readiness == CommandReadiness::Ready)
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["pvscan", "--cache"])
    }));
}

#[test]
fn lvm_physical_volume_lifecycle_accepts_path_aliases_for_logical_names() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "physicalVolumes": {
                  "new-pv": {
                    "operation": "create",
                    "path": "/dev/disk/by-id/nvme-pv-new"
                  },
                  "grow-pv": {
                    "operation": "grow",
                    "target": "/dev/disk/by-id/nvme-pv-grow"
                  },
                  "old-pv": {
                    "destroy": true,
                    "device": "/dev/disk/by-id/nvme-pv-old"
                  }
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowGrow": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "physicalvolumes:new-pv:create"
            && step.commands.iter().any(|command| {
                command.argv == ["pvcreate", "/dev/disk/by-id/nvme-pv-new"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "physicalvolumes:grow-pv:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["pvresize", "/dev/disk/by-id/nvme-pv-grow"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "physicalvolumes:old-pv:destroy"
            && step.commands.iter().any(|command| {
                command.argv == ["pvremove", "--yes", "/dev/disk/by-id/nvme-pv-old"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
}

#[test]
fn lvm_snapshot_lifecycle_reports_lvm_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "lvmSnapshots": {
                  "vg0/root-snap": {
                    "operation": "snapshot",
                    "target": "vg0/root",
                    "desiredSize": "20GiB"
                  },
                  "vg0/root-rollback": {
                    "operation": "rollback"
                  },
                  "vg0/root-inspect": {
                    "operation": "rescan"
                  },
                  "vg0/old-snap": {
                    "destroy": true
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

    assert_eq!(report.status, ExecutionStatus::Blocked);
    assert!(report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv
                == [
                    "lvcreate",
                    "--snapshot",
                    "--size",
                    "20GiB",
                    "--name",
                    "vg0/root-snap",
                    "vg0/root",
                ]
        })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["lvremove", "--yes", "vg0/old-snap"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "lvmsnapshots:vg0/root-inspect:rescan"
            && step
                .commands
                .iter()
                .all(|command| command.readiness == CommandReadiness::Ready && !command.mutates)
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "lvs",
                        "--reportformat",
                        "json",
                        "-o",
                        "lv_name,origin,lv_attr,data_percent,metadata_percent,lv_size",
                        "vg0/root-inspect",
                    ]
            })
    }));
    assert!(
        !report.command_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["lvconvert", "--merge", "vg0/root-rollback"])
        }),
        "potential-data-loss rollback remains blocked by apply policy"
    );
    assert!(report.verification_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["lvs", "--reportformat", "json", "vg0/root-snap"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "lvmsnapshots:vg0/root-inspect:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["lvs", "--reportformat", "json", "vg0/root-inspect"]
            })
    }));
}

#[test]
fn loop_device_lifecycle_reports_losetup_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "loopDevices": {
                  "/dev/loop7": {
                    "operation": "create",
                    "device": "/var/lib/images/root.img"
                  },
                  "/dev/loop8": {
                    "operation": "grow"
                  },
                  "/dev/loop10": {
                    "operation": "rescan"
                  },
                  "/dev/loop9": {
                    "operation": "destroy"
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

    assert_eq!(report.status, ExecutionStatus::Blocked);
    assert!(report.command_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["losetup", "/dev/loop7", "/var/lib/images/root.img"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["losetup", "-c", "/dev/loop8"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "loopdevices:/dev/loop10:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["losetup", "--json", "--list", "/dev/loop10"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "/dev/loop10"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(
        !report.command_plan.iter().any(|step| {
            step.commands
                .iter()
                .any(|command| command.argv == ["losetup", "--detach", "/dev/loop9"])
        }),
        "offline detach remains blocked by default policy"
    );
    assert!(report.verification_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["losetup", "--json", "--list", "/dev/loop8"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "loopdevices:/dev/loop10:rescan"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "/dev/loop10", "--json"])
    }));
}

#[test]
fn loop_device_property_reports_blockdev_command() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "loopDevices": {
                  "/dev/loop7": {
                    "properties": {
                      "loop.read-only": true
                    }
                  },
                  "/dev/loop8": {
                    "properties": {
                      "loop.direct-io": false
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
        step.action_id == "loopDevices:/dev/loop7:set-property:loop.read-only"
            && step.commands.iter().any(|command| {
                command.argv == ["blockdev", "--setro", "/dev/loop7"]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "loopDevices:/dev/loop8:set-property:loop.direct-io"
            && step.commands.iter().any(|command| {
                command.argv == ["losetup", "--direct-io=off", "/dev/loop8"]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
}

#[test]
fn loop_device_update_and_detach_require_stable_loop_path_for_execute_readiness() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "loopDevices": {
                  "root-image": {
                    "operation": "grow"
                  },
                  "inventory-image": {
                    "operation": "rescan"
                  },
                  "old-image": {
                    "operation": "destroy"
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
    assert!(!report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "loopdevices:root-image:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["losetup", "-c", "<loop-device>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["loop device path"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "loopdevices:inventory-image:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "<loop-device>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["loop device path"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "loopdevices:old-image:destroy"
            && step.commands.iter().any(|command| {
                command.argv == ["losetup", "--detach", "<loop-device>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["loop device path"]
            })
    }));
}

#[test]
fn backing_file_lifecycle_reports_file_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "backingFiles": {
                  "/var/lib/images/new.img": {
                    "operation": "create",
                    "desiredSize": "8GiB"
                  },
                  "/var/lib/images/root.img": {
                    "operation": "grow",
                    "desiredSize": "16GiB"
                  },
                  "inventory-image": {
                    "operation": "rescan",
                    "path": "/var/lib/images/inventory.img"
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
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "backingfiles:/var/lib/images/new.img:create"
            && step.commands.iter().any(|command| {
                command.argv == ["test", "!", "-e", "/var/lib/images/new.img"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["truncate", "--size", "8GiB", "/var/lib/images/new.img"]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "backingfiles:/var/lib/images/root.img:grow"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "stat",
                        "--printf=%n %s %b %B\\n",
                        "/var/lib/images/root.img",
                    ]
                    && !command.mutates
            })
            && step.commands.iter().any(|command| {
                command.argv == ["truncate", "--size", "16GiB", "/var/lib/images/root.img"]
                    && command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "backingfiles:inventory-image:rescan"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "du",
                        "--bytes",
                        "--apparent-size",
                        "/var/lib/images/inventory.img",
                    ]
                    && !command.mutates
            })
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "/var/lib/images/inventory.img"]
                    && !command.mutates
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "backingfiles:/var/lib/images/new.img:create"
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "/var/lib/images/new.img", "--json"]
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "backingfiles:/var/lib/images/root.img:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "/var/lib/images/root.img", "--json"]
            })
    }));
}

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

#[test]
fn failed_iscsi_session_login_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "iscsiSessions": {
                "iqn.2026-06.example:storage.root": {
                  "operation": "login",
                  "portal": "192.0.2.10:3260"
                }
              },
              "apply": {
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_login = [
        "iscsiadm",
        "--mode",
        "node",
        "--targetname",
        "iqn.2026-06.example:storage.root",
        "--portal",
        "192.0.2.10:3260",
        "--login",
    ];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_login,
            status_code: Some(if argv == failed_login { 15 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_login {
                "login failed".to_string()
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
                    "iscsiadm",
                    "--mode",
                    "node",
                    "--targetname",
                    "iqn.2026-06.example:storage.root",
                    "--portal",
                    "192.0.2.10:3260",
                    "--login",
                ]
    }));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("iSCSI session domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Login"));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["iscsiadm", "--mode", "session"] && !command.mutates }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv
            == [
                "iscsiadm",
                "--mode",
                "node",
                "--targetname",
                "iqn.2026-06.example:storage.root",
            ]
            && !command.mutates
    }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["lsscsi", "-t", "-s"] && !command.mutates }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["multipath", "-ll"] && !command.mutates }));
    assert!(domain_recovery.notes.iter().any(|note| {
        note.contains("iSCSI session changes") && note.contains("login or logout")
    }));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("iSCSI roll-forward recovery review is reported");
    assert!(roll_forward
        .commands
        .iter()
        .any(|command| { command.argv == ["iscsiadm", "--mode", "session"] && !command.mutates }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("iSCSI rollback recovery review is reported");
    assert!(rollback
        .commands
        .iter()
        .any(|command| { command.argv == ["multipath", "-ll"] && !command.mutates }));
}

#[test]
fn failed_vdo_growth_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "vdoVolumes": {
                "archive": {
                  "operation": "grow",
                  "desiredSize": "4TiB"
                }
              },
              "apply": {
                "allowGrow": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_grow = [
        "vdo",
        "growLogical",
        "--name",
        "archive",
        "--vdoLogicalSize",
        "4TiB",
    ];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_grow,
            status_code: Some(if argv == failed_grow { 1 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_grow {
                "growth failed".to_string()
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
                    "vdo",
                    "growLogical",
                    "--name",
                    "archive",
                    "--vdoLogicalSize",
                    "4TiB",
                ]
    }));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("VDO domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Grow"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["vdo", "status", "--name", "archive"] && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["vdostats", "--human-readable", "archive"] && !command.mutates
    }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["disk-nix", "vdo", "--json"] && !command.mutates }));
    assert!(domain_recovery.notes.iter().any(|note| {
        note.contains("VDO lifecycle changes") && note.contains("create, grow, start, stop")
    }));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("VDO roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv == ["vdo", "status", "--name", "archive"] && !command.mutates
    }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("VDO rollback recovery review is reported");
    assert!(rollback.commands.iter().any(|command| {
        command.argv == ["vdostats", "--human-readable", "archive"] && !command.mutates
    }));
}

#[test]
fn failed_multipath_resize_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "multipathMaps": {
                "root-map": {
                  "device": "/dev/mapper/mpatha",
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

    let failed_resize = ["multipathd", "resize", "map", "/dev/mapper/mpatha"];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_resize,
            status_code: Some(if argv == failed_resize { 1 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_resize {
                "resize failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report.execution_results.iter().any(|result| {
        !result.success && result.argv == ["multipathd", "resize", "map", "/dev/mapper/mpatha"]
    }));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("multipath domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Grow"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["multipath", "-ll", "/dev/mapper/mpatha"] && !command.mutates
    }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["lsscsi", "-t", "-s"] && !command.mutates }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["disk-nix", "multipath", "--json"] && !command.mutates
    }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| { note.contains("multipath changes") && note.contains("reload, resize") }));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("multipath roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv == ["multipath", "-ll", "/dev/mapper/mpatha"] && !command.mutates
    }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("multipath rollback recovery review is reported");
    assert!(rollback.commands.iter().any(|command| {
        command.argv == ["disk-nix", "multipath", "--json"] && !command.mutates
    }));
}

#[test]
fn failed_multipath_replace_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "multipathMaps": {
                "root-map": {
                  "device": "/dev/mapper/mpatha",
                  "replaceDevices": {
                    "/dev/sdc": "/dev/sdd"
                  }
                }
              },
              "apply": {
                "allowOffline": true,
                "allowDeviceReplacement": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_delete = ["multipathd", "del", "path", "/dev/sdc"];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_delete,
            status_code: Some(if argv == failed_delete { 87 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_delete {
                "multipath replacement delete failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report.execution_results.iter().any(|result| {
        result.success && result.argv == ["multipathd", "add", "path", "/dev/sdd"]
    }));
    assert!(report.execution_results.iter().any(|result| {
        !result.success && result.argv == ["multipathd", "del", "path", "/dev/sdc"]
    }));
    let partial = report
        .partial_execution_recovery
        .as_ref()
        .expect("partial execution recovery is reported");
    assert_eq!(
        partial.failed_action_id,
        "multipathMaps:root-map:replace-device:/dev/sdc"
    );
    assert_eq!(
        partial.failed_command,
        vec!["multipathd", "del", "path", "/dev/sdc"]
    );
    assert_eq!(partial.completed_mutating_command_count, 1);
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("multipath replacement domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("ReplaceDevice"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["multipath", "-ll", "/dev/mapper/mpatha"] && !command.mutates
    }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["lsscsi", "-t", "-s"] && !command.mutates }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["disk-nix", "multipath", "--json"] && !command.mutates
    }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| { note.contains("multipath changes") && note.contains("path removal") }));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("multipath replacement roll-forward recovery review is reported");
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
        .expect("multipath replacement rollback recovery review is reported");
    assert!(rollback.commands.iter().all(|command| !command.mutates));
    assert!(rollback.commands.iter().any(|command| {
        command.argv == ["disk-nix", "multipath", "--json"] && !command.mutates
    }));
}

#[test]
fn failed_luks_open_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "luks": {
                "devices": {
                  "cryptarchive": {
                    "name": "cryptarchive",
                    "device": "/dev/disk/by-id/archive-luks",
                    "operation": "open"
                  }
                }
              },
              "apply": {
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_open = [
        "cryptsetup",
        "open",
        "/dev/disk/by-id/archive-luks",
        "cryptarchive",
    ];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_open,
            status_code: Some(if argv == failed_open { 2 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_open {
                "open failed".to_string()
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
                    "cryptsetup",
                    "open",
                    "/dev/disk/by-id/archive-luks",
                    "cryptarchive",
                ]
    }));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("LUKS domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Open"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/archive-luks"]
            && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["cryptsetup", "status", "cryptarchive"] && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv
            == [
                "disk-nix",
                "inspect",
                "/dev/disk/by-id/archive-luks",
                "--json",
            ]
            && !command.mutates
    }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| note.contains("LUKS changes")));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| note.contains("alternate unlock paths")));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("LUKS roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv == ["cryptsetup", "status", "cryptarchive"] && !command.mutates
    }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("LUKS rollback recovery review is reported");
    assert!(rollback.commands.iter().any(|command| {
        command.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/archive-luks"]
            && !command.mutates
    }));
}

#[test]
fn failed_lvm_volume_growth_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "volumes": {
                "root": {
                  "target": "vg0/root",
                  "operation": "grow",
                  "desiredSize": "50GiB"
                }
              },
              "apply": {
                "allowGrow": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_extend = ["lvextend", "--resizefs", "--size", "50GiB", "vg0/root"];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_extend,
            status_code: Some(if argv == failed_extend { 5 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_extend {
                "extend failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report.execution_results.iter().any(|result| {
        !result.success && result.argv == ["lvextend", "--resizefs", "--size", "50GiB", "vg0/root"]
    }));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("LVM domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Grow"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["lvs", "--reportformat", "json", "vg0/root"] && !command.mutates
    }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["vgs", "--reportformat", "json"] && !command.mutates }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["pvs", "--reportformat", "json"] && !command.mutates }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["disk-nix", "inspect", "vg0/root", "--json"] && !command.mutates
    }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| { note.contains("LVM changes") && note.contains("activation, resize") }));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("LVM roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv == ["lvs", "--reportformat", "json", "vg0/root"] && !command.mutates
    }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("LVM rollback recovery review is reported");
    assert!(rollback.commands.iter().any(|command| {
        command.argv == ["disk-nix", "inspect", "vg0/root", "--json"] && !command.mutates
    }));
}

#[test]
fn failed_lvm_volume_group_rename_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "volumeGroups": {
                "vg-old": {
                  "operation": "rename",
                  "renameTo": "vg-new"
                }
              },
              "apply": {
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_rename = ["vgrename", "vg-old", "vg-new"];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_rename,
            status_code: Some(if argv == failed_rename { 5 } else { 0 }),
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
        .expect("LVM VG domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Rename"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["vgs", "--reportformat", "json", "vg-old"] && !command.mutates
    }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["pvs", "--reportformat", "json"] && !command.mutates }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["lvs", "--reportformat", "json", "-a"] && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["disk-nix", "inspect", "vg-old", "--json"] && !command.mutates
    }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| note.contains("LVM changes") && note.contains("import, export")));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("LVM VG roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv == ["vgs", "--reportformat", "json", "vg-old"] && !command.mutates
    }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("LVM VG rollback recovery review is reported");
    assert!(rollback.commands.iter().any(|command| {
        command.argv == ["disk-nix", "inspect", "vg-old", "--json"] && !command.mutates
    }));
}

#[test]
fn failed_bcache_property_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "caches": {
                "writeback-cache": {
                  "path": "/dev/bcache1",
                  "properties": {
                    "bcache.cache-mode": "writearound"
                  }
                }
              },
              "apply": {
                "allowPropertyChanges": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_property = [
        "sh",
        "-c",
        "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/$3\"",
        "disk-nix-bcache-property",
        "/dev/bcache1",
        "writearound",
        "cache_mode",
    ];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_property,
            status_code: Some(if argv == failed_property { 5 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_property {
                "cache mode failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report
        .execution_results
        .iter()
        .any(|result| !result.success && result.argv == failed_property));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("bcache domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("SetProperty"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv
            == [
                "sh",
                "-c",
                "cat \"/sys/block/${1#/dev/}/bcache/$2\"",
                "disk-nix-bcache-read",
                "/dev/bcache1",
                "state",
            ]
            && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv
            == [
                "sh",
                "-c",
                "cat \"/sys/block/${1#/dev/}/bcache/$2\"",
                "disk-nix-bcache-read",
                "/dev/bcache1",
                "dirty_data",
            ]
            && !command.mutates
    }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["disk-nix", "cache", "--json"] && !command.mutates }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["disk-nix", "inspect", "/dev/bcache1", "--json"] && !command.mutates
    }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| note.contains("cache changes") && note.contains("dirty-data")));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("bcache rollback recovery review is reported");
    assert!(rollback.commands.iter().any(|command| {
        command.argv
            == [
                "sh",
                "-c",
                "cat \"/sys/block/${1#/dev/}/bcache/$2\"",
                "disk-nix-bcache-read",
                "/dev/bcache1",
                "cache_mode",
            ]
            && !command.mutates
    }));
}

#[test]
fn failed_lvm_cache_property_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "lvmCaches": {
                "vg0/root": {
                  "properties": {
                    "lvm.cache-mode": "writethrough"
                  }
                }
              },
              "apply": {
                "allowPropertyChanges": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_property = ["lvchange", "--cachemode", "writethrough", "vg0/root"];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_property,
            status_code: Some(if argv == failed_property { 5 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_property {
                "cache mode failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report
        .execution_results
        .iter()
        .any(|result| !result.success && result.argv == failed_property));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("LVM cache domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("SetProperty"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv
            == [
                "lvs",
                "--reportformat",
                "json",
                "-a",
                "-o",
                "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent",
                "vg0/root",
            ]
            && !command.mutates
    }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["vgs", "--reportformat", "json"] && !command.mutates }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["pvs", "--reportformat", "json"] && !command.mutates }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["disk-nix", "inspect", "vg0/root", "--json"] && !command.mutates
    }));
}

#[test]
fn failed_vdo_property_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "vdoVolumes": {
                "archive": {
                  "properties": {
                    "writePolicy": "sync"
                  }
                }
              },
              "apply": {
                "allowPropertyChanges": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_property = [
        "vdo",
        "changeWritePolicy",
        "--name",
        "archive",
        "--writePolicy",
        "sync",
    ];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_property,
            status_code: Some(if argv == failed_property { 86 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_property {
                "VDO write policy failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report
        .execution_results
        .iter()
        .any(|result| !result.success && result.argv == failed_property));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("VDO domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("SetProperty"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["vdo", "status", "--name", "archive"] && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["vdostats", "--human-readable", "archive"] && !command.mutates
    }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["disk-nix", "vdo", "--json"] && !command.mutates }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| { note.contains("VDO lifecycle changes") && note.contains("operating mode") }));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("VDO roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv == ["vdo", "status", "--name", "archive"] && !command.mutates
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
        .expect("VDO rollback recovery review is reported");
    assert!(rollback.commands.iter().all(|command| !command.mutates));
    assert!(rollback.commands.iter().any(|command| {
        command.argv == ["vdostats", "--human-readable", "archive"] && !command.mutates
    }));
    assert!(report
        .recovery_actions
        .iter()
        .any(|action| action.kind == RecoveryActionKind::PreserveRecoveryPoints));
}

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

#[test]
fn failed_partition_growth_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "partitions": {
                "root": {
                  "operation": "grow",
                  "target": "/dev/disk/by-id/nvme-root-part2",
                  "device": "/dev/disk/by-id/nvme-root",
                  "partitionNumber": 2,
                  "end": "100%"
                }
              },
              "apply": {
                "allowOffline": true,
                "allowGrow": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_resize = [
        "parted",
        "-s",
        "/dev/disk/by-id/nvme-root",
        "resizepart",
        "2",
        "100%",
    ];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_resize,
            status_code: Some(if argv == failed_resize { 1 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_resize {
                "resizepart failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report
        .execution_results
        .iter()
        .any(|result| !result.success && result.argv == failed_resize));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("partition domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Grow"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["parted", "-lm", "/dev/disk/by-id/nvme-root"] && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv
            == [
                "lsblk",
                "--json",
                "--bytes",
                "--output-all",
                "/dev/disk/by-id/nvme-root",
            ]
            && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["disk-nix", "inspect", "/dev/disk/by-id/nvme-root", "--json"]
            && !command.mutates
    }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| note.contains("partition-table changes") && note.contains("kernel")));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("partition roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv == ["parted", "-lm", "/dev/disk/by-id/nvme-root"] && !command.mutates
    }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("partition rollback recovery review is reported");
    assert!(rollback.commands.iter().any(|command| {
        command.argv
            == [
                "lsblk",
                "--json",
                "--bytes",
                "--output-all",
                "/dev/disk/by-id/nvme-root",
            ]
            && !command.mutates
    }));
}

#[test]
fn failed_dm_map_rename_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "dmMaps": {
                "cryptswap": {
                  "operation": "rename",
                  "target": "/dev/mapper/cryptswap",
                  "renameTo": "/dev/mapper/cryptswap-retired"
                }
              },
              "apply": {
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_rename = [
        "dmsetup",
        "rename",
        "/dev/mapper/cryptswap",
        "cryptswap-retired",
    ];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_rename,
            status_code: Some(if argv == failed_rename { 1 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_rename {
                "dm rename failed".to_string()
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
        .expect("device-mapper domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Rename"));
    assert!(domain_recovery.commands.iter().any(|command| {
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
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["dmsetup", "deps", "-o", "devname", "/dev/mapper/cryptswap"]
            && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["dmsetup", "table", "/dev/mapper/cryptswap"] && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["dmsetup", "status", "/dev/mapper/cryptswap"] && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["disk-nix", "inspect", "/dev/mapper/cryptswap", "--json"]
            && !command.mutates
    }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| note.contains("local mapping changes") && note.contains("dependencies")));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("device-mapper roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv == ["dmsetup", "status", "/dev/mapper/cryptswap"] && !command.mutates
    }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("device-mapper rollback recovery review is reported");
    assert!(rollback.commands.iter().any(|command| {
        command.argv == ["dmsetup", "table", "/dev/mapper/cryptswap"] && !command.mutates
    }));
}

#[test]
fn failed_nfs_mount_remount_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "nfs": {
                "mounts": {
                  "/srv/tuned": {
                    "operation": "remount",
                    "source": "nas.example.com:/srv/tuned",
                    "options": ["_netdev", "ro", "vers=4.2"]
                  }
                }
              },
              "apply": {
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_remount = ["mount", "-o", "remount,_netdev,ro,vers=4.2", "/srv/tuned"];
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
    assert!(report
        .execution_results
        .iter()
        .any(|result| !result.success && result.argv == failed_remount));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("NFS domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Remount"));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["findmnt", "--json", "/srv/tuned"] && !command.mutates
    }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["nfsstat", "-m", "/srv/tuned"] && !command.mutates }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["disk-nix", "inspect", "/srv/tuned", "--json"] && !command.mutates
    }));
    assert!(domain_recovery
        .notes
        .iter()
        .any(|note| note.contains("NFS changes") && note.contains("negotiated mount options")));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("NFS roll-forward recovery review is reported");
    assert!(roll_forward
        .commands
        .iter()
        .any(|command| { command.argv == ["nfsstat", "-m", "/srv/tuned"] && !command.mutates }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("NFS rollback recovery review is reported");
    assert!(rollback.commands.iter().any(|command| {
        command.argv == ["findmnt", "--json", "/srv/tuned"] && !command.mutates
    }));
}

#[test]
fn blocked_policy_reports_blocked_status() {
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

    assert_eq!(report.status, ExecutionStatus::Blocked);
    assert_eq!(report.apply.blocked_count, 1);
    assert_eq!(report.command_summary.command_count, 0);
    assert!(report.command_summary.all_commands_ready());
    assert!(report.command_plan.is_empty());
    assert_eq!(report.verification_summary.step_count, 0);
    assert!(report.verification_plan.is_empty());
    assert!(report.recovery_actions.iter().any(|action| {
        action.kind == RecoveryActionKind::ReviewPolicy
            && action.summary.contains("Review blocked actions")
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
fn filesystem_growth_reports_read_only_verification_steps() {
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

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert_eq!(report.verification_plan.len(), 1);
    assert!(report.verification_plan[0].commands.iter().any(|command| {
        command.argv == ["findmnt", "--json", "--bytes", "/"] && !command.mutates
    }));
    assert!(report.verification_plan[0]
        .checks
        .iter()
        .any(|check| check.contains("filesystem size")));
}

#[test]
fn allowed_lun_growth_reports_rescan_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "luns": {
                "iqn.2026-06.example:storage/root:0": {
                  "operation": "grow",
                  "device": "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0",
                  "devices": [
                    "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-0"
                  ]
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
    assert_eq!(report.command_plan.len(), 1);
    assert!(report.command_plan[0].commands.iter().any(|command| {
        command.argv == ["iscsiadm", "--mode", "session", "--rescan"] && command.mutates
    }));
    assert!(report.command_plan[0].commands.iter().any(|command| {
        command.argv == ["lsscsi", "-t", "-s"]
            && !command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(report.command_plan[0].commands.iter().any(|command| {
            command.argv
                == [
                    "sh",
                    "-c",
                    "block=$(basename \"$(readlink -f \"$1\")\"); printf '1\\n' > \"/sys/class/block/${block}/device/rescan\"",
                    "disk-nix-scsi-rescan",
                    "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0",
                ]
                && command.mutates
                && command.readiness == CommandReadiness::Ready
        }));
    assert!(report.command_plan[0].commands.iter().any(|command| {
            command.argv
                == [
                    "sh",
                    "-c",
                    "block=$(basename \"$(readlink -f \"$1\")\"); printf '1\\n' > \"/sys/class/block/${block}/device/rescan\"",
                    "disk-nix-scsi-rescan",
                    "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-0",
                ]
                && command.mutates
                && command.readiness == CommandReadiness::Ready
        }));
    assert!(report.verification_plan[0].commands.iter().any(|command| {
        command.argv == ["lsscsi", "-t", "-s"]
            && !command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(report.verification_plan[0].commands.iter().any(|command| {
        command.argv
            == [
                "blockdev",
                "--getsize64",
                "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0",
            ]
            && !command.mutates
    }));
}

#[test]
fn host_storage_rescan_reports_online_refresh_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "luns": {
                  "iqn.2026-06.example:storage/root:0": {
                    "operation": "rescan",
                    "devices": [
                      "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0"
                    ]
                  }
                },
                "iscsiSessions": {
                  "iqn.2026-06.example:storage.root": {
                    "operation": "rescan"
                  }
                },
                "nvmeNamespaces": {
                  "/dev/nvme2": {
                    "operation": "rescan"
                  }
                }
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.apply.allowed_count >= 3);
    assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luns:iqn.2026-06.example:storage/root:0:rescan"
                && step.commands.iter().any(|command| {
                    command.argv == ["iscsiadm", "--mode", "session", "--rescan"]
                        && command.mutates
                })
                && step.commands.iter().any(|command| {
                    command.argv == ["lsscsi", "-t", "-s"] && !command.mutates
                })
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "sh",
                            "-c",
                            "block=$(basename \"$(readlink -f \"$1\")\"); printf '1\\n' > \"/sys/class/block/${block}/device/rescan\"",
                            "disk-nix-scsi-rescan",
                            "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["multipath", "-r"])
        }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "iscsisessions:iqn.2026-06.example:storage.root:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["iscsiadm", "--mode", "session", "--rescan"] && command.mutates
            })
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["lsscsi", "-t", "-s"] && !command.mutates)
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nvmenamespaces:/dev/nvme2:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["nvme", "list-subsys", "--output-format=json"] && !command.mutates
            })
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["nvme", "ns-rescan", "/dev/nvme2"])
    }));
    assert!(report.command_summary.all_commands_ready());
}

#[test]
fn lun_attach_and_detach_reports_host_path_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "luns": {
                "iqn.2026-06.example:storage/root:0": {
                  "operation": "attach",
                  "device": "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0"
                },
                "iqn.2026-06.example:storage/old:1": {
                  "operation": "detach",
                  "devices": [
                    "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-1"
                  ]
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
        step.action_id == "luns:iqn.2026-06.example:storage/root:0:attach"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["iscsiadm", "--mode", "session", "--rescan"])
    }));
    assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luns:iqn.2026-06.example:storage/root:0:attach"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "sh",
                            "-c",
                            "block=$(basename \"$(readlink -f \"$1\")\"); printf '1\\n' > \"/sys/class/block/${block}/device/rescan\"",
                            "disk-nix-scsi-rescan",
                            "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0",
                        ]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
    assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luns:iqn.2026-06.example:storage/root:0:attach"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "blockdev",
                            "--getsize64",
                            "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0",
                        ]
                        && !command.mutates
                })
        }));
    assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luns:iqn.2026-06.example:storage/old:1:detach"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "sh",
                            "-c",
                            "block=$(basename \"$(readlink -f \"$1\")\"); printf '1\\n' > \"/sys/class/block/${block}/device/delete\"",
                            "disk-nix-scsi-delete",
                            "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-1",
                        ]
                        && command.mutates
                        && command.readiness == CommandReadiness::Ready
                })
        }));
    assert!(report.verification_plan.iter().any(|step| {
            step.action_id == "luns:iqn.2026-06.example:storage/old:1:detach"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "test",
                            "!",
                            "-e",
                            "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-1",
                        ]
                        && !command.mutates
                })
        }));
}

#[test]
fn lun_lifecycle_accepts_stable_path_aliases() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "luns": {
                  "iqn.2026-06.example:storage/path:0": {
                    "operation": "attach",
                    "path": "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0"
                  },
                  "iqn.2026-06.example:storage/paths:1": {
                    "operation": "rescan",
                    "paths": [
                      "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-1"
                    ]
                  },
                  "iqn.2026-06.example:storage/device-paths:2": {
                    "operation": "detach",
                    "devicePaths": [
                      "/dev/disk/by-path/ip-192.0.2.12:3260-iscsi-iqn.2026-06.example:storage-lun-2"
                    ]
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
            step.action_id == "luns:iqn.2026-06.example:storage/path:0:attach"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["lsscsi", "-t", "-s"] && !command.mutates)
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "blockdev",
                            "--getsize64",
                            "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
    assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luns:iqn.2026-06.example:storage/paths:1:rescan"
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "sh",
                            "-c",
                            "block=$(basename \"$(readlink -f \"$1\")\"); printf '1\\n' > \"/sys/class/block/${block}/device/rescan\"",
                            "disk-nix-scsi-rescan",
                            "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-1",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
    assert!(report.command_plan.iter().any(|step| {
            step.action_id == "luns:iqn.2026-06.example:storage/device-paths:2:detach"
                && step
                    .commands
                    .iter()
                    .any(|command| command.argv == ["lsscsi", "-t", "-s"] && !command.mutates)
                && step.commands.iter().any(|command| {
                    command.argv
                        == [
                            "sh",
                            "-c",
                            "block=$(basename \"$(readlink -f \"$1\")\"); printf '1\\n' > \"/sys/class/block/${block}/device/delete\"",
                            "disk-nix-scsi-delete",
                            "/dev/disk/by-path/ip-192.0.2.12:3260-iscsi-iqn.2026-06.example:storage-lun-2",
                        ]
                        && command.readiness == CommandReadiness::Ready
                })
        }));
}

#[test]
fn target_lun_lifecycle_renders_provider_handoff_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "array-a/root": {
                    "operation": "create",
                    "desiredSize": "2TiB",
                    "source": "pool-a/volumes/root",
                    "provider": "netapp-ontap",
                    "vendor": "netapp",
                    "arrayId": "ontap-cluster-a",
                    "storagePool": "aggr1",
                    "volumeId": "vol-root",
                    "snapshotId": "snap-before",
                    "cloneSource": "vol-root@snap-before",
                    "maskingGroup": "linux-hosts",
                    "portal": "192.0.2.10:3260",
                    "client": "iqn.2026-06.example:host.primary",
                    "initiators": [
                      "iqn.2026-06.example:host.secondary"
                    ]
                  },
                  "array-a/root-grow": {
                    "operation": "grow",
                    "target": "array-a/root",
                    "desiredSize": "3TiB"
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
    assert_eq!(report.command_summary.needs_domain_implementation_count, 6);
    assert!(!report.command_summary.all_commands_ready());

    let create = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "targetluns:array-a/root:create")
        .expect("target-side LUN create command plan exists");
    assert!(create.requires_manual_review);
    assert!(create.commands.iter().any(|command| {
        command.argv
            == [
                "<target-lun-provider:netapp-ontap>",
                "create-lun",
                "--target",
                "array-a/root",
                "--provider",
                "netapp-ontap",
                "--vendor",
                "netapp",
                "--array-id",
                "ontap-cluster-a",
                "--storage-pool",
                "aggr1",
                "--volume-id",
                "vol-root",
                "--snapshot-id",
                "snap-before",
                "--clone-source",
                "vol-root@snap-before",
                "--masking-group",
                "linux-hosts",
                "--size",
                "2TiB",
                "--backing",
                "pool-a/volumes/root",
                "--portal",
                "192.0.2.10:3260",
                "--initiator",
                "iqn.2026-06.example:host.primary",
                "--initiator",
                "iqn.2026-06.example:host.secondary",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command
                .unresolved_inputs
                .contains(&"netapp-ontap target LUN provider implementation".to_string())
            && command
                .provider_capabilities
                .contains(&"target-lun.create".to_string())
            && command
                .provider_capabilities
                .contains(&"target-lun.persistence".to_string())
            && command
                .provider_capabilities
                .contains(&"target-lun.refusal".to_string())
            && command
                .provider_capabilities
                .contains(&"target-lun.initiator-scope.declared".to_string())
            && command
                .provider_capabilities
                .contains(&"target-lun.array-id.declared".to_string())
            && command
                .provider_capabilities
                .contains(&"target-lun.volume-id.declared".to_string())
            && command
                .provider_capabilities
                .contains(&"target-lun.snapshot-id.declared".to_string())
            && command
                .provider_capabilities
                .contains(&"target-lun.clone-source.declared".to_string())
            && command
                .provider_capabilities
                .contains(&"target-lun.masking-group.declared".to_string())
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "targetluns:array-a/root:create"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "<target-lun-provider:netapp-ontap>",
                        "show-mapping",
                        "--portal",
                        "192.0.2.10:3260",
                        "--target",
                        "array-a/root",
                    ]
                    && !command.mutates
                    && command
                        .provider_capabilities
                        .contains(&"target-lun.verification".to_string())
                    && command
                        .provider_capabilities
                        .contains(&"target-lun.portal.declared".to_string())
            })
            && step.commands.iter().any(|command| {
                command.argv == ["lsscsi", "-t", "-s"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["multipath", "-ll"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "array-a/root", "--json"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));

    let grow = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "targetluns:array-a/root-grow:grow")
        .expect("target-side LUN grow command plan exists");
    assert!(grow.commands.iter().any(|command| {
        command.argv
            == [
                "<target-lun-provider>",
                "grow-lun",
                "--target",
                "array-a/root",
                "--size",
                "3TiB",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command
                .provider_capabilities
                .contains(&"target-lun.grow".to_string())
            && command
                .provider_capabilities
                .contains(&"target-lun.capacity.expand".to_string())
            && command
                .provider_capabilities
                .contains(&"target-lun.consumer-refresh.handoff".to_string())
    }));

    let script = report
        .to_shell_script()
        .expect("not-ready plans still render a review script");
    assert!(script.contains("# Provider capabilities: target-lun.identity, target-lun.inventory"));
    assert!(script.contains("target-lun.capacity.expand"));
    assert!(script.contains("target-lun.refusal"));
}

#[test]
fn target_lun_lio_provider_renders_concrete_targetcli_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:storage.root": {
                    "operation": "create",
                    "provider": "lio",
                    "source": "/dev/zvol/tank/root",
                    "lun": 7,
                    "portal": "192.0.2.10:3260",
                    "client": "iqn.2026-06.example:host.primary",
                    "initiators": [
                      "iqn.2026-06.example:host.secondary"
                    ]
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
    assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
    assert!(report.command_summary.all_commands_ready());

    let step = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "targetluns:iqn.2026-06.example:storage.root:create")
        .expect("LIO target-side LUN create command plan exists");
    assert!(step.requires_manual_review);
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "targetcli",
                "/backstores/block",
                "create",
                "name=_dev_zvol_tank_root",
                "dev=/dev/zvol/tank/root",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "targetcli",
                "/iscsi",
                "create",
                "iqn.2026-06.example:storage.root",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "targetcli",
                "/iscsi/iqn.2026-06.example:storage.root/tpg1/luns",
                "create",
                "/backstores/block/_dev_zvol_tank_root",
                "lun=7",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "targetcli",
                "/iscsi/iqn.2026-06.example:storage.root/tpg1/acls",
                "create",
                "iqn.2026-06.example:host.primary",
            ]
            && command.mutates
    }));
    assert!(step
        .commands
        .iter()
        .any(|command| { command.argv == ["targetcli", "saveconfig"] && command.mutates }));

    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "targetluns:iqn.2026-06.example:storage.root:create"
            && step.commands.iter().any(|command| {
                command.argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));

    let targetcli = report
        .tool_requirements
        .iter()
        .find(|requirement| requirement.tool == "targetcli")
        .expect("targetcli tool requirement exists");
    assert!(targetcli
        .remediation
        .iter()
        .any(|hint| hint.contains("pkgs.targetcli-fb")));
}

#[test]
fn target_lun_lio_grow_and_property_use_native_inventory_and_capacity_validation() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:storage.root": {
                    "operation": "grow",
                    "provider": "lio",
                    "source": "/dev/zvol/tank/root",
                    "desiredSize": "4TiB",
                    "lun": 7,
                    "properties": {
                      "lio.writeCache": "off"
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
    assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
    assert!(report.command_summary.all_commands_ready());

    let grow = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "targetluns:iqn.2026-06.example:storage.root:grow")
        .expect("LIO target-side LUN grow command plan exists");
    assert!(grow.commands.iter().any(|command| {
        command.argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]
            && !command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(grow.commands.iter().any(|command| {
        command.argv == ["targetcli", "/backstores/block/_dev_zvol_tank_root", "ls"]
            && !command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(grow.commands.iter().any(|command| {
        command.argv == ["blockdev", "--getsize64", "/dev/zvol/tank/root"]
            && !command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(grow.commands.iter().any(|command| {
        command.argv
            == [
                "targetcli",
                "/iscsi/iqn.2026-06.example:storage.root/tpg1/luns",
                "ls",
            ]
            && !command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(grow.commands.iter().any(|command| {
        command.argv == ["targetcli", "saveconfig"]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    let grow_verification = report
        .verification_plan
        .iter()
        .find(|step| step.action_id == "targetluns:iqn.2026-06.example:storage.root:grow")
        .expect("LIO target-side LUN grow verification plan exists");
    assert!(grow_verification.commands.iter().any(|command| {
        command.argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]
            && !command.mutates
    }));
    assert!(grow_verification
        .commands
        .iter()
        .any(|command| { command.argv == ["lsscsi", "-t", "-s"] && !command.mutates }));
    assert!(grow_verification
        .commands
        .iter()
        .any(|command| { command.argv == ["multipath", "-ll"] && !command.mutates }));
    assert!(grow_verification.commands.iter().any(|command| {
        command.argv
            == [
                "disk-nix",
                "inspect",
                "iqn.2026-06.example:storage.root",
                "--json",
            ]
            && !command.mutates
    }));

    let property = report
        .command_plan
        .iter()
        .find(|step| {
            step.action_id
                == "targetLuns:iqn.2026-06.example:storage.root:set-property:lio.writeCache"
        })
        .expect("LIO target-side LUN property command plan exists");
    assert!(property.commands.iter().any(|command| {
        command.argv == ["targetcli", "/backstores/block/_dev_zvol_tank_root", "ls"]
            && !command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(property.commands.iter().any(|command| {
        command.argv
            == [
                "targetcli",
                "/backstores/block/_dev_zvol_tank_root",
                "set",
                "attribute",
                "emulate_write_cache=0",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(property.commands.iter().any(|command| {
        command.argv == ["targetcli", "saveconfig"]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
}

#[test]
fn target_lun_lio_fileio_grow_forces_backstore_resize_before_refresh() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:storage.file": {
                    "operation": "grow",
                    "provider": "lio",
                    "backstoreType": "fileio",
                    "source": "/var/lib/iscsi/root.img",
                    "desiredSize": "4TiB",
                    "lun": 3
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
    assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
    assert!(report.command_summary.all_commands_ready());

    let grow = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "targetluns:iqn.2026-06.example:storage.file:grow")
        .expect("LIO fileio target-side LUN grow command plan exists");
    assert!(grow.commands.iter().any(|command| {
        command.argv
            == [
                "targetcli",
                "/backstores/fileio/_var_lib_iscsi_root.img",
                "ls",
            ]
            && !command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(grow.commands.iter().any(|command| {
        command.argv == ["truncate", "--size", "4TiB", "/var/lib/iscsi/root.img"]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
            && command.note.contains("fileio backstore")
    }));
    assert!(grow.commands.iter().any(|command| {
        command.argv == ["stat", "--format=%s", "/var/lib/iscsi/root.img"]
            && !command.mutates
            && command.readiness == CommandReadiness::Ready
    }));

    let truncate_requirement = report
        .tool_requirements
        .iter()
        .find(|requirement| requirement.tool == "truncate")
        .expect("truncate tool requirement exists");
    assert!(truncate_requirement
        .remediation
        .iter()
        .any(|hint| hint.contains("pkgs.coreutils")));
}

#[test]
fn failed_target_lun_lio_create_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:storage.root": {
                    "operation": "create",
                    "provider": "lio",
                    "source": "/dev/zvol/tank/root",
                    "lun": 7,
                    "portal": "192.0.2.10:3260",
                    "client": "iqn.2026-06.example:host.primary"
                  }
                }
              },
              "apply": {
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let failed_lun_create = [
        "targetcli",
        "/iscsi/iqn.2026-06.example:storage.root/tpg1/luns",
        "create",
        "/backstores/block/_dev_zvol_tank_root",
        "lun=7",
    ];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_lun_create,
            status_code: Some(if argv == failed_lun_create { 85 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_lun_create {
                "target LUN mapping failed".to_string()
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
                    "targetcli",
                    "/iscsi/iqn.2026-06.example:storage.root/tpg1/luns",
                    "create",
                    "/backstores/block/_dev_zvol_tank_root",
                    "lun=7",
                ]
    }));
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("target-side LUN domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("Create"));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["targetcli", "/iscsi", "ls"] && !command.mutates }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]
            && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show",
            ]
            && !command.mutates
    }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["lsscsi", "-t", "-s"] && !command.mutates }));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["multipath", "-ll"] && !command.mutates }));
    assert!(domain_recovery.notes.iter().any(|note| {
        note.contains("target-side LUN changes") && note.contains("provider inventory")
    }));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("target-side LUN roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]
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
        .expect("target-side LUN rollback recovery review is reported");
    assert!(rollback.commands.iter().all(|command| !command.mutates));
    assert!(rollback.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show",
            ]
            && !command.mutates
    }));
    assert!(report
        .recovery_actions
        .iter()
        .any(|action| action.kind == RecoveryActionKind::PreserveRecoveryPoints));
}

#[test]
fn failed_target_lun_lio_property_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:storage.root": {
                    "provider": "lio",
                    "source": "/dev/zvol/tank/root",
                    "lun": 7,
                    "properties": {
                      "lio.writeCache": "off"
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

    let failed_property = [
        "targetcli",
        "/backstores/block/_dev_zvol_tank_root",
        "set",
        "attribute",
        "emulate_write_cache=0",
    ];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_property,
            status_code: Some(if argv == failed_property { 88 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_property {
                "target LUN property failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert_eq!(
        report
            .partial_execution_recovery
            .as_ref()
            .expect("partial execution recovery is reported")
            .failed_action_id,
        "targetLuns:iqn.2026-06.example:storage.root:set-property:lio.writeCache"
    );
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("target-side LUN property domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("SetProperty"));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["targetcli", "/iscsi", "ls"] && !command.mutates }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]
            && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show",
            ]
            && !command.mutates
    }));
    assert!(domain_recovery.notes.iter().any(|note| {
        note.contains("target-side LUN changes") && note.contains("provider inventory")
    }));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("target-side LUN property roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]
            && !command.mutates
    }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("target-side LUN property rollback recovery review is reported");
    assert!(rollback.commands.iter().all(|command| !command.mutates));
}

#[test]
fn target_lun_lio_destroy_renders_concrete_targetcli_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:storage.root": {
                    "destroy": true,
                    "provider": "lio",
                    "source": "/dev/zvol/tank/root",
                    "lun": 7,
                    "client": "iqn.2026-06.example:host.primary",
                    "initiators": [
                      "iqn.2026-06.example:host.secondary"
                    ]
                  }
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowOffline": true,
                "backupVerified": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
    assert!(report.command_summary.all_commands_ready());

    let step = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "targetluns:iqn.2026-06.example:storage.root:destroy")
        .expect("LIO target-side LUN destroy command plan exists");
    assert!(step.requires_manual_review);
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "targetcli",
                "/iscsi/iqn.2026-06.example:storage.root/tpg1/acls",
                "delete",
                "iqn.2026-06.example:host.primary",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "targetcli",
                "/iscsi/iqn.2026-06.example:storage.root/tpg1/luns",
                "delete",
                "7",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "targetcli",
                "/iscsi",
                "delete",
                "iqn.2026-06.example:storage.root",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "targetcli",
                "/backstores/block",
                "delete",
                "_dev_zvol_tank_root",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(step
        .commands
        .iter()
        .any(|command| command.argv == ["targetcli", "saveconfig"] && command.mutates));
}

#[test]
fn target_lun_lio_destroy_requires_backstore_identity_for_backstore_removal() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:storage.root": {
                    "destroy": true,
                    "provider": "lio",
                    "lun": 7
                  }
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowOffline": true,
                "backupVerified": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert_eq!(report.command_summary.needs_domain_implementation_count, 2);
    assert!(!report.command_summary.all_commands_ready());

    let step = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "targetluns:iqn.2026-06.example:storage.root:destroy")
        .expect("LIO target-side LUN destroy command plan exists");
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "targetcli",
                "/backstores/block",
                "delete",
                "<backstore-name>",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command
                .unresolved_inputs
                .contains(&"LIO backstore name or backing device for removal".to_string())
    }));
}

#[test]
fn target_lun_tgt_provider_renders_concrete_tgtadm_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:tgt.root": {
                    "operation": "create",
                    "provider": "tgt",
                    "targetId": 42,
                    "source": "/dev/zvol/tank/root",
                    "lun": 8,
                    "client": "ALL"
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
    assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
    assert!(report.command_summary.all_commands_ready());

    let step = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "targetluns:iqn.2026-06.example:tgt.root:create")
        .expect("Linux tgt target-side LUN create command plan exists");
    assert!(step.requires_manual_review);
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm",
                "--lld",
                "iscsi",
                "--mode",
                "target",
                "--op",
                "new",
                "--tid",
                "42",
                "--targetname",
                "iqn.2026-06.example:tgt.root",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm",
                "--lld",
                "iscsi",
                "--mode",
                "logicalunit",
                "--op",
                "new",
                "--tid",
                "42",
                "--lun",
                "8",
                "--backing-store",
                "/dev/zvol/tank/root",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm",
                "--lld",
                "iscsi",
                "--mode",
                "target",
                "--op",
                "bind",
                "--tid",
                "42",
                "--initiator-address",
                "ALL",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "targetluns:iqn.2026-06.example:tgt.root:create"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show", "--tid",
                        "42",
                    ]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));

    let tgtadm = report
        .tool_requirements
        .iter()
        .find(|requirement| requirement.tool == "tgtadm")
        .expect("tgtadm tool requirement exists");
    assert!(tgtadm
        .remediation
        .iter()
        .any(|hint| hint.contains("pkgs.tgt")));
}

#[test]
fn target_lun_tgt_grow_and_property_use_native_refresh_and_capacity_validation() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:tgt.root": {
                    "operation": "grow",
                    "provider": "tgt",
                    "targetId": 42,
                    "source": "/dev/zvol/tank/root",
                    "desiredSize": "4TiB",
                    "lun": 8,
                    "properties": {
                      "tgt.writeCache": "off"
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
    assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
    assert!(report.command_summary.all_commands_ready());

    let grow = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "targetluns:iqn.2026-06.example:tgt.root:grow")
        .expect("Linux tgt target-side LUN grow command plan exists");
    assert!(grow.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show", "--tid", "42",
            ]
            && !command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(grow.commands.iter().any(|command| {
        command.argv == ["blockdev", "--getsize64", "/dev/zvol/tank/root"]
            && !command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(grow.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm",
                "--lld",
                "iscsi",
                "--mode",
                "logicalunit",
                "--op",
                "update",
                "--tid",
                "42",
                "--lun",
                "8",
                "--params",
                "online=1",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(grow.commands.iter().any(|command| {
        command.argv == ["tgt-admin", "--dump"]
            && !command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    let grow_verification = report
        .verification_plan
        .iter()
        .find(|step| step.action_id == "targetluns:iqn.2026-06.example:tgt.root:grow")
        .expect("Linux tgt target-side LUN grow verification plan exists");
    assert!(grow_verification.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show", "--tid", "42",
            ]
            && !command.mutates
    }));
    assert!(grow_verification
        .commands
        .iter()
        .any(|command| { command.argv == ["lsscsi", "-t", "-s"] && !command.mutates }));
    assert!(grow_verification
        .commands
        .iter()
        .any(|command| { command.argv == ["multipath", "-ll"] && !command.mutates }));
    assert!(grow_verification.commands.iter().any(|command| {
        command.argv
            == [
                "disk-nix",
                "inspect",
                "iqn.2026-06.example:tgt.root",
                "--json",
            ]
            && !command.mutates
    }));

    let property = report
        .command_plan
        .iter()
        .find(|step| {
            step.action_id == "targetLuns:iqn.2026-06.example:tgt.root:set-property:tgt.writeCache"
        })
        .expect("Linux tgt target-side LUN property command plan exists");
    assert!(property.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show", "--tid", "42",
            ]
            && !command.mutates
    }));
    assert!(property.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm",
                "--lld",
                "iscsi",
                "--mode",
                "logicalunit",
                "--op",
                "update",
                "--tid",
                "42",
                "--lun",
                "8",
                "--name",
                "tgt.writeCache",
                "--value",
                "off",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
}

#[test]
fn failed_target_lun_tgt_property_reports_domain_recovery_guidance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:tgt.root": {
                    "provider": "tgt",
                    "targetId": 42,
                    "source": "/dev/zvol/tank/root",
                    "lun": 8,
                    "properties": {
                      "tgt.writeCache": "off"
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

    let failed_property = [
        "tgtadm",
        "--lld",
        "iscsi",
        "--mode",
        "logicalunit",
        "--op",
        "update",
        "--tid",
        "42",
        "--lun",
        "8",
        "--name",
        "tgt.writeCache",
        "--value",
        "off",
    ];
    let report = prepare_execution_with_runner(&plan, policy, ExecutionMode::Execute, |argv| {
        CommandRunResult {
            success: argv != failed_property,
            status_code: Some(if argv == failed_property { 89 } else { 0 }),
            stdout: String::new(),
            stderr: if argv == failed_property {
                "tgt property failed".to_string()
            } else {
                String::new()
            },
        }
    });

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert_eq!(
        report
            .partial_execution_recovery
            .as_ref()
            .expect("partial execution recovery is reported")
            .failed_action_id,
        "targetLuns:iqn.2026-06.example:tgt.root:set-property:tgt.writeCache"
    );
    let domain_recovery = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::DomainRecovery)
        .expect("tgt target-side LUN property domain-specific recovery action is reported");
    assert!(domain_recovery.summary.contains("SetProperty"));
    assert!(domain_recovery
        .commands
        .iter()
        .any(|command| { command.argv == ["targetcli", "/iscsi", "ls"] && !command.mutates }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv == ["targetcli", "/iscsi/iqn.2026-06.example:tgt.root", "ls"]
            && !command.mutates
    }));
    assert!(domain_recovery.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show",
            ]
            && !command.mutates
    }));
    assert!(domain_recovery.notes.iter().any(|note| {
        note.contains("target-side LUN changes") && note.contains("provider inventory")
    }));
    let roll_forward = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollForwardReview)
        .expect("tgt target-side LUN property roll-forward recovery review is reported");
    assert!(roll_forward.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show", "--tid", "42",
            ]
            && !command.mutates
    }));
    let rollback = report
        .recovery_actions
        .iter()
        .find(|action| action.kind == RecoveryActionKind::RollbackReview)
        .expect("tgt target-side LUN property rollback recovery review is reported");
    assert!(rollback.commands.iter().all(|command| !command.mutates));
}

#[test]
fn target_lun_tgt_provider_requires_reviewed_target_id_and_lun_inputs() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:tgt.root": {
                    "operation": "create",
                    "provider": "tgt"
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

    let step = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "targetluns:iqn.2026-06.example:tgt.root:create")
        .expect("Linux tgt target-side LUN create command plan exists");
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm",
                "--lld",
                "iscsi",
                "--mode",
                "target",
                "--op",
                "new",
                "--tid",
                "<tid>",
                "--targetname",
                "iqn.2026-06.example:tgt.root",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command
                .unresolved_inputs
                .contains(&"Linux tgt numeric target id (targetId or tid)".to_string())
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm",
                "--lld",
                "iscsi",
                "--mode",
                "logicalunit",
                "--op",
                "new",
                "--tid",
                "<tid>",
                "--lun",
                "<lun>",
                "--backing-store",
                "<backing-block-device-or-file>",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command
                .unresolved_inputs
                .contains(&"Linux tgt LUN number".to_string())
            && command
                .unresolved_inputs
                .contains(&"Linux tgt backing store path".to_string())
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "tgtadm",
                "--lld",
                "iscsi",
                "--mode",
                "target",
                "--op",
                "bind",
                "--tid",
                "<tid>",
                "--initiator-address",
                "<initiator-address-or-ALL>",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command
                .unresolved_inputs
                .contains(&"Linux tgt initiator address or ALL ACL value".to_string())
    }));
}

#[test]
fn target_lun_scst_provider_renders_concrete_scstadmin_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:scst.root": {
                    "operation": "create",
                    "provider": "scst",
                    "source": "/dev/zvol/tank/root",
                    "lun": 9,
                    "group": "hosts",
                    "client": "iqn.2026-06.example:host.primary",
                    "initiators": [
                      "iqn.2026-06.example:host.secondary"
                    ]
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
    assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
    assert!(report.command_summary.all_commands_ready());

    let step = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "targetluns:iqn.2026-06.example:scst.root:create")
        .expect("SCST target-side LUN create command plan exists");
    assert!(step.requires_manual_review);
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "scstadmin",
                "-open_dev",
                "_dev_zvol_tank_root",
                "-handler",
                "vdisk_blockio",
                "-attributes",
                "filename=/dev/zvol/tank/root",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "scstadmin",
                "-add_target",
                "iqn.2026-06.example:scst.root",
                "-driver",
                "iscsi",
            ]
            && command.mutates
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "scstadmin",
                "-add_group",
                "hosts",
                "-driver",
                "iscsi",
                "-target",
                "iqn.2026-06.example:scst.root",
            ]
            && command.mutates
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "scstadmin",
                "-add_init",
                "iqn.2026-06.example:host.primary",
                "-driver",
                "iscsi",
                "-target",
                "iqn.2026-06.example:scst.root",
                "-group",
                "hosts",
            ]
            && command.mutates
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "scstadmin",
                "-add_lun",
                "9",
                "-driver",
                "iscsi",
                "-target",
                "iqn.2026-06.example:scst.root",
                "-group",
                "hosts",
                "-device",
                "_dev_zvol_tank_root",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "scstadmin",
                "-enable_target",
                "iqn.2026-06.example:scst.root",
                "-driver",
                "iscsi",
            ]
            && command.mutates
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv == ["scstadmin", "-write_config", "/etc/scst.conf"] && command.mutates
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "targetluns:iqn.2026-06.example:scst.root:create"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "scstadmin",
                        "-list_target",
                        "iqn.2026-06.example:scst.root",
                        "-driver",
                        "iscsi",
                    ]
                    && !command.mutates
            })
    }));

    let scstadmin = report
        .tool_requirements
        .iter()
        .find(|requirement| requirement.tool == "scstadmin")
        .expect("scstadmin tool requirement exists");
    assert!(scstadmin
        .remediation
        .iter()
        .any(|hint| hint.contains("provides scstadmin")));
}

#[test]
fn target_lun_scst_grow_and_property_use_native_scstadmin_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:scst.root": {
                    "operation": "grow",
                    "provider": "scst",
                    "source": "/dev/zvol/tank/root",
                    "desiredSize": "4TiB",
                    "lun": 9,
                    "group": "hosts",
                    "properties": {
                      "read_only": "0"
                    }
                  }
                }
              },
              "apply": {
                "allowOffline": true,
                "allowGrow": true,
                "allowPropertyChanges": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert_eq!(report.command_summary.needs_domain_implementation_count, 0);
    assert!(report.command_summary.all_commands_ready());

    let grow = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "targetluns:iqn.2026-06.example:scst.root:grow")
        .expect("SCST target-side LUN grow command plan exists");
    assert!(grow.commands.iter().any(|command| {
        command.argv == ["scstadmin", "-list_dev_attr", "_dev_zvol_tank_root"]
            && !command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
    assert!(grow.commands.iter().any(|command| {
        command.argv == ["scstadmin", "-resync_dev", "_dev_zvol_tank_root"]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));

    let property = report
        .command_plan
        .iter()
        .find(|step| {
            step.action_id == "targetLuns:iqn.2026-06.example:scst.root:set-property:read_only"
        })
        .expect("SCST target-side LUN property command plan exists");
    assert!(property.commands.iter().any(|command| {
        command.argv
            == [
                "scstadmin",
                "-set_lun_attr",
                "9",
                "-driver",
                "iscsi",
                "-target",
                "iqn.2026-06.example:scst.root",
                "-group",
                "hosts",
                "-attributes",
                "read_only=0",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::Ready
    }));
}

#[test]
fn target_lun_scst_provider_requires_reviewed_lun_and_backing_inputs() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "targetLuns": {
                  "iqn.2026-06.example:scst.root": {
                    "operation": "create",
                    "provider": "scst"
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

    let step = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "targetluns:iqn.2026-06.example:scst.root:create")
        .expect("SCST target-side LUN create command plan exists");
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "scstadmin",
                "-open_dev",
                "iqn.2026-06.example_scst.root",
                "-handler",
                "vdisk_blockio",
                "-attributes",
                "filename=<backing-block-device-or-file>",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command
                .unresolved_inputs
                .contains(&"SCST backing block device or file".to_string())
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "scstadmin",
                "-add_lun",
                "<lun>",
                "-driver",
                "iscsi",
                "-target",
                "iqn.2026-06.example:scst.root",
                "-device",
                "iqn.2026-06.example_scst.root",
            ]
            && command.mutates
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command
                .unresolved_inputs
                .contains(&"SCST LUN number".to_string())
    }));
}

#[test]
fn nvme_namespace_lifecycle_reports_nvme_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "nvmeNamespaces": {
                "/dev/nvme0": {
                  "operation": "create",
                  "desiredSize": "100G",
                  "namespaceId": "4",
                  "controllers": "0x1"
                },
                "/dev/nvme1": {
                  "operation": "grow"
                },
                "/dev/nvme2": {
                  "operation": "attach",
                  "namespaceId": "7",
                  "controllers": "0x2"
                },
                "/dev/nvme3": {
                  "operation": "detach",
                  "namespaceId": "8",
                  "controllers": "0x3"
                },
                "/dev/nvme4": {
                  "destroy": true,
                  "namespaceId": "9",
                  "controllers": "0x4"
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowGrow": true,
                "allowOffline": true
              }
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nvmenamespaces:/dev/nvme0:create"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "nvme",
                        "create-ns",
                        "/dev/nvme0",
                        "--nsze-si",
                        "100G",
                        "--ncap-si",
                        "100G",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "nvme",
                        "attach-ns",
                        "/dev/nvme0",
                        "--namespace-id",
                        "4",
                        "--controllers",
                        "0x1",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nvmenamespaces:/dev/nvme1:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["nvme", "list-subsys", "--output-format=json"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["nvme", "ns-rescan", "/dev/nvme1"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nvmenamespaces:/dev/nvme2:attach"
            && step.commands.iter().any(|command| {
                command.argv == ["nvme", "list-subsys", "--output-format=json"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "nvme",
                        "attach-ns",
                        "/dev/nvme2",
                        "--namespace-id",
                        "7",
                        "--controllers",
                        "0x2",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["nvme", "ns-rescan", "/dev/nvme2"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nvmenamespaces:/dev/nvme3:detach"
            && step.commands.iter().any(|command| {
                command.argv == ["nvme", "list-subsys", "--output-format=json"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "nvme",
                        "detach-ns",
                        "/dev/nvme3",
                        "--namespace-id",
                        "8",
                        "--controllers",
                        "0x3",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["nvme", "ns-rescan", "/dev/nvme3"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nvmenamespaces:/dev/nvme4:destroy"
            && step.commands.iter().any(|command| {
                command.argv == ["nvme", "list-subsys", "--output-format=json"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "nvme",
                        "detach-ns",
                        "/dev/nvme4",
                        "--namespace-id",
                        "9",
                        "--controllers",
                        "0x4",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["nvme", "delete-ns", "/dev/nvme4", "--namespace-id", "9"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "nvmenamespaces:/dev/nvme0:create"
            && step.commands.iter().any(|command| {
                command.argv == ["nvme", "list", "--output-format=json"] && !command.mutates
            })
    }));
}

#[test]
fn nvme_namespace_lifecycle_requires_explicit_namespace_inputs() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "nvmeNamespaces": {
                "logical-ns": {
                  "operation": "create"
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
    let step = report
        .command_plan
        .iter()
        .find(|step| step.action_id == "nvmenamespaces:logical-ns:create")
        .expect("NVMe namespace create command plan exists");
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "nvme",
                "create-ns",
                "<nvme-controller>",
                "--nsze-si",
                "<size>",
                "--ncap-si",
                "<size>",
            ]
            && command.readiness == CommandReadiness::NeedsDomainImplementation
            && command
                .unresolved_inputs
                .contains(&"NVMe controller path such as /dev/nvme0".to_string())
            && command
                .unresolved_inputs
                .contains(&"desired namespace size".to_string())
    }));
    assert!(step.commands.iter().any(|command| {
        command.argv
            == [
                "nvme",
                "attach-ns",
                "<nvme-controller>",
                "--namespace-id",
                "<namespace-id>",
                "--controllers",
                "<controller-id-list>",
            ]
            && command.readiness == CommandReadiness::NeedsDomainImplementation
    }));
}

#[test]
fn nvme_namespace_lifecycle_accepts_controller_path_aliases() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "nvmeNamespaces": {
                  "logical-create": {
                    "operation": "create",
                    "path": "/dev/nvme0",
                    "desiredSize": "100G",
                    "namespaceId": "4",
                    "controllers": "0x1"
                  },
                  "logical-grow": {
                    "operation": "grow",
                    "device": "/dev/nvme1"
                  },
                  "logical-attach": {
                    "operation": "attach",
                    "target": "/dev/nvme2",
                    "device": "/dev/nvme2n1",
                    "namespaceId": "7",
                    "controllers": "0x2"
                  },
                  "logical-detach": {
                    "operation": "detach",
                    "target": "/dev/nvme3",
                    "device": "/dev/nvme3n1",
                    "namespaceId": "8",
                    "controllers": "0x3"
                  },
                  "logical-destroy": {
                    "destroy": true,
                    "target": "/dev/nvme4",
                    "namespaceId": "9",
                    "controllers": "0x4"
                  }
                }
              },
              "apply": {
                "allowDestructive": true,
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
        step.action_id == "nvmenamespaces:logical-create:create"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "nvme",
                        "create-ns",
                        "/dev/nvme0",
                        "--nsze-si",
                        "100G",
                        "--ncap-si",
                        "100G",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nvmenamespaces:logical-grow:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["nvme", "ns-rescan", "/dev/nvme1"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nvmenamespaces:logical-attach:attach"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "nvme",
                        "attach-ns",
                        "/dev/nvme2",
                        "--namespace-id",
                        "7",
                        "--controllers",
                        "0x2",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nvmenamespaces:logical-detach:detach"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "nvme",
                        "detach-ns",
                        "/dev/nvme3",
                        "--namespace-id",
                        "8",
                        "--controllers",
                        "0x3",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nvmenamespaces:logical-destroy:destroy"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "nvme",
                        "detach-ns",
                        "/dev/nvme4",
                        "--namespace-id",
                        "9",
                        "--controllers",
                        "0x4",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
}

#[test]
fn lun_attach_and_grow_without_stable_path_reports_unresolved_input() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "luns": {
                "iqn.2026-06.example:storage/new:0": {
                  "operation": "create"
                },
                "iqn.2026-06.example:storage/grow:1": {
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

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "luns:iqn.2026-06.example:storage/new:0:create"
            && step.commands.iter().any(|command| {
                command.argv == ["<scsi-rescan-device>", "<lun-path>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["stable LUN device path"]
            })
            && step.commands.iter().any(|command| {
                command.argv == ["blockdev", "--getsize64", "<lun-path>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["stable LUN device path"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "luns:iqn.2026-06.example:storage/grow:1:grow"
            && step.commands.iter().any(|command| {
                command.argv == ["<scsi-rescan-device>", "<lun-path>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["stable LUN device path"]
            })
    }));
    assert_eq!(report.command_summary.needs_domain_implementation_count, 3);
    assert!(!report.command_summary.all_commands_ready());
}

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

#[test]
fn lvm_cache_lifecycle_requires_origin_and_cache_pool_for_execute_readiness() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "lvmCaches": {
                  "root-cache": {
                    "operation": "create",
                    "properties": {
                      "lvm.cache-policy": "smq"
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
        step.action_id == "lvmcaches:root-cache:create"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "lvconvert",
                        "--type",
                        "cache",
                        "--cachepool",
                        "<cache-pool>",
                        "<origin-logical-volume>",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs
                        == [
                            "target in volume-group/logical-volume form",
                            "cache-pool logical volume",
                        ]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "lvmCaches:root-cache:set-property:lvm.cache-policy"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "lvchange",
                        "--cachepolicy",
                        "smq",
                        "<origin-logical-volume>",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["target in volume-group/logical-volume form"]
            })
    }));
}

#[test]
fn cache_lifecycle_requires_bcache_device_path_for_execute_readiness() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "caches": {
                  "cache0": {
                    "addDevices": ["cache-set-uuid"],
                    "removeDevices": ["cache-set-uuid"],
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
    assert!(!report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "caches:cache0:add-device:cache-set-uuid"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "sh",
                        "-c",
                        "printf '%s\\n' \"$2\" > \"/sys/block/${1#/dev/}/bcache/attach\"",
                        "disk-nix-bcache-attach",
                        "<cache-device>",
                        "cache-set-uuid",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["bcache device path"]
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
                        "<cache-device>",
                        "writethrough",
                        "cache_mode",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["bcache device path"]
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
                        "<cache-set-uuid>",
                        "100",
                        "journal_delay_ms",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["cache-set UUID"]
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
                        "<cache-device>",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["bcache device path"]
            })
    }));
}

#[test]
fn nfs_export_lifecycle_reports_exportfs_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "exports": {
                  "/srv/share": {
                    "operation": "export",
                    "client": "192.0.2.0/24",
                    "options": "rw,sync,no_subtree_check"
                  },
                  "/srv/changed": {
                    "client": "192.0.2.0/24",
                    "properties": {
                      "options": "ro,sync,no_subtree_check"
                    }
                  },
                  "/srv/inventory": {
                    "operation": "rescan"
                  },
                  "/srv/unresolved": {
                    "properties": {
                      "options": "rw,sync"
                    }
                  },
                  "/srv/old": {
                    "operation": "unexport",
                    "client": "192.0.2.55"
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
                    "exportfs",
                    "-i",
                    "-o",
                    "rw,sync,no_subtree_check",
                    "192.0.2.0/24:/srv/share",
                ]
        })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "exports:/srv/inventory:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["exportfs", "-v"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "/srv/inventory"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "exports:/srv/inventory:rescan"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "/srv/inventory", "--json"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv
                == [
                    "exportfs",
                    "-i",
                    "-o",
                    "ro,sync,no_subtree_check",
                    "192.0.2.0/24:/srv/changed",
                ]
        })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands.iter().any(|command| {
            command.argv
                == [
                    "exportfs",
                    "-i",
                    "-o",
                    "rw,sync",
                    "<client>:/srv/unresolved",
                ]
                && command.readiness == CommandReadiness::NeedsDomainImplementation
                && command.unresolved_inputs == ["NFS client selector"]
        })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["exportfs", "-u", "192.0.2.55:/srv/old"])
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.commands
            .iter()
            .any(|command| command.argv == ["exportfs", "-v"])
    }));
}

#[test]
fn nfs_export_lifecycle_requires_path_for_execute_readiness() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "exports": {
                  "share": {
                    "operation": "create",
                    "client": "192.0.2.0/24",
                    "options": "rw,sync,no_subtree_check"
                  },
                  "inventory": {
                    "operation": "rescan"
                  },
                  "oldshare": {
                    "destroy": true,
                    "client": "192.0.2.55"
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
        step.action_id == "exports:share:create"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "exportfs",
                        "-i",
                        "-o",
                        "rw,sync,no_subtree_check",
                        "192.0.2.0/24:<export-path>",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["NFS export path"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "exports:inventory:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "<export-path>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["NFS export path"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "exports:oldshare:destroy"
            && step.commands.iter().any(|command| {
                command.argv == ["exportfs", "-u", "192.0.2.55:<export-path>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["NFS export path"]
            })
    }));
}

#[test]
fn nfs_mount_lifecycle_reports_mount_commands() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "nfs": {
                  "mounts": {
                    "/srv/shared": {
                      "operation": "mount",
                      "source": "nas.example.com:/srv/shared",
                      "fsType": "nfs4",
                      "options": ["_netdev", "vers=4.2"]
                    },
                    "/srv/tuned": {
                      "operation": "remount",
                      "source": "nas.example.com:/srv/tuned",
                      "options": ["_netdev", "ro", "vers=4.2"]
                    },
                    "/srv/inventory": {
                      "operation": "rescan",
                      "source": "nas.example.com:/srv/inventory"
                    },
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
            }"#,
    )
    .expect("document parses");

    let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert!(report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nfs.mounts:/srv/shared:mount"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "mount",
                        "-t",
                        "nfs4",
                        "-o",
                        "_netdev,vers=4.2",
                        "nas.example.com:/srv/shared",
                        "/srv/shared",
                    ]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nfs.mounts:/srv/tuned:remount"
            && step.commands.iter().any(|command| {
                command.argv == ["mount", "-o", "remount,_netdev,ro,vers=4.2", "/srv/tuned"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nfs.mounts:/srv/inventory:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["findmnt", "--json", "/srv/inventory"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
            && step.commands.iter().any(|command| {
                command.argv == ["nfsstat", "-m", "/srv/inventory"]
                    && !command.mutates
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.verification_plan.iter().any(|step| {
        step.action_id == "nfs.mounts:/srv/inventory:rescan"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["disk-nix", "inspect", "/srv/inventory", "--json"])
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nfs.mounts:/srv/old:unmount"
            && step
                .commands
                .iter()
                .any(|command| command.argv == ["umount", "/srv/old"])
    }));
}

#[test]
fn nfs_mount_lifecycle_requires_mountpoint_for_execute_readiness() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "nfs": {
                  "mounts": {
                    "shared": {
                      "operation": "mount",
                      "source": "nas.example.com:/srv/shared"
                    },
                    "tuned": {
                      "operation": "remount",
                      "source": "nas.example.com:/srv/tuned",
                      "options": ["ro"]
                    },
                    "inventory": {
                      "operation": "rescan",
                      "source": "nas.example.com:/srv/inventory"
                    },
                    "old": {
                      "operation": "unmount",
                      "source": "nas.example.com:/srv/old"
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
        step.action_id == "nfs.mounts:shared:mount"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "mount",
                        "-t",
                        "nfs4",
                        "nas.example.com:/srv/shared",
                        "<mountpoint>",
                    ]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["mountpoint path"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nfs.mounts:tuned:remount"
            && step.commands.iter().any(|command| {
                command.argv == ["mount", "-o", "remount,ro", "<mountpoint>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["mountpoint path"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nfs.mounts:inventory:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["nfsstat", "-m", "<mountpoint>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["mountpoint path"]
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nfs.mounts:old:unmount"
            && step.commands.iter().any(|command| {
                command.argv == ["umount", "<mountpoint>"]
                    && command.readiness == CommandReadiness::NeedsDomainImplementation
                    && command.unresolved_inputs == ["mountpoint path"]
            })
    }));
}

#[test]
fn nfs_lifecycle_accepts_path_aliases_for_logical_names() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "exports": {
                  "share": {
                    "operation": "export",
                    "path": "/srv/share",
                    "client": "192.0.2.0/24",
                    "options": "rw,sync,no_subtree_check"
                  },
                  "inventory": {
                    "operation": "rescan",
                    "target": "/srv/inventory"
                  },
                  "oldshare": {
                    "operation": "unexport",
                    "path": "/srv/old",
                    "client": "192.0.2.55"
                  }
                },
                "nfs": {
                  "mounts": {
                    "shared": {
                      "operation": "mount",
                      "mountpoint": "/srv/shared",
                      "source": "nas.example.com:/srv/shared",
                      "fsType": "nfs4",
                      "options": ["_netdev", "vers=4.2"]
                    },
                    "tuned": {
                      "operation": "remount",
                      "mountpoint": "/srv/tuned",
                      "source": "nas.example.com:/srv/tuned",
                      "options": ["ro"]
                    },
                    "inventory": {
                      "operation": "rescan",
                      "mountpoint": "/srv/inventory",
                      "source": "nas.example.com:/srv/inventory"
                    },
                    "old": {
                      "operation": "unmount",
                      "mountpoint": "/srv/old",
                      "source": "nas.example.com:/srv/old"
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
    assert!(report.command_summary.all_commands_ready());
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "exports:share:export"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "exportfs",
                        "-i",
                        "-o",
                        "rw,sync,no_subtree_check",
                        "192.0.2.0/24:/srv/share",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "exports:inventory:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["disk-nix", "inspect", "/srv/inventory"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "exports:oldshare:unexport"
            && step.commands.iter().any(|command| {
                command.argv == ["exportfs", "-u", "192.0.2.55:/srv/old"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nfs.mounts:shared:mount"
            && step.commands.iter().any(|command| {
                command.argv
                    == [
                        "mount",
                        "-t",
                        "nfs4",
                        "-o",
                        "_netdev,vers=4.2",
                        "nas.example.com:/srv/shared",
                        "/srv/shared",
                    ]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nfs.mounts:tuned:remount"
            && step.commands.iter().any(|command| {
                command.argv == ["mount", "-o", "remount,ro", "/srv/tuned"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nfs.mounts:inventory:rescan"
            && step.commands.iter().any(|command| {
                command.argv == ["nfsstat", "-m", "/srv/inventory"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
    assert!(report.command_plan.iter().any(|step| {
        step.action_id == "nfs.mounts:old:unmount"
            && step.commands.iter().any(|command| {
                command.argv == ["umount", "/srv/old"]
                    && command.readiness == CommandReadiness::Ready
            })
    }));
}
