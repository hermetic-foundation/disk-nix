    use disk_nix_model::{NodeKind, Relationship, StorageGraph};

    use super::*;

    #[test]
    fn empty_probe_result_has_empty_graph_and_reports() {
        let result = ProbeResult::empty();
        assert!(result.graph.nodes.is_empty());
        assert!(result.reports.is_empty());
    }

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

    const CLUSTERED_NVME_LIST: &[u8] = br#"{
      "Devices": [
        {
          "Name": "/dev/nvme2n1",
          "GenericDevice": "/dev/ng2n1",
          "ModelNumber": "Fabric Array Namespace",
          "SerialNumber": "FABRIC123",
          "Firmware": "9.9",
          "Index": 2,
          "Namespace": 1,
          "NSID": 1,
          "NamespaceUUID": "11111111-2222-3333-4444-555555555555",
          "NGUID": "0123456789abcdef0123456789abcdef",
          "SubSystem": "nqn.2014-08.org.nvmexpress:uuid:clustered-array",
          "Controller": "nvme2",
          "Transport": "tcp",
          "Address": "traddr=192.0.2.50,trsvcid=4420",
          "ControllerID": 7,
          "NamespaceSize": 500000000000,
          "NamespaceUsage": 300000000000,
          "NamespaceCapacity": 500000000000,
          "LBAFormat": "0: 512B + 0B",
          "MaximumLBA": 976562500,
          "SectorSize": 512,
          "ANAState": "optimized"
        }
      ]
    }"#;

    const CLUSTERED_NVME_SUBSYSTEMS: &[u8] = br#"{
      "Subsystems": [
        {
          "Name": "nvme-subsys2",
          "NQN": "nqn.2014-08.org.nvmexpress:uuid:clustered-array",
          "HostNQN": "nqn.2014-08.org.nvmexpress:host:node-a",
          "Paths": [
            {
              "Name": "nvme2",
              "Transport": "tcp",
              "Address": "traddr=192.0.2.50,trsvcid=4420",
              "HostTRADDR": "192.0.2.20",
              "HostIface": "ens3f0",
              "State": "live",
              "ANAState": "optimized",
              "Namespaces": [
                {
                  "Name": "/dev/nvme2n1",
                  "NSID": 1
                }
              ]
            },
            {
              "Name": "nvme3",
              "Transport": "tcp",
              "Address": "traddr=192.0.2.51,trsvcid=4420",
              "HostTRADDR": "192.0.2.21",
              "HostIface": "ens3f1",
              "State": "connecting",
              "ANAState": "non-optimized",
              "Namespaces": [
                {
                  "Name": "/dev/nvme2n1",
                  "NSID": 1
                }
              ]
            }
          ]
        }
      ]
    }"#;

    const NVME_TCP_MULTIPATH_LIST: &[u8] = br#"{
      "Devices": [
        {
          "Name": "/dev/nvme4n1",
          "GenericDevice": "/dev/ng4n1",
          "ModelNumber": "NVMe/TCP Array Namespace",
          "SerialNumber": "NVMETCP001",
          "Firmware": "4.2",
          "Index": 4,
          "Namespace": 1,
          "NSID": 1,
          "NamespaceUUID": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
          "NGUID": "aaaaaaaa11111111bbbbbbbb22222222",
          "SubSystem": "nqn.2014-08.org.nvmexpress:uuid:nvme-tcp-array",
          "Controller": "nvme4",
          "Transport": "tcp",
          "Address": "traddr=198.51.100.10,trsvcid=4420,host_traddr=198.51.100.20",
          "ControllerID": 41,
          "NamespaceSize": 800000000000,
          "NamespaceUsage": 640000000000,
          "NamespaceCapacity": 800000000000,
          "LBAFormat": "0: 4096B + 0B",
          "MaximumLBA": 195312500,
          "SectorSize": 4096,
          "ANAState": "optimized"
        }
      ]
    }"#;

    const NVME_TCP_MULTIPATH_SUBSYSTEMS: &[u8] = br#"{
      "Subsystems": [
        {
          "Name": "nvme-subsys4",
          "NQN": "nqn.2014-08.org.nvmexpress:uuid:nvme-tcp-array",
          "HostNQN": "nqn.2014-08.org.nvmexpress:host:multipath-node",
          "Paths": [
            {
              "Name": "nvme4",
              "Transport": "tcp",
              "Address": "traddr=198.51.100.10,trsvcid=4420",
              "HostTRADDR": "198.51.100.20",
              "HostIface": "ens5f0",
              "State": "live",
              "ANAState": "optimized",
              "Namespaces": [
                {
                  "Name": "/dev/nvme4n1",
                  "NSID": 1
                }
              ]
            },
            {
              "Name": "nvme5",
              "Transport": "tcp",
              "Address": "traddr=198.51.100.11,trsvcid=4420",
              "HostTRADDR": "198.51.100.21",
              "HostIface": "ens5f1",
              "State": "reconnecting",
              "ANAState": "inaccessible",
              "Namespaces": [
                {
                  "Name": "/dev/nvme5n1",
                  "NSID": 1
                }
              ]
            }
          ]
        }
      ]
    }"#;

    const NVME_TCP_MULTIPATH: &[u8] = br#"
mpathnvme (uuid.aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee) dm-9 NVME,Array
size=800G features='1 queue_if_no_path' hwhandler='0' wp=rw
|-+- policy='service-time 0' prio=50 status=active
| `- nvme4n1 259:12 active ready running optimized
`-+- policy='service-time 0' prio=1 status=enabled
  `- nvme5n1 259:13 failed faulty offline inaccessible
"#;

    const NVME_OF_MIXED_LIST: &[u8] = br#"{
      "Devices": [
        {
          "Name": "/dev/nvme6n1",
          "GenericDevice": "/dev/ng6n1",
          "ModelNumber": "NVMe-oF Shared Namespace",
          "SerialNumber": "NVMEOF-SHARED",
          "Firmware": "7.1",
          "Index": 6,
          "Namespace": 1,
          "NSID": 1,
          "NamespaceUUID": "bbbbbbbb-cccc-dddd-eeee-ffffffffffff",
          "NGUID": "bbbbbbbb11111111cccccccc22222222",
          "SubSystem": "nqn.2014-08.org.nvmexpress:uuid:mixed-fabric-array",
          "Controller": "nvme6",
          "Transport": "rdma",
          "Address": "traddr=203.0.113.10,trsvcid=4420,host_traddr=203.0.113.20",
          "ControllerID": 61,
          "NamespaceSize": 1200000000000,
          "NamespaceUsage": 900000000000,
          "NamespaceCapacity": 1200000000000,
          "LBAFormat": "0: 4096B + 0B",
          "MaximumLBA": 292968750,
          "SectorSize": 4096,
          "ANAState": "optimized"
        },
        {
          "Name": "/dev/nvme7n1",
          "GenericDevice": "/dev/ng7n1",
          "ModelNumber": "NVMe-oF Shared Namespace",
          "SerialNumber": "NVMEOF-SHARED",
          "Firmware": "7.1",
          "Index": 7,
          "Namespace": 1,
          "NSID": 1,
          "NamespaceUUID": "bbbbbbbb-cccc-dddd-eeee-ffffffffffff",
          "NGUID": "bbbbbbbb11111111cccccccc22222222",
          "SubSystem": "nqn.2014-08.org.nvmexpress:uuid:mixed-fabric-array",
          "Controller": "nvme7",
          "Transport": "fc",
          "Address": "nn-0x200100109babcdef:pn-0x210100109babcdef",
          "ControllerID": 62,
          "NamespaceSize": 1200000000000,
          "NamespaceUsage": 900000000000,
          "NamespaceCapacity": 1200000000000,
          "LBAFormat": "0: 4096B + 0B",
          "MaximumLBA": 292968750,
          "SectorSize": 4096,
          "ANAState": "non-optimized"
        }
      ]
    }"#;

    const NVME_OF_MIXED_SUBSYSTEMS: &[u8] = br#"{
      "Subsystems": [
        {
          "Name": "nvme-subsys-mixed",
          "NQN": "nqn.2014-08.org.nvmexpress:uuid:mixed-fabric-array",
          "HostNQN": "nqn.2014-08.org.nvmexpress:host:mixed-node",
          "Paths": [
            {
              "Name": "nvme6",
              "Transport": "rdma",
              "Address": "traddr=203.0.113.10,trsvcid=4420",
              "TRADDR": "203.0.113.10",
              "TRSVCID": "4420",
              "HostTRADDR": "203.0.113.20",
              "HostIface": "roce0",
              "State": "live",
              "ANAState": "optimized",
              "Namespaces": [
                {
                  "Name": "/dev/nvme6n1",
                  "NSID": 1
                }
              ]
            },
            {
              "Name": "nvme7",
              "Transport": "fc",
              "Address": "nn-0x200100109babcdef:pn-0x210100109babcdef",
              "TRADDR": "nn-0x200100109babcdef:pn-0x210100109babcdef",
              "HostIface": "fc0",
              "State": "reconnecting",
              "ANAState": "non-optimized",
              "Namespaces": [
                {
                  "Name": "/dev/nvme7n1",
                  "NSID": 1
                }
              ]
            },
            {
              "Name": "nvme8",
              "Transport": "rdma",
              "Address": "traddr=203.0.113.11,trsvcid=4420",
              "TRADDR": "203.0.113.11",
              "TRSVCID": "4420",
              "HostTRADDR": "203.0.113.21",
              "HostIface": "roce1",
              "State": "connecting",
              "ANAState": "change",
              "Namespaces": [
                {
                  "Name": "/dev/nvme8n1",
                  "NSID": 1
                }
              ]
            },
            {
              "Name": "nvme9",
              "Transport": "fc",
              "Address": "nn-0x200100109babcd00:pn-0x210100109babcd00",
              "TRADDR": "nn-0x200100109babcd00:pn-0x210100109babcd00",
              "HostIface": "fc1",
              "State": "lost",
              "ANAState": "inaccessible",
              "Namespaces": [
                {
                  "Name": "/dev/nvme9n1",
                  "NSID": 1
                }
              ]
            }
          ]
        }
      ]
    }"#;

    const NVME_OF_MIXED_MULTIPATH: &[u8] = br#"
mpathnvmemixed (uuid.bbbbbbbb-cccc-dddd-eeee-ffffffffffff) dm-12 NVME,Array
size=1.2T features='1 queue_if_no_path' hwhandler='0' wp=rw
|-+- policy='service-time 0' prio=50 status=active
| `- nvme6n1 259:16 active ready running optimized
|-+- policy='service-time 0' prio=20 status=enabled
| `- nvme7n1 259:17 active ready running nonoptimized
`-+- policy='service-time 0' prio=1 status=enabled
  |- nvme8n1 259:18 active ready running change
  `- nvme9n1 259:19 failed faulty offline inaccessible
"#;

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

    const NFS_SERVER_CLIENT_FINDMNT: &[u8] = br#"{
      "filesystems": [
        {
          "target": "/mnt/projects",
          "source": "nas01.example:/exports/projects",
          "fstype": "nfs4",
          "options": "rw,relatime,vers=4.2,sec=krb5p,proto=tcp,local_lock=none",
          "size": 1099511627776,
          "used": 274877906944,
          "avail": 824633720832
        }
      ]
    }"#;

    const NFS_SERVER_CLIENT_NFSSTAT: &[u8] = br#"
nas01.example:/exports/projects mounted on /mnt/projects:
   Flags: rw,relatime,vers=4.2,rsize=1048576,wsize=1048576,namlen=255,hard,proto=tcp,timeo=600,retrans=2,sec=krb5p,clientaddr=10.20.30.40,local_lock=none,addr=10.20.0.10,port=2049,mountaddr=10.20.0.10,mountvers=4,mountproto=tcp,lookupcache=positive,fsc
   Caps: caps=0x3fffdf,wtmult=512,dtsize=32768,bsize=0
   Sec: flavor=390003,pseudoflavor=390003
   Age: 456
"#;

    const NFS_SERVER_CLIENT_EXPORTFS: &[u8] = br#"
/exports/projects
        10.20.0.0/16(rw,sync,no_subtree_check,sec=krb5p,root_squash)
        [2001:db8:120::]/64(ro,sync,no_subtree_check,sec=sys,root_squash)
"#;
