<script lang="ts">
  import type { Song } from "./engine";

  export type Job = {
    args: string[]; // subcomando de engine.py
    key: string;
    name: string;
    label: string;
    step: number;
    total: number;
    pct: number;
    error?: string;
  };

  let { songs, jobs, selected, searching, onselect, onadd, onsearch, ondismiss }: {
    songs: Song[];
    jobs: Job[];
    selected: string | null;
    searching: boolean;
    onselect: (id: string) => void;
    onadd: () => void;
    onsearch: () => void;
    ondismiss: (job: Job) => void;
  } = $props();

  const progress = (j: Job) => (j.step ? ((j.step - 1 + j.pct / 100) / j.total) * 100 : 0);
</script>

<aside>
  <div class="brand">
    <div class="logo">
      <svg viewBox="0 0 24 24"><path d="M3 12h2M7 8v8M11 5v14M15 9v6M19 7v10M21 12h0" /></svg>
    </div>
    <span>StemLab</span>
  </div>

  <button class="add" onclick={onadd}>
    <svg viewBox="0 0 24 24"><path d="M12 5v14M5 12h14" /></svg>
    Añadir canción
  </button>
  <p class="hint">o arrastra audio a la ventana</p>

  <button class="nav" class:active={searching} onclick={onsearch}>
    <svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" /></svg>
    Buscar en YouTube
  </button>

  {#if jobs.length}
    <h2>Procesando</h2>
    {#each jobs as job (job.key)}
      <div class="job" class:failed={job.error}>
        <div class="row">
          <span class="title">{job.name}</span>
          <button class="x" onclick={() => ondismiss(job)} title={job.error ? "Quitar" : "Cancelar"}>×</button>
        </div>
        {#if job.error}
          <small class="err" title={job.error}>{job.error}</small>
        {:else}
          <small>{job.step ? `${job.label} · ${job.step}/${job.total}` : "En cola"}</small>
          <div class="bar"><div style:width="{progress(job)}%"></div></div>
        {/if}
      </div>
    {/each}
  {/if}

  <h2>Biblioteca</h2>
  {#each songs as song (song.id)}
    <button class="song" class:active={song.id === selected} onclick={() => onselect(song.id)}>
      <span class="title">{song.title}</span>
      <span class="chips">
        {#each song.stems.filter((s) => s.present) as s}<i class="c-{s.name}"></i>{/each}
      </span>
    </button>
  {:else}
    <p class="empty">Aún no hay canciones.</p>
  {/each}
</aside>

<style>
  aside {
    width: 280px; flex-shrink: 0; height: 100vh; overflow-y: auto; padding: 20px 14px;
    background: var(--sidebar); border-right: 1px solid var(--line); box-sizing: border-box;
  }
  .brand { display: flex; align-items: center; gap: 10px; font-weight: 700; font-size: 17px; padding: 0 6px 20px; }
  .logo { width: 32px; height: 32px; border-radius: 9px; background: var(--accent-grad); display: grid; place-items: center; }
  .logo svg, .add svg { width: 18px; height: 18px; stroke: white; stroke-width: 2.4; stroke-linecap: round; fill: none; }
  .add {
    width: 100%; display: flex; align-items: center; justify-content: center; gap: 8px;
    padding: 11px; border-radius: 11px; border: 0; background: var(--accent-grad); color: white;
    font-weight: 600; font-size: 14px; cursor: pointer; box-shadow: 0 8px 24px -10px var(--accent);
  }
  .add:hover { filter: brightness(1.1); }
  .nav {
    width: 100%; display: flex; align-items: center; gap: 10px; margin-top: 14px; padding: 10px 12px;
    border-radius: 10px; border: 1px solid transparent; background: transparent; color: var(--text-2); cursor: pointer;
  }
  .nav svg { width: 17px; height: 17px; stroke: currentColor; stroke-width: 2; fill: none; stroke-linecap: round; }
  .nav:hover { background: var(--panel); color: var(--text); }
  .nav.active { background: var(--panel-hi); border-color: var(--line); color: var(--text); }
  .hint { text-align: center; font-size: 11px; color: var(--text-3); margin: 8px 0 0; }
  h2 { font-size: 11px; text-transform: uppercase; letter-spacing: 0.08em; color: var(--text-3); margin: 24px 6px 8px; }

  .job { padding: 10px 12px; border-radius: 10px; background: var(--panel); border: 1px solid var(--line); margin-bottom: 6px; }
  .job.failed { border-color: #5a2a2e; }
  .row { display: flex; justify-content: space-between; gap: 6px; }
  .job small { display: block; color: var(--text-2); font-size: 11px; margin-top: 3px; }
  .job .err { color: #ff8a8a; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .x { background: none; border: 0; color: var(--text-2); cursor: pointer; font-size: 16px; line-height: 1; }
  .bar { height: 4px; border-radius: 2px; background: var(--line); margin-top: 8px; overflow: hidden; }
  .bar div { height: 100%; background: var(--accent-grad); transition: width 0.3s; }

  .song {
    width: 100%; display: flex; flex-direction: column; gap: 6px; text-align: left; padding: 10px 12px;
    border-radius: 10px; border: 1px solid transparent; background: transparent; color: var(--text); cursor: pointer;
  }
  .song:hover { background: var(--panel); }
  .song.active { background: var(--panel-hi); border-color: var(--line); }
  .title { font-size: 13.5px; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .chips { display: flex; gap: 4px; }
  .chips i { width: 7px; height: 7px; border-radius: 50%; }
  .c-vocals { background: #ff6b9d; } .c-drums { background: #ffb347; } .c-bass { background: #7c7cff; }
  .c-guitar { background: #4fd1a5; } .c-piano { background: #6ec8ff; } .c-other { background: #c49bff; }
  .empty { color: var(--text-3); font-size: 13px; padding: 0 6px; }
</style>
