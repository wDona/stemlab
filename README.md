# StemLab

Separa canciones en pistas (voz, batería, bajo, guitarra, piano, otros) y saca sus notas, todo en local.

- **Separación**: BS-RoFormer para la voz y BS-Roformer-SW para los instrumentos (vía `audio-separator`, en GPU).
- **Entrada**: cualquier audio/vídeo que lea ffmpeg, o búsqueda / enlace de YouTube y demás webs de `yt-dlp`.
- **Biblioteca**: carpetas, reordenar arrastrando, menús con clic derecho, borrar canciones.
- **Vista de canción**: pestañas Mezclador / Piano roll / Partitura con la barra de reproducción siempre visible, volumen general, mini-mezclador por instrumento y metrónomo que sigue los pulsos reales (librosa). Los ajustes (volúmenes, M/S, qué se ve) se guardan por canción.
- **Notas**: transcripción con basic-pitch (ONNX, CPU) a MusicXML + MIDI. Partitura de conjunto (un pentagrama por instrumento, OpenSheetMusicDisplay) que se pinta al reproducir y se exporta a PDF; piano roll estilo Synthesia. Ambos a pantalla completa.
- **Letra**: Whisper sobre la voz aislada, sílabas (pyphen) repartidas bajo cada nota de la partitura de voz.
- Las operaciones de IA van a una cola en segundo plano; una sola a la vez en la GPU (`flock`).

## Estructura

- `engine/`: motor Python (`engine.py`). Cada feature es un subcomando que emite JSON lines.
- `app/`: Tauri v2 + SvelteKit. Rust lanza el motor y reenvía sus eventos por un `Channel`.

Datos, modelos y canciones en la carpeta de datos (ver «Instalar»).

## Instalar (Linux)

```sh
git clone https://github.com/wDona/stemlab.git && cd stemlab
./install.sh            # detecta la GPU: NVIDIA (CUDA), AMD (ROCm) o ninguna (CPU)
```

Compila la app, la instala en `~/.local/bin/stemlab` y añade **StemLab** al lanzador de aplicaciones
(rofi, quickshell, GNOME, KDE…). No usa `sudo`: si falta algo del sistema, dice qué paquete instalar
en Arch, Debian/Ubuntu o Fedora.

- Opciones: `./install.sh --gpu cuda|rocm|cpu` para forzar la GPU, `./install.sh --uninstall` para quitarla
  (las canciones no se borran).
- Necesita: `uv`, Node + npm, Rust, `ffmpeg`, WebKitGTK 4.1 y `gst-plugins-good`.
- No muevas la carpeta del repo después: el motor se ejecuta desde `engine/`. Para actualizar,
  `git pull && ./install.sh`.
- La primera separación y la primera letra descargan los modelos (~3 GB en total).
- Datos (canciones, modelos, ajustes): `$STEMLAB_DATA` si está definida; si no, `/data/stemlab` si existe;
  si no, `~/.local/share/stemlab`.
- Sin GPU funciona todo, pero separar una canción tarda bastante más.

## Desarrollo

```sh
cd engine && uv sync
cd ../app && npm install && npm run tauri dev
```

Tests: `cd engine && uv run python test_engine.py` y `cd app/src-tauri && cargo test`.
