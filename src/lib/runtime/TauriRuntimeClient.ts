import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import type {
  ApplicationSnapshot,
  Locale,
  ProjectAction,
  ProjectImportKind,
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

  async importProject(kind: ProjectImportKind): Promise<ApplicationSnapshot | null> {
    const selection = await open({
      directory: kind === 'directory',
      filters:
        kind === 'files' ? [{ name: 'Docker Compose', extensions: ['yaml', 'yml'] }] : undefined,
      multiple: kind === 'files',
    });
    if (selection === null) return null;
    const paths = Array.isArray(selection) ? selection : [selection];
    return invoke('import_project', { paths });
  }

  removeProject(projectId: string): Promise<ApplicationSnapshot> {
    return invoke('remove_project', { projectId });
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

  setProjectProfiles(projectId: string, profiles: string[]): Promise<ApplicationSnapshot> {
    return invoke('set_project_profiles', { profiles, projectId });
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
