<template>
  <div
    class="bg-surface border border-border rounded-lg p-3 cursor-pointer relative group transition-all duration-150 hover:bg-hover hover:border-border-strong"
    :class="{
      'opacity-60 pointer-events-none': isSwitching,
      'border-accent bg-accent/5': isSelected
    }"
    @click="handleCardClick"
  >
    <!-- 头部：选择框 + 邮箱标题 -->
    <div class="flex items-center gap-2 mb-3 pr-8">
      <!-- 选择框 -->
      <div
        class="selection-checkbox"
        :class="{ 'visible': selectionMode || isSelected }"
        @click.stop="toggleSelection"
      >
        <div class="checkbox-inner" :class="{ 'checked': isSelected }">
          <svg v-if="isSelected" class="h-3 w-3" viewBox="0 0 24 24" fill="currentColor">
            <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z" />
          </svg>
        </div>
      </div>

      <!-- 邮箱 -->
      <div
        class="inline-flex items-center gap-1 cursor text-[15px] font-semibold text-text truncate flex-1"
        v-tooltip="account.email"
        @click.stop="copyEmail"
      >
        <span>{{ showRealEmail ? account.email : maskedEmail }}</span>
      </div>
    </div>

    <!-- 右上角状态徽章 -->
    <div class="absolute right-3 top-3 z-10 flex items-center gap-1.5">
      <span v-if="isCurrent" :class="['badge', statusBadgeClass]">
        <span class="status-dot" :class="statusDotClass"></span>
        {{ statusLabel }}
      </span>
    </div>

    <!-- 右上角按钮组（悬停显示） -->
    <div
      class="absolute right-3 top-3 z-20 flex items-center gap-1.5 opacity-0 group-hover:opacity-100 transition-opacity"
      :class="{ 'opacity-100': isMenuOpen }"
      @click.stop
    >
      <!-- 切换按钮 -->
      <button
        v-if="!isCurrent"
        @click="$emit('switch', account.id)"
        class="w-7 h-7 rounded border-none bg-surface text-text-secondary cursor-pointer flex items-center justify-center shadow-sm hover:bg-hover hover:text-accent transition-colors"
        :disabled="isSwitching"
        v-tooltip="$t('platform.cursor.switch')"
      >
        <svg v-if="!isSwitching" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="currentColor">
          <path d="M6.99 11L3 15l3.99 4v-3H14v-2H6.99v-3zM21 9l-3.99-4v3H10v2h7.01v3L21 9z"/>
        </svg>
        <span v-else class="btn-spinner btn-spinner--sm text-accent"></span>
      </button>

      <!-- 刷新配额按钮 -->
      <button
        v-if="hasSessionToken"
        @click="$emit('refresh-quota', account.id)"
        class="w-7 h-7 rounded border-none bg-surface text-text-secondary cursor-pointer flex items-center justify-center shadow-sm hover:bg-hover hover:text-accent transition-colors"
        :disabled="isRefreshing"
        v-tooltip="$t('platform.cursor.refreshQuota')"
      >
        <svg v-if="!isRefreshing" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="currentColor">
          <path d="M17.65 6.35C16.2 4.9 14.21 4 12 4c-4.42 0-7.99 3.58-7.99 8s3.57 8 7.99 8c3.73 0 6.84-2.55 7.73-6h-2.08c-.82 2.33-3.04 4-5.65 4-3.31 0-6-2.69-6-6s2.69-6 6-6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z"/>
        </svg>
        <span v-else class="btn-spinner btn-spinner--sm text-accent"></span>
      </button>

      <!-- 操作菜单 -->
      <FloatingDropdown
        ref="menuRef"
        placement="bottom-end"
        :close-on-select="true"
        @open="isMenuOpen = true"
        @close="isMenuOpen = false"
      >
        <template #trigger>
          <button
            class="w-7 h-7 rounded border-none bg-surface text-text-secondary cursor-pointer flex items-center justify-center shadow-sm hover:bg-hover hover:text-text transition-colors"
            v-tooltip="$t('app.moreOptions')"
          >
            <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 8c1.1 0 2-.9 2-2s-.9-2-2-2-2 .9-2 2 .9 2 2 2zm0 2c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2zm0 6c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2z"/>
            </svg>
          </button>
        </template>
        <template #default="{ close }">
          <button @click="handleMenuClick('copyAccessToken', close)" class="dropdown-item">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
              <path d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/>
            </svg>
            <span>{{ $t('accountCard.copyAccessToken') }}</span>
          </button>
          <button
            v-if="hasSessionToken"
            @click="handleMenuClick('copySessionToken', close)"
            class="dropdown-item"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
              <path d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/>
            </svg>
            <span>{{ $t('accountCard.copySessionToken') }}</span>
          </button>
          <button @click="handleMenuClick('generateMachineId', close)" class="dropdown-item">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
              <path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/>
            </svg>
            <span>{{ $t('accountCard.generateAndBindMachineId') }}</span>
          </button>
          <button @click="handleMenuClick('export', close)" class="dropdown-item">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
              <path d="M19 9h-4V3H9v6H5l7 7 7-7zM5 18v2h14v-2H5z"/>
            </svg>
            <span>{{ $t('accountCard.export') }}</span>
          </button>
          <button @click="handleMenuClick('delete', close)" class="dropdown-item text-danger hover:bg-danger/10">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
              <path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/>
            </svg>
            <span>{{ $t('common.delete') }}</span>
          </button>
        </template>
      </FloatingDropdown>
    </div>

    <!-- 属性列表 -->
    <div class="flex flex-col gap-1.5">
      <!-- 创建时间 -->
      <div v-if="account.created_at" class="flex items-center gap-1 min-h-6">
        <div class="flex items-center gap-1.5 w-[90px] shrink-0 text-text-muted text-xs">
          <svg class="w-3.5 h-3.5 shrink-0 opacity-70" viewBox="0 0 24 24" fill="currentColor">
            <path d="M11.99 2C6.47 2 2 6.48 2 12s4.47 10 9.99 10C17.52 22 22 17.52 22 12S17.52 2 11.99 2zM12 20c-4.42 0-8-3.58-8-8s3.58-8 8-8 8 3.58 8 8-3.58 8-8 8zm.5-13H11v6l5.25 3.15.75-1.23-4.5-2.67z"/>
          </svg>
          <span>{{ $t('tokenCard.createdAt') }}</span>
        </div>
        <div class="flex-1 text-[13px] text-text-muted truncate">
          {{ formatDate(account.created_at) }}
        </div>
      </div>

      <!-- 订阅计划类型 -->
      <div v-if="account.membership_type" class="flex items-center gap-1 min-h-6">
        <div class="flex items-center gap-1.5 w-[90px] shrink-0 text-text-muted text-xs">
          <svg class="w-3.5 h-3.5 shrink-0 opacity-70" viewBox="0 0 24 24" fill="currentColor">
            <path d="M20 4H4c-1.11 0-1.99.89-1.99 2L2 18c0 1.11.89 2 2 2h16c1.11 0 2-.89 2-2V6c0-1.11-.89-2-2-2zm0 14H4v-6h16v6zm0-10H4V6h16v2z"/>
          </svg>
          <span>{{ $t('platform.cursor.membershipType') }}</span>
        </div>
        <div class="flex-1">
          <span :class="getMembershipBadgeClass(account.membership_type)">
            <svg v-if="account.membership_type?.toLowerCase() === 'ultra'" class="w-3 h-3" viewBox="0 0 24 24" fill="currentColor">
              <path d="M19 3H5L2 9l10 12L22 9l-3-6zM9.62 8l1.5-3h1.76l1.5 3H9.62zM11 10v6.68L5.44 10H11zm2 0h5.56L13 16.68V10zm6.26-2h-2.65l-1.5-3h2.65l1.5 3zM6.24 5h2.65l-1.5 3H4.74l1.5-3z"/>
            </svg>
            <svg v-else-if="account.membership_type?.toLowerCase() === 'pro'" class="w-3 h-3" viewBox="0 0 24 24" fill="currentColor">
              <path d="M5 16L3 5l5.5 5L12 4l3.5 6L21 5l-2 11H5zm14 3c0 .6-.4 1-1 1H6c-.6 0-1-.4-1-1v-1h14v1z"/>
            </svg>
            <svg v-else-if="account.membership_type?.toLowerCase() === 'pro plus'" class="w-3 h-3" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 2l2.4 7.2L22 12l-7.6 2.8L12 22l-2.4-7.2L2 12l7.6-2.8z"/>
            </svg>
            {{ account.membership_type }}
          </span>
        </div>
      </div>

      <!-- 套餐剩余（官方账单口径） -->
      <div v-if="showPlanQuota && hasSessionToken" class="flex items-center gap-1 min-h-6">
        <div class="flex items-center gap-1.5 w-[90px] shrink-0 text-text-muted text-xs">
          <svg class="w-3.5 h-3.5 shrink-0 opacity-70" viewBox="0 0 24 24" fill="currentColor">
            <path d="M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm0 16H5V5h14v14zM7 12h2v5H7zm4-3h2v8h-2zm4-3h2v11h-2z"/>
          </svg>
          <span v-tooltip="planHint">{{ $t('platform.cursor.planAvailable') }}</span>
        </div>
        <div class="flex-1 flex items-center gap-1">
          <div class="flex-1 h-1.5 bg-muted rounded overflow-hidden">
            <div class="h-full rounded transition-all"
                 :class="getQuotaBarClass(planRemainingPercent)"
                 :style="{ width: planRemainingPercent + '%' }">
            </div>
          </div>
          <span class="text-[11px] font-medium tabular-nums text-text-muted w-8 text-right">
            {{ planRemainingPercent }}%
          </span>
        </div>
      </div>

      <!-- 两池合计剩余（Spending 页 Total 进度条） -->
      <div v-if="totalRemainingPercent !== null && hasSessionToken" class="flex items-center gap-1 min-h-6">
        <div class="flex items-center gap-1.5 w-[90px] shrink-0 text-text-muted text-xs">
          <svg class="w-3.5 h-3.5 shrink-0 opacity-70" viewBox="0 0 24 24" fill="currentColor">
            <path d="M11 2v20c-5.07-.5-9-4.79-9-10s3.93-9.5 9-10zm2.03 0v8.99H22c-.47-4.74-4.24-8.52-8.97-8.99zm0 11.01V22c4.74-.47 8.5-4.25 8.97-8.99h-8.97z"/>
          </svg>
          <span v-tooltip="$t('platform.cursor.totalAvailableHint')">{{ $t('platform.cursor.totalAvailable') }}</span>
        </div>
        <div class="flex-1 flex items-center gap-1">
          <div class="flex-1 h-1.5 bg-muted rounded overflow-hidden">
            <div class="h-full rounded transition-all"
                 :class="getQuotaBarClass(totalRemainingPercent)"
                 :style="{ width: totalRemainingPercent + '%' }">
            </div>
          </div>
          <span class="text-[11px] font-medium tabular-nums text-text-muted w-8 text-right">
            {{ totalRemainingPercent }}%
          </span>
        </div>
      </div>

      <!-- Auto 剩余 -->
      <div v-if="autoRemainingPercent !== null && hasSessionToken" class="flex items-center gap-1 min-h-6">
        <div class="flex items-center gap-1.5 w-[90px] shrink-0 text-text-muted text-xs">
          <svg class="w-3.5 h-3.5 shrink-0 opacity-70" viewBox="0 0 24 24" fill="currentColor">
            <path d="M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm0 16H5V5h14v14zM7 12h2v5H7zm4-3h2v8h-2zm4-3h2v11h-2z"/>
          </svg>
          <span v-tooltip="$t('platform.cursor.autoAvailableHint')">{{ $t('platform.cursor.autoAvailable') }}</span>
        </div>
        <div class="flex-1 flex items-center gap-1">
          <div class="flex-1 h-1.5 bg-muted rounded overflow-hidden">
            <div class="h-full rounded transition-all"
                 :class="getQuotaBarClass(autoRemainingPercent)"
                 :style="{ width: autoRemainingPercent + '%' }">
            </div>
          </div>
          <span class="text-[11px] font-medium tabular-nums text-text-muted w-8 text-right">
            {{ autoRemainingPercent }}%
          </span>
        </div>
      </div>

      <!-- API 剩余 -->
      <div v-if="apiRemainingPercent !== null && hasSessionToken" class="flex items-center gap-1 min-h-6">
        <div class="flex items-center gap-1.5 w-[90px] shrink-0 text-text-muted text-xs">
          <svg class="w-3.5 h-3.5 shrink-0 opacity-70" viewBox="0 0 24 24" fill="currentColor">
            <path d="M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm0 16H5V5h14v14zM7 12h2v5H7zm4-3h2v8h-2zm4-3h2v11h-2z"/>
          </svg>
          <span v-tooltip="$t('platform.cursor.apiAvailableHint')">{{ $t('platform.cursor.apiAvailable') }}</span>
        </div>
        <div class="flex-1 flex items-center gap-1">
          <div class="flex-1 h-1.5 bg-muted rounded overflow-hidden">
            <div class="h-full rounded transition-all"
                 :class="getQuotaBarClass(apiRemainingPercent)"
                 :style="{ width: apiRemainingPercent + '%' }">
            </div>
          </div>
          <span class="text-[11px] font-medium tabular-nums text-text-muted w-8 text-right">
            {{ apiRemainingPercent }}%
          </span>
        </div>
      </div>

      <!-- Grok Bot 周额 -->
      <div v-if="showGrokBot && hasSessionToken" class="flex items-center gap-1 min-h-6">
        <div class="flex items-center gap-1.5 w-[90px] shrink-0 text-text-muted text-xs">
          <svg class="w-3.5 h-3.5 shrink-0 opacity-70" viewBox="0 0 24 24" fill="currentColor">
            <path d="M20 9V7c0-1.1-.9-2-2-2h-3c0-1.66-1.34-3-3-3S9 3.34 9 5H6c-1.1 0-2 .9-2 2v2c-1.66 0-3 1.34-3 3s1.34 3 3 3v4c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2v-4c1.66 0 3-1.34 3-3s-1.34-3-3-3zM7.5 11.5c0-.83.67-1.5 1.5-1.5s1.5.67 1.5 1.5S9.83 13 9 13s-1.5-.67-1.5-1.5zM16 17H8v-2h8v2zm-1-4c-.83 0-1.5-.67-1.5-1.5S14.17 10 15 10s1.5.67 1.5 1.5S15.83 13 15 13z"/>
          </svg>
          <span v-tooltip="grokBotHint">{{ $t('platform.cursor.grokBotAvailable') }}</span>
        </div>
        <div class="flex-1 flex items-center gap-1">
          <div class="flex-1 h-1.5 bg-muted rounded overflow-hidden">
            <div class="h-full rounded transition-all"
                 :class="getQuotaBarClass(grokBotRemaining)"
                 :style="{ width: grokBotRemaining + '%' }">
            </div>
          </div>
          <span class="text-[11px] font-medium tabular-nums text-text-muted w-8 text-right">
            {{ grokBotRemaining }}%
          </span>
        </div>
      </div>

      <!-- 配额（按钮） -->
      <div v-if="hasSessionToken" class="flex items-center gap-1 min-h-6">
        <div class="flex items-center gap-1.5 w-[90px] shrink-0 text-text-muted text-xs">
          <svg class="w-3.5 h-3.5 shrink-0 opacity-70" viewBox="0 0 24 24" fill="currentColor">
            <path d="M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm0 16H5V5h14v14zM7 12h2v5H7zm4-3h2v8h-2zm4-3h2v11h-2z"/>
          </svg>
          <span>{{ $t('platform.cursor.quotaLabel') }}</span>
        </div>
        <div class="flex-1 text-[13px]">
          <button
            @click.stop="showUsageModal = true"
            class="inline-flex items-center gap-1 px-2 py-0.5 text-xs text-accent bg-accent/10 border border-accent/30 rounded hover:bg-accent/20 transition-colors cursor-pointer"
          >
            <svg class="w-3 h-3" viewBox="0 0 24 24" fill="currentColor">
              <path d="M3 3h2v18H3V3zm16 8h2v10h-2V11zm-8 4h2v6h-2v-6zm4-8h2v14h-2V7zm-8 6h2v8H7v-8z"/>
            </svg>
            {{ $t('cursorUsage.viewUsage') }}
          </button>
        </div>
      </div>

      <!-- 标签 -->
      <div class="flex items-center gap-1 min-h-6">
        <div class="flex items-center gap-1.5 w-[90px] shrink-0 text-text-muted text-xs">
          <svg class="w-3.5 h-3.5 shrink-0 opacity-70" viewBox="0 0 24 24" fill="currentColor">
            <path d="M17.63 5.84C17.27 5.33 16.67 5 16 5L5 5.01C3.9 5.01 3 5.9 3 7v10c0 1.1.9 1.99 2 1.99L16 19c.67 0 1.27-.33 1.63-.84L22 12l-4.37-6.16z"/>
          </svg>
          <span>{{ $t('subscriptions.fields.tag') }}</span>
        </div>
        <div class="flex-1 text-[13px]">
          <span
            v-if="!account.tag"
            class="inline-flex items-center gap-0.5 px-1.5 py-0.5 border border-dashed border-border rounded text-text-muted text-xs cursor-pointer hover:border-accent hover:text-accent transition-colors"
            @click.stop="openTagEditor"
          >
            <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
              <path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/>
            </svg>
            {{ $t('tokenList.clickToAddTag') }}
          </span>
          <span
            v-else
            class="badge editable badge--sm"
            :style="{ '--tag-color': account.tag_color || DEFAULT_TAG_COLOR }"
            @click.stop="openTagEditor"
          >
            {{ account.tag }}
          </span>
        </div>
      </div>
    </div>
  </div>

  <!-- 标签编辑模态框 -->
  <TagEditorModal
    v-model:visible="showTagEditor"
    :token="accountAsToken"
    :all-tokens="allAccountsAsTokens"
    @save="handleTagSave"
    @clear="handleTagClear"
  />

  <!-- 使用详情模态框 -->
  <CursorUsageModal
    v-if="showUsageModal"
    :account="account"
    @close="showUsageModal = false"
    @account-synced="(id) => $emit('account-synced', id)"
  />
</template>

<script setup>
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import FloatingDropdown from '../common/FloatingDropdown.vue'
import TagEditorModal from '../token/TagEditorModal.vue'
import CursorUsageModal from './CursorUsageModal.vue'
import { useCursorQuota } from '../../composables/useCursorQuota'

const { t: $t } = useI18n()

const props = defineProps({
  account: { type: Object, required: true },
  isCurrent: { type: Boolean, default: false },
  isSwitching: { type: Boolean, default: false },
  isRefreshing: { type: Boolean, default: false },
  isSelected: { type: Boolean, default: false },
  selectionMode: { type: Boolean, default: false },
  showRealEmail: { type: Boolean, default: true },
  allAccounts: { type: Array, default: () => [] }
})

const emit = defineEmits(['switch', 'delete', 'select', 'account-updated', 'account-synced', 'machine-id-generated', 'refresh-quota'])

const menuRef = ref(null)
const showTagEditor = ref(false)
const showUsageModal = ref(false)
const isMenuOpen = ref(false)
const isGeneratingMachineId = ref(false)
const DEFAULT_TAG_COLOR = '#f97316'

// 订阅计划徽章样式
const getMembershipBadgeClass = (type) => {
  const base = 'badge badge--sm uppercase'
  switch (type?.toLowerCase()) {
    case 'ultra':
      return `${base} bg-gradient-to-r from-rose-400 to-pink-500 text-white border-pink-500/50 shadow-sm shadow-pink-500/30`
    case 'pro':
      return `${base} bg-gradient-to-r from-amber-400 to-amber-500 text-amber-900 border-amber-500/50`
    case 'pro plus':
      return `${base} bg-gradient-to-r from-emerald-400 to-teal-500 text-white border-teal-500/50`
    default:
      return base
  }
}

const hasSessionToken = computed(() => !!props.account.token?.workos_cursor_session_token)

const {
  planRemainingPercent,
  planSpend,
  showPlanQuota,
  totalRemainingPercent,
  autoRemainingPercent,
  apiRemainingPercent,
  grokBotRemaining,
  showGrokBot,
  grokBotResetLabel,
  getQuotaBarClass
} = useCursorQuota(() => props.account)

const planHint = computed(() => {
  const base = $t('platform.cursor.planAvailableHint')
  return planSpend.value ? `${base} · ${planSpend.value}` : base
})

const grokBotHint = computed(() => {
  const base = $t('platform.cursor.grokBotAvailableHint')
  return grokBotResetLabel.value
    ? `${base} · ${$t('platform.cursor.grokBotResets', { date: grokBotResetLabel.value })}`
    : base
})

const maskedEmail = computed(() => {
  const email = props.account.email
  if (!email || !email.includes('@')) return email
  return 'hello@cursor.com'
})

const statusClass = computed(() => {
  if (props.isCurrent) return 'current'
  if (props.account.disabled) return 'disabled'
  return 'available'
})

const statusBadgeClass = computed(() => {
  switch (statusClass.value) {
    case 'current': return 'badge--success-tech'
    default: return ''
  }
})

const statusDotClass = computed(() => {
  switch (statusClass.value) {
    case 'current': return 'text-success'
    default: return ''
  }
})

const statusLabel = computed(() => {
  if (props.isCurrent) return $t('platform.cursor.status.current')
  return ''
})

const formatDate = (timestamp) => {
  if (!timestamp) return '-'
  const date = new Date(timestamp * 1000)
  return date.toLocaleDateString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit'
  })
}

const toggleSelection = () => emit('select', props.account.id)

const handleCardClick = () => {
  if (props.selectionMode) toggleSelection()
}

const copyEmail = async () => {
  try {
    await navigator.clipboard.writeText(props.account.email)
    window.$notify?.success($t('messages.emailNoteCopied'))
  } catch (err) {
    window.$notify?.error($t('messages.copyEmailNoteFailed'))
  }
}

// 菜单操作
const handleMenuClick = async (type, close) => {
  close?.()
  switch (type) {
    case 'copyAccessToken':
      await copyAccessToken()
      break
    case 'copySessionToken':
      await copySessionToken()
      break
    case 'generateMachineId':
      await generateAndBindMachineId()
      break
    case 'export':
      await exportAccount()
      break
    case 'delete':
      emit('delete', props.account.id)
      break
  }
}

const copyAccessToken = async () => {
  try {
    const accessToken = props.account.token?.access_token
    if (!accessToken) {
      window.$notify?.error($t('messages.noAccessToken'))
      return
    }
    await navigator.clipboard.writeText(accessToken)
    window.$notify?.success($t('messages.accessTokenCopied'))
  } catch (err) {
    window.$notify?.error($t('messages.copyFailed'))
  }
}

const copySessionToken = async () => {
  try {
    const sessionToken = props.account.token?.workos_cursor_session_token
    if (!sessionToken) {
      window.$notify?.error($t('messages.noSessionToken'))
      return
    }
    await navigator.clipboard.writeText(sessionToken)
    window.$notify?.success($t('messages.sessionTokenCopied'))
  } catch (err) {
    window.$notify?.error($t('messages.copyFailed'))
  }
}

const generateAndBindMachineId = async () => {
  if (isGeneratingMachineId.value) return

  isGeneratingMachineId.value = true
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    const result = await invoke('cursor_generate_and_bind_machine_id', {
      accountId: props.account.id
    })
    window.$notify?.success(result.message || $t('platform.cursor.machineIdGenerated'))
    emit('machine-id-generated', props.account.id)
  } catch (err) {
    console.error('Generate machine ID error:', err)
    window.$notify?.error(err?.message || err || $t('platform.cursor.machineIdGenerateFailed'))
  } finally {
    isGeneratingMachineId.value = false
  }
}

const exportAccount = async () => {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    const { save } = await import('@tauri-apps/plugin-dialog')
    const { writeTextFile } = await import('@tauri-apps/plugin-fs')

    // 获取导出数据
    const jsonData = await invoke('cursor_export_accounts', {
      accountIds: [props.account.id]
    })

    // 生成默认文件名
    const defaultFileName = `cursor_account_${props.account.email.replace(/[^a-zA-Z0-9]/g, '_')}.json`

    // 让用户选择保存位置
    const filePath = await save({
      defaultPath: defaultFileName,
      filters: [
        {
          name: 'JSON',
          extensions: ['json']
        }
      ]
    })

    if (!filePath) {
      return // 用户取消
    }

    // 写入文件
    await writeTextFile(filePath, jsonData)

    window.$notify?.success($t('platform.cursor.messages.exportSuccess'))
  } catch (err) {
    console.error('Export account error:', err)
    if (err?.message?.includes('Cancelled') || err?.code === 'Cancelled') {
      return // 用户取消，不显示错误
    }
    window.$notify?.error(err?.message || err || $t('platform.cursor.messages.exportFailed'))
  }
}

// 标签操作
const accountAsToken = computed(() => ({
  tag_name: props.account.tag || '',
  tag_color: props.account.tag_color || ''
}))

const allAccountsAsTokens = computed(() =>
  props.allAccounts.map(acc => ({
    tag_name: acc.tag || '',
    tag_color: acc.tag_color || ''
  }))
)

const openTagEditor = () => {
  showTagEditor.value = true
}

const handleTagSave = ({ tagName, tagColor }) => {
  props.account.tag = tagName
  props.account.tag_color = tagColor
  props.account.updated_at = Math.floor(Date.now() / 1000)
  emit('account-updated', props.account)
  window.$notify?.success($t('messages.tagUpdated'))
}

const handleTagClear = () => {
  props.account.tag = ''
  props.account.tag_color = ''
  props.account.updated_at = Math.floor(Date.now() / 1000)
  emit('account-updated', props.account)
  window.$notify?.success($t('messages.tagCleared'))
}
</script>

