<script lang="ts">
  let {
    checked,
    disabled = false,
    label,
    onchange,
  }: {
    checked: boolean;
    disabled?: boolean;
    label: string;
    onchange: (checked: boolean) => void;
  } = $props();
</script>

<label class="toggle" class:disabled>
  <input
    aria-label={label}
    {checked}
    {disabled}
    onchange={(event) => onchange(event.currentTarget.checked)}
    type="checkbox"
  />
  <span aria-hidden="true" class="track"><span class="thumb"></span></span>
</label>

<style>
  .toggle {
    display: inline-flex;
    cursor: pointer;
  }

  input {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    clip-path: inset(50%);
    white-space: nowrap;
  }

  .track {
    display: flex;
    width: 42px;
    height: 24px;
    align-items: center;
    padding: 3px;
    border: 1px solid var(--control-border);
    border-radius: 999px;
    background: var(--control-muted);
    transition:
      background 180ms ease-out,
      border-color 180ms ease-out;
  }

  .thumb {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 2px 5px rgb(0 0 0 / 22%);
    transition: transform 180ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  input:checked + .track {
    border-color: var(--accent);
    background: var(--accent);
  }

  input:checked + .track .thumb {
    transform: translateX(18px);
  }

  input:focus-visible + .track {
    outline: 3px solid var(--focus-ring);
    outline-offset: 3px;
  }

  .disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }
</style>
