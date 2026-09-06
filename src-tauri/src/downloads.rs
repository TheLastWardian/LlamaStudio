use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::Emitter;

use crate::hf::FileGroup;

type JobId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskState { Queued, Downloading, Paused, Completed, Failed }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobState { Queued, Downloading, Paused, Completed, Failed }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTask {
    pub task_id: String,
    pub path_in_repo: String,
    pub file_name: String,
    pub total_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    pub group: FileGroup,
    pub downloaded_bytes: u64,
    pub state: TaskState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub retry_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadJob {
    pub job_id: JobId,
    pub repo_owner: String,
    pub repo_name: String,
    pub files: Vec<DownloadTask>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskView {
    pub task_id: String,
    pub file_name: String,
    pub path_in_repo: String,
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub group: FileGroup,
    pub state: TaskState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub retry_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    pub retry_in_sec: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobView {
    pub job_id: JobId,
    pub repo_owner: String,
    pub repo_name: String,
    pub files: Vec<TaskView>,
    pub created_at: i64,
    pub state: JobState,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    job_id: String,
    task_id: String,
    downloaded_bytes: u64,
    total_bytes: u64,
    speed_bps: u64,
    eta_sec: i64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct StateEvent {
    job_id: String,
    state: JobState,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_in_sec: Option<i64>,
}

#[derive(Serialize, Deserialize)]
struct PersistedState {
    jobs: Vec<DownloadJob>,
}

// S4: un archivo del batch (wire camelCase, igual que el resto de los structs de S2)
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileSpec {
    pub path: String,
    pub size: u64,
    pub sha256: Option<String>,
    pub group: FileGroup,
}

// S4: progreso desacoplado del mutex de jobs. Lo escribe la task de streaming
// (~4 veces/s por task) y lo lee job_view; nunca bloquea la descarga.
pub struct ProgressState {
    pub downloaded: AtomicU64,
    pub speed_bps: AtomicU64,
    pub retry_in_sec: AtomicI64,
}

// Campos Arc<Mutex<..>> para clonar un handle (Arc<ManagerCore>) hacia la tokio task
// sin tocar el state de Tauri. El lock solo se toma en cambios de estado (no en la hot path).
pub struct ManagerCore {
    jobs: Arc<Mutex<HashMap<JobId, DownloadJob>>>,
    active: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    path: Arc<Mutex<Option<PathBuf>>>,
    // S5: gate de conteo activo (reemplaza el Semaphore de S4; respeta el límite al bajar)
    running: Arc<AtomicU32>,
    max_parallelism: Arc<AtomicU32>,
    // S5: settings de retry/cancel (el comando los escribe antes de drain_queue)
    auto_retry: Arc<AtomicBool>,
    max_retries: Arc<AtomicU32>,
    keep_part_on_cancel: Arc<AtomicBool>,
    // S4: cola FIFO global de pendientes "job_id/task_id"
    queue: Arc<Mutex<VecDeque<String>>>,
    // S4: progreso vivo por task "job_id/task_id" (atomics → hot path sin lock de jobs)
    progress: Arc<Mutex<HashMap<String, Arc<ProgressState>>>>,
    // S4: models_path del último comando (lo necesita el scheduler; la cola solo guarda keys)
    models_path: Arc<Mutex<Option<String>>>,
}

#[derive(Clone)]
pub struct DownloadManager(pub Arc<ManagerCore>);

impl DownloadManager {
    pub fn new() -> Self {
        Self(Arc::new(ManagerCore::new()))
    }
}

impl ManagerCore {
    fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(HashMap::new())),
            active: Arc::new(Mutex::new(HashMap::new())),
            path: Arc::new(Mutex::new(None)),
            running: Arc::new(AtomicU32::new(0)),
            max_parallelism: Arc::new(AtomicU32::new(2)),
            auto_retry: Arc::new(AtomicBool::new(true)),
            max_retries: Arc::new(AtomicU32::new(5)),
            keep_part_on_cancel: Arc::new(AtomicBool::new(false)),
            queue: Arc::new(Mutex::new(VecDeque::new())),
            progress: Arc::new(Mutex::new(HashMap::new())),
            models_path: Arc::new(Mutex::new(None)),
        }
    }

    fn json_path(&self) -> Option<PathBuf> {
        self.path.lock().unwrap().as_ref().map(|p| p.join("downloads.json"))
    }

    pub fn init_path(&self, dir: PathBuf) {
        *self.path.lock().unwrap() = Some(dir);
    }

    fn persist_map(&self, jobs: &HashMap<JobId, DownloadJob>) {
        let Some(p) = self.json_path() else { return; };
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let state = PersistedState { jobs: jobs.values().cloned().collect() };
        if let Ok(s) = serde_json::to_string_pretty(&state) {
            let _ = std::fs::write(p, s);
        }
    }

    fn persist(&self) {
        let jobs = self.jobs.lock().unwrap();
        self.persist_map(&jobs);
    }

    fn add_job(&self, job: DownloadJob) {
        let mut jobs = self.jobs.lock().unwrap();
        jobs.insert(job.job_id.clone(), job);
        self.persist_map(&jobs);
    }

    fn set_task_state(&self, job_id: &str, task_id: &str, state: TaskState, error: Option<String>, error_code: Option<String>) -> JobState {
        let mut jobs = self.jobs.lock().unwrap();
        let mut derived = JobState::Queued;
        if let Some(j) = jobs.get_mut(job_id) {
            if let Some(t) = j.files.iter_mut().find(|t| t.task_id == task_id) {
                t.state = state;
                t.error = error;
                t.error_code = error_code;
                if state == TaskState::Downloading {
                    t.retry_count = 0;
                }
                derived = derive_state(&j.files);
            }
        }
        self.persist_map(&jobs);
        derived
    }

    fn task_state(&self, job_id: &str, task_id: &str) -> Option<TaskState> {
        let jobs = self.jobs.lock().unwrap();
        jobs.get(job_id)?.files.iter().find(|t| t.task_id == task_id).map(|t| t.state)
    }

    fn task_states(&self, job_id: &str) -> Vec<(String, TaskState)> {
        let jobs = self.jobs.lock().unwrap();
        jobs.get(job_id)
            .map(|j| j.files.iter().map(|t| (t.task_id.clone(), t.state)).collect())
            .unwrap_or_default()
    }

    fn register_active(&self, key: &str, flag: Arc<AtomicBool>) {
        self.active.lock().unwrap().insert(key.to_string(), flag);
    }

    fn unregister_active(&self, key: &str) {
        self.active.lock().unwrap().remove(key);
    }

    fn register_progress(&self, key: &str, ps: Arc<ProgressState>) {
        self.progress.lock().unwrap().insert(key.to_string(), ps);
    }

    fn unregister_progress(&self, key: &str) {
        self.progress.lock().unwrap().remove(key);
    }

    fn job_view(&self, job_id: &str) -> Option<JobView> {
        let view = {
            let jobs = self.jobs.lock().unwrap();
            jobs.get(job_id).map(Self::view_from)?
        };
        let progress = self.progress.lock().unwrap();
        Some(view_live(view, &progress))
    }

    pub fn list(&self, models_path: Option<&str>) -> Vec<JobView> {
        self.ensure_loaded();
        if let Some(mp) = models_path {
            self.reconcile(mp);
        }
        let jobs = self.jobs.lock().unwrap();
        jobs.values().map(Self::view_from).collect()
    }

    fn ensure_loaded(&self) {
        let mut jobs = self.jobs.lock().unwrap();
        if !jobs.is_empty() {
            return;
        }
        let Some(p) = self.json_path() else { return; };
        let Ok(data) = std::fs::read_to_string(p) else { return; };
        if let Ok(parsed) = serde_json::from_str::<PersistedState>(&data) {
            for job in parsed.jobs {
                jobs.insert(job.job_id.clone(), job);
            }
        }
    }

    // Reconcilia contra el disco (list_downloads lo llama con models_path). Usa solo
    // tamaño (sin re-hasar archivos enormes): un target que existe ya pasó por sha256.
    fn reconcile(&self, models_path: &str) {
        {
            let mut jobs = self.jobs.lock().unwrap();
            for job in jobs.values_mut() {
                for task in job.files.iter_mut() {
                    let target = target_path(models_path, &job.repo_owner, &job.repo_name, &task.path_in_repo);
                    let complete = std::fs::metadata(&target).map(|m| m.len() == task.total_bytes).unwrap_or(false);
                    match task.state {
                        TaskState::Completed | TaskState::Failed => {
                            if complete {
                                task.state = TaskState::Completed;
                            } else if task.state == TaskState::Failed && part_path(&target).exists() {
                                task.downloaded_bytes = std::fs::metadata(part_path(&target)).map(|m| m.len()).unwrap_or(0);
                                task.state = TaskState::Paused;
                            }
                        }
                        _ => {
                            task.state = if complete {
                                TaskState::Completed
                            } else if part_path(&target).exists() {
                                task.downloaded_bytes = std::fs::metadata(part_path(&target)).map(|m| m.len()).unwrap_or(0);
                                TaskState::Paused
                            } else {
                                task.downloaded_bytes = 0;
                                TaskState::Queued
                            };
                        }
                    }
                }
            }
        }
        self.persist();
    }

    fn view_from(job: &DownloadJob) -> JobView {
        JobView {
            job_id: job.job_id.clone(),
            repo_owner: job.repo_owner.clone(),
            repo_name: job.repo_name.clone(),
            files: job
                .files
                .iter()
                .map(|t| TaskView {
                    task_id: t.task_id.clone(),
                    file_name: t.file_name.clone(),
                    path_in_repo: t.path_in_repo.clone(),
                    total_bytes: t.total_bytes,
                    downloaded_bytes: t.downloaded_bytes,
                    group: t.group,
                    state: t.state,
                    error: t.error.clone(),
                    retry_count: t.retry_count,
                    error_code: t.error_code.clone(),
                    retry_in_sec: -1,
                })
                .collect(),
            created_at: job.created_at,
            state: derive_state(&job.files),
        }
    }
}

// S4: reemplaza downloaded_bytes por el valor atómico si la task tiene
// entrada viva (monotónico mientras streama; más fresco que el map de jobs).
fn view_live(view: JobView, progress: &HashMap<String, Arc<ProgressState>>) -> JobView {
    let mut files = view.files;
    for f in files.iter_mut() {
        if let Some(ps) = progress.get(&format!("{}/{}", view.job_id, f.task_id)) {
            f.downloaded_bytes = ps.downloaded.load(Ordering::Relaxed);
            f.retry_in_sec = ps.retry_in_sec.load(Ordering::Relaxed);
        }
    }
    JobView { files, ..view }
}

// S4: reemplaza el derive_state de S2. Prioridad: activo → todos completos
// → fallido → pausado → en cola (un task Failed no oculta una descarga en curso).
fn derive_state(tasks: &[DownloadTask]) -> JobState {
    if tasks.is_empty() {
        return JobState::Completed;
    }
    if tasks.iter().any(|t| t.state == TaskState::Downloading) {
        return JobState::Downloading;
    }
    if tasks.iter().all(|t| t.state == TaskState::Completed) {
        return JobState::Completed;
    }
    if tasks.iter().any(|t| t.state == TaskState::Failed) {
        return JobState::Failed;
    }
    if tasks.iter().any(|t| t.state == TaskState::Paused) {
        return JobState::Paused;
    }
    JobState::Queued
}

// ---------- Rutas / URLs / helpers (puras, testeables) ----------

pub fn target_path(models_path: &str, owner: &str, repo: &str, path_in_repo: &str) -> PathBuf {
    let mut p = PathBuf::from(models_path);
    p.push(owner);
    p.push(repo);
    for seg in path_in_repo.split('/') {
        if !seg.is_empty() {
            p.push(seg);
        }
    }
    p
}

pub fn part_path(target: &Path) -> PathBuf {
    let mut s = target.as_os_str().to_os_string();
    s.push(".part");
    PathBuf::from(s)
}

fn resolve_url(owner: &str, repo: &str, path_in_repo: &str) -> String {
    format!(
        "https://huggingface.co/{owner}/{repo}/resolve/main/{path}?download=true",
        path = url_path(path_in_repo)
    )
}

/// Percent-encode por segmento (el slash se preserva). Suficiente para nombres GGUF
/// (`[A-Za-z0-9._-]`); cubre chars reservados si aparecieran.
pub fn url_path(path_in_repo: &str) -> String {
    path_in_repo
        .split('/')
        .map(|seg| {
            let mut out = String::new();
            for b in seg.bytes() {
                match b {
                    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => out.push(b as char),
                    _ => out.push_str(&format!("%{b:02X}")),
                }
            }
            out
        })
        .collect::<Vec<_>>()
        .join("/")
}

pub fn backoff_delay(retry: u32) -> u64 {
    (1u64 << retry.min(6)).min(60)
}

fn hex_lower(bytes: impl AsRef<[u8]>) -> String {
    bytes.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

pub fn sha256_file(path: &Path) -> Option<String> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = f.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(hex_lower(hasher.finalize()))
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

static COUNTER: AtomicU32 = AtomicU32::new(0);
fn next_id(prefix: &str) -> String {
    format!("{prefix}_{:x}{}", now_ms(), COUNTER.fetch_add(1, Ordering::SeqCst))
}

// Cliente propio de descarga: connect_timeout pero SIN timeout de cuerpo
// (un download puede streamear horas). No comparte el client de hf.rs (30 s, para JSON).
fn download_client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent("LlamaStudio/0.1.0")
            .connect_timeout(Duration::from_secs(30))
            .build()
            .expect("reqwest download client")
    })
}

// S5: códigos de error estables que el frontend mapea a i18n (Rust nunca emite texto localizado)
#[derive(Debug, Clone, Copy)]
pub enum ErrorCode {
    RateLimited,
    NotFound,
    Gated,
    DiskFull,
    ShaMismatch,
    Network,
    Cancelled,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::RateLimited => "rate_limited",
            ErrorCode::NotFound => "not_found",
            ErrorCode::Gated => "gated",
            ErrorCode::DiskFull => "disk_full",
            ErrorCode::ShaMismatch => "sha_mismatch",
            ErrorCode::Network => "network",
            ErrorCode::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug)]
enum TaskOutcome {
    Completed,
    Stopped,
    Transient { code: ErrorCode, msg: String },
    Permanent { code: ErrorCode, msg: String },
}

fn classify_net_error(e: &reqwest::Error) -> TaskOutcome {
    let msg = if e.is_timeout() {
        "timeout de red".into()
    } else if e.is_connect() {
        format!("error de conexión: {e}")
    } else {
        format!("error de red: {e}")
    };
    TaskOutcome::Transient { code: ErrorCode::Network, msg }
}

fn http_error_outcome(status: reqwest::StatusCode) -> TaskOutcome {
    match status.as_u16() {
        404 => TaskOutcome::Permanent { code: ErrorCode::NotFound, msg: "el archivo no existe en el repo (404)".into() },
        401 | 403 => TaskOutcome::Permanent { code: ErrorCode::Gated, msg: "repo privado o gated — se requiere token HF (401/403)".into() },
        429 => TaskOutcome::Transient { code: ErrorCode::RateLimited, msg: "rate limit de HF (429)".into() },
        500..=599 => TaskOutcome::Transient { code: ErrorCode::Network, msg: format!("error del servidor (HTTP {status})") },
        other => TaskOutcome::Permanent { code: ErrorCode::Network, msg: format!("error HTTP {other}") },
    }
}

// Una intent de descarga: ya-existe → offset desde el .part → request (416 → restart) → stream.
async fn download_once(
    core: &ManagerCore,
    app: Option<&tauri::AppHandle>,
    job_id: &str,
    task: &DownloadTask,
    url: &str,
    target: &Path,
    part: &Path,
    stop: &AtomicBool,
) -> TaskOutcome {
    // 1) ¿Ya completo? (solo al arrancar: valida sha una vez)
    if let Ok(meta) = std::fs::metadata(target) {
        if meta.len() == task.total_bytes {
            return match &task.sha256 {
                Some(sha) => match sha256_file(target) {
                    Some(h) if h == *sha => TaskOutcome::Completed,
                    Some(h) => {
                        let _ = std::fs::remove_file(target);
                        TaskOutcome::Permanent { code: ErrorCode::ShaMismatch, msg: format!("sha256 de archivo existente inválido: {h}") }
                    }
                    None => TaskOutcome::Completed,
                },
                None => TaskOutcome::Completed,
            };
        }
    }

    // 2) offset = tamaño real del .part en disco
    let mut offset = std::fs::metadata(part).map(|m| m.len()).unwrap_or(0);
    if offset > task.total_bytes {
        let _ = std::fs::remove_file(part);
        offset = 0;
    }

    // 3) request; si el CDN responde 416 (el remoto es más corto) → restart desde 0
    let resp = 'req: loop {
        let mut r = download_client().get(url);
        if offset > 0 {
            r = r.header(reqwest::header::RANGE, format!("bytes={offset}-"));
        }
        let res = match r.send().await {
            Ok(res) => res,
            Err(e) => return classify_net_error(&e),
        };
        let status = res.status();
        if status.as_u16() == 416 {
            let _ = std::fs::remove_file(part);
            offset = 0;
            continue 'req;
        }
        if !status.is_success() {
            return http_error_outcome(status);
        }
        break 'req res;
    };

    do_stream(core, app, job_id, task, resp, target, part, offset, stop).await
}

async fn do_stream(
    core: &ManagerCore,
    app: Option<&tauri::AppHandle>,
    job_id: &str,
    task: &DownloadTask,
    resp: reqwest::Response,
    target: &Path,
    part: &Path,
    start_offset: u64,
    stop: &AtomicBool,
) -> TaskOutcome {
    let mut file = match std::fs::OpenOptions::new().create(true).append(true).open(part) {
        Ok(f) => f,
        Err(e) => return TaskOutcome::Permanent { code: ErrorCode::DiskFull, msg: format!("no se pudo abrir el .part: {e}") },
    };

    let incremental = start_offset == 0;
    let mut hasher = Sha256::new();
    let mut downloaded = start_offset;
    let mut last_emit = Instant::now();
    let mut last_chunk_time = Instant::now();
    let mut ema_speed: f64 = 0.0;

    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        if stop.load(Ordering::SeqCst) {
            return TaskOutcome::Stopped;
        }
        let bytes = match chunk {
            Ok(b) => b,
            Err(e) => return TaskOutcome::Transient { code: ErrorCode::Network, msg: format!("error en el stream: {e}") },
        };
        if bytes.is_empty() {
            continue;
        }
        if let Err(e) = file.write_all(&bytes) {
            return TaskOutcome::Permanent { code: ErrorCode::DiskFull, msg: format!("error al escribir (¿disco lleno?): {e}") };
        }
        if incremental {
            hasher.update(&bytes);
        }
        downloaded += bytes.len() as u64;

        if last_emit.elapsed() >= Duration::from_millis(250) {
            let now = Instant::now();
            let dt = last_chunk_time.elapsed().as_secs_f64().max(1e-6);
            let inst = bytes.len() as f64 / dt;
            ema_speed = if ema_speed == 0.0 { inst } else { ema_speed * 0.9 + inst * 0.1 };
            last_chunk_time = now;
            last_emit = now;
            let remaining = task.total_bytes.saturating_sub(downloaded);
            let eta = if ema_speed > 0.0 { (remaining as f64 / ema_speed) as i64 } else { -1 };

            // S4: atomics vivos (lock-free; nunca se toca el mutex de jobs en la hot path)
            let key = format!("{job_id}/{}", task.task_id);
            if let Some(ps) = core.progress.lock().unwrap().get(&key) {
                ps.downloaded.store(downloaded, Ordering::Relaxed);
                ps.speed_bps.store(ema_speed as u64, Ordering::Relaxed);
            }

            if let Some(app) = app {
                let _ = app.emit(
                    "download-progress",
                    ProgressEvent {
                        job_id: job_id.to_string(),
                        task_id: task.task_id.clone(),
                        downloaded_bytes: downloaded,
                        total_bytes: task.total_bytes,
                        speed_bps: ema_speed as u64,
                        eta_sec: eta,
                    },
                );
            }
        }
    }

    let _ = file.flush();
    drop(file);

    // 4) sha256: incremental si empezó en 0; si fue reanudación, re-lectura completa
    if let Some(sha) = &task.sha256 {
        let actual = if incremental {
            hex_lower(hasher.finalize())
        } else {
            match sha256_file(part) {
                Some(h) => h,
                None => return TaskOutcome::Permanent { code: ErrorCode::ShaMismatch, msg: "no se pudo calcular el sha256 final".into() },
            }
        };
        if actual != *sha {
            let _ = std::fs::remove_file(part);
            return TaskOutcome::Permanent { code: ErrorCode::ShaMismatch, msg: format!("sha256 mismatch: esperado {sha}, obtenido {actual}") };
        }
    }

    // 5) rename con retry (bloqueo Windows / antivirus)
    match rename_with_retry(part, target).await {
        Ok(()) => TaskOutcome::Completed,
        Err(e) => TaskOutcome::Permanent { code: ErrorCode::Network, msg: format!("no se pudo renombrar a destino: {e}") },
    }
}

async fn rename_with_retry(from: &Path, to: &Path) -> std::io::Result<()> {
    let mut attempt = 0;
    loop {
        match std::fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) => {
                attempt += 1;
                if attempt >= 3 {
                    return Err(e);
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    }
}

// Loop con retry por task: Downloading → (Completed | Stopped | Failed | Transient→backoff).
async fn run_task(
    app: tauri::AppHandle,
    core: Arc<ManagerCore>,
    job_id: JobId,
    models_path: String,
    owner: String,
    repo: String,
    task: DownloadTask,
    stop: Arc<AtomicBool>,
    auto_retry: bool,
    max_retries: u32,
    keep_part_on_cancel: bool,
) {
    let key = format!("{job_id}/{}", task.task_id);
    core.register_active(&key, Arc::clone(&stop));

    let target = target_path(&models_path, &owner, &repo, &task.path_in_repo);
    if let Some(parent) = target.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let part = part_path(&target);
    let url = resolve_url(&owner, &repo, &task.path_in_repo);

    // S4: el progreso vivo arranca en el offset real de reanudación (tamaño del .part)
    let initial = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0).min(task.total_bytes);
    core.register_progress(&key, Arc::new(ProgressState {
        downloaded: AtomicU64::new(initial),
        speed_bps: AtomicU64::new(0),
        retry_in_sec: AtomicI64::new(-1),
    }));

    let mut retry = 0u32;
    loop {
        if stop.load(Ordering::SeqCst) {
            finish_stopped(&core, &app, &job_id, &task, &part, keep_part_on_cancel);
            break;
        }

        let st = core.set_task_state(&job_id, &task.task_id, TaskState::Downloading, None, None);
        emit_state(Some(&app), &job_id, st, None, None);

        let outcome = download_once(core.as_ref(), Some(&app), &job_id, &task, &url, &target, &part, &stop).await;

        match outcome {
            TaskOutcome::Completed => {
                let st = core.set_task_state(&job_id, &task.task_id, TaskState::Completed, None, None);
                emit_state(Some(&app), &job_id, st, None, None);
                break;
            }
            TaskOutcome::Stopped => {
                finish_stopped(&core, &app, &job_id, &task, &part, keep_part_on_cancel);
                break;
            }
            TaskOutcome::Permanent { code, msg } => {
                let st = core.set_task_state(&job_id, &task.task_id, TaskState::Failed, Some(msg), Some(code.as_str().to_string()));
                emit_state(Some(&app), &job_id, st, None, None);
                break;
            }
            TaskOutcome::Transient { code, msg } => {
                // S5: auto_retry = off → el primer error transitorio es Failed (sin retry)
                if !auto_retry {
                    let st = core.set_task_state(&job_id, &task.task_id, TaskState::Failed, Some(msg.clone()), Some(code.as_str().to_string()));
                    emit_state(Some(&app), &job_id, st, None, None);
                    break;
                }
                retry += 1;
                if retry > max_retries {
                    let msg = format!("{msg} (tras {max_retries} reintentos)");
                    let st = core.set_task_state(&job_id, &task.task_id, TaskState::Failed, Some(msg), Some(code.as_str().to_string()));
                    emit_state(Some(&app), &job_id, st, None, None);
                    break;
                }
                let delay_sec = backoff_delay(retry);
                if let Some(ps) = core.progress.lock().unwrap().get(&key) {
                    ps.retry_in_sec.store(delay_sec as i64, Ordering::Relaxed);
                }
                core.set_task_state(&job_id, &task.task_id, TaskState::Queued, Some(msg.clone()), None);
                let st = core.job_view(&job_id).map(|v| v.state).unwrap_or(JobState::Queued);
                emit_state(Some(&app), &job_id, st, Some(msg.clone()), Some(delay_sec as i64));

                // S5: el backoff de S2 comparaba ms contra segundos (~100 ms); ahora delay*1000 + countdown 1/s
                let delay_ms = delay_sec * 1000;
                let mut waited_ms = 0u64;
                let mut last_tick = 0u64;
                let stopped = loop {
                    if stop.load(Ordering::SeqCst) {
                        break true;
                    }
                    if waited_ms >= delay_ms {
                        break false;
                    }
                    if waited_ms / 1000 > last_tick {
                        last_tick = waited_ms / 1000;
                        let remaining = (delay_ms - waited_ms) / 1000;
                        if let Some(ps) = core.progress.lock().unwrap().get(&key) {
                            ps.retry_in_sec.store(remaining as i64, Ordering::Relaxed);
                        }
                        emit_state(Some(&app), &job_id, JobState::Queued, Some(msg.clone()), Some(remaining as i64));
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    waited_ms += 100;
                };
                if let Some(ps) = core.progress.lock().unwrap().get(&key) {
                    ps.retry_in_sec.store(-1, Ordering::Relaxed);
                }
                if stopped {
                    finish_stopped(&core, &app, &job_id, &task, &part, keep_part_on_cancel);
                    break;
                }
                // else: reintenta (loop) reanudando desde el .part
            }
        }
    }
    core.unregister_active(&key);
    core.unregister_progress(&key);
}

// Pausa (conserva .part) vs cancel (borra .part salvo keep_part): se distingue por el
// estado que el comando fijó antes de levantar la bandera de stop.
fn finish_stopped(
    core: &ManagerCore,
    app: &tauri::AppHandle,
    job_id: &str,
    task: &DownloadTask,
    part: &Path,
    keep_part_on_cancel: bool,
) {
    let state = core.task_state(job_id, &task.task_id).unwrap_or(TaskState::Paused);
    if !matches!(state, TaskState::Paused) && !keep_part_on_cancel {
        let _ = std::fs::remove_file(part);
    }
    let st = core.job_view(job_id).map(|v| v.state).unwrap_or(JobState::Paused);
    emit_state(Some(app), job_id, st, None, None);
}

fn emit_state(app: Option<&tauri::AppHandle>, job_id: &str, state: JobState, error: Option<String>, retry_in_sec: Option<i64>) {
    if let Some(app) = app {
        let _ = app.emit("download-state", StateEvent { job_id: job_id.into(), state, error, retry_in_sec });
    }
}

// S5: helper compartido por start_downloads y resume_download (patrón models_path)
fn apply_download_settings(core: &ManagerCore, parallelism: u32, auto_retry: bool, max_retries: u32, keep_part_on_cancel: bool) {
    core.max_parallelism.store(parallelism.clamp(1, 4), Ordering::Relaxed);
    core.auto_retry.store(auto_retry, Ordering::Relaxed);
    core.max_retries.store(max_retries.min(10), Ordering::Relaxed);
    core.keep_part_on_cancel.store(keep_part_on_cancel, Ordering::Relaxed);
}

// ---------- Comandos ----------

#[tauri::command]
pub fn start_downloads(
    app: tauri::AppHandle,
    state: tauri::State<DownloadManager>,
    models_path: String,
    owner: String,
    repo: String,
    files: Vec<FileSpec>,
    parallelism: u32,
    auto_retry: bool,
    max_retries: u32,
    keep_part_on_cancel: bool,
) -> Result<JobView, String> {
    let core = state.0.clone();
    if files.is_empty() {
        return Err("sin archivos para descargar".into());
    }

    // Dedup (la guarda single-file de S2 adaptada al batch): si un job activo
    // ya contiene alguno de estos paths (no Completed/Failed) → devolver ese job.
    {
        let jobs = core.jobs.lock().unwrap();
        let wanted: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        if let Some(existing) = jobs.values().find(|j| {
            j.repo_owner == owner
                && j.repo_name == repo
                && j.files.iter().any(|t| {
                    wanted.contains(&t.path_in_repo.as_str())
                        && !matches!(t.state, TaskState::Completed | TaskState::Failed)
                })
        }) {
            return Ok(ManagerCore::view_from(existing));
        }
    }

    *core.models_path.lock().unwrap() = Some(models_path);
    apply_download_settings(&core, parallelism, auto_retry, max_retries, keep_part_on_cancel);
    let job_id = next_id("job");
    let mut queued: Vec<String> = Vec::with_capacity(files.len());
    let job = DownloadJob {
        job_id: job_id.clone(),
        repo_owner: owner,
        repo_name: repo,
        files: files
            .into_iter()
            .map(|f| {
                let task_id = next_id("task");
                queued.push(format!("{job_id}/{task_id}"));
                DownloadTask {
                    task_id,
                    path_in_repo: f.path.clone(),
                    file_name: f.path.rsplit('/').next().unwrap_or(&f.path).to_string(),
                    total_bytes: f.size,
                    sha256: f.sha256,
                    group: f.group,
                    downloaded_bytes: 0,
                    state: TaskState::Queued,
                    error: None,
                    retry_count: 0,
                    error_code: None,
                }
            })
            .collect(),
        created_at: now_ms(),
    };
    core.add_job(job);

    {
        let mut q = core.queue.lock().unwrap();
        for key in queued {
            q.push_back(key);
        }
    }
    drain_queue(app, core.clone());

    Ok(core.job_view(&job_id).unwrap())
}

// S5: scheduler sobre gate de conteo activo. Spawnea mientras `running < max_parallelism`
// y hay cola. El fetch_add va BAJO el lock de la cola (nunca se supera el límite). Al
// terminar cada task: fetch_sub + re-pump (re-encadena hasta llenar el límite o vaciar la FIFO).
fn drain_queue(app: tauri::AppHandle, core: Arc<ManagerCore>) {
    let mut q = core.queue.lock().unwrap();
    loop {
        if core.running.load(Ordering::Relaxed) >= core.max_parallelism.load(Ordering::Relaxed) {
            break;
        }
        let Some(key) = q.pop_front() else { break };
        let (job_id, task_id) = match key.split_once('/') {
            Some((j, t)) => (j.to_string(), t.to_string()),
            None => continue,
        };
        let (job, task) = {
            let jobs = core.jobs.lock().unwrap();
            let job = match jobs.get(&job_id).cloned() {
                Some(j) => j,
                None => continue, // job removido mientras estaba en cola
            };
            let task = match job.files.iter().find(|t| t.task_id == task_id).cloned() {
                Some(t) if t.state == TaskState::Queued => t,
                _ => continue, // entry stale (la sacó una pausa; resume re-encola)
            };
            (job, task)
        };
        let Some(models_path) = core.models_path.lock().unwrap().clone() else { continue };

        let stop = Arc::new(AtomicBool::new(false));
        core.register_active(&key, Arc::clone(&stop));
        core.running.fetch_add(1, Ordering::Relaxed);
        // S5: congela las settings del momento del spawn
        let (auto_retry, max_retries, keep_part) = (
            core.auto_retry.load(Ordering::Relaxed),
            core.max_retries.load(Ordering::Relaxed),
            core.keep_part_on_cancel.load(Ordering::Relaxed),
        );
        let app2 = app.clone();
        let core2 = core.clone();
        let repump_app = app.clone();
        let repump_core = core.clone();
        tauri::async_runtime::spawn(async move {
            run_task(
                app2, core2, job_id, models_path,
                job.repo_owner, job.repo_name, task, stop,
                auto_retry, max_retries, keep_part,
            ).await;
            repump_core.running.fetch_sub(1, Ordering::Relaxed);
            drain_queue(repump_app, repump_core.clone());
        });
    }
}

#[tauri::command]
pub fn pause_download(
    app: tauri::AppHandle,
    state: tauri::State<DownloadManager>,
    job_id: String,
) -> Result<JobView, String> {
    let core = state.0.clone();
    for (task_id, ts) in core.task_states(&job_id) {
        if matches!(ts, TaskState::Downloading | TaskState::Queued) {
            let key = format!("{job_id}/{task_id}");
            if let Some(flag) = core.active.lock().unwrap().get(&key) {
                flag.store(true, Ordering::SeqCst);
            }
            core.set_task_state(&job_id, &task_id, TaskState::Paused, None, None);
        }
    }
    let view = core.job_view(&job_id).ok_or("job no encontrado")?;
    emit_state(Some(&app), &job_id, view.state, None, None);
    Ok(view)
}

#[tauri::command]
pub fn resume_download(
    app: tauri::AppHandle,
    state: tauri::State<DownloadManager>,
    job_id: String,
    models_path: String,
    parallelism: u32,
    auto_retry: bool,
    max_retries: u32,
    keep_part_on_cancel: bool,
) -> Result<JobView, String> {
    let core = state.0.clone();
    *core.models_path.lock().unwrap() = Some(models_path);
    apply_download_settings(&core, parallelism, auto_retry, max_retries, keep_part_on_cancel);

    let resumable: Vec<String> = {
        let active = core.active.lock().unwrap();
        core.task_states(&job_id)
            .into_iter()
            .filter(|(task_id, s)| {
                let key = format!("{job_id}/{task_id}");
                match s {
                    TaskState::Paused => true,
                    TaskState::Queued => !active.contains_key(&key),
                    _ => false,
                }
            })
            .map(|(id, _)| id)
            .collect()
    };

    let was_empty = resumable.is_empty();
    for task_id in resumable {
        core.set_task_state(&job_id, &task_id, TaskState::Queued, None, None);
        core.queue.lock().unwrap().push_back(format!("{job_id}/{task_id}"));
    }
    if !was_empty {
        drain_queue(app.clone(), core.clone());
    }

    let view = core.job_view(&job_id).ok_or("job no encontrado")?;
    emit_state(Some(&app), &job_id, view.state, None, None);
    Ok(view)
}

#[tauri::command]
pub fn cancel_download(
    app: tauri::AppHandle,
    state: tauri::State<DownloadManager>,
    job_id: String,
) -> Result<JobView, String> {
    let core = state.0.clone();
    for (task_id, ts) in core.task_states(&job_id) {
        if !matches!(ts, TaskState::Completed) {
            let key = format!("{job_id}/{task_id}");
            if let Some(flag) = core.active.lock().unwrap().get(&key) {
                flag.store(true, Ordering::SeqCst);
            }
            core.set_task_state(&job_id, &task_id, TaskState::Failed, Some("cancelado".into()), Some(ErrorCode::Cancelled.as_str().to_string()));
        }
    }
    let view = core.job_view(&job_id).ok_or("job no encontrado")?;
    emit_state(Some(&app), &job_id, view.state, None, None);
    Ok(view)
}

#[tauri::command]
pub fn remove_download(state: tauri::State<DownloadManager>, job_id: String) -> Result<(), String> {
    let core = state.0.clone();
    let keys: Vec<String> = core
        .task_states(&job_id)
        .into_iter()
        .map(|(task_id, _)| format!("{job_id}/{task_id}"))
        .collect();
    {
        let mut jobs = core.jobs.lock().unwrap();
        if jobs.remove(&job_id).is_none() {
            return Err("job no encontrado".into());
        }
        core.persist_map(&jobs);
    }
    for key in &keys {
        core.unregister_active(key);
        core.unregister_progress(key);
        core.queue.lock().unwrap().retain(|k| k != key);
    }
    Ok(())
}

#[tauri::command]
pub fn list_downloads(state: tauri::State<DownloadManager>, models_path: String) -> Vec<JobView> {
    state.0.list(Some(&models_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Digest;
    use std::sync::Arc as StdArc;

    fn mk_task(blob: &[u8], sha: Option<String>) -> DownloadTask {
        DownloadTask {
            task_id: "t1".into(),
            path_in_repo: "x.gguf".into(),
            file_name: "x.gguf".into(),
            total_bytes: blob.len() as u64,
            sha256: sha,
            group: FileGroup::Gguf,
            downloaded_bytes: 0,
            state: TaskState::Queued,
            error: None,
            retry_count: 0,
            error_code: None,
        }
    }

    fn blob_sha(blob: &[u8]) -> String {
        hex_lower(Sha256::digest(blob))
    }

    // ---------- tests puras (sin red) ----------

    #[test]
    fn backoff_table() {
        assert_eq!(backoff_delay(1), 2);
        assert_eq!(backoff_delay(2), 4);
        assert_eq!(backoff_delay(3), 8);
        assert_eq!(backoff_delay(4), 16);
        assert_eq!(backoff_delay(5), 32);
        assert_eq!(backoff_delay(10), 60);
    }

    #[test]
    fn derive_state_rules() {
        fn t(s: TaskState) -> DownloadTask {
            let mut task = mk_task(&[0u8], None);
            task.state = s;
            task
        }
        assert_eq!(derive_state(&[t(TaskState::Completed)]), JobState::Completed);
        assert_eq!(derive_state(&[t(TaskState::Failed)]), JobState::Failed);
        assert_eq!(derive_state(&[t(TaskState::Downloading)]), JobState::Downloading);
        assert_eq!(derive_state(&[t(TaskState::Queued)]), JobState::Queued);
        assert_eq!(derive_state(&[t(TaskState::Paused)]), JobState::Paused);
        assert_eq!(
            derive_state(&[t(TaskState::Completed), t(TaskState::Failed), t(TaskState::Queued)]),
            JobState::Failed
        );
        assert_eq!(
            derive_state(&[t(TaskState::Downloading), t(TaskState::Paused)]),
            JobState::Downloading
        );
    }

    #[test]
    fn path_and_url_helpers() {
        let target = target_path("F:/models", "unsloth", "Qwen3.8-27B-GGUF", "MTP/mtp-Qwen3.8-27B-Q4_0.gguf");
        let expected = Path::new("F:/models").join("unsloth").join("Qwen3.8-27B-GGUF").join("MTP").join("mtp-Qwen3.8-27B-Q4_0.gguf");
        assert_eq!(target, expected);
        assert!(part_path(&target).to_string_lossy().ends_with("mtp-Qwen3.8-27B-Q4_0.gguf.part"));
        assert_eq!(url_path("MTP/mtp-Q4.gguf"), "MTP/mtp-Q4.gguf");
        assert_eq!(url_path("a b/c d.gguf"), "a%20b/c%20d.gguf");
        let url = resolve_url("unsloth", "Qwen3.8-27B-GGUF", "MTP/mtp-Q4.gguf");
        assert!(url.starts_with("https://huggingface.co/unsloth/Qwen3.8-27B-GGUF/resolve/main/"));
        assert!(url.ends_with("download=true"));
    }

    #[test]
    fn sha256_file_known() {
        let dir = std::env::temp_dir().join(format!("ls-s2-sha-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let f = dir.join("known.bin");
        std::fs::write(&f, b"hello").unwrap();
        assert_eq!(
            sha256_file(&f).as_deref(),
            Some("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---------- servidor de test (axum) con Range + 416 + 500 ----------

    struct ServerState {
        blob: Vec<u8>,
        serve_max: Option<usize>, // máx bytes a servir en esta respuesta (None = todo)
        http_500: bool,
    }

    async fn serve(
        axum::extract::State(st): axum::extract::State<StdArc<ServerState>>,
        req: axum::extract::Request,
    ) -> axum::response::Response {
        if st.http_500 {
            return axum::response::Response::builder()
                .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                .body(axum::body::Body::empty())
                .unwrap();
        }
        let total = st.blob.len();
        let start: usize = req.headers()
            .get("range")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("bytes="))
            .and_then(|v| v.split('-').next())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        if start >= total {
            return axum::response::Response::builder()
                .status(axum::http::StatusCode::RANGE_NOT_SATISFIABLE)
                .body(axum::body::Body::empty())
                .unwrap();
        }
        let end = match st.serve_max {
            Some(m) => (start + m).min(total),
            None => total,
        };
        let bytes = axum::body::Bytes::copy_from_slice(&st.blob[start..end]);
        let status = if start > 0 {
            axum::http::StatusCode::PARTIAL_CONTENT
        } else {
            axum::http::StatusCode::OK
        };
        let mut resp = axum::response::Response::new(axum::body::Body::from(bytes));
        *resp.status_mut() = status;
        if start > 0 {
            resp.headers_mut()
                .insert("content-range", format!("bytes {start}-{}/{total}", end - 1).parse().unwrap());
        }
        resp
    }

    async fn start_server(state: ServerState) -> String {
        let app = axum::Router::new()
            .route("/blob", axum::routing::get(serve))
            .with_state(StdArc::new(state));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        format!("http://{addr}/blob")
    }

    #[tokio::test]
    async fn download_full_and_verify_sha() {
        let blob = vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let url = start_server(ServerState { blob: blob.clone(), serve_max: None, http_500: false }).await;
        let dir = std::env::temp_dir().join(format!("ls-s2-full-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let task = mk_task(&blob, Some(blob_sha(&blob)));
        let target = dir.join("out.gguf");
        let part = part_path(&target);
        let stop = Arc::new(AtomicBool::new(false));
        let core = ManagerCore::new();

        let outcome = download_once(&core, None, "j1", &task, &url, &target, &part, &stop).await;
        assert!(matches!(outcome, TaskOutcome::Completed), "got {outcome:?}");
        assert_eq!(std::fs::read(&target).unwrap(), blob);
        assert!(!part.exists(), "el .part debe haberse renombrado");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn resume_from_part_uses_range() {
        let blob: Vec<u8> = (0..100u32).map(|i| i as u8).collect();
        let url = start_server(ServerState { blob: blob.clone(), serve_max: None, http_500: false }).await;
        let dir = std::env::temp_dir().join(format!("ls-s2-resume-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("out.gguf");
        let part = part_path(&target);
        let task = mk_task(&blob, Some(blob_sha(&blob)));
        let stop = Arc::new(AtomicBool::new(false));

        // una descarga previa dejó 50 bytes en el .part → resume con Range
        std::fs::write(&part, &blob[..50]).unwrap();
        let core = ManagerCore::new();
        let outcome = download_once(&core, None, "j1", &task, &url, &target, &part, &stop).await;
        assert!(matches!(outcome, TaskOutcome::Completed), "got {outcome:?}");
        assert_eq!(std::fs::read(&target).unwrap(), blob, "íntegro tras resume + Range");
        assert!(!part.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn sha_mismatch_fails_and_deletes_part() {
        let blob = vec![0u8, 1, 2, 3, 4, 5];
        let url = start_server(ServerState { blob: blob.clone(), serve_max: None, http_500: false }).await;
        let dir = std::env::temp_dir().join(format!("ls-s2-mis-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let task = mk_task(&blob, Some("0".repeat(64)));
        let target = dir.join("out.gguf");
        let part = part_path(&target);
        let stop = Arc::new(AtomicBool::new(false));
        let core = ManagerCore::new();

        let outcome = download_once(&core, None, "j1", &task, &url, &target, &part, &stop).await;
        assert!(matches!(outcome, TaskOutcome::Permanent { .. }), "got {outcome:?}");
        assert!(!target.exists());
        assert!(!part.exists(), "el .part debe borrarse ante mismatch");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn range_416_restarts_from_zero() {
        let blob = vec![0u8, 1, 2, 3, 4, 5, 6, 7];
        let url = start_server(ServerState { blob: blob.clone(), serve_max: None, http_500: false }).await;
        let dir = std::env::temp_dir().join(format!("ls-s2-416-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("out.gguf");
        let part = part_path(&target);
        let task = mk_task(&blob, None);
        let stop = Arc::new(AtomicBool::new(false));

        // .part de tamaño == total → pide Range bytes=N- → el servidor responde 416 → restart desde 0
        std::fs::write(&part, vec![0u8; blob.len()]).unwrap();
        let core = ManagerCore::new();
        let outcome = download_once(&core, None, "j1", &task, &url, &target, &part, &stop).await;
        assert!(matches!(outcome, TaskOutcome::Completed), "got {outcome:?}");
        assert_eq!(std::fs::read(&target).unwrap(), blob);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn server_error_is_transient() {
        let blob = vec![0u8, 1, 2, 3];
        let url = start_server(ServerState { blob, serve_max: None, http_500: true }).await;
        let dir = std::env::temp_dir().join(format!("ls-s2-500-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let task = mk_task(&[0u8, 1, 2, 3], None);
        let target = dir.join("out.gguf");
        let part = part_path(&target);
        let stop = Arc::new(AtomicBool::new(false));
        let core = ManagerCore::new();

        let outcome = download_once(&core, None, "j1", &task, &url, &target, &part, &stop).await;
        assert!(matches!(outcome, TaskOutcome::Transient { .. }), "got {outcome:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
