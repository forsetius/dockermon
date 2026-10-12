<script lang="ts">
  import dockermonMarkDark from '../../assets/dockermon-mark-dark.png';
  import dockermonMarkLight from '../../assets/dockermon-mark-light.png';
  import type { Locale, NavigationView } from '../domain';
  import { translate, type TranslationKey } from '../i18n';
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
    icon: 'containers' | 'projects';
    label: TranslationKey;
    view: NavigationView;
  }[] = [
    { icon: 'projects', label: 'nav.projects', view: 'projects' },
    { icon: 'containers', label: 'nav.containers', view: 'containers' },
  ];
</script>

<aside class="sidebar" id="primary-sidebar">
  <div class="brand">
    <span aria-hidden="true" class="brand-mark">
      <img alt="" class="brand-mark-light" draggable="false" src={dockermonMarkLight} />
      <img alt="" class="brand-mark-dark" draggable="false" src={dockermonMarkDark} />
    </span>
    <span class="brand-label">Dockermon</span>
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
