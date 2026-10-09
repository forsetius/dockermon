<script lang="ts">
  let { size = 48 }: { size?: number } = $props();
</script>

<svg aria-hidden="true" class="dockermon-mark" height={size} viewBox="0 0 64 64" width={size}>
  <path d="M10 29 22 22l12 7v14L22 50l-12-7Z" fill="#1677ee" />
  <path d="m30 18 12-7 12 7v14l-12 7-12-7Z" fill="#2589ff" />
  <path d="m30 43 12-7 12 7v14l-12 7-12-7Z" fill="#1168d5" />
  <circle cx="50" cy="8" fill="#22c55e" r="6" stroke="var(--surface)" stroke-width="3" />
</svg>
