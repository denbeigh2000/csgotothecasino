const express = require("express");
const open = require("open");
const morgan = require("morgan");
const { createProxyMiddleware } = require("http-proxy-middleware");

// Create Express Server
const app = express();

// Configuration
const PORT = 7007;
const HOST = "localhost";
const API_SERVICE_URL = "http://localhost:7000";

// Logging
app.use(morgan("dev"));

// Proxy endpoints
const proxy = createProxyMiddleware({
  target: API_SERVICE_URL,
  changeOrigin: true,
});

const wsProxy = createProxyMiddleware("/stream", {
  target: API_SERVICE_URL,
  changeOrigin: true,
  logLevel: "debug",
  ws: true,
});

app.use(wsProxy);

app.use("/viz", express.static("src"));
app.use("/", proxy);

open(`http://${HOST}:${PORT}/viz/index.html`);

// Start the Proxy
app
  .listen(PORT, HOST, () => {
    console.log(`Starting Proxy at ${HOST}:${PORT}`);
  })
  .on("upgrade", wsProxy.upgrade); // <-- subscribe to http 'upgrade';
