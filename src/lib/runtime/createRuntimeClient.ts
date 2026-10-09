import type { RuntimeClient } from './RuntimeClient';
import { TauriRuntimeClient } from './TauriRuntimeClient';
import { TestRuntimeClient, type TestScenario } from './TestRuntimeClient';

function requestedScenario(): TestScenario {
  const scenario = new URLSearchParams(window.location.search).get('scenario');
  return scenario === 'empty' || scenario === 'error' || scenario === 'loading'
    ? scenario
    : 'ready';
}

export function createRuntimeClient(): RuntimeClient {
  if ('__TAURI_INTERNALS__' in window) return new TauriRuntimeClient();
  return new TestRuntimeClient({ scenario: requestedScenario() });
}
