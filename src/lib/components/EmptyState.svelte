<script lang="ts">
  import type { Locale } from '../domain';
  import { translate, type TranslationKey } from '../i18n';
  import Icon from './Icon.svelte';

  let {
    kind,
    locale,
    onretry,
  }: {
    kind: 'empty' | 'error';
    locale: Locale;
    onretry: () => void;
  } = $props();

  const titleKeys: Record<'empty' | 'error', TranslationKey> = {
    empty: 'empty.title',
    error: 'error.title',
  };
  const descriptionKeys: Record<'empty' | 'error', TranslationKey> = {
    empty: 'empty.description',
    error: 'error.description',
  };
</script>

<section class="state-panel" class:error-state={kind === 'error'}>
  <span aria-hidden="true" class="state-icon">
    <Icon name={kind === 'error' ? 'activity' : 'folder'} size={28} />
  </span>
  <h1>{translate(locale, titleKeys[kind])}</h1>
  <p>{translate(locale, descriptionKeys[kind])}</p>
  <button class="primary-button" onclick={onretry} type="button">
    <Icon name="restart" size={17} />
    {translate(locale, 'action.refresh')}
  </button>
</section>
