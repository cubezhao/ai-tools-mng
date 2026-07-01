<template>
  <div class="flex flex-col gap-3">
    <!-- 工具条：搜索 + 开发商筛选 + 同步 -->
    <div class="flex flex-wrap items-center gap-2">
      <div class="relative min-w-[180px] flex-1">
        <input
          v-model.trim="query"
          class="input input--sm w-full pr-8"
          :placeholder="$t('gateway.models.searchPlaceholder')"
        />
        <button
          v-if="query"
          type="button"
          class="absolute right-2 top-1/2 flex h-4 w-4 -translate-y-1/2 items-center justify-center rounded-full text-text-muted transition hover:bg-hover hover:text-text"
          v-tooltip="$t('gateway.models.clearSearch')"
          @click="query = ''"
        >
          <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/></svg>
        </button>
      </div>
      <FloatingDropdown width="wide" placement="bottom-start">
        <template #trigger="{ isOpen }">
          <button type="button" class="input input--sm flex w-auto items-center gap-1.5 text-left">
            <span>{{ developerLabel }}</span>
            <svg class="h-3.5 w-3.5 shrink-0 transition-transform" :class="{ 'rotate-180': isOpen }" viewBox="0 0 24 24" fill="currentColor"><path d="M7 10l5 5 5-5z"/></svg>
          </button>
        </template>
        <template #default="{ close }">
          <button type="button" class="dropdown-item" :class="{ 'dropdown-item--active': developer === '' }" @click="selectDeveloper('', close)">{{ $t('gateway.models.allDevelopers') }}</button>
          <button
            v-for="p in sortedProviders"
            :key="p.id"
            type="button"
            class="dropdown-item"
            :class="{ 'dropdown-item--active': developer === p.id }"
            @click="selectDeveloper(p.id, close)"
          >{{ p.name }}</button>
        </template>
      </FloatingDropdown>
      <div class="flex rounded-lg border border-border bg-muted p-1">
        <button
          v-for="option in coverageOptions"
          :key="option.value"
          type="button"
          :class="['rounded-md px-2 py-1 text-[12px] transition', coverage === option.value ? 'bg-surface text-accent shadow-sm' : 'text-text-secondary hover:text-text']"
          @click="coverage = option.value"
        >
          {{ $t(option.label) }}
        </button>
      </div>
      <span class="text-[11px] text-text-muted">{{ lastSyncLabel }}</span>
      <button class="btn btn--sm btn--secondary" @click="openCreate">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/></svg>
        {{ $t('gateway.models.addModel') }}
      </button>
      <button class="btn btn--sm btn--primary" :disabled="store.isSyncingModels" @click="sync">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" :class="{ 'gw-spin': store.isSyncingModels }"><path d="M17.65 6.35C16.2 4.9 14.21 4 12 4c-4.42 0-7.99 3.58-7.99 8s3.57 8 7.99 8c3.73 0 6.84-2.55 7.73-6h-2.08c-.82 2.33-3.04 4-5.65 4-3.31 0-6-2.69-6-6s2.69-6 6-6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z"/></svg>
        {{ store.isSyncingModels ? $t('gateway.models.syncing') : $t('gateway.models.sync') }}
      </button>
    </div>

    <!-- 空态：未同步 -->
    <div v-if="!store.models.length" class="flex flex-col items-center justify-center py-12 text-text-secondary">
      <p class="text-sm">{{ $t('gateway.models.empty') }}</p>
      <button class="btn btn--secondary btn--sm mt-3" :disabled="store.isSyncingModels" @click="sync">{{ $t('gateway.models.emptySync') }}</button>
    </div>

    <!-- 无匹配 -->
    <div v-else-if="!filteredGroups.length" class="flex items-center justify-center py-12 text-text-secondary">
      <p class="text-sm">{{ $t('gateway.models.noMatch') }}</p>
    </div>

    <!-- 按开发商分组的卡片网格 -->
    <div v-else class="flex flex-col gap-5">
      <section v-for="group in filteredGroups" :key="group.id" class="flex flex-col gap-2">
        <div class="flex items-center gap-2">
          <h3 class="text-[13px] font-semibold text-text">{{ group.name }}</h3>
          <span class="badge badge--sm">{{ $t('gateway.models.count', { count: group.models.length }) }}</span>
        </div>
        <div class="grid grid-cols-1 gap-2 sm:grid-cols-2 xl:grid-cols-3">
          <div v-for="m in group.models" :key="m.id" class="group card flex flex-col gap-2 py-3">
            <div class="flex items-start justify-between gap-2">
              <div class="flex min-w-0 flex-wrap items-center gap-1.5">
                <span class="min-w-0 break-all font-mono text-[12px] font-medium text-text">{{ m.id }}</span>
                <span v-if="m.custom" class="badge badge--sm badge--accent-tech">{{ $t('gateway.models.custom') }}</span>
                <span
                  :class="['badge badge--sm', modelCoverage(m.id).length ? 'badge--success-tech' : 'text-text-muted']"
                  v-tooltip="modelCoverageTip(m.id)"
                >
                  {{ modelCoverage(m.id).length ? $t('gateway.models.routed') : $t('gateway.models.unrouted') }}
                </span>
              </div>
              <div class="flex shrink-0 items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100">
                <button class="btn btn--icon-sm btn--ghost" v-tooltip="$t('gateway.models.details')" @click="openDetails(m)">
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M11 7h2v2h-2zm0 4h2v6h-2zm1-9C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8z"/></svg>
                </button>
                <button class="btn btn--icon-sm btn--ghost" v-tooltip="$t('gateway.models.copy')" @click="copy(m.id)">
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/></svg>
                </button>
                <template v-if="m.custom">
                  <button class="btn btn--icon-sm btn--ghost" v-tooltip="$t('common.edit')" @click="openEditCustom(m)">
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04a1 1 0 0 0 0-1.41l-2.34-2.34a1 1 0 0 0-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"/></svg>
                  </button>
                  <button class="btn btn--icon-sm btn--ghost text-danger" v-tooltip="$t('common.delete')" @click="confirmDeleteCustom(m)">
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/></svg>
                  </button>
                </template>
              </div>
            </div>
            <div v-if="m.release_date" class="flex items-center gap-1.5 text-[11px] text-text-muted">
              <span>{{ $t('gateway.models.releaseDate') }}</span>
              <span class="font-mono">{{ m.release_date }}</span>
            </div>
            <div v-if="hasPrice(m)" class="flex flex-wrap items-center gap-x-2 gap-y-0.5 text-[11px] text-text-muted">
              <span>{{ $t('gateway.models.priceInput') }}</span>
              <span class="font-mono text-text-secondary">{{ fmtPrice(m.cost.input) }}</span>
              <span>{{ $t('gateway.models.priceOutput') }}</span>
              <span class="font-mono text-text-secondary">{{ fmtPrice(m.cost.output) }}</span>
              <template v-if="typeof m.cost.cache_read === 'number'">
                <span>{{ $t('gateway.models.priceCacheRead') }}</span>
                <span class="font-mono text-text-secondary">{{ fmtPrice(m.cost.cache_read) }}</span>
              </template>
              <template v-if="typeof m.cost.cache_write === 'number'">
                <span>{{ $t('gateway.models.priceCacheWrite') }}</span>
                <span class="font-mono text-text-secondary">{{ fmtPrice(m.cost.cache_write) }}</span>
              </template>
              <span class="text-[10px]">{{ $t('gateway.models.priceUnit') }}</span>
            </div>
          </div>
        </div>
      </section>
    </div>

    <CustomModelDialog
      :visible="showCustom"
      :model="editingCustom"
      @close="showCustom = false"
      @save="onSaveCustom"
    />

    <!-- 模型详情 -->
    <BaseModal
      :visible="showDetails"
      :title="detailModel ? `${$t('gateway.models.detailsTitle')} · ${detailModel.id}` : $t('gateway.models.detailsTitle')"
      @close="showDetails = false"
    >
      <div class="flex max-h-[60vh] flex-col gap-4 overflow-y-auto pr-1">
        <section v-for="g in detailGroups" :key="g.key" class="flex flex-col gap-1.5">
          <h4 class="text-[11px] font-semibold uppercase tracking-wide text-text-muted">{{ g.title }}</h4>
          <div class="divide-y divide-border overflow-hidden rounded-lg border border-border">
            <div v-for="row in g.items" :key="row.label" class="flex items-start justify-between gap-3 px-3 py-1.5">
              <span class="shrink-0 text-[12px] text-text-secondary">{{ row.label }}</span>
              <span class="break-all text-right text-[12px] text-text" :class="{ 'font-mono': row.mono }">{{ row.value }}</span>
            </div>
          </div>
        </section>
      </div>
      <template #footer>
        <button class="btn btn--secondary" @click="copy(detailJson, $t('common.copySuccess'))">{{ $t('gateway.models.copyDetails') }}</button>
        <button class="btn btn--primary" @click="showDetails = false">{{ $t('common.close') }}</button>
      </template>
    </BaseModal>
  </div>
</template>

<script setup>
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useGatewayStore } from '../../stores/gateway'
import BaseModal from '../common/BaseModal.vue'
import FloatingDropdown from '../common/FloatingDropdown.vue'
import CustomModelDialog from './CustomModelDialog.vue'

const { t } = useI18n()
const store = useGatewayStore()

const query = ref('')
const developer = ref('')
const coverage = ref('all')
const showDetails = ref(false)
const detailModel = ref(null)
const showCustom = ref(false)
const editingCustom = ref(null)

const lastSyncLabel = computed(() => {
  if (!store.modelsSyncedAt) return t('gateway.models.neverSynced')
  const d = new Date(store.modelsSyncedAt)
  return t('gateway.models.lastSync', { time: Number.isNaN(d.getTime()) ? store.modelsSyncedAt : d.toLocaleString() })
})

const developerLabel = computed(() => {
  if (!developer.value) return t('gateway.models.allDevelopers')
  return store.models.find((p) => p.id === developer.value)?.name || developer.value
})

const priorityDevelopers = ['anthropic', 'openai']
const coverageOptions = [
  { value: 'all', label: 'gateway.models.coverageAll' },
  { value: 'routed', label: 'gateway.models.routed' },
  { value: 'unrouted', label: 'gateway.models.unrouted' }
]

const coverageMap = computed(() => {
  const map = new Map()
  for (const c of store.channels) {
    if (c.enabled === false) continue
    for (const id of c.models || []) {
      if (!map.has(id)) map.set(id, [])
      map.get(id).push(c.name || c.id)
    }
  }
  return map
})
const modelCoverage = (id) => coverageMap.value.get(id) || []
const modelCoverageTip = (id) => {
  const channels = modelCoverage(id)
  return channels.length ? channels.join(', ') : t('gateway.models.unroutedHint')
}

const sortedProviders = computed(() => {
  const rank = (p) => {
    const i = priorityDevelopers.indexOf(p.id)
    return i === -1 ? priorityDevelopers.length : i
  }
  return [...store.models].sort((a, b) => {
    const ra = rank(a)
    const rb = rank(b)
    if (ra !== rb) return ra - rb
    return a.name.localeCompare(b.name)
  })
})

const detailJson = computed(() => (detailModel.value ? JSON.stringify(detailModel.value, null, 2) : ''))

const selectDeveloper = (id, close) => {
  developer.value = id
  close?.()
}

const hasPrice = (m) =>
  Boolean(m.cost) && (typeof m.cost.input === 'number' || typeof m.cost.output === 'number')

const fmtPrice = (n) => (typeof n === 'number' ? `$${n}` : '—')

const openDetails = (m) => {
  detailModel.value = m
  showDetails.value = true
}

const fmtBool = (v) => (v ? t('gateway.models.supported') : t('gateway.models.notSupported'))
const fmtNum = (n) => (typeof n === 'number' ? n.toLocaleString() : '—')

const costLabels = {
  input: 'priceInput',
  output: 'priceOutput',
  cache_read: 'priceCacheRead',
  cache_write: 'priceCacheWrite'
}

const effortValues = (m) => {
  const opt = Array.isArray(m.reasoning_options)
    ? m.reasoning_options.find((o) => o && o.type === 'effort')
    : null
  if (opt && Array.isArray(opt.values) && opt.values.length) return opt.values
  const ec = m.extra_capabilities?.reasoning?.effort_options
  return Array.isArray(ec) ? ec : []
}

const detailGroups = computed(() => {
  const m = detailModel.value
  if (!m) return []
  const groups = []
  const pushRow = (arr, label, value, mono = false) => {
    if (value === undefined || value === null || value === '') return
    arr.push({ label, value: String(value), mono })
  }

  const base = []
  pushRow(base, 'ID', m.id, true)
  pushRow(base, t('gateway.models.family'), m.family, true)
  pushRow(base, t('gateway.models.type'), m.type)
  pushRow(base, t('gateway.models.releaseDate'), m.release_date, true)
  pushRow(base, t('gateway.models.lastUpdated'), m.last_updated, true)
  pushRow(base, t('gateway.models.knowledge'), m.knowledge, true)
  if (base.length) groups.push({ key: 'base', title: t('gateway.models.detailBase'), items: base })

  const cap = []
  const inMod = m.modalities?.input
  const outMod = m.modalities?.output
  if (Array.isArray(inMod) && inMod.length) pushRow(cap, t('gateway.models.modalitiesIn'), inMod.join(', '))
  if (Array.isArray(outMod) && outMod.length) pushRow(cap, t('gateway.models.modalitiesOut'), outMod.join(', '))
  if (m.limit?.context != null) pushRow(cap, t('gateway.models.context'), fmtNum(m.limit.context), true)
  if (m.limit?.output != null) pushRow(cap, t('gateway.models.maxOutput'), fmtNum(m.limit.output), true)
  if (typeof m.temperature === 'boolean') pushRow(cap, t('gateway.models.temperature'), fmtBool(m.temperature))
  if (typeof m.tool_call === 'boolean') pushRow(cap, t('gateway.models.toolCall'), fmtBool(m.tool_call))
  if (typeof m.attachment === 'boolean') pushRow(cap, t('gateway.models.attachment'), fmtBool(m.attachment))
  const reasoningSupported = m.reasoning && typeof m.reasoning === 'object' ? m.reasoning.supported : m.reasoning
  if (typeof reasoningSupported === 'boolean') pushRow(cap, t('gateway.models.reasoning'), fmtBool(reasoningSupported))
  const efforts = effortValues(m)
  if (efforts.length) pushRow(cap, t('gateway.models.reasoningEffort'), efforts.join(', '), true)
  if (typeof m.open_weights === 'boolean') pushRow(cap, t('gateway.models.openWeights'), fmtBool(m.open_weights))
  if (cap.length) groups.push({ key: 'cap', title: t('gateway.models.detailCapabilities'), items: cap })

  const price = []
  if (m.cost && typeof m.cost === 'object') {
    for (const [k, v] of Object.entries(m.cost)) {
      if (typeof v !== 'number') continue
      const label = costLabels[k] ? t(`gateway.models.${costLabels[k]}`) : k.replace(/_/g, ' ')
      price.push({ label, value: `${fmtPrice(v)} / 1M`, mono: true })
    }
  }
  if (price.length) groups.push({ key: 'price', title: t('gateway.models.detailPricing'), items: price })

  return groups
})

const filteredGroups = computed(() => {
  const q = query.value.toLowerCase()
  const result = []
  for (const p of store.models) {
    if (developer.value && p.id !== developer.value) continue
    const nameHit = p.name.toLowerCase().includes(q)
    const models = (p.models || []).filter((m) => {
      const id = m.id || ''
      if (q && !nameHit && !id.toLowerCase().includes(q)) return false
      const routed = modelCoverage(id).length > 0
      if (coverage.value === 'routed' && !routed) return false
      if (coverage.value === 'unrouted' && routed) return false
      return true
    })
    if (models.length) result.push({ id: p.id, name: p.name, models })
  }
  return result
})

const sync = async () => {
  try {
    await store.syncModels()
    window.$notify?.success(t('gateway.models.syncSuccess'))
  } catch {
    window.$notify?.error(t('gateway.models.syncFailed'))
  }
}

const copy = async (text, successMsg) => {
  try {
    await navigator.clipboard.writeText(text)
    window.$notify?.success(successMsg || t('gateway.models.copied'))
  } catch (e) {
    console.warn('copy failed', e)
  }
}

const openCreate = () => {
  editingCustom.value = null
  showCustom.value = true
}

const openEditCustom = (m) => {
  editingCustom.value = { ...m }
  showCustom.value = true
}

const onSaveCustom = async (model, originalId) => {
  await store.upsertCustomModel(model, originalId)
  showCustom.value = false
  window.$notify?.success(t('gateway.models.customSaved'))
}

const confirmDeleteCustom = async (m) => {
  const ok = await window.$confirm?.({
    title: t('gateway.models.deleteModelTitle'),
    message: t('gateway.models.deleteModelMessage', { id: m.id }),
    confirmText: t('common.delete'),
    variant: 'danger'
  })
  if (!ok) return
  await store.removeCustomModel(m.id)
  window.$notify?.success(t('gateway.models.customDeleted'))
}

</script>

<style scoped>
@keyframes gw-spin {
  to { transform: rotate(360deg); }
}
.gw-spin {
  animation: gw-spin 0.9s linear infinite;
}
@media (prefers-reduced-motion: reduce) {
  .gw-spin {
    animation: none;
  }
}
</style>
