#[test]
fn solved_zfs_layout_uses_all_redundant_slices_for_mixed_disk_set() {
    let spec = serde_json::json!({
        "solve": {
            "layouts": {
                "desktop": {
                    "disks": {
                        "nvme": {
                            "path": "/dev/disk/by-id/nvme-os",
                            "size": "232.9G",
                            "media": "nvme",
                            "primaryBoot": true
                        },
                        "ssd1": {
                            "path": "/dev/disk/by-id/ata-ssd-1",
                            "size": "465.8G",
                            "media": "ssd"
                        },
                        "ssd2": {
                            "path": "/dev/disk/by-id/ata-ssd-2",
                            "size": "465.8G",
                            "media": "ssd"
                        },
                        "ssd3": {
                            "path": "/dev/disk/by-id/ata-ssd-3",
                            "size": "465.8G",
                            "media": "ssd"
                        },
                        "hdd": {
                            "path": "/dev/disk/by-id/ata-hdd",
                            "size": "931.5G",
                            "media": "hdd"
                        }
                    },
                    "boot": {
                        "type": "efi-replicated",
                        "size": "1GiB",
                        "mountpoint": "/boot"
                    },
                    "swap": {
                        "type": "tail",
                        "priorities": {
                            "nvme": 10,
                            "ssd": 5,
                            "hdd": 1
                        }
                    },
                    "zfs": {
                        "pool": "zroot",
                        "sliceSize": "100GiB",
                        "vdevs": {
                            "prefer": [
                                { "type": "raidz1", "width": 3 },
                                { "type": "mirror", "width": 2 }
                            ],
                            "requireRedundant": true,
                            "unassignedSlicePolicy": "forbid"
                        },
                        "properties": {
                            "ashift": "12",
                            "autotrim": "on",
                            "mountpoint": "none"
                        },
                        "datasets": {
                            "zroot/root": {
                                "operation": "create",
                                "properties": {
                                    "mountpoint": "legacy",
                                    "compression": "zstd"
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    let lowered = lower_solved_layouts(&spec)
        .expect("solve layout should lower")
        .expect("solve layout should be present");
    let pool_devices = lowered
        .pointer("/pools/zroot/devices")
        .and_then(serde_json::Value::as_array)
        .expect("zroot devices should be generated");
    let raidz_count = pool_devices
        .iter()
        .filter(|device| device.as_str() == Some("raidz1"))
        .count();
    let mirror_count = pool_devices
        .iter()
        .filter(|device| device.as_str() == Some("mirror"))
        .count();
    let member_count = pool_devices
        .iter()
        .filter(|device| {
            device
                .as_str()
                .is_some_and(|device| device.starts_with("/dev/disk/by-id/"))
        })
        .count();

    assert_eq!(raidz_count, 5);
    assert_eq!(mirror_count, 4);
    assert_eq!(member_count, 23);
    assert_eq!(
        lowered.pointer("/partitions/desktop-nvme-swap/target"),
        Some(&serde_json::json!("/dev/disk/by-id/nvme-os-part4"))
    );
    assert_eq!(
        lowered.pointer("/partitions/desktop-nvme-zfs-1/start"),
        Some(&serde_json::json!("1025MiB"))
    );
    assert_eq!(
        lowered.pointer("/partitions/desktop-nvme-zfs-2/start"),
        Some(&serde_json::json!("101GiB"))
    );
    assert_eq!(
        lowered.pointer("/partitions/desktop-hdd-swap/target"),
        Some(&serde_json::json!("/dev/disk/by-id/ata-hdd-part11"))
    );
    assert_eq!(
        lowered.pointer("/filesystems/desktop-boot-nvme/mountpoint"),
        Some(&serde_json::json!("/boot"))
    );
    assert_eq!(
        lowered.pointer("/filesystems/desktop-boot-ssd1/mountpoint"),
        None
    );
}

#[test]
fn partition_create_actions_order_by_disk_offset_not_lexical_name() {
    let spec = serde_json::json!({
        "partitions": {
            "desktop-disk-slice-1": {
                "operation": "create",
                "device": "/dev/disk/by-id/ata-test",
                "partitionNumber": "2",
                "partitionType": "primary",
                "start": "1025MiB",
                "end": "101GiB",
                "target": "/dev/disk/by-id/ata-test-part2"
            },
            "desktop-disk-slice-10": {
                "operation": "create",
                "device": "/dev/disk/by-id/ata-test",
                "partitionNumber": "11",
                "partitionType": "primary",
                "start": "901GiB",
                "end": "1001GiB",
                "target": "/dev/disk/by-id/ata-test-part11"
            },
            "desktop-disk-slice-2": {
                "operation": "create",
                "device": "/dev/disk/by-id/ata-test",
                "partitionNumber": "3",
                "partitionType": "primary",
                "start": "101GiB",
                "end": "201GiB",
                "target": "/dev/disk/by-id/ata-test-part3"
            },
            "desktop-disk-swap": {
                "operation": "create",
                "device": "/dev/disk/by-id/ata-test",
                "partitionNumber": "12",
                "partitionType": "primary",
                "start": "1001GiB",
                "end": "100%",
                "target": "/dev/disk/by-id/ata-test-part12"
            }
        }
    });

    let plan = plan_from_value(&spec);
    let partition_actions: Vec<&str> = plan
        .actions
        .iter()
        .filter(|action| action.context.collection.as_deref() == Some("partitions"))
        .map(|action| action.id.as_str())
        .collect();

    assert_eq!(
        partition_actions,
        vec![
            "partitions:desktop-disk-slice-1:create",
            "partitions:desktop-disk-slice-2:create",
            "partitions:desktop-disk-slice-10:create",
            "partitions:desktop-disk-swap:create",
        ]
    );
}

#[test]
fn zfs_dataset_create_actions_order_parents_before_children() {
    let spec = serde_json::json!({
        "datasets": {
            "zpool/root/home": {
                "operation": "create",
                "properties": {
                    "mountpoint": "legacy"
                }
            },
            "zpool/root": {
                "operation": "create",
                "properties": {
                    "mountpoint": "none"
                }
            },
            "zpool/root/home/projects": {
                "operation": "create",
                "properties": {
                    "mountpoint": "legacy"
                }
            }
        }
    });

    let plan = plan_from_value(&spec);
    let dataset_create_actions: Vec<&str> = plan
        .actions
        .iter()
        .filter(|action| {
            action.context.collection.as_deref() == Some("datasets")
                && action.operation == Operation::Create
        })
        .map(|action| action.id.as_str())
        .collect();

    assert_eq!(
        dataset_create_actions,
        vec![
            "datasets:zpool/root:create",
            "datasets:zpool/root/home:create",
            "datasets:zpool/root/home/projects:create",
        ]
    );
}

#[test]
fn solved_zfs_layout_splits_fast_and_cold_pools_by_inferred_tier() {
    let spec = serde_json::json!({
        "solve": {
            "layouts": {
                "desktop": {
                    "disks": {
                        "nvme": {
                            "path": "/dev/disk/by-id/nvme-system",
                            "size": "201GiB",
                            "primaryBoot": true
                        },
                        "ssd": {
                            "path": "/dev/disk/by-id/ata-samsung-fast",
                            "size": "201GiB",
                            "solidState": true
                        },
                        "hdd1": {
                            "path": "/dev/disk/by-id/ata-wd-cold-1",
                            "size": "201GiB",
                            "rotational": true
                        },
                        "hdd2": {
                            "path": "/dev/disk/by-id/ata-wd-cold-2",
                            "size": "201GiB",
                            "media": "hdd"
                        },
                        "usb": {
                            "path": "/dev/disk/by-id/usb-backup",
                            "size": "201GiB"
                        }
                    },
                    "boot": {
                        "type": "efi-replicated",
                        "size": "1GiB"
                    },
                    "swap": { "type": "tail" },
                    "zfs": {
                        "sliceSize": "100GiB",
                        "vdevs": {
                            "prefer": [ { "type": "mirror", "width": 2 } ],
                            "requireRedundant": true,
                            "unassignedSlicePolicy": "forbid"
                        },
                        "pools": {
                            "fast": {
                                "pool": "zfast",
                                "tier": "fast",
                                "properties": {
                                    "autotrim": "on"
                                }
                            },
                            "cold": {
                                "pool": "zcold",
                                "tier": "cold",
                                "properties": {
                                    "autotrim": "off"
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    let lowered = lower_solved_layouts(&spec)
        .expect("solve layout should lower")
        .expect("solve layout should be present");
    let fast_devices = lowered
        .pointer("/pools/zfast/devices")
        .and_then(serde_json::Value::as_array)
        .expect("fast pool devices should be generated");
    let cold_devices = lowered
        .pointer("/pools/zcold/devices")
        .and_then(serde_json::Value::as_array)
        .expect("cold pool devices should be generated");

    assert_eq!(
        fast_devices
            .iter()
            .filter(|device| device.as_str() == Some("mirror"))
            .count(),
        2
    );
    assert_eq!(
        cold_devices
            .iter()
            .filter(|device| device.as_str() == Some("mirror"))
            .count(),
        2
    );
    assert!(fast_devices.iter().any(|device| device
        .as_str()
        .is_some_and(|device| device == "/dev/disk/by-id/nvme-system-part2")));
    assert!(fast_devices.iter().any(|device| device
        .as_str()
        .is_some_and(|device| device == "/dev/disk/by-id/ata-samsung-fast-part2")));
    assert!(!fast_devices.iter().any(|device| device
        .as_str()
        .is_some_and(|device| device.contains("wd-cold"))));
    assert!(cold_devices.iter().any(|device| device
        .as_str()
        .is_some_and(|device| device == "/dev/disk/by-id/ata-wd-cold-1-part2")));
    assert!(cold_devices.iter().any(|device| device
        .as_str()
        .is_some_and(|device| device == "/dev/disk/by-id/ata-wd-cold-2-part2")));
    assert!(!cold_devices.iter().any(|device| device
        .as_str()
        .is_some_and(|device| device.contains("nvme-system"))));
    assert_eq!(
        lowered.pointer("/disks/nvme/properties/tier"),
        Some(&serde_json::json!("fast"))
    );
    assert_eq!(
        lowered.pointer("/disks/hdd1/properties/tier"),
        Some(&serde_json::json!("cold"))
    );
    assert_eq!(
        lowered.pointer("/partitions/desktop-nvme-fast-zfs-1/target"),
        Some(&serde_json::json!("/dev/disk/by-id/nvme-system-part2"))
    );
    assert_eq!(
        lowered.pointer("/partitions/desktop-hdd1-cold-zfs-1/target"),
        Some(&serde_json::json!("/dev/disk/by-id/ata-wd-cold-1-part2"))
    );
    assert_eq!(
        lowered.pointer("/partitions/desktop-usb-zfs-1"),
        None
    );
    assert_eq!(
        lowered.pointer("/partitions/desktop-usb-swap/target"),
        Some(&serde_json::json!("/dev/disk/by-id/usb-backup-part2"))
    );
}

#[test]
fn solved_zfs_layout_rejects_disk_selected_by_multiple_pools() {
    let document = serde_json::json!({
        "version": 1,
        "spec": {
            "solve": {
                "layouts": {
                    "desktop": {
                        "disks": {
                            "a": { "path": "/dev/disk/by-id/a", "size": "201GiB", "tier": "fast" },
                            "b": { "path": "/dev/disk/by-id/b", "size": "201GiB", "tier": "fast" }
                        },
                        "zfs": {
                            "sliceSize": "100GiB",
                            "vdevs": {
                                "prefer": [ { "type": "mirror", "width": 2 } ]
                            },
                            "pools": {
                                "left": { "tier": "fast" },
                                "right": { "disks": [ "a" ] }
                            }
                        }
                    }
                }
            }
        }
    });

    let error = plan_from_value_checked(&document).expect_err("overlap should be rejected");

    assert!(error
        .to_string()
        .contains("disk a is selected by both zfs pools left and right"));
}

#[test]
fn solved_zfs_layout_allows_single_member_pool_for_one_disk_cold_tier() {
    let spec = serde_json::json!({
        "solve": {
            "layouts": {
                "desktop": {
                    "disks": {
                        "hdd": {
                            "path": "/dev/disk/by-id/ata-cold",
                            "size": "301GiB",
                            "rotational": true
                        }
                    },
                    "boot": { "size": "1GiB" },
                    "zfs": {
                        "sliceSize": "100GiB",
                        "pools": {
                            "cold": {
                                "pool": "zcold",
                                "tier": "cold",
                                "vdevs": {
                                    "prefer": [ { "type": "single", "width": 1 } ],
                                    "unassignedSlicePolicy": "forbid"
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    let lowered = lower_solved_layouts(&spec)
        .expect("solve layout should lower")
        .expect("solve layout should be present");
    let cold_devices = lowered
        .pointer("/pools/zcold/devices")
        .and_then(serde_json::Value::as_array)
        .expect("cold pool devices should be generated");

    assert_eq!(cold_devices.len(), 3);
    assert!(!cold_devices
        .iter()
        .any(|device| device.as_str() == Some("single")));
    assert_eq!(
        cold_devices.first(),
        Some(&serde_json::json!("/dev/disk/by-id/ata-cold-part2"))
    );
}

#[test]
fn solved_zfs_layout_is_stable_independent_of_input_object_order() {
    let left = serde_json::json!({
        "solve": {
            "layouts": {
                "desktop": {
                    "disks": {
                        "b": { "path": "/dev/disk/by-id/b", "size": "301GiB", "media": "ssd" },
                        "a": { "path": "/dev/disk/by-id/a", "size": "301GiB", "media": "ssd" },
                        "c": { "path": "/dev/disk/by-id/c", "size": "301GiB", "media": "ssd" }
                    },
                    "boot": { "type": "efi-replicated", "primaryDisk": "a" },
                    "swap": { "type": "tail" },
                    "zfs": {
                        "pool": "zroot",
                        "sliceSize": "100GiB",
                        "vdevs": { "prefer": [ { "type": "raidz1", "width": 3 } ] }
                    }
                }
            }
        }
    });
    let right = serde_json::json!({
        "solve": {
            "layouts": {
                "desktop": {
                    "zfs": {
                        "vdevs": { "prefer": [ { "width": 3, "type": "raidz1" } ] },
                        "sliceSize": "100GiB",
                        "pool": "zroot"
                    },
                    "swap": { "type": "tail" },
                    "boot": { "primaryDisk": "a", "type": "efi-replicated" },
                    "disks": {
                        "c": { "media": "ssd", "size": "301GiB", "path": "/dev/disk/by-id/c" },
                        "a": { "media": "ssd", "size": "301GiB", "path": "/dev/disk/by-id/a" },
                        "b": { "media": "ssd", "size": "301GiB", "path": "/dev/disk/by-id/b" }
                    }
                }
            }
        }
    });

    let left_devices = lower_solved_layouts(&left)
        .expect("left should lower")
        .expect("left solve layout should be present")
        .pointer("/pools/zroot/devices")
        .cloned();
    let right_devices = lower_solved_layouts(&right)
        .expect("right should lower")
        .expect("right solve layout should be present")
        .pointer("/pools/zroot/devices")
        .cloned();

    assert_eq!(left_devices, right_devices);
}

#[test]
fn solved_zfs_layout_rejects_forbidden_unassigned_full_slices() {
    let document = serde_json::json!({
        "version": 1,
        "spec": {
            "solve": {
                "layouts": {
                    "desktop": {
                        "disks": {
                            "a": { "path": "/dev/disk/by-id/a", "size": "101GiB", "media": "ssd" },
                            "b": { "path": "/dev/disk/by-id/b", "size": "101GiB", "media": "ssd" },
                            "c": { "path": "/dev/disk/by-id/c", "size": "101GiB", "media": "ssd" },
                            "d": { "path": "/dev/disk/by-id/d", "size": "101GiB", "media": "ssd" }
                        },
                        "boot": { "type": "efi-replicated", "primaryDisk": "a" },
                        "swap": { "type": "tail" },
                        "zfs": {
                            "pool": "zroot",
                            "sliceSize": "100GiB",
                            "vdevs": {
                                "prefer": [ { "type": "raidz1", "width": 3 } ],
                                "requireRedundant": true,
                                "unassignedSlicePolicy": "forbid"
                            }
                        }
                    }
                }
            }
        }
    });

    let error = plan_from_value_checked(&document).expect_err("layout should reject leftovers");

    assert!(error
        .to_string()
        .contains("leaves 1 full ZFS slice(s) unassigned"));
}

#[test]
fn solved_zfs_layout_reports_when_required_redundancy_is_impossible() {
    let document = serde_json::json!({
        "version": 1,
        "spec": {
            "solve": {
                "layouts": {
                    "desktop": {
                        "disks": {
                            "a": { "path": "/dev/disk/by-id/a", "size": "301GiB", "media": "ssd" }
                        },
                        "zfs": {
                            "pool": "zroot",
                            "sliceSize": "100GiB",
                            "vdevs": {
                                "prefer": [ { "type": "mirror", "width": 2 } ],
                                "requireRedundant": true
                            }
                        }
                    }
                }
            }
        }
    });

    let error = plan_from_value_checked(&document).expect_err("layout should require redundancy");

    assert!(error
        .to_string()
        .contains("cannot form any redundant vdev"));
}

#[test]
fn solved_zfs_layout_rejects_non_redundant_shapes_when_redundancy_required() {
    let document = serde_json::json!({
        "version": 1,
        "spec": {
            "solve": {
                "layouts": {
                    "desktop": {
                        "disks": {
                            "a": { "path": "/dev/disk/by-id/a", "size": "101GiB" },
                            "b": { "path": "/dev/disk/by-id/b", "size": "101GiB" }
                        },
                        "zfs": {
                            "pool": "zroot",
                            "sliceSize": "100GiB",
                            "vdevs": {
                                "prefer": [ { "type": "stripe", "width": 2 } ],
                                "requireRedundant": true
                            }
                        }
                    }
                }
            }
        }
    });

    let error = plan_from_value_checked(&document).expect_err("stripe should be rejected");

    assert!(error
        .to_string()
        .contains("vdev shape stripe is not a redundant ZFS shape"));
}

#[test]
fn planner_builds_actions_from_solved_layouts() {
    let document = serde_json::json!({
        "version": 1,
        "spec": {
            "solve": {
                "layouts": {
                    "desktop": {
                        "disks": {
                            "a": {
                                "path": "/dev/disk/by-id/a",
                                "size": "301GiB",
                                "media": "ssd",
                                "primaryBoot": true
                            },
                            "b": {
                                "path": "/dev/disk/by-id/b",
                                "size": "301GiB",
                                "media": "ssd"
                            },
                            "c": {
                                "path": "/dev/disk/by-id/c",
                                "size": "301GiB",
                                "media": "ssd"
                            }
                        },
                        "boot": { "type": "efi-replicated", "mountpoint": "/boot" },
                        "swap": { "type": "tail" },
                        "zfs": {
                            "pool": "zroot",
                            "sliceSize": "100GiB",
                            "vdevs": {
                                "prefer": [ { "type": "raidz1", "width": 3 } ]
                            },
                            "datasets": {
                                "zroot/root": {
                                    "operation": "create",
                                    "properties": {
                                        "mountpoint": "legacy"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    let plan = plan_from_value(&document);
    let action_ids = plan
        .actions
        .iter()
        .map(|action| action.id.as_str())
        .collect::<BTreeSet<_>>();

    assert!(action_ids.contains("disks:a:create"));
    assert!(action_ids.contains("partitions:desktop-a-efi:create"));
    assert!(action_ids.contains("filesystem:desktop-boot-a:preserve-data-disabled"));
    assert!(action_ids.contains("swaps:desktop-swap-a:format"));
    assert!(action_ids.contains("pools:zroot:create"));
    assert!(action_ids.contains("datasets:zroot/root:create"));
}
