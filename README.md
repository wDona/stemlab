# StemLab

Separa canciones en pistas (voz, batería, bajo, guitarra, piano, otros) y saca sus notas, todo en local.

- **Separación**: BS-RoFormer para la voz y BS-Roformer-SW para los instrumentos (vía `audio-separator`, en GPU).
- **Entrada**: cualquier audio/vídeo que lea ffmpeg, o búsqueda / enlace de YouTube y demás webs de `yt-dlp`.
- **Mezclador**: forma de onda por pista, mute/solo/volumen, reproducción sincronizada.
- **Notas**: transcripción con basic-pitch (ONNX, CPU) a MusicXML + MIDI, partitura con OpenSheetMusicDisplay y piano roll estilo Synthesia.
- Las operaciones de IA van a una cola en segundo plano; una sola a la vez en la GPU (`flock`).

## Estructura

- `engine/`: motor Python (`engine.py`). Cada feature es un subcomando que emite JSON lines.
- `app/`: Tauri v2 + SvelteKit. Rust lanza el motor y reenvía sus eventos por un `Channel`.

Datos, modelos y canciones en `/data/stemlab`.

## Requisitos (Linux)

- GPU con ROCm (probado en RX 7700 XT, ROCm 7.2) o CUDA ajustando el índice de torch en `engine/pyproject.toml`.
- `uv`, Node, Rust, `ffmpeg`, WebKitGTK 4.1 y `gst-plugins-good` (sin él, el audio del WebView no funciona).

## Uso

```sh
cd engine && uv sync
cd ../app && npm install && npm run tauri dev
```

Tests: `cd engine && uv run python test_engine.py` y `cd app/src-tauri && cargo test`.
