{ overlay, rust-overlay }:
{ config, pkgs, lib, ... }:
let
  cfg = config.casino.backend;
in
{
  options =
    with lib;
    with lib.types;
    {
      casino.backend = {
        enable = mkEnableOption "Enable backend service";

        bindAddr = mkOption {
          type = str;
          default = "localhost:7000";
          description = "Address to bind service to";
        };

        redisPort = mkOption {
          type = port;
          default = 6379;
          description = "Redis port to use";
        };

        csgoFloatKeyFile = mkOption {
          type = path;
          description = "Path to file containing CSGOFloat Key";
        };

        keystorePath = mkOption {
          type = path;
          description = "Path to keystore file";
        };

        logLevel = mkOption {
          type = str;
          description = "Level to loge at";
          default = "info";
        };

        package = mkOption {
          type = package;
          description = "Aggregator derivation to run";
          default = pkgs.casino.aggregator;
        };
      };
    };

  config = lib.mkIf cfg.enable {
    nixpkgs.overlays = [ rust-overlay.overlays.default overlay ];

    services.redis.servers.casino = {
      enable = true;
      port = cfg.redisPort;
    };

    systemd.services.casino = {
      description = "Casino aggregator backend";
      after = [ "redis-casino.service" ];
      wants = [ "redis-casino.service" ];
      wantedBy = [ "multi-user.target" ];

      environment = {
        REDIS_URL = "redis://127.0.0.1:${toString cfg.redisPort}";
        BIND_ADDR = cfg.bindAddr;
        KEYSTORE_PATH = cfg.keystorePath;
        LOG_LEVEL = cfg.logLevel;
      };

      serviceConfig = {
        ExecStart = ''
          ${pkgs.bash}/bin/bash -c "${cfg.package}/bin/aggregator --csgofloat-key $(cat ${cfg.csgoFloatKeyFile})"
        '';
      };
    };
  };
}
