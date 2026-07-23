{
  pkgs,
  root,
}:

{
  integrationTargetLunSmoke = pkgs.runCommand "disk-nix-integration-target-lun-smoke-check" { } ''
    ${pkgs.bash}/bin/bash -n ${root + /scripts/integration-target-lun-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${
      root + /scripts/integration-target-lun-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'targetcli /backstores/block create' ${
      root + /scripts/integration-target-lun-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'targetcli /iscsi create' ${
      root + /scripts/integration-target-lun-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'targetLuns' ${root + /scripts/integration-target-lun-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'operation: "attach"' ${root + /scripts/integration-target-lun-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'operation: "detach"' ${root + /scripts/integration-target-lun-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'targetluns:" + $target_iqn + ":attach' ${
      root + /scripts/integration-target-lun-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'targetluns:" + $target_iqn + ":detach' ${
      root + /scripts/integration-target-lun-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'destroy: true' ${root + /scripts/integration-target-lun-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'targetluns:" + $target_iqn + ":destroy' ${
      root + /scripts/integration-target-lun-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'allowDestructive=true' ${
      root + /scripts/integration-target-lun-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'lio.writeCache' ${root + /scripts/integration-target-lun-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'emulate_write_cache=0' ${
      root + /scripts/integration-target-lun-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix target-side LUN sentinel' ${
      root + /scripts/integration-target-lun-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN detach failure for disk-nix data-survival coverage' ${
      root + /scripts/integration-target-lun-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'failed-detach-apply.json' ${
      root + /scripts/integration-target-lun-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'failed-and-resumed detach data survival' ${
      root + /scripts/integration-target-lun-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'target-side LUN integration smoke test' ${
      root + /scripts/integration-target-lun-smoke.sh
    }
    touch "$out"
  '';
}
