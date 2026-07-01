<template>
  <div class="flex flex-col gap-3">
    <div class="flex items-center justify-between">
      <p class="text-[12px] text-text-muted">{{ $t('gateway.usage.subtitle') }}</p>
      <div class="flex items-center gap-2">
        <button class="btn btn--sm btn--secondary" :disabled="!recentFailures.length" @click="showRecentFailures">
          {{ $t('gateway.usage.recentFailures') }}
        </button>
        <button class="btn btn--sm btn--secondary" :disabled="store.isLoadingUsage" @click="refresh">
          {{ $t('common.refresh') }}
        </button>
        <button class="btn btn--sm btn--ghost text-danger" :disabled="!store.usage.length" @click="confirmClear">
          {{ $t('gateway.usage.clear') }}
        </button>
      </div>
    </div>

    <!-- 筛选：模型 / 渠道（可搜索输入下拉框） -->
    <div class="flex flex-wrap items-center gap-2">
      <FloatingDropdown placement="bottom-start" :offset="4">
        <template #trigger="{ isOpen }">
          <button type="button" class="btn btn--secondary btn--sm h-8 flex items-center gap-1 px-2" :class="{ 'btn--light': !isOpen }">
            <span class="text-[13px] truncate max-w-[160px]">{{ filters.model || ($t('gateway.usage.filterAll') + ' · ' + $t('gateway.usage.model')) }}</span>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 9l6 6 6-6"/></svg>
          </button>
        </template>
        <template #default="{ close }">
          <div class="py-1">
            <div class="px-2 pb-1" @click.stop>
              <input v-model="modelSearch" class="input h-7 w-full text-[12px]" :placeholder="$t('common.search')" />
            </div>
            <div class="max-h-[220px] overflow-y-auto">
              <button class="dropdown-item flex items-center px-3 py-1.5 text-[13px]" :class="{ 'bg-primary/10': !filters.model }" @click="selectModel('', close)">{{ $t('gateway.usage.filterAll') }}</button>
              <button v-for="m in filteredModelOptions" :key="m" class="dropdown-item flex items-center px-3 py-1.5 text-[13px]" :class="{ 'bg-primary/10': m === filters.model }" @click="selectModel(m, close)">
                <span class="truncate">{{ m }}</span>
              </button>
              <div v-if="!filteredModelOptions.length" class="px-3 py-2 text-center text-[12px] text-text-muted">{{ $t('gateway.usage.noMatch') }}</div>
            </div>
          </div>
        </template>
      </FloatingDropdown>
      <FloatingDropdown placement="bottom-start" :offset="4">
        <template #trigger="{ isOpen }">
          <button type="button" class="btn btn--secondary btn--sm h-8 flex items-center gap-1 px-2" :class="{ 'btn--light': !isOpen }">
            <span class="text-[13px] truncate max-w-[160px]">{{ selectedChannelName || ($t('gateway.usage.filterAll') + ' · ' + $t('gateway.usage.channel')) }}</span>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 9l6 6 6-6"/></svg>
          </button>
        </template>
        <template #default="{ close }">
          <div class="py-1">
            <div class="px-2 pb-1" @click.stop>
              <input v-model="channelSearch" class="input h-7 w-full text-[12px]" :placeholder="$t('common.search')" />
            </div>
            <div class="max-h-[220px] overflow-y-auto">
              <button class="dropdown-item flex items-center px-3 py-1.5 text-[13px]" :class="{ 'bg-primary/10': !filters.channel }" @click="selectChannel('', close)">{{ $t('gateway.usage.filterAll') }}</button>
              <button v-for="c in filteredChannelOptions" :key="c.id" class="dropdown-item flex items-center px-3 py-1.5 text-[13px]" :class="{ 'bg-primary/10': c.id === filters.channel }" @click="selectChannel(c.id, close)">
                <span class="truncate">{{ c.name }}</span>
              </button>
              <div v-if="!filteredChannelOptions.length" class="px-3 py-2 text-center text-[12px] text-text-muted">{{ $t('gateway.usage.noMatch') }}</div>
            </div>
          </div>
        </template>
      </FloatingDropdown>
      <div class="flex rounded-lg border border-border bg-muted p-1">
        <button
          v-for="option in statusOptions"
          :key="option.value"
          type="button"
          :class="['rounded-md px-2 py-1 text-[12px] transition', filters.status === option.value ? 'bg-surface text-accent shadow-sm' : 'text-text-secondary hover:text-text']"
          @click="filters.status = option.value"
        >
          {{ $t(option.label) }}
        </button>
      </div>
      <div class="flex rounded-lg border border-border bg-muted p-1">
        <button
          v-for="option in rangeOptions"
          :key="option.value"
          type="button"
          :class="['rounded-md px-2 py-1 text-[12px] transition', filters.range === option.value ? 'bg-surface text-accent shadow-sm' : 'text-text-secondary hover:text-text']"
          @click="filters.range = option.value"
        >
          {{ $t(option.label) }}
        </button>
      </div>
      <button v-if="hasFilter" class="btn btn--sm btn--ghost" @click="resetFilters">{{ $t('gateway.usage.reset') }}</button>
      <span class="ml-auto text-[12px] text-text-muted">{{ $t('gateway.usage.totalCount', { count: filtered.length }) }}</span>
    </div>

    <div v-if="!store.usage.length" class="flex flex-col items-center justify-center py-12 text-text-secondary">
      <p class="text-sm">{{ $t('gateway.usage.empty') }}</p>
    </div>

    <div v-else-if="!filtered.length" class="flex flex-col items-center justify-center py-12 text-text-secondary">
      <p class="text-sm">{{ $t('gateway.usage.noMatch') }}</p>
    </div>

    <template v-else>
      <div class="table-container rounded-lg">
        <table class="table">
          <thead>
            <tr>
              <th class="th w-[140px]">{{ $t('gateway.usage.time') }}</th>
              <th class="th">{{ $t('gateway.usage.model') }}</th>
              <th class="th">{{ $t('gateway.usage.channel') }}</th>
              <th class="th text-right">{{ $t('gateway.usage.tokens') }}</th>
              <th class="th text-right">{{ $t('gateway.usage.cached') }}</th>
              <th class="th w-[70px] text-center">{{ $t('gateway.usage.stream') }}</th>
              <th class="th text-right">{{ $t('gateway.usage.latency') }}</th>
              <th class="th w-[80px] text-center">{{ $t('gateway.usage.status') }}</th>
            </tr>
          </thead>
          <tbody>
            <template v-for="u in paged" :key="u.requestId || u.createdAt">
              <tr>
                <td class="text-meta">{{ formatTime(u.createdAt) }}</td>
                <td class="font-mono text-[12px]">
                  <span>{{ u.model }}</span>
                  <span v-if="u.status !== 'error' && !hasPrice(u.model)" class="badge badge--sm ml-1.5 text-text-muted" v-tooltip="$t('gateway.usage.noPriceHint')">{{ $t('gateway.usage.noPrice') }}</span>
                </td>
                <td class="text-[12px] text-text-secondary">{{ channelName(u.channelId) }}</td>
                <td class="text-right font-mono text-[12px]">
                  <div class="text-text">{{ tokenTotal(u).toLocaleString() }}</div>
                  <div class="text-[11px] text-text-muted" v-tooltip="$t('gateway.usage.tokensHint')">↑{{ u.promptTokens ?? 0 }} ↓{{ u.completionTokens ?? 0 }}</div>
                </td>
                <td class="text-right font-mono text-[12px]">
                  <div class="text-text">{{ formatHitRate(u) }}</div>
                  <div class="text-[11px] text-text-muted" v-tooltip="$t('gateway.usage.cacheHint')">{{ u.cachedTokens ?? 0 }} / {{ u.cacheWriteTokens ?? 0 }}</div>
                </td>
                <td class="text-center">
                  <span v-if="u.stream" class="inline-block h-2 w-2 rounded-full bg-accent" v-tooltip="$t('gateway.usage.streamOn')"></span>
                  <span v-else class="inline-block h-2 w-2 rounded-full bg-text-muted/30" v-tooltip="$t('gateway.usage.streamOff')"></span>
                </td>
                <td class="text-right font-mono text-[12px]">
                  <div class="text-text">{{ formatMs(u.durationMs) }}</div>
                  <div class="text-[11px] text-text-muted">TTFT {{ formatMs(u.ttftMs) }}</div>
                </td>
                <td class="text-center">
                  <button
                    type="button"
                    :class="['badge badge--sm', u.status === 'error' ? 'badge--danger' : 'badge--success']"
                    :disabled="u.status !== 'error'"
                    @click="toggleDetails(u)"
                  >
                    {{ u.status === 'error' ? $t('gateway.usage.statusError') : $t('gateway.usage.statusOk') }}
                  </button>
                </td>
              </tr>
              <tr v-if="isExpanded(u)" class="bg-danger/5">
                <td colspan="8" class="px-4 py-3">
                  <div class="grid grid-cols-1 gap-2 text-[12px] md:grid-cols-4">
                    <div><span class="text-text-muted">{{ $t('gateway.usage.inbound') }}</span><div class="mt-0.5 font-mono text-text">{{ u.inbound || '—' }}</div></div>
                    <div><span class="text-text-muted">{{ $t('gateway.usage.kind') }}</span><div class="mt-0.5 font-mono text-text">{{ u.kind || '—' }}</div></div>
                    <div><span class="text-text-muted">{{ $t('gateway.usage.statusCode') }}</span><div class="mt-0.5 font-mono text-text">{{ u.statusCode || '—' }}</div></div>
                    <div><span class="text-text-muted">{{ $t('gateway.usage.requestId') }}</span><div class="mt-0.5 truncate font-mono text-text" v-tooltip="u.requestId">{{ u.requestId || '—' }}</div></div>
                  </div>
                  <div class="mt-2 rounded-lg border border-danger/20 bg-surface px-3 py-2 font-mono text-[12px] text-danger">
                    {{ u.error || $t('gateway.usage.noErrorDetail') }}
                  </div>
                </td>
              </tr>
            </template>
          </tbody>
        </table>
      </div>

      <!-- 分页（复用通用组件，支持调整每页数量） -->
      <Pagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :total-pages="totalPages"
        :total-items="filtered.length"
        :page-size-options="PAGE_SIZE_OPTIONS"
      />
    </template>
  </div>
</template>

<script setup>
import { computed, onActivated, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useGatewayStore } from '../../stores/gateway'
import { useGatewayPricing } from '../../composables/useGatewayPricing'
import FloatingDropdown from '../common/FloatingDropdown.vue'
import Pagination from '../common/Pagination.vue'

const { t } = useI18n()
const store = useGatewayStore()
const { hasPrice } = useGatewayPricing()

const PAGE_SIZE_OPTIONS = [10, 20, 50, 100, 200]
const pageSize = ref(20)
const page = ref(1)
const filters = ref({ model: '', channel: '', status: '', range: 'all' })
const modelSearch = ref('')
const channelSearch = ref('')
const expandedId = ref('')

const statusOptions = [
  { value: '', label: 'gateway.usage.filterAll' },
  { value: 'success', label: 'gateway.usage.statusOk' },
  { value: 'error', label: 'gateway.usage.statusError' }
]
const rangeOptions = [
  { value: 'all', label: 'gateway.usage.rangeAll' },
  { value: 'today', label: 'gateway.usage.rangeToday' },
  { value: '7d', label: 'gateway.usage.range7d' },
  { value: '30d', label: 'gateway.usage.range30d' }
]

const channelName = (id) => store.channels.find((c) => c.id === id)?.name || id || '—'
const selectedChannelName = computed(() => (filters.value.channel ? channelName(filters.value.channel) : ''))

// 筛选下拉选项：取自当前已加载记录
const modelOptions = computed(() => [...new Set(store.usage.map((u) => u.model).filter(Boolean))].sort())
const channelOptions = computed(() => {
  const ids = [...new Set(store.usage.map((u) => u.channelId).filter(Boolean))]
  return ids.map((id) => ({ id, name: channelName(id) }))
})

const filteredModelOptions = computed(() => {
  const q = modelSearch.value.trim().toLowerCase()
  return q ? modelOptions.value.filter((m) => m.toLowerCase().includes(q)) : modelOptions.value
})
const filteredChannelOptions = computed(() => {
  const q = channelSearch.value.trim().toLowerCase()
  return q ? channelOptions.value.filter((c) => c.name.toLowerCase().includes(q)) : channelOptions.value
})

const selectModel = (v, close) => {
  filters.value.model = v
  modelSearch.value = ''
  close?.()
}
const selectChannel = (v, close) => {
  filters.value.channel = v
  channelSearch.value = ''
  close?.()
}

const hasFilter = computed(() =>
  Boolean(filters.value.model || filters.value.channel || filters.value.status || filters.value.range !== 'all')
)
const resetFilters = () => {
  filters.value = { model: '', channel: '', status: '', range: 'all' }
}

// 应用筛选
const filtered = computed(() =>
  store.usage.filter((u) => {
    const f = filters.value
    if (f.model && u.model !== f.model) return false
    if (f.channel && u.channelId !== f.channel) return false
    if (f.status && u.status !== f.status) return false
    if (!inRange(u.createdAt, f.range)) return false
    return true
  })
)
const recentFailures = computed(() =>
  [...store.usage]
    .filter((u) => u.status === 'error')
    .sort((a, b) => (b.createdAt || 0) - (a.createdAt || 0))
    .slice(0, 10)
)

const inRange = (ts, range) => {
  if (!range || range === 'all') return true
  if (!ts) return false
  const now = Date.now()
  const day = 24 * 60 * 60 * 1000
  if (range === 'today') {
    const d = new Date()
    d.setHours(0, 0, 0, 0)
    return ts >= d.getTime()
  }
  if (range === '7d') return ts >= now - 7 * day
  if (range === '30d') return ts >= now - 30 * day
  return true
}

const showRecentFailures = () => {
  filters.value = { ...filters.value, status: 'error', range: 'all' }
}

const totalPages = computed(() => Math.max(1, Math.ceil(filtered.value.length / pageSize.value)))
const paged = computed(() => filtered.value.slice((page.value - 1) * pageSize.value, page.value * pageSize.value))

// 翻页越界回到首页；筛选或每页数量变化重置到第一页
watch(totalPages, () => {
  if (page.value > totalPages.value) page.value = 1
})
watch([filters, pageSize], () => {
  page.value = 1
}, { deep: true })

const tokenTotal = (u) => (u.promptTokens || 0) + (u.completionTokens || 0)

const formatTime = (ts) => {
  if (!ts) return '—'
  const d = new Date(ts)
  if (Number.isNaN(d.getTime())) return String(ts)
  return d.toLocaleString()
}

const formatMs = (ms) => {
  if (ms == null) return '—'
  return ms >= 1000 ? `${(ms / 1000).toFixed(2)}s` : `${ms}ms`
}

// 缓存命中率 = 读缓存 / 输入 token（无输入则不展示）
const formatHitRate = (u) => {
  const prompt = u.promptTokens || 0
  if (!prompt) return '—'
  return `${(((u.cachedTokens || 0) / prompt) * 100).toFixed(1)}%`
}

const rowId = (u) => u.requestId || String(u.createdAt)
const isExpanded = (u) => expandedId.value === rowId(u)
const toggleDetails = (u) => {
  if (u.status !== 'error') return
  const id = rowId(u)
  expandedId.value = expandedId.value === id ? '' : id
}

const refresh = () => store.loadUsage()

const confirmClear = async () => {
  const ok = await window.$confirm?.({
    title: t('gateway.usage.clearTitle'),
    message: t('gateway.usage.clearMessage'),
    confirmText: t('gateway.usage.clear'),
    variant: 'danger'
  })
  if (!ok) return
  await store.clearUsage()
  window.$notify?.success(t('gateway.usage.cleared'))
}

onActivated(() => {
  store.loadUsage()
})
</script>
