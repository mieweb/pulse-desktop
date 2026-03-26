const DEV_HOST = "127.0.0.1";
const DEV_PORT = 1420;

async function isPortOpen(host: string, port: number): Promise<boolean> {
  try {
    const connection = await Deno.connect({ hostname: host, port });
    connection.close();
    return true;
  } catch {
    return false;
  }
}

if (await isPortOpen(DEV_HOST, DEV_PORT)) {
  console.log(`ℹ️ Reusing existing Vite dev server on ${DEV_HOST}:${DEV_PORT}`);
  Deno.exit(0);
}

console.log(`▶️ Starting Vite dev server on ${DEV_HOST}:${DEV_PORT}`);

const command = new Deno.Command("npm", {
  args: ["run", "dev"],
  stdin: "inherit",
  stdout: "inherit",
  stderr: "inherit",
});

const child = command.spawn();
const status = await child.status;

Deno.exit(status.code);
