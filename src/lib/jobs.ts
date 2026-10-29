import { ipc, on } from "./ipc";
import type { JobProgressEvent } from "../types/coral";

/** A running backend job as the UI shows it (JobProgress). */
export interface RunningJob extends JobProgressEvent {
  label: string;
  startedAt: number;
  cancelling: boolean;
}

/**
 * Run a job-backed command while `coral://progress` updates `set`. The first progress event of the
 * job ("Starting…") carries its id, which Cancel needs.
 */
export async function withJob<T>(label: string, set: (job: RunningJob | null) => void, run: () => Promise<T>): Promise<T> {
  let job: RunningJob = { jobId: 0, done: 0, total: 0, message: "Starting…", label, startedAt: Date.now(), cancelling: false };
  set(job);
  const unlisten = await on.progress((p) => {
    if (job.jobId && p.jobId !== job.jobId) return;
    job = { ...job, ...p };
    set(job);
  });
  try {
    return await run();
  } finally {
    unlisten();
    set(null);
  }
}

export async function cancelJob(job: RunningJob | null, set: (job: RunningJob | null) => void) {
  if (!job?.jobId) return;
  set({ ...job, cancelling: true });
  await ipc.cancelJob(job.jobId);
}

export const fmtBytes = (b: number) => {
  const u = ["B", "KB", "MB", "GB"];
  let v = b;
  let i = 0;
  while (v >= 1024 && i < u.length - 1) {
    v /= 1024;
    i++;
  }
  return i ? `${v.toFixed(1)} ${u[i]}` : `${b} B`;
};

export const baseName = (p: string) => p.split(/[\\/]/).pop() ?? p;
export const dirName = (p: string) => p.replace(/[\\/][^\\/]*$/, "");
export const joinPath = (dir: string, name: string) => `${dir.replace(/[\\/]$/, "")}${dir.includes("/") && !dir.includes("\\") ? "/" : "\\"}${name}`;
