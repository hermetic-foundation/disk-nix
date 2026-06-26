{
  description = "NixOS-native storage topology and lifecycle manager";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    {
      self,
      nixpkgs,
    }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      perSystem = forAllSystems (
        system:
        let
        pkgs = import nixpkgs { inherit system; };
        formatFiles = ''
          find . \
            -path ./.git -prune -o \
            -path ./target -prune -o \
            -path ./build -prune -o \
            -type f -name '*.nix' \
            -print0
        '';
        formatProgram = pkgs.writeShellApplication {
          name = "disk-nix-format";
          runtimeInputs = [
            pkgs.findutils
            pkgs.nixfmt
          ];
          text = ''
            if [ "$#" -gt 0 ]; then
              for file in "$@"; do
                case "$file" in
                  *.nix) nixfmt "$file" ;;
                esac
              done
              exit 0
            fi

            while IFS= read -r -d "" file; do
              case "$file" in
                *.nix) nixfmt "$file" ;;
              esac
            done < <(${formatFiles})
          '';
        };
        diskNix = pkgs.rustPlatform.buildRustPackage {
          pname = "disk-nix";
          version = "0.1.0";
          src = self;
          cargoLock.lockFile = ./Cargo.lock;
          cargoBuildFlags = [
            "-p"
            "disk-nix-cli"
          ];
          cargoTestFlags = [ "--workspace" ];
          postInstall = ''
            install -Dm644 <("$out/bin/disk-nix" completions bash) \
              "$out/share/bash-completion/completions/disk-nix"
            install -Dm644 <("$out/bin/disk-nix" completions zsh) \
              "$out/share/zsh/site-functions/_disk-nix"
            install -Dm644 <("$out/bin/disk-nix" completions fish) \
              "$out/share/fish/vendor_completions.d/disk-nix.fish"
            install -Dm644 <("$out/bin/disk-nix" manpage) \
              "$out/share/man/man1/disk-nix.1"
            install -Dm644 <("$out/bin/disk-nix" schema) \
              "$out/share/disk-nix/schema/disk-nix-spec.schema.json"
          '';
          meta = {
            description = "NixOS-native storage topology and lifecycle manager";
            homepage = "https://github.com/midischwarz12/disk-nix";
            license = pkgs.lib.licenses.agpl3Plus;
            mainProgram = "disk-nix";
          };
        };
        integrationLoopSmoke = pkgs.writeShellApplication {
          name = "disk-nix-integration-loop-smoke";
          runtimeInputs = [
            diskNix
            pkgs.coreutils
            pkgs.e2fsprogs
            pkgs.jq
            pkgs.util-linux
          ];
          text = builtins.readFile ./scripts/integration-loop-smoke.sh;
        };
        integrationBtrfsSmoke = pkgs.writeShellApplication {
          name = "disk-nix-integration-btrfs-smoke";
          runtimeInputs = [
            diskNix
            pkgs.btrfs-progs
            pkgs.coreutils
            pkgs.jq
            pkgs.util-linux
          ];
          text = builtins.readFile ./scripts/integration-btrfs-smoke.sh;
        };
        integrationBcachefsSmoke = pkgs.writeShellApplication {
          name = "disk-nix-integration-bcachefs-smoke";
          runtimeInputs = [
            diskNix
            pkgs.bcachefs-tools
            pkgs.coreutils
            pkgs.jq
            pkgs.util-linux
          ];
          text = builtins.readFile ./scripts/integration-bcachefs-smoke.sh;
        };
        integrationLuksSmoke = pkgs.writeShellApplication {
          name = "disk-nix-integration-luks-smoke";
          runtimeInputs = [
            diskNix
            pkgs.coreutils
            pkgs.cryptsetup
            pkgs.jq
            pkgs.util-linux
          ];
          text = builtins.readFile ./scripts/integration-luks-smoke.sh;
        };
        integrationLvmSmoke = pkgs.writeShellApplication {
          name = "disk-nix-integration-lvm-smoke";
          runtimeInputs = [
            diskNix
            pkgs.coreutils
            pkgs.jq
            pkgs.lvm2
            pkgs.util-linux
          ];
          text = builtins.readFile ./scripts/integration-lvm-smoke.sh;
        };
        integrationMdraidSmoke = pkgs.writeShellApplication {
          name = "disk-nix-integration-mdraid-smoke";
          runtimeInputs = [
            diskNix
            pkgs.coreutils
            pkgs.jq
            pkgs.mdadm
            pkgs.util-linux
          ];
          text = builtins.readFile ./scripts/integration-mdraid-smoke.sh;
        };
        integrationZfsSmoke = pkgs.writeShellApplication {
          name = "disk-nix-integration-zfs-smoke";
          runtimeInputs = [
            diskNix
            pkgs.coreutils
            pkgs.jq
            pkgs.util-linux
            pkgs.zfs
          ];
          text = builtins.readFile ./scripts/integration-zfs-smoke.sh;
        };
        integrationNfsSmoke = pkgs.writeShellApplication {
          name = "disk-nix-integration-nfs-smoke";
          runtimeInputs = [
            diskNix
            pkgs.coreutils
            pkgs.jq
            pkgs.nfs-utils
            pkgs.util-linux
          ];
          text = builtins.readFile ./scripts/integration-nfs-smoke.sh;
        };
        integrationVdoSmoke = pkgs.writeShellApplication {
          name = "disk-nix-integration-vdo-smoke";
          runtimeInputs = [
            diskNix
            pkgs.coreutils
            pkgs.jq
            pkgs.vdo
          ];
          text = builtins.readFile ./scripts/integration-vdo-smoke.sh;
        };
        integrationIscsiSmoke = pkgs.writeShellApplication {
          name = "disk-nix-integration-iscsi-smoke";
          runtimeInputs = [
            diskNix
            pkgs.coreutils
            pkgs.gnugrep
            pkgs.jq
            pkgs.lsscsi
            pkgs.openiscsi
          ];
          text = builtins.readFile ./scripts/integration-iscsi-smoke.sh;
        };
        integrationMultipathSmoke = pkgs.writeShellApplication {
          name = "disk-nix-integration-multipath-smoke";
          runtimeInputs = [
            diskNix
            pkgs.coreutils
            pkgs.jq
            pkgs.lsscsi
            pkgs.multipath-tools
          ];
          text = builtins.readFile ./scripts/integration-multipath-smoke.sh;
        };
        integrationNvmeSmoke = pkgs.writeShellApplication {
          name = "disk-nix-integration-nvme-smoke";
          runtimeInputs = [
            diskNix
            pkgs.coreutils
            pkgs.jq
            pkgs.nvme-cli
          ];
          text = builtins.readFile ./scripts/integration-nvme-smoke.sh;
        };
        integrationFailureRecoverySmoke = pkgs.writeShellApplication {
          name = "disk-nix-integration-failure-recovery-smoke";
          runtimeInputs = [
            diskNix
            pkgs.coreutils
            pkgs.jq
          ];
          text = builtins.readFile ./scripts/integration-failure-recovery-smoke.sh;
        };
        integrationVmSmoke = pkgs.writeShellApplication {
          name = "disk-nix-integration-vm-smoke";
          runtimeInputs = [
            integrationLoopSmoke
            integrationBtrfsSmoke
            integrationBcachefsSmoke
            integrationLuksSmoke
            integrationLvmSmoke
            integrationMdraidSmoke
            integrationZfsSmoke
            integrationNfsSmoke
            integrationVdoSmoke
            integrationIscsiSmoke
            integrationMultipathSmoke
            integrationNvmeSmoke
            integrationFailureRecoverySmoke
            pkgs.systemd
          ];
          text = builtins.readFile ./scripts/integration-vm-smoke.sh;
        };
        integrationVmTest = pkgs.testers.nixosTest {
          name = "disk-nix-integration-vm-test";
          nodes.machine =
            { ... }:
            {
              system.stateVersion = "26.05";
              virtualisation = {
                diskSize = 4096;
                memorySize = 2048;
              };
              boot.kernelModules = [
                "loop"
                "dm_mod"
                "md_mod"
                "raid1"
                "bcachefs"
              ];
              environment.systemPackages = [ integrationVmSmoke ];
            };
          testScript = ''
            machine.start()
            machine.wait_for_unit("multi-user.target")
            machine.succeed("DISK_NIX_INTEGRATION_DESTRUCTIVE=1 disk-nix-integration-vm-smoke")
          '';
        };
        nixosModuleTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            networking.hostId = "8425e349";
            boot.loader.grub.enable = false;
            boot.initrd.systemd.enable = false;
            services.disk-nix = {
              enable = true;
              apply = {
                mode = "activation";
                probeCurrent = true;
                allowDeviceReplacement = true;
                allowRebalance = true;
                allowPotentialDataLoss = false;
                requireBackup = false;
                backupVerified = false;
                requireConfirmation = false;
                confirmation = false;
                requireConfirmationFile = "/run/disk-nix/confirm";
                failOnBlocked = false;
                scriptOut = "/run/disk-nix/apply.sh";
                reportOut = "/run/disk-nix/apply-report.json";
                receiptOut = "/run/disk-nix/apply-receipt.json";
              };
              luks.devices.cryptroot = {
                device = "/dev/disk/by-partuuid/d024c121-4300-4493-a643-055bc4d5caa7";
                operation = "grow";
                desiredSize = "100%";
                allowDiscards = true;
                properties.label = "cryptroot";
                properties."luks.subsystem" = "nixos";
              };
              luks.devices.cryptTargetSize = {
                target = "cryptTargetSizeMapper";
                device = "/dev/disk/by-id/target-size-luks";
                operation = "grow";
                targetSize = "90%";
              };
              luks.devices.cryptSize = {
                device = "/dev/disk/by-id/size-luks";
                operation = "grow";
                size = "80%";
              };
              luks.devices.cryptold = {
                device = "/dev/disk/by-partuuid/old-luks";
                destroy = true;
              };
              luks.devices.cryptarchive = {
                device = "/dev/disk/by-id/archive-luks";
                operation = "open";
              };
              luks.devices.cryptclosed = {
                device = "/dev/disk/by-id/closed-luks";
                operation = "close";
              };
              filesystems.root = {
                device = "/dev/disk/by-label/nixos-root";
                fsType = "xfs";
                mountpoint = "/";
                neededForBoot = true;
                resizePolicy = "grow-only";
                desiredSize = "100%";
              };
              filesystems.data = {
                device = "/dev/disk/by-label/data";
                fsType = "btrfs";
                mountpoint = "/data";
                operation = "rebalance";
                addDevices = [ "/dev/disk/by-id/nvme-btrfs-new" ];
                removeDevices = [ "/dev/disk/by-id/nvme-btrfs-old" ];
                replaceDevices = {
                  "/dev/disk/by-id/nvme-btrfs-aging" = "/dev/disk/by-id/nvme-btrfs-replacement";
                };
                properties = {
                  label = "bulk-data";
                  "btrfs.balance.data" = "usage=50";
                };
                metadata = {
                  pool = "tank";
                  role = "bulk-data";
                };
              };
              filesystems.scratch = {
                device = "/dev/disk/by-label/scratch";
                fsType = "xfs";
                mountpoint = "/scratch";
                operation = "check";
              };
              filesystems.scrub = {
                device = "/dev/disk/by-label/scrub";
                fsType = "btrfs";
                mountpoint = "/scrub";
                operation = "scrub";
              };
              filesystems.trim = {
                device = "/dev/disk/by-label/trim";
                fsType = "xfs";
                mountpoint = "/trim";
                operation = "trim";
              };
              filesystems.remount = {
                device = "/dev/disk/by-label/remount";
                fsType = "xfs";
                mountpoint = "/remount";
                operation = "remount";
                options = [
                  "rw"
                  "noatime"
                  "discard=async"
                ];
              };
              filesystems.localMount = {
                device = "/dev/disk/by-label/local-mount";
                fsType = "xfs";
                mountpoint = "/mnt/local-mount";
                operation = "mount";
                options = [
                  "rw"
                  "noatime"
                ];
              };
              filesystems.localUnmount = {
                device = "/dev/disk/by-label/local-unmount";
                fsType = "ext4";
                mountpoint = "/mnt/local-unmount";
                operation = "unmount";
              };
              filesystems.localRescan = {
                device = "/dev/disk/by-label/local-rescan";
                fsType = "xfs";
                mountpoint = "/mnt/local-rescan";
                operation = "rescan";
              };
              filesystems.actionRescan = {
                device = "/dev/disk/by-label/action-rescan";
                fsType = "xfs";
                mountpoint = "/mnt/action-rescan";
                action = "rescan";
              };
              filesystems.actionUnmount = {
                device = "/dev/disk/by-label/action-unmount";
                fsType = "xfs";
                mountpoint = "/mnt/action-unmount";
                action = "unmount";
              };
              filesystems.teardownOnly = {
                device = "/dev/disk/by-label/teardown-only";
                fsType = "jfs";
                mountpoint = "/mnt/teardown-only";
                operation = "unmount";
              };
              filesystems.destroyed = {
                device = "/dev/disk/by-label/destroyed";
                fsType = "ext4";
                mountpoint = "/mnt/destroyed";
                destroy = true;
              };
              filesystems.targetSizeAlias = {
                device = "/dev/disk/by-label/target-size";
                fsType = "xfs";
                mountpoint = "/mnt/target-size";
                operation = "rescan";
                targetSize = "200GiB";
              };
              filesystems.sizeAlias = {
                device = "/dev/disk/by-label/size-alias";
                fsType = "ext4";
                mountpoint = "/mnt/size-alias";
                operation = "rescan";
                size = "150GiB";
              };
              filesystems.runTmpfs = {
                device = "tmpfs";
                fsType = "tmpfs";
                mountpoint = "/run/disk-nix-tmp";
                options = [
                  "mode=0755"
                  "size=64M"
                  "nosuid"
                  "nodev"
                ];
              };
              filesystems.bindCache = {
                device = "/var/cache/disk-nix";
                fsType = "none";
                mountpoint = "/srv/disk-nix-cache";
                options = [
                  "bind"
                  "x-systemd.requires-mounts-for=/var/cache/disk-nix"
                ];
              };
              filesystems.overlayScratch = {
                device = "overlay";
                fsType = "overlay";
                mountpoint = "/srv/disk-nix-overlay";
                options = [
                  "lowerdir=/nix/store"
                  "upperdir=/var/lib/disk-nix/overlay/upper"
                  "workdir=/var/lib/disk-nix/overlay/work"
                  "index=off"
                ];
              };
              filesystems.mobile = {
                device = "/dev/disk/by-label/mobile";
                fsType = "f2fs";
                mountpoint = "/mobile";
                operation = "check";
              };
              filesystems.bulk = {
                device = "/dev/disk/by-label/bulk";
                fsType = "bcachefs";
                mountpoint = "/bulk";
                operation = "repair";
              };
              swaps.primary = {
                device = "/dev/disk/by-label/swap";
                operation = "format";
                desiredSize = "8GiB";
                priority = 5;
                properties.label = "swap";
                properties."swap.uuid" = "01234567-89ab-cdef-0123-456789abcdef";
              };
              swaps.inventory = {
                device = "/dev/disk/by-label/swap-inventory";
                operation = "rescan";
              };
              swaps.targetSizeAlias = {
                device = "/dev/disk/by-label/swap-target-size";
                operation = "grow";
                targetSize = "12GiB";
              };
              swaps.sizeAlias = {
                device = "/dev/disk/by-label/swap-size";
                operation = "grow";
                size = "10GiB";
              };
              swaps.old = {
                device = "/dev/disk/by-label/old-swap";
                operation = "destroy";
              };
              swaps.actionOld = {
                device = "/dev/disk/by-label/action-old-swap";
                action = "destroy";
              };
              swaps.destroyed = {
                device = "/dev/disk/by-label/destroyed-swap";
                destroy = true;
              };
              zram = {
                enable = true;
                operation = "rescan";
                swapDevices = 2;
                memoryPercent = 40;
                memoryMax = 8589934592;
                priority = 20;
                algorithm = "zstd";
                properties."zram.compression-ratio-target" = "2.0";
              };
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
              btrfsSubvolumes."/mnt/persist/@home" = {
                operation = "create";
                path = "/mnt/persist/@home";
              };
              btrfsSubvolumes."/mnt/persist/@inventory" = {
                operation = "rescan";
                path = "/mnt/persist/@inventory";
              };
              btrfsSubvolumes."/mnt/persist/@old-name" = {
                operation = "rename";
                renameTo = "/mnt/persist/@new-name";
              };
              btrfsQgroups."0/257" = {
                target = "/mnt/persist";
                properties.limit = "25GiB";
              };
              btrfsQgroups."0/258" = {
                operation = "rescan";
                target = "/mnt/persist";
              };
              volumes.scratch = {
                operation = "create";
                target = "vg0/scratch";
                desiredSize = "10GiB";
              };
              volumes."vg0/size-alias" = {
                operation = "create";
                size = "12GiB";
              };
              volumes."vg0/archive".operation = "deactivate";
              volumes."vg0/reporting".operation = "rescan";
              datasets."tank/archive" = {
                destroy = true;
              };
              datasets."tank/home" = {
                operation = "create";
              };
              datasets."tank/legacy" = {
                operation = "rename";
                renameTo = "tank/legacy-staged";
              };
              datasets."tank/legacy-alias" = {
                operation = "rename";
                renameTarget = "tank/legacy-alias-staged";
              };
              datasets."tank/legacy-short" = {
                operation = "rename";
                newName = "tank/legacy-short-staged";
              };
              datasets."tank/home-review" = {
                operation = "promote";
              };
              datasets."tank/inventory" = {
                operation = "rescan";
              };
              zvols."tank/vm/root" = {
                operation = "grow";
                desiredSize = "80GiB";
              };
              zvols."tank/vm/inventory" = {
                operation = "rescan";
              };
              thinPools.primaryPool = {
                operation = "grow";
                path = "vg0/thinpool";
                desiredSize = "500GiB";
              };
              thinPools."vg0/newthin" = {
                operation = "create";
                desiredSize = "100GiB";
              };
              thinPools."vg0/reporting".operation = "rescan";
              lvmSnapshots."vg0/root-snap" = {
                operation = "snapshot";
                target = "vg0/root";
                desiredSize = "20GiB";
              };
              lvmSnapshots."vg0/root-inspect".operation = "rescan";
              lvmCaches."vg0/root" = {
                operation = "create";
                device = "vg0/root-cache";
                properties."lvm.cache-mode" = "writethrough";
              };
              lvmCaches."vg0/archive".operation = "rescan";
              loopDevices.rootImage = {
                operation = "create";
                path = "/dev/loop7";
                device = "/var/lib/images/root.img";
              };
              loopDevices."/dev/loop10".operation = "rescan";
              backingFiles."/var/lib/images/new.img" = {
                operation = "create";
                desiredSize = "8GiB";
              };
              backingFiles."/var/lib/images/root.img" = {
                operation = "grow";
                desiredSize = "16GiB";
              };
              backingFiles.inventoryImage = {
                operation = "rescan";
                path = "/var/lib/images/inventory.img";
              };
              dmMaps.cryptroot = {
                operation = "rescan";
                target = "/dev/mapper/cryptroot";
              };
              dmMaps.cryptswap = {
                operation = "rename";
                target = "/dev/mapper/cryptswap";
                renameTo = "cryptswap-retired";
              };
              dmMaps.oldmap = {
                operation = "destroy";
                target = "/dev/mapper/oldmap";
              };
              mdRaids.root = {
                target = "/dev/md/root";
                raidLevel = "1";
                devices = [
                  "/dev/disk/by-id/nvme-md-a"
                  "/dev/disk/by-id/nvme-md-b"
                ];
                addDevices = [ "/dev/disk/by-id/nvme-md-spare" ];
                replaceDevices = {
                  "/dev/disk/by-id/nvme-md-aging" = "/dev/disk/by-id/nvme-md-replacement";
                };
              };
              mdRaids.existing = {
                target = "/dev/md/existing";
                operation = "assemble";
                devices = [
                  "/dev/disk/by-id/existing-md-a"
                  "/dev/disk/by-id/existing-md-b"
                ];
              };
              mdRaids.oldroot = {
                target = "/dev/md/oldroot";
                operation = "stop";
              };
              mdRaids.inventory.operation = "rescan";
              multipathMaps.mpatha = {
                target = "mpatha";
                addDevices = [ "/dev/sdb" ];
                replaceDevices = {
                  "/dev/sdc" = "/dev/sdd";
                };
              };
              multipathMaps.mpathb = {
                target = "mpathb";
                operation = "rescan";
              };
              multipathMaps.mpathOld = {
                target = "mpath-old";
                operation = "destroy";
              };
              luns."iqn.2026-06.example:storage/root:0" = {
                operation = "grow";
                device = "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0";
                devices = [
                  "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-0"
                ];
                metadata = {
                  target = "iqn.2026-06.example:storage/root";
                  lun = 0;
                };
              };
              luns."iqn.2026-06.example:storage/new:2" = {
                operation = "attach";
                device = "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-2";
              };
              luns."iqn.2026-06.example:storage/old:3" = {
                operation = "detach";
                devices = [
                  "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-3"
                ];
              };
              luns."iqn.2026-06.example:storage/rescan:4" = {
                operation = "rescan";
                paths = [
                  "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-4"
                ];
              };
              nvmeNamespaces.rootNamespace = {
                operation = "create";
                path = "/dev/nvme0";
                desiredSize = "100G";
                namespaceId = "4";
                controllers = "0x1";
              };
              nvmeNamespaces."/dev/nvme1".operation = "rescan";
              nvmeNamespaces."/dev/nvme2" = {
                operation = "attach";
                nsid = "7";
                controllerId = "0x2";
              };
              nvmeNamespaces."/dev/nvme3" = {
                operation = "detach";
                namespaceId = "8";
                controller = "0x3";
              };
              exports.share = {
                operation = "export";
                path = "/srv/share";
                client = "192.0.2.0/24";
                options = "rw,sync,no_subtree_check";
              };
              exports."/srv/inventory".operation = "rescan";
              exports."/srv/old-share" = {
                operation = "unexport";
                client = "192.0.2.55";
              };
              caches."tank/l2arc0" = {
                operation = "replace-device";
                replaceDevices = {
                  "/dev/disk/by-id/old-cache" = "/dev/disk/by-id/new-cache";
                };
                cacheSetUuid = "11111111-2222-3333-4444-555555555555";
              };
              caches."/dev/bcache0" = {
                operation = "rescan";
                addDevices = [ "cache-set-uuid" ];
                cacheSetUuid = "cache-set-uuid";
                properties."bcache.cache-mode" = "writethrough";
                properties."bcache.set-journal-delay-ms" = "100";
              };
              snapshots."tank/home@before-upgrade" = {
                target = "tank/home";
                hold = "disk-nix-retain";
                rollback = true;
                cloneTo = "tank/home-review";
                renameTo = "tank/home@before-upgrade-retained";
                recursiveRollback = true;
              };
              snapshots."tank/home@clone-only" = {
                operation = "clone";
                target = "tank/home";
                cloneTo = "tank/home-clone";
              };
              snapshots."tank/home@action-rescan" = {
                action = "rescan";
                target = "tank/home";
              };
              snapshots.aliases = {
                operation = "clone";
                target = "tank/home";
                "snapshot-path" = "tank/home@alias-source";
                cloneTarget = "tank/home-alias-clone";
                clone = "tank/home-short-clone";
                renameTarget = "tank/home@alias-retained";
                newName = "tank/home@alias-new";
                recursive = true;
                "zfs.rollbackRecursive" = true;
                readonly = true;
              };
              snapshots."tank/home@old" = {
                target = "tank/home";
                releaseHold = "old-retention";
              };
              snapshots."/mnt/persist/@home-before-upgrade" = {
                target = "/mnt/persist/@home";
                readOnly = true;
              };
              snapshots."/mnt/persist/@home-before-clone" = {
                target = "/mnt/persist/@home";
                cloneTo = "/mnt/persist/@home-review";
                readOnly = true;
              };
              snapshots."tank/home@inventory" = {
                operation = "rescan";
                target = "tank/home";
              };
              snapshots."/mnt/persist/@home-inventory" = {
                operation = "rescan";
                target = "/mnt/persist/@home";
                readOnly = true;
              };
              snapshots.home-before-friendly = {
                operation = "rescan";
                target = "/mnt/persist/@home";
                snapshotPath = "/mnt/persist/@home-before-friendly";
              };
            };
          }
        ];
        zramTuningOnlyModuleTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              zram = {
                swapDevices = 3;
                memoryPercent = 35;
                priority = 15;
                algorithm = "lz4";
                preserveData = false;
              };
            };
          }
        ];
        nixosModuleExecuteTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              apply = {
                mode = "activation";
                execute = true;
                probeCurrent = true;
                failOnBlocked = true;
                scriptOut = "/run/disk-nix/execute.sh";
                reportOut = "/run/disk-nix/execute-report.json";
                receiptOut = "/run/disk-nix/execute-receipt.json";
              };
            };
          }
        ];
        nixosModuleHandoffAutoImportTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              apply = {
                mode = "activation";
                execute = true;
                failOnBlocked = true;
                reportOut = "/run/disk-nix/handoff-report.json";
                declarativeHandoff.autoImport = {
                  enable = true;
                  configurationPath = "/etc/nixos/storage.nix";
                  backupDirectory = "/var/backups/disk-nix-handoff";
                };
              };
            };
          }
        ];
        nixosModuleBootModeTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              apply.mode = "boot";
            };
          }
        ];
        nixosModuleInstallModeTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              apply.mode = "install";
            };
          }
        ];
        nixosModuleCollisionTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              filesystems.local = {
                device = "/dev/disk/by-label/local";
                fsType = "xfs";
                mountpoint = "/srv/collision";
              };
              filesystems.secondary = {
                device = "/dev/disk/by-label/secondary";
                fsType = "ext4";
                mountpoint = "/srv/collision";
              };
              swaps.primary.path = "/dev/disk/by-label/swap-collision";
              swaps.secondary.target = "/dev/disk/by-label/swap-collision";
              luks.devices.primary = {
                target = "crypt-collision";
                device = "/dev/disk/by-id/primary-luks";
              };
              luks.devices.secondary = {
                mapper = "crypt-collision";
                device = "/dev/disk/by-id/secondary-luks";
              };
              exports.primary = {
                path = "/srv/export-collision";
                client = "192.0.2.0/24";
                options = "rw,sync";
              };
              exports.secondary = {
                target = "/srv/export-collision";
                client = "192.0.2.0/24";
                options = "ro,sync";
              };
            };
          }
        ];
        nixosModuleDiskCollisionTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              disks."/dev/disk/by-id/nvme-root".operation = "rescan";
              disks.rootAlias = {
                path = "/dev/disk/by-id/nvme-root";
                operation = "grow";
              };
            };
          }
        ];
        nixosModulePartitionCollisionTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              partitions.root = {
                device = "/dev/disk/by-id/nvme-root";
                number = "2";
                operation = "grow";
              };
              partitions.rootAlias = {
                device = "/dev/disk/by-id/nvme-root";
                partitionNumber = "2";
                operation = "rescan";
              };
            };
          }
        ];
        nixosModuleLuksKeyslotCollisionTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              luksKeyslots.rootAdd = {
                operation = "add-key";
                device = "/dev/disk/by-id/root-luks";
                keySlot = "4";
                newKeyFile = "/run/keys/root-new";
              };
              luksKeyslots.rootRotate = {
                device = "/dev/disk/by-id/root-luks";
                "key-slot" = "4";
                "key-file" = "/run/keys/root-old";
                properties.keyFile = "/run/keys/root-rotated";
              };
            };
          }
        ];
        nixosModuleLuksTokenCollisionTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              luksTokens.rootImport = {
                operation = "import-token";
                device = "/dev/disk/by-id/root-luks";
                tokenId = "7";
                tokenFile = "/run/keys/root-token.json";
              };
              luksTokens.rootRotate = {
                device = "/dev/disk/by-id/root-luks";
                "token-id" = "7";
                properties.tokenFile = "/run/keys/root-token-rotated.json";
              };
            };
          }
        ];
        nixosModuleBackingFileCollisionTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              backingFiles.rootImage = {
                operation = "rescan";
                path = "/var/lib/images/root.img";
              };
              backingFiles.duplicateRootImage = {
                operation = "grow";
                target = "/var/lib/images/root.img";
                desiredSize = "16GiB";
              };
            };
          }
        ];
        nixosModuleBtrfsSubvolumeCollisionTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              btrfsSubvolumes."/mnt/persist/@home".operation = "rescan";
              btrfsSubvolumes.homeAlias = {
                path = "/mnt/persist/@home";
                operation = "create";
              };
            };
          }
        ];
        nixosModuleBtrfsQgroupCollisionTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              btrfsQgroups."0/257" = {
                target = "/mnt/persist";
                operation = "rescan";
              };
              btrfsQgroups.homeLimit = {
                target = "0/257";
                path = "/mnt/persist";
                properties.limit = "25GiB";
              };
            };
          }
        ];
        nixosModuleDmMapCollisionTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              dmMaps.cryptroot = {
                operation = "rescan";
                target = "/dev/mapper/cryptroot";
              };
              dmMaps.duplicateCryptroot = {
                operation = "rescan";
                path = "/dev/mapper/cryptroot";
              };
            };
          }
        ];
        nixosModuleVdoVolumeCollisionTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              vdoVolumes.archive.operation = "rescan";
              vdoVolumes.archiveAlias = {
                target = "archive";
                operation = "grow";
                desiredSize = "4TiB";
              };
            };
          }
        ];
        nixosModulePhysicalVolumeCollisionTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              physicalVolumes."/dev/disk/by-id/nvme-pv".operation = "rescan";
              physicalVolumes.nvmeAlias = {
                path = "/dev/disk/by-id/nvme-pv";
                operation = "grow";
              };
            };
          }
        ];
        nixosModuleLoopDeviceCollisionTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              loopDevices."/dev/loop7".operation = "rescan";
              loopDevices.rootImage = {
                target = "/dev/loop7";
                operation = "create";
                device = "/var/lib/images/root.img";
              };
            };
          }
        ];
        nixosModuleMdRaidCollisionTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              mdRaids."/dev/md/root" = {
                operation = "assemble";
                devices = [
                  "/dev/disk/by-id/md-a"
                  "/dev/disk/by-id/md-b"
                ];
              };
              mdRaids.rootAlias = {
                target = "/dev/md/root";
                operation = "rescan";
              };
            };
          }
        ];
        nixosModuleMultipathMapCollisionTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              multipathMaps.mpatha = {
                operation = "rescan";
              };
              multipathMaps.primaryPath = {
                target = "mpatha";
                operation = "grow";
              };
            };
          }
        ];
        nixosModuleNvmeNamespaceCollisionTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              nvmeNamespaces.root = {
                path = "/dev/nvme0";
                namespaceId = "4";
                operation = "rescan";
              };
              nvmeNamespaces.rootAlias = {
                target = "/dev/nvme0";
                nsid = "4";
                operation = "grow";
              };
            };
          }
        ];
        nixosModuleCacheCollisionTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              caches."/dev/bcache0".operation = "rescan";
              caches.writeback = {
                target = "/dev/bcache0";
                operation = "add-device";
                addDevices = [ "cache-set-uuid" ];
              };
            };
          }
        ];
        nixosModulePoolCollisionTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              pools.tank.operation = "rescan";
              pools.primaryPool = {
                target = "tank";
                operation = "import";
              };
            };
          }
        ];
        nixosModuleDatasetCollisionTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              datasets."tank/home".operation = "rescan";
              datasets.homeAlias = {
                target = "tank/home";
                operation = "create";
              };
            };
          }
        ];
        nixosModuleZvolCollisionTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              zvols."tank/vm/root".operation = "rescan";
              zvols.vmRootAlias = {
                path = "tank/vm/root";
                operation = "grow";
                desiredSize = "80GiB";
              };
            };
          }
        ];
        nixosModuleVolumeGroupCollisionTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              volumeGroups.vg0.operation = "rescan";
              volumeGroups.primaryVg = {
                target = "vg0";
                operation = "activate";
              };
            };
          }
        ];
        nixosModuleVolumeCollisionTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              volumes."vg0/root".operation = "rescan";
              volumes.rootAlias = {
                path = "vg0/root";
                operation = "grow";
                desiredSize = "80GiB";
              };
            };
          }
        ];
        nixosModuleThinPoolCollisionTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              thinPools."vg0/thinpool".operation = "rescan";
              thinPools.primaryThin = {
                target = "vg0/thinpool";
                operation = "grow";
                desiredSize = "500GiB";
              };
            };
          }
        ];
        nixosModuleLvmCacheCollisionTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              lvmCaches."vg0/root".operation = "rescan";
              lvmCaches.rootCacheAlias = {
                target = "vg0/root";
                operation = "create";
                device = "vg0/root-cache";
              };
            };
          }
        ];
        nixosModuleSnapshotCollisionTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              snapshots."/mnt/persist/@home-before" = {
                target = "/mnt/persist/@home";
                readOnly = true;
              };
              snapshots.homeBeforeAlias = {
                target = "/mnt/persist/@home";
                snapshotPath = "/mnt/persist/@home-before";
                operation = "rescan";
              };
            };
          }
        ];
        nixosModuleIscsiSessionCollisionTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              iscsi.sessions."iqn.2026-06.example:storage.root" = {
                portal = "192.0.2.10:3260";
                operation = "rescan";
              };
              iscsi.sessions.rootAlias = {
                target = "iqn.2026-06.example:storage.root";
                portal = "192.0.2.11:3260";
                operation = "login";
              };
            };
          }
        ];
        nixosModuleLunPathCollisionTest = pkgs.nixos [
          self.nixosModules.default
          {
            system.stateVersion = "26.05";
            boot.loader.grub.enable = false;
            services.disk-nix = {
              enable = true;
              luns.rootPrimary = {
                operation = "rescan";
                device = "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0";
              };
              luns.rootMultipath = {
                operation = "attach";
                paths = [
                  "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0"
                ];
              };
            };
          }
        ];
      in
      {
        formatter = formatProgram;

        packages = {
          default = diskNix;
          disk-nix = diskNix;
          integration-bcachefs-smoke = integrationBcachefsSmoke;
          integration-btrfs-smoke = integrationBtrfsSmoke;
          integration-luks-smoke = integrationLuksSmoke;
          integration-lvm-smoke = integrationLvmSmoke;
          integration-mdraid-smoke = integrationMdraidSmoke;
          integration-zfs-smoke = integrationZfsSmoke;
          integration-nfs-smoke = integrationNfsSmoke;
          integration-vdo-smoke = integrationVdoSmoke;
          integration-iscsi-smoke = integrationIscsiSmoke;
          integration-multipath-smoke = integrationMultipathSmoke;
          integration-nvme-smoke = integrationNvmeSmoke;
          integration-failure-recovery-smoke = integrationFailureRecoverySmoke;
          integration-vm-smoke = integrationVmSmoke;
          integration-vm-test = integrationVmTest;
          integration-loop-smoke = integrationLoopSmoke;
        };

        apps = {
          default = {
            type = "app";
            program = "${diskNix}/bin/disk-nix";
            meta = diskNix.meta;
          };
          integration-loop-smoke = {
            type = "app";
            program = "${integrationLoopSmoke}/bin/disk-nix-integration-loop-smoke";
            meta = {
              description = "Root-only loop-backed disk-nix smoke integration harness";
            };
          };
          integration-btrfs-smoke = {
            type = "app";
            program = "${integrationBtrfsSmoke}/bin/disk-nix-integration-btrfs-smoke";
            meta = {
              description = "Root-only Btrfs loop-backed disk-nix smoke integration harness";
            };
          };
          integration-bcachefs-smoke = {
            type = "app";
            program = "${integrationBcachefsSmoke}/bin/disk-nix-integration-bcachefs-smoke";
            meta = {
              description = "Root-only bcachefs loop-backed disk-nix smoke integration harness";
            };
          };
          integration-luks-smoke = {
            type = "app";
            program = "${integrationLuksSmoke}/bin/disk-nix-integration-luks-smoke";
            meta = {
              description = "Root-only LUKS loop-backed disk-nix smoke integration harness";
            };
          };
          integration-lvm-smoke = {
            type = "app";
            program = "${integrationLvmSmoke}/bin/disk-nix-integration-lvm-smoke";
            meta = {
              description = "Root-only LVM loop-backed disk-nix smoke integration harness";
            };
          };
          integration-mdraid-smoke = {
            type = "app";
            program = "${integrationMdraidSmoke}/bin/disk-nix-integration-mdraid-smoke";
            meta = {
              description = "Root-only MD RAID loop-backed disk-nix smoke integration harness";
            };
          };
          integration-zfs-smoke = {
            type = "app";
            program = "${integrationZfsSmoke}/bin/disk-nix-integration-zfs-smoke";
            meta = {
              description = "Root-only ZFS loop-backed disk-nix smoke integration harness";
            };
          };
          integration-nfs-smoke = {
            type = "app";
            program = "${integrationNfsSmoke}/bin/disk-nix-integration-nfs-smoke";
            meta = {
              description = "Root-only NFS client disk-nix smoke integration harness";
            };
          };
          integration-vdo-smoke = {
            type = "app";
            program = "${integrationVdoSmoke}/bin/disk-nix-integration-vdo-smoke";
            meta = {
              description = "Root-only VDO disk-nix smoke integration harness";
            };
          };
          integration-iscsi-smoke = {
            type = "app";
            program = "${integrationIscsiSmoke}/bin/disk-nix-integration-iscsi-smoke";
            meta = {
              description = "Root-only iSCSI session disk-nix smoke integration harness";
            };
          };
          integration-multipath-smoke = {
            type = "app";
            program = "${integrationMultipathSmoke}/bin/disk-nix-integration-multipath-smoke";
            meta = {
              description = "Root-only multipath map disk-nix smoke integration harness";
            };
          };
          integration-nvme-smoke = {
            type = "app";
            program = "${integrationNvmeSmoke}/bin/disk-nix-integration-nvme-smoke";
            meta = {
              description = "Root-only NVMe namespace disk-nix smoke integration harness";
            };
          };
          integration-failure-recovery-smoke = {
            type = "app";
            program = "${integrationFailureRecoverySmoke}/bin/disk-nix-integration-failure-recovery-smoke";
            meta = {
              description = "Synthetic failed-apply disk-nix partial recovery smoke integration harness";
            };
          };
          integration-vm-smoke = {
            type = "app";
            program = "${integrationVmSmoke}/bin/disk-nix-integration-vm-smoke";
            meta = {
              description = "VM-only destructive disk-nix integration suite orchestrator";
            };
          };
        };

        checks = {
          inherit diskNix;
          clippy = pkgs.rustPlatform.buildRustPackage {
            pname = "disk-nix-clippy";
            version = "0.1.0";
            src = self;
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = [ pkgs.clippy ];
            buildPhase = ''
              runHook preBuild
              cargo clippy --workspace --all-targets --offline -- -D warnings
              runHook postBuild
            '';
            doCheck = false;
            installPhase = ''
              runHook preInstall
              touch "$out"
              runHook postInstall
            '';
          };
          integrationLoopSmoke = pkgs.runCommand "disk-nix-integration-loop-smoke-check" { } ''
            ${pkgs.bash}/bin/bash -n ${./scripts/integration-loop-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${./scripts/integration-loop-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'losetup --find --show' ${./scripts/integration-loop-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'losetup --set-capacity' ${./scripts/integration-loop-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'mkfs.ext4' ${./scripts/integration-loop-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'resize2fs' ${./scripts/integration-loop-smoke.sh}
            touch "$out"
          '';
          integrationBtrfsSmoke = pkgs.runCommand "disk-nix-integration-btrfs-smoke-check" { } ''
            ${pkgs.bash}/bin/bash -n ${./scripts/integration-btrfs-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${./scripts/integration-btrfs-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'losetup --find --show' ${./scripts/integration-btrfs-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'mkfs.btrfs' ${./scripts/integration-btrfs-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'mount -t btrfs' ${./scripts/integration-btrfs-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'btrfs", "scrub", "start", "-B"' ${./scripts/integration-btrfs-smoke.sh}
            touch "$out"
          '';
          integrationBcachefsSmoke = pkgs.runCommand "disk-nix-integration-bcachefs-smoke-check" { } ''
            ${pkgs.bash}/bin/bash -n ${./scripts/integration-bcachefs-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${./scripts/integration-bcachefs-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'losetup --find --show' ${./scripts/integration-bcachefs-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'bcachefs format' ${./scripts/integration-bcachefs-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'mount -t bcachefs' ${./scripts/integration-bcachefs-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'bcachefs", "scrub"' ${./scripts/integration-bcachefs-smoke.sh}
            touch "$out"
          '';
          integrationLuksSmoke = pkgs.runCommand "disk-nix-integration-luks-smoke-check" { } ''
            ${pkgs.bash}/bin/bash -n ${./scripts/integration-luks-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${./scripts/integration-luks-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'losetup --find --show' ${./scripts/integration-luks-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'cryptsetup luksFormat' ${./scripts/integration-luks-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'cryptsetup open' ${./scripts/integration-luks-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'cryptsetup", "close"' ${./scripts/integration-luks-smoke.sh}
            touch "$out"
          '';
          integrationLvmSmoke = pkgs.runCommand "disk-nix-integration-lvm-smoke-check" { } ''
            ${pkgs.bash}/bin/bash -n ${./scripts/integration-lvm-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${./scripts/integration-lvm-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'losetup --find --show' ${./scripts/integration-lvm-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'pvcreate --force --yes' ${./scripts/integration-lvm-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'vgcreate' ${./scripts/integration-lvm-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'lvcreate --yes --type thin-pool' ${./scripts/integration-lvm-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'lvcreate --yes --snapshot' ${./scripts/integration-lvm-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'vgchange", "--refresh"' ${./scripts/integration-lvm-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'thinpools:" + $thinpool + ":rescan' ${./scripts/integration-lvm-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'lvmsnapshots:" + $snapshot + ":rescan' ${./scripts/integration-lvm-smoke.sh}
            touch "$out"
          '';
          integrationMdraidSmoke = pkgs.runCommand "disk-nix-integration-mdraid-smoke-check" { } ''
            ${pkgs.bash}/bin/bash -n ${./scripts/integration-mdraid-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${./scripts/integration-mdraid-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'losetup --find --show' ${./scripts/integration-mdraid-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'mdadm --create' ${./scripts/integration-mdraid-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'mdadm", "--detail", "--scan"' ${./scripts/integration-mdraid-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'mdadm", "--examine", "--scan"' ${./scripts/integration-mdraid-smoke.sh}
            touch "$out"
          '';
          integrationZfsSmoke = pkgs.runCommand "disk-nix-integration-zfs-smoke-check" { } ''
            ${pkgs.bash}/bin/bash -n ${./scripts/integration-zfs-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${./scripts/integration-zfs-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'losetup --find --show' ${./scripts/integration-zfs-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'zpool create' ${./scripts/integration-zfs-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'zpool destroy' ${./scripts/integration-zfs-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'zpool", "scrub"' ${./scripts/integration-zfs-smoke.sh}
            touch "$out"
          '';
          integrationNfsSmoke = pkgs.runCommand "disk-nix-integration-nfs-smoke-check" { } ''
            ${pkgs.bash}/bin/bash -n ${./scripts/integration-nfs-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${./scripts/integration-nfs-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q DISK_NIX_NFS_SOURCE ${./scripts/integration-nfs-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'mount -t "$fs_type"' ${./scripts/integration-nfs-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'findmnt", "--json"' ${./scripts/integration-nfs-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'nfsstat", "-m"' ${./scripts/integration-nfs-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'mount", "-o", ("remount,"' ${./scripts/integration-nfs-smoke.sh}
            touch "$out"
          '';
          integrationVdoSmoke = pkgs.runCommand "disk-nix-integration-vdo-smoke-check" { } ''
            ${pkgs.bash}/bin/bash -n ${./scripts/integration-vdo-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${./scripts/integration-vdo-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q DISK_NIX_VDO_NAME ${./scripts/integration-vdo-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'vdo status --name' ${./scripts/integration-vdo-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'vdostats --human-readable' ${./scripts/integration-vdo-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'vdo", "status", "--name"' ${./scripts/integration-vdo-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'vdostats", "--human-readable"' ${./scripts/integration-vdo-smoke.sh}
            touch "$out"
          '';
          integrationIscsiSmoke = pkgs.runCommand "disk-nix-integration-iscsi-smoke-check" { } ''
            ${pkgs.bash}/bin/bash -n ${./scripts/integration-iscsi-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${./scripts/integration-iscsi-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q DISK_NIX_ISCSI_TARGET ${./scripts/integration-iscsi-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'iscsiadm --mode session' ${./scripts/integration-iscsi-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'lsscsi -t -s' ${./scripts/integration-iscsi-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'iscsiadm", "--mode", "session", "--rescan"' ${./scripts/integration-iscsi-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'lsscsi", "-t", "-s"' ${./scripts/integration-iscsi-smoke.sh}
            touch "$out"
          '';
          integrationMultipathSmoke = pkgs.runCommand "disk-nix-integration-multipath-smoke-check" { } ''
            ${pkgs.bash}/bin/bash -n ${./scripts/integration-multipath-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${./scripts/integration-multipath-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q DISK_NIX_MULTIPATH_MAP ${./scripts/integration-multipath-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'multipath -ll' ${./scripts/integration-multipath-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'lsscsi -t -s' ${./scripts/integration-multipath-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'multipath", "-ll"' ${./scripts/integration-multipath-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'multipath", "-r"' ${./scripts/integration-multipath-smoke.sh}
            touch "$out"
          '';
          integrationNvmeSmoke = pkgs.runCommand "disk-nix-integration-nvme-smoke-check" { } ''
            ${pkgs.bash}/bin/bash -n ${./scripts/integration-nvme-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${./scripts/integration-nvme-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q DISK_NIX_NVME_CONTROLLER ${./scripts/integration-nvme-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'nvme list-ns' ${./scripts/integration-nvme-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'nvme list-subsys' ${./scripts/integration-nvme-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'nvme", "list-ns"' ${./scripts/integration-nvme-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'nvme", "ns-rescan"' ${./scripts/integration-nvme-smoke.sh}
            touch "$out"
          '';
          integrationFailureRecoverySmoke =
            pkgs.runCommand "disk-nix-integration-failure-recovery-smoke-check" { }
              ''
                ${pkgs.bash}/bin/bash -n ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'fake_tools/lvs' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'fake-swap-label-tools' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'fake-dm-rename-tools' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'fake-zfs-dataset-rename-tools' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'fake-btrfs-snapshot-clone-tools' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'fake-lvm-vg-rename-tools' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'fake-rollback-tools' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'fake-nvme-create-tools' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'fake-nvme-grow-tools' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'fake-nvme-attach-tools' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'fake-nvme-detach-tools' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'fake-nvme-tools' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-lio-tools' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'fake-multipath-replace-tools' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'fake-iscsi-tools' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'fake-iscsi-login-tools' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'fake-lvm-cache-attach-tools' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'fake-lvm-cache-detach-tools' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'fake-vdo-property-tools' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'fake-lvm-cache-tools' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q partialExecutionRecovery ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic resize failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic swap label failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic device-mapper rename failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic ZFS dataset rename failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic Btrfs snapshot clone failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic LVM VG rename failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic zfs rollback failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic nvme namespace create failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic nvme namespace grow rescan failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic nvme namespace attach failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic nvme namespace detach failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic nvme namespace delete failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN LIO create failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN tgt create failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic multipath replace delete failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic MD RAID replace failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic LUKS open failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic partition grow failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic NFS remount failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic iscsi logout failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic iscsi login failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic lvm cache attach failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic lvm cache detach failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic VDO grow failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic VDO property failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic bcache property failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'synthetic lvm cache property failure' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'snapshot:tank/home@before:rollback' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'nvmenamespaces:/dev/nvme0:create' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'nvmenamespaces:/dev/nvme1:grow' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'nvmenamespaces:/dev/nvme2:attach' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'nvmenamespaces:/dev/nvme3:detach' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'nvmenamespaces:/dev/nvme4:destroy' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:storage.root:create' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:tgt.root:create' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'multipathMaps:root-map:replace-device:/dev/sdc' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'mdRaids:root:replace-device:/dev/disk/by-id/old-md-member' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'luks.devices:cryptarchive:open' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'partitions:root:grow' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'nfs.mounts:/srv/tuned:remount' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'iscsisessions:iqn.2026-06.example:storage.old:logout' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'iscsisessions:iqn.2026-06.example:storage.root:login' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'lvmCaches:vg0/root:add-device:vg0/root-cache' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'lvmCaches:vg0/root:remove-device:vg0/root-cache' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'vdovolumes:archive:grow' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'vdoVolumes:archive:set-property:writePolicy' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'caches:writeback-cache:set-property:bcache.cache-mode' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'lvmCaches:vg0/root:set-property:lvm.cache-mode' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'completedMutatingCommandCount' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'swaps:primary:set-property:label' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'dmmaps:cryptswap:rename' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'datasets:tank/home:rename' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'snapshot:/mnt/persist/@home-before:clone:/mnt/persist/@home-review' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'volumegroups:vg-old:rename' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'roll-forward-review' ${./scripts/integration-failure-recovery-smoke.sh}
                ${pkgs.gnugrep}/bin/grep -q 'rollback-review' ${./scripts/integration-failure-recovery-smoke.sh}
                touch "$out"
              '';
          integrationVmSmoke = pkgs.runCommand "disk-nix-integration-vm-smoke-check" { } ''
            ${pkgs.bash}/bin/bash -n ${./scripts/integration-vm-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${./scripts/integration-vm-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_ASSUME_VM ${./scripts/integration-vm-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'systemd-detect-virt --quiet --vm' ${./scripts/integration-vm-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'disk-nix-integration-loop-smoke' ${./scripts/integration-vm-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'disk-nix-integration-bcachefs-smoke' ${./scripts/integration-vm-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'disk-nix-integration-mdraid-smoke' ${./scripts/integration-vm-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'disk-nix-integration-zfs-smoke' ${./scripts/integration-vm-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'disk-nix-integration-nfs-smoke' ${./scripts/integration-vm-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'disk-nix-integration-vdo-smoke' ${./scripts/integration-vm-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'disk-nix-integration-iscsi-smoke' ${./scripts/integration-vm-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'disk-nix-integration-multipath-smoke' ${./scripts/integration-vm-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'disk-nix-integration-nvme-smoke' ${./scripts/integration-vm-smoke.sh}
            ${pkgs.gnugrep}/bin/grep -q 'disk-nix-integration-failure-recovery-smoke' ${./scripts/integration-vm-smoke.sh}
            touch "$out"
          '';
          documentation = pkgs.runCommand "disk-nix-documentation-check" { } ''
            checklist=${./docs/feature-checklist.md}
            ${pkgs.gnugrep}/bin/grep -q 'docs/feature-checklist.md' ${./README.md}
            ${pkgs.gnugrep}/bin/grep -q 'docs/operator-runbooks.md' ${./README.md}
            ${pkgs.gnugrep}/bin/grep -q 'feature-checklist.md' ${./docs/status.md}
            ${pkgs.gnugrep}/bin/grep -q 'operator-runbooks.md' ${./docs/status.md}
            ${pkgs.gnugrep}/bin/grep -q 'Status labels:' "$checklist"
            ${pkgs.gnugrep}/bin/grep -q 'Update rules:' "$checklist"
            ${pkgs.gnugrep}/bin/grep -q '\*\*Finished:\*\*' "$checklist"
            ${pkgs.gnugrep}/bin/grep -q '\*\*Partial:\*\*' "$checklist"
            ${pkgs.gnugrep}/bin/grep -q '\*\*Desired:\*\*' "$checklist"
            ${pkgs.gnugrep}/bin/grep -q 'Operator runbooks for high-risk workflows' "$checklist"
            ${pkgs.gnugrep}/bin/grep -q 'reconciliationGroups' "$checklist"
            ${pkgs.gnugrep}/bin/grep -q 'reconciliationGroups' ${./docs/planning.md}
            ${pkgs.gnugrep}/bin/grep -q 'partiallySuppressed' ${./docs/cli.md}
            runbooks=${./docs/operator-runbooks.md}
            for runbook in \
              "Device replacement" \
              Rollback \
              "Failed apply recovery" \
              "Degraded arrays and pools" \
              "Shared storage and network storage" \
              "Change record"
            do
              ${pkgs.gnugrep}/bin/grep -q "^## $runbook$" "$runbooks"
            done
            for section in \
              Foundation \
              "Read-only storage awareness" \
              "Planning and apply safety" \
              "Lifecycle operations" \
              "Current-topology reconciliation" \
              "Recovery guidance" \
              "NixOS integration" \
              "Testing and proof" \
              Documentation
            do
              ${pkgs.gnugrep}/bin/grep -q "^## $section$" "$checklist"
            done
            touch "$out"
          '';
          examples = pkgs.runCommand "disk-nix-examples-check" { nativeBuildInputs = [ pkgs.jq ]; } ''
            simplePlan=$(mktemp)
            lifecyclePlan=$(mktemp)
            simpleApply=$(mktemp)
            lifecycleApply=$(mktemp)
            lifecycleValidate=$(mktemp)
            lifecycleApplyReport=$(mktemp)
            lifecycleValidateReport=$(mktemp)
            emptySpec=$(mktemp)
            emptyExecute=$(mktemp)
            legacySpec=$(mktemp)
            legacyMigration=$(mktemp)
            preflightStatus=$(mktemp)
            schema=$(mktemp)
            scriptOut=$(mktemp)

            ${diskNix}/bin/disk-nix --help | grep -- 'usage'
            ${diskNix}/bin/disk-nix --help | grep -- 'encryption'
            ${diskNix}/bin/disk-nix --help | grep -- 'complex-filesystems'
            ${diskNix}/bin/disk-nix --help | grep -- 'zfs'
            ${diskNix}/bin/disk-nix --help | grep -- 'cache'
            ${diskNix}/bin/disk-nix --help | grep -- 'lvm'
            ${diskNix}/bin/disk-nix --help | grep -- 'vdo'
            ${diskNix}/bin/disk-nix --help | grep -- 'multipath'
            ${diskNix}/bin/disk-nix --help | grep -- 'nvme'
            ${diskNix}/bin/disk-nix --help | grep -- 'raid'
            ${diskNix}/bin/disk-nix --help | grep -- 'loop'
            ${diskNix}/bin/disk-nix --help | grep -- 'swap'
            ${diskNix}/bin/disk-nix --help | grep -- 'iscsi'
            ${diskNix}/bin/disk-nix --help | grep -- 'nfs'
            ${diskNix}/bin/disk-nix probe-status --help | grep -- '--preflight'
            ${diskNix}/bin/disk-nix probe-status --preflight --json > "$preflightStatus"
            jq -e '
              (.environment | has("toolVersions"))
              and (.preflightChecks | has("status"))
              and (.preflightChecks | has("root"))
              and (.preflightChecks | has("unavailableToolCount"))
              and (.preflightChecks | has("failedToolCount"))
              and (.preflightChecks.missingTools | type == "array")
              and (.preflightChecks.failedTools | type == "array")
              and (.preflightChecks.remediation | type == "array")
              and (.preflightChecks.adapterRemediation | type == "array")
              and (.preflightChecks.adapterRemediation | any(.adapter == "nvme-id-ns" and .canonicalAdapter == "nvme" and (.nixPackages | index("pkgs.nvme-cli") != null)))
              and (.preflightChecks.adapterRemediation | any(.adapter == "mdadm-scan" and .canonicalAdapter == "mdraid" and (.nixPackages | index("pkgs.mdadm") != null)))
              and (.preflightChecks.adapterRemediation | any(.adapter == "zramctl" and .canonicalAdapter == "zram" and (.tools | index("zramctl") != null)))
              and (.reports | type == "array")
            ' "$preflightStatus"
            if grep -R -E 'executor-unavailable|does not mutate storage yet|future mutating executor|future `btrfs device remove`|does not run mutating storage commands directly|non-executed command' ${./README.md} ${./docs}; then
              echo "stale executor documentation found" >&2
              exit 1
            fi
            ${diskNix}/bin/disk-nix schema > "$schema"
            cmp "$schema" ${diskNix}/share/disk-nix/schema/disk-nix-spec.schema.json
            cat > "$legacySpec" <<'EOF'
            {
              "fileSystems": {
                "root": {
                  "mountpoint": "/",
                  "fsType": "ext4"
                }
              },
              "swapDevices": {
                "swap": {
                  "device": "/dev/disk/by-label/swap",
                  "operation": "rescan"
                }
              },
              "luksDevices": {
                "cryptroot": {
                  "device": "/dev/disk/by-id/luks-root",
                  "operation": "open"
                }
              },
              "nfsMounts": {
                "/srv/shared": {
                  "source": "nas.example.com:/srv/shared",
                  "operation": "mount"
                }
              },
              "iscsiSessions": {
                "iqn.2026-06.example:storage.root": {
                  "portal": "192.0.2.10:3260",
                  "operation": "login"
                }
              }
            }
            EOF
            ${diskNix}/bin/disk-nix migrate --spec "$legacySpec" --json > "$legacyMigration"
            jq -e '
              .targetVersion == 1
              and .migrated == true
              and .spec.version == 1
              and (.spec | has("fileSystems") | not)
              and (.spec | has("swapDevices") | not)
              and (.spec | has("luksDevices") | not)
              and (.spec | has("nfsMounts") | not)
              and (.spec | has("iscsiSessions") | not)
              and .spec.filesystems.root.mountpoint == "/"
              and .spec.swaps.swap.operation == "rescan"
              and .spec.luks.devices.cryptroot.operation == "open"
              and .spec.nfs.mounts."/srv/shared".source == "nas.example.com:/srv/shared"
              and .spec.iscsi.sessions."iqn.2026-06.example:storage.root".operation == "login"
              and (.changes | any(. == "mapped legacy field fileSystems to filesystems"))
              and (.changes | any(. == "mapped legacy field luksDevices to luks.devices"))
              and (.legacyMappings | any(.source == "fileSystems" and .target == "filesystems" and .scope == "top-level"))
              and (.legacyMappings | any(.source == "spec.fileSystems" and .target == "spec.filesystems" and .scope == "spec"))
              and (.legacyMappings | any(.source == "iscsiSessions" and .target == "iscsi.sessions" and .scope == "top-level"))
              and (.appliedMappings | length == 5)
              and (.appliedMappings | any(.source == "fileSystems" and .target == "filesystems" and .scope == "top-level"))
              and (.appliedMappings | any(.source == "luksDevices" and .target == "luks.devices" and .scope == "top-level"))
              and (.appliedMappings | any(.source == "iscsiSessions" and .target == "iscsi.sessions" and .scope == "top-level"))
            ' "$legacyMigration"
            jq -e '
              ."$schema" == "https://json-schema.org/draft/2020-12/schema"
              and .properties.version.const == 1
              and .properties.spec["$ref"] == "#/$defs/specBody"
              and .properties.apply["$ref"] == "#/$defs/applyPolicy"
              and .properties.swaps["$ref"] == "#/$defs/lifecycleMap"
              and .properties.zram["$ref"] == "#/$defs/zramSpec"
              and ."$defs".specBody.properties.version.const == 1
              and ."$defs".specBody.properties.zram["$ref"] == "#/$defs/zramSpec"
              and ."$defs".zramSpec.properties.operation["$ref"] == "#/$defs/operation"
              and ."$defs".zramSpec.properties.swapDevices.minimum == 1
              and .properties.luks["$ref"] == "#/$defs/luksSpec"
              and .properties.nfs["$ref"] == "#/$defs/nfsSpec"
              and .properties.iscsi["$ref"] == "#/$defs/iscsiSpec"
              and .properties.disks["$ref"] == "#/$defs/lifecycleMap"
              and .properties.partitions["$ref"] == "#/$defs/lifecycleMap"
              and .properties.btrfsSubvolumes["$ref"] == "#/$defs/lifecycleMap"
              and .properties.btrfsQgroups["$ref"] == "#/$defs/lifecycleMap"
              and .properties.vdoVolumes["$ref"] == "#/$defs/lifecycleMap"
              and .properties.physicalVolumes["$ref"] == "#/$defs/lifecycleMap"
              and .properties.luksKeyslots["$ref"] == "#/$defs/lifecycleMap"
              and .properties.luksTokens["$ref"] == "#/$defs/lifecycleMap"
              and .properties.volumes["$ref"] == "#/$defs/lifecycleMap"
              and .properties.volumeGroups["$ref"] == "#/$defs/lifecycleMap"
              and .properties.zvols["$ref"] == "#/$defs/lifecycleMap"
              and .properties.thinPools["$ref"] == "#/$defs/lifecycleMap"
              and .properties.lvmSnapshots["$ref"] == "#/$defs/lifecycleMap"
              and .properties.lvmCaches["$ref"] == "#/$defs/lifecycleMap"
              and .properties.loopDevices["$ref"] == "#/$defs/lifecycleMap"
              and .properties.backingFiles["$ref"] == "#/$defs/lifecycleMap"
              and .properties.dmMaps["$ref"] == "#/$defs/lifecycleMap"
              and .properties.mdRaids["$ref"] == "#/$defs/lifecycleMap"
              and .properties.multipathMaps["$ref"] == "#/$defs/lifecycleMap"
              and .properties.pools["$ref"] == "#/$defs/lifecycleMap"
              and .properties.datasets["$ref"] == "#/$defs/lifecycleMap"
              and .properties.luns["$ref"] == "#/$defs/lifecycleMap"
              and .properties.nvmeNamespaces["$ref"] == "#/$defs/lifecycleMap"
              and .properties.iscsiSessions["$ref"] == "#/$defs/lifecycleMap"
              and .properties.exports["$ref"] == "#/$defs/lifecycleMap"
              and .properties.caches["$ref"] == "#/$defs/lifecycleMap"
              and .properties.snapshots["$ref"] == "#/$defs/snapshotMap"
              and ."$defs".lifecycleObject.properties.physicalSize.type == ["string", "number"]
              and ."$defs".lifecycleObject.properties.vdoPhysicalSize.type == ["string", "number"]
              and ."$defs".lifecycleObject.properties.provider.type == "string"
              and ."$defs".lifecycleObject.properties.storageProvider.type == "string"
              and ."$defs".lifecycleObject.properties.arrayProvider.type == "string"
              and ."$defs".lifecycleObject.properties.lun.type == ["string", "number"]
              and ."$defs".snapshot.properties.operation["$ref"] == "#/$defs/operation"
              and ."$defs".snapshot.properties.action["$ref"] == "#/$defs/operation"
              and (."$defs".operation.enum | index("grow") != null)
              and (."$defs".operation.enum | index("check") != null)
              and (."$defs".operation.enum | index("repair") != null)
              and (."$defs".operation.enum | index("scrub") != null)
              and (."$defs".operation.enum | index("trim") != null)
              and (."$defs".operation.enum | index("rescan") != null)
              and (."$defs".operation.enum | index("replace-device") != null)
              and (."$defs".operation.enum | index("add-key") != null)
              and (."$defs".operation.enum | index("remove-key") != null)
              and (."$defs".operation.enum | index("import-token") != null)
                    and (."$defs".operation.enum | index("remove-token") != null)
                    and (."$defs".operation.enum | index("clone") != null)
                    and (."$defs".specBody.properties.luks["$ref"] == "#/$defs/luksSpec")
              and (."$defs".specBody.properties.nfs["$ref"] == "#/$defs/nfsSpec")
              and (."$defs".specBody.properties.iscsi["$ref"] == "#/$defs/iscsiSpec")
              and (."$defs".specBody.properties.disks["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.btrfsSubvolumes["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.btrfsQgroups["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.vdoVolumes["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.physicalVolumes["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.luksKeyslots["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.luksTokens["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.volumes["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.volumeGroups["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.zvols["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.thinPools["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.lvmSnapshots["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.lvmCaches["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.loopDevices["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.backingFiles["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.dmMaps["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.mdRaids["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.multipathMaps["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.pools["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.datasets["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.luns["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.targetLuns["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.nvmeNamespaces["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.iscsiSessions["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.exports["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.caches["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.snapshots["$ref"] == "#/$defs/snapshotMap")
              and ."$defs".snapshot.properties.operation["$ref"] == "#/$defs/operation"
              and ."$defs".snapshot.properties.action["$ref"] == "#/$defs/operation"
              and ."$defs".snapshot.properties.path.type == "string"
              and ."$defs".snapshot.properties.snapshotPath.type == "string"
              and ."$defs".snapshot.properties.readOnly.type == "boolean"
              and ."$defs".snapshot.properties.readonly.type == "boolean"
              and ."$defs".snapshot.properties.cloneTo.type == "string"
              and ."$defs".snapshot.properties.recursiveRollback.type == "boolean"
              and ."$defs".snapshot.properties."zfs.rollbackRecursive".type == "boolean"
              and (."$defs".operation.enum | index("promote") != null)
              and (."$defs".operation.enum | index("import") != null)
              and (."$defs".operation.enum | index("export") != null)
              and (."$defs".operation.enum | index("unexport") != null)
              and (."$defs".operation.enum | index("attach") != null)
              and (."$defs".operation.enum | index("detach") != null)
              and (."$defs".operation.enum | index("activate") != null)
              and (."$defs".operation.enum | index("deactivate") != null)
              and (."$defs".operation.enum | index("assemble") != null)
              and (."$defs".operation.enum | index("start") != null)
              and (."$defs".operation.enum | index("stop") != null)
              and (."$defs".operation.enum | index("login") != null)
              and (."$defs".operation.enum | index("logout") != null)
              and (."$defs".operation.enum | index("open") != null)
              and (."$defs".operation.enum | index("close") != null)
              and (."$defs".operation.enum | index("mount") != null)
              and (."$defs".operation.enum | index("unmount") != null)
              and (."$defs".operation.enum | index("remount") != null)
              and ."$defs".filesystem.properties.device.type == "string"
              and ."$defs".filesystem.properties.operation["$ref"] == "#/$defs/operation"
              and ."$defs".filesystem.properties.action["$ref"] == "#/$defs/operation"
              and ."$defs".filesystem.properties.neededForBoot.type == "boolean"
              and ."$defs".filesystem.properties.destroy.type == "boolean"
              and ."$defs".filesystem.properties.properties.type == "object"
              and ."$defs".filesystem.properties.metadata.type == "object"
              and ."$defs".filesystem.properties.addDevices.type == "array"
              and ."$defs".filesystem.properties.removeDevices.type == "array"
              and ."$defs".filesystem.properties.replaceDevices.type == "object"
              and ."$defs".lifecycleObject.properties.cacheSetUuid.type == "string"
              and ."$defs".lifecycleObject.properties.cacheSetUUID.type == "string"
              and ."$defs".lifecycleObject.properties."cache-set-uuid".type == "string"
              and ."$defs".lifecycleObject.properties.cache_set_uuid.type == "string"
              and ."$defs".luksSpec.properties.devices["$ref"] == "#/$defs/lifecycleMap"
              and ."$defs".nfsSpec.properties.mounts["$ref"] == "#/$defs/nfsMountMap"
              and ."$defs".nfsMount.properties.source.type == "string"
              and ."$defs".nfsMount.properties.operation["$ref"] == "#/$defs/operation"
              and ."$defs".nfsMount.properties.action["$ref"] == "#/$defs/operation"
              and ."$defs".nfsMount.properties.destroy.type == "boolean"
              and ."$defs".nfsMount.properties.options.type == "array"
              and ."$defs".nfsMount.properties.metadata.type == "object"
              and ."$defs".iscsiSpec.properties.sessions["$ref"] == "#/$defs/lifecycleMap"
              and ."$defs".iscsiSpec.properties.boot["$ref"] == "#/$defs/iscsiBoot"
              and ."$defs".iscsiBoot.properties.loginAll.type == "boolean"
              and (."$defs".iscsiBoot.properties.extraConfig.type | index("null") != null)
              and ."$defs".lifecycleObject.properties.action["$ref"] == "#/$defs/operation"
              and ."$defs".lifecycleObject.properties.renameTo.type == "string"
              and ."$defs".lifecycleObject.properties.renameTarget.type == "string"
              and ."$defs".lifecycleObject.properties.newName.type == "string"
              and ."$defs".lifecycleObject.properties.readOnly.type == "boolean"
              and ."$defs".lifecycleObject.properties.readonly.type == "boolean"
              and ."$defs".lifecycleObject.properties.partitionType.type == "string"
              and (."$defs".lifecycleObject.properties.partitionNumber.type | index("string") != null)
              and (."$defs".lifecycleObject.properties.partitionNumber.type | index("number") != null)
              and (."$defs".lifecycleObject.properties.number.type | index("string") != null)
              and (."$defs".lifecycleObject.properties.startOffset.type | index("number") != null)
              and (."$defs".lifecycleObject.properties.endOffset.type | index("string") != null)
              and ."$defs".lifecycleObject.properties.level.type == "string"
              and ."$defs".lifecycleObject.properties.raidLevel.type == "string"
              and ."$defs".lifecycleObject.properties.devices.type == "array"
              and ."$defs".lifecycleObject.properties.paths.type == "array"
              and ."$defs".lifecycleObject.properties.devicePaths.type == "array"
              and ."$defs".lifecycleObject.properties.path.type == "string"
              and ."$defs".lifecycleObject.properties.client.type == "string"
              and ."$defs".lifecycleObject.properties.portal.type == "string"
              and (."$defs".lifecycleObject.properties.namespaceId.type | index("string") != null)
              and (."$defs".lifecycleObject.properties.nsid.type | index("string") != null)
              and ."$defs".lifecycleObject.properties.controllers.type == "string"
              and (."$defs".lifecycleObject.properties.controllerId.type | index("string") != null)
              and (."$defs".lifecycleObject.properties.controller.type | index("string") != null)
              and (."$defs".lifecycleObject.properties.keySlot.type | index("string") != null)
              and (."$defs".lifecycleObject.properties."key-slot".type | index("string") != null)
              and (."$defs".lifecycleObject.properties.slot.type | index("string") != null)
              and ."$defs".lifecycleObject.properties.keyFile.type == "string"
              and ."$defs".lifecycleObject.properties."key-file".type == "string"
              and ."$defs".lifecycleObject.properties.currentKeyFile.type == "string"
              and ."$defs".lifecycleObject.properties.newKeyFile.type == "string"
              and ."$defs".lifecycleObject.properties."new-key-file".type == "string"
              and (."$defs".lifecycleObject.properties.tokenId.type | index("string") != null)
              and (."$defs".lifecycleObject.properties."token-id".type | index("string") != null)
              and (."$defs".lifecycleObject.properties.token.type | index("string") != null)
              and ."$defs".lifecycleObject.properties.tokenFile.type == "string"
              and ."$defs".lifecycleObject.properties."token-file".type == "string"
              and ."$defs".lifecycleObject.properties.jsonFile.type == "string"
              and ."$defs".lifecycleObject.properties.options.type == "string"
              and ."$defs".applyPolicy.properties.failOnBlocked.default == true
              and ."$defs".applyPolicy.properties.allowPotentialDataLoss.default == false
              and (."$defs".applyPolicy.properties.reportOut.type | index("string") != null)
              and (."$defs".applyPolicy.properties.receiptOut.type | index("string") != null)
            ' "$schema"

            ${diskNix}/bin/disk-nix plan --spec ${./examples/simple-root.json} --json > "$simplePlan"
            jq -e '
              .summary.actionCount == 1
              and .summary.offlineRequiredCount == 0
              and .summary.destructiveCount == 0
              and .summary.potentialDataLossCount == 0
              and .summary.unsupportedCount == 0
              and .actions[0].id == "filesystem:root:grow"
              and .dependencyOrder[0].actionId == "filesystem:root:grow"
              and .dependencyOrder[0].phase == "mutate-in-place"
              and .dependencyOrder[0].direction == "lower-layers-first"
              and .dependencyOrder[0].layerRank == 90
              and .actions[0].operation == "grow"
              and .actions[0].risk == "online"
              and .actions[0].context.desiredSize == "100%"
            ' "$simplePlan"

            ${diskNix}/bin/disk-nix plan --spec ${./examples/lifecycle-update.json} --json > "$lifecyclePlan"
            jq -e '
              .summary.actionCount == 105
              and (.dependencyOrder | length) == .summary.actionCount
              and (.dependencyOrder | any(.actionId == "datasets:tank/home:create" and (.unblocks | index("snapshot:tank/home@before-upgrade:create") != null)))
              and (.dependencyOrder | any(.actionId == "snapshot:tank/home@before-upgrade:create" and (.dependsOn | index("datasets:tank/home:create") != null)))
              and (.dependencyOrder | any(.actionId == "btrfssubvolumes:/mnt/persist/@home:create" and (.unblocks | index("snapshot:/mnt/persist/@home-inventory:rescan") != null)))
              and (.dependencyOrder | any(.actionId == "snapshot:/mnt/persist/@home-inventory:rescan" and (.dependsOn | index("btrfssubvolumes:/mnt/persist/@home:create") != null)))
              and .summary.offlineRequiredCount == 31
              and .summary.destructiveCount == 4
              and .summary.potentialDataLossCount == 4
              and .summary.unsupportedCount == 0
              and (.actions | any(.id == "filesystems:home-check:check" and .risk == "offline-required"))
              and (.actions | any(.id == "filesystems:data-scrub:scrub" and .risk == "online"))
              and (.actions | any(.id == "filesystems:scratch-trim:trim" and .risk == "online"))
              and (.actions | any(.id == "filesystems:scratch-remount:remount" and .risk == "online"))
              and (.actions | any(.id == "btrfssubvolumes:/mnt/persist/@home:create" and .risk == "online"))
              and (.actions | any(.id == "btrfssubvolumes:/mnt/persist/@inventory:rescan" and .risk == "online"))
              and (.actions | any(.id == "btrfssubvolumes:/mnt/persist/@old-name:rename" and .risk == "offline-required"))
              and (.actions | any(.id == "btrfsQgroups:0/257:set-property:limit" and .risk == "safe"))
              and (.actions | any(.id == "btrfsQgroups:0/257:set-property:maxExclusive" and .risk == "safe"))
              and (.actions | any(.id == "btrfsqgroups:0/258:rescan" and .risk == "online"))
              and (.actions | any(.id == "volumes:vg0/scratch:create" and .risk == "online"))
              and (.actions | any(.id == "volumes:vg0/archive:deactivate" and .risk == "offline-required"))
              and (.actions | any(.id == "volumes:vg0/reporting:rescan" and .risk == "online"))
              and (.actions | any(.id == "vdovolumes:archive:grow" and .risk == "online"))
              and (.actions | any(.id == "vdovolumes:warmarchive:start" and .risk == "offline-required"))
              and (.actions | any(.id == "vdovolumes:coldarchive:stop" and .risk == "offline-required"))
              and (.actions | any(.id == "vdoVolumes:archive:set-property:writePolicy" and .risk == "safe"))
              and (.actions | any(.id == "vdoVolumes:archive:set-property:compression" and .risk == "safe"))
              and (.actions | any(.id == "vdoVolumes:archive:set-property:deduplication" and .risk == "safe"))
              and (.actions | any(.id == "physicalvolumes:/dev/disk/by-id/nvme-pv-grow:grow" and .risk == "online"))
              and (.actions | any(.id == "lukskeyslots:cryptroot:1:add-key" and .risk == "offline-required"))
              and (.actions | any(.id == "lukskeyslots:cryptroot:2:remove-key" and .risk == "potential-data-loss"))
              and (.actions | any(.id == "lukstokens:cryptroot:0:import-token" and .risk == "offline-required"))
              and (.actions | any(.id == "lukstokens:cryptroot:1:remove-token" and .risk == "potential-data-loss"))
              and (.actions | any(.id == "iscsisessions:iqn.2026-06.example:storage.login:login" and .risk == "online"))
              and (.actions | any(.id == "iscsisessions:iqn.2026-06.example:storage.logout:logout" and .risk == "offline-required"))
              and (.actions | any(.id == "iscsisessions:iqn.2026-06.example:storage.rescan:rescan" and .risk == "online"))
              and (.actions | any(.id == "luns:iqn.2026-06.example:storage/new:2:attach" and .risk == "online"))
              and (.actions | any(.id == "luns:iqn.2026-06.example:storage/old:3:detach" and .risk == "offline-required"))
              and (.actions | any(.id == "luns:iqn.2026-06.example:storage/rescan:4:rescan" and .risk == "online"))
              and (.actions | any(.id == "zvols:tank/vm/root:grow" and .risk == "online"))
              and (.actions | any(.id == "zvols:tank/vm/inventory:rescan" and .risk == "online"))
              and (.actions | any(.id == "thinpools:vg0/thinpool:grow" and .risk == "online"))
              and (.actions | any(.id == "thinpools:vg0/newthin:create" and .risk == "online"))
              and (.actions | any(.id == "thinpools:vg0/reporting:rescan" and .risk == "online"))
              and (.actions | any(.id == "lvmsnapshots:vg0/root-snap:snapshot" and .risk == "reversible"))
              and (.actions | any(.id == "lvmsnapshots:vg0/root-inspect:rescan" and .risk == "online"))
              and (.actions | any(.id == "lvmcaches:vg0/root:create" and .risk == "offline-required"))
              and (.actions | any(.id == "lvmCaches:vg0/root:set-property:lvm.cache-mode" and .risk == "safe"))
              and (.actions | any(.id == "lvmcaches:vg0/archive:rescan" and .risk == "online"))
              and (.actions | any(.id == "loopdevices:/dev/loop7:create" and .risk == "online"))
              and (.actions | any(.id == "loopdevices:/dev/loop10:rescan" and .risk == "online"))
              and (.actions | any(.id == "backingfiles:/var/lib/images/new.img:create" and .risk == "online"))
              and (.actions | any(.id == "backingfiles:/var/lib/images/root.img:grow" and .risk == "online"))
              and (.actions | any(.id == "backingfiles:inventory-image:rescan" and .risk == "online"))
              and (.actions | any(.id == "mdraids:existing:assemble" and .risk == "offline-required"))
              and (.actions | any(.id == "mdraids:oldroot:stop" and .risk == "offline-required"))
              and (.actions | any(.id == "mdRaids:root:add-device:/dev/disk/by-id/nvme-md-spare" and .risk == "online"))
              and (.actions | any(.id == "multipathMaps:mpatha:add-device:/dev/sdb" and .risk == "online"))
              and (.actions | any(.id == "multipathmaps:mpathb:rescan" and .risk == "online"))
              and (.actions | any(.id == "multipathmaps:mpathold:destroy" and .risk == "offline-required"))
              and (.actions | any(.id == "partitions:root:grow" and .risk == "offline-required"))
              and (.actions | any(.id == "partitions:data-table:rescan" and .risk == "online"))
              and (.actions | any(.id == "swaps:primary:format" and .risk == "destructive"))
              and (.actions | any(.id == "swaps:inventory:rescan" and .risk == "online"))
              and (.actions | any(.id == "swaps:retired:deactivate" and .risk == "offline-required"))
              and (.actions | any(.id == "swaps:remove:destroy" and .risk == "destructive"))
              and (.actions | any(.id == "luks.devices:cryptroot:grow" and .risk == "offline-required"))
              and (.actions | any(.id == "luks.devices:cryptarchive:open" and .risk == "offline-required"))
              and (.actions | any(.id == "luks.devices:cryptclosed:close" and .risk == "offline-required"))
              and (.actions | any(.id == "nvmenamespaces:/dev/nvme0:create" and .risk == "destructive"))
              and (.actions | any(.id == "nvmenamespaces:/dev/nvme1:rescan" and .risk == "online"))
              and (.actions | any(.id == "pools:vault:import" and .risk == "offline-required" and .context.readOnly == true))
              and (.actions | any(.id == "pools:moveme:export" and .risk == "offline-required"))
              and (.actions | any(.id == "volumegroups:importvg:import" and .risk == "offline-required"))
              and (.actions | any(.id == "volumegroups:exportvg:export" and .risk == "offline-required"))
              and (.actions | any(.id == "volumegroups:activevg:activate" and .risk == "offline-required"))
              and (.actions | any(.id == "datasets:tank/home:create" and .risk == "online"))
              and (.actions | any(.id == "datasets:tank/inventory:rescan" and .risk == "online"))
              and (.actions | any(.id == "datasets:tank/home-review:promote" and .risk == "offline-required"))
              and (.actions | any(.id == "datasets:tank/legacy:rename" and .risk == "offline-required"))
              and (.actions | any(.id == "datasets:tank/archive:destroy"))
              and (.actions | any(.id == "snapshot:tank/home@before-prune:rename:tank/home@retained" and .risk == "offline-required"))
              and (.actions | any(.id == "snapshot:/mnt/persist/@home-before-clone:clone:/mnt/persist/@home-review" and .risk == "reversible" and .context.readOnly == true))
              and (.actions | any(.id == "snapshot:tank/root@rollback-point:rollback"))
              and (.actions | any(.id == "snapshot:tank/home@inventory:rescan" and .risk == "online"))
              and (.actions | any(.id == "snapshot:/mnt/persist/@home-inventory:rescan" and .risk == "online"))
              and (.actions | any(.id == "exports:/srv/share:export" and .risk == "online"))
              and (.actions | any(.id == "exports:/srv/inventory:rescan" and .risk == "online"))
              and (.actions | any(.id == "exports:/srv/old-share:unexport" and .risk == "offline-required"))
              and (.actions | any(.id == "nfs.mounts:/srv/shared:mount" and .risk == "online"))
              and (.actions | any(.id == "nfs.mounts:/srv/inventory:rescan" and .risk == "online"))
              and (.actions | any(.id == "nfs.mounts:/srv/tuned:remount" and .risk == "online"))
              and (.actions | any(.id == "nfs.mounts:/srv/old:unmount" and .risk == "offline-required"))
              and (.actions | any(.id == "caches:/dev/bcache0:add-device:cache-set-uuid" and .risk == "online"))
              and (.actions | any(.id == "caches:/dev/bcache0:rescan" and .risk == "online"))
              and (.actions | any(.id == "caches:/dev/bcache0:set-property:bcache.cache-mode" and .risk == "safe"))
              and (.actions | any(.id == "caches:/dev/bcache0:set-property:bcache.set-journal-delay-ms" and .risk == "safe"))
              and (.actions | any(.id == "caches:tank/l2arc0:replace-device:/dev/disk/by-id/old-cache"))
            ' "$lifecyclePlan"

            ${diskNix}/bin/disk-nix apply --spec ${./examples/simple-root.json} --script-out "$scriptOut" --json > "$simpleApply"
            jq -e '
              .status == "dry-run"
              and .apply.blockedCount == 0
              and .commandSummary.commandCount == 2
              and .commandSummary.needsDesiredSizeCount == 0
              and .verificationSummary.stepCount == 1
            ' "$simpleApply"
            test -x "$scriptOut"
            grep -- 'xfs_growfs /' "$scriptOut"
            grep -- 'Post-apply verification commands' "$scriptOut"

            printf '%s\n' '{"spec":{},"apply":{}}' > "$emptySpec"
            ${diskNix}/bin/disk-nix apply --spec "$emptySpec" --execute --json > "$emptyExecute"
            jq -e '
              .status == "succeeded"
              and .apply.blockedCount == 0
              and .commandSummary.commandCount == 0
              and .verificationSummary.commandCount == 0
              and (.executionResults | length) == 0
            ' "$emptyExecute"

            failingToolDir="$TMPDIR/failing-tools"
            mkdir -p "$failingToolDir"
            cat > "$failingToolDir/truncate" <<'EOF'
            #!${pkgs.bash}/bin/bash
            echo "synthetic truncate failure for disk-nix report coverage" >&2
            exit 73
            EOF
            chmod +x "$failingToolDir/truncate"
            failingSpec="$TMPDIR/failing-apply.json"
            failingApply="$TMPDIR/failing-apply.out"
            failingApplyReport="$TMPDIR/failing-apply-report.json"
            failingApplyReceipt="$TMPDIR/failing-apply-receipt.json"
            jq -n --arg target "$TMPDIR/failing-backing.img" '{
              spec: {
                backingFiles: {
                  ($target): {
                    operation: "create",
                    desiredSize: "1M"
                  }
                }
              }
            }' > "$failingSpec"
            if PATH="$failingToolDir:${diskNix}/bin:$PATH" ${diskNix}/bin/disk-nix apply \
              --spec "$failingSpec" \
              --execute \
              --report-out "$failingApplyReport" \
              --receipt-out "$failingApplyReceipt" \
              --json > "$failingApply"; then
              echo "expected failing backing-file apply to fail" >&2
              exit 1
            fi
            jq -e --arg target "$TMPDIR/failing-backing.img" '
              .status == "failed"
              and .apply.blockedCount == 0
              and .commandSummary.commandCount == 3
              and (.executionResults | length) == 2
              and .executionResults[0].success == true
              and .executionResults[0].argv == ["test", "!", "-e", $target]
              and .executionResults[1].success == false
              and .executionResults[1].statusCode == 73
              and .executionResults[1].argv == ["truncate", "--size", "1M", $target]
              and (.executionResults[1].stderr | contains("synthetic truncate failure"))
              and .partialExecutionRecovery.failedPhase == "command"
              and .partialExecutionRecovery.failedCommand == ["truncate", "--size", "1M", $target]
              and .partialExecutionRecovery.completedMutatingCommandCount == 0
              and (.partialExecutionRecovery.retryReviewActionIds | length == 1)
              and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
              and (.messages[] | contains("execute failed: stopped after 2 command result(s)"))
              and (.recoveryActions | any(.kind == "review-execution-failure"))
              and (.recoveryActions | any(.kind == "inspect-current-state"))
              and (.recoveryActions | any(.kind == "preserve-recovery-points"))
            ' "$failingApply"
            cmp "$failingApply" "$failingApplyReport"
            jq -e --arg spec "$failingSpec" --arg target "$TMPDIR/failing-backing.img" '
              .receiptVersion == 1
              and .command == "apply"
              and .specPath == $spec
              and .executeRequested == true
              and .report.status == "failed"
              and .report.executionResults[1].argv == ["truncate", "--size", "1M", $target]
              and .report.partialExecutionRecovery.failedCommand == ["truncate", "--size", "1M", $target]
              and (.report.recoveryActions | any(.kind == "review-execution-failure"))
            ' "$failingApplyReceipt"

            rollbackToolDir="$TMPDIR/rollback-tools"
            mkdir -p "$rollbackToolDir"
            cat > "$rollbackToolDir/zfs" <<'EOF'
            #!${pkgs.bash}/bin/bash
            if [ "$1" = rollback ]; then
              echo "synthetic zfs rollback failure for disk-nix recovery coverage" >&2
              exit 74
            fi
            printf '{}\n'
            EOF
            chmod +x "$rollbackToolDir/zfs"
            rollbackSpec="$TMPDIR/failing-rollback.json"
            rollbackApply="$TMPDIR/failing-rollback.out"
            jq -n '{
              spec: {
                snapshots: {
                  "tank/home@before": {
                    rollback: true
                  }
                }
              },
              apply: {
                allowPotentialDataLoss: true
              }
            }' > "$rollbackSpec"
            if PATH="$rollbackToolDir:${diskNix}/bin:$PATH" ${diskNix}/bin/disk-nix apply \
              --spec "$rollbackSpec" \
              --execute \
              --json > "$rollbackApply"; then
              echo "expected failing ZFS rollback apply to fail" >&2
              exit 1
            fi
            jq -e '
              .status == "failed"
              and .apply.blockedCount == 0
              and .commandSummary.commandCount == 2
              and (.executionResults | length) == 2
              and .executionResults[0].argv == ["zfs", "list", "-t", "snapshot", "-H", "-p", "tank/home@before"]
              and .executionResults[0].success == true
              and .executionResults[1].argv == ["zfs", "rollback", "tank/home@before"]
              and .executionResults[1].success == false
              and .executionResults[1].statusCode == 74
              and (.executionResults[1].stderr | contains("synthetic zfs rollback failure"))
              and .partialExecutionRecovery.failedPhase == "command"
              and .partialExecutionRecovery.failedCommand == ["zfs", "rollback", "tank/home@before"]
              and .partialExecutionRecovery.completedMutatingCommandCount == 0
              and (.partialExecutionRecovery.retryReviewActionIds | index("snapshot:tank/home@before:rollback") != null)
              and (.recoveryActions | any(
                .kind == "domain-recovery"
                and (.commands | any(.argv == ["zfs", "list", "-t", "snapshot", "-H", "-p", "tank/home@before"]))
                and (.commands | any(.argv == ["zfs", "list", "-H", "-p", "tank/home"]))
                and (.notes | any(contains("prefer cloning the snapshot")))
              ))
              and (.recoveryActions | any(
                .kind == "roll-forward-review"
                and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
                and (.commands | any(.argv == ["zfs", "list", "-t", "snapshot", "-H", "-p", "-o", "name,creation,used,referenced,userrefs", "-r", "tank/home"]))
              ))
              and (.recoveryActions | any(
                .kind == "rollback-review"
                and (.commands | all(.mutates == false))
                and (.commands | any(.argv == ["zfs", "list", "-t", "snapshot", "-H", "-p", "tank/home@before"]))
                and (.commands | any(.argv == ["zfs", "list", "-H", "-p", "tank/home"]))
              ))
              and (.recoveryActions | any(.kind == "preserve-recovery-points"))
            ' "$rollbackApply"

            if ${diskNix}/bin/disk-nix apply --spec ${./examples/lifecycle-update.json} --report-out "$lifecycleApplyReport" --json > "$lifecycleApply"; then
              echo "expected lifecycle example apply to be blocked" >&2
              exit 1
            fi
            jq -e '
              .status == "blocked"
              and .apply.blockedCount == 39
              and .apply.blockedSummary.offlineRequiredCount == 31
              and .apply.blockedSummary.destructiveCount == 4
              and .apply.blockedSummary.potentialDataLossCount == 4
              and .apply.blockedSummary.unsupportedCount == 0
              and (.apply.blocked | any(.id == "datasets:tank/legacy:rename"))
              and (.apply.blocked | any(.id == "datasets:tank/home-review:promote"))
              and (.apply.blocked | any(.id == "pools:vault:import"))
              and (.apply.blocked | any(.id == "btrfssubvolumes:/mnt/persist/@old-name:rename"))
              and (.apply.blocked | any(.id == "pools:moveme:export"))
              and (.apply.blocked | any(.id == "volumegroups:importvg:import"))
              and (.apply.blocked | any(.id == "volumegroups:exportvg:export"))
              and (.apply.blocked | any(.id == "volumegroups:activevg:activate"))
              and (.apply.blocked | any(.id == "iscsisessions:iqn.2026-06.example:storage.logout:logout"))
              and (.apply.blocked | any(.id == "luns:iqn.2026-06.example:storage/old:3:detach"))
              and (.apply.blocked | any(.id == "exports:/srv/old-share:unexport"))
              and (.apply.blocked | any(.id == "nfs.mounts:/srv/old:unmount"))
              and (.apply.blocked | any(.id == "volumes:vg0/archive:deactivate"))
              and (.apply.blocked | any(.id == "swaps:retired:deactivate"))
              and (.apply.blocked | any(.id == "swaps:remove:destroy"))
              and (.apply.blocked | any(.id == "vdovolumes:warmarchive:start"))
              and (.apply.blocked | any(.id == "vdovolumes:coldarchive:stop"))
              and (.apply.blocked | any(.id == "luks.devices:cryptarchive:open"))
              and (.apply.blocked | any(.id == "luks.devices:cryptclosed:close"))
              and (.apply.blocked | any(.id == "lukskeyslots:cryptroot:2:remove-key"))
              and (.apply.blocked | any(.id == "lukstokens:cryptroot:1:remove-token"))
              and (.apply.blocked | any(.id == "mdraids:existing:assemble"))
              and (.apply.blocked | any(.id == "mdraids:oldroot:stop"))
              and (.apply.blocked | any(.id == "multipathmaps:mpathold:destroy"))
              and (.apply.blocked | any(.id == "snapshot:tank/home@before-prune:rename:tank/home@retained"))
            ' "$lifecycleApply"
            jq -e '
              .status == "blocked"
              and .apply.blockedCount == 39
            ' "$lifecycleApplyReport"

            ${diskNix}/bin/disk-nix validate --spec ${./examples/lifecycle-update.json} --report-out "$lifecycleValidateReport" --json > "$lifecycleValidate"
            jq -e '
              .status == "blocked"
              and .apply.blockedCount == 39
              and .messages[0] == "apply policy blocked 39 action(s)"
            ' "$lifecycleValidate"
            cmp "$lifecycleValidate" "$lifecycleValidateReport"

            touch "$out"
          '';
          formatting = pkgs.runCommand "disk-nix-formatting-check" { nativeBuildInputs = [ pkgs.findutils pkgs.nixfmt ]; } ''
            cp -R ${self} source
            chmod -R u+w source
            cd source
            while IFS= read -r -d "" file; do
              case "$file" in
                *.nix) nixfmt --check "$file" ;;
              esac
            done < <(${formatFiles})
            touch "$out"
          '';
          nixosModule = nixosModuleTest.config.system.build.toplevel;
          nixosModuleSpec =
            pkgs.runCommand "disk-nix-nixos-module-spec-check" { nativeBuildInputs = [ pkgs.jq ]; }
              ''
                  spec=${nixosModuleTest.config.environment.etc."disk-nix/spec.json".source}
                  jq -e '
                    .version == 1
                    and .spec.filesystems.root.device == "/dev/disk/by-label/nixos-root"
                    and .spec.filesystems.root.resizePolicy == "grow-only"
                    and .spec.filesystems.root.desiredSize == "100%"
                    and .spec.filesystems.data.device == "/dev/disk/by-label/data"
                    and .spec.filesystems.data.fsType == "btrfs"
                    and .spec.filesystems.data.operation == "rebalance"
                    and (.spec.filesystems.data.addDevices | index("/dev/disk/by-id/nvme-btrfs-new") != null)
                    and (.spec.filesystems.data.removeDevices | index("/dev/disk/by-id/nvme-btrfs-old") != null)
                    and .spec.filesystems.data.replaceDevices."/dev/disk/by-id/nvme-btrfs-aging" == "/dev/disk/by-id/nvme-btrfs-replacement"
                    and .spec.filesystems.data.properties.label == "bulk-data"
                    and .spec.filesystems.data.properties."btrfs.balance.data" == "usage=50"
                    and .spec.filesystems.data.metadata.pool == "tank"
                    and .spec.filesystems.data.metadata.role == "bulk-data"
                    and .spec.filesystems.scratch.operation == "check"
                    and .spec.filesystems.scratch.device == "/dev/disk/by-label/scratch"
                    and .spec.filesystems.scrub.operation == "scrub"
                    and .spec.filesystems.scrub.device == "/dev/disk/by-label/scrub"
                    and .spec.filesystems.scrub.mountpoint == "/scrub"
                    and .spec.filesystems.trim.operation == "trim"
                    and .spec.filesystems.trim.device == "/dev/disk/by-label/trim"
                    and .spec.filesystems.remount.operation == "remount"
                    and .spec.filesystems.remount.mountpoint == "/remount"
                    and (.spec.filesystems.remount.options | index("discard=async") != null)
                    and .spec.filesystems.localMount.operation == "mount"
                    and .spec.filesystems.localMount.device == "/dev/disk/by-label/local-mount"
                    and .spec.filesystems.localMount.mountpoint == "/mnt/local-mount"
                    and (.spec.filesystems.localMount.options | index("noatime") != null)
                    and .spec.filesystems.localUnmount.operation == "unmount"
                    and .spec.filesystems.localUnmount.device == "/dev/disk/by-label/local-unmount"
                    and .spec.filesystems.localUnmount.mountpoint == "/mnt/local-unmount"
                    and .spec.filesystems.localRescan.operation == "rescan"
                    and .spec.filesystems.localRescan.device == "/dev/disk/by-label/local-rescan"
                    and .spec.filesystems.localRescan.mountpoint == "/mnt/local-rescan"
                    and .spec.filesystems.actionRescan.action == "rescan"
                    and .spec.filesystems.actionUnmount.action == "unmount"
                    and .spec.filesystems.destroyed.destroy == true
                    and .spec.filesystems.destroyed.device == "/dev/disk/by-label/destroyed"
                    and .spec.filesystems.targetSizeAlias.operation == "rescan"
                    and .spec.filesystems.targetSizeAlias.targetSize == "200GiB"
                    and .spec.filesystems.sizeAlias.operation == "rescan"
                    and .spec.filesystems.sizeAlias.size == "150GiB"
                    and .spec.filesystems.runTmpfs.device == "tmpfs"
                    and .spec.filesystems.runTmpfs.fsType == "tmpfs"
                    and .spec.filesystems.runTmpfs.mountpoint == "/run/disk-nix-tmp"
                    and (.spec.filesystems.runTmpfs.options | index("size=64M") != null)
                    and .spec.filesystems.bindCache.device == "/var/cache/disk-nix"
                    and .spec.filesystems.bindCache.fsType == "none"
                    and .spec.filesystems.bindCache.mountpoint == "/srv/disk-nix-cache"
                    and (.spec.filesystems.bindCache.options | index("bind") != null)
                    and .spec.filesystems.overlayScratch.device == "overlay"
                    and .spec.filesystems.overlayScratch.fsType == "overlay"
                    and .spec.filesystems.overlayScratch.mountpoint == "/srv/disk-nix-overlay"
                    and (.spec.filesystems.overlayScratch.options | index("lowerdir=/nix/store") != null)
                    and (.spec.filesystems.overlayScratch.options | index("upperdir=/var/lib/disk-nix/overlay/upper") != null)
                    and (.spec.filesystems.overlayScratch.options | index("workdir=/var/lib/disk-nix/overlay/work") != null)
                    and .spec.swaps.primary.device == "/dev/disk/by-label/swap"
                    and .spec.swaps.primary.operation == "format"
                    and .spec.swaps.primary.desiredSize == "8GiB"
                    and .spec.swaps.primary.preserveData == false
                    and .spec.swaps.primary.properties.label == "swap"
                    and .spec.swaps.primary.properties."swap.uuid" == "01234567-89ab-cdef-0123-456789abcdef"
                    and .spec.swaps.inventory.operation == "rescan"
                    and .spec.swaps.inventory.device == "/dev/disk/by-label/swap-inventory"
                    and .spec.swaps.targetSizeAlias.operation == "grow"
                    and .spec.swaps.targetSizeAlias.targetSize == "12GiB"
                    and .spec.swaps.sizeAlias.operation == "grow"
                    and .spec.swaps.sizeAlias.size == "10GiB"
                    and .spec.swaps.old.operation == "destroy"
                    and .spec.swaps.actionOld.action == "destroy"
                    and .spec.swaps.destroyed.destroy == true
                    and .spec.swaps.destroyed.device == "/dev/disk/by-label/destroyed-swap"
                    and .spec.zram.enable == true
                    and .spec.zram.operation == "rescan"
                    and .spec.zram.swapDevices == 2
                    and .spec.zram.memoryPercent == 40
                    and .spec.zram.memoryMax == 8589934592
                    and .spec.zram.priority == 20
                    and .spec.zram.algorithm == "zstd"
                    and .spec.zram.properties."zram.compression-ratio-target" == "2.0"
                    and .spec.luks.devices.cryptaction.action == "open"
                    and .spec.swaps.old.device == "/dev/disk/by-label/old-swap"
                    and .spec.luks.devices.cryptroot.device == "/dev/disk/by-partuuid/d024c121-4300-4493-a643-055bc4d5caa7"
                    and .spec.luks.devices.cryptroot.name == "cryptroot"
                    and .spec.luks.devices.cryptroot.operation == "grow"
                    and .spec.luks.devices.cryptroot.desiredSize == "100%"
                    and .spec.luks.devices.cryptroot.properties.label == "cryptroot"
                    and .spec.luks.devices.cryptroot.properties."luks.subsystem" == "nixos"
                    and .spec.luks.devices.cryptTargetSize.operation == "grow"
                    and .spec.luks.devices.cryptTargetSize.target == "cryptTargetSizeMapper"
                    and .spec.luks.devices.cryptTargetSize.targetSize == "90%"
                    and .spec.luks.devices.cryptSize.operation == "grow"
                    and .spec.luks.devices.cryptSize.size == "80%"
                    and .spec.luks.devices.cryptold.destroy == true
                    and .spec.luks.devices.cryptold.device == "/dev/disk/by-partuuid/old-luks"
                    and .spec.luks.devices.cryptarchive.operation == "open"
                    and .spec.luks.devices.cryptarchive.device == "/dev/disk/by-id/archive-luks"
                    and .spec.luks.devices.cryptclosed.operation == "close"
                    and .spec.luks.devices.cryptclosed.device == "/dev/disk/by-id/closed-luks"
                    and .spec.filesystems.shared.device == "nas.example.com:/srv/shared"
                    and .spec.filesystems.shared.mountpoint == "/srv/shared"
                    and .spec.filesystems.shared.fsType == "nfs4"
                    and (.spec.filesystems.shared.options | index("x-systemd.automount") != null)
                    and (.spec.filesystems | has("/srv/old") | not)
                    and .spec.nfs.mounts.shared.source == "nas.example.com:/srv/shared"
                    and .spec.nfs.mounts.shared.mountpoint == "/srv/shared"
                    and .spec.nfs.mounts.shared.operation == "mount"
                    and .spec.nfs.mounts.shared.metadata.server == "nas.example.com"
                    and .spec.nfs.mounts.shared.metadata.export == "/srv/shared"
                    and .spec.nfs.mounts."/srv/tuned".operation == "remount"
                    and (.spec.nfs.mounts."/srv/tuned".options | index("ro") != null)
                    and .spec.nfs.mounts."/srv/action".action == "remount"
                    and .spec.nfs.mounts."/srv/inventory".operation == "rescan"
                    and .spec.nfs.mounts."/srv/inventory".source == "nas.example.com:/srv/inventory"
                    and .spec.nfs.mounts."/srv/old".source == "nas.example.com:/srv/old"
                    and .spec.nfs.mounts."/srv/old".operation == "unmount"
                    and .spec.iscsi.initiatorName == "iqn.2026-06.example:host"
                    and (.spec.iscsi | has("discoverPortal") | not)
                    and (.spec.iscsi.boot | has("discoverPortal") | not)
                    and .spec.iscsi.boot.target == "iqn.2026-06.example:storage.root"
                    and .spec.iscsi.sessions."iqn.2026-06.example:storage.root".operation == "grow"
                    and .spec.iscsi.sessions."iqn.2026-06.example:storage.root".desiredSize == "2TiB"
                    and .spec.iscsi.sessions."iqn.2026-06.example:storage.alias".targetSize == "3TiB"
                    and .spec.iscsi.sessions."iqn.2026-06.example:storage.login".operation == "login"
                    and .spec.iscsi.sessions."iqn.2026-06.example:storage.logout".operation == "logout"
                    and .spec.iscsi.sessions."iqn.2026-06.example:storage.rescan".operation == "rescan"
                    and .spec.iscsiSessions."iqn.2026-06.example:storage.root".portal == "192.0.2.10:3260"
                    and .spec.iscsiSessions."iqn.2026-06.example:storage.root".operation == "grow"
                    and .spec.iscsiSessions."iqn.2026-06.example:storage.root".desiredSize == "2TiB"
                    and .spec.iscsiSessions."iqn.2026-06.example:storage.alias".targetSize == "3TiB"
                    and .spec.iscsiSessions."iqn.2026-06.example:storage.login".operation == "login"
                    and .spec.iscsiSessions."iqn.2026-06.example:storage.login".portal == "192.0.2.10:3260"
                    and .spec.iscsiSessions."iqn.2026-06.example:storage.logout".operation == "logout"
                    and .spec.iscsiSessions."iqn.2026-06.example:storage.logout".portal == "192.0.2.11:3260"
                    and .spec.iscsiSessions."iqn.2026-06.example:storage.rescan".operation == "rescan"
                    and .spec.iscsiSessions."iqn.2026-06.example:storage.rescan".portal == "192.0.2.10:3260"
                    and .spec.luns."iqn.2026-06.example:storage/root:0".lun == 0
                    and .spec.luns."iqn.2026-06.example:storage/root:0".device == "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0"
                    and (.spec.luns."iqn.2026-06.example:storage/root:0".devices | index("/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-0") != null)
                    and .spec.luns."iqn.2026-06.example:storage/new:2".operation == "attach"
                    and .spec.luns."iqn.2026-06.example:storage/new:2".device == "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-2"
                    and .spec.luns."iqn.2026-06.example:storage/old:3".operation == "detach"
                    and (.spec.luns."iqn.2026-06.example:storage/old:3".devices | index("/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-3") != null)
                    and .spec.luns."iqn.2026-06.example:storage/rescan:4".operation == "rescan"
                    and (.spec.luns."iqn.2026-06.example:storage/rescan:4".paths | index("/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-4") != null)
                    and .spec.nvmeNamespaces.rootNamespace.operation == "create"
                    and .spec.nvmeNamespaces.rootNamespace.path == "/dev/nvme0"
                    and .spec.nvmeNamespaces.rootNamespace.desiredSize == "100G"
                    and .spec.nvmeNamespaces.rootNamespace.namespaceId == "4"
                    and .spec.nvmeNamespaces.rootNamespace.controllers == "0x1"
                    and .spec.nvmeNamespaces."/dev/nvme1".operation == "rescan"
                    and .spec.nvmeNamespaces."/dev/nvme2".nsid == "7"
                    and .spec.nvmeNamespaces."/dev/nvme2".controllerId == "0x2"
                    and .spec.nvmeNamespaces."/dev/nvme3".namespaceId == "8"
                    and .spec.nvmeNamespaces."/dev/nvme3".controller == "0x3"
                    and .spec.exports.share.operation == "export"
                    and .spec.exports.share.path == "/srv/share"
                    and .spec.exports.share.client == "192.0.2.0/24"
                    and .spec.exports.share.options == "rw,sync,no_subtree_check"
                    and .spec.exports."/srv/inventory".operation == "rescan"
                    and .spec.exports."/srv/old-share".operation == "unexport"
                    and .spec.exports."/srv/old-share".client == "192.0.2.55"
                    and .spec.partitions.root.operation == "grow"
                    and .spec.partitions.root.device == "/dev/disk/by-id/nvme-root"
                    and .spec.partitions.root.number == "2"
                    and .spec.partitions.root.endOffset == "100%"
                    and .spec.partitions.dataTable.operation == "rescan"
                    and .spec.partitions.dataTable.device == "/dev/disk/by-id/nvme-data"
                    and .spec.btrfsSubvolumes."/mnt/persist/@home".operation == "create"
                    and .spec.btrfsSubvolumes."/mnt/persist/@home".path == "/mnt/persist/@home"
                    and .spec.btrfsSubvolumes."/mnt/persist/@inventory".operation == "rescan"
                    and .spec.btrfsSubvolumes."/mnt/persist/@inventory".path == "/mnt/persist/@inventory"
                    and .spec.btrfsSubvolumes."/mnt/persist/@old-name".operation == "rename"
                    and .spec.btrfsSubvolumes."/mnt/persist/@old-name".renameTo == "/mnt/persist/@new-name"
                    and .spec.btrfsQgroups."0/257".target == "/mnt/persist"
                    and .spec.btrfsQgroups."0/257".properties.limit == "25GiB"
                    and .spec.btrfsQgroups."0/258".operation == "rescan"
                    and .spec.btrfsQgroups."0/258".target == "/mnt/persist"
                    and .spec.volumes.scratch.operation == "create"
                    and .spec.volumes.scratch.target == "vg0/scratch"
                    and .spec.volumes.scratch.desiredSize == "10GiB"
                    and .spec.volumes."vg0/size-alias".size == "12GiB"
                    and .spec.volumes."vg0/reporting".operation == "rescan"
                    and .spec.datasets."tank/home".operation == "create"
                    and .spec.datasets."tank/inventory".operation == "rescan"
                    and .spec.vdoVolumes.archiveLifecycle.target == "archive"
                    and .spec.vdoVolumes.archiveLifecycle.operation == "grow"
                    and .spec.vdoVolumes.archiveLifecycle.desiredSize == "4TiB"
                    and .spec.vdoVolumes.archiveLifecycle.physicalSize == "6TiB"
                    and .spec.vdoVolumes.archiveLifecycle.properties.writePolicy == "sync"
                    and .spec.vdoVolumes.archiveLifecycle.properties.compression == "enabled"
                    and .spec.vdoVolumes.archiveLifecycle.properties.deduplication == "disabled"
                    and .spec.vdoVolumes.warmArchive.target == "warm-archive"
                    and .spec.vdoVolumes.warmArchive.operation == "start"
                    and .spec.vdoVolumes.coldArchive.target == "cold-archive"
                    and .spec.vdoVolumes.coldArchive.operation == "stop"
                    and .spec.vdoVolumes.refreshArchive.target == "refresh-archive"
                    and .spec.vdoVolumes.refreshArchive.operation == "rescan"
                    and .spec.physicalVolumes.nvmePvGrow.operation == "grow"
                    and .spec.physicalVolumes.nvmePvGrow.path == "/dev/disk/by-id/nvme-pv-grow"
                    and .spec.physicalVolumes."/dev/disk/by-id/nvme-pv-refresh".operation == "rescan"
                    and .spec.luksKeyslots."cryptroot:1".operation == "add-key"
                    and .spec.luksKeyslots."cryptroot:1".device == "/dev/disk/by-id/root-luks"
                    and .spec.luksKeyslots."cryptroot:1".keySlot == "1"
                    and .spec.luksKeyslots."cryptroot:1".newKeyFile == "/run/keys/root-new"
                    and .spec.luksKeyslots."cryptroot:2".operation == "remove-key"
                    and .spec.luksKeyslots."cryptroot:2".device == "/dev/disk/by-id/root-luks"
                    and .spec.luksKeyslots."cryptroot:2".keySlot == "2"
                    and .spec.luksKeyslots."cryptroot:3"."key-slot" == "3"
                    and .spec.luksKeyslots."cryptroot:3"."new-key-file" == "/run/keys/root-new-alias"
                    and .spec.luksKeyslots."cryptroot:4".slot == "4"
                    and .spec.luksTokens."cryptroot:0".operation == "import-token"
                    and .spec.luksTokens."cryptroot:0".device == "/dev/disk/by-id/root-luks"
                    and .spec.luksTokens."cryptroot:0".tokenId == "0"
                    and .spec.luksTokens."cryptroot:0".tokenFile == "/run/keys/root-token.json"
                    and .spec.luksTokens."cryptroot:1".operation == "remove-token"
                    and .spec.luksTokens."cryptroot:1".device == "/dev/disk/by-id/root-luks"
                    and .spec.luksTokens."cryptroot:1".tokenId == "1"
                    and .spec.luksTokens."cryptroot:2".token == "2"
                    and .spec.luksTokens."cryptroot:2"."token-file" == "/run/keys/root-token-alias.json"
                    and .spec.luksTokens."cryptroot:3"."token-id" == "3"
                    and .spec.zvols."tank/vm/root".operation == "grow"
                    and .spec.zvols."tank/vm/root".desiredSize == "80GiB"
                    and .spec.zvols."tank/vm/inventory".operation == "rescan"
                    and .spec.thinPools.primaryPool.operation == "grow"
                    and .spec.thinPools.primaryPool.path == "vg0/thinpool"
                    and .spec.thinPools.primaryPool.desiredSize == "500GiB"
                    and .spec.thinPools."vg0/newthin".operation == "create"
                    and .spec.thinPools."vg0/newthin".desiredSize == "100GiB"
                    and .spec.thinPools."vg0/reporting".operation == "rescan"
                    and .spec.lvmSnapshots."vg0/root-snap".operation == "snapshot"
                    and .spec.lvmSnapshots."vg0/root-snap".target == "vg0/root"
                    and .spec.lvmSnapshots."vg0/root-snap".desiredSize == "20GiB"
                    and .spec.lvmSnapshots."vg0/root-inspect".operation == "rescan"
                    and .spec.lvmCaches."vg0/root".operation == "create"
                    and .spec.lvmCaches."vg0/root".device == "vg0/root-cache"
                    and .spec.lvmCaches."vg0/root".properties."lvm.cache-mode" == "writethrough"
                    and .spec.lvmCaches."vg0/archive".operation == "rescan"
                    and .spec.volumes."vg0/archive".operation == "deactivate"
                    and .spec.loopDevices.rootImage.operation == "create"
                    and .spec.loopDevices.rootImage.path == "/dev/loop7"
                    and .spec.loopDevices.rootImage.device == "/var/lib/images/root.img"
                    and .spec.loopDevices."/dev/loop10".operation == "rescan"
                    and .spec.backingFiles."/var/lib/images/new.img".operation == "create"
                    and .spec.backingFiles."/var/lib/images/new.img".desiredSize == "8GiB"
                    and .spec.backingFiles."/var/lib/images/root.img".operation == "grow"
                    and .spec.backingFiles."/var/lib/images/root.img".desiredSize == "16GiB"
                    and .spec.backingFiles.inventoryImage.operation == "rescan"
                    and .spec.backingFiles.inventoryImage.path == "/var/lib/images/inventory.img"
                    and .spec.dmMaps.cryptroot.operation == "rescan"
                    and .spec.dmMaps.cryptroot.target == "/dev/mapper/cryptroot"
                    and .spec.dmMaps.cryptswap.operation == "rename"
                    and .spec.dmMaps.cryptswap.target == "/dev/mapper/cryptswap"
                    and .spec.dmMaps.cryptswap.renameTo == "cryptswap-retired"
                    and .spec.dmMaps.oldmap.operation == "destroy"
                    and .spec.dmMaps.oldmap.target == "/dev/mapper/oldmap"
                    and .spec.mdRaids.root.target == "/dev/md/root"
                    and .spec.mdRaids.root.raidLevel == "1"
                    and (.spec.mdRaids.root.devices | index("/dev/disk/by-id/nvme-md-a") != null)
                    and (.spec.mdRaids.root.devices | index("/dev/disk/by-id/nvme-md-b") != null)
                    and (.spec.mdRaids.root.addDevices | index("/dev/disk/by-id/nvme-md-spare") != null)
                    and .spec.mdRaids.root.replaceDevices."/dev/disk/by-id/nvme-md-aging" == "/dev/disk/by-id/nvme-md-replacement"
                    and .spec.mdRaids.existing.operation == "assemble"
                    and .spec.mdRaids.existing.target == "/dev/md/existing"
                    and (.spec.mdRaids.existing.devices | index("/dev/disk/by-id/existing-md-a") != null)
                    and .spec.mdRaids.oldroot.operation == "stop"
                    and .spec.mdRaids.oldroot.target == "/dev/md/oldroot"
                    and .spec.mdRaids.inventory.operation == "rescan"
                    and .spec.multipathMaps.mpatha.target == "mpatha"
                    and (.spec.multipathMaps.mpatha.addDevices | index("/dev/sdb") != null)
                    and .spec.multipathMaps.mpatha.replaceDevices."/dev/sdc" == "/dev/sdd"
                    and .spec.multipathMaps.mpathb.operation == "rescan"
                    and .spec.multipathMaps.mpathb.target == "mpathb"
                    and .spec.multipathMaps.mpathOld.operation == "destroy"
                    and .spec.multipathMaps.mpathOld.target == "mpath-old"
                    and .spec.caches."tank/l2arc0".cacheSetUuid == "11111111-2222-3333-4444-555555555555"
                    and (.spec.caches."/dev/bcache0".addDevices | index("cache-set-uuid") != null)
                    and .spec.caches."/dev/bcache0".cacheSetUuid == "cache-set-uuid"
                    and .spec.caches."/dev/bcache0".operation == "rescan"
                    and .spec.caches."/dev/bcache0".properties."bcache.cache-mode" == "writethrough"
                    and .spec.caches."/dev/bcache0".properties."bcache.set-journal-delay-ms" == "100"
                    and .spec.pools.vault.operation == "import"
                    and .spec.pools.vault.readOnly == true
                    and .spec.pools.archiveImport.readonly == true
                    and .spec.pools.moveme.operation == "export"
                    and .spec.volumeGroups.importvg.operation == "import"
                    and .spec.volumeGroups.exportvg.operation == "export"
                    and .spec.volumeGroups.activevg.operation == "activate"
                    and .spec.volumeGroups.refreshvg.operation == "rescan"
                    and .spec.volumeGroups.actionvg.action == "rescan"
                    and .spec.datasets."tank/home-review".operation == "promote"
                    and .spec.datasets."tank/legacy-alias".renameTarget == "tank/legacy-alias-staged"
                    and .spec.datasets."tank/legacy-short".newName == "tank/legacy-short-staged"
                    and .spec.snapshots."tank/home@before-upgrade".target == "tank/home"
                    and .spec.snapshots."tank/home@before-upgrade".hold == "disk-nix-retain"
                    and .spec.snapshots."tank/home@before-upgrade".rollback == true
                and .spec.snapshots."tank/home@before-upgrade".cloneTo == "tank/home-review"
                and .spec.snapshots."tank/home@before-upgrade".renameTo == "tank/home@before-upgrade-retained"
                and .spec.snapshots."tank/home@before-upgrade".recursiveRollback == true
                and .spec.snapshots."tank/home@clone-only".operation == "clone"
                and .spec.snapshots."tank/home@clone-only".cloneTo == "tank/home-clone"
                and .spec.snapshots."tank/home@action-rescan".action == "rescan"
                and .spec.snapshots.aliases.operation == "clone"
                and .spec.snapshots.aliases."snapshot-path" == "tank/home@alias-source"
                and .spec.snapshots.aliases.cloneTarget == "tank/home-alias-clone"
                and .spec.snapshots.aliases.clone == "tank/home-short-clone"
                and .spec.snapshots.aliases.renameTarget == "tank/home@alias-retained"
                and .spec.snapshots.aliases.newName == "tank/home@alias-new"
                and .spec.snapshots.aliases.recursive == true
                and .spec.snapshots.aliases."zfs.rollbackRecursive" == true
                and .spec.snapshots.aliases.readonly == true
                and .spec.datasets."tank/legacy".renameTo == "tank/legacy-staged"
                    and .spec.snapshots."tank/home@old".releaseHold == "old-retention"
                    and .spec.snapshots."/mnt/persist/@home-before-upgrade".target == "/mnt/persist/@home"
                    and .spec.snapshots."/mnt/persist/@home-before-upgrade".readOnly == true
                    and .spec.snapshots."/mnt/persist/@home-before-clone".target == "/mnt/persist/@home"
                    and .spec.snapshots."/mnt/persist/@home-before-clone".cloneTo == "/mnt/persist/@home-review"
                    and .spec.snapshots."/mnt/persist/@home-before-clone".readOnly == true
                    and .spec.snapshots."tank/home@inventory".operation == "rescan"
                    and .spec.snapshots."/mnt/persist/@home-inventory".operation == "rescan"
                    and .spec.snapshots."/mnt/persist/@home-inventory".readOnly == true
                    and .spec.snapshots."home-before-friendly".operation == "rescan"
                    and .spec.snapshots."home-before-friendly".target == "/mnt/persist/@home"
                    and .spec.snapshots."home-before-friendly".snapshotPath == "/mnt/persist/@home-before-friendly"
                    and .apply.mode == "activation"
                    and .apply.allowGrow == true
                    and .apply.allowOffline == false
                    and .apply.probeCurrent == true
                    and .apply.allowDeviceReplacement == true
                    and .apply.allowRebalance == true
                    and .apply.allowPotentialDataLoss == false
                    and .apply.requireBackup == false
                    and .apply.backupVerified == false
                    and .apply.requireConfirmation == false
                    and .apply.confirmation == false
                    and .apply.requireConfirmationFile == "/run/disk-nix/confirm"
                    and .apply.failOnBlocked == false
                    and .apply.scriptOut == "/run/disk-nix/apply.sh"
                    and .apply.reportOut == "/run/disk-nix/apply-report.json"
                    and .apply.receiptOut == "/run/disk-nix/apply-receipt.json"
                  ' "$spec"
                  applyScript='${nixosModuleTest.config.systemd.services.disk-nix-plan.serviceConfig.ExecStart}'
                  grep -- 'validate' "$applyScript"
                  grep -- '--probe-current' "$applyScript"
                  grep -- '--script-out' "$applyScript"
                  grep -- '/run/disk-nix/apply.sh' "$applyScript"
                  grep -- '--report-out' "$applyScript"
                  grep -- '/run/disk-nix/apply-report.json' "$applyScript"
                  grep -- '--receipt-out' "$applyScript"
                  grep -- '/run/disk-nix/apply-receipt.json' "$applyScript"
                  printf '%s\n' ${pkgs.lib.escapeShellArgs (map toString nixosModuleTest.config.systemd.services.disk-nix-plan.path)} > service-paths
                  grep -- 'bcachefs-tools-' service-paths
                  grep -- 'btrfs-progs-' service-paths
                  grep -- 'dosfstools-' service-paths
                  grep -- 'exfatprogs-' service-paths
                  grep -- 'f2fs-tools-' service-paths
                  grep -- 'lvm2-' service-paths
                  grep -- 'lsscsi-' service-paths
                  grep -- 'ntfs3g-' service-paths
                  grep -- 'open-iscsi-' service-paths
                  grep -- 'smartmontools-' service-paths
                  grep -- 'targetcli-fb-' service-paths
                  grep -- 'tgt-' service-paths
                  grep -- 'util-linux-' service-paths
                  grep -- 'zfs-user-' service-paths
                  swapDevices=${
                    pkgs.lib.escapeShellArg (
                      builtins.toJSON (map (swap: { inherit (swap) device; }) nixosModuleTest.config.swapDevices)
                    )
                  }
                  printf '%s\n' "$swapDevices" > swap-devices
                  jq -e '
                    length == 4
                    and any(.[]; .device == "/dev/disk/by-label/swap")
                    and any(.[]; .device == "/dev/disk/by-label/swap-inventory")
                    and any(.[]; .device == "/dev/disk/by-label/swap-target-size")
                    and any(.[]; .device == "/dev/disk/by-label/swap-size")
                    and all(.[]; .device != "/dev/disk/by-label/action-old-swap")
                    and all(.[]; .device != "/dev/disk/by-label/destroyed-swap")
                  ' swap-devices
                  luksDevices=${
                    pkgs.lib.escapeShellArg (
                      builtins.toJSON (
                        pkgs.lib.mapAttrs (_: luks: {
                          inherit (luks) device;
                        }) nixosModuleTest.config.boot.initrd.luks.devices
                      )
                    )
                  }
                  printf '%s\n' "$luksDevices" > luks-devices
                  jq -e '
                    has("cryptroot")
                    and .cryptroot.device == "/dev/disk/by-partuuid/d024c121-4300-4493-a643-055bc4d5caa7"
                    and has("cryptTargetSizeMapper")
                    and .cryptTargetSizeMapper.device == "/dev/disk/by-id/target-size-luks"
                    and (has("cryptTargetSize") | not)
                    and has("cryptSize")
                    and .cryptSize.device == "/dev/disk/by-id/size-luks"
                    and has("cryptarchive")
                    and .cryptarchive.device == "/dev/disk/by-id/archive-luks"
                    and (has("cryptold") | not)
                    and (has("cryptclosed") | not)
                  ' luks-devices
                  fileSystems=${
                    pkgs.lib.escapeShellArg (
                      builtins.toJSON (
                        pkgs.lib.mapAttrs (_: fs: {
                          inherit (fs) device fsType;
                        }) nixosModuleTest.config.fileSystems
                      )
                    )
                  }
                  printf '%s\n' "$fileSystems" > file-systems
                  jq -e '
                    has("/srv/shared")
                    and ."/srv/shared".device == "nas.example.com:/srv/shared"
                    and has("/srv/tuned")
                    and ."/srv/tuned".device == "nas.example.com:/srv/tuned"
                    and ."/srv/tuned".fsType == "nfs4"
                    and has("/mnt/local-mount")
                    and ."/mnt/local-mount".device == "/dev/disk/by-label/local-mount"
                    and ."/mnt/local-mount".fsType == "xfs"
                    and (has("/mnt/local-unmount") | not)
                    and has("/mnt/local-rescan")
                    and ."/mnt/local-rescan".device == "/dev/disk/by-label/local-rescan"
                    and ."/mnt/local-rescan".fsType == "xfs"
                    and has("/mnt/action-rescan")
                    and ."/mnt/action-rescan".device == "/dev/disk/by-label/action-rescan"
                    and ."/mnt/action-rescan".fsType == "xfs"
                    and (has("/mnt/action-unmount") | not)
                    and (has("/mnt/teardown-only") | not)
                    and (has("/mnt/destroyed") | not)
                    and has("/srv/action")
                    and ."/srv/action".device == "nas.example.com:/srv/action"
                    and ."/srv/action".fsType == "nfs4"
                    and has("/run/disk-nix-tmp")
                    and ."/run/disk-nix-tmp".device == "tmpfs"
                    and ."/run/disk-nix-tmp".fsType == "tmpfs"
                    and has("/srv/disk-nix-cache")
                    and ."/srv/disk-nix-cache".device == "/var/cache/disk-nix"
                    and ."/srv/disk-nix-cache".fsType == "none"
                    and has("/srv/disk-nix-overlay")
                    and ."/srv/disk-nix-overlay".device == "overlay"
                    and ."/srv/disk-nix-overlay".fsType == "overlay"
                    and (has("/srv/old") | not)
                  ' file-systems
                  supportedFilesystems=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleTest.config.boot.supportedFilesystems)}
                  printf '%s\n' "$supportedFilesystems" > supported-filesystems
                  jq -e '
                    .xfs == true
                    and .btrfs == true
                    and .bcachefs == true
                    and .f2fs == true
                    and .tmpfs == true
                    and .overlay == true
                    and .nfs4 == true
                    and .zfs == true
                    and (has("jfs") | not)
                  ' supported-filesystems
                  nativeStorage=${
                    pkgs.lib.escapeShellArg (
                      builtins.toJSON {
                        lvm = nixosModuleTest.config.services.lvm.enable;
                        lvmInitrd = nixosModuleTest.config.boot.initrd.services.lvm.enable;
                        lvmThin = nixosModuleTest.config.services.lvm.boot.thin.enable;
                        lvmVdo = nixosModuleTest.config.services.lvm.boot.vdo.enable;
                        swraid = nixosModuleTest.config.boot.swraid.enable;
                        mdadmConf = nixosModuleTest.config.boot.swraid.mdadmConf;
                        multipath = nixosModuleTest.config.services.multipath.enable;
                        zfsExtraPools = nixosModuleTest.config.boot.zfs.extraPools;
                        zfsForceImportRoot = nixosModuleTest.config.boot.zfs.forceImportRoot;
                        bcache = nixosModuleTest.config.boot.bcache.enable;
                        bcacheInitrd = nixosModuleTest.config.boot.initrd.services.bcache.enable;
                        zram = nixosModuleTest.config.zramSwap.enable;
                        zramSwapDevices = nixosModuleTest.config.zramSwap.swapDevices;
                        zramMemoryPercent = nixosModuleTest.config.zramSwap.memoryPercent;
                        zramMemoryMax = nixosModuleTest.config.zramSwap.memoryMax;
                        zramPriority = nixosModuleTest.config.zramSwap.priority;
                        zramAlgorithm = nixosModuleTest.config.zramSwap.algorithm;
                        openIscsiDiscoverPortal = nixosModuleTest.config.services.openiscsi.discoverPortal;
                        bootIscsiDiscoverPortal = nixosModuleTest.config.boot.iscsi-initiator.discoverPortal;
                      }
                    )
                  }
                  printf '%s\n' "$nativeStorage" > native-storage
                  jq -e '
                    .lvm == true
                    and .lvmInitrd == true
                    and .lvmThin == true
                    and .lvmVdo == true
                    and .swraid == true
                    and (.mdadmConf | test("^PROGRAM .*/bin/true$"))
                    and .multipath == true
                    and (.zfsExtraPools | index("tank") != null)
                    and (.zfsExtraPools | index("mnt") == null)
                    and .zfsForceImportRoot == false
                    and .bcache == true
                    and .bcacheInitrd == true
                    and .zram == true
                    and .zramSwapDevices == 2
                    and .zramMemoryPercent == 40
                    and .zramMemoryMax == 8589934592
                    and .zramPriority == 20
                    and .zramAlgorithm == "zstd"
                    and .openIscsiDiscoverPortal == "192.0.2.10:3260"
                    and .bootIscsiDiscoverPortal == "192.0.2.10:3260"
                  ' native-storage
                  steadyState=${
                    pkgs.lib.escapeShellArg (
                      builtins.readFile nixosModuleTest.config.environment.etc."disk-nix/steady-state.json".source
                    )
                  }
                  printf '%s\n' "$steadyState" > steady-state
                  jq -e '
                    .version == 1
                    and .fileSystems."/srv/tuned".device == "nas.example.com:/srv/tuned"
                    and .fileSystems."/srv/tuned".fsType == "nfs4"
                    and .fileSystems."/mnt/local-mount".device == "/dev/disk/by-label/local-mount"
                    and .fileSystems."/mnt/local-mount".fsType == "xfs"
                    and (.fileSystems | has("/mnt/local-unmount") | not)
                    and (.fileSystems | has("/srv/old") | not)
                    and (.swapDevices | length == 4)
                    and (.swapDevices | any(.device == "/dev/disk/by-label/swap"))
                    and (.swapDevices | all(.device != "/dev/disk/by-label/destroyed-swap"))
                    and .luksDevices.cryptroot.device == "/dev/disk/by-partuuid/d024c121-4300-4493-a643-055bc4d5caa7"
                    and (.luksDevices | has("cryptclosed") | not)
                    and .zramSwap.enable == true
                    and .zramSwap.swapDevices == 2
                    and .zramSwap.memoryMax == 8589934592
                    and (.supportedFilesystems | index("xfs") != null)
                    and (.supportedFilesystems | index("nfs4") != null)
                    and (.supportedFilesystems | index("zfs") != null)
                    and (.nfsExports | index("/srv/share 192.0.2.0/24(rw,sync,no_subtree_check)") != null)
                    and (.nfsExports | all(. | contains("/srv/old-share") | not))
                    and (.storageIdentities.filesystemMountpoints | index("/mnt/local-mount") != null)
                    and (.storageIdentities.filesystemMountpoints | index("/mnt/local-unmount") == null)
                    and (.storageIdentities.swapDevices | index("/dev/disk/by-label/swap") != null)
                    and (.storageIdentities.swapDevices | index("/dev/disk/by-label/destroyed-swap") == null)
                    and (.storageIdentities.physicalVolumes | index("/dev/disk/by-id/nvme-pv-grow") != null)
                    and (.storageIdentities.volumes | index("vg0/scratch") != null)
                    and (.storageIdentities.volumes | index("vg0/archive") == null)
                    and (.storageIdentities.thinPools | index("vg0/thinpool") != null)
                    and (.storageIdentities.lvmCaches | index("vg0/root") != null)
                    and (.storageIdentities.vdoVolumes | index("archive") != null)
                    and (.storageIdentities.vdoVolumes | index("cold-archive") == null)
                    and (.storageIdentities.luksKeyslots | index("/dev/disk/by-id/root-luks keyslot 1") != null)
                    and (.storageIdentities.luksTokens | index("/dev/disk/by-id/root-luks token 0") != null)
                    and (.storageIdentities.backingFiles | index("/var/lib/images/root.img") != null)
                    and (.storageIdentities.loopDevices | index("/dev/loop7") != null)
                    and (.storageIdentities.dmMaps | index("/dev/mapper/cryptroot") != null)
                    and (.storageIdentities.mdRaids | index("/dev/md/root") != null)
                    and (.storageIdentities.mdRaids | index("/dev/md/oldroot") == null)
                    and (.storageIdentities.multipathMaps | index("mpatha") != null)
                    and (.storageIdentities.pools | index("vault") != null)
                    and (.storageIdentities.pools | index("moveme") == null)
                    and (.storageIdentities.datasets | index("tank/home") != null)
                    and (.storageIdentities.zvols | index("tank/vm/root") != null)
                    and (.storageIdentities.btrfsSubvolumes | index("/mnt/persist/@home") != null)
                    and (.storageIdentities.btrfsQgroups | index("0/257 /mnt/persist") != null)
                    and (.storageIdentities.snapshots | index("tank/home@before-upgrade") != null)
                    and (.storageIdentities.caches | index("tank/l2arc0") != null)
                    and (.storageIdentities.nvmeNamespaces | index("/dev/nvme0 nsid 4") != null)
                    and (.networkStorage.iscsiSessionTargets | index("iqn.2026-06.example:storage.root") != null)
                    and (.networkStorage.iscsiSessionTargets | index("iqn.2026-06.example:storage.logout") == null)
                    and (.networkStorage.lunHostPaths | index("/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0") != null)
                    and (.networkStorage.lunHostPaths | index("/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-0") != null)
                    and (.networkStorage.lunHostPaths | index("/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-3") == null)
                    and (.networkStorage.nfsExportSelectors | index("/srv/share 192.0.2.0/24") != null)
                    and (.networkStorage.nfsExportSelectors | index("/srv/old-share 192.0.2.55") == null)
                    and .lifecycleManaged.filesystems."/mnt/local-mount".operation == "mount"
                    and .lifecycleManaged.filesystems."/mnt/local-mount".identity == "/mnt/local-mount"
                    and (.lifecycleManaged.filesystems | has("/mnt/local-unmount") | not)
                    and .lifecycleManaged.swapDevices."/dev/disk/by-label/swap".operation == "format"
                    and .lifecycleManaged.swapDevices."/dev/disk/by-label/swap".desiredSize == "8GiB"
                    and (.lifecycleManaged.swapDevices | has("/dev/disk/by-label/destroyed-swap") | not)
                    and .lifecycleManaged.physicalVolumes."/dev/disk/by-id/nvme-pv-grow".operation == "grow"
                    and .lifecycleManaged.volumes."vg0/scratch".operation == "create"
                    and .lifecycleManaged.volumes."vg0/scratch".desiredSize == "10GiB"
                    and (.lifecycleManaged.volumes | has("vg0/archive") | not)
                    and .lifecycleManaged.thinPools."vg0/thinpool".operation == "grow"
                    and .lifecycleManaged.lvmCaches."vg0/root".operation == "create"
                    and .lifecycleManaged.vdoVolumes.archive.operation == "grow"
                    and .lifecycleManaged.vdoVolumes.archive.desiredSize == "4TiB"
                    and (.lifecycleManaged.vdoVolumes | has("cold-archive") | not)
                    and .lifecycleManaged.luksKeyslots."/dev/disk/by-id/root-luks keyslot 1".operation == "add-key"
                    and (.lifecycleManaged.luksKeyslots | has("/dev/disk/by-id/root-luks keyslot 2") | not)
                    and .lifecycleManaged.btrfsQgroups."0/257 /mnt/persist".identity == "0/257 /mnt/persist"
                    and .lifecycleManaged.snapshots."tank/home@before-upgrade".operation == "create"
                    and .lifecycleManaged.luns."/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0".operation == "grow"
                    and (.lifecycleManaged.luns | has("/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-3") | not)
                    and .lifecycleManaged.iscsiSessions."iqn.2026-06.example:storage.root".operation == "grow"
                    and (.lifecycleManaged.iscsiSessions | has("iqn.2026-06.example:storage.logout") | not)
                    and .lifecycleManaged.nfsExports."/srv/share 192.0.2.0/24".operation == "export"
                    and (.lifecycleManaged.nfsExports | has("/srv/old-share 192.0.2.55") | not)
                    and .iscsi.openiscsi.enable == true
                    and .iscsi.openiscsi.discoverPortal == "192.0.2.10:3260"
                    and .iscsi.bootInitiator.enable == true
                    and .iscsi.bootInitiator.discoverPortal == "192.0.2.10:3260"
                    and (.declarativeHandoff.fileSystems | index("/mnt/local-mount") != null)
                    and (.declarativeHandoff.fileSystems | index("/mnt/local-unmount") == null)
                    and (.declarativeHandoff.swapDevices | index("/dev/disk/by-label/swap") != null)
                    and (.declarativeHandoff.swapDevices | index("/dev/disk/by-label/destroyed-swap") == null)
                    and (.declarativeHandoff.luksDevices | index("cryptroot") != null)
                    and (.declarativeHandoff.luksDevices | index("cryptclosed") == null)
                    and (.declarativeHandoff.nfsExports | index("/srv/share 192.0.2.0/24") != null)
                    and (.declarativeHandoff.nfsExports | index("/srv/old-share 192.0.2.55") == null)
                    and (.declarativeHandoff.iscsiSessions | index("iqn.2026-06.example:storage.root") != null)
                    and (.declarativeHandoff.iscsiSessions | index("iqn.2026-06.example:storage.logout") == null)
                    and .declarativeHandoff.iscsiBoot == true
                    and .declarativeHandoff.nixModule == "/etc/disk-nix/declarative-handoff.nix"
                    and .declarativeHandoff.importPatch == "/etc/disk-nix/declarative-handoff-import.patch"
                    and .declarativeHandoff.autoImport.enabled == false
                    and .declarativeHandoff.autoImport.configurationPath == "/etc/nixos/configuration.nix"
                    and .declarativeHandoff.autoImport.backupDirectory == "/var/backups/disk-nix"
                    and (.declarativeHandoff.generatedFiles | index("/etc/disk-nix/spec.json") != null)
                    and (.declarativeHandoff.generatedFiles | index("/etc/disk-nix/steady-state.json") != null)
                    and (.declarativeHandoff.generatedFiles | index("/etc/disk-nix/declarative-handoff.nix") != null)
                    and (.declarativeHandoff.generatedFiles | index("/etc/disk-nix/declarative-handoff-import.patch") != null)
                    and (.declarativeHandoff.generatedFiles | index("/run/disk-nix/apply.sh") != null)
                    and (.declarativeHandoff.generatedFiles | index("/run/disk-nix/apply-report.json") != null)
                    and (.declarativeHandoff.generatedFiles | index("/run/disk-nix/apply-receipt.json") != null)
                    and .nativeServices.lvm == true
                    and .nativeServices.lvmThin == true
                    and .nativeServices.lvmVdo == true
                    and .nativeServices.mdraid == true
                    and .nativeServices.multipath == true
                    and .nativeServices.bcache == true
                    and .nativeServices.nfsServer == true
                    and (.nativeServices.zfsExtraPools | index("tank") != null)
                    and (.nativeServices.zfsExtraPools | index("moveme") == null)
                    and (.nativeServices.zfsExtraPools | index("mnt") == null)
                  ' steady-state
                  handoffNix=${nixosModuleTest.config.environment.etc."disk-nix/declarative-handoff.nix".source}
                  grep -F -- 'Generated by services.disk-nix' "$handoffNix"
                  grep -F -- 'This file is not imported by default' "$handoffNix"
                  grep -F -- 'fileSystems = {' "$handoffNix"
                  grep -F -- '"/mnt/local-mount" = {' "$handoffNix"
                  grep -F -- 'swapDevices = [' "$handoffNix"
                  grep -F -- 'zramSwap = {' "$handoffNix"
                  grep -F -- 'luks = {' "$handoffNix"
                  grep -F -- 'devices = {' "$handoffNix"
                  grep -F -- 'supportedFilesystems = [' "$handoffNix"
                  grep -F -- 'openiscsi = {' "$handoffNix"
                  grep -F -- 'extraPools = [' "$handoffNix"
                  handoffPatch=${
                    nixosModuleTest.config.environment.etc."disk-nix/declarative-handoff-import.patch".source
                  }
                  grep -F -- 'Generated by services.disk-nix' "$handoffPatch"
                  grep -F -- 'This patch is intentionally not applied by default' "$handoffPatch"
                  grep -F -- 'imports = [' "$handoffPatch"
                  grep -F -- '/etc/disk-nix/declarative-handoff.nix' "$handoffPatch"
                  printf '%s\n' ${pkgs.lib.escapeShellArg nixosModuleTest.config.services.nfs.server.exports} > nfs-exports
                  grep -- '/srv/share 192.0.2.0/24(rw,sync,no_subtree_check)' nfs-exports
                  ! grep -- '/srv/old-share' nfs-exports
                  tuningOnlySpec=${zramTuningOnlyModuleTest.config.environment.etc."disk-nix/spec.json".source}
                  jq -e '
                    .spec.zram.swapDevices == 3
                    and .spec.zram.memoryPercent == 35
                    and .spec.zram.priority == 15
                    and .spec.zram.algorithm == "lz4"
                    and .spec.zram.preserveData == false
                    and ((.spec.zram.enable // false) == false)
                  ' "$tuningOnlySpec"
                  tuningOnlyNative=${
                    pkgs.lib.escapeShellArg (
                      builtins.toJSON {
                        zram = zramTuningOnlyModuleTest.config.zramSwap.enable;
                      }
                    )
                  }
                  printf '%s\n' "$tuningOnlyNative" > tuning-only-native-storage
                  jq -e '.zram == false' tuning-only-native-storage
                  touch "$out"
              '';
          nixosModuleExecute =
            pkgs.runCommand "disk-nix-nixos-module-execute-check" { nativeBuildInputs = [ pkgs.jq ]; }
              ''
                spec=${nixosModuleExecuteTest.config.environment.etc."disk-nix/spec.json".source}
                jq -e '
                  .apply.mode == "activation"
                  and .apply.failOnBlocked == true
                  and .apply.probeCurrent == true
                  and has("apply")
                  and (.apply | has("execute") | not)
                ' "$spec"
                applyScript='${nixosModuleExecuteTest.config.systemd.services.disk-nix-plan.serviceConfig.ExecStart}'
                grep -- 'apply' "$applyScript"
                grep -- '--execute' "$applyScript"
                grep -- '--probe-current' "$applyScript"
                grep -- '--script-out' "$applyScript"
                grep -- '/run/disk-nix/execute.sh' "$applyScript"
                grep -- '--report-out' "$applyScript"
                grep -- '/run/disk-nix/execute-report.json' "$applyScript"
                grep -- '--receipt-out' "$applyScript"
                grep -- '/run/disk-nix/execute-receipt.json' "$applyScript"
                touch "$out"
              '';
          nixosModuleHandoffAutoImport =
            pkgs.runCommand "disk-nix-nixos-module-handoff-auto-import-check"
              { nativeBuildInputs = [ pkgs.jq ]; }
              ''
                spec=${nixosModuleHandoffAutoImportTest.config.environment.etc."disk-nix/spec.json".source}
                jq -e '
                  .apply.mode == "activation"
                  and .apply.failOnBlocked == true
                  and (.apply | has("execute") | not)
                  and (.apply | has("declarativeHandoff") | not)
                ' "$spec"
                steadyState=${
                  pkgs.lib.escapeShellArg (
                    builtins.readFile
                      nixosModuleHandoffAutoImportTest.config.environment.etc."disk-nix/steady-state.json".source
                  )
                }
                printf '%s\n' "$steadyState" > steady-state
                jq -e '
                  .declarativeHandoff.autoImport.enabled == true
                  and .declarativeHandoff.autoImport.configurationPath == "/etc/nixos/storage.nix"
                  and .declarativeHandoff.autoImport.backupDirectory == "/var/backups/disk-nix-handoff"
                ' steady-state
                applyScript='${nixosModuleHandoffAutoImportTest.config.systemd.services.disk-nix-plan.serviceConfig.ExecStart}'
                grep -- 'apply' "$applyScript"
                grep -- '--execute' "$applyScript"
                grep -F -- 'config_path=/etc/nixos/storage.nix' "$applyScript"
                grep -F -- 'backup_dir=/var/backups/disk-nix-handoff' "$applyScript"
                grep -F -- 'handoff_module=/etc/disk-nix/declarative-handoff.nix' "$applyScript"
                grep -F -- 'import_patch=/etc/disk-nix/declarative-handoff-import.patch' "$applyScript"
                grep -F -- 'grep -F -q "$handoff_module" "$config_path"' "$applyScript"
                grep -F -- 'cp --preserve=mode,ownership,timestamps "$config_path" "$backup_path"' "$applyScript"
                grep -F -- 'patch --forward --backup --input="$import_patch" "$config_path"' "$applyScript"
                touch "$out"
              '';
          nixosModuleApplyModes =
            pkgs.runCommand "disk-nix-nixos-module-apply-modes-check" { nativeBuildInputs = [ pkgs.jq ]; }
              ''
                bootWarnings=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleBootModeTest.config.warnings)}
                installWarnings=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleInstallModeTest.config.warnings)}
                ! printf '%s\n' "$bootWarnings" | grep -- 'apply.mode = \\"boot\\" is reserved'
                ! printf '%s\n' "$installWarnings" | grep -- 'apply.mode = \\"install\\" is reserved'
                bootSpec=${nixosModuleBootModeTest.config.environment.etc."disk-nix/spec.json".source}
                jq -e '.apply.mode == "boot"' "$bootSpec"
                bootScript='${nixosModuleBootModeTest.config.systemd.services.disk-nix-plan.serviceConfig.ExecStart}'
                grep -- 'apply' "$bootScript"
                bootWantedBy=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleBootModeTest.config.systemd.services.disk-nix-plan.wantedBy)}
                printf '%s\n' "$bootWantedBy" | jq -e 'index("multi-user.target") != null'
                bootWants=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleBootModeTest.config.systemd.services.disk-nix-plan.wants)}
                printf '%s\n' "$bootWants" | jq -e 'index("systemd-udev-settle.service") != null'
                bootAfter=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleBootModeTest.config.systemd.services.disk-nix-plan.after)}
                printf '%s\n' "$bootAfter" | jq -e 'index("local-fs.target") != null and index("systemd-udev-settle.service") != null'
                bootBefore=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleBootModeTest.config.systemd.services.disk-nix-plan.before)}
                printf '%s\n' "$bootBefore" | jq -e 'index("multi-user.target") != null'
                installSpec=${nixosModuleInstallModeTest.config.environment.etc."disk-nix/spec.json".source}
                jq -e '.apply.mode == "install"' "$installSpec"
                installScript='${nixosModuleInstallModeTest.config.systemd.services.disk-nix-plan.serviceConfig.ExecStart}'
                grep -- 'apply' "$installScript"
                installWantedBy=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleInstallModeTest.config.systemd.services.disk-nix-plan.wantedBy)}
                printf '%s\n' "$installWantedBy" | jq -e 'index("multi-user.target") != null'
                touch "$out"
              '';
          nixosModuleAssertions = pkgs.runCommand "disk-nix-nixos-module-assertions-check" { } ''
            collisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleCollisionTest.config.system.build.toplevel).success))}
            diskCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleDiskCollisionTest.config.system.build.toplevel).success))}
            partitionCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModulePartitionCollisionTest.config.system.build.toplevel).success))}
            luksKeyslotCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleLuksKeyslotCollisionTest.config.system.build.toplevel).success))}
            luksTokenCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleLuksTokenCollisionTest.config.system.build.toplevel).success))}
            backingFileCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleBackingFileCollisionTest.config.system.build.toplevel).success))}
            btrfsSubvolumeCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleBtrfsSubvolumeCollisionTest.config.system.build.toplevel).success))}
            btrfsQgroupCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleBtrfsQgroupCollisionTest.config.system.build.toplevel).success))}
            dmMapCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleDmMapCollisionTest.config.system.build.toplevel).success))}
            vdoVolumeCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleVdoVolumeCollisionTest.config.system.build.toplevel).success))}
            physicalVolumeCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModulePhysicalVolumeCollisionTest.config.system.build.toplevel).success))}
            loopDeviceCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleLoopDeviceCollisionTest.config.system.build.toplevel).success))}
            mdRaidCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleMdRaidCollisionTest.config.system.build.toplevel).success))}
            multipathMapCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleMultipathMapCollisionTest.config.system.build.toplevel).success))}
            nvmeNamespaceCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleNvmeNamespaceCollisionTest.config.system.build.toplevel).success))}
            cacheCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleCacheCollisionTest.config.system.build.toplevel).success))}
            poolCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModulePoolCollisionTest.config.system.build.toplevel).success))}
            datasetCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleDatasetCollisionTest.config.system.build.toplevel).success))}
            zvolCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleZvolCollisionTest.config.system.build.toplevel).success))}
            volumeGroupCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleVolumeGroupCollisionTest.config.system.build.toplevel).success))}
            volumeCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleVolumeCollisionTest.config.system.build.toplevel).success))}
            thinPoolCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleThinPoolCollisionTest.config.system.build.toplevel).success))}
            lvmCacheCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleLvmCacheCollisionTest.config.system.build.toplevel).success))}
            snapshotCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleSnapshotCollisionTest.config.system.build.toplevel).success))}
            iscsiSessionCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleIscsiSessionCollisionTest.config.system.build.toplevel).success))}
            lunPathCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleLunPathCollisionTest.config.system.build.toplevel).success))}
            test "$collisionSuccess" = false
            test "$diskCollisionSuccess" = false
            test "$partitionCollisionSuccess" = false
            test "$luksKeyslotCollisionSuccess" = false
            test "$luksTokenCollisionSuccess" = false
            test "$backingFileCollisionSuccess" = false
            test "$btrfsSubvolumeCollisionSuccess" = false
            test "$btrfsQgroupCollisionSuccess" = false
            test "$dmMapCollisionSuccess" = false
            test "$vdoVolumeCollisionSuccess" = false
            test "$physicalVolumeCollisionSuccess" = false
            test "$loopDeviceCollisionSuccess" = false
            test "$mdRaidCollisionSuccess" = false
            test "$multipathMapCollisionSuccess" = false
            test "$nvmeNamespaceCollisionSuccess" = false
            test "$cacheCollisionSuccess" = false
            test "$poolCollisionSuccess" = false
            test "$datasetCollisionSuccess" = false
            test "$zvolCollisionSuccess" = false
            test "$volumeGroupCollisionSuccess" = false
            test "$volumeCollisionSuccess" = false
            test "$thinPoolCollisionSuccess" = false
            test "$lvmCacheCollisionSuccess" = false
            test "$snapshotCollisionSuccess" = false
            test "$iscsiSessionCollisionSuccess" = false
            test "$lunPathCollisionSuccess" = false
            touch "$out"
          '';
        };

        devShells.default = pkgs.mkShell {
          packages = [
            pkgs.cargo
            pkgs.clippy
            pkgs.rustc
            pkgs.rustfmt
            pkgs.rust-analyzer
            pkgs.pkg-config
            pkgs.just
            formatProgram
          ];
        };
      }
      );
    in
    {
      formatter = forAllSystems (system: perSystem.${system}.formatter);
      packages = forAllSystems (system: perSystem.${system}.packages);
      apps = forAllSystems (system: perSystem.${system}.apps);
      checks = forAllSystems (system: perSystem.${system}.checks);
      devShells = forAllSystems (system: perSystem.${system}.devShells);
      nixosModules.default = import ./nix/modules/disk-nix.nix self;
      overlays.default = final: _prev: {
        disk-nix = self.packages.${final.stdenv.hostPlatform.system}.disk-nix;
      };
    };
}
