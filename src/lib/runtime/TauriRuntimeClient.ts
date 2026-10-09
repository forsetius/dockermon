import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type {
  ApplicationSnapshot,
  Locale,
  ProjectAction,
  ServiceAction,
  ServiceLogSnapshot,
  ThemePreference,
} from '../domain';
import type { RuntimeClient, SnapshotListener } from './RuntimeClient';

export class TauriRuntimeClient implements RuntimeClient {
  getSnapshot(): Promise<ApplicationSnapshot> {
    return invoke('application_snapshot');
  }

  getServiceLogs(serviceId: string): Promise<ServiceLogSnapshot> {
    return invoke('service_logs', { serviceId });
  }

  runProjectAction(projectId: string, action: ProjectAction): Promise<ApplicationSnapshot> {
    return invoke('run_project_action', { action, projectId });
  }

  runServiceAction(serviceId: string, action: ServiceAction): Promise<ApplicationSnapshot> {
    return invoke('run_service_action', { action, serviceId });
  }

  setBulkSelected(
    projectId: string,
    serviceId: string,
    selected: boolean,
  ): Promise<ApplicationSnapshot> {
    return invoke('set_bulk_selected', { projectId, selected, serviceId });
  }

  setLanguage(language: Locale): Promise<ApplicationSnapshot> {
    return invoke('set_language', { language });
  }

  setProjectActive(projectId: string, active: boolean): Promise<ApplicationSnapshot> {
    return invoke('set_project_active', { active, projectId });
  }

  setTheme(theme: ThemePreference): Promise<ApplicationSnapshot> {
    return invoke('set_theme', { theme });
  }

  stopAll(): Promise<ApplicationSnapshot> {
    return invoke('stop_all');
  }

  async subscribe(listener: SnapshotListener): Promise<() => void> {
    return listen<ApplicationSnapshot>('state-changed', (event) => listener(event.payload));
  }
}
