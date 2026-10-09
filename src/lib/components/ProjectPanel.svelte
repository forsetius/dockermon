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
    locale,
    onactive,
    onbulk,
    onprojectaction,
    onserviceaction,
    onlogs,
    project,
    selectedServiceId = null,
  }: {
    busyProjectId?: string | null;
    busyServiceId?: string | null;
    locale: Locale;
    onactive: (active: boolean) => void;
    onbulk: (service: ServiceSnapshot, selected: boolean) => void;
    onprojectaction: (action: ProjectAction) => void;
    onserviceaction: (service: ServiceSnapshot, action: ServiceAction) => void;
    onlogs: (service: ServiceSnapshot) => void;
    project: ProjectSnapshot;
    selectedServiceId?: string | null;
  } = $props();

  let expanded = $state(true);
  let selectAllElement = $state<HTMLInputElement>();

  const selectedCount = $derived(project.services.filter((service) => service.bulkSelected).length);
  const allSelected = $derived(selectedCount > 0 && selectedCount === project.services.length);
  const mixedSelection = $derived(selectedCount > 0 && !allSelected);

  $effect(() => {
    if (selectAllElement) selectAllElement.indeterminate = mixedSelection;
  });

  const selectAll = (selected: boolean): void => {
    for (const service of project.services) onbulk(service, selected);
  };
</script>

<section class="project-panel" data-testid={`project-${project.id}`}>
  <header class="project-header">
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
        disabled={selectedCount === 0 || busyProjectId === project.id}
        onclick={() => onprojectaction('start-selected')}
        type="button"
      >
        <Icon name="play" size={16} />
        {translate(locale, 'action.startSelected')}
      </button>
      <button
        class="danger-button"
        disabled={selectedCount === 0 || busyProjectId === project.id}
        onclick={() => onprojectaction('stop-selected')}
        type="button"
      >
        <Icon name="stop" size={15} />
        {translate(locale, 'action.stopSelected')}
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
          onchange={(event) => selectAll(event.currentTarget.checked)}
          type="checkbox"
        />
      </div>
      <ServiceTable
        {busyServiceId}
        {locale}
        onaction={onserviceaction}
        {onbulk}
        {onlogs}
        {selectedServiceId}
        services={project.services}
      />
    </div>
  {/if}
</section>
