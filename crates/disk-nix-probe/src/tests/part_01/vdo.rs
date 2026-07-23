    const LVM_BACKED_VDO_STATUS: &[u8] = br#"
VDO status:
  Date: '2026-06-26 10:00:00-05:00'
VDOs:
  vgvdo-vdoarchive:
    VDO device: /dev/mapper/vgvdo-vdoarchive
    Storage device: /dev/mapper/vgvdo-vdopool
    Logical size: 2T
    Physical size: 512G
    Compression: enabled
    Deduplication: enabled
    Configured write policy: auto
    Write policy: async
    Index memory setting: 0.50
    Block map cache size: 256M
"#;

    const LVM_BACKED_VDOSTATS: &[u8] = br#"
Device                         1K-blocks     Used Available Use% Space saving%
/dev/mapper/vgvdo-vdoarchive          2T     512G      1.5T  25%           68%
"#;

    const LVM_BACKED_VDOSTATS_VERBOSE: &[u8] = br#"
/dev/mapper/vgvdo-vdoarchive:
  version: 47
  operating mode: normal
  recovery percentage: 100
  write policy: async
  data blocks used: 98304
  overhead blocks used: 16384
  logical blocks used: 524288
"#;

    const LVM_BACKED_VDO_PVS: &[u8] = br#"{
      "report": [{
        "pv": [{
          "pv_name": "/dev/sdf1",
          "vg_name": "vgvdo",
          "pv_fmt": "lvm2",
          "pv_uuid": "vdo-pv-uuid",
          "pv_size": "1.00t",
          "pv_free": "448.00g",
          "pv_used": "576.00g",
          "pv_attr": "a--",
          "pv_allocatable": "allocatable",
          "pv_tags": "vdo,archive",
          "pv_in_use": "used"
        }]
      }]
    }"#;

    const LVM_BACKED_VDO_VGS: &[u8] = br#"{
      "report": [{
        "vg": [{
          "vg_name": "vgvdo",
          "vg_fmt": "lvm2",
          "vg_uuid": "vdo-vg-uuid",
          "vg_attr": "wz--n-",
          "vg_permissions": "writeable",
          "vg_extendable": "extendable",
          "vg_size": "1.00t",
          "vg_free": "448.00g",
          "vg_extent_size": "4.00m",
          "vg_extent_count": "262144",
          "vg_free_count": "114688",
          "pv_count": "1",
          "lv_count": "2",
          "vg_seqno": "17",
          "vg_tags": "vdo,archive"
        }]
      }]
    }"#;

    const LVM_BACKED_VDO_LVS: &[u8] = br#"{
      "report": [{
        "lv": [
          {
            "lv_name": "vdoarchive",
            "vg_name": "vgvdo",
            "lv_uuid": "vdo-lv-uuid",
            "lv_path": "/dev/mapper/vgvdo-vdoarchive",
            "lv_size": "2.00t",
            "lv_attr": "Vwi-a-v---",
            "lv_layout": "vdo",
            "lv_active": "active",
            "lv_active_locally": "active locally",
            "lv_permissions": "writeable",
            "lv_health_status": "",
            "lv_tags": "archive,compressed",
            "lv_dm_path": "/dev/mapper/vgvdo-vdoarchive",
            "lv_modules": "vdo",
            "lv_device_open": "open",
            "lv_role": "public",
            "pool_lv": "vdopool",
            "data_percent": "25.00",
            "metadata_percent": "12.50",
            "vdo_operating_mode": "normal",
            "vdo_compression_state": "online",
            "vdo_index_state": "online",
            "vdo_used_size": "512.00g",
            "vdo_saving_percent": "68.00"
          },
          {
            "lv_name": "vdopool",
            "vg_name": "vgvdo",
            "lv_uuid": "vdo-pool-uuid",
            "lv_path": "/dev/mapper/vgvdo-vdopool",
            "lv_size": "576.00g",
            "lv_attr": "-wi-a-----",
            "lv_layout": "linear",
            "lv_active": "active",
            "lv_permissions": "writeable",
            "lv_tags": "archive,pool",
            "lv_dm_path": "/dev/mapper/vgvdo-vdopool",
            "lv_modules": "linear",
            "lv_device_open": "open",
            "lv_role": "private"
          }
        ]
      }]
    }"#;

    const LVM_BACKED_VDO_SEGMENTS: &[u8] = br#"{
      "report": [{
        "seg": [{
          "lv_name": "vdoarchive",
          "vg_name": "vgvdo",
          "segtype": "vdo",
          "seg_size": "2.00t",
          "seg_size_pe": "524288",
          "devices": "vdopool(0)",
          "metadata_devices": "vdopool(0)",
          "vdo_compression": "enabled",
          "vdo_deduplication": "enabled",
          "vdo_minimum_io_size": "4096",
          "vdo_block_map_cache_size": "256.00m",
          "vdo_block_map_era_length": "16380",
          "vdo_use_sparse_index": "enabled",
          "vdo_index_memory_size": "512.00m",
          "vdo_slab_size": "2.00g",
          "vdo_ack_threads": "1",
          "vdo_bio_threads": "4",
          "vdo_bio_rotation": "64",
          "vdo_cpu_threads": "2",
          "vdo_hash_zone_threads": "1",
          "vdo_logical_threads": "2",
          "vdo_physical_threads": "2",
          "vdo_max_discard": "4.00m",
          "vdo_header_size": "512.00k",
          "vdo_use_metadata_hints": "enabled",
          "vdo_write_policy": "auto"
        }]
      }]
    }"#;

    const VDO_PRESSURE_STATUS: &[u8] = br#"
VDO status:
  Date: '2026-06-26 11:00:00-05:00'
VDOs:
  archive-pressure:
    VDO device: /dev/mapper/archive-pressure
    Storage device: /dev/disk/by-id/scsi-vdo-pressure
    Logical size: 8T
    Physical size: 1T
    Compression: disabled
    Deduplication: enabled
    Configured write policy: sync
    Write policy: async
    Operating mode: recovering
    Index state: rebuilding
    Index rebuild progress: 42%
    Physical space status: near-full
    Last start result: failed
    Last stop result: timeout
  archive-stopped:
    VDO device: /dev/mapper/archive-stopped
    Storage device: /dev/disk/by-id/scsi-vdo-stopped
    Logical size: 4T
    Physical size: 2T
    Compression: enabled
    Deduplication: disabled
    Configured write policy: auto
    Write policy: read-only
    Operating mode: read-only
    VDO service state: stopped
    Last start result: device busy
    Last stop result: failed
"#;

    const VDO_PRESSURE_STATS: &[u8] = br#"
Device                         1K-blocks     Used Available Use% Space saving%
/dev/mapper/archive-pressure          8T     7.6T     400G  95%           12%
/dev/mapper/archive-stopped           4T     3.8T     200G  95%            0%
"#;

    const VDO_PRESSURE_VERBOSE: &[u8] = br#"
/dev/mapper/archive-pressure:
  operating mode: recovering
  recovery percentage: 42
  index state: rebuilding
  physical space status: near-full
  compression state: offline
  deduplication state: online
  data blocks used: 1992294
  overhead blocks used: 204800
  logical blocks used: 2097152
/dev/mapper/archive-stopped:
  operating mode: read-only
  recovery percentage: 0
  index state: offline
  physical space status: full
  compression state: online
  deduplication state: offline
  data blocks used: 996147
  overhead blocks used: 102400
  logical blocks used: 1048576
"#;
