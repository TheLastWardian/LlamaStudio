<template>
  <div class="disc-overlay" @click.self="close">
    <div class="disc-modal" role="dialog" aria-modal="true">
      <header class="disc-head">
        <div class="disc-title">
          <MessagesSquare :size="15" />
          {{ t('discover.comments') }}
          <span v-if="!pending" class="disc-title-count">{{ list.length }}</span>
        </div>
        <button class="disc-close" :title="t('discover.close')" @click="close">
          <X :size="17" />
        </button>
      </header>
      <div class="disc-body">
        <div v-if="pending" class="disc-status">{{ t('discover.commentsLoading') }}</div>
        <div v-else-if="listError" class="disc-status disc-status-error">{{ listError }}</div>
        <div v-else-if="!list.length" class="disc-status">{{ t('discover.noComments') }}</div>
        <template v-else>
          <div v-for="d in list" :key="d.num" class="disc-item">
            <button class="disc-row" :class="{ open: openNum === d.num }" @click="toggle(d)">
              <div class="disc-row-main">
                <div class="disc-row-title">
                  <span v-if="d.pinned" class="disc-badge disc-badge-pin"><Pin :size="10" />{{ t('discover.pinned') }}</span>
                  <span v-if="d.is_pull_request" class="disc-badge disc-badge-pr">{{ t('discover.pr') }}</span>
                  <span v-if="d.status === 'closed'" class="disc-badge disc-badge-closed">{{ t('discover.closed') }}</span>
                  <span class="disc-row-name" :class="{ 'disc-name-closed': d.status === 'closed' }">{{ d.title }}</span>
                </div>
                <div class="disc-row-meta">#{{ d.num }} · {{ d.author }} · {{ timeOf(d.created_at) }}</div>
              </div>
              <div class="disc-row-stats">
                <span v-if="d.num_reactions > 0" class="disc-stat"><ThumbsUp :size="12" />{{ d.num_reactions }}</span>
                <span class="disc-stat"><MessageSquare :size="12" />{{ d.num_comments }}</span>
              </div>
            </button>
            <div v-if="openNum === d.num" class="disc-thread">
              <div v-if="threadLoading[d.num]" class="disc-status">{{ t('discover.loadingThread') }}</div>
              <div v-else-if="threadError[d.num]" class="disc-status disc-status-error">{{ threadError[d.num] }}</div>
              <div v-else-if="!(threadComments[d.num] ?? []).length" class="disc-status">{{ t('discover.noThreadComments') }}</div>
              <div
                v-for="c in threadComments[d.num] ?? []"
                :key="c.created_at + c.author + c.content.slice(0, 32)"
                class="disc-comment"
              >
                <div class="disc-comment-head">
                  <span class="disc-comment-author">{{ c.author }}</span>
                  <span class="disc-comment-time">{{ timeOf(c.created_at) }}</span>
                </div>
                <div class="disc-comment-body" v-html="htmlOf(c.content)"></div>
              </div>
            </div>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>
<script lang="ts">
// Contrato con Rust (hf.rs), tal como queda serializado por
// get_repo_discussions / get_repo_discussion_comments.
export interface Discussion {
  num: number
  title: string
  author: string
  created_at: string
  status: string
  is_pull_request: boolean
  num_comments: number
  num_reactions: number
  pinned: boolean
}

export interface DiscussionComment {
  author: string
  created_at: string
  content: string
}
</script>
<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { marked } from 'marked'
import DOMPurify from 'dompurify'
import { MessageSquare, MessagesSquare, Pin, ThumbsUp, X } from '@lucide/vue'
import { t } from '../i18n'
import { relativeTime, type HfRepo } from '../views/DiscoverView.vue'
import { routeReadmeImages } from '../lib/hfimg'

const props = defineProps<{
  repo: HfRepo
  list: Discussion[] | null
  listError: string | null
}>()

const emit = defineEmits<{ (e: 'close'): void }>()

const openNum = ref<number | null>(null)
const threadComments = ref<Record<number, DiscussionComment[]>>({})
const threadLoading = ref<Record<number, boolean>>({})
const threadError = ref<Record<number, string | null>>({})

const list = computed(() => props.list ?? [])
const pending = computed(() => props.list === null && !props.listError)
// Cuerpo del comentario: markdown → sanitizado → imgs por hfimg (mismo pipeline que el README).
function htmlOf(md: string): string {
  return routeReadmeImages(DOMPurify.sanitize(marked.parse(md, { async: false }) as string))
}

function timeOf(iso: string): string {
  const rel = relativeTime(iso)
  return rel || t('discover.now')
}

// Abre/cierra el hilo; los comentarios se fetchean una sola vez (cache en threadComments).
async function toggle(d: Discussion) {
  if (openNum.value === d.num) {
    openNum.value = null
    return
  }
  openNum.value = d.num
  if (d.num in threadComments.value || threadLoading.value[d.num]) return
  threadLoading.value[d.num] = true
  threadError.value[d.num] = null
  const [owner, name] = props.repo.id.split('/')
  try {
    threadComments.value[d.num] = await invoke<DiscussionComment[]>('get_repo_discussion_comments', { owner, repo: name, num: d.num })
  } catch (e) {
    threadError.value[d.num] = String(e)
  } finally {
    threadLoading.value[d.num] = false
  }
}

function close() {
  emit('close')
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') close()
}

onMounted(() => document.addEventListener('keydown', onKeydown))
onUnmounted(() => document.removeEventListener('keydown', onKeydown))
</script>

<style scoped>
.disc-overlay {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.55);
  z-index: 600;
  display: flex;
  align-items: center;
  justify-content: center;
}

.disc-modal {
  width: min(780px, 92%);
  height: 84%;
  background: #1e1e1e;
  border: 1px solid #333;
  border-radius: 10px;
  display: flex;
  flex-direction: column;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.6);
}

.disc-head {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 14px;
  border-bottom: 1px solid #2a2a2a;
}

.disc-title {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 8px;
  color: #fff;
  font-size: 14px;
  font-weight: 600;
}

.disc-title-count {
  background: rgba(90, 138, 245, 0.25);
  color: #85aef9;
  border-radius: 8px;
  padding: 0 7px;
  font-size: 11px;
}

.disc-close {
  background: rgba(255, 107, 107, 0.12);
  border: 1px solid rgba(255, 107, 107, 0.4);
  border-radius: 7px;
  color: #ff8f8f;
  width: 30px;
  height: 30px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.15s;
}

.disc-close:hover {
  background: rgba(255, 107, 107, 0.25);
  border-color: #ff6b6b;
  color: #ffc2c2;
}

.disc-body {
  flex: 1;
  overflow-y: auto;
  padding: 12px 14px;
}

.disc-status {
  color: #666;
  font-size: 13px;
  text-align: center;
  padding: 16px 0;
  word-break: break-word;
}

.disc-status-error {
  color: #f55a5a;
}

.disc-item {
  margin-bottom: 8px;
}

.disc-row {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 10px;
  text-align: left;
  background: #1a1a1a;
  border: 1px solid #2a2a2a;
  border-radius: 8px;
  padding: 10px 12px;
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s;
}

.disc-row:hover {
  border-color: #3a5aa0;
}

.disc-row.open {
  border-color: #5a8af5;
  background: #1a1a2a;
}

.disc-row-main {
  flex: 1;
  min-width: 0;
}

.disc-row-title {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 6px;
}

.disc-row-name {
  color: #e8e8e8;
  font-size: 13px;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 100%;
}

.disc-name-closed {
  color: #999;
}

.disc-badge {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: 10px;
  font-weight: 600;
  border-radius: 4px;
  padding: 1px 6px;
  flex-shrink: 0;
}

.disc-badge-pin { background: #2a2416; color: #e0c35a; }
.disc-badge-pr { background: #1a2a3a; color: #6ab0f5; }
.disc-badge-closed { background: #2a1a1a; color: #f58a8a; }

.disc-row-meta {
  color: #777;
  font-size: 11px;
  margin-top: 3px;
}

.disc-row-stats {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}

.disc-stat {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: #999;
  font-size: 12px;
}

.disc-thread {
  margin-top: 8px;
  margin-left: 14px;
  border-left: 2px solid #2a2a2a;
  padding-left: 12px;
}

.disc-comment {
  margin: 10px 0;
}

.disc-comment-head {
  display: flex;
  align-items: baseline;
  gap: 8px;
  margin-bottom: 4px;
}

.disc-comment-author {
  color: #d4d4d4;
  font-size: 12px;
  font-weight: 600;
}

.disc-comment-time {
  color: #666;
  font-size: 11px;
}
.disc-comment-body {
  color: #ccc;
  font-size: 13px;
  line-height: 1.55;
  word-wrap: break-word;
}

.disc-comment-body :deep(p) { margin: 6px 0; }
.disc-comment-body :deep(a) { color: #5a8af5; text-decoration: none; }
.disc-comment-body :deep(a:hover) { text-decoration: underline; }
.disc-comment-body :deep(img) { max-width: 100%; height: auto; border-radius: 6px; }
.disc-comment-body :deep(code) { background: #262626; padding: 1px 5px; border-radius: 4px; font-size: 12px; }
.disc-comment-body :deep(pre) {
  background: #0d0d0d;
  border: 1px solid #2a2a2a;
  border-radius: 8px;
  padding: 10px;
  overflow-x: auto;
  margin: 8px 0;
}
.disc-comment-body :deep(pre code) { background: none; padding: 0; }
.disc-comment-body :deep(blockquote) { border-left: 3px solid #333; margin: 8px 0; padding: 2px 12px; color: #999; }
.disc-comment-body :deep(ul),
.disc-comment-body :deep(ol) { margin: 6px 0; padding-left: 22px; }
</style>
