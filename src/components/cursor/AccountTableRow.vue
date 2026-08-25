<template>
  <tr
    :class="[
      'group transition-colors duration-200',
      'hover:bg-accent/6',
      isSelected ? 'bg-accent/10' : ''
    ]"
    @click="handleRowClick"
  >
    <!-- 多选框 -->
    <td class="w-11 text-center px-2.5 py-3.5 border-b border-border/50 align-top whitespace-nowrap text-[13px] text-text relative first-cell">
      <div class="inline-flex items-center justify-center h-5 cursor-pointer align-middle leading-none" @click.stop="toggleSelection">
        <div class="checkbox-inner" :class="{ 'checked': isSelected }">
          <svg v-if="isSelected" class="h-3 w-3" viewBox="0 0 24 24" fill="currentColor">
            <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z" />
          </svg>
        </div>
      </div>
    </td>

    <!-- 标签 -->
    <td class="w-[60px] px-2.5 py-3.5 border-b border-border/50 align-top whitespace-nowrap text-[13px] text-text">
      <span
        v-if="!hasTag"
        class="btn btn--icon-sm btn--dashed"
        v-tooltip="$t('tokenList.clickToAddTag')"
        @click.stop="openTagEditor"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
          <path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/>
        </svg>
      </span>
      <span
        v-else
        class="badge editable badge--sm max-w-[50px]"
        :style="tagBadgeStyle"
        v-tooltip="$t('tokenList.clickToEditTag')"
        @click.stop="openTagEditor"
      >
        {{ tagDisplayName }}
      </span>
    </td>

    <!-- 邮箱 -->
    <td class="px-2.5 py-3.5 border-b border-border/50 align-top whitespace-nowrap text-[13px] text-text">
      <div class="flex items-center gap-1.5">
        <div class="text-copyable" @click.stop="copyEmail" v-tooltip="account.email">
          <span class="text-copyable__content">{{ showRealEmail ? account.email : maskedEmail }}</span>
        </div>
        <span v-if="account.membership_type" :class="getMembershipBadgeClass(account.membership_type)">
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
    </td>

    <!-- 可用额度 -->
    <td class="w-[150px] px-2.5 py-3.5 border-b border-border/50 align-top whitespace-nowrap text-[12px] text-text-muted">
      <div v-if="hasSessionToken && (showPlanQuota || totalRemainingPercent !== null || autoRemainingPercent !== null || apiRemainingPercent !== null || showGrokBot)" class="flex flex-col gap-1">
        <div v-if="showPlanQuota" class="flex items-center gap-1">
          <span class="w-7 shrink-0 text-text-muted/60" v-tooltip="planHint">{{ $t('platform.cursor.planShort') }}</span>
          <div class="flex-1 h-1.5 bg-muted rounded overflow-hidden">
            <div class="h-full rounded transition-all"
                 :class="getQuotaBarClass(planRemainingPercent)"
                 :style="{ width: planRemainingPercent + '%' }">
            </div>
          </div>
          <span class="text-[11px] font-medium tabular-nums w-7 text-right">{{ planRemainingPercent }}%</span>
        </div>
        <div v-if="totalRemainingPercent !== null" class="flex items-center gap-1">
          <span class="w-7 shrink-0 text-text-muted/60" v-tooltip="$t('platform.cursor.totalAvailableHint')">{{ $t('platform.cursor.totalShort') }}</span>
          <div class="flex-1 h-1.5 bg-muted rounded overflow-hidden">
            <div class="h-full rounded transition-all"
                 :class="getQuotaBarClass(totalRemainingPercent)"
                 :style="{ width: totalRemainingPercent + '%' }">
            </div>
          </div>
          <span class="text-[11px] font-medium tabular-nums w-7 text-right">{{ totalRemainingPercent }}%</span>
        </div>
        <div v-if="autoRemainingPercent !== null" class="flex items-center gap-1">
          <span class="w-7 shrink-0 text-text-muted/60" v-tooltip="$t('platform.cursor.autoAvailableHint')">{{ $t('platform.cursor.autoShort') }}</span>
          <div class="flex-1 h-1.5 bg-muted rounded overflow-hidden">
            <div class="h-full rounded transition-all"
                 :class="getQuotaBarClass(autoRemainingPercent)"
                 :style="{ width: autoRemainingPercent + '%' }">
            </div>
          </div>
          <span class="text-[11px] font-medium tabular-nums w-7 text-right">{{ autoRemainingPercent }}%</span>
        </div>
        <div v-if="apiRemainingPercent !== null" class="flex items-center gap-1">
          <span class="w-7 shrink-0 text-text-muted/60" v-tooltip="$t('platform.cursor.apiAvailableHint')">{{ $t('platform.cursor.apiShort') }}</span>
          <div class="flex-1 h-1.5 bg-muted rounded overflow-hidden">
            <div class="h-full rounded transition-all"
                 :class="getQuotaBarClass(apiRemainingPercent)"
                 :style="{ width: apiRemainingPercent + '%' }">
            </div>
          </div>
          <span class="text-[11px] font-medium tabular-nums w-7 text-right">{{ apiRemainingPercent }}%</span>
        </div>
        <div v-if="showGrokBot" class="flex items-center gap-1">
          <span class="w-7 shrink-0 text-text-muted/60" v-tooltip="grokBotHint">{{ $t('platform.cursor.grokBotShort') }}</span>
          <div class="flex-1 h-1.5 bg-muted rounded overflow-hidden">
            <div class="h-full rounded transition-all"
                 :class="getQuotaBarClass(grokBotRemaining)"
                 :style="{ width: grokBotRemaining + '%' }">
            </div>
          </div>
          <span class="text-[11px] font-medium tabular-nums w-7 text-right">{{ grokBotRemaining }}%</span>
        </div>
      </div>
      <span v-else class="text-text-muted/50">-</span>
    </td>

    <!-- 过期时间（合并显示） -->
    <td class="w-[105px] px-2.5 py-3.5 border-b border-border/50 align-top whitespace-nowrap text-[12px] text-text-muted">
      <div class="flex flex-col gap-0.5">
        <div class="flex items-center gap-1" v-tooltip="$t('platform.cursor.accessTokenExpiry')">
          <span class="text-text-muted/60">A:</span>
          <span>{{ account.token?.expiry_timestamp ? formatDate(account.token.expiry_timestamp) : '-' }}</span>
        </div>
        <div class="flex items-center gap-1" v-tooltip="$t('platform.cursor.sessionExpiry')">
          <span class="text-text-muted/60">S:</span>
          <span>{{ account.token?.session_expiry_timestamp ? formatDate(account.token.session_expiry_timestamp) : '-' }}</span>
        </div>
      </div>
    </td>

    <!-- 配额 -->
    <td class="w-[100px] px-2.5 py-3.5 border-b border-border/50 align-top whitespace-nowrap text-[13px] text-text">
      <button
        v-if="hasSessionToken"
        @click.stop="showUsageModal = true"
        class="inline-flex items-center gap-1 px-2 py-0.5 text-xs text-accent bg-accent/10 border border-accent/30 rounded hover:bg-accent/20 transition-colors cursor-pointer"
      >
        <svg class="w-3 h-3" viewBox="0 0 24 24" fill="currentColor">
          <path d="M3 3h2v18H3V3zm16 8h2v10h-2V11zm-8 4h2v6h-2v-6zm4-8h2v14h-2V7zm-8 6h2v8H7v-8z"/>
        </svg>
        {{ $t('cursorUsage.viewUsage') }}
      </button>
      <span v-else class="text-text-muted/50">-</span>
    </td>

    <!-- 操作 -->
    <td class="w-[80px] px-2.5 py-3.5 border-b border-border/50 align-top whitespace-nowrap text-[13px] text-text text-center">
      <div class="flex items-center justify-center gap-1">
        <button
          v-if="!isCurrent"
          @click.stop="$emit('switch', account.id)"
          class="btn btn--ghost btn--icon-sm"
          :disabled="isSwitching"
          v-tooltip="$t('platform.cursor.switch')"
        >
          <svg v-if="!isSwitching" class="h-4 w-4" viewBox="0 0 24 24" fill="currentColor">
            <path d="M6.99 11L3 15l3.99 4v-3H14v-2H6.99v-3zM21 9l-3.99-4v3H10v2h7.01v3L21 9z" />
          </svg>
          <span v-else class="btn-spinner btn-spinner--xs text-accent" aria-hidden="true"></span>
        </button>

        <FloatingDropdown ref="menuRef" placement="bottom-end" :close-on-select="true" @click.stop>
          <template #trigger>
            <button class="btn btn--ghost btn--icon-sm" v-tooltip="$t('app.moreOptions')">
              <svg class="h-4 w-4" viewBox="0 0 24 24" fill="currentColor">
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
    </td>
  </tr>

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

const props = defineProps({
  account: { type: Object, required: true },
  isCurrent: { type: Boolean, default: false },
  isSwitching: { type: Boolean, default: false },
  isSelected: { type: Boolean, default: false },
  selectionMode: { type: Boolean, default: false },
  showRealEmail: { type: Boolean, default: true },
  allAccounts: { type: Array, default: () => [] }
})

const emit = defineEmits(['switch', 'delete', 'select', 'account-updated', 'account-synced', 'machine-id-generated'])

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

const menuRef = ref(null)
const showTagEditor = ref(false)
const showUsageModal = ref(false)
const isGeneratingMachineId = ref(false)

const maskedEmail = computed(() => {
  const email = props.account.email
  if (!email || !email.includes('@')) return email
  return 'hello@cursor.com'
})

// 标签相关
const tagDisplayName = computed(() => (props.account.tag ?? '').trim())
const hasTag = computed(() => tagDisplayName.value.length > 0)

const tagBadgeStyle = computed(() => {
  if (!hasTag.value) return {}
  return { '--tag-color': props.account.tag_color || DEFAULT_TAG_COLOR }
})

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

// 状态
const statusClass = computed(() => {
  if (props.isCurrent) return 'current'
  if (props.account.disabled) return 'disabled'
  return 'available'
})

const statusBadgeClass = computed(() => {
  switch (statusClass.value) {
    case 'current': return 'badge--accent-tech'
    case 'disabled': return 'badge--danger-tech'
    case 'available': return 'badge--success-tech'
    default: return ''
  }
})

const statusDotClass = computed(() => {
  switch (statusClass.value) {
    case 'current': return 'text-accent'
    case 'disabled': return 'text-danger'
    case 'available': return 'text-success'
    default: return ''
  }
})

const statusLabel = computed(() => {
  if (props.isCurrent) return $t('platform.cursor.status.current')
  if (props.account.disabled) return $t('platform.cursor.status.disabled')
  return $t('platform.cursor.status.available')
})

const formatDate = (timestamp) => {
  if (!timestamp) return '-'
  const date = new Date(timestamp * 1000)
  return date.toLocaleDateString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit'
  })
}

const toggleSelection = () => emit('select', props.account.id)

const handleRowClick = () => {
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
    window.$notify?.success(result.message || $t('platform.cursor.messages.machineIdGenerated'))
    emit('machine-id-generated', props.account.id)
  } catch (err) {
    console.error('Generate machine ID error:', err)
    window.$notify?.error(err?.message || err || $t('platform.cursor.messages.machineIdGenerateFailed'))
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
</script>
