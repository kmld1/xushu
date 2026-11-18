// Installed-build E2E scenario. Drives the real Tauri IPC surface of an
// installed XUSHU build through WebView2's CDP endpoint. No screenshots.

import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

const [, , portText, mode, repo, gitExecutable, workDirectory] = process.argv;
const port = Number(portText);
const evidenceDirectory = workDirectory ?? path.resolve(process.cwd(), "target", "phase7a");
fs.mkdirSync(evidenceDirectory, { recursive: true });

async function connect() {
  let page;
  for (let attempt = 0; attempt < 300 && !page; attempt += 1) {
    try {
      const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
      page = targets.find(
        (target) => target.type === "page" && !target.url.startsWith("devtools://"),
      );
    } catch {
      // WebView2 is still starting.
    }
    if (!page) await new Promise((resolve) => setTimeout(resolve, 100));
  }
  assert.ok(page, "installed desktop WebView did not expose a CDP page");

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
      throw new Error(reply.exceptionDetails.exception?.description ?? "evaluation failed");
    }
    return reply.result.value;
  }

  for (let attempt = 0; attempt < 200; attempt += 1) {
    const ready = await evaluate(
      `Boolean(window.__TAURI_INTERNALS__ && document.body && document.body.innerText.length > 0)`,
    );
    if (ready) break;
    if (attempt === 199) throw new Error("installed desktop UI did not become ready");
    await new Promise((resolve) => setTimeout(resolve, 100));
  }

  return { evaluate, disconnect: () => socket.close() };
}

function writeEvidence(name, value) {
  fs.writeFileSync(
    path.join(evidenceDirectory, name),
    `${JSON.stringify(value, null, 2)}\n`,
    "utf8",
  );
}

async function closeDesktop(desktop) {
  await desktop.evaluate(`setTimeout(() => window.__TAURI_INTERNALS__.invoke('plugin:window|close', {
    label: 'main',
  }), 0); true`);
}

async function runFull(desktop) {
  const result = await desktop.evaluate(`(async () => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    const repo = ${JSON.stringify(repo)};
    const git = ${JSON.stringify(gitExecutable)};
    const backupTarget = ${JSON.stringify(path.join(path.dirname(repo), "phase7a-backups"))};

    async function waitForRun(runId) {
      for (let attempt = 0; attempt < 600; attempt += 1) {
        const run = await invoke('get_verification_run', { runId });
        if (run.executionStatus !== 'running') return run;
        await new Promise((resolve) => setTimeout(resolve, 100));
      }
      throw new Error('verification run did not finish');
    }

    const health = await invoke('get_database_health');
    const appInfo = await invoke('get_app_info');
    const project = await invoke('import_project', {
      input: { rootPath: repo, name: 'Phase 7A 安装验证项目' },
    });
    let task = await invoke('create_task', {
      input: { projectId: project.id, title: 'Phase 7A 安装版任务', priority: 'high' },
    });

    const autoCriterion = await invoke('create_acceptance_criterion', {
      input: {
        taskId: task.id, expectedTaskRevision: task.revision,
        description: 'Git 仓库可读取', kind: 'auto', required: true,
      },
    });
    let detail = await invoke('get_task', { id: task.id });
    task = detail.task;
    const manualCriterion = await invoke('create_acceptance_criterion', {
      input: {
        taskId: task.id, expectedTaskRevision: task.revision,
        description: '人工确认安装版可操作', kind: 'manual', required: true,
      },
    });

    detail = await invoke('get_task', { id: task.id });
    task = await invoke('transition_task_status', {
      input: { taskId: task.id, expectedRevision: detail.task.revision, toStatus: 'ready', reason: null },
    });
    task = await invoke('transition_task_status', {
      input: { taskId: task.id, expectedRevision: task.revision, toStatus: 'running', reason: null },
    });
    task = await invoke('transition_task_status', {
      input: { taskId: task.id, expectedRevision: task.revision, toStatus: 'awaiting_verification', reason: null },
    });

    detail = await invoke('get_task', { id: task.id });
    const autoRevision = detail.criteria.find((view) => view.criterion.id === autoCriterion.id).criterion.revision;
    const failingSpec = await invoke('save_verification_spec', {
      input: {
        criterionId: autoCriterion.id,
        expectedTaskRevision: detail.task.revision,
        expectedCriterionRevision: autoRevision,
        expectedSpecRevision: null,
        executable: git,
        args: ['rev-parse', '--verify', 'refs/heads/phase7a-missing-branch'],
        cwd: repo,
        timeoutSeconds: 120,
        outputLimitBytes: 65536,
      },
    });
    detail = await invoke('get_task', { id: task.id });
    const preview = await invoke('preview_verification', { criterionId: autoCriterion.id });
    const failingRun = await invoke('start_verification', {
      input: {
        criterionId: autoCriterion.id,
        expectedTaskRevision: detail.task.revision,
        expectedCriterionRevision: detail.criteria.find((view) => view.criterion.id === autoCriterion.id).criterion.revision,
        expectedSpecRevision: failingSpec.revision,
        approved: true,
      },
    });
    const finishedFailure = await waitForRun(failingRun.id);
    const failingCheck = await invoke('get_verification_check', { runId: failingRun.id });
    detail = await invoke('get_task', { id: task.id });
    let doneRejected = null;
    try {
      await invoke('transition_task_status', {
        input: { taskId: task.id, expectedRevision: detail.task.revision, toStatus: 'done', reason: null },
      });
    } catch (error) {
      doneRejected = { code: error.code, message: error.message };
    }

    const passingSpec = await invoke('save_verification_spec', {
      input: {
        criterionId: autoCriterion.id,
        expectedTaskRevision: detail.task.revision,
        expectedCriterionRevision: detail.criteria.find((view) => view.criterion.id === autoCriterion.id).criterion.revision,
        expectedSpecRevision: failingSpec.revision,
        executable: git,
        args: ['rev-parse', '--git-dir'],
        cwd: repo,
        timeoutSeconds: 120,
        outputLimitBytes: 65536,
      },
    });
    detail = await invoke('get_task', { id: task.id });
    const passingRun = await invoke('start_verification', {
      input: {
        criterionId: autoCriterion.id,
        expectedTaskRevision: detail.task.revision,
        expectedCriterionRevision: detail.criteria.find((view) => view.criterion.id === autoCriterion.id).criterion.revision,
        expectedSpecRevision: passingSpec.revision,
        approved: true,
      },
    });
    const finishedPass = await waitForRun(passingRun.id);
    const passingCheck = await invoke('get_verification_check', { runId: passingRun.id });
    detail = await invoke('get_task', { id: task.id });
    assertOk(detail.completion.canComplete === false, 'manual confirmation must still gate completion');

    const confirmation = await invoke('confirm_manual_criterion', {
      input: {
        criterionId: manualCriterion.id,
        expectedTaskRevision: detail.task.revision,
        expectedCriterionRevision: detail.criteria.find((view) => view.criterion.id === manualCriterion.id).criterion.revision,
        note: '安装版 E2E 人工确认',
      },
    });
    detail = await invoke('get_task', { id: task.id });
    assertOk(detail.completion.canComplete === true, 'completion must be allowed after pass + manual confirmation');
    const completedTask = await invoke('transition_task_status', {
      input: { taskId: task.id, expectedRevision: detail.task.revision, toStatus: 'done', reason: null },
    });

    const memory = await invoke('create_project_memory', {
      input: {
        projectId: project.id, category: 'constraint', title: '安装版恢复记忆',
        content: '升级与重启后必须保留此记忆。', sourceType: 'user', confirmed: true,
        linkedTaskId: task.id, linkedEvidenceId: null,
      },
    });
    const decision = await invoke('create_project_decision', {
      input: {
        projectId: project.id, taskId: task.id, title: '安装版验收决策',
        problem: '需要验证安装版数据生命周期', context: 'Windows 本地优先',
        alternatives: ['仅开发构建', '安装版验证'], decision: '使用安装版验证',
        rationale: '安装路径与 sidecar 必须真实可用', risks: '需要隔离数据目录',
        supersedesId: null,
      },
    });
    const confirmedDecision = await invoke('confirm_project_decision', {
      input: {
        projectId: project.id, decisionId: decision.decision.id,
        expectedRevision: decision.decision.revision,
      },
    });
    const snapshot = await invoke('generate_recovery_package', { projectId: project.id });
    const backup = await invoke('create_backup', { targetDirectory: backupTarget });
    const verification = await invoke('verify_backup', { backupId: backup.backupId });
    const mutatedMemory = await invoke('update_project_memory', {
      input: {
        projectId: project.id, memoryId: memory.memory.id,
        expectedRevision: memory.memory.revision, category: 'constraint',
        title: '备份之后修改的安装版标题', content: '此变更不在备份中。',
        linkedTaskId: task.id, linkedEvidenceId: null, changeNote: '安装版 E2E 备份后修改',
      },
    });
    const preflight = await invoke('preflight_restore', {
      input: { backupDirectory: backup.directoryPath },
    });
    const restoreResult = await invoke('perform_restore', {
      input: {
        token: preflight.token, confirmationText: '恢复', acknowledged: true,
      },
    });
    const restoredHealth = await invoke('get_database_health');
    const memories = await invoke('list_project_memories', { filter: { projectId: project.id } });
    const decisions = await invoke('list_project_decisions', { filter: { projectId: project.id } });
    const snapshots = await invoke('list_recovery_snapshots', { projectId: project.id });
    const tasks = await invoke('list_tasks', { filter: { projectId: project.id } });
    const postRestoreBackup = await invoke('create_backup', { targetDirectory: backupTarget });
    const postRestoreVerify = await invoke('verify_backup', {
      backupId: postRestoreBackup.backupId,
    });

    const hookPreview = await invoke('preview_claude_hooks', { projectId: project.id });
    const hookStatus = await invoke('install_claude_hooks', {
      input: {
        projectId: project.id, previewToken: hookPreview.previewToken, approved: true,
      },
    });
    const overview = await invoke('get_data_protection_overview');

    const ui = await (async () => {
      async function waitForText(text, timeout = 20000) {
        const started = Date.now();
        while (Date.now() - started < timeout) {
          if (document.body.innerText.includes(text)) return true;
          await new Promise((resolve) => setTimeout(resolve, 100));
        }
        return false;
      }
      window.location.hash = '#/data-protection';
      const backupSection = await waitForText('数据库备份');
      const integritySection = await waitForText('数据完整性');
      const privacySection = await waitForText('Hook 待导入 Spool');
      const headerHeight = document.querySelector('header')?.getBoundingClientRect().height ?? null;
      const sidebarWidth = document.querySelector('aside')?.getBoundingClientRect().width ?? null;
      const overflow = document.documentElement.scrollWidth > window.innerWidth + 1;
      return { backupSection, integritySection, privacySection, headerHeight, sidebarWidth, overflow };
    })();

    function assertOk(condition, message) {
      if (!condition) throw new Error(message);
    }

    return {
      appInfo, health, project,
      taskId: task.id, completedTask, autoCriterion, manualCriterion, failingSpec,
      preview, failingRun: finishedFailure, failingCheck, doneRejected,
      passingSpec, passingRun: finishedPass, passingCheck, confirmation,
      completion: detail.completion,
      memory, mutatedMemory, decision, confirmedDecision, snapshot,
      backup, verification, preflight, restoreResult, restoredHealth,
      memories, decisions, snapshots, tasks, hookPreview, hookStatus, overview, ui,
      postRestoreBackup, postRestoreVerify, backupTarget,
    };
  })()`);

  assert.equal(result.health.ok, true);
  assert.equal(result.health.schemaVersion, 7);
  assert.equal(result.appInfo.phase, "Phase 7A");
  assert.equal(result.failingRun.executionStatus, "completed");
  assert.equal(result.failingRun.exitCode, 128);
  assert.equal(result.failingCheck.result, "fail");
  assert.ok(result.doneRejected, "done must be rejected after a failed verification");
  assert.equal(result.passingRun.executionStatus, "completed");
  assert.equal(result.passingRun.exitCode, 0);
  assert.equal(result.passingCheck.result, "pass");
  assert.equal(result.completedTask.status, "done");
  assert.equal(result.verification.verified, true);
  assert.equal(result.preflight.ready, true);
  assert.equal(result.restoreResult.status, "completed");
  assert.equal(result.restoreResult.switchVerified, true);
  assert.ok(result.restoreResult.finalReport?.ok);
  assert.equal(result.restoredHealth.ok, true);
  assert.equal(result.memories.length, 1);
  assert.equal(result.memories[0].memory.title, "安装版恢复记忆");
  assert.equal(result.decisions[0].decision.status, "confirmed");
  assert.ok(result.snapshots.some((item) => item.id === result.snapshot.id));
  assert.equal(result.tasks.length, 1);
  assert.equal(result.hookStatus.installState, "installed");
  assert.equal(result.hookStatus.hookBinaryAvailable, true);
  assert.equal(result.postRestoreVerify.verified, true);
  assert.equal(result.overview.backups.length, 1);
  assert.equal(result.ui.backupSection, true);
  assert.equal(result.ui.integritySection, true);
  assert.equal(result.ui.privacySection, true);
  assert.equal(result.ui.headerHeight, 44);
  assert.ok(result.ui.sidebarWidth === 240 || result.ui.sidebarWidth === 52);
  assert.equal(result.ui.overflow, false);

  writeEvidence("installed-full-result.json", result);
  console.log(
    "PHASE7A_INSTALLED_FULL_RESULT",
    JSON.stringify({
      appVersion: result.appInfo.version,
      projectId: result.project.id,
      taskId: result.taskId,
      failingResult: result.failingCheck.result,
      passingResult: result.passingCheck.result,
      doneRejected: result.doneRejected?.code,
      doneStatus: result.completedTask.status,
      restoreStatus: result.restoreResult.status,
      hookState: result.hookStatus.installState,
      backupId: result.backup.backupId,
      ui: result.ui,
    }),
  );
  return result;
}

async function runReopen(desktop, expectedVersion, receiverChanged = false) {
  const initial = JSON.parse(
    fs.readFileSync(path.join(evidenceDirectory, "installed-full-result.json"), "utf8"),
  );
  const result = await desktop.evaluate(`(async () => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    const health = await invoke('get_database_health');
    const appInfo = await invoke('get_app_info');
    const projects = await invoke('list_projects');
    const project = projects.find((value) => value.name === 'Phase 7A 安装验证项目');
    if (!project) throw new Error('installed project missing after relaunch');
    const tasks = await invoke('list_tasks', { filter: { projectId: project.id } });
    const memories = await invoke('list_project_memories', { filter: { projectId: project.id } });
    const decisions = await invoke('list_project_decisions', { filter: { projectId: project.id } });
    const snapshots = await invoke('list_recovery_snapshots', { projectId: project.id });
    const runs = await invoke('list_verification_runs', { taskId: tasks[0].id });
    const passed = await invoke('get_evidence', { evidenceId: ${JSON.stringify(initial.passingCheck.evidenceId)} });
    const failed = await invoke('get_evidence', { evidenceId: ${JSON.stringify(initial.failingCheck.evidenceId)} });
    const memory = await invoke('get_project_memory', { projectId: project.id, memoryId: memories[0].memory.id });
    const decision = await invoke('get_project_decision', { projectId: project.id, decisionId: decisions[0].decision.id });
    const integration = await invoke('get_claude_integration', { projectId: project.id });
    const activity = await invoke('get_claude_activity', { projectId: project.id });
    const backupVerification = await invoke('verify_backup', {
      backupId: ${JSON.stringify(initial.postRestoreBackup.backupId)},
    });
    const overview = await invoke('get_data_protection_overview');
    const ui = await (async () => {
      window.location.hash = '#/data-protection';
      const started = Date.now();
      while (Date.now() - started < 20000) {
        if (document.body.innerText.includes('数据库备份')) break;
        await new Promise((resolve) => setTimeout(resolve, 100));
      }
      return {
        backupSection: document.body.innerText.includes('数据库备份'),
        completedRestore: document.body.innerText.includes('恢复完成'),
        overflow: document.documentElement.scrollWidth > window.innerWidth + 1,
      };
    })();
    return { health, appInfo, project, tasks, memories, decisions, snapshots, runs,
      passedEvidenceId: passed.id, failedEvidenceId: failed.id,
      memoryRevisionCount: memory.revisions.length, decisionRevisionCount: decision.revisions.length,
      integration, activity, backupVerification, overview, ui };
  })()`);

  assert.equal(result.health.ok, true);
  assert.equal(result.health.schemaVersion, 7);
  if (expectedVersion) assert.equal(result.appInfo.version, expectedVersion);
  assert.equal(result.tasks.length, 1);
  assert.equal(result.tasks[0].status, "done");
  assert.equal(result.memories.length, 1);
  assert.equal(result.memories[0].memory.title, "安装版恢复记忆");
  assert.equal(result.decisions[0].decision.status, "confirmed");
  assert.ok(result.snapshots.some((item) => item.id === initial.snapshot.id));
  assert.ok(result.runs.some((run) => run.id === initial.passingRun.id && run.exitCode === 0));
  assert.ok(result.runs.some((run) => run.id === initial.failingRun.id && run.exitCode === 128));
  assert.equal(result.passedEvidenceId, initial.passingCheck.evidenceId);
  assert.equal(result.failedEvidenceId, initial.failingCheck.evidenceId);
  assert.ok(result.memoryRevisionCount >= 1);
  assert.ok(result.decisionRevisionCount >= 1);
  assert.equal(result.integration.installState, receiverChanged ? "needs_repair" : "installed");
  assert.equal(result.integration.receiverChanged, receiverChanged);
  assert.equal(result.integration.desiredEnabled, true);
  assert.ok(result.integration.eventCount >= 1, "sidecar event was not imported after reopen");
  assert.ok(result.activity.events.length >= 1, "activity timeline is empty");
  assert.equal(result.backupVerification.backupId, initial.postRestoreBackup.backupId);
  assert.equal(
    result.backupVerification.verified,
    true,
    JSON.stringify(result.backupVerification.checks),
  );
  assert.equal(result.overview.backups.length, 1);
  assert.equal(result.overview.backups[0].payloadSha256, initial.postRestoreBackup.payloadSha256);
  assert.equal(result.overview.restoreOperations.length, 1);
  assert.equal(result.ui.backupSection, true);
  assert.equal(result.ui.completedRestore, true);
  assert.equal(result.ui.overflow, false);

  writeEvidence(`installed-${mode}-result.json`, result);
  console.log(
    "PHASE7A_INSTALLED_REOPEN_RESULT",
    JSON.stringify({
      mode,
      appVersion: result.appInfo.version,
      taskStatus: result.tasks[0].status,
      memoryTitle: result.memories[0].memory.title,
      eventCount: result.integration.eventCount,
      activityEvents: result.activity.events.length,
      restoreOperations: result.overview.restoreOperations.length,
    }),
  );
  return result;
}

async function runWindow(desktop) {
  const result = await desktop.evaluate(`(async () => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const headerHeight = document.querySelector('header')?.getBoundingClientRect().height ?? null;
    const sidebarWidth = document.querySelector('aside')?.getBoundingClientRect().width ?? null;
    const overflow = document.documentElement.scrollWidth > window.innerWidth + 1;
    const dragRegions = document.querySelectorAll('[data-tauri-drag-region]').length;
    const initiallyMaximized = await invoke('plugin:window|is_maximized', { label: 'main' });
    await invoke('plugin:window|toggle_maximize', { label: 'main' });
    await wait(700);
    const maximized = await invoke('plugin:window|is_maximized', { label: 'main' });
    await invoke('plugin:window|toggle_maximize', { label: 'main' });
    await wait(700);
    const restored = await invoke('plugin:window|is_maximized', { label: 'main' });
    await invoke('plugin:window|minimize', { label: 'main' });
    return {
      headerHeight, sidebarWidth, overflow, dragRegions,
      initiallyMaximized, maximized, restored,
      width: window.innerWidth, height: window.innerHeight,
    };
  })()`);
  assert.equal(result.headerHeight, 44);
  assert.equal(result.sidebarWidth, 52);
  assert.equal(result.overflow, false);
  assert.ok(result.dragRegions > 0);
  assert.equal(result.maximized, true);
  assert.equal(result.restored, false);
  writeEvidence("installed-window-result.json", result);
  console.log("PHASE7A_WINDOW_RESULT", JSON.stringify(result));
  return result;
}

async function runSidecarUnavailable(desktop, expectedInstallState) {
  const result = await desktop.evaluate(`(async () => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    const health = await invoke('get_database_health');
    const projects = await invoke('list_projects');
    const project = projects.find((value) => value.name === 'Phase 7A 安装验证项目');
    if (!project) throw new Error('installed project missing');
    const integration = await invoke('get_claude_integration', { projectId: project.id });
    return { health, integration };
  })()`);
  assert.equal(result.health.ok, true);
  assert.equal(result.integration.installState, expectedInstallState);
  assert.equal(result.integration.hookBinaryAvailable, false);
  writeEvidence(`installed-${mode}-result.json`, result);
  console.log(
    "PHASE7A_SIDECAR_STATE",
    JSON.stringify({
      mode,
      installState: result.integration.installState,
      hookBinaryAvailable: result.integration.hookBinaryAvailable,
    }),
  );
  return result;
}

const desktop = await connect();
try {
  if (mode === "full") {
    await runFull(desktop);
  } else if (mode === "reopen") {
    await runReopen(desktop, null, false);
  } else if (mode === "upgrade") {
    await runReopen(desktop, "0.1.1", true);
  } else if (mode === "upgrade-reopen") {
    await runReopen(desktop, null, true);
  } else if (mode === "sidecar-unavailable") {
    await runSidecarUnavailable(desktop, "receiver_unavailable");
  } else if (mode === "window") {
    await runWindow(desktop);
  } else if (mode === "close") {
    await closeDesktop(desktop);
  } else {
    throw new Error(`unknown mode: ${mode}`);
  }
  if (mode !== "window" && mode !== "close") await closeDesktop(desktop);
} finally {
  desktop.disconnect();
}
