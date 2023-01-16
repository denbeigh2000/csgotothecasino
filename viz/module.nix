{ overlay }:
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

        collector = mkOption {
          type = package;
          description = "Derivation of Windows collector to serve";
          default = pkgs.casino.collector-windows;
        };

        bootstrap = mkOption {
          type = package;
          description = "Derivation of Windows bootstrapper to serve";
          default = pkgs.casino.bootstrap-windows;
        };
      };
    };

  config = lib.mkIf cfg.enable {
    nixpkgs.overlays = [ overlay ];

    services.nginx = {
      enable = true;
      virtualHosts."casino" = {
        serverName = cfg.domainName;
        enableACME = true;
        forceSSL = true;
        acmeRoot = null;

        listen = [
          {
            addr = "0.0.0.0";
            port =  443;
            ssl = true;
          }
        ];

        locations = {
          "/api/stream" = {
            proxyPass = "http://${cfg.backend}";
            extraConfig = ''
              rewrite ^/api/(.*) /$1 break;
              proxy_http_version 1.1;

              proxy_set_header Upgrade $http_upgrade;
              proxy_set_header Connection "Upgrade";
              proxy_set_header Host $host;
            '';
          };

          "/api/sync" = {
            proxyPass = "http://${cfg.backend}";
            extraConfig = ''
              rewrite ^/api/(.*) /$1 break;
              proxy_http_version 1.1;

              proxy_set_header Upgrade $http_upgrade;
              proxy_set_header Connection "Upgrade";
              proxy_set_header Host $host;
            '';
          };

          "/api" = {
            proxyPass = "http://${cfg.backend}";
            extraConfig = ''
              rewrite ^/api/(.*) /$1 break;
            '';
          };

          "/dl/bootstrap.exe" = {
            root = "${cfg.bootstrap}/bin";
            index = "bootstrap.exe";
            extraConfig = ''
              rewrite ^/dl/(.*) /$1 break;
            '';
          };

          "/dl/collector.exe" = {
            root = "${cfg.collector}/bin";
            index = "collector.exe";
            extraConfig = ''
              rewrite ^/dl/(.*) /$1 break;
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
