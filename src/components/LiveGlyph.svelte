<script lang="ts">
  import Activity from "@lucide/svelte/icons/activity";
  import Download from "@lucide/svelte/icons/download";
  import FlaskConical from "@lucide/svelte/icons/flask-conical";
  import Hammer from "@lucide/svelte/icons/hammer";
  import Rocket from "@lucide/svelte/icons/rocket";
  import type { LiveIcon } from "../lib/activity.svelte";
  import ClaudeMark from "./ClaudeMark.svelte";

  let { icon, size, color }: { icon: LiveIcon; size: number; color: string } = $props();

  // Catálogo cerrado (igual que en Rust): nada de URLs ni SVG externos en la isla.
  const ICONS = {
    build: Hammer,
    deploy: Rocket,
    download: Download,
    test: FlaskConical,
    generic: Activity,
  };
</script>

{#if icon === "claude"}
  <!-- Va siempre en su naranja de marca: el estado lo dicen el ícono de la derecha y el texto. -->
  <ClaudeMark height={size * 0.8} />
{:else}
  {@const Icon = ICONS[icon]}
  <Icon {size} {color} strokeWidth={2.25} aria-hidden="true" />
{/if}
