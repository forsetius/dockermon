<script lang="ts">
  type IconName =
    | 'activity'
    | 'chevron'
    | 'close'
    | 'containers'
    | 'folder'
    | 'folder-plus'
    | 'files'
    | 'language'
    | 'logs'
    | 'maximize'
    | 'minimize'
    | 'more-vertical'
    | 'play'
    | 'projects'
    | 'restart'
    | 'settings'
    | 'stop'
    | 'trash';

  let { name, size = 18 }: { name: IconName; size?: number } = $props();
</script>

<svg aria-hidden="true" class="icon" fill="none" height={size} viewBox="0 0 24 24" width={size}>
  {#if name === 'projects' || name === 'folder' || name === 'folder-plus'}
    <path
      d="M3.5 6.75h6l2 2h9v9.75a1.75 1.75 0 0 1-1.75 1.75H5.25A1.75 1.75 0 0 1 3.5 18.5V6.75Z"
    />
    <path d="M3.5 8.75V5.5c0-.97.78-1.75 1.75-1.75h3.62l2 2h4.38" />
    {#if name === 'folder-plus'}
      <path d="M16.5 12.5v5M14 15h5" />
    {/if}
  {:else if name === 'files'}
    <path d="M7 3.5h7l3 3v13H7v-16Z" />
    <path d="M14 3.5v3h3M4 7.5v13h9" />
  {:else if name === 'containers'}
    <path d="m12 2.75 8 4.5v9.5l-8 4.5-8-4.5v-9.5l8-4.5Z" />
    <path d="m4.3 7.42 7.7 4.33 7.7-4.33M12 11.75v9.5" />
  {:else if name === 'activity'}
    <circle cx="12" cy="12" r="9" />
    <path d="M12 7v5l3.5 2" />
  {:else if name === 'settings'}
    <circle cx="12" cy="12" r="3" />
    <path
      d="M19.1 13.4a7 7 0 0 0 .05-2.9l2-1.55-2-3.46-2.46 1a7.2 7.2 0 0 0-2.5-1.44L13.85 2h-4l-.35 3.05A7.2 7.2 0 0 0 7 6.5l-2.45-1-2 3.45 2 1.55a7 7 0 0 0 .05 2.9l-2.05 1.6 2 3.45 2.55-1.04a7.1 7.1 0 0 0 2.4 1.38L9.85 22h4l.36-3.2a7.1 7.1 0 0 0 2.4-1.39l2.54 1.04 2-3.45-2.05-1.6Z"
    />
  {:else if name === 'chevron'}
    <path d="m8.5 10 3.5 3.5 3.5-3.5" />
  {:else if name === 'play'}
    <path d="m9 6 8 6-8 6V6Z" fill="currentColor" stroke="none" />
  {:else if name === 'stop'}
    <rect fill="currentColor" height="10" rx="1" stroke="none" width="10" x="7" y="7" />
  {:else if name === 'restart'}
    <path d="M19.2 9A8 8 0 1 0 20 14" />
    <path d="M19.2 4.5V9h-4.5" />
  {:else if name === 'more-vertical'}
    <circle cx="12" cy="5" fill="currentColor" r="1.45" stroke="none" />
    <circle cx="12" cy="12" fill="currentColor" r="1.45" stroke="none" />
    <circle cx="12" cy="19" fill="currentColor" r="1.45" stroke="none" />
  {:else if name === 'logs'}
    <path d="M6 3.5h9l3 3v14H6v-17Z" />
    <path d="M15 3.5v3h3M9 11h6M9 15h6" />
  {:else if name === 'trash'}
    <path d="M4.5 7h15M9 7V4.5h6V7M7 7l.75 13h8.5L17 7M10 10v6M14 10v6" />
  {:else if name === 'close'}
    <path d="m6 6 12 12M18 6 6 18" />
  {:else if name === 'minimize'}
    <path d="M7 12h10" />
  {:else if name === 'maximize'}
    <rect height="10" rx="1" width="10" x="7" y="7" />
  {:else if name === 'language'}
    <circle cx="12" cy="12" r="9" />
    <path
      d="M3.5 12h17M12 3c2.2 2.45 3.35 5.45 3.35 9S14.2 18.55 12 21c-2.2-2.45-3.35-5.45-3.35-9S9.8 5.45 12 3Z"
    />
  {/if}
</svg>

<style>
  .icon {
    display: block;
    flex: none;
    stroke: currentColor;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-width: 1.8;
  }
</style>
