<template>
  <div class="modal-overlay" @click.self="$emit('close')">
    <div class="modal-safe" @click.stop>
    <div class="modal" :class="{ 'modal-wide': view === 'config' }">
      
      <!-- Vista: Lista -->
      <template v-if="view === 'list'">
        <div class="modal-header">
          <input class="search-box" v-model="search" :placeholder="t('modal.filter')" autofocus />
          <button class="modal-close" @click="$emit('close')">✕</button>
        </div>

        <!-- Modelo cargado actualmente -->
        <div v-if="loadedModel" class="modal-section-title">{{ t('modal.currentlyLoaded') }} (1)</div>
        <div v-if="loadedModel" class="modal-model-row loaded">
          <span class="pin-indicator" v-if="modelMeta[loadedModel.path]?.pinned">📌</span>
          <span class="modal-model-name">{{ modelDisplayNames[loadedModel.path] || loadedModel.name }}</span>
          <span class="tag quant">{{ loadedModel.name.split('-').pop()?.replace('.gguf','').replace('.GGUF','') }}</span>
          <span class="tag" style="background:#1a2a1a; color:#4af54a;">GGUF</span>
          <div style="flex:1"></div>
          <button class="btn-eject" @click.stop="eject">⏏ {{ t('modal.eject') }}</button>
        </div>

        <!-- Lista agrupada -->
        <div class="modal-model-list">
          <template v-for="section in groupedFilteredModels" :key="section.groupName">
            <div v-if="section.models.length > 0" class="modal-section-title">
              {{ section.groupName }}
            </div>
            <div
              v-for="model in section.models"
              :key="model.path"
              class="modal-model-row"
              :class="{ active: loadedModel?.path === model.path }"
              @click="onModelClick($event, model)"
            >
              <span class="pin-indicator" v-if="modelMeta[model.path]?.pinned">📌</span>
              <span class="modal-model-name">{{ modelDisplayNames[model.path] || model.name }}</span>
              <span class="tag quant">{{ model.name.split('-').pop()?.replace('.gguf','').replace('.GGUF','') }}</span>
              <span class="tag" style="background:#2a2a2a; color:#888;">GGUF</span>
              <div style="flex:1"></div>
              <span style="color:#555; font-size:11px;">{{ (model.size_bytes / 1024 / 1024 / 1024).toFixed(2) }} GB</span>
            </div>
          </template>
        </div>
      </template>

      <!-- Vista: Config -->
      <template v-else-if="view === 'config' && configModel && tempCfg">
        <div class="modal-header">
          <button class="btn-secondary" @click="view = 'list'">{{ t('modal.back') }}</button>
          <span style="color:#fff; font-size:13px; font-weight:600; flex:1; text-align:center;">
            {{ modelDisplayNames[configModel.path] || configModel.name }}
          </span>
          <button class="modal-close" @click="$emit('close')">✕</button>
        </div>

        <div class="modal-config-body">
          <div class="panel-section">
            <div class="section-title">⚙ {{ t('load.contextAndOffload') }}</div>
            <div class="field">
              <label>{{ t('load.contextLength') }}</label>
              <input type="number" v-model="tempCfg.contextLength" class="field-input" />
            </div>
            <input type="range" min="512" :max="configModel.max_context || 262144" step="512" v-model="tempCfg.contextLength" class="slider full-width" />
            <div class="field">
              <label>{{ t('load.gpuOffload') }}</label>
              <input type="range" min="0" :max="configModel?.layer_count ?? 999" v-model="tempCfg.gpuOffload" class="slider" />
              <span class="slider-value">{{ tempCfg.gpuOffload }}</span>
            </div>
          </div>

          <div class="panel-section">
            <div class="section-title">≋ {{ t('load.advanced') }}</div>
            <div class="field">
              <label>{{ t('load.cpuThreads') }}</label>
              <input type="number" v-model="tempCfg.cpuThreads" class="field-input" min="1" max="128" />
            </div>
            <div class="field">
              <label>{{ t('load.evalBatch') }}</label>
              <input type="number" v-model="tempCfg.evalBatch" class="field-input" />
            </div>
            <div class="field">
              <label>{{ t('load.physicalBatch') }}</label>
              <input type="number" v-model="tempCfg.physicalBatch" class="field-input" />
            </div>
            <div class="field">
              <label>{{ t('load.parallelSlots') }}</label>
              <input type="number" v-model="tempCfg.parallel" class="field-input" min="1" max="16" />
            </div>
            <div class="field">
              <label>{{ t('load.flashAttention') }}</label>
              <input type="checkbox" v-model="tempCfg.flashAttention" class="toggle" />
            </div>
            <div class="field">
              <label>{{ t('load.mlock') }}</label>
              <input type="checkbox" v-model="tempCfg.mlock" class="toggle" />
            </div>
            <div class="field">
              <label>{{ t('load.mmap') }}</label>
              <input type="checkbox" v-model="tempCfg.mmap" class="toggle" />
            </div>
            <div class="field">
              <label>{{ t('load.kvUnified') }}</label>
              <input type="checkbox" v-model="tempCfg.kvUnified" class="toggle" />
            </div>
            <div class="field">
              <label>{{ t('load.kvOffload') }}</label>
              <input type="checkbox" v-model="tempCfg.kvOffload" class="toggle" />
            </div>
            <div class="field">
              <label>{{ t('load.cacheRam') }}</label>
              <input type="number" v-model="tempCfg.cacheRam" class="field-input" min="-1" />
            </div>
            <div v-if="cacheRamWarning === 'unlimited'" style="color:#f5a55a; font-size:11px; margin-bottom:8px;">
              {{ t('load.cacheRamUnlimited') }}
            </div>
            <div v-else-if="cacheRamWarning === 'critical'" style="color:#f55a5a; font-size:11px; margin-bottom:8px;">
              {{ t('load.cacheRamCritical', { pct: cacheRamPct }) }}
            </div>
            <div v-else-if="cacheRamWarning === 'warning'" style="color:#f5a55a; font-size:11px; margin-bottom:8px;">
              {{ t('load.cacheRamWarning', { pct: cacheRamPct }) }}
            </div>
            <div class="field" v-if="configModel?.is_moe" :title="t('load.numExpertsTooltip')">
              <label>{{ t('load.numExperts') }}</label>
              <input type="number" v-model="tempCfg.expertsPerToken" class="field-input" min="0" :max="configModel?.expert_count || undefined" :placeholder="configModel?.expert_used_count > 0 ? String(configModel?.expert_used_count) : ''" />
            </div>
            <div class="field">
              <label>{{ t('load.cpuMoE') }}</label>
              <input type="number" v-model="tempCfg.nCpuMoe" class="field-input" min="0" />
            </div>
            <div class="field">
              <label>{{ t('load.speculativeDecoding') }}</label>
              <select class="field-select" v-model="tempCfg.specType">
                <option value="None">None</option>
                <option value="MTP">MTP</option>
                <option value="Draft">Draft Model</option>
              </select>
            </div>
            <template v-if="tempCfg.specType !== 'None'">
              <div class="subpanel">
                <template v-if="tempCfg.specType === 'Draft'">
                  <div class="field">
                    <label>{{ t('load.draftModel') }}</label>
                  </div>
                  <select class="field-select" style="width:100%; margin-bottom:8px;" v-model="tempCfg.draftModelPath">
                    <option value="">{{ t('load.selectDraft') }}</option>
                    <option v-for="m in draftModels" :key="m.path" :value="m.path">
                      {{ m.name }}
                    </option>
                  </select>
                  <div class="field">
                    <label>{{ t('load.draftType') }}</label>
                    <select class="field-select" v-model="tempCfg.draftSpecType">
                      <option value="simple">Simple</option>
                      <option value="mtp">MTP</option>
                      <option value="dflash">DFlash</option>
                    </select>
                  </div>
                </template>
                <div class="field" :title="t('load.maxDraftTokensTooltip')">
                  <label>{{ t('load.maxDraftTokens') }}</label>
                  <input type="number" v-model.number="tempCfg.draftParams[draftKind].maxDraftTokens" class="field-input" min="1" />
                </div>
                <div class="field" :title="t('load.minDraftTokensTooltip')">
                  <label>{{ t('load.minDraftTokens') }}</label>
                  <input type="number" v-model.number="tempCfg.draftParams[draftKind].minDraftTokens" min="0" step="1" class="field-input" />
                </div>
                <div class="field" :title="t('load.draftProbabilityTooltip')">
                  <label>{{ t('load.draftProbability') }}</label>
                  <input type="number" v-model.number="tempCfg.draftParams[draftKind].probability" step="0.05" class="field-input" />
                </div>
                <div class="field" :title="t('load.draftSplitProbabilityTooltip')">
                  <label>{{ t('load.draftSplitProbability') }}</label>
                  <input type="number" v-model.number="tempCfg.draftParams[draftKind].splitProbability" min="0" max="1" step="0.05" class="field-input" />
                </div>
                <details class="delicate-zone">
                  <summary>{{ t('load.draftKvQuantZone') }}</summary>
                  <div class="field" :title="t('load.draftKCacheQuantTooltip')">
                    <label>{{ t('load.draftKCacheQuant') }}</label>
                    <select class="field-select" v-model="tempCfg.draftParams[draftKind].kCacheQuant">
                      <option>F16</option>
                      <option>F32</option>
                      <option>BF16</option>
                      <option>Q8_0</option>
                      <option>Q4_0</option>
                      <option>Q4_1</option>
                      <option>IQ4_NL</option>
                      <option>Q5_0</option>
                      <option>Q5_1</option>
                    </select>
                  </div>
                  <div class="field" :title="t('load.draftVCacheQuantTooltip')">
                    <label>{{ t('load.draftVCacheQuant') }}</label>
                    <select class="field-select" v-model="tempCfg.draftParams[draftKind].vCacheQuant">
                      <option>F16</option>
                      <option>F32</option>
                      <option>BF16</option>
                      <option>Q8_0</option>
                      <option>Q4_0</option>
                      <option>Q4_1</option>
                      <option>IQ4_NL</option>
                      <option>Q5_0</option>
                      <option>Q5_1</option>
                    </select>
                  </div>
                  <div v-if="ngramK4vAvailable" class="field" :title="t('load.dflashNgramK4vTooltip')">
                    <label>{{ t('load.dflashNgramK4v') }}</label>
                    <input type="checkbox" v-model="tempCfg.dflashNgramK4v" class="toggle" />
                  </div>
                  <template v-if="ngramK4vAvailable && tempCfg.dflashNgramK4v">
                    <div class="field" :title="t('load.ngramK4vSizeNTooltip')">
                      <label>{{ t('load.ngramK4vSizeN') }}</label>
                      <input type="number" v-model.number="tempCfg.ngramK4vSizeN" min="1" class="field-input" />
                    </div>
                    <div class="field" :title="t('load.ngramK4vSizeMTooltip')">
                      <label>{{ t('load.ngramK4vSizeM') }}</label>
                      <input type="number" v-model.number="tempCfg.ngramK4vSizeM" min="1" class="field-input" />
                    </div>
                    <div class="field" :title="t('load.ngramK4vMinHitsTooltip')">
                      <label>{{ t('load.ngramK4vMinHits') }}</label>
                      <input type="number" v-model.number="tempCfg.ngramK4vMinHits" min="1" class="field-input" />
                    </div>
                  </template>
                  <div v-if="ngramModAvailable" class="field" :title="t('load.ngramModTooltip')">
                    <label>{{ t('load.ngramMod') }}</label>
                    <input type="checkbox" v-model="tempCfg.ngramMod" class="toggle" />
                  </div>
                  <template v-if="ngramModAvailable && tempCfg.ngramMod">
                    <div class="field" :title="t('load.ngramModNMatchTooltip')">
                      <label>{{ t('load.ngramModNMatch') }}</label>
                      <input type="number" v-model.number="tempCfg.ngramModNMatch" min="1" class="field-input" />
                    </div>
                    <div class="field" :title="t('load.ngramModNMinTooltip')">
                      <label>{{ t('load.ngramModNMin') }}</label>
                      <input type="number" v-model.number="tempCfg.ngramModNMin" min="1" class="field-input" />
                    </div>
                    <div class="field" :title="t('load.ngramModNMaxTooltip')">
                      <label>{{ t('load.ngramModNMax') }}</label>
                      <input type="number" v-model.number="tempCfg.ngramModNMax" min="1" class="field-input" />
                    </div>
                  </template>
                </details>
              </div>
            </template>
            <div class="field">
              <label>{{ t('load.kCacheQuant') }}</label>
              <select class="field-select" v-model="tempCfg.kCacheQuant">
                <option>F16</option>
                <option>Q8_0</option>
                <option>Q4_0</option>
              </select>
            </div>
            <div class="field">
              <label>{{ t('load.vCacheQuant') }}</label>
              <select class="field-select" v-model="tempCfg.vCacheQuant">
                <option>F16</option>
                <option>Q8_0</option>
                <option>Q4_0</option>
              </select>
            </div>
            <div class="field" :title="t('load.cacheReuseTooltip')">
              <label>{{ t('load.cacheReuse') }}</label>
              <input type="number" v-model="tempCfg.cacheReuse" class="field-input" min="0" />
            </div>
            <div class="field" :title="t('load.ctxCheckpointsTooltip')">
              <label>{{ t('load.ctxCheckpoints') }}</label>
              <input type="number" v-model="tempCfg.ctxCheckpoints" class="field-input" min="0" />
            </div>
            <div class="field" :title="t('load.checkpointMinStepTooltip')">
              <label>{{ t('load.checkpointMinStep') }}</label>
              <input type="number" v-model="tempCfg.checkpointMinStep" class="field-input" min="0" />
            </div>
          </div>

          <template v-if="configModel?.mmproj_paths?.length > 0">
            <div class="panel-section">
              <div class="section-title">👁️ {{ t('load.vision') }}</div>
              <div class="field">
                <label>{{ t('load.visionLabel') }}</label>
                <input type="checkbox" v-model="tempCfg.visionEnabled" class="toggle" />
              </div>
              <div class="field" v-if="tempCfg.visionEnabled">
                <label>{{ t('load.mmprojModel') }}</label>
                <select class="field-select" v-model="tempCfg.mmprojPath" style="max-width:160px; font-size:10px;">
                  <option v-for="p in configModel.mmproj_paths" :key="p" :value="p">
                    {{ p.split('\\').pop() }}
                  </option>
                </select>
              </div>
            </div>
          </template>

          <div class="panel-section">
            <div class="section-title">🧠 {{ t('load.reasoning') }}</div>
            <div class="field" v-if="configModel?.supports_thinking" :title="t('load.thinkingModeTooltip')">
              <label>{{ t('load.thinkingMode') }}</label>
              <select class="field-select" v-model="tempCfg.reasoning">
                <option value="auto">Auto</option>
                <option value="on">On</option>
                <option value="off">Off (disabled)</option>
              </select>
            </div>
            <div class="field">
              <label>{{ t('load.reasoningBudget') }}</label>
              <select class="field-select" v-model="tempCfg.reasoningBudget">
                <option value="-1">Unrestricted (default)</option>
                <option value="0">Off (disabled)</option>
                <option value="1024">1024 tokens</option>
                <option value="4096">4096 tokens</option>
                <option value="8192">8192 tokens</option>
                <option value="16384">16384 tokens</option>
                <option value="custom">{{ t('load.customBudget') }}</option>
              </select>
            </div>
            <div class="field-extra" v-if="tempCfg.reasoningBudget === 'custom'" :title="t('load.customBudgetTooltip')">
              <input type="number" v-model.number="tempCfg.reasoningBudgetCustom" min="1" step="256" class="field-input" />
            </div>
            <div class="field" v-if="configModel?.supports_effort">
              <label>{{ t('load.reasoningEffort') }}</label>
              <select class="field-select" v-model="tempCfg.reasoningEffort">
                <option v-for="lvl in effortOptions" :key="lvl" :value="lvl">{{ lvl }}</option>
              </select>
            </div>
          </div>

          <div class="panel-section">
            <div class="section-title">🌐 {{ t('load.server') }}</div>
            <div class="field">
              <label>{{ t('load.host') }}</label>
              <select class="field-select" v-model="tempCfg.host">
                <option value="127.0.0.1">localhost (127.0.0.1)</option>
                <option value="0.0.0.0">All interfaces (0.0.0.0)</option>
              </select>
            </div>
            <div class="field">
              <label>{{ t('load.alias') }}</label>
              <input type="text" v-model="tempCfg.alias" class="field-input" placeholder="optional" />
            </div>
            <div class="field">
              <label>{{ t('load.httpThreads') }}</label>
              <input type="number" v-model="tempCfg.threadsHttp" class="field-input" min="1" max="8" />
            </div>
            <div class="field">
              <label>{{ t('load.noWarmup') }}</label>
              <input type="checkbox" v-model="tempCfg.noWarmup" class="toggle" />
            </div>
            <div class="field">
              <label>{{ t('load.sleepIdle') }}</label>
              <input type="number" v-model="tempCfg.sleepIdle" class="field-input" min="-1" />
            </div>
            <div class="field">
              <label>{{ t('load.reasoningPreserve') }}</label>
              <input type="checkbox" v-model="tempCfg.reasoningPreserve" class="toggle" />
            </div>
            <div class="field">
              <label>{{ t('load.fit') }}</label>
              <select class="field-select" v-model="tempCfg.fit">
                <option value="on">On</option>
                <option value="off">Off</option>
              </select>
            </div>
          </div>
        </div>

        <div class="modal-config-footer">
          <button class="btn-secondary" @click="view = 'list'">{{ t('modal.back') }}</button>
          <button class="btn-load" style="width:auto; padding: 6px 24px;" @click="loadWithConfig">
            {{ t('modal.load') }}
          </button>
        </div>
      </template>

    </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { allModels, loadedModel, selectedModel, modelLoading, loadingModel, loadedModelConfig, loadedServerPort } from '../stores/selectedModel'
import { modelDisplayNames, modelMeta, groups } from '../stores/groups'
import { invoke } from '@tauri-apps/api/core'
import { loadConfig, loadModelConfig, saveModelConfig, type ModelConfig, defaultDraftParams, activeSpecKind } from '../stores/config'
import { t } from '../i18n'
import type { ModelFile } from '../stores/selectedModel'

const search = ref('')
const view = ref<'list' | 'config'>('list')
const configModel = ref<ModelFile | null>(null)

const effortOptions = computed(() => {
  const levels = configModel.value?.supported_effort_levels
  if (levels && levels.length > 0) return ['default', ...levels]
  return ['default', 'minimal', 'low', 'medium', 'high', 'xhigh', 'max']
})
const tempCfg = ref<ModelConfig | null>(null)
const emit = defineEmits(['close'])

const draftKind = computed(() => tempCfg.value ? activeSpecKind(tempCfg.value) : 'simple')

const ngramK4vAvailable = computed(() => {
  if (!tempCfg.value) return false
  return tempCfg.value.specType === 'MTP' || (tempCfg.value.specType === 'Draft' && tempCfg.value.draftSpecType === 'dflash')
})

const ngramModAvailable = computed(() => tempCfg.value ? tempCfg.value.specType !== 'None' : false)

const systemRamTotal = ref(0)
const systemRamAvailable = ref(0)

async function refreshSystemRam() {
  const [total, available] = await invoke<[number, number]>('get_system_ram')
  systemRamTotal.value = total
  systemRamAvailable.value = available
}

onMounted(() => {
  refreshSystemRam()
  window.addEventListener('focus', refreshSystemRam)
})

onUnmounted(() => {
  window.removeEventListener('focus', refreshSystemRam)
})

const cacheRamPct = computed(() => {
  if (!systemRamAvailable.value || !tempCfg.value || tempCfg.value.cacheRam <= 0) return 0
  return Math.round((tempCfg.value.cacheRam / systemRamAvailable.value) * 100)
})

const cacheRamWarning = computed(() => {
  const v = tempCfg.value?.cacheRam ?? 0
  if (v === -1) return 'unlimited'
  if (v <= 0 || !systemRamAvailable.value) return null
  if (v > systemRamAvailable.value) return 'critical'
  if (v > systemRamAvailable.value * 0.8) return 'warning'
  return null
})

const groupedFilteredModels = computed(() => {
  const filtered = allModels.value.filter(m =>
    m.path !== loadedModel.value?.path &&
    (m.name.toLowerCase().includes(search.value.toLowerCase()) ||
     (modelDisplayNames[m.path] ?? '').toLowerCase().includes(search.value.toLowerCase()))
  )

  const sortModel = (a: ModelFile, b: ModelFile) => {
    const aPinned = modelMeta[a.path]?.pinned ?? false
    const bPinned = modelMeta[b.path]?.pinned ?? false
    if (aPinned !== bPinned) return aPinned ? -1 : 1
    return (modelMeta[a.path]?.order ?? 0) - (modelMeta[b.path]?.order ?? 0)
  }

  const result: { groupName: string, models: ModelFile[] }[] = []

  const sortedGroups = [...groups.value].sort((a, b) => a.order - b.order)
  for (const group of sortedGroups) {
    const groupModels = filtered
      .filter(m => modelMeta[m.path]?.groupId === group.id)
      .sort(sortModel)
    result.push({ groupName: `📁 ${group.name}`, models: groupModels })
  }

  const ungrouped = filtered
    .filter(m => !modelMeta[m.path]?.groupId || !groups.value.find(g => g.id === modelMeta[m.path]?.groupId))
    .sort(sortModel)
  
  if (ungrouped.length > 0) {
    result.push({ groupName: groups.value.length > 0 ? 'Ungrouped' : 'Your Models', models: ungrouped })
  }

  return result
})

const draftModels = computed(() => {
  if (!configModel.value) return []
  return allModels.value.filter(m =>
    m.model_family === configModel.value!.model_family &&
    m.path !== configModel.value!.path
  )
})

async function onModelClick(e: MouseEvent, model: ModelFile) {
  if (e.altKey) {
    configModel.value = model
    const cfg = { ...await loadModelConfig(model.path) }
    if (cfg.reasoningEffort !== 'default' && model.supported_effort_levels?.length && !model.supported_effort_levels.includes(cfg.reasoningEffort)) {
      cfg.reasoningEffort = 'default'
    }
    tempCfg.value = cfg
    view.value = 'config'
  } else {
    selectModel(model)
  }
}

async function invokeLoad(modelPath: string, cfg: ModelConfig) {
  const config = await loadConfig()
  const cpuThreads = cfg.cpuThreads ?? 0
  const resolvedThreads = cpuThreads > 0 ? cpuThreads : await invoke<number>('get_cpu_threads')
  const gpuLayers = cfg.gpuOffload ?? 0
  const resolvedGpu = gpuLayers > 0 ? gpuLayers : (configModel.value?.layer_count ?? 999)
  const dp = cfg.draftParams?.[activeSpecKind(cfg)] ?? defaultDraftParams
  await invoke('load_model', {
    llamaPath: config.llamaPath,
    cudaGraphOpt: config.cudaGraphOpt ?? '',
    logVerbosity: Number(config.logVerbosity ?? 3),
    modelPath,
    gpuLayers: resolvedGpu,
    contextLength: Number(cfg.contextLength ?? 4096),
    cpuThreads: resolvedThreads,
    evalBatch: Number(cfg.evalBatch ?? 2048),
    physicalBatch: Number(cfg.physicalBatch ?? 512),
    flashAttention: cfg.flashAttention ?? true,
    specType: cfg.specType ?? 'None',
    draftSpecType: cfg.draftSpecType ?? 'simple',
    draftModelPath: cfg.draftModelPath ?? '',
    maxDraftTokens: Number(dp.maxDraftTokens ?? 2),
    minDraftTokens: Number(dp.minDraftTokens ?? 0),
    draftProbability: Number(dp.probability ?? 0.75),
    draftSplitProbability: Number(dp.splitProbability ?? 0.10),
    dflashNgramK4v: cfg.dflashNgramK4v ?? false,
    ngramK4vSizeN: Number(cfg.ngramK4vSizeN ?? 12),
    ngramK4vSizeM: Number(cfg.ngramK4vSizeM ?? 48),
    ngramK4vMinHits: Number(cfg.ngramK4vMinHits ?? 1),
    ngramMod: cfg.ngramMod ?? false,
    ngramModNMatch: Number(cfg.ngramModNMatch ?? 24),
    ngramModNMin: Number(cfg.ngramModNMin ?? 48),
    ngramModNMax: Number(cfg.ngramModNMax ?? 64),
    kCacheQuant: cfg.kCacheQuant ?? 'Q8_0',
    vCacheQuant: cfg.vCacheQuant ?? 'Q8_0',
    draftKCacheQuant: dp.kCacheQuant ?? 'F16',
    draftVCacheQuant: dp.vCacheQuant ?? 'F16',
    cacheReuse: Number(cfg.cacheReuse ?? 0),
    ctxCheckpoints: Number(cfg.ctxCheckpoints ?? 32),
    checkpointMinStep: Number(cfg.checkpointMinStep ?? 8192),
    port: Number(config.port ?? 8080),
    host: cfg.host ?? '127.0.0.1',
    alias: cfg.alias ?? '',
    threadsHttp: Number(cfg.threadsHttp ?? 2),
    noWarmup: cfg.noWarmup ?? false,
    sleepIdle: Number(cfg.sleepIdle ?? -1),
    reasoningPreserve: cfg.reasoningPreserve ?? false,
    fit: cfg.fit ?? 'on',
    reasoning: cfg.reasoning ?? 'auto',
    reasoningBudget: cfg.reasoningBudget === 'custom' ? Number(cfg.reasoningBudgetCustom ?? 2048) : Number(cfg.reasoningBudget ?? -1),
    reasoningEffort: cfg.reasoningEffort ?? 'default',
    parallel: Number(cfg.parallel ?? 1),
    mlock: cfg.mlock ?? false,
    nCpuMoe: Number(cfg.nCpuMoe ?? 0),
    expertsPerToken: Number(cfg.expertsPerToken ?? 0),
    visionEnabled: cfg.visionEnabled ?? false,
    mmprojPath: cfg.mmprojPath ?? '',
    mmap: cfg.mmap ?? false,
    kvUnified: cfg.kvUnified ?? false,
    kvOffload: cfg.kvOffload ?? false,
    cacheRam: Number(cfg.cacheRam ?? 0),
    seed: Number(cfg.seed ?? -1),
    temp: Number(cfg.temp ?? 0.8),
    topP: Number(cfg.topP ?? 0.95),
    topK: Number(cfg.topK ?? 40),
    minP: Number(cfg.minP ?? 0.05),
    repeatPenalty: Number(cfg.repeatPenalty ?? 1.0),
  })
  loadedServerPort.value = Number(config.port ?? 8080)
}

async function selectModel(model: ModelFile) {
  selectedModel.value = model
  loadingModel.value = model
  emit('close')
  modelLoading.value = true
  const cfg = await loadModelConfig(model.path)
  if (cfg.reasoningEffort !== 'default' && model.supported_effort_levels?.length && !model.supported_effort_levels.includes(cfg.reasoningEffort)) {
    cfg.reasoningEffort = 'default'
  }
  try {
    await invokeLoad(model.path, cfg)
  } catch (e) {
    modelLoading.value = false
    console.error(e)
  }
}

async function loadWithConfig() {
  if (!configModel.value || !tempCfg.value) return

  await saveModelConfig(configModel.value.path, tempCfg.value)
  
  selectedModel.value = configModel.value
  loadingModel.value = configModel.value
  emit('close')
  modelLoading.value = true
  try {
    await invokeLoad(configModel.value.path, tempCfg.value)
  } catch (e) {
    modelLoading.value = false
  }
}

async function eject() {
  await invoke('stop_model')
  loadedModel.value = null
  loadedServerPort.value = null
  modelLoading.value = false
  emit('close')
}
</script>
