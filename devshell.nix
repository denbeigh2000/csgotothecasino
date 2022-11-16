{ devshell
, node-tools
, rust-toolchain
, redis
, nodejs-18_x
, tmux
, yarn2nix-moretea
, writeShellScriptBin
}:

let
  node-pkg = yarn2nix-moretea.mkYarnModules {
    pname = "dev-tools";
    version = "0.0.0";

    packageJSON = ./viz/package.json;
    yarnLock = ./viz/yarn.lock;
  };

  runInSubdir = name: subdir: text: (writeShellScriptBin name ''
    set -euo pipefail

    pushd $PRJ_ROOT/${subdir} >/dev/null
    ${text}
    popd >/dev/null
  '');

  runRust = name: target: (runInSubdir name "casino" ''
    ${rust-toolchain}/bin/cargo run --bin ${target} -- "$@"
  '');

  serve-backend = runRust "serve-backend" "aggregator";
  run-collector = runRust "run-collector" "collector";
  run-bootstrap = runRust "run-bootstrap" "bootstrap";

  run-tests = runInSubdir "run-tests" "casino" ''
    ${rust-toolchain}/bin/cargo test "$@"
  '';

  serve-web = runInSubdir "serve-web" "viz" ''
    ${nodejs-18_x}/bin/node index.js
  '';

  setup-yarn = runInSubdir "setup-yarn" "viz" ''
    node_modules=${node-pkg}/node_modules

    if [[ -d node_modules ]]
    then
      mv node_modules node_modules.old
    fi

    ${yarn2nix-moretea.linkNodeModulesHook}
  '';

in
devshell.mkShell {
  name = "dev";
  commands = [
    {
      name = "redis";
      help = "Start dev redis instance";
      category = "backend";
      command = "${redis}/bin/redis-server $@";
    }
    {
      name = "serve-backend";
      help = "Serve back-end server";
      category = "backend";
      command = "${serve-backend}/bin/serve-backend $@";
    }
    {
      name = "run-collector";
      help = "Run local stats collector";
      category = "backend";
      command = "${run-collector}/bin/run-collector $@";
    }
    {
      name = "serve-web";
      help = "Serve front-end files";
      category = "frontend";
      command = "${serve-web}/bin/serve-web $@";
    }
    {
      name = "setup-yarn";
      help = "Sets up frontend dev dependencies (clobbers existing node_modules)";
      category = "frontend";
      command = "${setup-yarn}/bin/setup-yarn $@";
    }
  ];
  env = [
    { name = "ENV"; value = "dev"; }
  ];

  packages = [ tmux redis rust-toolchain ] ++ node-tools;
}
