use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use serde::Serialize;
use tauri::{Emitter, Manager, State, Runtime};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use std::io::{BufRead, BufReader, Read};
use std::thread;
use std::collections::HashMap;
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Serialize)]
pub struct ModelFile {
    pub name: String,
    pub publisher: String,
    pub model_family: String,
    pub size_bytes: u64,
    pub path: String,
    pub arch: String,
    pub params: String,
    pub max_context: u32,
    pub mmproj_paths: Vec<String>,
}

fn read_gguf_metadata(path: &str) -> HashMap<String, String> {
    let mut meta = HashMap::new();
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return meta,
    };
    let mut reader = std::io::BufReader::new(file);
    
    let mut magic = [0u8; 4];
    if std::io::Read::read_exact(&mut reader, &mut magic).is_err() { return meta; }
    if &magic != b"GGUF" { return meta; }
    
    let mut version = [0u8; 4];
    if std::io::Read::read_exact(&mut reader, &mut version).is_err() { return meta; }
    
    let mut buf8 = [0u8; 8];
    if std::io::Read::read_exact(&mut reader, &mut buf8).is_err() { return meta; }
    
    if std::io::Read::read_exact(&mut reader, &mut buf8).is_err() { return meta; }
    let kv_count = u64::from_le_bytes(buf8);
    
    for _ in 0..kv_count {
        let mut key_len_buf = [0u8; 8];
        if std::io::Read::read_exact(&mut reader, &mut key_len_buf).is_err() { break; }
        let key_len = u64::from_le_bytes(key_len_buf) as usize;
        if key_len > 256 { break; }
        let mut key_buf = vec![0u8; key_len];
        if std::io::Read::read_exact(&mut reader, &mut key_buf).is_err() { break; }
        let key = String::from_utf8_lossy(&key_buf).to_string();
        
        let mut type_buf = [0u8; 4];
        if std::io::Read::read_exact(&mut reader, &mut type_buf).is_err() { break; }
        let val_type = u32::from_le_bytes(type_buf);
        
        match val_type {
            4 => {
                let mut v = [0u8; 4];
                if std::io::Read::read_exact(&mut reader, &mut v).is_err() { break; }
                meta.insert(key, u32::from_le_bytes(v).to_string());
            }
            5 => {
                let mut v = [0u8; 4];
                if std::io::Read::read_exact(&mut reader, &mut v).is_err() { break; }
                meta.insert(key, i32::from_le_bytes(v).to_string());
            }
            6 => {
                let mut v = [0u8; 4];
                if std::io::Read::read_exact(&mut reader, &mut v).is_err() { break; }
                meta.insert(key, f32::from_le_bytes(v).to_string());
            }
            7 => {
                let mut v = [0u8; 1];
                if std::io::Read::read_exact(&mut reader, &mut v).is_err() { break; }
                meta.insert(key, v[0].to_string());
            }
            8 => {
                let mut slen_buf = [0u8; 8];
                if std::io::Read::read_exact(&mut reader, &mut slen_buf).is_err() { break; }
                let slen = u64::from_le_bytes(slen_buf) as usize;
                if slen > 4096 { break; }
                let mut sbuf = vec![0u8; slen];
                if std::io::Read::read_exact(&mut reader, &mut sbuf).is_err() { break; }
                meta.insert(key, String::from_utf8_lossy(&sbuf).to_string());
            }
            9 => break,
            10 => {
                let mut v = [0u8; 8];
                if std::io::Read::read_exact(&mut reader, &mut v).is_err() { break; }
                meta.insert(key, u64::from_le_bytes(v).to_string());
            }
            11 => {
                let mut v = [0u8; 8];
                if std::io::Read::read_exact(&mut reader, &mut v).is_err() { break; }
                meta.insert(key, i64::from_le_bytes(v).to_string());
            }
            12 => {
                let mut v = [0u8; 8];
                if std::io::Read::read_exact(&mut reader, &mut v).is_err() { break; }
                meta.insert(key, f64::from_le_bytes(v).to_string());
            }
            _ => break,
        }
    }
    
    meta
}

#[tauri::command]
fn scan_models(models_path: String) -> Vec<ModelFile> {
    let base = PathBuf::from(&models_path);
    let mut models = Vec::new();

    let publishers = match fs::read_dir(&base) {
        Ok(d) => d,
        Err(_) => return models,
    };

    for publisher_entry in publishers.flatten() {
        let publisher_path = publisher_entry.path();
        if !publisher_path.is_dir() { continue; }
        let publisher = publisher_entry.file_name().to_string_lossy().to_string();

        let model_families = match fs::read_dir(&publisher_path) {
            Ok(d) => d,
            Err(_) => continue,
        };

        for family_entry in model_families.flatten() {
            let family_path = family_entry.path();
            if !family_path.is_dir() { continue; }
            let model_family = family_entry.file_name().to_string_lossy().to_string();

            let files = match fs::read_dir(&family_path) {
                Ok(d) => d,
                Err(_) => continue,
            };

            let mut mmproj_files: Vec<String> = Vec::new();
            let mut gguf_files: Vec<(String, u64)> = Vec::new();

            for file_entry in files.flatten() {
                let file_path = file_entry.path();
                let file_name = file_entry.file_name().to_string_lossy().to_string();
                if !file_name.ends_with(".gguf") { continue; }

                let size_bytes = file_entry.metadata().map(|m| m.len()).unwrap_or(0);

                if file_name.contains("mmproj") {
                    mmproj_files.push(file_path.to_string_lossy().to_string());
                } else {
                    gguf_files.push((file_path.to_string_lossy().to_string(), size_bytes));
                }
            }

            for (file_path, size_bytes) in gguf_files {
                let file_name = std::path::Path::new(&file_path)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                let meta = read_gguf_metadata(&file_path);
                let arch = meta.get("general.architecture").cloned().unwrap_or_default();
                let params = meta.get("general.parameter_count")
                    .and_then(|v| v.parse::<u64>().ok())
                    .map(|n| format!("{:.0}B", n as f64 / 1_000_000_000.0))
                    .or_else(|| meta.get("general.size_label").cloned())
                    .unwrap_or_default();
                let ctx_key = format!("{}.context_length", arch);
                let max_context = meta.get(&ctx_key)
                    .or_else(|| meta.get("llama.context_length"))
                    .or_else(|| meta.get("model.context_length"))
                    .and_then(|v| v.parse::<u32>().ok())
                    .unwrap_or(0);

                models.push(ModelFile {
                    name: file_name,
                    publisher: publisher.clone(),
                    model_family: model_family.clone(),
                    size_bytes,
                    path: file_path,
                    arch,
                    params,
                    max_context,
                    mmproj_paths: mmproj_files.clone(),
                });
            }
        }
    }

    models
}

pub struct ServerProcess(pub Mutex<Option<std::process::Child>>);

#[tauri::command]
fn load_model(
    app: tauri::AppHandle,
    state: State<ServerProcess>,
    llama_path: String,
    model_path: String,
    gpu_layers: i32,
    context_length: i32,
    cpu_threads: i32,
    eval_batch: i32,
    physical_batch: i32,
    flash_attention: bool,
    spec_type: String,
    draft_model_path: String,
    max_draft_tokens: i32,
    draft_probability: f32,
    k_cache_quant: String,
    v_cache_quant: String,
    port: i32,
    host: String,
    alias: String,
    threads_http: i32,
    no_warmup: bool,
    sleep_idle: i32,
    reasoning_preserve: bool,
    fit: String,
    reasoning_budget: i32,
    reasoning_effort: String,
    parallel: i32,
    mlock: bool,
    mmap: bool,
    kv_unified: bool,
    n_cpu_moe: i32,
    vision_enabled: bool,
    mmproj_path: String,
    system_prompt: String,
) -> Result<String, String> {
    let mut child_lock = state.0.lock().unwrap();

    if let Some(mut child) = child_lock.take() {
        let _ = child.kill();
    }

    let mut cmd = Command::new(&llama_path);
    cmd.arg("-m").arg(&model_path)
       .arg("-ngl").arg(gpu_layers.to_string())
       .arg("-c").arg(context_length.to_string())
       .arg("-t").arg(cpu_threads.to_string())
       .arg("-b").arg(eval_batch.to_string())
       .arg("-ub").arg(physical_batch.to_string())
       .arg("--port").arg(port.to_string())
       .arg("--host").arg(&host)
       .arg("--threads-http").arg(threads_http.to_string())
       .arg("--cache-prompt")
       .arg("--props")
       .arg("--jinja")
        .arg("-np").arg(parallel.to_string())
        .arg("-kvo")
       .arg("--fit").arg(&fit)
        .arg("--load-mode").arg(if mmap { "mmap" } else { "none" })
       .stdout(Stdio::inherit())
       .stderr(Stdio::piped());

    if !alias.is_empty() {
        cmd.arg("--alias").arg(&alias);
    }

    if no_warmup {
        cmd.arg("--no-warmup");
    }

    if sleep_idle > 0 {
        cmd.arg("--sleep-idle-seconds").arg(sleep_idle.to_string());
    }

    if reasoning_preserve {
        cmd.arg("--reasoning-preserve");
    }

    if flash_attention {
        cmd.arg("-fa").arg("on");
    }

    if spec_type == "MTP" {
        cmd.arg("--spec-type").arg("draft-mtp")
           .arg("--spec-draft-n-max").arg(max_draft_tokens.to_string());
    }

    if spec_type == "Draft" && !draft_model_path.is_empty() {
        cmd.arg("--spec-type").arg("draft-simple")
           .arg("-md").arg(&draft_model_path)
           .arg("--spec-draft-n-max").arg(max_draft_tokens.to_string())
           .arg("--spec-draft-ngl").arg("99");
    }

    if k_cache_quant != "F16" {
        cmd.arg("--cache-type-k").arg(k_cache_quant.to_lowercase());
    }
    if v_cache_quant != "F16" {
        cmd.arg("--cache-type-v").arg(v_cache_quant.to_lowercase());
    }

    cmd.arg("--reasoning-budget").arg(reasoning_budget.to_string());

    if reasoning_effort != "default" {
        cmd.arg("--reasoning-effort").arg(&reasoning_effort);
    }

    if mlock {
        cmd.arg("--mlock");
    }

    if kv_unified {
        cmd.arg("--kv-unified");
    } else {
        cmd.arg("--no-kv-unified");
    }

    if n_cpu_moe > 0 {
        cmd.arg("--n-cpu-moe").arg(n_cpu_moe.to_string());
    }

    if vision_enabled && !mmproj_path.is_empty() {
        cmd.arg("--mmproj").arg(&mmproj_path);
    }

    if !system_prompt.is_empty() {
        cmd.arg("--system-prompt").arg(&system_prompt);
    }

    let mut child = cmd
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("Failed to spawn: {}", e))?;

    let stderr = child.stderr.take().unwrap();
    let app_handle = app.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line) = line {
                let _ = app_handle.emit("llama-log", line);
            }
        }
    });

    *child_lock = Some(child);
    Ok(format!("Started: {}", model_path))
}

#[tauri::command]
fn stop_model(state: State<ServerProcess>) -> Result<(), String> {
    let mut child_lock = state.0.lock().unwrap();
    if let Some(mut child) = child_lock.take() {
        child.kill().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn save_window_state<R: Runtime>(app: tauri::AppHandle<R>) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("no window")?;
    let maximized = window.is_maximized().map_err(|e| e.to_string())?;
    let size = window.outer_size().map_err(|e| e.to_string())?;
    let pos = window.outer_position().map_err(|e| e.to_string())?;
    
    let config_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
    let path = config_dir.join("window.json");
    let data = format!(
        r#"{{"x":{},"y":{},"w":{},"h":{},"maximized":{}}}"#,
        pos.x, pos.y, size.width, size.height, maximized
    );
    std::fs::write(path, data).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn load_window_state<R: Runtime>(app: tauri::AppHandle<R>) -> Result<(), String> {
    let config_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let path = config_dir.join("window.json");
    let data = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    let window = app.get_webview_window("main").ok_or("no window")?;
    
    if v["maximized"].as_bool().unwrap_or(false) {
        window.maximize().map_err(|e| e.to_string())?;
    } else {
        window.set_size(tauri::Size::Physical(tauri::PhysicalSize {
            width: v["w"].as_u64().unwrap_or(1200) as u32,
            height: v["h"].as_u64().unwrap_or(800) as u32,
        })).map_err(|e| e.to_string())?;
        window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: v["x"].as_i64().unwrap_or(100) as i32,
            y: v["y"].as_i64().unwrap_or(100) as i32,
        })).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn get_cpu_threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            let _tray = TrayIconBuilder::new()
                .tooltip("LlamaStudio")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.set_focus();
                            let _ = window.show();
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.set_focus();
                            let _ = window.show();
                        }
                    }
                })
                .build(app)?;
            Ok(())
        })
        .manage(ServerProcess(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![scan_models, load_model, stop_model, save_window_state, load_window_state, get_cpu_threads])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}