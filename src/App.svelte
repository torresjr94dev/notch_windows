<script lang="ts">
  import { hitArea } from "./lib/hitArea";
</script>

<main class="island" {@attach hitArea}>Notch Windows</main>

<style>
  .island {
    margin: 8px auto 0;
    width: 180px;
    height: 34px;
    border-radius: 17px;
    background: #000;
    display: grid;
    place-items: center;
    font-size: 12px;
  }
</style>
