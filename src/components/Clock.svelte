<script lang="ts">
  // Actividad por defecto mientras no hay providers (media, batería, etc.).
  let now = $state(new Date());

  $effect(() => {
    const id = setInterval(() => (now = new Date()), 1000);
    return () => clearInterval(id);
  });

  // `undefined` = el idioma y formato de hora del sistema.
  const time = new Intl.DateTimeFormat(undefined, { hour: "2-digit", minute: "2-digit" });
  const date = new Intl.DateTimeFormat(undefined, {
    weekday: "long",
    day: "numeric",
    month: "long",
  });
</script>

<div class="clock">
  <span class="time">{time.format(now)}</span>
  <span class="date">{date.format(now)}</span>
</div>

<style>
  .clock {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }

  .time {
    font-size: 44px;
    font-weight: 600;
    letter-spacing: -0.02em;
    font-variant-numeric: tabular-nums;
  }

  .date {
    font-size: 14px;
    color: rgb(255 255 255 / 0.6);
  }

  .date::first-letter {
    text-transform: uppercase;
  }
</style>
