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
