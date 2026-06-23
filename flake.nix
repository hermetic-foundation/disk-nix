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
                  metadata = {
                    portal = "192.0.2.10:3260";
                  };
                };
              };
              pools.tank = {
                operation = "rebalance";
                addDevices = [ "/dev/disk/by-id/nvme-replacement" ];
                removeDevices = [ "/dev/disk/by-id/old-disk" ];
                properties.autotrim = "on";
              };
              datasets."tank/archive" = {
                destroy = true;
              };
              luns."iqn.2026-06.example:storage/root:0" = {
                operation = "grow";
                metadata = {
                  target = "iqn.2026-06.example:storage/root";
                  lun = 0;
                };
              };
              caches."tank/l2arc0" = {
                operation = "replace-device";
                replaceDevices = {
                  "/dev/disk/by-id/old-cache" = "/dev/disk/by-id/new-cache";
                };
              };
              snapshots."tank/home@before-upgrade" = {
                target = "tank/home";
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
            schema=$(mktemp)
            scriptOut=$(mktemp)

            ${diskNix}/bin/disk-nix --help | grep -- 'usage'
            ${diskNix}/bin/disk-nix schema > "$schema"
            cmp "$schema" ${diskNix}/share/disk-nix/schema/disk-nix-spec.schema.json
            jq -e '
              ."$schema" == "https://json-schema.org/draft/2020-12/schema"
              and .properties.spec["$ref"] == "#/$defs/specBody"
              and .properties.apply["$ref"] == "#/$defs/applyPolicy"
              and (."$defs".operation.enum | index("grow") != null)
              and (."$defs".operation.enum | index("replace-device") != null)
              and (."$defs".specBody.properties.snapshots["$ref"] == "#/$defs/snapshotMap")
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
              .summary.actionCount == 11
              and .summary.offlineRequiredCount == 3
              and .summary.destructiveCount == 1
              and .summary.potentialDataLossCount == 2
              and .summary.unsupportedCount == 0
              and (.actions | any(.id == "datasets:tank/archive:destroy"))
              and (.actions | any(.id == "snapshot:tank/root@rollback-point:rollback"))
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

            if ${diskNix}/bin/disk-nix apply --spec ${./examples/lifecycle-update.json} --report-out "$lifecycleApplyReport" --json > "$lifecycleApply"; then
              echo "expected lifecycle example apply to be blocked" >&2
              exit 1
            fi
            jq -e '
              .status == "blocked"
              and .apply.blockedCount == 6
              and .apply.blockedSummary.offlineRequiredCount == 3
              and .apply.blockedSummary.destructiveCount == 1
              and .apply.blockedSummary.potentialDataLossCount == 2
              and .apply.blockedSummary.unsupportedCount == 0
            ' "$lifecycleApply"
            jq -e '
              .status == "blocked"
              and .apply.blockedCount == 6
            ' "$lifecycleApplyReport"

            ${diskNix}/bin/disk-nix validate --spec ${./examples/lifecycle-update.json} --report-out "$lifecycleValidateReport" --json > "$lifecycleValidate"
            jq -e '
              .status == "blocked"
              and .apply.blockedCount == 6
              and .messages[0] == "apply policy blocked 6 action(s)"
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
