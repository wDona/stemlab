<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { OpenSheetMusicDisplay } from "opensheetmusicdisplay";
  import { STEM_INFO, type Song, type Stem } from "./engine";

  let { song, stem, onclose }: { song: Song; stem: Stem; onclose: () => void } = $props();

  let container: HTMLDivElement;
  let osmd: OpenSheetMusicDisplay | undefined;
  let rendering = $state(true);
  let error = $state("");
  let zoom = $state(0.8);

  $effect(() => {
    const path = `${song.dir}/${stem.score}`;
    rendering = true;
    error = "";
    osmd ??= new OpenSheetMusicDisplay(container, {
      autoResize: true,
      backend: "svg",
      drawTitle: false,
      drawComposer: false,
      drawPartNames: false,
      drawingParameters: "compacttight",
    });
    const o = osmd;
    invoke<ArrayBuffer>("read_file", { path })
      .then((buf) => o.load(new TextDecoder().decode(buf)))
      .then(() => {
        o.zoom = zoom;
        o.render();
      })
      .catch((e) => (error = String(e)))
      .finally(() => (rendering = false));
  });

  function setZoom(z: number) {
    zoom = Math.min(2, Math.max(0.4, z));
    if (osmd && !rendering) {
      osmd.zoom = zoom;
      osmd.render();
    }
  }

  const info = $derived(STEM_INFO[stem.name]);
</script>

<section class="score" style:--c={info.color}>
  <header>
    <div class="title">
      <span class="dot"></span>
      <strong>Partitura · {info.label}</strong>
      <small>{stem.notes} notas{song.bpm ? ` · ${song.bpm} BPM` : ""}</small>
    </div>
    <div class="tools">
      <button class="ghost" onclick={() => setZoom(zoom - 0.1)} aria-label="Alejar">−</button>
      <button class="ghost" onclick={() => setZoom(zoom + 0.1)} aria-label="Acercar">+</button>
      <button class="ghost" onclick={() => openPath(`${song.dir}/scores`)}>MIDI y MusicXML</button>
      <button class="ghost" onclick={onclose} aria-label="Cerrar">×</button>
    </div>
  </header>
  {#if error}<p class="error">{error}</p>{/if}
  {#if rendering}<div class="loading">Dibujando partitura…</div>{/if}
  <!-- nunca display:none mientras dibuja: OSMD mide el ancho del contenedor y con 0 no pinta nada -->
  <div class="paper" class:hidden={!!error}>
    <div bind:this={container}></div>
  </div>
</section>

<style>
  .score { margin-top: 18px; background: var(--panel); border: 1px solid var(--line); border-radius: 14px; overflow: hidden; }
  header { display: flex; justify-content: space-between; align-items: center; gap: 12px; padding: 10px 14px; border-bottom: 1px solid var(--line); }
  .title { display: flex; align-items: center; gap: 10px; }
  .title small { color: var(--text-3); }
  .dot { width: 10px; height: 10px; border-radius: 50%; background: var(--c); box-shadow: 0 0 10px var(--c); }
  .tools { display: flex; gap: 6px; }
  .tools button { padding: 6px 12px; }
  .paper { background: #fbfaf7; max-height: 60vh; overflow: auto; padding: 8px 0; }
  .paper.hidden { display: none; }
  .loading { padding: 24px; color: var(--text-2); }
  .error { color: #ff8a8a; padding: 0 14px; }
</style>
