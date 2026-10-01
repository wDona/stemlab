<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { Player } from "./player";

  type Note = [start: number, end: number, pitch: number, velocity: number];
  type Track = { stem: string; name: string; color: string; path: string; hidden: boolean };

  let { player, tracks, ontoggle, onclose }: {
    player: Player;
    tracks: Track[];
    ontoggle: (stem: string) => void;
    onclose: () => void;
  } = $props();

  const WINDOW = 2.5; // segundos visibles por encima del teclado
  const KEYS_H = 90;
  const HEIGHT = 380;
  const BLACK = new Set([1, 3, 6, 8, 10]);
  const isBlack = (p: number) => BLACK.has(p % 12);

  let canvas: HTMLCanvasElement;
  let width = $state(0);
  let notes = $state.raw<Record<string, Note[]>>({}); // raw: sin proxy en miles de notas leídas cada frame

  // notas de cada pista (cacheadas por ruta)
  $effect(() => {
    for (const t of tracks) {
      if (notes[t.path]) continue;
      invoke<ArrayBuffer>("read_file", { path: t.path }).then((buf) => {
        notes = { ...notes, [t.path]: JSON.parse(new TextDecoder().decode(buf)) };
      });
    }
  });

  // teclado: de la nota más grave a la más aguda de todas las pistas, en octavas completas
  const range = $derived.by(() => {
    const all = Object.values(notes).flat();
    if (!all.length) return { lo: 48, hi: 83 };
    let lo = Math.min(...all.map((n) => n[2]));
    let hi = Math.max(...all.map((n) => n[2]));
    lo -= lo % 12;
    hi += 11 - (hi % 12);
    if (hi - lo < 35) hi = lo + 35; // mínimo 3 octavas
    return { lo, hi };
  });

  /** x y ancho de cada nota midi en el teclado. */
  const keys = $derived.by(() => {
    const { lo, hi } = range;
    const whites = [];
    for (let p = lo; p <= hi; p++) if (!isBlack(p)) whites.push(p);
    const ww = width / whites.length;
    const map = new Map<number, { x: number; w: number; black: boolean }>();
    let i = 0;
    for (let p = lo; p <= hi; p++) {
      if (isBlack(p)) map.set(p, { x: i * ww - ww * 0.3, w: ww * 0.6, black: true });
      else map.set(p, { x: i++ * ww, w: ww, black: false });
    }
    return map;
  });

  $effect(() => {
    if (!width) return;
    const dpr = devicePixelRatio;
    canvas.width = width * dpr;
    canvas.height = HEIGHT * dpr;
    const g = canvas.getContext("2d")!;
    let raf = 0;

    const draw = () => {
      g.setTransform(dpr, 0, 0, dpr, 0, 0);
      const t = player.time();
      const rollH = HEIGHT - KEYS_H;
      const bg = g.createLinearGradient(0, 0, 0, rollH);
      bg.addColorStop(0, "#0b0c10");
      bg.addColorStop(1, "#161822");
      g.fillStyle = bg;
      g.fillRect(0, 0, width, rollH);

      // guías en cada Do
      g.fillStyle = "#ffffff0d";
      for (const [p, k] of keys) if (p % 12 === 0) g.fillRect(k.x, 0, 1, rollH);

      const active = new Map<number, string>();
      for (const track of tracks) {
        if (track.hidden) continue;
        // ponytail: recorre todas las notas cada frame; con decenas de miles, búsqueda binaria por inicio
        for (const [start, end, pitch, vel] of notes[track.path] ?? []) {
          if (end < t || start > t + WINDOW) continue;
          const k = keys.get(pitch);
          if (!k) continue;
          const bottom = rollH * (1 - (start - t) / WINDOW);
          const top = rollH * (1 - (end - t) / WINDOW);
          const on = start <= t && t < end;
          if (on) active.set(pitch, track.color);
          g.globalAlpha = on ? 1 : 0.55 + vel * 0.45;
          g.shadowColor = track.color;
          g.shadowBlur = on ? 18 : 0;
          g.fillStyle = track.color;
          g.beginPath();
          g.roundRect(k.x + 1, top, k.w - 2, Math.max(4, Math.min(bottom, rollH) - top), 4);
          g.fill();
        }
      }
      g.globalAlpha = 1;
      g.shadowBlur = 0;

      // línea de golpeo
      g.fillStyle = "#ffffff55";
      g.fillRect(0, rollH - 2, width, 2);

      // teclado: blancas y luego negras por encima
      for (const black of [false, true]) {
        for (const [p, k] of keys) {
          if (k.black !== black) continue;
          const lit = active.get(p);
          const h = black ? KEYS_H * 0.62 : KEYS_H;
          g.fillStyle = lit ?? (black ? "#15161c" : "#eceae4");
          g.beginPath();
          g.roundRect(k.x + 0.5, rollH, k.w - 1, h, [0, 0, 4, 4]);
          g.fill();
          if (!black && p % 12 === 0) {
            g.fillStyle = "#00000066";
            g.font = "10px system-ui";
            g.fillText(`C${Math.floor(p / 12) - 1}`, k.x + 4, rollH + KEYS_H - 8);
          }
        }
      }
      raf = requestAnimationFrame(draw);
    };
    draw();
    return () => cancelAnimationFrame(raf);
  });
</script>

<section class="roll">
  <header>
    <div class="legend">
      <strong>Piano roll</strong>
      {#each tracks as t (t.stem)}
        <button class="chip" class:off={t.hidden} onclick={() => ontoggle(t.stem)} title={t.hidden ? "Mostrar" : "Ocultar"}>
          <i style:background={t.color}></i>{t.name}
        </button>
      {/each}
      {#if !tracks.length}<small>Ninguna pista con notas suena ahora (revisa M/S).</small>{/if}
    </div>
    <button class="ghost" onclick={onclose} aria-label="Cerrar">×</button>
  </header>
  <div bind:clientWidth={width}>
    <canvas bind:this={canvas} style:height="{HEIGHT}px"></canvas>
  </div>
</section>

<style>
  .roll { margin-top: 18px; background: var(--panel); border: 1px solid var(--line); border-radius: 14px; overflow: hidden; }
  header { display: flex; justify-content: space-between; align-items: center; padding: 8px 14px; border-bottom: 1px solid var(--line); }
  .legend { display: flex; align-items: center; gap: 14px; flex-wrap: wrap; }
  .chip {
    display: flex; align-items: center; gap: 6px; padding: 3px 9px; border-radius: 99px; cursor: pointer;
    border: 1px solid var(--line); background: transparent; color: var(--text-2); font-size: 12px;
  }
  .chip:hover { color: var(--text); }
  .chip.off { opacity: 0.4; text-decoration: line-through; }
  .legend i { width: 9px; height: 9px; border-radius: 50%; }
  .legend small { color: var(--text-3); }
  header button { padding: 4px 10px; }
  canvas { width: 100%; display: block; }
</style>
