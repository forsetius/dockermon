import { describe, expect, it } from 'vitest';
import { tokenizeLogLine } from './logHighlight';

describe('tokenizeLogLine', () => {
  it.each([
    ['GET', 'http-method-get'],
    ['POST', 'http-method-post'],
    ['PATCH', 'http-method-patch'],
    ['PUT', 'http-method-put'],
    ['DELETE', 'http-method-delete'],
    ['HEAD', 'http-method-head'],
  ] as const)('classifies the %s HTTP method', (method, kind) => {
    expect(tokenizeLogLine(`${method} /api/items`)).toContainEqual({ kind, value: method });
  });

  it.each([
    ['204', 'http-status-success'],
    ['304', 'http-status-redirect'],
    ['404', 'http-status-client-error'],
    ['503', 'http-status-server-error'],
  ] as const)('classifies the %s HTTP status', (status, kind) => {
    expect(tokenizeLogLine(`status=${status} method=GET`)).toContainEqual({ kind, value: status });
  });

  it('leaves status-like numbers unstyled when the line has no HTTP method', () => {
    const line = 'Loaded 200 records, redirect batch 304, errors 404 and 503';

    expect(tokenizeLogLine(line)).toEqual([{ kind: 'text', value: line }]);
  });

  it.each([
    ['level=info request completed', 'info', 'log-level-info'],
    ['level=LOG request completed', 'LOG', 'log-level-info'],
    ['WARN: cache is almost full', 'WARN', 'log-level-warning'],
    ['WARNING: cache is almost full', 'WARNING', 'log-level-warning'],
    ['level=error request failed', 'error', 'log-level-error'],
    ['ERR: request failed', 'ERR', 'log-level-error'],
    ['FATAL: database unavailable', 'FATAL', 'log-level-fatal'],
    ['[INFO] server started', 'INFO', 'log-level-info'],
  ] as const)('classifies the log level in "%s"', (line, value, kind) => {
    expect(tokenizeLogLine(line)).toContainEqual({ kind, value });
  });

  it('leaves unsupported or unmarked log-level words unstyled', () => {
    expect(tokenizeLogLine('level=debug info about an error')).toEqual([
      { kind: 'text', value: 'level=debug info about an error' },
    ]);
  });

  it('highlights only the time portion of date-time values', () => {
    expect(tokenizeLogLine('2026-10-10T21:37:42.125Z GET /health 200')).toEqual([
      { kind: 'text', value: '2026-10-10T' },
      { kind: 'time', value: '21:37:42.125Z' },
      { kind: 'text', value: ' ' },
      { kind: 'http-method-get', value: 'GET' },
      { kind: 'text', value: ' /health ' },
      { kind: 'http-status-success', value: '200' },
    ]);
  });

  it('supports standalone times without treating ports and durations as HTTP tokens', () => {
    expect(tokenizeLogLine('[08:05:09] listening on 3000 after 200ms')).toEqual([
      { kind: 'text', value: '[' },
      { kind: 'time', value: '08:05:09' },
      { kind: 'text', value: '] listening on 3000 after 200ms' },
    ]);
  });
});
