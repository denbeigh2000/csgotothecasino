const express = require("express");
const open = require("open");
const morgan = require("morgan");
const { createProxyMiddleware } = require("http-proxy-middleware");

// Create Express Server
const app = express();

// Use environment
const ENV = "prod";

// Configuration
const PORT = 7007;
const HOST = "localhost";
const ENVS = {
    prod: {
        API_SERVICE_URL: "https://casino.denb.ee",
        WS_SERVICE_URL: "wss://casino.denb.ee",
        SERVICE_URL_PREFIX: "api/",
    },
    dev: {
        API_SERVICE_URL: "http://127.0.0.1:7000",
        WS_SERVICE_URL: "ws://127.0.0.1:7000",
        SERVICE_URL_PREFIX: "",
    }
}
const { API_SERVICE_URL, WS_SERVICE_URL, SERVICE_URL_PREFIX } = ENVS[ENV];

// Logging
app.use(morgan("dev"));

// Proxy endpoints
const proxy = createProxyMiddleware({
  target: API_SERVICE_URL,
  changeOrigin: true,
});

const wsProxy = createProxyMiddleware(`/${SERVICE_URL_PREFIX}stream`, {
  target: WS_SERVICE_URL,
  changeOrigin: true,
  logLevel: "debug",
  ws: true,
});

app.use(`/${SERVICE_URL_PREFIX}stream`, wsProxy);

app.use("/viz", express.static("src"));
app.use(`/${SERVICE_URL_PREFIX}`, proxy);

open(`http://${HOST}:${PORT}/viz/index.html`);

// Start the Proxy
app
  .listen(PORT, HOST, () => {
    console.log(`Starting Proxy at ${HOST}:${PORT}`);
  })
  .on("upgrade", wsProxy.upgrade); // <-- subscribe to http 'upgrade';
