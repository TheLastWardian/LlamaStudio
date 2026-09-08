<template>
  <div class="discover-layout">
    <div class="topbar">
      <span class="topbar-title">{{ t('discover.title') }}</span>
      <input class="search-box discover-search" v-model="query" :placeholder="t('discover.searchPlaceholder')" />
      <select
        class="discover-select discover-size-select"
        v-model="sizeMin"
        :disabled="inLibraryOnly"
        :title="inLibraryOnly ? t('discover.sizeDisabledLocal') : t('discover.sizeMin')"
      >
        <option value="">{{ t('discover.sizeAny') }}</option>
        <option v-for="b in SIZE_BUCKETS" :key="b" :value="b">{{ b }}</option>
      </select>
      <select
        class="discover-select discover-size-select"
        v-model="sizeMax"
        :disabled="inLibraryOnly"
        :title="inLibraryOnly ? t('discover.sizeDisabledLocal') : t('discover.sizeMax')"
      >
        <option value="">{{ t('discover.sizeAny') }}</option>
        <option v-for="b in SIZE_BUCKETS" :key="b" :value="b">{{ b }}</option>
      </select>
      <select class="discover-select" v-model="sort">
        <option v-for="s in SORT_OPTIONS" :key="s.value" :value="s.value">{{ t(s.labelKey) }}</option>
      </select>
      <select
        class="discover-select"
        v-model="dateFilter"
        :disabled="inLibraryOnly"
        :title="inLibraryOnly ? t('discover.filterDisabledLocal') : undefined"
      >
        <option v-for="d in DATE_OPTIONS" :key="d.value" :value="d.value">{{ t(d.labelKey) }}</option>
      </select>
      <label class="discover-toggle-label">
        <input type="checkbox" class="discover-toggle" v-model="ggufOnly" />
        {{ t('discover.ggufOnly') }}
      </label>
      <label class="discover-toggle-label">
        <input type="checkbox" class="discover-toggle" v-model="inLibraryOnly" />
        {{ t('discover.inLibraryOnly') }}
      </label>
      <input class="discover-author" v-model="author" :placeholder="t('discover.authorPlaceholder')" />
      <span class="discover-hint">{{ t('discover.hint') }}</span>
    </div>

    <div class="content">
      <div v-if="error" class="discover-error">{{ error }}</div>
      <div v-if="loading && !repos.length" class="discover-status">{{ t('discover.searching') }}</div>
      <div v-else-if="!visibleRepos.length && !error" class="discover-status">
        {{ searched ? t('discover.noResults') : t('discover.empty') }}
      </div>
      <div
        v-for="repo in visibleRepos"
        :key="repo.id"
        class="repo-row"
        :class="{ selected: panelRepo?.id === repo.id }"
        @click="openPanel(repo)"
      >
        <div class="avatar" :style="{ background: avatarColor(repo.id) }">{{ initials(repo.id) }}</div>
        <div class="repo-main">
          <div class="repo-name">
            <span class="owner">{{ ownerOf(repo.id) }}/</span>{{ nameOf(repo.id) }}
          </div>
          <div v-if="isLocalRepo(repo)" class="repo-desc">{{ repo.fileCount }} {{ t('discover.files') }} · {{ fmtBytes(repo.totalSize) }}</div>
          <div v-else-if="summaryOf(repo)" class="repo-desc">{{ summaryOf(repo) }}</div>
          <div class="repo-tags">
            <span v-if="repo.tags.includes('gguf')" class="tag tag-gguf">GGUF</span>
            <span v-if="hasSpeculativeTag(repo)" class="tag tag-mtp">MTP</span>
            <span v-if="hasVisionTag(repo)" class="tag tag-vision">Vision</span>
            <span v-if="licenseOf(repo)" class="tag tag-license">{{ licenseOf(repo) }}</span>
            <span v-if="!isLocalRepo(repo) && libraryCount(repo.id) > 0" class="tag tag-library">✓ {{ libraryCount(repo.id) }} {{ t('discover.inLibrary') }}</span>
          </div>
        </div>
        <div v-if="hasQ4(repo)" class="repo-q4k">
          <span class="q4k-quant">Q4_K_M</span>
          <span v-if="q4Size(repo) !== null" class="q4k-size">{{ fmtBytes(q4Size(repo)!) }}</span>
        </div>
        <div v-if="!isLocalRepo(repo)" class="repo-stats">
          <div class="repo-stat"><span class="stat-downloads">↓ {{ fmtNum(repo.downloads) }}</span><span class="stat-likes"><span class="stat-star">★</span> {{ fmtNum(repo.likes) }}</span></div>
          <div class="repo-time">{{ relativeTime(repo.lastModified ?? repo.createdAt) || t('discover.now') }}</div>
        </div>
      </div>
      <div v-if="!inLibraryOnly && nextCursor && !loading" class="load-more-wrap">
        <button class="load-more-btn" :disabled="loadingMore" @click="loadMore">
          {{ loadingMore ? t('discover.loadingMore') : t('discover.loadMore') }}
        </button>
      </div>
    </div>

    <RepoDownloadPanel v-if="panelRepo" :repo="panelRepo" @close="panelRepo = null" />
  </div>
</template>
<script lang="ts">
// Contrato con Rust (hf.rs de S1), tal como queda serializado:
// HfRepo conserva `pipeline_tag`/`library_name` en snake_case (sin serde rename);
// RepoFile.group llega en lowercase (rename_all = "lowercase") y `possible_draft` en snake_case.
export interface HfRepo {
  id: string
  modelId: string
  likes: number
  downloads: number
  private: boolean
  tags: string[]
  pipeline_tag: string | null
  library_name: string | null
  createdAt: string
  lastModified: string | null
  has_q4_k_m: boolean
}

// Repo local (ya en librería) para la vista "In library only": se sintetiza a partir
// de allModels. Extiende HfRepo con tags=[] para que el row y el panel de descarga
// rendericen igual; isLocal distingue la fuente (desc/stats propios).
export interface LocalRepo extends HfRepo {
  isLocal: true
  fileCount: number
  totalSize: number
}

export interface RepoFile {
  path: string
  size: number
  sha256: string | null
  group: 'gguf' | 'mtp' | 'vision' | 'other'
  quant: string | null
  possible_draft: boolean
}

// Los 5 sorts de §2 (los valores son el parámetro `sort` de la API de HF, pasados tal cual por S1)
export const SORT_OPTIONS: { value: string, labelKey: string }[] = [
  { value: 'trendingScore', labelKey: 'discover.sort.trending' },
  { value: 'downloads', labelKey: 'discover.sort.downloads' },
  { value: 'likes', labelKey: 'discover.sort.likes' },
  { value: 'lastModified', labelKey: 'discover.sort.lastModified' },
  { value: 'createdAt', labelKey: 'discover.sort.createdAt' },
]

export function hasSpeculativeTag(repo: HfRepo): boolean {
  return repo.tags.includes('mtp') || repo.tags.includes('speculative-decoding')
}

export function licenseOf(repo: HfRepo): string {
  return repo.tags.find(tg => tg.startsWith('license:'))?.slice('license:'.length) ?? ''
}

// "5m" / "3h" / "18d" / "2y"; '' si tiene menos de un minuto (el caller muestra "just now")
export function relativeTime(iso: string): string {
  const then = new Date(iso).getTime()
  if (Number.isNaN(then)) return ''
  const sec = Math.max(0, (Date.now() - then) / 1000)
  if (sec < 60) return ''
  const min = Math.floor(sec / 60)
  if (min < 60) return min + 'm'
  const h = Math.floor(min / 60)
  if (h < 48) return h + 'h'
  const d = Math.floor(h / 24)
  if (d < 365) return d + 'd'
  return Math.floor(d / 365) + 'y'
}

export function fmtNum(n: number): string {
  return n.toLocaleString()
}

// Respuesta de search_hf_models (hf::HfSearchPage)
export interface SearchPage {
  repos: HfRepo[]
  next_cursor: string | null
}
</script>

<script setup lang="ts">
import { ref, reactive, computed, watch, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { allModels } from '../stores/selectedModel'
import { t } from '../i18n'
import RepoDownloadPanel from '../components/RepoDownloadPanel.vue'

const query = ref('')
const author = ref('')
const sort = ref('trendingScore')
const ggufOnly = ref(true)
const inLibraryOnly = ref(false)
const dateFilter = ref<'all' | '10d' | '30d' | '3m' | '6m' | '1y'>('all')
const sizeMin = ref('')
const sizeMax = ref('')
const repos = ref<HfRepo[]>([])
const nextCursor = ref<string | null>(null)
const loading = ref(false)
const loadingMore = ref(false)
const searched = ref(false)
const error = ref<string | null>(null)
const panelRepo = ref<HfRepo | null>(null)

const DATE_OPTIONS: { value: 'all' | '10d' | '30d' | '3m' | '6m' | '1y', labelKey: string }[] = [
  { value: 'all', labelKey: 'discover.filter.all' },
  { value: '10d', labelKey: 'discover.filter.10d' },
  { value: '30d', labelKey: 'discover.filter.30d' },
  { value: '3m', labelKey: 'discover.filter.3m' },
  { value: '6m', labelKey: 'discover.filter.6m' },
  { value: '1y', labelKey: 'discover.filter.1y' },
]

function cutoffMs(): number | null {
  const now = Date.now()
  switch (dateFilter.value) {
    case '10d': return now - 10 * 86400_000
    case '30d': return now - 30 * 86400_000
    case '3m': { const d = new Date(); d.setMonth(d.getMonth() - 3); return d.getTime() }
    case '6m': { const d = new Date(); d.setMonth(d.getMonth() - 6); return d.getTime() }
    case '1y': { const d = new Date(); d.setFullYear(d.getFullYear() - 1); return d.getTime() }
    default: return null
  }
}

// Buckets de parámetros de HF (los mismos que el slider de huggingface.co/models).
// El value es el label tal cual: la API lo recibe en `num_parameters=min:..,max:..`
// y un extremo vacío se omite (sin cota).
const SIZE_BUCKETS = ['< 1B', '3B', '6B', '9B', '12B', '24B', '32B', '64B', '128B', '256B', '> 500B']

// Si el usuario elige min > max se intercambian (la API espera rango coherente).
function sizeParams(): { min: string | null, max: string | null } {
  let lo = sizeMin.value
  let hi = sizeMax.value
  if (lo && hi && SIZE_BUCKETS.indexOf(lo) > SIZE_BUCKETS.indexOf(hi)) [lo, hi] = [hi, lo]
  return { min: lo || null, max: hi || null }
}

let debounceTimer: ReturnType<typeof setTimeout> | null = null
let searchSeq = 0

// Debounce ~400 ms solo sobre los inputs de texto; sort/toggle buscan de inmediato
const textParams = ref({ query: '', author: '' })

function scheduleSearch() {
  if (debounceTimer) clearTimeout(debounceTimer)
  debounceTimer = setTimeout(() => {
    textParams.value = { query: query.value.trim(), author: author.value.trim() }
  }, 400)
}

watch([query, author], scheduleSearch)
watch([textParams, sort, ggufOnly, sizeMin, sizeMax], () => {
  void doSearch()
})

async function doSearch() {
  const seq = ++searchSeq
  loading.value = true
  loadingMore.value = false
  error.value = null
  nextCursor.value = null
  autoFillPages = 0
  try {
    await waitPageCd()
    if (seq !== searchSeq) return
    lastPageFetchAt = Date.now()
    const size = sizeParams()
    const page = await invoke<SearchPage>('search_hf_models', {
      query: textParams.value.query,
      sort: sort.value,
      limit: 50,
      author: textParams.value.author || null,
      ggufOnly: ggufOnly.value,
      cursor: null,
      paramMin: size.min,
      paramMax: size.max,
    })
    if (seq !== searchSeq) return
    repos.value = page.repos
    nextCursor.value = page.next_cursor
    searched.value = true
    q4Queue.length = 0
    scheduleQ4Sizes(page.repos)
  } catch (e) {
    if (seq !== searchSeq) return
    error.value = String(e)
  } finally {
    if (seq === searchSeq) loading.value = false
  }
  if (seq === searchSeq) maybeAutoFill()
}

// Rate limit de HF: 500 req/5 min compartidos con el resto de la app.
// CD entre llamadas de búsqueda (auto-fill incluido) para no burstear.
let lastPageFetchAt = 0
const PAGE_FETCH_CD_MS = 1500

function waitPageCd(): Promise<void> {
  const wait = lastPageFetchAt + PAGE_FETCH_CD_MS - Date.now()
  return wait <= 0 ? Promise.resolve() : new Promise(res => setTimeout(res, wait))
}

// Página siguiente del cursor: appendea deduplicando por id.
async function loadMore() {
  if (!nextCursor.value || loadingMore.value) return
  const seq = searchSeq
  loadingMore.value = true
  try {
    await waitPageCd()
    if (seq !== searchSeq) return
    lastPageFetchAt = Date.now()
    const size = sizeParams()
    const page = await invoke<SearchPage>('search_hf_models', {
      query: textParams.value.query,
      sort: sort.value,
      limit: 50,
      author: textParams.value.author || null,
      ggufOnly: ggufOnly.value,
      cursor: nextCursor.value,
      paramMin: size.min,
      paramMax: size.max,
    })
    if (seq !== searchSeq) return
    const seen = new Set(repos.value.map(r => r.id))
    for (const r of page.repos) if (!seen.has(r.id)) repos.value.push(r)
    nextCursor.value = page.next_cursor
    scheduleQ4Sizes(page.repos)
  } catch (e) {
    if (seq === searchSeq) error.value = String(e)
    return
  } finally {
    if (seq === searchSeq) loadingMore.value = false
  }
  if (seq === searchSeq) maybeAutoFill()
}

// Con filtro de fecha activo, se cargan páginas solas (el botón "Cargar más"
// sigue disponible para seguir a mano):
// - sort "last modified": hasta que el último repo queda fuera de la ventana (exacto).
// - otros sorts: hasta juntar AUTOFILL_MATCHES dentro de la ventana o el tope de páginas.
let autoFillPages = 0
const AUTOFILL_MAX = 20
const AUTOFILL_MATCHES = 25

function windowMatchCount(): number {
  const cutoff = cutoffMs()
  if (cutoff === null) return 0
  return repos.value.filter(r => {
    const t = new Date(r.lastModified ?? r.createdAt).getTime()
    return !Number.isNaN(t) && t >= cutoff
  }).length
}

function maybeAutoFill() {
  if (dateFilter.value === 'all' || !nextCursor.value || autoFillPages >= AUTOFILL_MAX) return
  if (sort.value === 'lastModified') {
    const last = repos.value[repos.value.length - 1]
    const cutoff = cutoffMs()
    if (!last || cutoff === null) return
    const t = new Date(last.lastModified ?? last.createdAt).getTime()
    if (!Number.isNaN(t) && t >= cutoff) {
      autoFillPages++
      void loadMore()
    }
    return
  }
  if (windowMatchCount() >= AUTOFILL_MATCHES) return
  autoFillPages++
  void loadMore()
}

watch(dateFilter, () => {
  autoFillPages = 0
  scheduleQ4Sizes(repos.value)
  maybeAutoFill()
})

onMounted(() => {
  void doSearch()
})
onUnmounted(() => {
  if (debounceTimer) clearTimeout(debounceTimer)
  searchSeq++
})

// "✓ N en librería": cruza la key {owner}/{repo} (últimos 2 segmentos del path)
// contra los archivos ya escaneados (layout de librería {modelsPath}/{owner}/{repo}/{file})
const libraryCounts = computed(() => {
  const counts: Record<string, number> = {}
  for (const m of allModels.value) {
    const parts = m.path.replace(/\\/g, '/').split('/')
    if (parts.length < 3) continue
    const key = parts[parts.length - 3] + '/' + parts[parts.length - 2]
    counts[key] = (counts[key] ?? 0) + 1
  }
  return counts
})

function libraryCount(id: string): number {
  return libraryCounts.value[id] ?? 0
}

// Badge Q4_K_M: los repos locales se calculan de allModels (sin red); los remotos se
// detectan con has_q4_k_m (la búsqueda trae expand[]=siblings) y el peso se fetchea
// del tree de forma progresiva (concurrencia limitada + caché por sesión por repo id).
const localQ4 = computed(() => {
  const map: Record<string, number> = {}
  for (const m of allModels.value) {
    if (!/q4_k_m\.gguf$/i.test(m.name)) continue
    if (!m.publisher || !m.model_family) continue
    const id = m.publisher + '/' + m.model_family
    map[id] = (map[id] ?? 0) + m.size_bytes
  }
  return map
})

const q4Sizes = reactive<Record<string, number | null>>({})
const q4Queue: string[] = []
const Q4_CONCURRENCY = 4
let q4InFlight = 0

function hasQ4(repo: HfRepo | LocalRepo): boolean {
  return localQ4.value[repo.id] !== undefined || repo.has_q4_k_m
}

function q4Size(repo: HfRepo | LocalRepo): number | null {
  return localQ4.value[repo.id] ?? q4Sizes[repo.id] ?? null
}

const Q4_QUEUE_MAX = 150

function scheduleQ4Sizes(list: HfRepo[]) {
  for (const r of list) {
    if (!r.has_q4_k_m || r.id in q4Sizes || localQ4.value[r.id] !== undefined) continue
    if (q4Queue.length >= Q4_QUEUE_MAX) break
    q4Queue.push(r.id)
  }
  pumpQ4()
}

// CD entre fetches de tree: con el queue de 150, sin pausa sería un burst.
let lastQ4FetchAt = 0
const Q4_FETCH_CD_MS = 600

function pumpQ4() {
  while (q4InFlight < Q4_CONCURRENCY && q4Queue.length > 0) {
    const id = q4Queue.shift()!
    q4InFlight++
    const [owner, repo] = id.split('/')
    const wait = lastQ4FetchAt + Q4_FETCH_CD_MS - Date.now()
    const start = wait <= 0 ? Promise.resolve() : new Promise(res => setTimeout(res, wait))
    void start.then(() => {
      lastQ4FetchAt = Date.now()
      return invoke<number | null>('q4k_size', { owner, repo })
    })
      .then(size => { q4Sizes[id] = size ?? null })
      .catch(() => { q4Sizes[id] = null })
      .finally(() => {
        q4InFlight--
        pumpQ4()
      })
  }
}

// "In library only": la fuente es la librería local (allModels), NO el filtro de la
// búsqueda de HF. Agrupa por repo {publisher}/{model_family} (los mismos campos que usa
// el panel para marcar archivos "ya en librería") y sintetiza un HfRepo por repo.
const libraryRepos = computed<LocalRepo[]>(() => {
  const map = new Map<string, { fileCount: number, totalSize: number }>()
  for (const m of allModels.value) {
    if (!m.publisher || !m.model_family) continue
    const id = m.publisher + '/' + m.model_family
    const agg = map.get(id) ?? { fileCount: 0, totalSize: 0 }
    agg.fileCount += 1
    agg.totalSize += m.size_bytes
    map.set(id, agg)
  }
  return [...map.entries()]
    .map(([id, a]): LocalRepo => ({
      id,
      modelId: id,
      likes: 0,
      downloads: 0,
      private: false,
      tags: [],
      pipeline_tag: null,
      library_name: null,
      createdAt: '',
      lastModified: null,
      has_q4_k_m: false,
      isLocal: true,
      fileCount: a.fileCount,
      totalSize: a.totalSize,
    }))
    .sort((x, y) => x.id.localeCompare(y.id))
})

const visibleRepos = computed<(HfRepo | LocalRepo)[]>(() => {
  if (inLibraryOnly.value) return libraryRepos.value
  const cutoff = cutoffMs()
  if (cutoff === null) return repos.value
  return repos.value.filter(r => {
    const t = new Date(r.lastModified ?? r.createdAt).getTime()
    return !Number.isNaN(t) && t >= cutoff
  })
})

function isLocalRepo(r: HfRepo | LocalRepo): r is LocalRepo {
  return (r as LocalRepo).isLocal === true
}

function fmtBytes(n: number): string {
  if (n >= 1024 ** 3) return (n / 1024 ** 3).toFixed(1) + ' GB'
  if (n >= 1024 ** 2) return Math.round(n / 1024 ** 2) + ' MB'
  return Math.max(1, Math.round(n / 1024)) + ' KB'
}
function hasVisionTag(repo: HfRepo): boolean {
  return repo.tags.includes('image-text-to-text') || repo.tags.includes('vision')
}

// HfRepo no trae description: se compone 1 línea con los metadatos del contrato de S1
function summaryOf(repo: HfRepo): string {
  const parts: string[] = []
  if (repo.pipeline_tag) parts.push(repo.pipeline_tag)
  if (repo.library_name) parts.push(repo.library_name)
  if (repo.tags.includes('long-context')) parts.push('long-context')
  return parts.join(' · ')
}

function ownerOf(id: string): string {
  const i = id.indexOf('/')
  return i === -1 ? '' : id.slice(0, i)
}

function nameOf(id: string): string {
  const i = id.indexOf('/')
  return i === -1 ? id : id.slice(i + 1)
}

function initials(id: string): string {
  const owner = ownerOf(id)
  const name = nameOf(id)
  return ((owner.charAt(0) || '') + (name.charAt(0) || '')).toUpperCase()
}

const AVATAR_PALETTE = ['#2563eb', '#7c3aed', '#db2777', '#ea580c', '#059669', '#0891b2']

function avatarColor(id: string): string {
  let h = 0
  for (let i = 0; i < id.length; i++) h = (h * 31 + id.charCodeAt(i)) >>> 0
  return AVATAR_PALETTE[h % AVATAR_PALETTE.length]
}

function openPanel(repo: HfRepo | LocalRepo) {
  panelRepo.value = repo
}
</script>

<style scoped>
.discover-layout {
  flex: 1;
  min-height: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  position: relative;
}

.discover-search {
  flex: 1 1 200px;
  max-width: 340px;
}

.discover-size-select {
  width: 78px;
  flex-shrink: 0;
}

.discover-select {
  background: #2a2a2a;
  border: 1px solid #333;
  color: #d4d4d4;
  border-radius: 8px;
  padding: 7px 10px;
  font-size: 13px;
  outline: none;
  cursor: pointer;
}

.discover-toggle-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  color: #aaa;
  cursor: pointer;
  white-space: nowrap;
  user-select: none;
}

.discover-toggle {
  accent-color: #5a8af5;
  cursor: pointer;
}

.discover-author {
  width: 170px;
  background: #2a2a2a;
  border: 1px solid #333;
  color: #d4d4d4;
  border-radius: 8px;
  padding: 7px 10px;
  font-size: 13px;
  outline: none;
}

.discover-author:focus,
.discover-search:focus {
  border-color: #5a8af5;
}

.discover-hint {
  font-size: 12px;
  color: #666;
  margin-left: auto;
  white-space: nowrap;
}

.discover-error {
  color: #f87171;
  font-size: 13px;
  padding: 10px 0;
  word-break: break-word;
}

.discover-status {
  color: #666;
  font-size: 13px;
  padding: 18px 0;
  text-align: center;
}
.repo-row {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 10px 12px;
  margin: 0 8px;
  border-radius: 8px;
  cursor: pointer;
  border: 1px solid transparent;
  position: relative;
}

.repo-row + .repo-row::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 1px;
  background: #2a2a2a;
  pointer-events: none;
}

.repo-row:hover {
  background: #222;
}

.repo-row.selected {
  background: #252535;
  border-color: #5a8af5;
}

.avatar {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-size: 13px;
  font-weight: 700;
  flex-shrink: 0;
}

.repo-main {
  flex: 1;
  min-width: 0;
}

.repo-name {
  color: #fff;
  font-size: 13px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.repo-name .owner {
  color: #888;
  font-weight: 400;
}

.repo-desc {
  color: #888;
  font-size: 12px;
  margin-top: 2px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.repo-tags {
  display: flex;
  gap: 6px;
  margin-top: 6px;
  flex-wrap: wrap;
}

/* heredan el .tag global (padding/radio/tipo); aquí solo el color por variante */
.tag-gguf { background: #1a3a5c; color: #5aacf5; }
.tag-mtp { background: #3a2a1a; color: #e0a35a; }
.tag-vision { background: #2a1a3a; color: #c58af5; }
.tag-license { background: #2a2a2a; color: #888; }
.tag-library { background: #1a3a2a; color: #5ac98a; }

.repo-stats {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 4px;
  width: 165px;
  flex-shrink: 0;
}

.repo-stat {
  display: flex;
  gap: 10px;
  color: #aaa;
  font-size: 12px;
}

.stat-star { color: #d4af37; }
.stat-likes { color: #f5d77b; }
.stat-downloads { color: #d4d4d4; }

.repo-q4k {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 150px;
  margin-right: 8px;
  flex-shrink: 0;
  font-size: 13px;
}
.q4k-quant { color: #d4d4d4; }
.q4k-size { color: #7ee787; }

.discover-select:disabled {
  opacity: 0.5;
  cursor: default;
}

.load-more-wrap {
  display: flex;
  justify-content: center;
  padding: 12px 0 16px;
}

.load-more-btn {
  background: #2a2a2a;
  border: 1px solid #333;
  color: #d4d4d4;
  border-radius: 8px;
  padding: 8px 18px;
  font-size: 13px;
  cursor: pointer;
}

.load-more-btn:hover:not(:disabled) {
  border-color: #5a8af5;
}

.load-more-btn:disabled {
  opacity: 0.6;
  cursor: default;
}

.repo-time {
  color: #d4d4d4;
  font-size: 11px;
}
</style>
