import { Channel, invoke } from "@tauri-apps/api/core";

export type Stem = {
  name: string;
  file: string;
  level: number;
  present: boolean;
  score?: string; // MusicXML, tras `transcribe`
  midi?: string;
  roll?: string; // notas con tiempos reales (JSON) para el piano roll
  notes?: number;
  version?: number; // hora de la última transcripción
};
export type Song = {
  id: string;
  title: string;
  dir: string;
  mix: string;
  bpm?: number;
  beats?: string; // pulsos para el metrónomo (beats.json)
  stems: Stem[];
};

/** Pistas con altura de tono: la batería no se transcribe. */
export const PITCHED = ["vocals", "bass", "guitar", "piano", "other"];
export type EngineEvent =
  | { event: "progress"; step: number; total: number; label: string }
  | { event: "pct"; value: number }
  | { event: "waiting"; label: string }
  | { event: "done"; [k: string]: unknown }
  | { event: "error"; message: string };

/** Lanza un subcomando de engine.py. Resuelve con el evento `done`; `id` sirve para cancelarlo. */
export function engine<T>(
  args: string[],
  onEvent: (e: EngineEvent) => void = () => {},
  id: string = crypto.randomUUID(),
): Promise<T> {
  const channel = new Channel<EngineEvent>();
  channel.onmessage = onEvent;
  return invoke<T>("engine", { id, args, onEvent: channel });
}

export const listSongs = () => engine<{ songs: Song[] }>(["list"]).then((r) => r.songs);


export const STEM_INFO: Record<string, { label: string; color: string }> = {
  vocals: { label: "Voz", color: "#ff6b9d" },
  drums: { label: "Batería", color: "#ffb347" },
  bass: { label: "Bajo", color: "#7c7cff" },
  guitar: { label: "Guitarra", color: "#4fd1a5" },
  piano: { label: "Piano", color: "#6ec8ff" },
  other: { label: "Otros", color: "#c49bff" },
};

/** Escribe en la terminal de la app (stderr de Rust). */
export const log = (msg: string) => void invoke("log", { msg });

/** JSON de /data/stemlab; `fallback` si no existe o está roto. */
export async function readJson<T>(path: string, fallback: T): Promise<T> {
  try {
    return { ...fallback, ...JSON.parse(new TextDecoder().decode(await invoke<ArrayBuffer>("read_file", { path }))) };
  } catch {
    return fallback;
  }
}

export const writeJson = (path: string, data: unknown) =>
  invoke("write_file", new TextEncoder().encode(JSON.stringify(data, null, 1)), {
    headers: { path: encodeURIComponent(path) },
  }).catch((e) => log(`guardar ${path}: ${e}`));

export const SETTINGS = "/data/stemlab/settings.json";
