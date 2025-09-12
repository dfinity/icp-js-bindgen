import { mkdir, rm } from "fs/promises";

export async function ensureDir(dir: string) {
  await mkdir(dir, { recursive: true });
}

export async function emptyDir(dir: string) {
  await rm(dir, { recursive: true, force: true });
}
