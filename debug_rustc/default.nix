# default.nix
let
  nixpkgs = fetchTarball "https://github.com/NixOS/nixpkgs/tarball/nixos-25.11";
  pkgs = import nixpkgs { config = {}; overlays = []; };
in
{
  rustc = pkgs.callPackage ./rustc_with_debug_symbols.nix { fastCross = true; llvmShared = pkgs.llvm; llvmSharedForBuild = pkgs.llvm; llvmSharedForHost = pkgs.llvm; llvmSharedForTarget = pkgs.llvm; sha256 = "sha256-uD+SHNPzIf9hT5wGqLhw2JKZ/AKIi0ilVJaDo2gjR0w="; version = "1.94.0"; };
}