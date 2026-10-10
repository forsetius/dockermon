export type LogTokenKind =
  | 'http-method-delete'
  | 'http-method-get'
  | 'http-method-head'
  | 'http-method-patch'
  | 'http-method-post'
  | 'http-method-put'
  | 'http-status-client-error'
  | 'http-status-redirect'
  | 'http-status-server-error'
  | 'http-status-success'
  | 'log-level-error'
  | 'log-level-fatal'
  | 'log-level-info'
  | 'log-level-warning'
  | 'text'
  | 'time';

export interface LogToken {
  kind: LogTokenKind;
  value: string;
}

const highlightedTokenPattern =
  /(?:^|[^0-9])(?<time>(?:[01]\d|2[0-3]):[0-5]\d(?::[0-5]\d(?:[.,]\d+)?)?(?:Z|[+-]\d{2}:?\d{2})?(?!\d))|(?<method>\b(?:GET|POST|PATCH|PUT|DELETE|HEAD)\b)|(?<status>\b[2-5]\d{2}\b)|(?:\b[lL][eE][vV][eE][lL]\s*=\s*["']?)(?<assignedLevel>[A-Za-z]+)\b|(?<colonLevel>\b(?:LOG|INFO|WARN|WARNING|ERROR|ERR|FATAL)\b)(?=\s*:)|\[(?<bracketLevel>LOG|INFO|WARN|WARNING|ERROR|ERR|FATAL)\]/g;

const methodKinds: Record<string, LogTokenKind> = {
  DELETE: 'http-method-delete',
  GET: 'http-method-get',
  HEAD: 'http-method-head',
  PATCH: 'http-method-patch',
  POST: 'http-method-post',
  PUT: 'http-method-put',
};

const statusKind = (status: string): LogTokenKind => {
  if (status.startsWith('2')) return 'http-status-success';
  if (status.startsWith('3')) return 'http-status-redirect';
  if (status.startsWith('4')) return 'http-status-client-error';
  return 'http-status-server-error';
};

const logLevelKind = (level: string): LogTokenKind | null => {
  switch (level.toUpperCase()) {
    case 'LOG':
    case 'INFO':
      return 'log-level-info';
    case 'WARN':
    case 'WARNING':
      return 'log-level-warning';
    case 'ERROR':
    case 'ERR':
      return 'log-level-error';
    case 'FATAL':
      return 'log-level-fatal';
    default:
      return null;
  }
};

export const tokenizeLogLine = (line: string): LogToken[] => {
  const tokens: LogToken[] = [];
  const matches = [...line.matchAll(highlightedTokenPattern)];
  const containsHttpMethod = matches.some((match) => Boolean(match.groups?.method));
  let cursor = 0;

  for (const match of matches) {
    const groups = match.groups;
    const logLevel = groups?.assignedLevel ?? groups?.colonLevel ?? groups?.bracketLevel;
    const recognizedLogLevelKind = logLevel ? logLevelKind(logLevel) : null;
    const value = groups?.time ?? groups?.method ?? groups?.status ?? logLevel;
    if (!value) continue;
    if (groups?.status && !containsHttpMethod) continue;
    if (logLevel && !recognizedLogLevelKind) continue;

    const matchIndex = match.index ?? cursor;
    const tokenStart = matchIndex + match[0].lastIndexOf(value);
    if (tokenStart > cursor) tokens.push({ kind: 'text', value: line.slice(cursor, tokenStart) });

    const kind = groups?.time
      ? 'time'
      : groups?.method
        ? methodKinds[groups.method]
        : groups?.status
          ? statusKind(value)
          : recognizedLogLevelKind;
    if (!kind) continue;
    tokens.push({ kind, value });
    cursor = tokenStart + value.length;
  }

  if (cursor < line.length) tokens.push({ kind: 'text', value: line.slice(cursor) });
  return tokens;
};
