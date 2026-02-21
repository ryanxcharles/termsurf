import { join } from "path";

const publicDir = join(import.meta.dir, "public");

Bun.serve({
  port: 9616,
  async fetch(req) {
    const url = new URL(req.url);

    // Delayed resource route: sleeps for `delay` ms then returns a 1x1 PNG.
    if (url.pathname === "/slow-resource") {
      const delay = Math.min(
        Math.max(parseInt(url.searchParams.get("delay") || "500"), 0),
        30000,
      );
      await Bun.sleep(delay);
      // 1x1 transparent PNG (67 bytes).
      const png = new Uint8Array([
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00,
        0x0d, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
        0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4, 0x89,
        0x00, 0x00, 0x00, 0x0a, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x62,
        0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0xe5, 0x27, 0xde, 0xfc, 0x00,
        0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
      ]);
      return new Response(png, {
        headers: {
          "Content-Type": "image/png",
          "Cache-Control": "no-store",
        },
      });
    }

    // Slow-load route: page with many delayed subresources.
    if (url.pathname === "/slow") {
      const seconds = Math.min(
        Math.max(parseInt(url.searchParams.get("seconds") || "10"), 1),
        120,
      );
      const count = Math.min(
        Math.max(parseInt(url.searchParams.get("count") || "20"), 1),
        200,
      );
      const delayMs = Math.round((seconds * 1000) / count);

      // Build <img> tags — each loads a delayed resource.
      let imgs = "";
      for (let i = 0; i < count; i++) {
        imgs += `<img src="/slow-resource?id=${i}&delay=${delayMs}" width="1" height="1" onload="loaded()" style="position:absolute;opacity:0">\n`;
      }

      const html = `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Slow Load Test (${seconds}s, ${count} resources)</title>
<style>
* { margin: 0; padding: 0; box-sizing: border-box; }
body {
  background: #1a1b26;
  color: #c0caf5;
  font-family: system-ui, -apple-system, sans-serif;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100vh;
  gap: 24px;
}
h1 { color: #7aa2f7; font-size: 28px; }
.bar-container {
  width: 400px;
  height: 32px;
  background: #24283b;
  border-radius: 16px;
  overflow: hidden;
  border: 1px solid #565f89;
}
.bar-fill {
  height: 100%;
  width: 0%;
  background: linear-gradient(90deg, #7aa2f7, #7dcfff);
  border-radius: 16px;
  transition: width 0.3s ease;
}
.pct { font-size: 48px; color: #7dcfff; font-weight: bold; }
.status { color: #565f89; font-size: 16px; }
.done { color: #9ece6a; font-size: 24px; display: none; }
</style>
</head>
<body>
<h1>Slow Load Test</h1>
<div class="bar-container"><div class="bar-fill" id="bar"></div></div>
<div class="pct" id="pct">0%</div>
<div class="status" id="status">Loading... 0 / ${count} resources</div>
<div class="done" id="done"></div>
${imgs}
<script>
var total = ${count};
var n = 0;
var t0 = Date.now();
function loaded() {
  n++;
  var pct = Math.round((n / total) * 100);
  document.getElementById('bar').style.width = pct + '%';
  document.getElementById('pct').textContent = pct + '%';
  document.getElementById('status').textContent = 'Loading... ' + n + ' / ' + total + ' resources';
  if (n >= total) {
    var elapsed = ((Date.now() - t0) / 1000).toFixed(1);
    document.getElementById('status').style.display = 'none';
    document.getElementById('done').style.display = 'block';
    document.getElementById('done').textContent = 'Done! ' + total + ' resources in ' + elapsed + 's';
  }
}
</script>
</body>
</html>`;

      return new Response(html, {
        headers: { "Content-Type": "text/html; charset=utf-8" },
      });
    }

    // Static file serving.
    const path = url.pathname === "/" ? "/index.html" : url.pathname;
    const file = Bun.file(join(publicDir, path));

    if (await file.exists()) {
      return new Response(file);
    }

    return new Response("Not Found", { status: 404 });
  },
});

console.log("http://localhost:9616");
