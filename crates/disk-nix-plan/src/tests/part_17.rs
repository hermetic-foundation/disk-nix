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
