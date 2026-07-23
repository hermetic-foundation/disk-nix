args@{
  pkgs,
  self,
  root,
  diskNix,
  integrationDiskoExamples,
  ...
}:

let
  chunks = [
    (import ./documentation-checks/part-01.nix args)
    (import ./documentation-checks/part-02.nix args)
    (import ./documentation-checks/part-03.nix args)
    (import ./documentation-checks/part-04.nix args)
  ];
in
{
  documentation = pkgs.runCommand "disk-nix-documentation-check" { } (
    builtins.concatStringsSep "\n" (
      chunks
      ++ [
        ''
          touch "$out"
        ''
      ]
    )
  );
}
