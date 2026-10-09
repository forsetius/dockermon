import type { ApplicationSnapshot } from '../domain';

const megabyte = 1024 * 1024;

export function createFixtureSnapshot(): ApplicationSnapshot {
  return {
    activity: [
      {
        action: 'restart',
        id: 'activity-1',
        occurredAt: '12:31:04',
        subject: 'api-local/api',
        successful: true,
      },
      {
        action: 'stop',
        id: 'activity-2',
        occurredAt: '11:58:20',
        subject: 'api-local/mailpit',
        successful: true,
      },
    ],
    connection: 'connected',
    globalStopInProgress: false,
    preferences: {
      language: navigator.language.toLowerCase().startsWith('pl') ? 'pl' : 'en',
      theme: 'system',
    },
    projects: [
      {
        active: true,
        id: 'storefront',
        name: 'Sklep lokalny',
        services: [
          {
            bulkSelected: true,
            cpuPercent: 1.8,
            id: 'storefront-web',
            memoryBytes: 284 * megabyte,
            name: 'web',
            ports: ['3000:3000'],
            status: 'running',
          },
          {
            bulkSelected: false,
            cpuPercent: 0.2,
            id: 'storefront-api',
            memoryBytes: 96 * megabyte,
            name: 'api',
            ports: ['3001:3000'],
            status: 'healthy',
          },
          {
            bulkSelected: true,
            cpuPercent: 1.1,
            id: 'storefront-postgres',
            memoryBytes: 512 * megabyte,
            name: 'postgres',
            ports: ['5432:5432'],
            status: 'running',
          },
          {
            bulkSelected: false,
            cpuPercent: 0.4,
            id: 'storefront-redis',
            memoryBytes: 128 * megabyte,
            name: 'redis',
            ports: ['6379:6379'],
            status: 'running',
          },
        ],
      },
      {
        active: true,
        id: 'api-local',
        name: 'API lokalne',
        services: [
          {
            bulkSelected: false,
            cpuPercent: 0.7,
            id: 'api-local-web',
            memoryBytes: 180 * megabyte,
            name: 'web',
            ports: ['8080:3000'],
            status: 'running',
          },
          {
            bulkSelected: true,
            cpuPercent: 1.2,
            id: 'api-local-api',
            memoryBytes: 256 * megabyte,
            name: 'api',
            ports: ['3000:3000'],
            status: 'healthy',
          },
          {
            bulkSelected: false,
            cpuPercent: null,
            id: 'api-local-postgres',
            memoryBytes: null,
            name: 'postgres',
            ports: ['5432:5432'],
            status: 'stopped',
          },
          {
            bulkSelected: false,
            cpuPercent: 0,
            id: 'api-local-redis',
            memoryBytes: 12 * megabyte,
            name: 'redis',
            ports: ['6379:6379'],
            status: 'starting',
          },
          {
            bulkSelected: false,
            cpuPercent: null,
            id: 'api-local-mailpit',
            memoryBytes: null,
            name: 'mailpit',
            ports: ['8025:8025'],
            status: 'stopped',
          },
        ],
      },
      {
        active: false,
        id: 'docs-preview',
        name: 'Dokumentacja',
        services: [
          {
            bulkSelected: true,
            cpuPercent: null,
            id: 'docs-preview-site',
            memoryBytes: null,
            name: 'site',
            ports: ['4173:4173'],
            status: 'stopped',
          },
        ],
      },
    ],
    standaloneContainers: [
      {
        bulkSelected: false,
        cpuPercent: 0.1,
        id: 'standalone-minio',
        memoryBytes: 148 * megabyte,
        name: 'local-minio',
        ports: ['9000:9000', '9001:9001'],
        status: 'running',
      },
      {
        bulkSelected: false,
        cpuPercent: null,
        id: 'standalone-mailhog',
        memoryBytes: null,
        name: 'legacy-mailhog',
        ports: ['8026:8025'],
        status: 'stopped',
      },
    ],
  };
}

export const fixtureLogs: Record<string, string[]> = {
  'api-local-api': [
    '[12:36:14] [INFO] Starting api service...',
    '[12:36:15] [INFO] Loading environment from .env',
    '[12:36:15] [INFO] Connecting to database...',
    '[12:36:16] [INFO] Database connection ready',
    '[12:36:16] [INFO] Running migrations...',
    '[12:36:17] [INFO] Migrations completed',
    '[12:36:17] [INFO] Server listening on :3000',
    '[12:36:21] [INFO] GET /api/health 200',
    '[12:36:28] [INFO] GET /api/users 200',
    '[12:36:32] [INFO] POST /api/login 201',
    '[12:36:45] [INFO] GET /api/health 200',
  ],
};
