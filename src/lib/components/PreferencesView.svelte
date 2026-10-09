<script lang="ts">
  import type { Locale, Preferences, ThemePreference } from '../domain';
  import { translate, type TranslationKey } from '../i18n';

  let {
    locale,
    onchangeLanguage,
    onchangeTheme,
    preferences,
  }: {
    locale: Locale;
    onchangeLanguage: (locale: Locale) => void;
    onchangeTheme: (theme: ThemePreference) => void;
    preferences: Preferences;
  } = $props();

  const themes: ThemePreference[] = ['system', 'light', 'dark'];
  const themeKeys: Record<ThemePreference, TranslationKey> = {
    dark: 'theme.dark',
    light: 'theme.light',
    system: 'theme.system',
  };
</script>

<section class="settings-view" aria-labelledby="settings-title">
  <header class="page-heading settings-heading">
    <div>
      <h1 id="settings-title">{translate(locale, 'settings.title')}</h1>
      <p>{translate(locale, 'settings.description')}</p>
    </div>
  </header>

  <div class="settings-list">
    <fieldset>
      <legend>{translate(locale, 'settings.theme')}</legend>
      <div class="segmented-control">
        {#each themes as theme (theme)}
          <button
            aria-pressed={preferences.theme === theme}
            class:active={preferences.theme === theme}
            onclick={() => onchangeTheme(theme)}
            type="button"
          >
            {translate(locale, themeKeys[theme])}
          </button>
        {/each}
      </div>
    </fieldset>

    <label class="setting-row">
      <span>{translate(locale, 'settings.language')}</span>
      <select
        aria-label={translate(locale, 'language.label')}
        onchange={(event) => onchangeLanguage(event.currentTarget.value as Locale)}
        value={preferences.language}
      >
        <option value="pl">{translate(locale, 'language.pl')}</option>
        <option value="en">{translate(locale, 'language.en')}</option>
      </select>
    </label>
  </div>
</section>
