<script lang="ts">
  import { revealItemInDir, openPath } from "@tauri-apps/plugin-opener";
  import { untrack } from "svelte";
  import { engine, log, PITCHED, STEM_INFO, type Song } from "./engine";
  import type { Job } from "./Library.svelte";
  import Roll from "./Roll.svelte";
  import Score from "./Score.svelte";
  import Transport from "./Transport.svelte";
  import { DecodeError, Player } from "./player";
  import Waveform from "./Waveform.svelte";

  let { song, jobFor, ontranscribe }: {
    song: Song;
    jobFor: (stem: string) => Job | undefined; // transcripción en cola o en marcha
    ontranscribe: (stem: string) => void;
  } = $props();

  const player = new Player();
  let loading = $state(true);
  let loadError = $state("");
  let converting = $state(false);
  let playing = $state(false);
  let time = $state(0);
  let duration = $state(0);
  let volume = $state<Record<string, number>>({});
  let muted = $state<Record<string, boolean>>({});
  let solo = $state<Record<string, boolean>>({});
  let scoreOf = $state<string | null>(null);
  let showRoll = $state(false);
  let rollHidden = $state<Record<string, boolean>>({}); // instrumentos quitados del piano roll

  const audible = (name: string) => (Object.values(solo).some(Boolean) ? solo[name] : !muted[name]);
  // en el piano roll caen las notas de lo que suena: si haces solo de la voz, solo la voz
  const rollTracks = $derived(
    song.stems
      .filter((s) => s.roll && audible(s.name))
      .map((s) => ({
        stem: s.name,
        name: STEM_INFO[s.name].label,
        color: STEM_INFO[s.name].color,
        path: `${song.dir}/${s.roll}`,
        hidden: !!rollHidden[s.name],
      })),
  );
  const hasRoll = $derived(song.stems.some((s) => s.roll));
  let openWhenReady = $state<string | null>(null);
  const scoreStem = $derived(song.stems.find((s) => s.name === scoreOf && s.score));

  // al terminar una transcripción pedida desde aquí, se abre su partitura
  $effect(() => {
    if (openWhenReady && song.stems.find((s) => s.name === openWhenReady)?.score) {
      scoreOf = openWhenReady;
      rollHidden[openWhenReady] = false;
      openWhenReady = null;
    }
  });

  function notes(stem: string) {
    if (song.stems.find((s) => s.name === stem)?.score) {
      // ocultar la partitura también quita sus notas del piano roll; verla las devuelve
      const show = scoreOf !== stem;
      scoreOf = show ? stem : null;
      rollHidden[stem] = !show;
    } else {
      openWhenReady = stem;
      ontranscribe(stem);
    }
  }

  $effect(() => {
    // solo al montar (el padre usa {#key song.id}); que llegue una partitura nueva no recarga el audio
    const s = untrack(() => song);
    loading = true;
    loadError = "";
    playing = false;
    time = 0;
    for (const st of s.stems) {
      volume[st.name] = 1;
      muted[st.name] = !st.present;
      solo[st.name] = false;
    }
    load(s);
    return () => player.stop();
  });

  const tracks = (s: Song) => s.stems.map((st) => ({ name: st.name, path: `${s.dir}/${st.file}` }));

  async function load(s: Song) {
    const t0 = performance.now();
    try {
      try {
        await player.load(tracks(s));
      } catch (e) {
        if (!(e instanceof DecodeError)) throw e;
        // formato que el navegador no entiende: el motor lo pasa a WAV y reintentamos una vez
        log(`mixer: ${s.id} ${e.message}, convirtiendo`);
        converting = true;
        const { song: fixed } = await engine<{ song: Song }>(["fix", s.id]);
        converting = false;
        await player.load(tracks(fixed));
      }
      duration = player.duration;
      loading = false;
      log(`mixer: ${s.id} ${player.buffers.size} pistas en ${Math.round(performance.now() - t0)} ms`);
    } catch (e) {
      converting = false;
      loadError = e instanceof Error ? e.message : String(e);
      log(`mixer: ${s.id} ${loadError}`);
    }
  }

  // ganancia efectiva: si hay algún solo, solo suenan esas pistas
  $effect(() => {
    if (loading) return;
    const anySolo = Object.values(solo).some(Boolean);
    for (const st of song.stems) {
      const on = anySolo ? solo[st.name] : !muted[st.name];
      player.setGain(st.name, on ? volume[st.name] : 0);
    }
  });

  $effect(() => {
    let raf = 0;
    const tick = () => {
      time = player.time();
      if (player.playing && time >= duration) {
        player.stop();
        playing = false;
      }
      raf = requestAnimationFrame(tick);
    };
    tick();
    return () => cancelAnimationFrame(raf);
  });

  function toggle() {
    if (player.playing) player.pause();
    else player.play();
    playing = player.playing;
  }

  const seek = (f: number) => player.seek(f * duration);

  function onkey(e: KeyboardEvent) {
    if (e.code === "Space" && !(e.target instanceof HTMLInputElement)) {
      e.preventDefault();
      toggle();
    }
  }
</script>

<svelte:window onkeydown={onkey} />

<header>
  <div>
    <h1>{song.title}</h1>
    <p class="sub">
      {song.stems.filter((s) => s.present).length} de {song.stems.length} pistas con sonido
    </p>
  </div>
  <div class="actions">
    <button class="ghost" onclick={() => openPath(`${song.dir}/stems`)}>Abrir carpeta</button>
    <button class="ghost" onclick={() => revealItemInDir(`${song.dir}/meta.json`)}>Mostrar en gestor</button>
  </div>
</header>

<div class="bar">
  <Transport {playing} {time} {duration} disabled={loading} ontoggle={toggle} onseek={(t) => player.seek(t)}>
    <button
      class="ghost"
      class:active={showRoll}
      disabled={!hasRoll || loading}
      title={hasRoll ? "Notas cayendo sobre un teclado" : "Saca las notas de alguna pista primero"}
      onclick={() => (showRoll = !showRoll)}>🎹 Piano roll</button
    >
  </Transport>
</div>

<!-- los mismos controles, dentro de las vistas a pantalla completa -->
{#snippet controls()}
  <Transport {playing} {time} {duration} disabled={loading} ontoggle={toggle} onseek={(t) => player.seek(t)} />
{/snippet}

{#if loadError}
  <p class="error">No se pudieron cargar las pistas: {loadError}</p>
{:else if loading}
  <div class="loading"><div class="spinner"></div> {converting ? "Convirtiendo a un formato compatible…" : "Cargando pistas…"}</div>
{:else}
  <section class="lanes">
    {#each song.stems as st (st.name)}
      {@const info = STEM_INFO[st.name] ?? { label: st.name, color: "#aaa" }}
      {@const buf = player.buffers.get(st.name)}
      <div class="lane" class:absent={!st.present} style:--c={info.color}>
        <div class="head">
          <span class="dot"></span>
          <div class="name">
            <strong>{info.label}</strong>
            <small>{st.present ? `${st.level} dB` : "no detectado"}</small>
          </div>
          <div class="ms">
            <button class:on={muted[st.name]} onclick={() => (muted[st.name] = !muted[st.name])} title="Silenciar">M</button>
            <button class:on={solo[st.name]} class="solo" onclick={() => (solo[st.name] = !solo[st.name])} title="Solo">S</button>
          </div>
          <input type="range" min="0" max="1.5" step="0.01" bind:value={volume[st.name]} title="Volumen" />
          {#if PITCHED.includes(st.name) && st.present}
            {@const job = jobFor(st.name)}
            <button class="notes" class:open={scoreOf === st.name} disabled={!!job && !job.error} onclick={() => notes(st.name)}>
              {#if job?.error}Reintentar notas
              {:else if job}{job.step ? `${job.label}…` : "En cola…"}
              {:else if st.score}{scoreOf === st.name ? "Ocultar partitura" : "Ver partitura"}
              {:else}♪ Sacar notas{st.name === "vocals" ? " y letra" : ""}{/if}
            </button>
          {/if}
        </div>
        <div class="track"><div class="inner">
          {#if buf}<Waveform buffer={buf} color={info.color} dim={!st.present || muted[st.name]} onseek={seek} />{/if}
          <div class="playhead" style:left="{(time / (duration || 1)) * 100}%"></div>
        </div></div>
      </div>
    {/each}
  </section>
  {#if showRoll}
    <Roll
      {player}
      tracks={rollTracks}
      {controls}
      ontoggle={(stem) => (rollHidden[stem] = !rollHidden[stem])}
      onclose={() => (showRoll = false)}
    />
  {/if}
  {#if scoreStem}
    {#key `${scoreStem.score}:${scoreStem.version}`}
      <Score
        {song}
        stem={scoreStem}
        {player}
        {controls}
        busy={!!jobFor(scoreStem.name) && !jobFor(scoreStem.name)?.error}
        onredo={() => ontranscribe(scoreStem.name)}
        onclose={() => (scoreOf = null)}
      />
    {/key}
  {/if}
{/if}

<style>
  header { display: flex; justify-content: space-between; align-items: flex-start; gap: 16px; }
  h1 { margin: 0; font-size: 26px; font-weight: 700; letter-spacing: -0.02em; }
  .sub { margin: 4px 0 0; color: var(--text-2); font-size: 13px; }
  .actions { display: flex; gap: 8px; flex-shrink: 0; }

  .bar { margin: 24px 0 18px; }

  .lanes { display: flex; flex-direction: column; gap: 8px; }
  .lane {
    display: grid; grid-template-columns: 260px 1fr; align-items: center;
    background: var(--panel); border: 1px solid var(--line); border-radius: 12px; overflow: hidden;
    transition: opacity 0.2s;
  }
  .lane.absent { opacity: 0.55; }
  .head {
    display: grid; grid-template-columns: auto 1fr auto; grid-template-rows: auto auto;
    gap: 6px 10px; align-items: center; padding: 10px 14px; border-right: 1px solid var(--line);
  }
  .dot { width: 10px; height: 10px; border-radius: 50%; background: var(--c); box-shadow: 0 0 10px var(--c); }
  .name { display: flex; flex-direction: column; line-height: 1.2; }
  .name small { color: var(--text-3); font-size: 11px; }
  .ms { display: flex; gap: 4px; }
  .ms button {
    width: 26px; height: 24px; border-radius: 6px; border: 1px solid var(--line); background: transparent;
    color: var(--text-2); font-size: 11px; font-weight: 700; cursor: pointer;
  }
  .ms button.on { background: #e5484d; border-color: #e5484d; color: white; }
  .ms button.solo.on { background: #f5c542; border-color: #f5c542; color: #1a1a1a; }
  .head input { grid-column: 1 / -1; width: 100%; accent-color: var(--c); }
  .notes {
    grid-column: 1 / -1; padding: 5px 8px; border-radius: 7px; border: 1px solid var(--line);
    background: transparent; color: var(--text-2); font-size: 12px; cursor: pointer;
  }
  .notes:hover:not(:disabled) { color: var(--text); border-color: var(--c); }
  .notes.open { color: var(--text); border-color: var(--c); background: color-mix(in srgb, var(--c) 15%, transparent); }
  .notes:disabled { cursor: default; opacity: 0.7; }
  .track { padding: 6px 12px; }
  .inner { position: relative; }
  .playhead {
    position: absolute; top: 0; bottom: 0; width: 2px; background: white;
    opacity: 0.85; pointer-events: none; box-shadow: 0 0 8px white;
  }

  .loading { display: flex; align-items: center; gap: 12px; color: var(--text-2); padding: 40px 0; }
  .spinner {
    width: 18px; height: 18px; border-radius: 50%; border: 2px solid var(--line); border-top-color: var(--accent);
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
  .error { color: #ff8a8a; }
</style>
