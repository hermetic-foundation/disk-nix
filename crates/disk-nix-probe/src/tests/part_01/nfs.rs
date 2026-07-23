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
