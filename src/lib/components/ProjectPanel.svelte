<script lang="ts">
  import type {
    Locale,
    ProjectAction,
    ProjectSnapshot,
    ServiceAction,
    ServiceSnapshot,
  } from '../domain';
  import { translate } from '../i18n';
  import Icon from './Icon.svelte';
  import ServiceTable from './ServiceTable.svelte';
  import Toggle from './Toggle.svelte';

  let {
    busyProjectId = null,
    busyServiceId = null,
    lifecycleActionsEnabled,
    logsEnabled,
    locale,
    onactive,
    onbulk,
    onprojectaction,
    onprofiles,
    onremove,
    onserviceaction,
    onlogs,
    project,
    selectedServiceId = null,
  }: {
    busyProjectId?: string | null;
    busyServiceId?: string | null;
    lifecycleActionsEnabled: boolean;
    logsEnabled: boolean;
    locale: Locale;
    onactive: (active: boolean) => void;
    onbulk: (service: ServiceSnapshot, selected: boolean) => void;
    onprojectaction: (action: ProjectAction) => void;
    onprofiles: (profiles: string[]) => void;
    onremove: () => void;
    onserviceaction: (service: ServiceSnapshot, action: ServiceAction) => void;
    onlogs: (service: ServiceSnapshot) => void;
    project: ProjectSnapshot;
    selectedServiceId?: string | null;
  } = $props();

  let expanded = $state(true);
  let selectAllElement = $state<HTMLInputElement>();

  const includedServices = $derived(project.services.filter((service) => service.included));
  const selectedCount = $derived(includedServices.filter((service) => service.bulkSelected).length);
  const allSelected = $derived(selectedCount > 0 && selectedCount === includedServices.length);
  const mixedSelection = $derived(selectedCount > 0 && !allSelected);

  $effect(() => {
    if (selectAllElement) selectAllElement.indeterminate = mixedSelection;
  });

  const selectAll = (selected: boolean): void => {
    for (const service of includedServices) onbulk(service, selected);
  };

  const toggleProfile = (profileName: string, enabled: boolean): void => {
    const profiles = project.profiles
      .filter((profile) => (profile.name === profileName ? enabled : profile.enabled))
      .map((profile) => profile.name);
    onprofiles(profiles);
  };
</script>

<section class="project-panel" data-testid={`project-${project.id}`}>
  <header class="project-header">
    <div class="project-identity">
      <button
        aria-expanded={expanded}
        aria-label={project.name}
        class="disclosure"
        onclick={() => (expanded = !expanded)}
        type="button"
      >
        <span class:expanded class="chevron"><Icon name="chevron" size={20} /></span>
        <Icon name="folder" size={24} />
        <strong>{project.name}</strong>
      </button>
      {#if project.profiles.length > 0}
        <div aria-label={translate(locale, 'profiles.label')} class="profile-controls" role="group">
          {#each project.profiles as profile (profile.name)}
            <label class:enabled={profile.enabled} class="profile-control">
              <input
                checked={profile.enabled}
                disabled={busyProjectId === project.id}
                onchange={(event) => toggleProfile(profile.name, event.currentTarget.checked)}
                type="checkbox"
              />
              <span>{profile.name}</span>
            </label>
          {/each}
        </div>
      {/if}
    </div>

    <div class="project-controls">
      <label class="active-control">
        <Toggle
          checked={project.active}
          disabled={busyProjectId === project.id}
          label={translate(locale, 'projects.activeToggle', { project: project.name })}
          onchange={onactive}
        />
        <span>
          {translate(locale, project.active ? 'projects.activeLabel' : 'projects.inactiveLabel')}
        </span>
      </label>
      <button
        class="primary-button"
        disabled={!lifecycleActionsEnabled || selectedCount === 0 || busyProjectId === project.id}
        onclick={() => onprojectaction('start-selected')}
        type="button"
      >
        <Icon name="play" size={16} />
        {translate(locale, 'action.startSelected')}
      </button>
      <button
        class="danger-button"
        disabled={!lifecycleActionsEnabled || selectedCount === 0 || busyProjectId === project.id}
        onclick={() => onprojectaction('stop-selected')}
        type="button"
      >
        <Icon name="stop" size={15} />
        {translate(locale, 'action.stopSelected')}
      </button>
      <button
        aria-label={translate(locale, 'projects.remove', { project: project.name })}
        class="project-remove-button"
        disabled={busyProjectId === project.id}
        onclick={onremove}
        title={translate(locale, 'projects.remove', { project: project.name })}
        type="button"
      >
        <Icon name="trash" size={17} />
        <span>{translate(locale, 'projects.removeAction')}</span>
      </button>
    </div>
  </header>

  {#if expanded}
    <div class="project-table-wrap">
      <div class="select-all-control">
        <input
          aria-label={translate(locale, 'bulk.selectAll', { project: project.name })}
          bind:this={selectAllElement}
          checked={allSelected}
          class="selection-checkbox"
          disabled={includedServices.length === 0 || busyProjectId === project.id}
          onchange={(event) => selectAll(event.currentTarget.checked)}
          type="checkbox"
        />
      </div>
      <ServiceTable
        actionsEnabled={lifecycleActionsEnabled}
        {busyServiceId}
        {locale}
        {logsEnabled}
        onaction={onserviceaction}
        {onbulk}
        {onlogs}
        {selectedServiceId}
        services={project.services}
      />
    </div>
  {/if}
</section>
