{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nixpkgs-codium.url = "github:nixos/nixpkgs?ref=91c9a64ce2a84e648d0cf9671274bb9c2fb9ba60";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  inputs.self.lfs = true;
  outputs = { self, nixpkgs, nixpkgs-codium, fenix, crane }: let
    inherit (nixpkgs) lib;
    pkgs = nixpkgs.legacyPackages."x86_64-linux";
    pkgs-codium = nixpkgs-codium.legacyPackages."x86_64-linux";
    fenixLib = fenix.packages."x86_64-linux";

    toolchain_sha = "sha256-qqF33vNuAdU5vua96VKVIwuc43j4EFeEXbjQ6+l4mO4=";

    rustToolchain = fenixLib.fromToolchainFile {
      file = ./rust-toolchain.toml;
      sha256 = toolchain_sha;
    };

    wasmToolchain = fenixLib.combine [
      (fenixLib.targets.wasm32-unknown-unknown.fromToolchainFile {
        file = ./rust-toolchain.toml;
        sha256 = toolchain_sha;
      })
      rustToolchain
    ];

    neededPackages = with pkgs; [
      wayland
      xorg.libX11
      xorg.libXcursor
      xorg.libXrandr
      xorg.libXi
      libxkbcommon

      openssl

      vulkan-headers vulkan-loader
    ];

    package_for_target = {
      target, toolchain
    }: ((crane.mkLib nixpkgs.legacyPackages.${pkgs.system}).overrideToolchain toolchain).buildPackage ({
      name = "sts2engine";

      CARGO_BUILD_TARGET = target;
      meta = {
        homepage = "https://www.github.com/BloodStainedCrow/FactoryGame/";
        maintainers = with lib.maintainers; [ BloodStainedCrow ];
        mainProgram = "sts2engine";
      };
      src = ./.;

      buildInputs = neededPackages;
      nativeBuildInputs = [ pkgs.pkg-config pkgs.makeWrapper ];
      cargoHash = "sha256-83+1Y486PUHM9+uyFw+yJ9bNMlMbN/fc8cYRzKmDdb8=";

      postInstall = ''
        wrapProgram "$out/bin/sts2engine" --prefix LD_LIBRARY_PATH : "${builtins.toString (pkgs.lib.makeLibraryPath neededPackages)}"
      '';
    });

    package = package_for_target { target = "x86_64-unknown-linux-gnu"; toolchain = rustToolchain; };
  in {

  
    devShells."x86_64-linux".codium = pkgs.mkShell {
      buildInputs = with pkgs; [
        bashInteractive
        rustToolchain

        perf
        samply
        bacon

        (vscode-with-extensions.override {
          vscode = pkgs-codium.vscodium;
          vscodeExtensions = with pkgs-codium.vscode-extensions; [
            rust-lang.rust-analyzer
            vadimcn.vscode-lldb
            gruntfuggly.todo-tree
            a5huynh.vscode-ron
          ];
        })
      ] ++ neededPackages;
      LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${builtins.toString (pkgs.lib.makeLibraryPath neededPackages)}";

      shellHook = ''
        export SHELL="${pkgs.bashInteractive}/bin/bash"
      '';
    };

    packages."x86_64-linux".default = package;
  };
}
