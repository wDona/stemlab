<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { OpenSheetMusicDisplay } from "opensheetmusicdisplay";
  import { untrack, type Snippet } from "svelte";
  import { STEM_INFO, type Song, type Stem } from "./engine";
  import { setWindowFullscreen } from "./fullscreen";
  import type { Player } from "./player";

  let { song, stem, player, controls, busy, onredo, onclose }: {
    song: Song;
    stem: Stem;
    player: Player;
    controls: Snippet; // transporte, para la pantalla completa
    busy: boolean; // ya hay una transcripción de esta pista en cola
    onredo: () => void;
    onclose: () => void;
  } = $props();

  let container: HTMLDivElement;
  let paper: HTMLDivElement;
  let osmd: OpenSheetMusicDisplay | undefined;
  let xml = "";
  let ready = $state(false);
  let error = $state("");
  let zoom = $state(0.8);
  let fullscreen = $state(false);
  let follow = $state(true);
  let exporting = $state(false);

  const info = $derived(STEM_INFO[stem.name]);
  const bpm = $derived(song.bpm ?? 120);

  // notas ya pintadas, para despintarlas al saltar hacia atrás o redibujar
  let painted: SVGElement[] = [];
  let lastBeat = -1;

  function paint(el: SVGElement, color: string) {
    el.style.fill = color;
    el.style.stroke = color;
  }

  /** Redibuja (zoom, tamaño) y deja el cursor al principio. */
  function draw() {
    const o = osmd!;
    o.zoom = zoom;
    o.render();
    o.cursor.reset();
    o.cursor.show();
    painted = [];
    lastBeat = -1;
  }

  $effect(() => {
    // solo al montar: el padre lo recrea con {#key} si cambia la partitura; un `song` nuevo
    // (porque acabó otra transcripción) no debe crear un segundo OSMD en el mismo contenedor
    const path = untrack(() => `${song.dir}/${stem.score}`);
    osmd = new OpenSheetMusicDisplay(container, {
      autoResize: false, // redibujamos nosotros: así el cursor no se pierde
      backend: "svg",
      drawTitle: false,
      drawComposer: false,
      drawPartNames: false,
      drawingParameters: "compacttight",
      followCursor: false,
      cursorsOptions: [{ type: 1, color: "#8b5cff", alpha: 1, follow: false }],
    });
    invoke<ArrayBuffer>("read_file", { path })
      .then((buf) => osmd!.load((xml = new TextDecoder().decode(buf))))
      .then(() => {
        draw();
        ready = true;
      })
      .catch((e) => (error = String(e)));
  });

  // redibujar al cambiar de tamaño (pantalla completa, ventana)
  $effect(() => {
    if (!ready) return;
    let w = paper.clientWidth;
    const ro = new ResizeObserver(() => {
      if (Math.abs(paper.clientWidth - w) < 20) return;
      w = paper.clientWidth;
      draw();
    });
    ro.observe(paper);
    return () => ro.disconnect();
  });

  // seguir la reproducción: la partitura se generó con el mismo tempo, así que segundo -> pulso es directo
  $effect(() => {
    if (!ready) return;
    let raf = 0;
    const tick = () => {
      const o = osmd!;
      const beat = (player.time() * bpm) / 240 + 1e-6; // en redondas, como los timestamps de OSMD
      if (beat < lastBeat) {
        for (const el of painted) el.style.fill = el.style.stroke = "";
        painted = [];
        o.cursor.reset();
      }
      let moved = false;
      while (!o.cursor.Iterator.EndReached && o.cursor.Iterator.currentTimeStamp.RealValue <= beat) {
        for (const gn of o.cursor.GNotesUnderCursor()) {
          const g = (gn as unknown as { getSVGGElement?: () => SVGGElement }).getSVGGElement?.();
          for (const el of g?.querySelectorAll<SVGElement>("path, rect") ?? []) {
            paint(el, info.color);
            painted.push(el);
          }
        }
        o.cursor.next();
        moved = true;
      }
      lastBeat = beat;
      if (moved && follow) scrollToCursor();
      raf = requestAnimationFrame(tick);
    };
    tick();
    return () => cancelAnimationFrame(raf);
  });

  /** Desplaza solo el papel (no la página) para que el cursor quede a la vista. */
  function scrollToCursor() {
    const c = osmd?.cursor.cursorElement;
    if (!c) return;
    const top = c.getBoundingClientRect().top - paper.getBoundingClientRect().top + paper.scrollTop;
    if (top < paper.scrollTop + 40 || top + c.clientHeight > paper.scrollTop + paper.clientHeight - 40) {
      paper.scrollTo({ top: top - paper.clientHeight / 3, behavior: "smooth" });
    }
  }

  function setZoom(z: number) {
    zoom = Math.round(Math.min(2, Math.max(0.4, z)) * 10) / 10;
    if (ready) draw();
  }

  function setFullscreen(on: boolean) {
    fullscreen = on;
    setWindowFullscreen(on);
  }
  $effect(() => () => {
    if (fullscreen) setWindowFullscreen(false);
  });

  /** A4 vectorial: OSMD pagina en SVG y svg2pdf pasa cada página a jsPDF. */
  async function exportPdf() {
    exporting = true;
    error = "";
    const div = document.createElement("div");
    div.style.cssText = "position:fixed;left:-10000px;top:0;width:794px"; // ancho A4 a 96 dpi
    document.body.append(div);
    try {
      const [{ jsPDF }] = await Promise.all([import("jspdf"), import("svg2pdf.js")]);
      const o = new OpenSheetMusicDisplay(div, {
        backend: "svg",
        autoResize: false,
        pageFormat: "A4_P",
        pageBackgroundColor: "#FFFFFF",
        drawTitle: true,
        drawComposer: true,
        drawPartNames: false,
      });
      await o.load(xml);
      o.render();
      const pdf = new jsPDF({ unit: "pt", format: "a4" });
      const w = pdf.internal.pageSize.getWidth();
      const h = pdf.internal.pageSize.getHeight();
      const pages = div.querySelectorAll("svg");
      for (let i = 0; i < pages.length; i++) {
        if (i) pdf.addPage();
        await pdf.svg(pages[i], { x: 0, y: 0, width: w, height: h });
      }
      const out = `${song.dir}/scores/${stem.name}.pdf`;
      await invoke("write_file", new Uint8Array(pdf.output("arraybuffer")), {
        headers: { path: encodeURIComponent(out) },
      });
      await openPath(out);
    } catch (e) {
      error = `No se pudo exportar: ${e}`;
    } finally {
      div.remove();
      exporting = false;
    }
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && fullscreen && setFullscreen(false)} />

<section class="score" class:fs={fullscreen} style:--c={info.color}>
  <header>
    <div class="title">
      <span class="dot"></span>
      <strong>Partitura · {info.label}</strong>
      <small>{stem.notes} notas · {bpm} BPM</small>
    </div>
    <div class="tools">
      <label title="Desplazar la partitura para seguir la reproducción">
        <input type="checkbox" bind:checked={follow} /> Seguir
      </label>
      <button class="ghost" onclick={() => setZoom(zoom - 0.1)} aria-label="Alejar">−</button>
      <span class="zoom">{Math.round(zoom * 100)}%</span>
      <button class="ghost" onclick={() => setZoom(zoom + 0.1)} aria-label="Acercar">+</button>
      <button class="ghost" onclick={exportPdf} disabled={!ready || exporting}>{exporting ? "Exportando…" : "PDF"}</button>
      <button class="ghost" onclick={() => openPath(`${song.dir}/scores`)} title="MIDI, MusicXML y PDF">Carpeta</button>
      <button class="ghost" onclick={onredo} disabled={busy} title="Volver a sacar notas{stem.name === 'vocals' ? ' y letra' : ''}">
        {busy ? "En cola…" : "↻ Rehacer"}
      </button>
      <button class="ghost" onclick={() => setFullscreen(!fullscreen)} title={fullscreen ? "Salir (Esc)" : "Pantalla completa"}>
        {fullscreen ? "Salir" : "⛶"}
      </button>
      {#if !fullscreen}<button class="ghost" onclick={onclose} aria-label="Cerrar">×</button>{/if}
    </div>
  </header>
  {#if error}<p class="error">{error}</p>{/if}
  {#if !ready && !error}<div class="loading">Dibujando partitura…</div>{/if}
  <!-- nunca display:none mientras dibuja: OSMD mide el ancho del contenedor y con 0 no pinta nada -->
  <div class="paper" bind:this={paper}>
    <div bind:this={container}></div>
  </div>
  {#if fullscreen}<footer>{@render controls()}</footer>{/if}
</section>

<style>
  .score { margin-top: 18px; background: var(--panel); border: 1px solid var(--line); border-radius: 14px; overflow: hidden; }
  header { display: flex; justify-content: space-between; align-items: center; gap: 12px; padding: 10px 14px; border-bottom: 1px solid var(--line); flex-wrap: wrap; }
  .title { display: flex; align-items: center; gap: 10px; }
  .title small { color: var(--text-3); }
  .dot { width: 10px; height: 10px; border-radius: 50%; background: var(--c); box-shadow: 0 0 10px var(--c); }
  .tools { display: flex; align-items: center; gap: 6px; }
  .tools button { padding: 6px 12px; }
  .tools label { display: flex; align-items: center; gap: 5px; color: var(--text-2); font-size: 12px; margin-right: 6px; }
  .zoom { font-size: 12px; color: var(--text-2); min-width: 36px; text-align: center; font-variant-numeric: tabular-nums; }
  .paper { background: #fbfaf7; max-height: 60vh; overflow: auto; padding: 8px 0; position: relative; }
  .loading { padding: 24px; color: var(--text-2); }
  .error { color: #ff8a8a; padding: 0 14px; }

  .fs { position: fixed; inset: 0; z-index: 50; margin: 0; border: 0; border-radius: 0; display: flex; flex-direction: column; }
  .fs .paper { flex: 1; max-height: none; min-height: 0; }
  footer { padding: 12px 16px; border-top: 1px solid var(--line); }
</style>
