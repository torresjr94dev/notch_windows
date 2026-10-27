<script lang="ts">
  import ArrowBigUp from "@lucide/svelte/icons/arrow-big-up";

  let { on }: { on: boolean } = $props();
</script>

<div class="notice" role="status" aria-label="Bloq Mayús {on ? 'activado' : 'desactivado'}">
  <span class="name">
    <ArrowBigUp
      size={18}
      strokeWidth={2.25}
      fill={on ? "currentColor" : "none"}
      aria-hidden="true"
    />
    Bloq Mayús
  </span>
  <span class="state" class:on>{on ? "Activado" : "Desactivado"}</span>
</div>

<style>
  .notice {
    width: 100%;
    height: 100%;
    padding: 0 18px 0 14px;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 13px;
    font-weight: 600;
  }

  .name {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .state {
    color: var(--text-secondary);
  }

  .state.on {
    color: var(--color-success);
  }
</style>
