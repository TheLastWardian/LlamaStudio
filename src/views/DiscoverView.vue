<template>
  <div class="discover-layout">
    <div class="topbar">
      <span class="topbar-title">{{ t('discover.title') }}</span>
      <input class="search-box discover-search" v-model="query" :placeholder="t('discover.searchPlaceholder')" />
      <select class="discover-select" v-model="sort">
        <option v-for="s in SORT_OPTIONS" :key="s.value" :value="s.value">{{ t(s.labelKey) }}</option>
      </select>
      <label class="discover-toggle-label">
        <input type="checkbox" class="discover-toggle" v-model="ggufOnly" />
        {{ t('discover.ggufOnly') }}
      </label>
      <input class="discover-author" v-model="author" :placeholder="t('discover.authorPlaceholder')" />
      <span class="discover-hint">{{ t('discover.hint') }}</span>
    </div>

    <div class="content">
      <div v-if="error" class="discover-error">{{ error }}</div>
      <div v-if="loading && !repos.length" class="discover-status">{{ t('discover.searching') }}</div>
      <div v-else-if="!repos.length && !error" class="discover-status">
        {{ searched ? t('discover.noResults') : t('discover.empty') }}
      </div>
      <div
        v-for="repo in repos"
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
          <div v-if="summaryOf(repo)" class="repo-desc">{{ summaryOf(repo) }}</div>
          <div class="repo-tags">
            <span v-if="repo.tags.includes('gguf')" class="tag tag-gguf">GGUF</span>
            <span v-if="hasSpeculativeTag(repo)" class="tag tag-mtp">MTP</span>
            <span v-if="hasVisionTag(repo)" class="tag tag-vision">Vision</span>
            <span v-if="licenseOf(repo)" class="tag tag-license">{{ licenseOf(repo) }}</span>
            <span v-if="libraryCount(repo.id) > 0" class="tag tag-library">✓ {{ libraryCount(repo.id) }} {{ t('discover.inLibrary') }}</span>
          </div>
        </div>
        <div class="repo-stats">
          <div class="repo-stat"><span class="stat-downloads">↓ {{ fmtNum(repo.downloads) }}</span><span class="stat-likes"><span class="stat-star">★</span> {{ fmtNum(repo.likes) }}</span></div>
          <div class="repo-time">{{ relativeTime(repo.lastModified ?? repo.createdAt) || t('discover.now') }}</div>
        </div>
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
</script>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { allModels } from '../stores/selectedModel'
import { t } from '../i18n'
import RepoDownloadPanel from '../components/RepoDownloadPanel.vue'

const query = ref('')
const author = ref('')
const sort = ref('trendingScore')
const ggufOnly = ref(true)
const repos = ref<HfRepo[]>([])
const loading = ref(false)
const searched = ref(false)
const error = ref<string | null>(null)
const panelRepo = ref<HfRepo | null>(null)

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
watch([textParams, sort, ggufOnly], () => {
  void doSearch()
})

async function doSearch() {
  const seq = ++searchSeq
  loading.value = true
  error.value = null
  try {
    const result = await invoke<HfRepo[]>('search_hf_models', {
      query: textParams.value.query,
      sort: sort.value,
      limit: 50,
      author: textParams.value.author || null,
      ggufOnly: ggufOnly.value,
    })
    if (seq !== searchSeq) return
    repos.value = result
    searched.value = true
  } catch (e) {
    if (seq !== searchSeq) return
    error.value = String(e)
  } finally {
    if (seq === searchSeq) loading.value = false
  }
}

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

function openPanel(repo: HfRepo) {
  panelRepo.value = repo
}
</script>

<style scoped>
.discover-layout {
  height: 100%;
  display: flex;
  flex-direction: column;
  position: relative;
}

.discover-search {
  flex: 1 1 240px;
  max-width: 420px;
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

.repo-time {
  color: #d4d4d4;
  font-size: 11px;
}
</style>
