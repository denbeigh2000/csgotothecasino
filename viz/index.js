const express = require("express");
const open = require("open");
const morgan = require("morgan");
const { createProxyMiddleware } = require("http-proxy-middleware");

// Create Express Server
const app = express();

// Configuration
const PORT = 7007;
const HOST = "localhost";
const API_SERVICE_URL = "https://casino.denb.ee";

// Logging
app.use(morgan("dev"));

// Proxy endpoints
const proxy = createProxyMiddleware({
  target: API_SERVICE_URL,
  changeOrigin: true,
});

const wsProxy = createProxyMiddleware("/api/stream", {
  target: "wss://casino.denb.ee",
  changeOrigin: true,
  logLevel: "debug",
  ws: true,
});

app.use("/api/stream", wsProxy);

app.use("/viz", express.static("src"));
app.use("/api/", proxy);

open(`http://${HOST}:${PORT}/viz/index.html`);

// Start the Proxy
app
  .listen(PORT, HOST, () => {
    console.log(`Starting Proxy at ${HOST}:${PORT}`);
  })
  .on("upgrade", wsProxy.upgrade); // <-- subscribe to http 'upgrade';
