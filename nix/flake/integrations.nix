{
  pkgs,
  root,
  diskNix,
}:

let
  shellApplications = import ./integrations/shell-applications.nix {
    inherit pkgs root diskNix;
  };
  vmTests = import ./integrations/vm-tests.nix (
    {
      inherit pkgs root;
    }
    // shellApplications
  );
in
shellApplications // vmTests
