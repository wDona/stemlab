<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { engine, listSongs, log, STEM_INFO, type Song } from "$lib/engine";
  import Library, { type Job } from "$lib/Library.svelte";
  import Mixer from "$lib/Mixer.svelte";
  import Search from "$lib/Search.svelte";

  // solo filtra el selector; lo arrastrado se intenta igual y ffmpeg decide
  const MEDIA = ["mp3", "wav", "flac", "ogg", "opus", "m4a", "aac", "aiff", "wma", "webm", "mp4", "mkv", "mov"];

  let songs = $state<Song[]>([]);
  let jobs = $state<Job[]>([]);
  let selected = $state<string | null>(null);
  let searching = $state(false);
  let dragging = $state(false);
  let busy = false;

  const current = $derived(songs.find((s) => s.id === selected));

  onMount(() => {
    // procesos de una carga anterior (recarga, hot reload): ya nadie los escucha
    void invoke("cancel_all");
    window.addEventListener("error", (e) => log(`error: ${e.message}`));
    window.addEventListener("unhandledrejection", (e) => log(`promesa: ${e.reason}`));
    listSongs().then((s) => {
      songs = s;
      selected ??= s[0]?.id ?? null;
    });
    const unlisten = getCurrentWebview().onDragDropEvent((e) => {
      const t = e.payload.type;
      dragging = t === "enter" || t === "over";
      if (t === "drop") enqueue(e.payload.paths);
    });
    return () => void unlisten.then((f) => f());
  });

  async function pick() {
    const picked = await open({ multiple: true, filters: [{ name: "Audio o vídeo", extensions: MEDIA }] });
    if (picked) enqueue(picked);
  }

  /** Rutas locales o URLs (name = título a mostrar mientras se procesa). */
  function enqueue(paths: string[], name?: string) {
    for (const path of paths) {
      queue(["separate", path], name ?? path.split("/").pop()!.replace(/\.[^.]+$/, ""));
    }
  }

  /** Cualquier operación de IA: va a la cola y se ejecuta en segundo plano, una cada vez. */
  function queue(args: string[], name: string) {
    const key = args.join("\0");
    if (!jobs.some((j) => j.key === key)) jobs.push({ args, key, name, label: "", step: 0, total: 3, pct: 0 });
    void runQueue();
  }

  const jobFor = (args: string[]) => jobs.find((j) => j.key === args.join("\0"));

  // una operación cada vez: la GPU no aguanta separaciones en paralelo
  async function runQueue() {
    if (busy) return;
    busy = true;
    let job: Job | undefined;
    while ((job = jobs.find((j) => !j.error))) {
      const j = job;
      try {
        const { song } = await engine<{ song: Song }>(
          j.args,
          (e) => {
          if (e.event === "waiting") j.label = e.label;
          else if (e.event === "progress") Object.assign(j, { step: e.step, total: e.total, label: e.label, pct: 0 });
          else if (e.event === "pct") j.pct = Math.max(j.pct, e.value); // demucs hace 2 pasadas: que no retroceda
          },
          j.key,
        );
        const isNew = !songs.some((s) => s.id === song.id);
        songs = [...songs.filter((s) => s.id !== song.id), song].sort((a, b) => a.id.localeCompare(b.id));
        if (isNew && !searching) selected = song.id;
        jobs.splice(jobs.indexOf(j), 1);
      } catch (err) {
        if (jobs.includes(j)) j.error = String(err); // si ya no está en la lista, se canceló
      }
    }
    busy = false;
  }
</script>

<div class="app">
  <Library
    {songs}
    {jobs}
    selected={searching ? null : selected}
    {searching}
    onselect={(id) => {
      selected = id;
      searching = false;
    }}
    onadd={pick}
    onsearch={() => (searching = true)}
    ondismiss={(j) => {
      jobs.splice(jobs.indexOf(j), 1);
      void invoke("cancel", { id: j.key }); // si estaba en marcha, mata el proceso
    }}
  />
  <main>
    {#if searching}
      <Search onpick={(url, title) => enqueue([url], title)} queued={(url) => !!jobFor(["separate", url])} />
    {:else if current}
      {#key current.id}
        <Mixer
          song={current}
          jobFor={(stem) => jobFor(["transcribe", current.id, stem])}
          ontranscribe={(stem) => queue(["transcribe", current.id, stem], `Notas · ${STEM_INFO[stem].label} · ${current.title}`)}
        />
      {/key}
    {:else}
      <div class="welcome">
        <h1>Separa cualquier canción en sus pistas</h1>
        <p>Voz, batería, bajo, guitarra, piano y el resto. Todo en local, en tu GPU.</p>
        <div class="cta">
          <button onclick={pick}>Elegir audio</button>
          <button class="ghost" onclick={() => (searching = true)}>Buscar en YouTube</button>
        </div>
      </div>
    {/if}
  </main>
  {#if dragging}
    <div class="drop"><div>Suelta para separar</div></div>
  {/if}
</div>

<style>
  :global(:root) {
    --bg: #0d0e12;
    --sidebar: #111217;
    --panel: #16181f;
    --panel-hi: #1d2029;
    --line: #262a35;
    --text: #eceef4;
    --text-2: #a3a8b8;
    --text-3: #6b7080;
    --accent: #b05cff;
    --accent-grad: linear-gradient(135deg, #ff5c9d, #8b5cff);
    color-scheme: dark;
    font-family: "Inter", system-ui, sans-serif;
    font-size: 14px;
    color: var(--text);
    background: var(--bg);
  }
  :global(body) { margin: 0; overflow: hidden; }
  :global(button) { font: inherit; }
  :global(button.ghost) {
    padding: 8px 14px; border-radius: 9px; border: 1px solid var(--line); background: var(--panel);
    color: var(--text-2); cursor: pointer; font-size: 13px;
  }
  :global(button.ghost:hover) { color: var(--text); background: var(--panel-hi); }
  :global(input[type="range"]) { accent-color: var(--accent); }

  .app { display: flex; height: 100vh; }
  main {
    flex: 1; overflow-y: auto; padding: 32px 36px;
    background: radial-gradient(1200px 500px at 70% -10%, #2a1a4a55, transparent 60%);
  }
  .welcome { height: 100%; display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center; }
  .welcome h1 { font-size: 32px; letter-spacing: -0.02em; margin: 0; }
  .welcome p { color: var(--text-2); margin: 10px 0 24px; }
  .cta { display: flex; gap: 10px; }
  .welcome button:not(.ghost) {
    padding: 12px 24px; border-radius: 12px; border: 0; background: var(--accent-grad); color: white;
    font-weight: 600; cursor: pointer;
  }
  .drop {
    position: fixed; inset: 0; display: grid; place-items: center; background: #0d0e12cc;
    backdrop-filter: blur(6px); z-index: 10;
  }
  .drop div {
    padding: 48px 72px; border: 2px dashed var(--accent); border-radius: 20px; font-size: 20px; font-weight: 600;
  }
</style>
