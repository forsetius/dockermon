<script lang="ts">
  import type { Locale } from '../domain';
  import { translate } from '../i18n';
  import Icon from './Icon.svelte';

  let {
    details = [],
    locale,
    message,
    ondismiss,
    tone = 'success',
  }: {
    details?: string[];
    locale: Locale;
    message: string;
    ondismiss: () => void;
    tone?: 'error' | 'success';
  } = $props();
</script>

<div aria-live="polite" class="toast {tone}" role="status">
  <div class="toast-content">
    <span>{message}</span>
    {#if details.length > 0}
      <ul>
        {#each details as detail (detail)}<li>{detail}</li>{/each}
      </ul>
    {/if}
  </div>
  <button aria-label={translate(locale, 'toast.dismiss')} onclick={ondismiss} type="button">
    <Icon name="close" size={16} />
  </button>
</div>
