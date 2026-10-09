<script lang="ts">
  import { tick } from 'svelte';
  import type { Locale } from '../domain';
  import { translate } from '../i18n';
  import Icon from './Icon.svelte';

  let {
    disabled = false,
    locale,
    onremove,
    projectName,
  }: {
    disabled?: boolean;
    locale: Locale;
    onremove: () => void;
    projectName: string;
  } = $props();

  let containerElement = $state<HTMLDivElement>();
  let menuItemElement = $state<HTMLButtonElement>();
  let menuOpen = $state(false);
  let triggerElement = $state<HTMLButtonElement>();

  async function toggleMenu(): Promise<void> {
    menuOpen = !menuOpen;
    if (menuOpen) {
      await tick();
      menuItemElement?.focus();
    }
  }

  function closeMenu(restoreFocus = false): void {
    menuOpen = false;
    if (restoreFocus) triggerElement?.focus();
  }

  function removeProject(): void {
    closeMenu();
    onremove();
  }

  function handleWindowPointerDown(event: PointerEvent): void {
    if (menuOpen && !containerElement?.contains(event.target as Node)) closeMenu();
  }

  function handleWindowKeyDown(event: KeyboardEvent): void {
    if (menuOpen && event.key === 'Escape') {
      event.preventDefault();
      closeMenu(true);
    }
  }

  function handleFocusOut(event: FocusEvent): void {
    const nextTarget = event.relatedTarget;
    if (menuOpen && nextTarget instanceof Node && !containerElement?.contains(nextTarget)) {
      closeMenu();
    }
  }
</script>

<svelte:window onkeydown={handleWindowKeyDown} onpointerdown={handleWindowPointerDown} />

<div bind:this={containerElement} class="project-actions-menu" onfocusout={handleFocusOut}>
  <button
    aria-expanded={menuOpen}
    aria-haspopup="menu"
    aria-label={translate(locale, 'projects.moreActions', { project: projectName })}
    bind:this={triggerElement}
    class="icon-button"
    {disabled}
    onclick={toggleMenu}
    title={translate(locale, 'projects.moreActions', { project: projectName })}
    type="button"
  >
    <Icon name="more-vertical" size={18} />
  </button>

  {#if menuOpen}
    <div class="project-actions-popover" role="menu">
      <button
        bind:this={menuItemElement}
        class="project-actions-item"
        onclick={removeProject}
        role="menuitem"
        type="button"
      >
        <Icon name="trash" size={17} />
        <span>{translate(locale, 'projects.removeAction')}</span>
      </button>
    </div>
  {/if}
</div>
