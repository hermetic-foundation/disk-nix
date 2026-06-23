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
            boot.loader.grub.enable = false;
            boot.initrd.systemd.enable = false;
            services.disk-nix = {
              enable = true;
              apply = {
                mode = "activation";
                probeCurrent = true;
                allowDeviceReplacement = true;
                allowRebalance = true;
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
              };
              filesystems.root = {
                device = "/dev/disk/by-label/nixos-root";
                fsType = "xfs";
                mountpoint = "/";
                neededForBoot = true;
                resizePolicy = "grow-only";
                desiredSize = "100%";
              };
              swaps.primary = {
                device = "/dev/disk/by-label/swap";
                operation = "format";
                desiredSize = "8GiB";
                priority = 5;
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
              iscsi = {
                initiatorName = "iqn.2026-06.example:host";
                discoverPortal = "192.0.2.10:3260";
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
              partitions.root = {
                operation = "grow";
                device = "/dev/disk/by-id/nvme-root";
                number = "2";
                endOffset = "100%";
              };
              vdoVolumes.archive = {
                operation = "grow";
                desiredSize = "4TiB";
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
              };
              multipathMaps.mpatha = {
                target = "mpatha";
                addDevices = [ "/dev/sdb" ];
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
            if grep -R -E 'executor-unavailable|does not mutate storage yet|future mutating executor|does not run mutating storage commands directly|non-executed command' ${./README.md} ${./docs}; then
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
              and .properties.partitions["$ref"] == "#/$defs/lifecycleMap"
              and .properties.btrfsSubvolumes["$ref"] == "#/$defs/lifecycleMap"
              and .properties.btrfsQgroups["$ref"] == "#/$defs/lifecycleMap"
              and .properties.vdoVolumes["$ref"] == "#/$defs/lifecycleMap"
              and .properties.zvols["$ref"] == "#/$defs/lifecycleMap"
              and .properties.thinPools["$ref"] == "#/$defs/lifecycleMap"
              and .properties.lvmSnapshots["$ref"] == "#/$defs/lifecycleMap"
              and .properties.loopDevices["$ref"] == "#/$defs/lifecycleMap"
              and .properties.mdRaids["$ref"] == "#/$defs/lifecycleMap"
              and .properties.multipathMaps["$ref"] == "#/$defs/lifecycleMap"
              and (."$defs".operation.enum | index("grow") != null)
              and (."$defs".operation.enum | index("replace-device") != null)
              and (."$defs".specBody.properties.luks["$ref"] == "#/$defs/luksSpec")
              and (."$defs".specBody.properties.disks["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.btrfsSubvolumes["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.btrfsQgroups["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.vdoVolumes["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.zvols["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.thinPools["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.lvmSnapshots["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.loopDevices["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.mdRaids["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.multipathMaps["$ref"] == "#/$defs/lifecycleMap")
              and (."$defs".specBody.properties.snapshots["$ref"] == "#/$defs/snapshotMap")
              and ."$defs".snapshot.properties.readOnly.type == "boolean"
              and ."$defs".snapshot.properties.readonly.type == "boolean"
              and ."$defs".filesystem.properties.device.type == "string"
              and ."$defs".filesystem.properties.operation["$ref"] == "#/$defs/operation"
              and ."$defs".filesystem.properties.properties.type == "object"
              and ."$defs".filesystem.properties.addDevices.type == "array"
              and ."$defs".filesystem.properties.removeDevices.type == "array"
              and ."$defs".filesystem.properties.replaceDevices.type == "object"
              and ."$defs".luksSpec.properties.devices["$ref"] == "#/$defs/lifecycleMap"
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
              and ."$defs".lifecycleObject.properties.options.type == "string"
              and ."$defs".applyPolicy.properties.failOnBlocked.default == true
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
              .summary.actionCount == 30
              and .summary.offlineRequiredCount == 5
              and .summary.destructiveCount == 2
              and .summary.potentialDataLossCount == 2
              and .summary.unsupportedCount == 0
              and (.actions | any(.id == "btrfssubvolumes:/mnt/persist/@home:create" and .risk == "online"))
              and (.actions | any(.id == "btrfsQgroups:0/257:set-property:limit" and .risk == "safe"))
              and (.actions | any(.id == "btrfsQgroups:0/257:set-property:maxExclusive" and .risk == "safe"))
              and (.actions | any(.id == "volumes:vg0/scratch:create" and .risk == "online"))
              and (.actions | any(.id == "vdovolumes:archive:grow" and .risk == "online"))
              and (.actions | any(.id == "zvols:tank/vm/root:grow" and .risk == "online"))
              and (.actions | any(.id == "thinpools:vg0/thinpool:grow" and .risk == "online"))
              and (.actions | any(.id == "thinpools:vg0/newthin:create" and .risk == "online"))
              and (.actions | any(.id == "lvmsnapshots:vg0/root-snap:snapshot" and .risk == "reversible"))
              and (.actions | any(.id == "loopdevices:/dev/loop7:create" and .risk == "online"))
              and (.actions | any(.id == "mdRaids:root:add-device:/dev/disk/by-id/nvme-md-spare" and .risk == "online"))
              and (.actions | any(.id == "multipathMaps:mpatha:add-device:/dev/sdb" and .risk == "online"))
              and (.actions | any(.id == "partitions:root:grow" and .risk == "offline-required"))
              and (.actions | any(.id == "swaps:primary:format" and .risk == "destructive"))
              and (.actions | any(.id == "luks.devices:cryptroot:grow" and .risk == "offline-required"))
              and (.actions | any(.id == "datasets:tank/home:create" and .risk == "online"))
              and (.actions | any(.id == "datasets:tank/archive:destroy"))
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
              and .apply.blockedCount == 9
              and .apply.blockedSummary.offlineRequiredCount == 5
              and .apply.blockedSummary.destructiveCount == 2
              and .apply.blockedSummary.potentialDataLossCount == 2
              and .apply.blockedSummary.unsupportedCount == 0
            ' "$lifecycleApply"
            jq -e '
              .status == "blocked"
              and .apply.blockedCount == 9
            ' "$lifecycleApplyReport"

            ${diskNix}/bin/disk-nix validate --spec ${./examples/lifecycle-update.json} --report-out "$lifecycleValidateReport" --json > "$lifecycleValidate"
            jq -e '
              .status == "blocked"
              and .apply.blockedCount == 9
              and .messages[0] == "apply policy blocked 9 action(s)"
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
                  and .spec.swaps.primary.device == "/dev/disk/by-label/swap"
                  and .spec.swaps.primary.operation == "format"
                  and .spec.swaps.primary.desiredSize == "8GiB"
                  and .spec.swaps.primary.preserveData == false
                  and .spec.luks.devices.cryptroot.device == "/dev/disk/by-partuuid/d024c121-4300-4493-a643-055bc4d5caa7"
                  and .spec.luks.devices.cryptroot.name == "cryptroot"
                  and .spec.luks.devices.cryptroot.operation == "grow"
                  and .spec.luks.devices.cryptroot.desiredSize == "100%"
                  and .spec.filesystems."/srv/shared".device == "nas.example.com:/srv/shared"
                  and .spec.filesystems."/srv/shared".fsType == "nfs4"
                  and (.spec.filesystems."/srv/shared".options | index("x-systemd.automount") != null)
                  and .spec.nfs.mounts."/srv/shared".source == "nas.example.com:/srv/shared"
                  and .spec.iscsi.initiatorName == "iqn.2026-06.example:host"
                  and .spec.iscsi.discoverPortal == "192.0.2.10:3260"
                  and .spec.iscsi.boot.target == "iqn.2026-06.example:storage.root"
                  and .spec.iscsi.sessions."iqn.2026-06.example:storage.root".operation == "grow"
                  and .spec.iscsi.sessions."iqn.2026-06.example:storage.root".desiredSize == "2TiB"
                  and .spec.iscsiSessions."iqn.2026-06.example:storage.root".portal == "192.0.2.10:3260"
                  and .spec.iscsiSessions."iqn.2026-06.example:storage.root".operation == "grow"
                  and .spec.iscsiSessions."iqn.2026-06.example:storage.root".desiredSize == "2TiB"
                  and .spec.luns."iqn.2026-06.example:storage/root:0".lun == 0
                  and .spec.luns."iqn.2026-06.example:storage/root:0".device == "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0"
                  and (.spec.luns."iqn.2026-06.example:storage/root:0".devices | index("/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-0") != null)
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
                  and .spec.zvols."tank/vm/root".operation == "grow"
                  and .spec.zvols."tank/vm/root".desiredSize == "80GiB"
                  and .spec.thinPools."vg0/thinpool".operation == "grow"
                  and .spec.thinPools."vg0/thinpool".desiredSize == "500GiB"
                  and .spec.thinPools."vg0/newthin".operation == "create"
                  and .spec.thinPools."vg0/newthin".desiredSize == "100GiB"
                  and .spec.lvmSnapshots."vg0/root-snap".operation == "snapshot"
                  and .spec.lvmSnapshots."vg0/root-snap".target == "vg0/root"
                  and .spec.lvmSnapshots."vg0/root-snap".desiredSize == "20GiB"
                  and .spec.loopDevices."/dev/loop7".operation == "create"
                  and .spec.loopDevices."/dev/loop7".device == "/var/lib/images/root.img"
                  and .spec.mdRaids.root.target == "/dev/md/root"
                  and .spec.mdRaids.root.raidLevel == "1"
                  and (.spec.mdRaids.root.devices | index("/dev/disk/by-id/nvme-md-a") != null)
                  and (.spec.mdRaids.root.devices | index("/dev/disk/by-id/nvme-md-b") != null)
                  and (.spec.mdRaids.root.addDevices | index("/dev/disk/by-id/nvme-md-spare") != null)
                  and .spec.multipathMaps.mpatha.target == "mpatha"
                  and (.spec.multipathMaps.mpatha.addDevices | index("/dev/sdb") != null)
                  and (.spec.caches."/dev/bcache0".addDevices | index("cache-set-uuid") != null)
                  and .spec.caches."/dev/bcache0".properties."bcache.cache-mode" == "writethrough"
                  and .spec.snapshots."tank/home@before-upgrade".target == "tank/home"
                  and .spec.snapshots."tank/home@before-upgrade".hold == "disk-nix-retain"
                  and .spec.snapshots."tank/home@old".releaseHold == "old-retention"
                  and .spec.snapshots."/mnt/persist/@home-before-upgrade".target == "/mnt/persist/@home"
                  and .spec.snapshots."/mnt/persist/@home-before-upgrade".readOnly == true
                  and .apply.mode == "activation"
                  and .apply.allowGrow == true
                  and .apply.allowOffline == false
                  and .apply.probeCurrent == true
                  and .apply.allowDeviceReplacement == true
                  and .apply.allowRebalance == true
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
