<script lang="ts">
  let { provider, onswitch } = $props<{
    provider: string;
    onswitch: (p: string) => void;
  }>();

  const providers = ['Ollama', 'DeepSeek', 'Gemini'];

  function select(p: string) {
    onswitch(p);
  }
</script>

<div class="provider-controls">
  <div class="provider-row">
    <span class="provider-name">Active Provider</span>
    <div class="provider-toggle">
      {#each providers as p}
        <button
          class="provider-btn"
          class:active={provider === p}
          onclick={() => select(p)}
        >{p}</button>
      {/each}
    </div>
  </div>
</div>
