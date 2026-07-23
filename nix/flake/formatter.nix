{ pkgs, self }:

let
  formatFiles = ''
    find . \
      -path ./.git -prune -o \
      -path ./target -prune -o \
      -path ./build -prune -o \
      -type f -name '*.nix' \
      -print0
  '';
  formatter = pkgs.writeShellApplication {
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
in
{
  inherit formatter;

  check =
    pkgs.runCommand "disk-nix-formatting-check"
      {
        nativeBuildInputs = [
          pkgs.findutils
          pkgs.nixfmt
        ];
      }
      ''
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
}
