<script lang="ts">
  import { onMount, tick } from 'svelte';
  import type { Locale } from '../domain';
  import { translate } from '../i18n';
  import Icon from './Icon.svelte';
  import Toggle from './Toggle.svelte';

  let {
    dock,
    follow,
    height,
    lines,
    loading,
    locale,
    onclear,
    onclose,
    onfollow,
    onheightchange,
    onwidthchange,
    serviceName,
    sidebarCollapsed,
    width,
  }: {
    dock: 'bottom' | 'right';
    follow: boolean;
    height: number;
    lines: string[];
    loading: boolean;
    locale: Locale;
    onclear: () => void;
    onclose: () => void;
    onfollow: (follow: boolean) => void;
    onheightchange: (height: number) => void;
    onwidthchange: (width: number) => void;
    serviceName: string;
    sidebarCollapsed: boolean;
    width: number;
  } = $props();

  const skeletonLines = [0, 1, 2, 3, 4, 5, 6];
  const defaultWidth = 560;
  const defaultHeight = 380;
  const minimumWidth = 360;
  const minimumHeight = 240;
  const maximumAbsoluteWidth = 1100;
  const maximumAbsoluteHeight = 720;
  const minimumWorkspaceWidth = 600;
  const minimumWorkspaceHeight = 280;
  const windowHeaderHeight = 62;
  const mobileBreakpoint = 700;
  const compactSidebarBreakpoint = 980;
  const compactSidebarWidth = 74;
  const desktopSidebarWidth = 210;
  const keyboardResizeStep = 24;
  let closeButton = $state<HTMLButtonElement>();
  let logViewport = $state<HTMLTextAreaElement>();
  let maximumSize = $state(calculateMaximumSize());
  let resizing = $state(false);
  const currentSize = $derived(dock === 'right' ? width : height);
  const minimumSize = $derived(dock === 'right' ? minimumWidth : minimumHeight);

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
    updateMaximumSize();
    closeButton?.focus();
    return () => {
      document.body.classList.remove('drawer-resizing-height', 'drawer-resizing-width');
      previousFocus?.focus();
    };
  });

  function calculateMaximumSize(): number {
    if (dock === 'bottom') {
      return Math.max(
        minimumHeight,
        Math.min(
          maximumAbsoluteHeight,
          window.innerHeight - windowHeaderHeight - minimumWorkspaceHeight,
        ),
      );
    }
    if (window.innerWidth <= mobileBreakpoint) return maximumAbsoluteWidth;
    const sidebarWidth =
      sidebarCollapsed || window.innerWidth <= compactSidebarBreakpoint
        ? compactSidebarWidth
        : desktopSidebarWidth;
    return Math.max(
      minimumWidth,
      Math.min(maximumAbsoluteWidth, window.innerWidth - sidebarWidth - minimumWorkspaceWidth),
    );
  }

  function updateMaximumSize(): void {
    if (window.innerWidth <= mobileBreakpoint) return;
    maximumSize = calculateMaximumSize();
    if (currentSize > maximumSize) setSize(maximumSize);
  }

  $effect(updateMaximumSize);

  function clampSize(nextSize: number): number {
    return Math.min(maximumSize, Math.max(minimumSize, Math.round(nextSize)));
  }

  function setSize(nextSize: number): void {
    if (dock === 'right') onwidthchange(nextSize);
    else onheightchange(nextSize);
  }

  function resizeFromPointer(event: PointerEvent): void {
    const nextSize =
      dock === 'right' ? window.innerWidth - event.clientX : window.innerHeight - event.clientY;
    setSize(clampSize(nextSize));
  }

  function startResize(event: PointerEvent): void {
    if (event.button !== 0 || window.innerWidth <= mobileBreakpoint) return;
    event.preventDefault();
    resizing = true;
    if (event.currentTarget instanceof HTMLElement) {
      event.currentTarget.setPointerCapture?.(event.pointerId);
    }
    document.body.classList.add(
      dock === 'right' ? 'drawer-resizing-width' : 'drawer-resizing-height',
    );
    resizeFromPointer(event);
  }

  function continueResize(event: PointerEvent): void {
    if (resizing) resizeFromPointer(event);
  }

  function finishResize(event: PointerEvent): void {
    if (!resizing) return;
    resizing = false;
    if (event.currentTarget instanceof HTMLElement) {
      event.currentTarget.releasePointerCapture?.(event.pointerId);
    }
    document.body.classList.remove('drawer-resizing-height', 'drawer-resizing-width');
  }

  function resizeWithKeyboard(event: KeyboardEvent): void {
    const resizeStep = event.shiftKey ? keyboardResizeStep * 2 : keyboardResizeStep;
    const nextSize =
      event.key === 'Home'
        ? minimumSize
        : event.key === 'End'
          ? maximumSize
          : dock === 'right' && event.key === 'ArrowLeft'
            ? currentSize + resizeStep
            : dock === 'right' && event.key === 'ArrowRight'
              ? currentSize - resizeStep
              : dock === 'bottom' && event.key === 'ArrowUp'
                ? currentSize + resizeStep
                : dock === 'bottom' && event.key === 'ArrowDown'
                  ? currentSize - resizeStep
                  : null;
    if (nextSize === null) return;
    event.preventDefault();
    setSize(clampSize(nextSize));
  }
</script>

<svelte:window
  onkeydown={(event) => event.key === 'Escape' && onclose()}
  onresize={updateMaximumSize}
/>

<button
  aria-label={translate(locale, 'action.close')}
  class="drawer-backdrop"
  onclick={onclose}
  type="button"
></button>

<aside
  aria-busy={loading}
  aria-label={translate(locale, 'drawer.title', { service: serviceName })}
  class="log-drawer drawer-{dock}"
>
  <button
    aria-label={translate(locale, 'drawer.resize')}
    aria-orientation={dock === 'right' ? 'horizontal' : 'vertical'}
    aria-valuemax={maximumSize}
    aria-valuemin={minimumSize}
    aria-valuenow={currentSize}
    aria-valuetext={`${currentSize} px`}
    class:active={resizing}
    class="drawer-resize-handle drawer-resize-handle-{dock}"
    onkeydown={resizeWithKeyboard}
    onpointercancel={finishResize}
    onpointerdown={startResize}
    onpointermove={continueResize}
    onpointerup={finishResize}
    ondblclick={() => setSize(clampSize(dock === 'right' ? defaultWidth : defaultHeight))}
    role="slider"
    tabindex="0"
    title={translate(locale, 'drawer.resizeHint')}
    type="button"
  >
    <span aria-hidden="true"></span>
  </button>
  <header class="drawer-header">
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
