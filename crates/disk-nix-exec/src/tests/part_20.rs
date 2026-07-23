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
