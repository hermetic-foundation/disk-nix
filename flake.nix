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
              };
              luks.devices.cryptroot = {
                device = "/dev/disk/by-partuuid/d024c121-4300-4493-a643-055bc4d5caa7";
                operation = "grow";
                desiredSize = "100%";
                allowDiscards = true;
                properties.label = "cryptroot";
                properties."luks.subsystem" = "nixos";
              };
              luks.devices.cryptold = {
                device = "/dev/disk/by-partuuid/old-luks";
                destroy = true;
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
              swaps.old = {
                device = "/dev/disk/by-label/old-swap";
                operation = "destroy";
              };
              nfs.mounts."/srv/shared" = {
                source = "nas.example.com:/srv/shared";
                fsType = "nfs4";
                options = [
                  "_netdev"
                  "x-systemd.automount"
                  "vers=4.2"
                ];
              };
              nfs.mounts."/srv/old" = {
                source = "nas.example.com:/srv/old";
                operation = "destroy";
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
              pools.moveme.operation = "export";
              partitions.root = {
                operation = "grow";
                device = "/dev/disk/by-id/nvme-root";
                number = "2";
                endOffset = "100%";
              };
              vdoVolumes.archive = {
                operation = "grow";
                desiredSize = "4TiB";
                properties = {
                  writePolicy = "sync";
                  compression = "enabled";
                  deduplication = "disabled";
                };
              };
              physicalVolumes."/dev/disk/by-id/nvme-pv-grow" = {
                operation = "grow";
              };
              luksKeyslots."cryptroot:1" = {
                operation = "create";
                device = "/dev/disk/by-id/root-luks";
                keySlot = "1";
                newKeyFile = "/run/keys/root-new";
              };
              luksTokens."cryptroot:0" = {
                operation = "create";
                device = "/dev/disk/by-id/root-luks";
                tokenId = "0";
                tokenFile = "/run/keys/root-token.json";
              };
              btrfsSubvolumes."/mnt/persist/@home" = {
                operation = "create";
                path = "/mnt/persist/@home";
              };
              btrfsQgroups."0/257" = {
                target = "/mnt/persist";
                properties.limit = "25GiB";
              };
              volumes."vg0/scratch" = {
                operation = "create";
                desiredSize = "10GiB";
              };
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
              datasets."tank/home-review" = {
                operation = "promote";
              };
              zvols."tank/vm/root" = {
                operation = "grow";
                desiredSize = "80GiB";
              };
              thinPools."vg0/thinpool" = {
                operation = "grow";
                desiredSize = "500GiB";
              };
              thinPools."vg0/newthin" = {
                operation = "create";
                desiredSize = "100GiB";
              };
              lvmSnapshots."vg0/root-snap" = {
                operation = "snapshot";
                target = "vg0/root";
                desiredSize = "20GiB";
              };
              lvmCaches."vg0/root" = {
                operation = "create";
                device = "vg0/root-cache";
                properties."lvm.cache-mode" = "writethrough";
              };
              loopDevices."/dev/loop7" = {
                operation = "create";
                device = "/var/lib/images/root.img";
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
              multipathMaps.mpatha = {
                target = "mpatha";
                addDevices = [ "/dev/sdb" ];
                replaceDevices = {
                  "/dev/sdc" = "/dev/sdd";
                };
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
              nvmeNamespaces."/dev/nvme0" = {
                operation = "create";
                desiredSize = "100G";
                namespaceId = "4";
                controllers = "0x1";
              };
              exports."/srv/share" = {
                operation = "create";
                client = "192.0.2.0/24";
                options = "rw,sync,no_subtree_check";
              };
              caches."tank/l2arc0" = {
                operation = "replace-device";
                replaceDevices = {
                  "/dev/disk/by-id/old-cache" = "/dev/disk/by-id/new-cache";
                };
              };
              caches."/dev/bcache0" = {
                addDevices = [ "cache-set-uuid" ];
                properties."bcache.cache-mode" = "writethrough";
              };
              snapshots."tank/home@before-upgrade" = {
                target = "tank/home";
                hold = "disk-nix-retain";
                rollback = true;
                cloneTo = "tank/home-review";
                renameTo = "tank/home@before-upgrade-retained";
                recursiveRollback = true;
              };
              snapshots."tank/home@old" = {
                target = "tank/home";
                releaseHold = "old-retention";
              };
              snapshots."/mnt/persist/@home-before-upgrade" = {
                target = "/mnt/persist/@home";
                readOnly = true;
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
      in
      {
        formatter = formatProgram;

        packages = {
          default = diskNix;
          disk-nix = diskNix;
        };

        apps.default = {
          type = "app";
          program = "${diskNix}/bin/disk-nix";
          meta = diskNix.meta;
        };

        checks = {
          inherit diskNix;
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
            schema=$(mktemp)
            scriptOut=$(mktemp)

            ${diskNix}/bin/disk-nix --help | grep -- 'usage'
            if grep -R -E 'executor-unavailable|does not mutate storage yet|future mutating executor|future `btrfs device remove`|does not run mutating storage commands directly|non-executed command' ${./README.md} ${./docs}; then
              echo "stale executor documentation found" >&2
              exit 1
            fi
            ${diskNix}/bin/disk-nix schema > "$schema"
            cmp "$schema" ${diskNix}/share/disk-nix/schema/disk-nix-spec.schema.json
            jq -e '
              ."$schema" == "https://json-schema.org/draft/2020-12/schema"
              and .properties.spec["$ref"] == "#/$defs/specBody"
              and .properties.apply["$ref"] == "#/$defs/applyPolicy"
              and .properties.swaps["$ref"] == "#/$defs/lifecycleMap"
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
              and (."$defs".operation.enum | index("grow") != null)
              and (."$defs".operation.enum | index("check") != null)
              and (."$defs".operation.enum | index("repair") != null)
              and (."$defs".operation.enum | index("scrub") != null)
              and (."$defs".operation.enum | index("trim") != null)
              and (."$defs".operation.enum | index("replace-device") != null)
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
              and (."$defs".specBody.properties.mdRaids["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.multipathMaps["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.pools["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.datasets["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.luns["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.nvmeNamespaces["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.iscsiSessions["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.exports["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.caches["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.snapshots["$ref"] == "#/$defs/snapshotMap")
              and ."$defs".snapshot.properties.readOnly.type == "boolean"
              and ."$defs".snapshot.properties.readonly.type == "boolean"
              and ."$defs".snapshot.properties.cloneTo.type == "string"
              and ."$defs".snapshot.properties.recursiveRollback.type == "boolean"
              and ."$defs".snapshot.properties."zfs.rollbackRecursive".type == "boolean"
              and (."$defs".operation.enum | index("promote") != null)
              and (."$defs".operation.enum | index("import") != null)
              and (."$defs".operation.enum | index("export") != null)
              and ."$defs".filesystem.properties.device.type == "string"
              and ."$defs".filesystem.properties.operation["$ref"] == "#/$defs/operation"
              and ."$defs".filesystem.properties.properties.type == "object"
              and ."$defs".filesystem.properties.addDevices.type == "array"
              and ."$defs".filesystem.properties.removeDevices.type == "array"
              and ."$defs".filesystem.properties.replaceDevices.type == "object"
              and ."$defs".luksSpec.properties.devices["$ref"] == "#/$defs/lifecycleMap"
              and ."$defs".nfsSpec.properties.mounts["$ref"] == "#/$defs/nfsMountMap"
              and ."$defs".nfsMount.properties.source.type == "string"
              and ."$defs".nfsMount.properties.operation["$ref"] == "#/$defs/operation"
              and ."$defs".nfsMount.properties.destroy.type == "boolean"
              and ."$defs".nfsMount.properties.options.type == "array"
              and ."$defs".iscsiSpec.properties.sessions["$ref"] == "#/$defs/lifecycleMap"
              and ."$defs".iscsiSpec.properties.boot["$ref"] == "#/$defs/iscsiBoot"
              and ."$defs".iscsiBoot.properties.loginAll.type == "boolean"
              and (."$defs".iscsiBoot.properties.extraConfig.type | index("null") != null)
              and ."$defs".lifecycleObject.properties.partitionType.type == "string"
              and (."$defs".lifecycleObject.properties.partitionNumber.type | index("string") != null)
              and (."$defs".lifecycleObject.properties.partitionNumber.type | index("number") != null)
              and (."$defs".lifecycleObject.properties.number.type | index("string") != null)
              and (."$defs".lifecycleObject.properties.startOffset.type | index("number") != null)
              and (."$defs".lifecycleObject.properties.endOffset.type | index("string") != null)
              and ."$defs".lifecycleObject.properties.level.type == "string"
              and ."$defs".lifecycleObject.properties.raidLevel.type == "string"
              and ."$defs".lifecycleObject.properties.devices.type == "array"
              and ."$defs".lifecycleObject.properties.path.type == "string"
              and ."$defs".lifecycleObject.properties.client.type == "string"
              and ."$defs".lifecycleObject.properties.portal.type == "string"
              and (."$defs".lifecycleObject.properties.namespaceId.type | index("string") != null)
              and ."$defs".lifecycleObject.properties.controllers.type == "string"
              and (."$defs".lifecycleObject.properties.keySlot.type | index("string") != null)
              and ."$defs".lifecycleObject.properties.keyFile.type == "string"
              and ."$defs".lifecycleObject.properties.newKeyFile.type == "string"
              and (."$defs".lifecycleObject.properties.tokenId.type | index("string") != null)
              and ."$defs".lifecycleObject.properties.tokenFile.type == "string"
              and ."$defs".lifecycleObject.properties.jsonFile.type == "string"
              and ."$defs".lifecycleObject.properties.options.type == "string"
              and ."$defs".applyPolicy.properties.failOnBlocked.default == true
              and ."$defs".applyPolicy.properties.allowPotentialDataLoss.default == false
              and (."$defs".applyPolicy.properties.reportOut.type | index("string") != null)
            ' "$schema"

            ${diskNix}/bin/disk-nix plan --spec ${./examples/simple-root.json} --json > "$simplePlan"
            jq -e '
              .summary.actionCount == 1
              and .summary.offlineRequiredCount == 0
              and .summary.destructiveCount == 0
              and .summary.potentialDataLossCount == 0
              and .summary.unsupportedCount == 0
              and .actions[0].id == "filesystem:root:grow"
              and .actions[0].operation == "grow"
              and .actions[0].risk == "online"
              and .actions[0].context.desiredSize == "100%"
            ' "$simplePlan"

            ${diskNix}/bin/disk-nix plan --spec ${./examples/lifecycle-update.json} --json > "$lifecyclePlan"
            jq -e '
              .summary.actionCount == 50
              and .summary.offlineRequiredCount == 14
              and .summary.destructiveCount == 3
              and .summary.potentialDataLossCount == 2
              and .summary.unsupportedCount == 0
              and (.actions | any(.id == "filesystems:home-check:check" and .risk == "offline-required"))
              and (.actions | any(.id == "filesystems:data-scrub:scrub" and .risk == "online"))
              and (.actions | any(.id == "filesystems:scratch-trim:trim" and .risk == "online"))
              and (.actions | any(.id == "btrfssubvolumes:/mnt/persist/@home:create" and .risk == "online"))
              and (.actions | any(.id == "btrfsQgroups:0/257:set-property:limit" and .risk == "safe"))
              and (.actions | any(.id == "btrfsQgroups:0/257:set-property:maxExclusive" and .risk == "safe"))
              and (.actions | any(.id == "volumes:vg0/scratch:create" and .risk == "online"))
              and (.actions | any(.id == "vdovolumes:archive:grow" and .risk == "online"))
              and (.actions | any(.id == "vdoVolumes:archive:set-property:writePolicy" and .risk == "safe"))
              and (.actions | any(.id == "vdoVolumes:archive:set-property:compression" and .risk == "safe"))
              and (.actions | any(.id == "vdoVolumes:archive:set-property:deduplication" and .risk == "safe"))
              and (.actions | any(.id == "physicalvolumes:/dev/disk/by-id/nvme-pv-grow:grow" and .risk == "online"))
              and (.actions | any(.id == "lukskeyslots:cryptroot:1:create" and .risk == "offline-required"))
              and (.actions | any(.id == "lukstokens:cryptroot:0:create" and .risk == "offline-required"))
              and (.actions | any(.id == "zvols:tank/vm/root:grow" and .risk == "online"))
              and (.actions | any(.id == "thinpools:vg0/thinpool:grow" and .risk == "online"))
              and (.actions | any(.id == "thinpools:vg0/newthin:create" and .risk == "online"))
              and (.actions | any(.id == "lvmsnapshots:vg0/root-snap:snapshot" and .risk == "reversible"))
              and (.actions | any(.id == "lvmcaches:vg0/root:create" and .risk == "offline-required"))
              and (.actions | any(.id == "lvmCaches:vg0/root:set-property:lvm.cache-mode" and .risk == "safe"))
              and (.actions | any(.id == "loopdevices:/dev/loop7:create" and .risk == "online"))
              and (.actions | any(.id == "mdRaids:root:add-device:/dev/disk/by-id/nvme-md-spare" and .risk == "online"))
              and (.actions | any(.id == "multipathMaps:mpatha:add-device:/dev/sdb" and .risk == "online"))
              and (.actions | any(.id == "partitions:root:grow" and .risk == "offline-required"))
              and (.actions | any(.id == "swaps:primary:format" and .risk == "destructive"))
              and (.actions | any(.id == "luks.devices:cryptroot:grow" and .risk == "offline-required"))
              and (.actions | any(.id == "nvmenamespaces:/dev/nvme0:create" and .risk == "destructive"))
              and (.actions | any(.id == "pools:vault:import" and .risk == "offline-required" and .context.readOnly == true))
              and (.actions | any(.id == "pools:moveme:export" and .risk == "offline-required"))
              and (.actions | any(.id == "datasets:tank/home:create" and .risk == "online"))
              and (.actions | any(.id == "datasets:tank/home-review:promote" and .risk == "offline-required"))
              and (.actions | any(.id == "datasets:tank/legacy:rename" and .risk == "offline-required"))
              and (.actions | any(.id == "datasets:tank/archive:destroy"))
              and (.actions | any(.id == "snapshot:tank/home@before-prune:rename:tank/home@retained" and .risk == "offline-required"))
              and (.actions | any(.id == "snapshot:tank/root@rollback-point:rollback"))
              and (.actions | any(.id == "exports:/srv/share:create" and .risk == "online"))
              and (.actions | any(.id == "caches:/dev/bcache0:add-device:cache-set-uuid" and .risk == "online"))
              and (.actions | any(.id == "caches:/dev/bcache0:set-property:bcache.cache-mode" and .risk == "safe"))
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

            if ${diskNix}/bin/disk-nix apply --spec ${./examples/lifecycle-update.json} --report-out "$lifecycleApplyReport" --json > "$lifecycleApply"; then
              echo "expected lifecycle example apply to be blocked" >&2
              exit 1
            fi
            jq -e '
              .status == "blocked"
              and .apply.blockedCount == 19
              and .apply.blockedSummary.offlineRequiredCount == 14
              and .apply.blockedSummary.destructiveCount == 3
              and .apply.blockedSummary.potentialDataLossCount == 2
              and .apply.blockedSummary.unsupportedCount == 0
              and (.apply.blocked | any(.id == "datasets:tank/legacy:rename"))
              and (.apply.blocked | any(.id == "datasets:tank/home-review:promote"))
              and (.apply.blocked | any(.id == "pools:vault:import"))
              and (.apply.blocked | any(.id == "pools:moveme:export"))
              and (.apply.blocked | any(.id == "snapshot:tank/home@before-prune:rename:tank/home@retained"))
            ' "$lifecycleApply"
            jq -e '
              .status == "blocked"
              and .apply.blockedCount == 19
            ' "$lifecycleApplyReport"

            ${diskNix}/bin/disk-nix validate --spec ${./examples/lifecycle-update.json} --report-out "$lifecycleValidateReport" --json > "$lifecycleValidate"
            jq -e '
              .status == "blocked"
              and .apply.blockedCount == 19
              and .messages[0] == "apply policy blocked 19 action(s)"
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
                  .spec.filesystems.root.device == "/dev/disk/by-label/nixos-root"
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
                  and .spec.filesystems.scratch.operation == "check"
                  and .spec.filesystems.scratch.device == "/dev/disk/by-label/scratch"
                  and .spec.filesystems.scrub.operation == "scrub"
                  and .spec.filesystems.scrub.device == "/dev/disk/by-label/scrub"
                  and .spec.filesystems.scrub.mountpoint == "/scrub"
                  and .spec.filesystems.trim.operation == "trim"
                  and .spec.filesystems.trim.device == "/dev/disk/by-label/trim"
                  and .spec.swaps.primary.device == "/dev/disk/by-label/swap"
                  and .spec.swaps.primary.operation == "format"
                  and .spec.swaps.primary.desiredSize == "8GiB"
                  and .spec.swaps.primary.preserveData == false
                  and .spec.swaps.primary.properties.label == "swap"
                  and .spec.swaps.primary.properties."swap.uuid" == "01234567-89ab-cdef-0123-456789abcdef"
                  and .spec.swaps.old.operation == "destroy"
                  and .spec.swaps.old.device == "/dev/disk/by-label/old-swap"
                  and .spec.luks.devices.cryptroot.device == "/dev/disk/by-partuuid/d024c121-4300-4493-a643-055bc4d5caa7"
                  and .spec.luks.devices.cryptroot.name == "cryptroot"
                  and .spec.luks.devices.cryptroot.operation == "grow"
                  and .spec.luks.devices.cryptroot.desiredSize == "100%"
                  and .spec.luks.devices.cryptroot.properties.label == "cryptroot"
                  and .spec.luks.devices.cryptroot.properties."luks.subsystem" == "nixos"
                  and .spec.luks.devices.cryptold.destroy == true
                  and .spec.luks.devices.cryptold.device == "/dev/disk/by-partuuid/old-luks"
                  and .spec.filesystems."/srv/shared".device == "nas.example.com:/srv/shared"
                  and .spec.filesystems."/srv/shared".fsType == "nfs4"
                  and (.spec.filesystems."/srv/shared".options | index("x-systemd.automount") != null)
                  and (.spec.filesystems | has("/srv/old") | not)
                  and .spec.nfs.mounts."/srv/shared".source == "nas.example.com:/srv/shared"
                  and .spec.nfs.mounts."/srv/old".source == "nas.example.com:/srv/old"
                  and .spec.nfs.mounts."/srv/old".operation == "destroy"
                  and .spec.iscsi.initiatorName == "iqn.2026-06.example:host"
                  and (.spec.iscsi | has("discoverPortal") | not)
                  and (.spec.iscsi.boot | has("discoverPortal") | not)
                  and .spec.iscsi.boot.target == "iqn.2026-06.example:storage.root"
                  and .spec.iscsi.sessions."iqn.2026-06.example:storage.root".operation == "grow"
                  and .spec.iscsi.sessions."iqn.2026-06.example:storage.root".desiredSize == "2TiB"
                  and .spec.iscsiSessions."iqn.2026-06.example:storage.root".portal == "192.0.2.10:3260"
                  and .spec.iscsiSessions."iqn.2026-06.example:storage.root".operation == "grow"
                  and .spec.iscsiSessions."iqn.2026-06.example:storage.root".desiredSize == "2TiB"
                  and .spec.luns."iqn.2026-06.example:storage/root:0".lun == 0
                  and .spec.luns."iqn.2026-06.example:storage/root:0".device == "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0"
                  and (.spec.luns."iqn.2026-06.example:storage/root:0".devices | index("/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-0") != null)
                  and .spec.nvmeNamespaces."/dev/nvme0".operation == "create"
                  and .spec.nvmeNamespaces."/dev/nvme0".desiredSize == "100G"
                  and .spec.nvmeNamespaces."/dev/nvme0".namespaceId == "4"
                  and .spec.nvmeNamespaces."/dev/nvme0".controllers == "0x1"
                  and .spec.exports."/srv/share".operation == "create"
                  and .spec.exports."/srv/share".client == "192.0.2.0/24"
                  and .spec.exports."/srv/share".options == "rw,sync,no_subtree_check"
                  and .spec.partitions.root.operation == "grow"
                  and .spec.partitions.root.device == "/dev/disk/by-id/nvme-root"
                  and .spec.partitions.root.number == "2"
                  and .spec.partitions.root.endOffset == "100%"
                  and .spec.btrfsSubvolumes."/mnt/persist/@home".operation == "create"
                  and .spec.btrfsSubvolumes."/mnt/persist/@home".path == "/mnt/persist/@home"
                  and .spec.btrfsQgroups."0/257".target == "/mnt/persist"
                  and .spec.btrfsQgroups."0/257".properties.limit == "25GiB"
                  and .spec.volumes."vg0/scratch".operation == "create"
                  and .spec.volumes."vg0/scratch".desiredSize == "10GiB"
                  and .spec.datasets."tank/home".operation == "create"
                  and .spec.vdoVolumes.archive.operation == "grow"
                  and .spec.vdoVolumes.archive.desiredSize == "4TiB"
                  and .spec.vdoVolumes.archive.properties.writePolicy == "sync"
                  and .spec.vdoVolumes.archive.properties.compression == "enabled"
                  and .spec.vdoVolumes.archive.properties.deduplication == "disabled"
                  and .spec.physicalVolumes."/dev/disk/by-id/nvme-pv-grow".operation == "grow"
                  and .spec.luksKeyslots."cryptroot:1".operation == "create"
                  and .spec.luksKeyslots."cryptroot:1".device == "/dev/disk/by-id/root-luks"
                  and .spec.luksKeyslots."cryptroot:1".keySlot == "1"
                  and .spec.luksKeyslots."cryptroot:1".newKeyFile == "/run/keys/root-new"
                  and .spec.luksTokens."cryptroot:0".operation == "create"
                  and .spec.luksTokens."cryptroot:0".device == "/dev/disk/by-id/root-luks"
                  and .spec.luksTokens."cryptroot:0".tokenId == "0"
                  and .spec.luksTokens."cryptroot:0".tokenFile == "/run/keys/root-token.json"
                  and .spec.zvols."tank/vm/root".operation == "grow"
                  and .spec.zvols."tank/vm/root".desiredSize == "80GiB"
                  and .spec.thinPools."vg0/thinpool".operation == "grow"
                  and .spec.thinPools."vg0/thinpool".desiredSize == "500GiB"
                  and .spec.thinPools."vg0/newthin".operation == "create"
                  and .spec.thinPools."vg0/newthin".desiredSize == "100GiB"
                  and .spec.lvmSnapshots."vg0/root-snap".operation == "snapshot"
                  and .spec.lvmSnapshots."vg0/root-snap".target == "vg0/root"
                  and .spec.lvmSnapshots."vg0/root-snap".desiredSize == "20GiB"
                  and .spec.lvmCaches."vg0/root".operation == "create"
                  and .spec.lvmCaches."vg0/root".device == "vg0/root-cache"
                  and .spec.lvmCaches."vg0/root".properties."lvm.cache-mode" == "writethrough"
                  and .spec.loopDevices."/dev/loop7".operation == "create"
                  and .spec.loopDevices."/dev/loop7".device == "/var/lib/images/root.img"
                  and .spec.mdRaids.root.target == "/dev/md/root"
                  and .spec.mdRaids.root.raidLevel == "1"
                  and (.spec.mdRaids.root.devices | index("/dev/disk/by-id/nvme-md-a") != null)
                  and (.spec.mdRaids.root.devices | index("/dev/disk/by-id/nvme-md-b") != null)
                  and (.spec.mdRaids.root.addDevices | index("/dev/disk/by-id/nvme-md-spare") != null)
                  and .spec.mdRaids.root.replaceDevices."/dev/disk/by-id/nvme-md-aging" == "/dev/disk/by-id/nvme-md-replacement"
                  and .spec.multipathMaps.mpatha.target == "mpatha"
                  and (.spec.multipathMaps.mpatha.addDevices | index("/dev/sdb") != null)
                  and .spec.multipathMaps.mpatha.replaceDevices."/dev/sdc" == "/dev/sdd"
                  and (.spec.caches."/dev/bcache0".addDevices | index("cache-set-uuid") != null)
                  and .spec.caches."/dev/bcache0".properties."bcache.cache-mode" == "writethrough"
                  and .spec.pools.vault.operation == "import"
                  and .spec.pools.vault.readOnly == true
                  and .spec.pools.moveme.operation == "export"
                  and .spec.datasets."tank/home-review".operation == "promote"
                  and .spec.snapshots."tank/home@before-upgrade".target == "tank/home"
                  and .spec.snapshots."tank/home@before-upgrade".hold == "disk-nix-retain"
                  and .spec.snapshots."tank/home@before-upgrade".rollback == true
                  and .spec.snapshots."tank/home@before-upgrade".cloneTo == "tank/home-review"
                  and .spec.snapshots."tank/home@before-upgrade".renameTo == "tank/home@before-upgrade-retained"
                  and .spec.snapshots."tank/home@before-upgrade".recursiveRollback == true
                  and .spec.datasets."tank/legacy".renameTo == "tank/legacy-staged"
                  and .spec.snapshots."tank/home@old".releaseHold == "old-retention"
                  and .spec.snapshots."/mnt/persist/@home-before-upgrade".target == "/mnt/persist/@home"
                  and .spec.snapshots."/mnt/persist/@home-before-upgrade".readOnly == true
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
                ' "$spec"
                applyScript='${nixosModuleTest.config.systemd.services.disk-nix-plan.serviceConfig.ExecStart}'
                grep -- 'validate' "$applyScript"
                grep -- '--probe-current' "$applyScript"
                grep -- '--script-out' "$applyScript"
                grep -- '/run/disk-nix/apply.sh' "$applyScript"
                grep -- '--report-out' "$applyScript"
                grep -- '/run/disk-nix/apply-report.json' "$applyScript"
                printf '%s\n' ${pkgs.lib.escapeShellArgs (map toString nixosModuleTest.config.systemd.services.disk-nix-plan.path)} > service-paths
                grep -- 'bcachefs-tools-' service-paths
                grep -- 'btrfs-progs-' service-paths
                grep -- 'dosfstools-' service-paths
                grep -- 'exfatprogs-' service-paths
                grep -- 'f2fs-tools-' service-paths
                grep -- 'lvm2-' service-paths
                grep -- 'ntfs3g-' service-paths
                grep -- 'open-iscsi-' service-paths
                grep -- 'zfs-user-' service-paths
                swapDevices=${
                  pkgs.lib.escapeShellArg (
                    builtins.toJSON (map (swap: { inherit (swap) device; }) nixosModuleTest.config.swapDevices)
                  )
                }
                printf '%s\n' "$swapDevices" > swap-devices
                jq -e '
                  length == 1
                  and .[0].device == "/dev/disk/by-label/swap"
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
                  and (has("cryptold") | not)
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
                  and (has("/srv/old") | not)
                ' file-systems
                supportedFilesystems=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleTest.config.boot.supportedFilesystems)}
                printf '%s\n' "$supportedFilesystems" > supported-filesystems
                jq -e '
                  .xfs == true
                  and .btrfs == true
                  and .bcachefs == true
                  and .f2fs == true
                  and .nfs4 == true
                  and .zfs == true
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
                      bcache = nixosModuleTest.config.boot.bcache.enable;
                      bcacheInitrd = nixosModuleTest.config.boot.initrd.services.bcache.enable;
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
                  and .bcache == true
                  and .bcacheInitrd == true
                  and .openIscsiDiscoverPortal == "192.0.2.10:3260"
                  and .bootIscsiDiscoverPortal == "192.0.2.10:3260"
                ' native-storage
                printf '%s\n' ${pkgs.lib.escapeShellArg nixosModuleTest.config.services.nfs.server.exports} > nfs-exports
                grep -- '/srv/share 192.0.2.0/24(rw,sync,no_subtree_check)' nfs-exports
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
                touch "$out"
              '';
          nixosModuleReservedModes = pkgs.runCommand "disk-nix-nixos-module-reserved-modes-check" { } ''
            bootWarnings=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleBootModeTest.config.warnings)}
            installWarnings=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleInstallModeTest.config.warnings)}
            printf '%s\n' "$bootWarnings" | grep -- 'apply.mode = \\"boot\\" is reserved'
            printf '%s\n' "$installWarnings" | grep -- 'apply.mode = \\"install\\" is reserved'
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
            pkgs.jujutsu
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
        disk-nix = self.packages.${final.system}.disk-nix;
      };
    };
}
