#!/usr/bin/env node
// Sigue un workflow de GitHub Actions en la isla, con progreso por jobs.
//
//   node scripts/ci-watch.mjs [run-id]
//
// Sin run-id toma el último run de la rama actual. Requiere `gh` autenticado.

import { execFileSync } from "node:child_process";
import { setTimeout as sleep } from "node:timers/promises";
import { setActivity } from "./island.mjs";

/** La API de GitHub tiene rate limit: cada 10 s es de sobra para seguir un CI. */
const POLL_MS = 10_000;

const gh = (...args) => JSON.parse(execFileSync("gh", args, { encoding: "utf8" }));

function latestRunId() {
  const branch = execFileSync("git", ["branch", "--show-current"], { encoding: "utf8" }).trim();
  const [run] = gh("run", "list", "--branch", branch, "--limit", "1", "--json", "databaseId");
  if (!run) throw new Error(`No hay runs de CI para la rama ${branch}`);
  return String(run.databaseId);
}

const runId = process.argv[2] ?? latestRunId();
const id = `ci-${runId}`;

for (;;) {
  const run = gh(
    "run",
    "view",
    runId,
    "--json",
    "status,conclusion,jobs,displayTitle,workflowName,headBranch",
  );
  const subtitle = `${run.workflowName} · ${run.headBranch}`.slice(0, 120);

  if (run.status === "completed") {
    const ok = run.conclusion === "success";
    await setActivity(id, {
      title: ok ? "CI en verde" : `CI: ${run.conclusion}`,
      subtitle,
      state: ok ? "success" : "error",
      icon: "build",
      ttlSeconds: 15,
    });
    console.log(ok ? "CI en verde" : `CI terminó: ${run.conclusion}`);
    process.exitCode = ok ? 0 : 1;
    break;
  }

  const total = run.jobs.length;
  const done = run.jobs.filter((job) => job.status === "completed").length;
  await setActivity(id, {
    title: total ? `CI ${done}/${total}` : "CI en cola",
    subtitle: run.displayTitle?.slice(0, 120) || subtitle,
    state: "running",
    progress: total ? done / total : undefined,
    icon: "build",
  });
  await sleep(POLL_MS);
}
