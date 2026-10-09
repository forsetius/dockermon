<script lang="ts">
  import { onMount, tick } from 'svelte';
  import type { Locale } from '../domain';
  import { translate } from '../i18n';
  import Icon from './Icon.svelte';
  import Toggle from './Toggle.svelte';

  let {
    follow,
    lines,
    loading,
    locale,
    onclear,
    onclose,
    onfollow,
    serviceName,
  }: {
    follow: boolean;
    lines: string[];
    loading: boolean;
    locale: Locale;
    onclear: () => void;
    onclose: () => void;
    onfollow: (follow: boolean) => void;
    serviceName: string;
  } = $props();

  let closeButton = $state<HTMLButtonElement>();
  let logViewport = $state<HTMLTextAreaElement>();
  const skeletonLines = [0, 1, 2, 3, 4, 5, 6];

  $effect(() => {
    const lineCount = lines.length;
    if (follow && logViewport && lineCount >= 0) {
      void tick().then(() => {
        if (logViewport) logViewport.scrollTop = logViewport.scrollHeight;
      });
    }
  });

  onMount(() => {
    const previousFocus =
      document.activeElement instanceof HTMLElement ? document.activeElement : null;
    closeButton?.focus();
    return () => previousFocus?.focus();
  });
</script>

<svelte:window onkeydown={(event) => event.key === 'Escape' && onclose()} />

<button
  aria-label={translate(locale, 'action.close')}
  class="drawer-backdrop"
  onclick={onclose}
  type="button"
></button>

<aside
  aria-busy={loading}
  aria-label={translate(locale, 'drawer.title', { service: serviceName })}
  class="log-drawer"
>
  <header class="drawer-header">
    <span aria-hidden="true" class="drawer-grip"></span>
    <h2>{translate(locale, 'drawer.title', { service: serviceName })}</h2>
    <button
      aria-label={translate(locale, 'action.close')}
      bind:this={closeButton}
      class="plain-icon-button"
      onclick={onclose}
      type="button"
    >
      <Icon name="close" size={20} />
    </button>
  </header>

  <div class="drawer-toolbar">
    <label>
      <Toggle checked={follow} label={translate(locale, 'drawer.follow')} onchange={onfollow} />
      <span>{translate(locale, 'drawer.follow')}</span>
    </label>
    <button
      class="text-button"
      disabled={loading || lines.length === 0}
      onclick={onclear}
      type="button"
    >
      <Icon name="trash" size={18} />
      {translate(locale, 'action.clear')}
    </button>
  </div>

  {#if loading}
    <div class="log-viewport">
      <div class="log-skeleton" aria-label={translate(locale, 'loading.title')}>
        {#each skeletonLines as skeletonLine (skeletonLine)}
          <span></span>
        {/each}
      </div>
    </div>
  {:else}
    <textarea
      aria-label={translate(locale, 'drawer.title', { service: serviceName })}
      aria-live="polite"
      class="log-viewport"
      bind:this={logViewport}
      placeholder={translate(locale, 'drawer.empty')}
      readonly
      value={lines.join('\n')}></textarea>
  {/if}
</aside>
