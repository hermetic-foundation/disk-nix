#[test]
fn plan_classifies_iscsi_session_login_and_logout() {
    let plan = plan_from_json_bytes(
        br#"{
              "iscsiSessions": {
                "iqn.2026-06.example:storage.root": {
                  "operation": "login",
                  "metadata": {
                    "portal": "192.0.2.10:3260"
                  }
                },
                "iqn.2026-06.example:storage.old": {
                  "operation": "logout",
                  "portal": "192.0.2.11:3260"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 2);
    assert_eq!(plan.summary.offline_required_count, 1);
    assert_eq!(plan.summary.destructive_count, 0);
    let create = plan
        .actions
        .iter()
        .find(|action| action.id == "iscsisessions:iqn.2026-06.example:storage.root:login")
        .expect("iSCSI login action exists");
    assert_eq!(create.operation, Operation::Login);
    assert_eq!(create.risk, RiskClass::Online);
    assert_eq!(create.context.portal.as_deref(), Some("192.0.2.10:3260"));

    let destroy = plan
        .actions
        .iter()
        .find(|action| action.id == "iscsisessions:iqn.2026-06.example:storage.old:logout")
        .expect("iSCSI logout action exists");
    assert_eq!(destroy.operation, Operation::Logout);
    assert_eq!(destroy.risk, RiskClass::OfflineRequired);
    assert_eq!(destroy.context.portal.as_deref(), Some("192.0.2.11:3260"));
}

#[test]
fn plan_classifies_nfs_export_lifecycle_without_data_destruction() {
    let plan = plan_from_json_bytes(
        br#"{
              "exports": {
                "/srv/share": {
                  "operation": "export",
                  "client": "192.0.2.0/24",
                  "options": "rw,sync,no_subtree_check"
                },
                "/srv/inventory": {
                  "operation": "rescan"
                },
                "/srv/old": {
                  "operation": "unexport",
                  "client": "192.0.2.55"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 3);
    assert_eq!(plan.summary.offline_required_count, 1);
    assert_eq!(plan.summary.destructive_count, 0);
    let create = plan
        .actions
        .iter()
        .find(|action| action.id == "exports:/srv/share:export")
        .expect("export action exists");
    assert_eq!(create.operation, Operation::Export);
    assert_eq!(create.risk, RiskClass::Online);
    assert_eq!(create.context.client.as_deref(), Some("192.0.2.0/24"));
    assert_eq!(
        create.context.options.as_deref(),
        Some("rw,sync,no_subtree_check")
    );
    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "exports:/srv/inventory:rescan")
        .expect("export rescan action exists");
    assert_eq!(rescan.operation, Operation::Rescan);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert_eq!(rescan.context.target.as_deref(), Some("/srv/inventory"));
    let destroy = plan
        .actions
        .iter()
        .find(|action| action.id == "exports:/srv/old:unexport")
        .expect("unexport action exists");
    assert_eq!(destroy.operation, Operation::Unexport);
    assert_eq!(destroy.risk, RiskClass::OfflineRequired);
    assert!(!destroy.destructive);
}

#[test]
fn plan_classifies_nfs_mount_lifecycle_without_remote_data_destruction() {
    let plan = plan_from_json_bytes(
        br#"{
              "nfs": {
                "mounts": {
                  "/srv/shared": {
                    "operation": "mount",
                    "source": "nas.example.com:/srv/shared",
                    "fsType": "nfs4",
                    "options": ["_netdev", "vers=4.2"]
                  },
                  "/srv/old": {
                    "operation": "unmount",
                    "source": "nas.example.com:/srv/old"
                  },
                  "/srv/tuned": {
                    "operation": "remount",
                    "source": "nas.example.com:/srv/tuned",
                    "options": ["_netdev", "ro", "vers=4.2"]
                  },
                  "/srv/inventory": {
                    "operation": "rescan",
                    "source": "nas.example.com:/srv/inventory"
                  }
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 4);
    assert_eq!(plan.summary.offline_required_count, 1);
    assert_eq!(plan.summary.destructive_count, 0);
    let create = plan
        .actions
        .iter()
        .find(|action| action.id == "nfs.mounts:/srv/shared:mount")
        .expect("NFS mount action exists");
    assert_eq!(create.operation, Operation::Mount);
    assert_eq!(create.risk, RiskClass::Online);
    assert_eq!(
        create.context.device.as_deref(),
        Some("nas.example.com:/srv/shared")
    );
    assert_eq!(create.context.mountpoint.as_deref(), Some("/srv/shared"));
    assert_eq!(create.context.fs_type.as_deref(), Some("nfs4"));
    assert_eq!(create.context.options.as_deref(), Some("_netdev,vers=4.2"));

    let destroy = plan
        .actions
        .iter()
        .find(|action| action.id == "nfs.mounts:/srv/old:unmount")
        .expect("NFS unmount action exists");
    assert_eq!(destroy.operation, Operation::Unmount);
    assert_eq!(destroy.risk, RiskClass::OfflineRequired);
    assert!(!destroy.destructive);
    assert_eq!(destroy.context.mountpoint.as_deref(), Some("/srv/old"));

    let remount = plan
        .actions
        .iter()
        .find(|action| action.id == "nfs.mounts:/srv/tuned:remount")
        .expect("NFS mount remount exists");
    assert_eq!(remount.operation, Operation::Remount);
    assert_eq!(remount.risk, RiskClass::Online);
    assert_eq!(remount.context.mountpoint.as_deref(), Some("/srv/tuned"));
    assert_eq!(
        remount.context.options.as_deref(),
        Some("_netdev,ro,vers=4.2")
    );

    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "nfs.mounts:/srv/inventory:rescan")
        .expect("NFS mount rescan exists");
    assert_eq!(rescan.operation, Operation::Rescan);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert_eq!(rescan.context.mountpoint.as_deref(), Some("/srv/inventory"));
}

#[test]
fn plan_classifies_cache_replacement_as_offline_required() {
    let plan = plan_from_json_bytes(
        br#"{
              "caches": {
                "vg0/root-cache": {
                  "operation": "replace-device",
                  "removeDevices": ["/dev/sdd"],
                  "replaceDevices": {
                    "/dev/sdb": "/dev/sdc"
                  }
                },
                "/dev/bcache0": {
                  "operation": "rescan"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    assert_eq!(plan.summary.action_count, 4);
    assert_eq!(plan.summary.offline_required_count, 3);
    assert!(plan
        .actions
        .iter()
        .filter(|action| action.operation == Operation::ReplaceDevice)
        .all(|action| {
            action.operation == Operation::ReplaceDevice
                && action.risk == RiskClass::OfflineRequired
                && action.advice.as_ref().is_some_and(|advice| {
                    advice
                        .alternatives
                        .iter()
                        .any(|alternative| alternative.contains("flush dirty data"))
                })
        }));
    let detach = plan
        .actions
        .iter()
        .find(|action| action.id == "caches:vg0/root-cache:remove-device:/dev/sdd")
        .expect("cache detach action exists");
    assert_eq!(detach.operation, Operation::RemoveDevice);
    assert_eq!(detach.risk, RiskClass::OfflineRequired);
    assert!(detach.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("dirty data"))
    }));
    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "caches:/dev/bcache0:rescan")
        .expect("cache rescan action exists");
    assert_eq!(rescan.operation, Operation::Rescan);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert!(!rescan.destructive);
    assert!(rescan.advice.as_ref().is_some_and(|advice| {
        advice
            .summary
            .contains("bcache rescan refreshes cache state")
    }));
}

#[test]
fn plan_classifies_lvm_cache_attach_and_detach() {
    let plan = plan_from_json_bytes(
        br#"{
              "lvmCaches": {
                "vg0/root": {
                  "operation": "create",
                  "device": "vg0/root-cache",
                  "addDevices": ["vg0/root-cache"],
                  "removeDevices": ["vg0/root-cache"],
                  "properties": {
                    "lvm.cache-mode": "writethrough"
                  }
                },
                "vg0/archive": {
                  "operation": "rescan"
                }
              }
            }"#,
    )
    .expect("plan should parse");

    let create = plan
        .actions
        .iter()
        .find(|action| action.id == "lvmcaches:vg0/root:create")
        .expect("LVM cache create action exists");
    let add = plan
        .actions
        .iter()
        .find(|action| action.id == "lvmCaches:vg0/root:add-device:vg0/root-cache")
        .expect("LVM cache add action exists");
    let remove = plan
        .actions
        .iter()
        .find(|action| action.id == "lvmCaches:vg0/root:remove-device:vg0/root-cache")
        .expect("LVM cache remove action exists");
    let property = plan
        .actions
        .iter()
        .find(|action| action.id == "lvmCaches:vg0/root:set-property:lvm.cache-mode")
        .expect("LVM cache property action exists");
    let rescan = plan
        .actions
        .iter()
        .find(|action| action.id == "lvmcaches:vg0/archive:rescan")
        .expect("LVM cache rescan action exists");

    assert_eq!(plan.summary.action_count, 5);
    assert_eq!(plan.summary.offline_required_count, 3);
    assert_eq!(create.risk, RiskClass::OfflineRequired);
    assert_eq!(add.risk, RiskClass::OfflineRequired);
    assert_eq!(remove.risk, RiskClass::OfflineRequired);
    assert_eq!(property.risk, RiskClass::Safe);
    assert_eq!(rescan.operation, Operation::Rescan);
    assert_eq!(rescan.risk, RiskClass::Online);
    assert!(!rescan.destructive);
    assert!(remove.advice.as_ref().is_some_and(|advice| {
        advice
            .alternatives
            .iter()
            .any(|alternative| alternative.contains("dirty data"))
    }));
}

#[test]
fn apply_policy_blocks_destructive_and_potential_data_loss_actions() {
    let (plan, mut policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "datasets": {
                  "tank/old": { "destroy": true }
                },
                "pools": {
                  "tank": { "removeDevices": ["/dev/sdb"] }
                }
              },
              "apply": {
                "mode": "manual",
                "allowDestructive": false,
                "allowFormat": false,
                "allowShrink": false,
                "allowPotentialDataLoss": false,
                "allowGrow": true,
                "allowOffline": false,
                "allowPropertyChanges": true
              }
            }"#,
    )
    .expect("document should parse");

    let report = evaluate_apply_policy(&plan, policy.clone());

    assert_eq!(report.blocked_count, 2);
    assert_eq!(report.blocked_summary.destructive_count, 1);
    assert_eq!(report.blocked_summary.potential_data_loss_count, 1);
    assert!(report.blocked.iter().any(|blocked| {
        blocked.reason == "potential-data-loss actions require allowPotentialDataLoss=true"
    }));
    assert!(!report.can_execute());

    policy.allow_destructive = true;
    policy.allow_potential_data_loss = true;
    let report = evaluate_apply_policy(&plan, policy);
    assert_eq!(report.blocked_count, 0);
    assert!(report.can_execute());
}

#[test]
fn apply_policy_requires_backup_and_confirmation_for_allowed_potential_data_loss() {
    let (plan, mut policy) = plan_and_policy_from_json_bytes(
        br#"{
              "filesystems": {
                "home": {
                  "mountpoint": "/home",
                  "fsType": "ext4",
                  "resizePolicy": "shrink-allowed"
                }
              },
              "apply": {
                "allowShrink": true,
                "allowPotentialDataLoss": true,
                "requireBackup": true,
                "backupVerified": false,
                "requireConfirmation": true,
                "confirmation": false
              }
            }"#,
    )
    .expect("document should parse");

    let report = evaluate_apply_policy(&plan, policy.clone());
    assert_eq!(report.blocked_count, 1);
    assert_eq!(
        report.blocked[0].reason,
        "backup-required actions require backupVerified=true"
    );

    policy.backup_verified = true;
    let report = evaluate_apply_policy(&plan, policy.clone());
    assert_eq!(report.blocked_count, 1);
    assert_eq!(
        report.blocked[0].reason,
        "confirmation-required actions require confirmation=true"
    );

    policy.confirmation = true;
    let report = evaluate_apply_policy(&plan, policy);
    assert_eq!(report.blocked_count, 0);
    assert!(report.can_execute());
}

#[test]
fn apply_policy_blocks_unsupported_actions_even_when_permissive() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "filesystems": {
                "archive": {
                  "mountpoint": "/archive",
                  "fsType": "xfs",
                  "resizePolicy": "shrink-allowed"
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowFormat": true,
                "allowShrink": true,
                "allowGrow": true,
                "allowOffline": true,
                "allowPropertyChanges": true
              }
            }"#,
    )
    .expect("document should parse");

    let report = evaluate_apply_policy(&plan, policy);

    assert_eq!(report.blocked_count, 1);
    assert_eq!(report.blocked_summary.unsupported_count, 1);
    assert_eq!(report.blocked[0].risk, RiskClass::Unsupported);
    assert_eq!(
        report.blocked[0].reason,
        "unsupported actions cannot be applied"
    );
}

#[test]
fn apply_policy_allows_grow_when_enabled() {
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
    .expect("document should parse");

    let report = evaluate_apply_policy(&plan, policy);

    assert_eq!(report.blocked_count, 0);
    assert!(report.can_execute());
}

#[test]
fn apply_policy_requires_offline_permission_for_offline_required_actions() {
    let (plan, mut policy) = plan_and_policy_from_json_bytes(
        br#"{
              "luns": {
                "iqn.2026-06.example:storage/root:0": {
                  "operation": "grow"
                }
              },
              "apply": {
                "allowGrow": true,
                "allowOffline": false
              }
            }"#,
    )
    .expect("document should parse");

    let report = evaluate_apply_policy(&plan, policy.clone());
    assert_eq!(report.blocked_count, 1);
    assert_eq!(report.blocked_summary.offline_required_count, 1);
    assert_eq!(report.blocked[0].risk, RiskClass::OfflineRequired);
    assert_eq!(
        report.blocked[0].reason,
        "offline-required actions require allowOffline=true"
    );

    policy.allow_offline = true;
    let report = evaluate_apply_policy(&plan, policy);
    assert_eq!(report.blocked_count, 0);
    assert!(report.can_execute());
}

#[test]
fn apply_policy_requires_format_and_destructive_permission_for_format() {
    let (plan, mut policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "filesystems": {
                  "scratch": {
                    "mountpoint": "/scratch",
                    "fsType": "xfs",
                    "preserveData": false
                  }
                }
              },
              "apply": {
                "allowDestructive": true,
                "allowFormat": false
              }
            }"#,
    )
    .expect("document should parse");

    let report = evaluate_apply_policy(&plan, policy.clone());
    assert!(report
        .blocked
        .iter()
        .any(|blocked| blocked.reason == "format actions require allowFormat=true"));

    policy.allow_format = true;
    let report = evaluate_apply_policy(&plan, policy);
    assert_eq!(report.blocked_count, 0);
}

#[test]
fn apply_policy_can_require_verified_backup_for_high_risk_actions() {
    let (plan, mut policy) = plan_and_policy_from_json_bytes(
        br#"{
              "spec": {
                "datasets": {
                  "tank/old": { "destroy": true }
                }
              },
              "apply": {
                "allowDestructive": true,
                "requireBackup": true,
                "backupVerified": false
              }
            }"#,
    )
    .expect("document should parse");

    let report = evaluate_apply_policy(&plan, policy.clone());
    assert_eq!(report.blocked_count, 1);
    assert_eq!(
        report.blocked[0].reason,
        "backup-required actions require backupVerified=true"
    );

    policy.backup_verified = true;
    let report = evaluate_apply_policy(&plan, policy);
    assert_eq!(report.blocked_count, 0);
}

#[test]
fn apply_policy_can_require_confirmation_for_offline_actions() {
    let (plan, mut policy) = plan_and_policy_from_json_bytes(
        br#"{
              "luns": {
                "iqn.2026-06.example:storage/root:0": {
                  "operation": "grow"
                }
              },
              "apply": {
                "allowGrow": true,
                "allowOffline": true,
                "requireConfirmation": true,
                "confirmation": false
              }
            }"#,
    )
    .expect("document should parse");

    let report = evaluate_apply_policy(&plan, policy.clone());
    assert_eq!(report.blocked_count, 1);
    assert_eq!(
        report.blocked[0].reason,
        "confirmation-required actions require confirmation=true"
    );

    policy.confirmation = true;
    let report = evaluate_apply_policy(&plan, policy);
    assert_eq!(report.blocked_count, 0);
}

#[test]
fn apply_policy_can_require_confirmation_file_for_offline_actions() {
    let (plan, mut policy) = plan_and_policy_from_json_bytes(
        br#"{
              "luns": {
                "iqn.2026-06.example:storage/root:0": {
                  "operation": "grow"
                }
              },
              "apply": {
                "allowGrow": true,
                "allowOffline": true,
                "requireConfirmationFile": "/run/disk-nix/confirm"
              }
            }"#,
    )
    .expect("document should parse");

    let report = evaluate_apply_policy(&plan, policy.clone());
    assert_eq!(report.blocked_count, 1);
    assert_eq!(
        report.blocked[0].reason,
        "confirmation-file policy requires confirmation=true after checking the configured file"
    );

    policy.confirmation = true;
    let report = evaluate_apply_policy(&plan, policy);
    assert_eq!(report.blocked_count, 0);
}

#[test]
fn apply_policy_can_disable_device_topology_changes_and_rebalance() {
    let (plan, policy) = plan_and_policy_from_json_bytes(
        br#"{
              "pools": {
                "tank": {
                  "operation": "rebalance",
                  "addDevices": ["/dev/disk/by-id/new"],
                  "replaceDevices": {
                    "/dev/disk/by-id/old": "/dev/disk/by-id/new"
                  }
                }
              },
              "apply": {
                "allowGrow": true,
                "allowDeviceReplacement": false,
                "allowRebalance": false
              }
            }"#,
    )
    .expect("document should parse");

    let report = evaluate_apply_policy(&plan, policy);

    assert_eq!(report.blocked_count, 3);
    assert!(report.blocked.iter().any(|blocked| {
        blocked.reason == "device topology changes require allowDeviceReplacement=true"
    }));
    assert!(report
        .blocked
        .iter()
        .any(|blocked| blocked.reason == "rebalance actions require allowRebalance=true"));
}
