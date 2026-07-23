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
