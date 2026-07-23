    const CLUSTERED_LVM_PVS: &[u8] = br#"{
      "report": [{
        "pv": [{
          "pv_name": "/dev/nvme2n1",
          "vg_name": "vgcluster",
          "pv_fmt": "lvm2",
          "pv_uuid": "cluster-pv-uuid",
          "dev_size": "465.66g",
          "pv_size": "465.66g",
          "pv_free": "165.66g",
          "pv_used": "300.00g",
          "pv_attr": "a--",
          "pv_allocatable": "allocatable",
          "pv_pe_count": "119209",
          "pv_pe_alloc_count": "76800",
          "pv_tags": "fabric,shared",
          "pv_in_use": "used",
          "pv_device_id": "nvme.0123456789abcdef0123456789abcdef",
          "pv_device_id_type": "sys_wwid"
        }]
      }]
    }"#;

    const CLUSTERED_LVM_VGS: &[u8] = br#"{
      "report": [{
        "vg": [{
          "vg_name": "vgcluster",
          "vg_fmt": "lvm2",
          "vg_uuid": "cluster-vg-uuid",
          "vg_attr": "wz--ns",
          "vg_permissions": "writeable",
          "vg_extendable": "extendable",
          "vg_autoactivation": "enabled",
          "vg_partial": "",
          "vg_allocation_policy": "cling",
          "vg_clustered": "clustered",
          "vg_shared": "shared",
          "vg_size": "465.66g",
          "vg_free": "165.66g",
          "vg_sysid": "node-a",
          "vg_lock_type": "sanlock",
          "vg_lock_args": "host_id=1",
          "vg_extent_size": "4.00m",
          "vg_extent_count": "119209",
          "vg_free_count": "42409",
          "pv_count": "1",
          "vg_missing_pv_count": "0",
          "lv_count": "1",
          "snap_count": "0",
          "vg_seqno": "42",
          "vg_tags": "clustered,fabric"
        }]
      }]
    }"#;

    const CLUSTERED_LVM_LVS: &[u8] = br#"{
      "report": [{
        "lv": [{
          "lv_name": "shareddata",
          "vg_name": "vgcluster",
          "lv_uuid": "cluster-lv-uuid",
          "lv_path": "/dev/vgcluster/shareddata",
          "lv_size": "300.00g",
          "lv_attr": "-wi-ao----",
          "lv_layout": "linear",
          "lv_active": "active",
          "lv_active_locally": "active locally",
          "lv_active_remotely": "active remotely",
          "lv_active_exclusively": "",
          "lv_permissions": "writeable",
          "lv_health_status": "",
          "lv_tags": "clustered,fabric",
          "lv_dm_path": "/dev/mapper/vgcluster-shareddata",
          "lv_read_ahead": "auto",
          "lv_kernel_read_ahead": "256",
          "lv_suspended": "not suspended",
          "lv_live_table": "live",
          "lv_modules": "linear",
          "lv_host": "node-a",
          "lv_kernel_major": "253",
          "lv_kernel_minor": "10",
          "lv_device_open": "open",
          "lv_role": "public"
        }]
      }]
    }"#;

    const CLUSTERED_FAILURE_PVS: &[u8] = br#"{
      "report": [{
        "pv": [
          {
            "pv_name": "/dev/mapper/mpath-cluster-a",
            "vg_name": "vgshared",
            "pv_fmt": "lvm2",
            "pv_uuid": "shared-pv-a",
            "pv_size": "1.00t",
            "pv_free": "256.00g",
            "pv_used": "768.00g",
            "pv_attr": "a--",
            "pv_allocatable": "allocatable",
            "pv_tags": "fabric-a,lockspace",
            "pv_in_use": "used",
            "pv_device_id": "dm.uuid.mpath-3600a098038314f6f2b5d514d43594c33",
            "pv_device_id_type": "sys_wwid"
          },
          {
            "pv_name": "/dev/mapper/mpath-cluster-b",
            "vg_name": "vgshared",
            "pv_fmt": "lvm2",
            "pv_uuid": "shared-pv-b",
            "pv_size": "1.00t",
            "pv_free": "512.00g",
            "pv_used": "512.00g",
            "pv_attr": "a--",
            "pv_allocatable": "allocatable",
            "pv_tags": "fabric-b,lockspace",
            "pv_in_use": "used",
            "pv_device_id": "dm.uuid.mpath-3600a098038314f6f2b5d514d43594c44",
            "pv_device_id_type": "sys_wwid"
          }
        ]
      }]
    }"#;

    const CLUSTERED_FAILURE_VGS: &[u8] = br#"{
      "report": [{
        "vg": [{
          "vg_name": "vgshared",
          "vg_fmt": "lvm2",
          "vg_uuid": "shared-vg-uuid",
          "vg_attr": "wz--ns",
          "vg_permissions": "writeable",
          "vg_extendable": "extendable",
          "vg_autoactivation": "disabled",
          "vg_partial": "partial",
          "vg_allocation_policy": "cling",
          "vg_clustered": "clustered",
          "vg_shared": "shared",
          "vg_size": "2.00t",
          "vg_free": "768.00g",
          "vg_sysid": "node-b",
          "vg_lock_type": "dlm",
          "vg_lock_args": "lockspace=vgshared host_id=2",
          "vg_lock_status": "partial",
          "vg_lock_failure": "lvmlockd unavailable",
          "vg_lock_reason": "quorum lost after fabric partition",
          "vg_split_brain": "suspected",
          "vg_extent_size": "4.00m",
          "vg_extent_count": "524288",
          "vg_free_count": "196608",
          "pv_count": "2",
          "vg_missing_pv_count": "1",
          "lv_count": "2",
          "snap_count": "0",
          "vg_seqno": "88",
          "vg_tags": "clustered,split-brain,lock-failure"
        }]
      }]
    }"#;

    const CLUSTERED_FAILURE_LVS: &[u8] = br#"{
      "report": [{
        "lv": [
          {
            "lv_name": "remoteactive",
            "vg_name": "vgshared",
            "lv_uuid": "remote-lv-uuid",
            "lv_path": "/dev/vgshared/remoteactive",
            "lv_size": "512.00g",
            "lv_attr": "-wi-ao----",
            "lv_layout": "linear",
            "lv_active": "active",
            "lv_active_locally": "",
            "lv_active_remotely": "active remotely",
            "lv_active_exclusively": "",
            "lv_permissions": "writeable",
            "lv_health_status": "warning",
            "lv_tags": "remote,clustered",
            "lv_dm_path": "/dev/mapper/vgshared-remoteactive",
            "lv_suspended": "not suspended",
            "lv_live_table": "live",
            "lv_modules": "linear",
            "lv_host": "node-a",
            "lv_lock_status": "remote",
            "lv_lock_args": "dlm remote-holder=node-a",
            "lv_role": "public"
          },
          {
            "lv_name": "blocked",
            "vg_name": "vgshared",
            "lv_uuid": "blocked-lv-uuid",
            "lv_path": "/dev/vgshared/blocked",
            "lv_size": "256.00g",
            "lv_attr": "-wi---p---",
            "lv_layout": "linear",
            "lv_active": "inactive",
            "lv_active_locally": "",
            "lv_active_remotely": "",
            "lv_permissions": "writeable",
            "lv_health_status": "lock-failed",
            "lv_tags": "blocked,split-brain",
            "lv_dm_path": "/dev/mapper/vgshared-blocked",
            "lv_suspended": "suspended",
            "lv_live_table": "inactive",
            "lv_modules": "linear",
            "lv_host": "node-b",
            "lv_lock_status": "failed",
            "lv_lock_args": "dlm local-holder=node-b",
            "lv_lock_failure": "resource busy",
            "lv_lock_reason": "split-brain protection refused activation",
            "lv_device_open": "closed",
            "lv_role": "public"
          }
        ]
      }]
    }"#;
