{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    crane.url = "github:ipetkov/crane";

    pre-commit-hooks-nix = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    blueprint = {
      url = "github:numtide/blueprint";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.systems.follows = "";
    };
  };

  outputs = inputs: let
    systems = ["x86_64-linux" "aarch64-linux"];
    bp = inputs.blueprint {
      inherit inputs systems;
      prefix = "nix/";
    };
    packages = builtins.listToAttrs (map (system: {
      name = system;
      value = bp.packages.${system} // {default = bp.packages.${system}.mujmap;};
    })
    systems);
    apps = builtins.listToAttrs (map (system: let
      mujmap = bp.packages.${system}.mujmap;
      app = {
        type = "app";
        program = "${mujmap}/bin/mujmap";
      };
    in {
      name = system;
      value = {
        mujmap = app;
        default = app;
      };
    })
    systems);
  in
    bp
    // {
      inherit packages apps;
      overlays.default = final: _prev: bp.mkPackagesFor final;
    };
}
