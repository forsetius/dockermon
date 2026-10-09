<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import type { ConnectionStatus, Locale } from '../domain';
  import { translate, type TranslationKey } from '../i18n';
  import Icon from './Icon.svelte';

  let {
    connection,
    locale,
  }: {
    connection: ConnectionStatus;
    locale: Locale;
  } = $props();

  const isTauri = (): boolean => '__TAURI_INTERNALS__' in window;

  const connectionKeys: Record<ConnectionStatus, TranslationKey> = {
    connected: 'connection.connected',
    connecting: 'connection.connecting',
    disconnected: 'connection.disconnected',
  };

  const minimize = async (): Promise<void> => {
    if (isTauri()) await getCurrentWindow().minimize();
  };

  const toggleMaximize = async (): Promise<void> => {
    if (isTauri()) await getCurrentWindow().toggleMaximize();
  };

  const close = async (): Promise<void> => {
    if (isTauri()) await getCurrentWindow().close();
  };
</script>

<header class="window-header" data-tauri-drag-region>
  <div class="connection" data-tauri-drag-region role="status">
    <span aria-hidden="true" class="connection-dot {connection}"></span>
    <span>{translate(locale, connectionKeys[connection])}</span>
  </div>

  <div class="window-controls">
    <button aria-label={translate(locale, 'action.minimize')} onclick={minimize} type="button">
      <Icon name="minimize" size={17} />
    </button>
    <button
      aria-label={translate(locale, 'action.maximize')}
      onclick={toggleMaximize}
      type="button"
    >
      <Icon name="maximize" size={15} />
    </button>
    <button aria-label={translate(locale, 'action.close')} onclick={close} type="button">
      <Icon name="close" size={17} />
    </button>
  </div>
</header>
