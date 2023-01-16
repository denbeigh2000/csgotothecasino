const express = require("express");
const open = require("open");
const process = require("process");
const morgan = require("morgan");
const { createProxyMiddleware } = require("http-proxy-middleware");

// Create Express Server
const app = express();

// Use environment
const ENV = process.env.ENV || "prod";

// Configuration
const PORT = 7007;
const HOST = "localhost";
const ENVS = {
  prod: {
    API_SERVICE_URL: "https://casino.denb.ee/api",
    WS_SERVICE_URL: "wss://casino.denb.ee/api",
  },
  dev: {
    API_SERVICE_URL: "http://127.0.0.1:7000",
    WS_SERVICE_URL: "ws://127.0.0.1:7000",
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

const wsProxy = createProxyMiddleware(`/stream`, {
  target: WS_SERVICE_URL,
  changeOrigin: true,
  logLevel: "debug",
  ws: true,
});

const wsSyncProxy = createProxyMiddleware(`/sync`, {
  target: WS_SERVICE_URL,
  changeOrigin: true,
  logLevel: "debug",
  ws: true,
});

app.use(`/stream`, wsProxy);
app.use(`/sync`, wsSyncProxy);

app.use("/viz", express.static("src"));
app.use(`/`, proxy);

// open(`http://${HOST}:${PORT}/viz/index.html`);

// Start the Proxy
app
  .listen(PORT, HOST, () => {
    console.log(`Starting Proxy at ${HOST}:${PORT}`);
  })
  .on("upgrade", wsProxy.upgrade) // <-- subscribe to http 'upgrade';
  .on("upgrade", wsSyncProxy.upgrade) // <-- subscribe to http 'upgrade';
