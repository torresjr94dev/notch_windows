<script lang="ts">
  import Activity from "@lucide/svelte/icons/activity";
  import Download from "@lucide/svelte/icons/download";
  import FlaskConical from "@lucide/svelte/icons/flask-conical";
  import Hammer from "@lucide/svelte/icons/hammer";
  import Rocket from "@lucide/svelte/icons/rocket";
  import Sparkles from "@lucide/svelte/icons/sparkles";
  import type { LiveIcon } from "../lib/activity.svelte";

  let { icon, size, color }: { icon: LiveIcon; size: number; color: string } = $props();

  // Catálogo cerrado (igual que en Rust): nada de URLs ni SVG externos en la isla.
  const ICONS = {
    claude: Sparkles,
    build: Hammer,
    deploy: Rocket,
    download: Download,
    test: FlaskConical,
    generic: Activity,
  };
  const Icon = $derived(ICONS[icon]);
</script>

<Icon {size} {color} strokeWidth={2.25} aria-hidden="true" />
