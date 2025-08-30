{
  description = "Zencan CAN bus library and tools";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    let
      system = flake-utils.lib.system.aarch64-linux;
    in 
    flake-utils.lib.eachSystem [system] (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        zencan = pkgs.rustPlatform.buildRustPackage {
          pname = "zencan";
          version = "0.1.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
            outputHashes = {
              "socketcan-3.5.0" = "sha256-Nfg22TOtvKr26m/jN632Z/SNsuxOGQd5hcqvwicqaLg=";
            };
          };


          nativeBuildInputs = with pkgs; [
            pkg-config
            rustToolchain
          ];

          buildInputs = with pkgs; [
            
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
          ];

          # Skip tests that require CAN hardware
          doCheck = false;

          meta = with pkgs.lib; {
            description = "CAN bus library and tools for embedded systems";
            homepage = "https://github.com/mcbridejc/zencan";
            license = licenses.mpl20;
            maintainers = [ ];
          };
        };
      in
      {
        packages = {
          default = zencan;
          zencan = zencan;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            pkg-config
          ];

          shellHook = ''
            echo "Zencan development environment"
          '';
        };
      });
}
