<template>
  <BaseModal
    :visible="visible"
    :title="isEdit ? $t('gateway.channels.editTitle') : $t('gateway.channels.addTitle')"
    modal-class="!max-w-[680px]"
    @close="$emit('close')"
  >
    <div class="flex gap-4">
      <!-- 侧边栏：服务商 -->
      <aside class="w-40 shrink-0 max-h-[440px] space-y-0.5 overflow-y-auto border-r border-border pr-2">
        <button
          v-for="p in providers"
          :key="p.id"
          type="button"
          :class="['w-full truncate rounded-md px-3 py-2 text-left text-[13px] transition-colors', activeProvider === p.id ? 'bg-primary/10 font-medium text-primary' : 'text-text-muted hover:bg-muted hover:text-text']"
          @click="selectProvider(p)"
        >{{ p.name || $t(p.labelKey) }}</button>
      </aside>

      <!-- 表单 -->
      <div class="flex min-w-0 flex-1 flex-col gap-1">
        <div class="form-group">
          <label class="label">{{ $t('gateway.channels.name') }}</label>
          <input v-model.trim="form.name" class="input" :placeholder="$t('gateway.channels.namePlaceholder')" />
        </div>

        <!-- OAuth 账号渠道：选择已有账号 -->
        <div v-if="form.kind === 'codex_oauth'" class="form-group">
          <label class="label">{{ $t('gateway.channels.account') }}</label>
          <FloatingDropdown class="w-full" width="wide" placement="bottom-start">
            <template #trigger="{ isOpen }">
              <button type="button" class="input flex items-center justify-between gap-2 text-left">
                <span class="truncate" :class="{ 'text-text-muted': !selectedAccountLabel }">{{ selectedAccountLabel || $t('gateway.channels.accountPlaceholder') }}</span>
                <svg class="h-4 w-4 shrink-0 transition-transform" :class="{ 'rotate-180': isOpen }" viewBox="0 0 24 24" fill="currentColor"><path d="M7 10l5 5 5-5z"/></svg>
              </button>
            </template>
            <template #default="{ close }">
              <button
                v-for="acc in accounts"
                :key="acc.id"
                type="button"
                class="dropdown-item"
                :class="{ 'dropdown-item--active': acc.id === form.accountId }"
                @click="selectAccount(acc.id, close)"
              >{{ acc.label }}</button>
            </template>
          </FloatingDropdown>
          <p v-if="!accounts.length" class="text-meta mt-1">{{ $t('gateway.channels.noAccounts') }}</p>
        </div>

        <!-- 第三方兼容 / Anthropic：base_url + key -->
        <template v-else>
          <div class="form-group">
            <label class="label">{{ $t('gateway.channels.baseUrl') }}</label>
            <input v-model.trim="form.baseUrl" class="input font-mono" :placeholder="basePlaceholder" />
          </div>
          <div class="form-group">
            <label class="label">{{ $t('gateway.channels.apiKey') }}</label>
            <input v-model.trim="form.apiKey" type="password" class="input font-mono" placeholder="sk-..." />
          </div>
          <div v-if="showApiType" class="form-group">
            <label class="label">{{ $t('gateway.channels.apiType') }}</label>
            <div class="grid grid-cols-2 gap-1 rounded-lg border border-border bg-muted p-1">
              <button
                v-for="option in apiTypeOptions"
                :key="option.value"
                type="button"
                :class="['rounded-md px-3 py-2 text-left text-[12px] transition', form.wire === option.value ? 'bg-surface text-accent shadow-sm' : 'text-text-secondary hover:text-text']"
                @click="form.wire = option.value"
              >
                <span class="block font-medium">{{ $t(option.label) }}</span>
                <span class="mt-0.5 block font-mono text-[11px] text-text-muted">{{ option.path }}</span>
              </button>
            </div>
          </div>
        </template>

        <div class="form-group">
          <div class="mb-1 flex items-center justify-between gap-2">
            <label class="label mb-0">{{ $t('gateway.channels.model') }}</label>
            <div v-if="form.kind !== 'codex_oauth'" class="flex flex-wrap items-center justify-end gap-2">
              <button
                type="button"
                class="btn btn--secondary btn--sm"
                :disabled="!canConnect || fetching"
                @click="doFetchModels"
              >{{ fetching ? $t('gateway.channels.fetching') : $t('gateway.channels.fetchModels') }}</button>
              <FloatingDropdown v-if="form.models.length" placement="bottom-end">
                <template #trigger="{ isOpen }">
                  <button
                    type="button"
                    class="btn btn--secondary btn--sm flex max-w-[160px] items-center gap-1 font-mono"
                    :title="$t('gateway.channels.testModelLabel')"
                  >
                    <span class="truncate">{{ testModel }}</span>
                    <svg class="h-3.5 w-3.5 shrink-0 transition-transform" :class="{ 'rotate-180': isOpen }" viewBox="0 0 24 24" fill="currentColor"><path d="M7 10l5 5 5-5z"/></svg>
                  </button>
                </template>
                <template #default="{ close }">
                  <button
                    v-for="m in form.models"
                    :key="m.id"
                    type="button"
                    class="dropdown-item font-mono"
                    :class="{ 'dropdown-item--active': m.id === testModel }"
                    @click="selectTestModel(m.id, close)"
                  >{{ m.id }}</button>
                </template>
              </FloatingDropdown>
              <button
                type="button"
                class="btn btn--secondary btn--sm"
                :disabled="!canConnect || testing"
                @click="doTestChannel"
              >{{ testing ? $t('gateway.channels.testing') : $t('gateway.channels.testChannel') }}</button>
              <span
                v-if="testMsg"
                class="max-w-[180px] truncate font-mono text-[12px]"
                :class="testMsg.ok ? 'text-success' : 'text-danger'"
                v-tooltip="testMsg.text"
              >{{ testMsg.text }}</span>
            </div>
          </div>
          <div v-if="form.models.length" class="mb-1.5 flex flex-wrap gap-1.5">
            <FloatingDropdown
              v-for="(m, i) in form.models"
              :key="m.id"
              placement="bottom-start"
              :close-on-select="false"
              @open="openEditor(m)"
            >
              <template #trigger>
                <span class="badge badge--sm inline-flex cursor-pointer items-center gap-1 font-mono hover:border-accent/40">
                  {{ modelLabel(m) }}
                  <button type="button" class="leading-none text-text-muted hover:text-danger" @click.stop="removeModel(m)">×</button>
                </span>
              </template>
              <template #default="{ close }">
                <div class="w-64 space-y-2 p-3" @click.stop>
                  <div class="form-group">
                    <label class="label">{{ $t('gateway.channels.alias') }}</label>
                    <input v-model.trim="editDraft.id" class="input font-mono" />
                  </div>
                  <div class="form-group">
                    <label class="label">{{ $t('gateway.channels.upstreamId') }}</label>
                    <input v-model.trim="editDraft.upstream" class="input font-mono" :placeholder="editDraft.id" />
                  </div>
                  <p v-if="editError" class="text-[12px] text-danger">{{ editError }}</p>
                  <p class="text-meta">{{ $t('gateway.channels.aliasHint') }}</p>
                  <div class="flex justify-end gap-2">
                    <button type="button" class="btn btn--secondary btn--sm" @click="close">{{ $t('common.cancel') }}</button>
                    <button type="button" class="btn btn--primary btn--sm" @click="saveEditor(i, close)">{{ $t('common.save') }}</button>
                  </div>
                </div>
              </template>
            </FloatingDropdown>
          </div>
          <input
            v-model.trim="modelInput"
            class="input font-mono"
            list="gw-channel-models"
            :placeholder="$t('gateway.channels.modelPlaceholder')"
            @keydown.enter.prevent="addModelFromInput"
          />
          <datalist id="gw-channel-models">
            <option v-for="id in modelOptions" :key="id" :value="id" />
          </datalist>
          <p v-if="fetchMsg" class="mt-1 text-[12px]" :class="fetchMsg.ok ? 'text-success' : 'text-danger'">{{ fetchMsg.text }}</p>
          <div v-if="fetchedModels.length" class="mt-2 rounded-lg border border-border bg-muted/30 p-2">
            <div class="mb-2 flex items-center justify-between gap-2">
              <div class="text-[12px] font-medium text-text">
                {{ $t('gateway.channels.fetchedCandidates') }}
                <span class="font-mono text-text-muted">{{ $t('gateway.channels.candidateCount', { count: fetchedCandidateModels.length }) }}</span>
              </div>
              <button
                type="button"
                class="btn btn--secondary btn--sm"
                :disabled="!fetchedCandidateModels.length"
                @click="addFetchedModels"
              >{{ $t('gateway.channels.addFetchedModels') }}</button>
            </div>
            <div v-if="fetchedCandidateModels.length" class="max-h-36 overflow-y-auto">
              <div class="grid grid-cols-1 gap-1 sm:grid-cols-2">
                <button
                  v-for="m in fetchedCandidateModels"
                  :key="m"
                  type="button"
                  class="flex min-w-0 items-center justify-between gap-2 rounded-md border border-border bg-surface px-2 py-1.5 text-left text-[12px] transition-colors hover:border-accent/40 hover:bg-accent/5"
                  @click="addModel(m)"
                >
                  <span class="truncate font-mono text-text" v-tooltip="m">{{ m }}</span>
                  <span class="shrink-0 text-[11px] text-accent">{{ $t('gateway.channels.addCandidate') }}</span>
                </button>
              </div>
            </div>
            <div v-else class="rounded-md border border-dashed border-border bg-surface px-3 py-3 text-center text-[12px] text-text-muted">
              {{ $t('gateway.channels.allFetchedAdded') }}
            </div>
          </div>
          <div v-else-if="suggestedModels.length" class="mt-2 rounded-lg border border-border bg-muted/30 p-2">
            <div class="mb-2 text-[12px] font-medium text-text">{{ $t('gateway.channels.recommendedModels') }}</div>
            <div class="flex flex-wrap gap-1.5">
              <button
                v-for="m in suggestedModels"
                :key="m"
                type="button"
                class="badge badge--sm cursor-pointer font-mono text-text-muted hover:text-primary"
                @click="addModel(m)"
              >+ {{ m }}</button>
            </div>
          </div>
          <p class="text-meta mt-1">{{ $t('gateway.channels.modelMultiHint') }}</p>
        </div>

        <div class="form-group">
          <label class="label">{{ $t('gateway.channels.priority') }}</label>
          <input v-model.number="form.priority" type="number" min="0" class="input !w-32 font-mono" />
          <p class="text-meta mt-1">{{ $t('gateway.channels.priorityHint') }}</p>
        </div>
      </div>
    </div>

    <template #footer>
      <button class="btn btn--secondary" @click="$emit('close')">{{ $t('common.cancel') }}</button>
      <button class="btn btn--primary" :disabled="!canSave" @click="submit">{{ $t('common.save') }}</button>
    </template>
  </BaseModal>
</template>

<script setup>
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import BaseModal from '../common/BaseModal.vue'
import FloatingDropdown from '../common/FloatingDropdown.vue'
import { useGatewayStore } from '../../stores/gateway'

const props = defineProps({
  visible: { type: Boolean, default: false },
  channel: { type: Object, default: null },
  accounts: { type: Array, default: () => [] }
})
const emit = defineEmits(['close', 'save'])

const store = useGatewayStore()
const { t } = useI18n()

// 侧边栏服务商：Codex OAuth + 各服务商预设；预填协议（kind）、默认地址与建议模型，全部可改
// developers 用于把渠道关联到已同步模型目录的开发商分组，过滤上游模型建议
const providers = [
  { id: 'codex_oauth', labelKey: 'gateway.channels.kindCodex', kind: 'codex_oauth', developers: ['openai'], models: ['gpt-4o', 'gpt-4o-mini', 'gpt-5', 'gpt-5.1'] },
  { id: 'openai', name: 'OpenAI', kind: 'openai_compat', wire: 'chat', supportsResponses: true, baseUrl: 'https://api.openai.com/v1', developers: ['openai'], models: ['gpt-4o', 'gpt-4o-mini', 'gpt-5', 'gpt-5.1'] },
  { id: 'anthropic', name: 'Anthropic', kind: 'anthropic', baseUrl: 'https://api.anthropic.com', developers: ['anthropic'], models: ['claude-opus-4-5', 'claude-sonnet-4-5'] },
  { id: 'gemini', name: 'Gemini', kind: 'openai_compat', wire: 'chat', baseUrl: 'https://generativelanguage.googleapis.com/v1beta/openai', developers: ['google', 'gemini'], models: ['gemini-2.5-pro', 'gemini-2.5-flash'] },
  { id: 'deepseek', name: 'DeepSeek', kind: 'openai_compat', wire: 'chat', baseUrl: 'https://api.deepseek.com/v1', developers: ['deepseek'], models: ['deepseek-chat', 'deepseek-reasoner'] },
  { id: 'moonshot', name: 'Moonshot', kind: 'openai_compat', wire: 'chat', baseUrl: 'https://api.moonshot.cn/v1', developers: ['moonshotai', 'moonshot'], models: ['kimi-k2-thinking', 'kimi-k2-0905-preview'] },
  { id: 'zhipu', name: 'Zhipu', kind: 'openai_compat', wire: 'chat', baseUrl: 'https://open.bigmodel.cn/api/paas/v4', developers: ['zhipu', 'zhipuai'], models: ['glm-4.6', 'glm-4.5-air'] },
  { id: 'xai', name: 'xAI', kind: 'openai_compat', wire: 'chat', baseUrl: 'https://api.x.ai/v1', developers: ['x-ai', 'xai'], models: ['grok-4', 'grok-3', 'grok-3-mini'] },
  { id: 'qwen', name: 'Qwen', kind: 'openai_compat', wire: 'chat', baseUrl: 'https://dashscope.aliyuncs.com/compatible-mode/v1', developers: ['qwen', 'alibaba'], models: ['qwen-max', 'qwen-plus', 'qwen-turbo'] }
]

const apiTypeOptions = [
  { value: 'chat', label: 'gateway.channels.apiTypeChat', path: '/chat/completions' },
  { value: 'responses', label: 'gateway.channels.apiTypeResponses', path: '/responses' }
]

const blank = () => ({ id: '', name: '', kind: 'codex_oauth', accountId: '', baseUrl: '', apiKey: '', wire: 'chat', models: [], enabled: true, priority: 100 })
const form = reactive(blank())
const activeProvider = ref('codex_oauth')
const modelInput = ref('')
const providerDrafts = ref({})

// 渠道连通性：拉取远端模型 / 测试可用性的本地状态
const fetchedModels = ref([])
const fetching = ref(false)
const fetchMsg = ref(null)
const testing = ref(false)
const testMsg = ref(null)
const testModel = ref('')

// 芯片编辑（方案 A）：单个浮层复用一份草稿
const editDraft = ref({ id: '', upstream: '' })
const editError = ref('')

// 模型条目内部统一为 { id, upstream }（upstream 为空表示与别名一致）
const normalizeModel = (m) => {
  if (typeof m === 'string') return { id: m.trim(), upstream: '' }
  const id = (m?.id || '').trim()
  const up = (m?.upstream || '').trim()
  return { id, upstream: up && up !== id ? up : '' }
}
const normalizeModels = (list) =>
  (Array.isArray(list) ? list.map(normalizeModel).filter((m) => m.id) : [])
// 持久化时回落最简形态：无别名映射存字符串，有映射存 { id, upstream }
const serializeModel = (m) => (m.upstream && m.upstream !== m.id ? { id: m.id, upstream: m.upstream } : m.id)
// 芯片展示：别名 → 上游（相等时仅显示别名）
const modelLabel = (m) => (m.upstream && m.upstream !== m.id ? `${m.id} → ${m.upstream}` : m.id)
const hasModel = (id) => form.models.some((m) => m.id === id)

const channelFields = () => ({
  kind: form.kind,
  accountId: form.accountId,
  baseUrl: form.baseUrl,
  apiKey: form.apiKey,
  wire: form.wire,
  models: form.models.map((m) => ({ ...m }))
})

const applyChannelFields = (draft) => {
  form.kind = draft.kind || 'codex_oauth'
  form.accountId = draft.accountId || ''
  form.baseUrl = draft.baseUrl || ''
  form.apiKey = draft.apiKey || ''
  form.wire = draft.wire || 'chat'
  form.models = normalizeModels(draft.models)
}

const clearTransientState = () => {
  modelInput.value = ''
  fetchedModels.value = []
  fetchMsg.value = null
  testMsg.value = null
}

const selectProvider = (p) => {
  if (activeProvider.value === p.id) return
  providerDrafts.value[activeProvider.value] = channelFields()
  activeProvider.value = p.id
  const draft = providerDrafts.value[p.id]
  if (draft) {
    applyChannelFields(draft)
  } else {
    form.kind = p.kind
    if (p.kind !== 'codex_oauth') form.baseUrl = p.baseUrl || ''
    if (p.kind === 'openai_compat') form.wire = p.wire || 'chat'
  }
  clearTransientState()
  testModel.value = form.models[0]?.id || ''
}

// 编辑既有渠道时，按 kind + baseUrl 回填侧边栏选中项；无匹配回退到该协议的默认服务商
const matchProvider = () => {
  if (form.kind === 'codex_oauth') return 'codex_oauth'
  const hit = providers.find((p) => p.kind === form.kind && p.baseUrl && p.baseUrl === form.baseUrl)
  if (hit) return hit.id
  return form.kind === 'anthropic' ? 'anthropic' : 'openai'
}

const activeProviderDef = computed(() => providers.find((p) => p.id === activeProvider.value))
const presetModels = computed(() => activeProviderDef.value?.models || [])

// API 类型（Chat/Responses）仅对支持 Responses 的服务商（OpenAI）显示，其余 openai_compat 仅走 Chat
const showApiType = computed(() => form.kind === 'openai_compat' && Boolean(activeProviderDef.value?.supportsResponses))

// 按当前服务商的 developers 取已同步模型目录中对应分组的模型 ID
const providerModelIds = computed(() => {
  const aliases = (activeProviderDef.value?.developers || []).map((d) => d.toLowerCase())
  if (!aliases.length) return []
  const ids = []
  for (const grp of store.models) {
    const key = (grp.id || '').toLowerCase()
    const name = (grp.name || '').toLowerCase()
    if (!aliases.includes(key) && !aliases.includes(name)) continue
    for (const m of grp.models || []) {
      if (m?.id) ids.push(m.id)
    }
  }
  return ids
})

// datalist 候选：拉取的远端模型 + 当前服务商建议模型 + 同步目录中该开发商的模型（去重，排除已选）
const modelOptions = computed(() => {
  const merged = [...new Set([...fetchedModels.value, ...presetModels.value, ...providerModelIds.value])]
  return merged.filter((id) => !hasModel(id))
})

// 一键添加建议：拉取过远端模型时优先用真实列表，否则取同步目录（后端已按发布时间倒序），再回退预设精选；排除已选、限量
const SUGGEST_LIMIT = 8
const suggestedModels = computed(() => {
  const ranked = providerModelIds.value.length
      ? providerModelIds.value
      : presetModels.value
  return ranked.filter((m) => !hasModel(m)).slice(0, SUGGEST_LIMIT)
})
const fetchedCandidateModels = computed(() => fetchedModels.value.filter((id) => !hasModel(id)))

// 仅第三方渠道可拉取/测试，需 baseUrl + apiKey 均存在
const canConnect = computed(() => form.kind !== 'codex_oauth' && Boolean(form.baseUrl) && Boolean(form.apiKey))

const doFetchModels = async () => {
  if (!canConnect.value || fetching.value) return
  fetching.value = true
  fetchMsg.value = null
  try {
    const ids = await store.fetchChannelModels({ kind: form.kind, baseUrl: form.baseUrl, apiKey: form.apiKey })
    fetchedModels.value = ids
    fetchMsg.value = { ok: true, text: t('gateway.channels.fetchModelsDone', { count: ids.length }) }
  } catch (e) {
    fetchMsg.value = { ok: false, text: t('gateway.channels.fetchModelsFailed', { msg: String(e?.message || e) }) }
  } finally {
    fetching.value = false
  }
}

const addFetchedModels = () => {
  const added = fetchedCandidateModels.value.map((id) => ({ id, upstream: '' }))
  if (added.length) form.models = [...form.models, ...added]
}

const doTestChannel = async () => {
  if (!canConnect.value || testing.value) return
  const id = testModel.value || form.models[0]?.id
  if (!id) {
    testMsg.value = { ok: false, text: t('gateway.channels.testNoModel') }
    return
  }
  // 测试直连上游，使用别名对应的上游真实 id
  const entry = form.models.find((m) => m.id === id)
  const model = entry ? entry.upstream || entry.id : id
  testing.value = true
  testMsg.value = null
  try {
    const r = await store.testChannel({ kind: form.kind, baseUrl: form.baseUrl, apiKey: form.apiKey, wire: form.wire, model })
    if (r.success) {
      testMsg.value = { ok: true, text: t('gateway.channels.testOk', { ms: r.latency_ms }) }
    } else {
      testMsg.value = { ok: false, text: t('gateway.channels.testFail', { status: r.status, msg: r.error || '' }) }
    }
  } catch (e) {
    testMsg.value = { ok: false, text: t('gateway.channels.testFail', { status: 0, msg: String(e?.message || e) }) }
  } finally {
    testing.value = false
  }
}

const addModel = (m) => {
  const id = (m || '').trim()
  if (!id || hasModel(id)) return
  form.models = [...form.models, { id, upstream: '' }]
}

const addModelFromInput = () => {
  addModel(modelInput.value)
  modelInput.value = ''
}

const removeModel = (m) => {
  form.models = form.models.filter((x) => x.id !== m.id)
}

const selectTestModel = (m, close) => {
  testModel.value = m
  close?.()
}

// 芯片编辑（方案 A）：打开时以「有效上游 id」预填，避免改别名后丢失映射
const openEditor = (m) => {
  editDraft.value = { id: m.id, upstream: m.upstream || m.id }
  editError.value = ''
}

const saveEditor = (idx, close) => {
  const id = (editDraft.value.id || '').trim()
  const up = (editDraft.value.upstream || '').trim()
  if (!id) {
    editError.value = t('gateway.channels.aliasRequired')
    return
  }
  if (form.models.some((m, i) => i !== idx && m.id === id)) {
    editError.value = t('gateway.channels.aliasDuplicate')
    return
  }
  const next = { id, upstream: up && up !== id ? up : '' }
  form.models = form.models.map((m, i) => (i === idx ? next : m))
  editError.value = ''
  close?.()
}

const isEdit = computed(() => Boolean(props.channel && props.channel.id))
const basePlaceholder = computed(() =>
  form.kind === 'anthropic' ? 'https://api.anthropic.com' : 'https://api.openai.com/v1'
)
const selectedAccountLabel = computed(() => props.accounts.find((a) => a.id === form.accountId)?.label || '')

const selectAccount = (id, close) => {
  form.accountId = id
  close?.()
}

const canSave = computed(() => {
  if (!form.name) return false
  if (form.kind === 'codex_oauth') return Boolean(form.accountId)
  return Boolean(form.apiKey)
})

watch(
  () => props.visible,
  (open) => {
    if (!open) return
    Object.assign(form, blank(), props.channel || {})
    form.models = normalizeModels(form.models)
    if (form.kind === 'openai_compat' && !form.wire) form.wire = 'chat'
    providerDrafts.value = {}
    clearTransientState()
    testModel.value = form.models[0]?.id || ''
    activeProvider.value = matchProvider()
  },
  { immediate: true }
)

// 测试模型选择项始终落在已选模型范围内
watch(
  () => form.models,
  (list) => {
    if (!list.some((m) => m.id === testModel.value)) testModel.value = list[0]?.id || ''
  }
)

watch(
  () => [form.kind, form.baseUrl, form.apiKey, form.wire],
  () => {
    clearTransientState()
  }
)

const submit = () => {
  if (!canSave.value) return
  emit('save', { ...form, models: form.models.map(serializeModel) })
}
</script>
