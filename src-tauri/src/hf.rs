use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

const HF_API: &str = "https://huggingface.co/api";
/// Un README > 1 MB es anómalo; limita la memoria que un repo malicioso puede forzar.
const README_MAX_BYTES: usize = 1024 * 1024;

fn client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent("LlamaStudio/0.1.0")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("reqwest client")
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfRepo {
    pub id: String,
    /// No es expandible: ausente cuando la búsqueda usa expand[] (y el app no lo usa).
    #[serde(rename = "modelId", default)]
    pub model_id: String,
    #[serde(default)]
    pub likes: u64,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub private: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub pipeline_tag: Option<String>,
    #[serde(default)]
    pub library_name: Option<String>,
    #[serde(rename = "createdAt", default)]
    pub created_at: String,
    /// Solo presente en la respuesta cuando sort=lastModified
    #[serde(rename = "lastModified", default)]
    pub last_modified: Option<String>,
    /// Archivos del repo vía `expand[]=siblings`; solo nombres, sin sizes.
    #[serde(default)]
    pub siblings: Vec<Sibling>,
    /// Derivado post-deserialización: ¿el repo trae algún GGUF con quant Q4_K_M?
    #[serde(skip_deserializing)]
    pub has_q4_k_m: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sibling {
    pub rfilename: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HfSearchPage {
    pub repos: Vec<HfRepo>,
    /// Token opaco para la página siguiente (header `Link: rel="next"`); None si no hay más.
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileGroup {
    Gguf,
    Mtp,
    Vision,
    Other,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepoFile {
    pub path: String,
    pub size: u64,
    /// lfs.oid (sha256) — None para archivos no-LFS
    pub sha256: Option<String>,
    pub group: FileGroup,
    pub quant: Option<String>,
    /// Segundo nivel: repo con tag HF speculative-decoding/mtp Y gguf < 20% del main más grande
    pub possible_draft: bool,
}

#[derive(Deserialize)]
struct TreeItem {
    #[serde(rename = "type")]
    kind: String,
    path: String,
    size: Option<u64>,
    lfs: Option<LfsInfo>,
}

#[derive(Deserialize)]
struct LfsInfo {
    oid: Option<String>,
    size: Option<u64>,
}

pub async fn search_models(
    query: &str,
    sort: &str,
    limit: u32,
    author: Option<&str>,
    gguf_only: bool,
    cursor: Option<&str>,
    param_min: Option<&str>,
    param_max: Option<&str>,
) -> Result<HfSearchPage, String> {
    let query = query.trim();
    let author = author.map(str::trim).filter(|a| !a.is_empty());

    // La API de HF hace match EXACTO (case-sensitive) sobre `author`:
    // "Unsloth" no encuentra el namespace "unsloth". Probamos variantes de
    // mayúsculas/minúsculas hasta que alguna devuelva resultados.
    let candidates = author_candidates(author);
    let mut fallback: Option<HfSearchPage> = None;
    for c in &candidates {
        match do_search(query, sort, limit, c.as_deref(), gguf_only, cursor, param_min, param_max).await {
            Ok(page) if !page.repos.is_empty() => return Ok(page),
            Ok(page) => {
                if fallback.is_none() {
                    fallback = Some(page);
                }
            }
            Err(e) => return Err(e),
        }
    }
    Ok(fallback.unwrap_or(HfSearchPage { repos: Vec::new(), next_cursor: None }))
}

/// Variantes de `author` a probar en orden: exacto, minúsculas y primera letra
/// en mayúscula. Sin autor -> una sola tentativa sin filtro.
fn author_candidates(author: Option<&str>) -> Vec<Option<String>> {
    let Some(a) = author else {
        return vec![None];
    };
    let mut out: Vec<Option<String>> = Vec::new();
    for v in [a.to_string(), a.to_lowercase()] {
        if !out.iter().any(|x| x.as_deref() == Some(v.as_str())) {
            out.push(Some(v));
        }
    }
    let mut t = a.chars();
    if let Some(first) = t.next() {
        let titled: String = first.to_uppercase().collect::<String>() + t.as_str();
        if !out.iter().any(|x| x.as_deref() == Some(titled.as_str())) {
            out.push(Some(titled));
        }
    }
    out
}

async fn do_search(
    query: &str,
    sort: &str,
    limit: u32,
    author: Option<&str>,
    gguf_only: bool,
    cursor: Option<&str>,
    param_min: Option<&str>,
    param_max: Option<&str>,
) -> Result<HfSearchPage, String> {
    // Con `expand[]` la API responde SOLO con los campos expandidos: hay que listar
    // todos los que usa el app (sin expand[] vienen por defecto, menos `modelId`).
    const EXPAND: [&str; 9] = [
        "siblings", "likes", "downloads", "private", "tags",
        "pipeline_tag", "library_name", "createdAt", "lastModified",
    ];
    let mut req = client()
        .get(format!("{HF_API}/models"))
        .query(&[("sort", sort), ("direction", "-1")])
        .query(&[("limit", limit.to_string())]);
    for e in EXPAND {
        req = req.query(&[("expand[]", e)]);
    }
    if !query.is_empty() {
        req = req.query(&[("search", query)]);
    }
    if let Some(a) = author.filter(|a| !a.is_empty()) {
        req = req.query(&[("author", a)]);
    }
    if gguf_only {
        req = req.query(&[("filter", "gguf")]);
    }
    if let Some(c) = cursor.filter(|c| !c.is_empty()) {
        req = req.query(&[("cursor", c)]);
    }
    // Filtro por parámetros de HF: `num_parameters=min:<bucket>,max:<bucket>`.
    // Los buckets son los del slider de la web ("< 1B", "3B", ... "> 500B"); un
    // extremo vacío se omite (sin cota).
    let mut np: Vec<String> = Vec::new();
    if let Some(m) = param_min.filter(|v| !v.is_empty()) {
        np.push(format!("min:{m}"));
    }
    if let Some(m) = param_max.filter(|v| !v.is_empty()) {
        np.push(format!("max:{m}"));
    }
    if !np.is_empty() {
        req = req.query(&[("num_parameters", np.join(","))]);
    }

    let resp = req.send().await.map_err(|e| format!("HF search: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("HF search: HTTP {status}"));
    }
    let next_cursor = resp
        .headers()
        .get(reqwest::header::LINK)
        .and_then(|v| v.to_str().ok())
        .and_then(extract_next_cursor);
    let mut repos: Vec<HfRepo> = resp
        .json::<Vec<HfRepo>>()
        .await
        .map_err(|e| format!("HF search JSON: {e}"))?;
    derive_q4k_flags(&mut repos);
    Ok(HfSearchPage { repos, next_cursor })
}

/// Extrae el parámetro `cursor` de un header `Link: <url>; rel="next"`.
/// None si no hay página siguiente o el header no trae cursor.
pub fn extract_next_cursor(link: &str) -> Option<String> {
    let url = link.split('<').nth(1)?.split('>').next()?;
    let encoded = url.split("cursor=").nth(1)?.split('&').next()?;
    percent_decode(encoded)
}

async fn fetch_tree(owner: &str, repo: &str) -> Result<Vec<TreeItem>, String> {
    let resp = client()
        .get(format!("{HF_API}/models/{owner}/{repo}/tree/main"))
        .query(&[("recursive", "true")])
        .send()
        .await
        .map_err(|e| format!("HF tree: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("HF tree: HTTP {status}"));
    }
    resp.json().await.map_err(|e| format!("HF tree JSON: {e}"))
}

pub async fn repo_files(owner: &str, repo: &str, speculative_tags: bool) -> Result<Vec<RepoFile>, String> {
    let items = fetch_tree(owner, repo).await?;

    let mut files: Vec<RepoFile> = items
        .into_iter()
        .filter(|it| it.kind == "file")
        .map(|it| {
            let size = it
                .lfs
                .as_ref()
                .and_then(|l| l.size)
                .or(it.size)
                .unwrap_or(0);
            let sha256 = it.lfs.and_then(|l| l.oid);
            let (group, quant) = classify_file(&it.path);
            RepoFile { path: it.path, size, sha256, group, quant, possible_draft: false }
        })
        .collect();

    mark_possible_drafts(&mut files, speculative_tags);
    Ok(files)
}

/// Peso total (bytes) de los GGUF main con quant Q4_K_M del repo; los splits se suman.
/// None si no existe. Misma limitación de paginación que repo_files.
pub async fn q4k_size(owner: &str, repo: &str) -> Result<Option<u64>, String> {
    let items = fetch_tree(owner, repo).await?;
    let total: u64 = items
        .iter()
        .filter(|it| it.kind == "file")
        .filter_map(|it| {
            let (group, quant) = classify_file(&it.path);
            if group == FileGroup::Gguf
                && quant.as_deref().is_some_and(|q| q.eq_ignore_ascii_case("Q4_K_M"))
            {
                Some(it.lfs.as_ref().and_then(|l| l.size).or(it.size).unwrap_or(0))
            } else {
                None
            }
        })
        .sum();
    Ok((total > 0).then_some(total))
}

// README.md crudo del repo (endpoint raw de HF; /api/models/{id}/readme no existe).
// 404 (sin README) → Ok("") para que el frontend muestre un fallback en vez de error.
pub async fn get_repo_readme(owner: &str, repo: &str) -> Result<String, String> {
    let resp = client()
        .get(format!("https://huggingface.co/{owner}/{repo}/raw/main/README.md"))
        .send()
        .await
        .map_err(|e| format!("HF readme: {e}"))?;
    let status = resp.status();
    if status.as_u16() == 404 {
        return Ok(String::new());
    }
    if !status.is_success() {
        return Err(format!("HF readme: HTTP {status}"));
    }
    let mut buf: Vec<u8> = Vec::new();
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| format!("HF readme stream: {e}"))?;
        if buf.len() + bytes.len() > README_MAX_BYTES {
            return Err(format!("HF readme: demasiado grande (> {} KB)", README_MAX_BYTES / 1024));
        }
        buf.extend_from_slice(&bytes);
    }
    Ok(String::from_utf8_lossy(&buf).into())
}

// ---------- Clasificación de archivos (puras, testeables sin red) ----------

/// Deriva has_q4_k_m de siblings (llega vía expand[]=siblings en la búsqueda).
pub fn derive_q4k_flags(repos: &mut [HfRepo]) {
    for r in repos.iter_mut() {
        r.has_q4_k_m = siblings_has_q4k(&r.siblings);
    }
}

/// ¿Alguna entrada `siblings` es un GGUF con quant Q4_K_M (case-insensitive)?
pub fn siblings_has_q4k(siblings: &[Sibling]) -> bool {
    siblings.iter().any(|s| {
        extract_quant(&s.rfilename)
            .as_deref()
            .is_some_and(|q| q.eq_ignore_ascii_case("Q4_K_M"))
    })
}

pub fn classify_file(path: &str) -> (FileGroup, Option<String>) {
    let lower = path.to_ascii_lowercase();
    let orig_name = path.rsplit('/').next().unwrap_or(path);
    let lower_name = lower.rsplit('/').next().unwrap_or(&lower);

    if !lower_name.ends_with(".gguf") {
        return (FileGroup::Other, None);
    }
    if is_mtp_path(&lower) {
        return (FileGroup::Mtp, extract_quant(orig_name));
    }
    if lower_name.starts_with("mmproj") {
        return (FileGroup::Vision, extract_quant(orig_name));
    }
    (FileGroup::Gguf, extract_quant(orig_name))
}

fn is_mtp_path(lower_path: &str) -> bool {
    let file_name = lower_path.rsplit('/').next().unwrap_or(lower_path);
    let in_mtp_dir = lower_path.split('/').any(|p| p == "mtp");
    in_mtp_dir
        || file_name.contains("mtp-")
        || file_name.contains("-mtp.")
        || file_name.contains("dflash")
        || file_name.contains("-draft")
}

/// Extrae la cuantización del nombre: Q4_K_M, IQ4_XS, Q8_0, Q2_K, BF16, F16, F32…
/// Case-insensitive; devuelve el sub-string con el casing original.
pub fn extract_quant(name: &str) -> Option<String> {
    let bytes = name.as_bytes();
    let n = bytes.len();

    // "Q"/"IQ" + dígito, seguido de grupos _XXX (1-3 chars)
    let mut i = 0;
    while i < n {
        let c = bytes[i] as char;
        if (c == 'Q' || c == 'q') && i + 1 < n && (bytes[i + 1] as char).is_ascii_digit() {
            let start = if i > 0 && bytes[i - 1] as char == 'I' { i - 1 } else { i };
            let mut j = i + 1;
            while j < n && (bytes[j] as char).is_ascii_digit() {
                j += 1;
            }
            let mut k = j;
            while k < n && bytes[k] == b'_' {
                let mut m = k + 1;
                while m < n && (bytes[m] as char).is_ascii_alphanumeric() && m - (k + 1) < 3 {
                    m += 1;
                }
                if m > k + 1 {
                    k = m;
                } else {
                    break;
                }
            }
            return Some(name[start..k].to_string());
        }
        i += 1;
    }

    for token in ["BF16", "F16", "F32"] {
        if let Some(pos) = find_token(name, token) {
            return Some(name[pos..pos + token.len()].to_string());
        }
    }
    None
}

fn find_token(name: &str, token: &str) -> Option<usize> {
    let upper = name.to_ascii_uppercase();
    upper
        .match_indices(token)
        .find(|&(i, _)| {
            let before_ok = i == 0 || !(upper.as_bytes()[i - 1] as char).is_ascii_alphanumeric();
            let end = i + token.len();
            let after_ok = end >= upper.len() || !(upper.as_bytes()[end] as char).is_ascii_alphanumeric();
            before_ok && after_ok
        })
        .map(|(i, _)| i)
}

/// Segundo nivel de drafts: solo si el repo trae los tags HF de speculative
/// decoding Y el GGUF pesa < 20% del GGUF main más grande del repo.
fn mark_possible_drafts(files: &mut [RepoFile], speculative_tags: bool) {
    if !speculative_tags {
        return;
    }
    let max_main = files
        .iter()
        .filter(|f| f.group == FileGroup::Gguf)
        .map(|f| f.size)
        .max()
        .unwrap_or(0);
    if max_main == 0 {
        return;
    }
    for f in files.iter_mut() {
        if f.group == FileGroup::Gguf && f.size > 0 && f.size < max_main / 5 {
            f.possible_draft = true;
        }
    }
}

// ---------- hfimg: imágenes del README vía llamastudio.exe (webview sin internet) ----------
// El webview pide http://hfimg.localhost/<url percent-encodiada>; el handler valida
// (https + allowlist) y fetchea con el reqwest compartido. El socket lo abre llamastudio.exe.

/// Hosts permitidos para servir imágenes del README (match exacto). No es proxy abierto.
/// `github.com` es el patrón `github.com/<o>/<r>/raw/...` (redirige a
/// raw.githubusercontent.com, ya confiable); `*.gitbook.io` se chequea aparte.
const IMG_HOSTS: [&str; 4] = [
    "cdn.huggingface.co",
    "huggingface.co",
    "raw.githubusercontent.com",
    "github.com",
];
fn host_allowed(host: &str) -> bool {
    // GitBook sirve los assets de sus docs desde subdominios numéricos
    // (p. ej. `3215535692-files.gitbook.io`), comunes en READMEs de HF.
    IMG_HOSTS.contains(&host) || host == "gitbook.io" || host.ends_with(".gitbook.io")
}
/// Imagen > 20 MB es anómala; limita la memoria que un repo malicioso puede forzar
/// (como README_MAX_BYTES). El cap se aplica en streaming en `fetch_image`, así
/// acota la memoria pico real (cap + 1 chunk), no solo lo que se acepta al final.
const IMG_MAX_BYTES: usize = 20 * 1024 * 1024;

/// Decodifica un segmento de path percent-encodiado por `encodeURIComponent` de JS.
fn percent_decode(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            // Escape incompleto o no hex → inválido (como decodeURIComponent que tira).
            if i + 2 >= b.len() {
                return None;
            }
            let hi = (b[i + 1] as char).to_digit(16)?;
            let lo = (b[i + 2] as char).to_digit(16)?;
            out.push((hi * 16 + lo) as u8);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Política: decodifica el path y valida que el target sea https con host en IMG_HOSTS.
fn resolve_image_target(path: &str) -> Result<String, ()> {
    let url = percent_decode(path).ok_or(())?;
    let rest = url.strip_prefix("https://").ok_or(())?;
    let host = rest
        .split('/')
        .next()
        .unwrap_or("")
        .rsplit('@')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("");
    if !host_allowed(host) {
        return Err(());
    }
    Ok(url)
}

/// Transporte: GET con cap de bytes. Devuelve (content-type, body).
async fn fetch_image(url: &str, max_bytes: usize) -> Result<(Option<String>, Vec<u8>), ()> {
    let resp = client().get(url).send().await.map_err(|_| ())?;
    if !resp.status().is_success() {
        return Err(());
    }
    // Content-Length es solo una pista (el servidor puede omitirlo o mentir); el
    // chequeo real es post-`bytes()` abajo. `content_length()` devuelve u64.
    if resp.content_length().unwrap_or(0) > max_bytes as u64 {
        return Err(());
    }
    let ct = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    // Streaming con aborto temprano: `bytes()` descargaba el body completo antes de
    // chequear el cap, y un servidor que omite/mente Content-Length podía forzar la
    // descarga entera en memoria.
    let mut body: Vec<u8> = Vec::new();
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|_| ())?;
        if body.len() + bytes.len() > max_bytes {
            return Err(());
        }
        body.extend_from_slice(&bytes);
    }
    Ok((ct, body))
}

/// Handler asíncrono del protocolo `hfimg` (corre en el proceso de la app).
pub fn image_protocol_handler(
    _ctx: tauri::UriSchemeContext<'_, tauri::Wry>,
    request: tauri::http::Request<Vec<u8>>,
    responder: tauri::UriSchemeResponder,
) {
    let path = request.uri().path().trim_start_matches('/').to_string();
    let target = match resolve_image_target(&path) {
        Ok(t) => t,
        Err(()) => {
            responder.respond(
                tauri::http::Response::builder()
                    .status(tauri::http::StatusCode::FORBIDDEN)
                    .body(Vec::new())
                    .expect("hfimg response"),
            );
            return;
        }
    };
    tauri::async_runtime::spawn(async move {
        match fetch_image(&target, IMG_MAX_BYTES).await {
            Ok((ct, bytes)) => {
                let mut builder =
                    tauri::http::Response::builder().status(tauri::http::StatusCode::OK);
                if let Some(t) = ct {
                    builder = builder.header(tauri::http::header::CONTENT_TYPE, t);
                }
                responder.respond(builder.body(bytes).expect("hfimg response"));
            }
            Err(()) => {
                responder.respond(
                    tauri::http::Response::builder()
                        .status(tauri::http::StatusCode::NOT_FOUND)
                        .body(Vec::new())
                        .expect("hfimg response"),
                );
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_quant_real_library_cases() {
        assert_eq!(extract_quant("Qwen3.8-27B-UD-Q4_K_M.gguf").as_deref(), Some("Q4_K_M"));
        assert_eq!(extract_quant("Qwen3.8-27B-Q8_0.gguf").as_deref(), Some("Q8_0"));
        assert_eq!(extract_quant("Qwen3.8-27B-UD-IQ4_XS.gguf").as_deref(), Some("IQ4_XS"));
        assert_eq!(
            extract_quant("gemma-4-26B-A4B-it-qat-uncensored-heretic-UDmerge-Q4_K_XXL.gguf").as_deref(),
            Some("Q4_K_XXL")
        );
        assert_eq!(extract_quant("Qwen3.8-27B-DFlash2-Q4_K_M.gguf").as_deref(), Some("Q4_K_M"));
        assert_eq!(extract_quant("mtp-Qwen3.8-27B-Q4_0.gguf").as_deref(), Some("Q4_0"));
        assert_eq!(extract_quant("mmproj-BF16.gguf").as_deref(), Some("BF16"));
        assert_eq!(extract_quant("mmproj-F32.gguf").as_deref(), Some("F32"));
        assert_eq!(extract_quant("Qwen3.8-27B-F16.gguf").as_deref(), Some("F16"));
        assert_eq!(
            extract_quant("text-embedding-nomic-embed-text-v1.5-Q4_K_M.gguf").as_deref(),
            Some("Q4_K_M")
        );
        assert_eq!(extract_quant("Qwen3.8-27B-Q2_K.gguf").as_deref(), Some("Q2_K"));
        assert_eq!(extract_quant("Qwen3.8-27B-Q5_K_M.gguf").as_deref(), Some("Q5_K_M"));
    }

    // Respuesta real de /api/models con expand[] (capturada 2026-09, recortada).
    // Con expand[] la API omite `modelId` y a veces `pipeline_tag`: el struct debe
    // deserializar igual y derivar has_q4_k_m de siblings.
    #[test]
    fn search_response_with_expands_deserializes() {
        let json = r#"[{"_id":"6a9e7e1cb1f1e4eb39556d8a","id":"neonoodles/Qwen3.8-27B-Wildthing-ARA-MTP-GGUF","lastModified":"2026-09-08T22:20:58.000Z","likes":0,"private":false,"downloads":6,"tags":["gguf","mtp"],"pipeline_tag":"text-generation","library_name":"gguf","siblings":[{"rfilename":".gitattributes"},{"rfilename":"Qwen3.8-27B-Wildthing-Q8_0.gguf"}],"createdAt":"2026-09-07T09:04:28.000Z"},{"_id":"6a89b39c39eb84b2b2421a0d","id":"mradermacher/Qwen3.8-27B-abliterated-GGUF","lastModified":"2026-09-08T21:47:30.000Z","likes":0,"private":false,"downloads":1795,"tags":["gguf"],"library_name":"transformers","siblings":[{"rfilename":".gitattributes"},{"rfilename":"Qwen3.8-27B-Abliterated.Q3_K_M.gguf"},{"rfilename":"Qwen3.8-27B-Abliterated.Q4_K_M.gguf"}],"createdAt":"2026-08-22T14:35:08.000Z"}]"#;
        let mut repos: Vec<HfRepo> = serde_json::from_str(json).expect("search JSON deserializa");
        derive_q4k_flags(&mut repos);
        assert_eq!(repos.len(), 2);
        assert!(!repos[0].has_q4_k_m);
        assert!(repos[1].has_q4_k_m);
        assert_eq!(repos[0].model_id, "");
        assert_eq!(repos[1].pipeline_tag, None);
        assert_eq!(repos[1].downloads, 1795);
    }

    // Header real capturado de /api/models (limit=2): el cursor va percent-encoded.
    #[test]
    fn link_header_cursor_extraction() {
        let link = "<https://huggingface.co/api/models?search=qwen3.8&limit=2&filter=gguf&cursor=eyIkb3IiOlt7InRyZW5kaW5nU2NvcmUiOjMyMSwiX2lkIjp7IiRndCI6IjZhOTY3ODM1NGUyNTQxYjgyOGI0YTQwZSJ9fSx7InRyZW5kaW5nU2NvcmUiOnsiJGx0IjozMjF9fSx7InRyZW5kaW5nU2NvcmUiOm51bGx9XSwic2VhcmNoU2VxdWVuY2VUb2tlbiI6IkNKZU15QUlhQ1NFQUFBQUFBQkIwUUJvT1dneXFsbmcxVGlWQnVDaTBwQTRpRGxvTWFwWjROVTRsUWJnb3RLUU8ifQ%3D%3D>; rel=\"next\"";
        let cursor = extract_next_cursor(link).expect("extrae el cursor");
        // El token es opaco (base64 de un JSON de estado): se reenvía tal cual,
        // reqwest lo percent-encodea igual que en el header original.
        assert!(cursor.len() > 100);
        assert!(cursor.ends_with("=="));
        assert!(!cursor.contains('%'));
        assert!(extract_next_cursor("<https://huggingface.co/api/models?limit=5>; rel=\"next\"").is_none());
        assert!(extract_next_cursor("").is_none());
        assert!(extract_next_cursor("no angle brackets").is_none());
    }

    #[test]
    fn siblings_q4k_detection() {
        let sib = |rf: &str| Sibling { rfilename: rf.into() };
        assert!(siblings_has_q4k(&[sib(".gitattributes"), sib("Qwen3.8-4B-Q4_K_M.gguf")]));
        assert!(siblings_has_q4k(&[sib("model-q4_k_m.gguf")]));
        assert!(!siblings_has_q4k(&[sib("model-Q8_0.gguf"), sib("model-Q4_0.gguf")]));
        assert!(!siblings_has_q4k(&[]));
    }

    #[test]
    fn classify_real_library_cases() {
        assert_eq!(classify_file("Qwen3.8-27B-UD-Q4_K_M.gguf"), (FileGroup::Gguf, Some("Q4_K_M".into())));
        assert_eq!(classify_file("MTP/mtp-Qwen3.8-27B-Q4_0.gguf"), (FileGroup::Mtp, Some("Q4_0".into())));
        assert_eq!(classify_file("Qwen3.8-27B-DFlash2-Q4_K_M.gguf"), (FileGroup::Mtp, Some("Q4_K_M".into())));
        assert_eq!(classify_file("Qwen3.8-27B-mini-draft.gguf"), (FileGroup::Mtp, None));
        assert_eq!(classify_file("qwen36-35b-a3b-dflash-Q4_K_M.gguf"), (FileGroup::Mtp, Some("Q4_K_M".into())));
        assert_eq!(classify_file("mmproj-BF16.gguf"), (FileGroup::Vision, Some("BF16".into())));
        assert_eq!(classify_file("BF16/Qwen3.8-27B-BF16.gguf"), (FileGroup::Gguf, Some("BF16".into())));
        assert_eq!(classify_file("MTP/mtp-x-Q8_0.gguf"), (FileGroup::Mtp, Some("Q8_0".into())));
        assert_eq!(classify_file(".gitattributes"), (FileGroup::Other, None));
        assert_eq!(classify_file("config.json"), (FileGroup::Other, None));
    }

    #[test]
    fn possible_draft_two_tier() {
        let mk = |path: &str, size: u64| RepoFile {
            path: path.into(),
            size,
            sha256: None,
            group: FileGroup::Gguf,
            quant: None,
            possible_draft: false,
        };
        let mut files = vec![mk("main-Q8_0.gguf", 30_000_000), mk("draft-Q4_0.gguf", 5_000_000)];
        mark_possible_drafts(&mut files, true);
        assert!(!files[0].possible_draft);
        assert!(files[1].possible_draft);

        let mut files = vec![mk("main-Q8_0.gguf", 30_000_000), mk("draft-Q4_0.gguf", 5_000_000)];
        mark_possible_drafts(&mut files, false);
        assert!(!files[0].possible_draft && !files[1].possible_draft);
    }

    #[tokio::test]
    #[ignore = "requiere red — cargo test -- --ignored"]
    async fn live_search_and_tree_smoke() {
        let page = search_models("qwen3.8", "lastModified", 3, None, false, None, None, None).await.unwrap();
        assert!(!page.repos.is_empty());
        assert!(page.repos.iter().any(|r| r.last_modified.is_some()));
        let (owner, name) = page.repos[0].id.split_once('/').unwrap();
        // Segunda página vía cursor: distinto contenido, sin duplicados
        let cursor = page.next_cursor.expect("hay página siguiente con limit=3");
        let page2 = search_models("qwen3.8", "lastModified", 3, None, false, Some(&cursor), None, None)
            .await
            .unwrap();
        // Filtro por parámetros: bucket 32B-64B (labels, tal como manda la web)
        let page3 = search_models("", "downloads", 3, None, true, None, Some("32B"), Some("64B"))
            .await
            .unwrap();
        assert!(!page3.repos.is_empty());
        assert!(!page2.repos.is_empty());
        assert!(page2.repos.iter().all(|r| !page.repos.iter().any(|p| p.id == r.id)));
        let files = repo_files(owner, name, false).await.unwrap();
        assert!(files.iter().any(|f| f.size > 0));
    }

    // ---------- hfimg: política + transporte ----------

    /// Equivalente de test a `encodeURIComponent` JS (solo para los chars que usamos).
    fn enc(s: &str) -> String {
        let mut out = String::new();
        for b in s.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => {
                    out.push(b as char)
                }
                _ => out.push_str(&format!("%{b:02X}")),
            }
        }
        out
    }

    #[test]
    fn hfimg_percent_decode_roundtrip() {
        assert_eq!(
            percent_decode("%68ttps%3A%2F%2Fcdn.huggingface.co%2Fa%20b.png"),
            Some("https://cdn.huggingface.co/a b.png".to_string())
        );
        assert_eq!(percent_decode("plain"), Some("plain".to_string()));
        assert_eq!(percent_decode("bad%zz"), None);
        assert_eq!(percent_decode("trunc%4"), None);
    }

    #[test]
    fn hfimg_resolve_accepts_allowlisted_https() {
        assert!(resolve_image_target(&enc("https://cdn.huggingface.co/img/x.png")).is_ok());
        assert!(resolve_image_target(&enc("https://huggingface.co/a/b/raw/main/x.png")).is_ok());
        assert!(resolve_image_target(&enc("https://raw.githubusercontent.com/o/r/main/x.png")).is_ok());
        assert!(resolve_image_target(&enc("https://github.com/o/r/raw/main/img/x.png")).is_ok());
        assert!(resolve_image_target(&enc("https://3215535692-files.gitbook.io/~/files/v0/x.gif?token=abc")).is_ok());
    }

    #[test]
    fn hfimg_resolve_rejects_scheme_host_garbage() {
        assert!(resolve_image_target(&enc("http://cdn.huggingface.co/x.png")).is_err());
        assert!(resolve_image_target(&enc("https://evil.com/x.png")).is_err());
        assert!(resolve_image_target(&enc("https://cdn.huggingface.co.evil.com/x.png")).is_err());
        assert!(resolve_image_target(&enc("https://sub.cdn.huggingface.co/x.png")).is_err());
        assert!(resolve_image_target(&enc("https://gitbook.io.evil.com/x.png")).is_err());
        assert!(resolve_image_target(&enc("https://evil-gitbook.io/x.png")).is_err());
        assert!(resolve_image_target(&enc("javascript:alert(1)")).is_err());
        assert!(resolve_image_target("no percent encoding").is_err());
        assert!(resolve_image_target("").is_err());
    }

    #[tokio::test]
    async fn hfimg_fetch_returns_bytes_and_content_type() {
        async fn serve_img() -> axum::response::Response {
            axum::response::Response::builder()
                .header("content-type", "image/png")
                .body(axum::body::Body::from(vec![1u8, 2, 3]))
                .unwrap()
        }
        let app = axum::Router::new().route("/img", axum::routing::get(serve_img));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        let (ct, bytes) = fetch_image(&format!("http://{addr}/img"), 1024)
            .await
            .expect("fetch ok");
        assert_eq!(ct.as_deref(), Some("image/png"));
        assert_eq!(bytes, vec![1u8, 2, 3]);
    }

    #[tokio::test]
    async fn hfimg_fetch_enforces_size_cap() {
        async fn serve_big() -> axum::body::Body {
            axum::body::Body::from(vec![7u8; 4096])
        }
        let app = axum::Router::new().route("/big", axum::routing::get(serve_big));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        assert!(fetch_image(&format!("http://{addr}/big"), 1024).await.is_err());
    }

    // Content-Length ausente o mentiroso: el cap debe aplicarse en streaming
    // (aborta al exceder el cap) en vez de esperar al body completo.
    #[tokio::test]
    async fn hfimg_fetch_enforces_size_cap_on_stream() {
        async fn serve_chunked() -> axum::response::Response {
            let stream = futures_util::stream::iter(vec![
                Ok::<_, std::io::Error>(axum::body::Bytes::from(vec![7u8; 2048])),
                Ok(axum::body::Bytes::from(vec![7u8; 2048])),
            ]);
            axum::response::Response::builder()
                .header("content-type", "image/png")
                .body(axum::body::Body::from_stream(stream))
                .unwrap()
        }
        let app = axum::Router::new().route("/big", axum::routing::get(serve_chunked));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        assert!(fetch_image(&format!("http://{addr}/big"), 1024).await.is_err());
    }
}
