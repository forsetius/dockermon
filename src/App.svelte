<script lang="ts">
  import { onMount } from 'svelte';
  import type {
    ApplicationSnapshot,
    Locale,
    NavigationView,
    ProjectAction,
    ServiceAction,
    ServiceSnapshot,
    ThemePreference,
  } from './lib/domain';
  import { translate, translateActiveProjectCount, type TranslationKey } from './lib/i18n';
  import { applyTheme } from './lib/theme';
  import ActivityView from './lib/components/ActivityView.svelte';
  import EmptyState from './lib/components/EmptyState.svelte';
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
  let busyProjectId = $state<string | null>(null);
  let busyServiceId = $state<string | null>(null);
  let selectedServiceId = $state<string | null>(null);
  let logLines = $state<string[]>([]);
  let logsLoading = $state(false);
  let followLogs = $state(true);
  let toast = $state<{ message: string; tone: 'error' | 'success' } | null>(null);

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
        snapshot = nextSnapshot;
      })
      .then((stopSubscription) => {
        unsubscribe = stopSubscription;
      });

    const liveLogTimer = window.setInterval(() => {
      if (!selectedService || !followLogs || logsLoading) return;
      const timestamp = new Intl.DateTimeFormat('en-GB', {
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
      }).format(new Date());
      logLines = [...logLines.slice(-198), `[${timestamp}] [INFO] GET /api/health 200`];
    }, 4000);

    return () => {
      window.clearInterval(liveLogTimer);
      unsubscribe?.();
    };
  });

  async function loadSnapshot(): Promise<void> {
    loadState = 'loading';
    try {
      snapshot = await runtimeClient.getSnapshot();
      loadState = 'ready';
    } catch {
      loadState = 'error';
    }
  }

  async function updateProjectActive(projectId: string, active: boolean): Promise<void> {
    busyProjectId = projectId;
    try {
      snapshot = await runtimeClient.setProjectActive(projectId, active);
    } catch {
      showError();
    } finally {
      busyProjectId = null;
    }
  }

  async function updateBulkSelection(
    projectId: string,
    service: ServiceSnapshot,
    selected: boolean,
  ): Promise<void> {
    try {
      snapshot = await runtimeClient.setBulkSelected(projectId, service.id, selected);
    } catch {
      showError();
    }
  }

  async function runProjectAction(
    projectId: string,
    projectName: string,
    action: ProjectAction,
  ): Promise<void> {
    busyProjectId = projectId;
    try {
      snapshot = await runtimeClient.runProjectAction(projectId, action);
      const actionKey: TranslationKey =
        action === 'start-selected' ? 'action.startSelected' : 'action.stopSelected';
      showSuccess(projectName, translate(locale, actionKey));
    } catch {
      showError();
    } finally {
      busyProjectId = null;
    }
  }

  async function runServiceAction(service: ServiceSnapshot, action: ServiceAction): Promise<void> {
    busyServiceId = service.id;
    try {
      snapshot = await runtimeClient.runServiceAction(service.id, action);
      const actionKey: TranslationKey =
        action === 'start' ? 'action.start' : action === 'stop' ? 'action.stop' : 'action.restart';
      showSuccess(service.name, translate(locale, actionKey, { service: service.name }));
    } catch {
      showError();
    } finally {
      busyServiceId = null;
    }
  }

  async function openLogs(service: ServiceSnapshot): Promise<void> {
    selectedServiceId = service.id;
    logsLoading = true;
    logLines = [];
    try {
      const result = await runtimeClient.getServiceLogs(service.id);
      if (selectedServiceId === result.serviceId) logLines = result.lines;
    } catch {
      showError();
    } finally {
      logsLoading = false;
    }
  }

  function closeLogs(): void {
    selectedServiceId = null;
    logLines = [];
  }

  async function changeTheme(theme: ThemePreference): Promise<void> {
    snapshot = await runtimeClient.setTheme(theme);
  }

  async function changeLanguage(language: Locale): Promise<void> {
    snapshot = await runtimeClient.setLanguage(language);
  }

  function showSuccess(subject: string, action: string): void {
    toast = {
      message: translate(locale, 'operation.finished', { action, subject }),
      tone: 'success',
    };
  }

  function showError(): void {
    toast = { message: translate(locale, 'operation.failed'), tone: 'error' };
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
      {#if currentView === 'projects'}
        <section aria-labelledby="projects-title" class="projects-view">
          <header class="page-heading">
            <div class="title-line">
              <h1 id="projects-title">{translate(locale, 'projects.title')}</h1>
              <span class="count-badge">
                {translateActiveProjectCount(locale, activeCount)}
              </span>
            </div>
            {#if activeCount === 0}
              <p>{translate(locale, 'projects.noneActive')}</p>
            {/if}
          </header>

          {#if snapshot.projects.length === 0}
            <EmptyState kind="empty" {locale} onretry={loadSnapshot} />
          {:else}
            <div class="project-list">
              {#each snapshot.projects as project (project.id)}
                <ProjectPanel
                  {busyProjectId}
                  {busyServiceId}
                  lifecycleActionsEnabled={snapshot.capabilities.lifecycleActions}
                  {locale}
                  logsEnabled={snapshot.capabilities.logs}
                  onactive={(active) => updateProjectActive(project.id, active)}
                  onbulk={(service, selected) => updateBulkSelection(project.id, service, selected)}
                  onlogs={openLogs}
                  onprojectaction={(action) => runProjectAction(project.id, project.name, action)}
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
                actionsEnabled={snapshot.capabilities.lifecycleActions}
                {busyServiceId}
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
  {/if}

  {#if toast}
    <Toast {locale} message={toast.message} ondismiss={() => (toast = null)} tone={toast.tone} />
  {/if}
</div>
