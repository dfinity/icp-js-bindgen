export function cyan(message: string): string {
  return `\x1b[36m${message}\x1b[0m`;
}

export function green(message: string): string {
  return `\x1b[32m${message}\x1b[0m`;
}

export function red(message: string): string {
  return `\x1b[31m${message}\x1b[0m`;
}
