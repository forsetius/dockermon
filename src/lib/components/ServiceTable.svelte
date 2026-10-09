<script lang="ts">
  import type { Locale, ServiceAction, ServiceSnapshot, ServiceStatus } from '../domain';
  import { isServiceRunning } from '../domain';
  import { translate, type TranslationKey } from '../i18n';
  import Icon from './Icon.svelte';

  let {
    actionsEnabled = true,
    actionsBlocked = false,
    busyServiceIds = [],
    locale,
    logsEnabled = true,
    onaction,
    onbulk,
    onlogs,
    selectedServiceId = null,
    services,
    showBulk = true,
  }: {
    actionsEnabled?: boolean;
    actionsBlocked?: boolean;
    busyServiceIds?: readonly string[];
    locale: Locale;
    logsEnabled?: boolean;
    onaction: (service: ServiceSnapshot, action: ServiceAction) => void;
    onbulk?: (service: ServiceSnapshot, selected: boolean) => void;
    onlogs: (service: ServiceSnapshot) => void;
    selectedServiceId?: string | null;
    services: ServiceSnapshot[];
    showBulk?: boolean;
  } = $props();

  const statusKeys: Record<ServiceStatus, TranslationKey> = {
    error: 'status.error',
    healthy: 'status.healthy',
    'not-created': 'status.not-created',
    paused: 'status.paused',
    running: 'status.running',
    starting: 'status.starting',
    stopped: 'status.stopped',
    unhealthy: 'status.unhealthy',
  };

  const formatCpu = (value: number | null): string =>
    value === null
      ? '—'
      : `${new Intl.NumberFormat(locale, { maximumFractionDigits: 1 }).format(value)}%`;

  const formatMemory = (value: number | null): string =>
    value === null
      ? '—'
      : `${new Intl.NumberFormat(locale, { maximumFractionDigits: 0 }).format(value / 1024 / 1024)} MB`;

  const canStart = (status: ServiceStatus): boolean =>
    !isServiceRunning(status) && status !== 'starting';
  const canStop = (status: ServiceStatus): boolean =>
    isServiceRunning(status) || status === 'paused' || status === 'starting';
  const canRestart = (status: ServiceStatus): boolean => isServiceRunning(status);
</script>

<div class="service-table" class:no-bulk={!showBulk} role="table">
  <div class="service-row table-header" role="row">
    {#if showBulk}<span aria-hidden="true" class="bulk-cell"></span>{/if}
    <span role="columnheader">{translate(locale, 'service.name')}</span>
    <span role="columnheader">{translate(locale, 'service.status')}</span>
    <span class="metric-heading resource-cpu" role="columnheader"
      >{translate(locale, 'service.cpu')}</span
    >
    <span class="metric-heading resource-memory" role="columnheader"
      >{translate(locale, 'service.memory')}</span
    >
    <span class="ports-column" role="columnheader">{translate(locale, 'service.ports')}</span>
    <span role="columnheader">{translate(locale, 'service.actions')}</span>
  </div>

  {#each services as service (service.id)}
    <div
      class="service-row service-data-row"
      class:selected={selectedServiceId === service.id}
      class:excluded={!service.included}
      data-testid={`service-${service.id}`}
      role="row"
    >
      {#if showBulk}
        <span class="bulk-cell" role="cell">
          <input
            aria-label={`${translate(locale, 'service.name')}: ${service.name}`}
            checked={service.bulkSelected}
            class="selection-checkbox"
            disabled={!service.included}
            onchange={(event) => onbulk?.(service, event.currentTarget.checked)}
            type="checkbox"
          />
        </span>
      {/if}
      <span class="service-name" data-label={translate(locale, 'service.name')} role="cell">
        <Icon name="containers" size={19} />
        <strong>{service.name}</strong>
        {#if service.profiles.length > 0}
          <span class="service-profile">
            {service.profiles.join(', ')}
            {#if !service.included}· {translate(locale, 'status.profile-disabled')}{/if}
          </span>
        {/if}
      </span>
      <span class="service-status" data-label={translate(locale, 'service.status')} role="cell">
        <span aria-hidden="true" class="status-dot {service.status}"></span>
        <span>{translate(locale, statusKeys[service.status])}</span>
      </span>
      <span class="metric resource-cpu" data-label={translate(locale, 'service.cpu')} role="cell"
        >{formatCpu(service.cpuPercent)}</span
      >
      <span
        class="metric resource-memory"
        data-label={translate(locale, 'service.memory')}
        role="cell">{formatMemory(service.memoryBytes)}</span
      >
      <span class="ports ports-column" data-label={translate(locale, 'service.ports')} role="cell">
        {service.ports.length > 0 ? service.ports.join(', ') : '—'}
      </span>
      <span class="row-actions" role="cell">
        <button
          aria-label={translate(
            locale,
            service.status === 'paused' ? 'action.resume' : 'action.start',
            { service: service.name },
          )}
          class="icon-button"
          disabled={!service.included ||
            !actionsEnabled ||
            !canStart(service.status) ||
            actionsBlocked ||
            busyServiceIds.includes(service.id)}
          onclick={() => onaction(service, service.status === 'paused' ? 'resume' : 'start')}
          title={translate(locale, service.status === 'paused' ? 'action.resume' : 'action.start', {
            service: service.name,
          })}
          type="button"
        >
          <Icon name="play" size={17} />
        </button>
        <button
          aria-label={translate(locale, 'action.stop', { service: service.name })}
          class="icon-button"
          disabled={!service.included ||
            !actionsEnabled ||
            !canStop(service.status) ||
            actionsBlocked ||
            busyServiceIds.includes(service.id)}
          onclick={() => onaction(service, 'stop')}
          title={translate(locale, 'action.stop', { service: service.name })}
          type="button"
        >
          <Icon name="stop" size={16} />
        </button>
        <button
          aria-label={translate(locale, 'action.restart', { service: service.name })}
          class="icon-button"
          disabled={!service.included ||
            !actionsEnabled ||
            !canRestart(service.status) ||
            actionsBlocked ||
            busyServiceIds.includes(service.id)}
          onclick={() => onaction(service, 'restart')}
          title={translate(locale, 'action.restart', { service: service.name })}
          type="button"
        >
          <Icon name="restart" size={18} />
        </button>
        <button
          aria-label={translate(locale, 'action.logs', { service: service.name })}
          class="icon-button"
          disabled={!service.included || !logsEnabled}
          onclick={() => onlogs(service)}
          title={translate(locale, 'action.logs', { service: service.name })}
          type="button"
        >
          <Icon name="logs" size={17} />
        </button>
      </span>
    </div>
  {/each}
</div>
