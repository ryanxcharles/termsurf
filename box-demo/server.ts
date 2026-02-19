const file = Bun.file(import.meta.dir + "/public/index.html");

Bun.serve({
  port: 9407,
  fetch() {
    return new Response(file, {
      headers: { "Content-Type": "text/html" },
    });
  },
});

console.log("http://localhost:9407");
