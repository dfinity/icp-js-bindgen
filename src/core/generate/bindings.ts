import { ESLINT_DISABLE_COMMENT, TS_NOCHECK_COMMENT, DISCLAIMER_COMMENT } from './constants.ts';
import { capitalizeFirstLetter } from './string.ts';

export function prepareBinding(binding: string): string {
  return `${ESLINT_DISABLE_COMMENT}

${TS_NOCHECK_COMMENT}

${DISCLAIMER_COMMENT}

${binding}`;
}

export function indexBinding(serviceFileName: string): string {
  const serviceName = capitalizeFirstLetter(serviceFileName);

  // we don't want to disable typescript checks for this file
  return `${ESLINT_DISABLE_COMMENT}

${DISCLAIMER_COMMENT}

import {
  Actor,
  HttpAgent,
  type Agent,
  type HttpAgentOptions,
  type ActorConfig,
} from "@icp-sdk/core/agent";
import { type _SERVICE, idlFactory } from "./declarations/${serviceFileName}.did";
import { ${serviceName}, type ProcessErrorFn } from "./${serviceFileName}";

export interface CreateActorOptions {
  /**
   * @see {@link Agent}
   */
  agent?: Agent;
  /**
   * @see {@link HttpAgentOptions}
   */
  agentOptions?: HttpAgentOptions;
  /**
   * @see {@link ActorConfig}
   */
  actorOptions?: ActorConfig;
}

export function createActor(
  canisterId: string,
  options: CreateActorOptions = {},
  processError?: ProcessErrorFn
): ${serviceName} {
  const agent =
    options.agent || HttpAgent.createSync({ ...options.agentOptions });

  if (options.agent && options.agentOptions) {
    console.warn(
      "Detected both agent and agentOptions passed to createActor. Ignoring agentOptions and proceeding with the provided agent."
    );
  }

  // Creates an actor with using the candid interface and the HttpAgent
  const actor = Actor.createActor<_SERVICE>(idlFactory, {
    agent,
    canisterId,
    ...options.actorOptions,
  });

  return new ${serviceName}(actor, processError);
}
`;
}
