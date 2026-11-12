import assert from "node:assert/strict";

export async function connectDesktop(port, url) {
  let page;
  for (let attempt = 0; attempt < 200 && !page; attempt += 1) {
    try {
      const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
      page = targets.find((target) => target.type === "page" && target.url.startsWith(`${url}/`));
    } catch {
      // WebView2 is still starting.
    }
    if (!page) await new Promise((resolve) => setTimeout(resolve, 100));
  }
  assert.ok(page, "isolated desktop WebView did not start");
  const socket = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    socket.addEventListener("open", resolve, { once: true });
    socket.addEventListener("error", reject, { once: true });
  });

  let nextId = 0;
  const pending = new Map();
  socket.addEventListener("message", ({ data }) => {
    const reply = JSON.parse(data);
    const request = pending.get(reply.id);
    if (!request) return;
    pending.delete(reply.id);
    if (reply.error) request.reject(new Error(reply.error.message));
    else request.resolve(reply.result);
  });

  function send(method, params = {}) {
    return new Promise((resolve, reject) => {
      const id = ++nextId;
      pending.set(id, { resolve, reject });
      socket.send(JSON.stringify({ id, method, params }));
    });
  }

  async function evaluate(expression) {
    const reply = await send("Runtime.evaluate", {
      expression,
      awaitPromise: true,
      returnByValue: true,
    });
    if (reply.exceptionDetails) {
      throw new Error(reply.exceptionDetails.exception?.description ?? "Desktop evaluation failed");
    }
    return reply.result.value;
  }

  return { send, evaluate, disconnect: () => socket.close() };
}

export async function writeFixture(evaluate, repo, mode) {
  const result = await evaluate(`(async () => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    const health = await invoke('get_database_health');
    if (!health.ok) throw new Error('isolated database unhealthy');
    const original = await invoke('list_projects');
    if (${JSON.stringify(mode)} === 'fresh' && original.length !== 0) {
      throw new Error('fresh test database already contains projects');
    }
    const project = await invoke('import_project', { input: {
      rootPath: ${JSON.stringify(repo)}, name: 'Desktop E2E isolation probe',
    } });
    const task = await invoke('create_task', { input: {
      projectId: project.id, title: 'E2E isolation task', priority: 'medium',
    } });
    return { original, project, task };
  })()`);
  assert.equal(result.task.projectId, result.project.id);
  assert.equal(result.task.status, "draft");
  return result;
}

export async function closeDesktop(evaluate) {
  await evaluate(`setTimeout(() => window.__TAURI_INTERNALS__.invoke('plugin:window|close', {
    label: 'main',
  }), 0); true`);
}

if (import.meta.main) {
  const [, , port, url, repo, , mode] = process.argv;
  const desktop = await connectDesktop(port, url);
  try {
    await writeFixture(desktop.evaluate, repo, mode);
    await closeDesktop(desktop.evaluate);
  } finally {
    desktop.disconnect();
  }
}
