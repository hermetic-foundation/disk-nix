{
  pkgs,
  root,
  integrationVmSmoke,
  integrationDiskoExamples,
  ...
}:

{
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
          "ext4"
          "dm_mod"
          "dm_crypt"
          "md_mod"
          "raid1"
          "bcache"
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
  integrationDiskoKernelExamplesVmTest = pkgs.testers.nixosTest {
    name = "disk-nix-disko-kernel-examples-vm-test";
    nodes.machine =
      { pkgs, ... }:
      {
        system.stateVersion = "26.05";
        networking.hostId = "8425e349";
        virtualisation = {
          diskSize = 4096;
          emptyDiskImages = [
            65536
            65536
            65536
            65536
            65536
          ];
          memorySize = 4096;
        };
        boot.supportedFilesystems = [
          "bcachefs"
          "zfs"
        ];
        boot.kernelModules = [
          "loop"
          "bcachefs"
          "zfs"
        ];
        environment.systemPackages = [
          integrationDiskoExamples
          pkgs.coreutils
          pkgs.kmod
          pkgs.util-linux
        ];
      };
    testScript = ''
      machine.start()
      machine.wait_for_unit("multi-user.target")
      machine.succeed("modprobe bcachefs")
      machine.succeed("modprobe zfs")
      machine.succeed("lsblk -o NAME,PATH,SIZE,TYPE,FSTYPE,MOUNTPOINTS /dev/vdb /dev/vdc /dev/vdd /dev/vde /dev/vdf")
      machine.succeed("printf '%s\n' disk-nix-e2e-passphrase > /tmp/secret.key")
      machine.succeed("mkdir -p /tmp/disko-kernel-examples")
      for spec in [
        "bcachefs.json",
        "complex.json",
        "non-root-zfs.json",
        "zfs-encrypted-root.json",
        "zfs-over-legacy.json",
        "zfs-with-vdevs.json",
        "zfs.json",
      ]:
          machine.succeed(f"cp ${root + /examples/disko}/{spec} /tmp/disko-kernel-examples/{spec}")
      machine.succeed(
          "DISK_NIX_DISKO_EXAMPLES_DIR=/tmp/disko-kernel-examples "
          "DISK_NIX_DISKO_E2E_DEVICES='/dev/vdb /dev/vdc /dev/vdd /dev/vde /dev/vdf' "
          "DISK_NIX_DISKO_E2E_EXECUTE=1 "
          "DISK_NIX_DISKO_E2E_REQUIRE_ALL_KERNELS=1 "
          "DISK_NIX_DISKO_E2E_CONFIRM='wipe-/dev/vdb-/dev/vdc-/dev/vdd-/dev/vde-/dev/vdf' "
          "disk-nix-integration-disko-examples"
      )
    '';
  };
}
