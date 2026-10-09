import type {
  ApplicationSnapshot,
  Locale,
  ProjectAction,
  RuntimeCapabilities,
  ServiceAction,
  ServiceLogSnapshot,
  ThemePreference,
} from '../domain';
import { isServiceRunning } from '../domain';
import type { RuntimeClient, SnapshotListener } from './RuntimeClient';
import { createFixtureSnapshot, fixtureLogs } from './fixtures';

export type TestScenario = 'ready' | 'empty' | 'error' | 'loading';

interface TestRuntimeOptions {
  capabilities?: Partial<RuntimeCapabilities>;
  latency?: number;
  scenario?: TestScenario;
}

const delay = (duration: number): Promise<void> =>
  new Promise((resolve) => window.setTimeout(resolve, duration));

export class TestRuntimeClient implements RuntimeClient {
  private readonly latency: number;
  private readonly listeners = new Set<SnapshotListener>();
  private readonly scenario: TestScenario;
  private snapshot = createFixtureSnapshot();

  constructor(options: TestRuntimeOptions = {}) {
    this.latency = options.latency ?? 180;
    this.scenario = options.scenario ?? 'ready';
    this.snapshot.capabilities = {
      ...this.snapshot.capabilities,
      ...options.capabilities,
    };

    if (this.scenario === 'empty') {
      this.snapshot.projects = [];
      this.snapshot.standaloneContainers = [];
    }
  }

  async getSnapshot(): Promise<ApplicationSnapshot> {
    if (this.scenario === 'loading') {
      return new Promise(() => undefined);
    }
    await delay(this.latency);

    if (this.scenario === 'error') {
      throw new Error('MOCK_RUNTIME_UNAVAILABLE');
    }

    return this.cloneSnapshot();
  }

  async getServiceLogs(serviceId: string): Promise<ServiceLogSnapshot> {
    await delay(Math.min(this.latency, 80));
    return {
      lines: fixtureLogs[serviceId] ?? [
        '[12:36:14] [INFO] Container started',
        '[12:36:15] [INFO] Waiting for application logs...',
      ],
      serviceId,
    };
  }

  async runProjectAction(projectId: string, action: ProjectAction): Promise<ApplicationSnapshot> {
    await delay(this.latency);
    const project = this.requireProject(projectId);

    for (const service of project.services) {
      if (!service.bulkSelected) continue;
      service.status = action === 'start-selected' ? 'running' : 'stopped';
      service.cpuPercent = action === 'start-selected' ? 0.1 : null;
      service.memoryBytes = action === 'start-selected' ? 32 * 1024 * 1024 : null;
    }

    this.recordActivity(action, project.name);
    return this.publish();
  }

  async runServiceAction(serviceId: string, action: ServiceAction): Promise<ApplicationSnapshot> {
    await delay(this.latency);
    const service = this.requireService(serviceId);
    service.status = action === 'stop' ? 'stopped' : 'running';
    service.cpuPercent = action === 'stop' ? null : Math.max(service.cpuPercent ?? 0.1, 0.1);
    service.memoryBytes = action === 'stop' ? null : (service.memoryBytes ?? 32 * 1024 * 1024);
    this.recordActivity(action, service.name);
    return this.publish();
  }

  async setBulkSelected(
    projectId: string,
    serviceId: string,
    selected: boolean,
  ): Promise<ApplicationSnapshot> {
    const project = this.requireProject(projectId);
    const service = project.services.find((candidate) => candidate.id === serviceId);
    if (!service) throw new Error('MOCK_SERVICE_NOT_FOUND');
    service.bulkSelected = selected;
    return this.publish();
  }

  async setLanguage(language: Locale): Promise<ApplicationSnapshot> {
    this.snapshot.preferences.language = language;
    return this.publish();
  }

  async setProjectActive(projectId: string, active: boolean): Promise<ApplicationSnapshot> {
    this.requireProject(projectId).active = active;
    return this.publish();
  }

  async setTheme(theme: ThemePreference): Promise<ApplicationSnapshot> {
    this.snapshot.preferences.theme = theme;
    return this.publish();
  }

  async stopAll(): Promise<ApplicationSnapshot> {
    await delay(this.latency);
    this.snapshot.globalStopInProgress = true;
    this.publish();
    await delay(this.latency);

    for (const service of this.allServices()) {
      if (isServiceRunning(service.status) || service.status === 'paused') {
        service.status = 'stopped';
        service.cpuPercent = null;
        service.memoryBytes = null;
      }
    }

    this.snapshot.globalStopInProgress = false;
    this.recordActivity('stop-all', 'Docker Engine');
    return this.publish();
  }

  async subscribe(listener: SnapshotListener): Promise<() => void> {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private allServices() {
    return [
      ...this.snapshot.projects.flatMap((project) => project.services),
      ...this.snapshot.standaloneContainers,
    ];
  }

  private cloneSnapshot(): ApplicationSnapshot {
    return structuredClone(this.snapshot);
  }

  private publish(): ApplicationSnapshot {
    const snapshot = this.cloneSnapshot();
    for (const listener of this.listeners) listener(snapshot);
    return snapshot;
  }

  private recordActivity(action: string, subject: string): void {
    this.snapshot.activity.unshift({
      action,
      id: crypto.randomUUID(),
      occurredAt: new Intl.DateTimeFormat('pl', {
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
      }).format(new Date()),
      subject,
      successful: true,
    });
  }

  private requireProject(projectId: string) {
    const project = this.snapshot.projects.find((candidate) => candidate.id === projectId);
    if (!project) throw new Error('MOCK_PROJECT_NOT_FOUND');
    return project;
  }

  private requireService(serviceId: string) {
    const service = this.allServices().find((candidate) => candidate.id === serviceId);
    if (!service) throw new Error('MOCK_SERVICE_NOT_FOUND');
    return service;
  }
}
