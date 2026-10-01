<script lang="ts">
  import { engine } from "./engine";

  type Result = { url: string; title: string; channel: string; duration: number | null; thumbnail: string };

  let { onpick, queued }: { onpick: (url: string, title: string) => void; queued: (url: string) => boolean } = $props();

  let query = $state("");
  let results = $state<Result[]>([]);
  let loading = $state(false);
  let error = $state("");
  let input: HTMLInputElement;

  $effect(() => input.focus());

  async function search(e: SubmitEvent) {
    e.preventDefault();
    const q = query.trim();
    if (!q) return;
    loading = true;
    error = "";
    try {
      results = (await engine<{ results: Result[] }>(["search", q])).results;
    } catch (err) {
      error = String(err);
      results = [];
    }
    loading = false;
  }

  const fmt = (t: number | null) => (t == null ? "" : `${Math.floor(t / 60)}:${String(Math.floor(t % 60)).padStart(2, "0")}`);
</script>

<h1>Buscar</h1>
<p class="sub">Busca en YouTube o pega un enlace (YouTube, SoundCloud, Bandcamp…).</p>

<form onsubmit={search}>
  <svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" /></svg>
  <input bind:this={input} bind:value={query} placeholder="Artista, canción o URL" spellcheck="false" />
  <button disabled={loading || !query.trim()}>{loading ? "Buscando…" : "Buscar"}</button>
</form>

{#if error}<p class="error">{error}</p>{/if}

<ul>
  {#each results as r (r.url)}
    <li>
      <img src={r.thumbnail} alt="" loading="lazy" />
      <div class="info">
        <strong title={r.title}>{r.title}</strong>
        <small>{r.channel}{r.duration ? ` · ${fmt(r.duration)}` : ""}</small>
      </div>
      {#if queued(r.url)}
        <span class="done">En cola</span>
      {:else}
        <button class="go" onclick={() => onpick(r.url, r.title)}>Separar</button>
      {/if}
    </li>
  {/each}
</ul>

<style>
  h1 { margin: 0; font-size: 26px; font-weight: 700; letter-spacing: -0.02em; }
  .sub { margin: 4px 0 20px; color: var(--text-2); font-size: 13px; }
  form {
    display: flex; align-items: center; gap: 10px; padding: 6px 6px 6px 14px;
    background: var(--panel); border: 1px solid var(--line); border-radius: 14px;
  }
  form:focus-within { border-color: var(--accent); }
  form svg { width: 18px; height: 18px; stroke: var(--text-3); stroke-width: 2; fill: none; stroke-linecap: round; flex-shrink: 0; }
  input { flex: 1; background: none; border: 0; outline: none; color: var(--text); font: inherit; font-size: 15px; }
  form button, .go {
    padding: 9px 18px; border-radius: 10px; border: 0; background: var(--accent-grad); color: white;
    font-weight: 600; cursor: pointer;
  }
  form button:disabled { opacity: 0.4; cursor: default; }
  ul { list-style: none; padding: 0; margin: 18px 0 0; display: flex; flex-direction: column; gap: 8px; }
  li {
    display: flex; align-items: center; gap: 14px; padding: 8px 14px 8px 8px;
    background: var(--panel); border: 1px solid var(--line); border-radius: 12px;
  }
  img { width: 112px; height: 63px; object-fit: cover; border-radius: 8px; background: var(--line); flex-shrink: 0; }
  .info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 3px; }
  .info strong { font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .info small { color: var(--text-2); font-size: 12px; }
  .done { color: var(--text-3); font-size: 13px; padding: 0 8px; }
  .error { color: #ff8a8a; }
</style>
