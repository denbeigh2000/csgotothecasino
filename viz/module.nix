{ overlay, rust-overlay }:
{ config, pkgs, lib, ... }:
let
  cfg = config.casino.frontend;
in
{
  options =
    with lib;
    with lib.types;
    {
      casino.frontend = {
        enable = mkEnableOption "Enable web service";
        backend = mkOption {
          type = str;
          default = "localhost:7000";
          description = "host:port of backend service";
        };
        domainName = mkOption {
          type = str;
          default = "casino.denb.ee";
          description = "Domain name pointing to web service";
        };
      };
    };

  config = lib.mkIf cfg.enable {
    nixpkgs.overlays = [ rust-overlay.overlays.default overlay ];

    services.nginx = {
      enable = true;
      virtualHosts."casino" = {
        serverName = cfg.domainName;

        locations = {
          "/api" = {
            proxyPass = "http://${cfg.backend}";
            extraConfig = ''
              rewrite ^/api/(.*) /$1 break;
            '';
          };

          "/" = {
            root = "${pkgs.viz}/share/www";
            index = "index.html";
          };
        };
      };
    };
  };
}
