<script lang="ts">
  import type { Locale } from '../domain';
  import { translate } from '../i18n';
  import Icon from './Icon.svelte';

  let {
    locale,
    message,
    ondismiss,
    tone = 'success',
  }: {
    locale: Locale;
    message: string;
    ondismiss: () => void;
    tone?: 'error' | 'success';
  } = $props();
</script>

<div aria-live="polite" class="toast {tone}" role="status">
  <span>{message}</span>
  <button aria-label={translate(locale, 'toast.dismiss')} onclick={ondismiss} type="button">
    <Icon name="close" size={16} />
  </button>
</div>
