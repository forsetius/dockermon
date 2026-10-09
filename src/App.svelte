<script lang="ts">
  import { onMount } from 'svelte';
  import type {
    ApplicationSnapshot,
    Locale,
    NavigationView,
    ProjectAction,
    ProjectImportKind,
    ServiceAction,
    ServiceLogBatch,
    ServiceSnapshot,
    ThemePreference,
  } from './lib/domain';
  import { translate, translateActiveProjectCount, type TranslationKey } from './lib/i18n';
  import { applyTheme } from './lib/theme';
  import ActivityView from './lib/components/ActivityView.svelte';
  import EmptyState from './lib/components/EmptyState.svelte';
  import Icon from './lib/components/Icon.svelte';
  import LoadingState from './lib/components/LoadingState.svelte';
  import LogDrawer from './lib/components/LogDrawer.svelte';
  import PreferencesView from './lib/components/PreferencesView.svelte';
  import ProjectPanel from './lib/components/ProjectPanel.svelte';
  import ServiceTable from './lib/components/ServiceTable.svelte';
  import Sidebar from './lib/components/Sidebar.svelte';
  import Toast from './lib/components/Toast.svelte';
  import WindowHeader from './lib/components/WindowHeader.svelte';
  import { createRuntimeClient } from './lib/runtime/createRuntimeClient';
  import type { RuntimeClient } from './lib/runtime/RuntimeClient';

  let { runtimeClient = createRuntimeClient() }: { runtimeClient?: RuntimeClient } = $props();

  let snapshot = $state<ApplicationSnapshot | null>(null);
  let loadState = $state<'error' | 'loading' | 'ready'>('loading');
  let currentView = $state<NavigationView>('projects');
  let busyProjectIds = $state<string[]>([]);
  let importInProgress = $state(false);
  let busyStandaloneServiceIds = $state<string[]>([]);
  let selectedServiceId = $state<string | null>(null);
  let logLines = $state<string[]>([]);
  let logsLoading = $state(false);
  let followLogs = $state(true);
  let toast = $state<{ details?: string[]; message: string; tone: 'error' | 'success' } | null>(
    null,
  );
  let lastGlobalStopReportSequence = 0;
  let logRequestSequence = 0;
  let stopLogSubscription: (() => void) | undefined;

  const logLineLimit = 2000;

  const fallbackLocale: Locale = navigator.language.toLowerCase().startsWith('pl') ? 'pl' : 'en';
  const locale = $derived(snapshot?.preferences.language ?? fallbackLocale);
  const activeCount = $derived(snapshot?.projects.filter((project) => project.active).length ?? 0);
  const selectedService = $derived.by(() => {
    if (!snapshot || !selectedServiceId) return null;
    return [
      ...snapshot.projects.flatMap((project) => project.services),
      ...snapshot.standaloneContainers,
    ].find((service) => service.id === selectedServiceId);
  });

  $effect(() => {
    applyTheme(snapshot?.preferences.theme ?? 'system');
    document.documentElement.lang = locale;
  });

  onMount(() => {
    let unsubscribe: (() => void) | undefined;
    void loadSnapshot();
    void runtimeClient
      .subscribe((nextSnapshot) => {
        receiveSnapshot(nextSnapshot);
      })
      .then((stopSubscription) => {
        unsubscribe = stopSubscription;
      });

    return () => {
      logRequestSequence += 1;
      stopActiveLogSubscription();
      unsubscribe?.();
    };
  });

  async function loadSnapshot(): Promise<void> {
    loadState = 'loading';
    try {
      receiveSnapshot(await runtimeClient.getSnapshot());
      loadState = 'ready';
    } catch {
      loadState = 'error';
    }
  }

  async function updateProjectActive(projectId: string, active: boolean): Promise<void> {
    setProjectBusy(projectId, true);
    try {
      snapshot = await runtimeClient.setProjectActive(projectId, active);
    } catch (error) {
      showError(error);
    } finally {
      setProjectBusy(projectId, false);
    }
  }

  async function updateProjectProfiles(projectId: string, profiles: string[]): Promise<void> {
    setProjectBusy(projectId, true);
    try {
      snapshot = await runtimeClient.setProjectProfiles(projectId, profiles);
    } catch (error) {
      showError(error);
    } finally {
      setProjectBusy(projectId, false);
    }
  }

  async function removeProject(projectId: string, projectName: string): Promise<void> {
    setProjectBusy(projectId, true);
    try {
      snapshot = await runtimeClient.removeProject(projectId);
      toast = {
        message: translate(locale, 'projects.removed', { project: projectName }),
        tone: 'success',
      };
    } catch (error) {
      showError(error);
    } finally {
      setProjectBusy(projectId, false);
    }
  }

  async function importProject(kind: ProjectImportKind): Promise<void> {
    importInProgress = true;
    try {
      const importedSnapshot = await runtimeClient.importProject(kind);
      if (importedSnapshot) {
        snapshot = importedSnapshot;
        toast = { message: translate(locale, 'projects.imported'), tone: 'success' };
      }
    } catch (error) {
      showError(error);
    } finally {
      importInProgress = false;
    }
  }

  async function updateBulkSelection(
    projectId: string,
    service: ServiceSnapshot,
    selected: boolean,
  ): Promise<void> {
    try {
      snapshot = await runtimeClient.setBulkSelected(projectId, service.id, selected);
    } catch (error) {
      showError(error);
    }
  }

  async function runProjectAction(
    projectId: string,
    projectName: string,
    action: ProjectAction,
  ): Promise<void> {
    setProjectBusy(projectId, true);
    try {
      snapshot = await runtimeClient.runProjectAction(projectId, action);
      const actionKey: TranslationKey =
        action === 'start-selected' ? 'action.startSelected' : 'action.stopSelected';
      showSuccess(projectName, translate(locale, actionKey));
    } catch (error) {
      showError(error);
    } finally {
      setProjectBusy(projectId, false);
    }
  }

  async function runServiceAction(service: ServiceSnapshot, action: ServiceAction): Promise<void> {
    const projectId = snapshot?.projects.find((project) =>
      project.services.some((candidate) => candidate.id === service.id),
    )?.id;
    if (projectId) setProjectBusy(projectId, true);
    else setStandaloneServiceBusy(service.id, true);
    try {
      snapshot = await runtimeClient.runServiceAction(service.id, action);
      const actionKeys: Record<ServiceAction, TranslationKey> = {
        restart: 'action.restart',
        resume: 'action.resume',
        start: 'action.start',
        stop: 'action.stop',
      };
      showSuccess(service.name, translate(locale, actionKeys[action], { service: service.name }));
    } catch (error) {
      showError(error);
    } finally {
      if (projectId) setProjectBusy(projectId, false);
      else setStandaloneServiceBusy(service.id, false);
    }
  }

  async function openLogs(service: ServiceSnapshot): Promise<void> {
    const requestSequence = ++logRequestSequence;
    stopActiveLogSubscription();
    selectedServiceId = service.id;
    logsLoading = true;
    followLogs = true;
    logLines = [];
    try {
      const stopSubscription = await runtimeClient.subscribeServiceLogs(service.id, (batch) =>
        receiveLogBatch(service.id, requestSequence, batch),
      );
      if (requestSequence !== logRequestSequence || selectedServiceId !== service.id) {
        stopSubscription();
        return;
      }
      stopLogSubscription = stopSubscription;
    } catch (error) {
      if (requestSequence === logRequestSequence) showError(error);
    } finally {
      if (requestSequence === logRequestSequence) logsLoading = false;
    }
  }

  function closeLogs(): void {
    logRequestSequence += 1;
    stopActiveLogSubscription();
    selectedServiceId = null;
    logLines = [];
    logsLoading = false;
  }

  async function changeTheme(theme: ThemePreference): Promise<void> {
    try {
      snapshot = await runtimeClient.setTheme(theme);
    } catch (error) {
      showError(error);
    }
  }

  async function changeLanguage(language: Locale): Promise<void> {
    try {
      snapshot = await runtimeClient.setLanguage(language);
    } catch (error) {
      showError(error);
    }
  }

  function showSuccess(subject: string, action: string): void {
    toast = {
      message: translate(locale, 'operation.finished', { action, subject }),
      tone: 'success',
    };
  }

  function showError(error?: unknown): void {
    const code =
      typeof error === 'object' && error !== null && 'code' in error
        ? String(error.code)
        : undefined;
    const errorKeys: Partial<Record<string, TranslationKey>> = {
      COMPOSE_CLI_NOT_FOUND: 'error.composeCliNotFound',
      COMPOSE_ACTION_FAILED: 'error.composeActionFailed',
      COMPOSE_CONFIG_UNAVAILABLE: 'error.composeConfigInvalid',
      COMPOSE_CONFIG_INVALID: 'error.composeConfigInvalid',
      COMPOSE_FILE_NOT_FOUND: 'error.composeFileNotFound',
      CONFIG_INVALID: 'error.configInvalid',
      CONFIG_READ_FAILED: 'error.configInvalid',
      CONFIG_VERSION_UNSUPPORTED: 'error.configInvalid',
      CONFIG_WRITE_FAILED: 'error.configWriteFailed',
      CONTAINER_ACTION_FAILED: 'error.containerActionFailed',
      CONTAINER_OPERATION_IN_PROGRESS: 'error.containerOperationInProgress',
      DOCKER_PERMISSION_DENIED: 'error.dockerPermissionDenied',
      DOCKER_UNAVAILABLE: 'error.dockerUnavailable',
      GLOBAL_STOP_IN_PROGRESS: 'error.globalStopInProgress',
      LOGS_READ_FAILED: 'error.logsReadFailed',
      LOG_STREAM_FAILED: 'error.logStreamFailed',
      NO_SERVICES_SELECTED: 'error.noServicesSelected',
      PROJECT_SOURCE_UNAVAILABLE: 'error.projectSourceUnavailable',
      PROJECT_OPERATION_IN_PROGRESS: 'error.projectOperationInProgress',
      SERVICE_HAS_NO_CONTAINERS: 'error.serviceHasNoContainers',
      SERVICE_NOT_FOUND: 'error.serviceNotFound',
    };
    toast = {
      message: translate(locale, (code && errorKeys[code]) || 'operation.failed'),
      tone: 'error',
    };
  }

  function setProjectBusy(projectId: string, busy: boolean): void {
    busyProjectIds = busy
      ? [...busyProjectIds.filter((candidate) => candidate !== projectId), projectId]
      : busyProjectIds.filter((candidate) => candidate !== projectId);
  }

  function setStandaloneServiceBusy(serviceId: string, busy: boolean): void {
    busyStandaloneServiceIds = busy
      ? [...busyStandaloneServiceIds.filter((candidate) => candidate !== serviceId), serviceId]
      : busyStandaloneServiceIds.filter((candidate) => candidate !== serviceId);
  }

  function receiveLogBatch(
    serviceId: string,
    requestSequence: number,
    batch: ServiceLogBatch,
  ): void {
    if (
      requestSequence !== logRequestSequence ||
      selectedServiceId !== serviceId ||
      batch.serviceId !== serviceId
    )
      return;
    if (batch.error) showError(batch.error);
    if (batch.lines.length > 0) {
      logLines = [...logLines, ...batch.lines].slice(-logLineLimit);
    }
  }

  function stopActiveLogSubscription(): void {
    stopLogSubscription?.();
    stopLogSubscription = undefined;
  }

  function receiveSnapshot(nextSnapshot: ApplicationSnapshot): void {
    snapshot = nextSnapshot;
    const report = nextSnapshot.globalStopReport;
    if (!report || report.sequence <= lastGlobalStopReportSequence) return;
    lastGlobalStopReportSequence = report.sequence;
    toast = {
      details: report.failures.map((failure) =>
        translate(locale, 'globalStop.failure', { container: failure.containerName }),
      ),
      message: translate(locale, 'globalStop.completed', {
        stopped: report.stopped,
        total: report.total,
      }),
      tone: report.failures.length === 0 ? 'success' : 'error',
    };
  }
</script>

<svelte:head>
  <meta name="color-scheme" content="light dark" />
  <title>Dockermon</title>
</svelte:head>

<div class="app-shell" class:drawer-open={selectedService !== null}>
  <Sidebar {currentView} {locale} onselect={(view) => (currentView = view)} />
  <WindowHeader connection={snapshot?.connection ?? 'connecting'} {locale} />

  <main class="content-area">
    {#if loadState === 'loading'}
      <LoadingState {locale} />
    {:else if loadState === 'error'}
      <EmptyState kind="error" {locale} onretry={loadSnapshot} />
    {:else if snapshot}
      {#if snapshot.globalStopInProgress}
        <div aria-live="polite" class="global-stop-progress" role="status">
          <Icon name="stop" size={16} />
          {#if snapshot.globalStopProgress && snapshot.globalStopProgress.total > 0}
            {translate(locale, 'globalStop.progress', {
              completed: snapshot.globalStopProgress.completed,
              total: snapshot.globalStopProgress.total,
            })}
          {:else}
            {translate(locale, 'globalStop.preparing')}
          {/if}
        </div>
      {/if}
      {#if currentView === 'projects'}
        <section aria-labelledby="projects-title" class="projects-view">
          <header class="page-heading projects-heading">
            <div>
              <div class="title-line">
                <h1 id="projects-title">{translate(locale, 'projects.title')}</h1>
                <span class="count-badge">
                  {translateActiveProjectCount(locale, activeCount)}
                </span>
              </div>
              {#if activeCount === 0}
                <p>{translate(locale, 'projects.noneActive')}</p>
              {/if}
            </div>
            <div class="import-actions">
              <button
                class="text-button"
                disabled={importInProgress}
                onclick={() => importProject('directory')}
                type="button"
              >
                <Icon name="folder-plus" size={17} />
                {translate(locale, 'projects.importDirectory')}
              </button>
              <button
                class="primary-button"
                disabled={importInProgress}
                onclick={() => importProject('files')}
                type="button"
              >
                <Icon name="files" size={17} />
                {translate(locale, 'projects.importFiles')}
              </button>
            </div>
          </header>

          {#if snapshot.projects.length === 0}
            <EmptyState kind="empty" {locale} onretry={loadSnapshot} />
          {:else}
            <div class="project-list">
              {#each snapshot.projects as project (project.id)}
                <ProjectPanel
                  busy={busyProjectIds.includes(project.id)}
                  lifecycleActionsEnabled={snapshot.capabilities.lifecycleActions &&
                    !snapshot.globalStopInProgress}
                  {locale}
                  logsEnabled={snapshot.capabilities.logs}
                  onactive={(active) => updateProjectActive(project.id, active)}
                  onbulk={(service, selected) => updateBulkSelection(project.id, service, selected)}
                  onlogs={openLogs}
                  onprojectaction={(action) => runProjectAction(project.id, project.name, action)}
                  onprofiles={(profiles) => updateProjectProfiles(project.id, profiles)}
                  onremove={() => removeProject(project.id, project.name)}
                  onserviceaction={runServiceAction}
                  {project}
                  {selectedServiceId}
                />
              {/each}
            </div>
          {/if}
        </section>
      {:else if currentView === 'containers'}
        <section aria-labelledby="containers-title" class="secondary-view">
          <header class="page-heading">
            <h1 id="containers-title">{translate(locale, 'containers.title')}</h1>
            <p>{translate(locale, 'containers.description')}</p>
          </header>
          {#if snapshot.standaloneContainers.length === 0}
            <p class="quiet-empty">{translate(locale, 'containers.empty')}</p>
          {:else}
            <div class="standalone-panel">
              <ServiceTable
                actionsEnabled={snapshot.capabilities.lifecycleActions &&
                  !snapshot.globalStopInProgress}
                busyServiceIds={busyStandaloneServiceIds}
                {locale}
                logsEnabled={snapshot.capabilities.logs}
                onaction={runServiceAction}
                onlogs={openLogs}
                {selectedServiceId}
                services={snapshot.standaloneContainers}
                showBulk={false}
              />
            </div>
          {/if}
        </section>
      {:else if currentView === 'activity'}
        <ActivityView activity={snapshot.activity} {locale} />
      {:else}
        <PreferencesView
          {locale}
          onchangeLanguage={changeLanguage}
          onchangeTheme={changeTheme}
          preferences={snapshot.preferences}
        />
      {/if}
    {/if}
  </main>

  {#if selectedService}
    {#key selectedService.id}
      <LogDrawer
        follow={followLogs}
        lines={logLines}
        loading={logsLoading}
        {locale}
        onclear={() => (logLines = [])}
        onclose={closeLogs}
        onfollow={(follow) => (followLogs = follow)}
        serviceName={selectedService.name}
      />
    {/key}
  {/if}

  {#if toast}
    <Toast
      details={toast.details}
      {locale}
      message={toast.message}
      ondismiss={() => (toast = null)}
      tone={toast.tone}
    />
  {/if}
</div>
