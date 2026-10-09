<script lang="ts">
  import type { Locale, NavigationView } from '../domain';
  import { translate, type TranslationKey } from '../i18n';
  import DockermonMark from '../DockermonMark.svelte';
  import Icon from './Icon.svelte';

  let {
    currentView,
    locale,
    onselect,
  }: {
    currentView: NavigationView;
    locale: Locale;
    onselect: (view: NavigationView) => void;
  } = $props();

  const primaryItems: {
    icon: 'activity' | 'containers' | 'projects';
    label: TranslationKey;
    view: NavigationView;
  }[] = [
    { icon: 'projects', label: 'nav.projects', view: 'projects' },
    { icon: 'containers', label: 'nav.containers', view: 'containers' },
    { icon: 'activity', label: 'nav.activity', view: 'activity' },
  ];
</script>

<aside class="sidebar">
  <div class="brand">
    <DockermonMark size={38} />
    <span>Dockermon</span>
  </div>

  <nav aria-label="Dockermon">
    {#each primaryItems as item (item.view)}
      <button
        aria-current={currentView === item.view ? 'page' : undefined}
        class:active={currentView === item.view}
        onclick={() => onselect(item.view)}
        type="button"
      >
        <Icon name={item.icon} size={21} />
        <span>{translate(locale, item.label)}</span>
      </button>
    {/each}
    <span aria-hidden="true" class="navigation-spacer"></span>
    <span class="sample-note">{translate(locale, 'app.sampleData')}</span>
    <span aria-hidden="true" class="navigation-separator"></span>
    <button
      aria-current={currentView === 'settings' ? 'page' : undefined}
      class:active={currentView === 'settings'}
      onclick={() => onselect('settings')}
      type="button"
    >
      <Icon name="settings" size={21} />
      <span>{translate(locale, 'nav.settings')}</span>
    </button>
  </nav>
</aside>
