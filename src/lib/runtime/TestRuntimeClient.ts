import type {
  ApplicationSnapshot,
  Locale,
  ProjectAction,
  RuntimeCapabilities,
  ServiceAction,
  ThemePreference,
} from '../domain';
import { isServiceRunning } from '../domain';
import type { RuntimeClient, ServiceLogListener, SnapshotListener } from './RuntimeClient';
import { createFixtureSnapshot, fixtureLogs } from './fixtures';

export type TestScenario = 'ready' | 'empty' | 'error' | 'loading';

interface TestRuntimeOptions {
  capabilities?: Partial<RuntimeCapabilities>;
  globalStopFailures?: string[];
  latency?: number;
  logInterval?: number;
  scenario?: TestScenario;
}

const delay = (duration: number): Promise<void> =>
  new Promise((resolve) => window.setTimeout(resolve, duration));

export class TestRuntimeClient implements RuntimeClient {
  private readonly latency: number;
  private readonly listeners = new Set<SnapshotListener>();
  private readonly logInterval: number;
  private readonly logSubscriptions = new Map<
    number,
    { listener: ServiceLogListener; serviceId: string; timer: number | null }
  >();
  private readonly scenario: TestScenario;
  private readonly globalStopFailures: ReadonlySet<string>;
  private globalStopSequence = 0;
  private logSequence = 0;
  private logSubscriptionSequence = 0;
  private snapshot = createFixtureSnapshot();

  constructor(options: TestRuntimeOptions = {}) {
    this.latency = options.latency ?? 180;
    this.logInterval = options.logInterval ?? 4000;
    this.scenario = options.scenario ?? 'ready';
    this.globalStopFailures = new Set(options.globalStopFailures ?? []);
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

  async importProject(): Promise<ApplicationSnapshot> {
    await delay(this.latency);
    if (!this.snapshot.projects.some((project) => project.id === 'compose:zerniki')) {
      this.snapshot.projects.push({
        active: false,
        id: 'compose:zerniki',
        name: 'zerniki',
        profiles: [{ enabled: false, name: 'test' }],
        services: [
          {
            bulkSelected: true,
            cpuPercent: null,
            id: 'compose:zerniki:postgres',
            included: true,
            memoryBytes: null,
            name: 'postgres',
            ports: [],
            profiles: [],
            status: 'not-created',
          },
          {
            bulkSelected: true,
            cpuPercent: null,
            id: 'compose:zerniki:postgres-test',
            included: false,
            memoryBytes: null,
            name: 'postgres-test',
            ports: [],
            profiles: ['test'],
            status: 'not-created',
          },
        ],
      });
    }
    return this.publish();
  }

  async removeProject(projectId: string): Promise<ApplicationSnapshot> {
    await delay(this.latency);
    const projectIndex = this.snapshot.projects.findIndex((project) => project.id === projectId);
    if (projectIndex === -1) throw new Error('MOCK_PROJECT_NOT_FOUND');
    this.snapshot.projects.splice(projectIndex, 1);
    return this.publish();
  }

  async runProjectAction(projectId: string, action: ProjectAction): Promise<ApplicationSnapshot> {
    await delay(this.latency);
    const project = this.requireProject(projectId);

    for (const service of project.services) {
      if (!service.included || !service.bulkSelected) continue;
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

  async setProjectProfiles(projectId: string, profiles: string[]): Promise<ApplicationSnapshot> {
    const project = this.requireProject(projectId);
    for (const profile of project.profiles) profile.enabled = profiles.includes(profile.name);
    for (const service of project.services) {
      service.included =
        service.profiles.length === 0 ||
        service.profiles.some((profile) => profiles.includes(profile));
    }
    return this.publish();
  }

  async setTheme(theme: ThemePreference): Promise<ApplicationSnapshot> {
    this.snapshot.preferences.theme = theme;
    return this.publish();
  }

  async stopAll(): Promise<ApplicationSnapshot> {
    this.snapshot.globalStopInProgress = true;
    this.snapshot.globalStopReport = null;
    this.snapshot.globalStopProgress = { completed: 0, total: 0 };
    this.publish();
    await delay(this.latency);

    const services = this.allServices().filter(
      (service) =>
        isServiceRunning(service.status) ||
        service.status === 'paused' ||
        service.status === 'starting',
    );
    this.snapshot.globalStopProgress = { completed: 0, total: services.length };
    this.publish();
    for (const [index, service] of services.entries()) {
      if (!this.globalStopFailures.has(service.id)) {
        service.status = 'stopped';
        service.cpuPercent = null;
        service.memoryBytes = null;
      }
      this.snapshot.globalStopProgress = { completed: index + 1, total: services.length };
      this.publish();
    }

    this.snapshot.globalStopInProgress = false;
    this.snapshot.globalStopProgress = null;
    this.globalStopSequence += 1;
    const failures = services
      .filter((service) => this.globalStopFailures.has(service.id))
      .map((service) => ({
        code: 'CONTAINER_STOP_FAILED',
        containerName: service.name,
        retryable: true,
      }));
    this.snapshot.globalStopReport = {
      failures,
      sequence: this.globalStopSequence,
      stopped: services.length - failures.length,
      total: services.length,
    };
    this.recordActivity('stop-all', 'Docker Engine');
    return this.publish();
  }

  async subscribe(listener: SnapshotListener): Promise<() => void> {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  async subscribeServiceLogs(serviceId: string, listener: ServiceLogListener): Promise<() => void> {
    await delay(Math.min(this.latency, 80));
    const subscriptionId = `test-logs-${++this.logSubscriptionSequence}`;
    listener({
      error: null,
      lines: fixtureLogs[serviceId] ?? [
        '[12:36:14] [INFO] Container started',
        '[12:36:15] [INFO] Waiting for application logs...',
      ],
      serviceId,
      subscriptionId,
    });
    const timer =
      this.logInterval > 0
        ? window.setInterval(() => {
            this.emitServiceLogLines(serviceId, [
              `[live:${++this.logSequence}] [INFO] GET /api/health 200`,
            ]);
          }, this.logInterval)
        : null;
    const sequence = this.logSubscriptionSequence;
    this.logSubscriptions.set(sequence, { listener, serviceId, timer });
    return () => {
      const subscription = this.logSubscriptions.get(sequence);
      if (subscription?.timer != null) window.clearInterval(subscription.timer);
      this.logSubscriptions.delete(sequence);
    };
  }

  emitServiceLogLines(serviceId: string, lines: string[]): void {
    for (const [sequence, subscription] of this.logSubscriptions) {
      if (subscription.serviceId !== serviceId) continue;
      subscription.listener({
        error: null,
        lines,
        serviceId,
        subscriptionId: `test-logs-${sequence}`,
      });
    }
  }

  activeLogSubscriptionCount(): number {
    return this.logSubscriptions.size;
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
