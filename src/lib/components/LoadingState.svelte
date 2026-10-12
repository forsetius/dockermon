<script lang="ts">
  import type { Locale } from '../domain';
  import { translate } from '../i18n';

  let { locale }: { locale: Locale } = $props();
  const projectSkeletons = [0, 1];
  const rowSkeletons = [0, 1, 2, 3];
</script>

<section aria-busy="true" aria-label={translate(locale, 'loading.title')} class="loading-state">
  <div class="loading-heading"></div>
  {#each projectSkeletons as projectSkeleton (projectSkeleton)}
    <div class="loading-project">
      <div class="loading-project-header"></div>
      {#each rowSkeletons as rowSkeleton (rowSkeleton)}
        <div class="loading-row"></div>
      {/each}
    </div>
  {/each}
</section>
