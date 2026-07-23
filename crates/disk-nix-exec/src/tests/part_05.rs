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
