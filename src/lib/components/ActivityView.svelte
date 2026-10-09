<script lang="ts">
  import type { ActivityEntry, Locale } from '../domain';
  import { translate, type TranslationKey } from '../i18n';
  import Icon from './Icon.svelte';

  let { activity, locale }: { activity: ActivityEntry[]; locale: Locale } = $props();

  const actionKeys: Record<string, TranslationKey> = {
    restart: 'activity.restart',
    start: 'activity.start',
    'start-selected': 'activity.start-selected',
    stop: 'activity.stop',
    'stop-all': 'activity.stop-all',
    'stop-selected': 'activity.stop-selected',
  };
</script>

<section aria-labelledby="activity-title" class="secondary-view">
  <header class="page-heading">
    <h1 id="activity-title">{translate(locale, 'activity.title')}</h1>
  </header>

  {#if activity.length === 0}
    <p class="quiet-empty">{translate(locale, 'activity.empty')}</p>
  {:else}
    <ol class="activity-list">
      {#each activity as entry (entry.id)}
        <li>
          <span aria-hidden="true" class:failed={!entry.successful} class="activity-icon">
            <Icon name={entry.action.includes('stop') ? 'stop' : 'restart'} size={17} />
          </span>
          <div>
            <strong>{entry.subject}</strong>
            <span>{translate(locale, actionKeys[entry.action] ?? 'activity.stop')}</span>
          </div>
          <time>{entry.occurredAt}</time>
        </li>
      {/each}
    </ol>
  {/if}
</section>
