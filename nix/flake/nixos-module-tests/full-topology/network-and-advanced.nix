{
  services.disk-nix = {
    luks.devices.cryptaction = {
      device = "/dev/disk/by-id/action-luks";
      action = "open";
    };
    nfs.mounts.shared = {
      source = "nas.example.com:/srv/shared";
      mountpoint = "/srv/shared";
      fsType = "nfs4";
      operation = "mount";
      options = [
        "_netdev"
        "x-systemd.automount"
        "vers=4.2"
      ];
      metadata = {
        server = "nas.example.com";
        export = "/srv/shared";
      };
    };
    nfs.mounts."/srv/tuned" = {
      source = "nas.example.com:/srv/tuned";
      fsType = "nfs4";
      operation = "remount";
      options = [
        "_netdev"
        "ro"
        "vers=4.2"
      ];
    };
    nfs.mounts."/srv/action" = {
      source = "nas.example.com:/srv/action";
      fsType = "nfs4";
      action = "remount";
    };
    nfs.mounts."/srv/inventory" = {
      source = "nas.example.com:/srv/inventory";
      fsType = "nfs4";
      operation = "rescan";
    };
    nfs.mounts."/srv/old" = {
      source = "nas.example.com:/srv/old";
      operation = "unmount";
    };
    iscsi = {
      initiatorName = "iqn.2026-06.example:host";
      enableAutoLoginOut = true;
      boot = {
        enable = true;
        target = "iqn.2026-06.example:storage.root";
      };
      sessions."iqn.2026-06.example:storage.root" = {
        operation = "grow";
        desiredSize = "2TiB";
        portal = "192.0.2.10:3260";
      };
      sessions."iqn.2026-06.example:storage.alias" = {
        operation = "grow";
        targetSize = "3TiB";
        portal = "192.0.2.10:3260";
      };
      sessions."iqn.2026-06.example:storage.login" = {
        operation = "login";
        portal = "192.0.2.10:3260";
      };
      sessions."iqn.2026-06.example:storage.logout" = {
        operation = "logout";
        portal = "192.0.2.11:3260";
      };
      sessions."iqn.2026-06.example:storage.rescan" = {
        operation = "rescan";
        portal = "192.0.2.10:3260";
      };
    };
    pools.tank = {
      operation = "rebalance";
      addDevices = [ "/dev/disk/by-id/nvme-replacement" ];
      removeDevices = [ "/dev/disk/by-id/old-disk" ];
      properties.autotrim = "on";
    };
    pools.vault = {
      operation = "import";
      readOnly = true;
    };
    pools.archiveImport = {
      operation = "import";
      readonly = true;
    };
    pools.moveme.operation = "export";
    volumeGroups.importvg.operation = "import";
    volumeGroups.exportvg.operation = "export";
    volumeGroups.activevg.operation = "activate";
    volumeGroups.refreshvg.operation = "rescan";
    volumeGroups.actionvg.action = "rescan";
    partitions.root = {
      operation = "grow";
      device = "/dev/disk/by-id/nvme-root";
      number = "2";
      endOffset = "100%";
    };
    partitions.dataTable = {
      operation = "rescan";
      device = "/dev/disk/by-id/nvme-data";
    };
    vdoVolumes.archiveLifecycle = {
      target = "archive";
      operation = "grow";
      desiredSize = "4TiB";
      physicalSize = "6TiB";
      properties = {
        writePolicy = "sync";
        compression = "enabled";
        deduplication = "disabled";
      };
    };
    vdoVolumes.warmArchive = {
      target = "warm-archive";
      operation = "start";
    };
    vdoVolumes.coldArchive = {
      target = "cold-archive";
      operation = "stop";
    };
    vdoVolumes.refreshArchive = {
      target = "refresh-archive";
      operation = "rescan";
    };
    physicalVolumes.nvmePvGrow = {
      operation = "grow";
      path = "/dev/disk/by-id/nvme-pv-grow";
    };
    physicalVolumes."/dev/disk/by-id/nvme-pv-refresh" = {
      operation = "rescan";
    };
    luksKeyslots."cryptroot:1" = {
      operation = "add-key";
      device = "/dev/disk/by-id/root-luks";
      keySlot = "1";
      newKeyFile = "/run/keys/root-new";
    };
    luksKeyslots."cryptroot:2" = {
      operation = "remove-key";
      device = "/dev/disk/by-id/root-luks";
      keySlot = "2";
    };
    luksKeyslots."cryptroot:3" = {
      operation = "add-key";
      device = "/dev/disk/by-id/root-luks";
      "key-slot" = "3";
      "new-key-file" = "/run/keys/root-new-alias";
    };
    luksKeyslots."cryptroot:4" = {
      operation = "remove-key";
      device = "/dev/disk/by-id/root-luks";
      slot = "4";
    };
    luksTokens."cryptroot:0" = {
      operation = "import-token";
      device = "/dev/disk/by-id/root-luks";
      tokenId = "0";
      tokenFile = "/run/keys/root-token.json";
    };
    luksTokens."cryptroot:1" = {
      operation = "remove-token";
      device = "/dev/disk/by-id/root-luks";
      tokenId = "1";
    };
    luksTokens."cryptroot:2" = {
      operation = "import-token";
      device = "/dev/disk/by-id/root-luks";
      token = "2";
      "token-file" = "/run/keys/root-token-alias.json";
    };
    luksTokens."cryptroot:3" = {
      operation = "remove-token";
      device = "/dev/disk/by-id/root-luks";
      "token-id" = "3";
    };
  };
}
