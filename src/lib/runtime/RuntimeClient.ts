import type {
  ApplicationSnapshot,
  Locale,
  ProjectAction,
  ProjectImportKind,
  ServiceAction,
  ServiceLogSnapshot,
  ThemePreference,
} from '../domain';

export type SnapshotListener = (snapshot: ApplicationSnapshot) => void;

export interface RuntimeClient {
  getSnapshot(): Promise<ApplicationSnapshot>;
  getServiceLogs(serviceId: string): Promise<ServiceLogSnapshot>;
  importProject(kind: ProjectImportKind): Promise<ApplicationSnapshot | null>;
  removeProject(projectId: string): Promise<ApplicationSnapshot>;
  runProjectAction(projectId: string, action: ProjectAction): Promise<ApplicationSnapshot>;
  runServiceAction(serviceId: string, action: ServiceAction): Promise<ApplicationSnapshot>;
  setBulkSelected(
    projectId: string,
    serviceId: string,
    selected: boolean,
  ): Promise<ApplicationSnapshot>;
  setLanguage(language: Locale): Promise<ApplicationSnapshot>;
  setProjectActive(projectId: string, active: boolean): Promise<ApplicationSnapshot>;
  setProjectProfiles(projectId: string, profiles: string[]): Promise<ApplicationSnapshot>;
  setTheme(theme: ThemePreference): Promise<ApplicationSnapshot>;
  stopAll(): Promise<ApplicationSnapshot>;
  subscribe(listener: SnapshotListener): Promise<() => void>;
}
