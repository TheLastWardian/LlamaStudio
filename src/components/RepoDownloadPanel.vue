<template>
  <div class="panel-overlay">
    <div class="readme-pane">
      <div v-if="readmeLoading" class="readme-status">{{ t('discover.loadingReadme') }}</div>
      <div v-else-if="readmeError" class="readme-status readme-status-error">{{ readmeError }}</div>
      <div v-else-if="!readmeHtml" class="readme-status">{{ t('discover.noReadme') }}</div>
      <div v-else class="readme-content" v-html="readmeHtml"></div>
    </div>
    <aside class="panel" role="dialog" aria-modal="true">
      <header class="panel-header">
        <div class="panel-avatar">{{ avatarInitials }}</div>
        <div class="panel-title-main">
          <div class="panel-name">
            <span class="owner">{{ owner }}/</span>{{ name }}
          </div>
          <div class="panel-sub">
            {{ fmtNum(repo.downloads) }} ↓ · {{ fmtNum(repo.likes) }} ★ · {{ updatedLabel }}
            <span v-if="license" class="tag tag-license">{{ license }}</span>
          </div>
        </div>
        <div class="panel-actions">
          <button class="icon-btn" :title="t('discover.openInBrowser')" @click="openInBrowser">
            <ExternalLink :size="16" />
          </button>
          <button class="icon-btn icon-btn-close" :title="t('discover.close')" @click="closePanel">
            <X :size="17" />
          </button>
        </div>
      </header>

      <div class="panel-body">
        <div v-if="filesLoading" class="panel-status">{{ t('discover.loadingFiles') }}</div>
        <div v-else-if="filesError" class="panel-status panel-status-error">{{ filesError }}</div>
        <div v-else-if="!files.length" class="panel-status">{{ t('discover.noFiles') }}</div>
        <template v-else>
          <div v-if="!hasMain" class="panel-warn panel-warn-orange">{{ t('discover.draftWithoutMain') }}</div>
          <div v-if="mmprojCount > 0" class="panel-warn panel-warn-info">{{ t('discover.mmprojInfo') }}</div>

          <section v-for="g in groups" :key="g.key" class="file-group">
            <header class="file-group-head">
              <button
                class="group-chevron"
                :class="{ open: !collapsed[g.key] }"
                :title="collapsed[g.key] ? t('discover.expand') : t('discover.collapse')"
                @click="toggleCollapsed(g.key)"
              >
                <ChevronRight :size="14" />
              </button>
              <label class="group-check-label" :title="t('discover.selectAll')">
                <input
                  type="checkbox"
                  :checked="groupAllChecked(g)"
                  :indeterminate.prop="groupIndeterminate(g)"
                  @change="toggleGroup(g, $event)"
                />
                <span class="group-label">{{ t(g.labelKey) }}</span>
              </label>
              <span class="group-meta">{{ g.files.length }} · {{ fmtBytes(groupSize(g)) }}</span>
              <span class="group-help" :title="t(g.helpKey)">?</span>
            </header>

            <div v-show="!collapsed[g.key]">
            <div
              v-for="f in g.files"
              :key="f.path"
              class="file-row"
              :class="{ 'file-row-disabled': inLibrary(f), 'file-row-selected': !!checked[f.path] }"
            >
              <label v-if="!inLibrary(f)" class="file-check">
                <input type="checkbox" :checked="!!checked[f.path]" @change="toggleFile(f, $event)" />
              </label>
              <span v-else class="file-check file-check-static" :title="t('discover.inLibraryBadge')">✓</span>
              <div class="file-main">
                <div class="file-name">{{ fileName(f) }}</div>
                <div class="file-sub">
                  <template v-if="f.quant">{{ f.quant }} · </template>{{ fmtBytes(f.size) }}
                  <span v-if="f.possible_draft" class="tag tag-draft">{{ t('discover.possibleDraft') }}</span>
                  <span v-if="inLibrary(f)" class="tag tag-library">{{ t('discover.inLibraryBadge') }}</span>
                </div>
              </div>
            </div>
            </div>
          </section>
        </template>
      </div>

      <footer class="panel-footer">
        <div class="panel-total">
          <div class="panel-total-main">{{ t('discover.total') }}: <strong>{{ fmtBytes(selectedSize) }}</strong></div>
          <div v-if="selectedCount > 0" class="panel-total-sub">{{ selectedCount }} {{ t('discover.filesSelected') }}</div>
          <div v-else class="panel-total-sub panel-total-warn">{{ t('discover.nothingSelected') }}</div>
        </div>
        <button class="panel-download-btn" :disabled="selectedCount === 0 || downloading" @click="download">
          <Download :size="15" />
          {{ downloading ? t('discover.starting') : t('discover.download') }}
        </button>
      </footer>
    </aside>
  </div>
</template>
<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { ChevronRight, Download, ExternalLink, X } from '@lucide/vue'
import { t } from '../i18n'
import { appConfig } from '../stores/config'
import { allModels } from '../stores/selectedModel'
import { marked } from 'marked'
import DOMPurify from 'dompurify'
import {
  hasSpeculativeTag,
  licenseOf,
  relativeTime,
  fmtNum,
  type HfRepo,
  type RepoFile,
} from '../views/DiscoverView.vue'

const props = defineProps<{ repo: HfRepo }>()
const emit = defineEmits<{ (e: 'close'): void }>()

const files = ref<RepoFile[]>([])
const filesLoading = ref(false)
const filesError = ref<string | null>(null)
const readme = ref('')
const readmeLoading = ref(false)
const readmeError = ref<string | null>(null)
const checked = ref<Record<string, boolean>>({})
const downloading = ref(false)
const collapsed = ref<Record<string, boolean>>({ other: true })
function toggleCollapsed(key: string) { collapsed.value[key] = !collapsed.value[key] }

const owner = computed(() => props.repo.id.split('/')[0] ?? '')
const name = computed(() => {
  const i = props.repo.id.indexOf('/')
  return i === -1 ? props.repo.id : props.repo.id.slice(i + 1)
})
const license = computed(() => licenseOf(props.repo))
const avatarInitials = computed(() => (owner.value.charAt(0) + name.value.charAt(0)).toUpperCase())

const updatedLabel = computed(() => {
  const iso = props.repo.lastModified ?? props.repo.createdAt
  const rel = relativeTime(iso)
  return rel ? t('discover.updatedAgo', { time: rel }) : t('discover.updatedJustNow')
})

// README → HTML (marked) → sanitizado (DOMPurify) antes de meterlo al DOM.
const readmeHtml = computed(() => {
  if (!readme.value) return ''
  const md = readme.value.replace(/^---\r?\n[\s\S]*?\r?\n---\r?\n?/, '')
  const html = marked.parse(md, { async: false }) as string
  // Unwrap image-only anchors (markdown "[![x](img)](link)") so banner
  // images are not clickable and don't navigate the webview.
  return DOMPurify.sanitize(html).replace(/<a\b[^>]*>(\s*<img\b[^>]*>\s*)<\/a>/gi, '$1')
})

// Orden fijo de grupos (mockup): Main → MTP → Vision → Otros
const GROUP_DEFS: { key: RepoFile['group'], labelKey: string, helpKey: string }[] = [
  { key: 'gguf', labelKey: 'discover.group.main', helpKey: 'discover.group.mainHelp' },
  { key: 'mtp', labelKey: 'discover.group.mtp', helpKey: 'discover.group.mtpHelp' },
  { key: 'vision', labelKey: 'discover.group.vision', helpKey: 'discover.group.visionHelp' },
  { key: 'other', labelKey: 'discover.group.other', helpKey: 'discover.group.otherHelp' },
]

function fileName(f: RepoFile): string {
  const i = f.path.lastIndexOf('/')
  return i === -1 ? f.path : f.path.slice(i + 1)
}

const groups = computed(() => GROUP_DEFS
  .map(def => {
    const fs = files.value
      .filter(f => f.group === def.key)
      .sort((a, b) => def.key === 'other'
        ? fileName(a).localeCompare(fileName(b))
        : b.size - a.size)
    return { ...def, files: fs }
  })
  .filter(g => g.files.length > 0))

// Warnings suaves (§7.3)
const hasMain = computed(() => files.value.some(f => f.group === 'gguf'))
const mmprojCount = computed(() => files.value.filter(f => f.group === 'vision').length)

// "Ya en librería": el layout de destino es {modelsPath}/{owner}/{repo}/{file} (S2) y
// scan_models devuelve publisher = owner, model_family = repo (lib.rs) → match directo
const inLibrarySet = computed(() => {
  const set = new Set<string>()
  for (const m of allModels.value) {
    if (m.publisher !== owner.value || m.model_family !== name.value) continue
    const base = m.path.replace(/\\/g, '/').split('/').pop()
    if (base) set.add(base)
    for (const p of m.mmproj_paths) {
      const b = p.replace(/\\/g, '/').split('/').pop()
      if (b) set.add(b)
    }
  }
  return set
})

function inLibrary(f: RepoFile): boolean {
  return inLibrarySet.value.has(fileName(f))
}

function fmtBytes(n: number): string {
  if (n >= 1024 ** 3) return (n / 1024 ** 3).toFixed(1) + ' GB'
  if (n >= 1024 ** 2) return Math.round(n / 1024 ** 2) + ' MB'
  return Math.max(1, Math.round(n / 1024)) + ' KB'
}

function groupSize(g: { files: RepoFile[] }): number {
  return g.files.reduce((acc, f) => acc + f.size, 0)
}
// ---- Selección (sin selección por defecto: solo explícita, §7.3) ----

function selectable(g: { files: RepoFile[] }): RepoFile[] {
  return g.files.filter(f => !inLibrary(f))
}

function groupAllChecked(g: { files: RepoFile[] }): boolean {
  const sel = selectable(g)
  return sel.length > 0 && sel.every(f => !!checked.value[f.path])
}

function groupIndeterminate(g: { files: RepoFile[] }): boolean {
  const sel = selectable(g)
  const n = sel.filter(f => !!checked.value[f.path]).length
  return n > 0 && n < sel.length
}

function toggleGroup(g: { files: RepoFile[] }, ev: Event) {
  const on = (ev.target as HTMLInputElement).checked
  for (const f of selectable(g)) checked.value[f.path] = on
}

function toggleFile(f: RepoFile, ev: Event) {
  checked.value[f.path] = (ev.target as HTMLInputElement).checked
}

const selectedFiles = computed(() => files.value.filter(f => !!checked.value[f.path] && !inLibrary(f)))
const selectedCount = computed(() => selectedFiles.value.length)
const selectedSize = computed(() => selectedFiles.value.reduce((acc, f) => acc + f.size, 0))

async function loadFiles() {
  filesLoading.value = true
  filesError.value = null
  try {
    files.value = await invoke<RepoFile[]>('get_repo_files', {
      owner: owner.value,
      repo: name.value,
      // el search ya devolvió los tags del repo → sin 2do call a la API (S1)
      speculativeTags: hasSpeculativeTag(props.repo),
    })
  } catch (e) {
    filesError.value = String(e)
  } finally {
    filesLoading.value = false
  }
}

async function loadReadme() {
  readmeLoading.value = true
  readmeError.value = null
  try {
    readme.value = await invoke<string>('get_repo_readme', { owner: owner.value, repo: name.value })
  } catch (e) {
    readmeError.value = String(e)
  } finally {
    readmeLoading.value = false
  }
}

onMounted(() => {
  void loadFiles()
  void loadReadme()
  document.addEventListener('keydown', onKeydown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', onKeydown)
})

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') closePanel()
}

function closePanel() {
  emit('close')
}

function openInBrowser() {
  void openUrl('https://huggingface.co/' + props.repo.id)
}

// S4: 1 job multi-archivo (start_downloads hace el batch; paralelismo + FIFO en el backend)
async function download() {
  const modelsPath = appConfig.value.modelsPath
  if (!modelsPath) return
  downloading.value = true
  try {
    await invoke('start_downloads', {
      modelsPath,
      owner: owner.value,
      repo: name.value,
      files: selectedFiles.value.map(f => ({ path: f.path, size: f.size, sha256: f.sha256, group: f.group })),
    })
  } finally {
    downloading.value = false
  }
}
</script>

<style scoped>
.panel-overlay {
  position: absolute;
  inset: 0;
  background: #161616;
  z-index: 500;
  display: flex;
}

.readme-pane {
  flex: 1;
  min-width: 0;
  overflow-y: auto;
  padding: 24px 32px;
}

.readme-status {
  color: #666;
  font-size: 13px;
  padding: 12px 0;
}
.readme-status-error {
  color: #f87171;
}

.readme-content {
  color: #d4d4d4;
  font-size: 14px;
  line-height: 1.65;
  word-wrap: break-word;
}
.readme-content :deep(h1) { font-size: 22px; color: #fff; margin: 18px 0 10px; }
.readme-content :deep(h2) { font-size: 18px; color: #fff; margin: 16px 0 8px; }
.readme-content :deep(h3) { font-size: 15px; color: #fff; margin: 14px 0 6px; }
.readme-content :deep(a) { color: #5a8af5; text-decoration: none; }
.readme-content :deep(a:hover) { text-decoration: underline; }
.readme-content :deep(img) { max-width: 100%; height: auto; border-radius: 6px; }
.readme-content :deep(p) { margin: 8px 0; }
.readme-content :deep(ul),
.readme-content :deep(ol) { margin: 8px 0; padding-left: 22px; }
.readme-content :deep(li) { margin: 3px 0; }
.readme-content :deep(blockquote) { border-left: 3px solid #333; margin: 10px 0; padding: 2px 14px; color: #999; }
.readme-content :deep(hr) { border: none; border-top: 1px solid #2a2a2a; margin: 18px 0; }
.readme-content :deep(table) { border-collapse: collapse; margin: 12px 0; }
.readme-content :deep(th),
.readme-content :deep(td) { border: 1px solid #2a2a2a; padding: 6px 10px; }
.readme-content :deep(th) { background: #222; }
.readme-content :deep(code) { background: #262626; padding: 1px 5px; border-radius: 4px; font-size: 12.5px; }
.readme-content :deep(pre) { background: #0d0d0d; border: 1px solid #2a2a2a; border-radius: 8px; padding: 12px; overflow-x: auto; margin: 12px 0; }
.readme-content :deep(pre code) { background: none; padding: 0; }

.panel {
  width: 420px;
  max-width: 90%;
  background: #1e1e1e;
  border-left: 1px solid #333;
  display: flex;
  flex-direction: column;
  box-shadow: -8px 0 24px rgba(0, 0, 0, 0.4);
}

.panel-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 16px;
  border-bottom: 1px solid #2a2a2a;
}

.panel-avatar {
  width: 40px;
  height: 40px;
  border-radius: 8px;
  background: #333;
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 15px;
  font-weight: 700;
  flex-shrink: 0;
}

.panel-title-main {
  flex: 1;
  min-width: 0;
}

.panel-name {
  color: #fff;
  font-size: 14px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.panel-name .owner {
  color: #888;
  font-weight: 400;
}

.panel-sub {
  display: flex;
  align-items: center;
  gap: 6px;
  color: #888;
  font-size: 11px;
  margin-top: 3px;
}

.panel-actions {
  display: flex;
  gap: 8px;
}

.icon-btn {
  background: rgba(90, 138, 245, 0.12);
  border: 1px solid rgba(90, 138, 245, 0.4);
  border-radius: 7px;
  color: #85aef9;
  width: 30px;
  height: 30px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.15s;
}

.icon-btn:hover {
  background: rgba(90, 138, 245, 0.25);
  border-color: #5a8af5;
  color: #c0d6ff;
}

.icon-btn.icon-btn-close {
  background: rgba(255, 107, 107, 0.12);
  border-color: rgba(255, 107, 107, 0.4);
  color: #ff8f8f;
}

.icon-btn.icon-btn-close:hover {
  background: rgba(255, 107, 107, 0.25);
  border-color: #ff6b6b;
  color: #ffc2c2;
}

.panel-body {
  flex: 1;
  overflow-y: auto;
  padding: 10px 12px;
}

.panel-status {
  color: #666;
  font-size: 12px;
  text-align: center;
  padding: 16px 0;
  word-break: break-word;
}

.panel-status-error {
  color: #f55a5a;
}

.panel-warn {
  font-size: 12px;
  padding: 8px 10px;
  border-radius: 6px;
  margin-bottom: 10px;
  border: 1px solid;
}

.panel-warn-orange {
  background: #2a1f10;
  border-color: #4a3a1a;
  color: #e0a35a;
}

.panel-warn-info {
  background: #1a2433;
  border-color: #2a3a4a;
  color: #7ab0e8;
}
.file-group {
  margin-bottom: 14px;
}

.file-group-head {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  background: #1a1a2a;
  border-bottom: 1px solid #2a2a2a;
}

.group-chevron {
  background: transparent;
  border: none;
  padding: 2px;
  margin-left: -4px;
  cursor: pointer;
  color: #777;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  flex-shrink: 0;
}
.group-chevron:hover {
  color: #d4d4d4;
  background: #2a2a2a;
}
.group-chevron svg {
  transition: transform 0.15s;
}
.group-chevron.open svg {
  transform: rotate(90deg);
}

.group-check-label {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  user-select: none;
}

.group-check-label input {
  accent-color: #5a8af5;
  cursor: pointer;
}

.group-label {
  color: #ccc;
  font-size: 12px;
  font-weight: 600;
}

.group-meta {
  color: #666;
  font-size: 11px;
  margin-left: auto;
}

.group-help {
  color: #666;
  font-size: 10px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  border: 1px solid #444;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: help;
  flex-shrink: 0;
}

.file-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 7px 8px;
  border-bottom: 1px solid #222;
}

.file-row:last-child {
  border-bottom: none;
}

.file-row:hover {
  background: #222;
}

.file-row-selected {
  background: #252535;
}

.file-row-disabled {
  opacity: 0.5;
}

.file-check {
  display: flex;
  cursor: pointer;
}

.file-check input {
  accent-color: #5a8af5;
  cursor: pointer;
}

.file-check-static {
  color: #5ac98a;
  font-size: 14px;
}

.file-main {
  flex: 1;
  min-width: 0;
}

.file-name {
  color: #d4d4d4;
  font-size: 12px;
  font-family: Consolas, monospace;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.file-sub {
  display: flex;
  align-items: center;
  gap: 6px;
  color: #666;
  font-size: 11px;
  margin-top: 2px;
}

/* heredan el .tag global; aquí solo el color (definidos en scope del panel) */
.tag-draft { background: #3a2a1a; color: #e0a35a; }
.tag-library { background: #1a3a2a; color: #5ac98a; }
.tag-license { background: #2a2a2a; color: #888; }

.panel-footer {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  border-top: 1px solid #2a2a2a;
}

.panel-total {
  flex: 1;
  min-width: 0;
}

.panel-total-main {
  color: #ccc;
  font-size: 12px;
}

.panel-total-main strong {
  color: #fff;
}

.panel-total-sub {
  color: #666;
  font-size: 11px;
  margin-top: 2px;
}

.panel-total-warn {
  color: #e0a35a;
}

.panel-download-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  background: #5a8af5;
  color: #fff;
  border: none;
  border-radius: 6px;
  padding: 8px 14px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  flex-shrink: 0;
}

.panel-download-btn:hover {
  background: #6d99f7;
}

.panel-download-btn:disabled {
  opacity: 0.5;
  cursor: default;
}
</style>
