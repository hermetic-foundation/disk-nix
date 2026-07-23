{ pkgs, self }:

pkgs.rustPlatform.buildRustPackage {
  pname = "disk-nix";
  version = "0.1.0";
  src = self;
  cargoLock.lockFile = ../../Cargo.lock;
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
}
