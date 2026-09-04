<template>
  <div class="right-panel">
    <div class="panel-header">
      <span class="model-title">{{ activeModel?.name ?? t('models.noModelSelected') }}</span>
      <div class="panel-tabs">
        <button class="tab" :class="{ active: activeTab === 'info' }" @click="activeTab = 'info'">{{ t('info.modelInfo') }}</button>
        <button class="tab" :class="{ active: activeTab === 'load' }" @click="activeTab = 'load'">{{ t('load.title') }}</button>
        <button class="tab" :class="{ active: activeTab === 'inference' }" @click="activeTab = 'inference'">{{ t('inference.settings') }}</button>
      </div>
    </div>

    <div v-if="activeTab === 'load'" class="panel-actions">
      <div v-if="error" style="color:#f55a5a; font-size:11px; margin-bottom:8px;">{{ error }}</div>
      <button 
        class="btn-load" 
        :class="{ 'btn-reload-changes': hasUnsavedChanges }"
        @click="loadModel" 
        :disabled="loading"
      >
        {{ loading ? t('load.loading') : hasUnsavedChanges ? t('load.reloadChanges') : ((currentView === 'developer' && loadedModel) ? t('load.reload') : t('load.loadModel')) }}
      </button>
      <button class="btn-secondary" style="width:100%; margin-top:6px;" @click="stopModel">
        {{ t('load.stop') }}
      </button>
    </div>

    <div class="panel-scroll">
    <div v-if="activeTab === 'load'">
      <div class="panel-section">
        <div class="section-title">⚙ {{ t('load.contextAndOffload') }}</div>
        <div class="field">
          <label>{{ t('load.contextLength') }}</label>
          <input type="number" v-model.number="modelCfg.contextLength" class="field-input" />
        </div>
        <div class="field">
          <label style="color:#555; font-size:11px;">{{ t('load.modelSupportsTokens', { max: maxContext }) }}</label>
        </div>
        <input 
          type="range" 
          min="512" 
          :max="maxContext" 
          step="512"
          v-model.number="modelCfg.contextLength" 
          class="slider full-width"
        />
        <div class="field">
          <label>{{ t('load.gpuOffload') }}</label>
          <input type="range" min="0" :max="activeModel?.layer_count ?? 999" v-model.number="modelCfg.gpuOffload" class="slider" />
          <span class="slider-value">{{ modelCfg.gpuOffload }}</span>
        </div>
      </div>

      <div class="panel-section">
        <div class="section-title">≋ {{ t('load.advanced') }}</div>
        <div class="field" :title="t('load.cpuThreadsTooltip')">
          <label>{{ t('load.cpuThreads') }}</label>
          <input type="number" v-model.number="modelCfg.cpuThreads" class="field-input" min="1" max="128" />
        </div>
        <div class="field">
          <label>{{ t('load.evalBatch') }}</label>
          <input type="number" v-model.number="modelCfg.evalBatch" class="field-input" />
        </div>
        <div class="field">
          <label>{{ t('load.physicalBatch') }}</label>
          <input type="number" v-model.number="modelCfg.physicalBatch" class="field-input" />
        </div>
        <div class="field">
          <label>{{ t('load.flashAttention') }}</label>
          <input type="checkbox" v-model="modelCfg.flashAttention" class="toggle" />
        </div>
        <div class="field">
          <label>{{ t('load.speculativeDecoding') }}</label>
          <select class="field-select" v-model="modelCfg.specType">
            <option value="None">{{ t('load.none') }}</option>
            <option value="MTP">{{ t('load.mtp') }}</option>
            <option value="Draft">{{ t('load.draftModel') }}</option>
          </select>
        </div>
        <template v-if="modelCfg.specType !== 'None'">
          <div class="subpanel">
            <template v-if="modelCfg.specType === 'Draft'">
              <div class="field">
                <label>{{ t('load.draftModel') }}</label>
              </div>
              <select class="field-select" style="width:100%; margin-bottom:8px;" v-model="modelCfg.draftModelPath">
                <option value="">{{ t('load.selectDraft') }}</option>
<option v-for="m in draftModels" :key="m.path" :value="m.path">
  {{ m.name }}
</option>
              </select>
              <div class="field">
                <label>{{ t('load.draftType') }}</label>
                <select class="field-select" v-model="modelCfg.draftSpecType">
                  <option value="simple">{{ t('load.draftSimple') }}</option>
                  <option value="mtp">{{ t('load.mtp') }}</option>
                  <option value="dflash">{{ t('load.draftDflash') }}</option>
                  <option value="eagle3">{{ t('load.draftEagle3') }}</option>
                  <option value="dspark">{{ t('load.draftDspark') }}</option>
                </select>
              </div>
            </template>
            <div class="field" :title="t('load.maxDraftTokensTooltip')">
              <label>{{ t('load.maxDraftTokens') }}</label>
              <input type="number" v-model.number="modelCfg.draftParams[draftKind].maxDraftTokens" class="field-input" />
            </div>
            <div class="field" :title="t('load.minDraftTokensTooltip')">
              <label>{{ t('load.minDraftTokens') }}</label>
              <input type="number" v-model.number="modelCfg.draftParams[draftKind].minDraftTokens" min="0" step="1" class="field-input" />
            </div>
            <div class="field" :title="t('load.draftProbabilityTooltip')">
              <label>{{ t('load.draftProbability') }}</label>
              <input type="number" v-model.number="modelCfg.draftParams[draftKind].probability" step="0.05" class="field-input" />
            </div>
            <div class="field" :title="t('load.draftSplitProbabilityTooltip')">
              <label>{{ t('load.draftSplitProbability') }}</label>
              <input type="number" v-model.number="modelCfg.draftParams[draftKind].splitProbability" min="0" max="1" step="0.05" class="field-input" />
            </div>
            <details class="delicate-zone">
              <summary>{{ t('load.draftKvQuantZone') }}</summary>
              <div class="field" :title="t('load.draftKCacheQuantTooltip')">
                <label>{{ t('load.draftKCacheQuant') }}</label>
                <select class="field-select" v-model="modelCfg.draftParams[draftKind].kCacheQuant">
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
                <select class="field-select" v-model="modelCfg.draftParams[draftKind].vCacheQuant">
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
                <input type="checkbox" v-model="modelCfg.dflashNgramK4v" class="toggle" />
              </div>
              <template v-if="ngramK4vAvailable && modelCfg.dflashNgramK4v">
                <div class="field" :title="t('load.ngramK4vSizeNTooltip')">
                  <label>{{ t('load.ngramK4vSizeN') }}</label>
                  <input type="number" v-model.number="modelCfg.ngramK4vSizeN" min="1" class="field-input" />
                </div>
                <div class="field" :title="t('load.ngramK4vSizeMTooltip')">
                  <label>{{ t('load.ngramK4vSizeM') }}</label>
                  <input type="number" v-model.number="modelCfg.ngramK4vSizeM" min="1" class="field-input" />
                </div>
                <div class="field" :title="t('load.ngramK4vMinHitsTooltip')">
                  <label>{{ t('load.ngramK4vMinHits') }}</label>
                  <input type="number" v-model.number="modelCfg.ngramK4vMinHits" min="1" class="field-input" />
                </div>
              </template>
              <div v-if="ngramModAvailable" class="field" :title="t('load.ngramModTooltip')">
                <label>{{ t('load.ngramMod') }}</label>
                <input type="checkbox" v-model="modelCfg.ngramMod" class="toggle" />
              </div>
              <template v-if="ngramModAvailable && modelCfg.ngramMod">
                <div class="field" :title="t('load.ngramModNMatchTooltip')">
                  <label>{{ t('load.ngramModNMatch') }}</label>
                  <input type="number" v-model.number="modelCfg.ngramModNMatch" min="1" class="field-input" />
                </div>
                <div class="field" :title="t('load.ngramModNMinTooltip')">
                  <label>{{ t('load.ngramModNMin') }}</label>
                  <input type="number" v-model.number="modelCfg.ngramModNMin" min="1" class="field-input" />
                </div>
                <div class="field" :title="t('load.ngramModNMaxTooltip')">
                  <label>{{ t('load.ngramModNMax') }}</label>
                  <input type="number" v-model.number="modelCfg.ngramModNMax" min="1" class="field-input" />
                </div>
              </template>
              <div v-if="ngramModAvailable" class="field" :title="t('load.ngramCacheTooltip')">
                <label>{{ t('load.ngramCache') }}</label>
                <input type="checkbox" v-model="modelCfg.ngramCache" class="toggle" />
              </div>
            </details>
          </div>
        </template>
        <div class="field">
          <label>{{ t('load.kCacheQuant') }}</label>
          <select class="field-select" v-model="modelCfg.kCacheQuant">
            <option value="F32">F32</option>
            <option value="F16">F16</option>
            <option value="Q8_0">Q8_0</option>
            <option value="Q4_0">Q4_0</option>
          </select>
        </div>
        <div class="field">
          <label>{{ t('load.vCacheQuant') }}</label>
          <select class="field-select" v-model="modelCfg.vCacheQuant">
            <option value="F32">F32</option>
            <option value="F16">F16</option>
            <option value="Q8_0">Q8_0</option>
            <option value="Q4_0">Q4_0</option>
          </select>
        </div>
        <div class="field" :title="t('load.cacheReuseTooltip')">
          <label>{{ t('load.cacheReuse') }}</label>
          <input type="number" v-model.number="modelCfg.cacheReuse" class="field-input" min="0" />
        </div>
        <div class="field" :title="t('load.ctxCheckpointsTooltip')">
          <label>{{ t('load.ctxCheckpoints') }}</label>
          <input type="number" v-model.number="modelCfg.ctxCheckpoints" class="field-input" min="0" />
        </div>
        <div class="field" :title="t('load.checkpointMinStepTooltip')">
          <label>{{ t('load.checkpointMinStep') }}</label>
          <input type="number" v-model.number="modelCfg.checkpointMinStep" class="field-input" min="0" />
        </div>
        <div class="field" :title="t('load.parallelTooltip')">
          <label>{{ t('load.parallelSlots') }}</label>
          <input type="number" v-model.number="modelCfg.parallel" class="field-input" min="1" max="16" />
        </div>
        <div class="field" :title="t('load.mlockTooltip')">
          <label>{{ t('load.mlock') }}</label>
          <input type="checkbox" v-model="modelCfg.mlock" class="toggle" />
        </div>
        <div class="field" :title="t('load.mmapTooltip')">
          <label>{{ t('load.mmap') }}</label>
          <input type="checkbox" v-model="modelCfg.mmap" class="toggle" />
        </div>
        <div class="field" :title="t('load.kvUnifiedTooltip')">
          <label>{{ t('load.kvUnified') }}</label>
          <input type="checkbox" v-model="modelCfg.kvUnified" class="toggle" />
        </div>
        <div class="field" :title="t('load.kvOffloadTooltip')">
          <label>{{ t('load.kvOffload') }}</label>
          <input type="checkbox" v-model="modelCfg.kvOffload" class="toggle" />
        </div>
        <div class="field" :title="t('load.cacheRamTooltip')">
          <label>{{ t('load.cacheRam') }}</label>
          <input type="number" v-model.number="modelCfg.cacheRam" class="field-input" min="-1" />
        </div>
        <input
          type="range"
          min="0"
          :max="systemRamTotal"
          step="256"
          v-model="modelCfg.cacheRam"
          class="slider full-width"
        />
        <div style="color:#555; font-size:11px; margin-bottom:4px;">
          {{ systemRamTotal > 0 ? t('load.systemRam', { total: systemRamTotal, free: systemRamAvailable }) : '' }}
        </div>
        <template v-if="activeModel?.path !== loadedModel?.path">
          <div v-if="cacheRamWarning === 'unlimited'" style="color:#f5a55a; font-size:11px; margin-bottom:8px;">
            {{ t('load.cacheRamUnlimited') }}
          </div>
          <div v-else-if="cacheRamWarning === 'critical'" style="color:#f55a5a; font-size:11px; margin-bottom:8px;">
            {{ t('load.cacheRamCritical', { pct: cacheRamPct }) }}
          </div>
          <div v-else-if="cacheRamWarning === 'warning'" style="color:#f5a55a; font-size:11px; margin-bottom:8px;">
            {{ t('load.cacheRamWarning', { pct: cacheRamPct }) }}
          </div>
        </template>
        <div class="field" :title="t('load.seedTooltip')">
          <label>{{ t('load.seed') }}</label>
          <input type="number" v-model.number="modelCfg.seed" class="field-input" />
        </div>
        <div class="field" v-if="activeModel?.is_moe" :title="t('load.cpuMoETooltip')">
          <label>{{ t('load.cpuMoE') }}</label>
          <input type="number" v-model.number="modelCfg.nCpuMoe" class="field-input" min="0" />
        </div>
        <div class="field" v-if="activeModel?.is_moe" :title="t('load.numExpertsTooltip')">
          <label>{{ t('load.numExperts') }}</label>
          <input type="number" v-model.number="modelCfg.expertsPerToken" class="field-input" min="0" :max="activeModel?.expert_count || undefined" :placeholder="activeModel?.expert_used_count > 0 ? String(activeModel?.expert_used_count) : ''" />
        </div>
      </div>

      <template v-if="activeModel && activeModel.mmproj_paths.length > 0">
        <div class="panel-section">
          <div class="section-title">👁️ {{ t('load.vision') }}</div>
          <div class="field" :title="t('load.visionTooltip')">
            <label>{{ t('load.visionLabel') }}</label>
            <input type="checkbox" v-model="modelCfg.visionEnabled" class="toggle" />
          </div>
          <div class="field" v-if="modelCfg.visionEnabled">
            <label>{{ t('load.mmprojModel') }}</label>
            <select class="field-select" v-model="modelCfg.mmprojPath" style="max-width:160px; font-size:10px;">
              <option v-for="p in activeModel.mmproj_paths" :key="p" :value="p">
                {{ p.split('\\').pop() }}
              </option>
            </select>
          </div>
        </div>
      </template>

      <div class="panel-section">
        <div class="section-title">🧠 {{ t('load.reasoning') }}</div>
        <div class="field" v-if="activeModel?.supports_thinking" :title="t('load.thinkingModeTooltip')">
          <label>{{ t('load.thinkingMode') }}</label>
          <select class="field-select" v-model="modelCfg.reasoning">
            <option value="auto">{{ t('load.auto') }}</option>
            <option value="on">{{ t('load.on') }}</option>
            <option value="off">{{ t('load.off') }}</option>
          </select>
        </div>
        <div class="field">
          <label>{{ t('load.reasoningBudget') }}</label>
          <select class="field-select" v-model="modelCfg.reasoningBudget">
            <option value="-1">{{ t('load.unrestricted') }}</option>
            <option value="0">{{ t('load.off') }}</option>
            <option value="1024">1024 tokens</option>
            <option value="4096">4096 tokens</option>
            <option value="8192">8192 tokens</option>
            <option value="16384">16384 tokens</option>
            <option value="custom">{{ t('load.customBudget') }}</option>
          </select>
        </div>
        <div class="field-extra" v-if="modelCfg.reasoningBudget === 'custom'" :title="t('load.customBudgetTooltip')">
          <input type="number" v-model.number="modelCfg.reasoningBudgetCustom" min="1" step="256" class="field-input" />
        </div>
        <div class="field" v-if="activeModel?.supports_effort">
          <label>{{ t('load.reasoningEffort') }}</label>
          <select class="field-select" v-model="modelCfg.reasoningEffort">
            <option v-for="lvl in effortOptions" :key="lvl" :value="lvl">{{ effortLabel(lvl) }}</option>
          </select>
        </div>
      </div>

      <div class="panel-section">
        <div class="section-title">🌐 {{ t('load.server') }}</div>
        <div class="field">
          <label>{{ t('load.host') }}</label>
          <select class="field-select" v-model="modelCfg.host">
            <option value="127.0.0.1">{{ t('load.localhost') }}</option>
            <option value="0.0.0.0">{{ t('load.allInterfaces') }}</option>
          </select>
        </div>
        <div class="field" :title="t('load.aliasTooltip')">
          <label>{{ t('load.alias') }}</label>
          <input type="text" v-model="modelCfg.alias" class="field-input" :placeholder="t('load.optional')" />
        </div>
        <div class="field" :title="t('load.httpThreadsTooltip')">
          <label>{{ t('load.httpThreads') }}</label>
          <input type="number" v-model.number="modelCfg.threadsHttp" class="field-input" min="1" max="8" />
        </div>
        <div class="field" :title="t('load.noWarmupTooltip')">
          <label>{{ t('load.noWarmup') }}</label>
          <input type="checkbox" v-model="modelCfg.noWarmup" class="toggle" />
        </div>
        <div class="field" :title="t('load.sleepIdleTooltip')">
          <label>{{ t('load.sleepIdle') }}</label>
          <input type="number" v-model.number="modelCfg.sleepIdle" class="field-input" min="-1" />
        </div>
        <div class="field" :title="t('load.reasoningPreserveTooltip')">
          <label>{{ t('load.reasoningPreserve') }}</label>
          <input type="checkbox" v-model="modelCfg.reasoningPreserve" class="toggle" />
        </div>
        <div class="field" :title="t('load.fitTooltip')">
          <label>{{ t('load.fit') }}</label>
          <select class="field-select" v-model="modelCfg.fit">
            <option value="on">{{ t('load.fitOn') }}</option>
            <option value="off">{{ t('load.fitOff') }}</option>
          </select>
        </div>
      </div>

    </div>

    <div v-if="activeTab === 'inference'">
      <div class="panel-section">
        <div class="field">
          <label>{{ t('inference.temperature') }}</label>
          <input type="number" v-model.number="modelCfg.temp" class="field-input" step="0.05" min="0" max="2" />
        </div>
        <div class="field">
          <label>{{ t('inference.topP') }}</label>
          <input type="number" v-model.number="modelCfg.topP" class="field-input" step="0.01" min="0" max="1" />
        </div>
        <div class="field">
          <label>{{ t('inference.topK') }}</label>
          <input type="number" v-model.number="modelCfg.topK" class="field-input" step="1" min="0" />
        </div>
        <div class="field">
          <label>{{ t('inference.minP') }}</label>
          <input type="number" v-model.number="modelCfg.minP" class="field-input" step="0.01" min="0" max="1" />
        </div>
        <div class="field">
          <label>{{ t('inference.repeatPenalty') }}</label>
          <input type="number" v-model.number="modelCfg.repeatPenalty" class="field-input" step="0.01" min="0" />
        </div>
      </div>
    </div>

    <div v-if="activeTab === 'info'">
      <div class="panel-section" v-if="activeModel">
        <div class="section-title">ⓘ {{ t('info.modelInfo') }}</div>
        
        <div class="info-row">
          <span class="info-label">{{ t('info.model') }}</span>
          <span class="info-value tag-pill">{{ activeModel.publisher }}/{{ activeModel.model_family }}</span>
        </div>
        <div class="info-row">
          <span class="info-label">{{ t('info.file') }}</span>
          <span class="info-value tag-pill">{{ activeModel.name }}</span>
        </div>
        <div class="info-row">
          <span class="info-label">{{ t('info.format') }}</span>
          <span class="info-value tag-pill">GGUF</span>
        </div>
        <div class="info-row">
          <span class="info-label">{{ t('info.quantization') }}</span>
          <span class="info-value tag-pill">{{ activeModel.name.split('-').pop()?.replace('.gguf','').replace('.GGUF','') }}</span>
        </div>
        <div class="info-row">
          <span class="info-label">{{ t('info.arch') }}</span>
          <div style="display:flex; gap:6px; flex-wrap:wrap;">
            <span class="info-value tag-pill">{{ activeModel.arch || '?' }}</span>
            <span class="info-value tag-pill" v-if="activeModel.name.toLowerCase().includes('mtp')">MTP</span>
          </div>
        </div>
        <div class="info-row">
          <span class="info-label">{{ t('info.params') }}</span>
          <span class="info-value tag-pill">{{ activeModel.params || '?' }}</span>
        </div>
        <div class="info-row">
          <span class="info-label">{{ t('info.maxContext') }}</span>
          <span class="info-value tag-pill">{{ activeModel.max_context?.toLocaleString() || '?' }}</span>
        </div>
        <div class="info-row">
          <span class="info-label">{{ t('info.sizeOnDisk') }}</span>
          <span class="info-value tag-pill">{{ (activeModel.size_bytes / 1024 / 1024 / 1024).toFixed(2) }} GB</span>
        </div>
      </div>

      <div class="panel-section" v-if="activeModel">
        <div class="section-title">🔗 {{ t('info.apiUsage') }}</div>
        <div class="info-label" style="margin-bottom:6px;">{{ t('info.serverReachable') }}</div>
        <div class="copy-row">
          <span class="tag-pill copy-pill">{{ serverUrl }}</span>
          <button class="btn-copy" @click="copy(serverUrl)">⧉</button>
        </div>
      </div>

      <div v-if="!activeModel" style="padding:16px; color:#555; font-size:12px;">
        {{ t('models.noModelSelected') }}
      </div>
    </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { selectedModel, allModels, modelLoading, loadedModel, loadingModel, loadedModelConfig, loadedServerPort, prefillProgress, generationTokens } from '../stores/selectedModel'
import { loadConfig, loadModelConfig, saveModelConfig, appConfig, type ModelConfig, defaultDraftParams, activeSpecKind, numOrDefault } from '../stores/config'
import { t } from '../i18n'

const props = defineProps<{ currentView?: string }>()

const activeTab = ref('load')

const activeModel = computed(() =>
  ((props.currentView === 'developer' || props.currentView === 'chat') && loadedModel.value) ? loadedModel.value : selectedModel.value
)

const serverUrl = computed(() => `http://127.0.0.1:${loadedServerPort.value ?? appConfig.value.port}`)

const hasUnsavedChanges = computed(() => {
  if (!loadedModelConfig.value || !loadedModel.value) return false
  if (activeModel.value?.path !== loadedModel.value.path) return false
  
  const keys: (keyof typeof modelCfg.value)[] = [
    'contextLength', 'gpuOffload', 'cpuThreads', 'evalBatch', 'physicalBatch',
    'flashAttention', 'specType', 'draftSpecType', 'dflashNgramK4v', 'ngramK4vSizeN', 'ngramK4vSizeM', 'ngramK4vMinHits', 'ngramMod', 'ngramModNMatch', 'ngramModNMin', 'ngramModNMax', 'ngramCache', 'kCacheQuant', 'vCacheQuant', 'cacheReuse',
    'ctxCheckpoints', 'checkpointMinStep',
    'reasoning', 'reasoningBudget', 'reasoningBudgetCustom', 'reasoningEffort', 'parallel', 'mlock', 'nCpuMoe', 'expertsPerToken',
    'mmap', 'kvUnified', 'seed', 'draftModelPath', 'threadsHttp', 'alias',
    'host', 'noWarmup', 'sleepIdle', 'reasoningPreserve', 'fit', 'visionEnabled', 'mmprojPath',
    'kvOffload', 'cacheRam', 'temp', 'topP', 'topK', 'minP', 'repeatPenalty'
  ]

  return JSON.stringify(modelCfg.value.draftParams) !== JSON.stringify(loadedModelConfig.value!.draftParams) ||
    keys.some(k => String(modelCfg.value[k]) !== String(loadedModelConfig.value![k]))
})

const effortOptions = computed(() => {
  const levels = activeModel.value?.supported_effort_levels
  if (levels && levels.length > 0) return ['default', ...levels]
  return ['default', 'minimal', 'low', 'medium', 'high', 'xhigh', 'max']
})

const effortLabel = (v: string) => {
  const k = t('load.' + v)
  return k.startsWith('load.') ? v : k
}

const modelCfg = ref<ModelConfig>({
  contextLength: 4096,
  gpuOffload: 0,
  cpuThreads: 0,
  evalBatch: 2048,
  physicalBatch: 512,
  flashAttention: true,
  specType: 'None',
  draftSpecType: 'simple',
  draftParams: {
    mtp: { ...defaultDraftParams },
    simple: { ...defaultDraftParams },
    draftMtp: { ...defaultDraftParams },
    dflash: { ...defaultDraftParams },
    eagle3: { ...defaultDraftParams },
    dspark: { ...defaultDraftParams },
  },
  dflashNgramK4v: false,
  ngramK4vSizeN: 12,
  ngramK4vSizeM: 48,
  ngramK4vMinHits: 1,
  ngramMod: false,
  ngramModNMatch: 24,
  ngramModNMin: 48,
  ngramModNMax: 64,
  ngramCache: false,
  kCacheQuant: 'Q8_0',
  vCacheQuant: 'Q8_0',
  cacheReuse: 0,
  ctxCheckpoints: 32,
  checkpointMinStep: 8192,
  reasoning: 'auto',
  reasoningBudget: '-1',
  reasoningBudgetCustom: 2048,
  reasoningEffort: 'default',
  draftModelPath: '',
  host: '127.0.0.1',
  alias: '',
  threadsHttp: 2,
  noWarmup: false,
  sleepIdle: -1,
  reasoningPreserve: false,
  fit: 'on',
  parallel: 1,
  mlock: false,
  mmap: false,
  kvUnified: false,
  kvOffload: false,
  cacheRam: 0,
  nCpuMoe: 0,
  expertsPerToken: 0,
  visionEnabled: false,
  mmprojPath: '',
  seed: -1,
  temp: 0.8,
  topP: 0.95,
  topK: 40,
  minP: 0.05,
  repeatPenalty: 1.0,
})

const draftKind = computed(() => activeSpecKind(modelCfg.value))

const ngramK4vAvailable = computed(() => {
  return modelCfg.value.specType === 'MTP' || (modelCfg.value.specType === 'Draft' && modelCfg.value.draftSpecType === 'dflash')
})

const ngramModAvailable = computed(() => modelCfg.value.specType !== 'None')

const maxContext = computed(() => activeModel.value?.max_context || 262144)

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
  if (!systemRamAvailable.value || modelCfg.value.cacheRam <= 0) return 0
  return Math.round((modelCfg.value.cacheRam / systemRamAvailable.value) * 100)
})

const cacheRamWarning = computed(() => {
  const v = modelCfg.value.cacheRam
  if (v === -1) return 'unlimited'
  if (v <= 0 || !systemRamAvailable.value) return null
  if (v > systemRamAvailable.value) return 'critical'
  if (v > systemRamAvailable.value * 0.8) return 'warning'
  return null
})

const draftModels = computed(() => {
  if (!activeModel.value) return []
  return allModels.value.filter(m => 
    m.model_family === activeModel.value!.model_family &&
    m.path !== activeModel.value!.path
  )
})

// Cargar config cuando cambia el modelo seleccionado
watch(activeModel, async (model) => {
  if (model) {
    const cfg = await loadModelConfig(model.path)
    if (cfg.reasoningEffort !== 'default' && model.supported_effort_levels?.length && !model.supported_effort_levels.includes(cfg.reasoningEffort)) {
      cfg.reasoningEffort = 'default'
    }
    if (!cfg.cpuThreads || cfg.cpuThreads === 0) {
      cfg.cpuThreads = await invoke<number>('get_cpu_threads')
    }
    const maxLayers = model.layer_count ?? 999
    if (cfg.gpuOffload > maxLayers) {
      cfg.gpuOffload = maxLayers
    }
    modelCfg.value = cfg
  }
}, { immediate: true })

// Guardar config cuando cambia cualquier valor
watch(modelCfg, async (cfg) => {
  if (activeModel.value) {
    await saveModelConfig(activeModel.value.path, cfg)
  }
}, { deep: true })

const loading = ref(false)
const error = ref('')

async function loadModel() {
  const model = activeModel.value
  if (!model) return
  loadingModel.value = model
  modelLoading.value = true
  loading.value = true
  error.value = ''

  if (modelCfg.value.specType === 'Draft' && !(modelCfg.value.draftModelPath ?? '').trim()) {
    error.value = t('load.draftNeedsModel')
    modelLoading.value = false
    return
  }
  if (modelCfg.value.visionEnabled && !(modelCfg.value.mmprojPath ?? '').trim()) {
    error.value = t('load.visionNeedsMmproj')
    modelLoading.value = false
    return
  }

  try {
    const config = await loadConfig()
    const cfg = modelCfg.value
    const resolvedThreads = numOrDefault(cfg.cpuThreads, 0) > 0 ? numOrDefault(cfg.cpuThreads, 0) : await invoke<number>('get_cpu_threads')
    const resolvedGpu = numOrDefault(cfg.gpuOffload, 999)
    const dp = cfg.draftParams[activeSpecKind(cfg)]
    await invoke('load_model', {
      llamaPath: config.llamaPath,
      cudaGraphOpt: config.cudaGraphOpt ?? '',
      logVerbosity: Number(config.logVerbosity ?? 3),
      modelPath: model.path,
      gpuLayers: resolvedGpu,
      contextLength: numOrDefault(cfg.contextLength, 4096),
      cpuThreads: resolvedThreads,
      evalBatch: numOrDefault(cfg.evalBatch, 2048),
      physicalBatch: numOrDefault(cfg.physicalBatch, 512),
      flashAttention: cfg.flashAttention ?? true,
      specType: cfg.specType ?? 'None',
      draftSpecType: cfg.draftSpecType ?? 'simple',
      draftModelPath: cfg.draftModelPath ?? '',
      maxDraftTokens: numOrDefault(dp.maxDraftTokens, 2),
      minDraftTokens: numOrDefault(dp.minDraftTokens, 0),
      draftProbability: numOrDefault(dp.probability, 0.75),
      draftSplitProbability: numOrDefault(dp.splitProbability, 0.10),
      dflashNgramK4v: cfg.dflashNgramK4v ?? false,
      ngramK4vSizeN: numOrDefault(cfg.ngramK4vSizeN, 12),
      ngramK4vSizeM: numOrDefault(cfg.ngramK4vSizeM, 48),
      ngramK4vMinHits: numOrDefault(cfg.ngramK4vMinHits, 1),
      ngramMod: cfg.ngramMod ?? false,
      ngramModNMatch: numOrDefault(cfg.ngramModNMatch, 24),
      ngramModNMin: numOrDefault(cfg.ngramModNMin, 48),
      ngramModNMax: numOrDefault(cfg.ngramModNMax, 64),
      ngramCache: cfg.ngramCache ?? false,
      kCacheQuant: cfg.kCacheQuant ?? 'Q8_0',
      vCacheQuant: cfg.vCacheQuant ?? 'Q8_0',
      draftKCacheQuant: dp.kCacheQuant ?? 'F16',
      draftVCacheQuant: dp.vCacheQuant ?? 'F16',
      cacheReuse: numOrDefault(cfg.cacheReuse, 0),
      ctxCheckpoints: numOrDefault(cfg.ctxCheckpoints, 32),
      checkpointMinStep: numOrDefault(cfg.checkpointMinStep, 8192),
      port: Number(config.port) || 8080,
      host: cfg.host ?? '127.0.0.1',
      alias: cfg.alias ?? '',
      threadsHttp: numOrDefault(cfg.threadsHttp, 2),
      noWarmup: cfg.noWarmup ?? false,
      sleepIdle: numOrDefault(cfg.sleepIdle, -1),
      reasoningPreserve: cfg.reasoningPreserve ?? false,
      fit: cfg.fit ?? 'on',
      reasoning: cfg.reasoning ?? 'auto',
      reasoningBudget: cfg.reasoningBudget === 'custom' ? Math.max(1, numOrDefault(cfg.reasoningBudgetCustom, 2048)) : numOrDefault(cfg.reasoningBudget, -1),
      reasoningEffort: cfg.reasoningEffort ?? 'default',
      parallel: numOrDefault(cfg.parallel, 1),
      mlock: cfg.mlock ?? false,
      mmap: cfg.mmap ?? false,
      kvUnified: cfg.kvUnified ?? false,
      kvOffload: cfg.kvOffload ?? false,
      cacheRam: numOrDefault(cfg.cacheRam, 0),
      nCpuMoe: numOrDefault(cfg.nCpuMoe, 0),
      expertsPerToken: numOrDefault(cfg.expertsPerToken, 0),
      visionEnabled: cfg.visionEnabled ?? false,
      mmprojPath: cfg.mmprojPath ?? '',
      seed: numOrDefault(cfg.seed, -1),
      temp: numOrDefault(cfg.temp, 0.8),
      topP: numOrDefault(cfg.topP, 0.95),
      topK: numOrDefault(cfg.topK, 40),
      minP: numOrDefault(cfg.minP, 0.05),
      repeatPenalty: numOrDefault(cfg.repeatPenalty, 1.0),
    })
    loadedServerPort.value = Number(config.port) || 8080
    loadedModel.value = model
  } catch (e) {
    error.value = String(e)
    modelLoading.value = false
    loadingModel.value = null
  } finally {
    loading.value = false
  }
}

async function stopModel() {
  await invoke('stop_model')
  modelLoading.value = false
  loadedModel.value = null
  loadedServerPort.value = null
  prefillProgress.value = null
  generationTokens.value = null
}

function copy(text: string) {
  navigator.clipboard.writeText(text)
}
</script>
