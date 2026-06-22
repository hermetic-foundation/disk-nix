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
          meta = {
            description = "NixOS-native storage topology and lifecycle manager";
            homepage = "https://github.com/midischwarz12/disk-nix";
            license = pkgs.lib.licenses.agpl3Plus;
            mainProgram = "disk-nix";
          };
        };
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
          nixosModule =
            (pkgs.nixos [
              self.nixosModules.default
              {
                system.stateVersion = "26.05";
                boot.loader.grub.enable = false;
                services.disk-nix = {
                  enable = true;
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
                  };
                  swaps.primary = {
                    device = "/dev/disk/by-label/swap";
                    priority = 5;
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
            ]).config.system.build.toplevel;
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
