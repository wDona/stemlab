<script lang="ts">
  import { revealItemInDir, openPath } from "@tauri-apps/plugin-opener";
  import { untrack } from "svelte";
  import { engine, log, PITCHED, readJson, SETTINGS, STEM_INFO, writeJson, type Song } from "./engine";
  import type { Job } from "./Library.svelte";
  import Roll from "./Roll.svelte";
  import Score from "./Score.svelte";
  import MiniMixer from "./MiniMixer.svelte";
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
  let masterVolume = $state(1);
  let showMini = $state(false); // tira compacta de volúmenes sobre la barra
  let rollSpeed = $state(2.5); // segundos de notas visibles en el piano roll
  let scoreFollow = $state(true);
  type Tab = "mix" | "roll" | "score";
  const TABS: [Tab, string][] = [["mix", "Mezclador"], ["roll", "Piano roll"], ["score", "Partitura"]];
  let tab = $state<Tab>("mix");
  let scoreHidden = $state<Record<string, boolean>>({}); // pentagramas ocultos en la pestaña Partitura
  let scorePath = $state<string | null>(null);
  let scoreError = $state("");
  let rollHidden = $state<Record<string, boolean>>({}); // instrumentos quitados del piano roll
  let unseen = $state(0); // partituras terminadas mientras estabas en otra pestaña

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
  const scored = $derived(song.stems.filter((s) => s.score));
  const scoreShown = $derived(scored.filter((s) => !scoreHidden[s.name]));
  // cambia al mostrar/ocultar un pentagrama o al rehacer una transcripción
  const scoreKey = $derived(scoreShown.map((s) => `${s.name}@${s.version ?? 0}`).join(","));

  // partitura de conjunto: el motor la monta al momento con las notas ya guardadas (sin IA)
  $effect(() => {
    const key = scoreKey;
    const stems = untrack(() => scoreShown.map((s) => s.name));
    scorePath = null;
    scoreError = "";
    if (!stems.length) return;
    engine<{ path: string }>(["ensemble", untrack(() => song.id), ...stems])
      .then((r) => key === scoreKey && (scorePath = r.path))
      .catch((e) => (scoreError = String(e)));
  });

  // aviso en la pestaña cuando acaba una transcripción (sin sacarte de donde estés)
  let seenVersions = untrack(() => song.stems.map((s) => s.version ?? 0).join());
  $effect(() => {
    const v = song.stems.map((s) => s.version ?? 0).join();
    if (v !== seenVersions) {
      seenVersions = v;
      if (untrack(() => tab) !== "score") unseen++;
    }
  });

  function go(t: Tab) {
    tab = t;
    if (t === "score") unseen = 0;
  }

  function noteLabel(stem: string, hasScore: boolean) {
    const job = jobFor(stem);
    const what = `notas${stem === "vocals" ? " y letra" : ""}`;
    if (job?.error) return `Reintentar ${what}`;
    if (job) return job.step ? `${job.label}…` : "En cola…";
    return hasScore ? `↻ Volver a sacar ${what}` : `♪ Sacar ${what}`;
  }
  const notesBusy = (stem: string) => !!jobFor(stem) && !jobFor(stem)?.error;

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
    restore(s);
    load(s);
    return () => player.stop();
  });

  // --- ajustes guardados: por canción (view.json) y generales (settings.json) ---
  let restored = $state(false); // hasta leerlos, no guardar: pisaríamos lo guardado con los valores por defecto
  const viewPath = (s: Song) => `${s.dir}/view.json`;

  async function restore(s: Song) {
    const [view, global] = await Promise.all([
      readJson(viewPath(s), { volume: {}, muted: {}, solo: {}, rollHidden: {}, scoreHidden: {}, tab: "mix" as Tab }),
      readJson(SETTINGS, { masterVolume: 1, showMini: false, rollSpeed: 2.5, scoreFollow: true }),
    ]);
    Object.assign(volume, view.volume);
    Object.assign(muted, view.muted);
    Object.assign(solo, view.solo);
    Object.assign(rollHidden, view.rollHidden);
    Object.assign(scoreHidden, view.scoreHidden);
    tab = view.tab;
    masterVolume = global.masterVolume;
    showMini = global.showMini;
    rollSpeed = global.rollSpeed;
    scoreFollow = global.scoreFollow;
    restored = true;
  }

  let flush: (() => void) | null = null;
  $effect(() => {
    if (!restored) return;
    const s = untrack(() => song);
    const view = $state.snapshot({ volume, muted, solo, rollHidden, scoreHidden, tab });
    const global = { masterVolume, showMini, rollSpeed, scoreFollow };
    flush = () => {
      flush = null;
      void writeJson(viewPath(s), view);
      void writeJson(SETTINGS, global);
    };
    const t = setTimeout(() => flush?.(), 500); // agrupa los movimientos de un deslizador
    return () => clearTimeout(t);
  });
  // al cambiar de canción o cerrar la ventana, lo pendiente se guarda ya
  $effect(() => {
    const now = () => flush?.();
    window.addEventListener("beforeunload", now);
    return () => {
      window.removeEventListener("beforeunload", now);
      now();
    };
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

  $effect(() => player.setMaster(masterVolume));

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
    const t = e.target as HTMLElement;
    if (t instanceof HTMLInputElement || t instanceof HTMLTextAreaElement || e.ctrlKey || e.metaKey || e.altKey) return;
    if (e.code === "Space") toggle();
    else if (e.key === "ArrowLeft") player.seek(time - 5);
    else if (e.key === "ArrowRight") player.seek(time + 5);
    else if (e.key === "1" || e.key === "2" || e.key === "3") go(TABS[+e.key - 1][0]);
    else if (e.key === "v" || e.key === "V") showMini = !showMini;
    else return;
    e.preventDefault();
  }
</script>

<svelte:window onkeydown={onkey} />

<!-- barra de abajo: la misma en las tres pestañas y dentro de las pantallas completas -->
{#snippet controls()}
  {#if showMini && !loading}<MiniMixer stems={song.stems} bind:volume bind:muted bind:solo />{/if}
  <Transport
    {playing}
    {time}
    {duration}
    bind:volume={masterVolume}
    disabled={loading}
    ontoggle={toggle}
    onseek={(t) => player.seek(t)}
  >
    <button class="ghost mix-toggle" class:active={showMini} onclick={() => (showMini = !showMini)} title="Volumen de cada instrumento (V)">
      🎚 Pistas
    </button>
  </Transport>
{/snippet}

<div class="view">
  <header>
    <div class="heading">
      <h1>{song.title}</h1>
      <p class="sub">
        {song.stems.filter((s) => s.present).length} de {song.stems.length} pistas con sonido{song.bpm ? ` · ${song.bpm} BPM` : ""}
      </p>
    </div>
    <div class="actions">
      <button class="ghost" onclick={() => openPath(`${song.dir}/stems`)}>Abrir carpeta</button>
      <button class="ghost" onclick={() => revealItemInDir(`${song.dir}/meta.json`)}>Mostrar en gestor</button>
    </div>
  </header>

  <nav class="tabs">
    {#each TABS as [id, label], i (id)}
      <button class:active={tab === id} onclick={() => go(id)} title="Atajo: {i + 1}">
        {label}
        {#if id === "score" && unseen}<span class="badge">{unseen}</span>{/if}
        {#if id !== "mix" && !(id === "roll" ? rollTracks.length : scored.length)}<span class="empty-dot" title="Sin notas todavía"></span>{/if}
      </button>
    {/each}
    <span class="keys">Espacio ▶︎ · ← → 5 s · 1 2 3 pestañas · V volúmenes</span>
  </nav>

  <div class="content" class:fill={tab !== "mix"}>
    {#if loadError}
      <p class="error">No se pudieron cargar las pistas: {loadError}</p>
    {:else if loading}
      <div class="loading"><div class="spinner"></div> {converting ? "Convirtiendo a un formato compatible…" : "Cargando pistas…"}</div>
    {:else if tab === "mix"}
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
                <button class="notes" class:done={!!st.score} disabled={notesBusy(st.name)} onclick={() => ontranscribe(st.name)}>
                  {noteLabel(st.name, !!st.score)}
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
    {:else if !(tab === "roll" ? song.stems.some((s) => s.roll) : scored.length)}
      <div class="empty">
        <h2>{tab === "roll" ? "El piano roll necesita notas" : "Aún no hay partituras"}</h2>
        <p>Las notas se sacan desde el mezclador, con el botón de cada instrumento. Se hace en segundo plano: puedes seguir escuchando.</p>
        <button class="go" onclick={() => go("mix")}>Ir al mezclador</button>
      </div>
    {:else if tab === "roll"}
      <Roll
        {player}
        tracks={rollTracks}
        {controls}
        bind:windowSec={rollSpeed}
        ontoggle={(stem) => (rollHidden[stem] = !rollHidden[stem])}
      />
    {:else}
      <div class="picker">
        {#each scored as st (st.name)}
          <button
            class="chip"
            class:off={scoreHidden[st.name]}
            style:--c={STEM_INFO[st.name].color}
            onclick={() => (scoreHidden[st.name] = !scoreHidden[st.name])}
            title={scoreHidden[st.name] ? "Mostrar" : "Ocultar"}
          >
            <span class="dot"></span>{STEM_INFO[st.name].label}
          </button>
        {/each}
      </div>
      {#if !scoreShown.length}
        <div class="empty"><p>Todos los instrumentos están ocultos. Pulsa uno arriba para verlo.</p></div>
      {:else if scoreError}
        <p class="error">{scoreError}</p>
      {:else if !scorePath}
        <div class="loading"><div class="spinner"></div> Montando partitura…</div>
      {:else}
        {#key `${scorePath}:${scoreKey}`}
          <Score {song} stems={scoreShown} path={scorePath} {player} {controls} bind:follow={scoreFollow} />
        {/key}
      {/if}
    {/if}
  </div>

  <footer class="bar">{@render controls()}</footer>
</div>

<style>
  .view { height: 100%; display: flex; flex-direction: column; min-height: 0; }
  header { display: flex; justify-content: space-between; align-items: flex-start; gap: 16px; flex-shrink: 0; }
  h1 { margin: 0; font-size: 26px; font-weight: 700; letter-spacing: -0.02em; }
  .sub { margin: 4px 0 0; color: var(--text-2); font-size: 13px; }
  .actions { display: flex; gap: 8px; flex-shrink: 0; }

  .tabs { display: flex; align-items: center; gap: 4px; margin: 18px 0 14px; border-bottom: 1px solid var(--line); flex-shrink: 0; }
  .tabs button {
    position: relative; display: flex; align-items: center; gap: 6px; padding: 9px 16px; margin-bottom: -1px;
    border: 0; border-bottom: 2px solid transparent; background: none; color: var(--text-2); font-size: 14px; cursor: pointer;
  }
  .tabs button:hover { color: var(--text); }
  .tabs button.active { color: var(--text); border-bottom-color: var(--accent); font-weight: 600; }
  .badge { min-width: 18px; height: 18px; padding: 0 5px; border-radius: 9px; background: var(--accent); color: white; font-size: 11px; display: grid; place-items: center; }
  .empty-dot { width: 5px; height: 5px; border-radius: 50%; background: var(--text-3); }
  .keys { margin-left: auto; color: var(--text-3); font-size: 11px; }

  .content { flex: 1; min-height: 0; overflow-y: auto; }
  .content.fill { display: flex; flex-direction: column; overflow: hidden; }
  .bar { flex-shrink: 0; padding-top: 14px; }
  .mix-toggle { padding: 6px 12px; flex-shrink: 0; }

  .empty { margin: auto; text-align: center; max-width: 520px; padding: 40px 0; }
  .empty h2 { margin: 0 0 8px; font-size: 20px; }
  .empty p { color: var(--text-2); margin: 0 0 20px; }

  .picker { display: flex; gap: 6px; flex-wrap: wrap; margin-bottom: 10px; flex-shrink: 0; }
  .chip {
    display: flex; align-items: center; gap: 7px; padding: 6px 12px; border-radius: 99px; cursor: pointer;
    border: 1px solid var(--line); background: transparent; color: var(--text-2); font-size: 13px;
  }
  .chip:hover:not(:disabled) { color: var(--text); border-color: var(--c); }
  .chip { color: var(--text); border-color: var(--c); background: color-mix(in srgb, var(--c) 15%, transparent); }
  .chip.off { opacity: 0.45; text-decoration: line-through; background: transparent; border-color: var(--line); }
  .notes.done { color: var(--text-3); }
  .go { padding: 10px 20px; border-radius: 10px; border: 0; background: var(--accent-grad); color: white; font-weight: 600; cursor: pointer; }

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
  .dot { flex-shrink: 0; width: 10px; height: 10px; border-radius: 50%; background: var(--c); box-shadow: 0 0 10px var(--c); }
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
