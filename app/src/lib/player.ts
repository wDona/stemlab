import { invoke } from "@tauri-apps/api/core";

export class DecodeError extends Error {
  constructor(public track: string) {
    super(`${track}: no se pudo decodificar`);
  }
}

/** Reproduce varias pistas sincronizadas: un AudioBufferSourceNode por pista, todas arrancadas en el mismo instante. */
export class Player {
  ctx = new AudioContext();
  /** Volumen general: todas las pistas pasan por aquí. */
  private master = (() => {
    const g = this.ctx.createGain();
    g.connect(this.ctx.destination);
    return g;
  })();
  buffers = new Map<string, AudioBuffer>();
  gains = new Map<string, GainNode>();
  private sources: AudioBufferSourceNode[] = [];
  private startedAt = 0;
  private offset = 0;
  playing = false;
  duration = 0;

  /** Lanza DecodeError si alguna pista no se puede decodificar (se arregla convirtiéndola). */
  async load(tracks: { name: string; path: string }[]) {
    this.stop();
    this.buffers.clear();
    // una a una: pedir las 6 a la vez (~160 MB por IPC) tumba el WebView de WebKitGTK
    for (const { name, path } of tracks) {
      const data = await invoke<ArrayBuffer>("read_file", { path });
      const buf = await this.ctx.decodeAudioData(data).catch(() => {
        throw new DecodeError(name);
      });
      this.buffers.set(name, buf);
      if (!this.gains.has(name)) this.gains.set(name, this.gainNode());
    }
    this.duration = Math.max(0, ...[...this.buffers.values()].map((b) => b.duration));
    this.offset = 0;
  }

  private gainNode() {
    const g = this.ctx.createGain();
    g.connect(this.master);
    return g;
  }

  setMaster(value: number) {
    this.master.gain.setTargetAtTime(value, this.ctx.currentTime, 0.015);
  }

  time() {
    const t = this.playing ? this.ctx.currentTime - this.startedAt : this.offset;
    return Math.min(t, this.duration);
  }

  play() {
    if (this.playing) return;
    if (this.offset >= this.duration) this.offset = 0;
    void this.ctx.resume();
    const at = this.ctx.currentTime + 0.05;
    this.sources = [...this.buffers].map(([name, buf]) => {
      const src = this.ctx.createBufferSource();
      src.buffer = buf;
      src.connect(this.gains.get(name)!);
      src.start(at, this.offset);
      return src;
    });
    this.startedAt = at - this.offset;
    this.playing = true;
  }

  pause() {
    if (!this.playing) return;
    this.offset = this.time();
    this.stopSources();
  }

  stop() {
    this.stopSources();
    this.offset = 0;
  }

  seek(t: number) {
    const was = this.playing;
    this.stopSources();
    this.offset = Math.max(0, Math.min(t, this.duration));
    if (was) this.play();
  }

  setGain(name: string, value: number) {
    this.gains.get(name)?.gain.setTargetAtTime(value, this.ctx.currentTime, 0.015);
  }

  private stopSources() {
    for (const s of this.sources) s.stop();
    this.sources = [];
    this.playing = false;
  }
}

/** Picos por columna para dibujar la forma de onda. */
export function peaks(buf: AudioBuffer, columns: number): Float32Array {
  const out = new Float32Array(columns);
  const step = Math.floor(buf.length / columns) || 1;
  for (let ch = 0; ch < buf.numberOfChannels; ch++) {
    const data = buf.getChannelData(ch);
    for (let c = 0; c < columns; c++) {
      let max = 0;
      for (let i = c * step, end = Math.min(i + step, data.length); i < end; i += 4) {
        const v = Math.abs(data[i]);
        if (v > max) max = v;
      }
      if (max > out[c]) out[c] = max;
    }
  }
  return out;
}
