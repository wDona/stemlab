use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{LazyLock, Mutex};
use tauri::ipc::Channel;

/// Carpeta del motor Python (uv project). Se puede mover con STEMLAB_ENGINE.
fn engine_dir() -> PathBuf {
    std::env::var("STEMLAB_ENGINE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../engine")))
}

/// Procesos del motor en marcha, por id de trabajo, para poder cancelarlos.
static RUNNING: LazyLock<Mutex<HashMap<String, Child>>> = LazyLock::new(Default::default);

/// Ejecuta `engine.py <args>`. Reenvía cada línea JSON del motor por `on_event`,
/// más eventos `pct` sacados de las barras tqdm de stderr. Devuelve el evento `done`.
#[tauri::command]
async fn engine(id: String, args: Vec<String>, on_event: Channel<Value>) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || run_engine(id, args, on_event))
        .await
        .map_err(|e| e.to_string())?
}

/// Mata un trabajo en marcha; su `engine` termina con error "cancelado".
#[tauri::command]
fn cancel(id: String) {
    if let Some(mut child) = RUNNING.lock().unwrap().remove(&id) {
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// Mata todos: el frontend lo llama al cargar, cuando ya nadie escucha a los que quedaran
/// (recarga de la página o hot reload).
#[tauri::command]
fn cancel_all() {
    for (_, mut child) in RUNNING.lock().unwrap().drain() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn run_engine(id: String, args: Vec<String>, on_event: Channel<Value>) -> Result<Value, String> {
    let dir = engine_dir();
    // python del venv directo, sin `uv run` en medio: así kill() mata el motor de verdad
    let python = dir.join(".venv/bin/python");
    if !python.exists() {
        return Err(format!("falta el entorno del motor: cd {} && uv sync", dir.display()));
    }
    let mut child = Command::new(python)
        .arg(dir.join("engine.py"))
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("no se pudo lanzar el motor: {e}"))?;

    let stderr = child.stderr.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    RUNNING.lock().unwrap().insert(id.clone(), child);
    let pct_events = on_event.clone();
    let stderr_thread = std::thread::spawn(move || tqdm_progress(stderr, pct_events));

    let mut result = Err("el motor terminó sin respuesta".to_string());
    for line in BufReader::new(stdout).lines() {
        let Ok(msg) = serde_json::from_str::<Value>(&line.map_err(|e| e.to_string())?) else {
            continue; // ruido de librerías en stdout
        };
        match msg["event"].as_str() {
            Some("done") => result = Ok(msg.clone()),
            Some("error") => result = Err(msg["message"].as_str().unwrap_or("error").to_string()),
            _ => {}
        }
        let _ = on_event.send(msg);
    }
    let Some(mut child) = RUNNING.lock().unwrap().remove(&id) else {
        return Err("cancelado".into()); // lo quitó cancel()
    };
    child.wait().map_err(|e| e.to_string())?;
    let tail = stderr_thread.join().unwrap_or_default();
    result.map_err(|e| if e.contains("sin respuesta") { format!("{e}\n{tail}") } else { e })
}

/// Lee stderr: manda `{"event":"pct","value":N}` por cada barra tqdm y devuelve las últimas líneas (para errores).
fn tqdm_progress(stderr: impl Read, on_event: Channel<Value>) -> String {
    let mut last = 0u32;
    let mut tail = Vec::new();
    for chunk in BufReader::new(stderr).split(b'\r') {
        let Ok(chunk) = chunk else { break };
        let text = String::from_utf8_lossy(&chunk);
        for part in text.split('\n') {
            match tqdm_pct(part) {
                Some(n) if n != last => {
                    last = n;
                    let _ = on_event.send(json!({"event": "pct", "value": n}));
                }
                Some(_) => {}
                None if !part.trim().is_empty() => {
                    tail.push(part.to_string());
                    if tail.len() > 20 {
                        tail.remove(0);
                    }
                }
                None => {}
            }
        }
    }
    tail.join("\n")
}

/// " 41%|████      | 11/27 [00:27<...]" -> Some(41)
fn tqdm_pct(line: &str) -> Option<u32> {
    let (head, _) = line.split_once("%|")?;
    head.rsplit(' ').next()?.parse().ok()
}

/// Lee un fichero de /data/stemlab (audio, partituras) como bytes crudos (ArrayBuffer en JS).
/// El asset protocol de WebKitGTK se atasca con WAVs grandes; el IPC binario no.
#[tauri::command]
fn read_file(path: String) -> Result<tauri::ipc::Response, String> {
    let path = std::fs::canonicalize(&path).map_err(|e| format!("{path}: {e}"))?;
    if !path.starts_with("/data/stemlab") {
        return Err(format!("fuera de /data/stemlab: {}", path.display()));
    }
    std::fs::read(&path)
        .map(tauri::ipc::Response::new)
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// Escribe bytes crudos (cuerpo de la petición) en `path`, dentro de /data/stemlab.
/// La ruta va en la cabecera `path` codificada con encodeURIComponent.
#[tauri::command]
fn write_file(request: tauri::ipc::Request) -> Result<(), String> {
    let tauri::ipc::InvokeBody::Raw(data) = request.body() else {
        return Err("se esperaban bytes".into());
    };
    let raw = request.headers().get("path").and_then(|v| v.to_str().ok()).ok_or("falta la cabecera path")?;
    let path = PathBuf::from(percent_decode(raw));
    let dir = path.parent().and_then(|d| std::fs::canonicalize(d).ok()).ok_or("carpeta inexistente")?;
    if !dir.starts_with("/data/stemlab") {
        return Err(format!("fuera de /data/stemlab: {}", path.display()));
    }
    std::fs::write(dir.join(path.file_name().ok_or("sin nombre")?), data).map_err(|e| e.to_string())
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match (b[i], s.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok())) {
            (b'%', Some(v)) => {
                out.push(v);
                i += 3;
            }
            (c, _) => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Errores del frontend a la terminal.
#[tauri::command]
fn log(msg: String) {
    eprintln!("[web] {msg}");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![engine, cancel, cancel_all, read_file, write_file, log])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_percent() {
        assert_eq!(percent_decode("%2Fdata%2Fstemlab%2F%C3%B1.pdf"), "/data/stemlab/ñ.pdf");
        assert_eq!(percent_decode("a%zz"), "a%zz");
    }

    #[test]
    fn parses_tqdm() {
        assert_eq!(tqdm_pct(" 41%|████      | 11/27 [00:27<00:06,  2.31it/s]"), Some(41));
        assert_eq!(tqdm_pct("100%|██████████| 27/27"), Some(100));
        assert_eq!(tqdm_pct("MIOpen(HIP): Warning"), None);
    }

    #[test]
    fn engine_list_roundtrip() {
        let events = std::sync::Arc::new(std::sync::Mutex::new(0));
        let n = events.clone();
        let ch = Channel::new(move |_| {
            *n.lock().unwrap() += 1;
            Ok(())
        });
        let done = run_engine("t1".into(), vec!["list".into()], ch).expect("engine list");
        assert!(done["songs"].is_array());
        assert_eq!(*events.lock().unwrap(), 1);
        let err = run_engine("t2".into(), vec!["nope".into()], Channel::new(|_| Ok(()))).unwrap_err();
        assert!(err.contains("subcomando desconocido"), "{err}");

        // cancelar a mitad: la búsqueda tarda segundos en red, la matamos antes
        let h = std::thread::spawn(|| run_engine("t3".into(), vec!["search".into(), "test".into()], Channel::new(|_| Ok(()))));
        std::thread::sleep(std::time::Duration::from_millis(300));
        cancel("t3".into());
        assert_eq!(h.join().unwrap().unwrap_err(), "cancelado");
        assert!(read_file("/etc/passwd".into()).is_err());
        assert!(read_file("/data/stemlab/../../etc/passwd".into()).is_err());
    }
}
