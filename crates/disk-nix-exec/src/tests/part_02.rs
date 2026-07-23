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
