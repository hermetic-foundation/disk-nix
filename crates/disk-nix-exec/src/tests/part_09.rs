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
