use serde::{Deserialize, Serialize};

const HF_API: &str = "https://huggingface.co/api";

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
    #[serde(rename = "modelId")]
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
    #[serde(rename = "createdAt")]
    pub created_at: String,
    /// Solo presente en la respuesta cuando sort=lastModified
    #[serde(rename = "lastModified", default)]
    pub last_modified: Option<String>,
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
) -> Result<Vec<HfRepo>, String> {
    let query = query.trim();
    let author = author.map(str::trim).filter(|a| !a.is_empty());

    // La API de HF hace match EXACTO (case-sensitive) sobre `author`:
    // "Unsloth" no encuentra el namespace "unsloth". Probamos variantes de
    // mayúsculas/minúsculas hasta que alguna devuelva resultados.
    let candidates = author_candidates(author);
    let mut fallback: Option<Vec<HfRepo>> = None;
    for c in &candidates {
        match do_search(query, sort, limit, c.as_deref(), gguf_only).await {
            Ok(repos) if !repos.is_empty() => return Ok(repos),
            Ok(repos) => {
                if fallback.is_none() {
                    fallback = Some(repos);
                }
            }
            Err(e) => return Err(e),
        }
    }
    Ok(fallback.unwrap_or_default())
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
) -> Result<Vec<HfRepo>, String> {
    let mut req = client()
        .get(format!("{HF_API}/models"))
        .query(&[("sort", sort), ("direction", "-1")])
        .query(&[("limit", limit.to_string())]);
    if !query.is_empty() {
        req = req.query(&[("search", query)]);
    }
    if let Some(a) = author.filter(|a| !a.is_empty()) {
        req = req.query(&[("author", a)]);
    }
    if gguf_only {
        req = req.query(&[("filter", "gguf")]);
    }

    let resp = req.send().await.map_err(|e| format!("HF search: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("HF search: HTTP {status}"));
    }
    resp.json::<Vec<HfRepo>>()
        .await
        .map_err(|e| format!("HF search JSON: {e}"))
}

pub async fn repo_files(owner: &str, repo: &str, speculative_tags: bool) -> Result<Vec<RepoFile>, String> {
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
    let items: Vec<TreeItem> = resp
        .json()
        .await
        .map_err(|e| format!("HF tree JSON: {e}"))?;

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
    resp.text().await.map_err(|e| format!("HF readme text: {e}"))
}

// ---------- Clasificación de archivos (puras, testeables sin red) ----------

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
        let repos = search_models("qwen3.8", "lastModified", 3, None, false).await.unwrap();
        assert!(!repos.is_empty());
        assert!(repos.iter().any(|r| r.last_modified.is_some()));

        let (owner, name) = repos[0].id.split_once('/').unwrap();
        let files = repo_files(owner, name, false).await.unwrap();
        assert!(files.iter().any(|f| f.size > 0));
    }
}
