<script lang="ts">
  import BatteryCharging from "@lucide/svelte/icons/battery-charging";
  import BatteryFull from "@lucide/svelte/icons/battery-full";
  import BatteryLow from "@lucide/svelte/icons/battery-low";
  import BatteryMedium from "@lucide/svelte/icons/battery-medium";

  interface Props {
    percent: number;
    charging: boolean;
    low: boolean;
  }

  let { percent, charging, low }: Props = $props();

  // Texto + ícono + color: el estado nunca depende solo del color.
  const status = $derived(charging ? "Cargando" : low ? "Batería baja" : "Con batería");
  const tone = $derived(charging ? "success" : low ? "danger" : "neutral");
  const Icon = $derived(
    charging ? BatteryCharging : low ? BatteryLow : percent >= 80 ? BatteryFull : BatteryMedium,
  );
</script>

<div class="notice {tone}" role="status" aria-label="{status}, {percent}%">
  <span class="status">{status}</span>
  <span class="level">
    <span class="value">{percent}%</span>
    <Icon size={22} strokeWidth={2} aria-hidden="true" />
  </span>
</div>

<style>
  .notice {
    width: 100%;
    height: 100%;
    padding: 0 14px 0 18px;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 13px;
    font-weight: 600;
  }

  .level {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .value {
    font-variant-numeric: tabular-nums;
  }

  .success {
    color: var(--color-success);
  }

  .danger {
    color: var(--color-danger);
  }

  .neutral .status {
    color: var(--text-secondary);
  }
</style>
