import { DISCLAIMER_COMMENT, ESLINT_DISABLE_COMMENT, TS_NOCHECK_COMMENT } from './constants.ts';

export function prepareBinding(binding: string): string {
  return `${ESLINT_DISABLE_COMMENT}

${TS_NOCHECK_COMMENT}

${DISCLAIMER_COMMENT}

${binding}`;
}

export function prepareTypescriptBinding(binding: string): string {
  return `${ESLINT_DISABLE_COMMENT}

${DISCLAIMER_COMMENT}

${binding}`;
}
