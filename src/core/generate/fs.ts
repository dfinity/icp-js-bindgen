import { mkdir, readdir, rm, stat, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';

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

export async function writeFileSafe(filePath: string, data: string | Uint8Array, force: boolean) {
  try {
    await stat(filePath);
    if (!force) {
      throw new Error(
        `The generated file already exists: ${filePath}. To overwrite it, use the \`force\` option.`,
      );
    }
  } catch (err) {
    const error = err as NodeJS.ErrnoException;
    if (error && error.code !== 'ENOENT') {
      throw error;
    }
  }

  await mkdir(dirname(filePath), { recursive: true });
  await writeFile(filePath, data);
}
