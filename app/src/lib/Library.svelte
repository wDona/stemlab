<script lang="ts">
  import { libraryPath, readJson, writeJson, type Song } from "./engine";

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

  let { songs, jobs, selected, searching, onselect, onadd, onsearch, ondismiss, ondelete }: {
    songs: Song[];
    jobs: Job[];
    selected: string | null;
    searching: boolean;
    onselect: (id: string) => void;
    onadd: () => void;
    onsearch: () => void;
    ondismiss: (job: Job) => void;
    ondelete: (song: Song) => void;
  } = $props();

  const progress = (j: Job) => (j.step ? ((j.step - 1 + j.pct / 100) / j.total) * 100 : 0);

  // --- carpetas: solo organizan la barra lateral; las canciones no se mueven en disco ---
  type Folder = { id: string; name: string; songs: string[]; open: boolean };
  let folders = $state<Folder[]>([]);
  let root = $state<string[]>([]); // orden de las canciones fuera de carpetas
  let loaded = false;
  let creating = $state<string | null>(null); // nombre que se está escribiendo; "" = recién abierto
  let moveAfterCreate: string | null = null; // canción a meter en la carpeta que se está creando
  let renaming = $state<string | null>(null);
  // menú contextual (clic derecho) de una canción o una carpeta, en la posición del ratón
  type MenuKind = "song" | "folder" | "library";
  let menu = $state<{ kind: MenuKind; id: string; x: number; y: number } | null>(null);
  let menuEl = $state<HTMLDivElement>();

  function openMenu(e: MouseEvent, kind: MenuKind, id: string) {
    e.preventDefault();
    e.stopPropagation();
    menu = { kind, id, x: e.clientX, y: e.clientY };
  }
  // que no se salga de la ventana
  $effect(() => {
    if (!menu || !menuEl) return;
    const r = menuEl.getBoundingClientRect();
    if (r.bottom > innerHeight) menu.y = Math.max(4, innerHeight - r.height - 4);
    if (r.right > innerWidth) menu.x = Math.max(4, innerWidth - r.width - 4);
  });
  let dropTarget = $state<string | null>(null); // carpeta, "root" o "song:<id>" bajo lo que se arrastra

  readJson(libraryPath(), { folders: [] as Folder[], root: [] as string[] }).then((l) => {
    folders = l.folders;
    root = l.root;
    loaded = true;
  });
  function save() {
    root = loose.map((s) => s.id); // fija el orden visible (incluye canciones nuevas, al final)
    if (loaded) void writeJson(libraryPath(), { folders: $state.snapshot(folders), root: $state.snapshot(root) });
  }

  // canciones borradas fuera de las carpetas (sin guardar: se limpia en el próximo cambio)
  const known = $derived(new Set(songs.map((s) => s.id)));
  const inFolder = $derived(new Set(folders.flatMap((f) => f.songs)));
  // sueltas en el orden guardado; las nuevas, al final
  const loose = $derived.by(() => {
    const pos = new Map(root.map((id, i) => [id, i]));
    return songs.filter((s) => !inFolder.has(s.id)).sort((a, b) => (pos.get(a.id) ?? 1e9) - (pos.get(b.id) ?? 1e9));
  });
  const songsOf = (f: Folder) => f.songs.filter((id) => known.has(id)).map((id) => songs.find((s) => s.id === id)!);

  function createFolder(name: string) {
    name = name.trim();
    creating = null;
    if (!name) return;
    const f: Folder = { id: crypto.randomUUID(), name, songs: [], open: true };
    folders.push(f);
    if (moveAfterCreate) moveTo(moveAfterCreate, f.id);
    moveAfterCreate = null;
    save();
  }

  /** Mueve la canción a la carpeta `folderId` (null = fuera de carpetas), antes de `beforeId` o al final. */
  function moveTo(songId: string, folderId: string | null, beforeId: string | null = null) {
    if (songId === beforeId) return;
    const list = (folderId === null ? loose.map((s) => s.id) : folders.find((f) => f.id === folderId)!.songs).filter(
      (id) => id !== songId && known.has(id),
    );
    for (const f of folders) f.songs = f.songs.filter((id) => id !== songId && known.has(id));
    const at = beforeId ? list.indexOf(beforeId) : -1;
    list.splice(at < 0 ? list.length : at, 0, songId);
    if (folderId === null) root = list;
    else {
      const f = folders.find((f) => f.id === folderId)!;
      f.songs = list;
      f.open = true;
    }
    menu = null;
    save();
  }

  /** Sube (-1) o baja (+1) una canción dentro de su lista. */
  function shift(songId: string, dir: number) {
    const f = folders.find((f) => f.songs.includes(songId));
    const list = f ? songsOf(f).map((s) => s.id) : loose.map((s) => s.id);
    const i = list.indexOf(songId) + dir;
    if (i < 0 || i >= list.length) return;
    // bajar = ponerla antes de la que va dos puestos más abajo (o al final)
    moveTo(songId, f?.id ?? null, dir < 0 ? list[i] : (list[i + 1] ?? null));
  }

  function moveFolder(id: string, beforeId: string) {
    const from = folders.findIndex((f) => f.id === id);
    if (from < 0 || id === beforeId) return;
    const [f] = folders.splice(from, 1);
    folders.splice(folders.findIndex((x) => x.id === beforeId), 0, f);
    save();
  }

  function removeFolder(id: string) {
    const i = folders.findIndex((f) => f.id === id);
    if (i < 0) return;
    folders.splice(i, 1); // sus canciones vuelven a la raíz
    save();
  }

  function rename(f: Folder, name: string) {
    renaming = null;
    if (name.trim()) f.name = name.trim();
    save();
  }

  const focus = (el: HTMLInputElement) => {
    el.focus();
    el.select();
  };

  const SONG = "text/stemlab-song";
  const FOLDER = "text/stemlab-folder";

  /** Soltar en una carpeta/raíz (al final) o sobre una canción (`beforeId`: justo antes de ella). */
  function drop(e: DragEvent, folderId: string | null, beforeId: string | null = null) {
    e.preventDefault();
    e.stopPropagation();
    dropTarget = null;
    const song = e.dataTransfer?.getData(SONG);
    const folder = e.dataTransfer?.getData(FOLDER);
    if (song) moveTo(song, folderId, beforeId);
    else if (folder && folderId) moveFolder(folder, folderId);
  }
  const over = (e: DragEvent, target: string, accepts: string[] = [SONG]) => {
    if (!accepts.some((t) => e.dataTransfer?.types.includes(t))) return;
    e.preventDefault();
    e.stopPropagation();
    dropTarget = target;
  };
</script>

<svelte:window
  onclick={() => (menu = null)}
  oncontextmenu={() => (menu = null)}
  onkeydown={(e) => e.key === "Escape" && (menu = null)}
  onblur={() => (menu = null)}
/>

{#snippet songRow(song: Song, folderId: string | null)}
  <div
    class="item"
    class:before={dropTarget === `song:${song.id}`}
    role="listitem"
    draggable="true"
    ondragstart={(e) => e.dataTransfer?.setData(SONG, song.id)}
    ondragover={(e) => over(e, `song:${song.id}`)}
    ondragleave={() => dropTarget === `song:${song.id}` && (dropTarget = null)}
    ondrop={(e) => drop(e, folderId, song.id)}
    oncontextmenu={(e) => openMenu(e, "song", song.id)}
  >
    <button class="song" class:active={song.id === selected} onclick={() => onselect(song.id)} title="Clic derecho: mover, ordenar, eliminar">
      <span class="title">{song.title}</span>
      <span class="chips">
        {#each song.stems.filter((s) => s.present) as s}<i class="c-{s.name}"></i>{/each}
      </span>
    </button>
    <div class="acts">
      <button class="act del" onclick={() => ondelete(song)} title="Eliminar canción" aria-label="Eliminar {song.title}">
        <svg viewBox="0 0 24 24"><path d="M4 7h16M10 11v6M14 11v6M6 7l1 12a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2l1-12M9 7V4h6v3" /></svg>
      </button>
    </div>
  </div>
{/snippet}

<!-- clic derecho en hueco libre de la barra: crear carpeta (canciones y carpetas abren el suyo y no llega aquí) -->
<aside oncontextmenu={(e) => openMenu(e, "library", "")}>
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

  <div class="lib-head">
    <h2>Biblioteca</h2>
    <button class="new" onclick={() => (creating = "")} title="Nueva carpeta" aria-label="Nueva carpeta">
      <svg viewBox="0 0 24 24"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2zM12 10v6M9 13h6" /></svg>
    </button>
  </div>

  {#if creating !== null}
    <input
      class="name-input"
      placeholder="Nombre de la carpeta"
      bind:value={creating}
      use:focus
      onkeydown={(e) => {
        if (e.key === "Enter") createFolder(creating ?? "");
        if (e.key === "Escape") creating = moveAfterCreate = null;
      }}
      onblur={() => createFolder(creating ?? "")}
    />
  {/if}

  {#each folders as f (f.id)}
    {@const list = songsOf(f)}
    <div
      class="folder"
      class:drop={dropTarget === f.id}
      role="group"
      ondragover={(e) => over(e, f.id, [SONG, FOLDER])}
      ondragleave={() => dropTarget === f.id && (dropTarget = null)}
      ondrop={(e) => drop(e, f.id)}
    >
      <div
        class="frow"
        role="listitem"
        draggable="true"
        ondragstart={(e) => e.dataTransfer?.setData(FOLDER, f.id)}
        oncontextmenu={(e) => openMenu(e, "folder", f.id)}
      >
        <button
          class="fname"
          onclick={() => {
            f.open = !f.open;
            save();
          }}
          ondblclick={() => (renaming = f.id)}
          title="Doble clic: renombrar · Clic derecho: opciones"
        >
          <svg class="chev" class:open={f.open} viewBox="0 0 24 24"><path d="m9 6 6 6-6 6" /></svg>
          {#if renaming === f.id}
            <input
              class="name-input inline"
              value={f.name}
              use:focus
              onclick={(e) => e.stopPropagation()}
              onkeydown={(e) => {
                if (e.key === "Enter") rename(f, e.currentTarget.value);
                if (e.key === "Escape") renaming = null;
              }}
              onblur={(e) => rename(f, e.currentTarget.value)}
            />
          {:else}
            <span class="title">{f.name}</span>
          {/if}
          <span class="count">{list.length}</span>
        </button>
      </div>
      {#if f.open}
        <div class="inside">
          {#each list as song (song.id)}{@render songRow(song, f.id)}{:else}<p class="hint-in">Arrastra canciones aquí</p>{/each}
        </div>
      {/if}
    </div>
  {/each}

  <div
    class="root"
    class:drop={dropTarget === "root"}
    role="list"
    ondragover={(e) => over(e, "root")}
    ondragleave={() => dropTarget === "root" && (dropTarget = null)}
    ondrop={(e) => drop(e, null)}
  >
    {#each loose as song (song.id)}{@render songRow(song, null)}{/each}
    {#if !songs.length}<p class="empty">Aún no hay canciones.</p>{/if}
  </div>
</aside>

{#if menu}
  {@const m = menu}
  <div
    class="menu"
    bind:this={menuEl}
    role="menu"
    tabindex="-1"
    style:left="{m.x}px"
    style:top="{m.y}px"
    onclick={(e) => e.stopPropagation()}
    oncontextmenu={(e) => e.preventDefault()}
    onkeydown={() => {}}
  >
    {#if m.kind === "song"}
      {@const song = songs.find((s) => s.id === m.id)}
      {#if song}
        <small class="head">{song.title}</small>
        <button onclick={() => shift(song.id, -1)}>↑ Subir</button>
        <button onclick={() => shift(song.id, 1)}>↓ Bajar</button>
        <hr />
        <small>Mover a</small>
        {#each folders as f (f.id)}
          <button onclick={() => moveTo(song.id, f.id)} class:current={f.songs.includes(song.id)}>📁 {f.name}</button>
        {/each}
        <button onclick={() => moveTo(song.id, null)} class:current={!inFolder.has(song.id)}>Sin carpeta</button>
        <button
          onclick={() => {
            moveAfterCreate = song.id;
            creating = "";
            menu = null;
          }}>+ Nueva carpeta…</button
        >
        <hr />
        <button
          class="danger"
          onclick={() => {
            ondelete(song); // antes de cerrar: `song` deriva de `menu` y con null deja de existir
            menu = null;
          }}>Eliminar canción…</button
        >
      {/if}
    {:else if m.kind === "library"}
      <button
        onclick={() => {
          menu = null;
          creating = "";
        }}>+ Nueva carpeta</button
      >
      <button
        onclick={() => {
          menu = null;
          onadd();
        }}>Añadir canción…</button
      >
      {#if folders.length}
        <hr />
        <button
          onclick={() => {
            const open = !folders.every((f) => f.open);
            for (const f of folders) f.open = open;
            menu = null;
            save();
          }}>{folders.every((f) => f.open) ? "Plegar" : "Desplegar"} todas las carpetas</button
        >
      {/if}
    {:else}
      {@const f = folders.find((f) => f.id === m.id)}
      {#if f}
        <small class="head">📁 {f.name}</small>
        <button
          onclick={() => {
            renaming = f.id;
            menu = null;
          }}>Renombrar</button
        >
        <button
          class="danger"
          onclick={() => {
            removeFolder(f.id); // antes de cerrar: `f` deriva de `menu`
            menu = null;
          }}>Quitar carpeta <span class="note">(las canciones se quedan)</span></button
        >
      {/if}
    {/if}
  </div>
{/if}

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
  .item { position: relative; }
  .item.before::before {
    content: ""; position: absolute; left: 6px; right: 6px; top: -2px; height: 2px; border-radius: 1px;
    background: var(--accent); box-shadow: 0 0 6px var(--accent);
  }
  .item .song { padding-right: 38px; }
  .acts {
    position: absolute; right: 6px; top: 50%; translate: 0 -50%; display: flex; gap: 2px;
    opacity: 0; transition: opacity 0.15s;
  }
  .item:hover > .acts, .acts:focus-within { opacity: 1; }
  .act {
    width: 26px; height: 26px; border-radius: 7px; display: grid; place-items: center;
    border: 0; background: transparent; color: var(--text-3); cursor: pointer;
  }
  .act:hover { color: var(--text); background: var(--panel-hi); }
  .act.del:hover { color: #ff6b6b; background: #ff6b6b1a; }
  .act svg, .new svg, .chev { width: 15px; height: 15px; stroke: currentColor; stroke-width: 2; fill: none; stroke-linecap: round; stroke-linejoin: round; }

  .lib-head { display: flex; align-items: center; justify-content: space-between; margin: 24px 0 8px; }
  .lib-head h2 { margin: 0 6px; }
  .new { width: 26px; height: 26px; border-radius: 7px; display: grid; place-items: center; border: 0; background: transparent; color: var(--text-3); cursor: pointer; }
  .new:hover { color: var(--text); background: var(--panel); }
  .name-input {
    width: 100%; box-sizing: border-box; margin-bottom: 6px; padding: 8px 10px; border-radius: 8px;
    border: 1px solid var(--accent); background: var(--panel); color: var(--text); font: inherit; font-size: 13px; outline: none;
  }
  .name-input.inline { margin: 0; padding: 3px 6px; flex: 1; min-width: 0; }

  .folder { border-radius: 10px; margin-bottom: 2px; border: 1px solid transparent; }
  .folder.drop, .root.drop { border-color: var(--accent); background: color-mix(in srgb, var(--accent) 8%, transparent); }
  .root { min-height: 40px; border-radius: 10px; border: 1px solid transparent; }
  .frow { position: relative; }
  .fname {
    width: 100%; display: flex; align-items: center; gap: 6px; padding: 8px 8px 8px 6px; border: 0; border-radius: 8px;
    background: transparent; color: var(--text); cursor: pointer; text-align: left;
  }
  .fname:hover { background: var(--panel); }
  .fname .title { flex: 1; }
  .chev { color: var(--text-3); transition: rotate 0.15s; flex-shrink: 0; }
  .chev.open { rotate: 90deg; }
  .count { font-size: 11px; color: var(--text-3); }
  .inside { padding-left: 12px; border-left: 1px solid var(--line); margin-left: 13px; }
  .hint-in { font-size: 11px; color: var(--text-3); margin: 4px 8px 8px; }

  .menu {
    position: fixed; z-index: 100; min-width: 190px; max-width: 260px; padding: 6px;
    background: var(--panel-hi); border: 1px solid var(--line); border-radius: 10px; box-shadow: 0 12px 30px -8px #000a;
    display: flex; flex-direction: column;
  }
  .menu small { color: var(--text-3); font-size: 10px; text-transform: uppercase; letter-spacing: 0.08em; padding: 4px 8px; }
  .menu button {
    text-align: left; padding: 7px 8px; border: 0; border-radius: 6px; background: none; color: var(--text-2);
    font-size: 13px; cursor: pointer; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  .menu button:hover { background: var(--panel); color: var(--text); }
  .menu button.current { color: var(--text); font-weight: 600; }
  .menu .head { text-transform: none; letter-spacing: 0; font-size: 12px; color: var(--text-2); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .menu hr { border: 0; border-top: 1px solid var(--line); margin: 4px 2px; }
  .menu .danger:hover { color: #ff6b6b; background: #ff6b6b1a; }
  .menu .note { color: var(--text-3); font-size: 11px; }
  .song.active { background: var(--panel-hi); border-color: var(--line); }
  .title { font-size: 13.5px; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .chips { display: flex; gap: 4px; }
  .chips i { width: 7px; height: 7px; border-radius: 50%; }
  .c-vocals { background: #ff6b9d; } .c-drums { background: #ffb347; } .c-bass { background: #7c7cff; }
  .c-guitar { background: #4fd1a5; } .c-piano { background: #6ec8ff; } .c-other { background: #c49bff; }
  .empty { color: var(--text-3); font-size: 13px; padding: 0 6px; }
</style>
