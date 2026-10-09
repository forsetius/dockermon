<script lang="ts">
  import DockermonMark from './lib/DockermonMark.svelte';
</script>

<svelte:head>
  <meta name="color-scheme" content="light dark" />
</svelte:head>

<main class="shell">
  <section class="welcome" aria-labelledby="welcome-title">
    <DockermonMark size={72} />
    <div class="welcome-copy">
      <h1 id="welcome-title">Dockermon jest gotowy</h1>
      <p>
        Powłoka aplikacji działa. Integracja z lokalnym Docker Engine i widok projektów pojawią się
        w kolejnych etapach.
      </p>
    </div>
    <div class="stage-status" role="status">
      <span class="status-dot" aria-hidden="true"></span>
      Etap 0 · szkielet aplikacji
    </div>
  </section>
</main>
