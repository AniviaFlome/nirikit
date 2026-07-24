{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.programs.nirikit;
  format = pkgs.formats.toml { };
in
{
  options.programs.nirikit = {
    enable = lib.mkEnableOption "nirikit — launch applications on niri workspaces";

    package = lib.mkPackageOption pkgs "nirikit" { } // {
      description = "nirikit package to use.";
    };

    settings = lib.mkOption {
      type = format.type;
      default = { };
      description = ''
        Configuration written to
        {file}`$XDG_CONFIG_HOME/nirikit/config.toml`.
      '';
      example = lib.literalExpression ''
        {
          profiles.term3 = {
            workspace = "3";
            no-focus = true;
            silent = true;
            command = [ "kitty" ];
          };
        }
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = [ cfg.package ];

    xdg.configFile."nirikit/config.toml" = lib.mkIf (cfg.settings != { }) {
      source = format.generate "nirikit-config.toml" cfg.settings;
    };
  };
}
