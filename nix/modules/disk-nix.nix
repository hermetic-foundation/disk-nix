self:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.disk-nix;
  json = pkgs.formats.json { };
in
{
  options.services.disk-nix = {
    enable = lib.mkEnableOption "disk-nix storage lifecycle integration";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.system}.disk-nix;
      defaultText = lib.literalExpression "inputs.disk-nix.packages.${pkgs.system}.disk-nix";
      description = "disk-nix CLI package used by the NixOS module.";
    };

    spec = lib.mkOption {
      type = json.type;
      default = { };
      description = ''
        Desired storage declaration emitted as JSON for the disk-nix planner.
        This is intentionally broad while the typed NixOS option hierarchy is
        developed.
      '';
    };

    apply = {
      mode = lib.mkOption {
        type = lib.types.enum [
          "manual"
          "activation"
          "boot"
          "install"
        ];
        default = "manual";
        description = "When disk-nix may perform imperative storage actions.";
      };

      allowDestructive = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Allow destructive storage actions such as wipe, format, or destroy.";
      };

      allowFormat = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Allow formatting filesystems.";
      };

      allowShrink = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Allow shrink operations.";
      };

      allowGrow = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Allow non-destructive grow operations.";
      };

      allowPropertyChanges = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Allow non-destructive storage property changes.";
      };
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];

    environment.etc."disk-nix/spec.json".source = json.generate "disk-nix-spec.json" {
      inherit (cfg) spec;
      apply = cfg.apply;
    };

    systemd.services.disk-nix-plan = {
      description = "Plan disk-nix storage changes";
      wantedBy = lib.mkIf (cfg.apply.mode == "activation") [ "multi-user.target" ];
      serviceConfig = {
        Type = "oneshot";
        ExecStart = "${lib.getExe cfg.package} plan --spec /etc/disk-nix/spec.json";
      };
    };

    assertions = [
      {
        assertion = !(cfg.apply.allowDestructive && cfg.apply.mode == "activation");
        message = "disk-nix refuses destructive activation-mode storage changes.";
      }
    ];
  };
}
