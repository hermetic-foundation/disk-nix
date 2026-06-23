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
                scriptOut = "/run/disk-nix/apply.sh";
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
                  and .apply.scriptOut == "/run/disk-nix/apply.sh"
                ' "$spec"
                applyScript='${nixosModuleTest.config.systemd.services.disk-nix-plan.serviceConfig.ExecStart}'
                grep -- '--probe-current' "$applyScript"
                grep -- '--script-out' "$applyScript"
                grep -- '/run/disk-nix/apply.sh' "$applyScript"
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
