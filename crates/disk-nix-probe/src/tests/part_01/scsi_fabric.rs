    const SHARED_ISCSI_SESSION: &[u8] = br#"
Target: iqn.2026-06.example:storage.shared
    Current Portal: 10.0.0.10:3260,1
    Persistent Portal: 10.0.0.10:3260,1
    Target Portal Group Tag: 1
    **********
    Interface:
    **********
    Iface Name: default
    Iface Transport: tcp
    Iface Initiatorname: iqn.2026-06.client:node1
    Iface IPaddress: 10.0.0.20
    Iface Netdev: eno1
    SID: 42
    iSCSI Connection State: LOGGED IN
    iSCSI Session State: LOGGED_IN
    Internal iscsid Session State: NO CHANGE
    HeaderDigest: None
    DataDigest: None
    MaxRecvDataSegmentLength: 262144
    CID: 0
    Connection State: LOGGED IN
    Local Address: 10.0.0.20
    Peer Address: 10.0.0.10
    Host Number: 2  State: running
    scsi2 Channel 00 Id 0 Lun: 1
        Attached scsi disk sdb          State: running
"#;

    const SHARED_ISCSI_NODE: &[u8] = br#"
Target: iqn.2026-06.example:storage.shared
    Portal: 10.0.0.10:3260,1
    Persistent Portal: 10.0.0.11:3260,1
    TPGT: 1
    Iface Name: default
    Startup: automatic
    Leading Login: Yes
    AuthMethod: CHAP
    Username: node-user
    Password: outbound-secret
    Username_in: target-user
    Password_in: inbound-secret
"#;

    const SHARED_LSSCSI_LIST: &[u8] = br#"
[2:0:0:1]    disk    LIO-ORG  shared-lun      4.0   /dev/sdb   /dev/sg2   100G
  device_blocked=0
  queue_depth=128
  queue_type=simple
  state=running
  timeout=60
[3:0:0:1]    disk    LIO-ORG  shared-lun      4.0   /dev/sdc   /dev/sg3   100G
  device_blocked=0
  queue_depth=128
  queue_type=simple
  state=running
  timeout=60
"#;

    const SHARED_LSSCSI_TRANSPORT: &[u8] = br#"
[2:0:0:1]    disk    iscsi:iqn.2026-06.example:storage.shared,t,0x1  /dev/sdb   /dev/sg2  /dev/disk/by-id/scsi-3600508b400105e210000900000490000  /dev/disk/by-id/wwn-0x600508b400105e210000900000490000  100G
[3:0:0:1]    disk    iscsi:iqn.2026-06.example:storage.shared,t,0x2  /dev/sdc   /dev/sg3  /dev/disk/by-id/scsi-3600508b400105e210000900000490000  /dev/disk/by-id/wwn-0x600508b400105e210000900000490000  100G
"#;

    const SHARED_LSSCSI_UNIT: &[u8] = br#"
[2:0:0:1]    disk    3600508b400105e210000900000490000  /dev/sdb   /dev/sg2  /dev/disk/by-id/scsi-3600508b400105e210000900000490000  /dev/disk/by-id/wwn-0x600508b400105e210000900000490000  100G
[3:0:0:1]    disk    3600508b400105e210000900000490000  /dev/sdc   /dev/sg3  /dev/disk/by-id/scsi-3600508b400105e210000900000490000  /dev/disk/by-id/wwn-0x600508b400105e210000900000490000  100G
"#;

    const SHARED_MULTIPATH: &[u8] = br#"
mpatha (3600508b400105e210000900000490000) dm-2 LIO-ORG,shared-lun
size=100G features='1 queue_if_no_path' hwhandler='1 alua' wp=rw
|-+- policy='service-time 0' prio=50 status=active
| `- 2:0:0:1 sdb 8:16 active ready running ghost
`-+- policy='service-time 0' prio=10 status=enabled
  `- 3:0:0:1 sdc 8:32 active ready running faulty shaky
"#;

    const FC_LSSCSI_LIST: &[u8] = br#"
[6:0:2:12]   disk    DGC      VRAID            0532  /dev/sdd   /dev/sg4   2.00T
  device_blocked=0
  queue_depth=64
  queue_type=simple
  state=running
  timeout=60
[7:0:3:12]   disk    DGC      VRAID            0532  /dev/sde   /dev/sg5   2.00T
  device_blocked=0
  queue_depth=64
  queue_type=simple
  state=blocked
  timeout=60
"#;

    const FC_LSSCSI_TRANSPORT: &[u8] = br#"
[6:0:2:12]   disk    fc:0x5006016841e0abcd,0x5006016041e0abcd           /dev/sdd   /dev/sg4  /dev/disk/by-id/scsi-36006016041e05d00c8b7f0a0d7a4ee11  /dev/disk/by-id/wwn-0x6006016041e05d00c8b7f0a0d7a4ee11  2.00T
[7:0:3:12]   disk    fc:0x5006016841e0abce,0x5006016041e0abce           /dev/sde   /dev/sg5  /dev/disk/by-id/scsi-36006016041e05d00c8b7f0a0d7a4ee11  /dev/disk/by-id/wwn-0x6006016041e05d00c8b7f0a0d7a4ee11  2.00T
"#;

    const FC_LSSCSI_UNIT: &[u8] = br#"
[6:0:2:12]   disk    36006016041e05d00c8b7f0a0d7a4ee11                  /dev/sdd   /dev/sg4  /dev/disk/by-id/scsi-36006016041e05d00c8b7f0a0d7a4ee11  /dev/disk/by-id/wwn-0x6006016041e05d00c8b7f0a0d7a4ee11  2.00T
[7:0:3:12]   disk    36006016041e05d00c8b7f0a0d7a4ee11                  /dev/sde   /dev/sg5  /dev/disk/by-id/scsi-36006016041e05d00c8b7f0a0d7a4ee11  /dev/disk/by-id/wwn-0x6006016041e05d00c8b7f0a0d7a4ee11  2.00T
"#;

    const FC_MULTIPATH: &[u8] = br#"
mpathfc (36006016041e05d00c8b7f0a0d7a4ee11) dm-7 DGC,VRAID
size=2.0T features='2 queue_if_no_path pg_init_retries 50' hwhandler='1 alua' wp=rw
|-+- policy='service-time 0' prio=50 status=active
| `- 6:0:2:12 sdd 8:48 active ready running
`-+- policy='service-time 0' prio=10 status=enabled
  `- 7:0:3:12 sde 8:64 failed faulty offline standby
"#;

    const FC_ZONED_LSSCSI_LIST: &[u8] = br#"
[8:0:1:23]   disk    NETAPP   LUN C-Mode      9171  /dev/sdf   /dev/sg8   4.00T
  device_blocked=0
  fabric_name=0x1000000533fedcba
  port_name=0x100000109babcdef
  queue_depth=128
  state=running
  target_port_name=0x500a098299aabb01
[9:0:2:23]   disk    NETAPP   LUN C-Mode      9171  /dev/sdg   /dev/sg9   4.00T
  device_blocked=0
  fabric_name=0x1000000533fedcbb
  port_name=0x100000109babcd00
  queue_depth=128
  state=running
  target_port_name=0x500a098399aabb01
[10:0:3:23]  disk    NETAPP   LUN C-Mode      9171  /dev/sdh   /dev/sg10  4.00T
  device_blocked=0
  fabric_name=0x1000000533fedcba
  port_name=0x100000109babcdef
  queue_depth=128
  state=running
  target_port_name=0x500a098299aabb02
[11:0:4:23]  disk    NETAPP   LUN C-Mode      9171  /dev/sdi   /dev/sg11  4.00T
  device_blocked=1
  fabric_name=0x1000000533fedcbb
  port_name=0x100000109babcd00
  queue_depth=128
  state=blocked
  target_port_name=0x500a098399aabb02
"#;

    const FC_ZONED_LSSCSI_TRANSPORT: &[u8] = br#"
[8:0:1:23]   disk    fc:0x100000109babcdef,0x500a098299aabb01           /dev/sdf   /dev/sg8   /dev/disk/by-id/scsi-3600a098038314f6f2b5d514d43594c33  /dev/disk/by-id/wwn-0x600a098038314f6f2b5d514d43594c33  4.00T
[9:0:2:23]   disk    fc:0x100000109babcd00,0x500a098399aabb01           /dev/sdg   /dev/sg9   /dev/disk/by-id/scsi-3600a098038314f6f2b5d514d43594c33  /dev/disk/by-id/wwn-0x600a098038314f6f2b5d514d43594c33  4.00T
[10:0:3:23]  disk    fc:0x100000109babcdef,0x500a098299aabb02           /dev/sdh   /dev/sg10  /dev/disk/by-id/scsi-3600a098038314f6f2b5d514d43594c33  /dev/disk/by-id/wwn-0x600a098038314f6f2b5d514d43594c33  4.00T
[11:0:4:23]  disk    fc:0x100000109babcd00,0x500a098399aabb02           /dev/sdi   /dev/sg11  /dev/disk/by-id/scsi-3600a098038314f6f2b5d514d43594c33  /dev/disk/by-id/wwn-0x600a098038314f6f2b5d514d43594c33  4.00T
"#;

    const FC_ZONED_LSSCSI_UNIT: &[u8] = br#"
[8:0:1:23]   disk    3600a098038314f6f2b5d514d43594c33                  /dev/sdf   /dev/sg8   /dev/disk/by-id/scsi-3600a098038314f6f2b5d514d43594c33  /dev/disk/by-id/wwn-0x600a098038314f6f2b5d514d43594c33  4.00T
[9:0:2:23]   disk    3600a098038314f6f2b5d514d43594c33                  /dev/sdg   /dev/sg9   /dev/disk/by-id/scsi-3600a098038314f6f2b5d514d43594c33  /dev/disk/by-id/wwn-0x600a098038314f6f2b5d514d43594c33  4.00T
[10:0:3:23]  disk    3600a098038314f6f2b5d514d43594c33                  /dev/sdh   /dev/sg10  /dev/disk/by-id/scsi-3600a098038314f6f2b5d514d43594c33  /dev/disk/by-id/wwn-0x600a098038314f6f2b5d514d43594c33  4.00T
[11:0:4:23]  disk    3600a098038314f6f2b5d514d43594c33                  /dev/sdi   /dev/sg11  /dev/disk/by-id/scsi-3600a098038314f6f2b5d514d43594c33  /dev/disk/by-id/wwn-0x600a098038314f6f2b5d514d43594c33  4.00T
"#;

    const FC_ZONED_MULTIPATH: &[u8] = br#"
mpathfczone (3600a098038314f6f2b5d514d43594c33) dm-11 NETAPP,LUN C-Mode
size=4.0T features='2 queue_if_no_path retain_attached_hw_handler' hwhandler='1 alua' wp=rw
|-+- policy='service-time 0' prio=50 status=active
| |- 8:0:1:23 sdf 8:80 active ready running optimized
| `- 9:0:2:23 sdg 8:96 active ready running optimized
`-+- policy='service-time 0' prio=10 status=enabled
  |- 10:0:3:23 sdh 8:112 active ready running nonoptimized
  `- 11:0:4:23 sdi 8:128 failed faulty offline standby
"#;

    const HARDWARE_ARRAY_LSSCSI_LIST: &[u8] = br#"
[12:0:0:0]   enclosu DELL     PowerVault ME5   1.2   -          /dev/sg12  -
  device_blocked=0
  enclosure_identifier=0x5000c500dead0001
  enclosure_logical_id=ME5-A
  enclosure_slot_count=24
  sas_address=0x5000c500dead0100
  ses_status=ok
  state=running
[12:0:1:0]   enclosu DELL     PowerVault ME5   1.2   -          /dev/sg13  -
  device_blocked=1
  enclosure_identifier=0x5000c500dead0002
  enclosure_logical_id=ME5-B
  enclosure_slot_count=24
  element_descriptor=expander-b
  element_status=critical
  fault_code=over_temperature
  sas_address=0x5000c500dead0200
  ses_status=failed
  state=blocked
[12:0:2:7]   disk    DELL     ME5 VirtualDisk  0520  /dev/sdj   /dev/sg14  8.00T
  array_serial=ME5SN12345
  device_blocked=0
  enclosure_identifier=0x5000c500dead0001
  enclosure_slot=7
  logical_unit_id=600c0ff0005a4bcd0000000000000077
  queue_depth=256
  sas_address=0x5000c500dead0177
  storage_pool=pool-a
  target_port_group=preferred-a
  vendor_lun_id=vdisk-prod-77
  volume_id=vol-prod
  state=running
[13:0:2:7]   disk    DELL     ME5 VirtualDisk  0520  /dev/sdk   /dev/sg15  8.00T
  array_serial=ME5SN12345
  device_blocked=0
  enclosure_identifier=0x5000c500dead0002
  enclosure_slot=7
  logical_unit_id=600c0ff0005a4bcd0000000000000088
  queue_depth=256
  sas_address=0x5000c500dead0277
  storage_pool=pool-a
  target_port_group=nonpreferred-b
  vendor_lun_id=vdisk-prod-77-replaced
  volume_id=vol-prod
  state=running
"#;

    const HARDWARE_ARRAY_LSSCSI_TRANSPORT: &[u8] = br#"
[12:0:2:7]   disk    sas:0x5000c500dead0177                                /dev/sdj   /dev/sg14  /dev/disk/by-id/scsi-3600c0ff0005a4bcd0000000000000077  /dev/disk/by-id/wwn-0x600c0ff0005a4bcd0000000000000077  8.00T
[13:0:2:7]   disk    sas:0x5000c500dead0277                                /dev/sdk   /dev/sg15  /dev/disk/by-id/scsi-3600c0ff0005a4bcd0000000000000088  /dev/disk/by-id/wwn-0x600c0ff0005a4bcd0000000000000088  8.00T
"#;

    const HARDWARE_ARRAY_LSSCSI_UNIT: &[u8] = br#"
[12:0:2:7]   disk    3600c0ff0005a4bcd0000000000000077                    /dev/sdj   /dev/sg14  /dev/disk/by-id/scsi-3600c0ff0005a4bcd0000000000000077  /dev/disk/by-id/wwn-0x600c0ff0005a4bcd0000000000000077  8.00T
[13:0:2:7]   disk    3600c0ff0005a4bcd0000000000000088                    /dev/sdk   /dev/sg15  /dev/disk/by-id/scsi-3600c0ff0005a4bcd0000000000000088  /dev/disk/by-id/wwn-0x600c0ff0005a4bcd0000000000000088  8.00T
"#;

    const HARDWARE_ARRAY_MULTIPATH: &[u8] = br#"
mpatharray (3600c0ff0005a4bcd0000000000000099) dm-14 DELL,ME5 VirtualDisk
size=8.0T features='1 queue_if_no_path' hwhandler='1 alua' wp=rw
|-+- policy='service-time 0' prio=50 status=active
| `- 12:0:2:7 sdj 8:144 active ready running preferred
`-+- policy='service-time 0' prio=10 status=enabled
  `- 13:0:2:7 sdk 8:160 active ready running nonpreferred identity-drift
"#;

    const ENCRYPTED_DEGRADED_MDSTAT: &[u8] = br#"
Personalities : [raid1]
md127 : active raid1 nvme1n1p2[1](F) nvme0n1p2[0]
      2097152 blocks super 1.2 [2/1] [U_]
      [=>...................]  recovery = 8.5% (178257/2097152) finish=3.5min speed=15360K/sec
      bitmap: 1/16 pages [4KB], 65536KB chunk

unused devices: <none>
"#;

    const ENCRYPTED_DEGRADED_CRYPT_STATUS: &[u8] =
        br#"/dev/mapper/cryptraid is active and is in use.
  type:    LUKS2
  cipher:  aes-xts-plain64
  keysize: 512 bits
  key location: keyring
  device:  /dev/md127
  sector size: 4096
  offset:  32768 sectors
  size:    4186112 sectors
  mode:    read/write
  UUID:    luks-raid-uuid
"#;

    const ENCRYPTED_DEGRADED_LUKS_DUMP: &[u8] = br#"
LUKS header information
Version:        2
Epoch:          5
Metadata area:  16384 [bytes]
Keyslots area:  16744448 [bytes]
UUID:           luks-raid-uuid
Label:          encrypted-md-root
Subsystem:      disk-nix-fixture
Flags:          allow-discards

Data segments:
  0: crypt
        offset: 32768 [bytes]
        length: (whole device)
        cipher: aes-xts-plain64
        sector: 4096 [bytes]

Keyslots:
  0: luks2
        Key:        512 bits
        Priority:   normal
        Cipher:     aes-xts-plain64
        Cipher key: 512 bits
        PBKDF:      argon2id
        AF stripes: 4000
        Area offset:32768 [bytes]
        Area length:258048 [bytes]
        Digest ID:  0

Tokens:
  0: systemd-tpm2
        Keyslot:    0
        Keyslots:   0
        TPM2 PCRs:  0+7
        TPM2 Hash:  sha256

Digests:
  0: pbkdf2
        Hash:       sha256
        Iterations: 1000
"#;
