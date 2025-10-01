import { mkdir, readdir, rm } from 'node:fs/promises';
import { join } from 'node:path';

export async function ensureDir(dir: string) {
  await mkdir(dir, { recursive: true });
}

export async function emptyDirWithFilter(dir: string, filter?: (path: string) => boolean) {
  if (!filter) {
    await emptyDir(dir);
    return;
  }

  const files = await readdir(dir);
  await Promise.all(
    files.filter(filter).map((file) => rm(join(dir, file), { recursive: true, force: true })),
  );
}

async function emptyDir(dir: string) {
  await rm(dir, { recursive: true, force: true });
}
