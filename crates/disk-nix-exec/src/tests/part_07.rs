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
