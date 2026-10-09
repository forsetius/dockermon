export type Locale = 'en' | 'pl';

export type ThemePreference = 'system' | 'light' | 'dark';

export type ConnectionStatus = 'connected' | 'connecting' | 'disconnected';

export type ServiceStatus =
  'not-created' | 'stopped' | 'starting' | 'running' | 'healthy' | 'unhealthy' | 'paused' | 'error';

export type ServiceAction = 'start' | 'stop' | 'restart';

export type ProjectAction = 'start-selected' | 'stop-selected';

export type NavigationView = 'projects' | 'containers' | 'activity' | 'settings';

export interface Preferences {
  language: Locale;
  theme: ThemePreference;
}

export interface ServiceSnapshot {
  bulkSelected: boolean;
  cpuPercent: number | null;
  id: string;
  memoryBytes: number | null;
  name: string;
  ports: string[];
  status: ServiceStatus;
}

export interface ProjectSnapshot {
  active: boolean;
  id: string;
  name: string;
  services: ServiceSnapshot[];
}

export interface ActivityEntry {
  action: string;
  id: string;
  occurredAt: string;
  subject: string;
  successful: boolean;
}

export interface ApplicationSnapshot {
  activity: ActivityEntry[];
  connection: ConnectionStatus;
  globalStopInProgress: boolean;
  preferences: Preferences;
  projects: ProjectSnapshot[];
  standaloneContainers: ServiceSnapshot[];
}

export interface ServiceLogSnapshot {
  lines: string[];
  serviceId: string;
}

export function isServiceReady(status: ServiceStatus): boolean {
  return status === 'running' || status === 'healthy';
}

export function isServiceRunning(status: ServiceStatus): boolean {
  return status === 'running' || status === 'healthy' || status === 'unhealthy';
}
