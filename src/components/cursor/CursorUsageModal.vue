<template>
  <BaseModal
    :visible="true"
    :title="$t('cursorUsage.title')"
    :close-on-overlay="true"
    :body-scroll="false"
    modal-class="!max-w-[900px]"
    @close="handleClose"
  >
    <template #header>
      <div class="flex items-center gap-3 flex-1">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor" class="text-accent">
          <path d="M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zM9 17H7v-7h2v7zm4 0h-2V7h2v10zm4 0h-2v-4h2v4z"/>
        </svg>
        <h3 class="modal-title">{{ $t('cursorUsage.title') }}</h3>
      </div>
      <button @click="refresh" class="btn btn--ghost btn--icon" :disabled="loading">
        <svg v-if="!loading" width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
          <path d="M17.65 6.35C16.2 4.9 14.21 4 12 4c-4.42 0-7.99 3.58-7.99 8s3.57 8 7.99 8c3.73 0 6.84-2.55 7.73-6h-2.08c-.82 2.33-3.04 4-5.65 4-3.31 0-6-2.69-6-6s2.69-6 6-6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z"/>
        </svg>
        <span v-else class="btn-spinner"></span>
      </button>
    </template>

    <!-- 基本信息区域 -->
    <div class="mb-6">
      <div v-if="loading && !aggregatedData" class="flex justify-center py-8">
        <span class="btn-spinner btn-spinner--lg"></span>
      </div>
      <div v-else-if="error" class="text-danger text-center py-4">{{ error }}</div>
      <div v-else class="grid grid-cols-3 gap-4">
        <div class="card hover:translate-y-0 p-4">
          <div class="text-xs text-text-muted mb-1">{{ $t('cursorUsage.totalInputTokens') }}</div>
          <div class="text-lg font-semibold text-text-strong" v-tooltip="formatNumberFull(aggregatedData?.totalInputTokens)">{{ formatTokenUnit(aggregatedData?.totalInputTokens) }}</div>
          <div v-if="toNum(aggregatedData?.totalCacheWriteTokens)" class="text-[11px] text-text-muted mt-1">{{ $t('cursorUsage.cacheWrite') }}: <span v-tooltip="formatNumberFull(aggregatedData?.totalCacheWriteTokens)">{{ formatTokenUnit(aggregatedData?.totalCacheWriteTokens) }}</span></div>
        </div>
        <div class="card hover:translate-y-0 p-4">
          <div class="text-xs text-text-muted mb-1">{{ $t('cursorUsage.totalOutputTokens') }}</div>
          <div class="text-lg font-semibold text-text-strong" v-tooltip="formatNumberFull(aggregatedData?.totalOutputTokens)">{{ formatTokenUnit(aggregatedData?.totalOutputTokens) }}</div>
          <div v-if="toNum(aggregatedData?.totalCacheReadTokens)" class="text-[11px] text-text-muted mt-1">{{ $t('cursorUsage.cacheRead') }}: <span v-tooltip="formatNumberFull(aggregatedData?.totalCacheReadTokens)">{{ formatTokenUnit(aggregatedData?.totalCacheReadTokens) }}</span></div>
        </div>
        <div class="card hover:translate-y-0 p-4">
          <div class="text-xs text-text-muted mb-1">{{ $t('cursorUsage.totalCost') }}</div>
          <div v-if="individualUsage?.plan?.breakdown" class="text-[11px] text-text-muted mb-1">{{ $t('cursorUsage.onPlan') }}: {{ $t('cursorUsage.included') }}: ${{ formatCents(individualUsage.plan.breakdown.included) }} {{ $t('cursorUsage.bonus') }}: ${{ formatCents(individualUsage.plan.breakdown.bonus) }}</div>
          <div class="text-lg font-semibold text-text-strong">${{ (aggregatedData?.totalCostCents / 100).toFixed(2) || '0.00' }}</div>
          <div v-if="individualUsage?.onDemand?.enabled" class="text-[11px] text-text-muted mt-1">{{ $t('cursorUsage.onDemand') }}: ${{ formatCents(individualUsage.onDemand.used) }}{{ onDemandLimitLabel }}</div>
        </div>
      </div>
      <div v-if="showPlanQuota || autoRemainingPercent !== null || apiRemainingPercent !== null || showGrokBot" class="mt-3 grid gap-3" :class="quotaGridClass">
        <div v-if="showPlanQuota" class="rounded-md border border-border px-3 py-2">
          <div class="text-[11px] text-text-muted" v-tooltip="$t('platform.cursor.planAvailableHint')">{{ $t('platform.cursor.planAvailable') }}</div>
          <div class="text-sm font-semibold tabular-nums">{{ planRemainingPercent }}%</div>
          <div v-if="planSpend" class="text-[11px] text-text-muted tabular-nums">{{ planSpend }}</div>
        </div>
        <div v-if="autoRemainingPercent !== null" class="rounded-md border border-border px-3 py-2">
          <div class="text-[11px] text-text-muted" v-tooltip="$t('platform.cursor.autoAvailableHint')">{{ $t('platform.cursor.autoAvailable') }}</div>
          <div class="text-sm font-semibold tabular-nums">{{ autoRemainingPercent }}%</div>
        </div>
        <div v-if="apiRemainingPercent !== null" class="rounded-md border border-border px-3 py-2">
          <div class="text-[11px] text-text-muted" v-tooltip="$t('platform.cursor.apiAvailableHint')">{{ $t('platform.cursor.apiAvailable') }}</div>
          <div class="text-sm font-semibold tabular-nums">{{ apiRemainingPercent }}%</div>
        </div>
        <div v-if="showGrokBot" class="rounded-md border border-border px-3 py-2">
          <div class="text-[11px] text-text-muted" v-tooltip="$t('platform.cursor.grokBotAvailableHint')">{{ $t('platform.cursor.grokBotAvailable') }}</div>
          <div class="text-sm font-semibold tabular-nums">{{ grokBotRemaining }}%</div>
        </div>
      </div>
    </div>

    <!-- Tab 切换区域 -->
    <div class="border-b border-border mb-4">
      <div class="flex gap-4">
        <button
          v-for="tab in tabs"
          :key="tab.key"
          :class="[
            'px-4 py-2 text-sm font-medium border-b-2 transition-colors',
            activeTab === tab.key
              ? 'text-accent border-accent'
              : 'text-text-muted border-transparent hover:text-text-strong'
          ]"
          @click="switchTab(tab.key)"
        >
          {{ tab.label }}
        </button>
      </div>
    </div>

    <!-- 内容区域 -->
    <div v-if="isChartView" class="max-h-[calc(65vh-120px)] overflow-y-auto">
      <div v-if="eventsLoading" class="flex justify-center py-8">
        <span class="btn-spinner btn-spinner--lg"></span>
      </div>
      <div v-else-if="eventsError" class="text-danger text-center py-4">{{ eventsError }}</div>
      <template v-else>
        <div class="flex items-center gap-3 text-xs text-text-muted mb-3">
          <span v-if="billingCycleLabel && !chartAllData">{{ $t('cursorUsage.billingCycle') }}: {{ billingCycleLabel }}</span>
          <span v-if="chartAllData">{{ $t('cursorUsage.allDataRange') }}</span>
          <button
            class="px-2 py-0.5 rounded border transition-colors text-[11px]"
            :class="chartAllData
              ? 'bg-accent/15 text-accent border-accent/30'
              : 'bg-transparent text-text-muted border-border hover:text-text-strong'"
            @click="toggleChartAllData"
          >
            {{ $t('cursorUsage.allData') }}
          </button>
        </div>
        <CursorUsageCharts :usage-events="chartEvents" />
      </template>
    </div>

    <!-- 使用事件表格 -->
    <div v-else class="overflow-x-auto max-h-[400px] overflow-y-auto">
      <div v-if="eventsLoading" class="flex justify-center py-8">
        <span class="btn-spinner btn-spinner--lg"></span>
      </div>
      <div v-else-if="eventsError" class="text-danger text-center py-4">{{ eventsError }}</div>
      <div v-else-if="!usageEvents?.length" class="text-text-muted text-center py-8">
        {{ $t('cursorUsage.noEvents') }}
      </div>
      <table v-else class="w-full text-sm">
        <thead>
          <tr>
            <th class="text-left text-xs font-medium text-text-muted uppercase tracking-wider py-3 px-4 border-b border-border">{{ $t('cursorUsage.timestamp') }}</th>
            <th class="text-left text-xs font-medium text-text-muted uppercase tracking-wider py-3 px-4 border-b border-border">{{ $t('cursorUsage.model') }}</th>
            <th class="text-right text-xs font-medium text-text-muted uppercase tracking-wider py-3 px-4 border-b border-border">{{ $t('cursorUsage.inputTokens') }}</th>
            <th class="text-right text-xs font-medium text-text-muted uppercase tracking-wider py-3 px-4 border-b border-border">{{ $t('cursorUsage.outputTokens') }}</th>
            <th class="text-right text-xs font-medium text-text-muted uppercase tracking-wider py-3 px-4 border-b border-border">{{ $t('cursorUsage.cost') }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(event, index) in usageEvents" :key="index" class="hover:bg-hover">
            <td class="py-3 px-4 border-b border-border text-text-strong">{{ formatTimestamp(event.timestamp) }}</td>
            <td class="py-3 px-4 border-b border-border text-text-strong">{{ event.model }}</td>
            <td class="py-3 px-4 border-b border-border text-text-strong text-right">
              <span v-tooltip="formatNumberFull(event.tokenUsage?.inputTokens) + (toNum(event.tokenUsage?.cacheWriteTokens) ? `\n${$t('cursorUsage.cacheWrite')}: ${formatNumberFull(event.tokenUsage?.cacheWriteTokens)}` : '')">
                {{ formatTokenUnit(event.tokenUsage?.inputTokens) }}
              </span>
            </td>
            <td class="py-3 px-4 border-b border-border text-text-strong text-right">
              <span v-tooltip="formatNumberFull(event.tokenUsage?.outputTokens) + (toNum(event.tokenUsage?.cacheReadTokens) ? `\n${$t('cursorUsage.cacheRead')}: ${formatNumberFull(event.tokenUsage?.cacheReadTokens)}` : '')">
                {{ formatTokenUnit(event.tokenUsage?.outputTokens) }}
              </span>
            </td>
            <td class="py-3 px-4 border-b border-border text-text-strong text-right">{{ event.tokenUsage?.totalCents ? `$${(event.tokenUsage.totalCents / 100).toFixed(4)}` : '-' }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </BaseModal>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import BaseModal from '../common/BaseModal.vue'
import CursorUsageCharts from './CursorUsageCharts.vue'
import { applyCursorUsageSummary } from '../../utils/cursorUsage'
import { useCursorQuota } from '../../composables/useCursorQuota'

const { t: $t } = useI18n()

const props = defineProps({
  account: {
    type: Object,
    required: true
  }
})

const emit = defineEmits(['close', 'account-synced'])

// 状态
const loading = ref(false)
const error = ref(null)
const eventsLoading = ref(false)
const eventsError = ref(null)
const aggregatedData = ref(null)
const usageEvents = ref([])
const activeTab = ref('1day')

const tabs = [
  { key: '1day', label: $t('cursorUsage.tab1Day'), days: 1 },
  { key: '1week', label: $t('cursorUsage.tab1Week'), days: 7 },
  { key: '1month', label: $t('cursorUsage.tab1Month'), days: 30 },
  { key: 'charts', label: $t('cursorUsage.tabCharts'), days: 30 }
]

const sessionToken = computed(() => props.account.token?.workos_cursor_session_token)
const individualUsage = computed(() => props.account.individual_usage)
const {
  planRemainingPercent,
  planSpend,
  showPlanQuota,
  autoRemainingPercent,
  apiRemainingPercent,
  grokBotRemaining,
  showGrokBot
} = useCursorQuota(() => props.account)
const QUOTA_GRID_CLASSES = ['grid-cols-1', 'grid-cols-1', 'grid-cols-2', 'grid-cols-3', 'grid-cols-4']
const quotaGridClass = computed(() => {
  const count = [
    showPlanQuota.value,
    autoRemainingPercent.value !== null,
    apiRemainingPercent.value !== null,
    showGrokBot.value
  ].filter(Boolean).length
  return QUOTA_GRID_CLASSES[count]
})
const isChartView = computed(() => activeTab.value === 'charts')
const chartEvents = ref([])
const chartAllData = ref(false)
const billingCycleLabel = computed(() => {
  const usage = individualUsage.value
  if (!usage?.billingCycleStart || !usage?.billingCycleEnd) return null
  const fmt = (iso) => new Date(iso).toLocaleDateString()
  return `${fmt(usage.billingCycleStart)} - ${fmt(usage.billingCycleEnd)}`
})

const formatCents = (value) => {
  if (!value && value !== 0) return '0.00'
  return (value / 100).toFixed(2)
}

// limit 为空表示未设置 On-Demand 上限，不能渲染成 $0.00
const onDemandLimitLabel = computed(() => {
  const limit = individualUsage.value?.onDemand?.limit
  return limit === null || limit === undefined ? '' : ` / $${formatCents(limit)}`
})

// 日期格式化
const getDateRange = (days) => {
  const end = new Date()
  const start = new Date()
  start.setDate(start.getDate() - days)
  return {
    startDate: start.toISOString().split('T')[0],
    endDate: end.toISOString().split('T')[0],
    startTimestamp: start.getTime(),  // 毫秒
    endTimestamp: end.getTime()       // 毫秒
  }
}

const formatNumber = (value) => {
  if (!value) return '0'
  return parseInt(value).toLocaleString()
}

const toNum = (value) => {
  if (!value) return 0
  return parseInt(value) || 0
}

const formatTokenUnit = (value) => {
  const n = toNum(value)
  if (n === 0) return '0'
  if (n >= 1e9) return (n / 1e9).toFixed(2) + 'B'
  if (n >= 1e6) return (n / 1e6).toFixed(2) + 'M'
  if (n >= 1e3) return (n / 1e3).toFixed(1) + 'K'
  return n.toString()
}

const formatNumberFull = (value) => {
  const n = toNum(value)
  return n.toLocaleString()
}

const formatTimestamp = (timestamp) => {
  if (!timestamp) return '-'
  // 处理毫秒时间戳（数字或字符串）和 ISO 字符串
  let date
  if (typeof timestamp === 'number') {
    date = new Date(timestamp)
  } else if (typeof timestamp === 'string') {
    // 尝试作为数字解析（毫秒时间戳字符串）
    const num = Number(timestamp)
    if (!isNaN(num) && num > 1000000000000) {
      date = new Date(num)
    } else {
      // 尝试作为 ISO 字符串解析
      date = new Date(timestamp)
    }
  } else {
    return '-'
  }
  return isNaN(date.getTime()) ? '-' : date.toLocaleString()
}

// 从 events 计算聚合数据
const calculateAggregatedFromEvents = (events) => {
  if (!events?.length) return null

  let totalInputTokens = 0
  let totalOutputTokens = 0
  let totalCostCents = 0

  for (const event of events) {
    if (event.tokenUsage) {
      totalInputTokens += event.tokenUsage.inputTokens || 0
      totalOutputTokens += event.tokenUsage.outputTokens || 0
      totalCostCents += (event.tokenUsage.totalCents || 0) * 100
    }
  }

  return {
    total_input_tokens: totalInputTokens.toString(),
    total_output_tokens: totalOutputTokens.toString(),
    total_cost_cents: totalCostCents
  }
}

// 获取聚合数据
const fetchBasicData = async () => {
  if (!sessionToken.value) {
    error.value = 'No session token'
    return
  }

  loading.value = true
  error.value = null

  try {
    const { startTimestamp, endTimestamp } = getDateRange(30)

    const aggData = await invoke('cursor_get_aggregated_usage', {
      sessionToken: sessionToken.value,
      startDate: startTimestamp,
      endDate: endTimestamp,
      teamId: 0
    })
    aggregatedData.value = aggData
  } catch (e) {
    error.value = e.toString()
    console.error('Failed to fetch basic data:', e)
  } finally {
    loading.value = false
  }
}

// 获取使用事件
const fetchUsageEvents = async () => {
  if (!sessionToken.value) return

  const currentTab = tabs.find(t => t.key === activeTab.value)
  if (!currentTab) return

  eventsLoading.value = true
  eventsError.value = null

  try {
    const { startTimestamp, endTimestamp } = getDateRange(currentTab.days)
    const result = await invoke('cursor_get_filtered_usage_events', {
      sessionToken: sessionToken.value,
      startDate: String(startTimestamp),
      endDate: String(endTimestamp),
      page: 1,
      pageSize: 1000,
      teamId: 0
    })
    const events = result?.usageEventsDisplay || []
    usageEvents.value = events
  } catch (e) {
    eventsError.value = e.toString()
    console.error('Failed to fetch usage events:', e)
  } finally {
    eventsLoading.value = false
  }
}

const switchTab = (tabKey) => {
  activeTab.value = tabKey
  if (tabKey === 'charts') {
    fetchChartEvents()
  } else {
    fetchUsageEvents()
  }
}

// 获取图表数据 (优先使用账期范围，否则回退30天)
const getChartDateRange = () => {
  const usage = individualUsage.value
  if (usage?.billingCycleStart && usage?.billingCycleEnd) {
    return {
      startTimestamp: new Date(usage.billingCycleStart).getTime(),
      endTimestamp: new Date(usage.billingCycleEnd).getTime()
    }
  }
  return getDateRange(30)
}

const fetchChartEvents = async () => {
  if (!sessionToken.value) return

  eventsLoading.value = true
  eventsError.value = null

  try {
    const params = {
      sessionToken: sessionToken.value,
      teamId: 0
    }
    if (chartAllData.value) {
      // 查全部数据：只传 teamId，不传日期和分页
    } else {
      const { startTimestamp, endTimestamp } = getChartDateRange()
      params.startDate = String(startTimestamp)
      params.endDate = String(endTimestamp)
      params.page = 1
      params.pageSize = 1000
    }
    const result = await invoke('cursor_get_filtered_usage_events', params)
    chartEvents.value = result?.usageEventsDisplay || []
  } catch (e) {
    eventsError.value = e.toString()
    console.error('Failed to fetch chart events:', e)
  } finally {
    eventsLoading.value = false
  }
}

const toggleChartAllData = () => {
  chartAllData.value = !chartAllData.value
  fetchChartEvents()
}

// 刷新用量摘要（更新账期和配额）
const fetchUsageSummary = async () => {
  if (!sessionToken.value) return
  try {
    const summary = await invoke('cursor_get_usage_summary', {
      sessionToken: sessionToken.value,
      accessToken: props.account.token?.access_token || null
    })
    applyCursorUsageSummary(props.account, summary)
    await invoke('cursor_update_account', { account: props.account })
    emit('account-synced', props.account.id)
  } catch (e) {
    console.error('Failed to fetch usage summary:', e)
  }
}

const refresh = async () => {
  if (loading.value || eventsLoading.value) return
  await fetchUsageSummary()
  fetchBasicData()
  if (isChartView.value) {
    fetchChartEvents()
  } else {
    fetchUsageEvents()
  }
}

const handleClose = () => {
  emit('close')
}

onMounted(() => {
  fetchBasicData()
  fetchUsageEvents()
})
</script>

