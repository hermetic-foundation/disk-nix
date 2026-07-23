{ pkgs, self }:

{
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
}
