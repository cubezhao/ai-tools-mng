<template>
  <div
    class="bg-surface border border-border rounded-lg p-3 cursor-pointer relative group transition-all duration-150 hover:bg-hover hover:border-border-strong"
    :class="{
      'opacity-60 pointer-events-none': isDeleting,
      'border-accent bg-accent/5': isSelected
    }"
    @click="handleCardClick"
  >
    <!-- 头部：选择框 + 邮箱（单行省略；右上角按钮为绝对定位，不为邮箱预留右侧空白） -->
    <div class="flex items-center gap-2 mb-3 min-w-0">
      <!-- 选择框（悬浮或选择模式时显示） -->
      <div
        class="selection-checkbox shrink-0"
        :class="{ 'visible': selectionMode || isSelected }"
        @click.stop="toggleSelection"
      >
        <div class="checkbox-inner" :class="{ 'checked': isSelected }">
          <svg v-if="isSelected" class="h-3 w-3" viewBox="0 0 24 24" fill="currentColor">
            <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z" />
          </svg>
        </div>
      </div>

      <!-- 邮箱（可点击复制，完整内容见 tooltip） -->
      <div
        class="min-w-0 flex-1 cursor text-[15px] font-semibold text-text truncate"
        v-tooltip="account.email"
        @click.stop="copyEmail"
      >
        <span class="block truncate">{{ showRealEmail ? account.email : maskedEmail }}</span>
      </div>
    </div>

    <!-- 右上角状态徽章 -->
    <div class="absolute right-3 top-3 z-10 flex items-center gap-1.5">
      <!-- 当前账号指示器 -->
      <span v-if="isCurrent" :class="['badge', statusBadgeClass]">
        <span class="status-dot" :class="statusDotClass"></span>
        {{ statusLabel }}
      </span>
    </div>
    <!-- 右上角按钮组（悬停显示，z-index 更高，覆盖状态徽章） -->
    <div
      class="absolute right-3 top-3 z-20 flex items-center gap-1.5 opacity-0 group-hover:opacity-100 transition-opacity"
      :class="{ 'opacity-100': isMenuOpen }"
      @click.stop
    >
      <!-- 切换按钮 -->
      <span v-tooltip="switchTooltip" class="inline-flex">
        <button
          @click="$emit('switch', account.id)"
          :class="[
            'w-7 h-7 rounded border-none bg-surface text-text-secondary flex items-center justify-center shadow-sm transition-colors',
            switchDisabled ? 'cursor-not-allowed opacity-50' : 'cursor-pointer hover:bg-hover hover:text-accent'
          ]"
          :disabled="switchDisabled"
        >
          <svg v-if="!isSwitching" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="currentColor">
            <path d="M6.99 11L3 15l3.99 4v-3H14v-2H6.99v-3zM21 9l-3.99-4v3H10v2h7.01v3L21 9z"/>
          </svg>
          <span v-else class="btn-spinner btn-spinner--sm text-accent"></span>
        </button>
      </span>

      <!-- 刷新配额按钮 (仅 OAuth 账号) -->
      <button
        v-if="!isApiAccount"
        @click="$emit('refresh-quota', account.id)"
        class="w-7 h-7 rounded border-none bg-surface text-text-secondary cursor-pointer flex items-center justify-center shadow-sm hover:bg-hover hover:text-accent transition-colors"
        :disabled="isRefreshing"
        v-tooltip="$t('platform.openai.actions.refreshQuota')"
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
        <template #trigger="{ isOpen }">
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
          <button v-if="!isApiAccount && account.token?.refresh_token" @click="handleMenuClick('copyRefreshToken', close)" class="dropdown-item">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
              <path d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/>
            </svg>
            <span>{{ $t('accountCard.copyRefreshToken') }}</span>
          </button>
          <button v-if="!isApiAccount && account.token?.access_token" @click="handleMenuClick('copyAccessToken', close)" class="dropdown-item">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z"/>
            </svg>
            <span>{{ $t('accountCard.copyAccessToken') }}</span>
          </button>
          <button v-if="hasThirdPartyCredentialTemplates" @click="handleMenuClick('copyThirdPartyCredentials', close)" class="dropdown-item">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
              <path d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/>
            </svg>
            <span>{{ $t('platform.openai.thirdPartyCredentials.openMenu') }}</span>
          </button>
          <button v-if="reverseProxyAction" @click="handleMenuClick('toggleReverseProxy', close)" class="dropdown-item">
            <svg v-if="reverseProxyAction === 'enable'" width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
              <path d="M7 7h10a4 4 0 1 1 0 8H7a4 4 0 1 1 0-8zm0 2a2 2 0 0 0 0 4h10a2 2 0 0 0 0-4H7zm9 1.5a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3z"/>
            </svg>
            <svg v-else width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
              <path d="M7 7h10a4 4 0 1 1 0 8H7a4 4 0 1 1 0-8zm0 2a2 2 0 0 0 0 4h10a2 2 0 0 0 0-4H7zm1 1.5a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3z"/>
            </svg>
            <span>{{ reverseProxyActionLabel }}</span>
          </button>
          <button v-if="isApiAccount && account.api_config?.key" @click="handleMenuClick('copyApiKey', close)" class="dropdown-item">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
              <path d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/>
            </svg>
            <span>{{ $t('accountCard.copyApiKey') }}</span>
          </button>
          <button v-if="isApiAccount && account.api_config?.base_url" @click="handleMenuClick('copyBaseUrl', close)" class="dropdown-item">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
              <path d="M3.9 12c0-1.71 1.39-3.1 3.1-3.1h4V7H7c-2.76 0-5 2.24-5 5s2.24 5 5 5h4v-1.9H7c-1.71 0-3.1-1.39-3.1-3.1zM8 13h8v-2H8v2zm9-6h-4v1.9h4c1.71 0 3.1 1.39 3.1 3.1s-1.39 3.1-3.1 3.1h-4V17h4c2.76 0 5-2.24 5-5s-2.24-5-5-5z"/>
            </svg>
            <span>{{ $t('accountCard.copyBaseUrl') }}</span>
          </button>
          <button @click="handleMenuClick('delete', close)" class="dropdown-item text-danger hover:bg-danger/10">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
              <path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/>
            </svg>
            <span>{{ $t('platform.openai.actions.delete') }}</span>
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
      <div v-if="authInfo?.chatgpt_plan_type || isApiAccount" class="flex items-center gap-1 min-h-6">
        <div class="flex items-center gap-1.5 w-[90px] shrink-0 text-text-muted text-xs">
          <svg class="w-3.5 h-3.5 shrink-0 opacity-70" viewBox="0 0 24 24" fill="currentColor">
            <path d="M20 4H4c-1.11 0-1.99.89-1.99 2L2 18c0 1.11.89 2 2 2h16c1.11 0 2-.89 2-2V6c0-1.11-.89-2-2-2zm0 14H4v-6h16v6zm0-10H4V6h16v2z"/>
          </svg>
          <span>{{ $t('platform.openai.planType') }}</span>
        </div>
        <div class="flex-1">
          <span :class="getPlanBadgeClass(isApiAccount ? 'api' : authInfo.chatgpt_plan_type)">
            <svg v-if="isApiAccount" class="w-3 h-3" viewBox="0 0 24 24" fill="currentColor">
              <path d="M19 3H5L2 9l10 12L22 9l-3-6zM9.62 8l1.5-3h1.76l1.5 3H9.62zM11 10v6.68L5.44 10H11zm2 0h5.56L13 16.68V10zm6.26-2h-2.65l-1.5-3h2.65l1.5 3zM6.24 5h2.65l-1.5 3H4.74l1.5-3z"/>
            </svg>
            <svg v-else-if="authInfo?.chatgpt_plan_type?.toLowerCase() === 'pro'" class="w-3 h-3" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 1L9 9H2l6 5-2.5 8L12 17l6.5 5L16 14l6-5h-7z"/>
            </svg>
            <svg v-else-if="authInfo?.chatgpt_plan_type?.toLowerCase() === 'team'" class="w-3 h-3" viewBox="0 0 24 24" fill="currentColor">
              <path d="M5 16L3 5l5.5 5L12 4l3.5 6L21 5l-2 11H5zm14 3c0 .6-.4 1-1 1H6c-.6 0-1-.4-1-1v-1h14v1z"/>
            </svg>
            <svg v-else-if="authInfo?.chatgpt_plan_type?.toLowerCase() === 'plus'" class="w-3 h-3" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 2l2.4 7.2L22 12l-7.6 2.8L12 22l-2.4-7.2L2 12l7.6-2.8z"/>
            </svg>
            {{ isApiAccount ? 'API' : authInfo.chatgpt_plan_type }}
          </span>
        </div>
      </div>

      <!-- 订阅到期时间 -->
      <div v-if="authInfo?.chatgpt_subscription_active_until" class="flex items-center gap-1 min-h-6"
           v-tooltip="atExpiryTooltip">
        <div class="flex items-center gap-1.5 w-[90px] shrink-0 text-text-muted text-xs">
          <svg class="w-3.5 h-3.5 shrink-0 opacity-70" viewBox="0 0 24 24" fill="currentColor">
            <path d="M19 3h-1V1h-2v2H8V1H6v2H5c-1.11 0-1.99.9-1.99 2L3 19c0 1.1.89 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm0 16H5V8h14v11zM9 10H7v2h2v-2zm4 0h-2v2h2v-2zm4 0h-2v2h2v-2z"/>
          </svg>
          <span>{{ $t('platform.openai.subscriptionExpires') }}</span>
        </div>
        <div class="flex-1 text-[13px] truncate">
          <span :class="['inline-flex items-center gap-1', subscriptionExpiryClass]">
            {{ formatISODate(authInfo.chatgpt_subscription_active_until) }}
            <span v-if="subscriptionDaysLeftText" class="text-[11px] opacity-80">({{ subscriptionDaysLeftText }})</span>
          </span>
        </div>
      </div>

      <!-- 禁用状态提示 -->
      <div v-if="!isApiAccount && account.quota?.is_forbidden" class="flex items-center gap-2 rounded bg-danger/10 px-2 py-1.5 text-xs text-danger">
        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="currentColor">
          <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.42 0-8-3.58-8-8 0-1.85.63-3.55 1.69-4.9L16.9 18.31C15.55 19.37 13.85 20 12 20zm6.31-3.1L7.1 5.69C8.45 4.63 10.15 4 12 4c4.42 0 8 3.58 8 8 0 1.85-.63 3.55-1.69 4.9z"/>
        </svg>
        <span>{{ $t('platform.antigravity.quotaForbidden') }}</span>
      </div>

      <!-- RT 失效警告 -->
      <div v-if="!isApiAccount && account.rt_invalid" class="flex items-center gap-2 rounded bg-warning/10 px-2 py-1.5 text-xs text-warning">
        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="currentColor">
          <path d="M1 21h22L12 2 1 21zm12-3h-2v-2h2v2zm0-4h-2v-4h2v4z"/>
        </svg>
        <span>{{ account.rt_invalid_reason === 'refresh_token_reused' ? $t('platform.openai.rtInvalidReused') : $t('platform.openai.rtInvalid') }}</span>
      </div>

      <!-- 5h 配额 (仅 OAuth 账号) -->
      <div v-if="!isApiAccount && !account.quota?.is_forbidden && !account.rt_invalid && account.quota?.codex_5h_used_percent !== null && account.quota?.codex_5h_used_percent !== undefined"
           class="flex items-center gap-1 min-h-6">
        <div class="flex flex-col gap-0.5 w-[90px] shrink-0 text-text-muted text-xs">
          <div class="flex items-center gap-1.5">
            <svg class="w-3.5 h-3.5 shrink-0 opacity-70" viewBox="0 0 24 24" fill="currentColor">
              <path d="M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm0 16H5V5h14v14zM7 12h2v5H7zm4-3h2v8h-2zm4-3h2v11h-2z"/>
            </svg>
            <span>{{ $t('platform.openai.quota5h') }}</span>
          </div>
          <span v-if="account.quota.codex_5h_reset_after_seconds" class="text-[10px] opacity-70 pl-5">
            {{ formatResetTimeShort(account.quota.codex_5h_reset_after_seconds) }}
          </span>
        </div>
        <div class="flex-1 flex items-center gap-1">
          <div class="flex-1 h-1.5 bg-muted rounded overflow-hidden">
            <div class="h-full rounded transition-all"
                 :class="getQuotaBarClass(100 - account.quota.codex_5h_used_percent)"
                 :style="{ width: (100 - account.quota.codex_5h_used_percent) + '%' }">
            </div>
          </div>
          <span class="text-[11px] font-medium tabular nums text-text-muted w-8 text-right">
            {{ 100 - account.quota.codex_5h_used_percent }}%
          </span>
        </div>
      </div>

      <!-- 7d 配额 (仅 OAuth 账号) -->
      <div v-if="!isApiAccount && !account.quota?.is_forbidden && !account.rt_invalid && account.quota?.codex_7d_used_percent !== null && account.quota?.codex_7d_used_percent !== undefined"
           class="flex items-center gap-1 min-h-6">
        <div class="flex flex-col gap-0.5 w-[90px] shrink-0 text-text-muted text-xs">
          <div class="flex items-center gap-1.5">
            <svg class="w-3.5 h-3.5 shrink-0 opacity-70" viewBox="0 0 24 24" fill="currentColor">
              <path d="M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm0 16H5V5h14v14zM7 12h2v5H7zm4-3h2v8h-2zm4-3h2v11h-2z"/>
            </svg>
            <span>{{ $t('platform.openai.quota7d') }}</span>
          </div>
          <span v-if="account.quota.codex_7d_reset_after_seconds" class="text-[10px] opacity-70 pl-5">
            {{ formatResetTimeLong(account.quota.codex_7d_reset_after_seconds) }}
          </span>
        </div>
        <div class="flex-1 flex items-center gap-1">
          <div class="flex-1 h-1.5 bg-muted rounded overflow-hidden">
            <div class="h-full rounded transition-all"
                 :class="getQuotaBarClass(100 - account.quota.codex_7d_used_percent)"
                 :style="{ width: (100 - account.quota.codex_7d_used_percent) + '%' }">
            </div>
          </div>
          <span class="text-[11px] font-medium tabular nums text-text-muted w-8 text-right">
            {{ 100 - account.quota.codex_7d_used_percent }}%
          </span>
        </div>
      </div>

      <!-- 重置额度 (仅 OAuth 账号，账号存在重置券时展示) -->
      <div v-if="showResetCredits" class="flex items-center gap-1 min-h-6">
        <div class="flex items-center gap-1.5 w-[90px] shrink-0 text-text-muted text-xs">
          <svg class="w-3.5 h-3.5 shrink-0 opacity-70" viewBox="0 0 24 24" fill="currentColor">
            <path d="M12 6v3l4-4-4-4v3c-4.42 0-8 3.58-8 8 0 1.57.46 3.03 1.24 4.26L6.7 14.8c-.45-.83-.7-1.79-.7-2.8 0-3.31 2.69-6 6-6zm6.76 1.74L17.3 9.2c.44.84.7 1.79.7 2.8 0 3.31-2.69 6-6 6v-3l-4 4 4 4v-3c4.42 0 8-3.58 8-8 0-1.57-.46-3.03-1.24-4.26z"/>
          </svg>
          <span>{{ $t('platform.openai.resetCredits') }}</span>
        </div>
        <div class="flex-1 flex items-center gap-2 text-[13px]">
          <span class="tabular-nums text-text">
            {{ resetCreditsAvailable }}<span class="text-text-muted"> / {{ resetCreditsTotal }}</span>
          </span>
          <button
            v-if="resetCreditsAvailable > 0"
            @click.stop="$emit('reset-credits', account.id)"
            :disabled="isResetting"
            class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded border border-border text-xs text-accent cursor-pointer hover:bg-hover disabled:cursor-not-allowed disabled:opacity-50 transition-colors"
            v-tooltip="$t('platform.openai.resetCreditsTooltip')"
          >
            <span v-if="isResetting" class="btn-spinner btn-spinner--sm text-accent"></span>
            <span>{{ $t('platform.openai.useResetCredit') }}</span>
          </button>
        </div>
      </div>

      <!-- API 配置信息 (仅 API 账号) -->
      <template v-if="isApiAccount && account.api_config">
        <!-- Model -->
        <div v-if="account.api_config.model" class="flex items-center gap-1 min-h-6">
          <div class="flex items-center gap-1.5 w-[90px] shrink-0 text-text-muted text-xs">
            <svg class="w-3.5 h-3.5 shrink-0 opacity-70" viewBox="0 0 24 24" fill="currentColor">
              <path d="M21 3H3c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h18c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm0 16H3V5h18v14zM9 10h6v2H9z"/>
            </svg>
            <span>Model</span>
          </div>
          <div class="flex-1 text-[13px] truncate text-text">
            {{ account.api_config.model }}
          </div>
        </div>

        <!-- Reasoning Effort -->
        <div v-if="account.api_config.model_reasoning_effort" class="flex items-center gap-1 min-h-6">
          <div class="flex items-center gap-1.5 w-[90px] shrink-0 text-text-muted text-xs">
            <svg class="w-3.5 h-3.5 shrink-0 opacity-70" viewBox="0 0 24 24" fill="currentColor">
              <path d="M9 21c0 .5.4 1 1 1h4c.6 0 1-.5 1-1v-1H9v1zm3-19C8.1 2 5 5.1 5 9c0 2.4 1.2 4.5 3 5.7V17c0 .5.4 1 1 1h6c.6 0 1-.5 1-1v-2.3c1.8-1.3 3-3.4 3-5.7 0-3.9-3.1-7-7-7z"/>
            </svg>
            <span>{{ $t('platform.openai.reasoningEffort') }}</span>
          </div>
          <div class="flex-1 text-[13px] truncate text-text">
            {{ account.api_config.model_reasoning_effort }}
          </div>
        </div>

        <!-- Base URL -->
        <div v-if="account.api_config.base_url" class="flex items-center gap-1 min-h-6">
          <div class="flex items-center gap-1.5 w-[90px] shrink-0 text-text-muted text-xs">
            <svg class="w-3.5 h-3.5 shrink-0 opacity-70" viewBox="0 0 24 24" fill="currentColor">
              <path d="M3.9 12c0-1.71 1.39-3.1 3.1-3.1h4V7H7c-2.76 0-5 2.24-5 5s2.24 5 5 5h4v-1.9H7c-1.71 0-3.1-1.39-3.1-3.1zM8 13h8v-2H8v2zm9-6h-4v1.9h4c1.71 0 3.1 1.39 3.1 3.1s-1.39 3.1-3.1 3.1h-4V17h4c2.76 0 5-2.24 5-5s-2.24-5-5-5z"/>
            </svg>
            <span>URL</span>
          </div>
          <div class="flex-1 text-[13px] truncate text-text">
            {{ account.api_config.base_url }}
          </div>
        </div>
      </template>

      <!-- 标签 -->
      <div class="flex items-center gap-1 min-h-6">
        <div class="flex items-center gap-1.5 w-[90px] shrink-0 text-text-muted text-xs">
          <svg class="w-3.5 h-3.5 shrink-0 opacity-70" viewBox="0 0 24 24" fill="currentColor">
            <path d="M17.63 5.84C17.27 5.33 16.67 5 16 5L5 5.01C3.9 5.01 3 5.9 3 7v10c0 1.1.9 1.99 2 1.99L16 19c.67 0 1.27-.33 1.63-.84L22 12l-4.37-6.16z"/>
          </svg>
          <span>{{ $t('subscriptions.fields.tag') }}</span>
        </div>
        <div class="flex-1 text-[13px]">
          <!-- 添加标签按钮（无标签时显示） -->
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
          <!-- 标签（有标签时显示） -->
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
</template>

<script setup>
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import FloatingDropdown from '../common/FloatingDropdown.vue'
import TagEditorModal from '../token/TagEditorModal.vue'
import { getReverseProxyAction, toggleReverseProxyForAccount } from '@/utils/openaiReverseProxy'
import { getAvailableOpenAIThirdPartyCredentialTemplates } from '@/utils/openaiThirdPartyCredentials'

const { t: $t } = useI18n()

const props = defineProps({
  account: {
    type: Object,
    required: true
  },
  isCurrent: {
    type: Boolean,
    default: false
  },
  isSwitching: {
    type: Boolean,
    default: false
  },
  isRefreshing: {
    type: Boolean,
    default: false
  },
  isResetting: {
    type: Boolean,
    default: false
  },
  isDeleting: {
    type: Boolean,
    default: false
  },
  isSelected: {
    type: Boolean,
    default: false
  },
  selectionMode: {
    type: Boolean,
    default: false
  },
  showRealEmail: {
    type: Boolean,
    default: true
  },
  allAccounts: {
    type: Array,
    default: () => []
  }
})

const emit = defineEmits(['refresh-quota', 'reset-credits', 'delete', 'select', 'switch', 'account-updated', 'edit', 'copy-third-party-credentials'])

const menuRef = ref(null)
const showTagEditor = ref(false)
const isMenuOpen = ref(false)
const DEFAULT_TAG_COLOR = '#f97316'

const isAccessTokenOnlyAccount = computed(() => {
  const token = props.account?.token
  return props.account?.account_type !== 'api' && Boolean(token?.access_token) && !token?.refresh_token
})

const switchDisabled = computed(() => props.isSwitching || isAccessTokenOnlyAccount.value)

const switchTooltip = computed(() => (
  isAccessTokenOnlyAccount.value
    ? $t('platform.openai.actions.switchDisabledAccessTokenOnly')
    : $t('platform.openai.switch')
))

// 状态相关
const statusClass = computed(() => {
  return props.isCurrent ? 'current' : ''
})

const statusBadgeClass = computed(() => {
  switch (statusClass.value) {
    case 'current':
      return 'badge--success-tech'
    default:
      return ''
  }
})

const statusDotClass = computed(() => {
  switch (statusClass.value) {
    case 'current':
      return 'text-success'
    default:
      return ''
  }
})

const statusLabel = computed(() => {
  if (props.isCurrent) return $t('platform.antigravity.status.current')
  return ''
})

// 隐藏邮箱
const maskedEmail = computed(() => {
  const email = props.account.email
  if (!email || !email.includes('@')) return email
  return 'hello@openai.com'
})

// 账号类型判断
const isApiAccount = computed(() => {
  return props.account.account_type === 'api'
})

// 解析 openai_auth_json
const authInfo = computed(() => {
  if (!props.account.openai_auth_json) return null
  try {
    return JSON.parse(props.account.openai_auth_json)
  } catch {
    return null
  }
})

// 重置额度（限流重置券）
const resetCreditsAvailable = computed(() => props.account.quota?.codex_reset_credits_available ?? 0)
const resetCreditsTotal = computed(() => props.account.quota?.codex_reset_credits_total ?? 0)
const showResetCredits = computed(() =>
  !isApiAccount.value &&
  !props.account.quota?.is_forbidden &&
  !props.account.rt_invalid &&
  resetCreditsTotal.value > 0
)

// 配额相关
const getQuotaBarClass = (percent) => {
  if (percent === null || percent === undefined) return 'bg-text-muted'
  if (percent < 10) return 'bg-danger'
  if (percent < 30) return 'bg-warning'
  return 'bg-success'
}

// 订阅计划徽章样式
const getPlanBadgeClass = (planType) => {
  const base = 'badge badge--sm uppercase'
  switch (planType?.toLowerCase()) {
    case 'pro':
    case 'api':
      return `${base} bg-gradient-to-r from-rose-400 to-pink-500 text-white border-pink-500/50 shadow-sm shadow-pink-500/30`
    case 'team':
      return `${base} bg-gradient-to-r from-amber-400 to-amber-500 text-amber-900 border-amber-500/50`
    case 'plus':
      return `${base} bg-gradient-to-r from-emerald-400 to-teal-500 text-white border-teal-500/50`
    default:
      return base
  }
}

const formatResetTimeShort = (seconds) => {
  if (!seconds) return ''
  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  if (hours > 0) return `${hours}h${minutes}m`
  return `${minutes}m`
}

const formatResetTimeLong = (seconds) => {
  if (!seconds) return ''
  const days = Math.floor(seconds / 86400)
  const hours = Math.floor((seconds % 86400) / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  const parts = []
  if (days > 0) parts.push(`${days}d`)
  if (hours > 0) parts.push(`${hours}h`)
  if (minutes > 0 || parts.length === 0) parts.push(`${minutes}m`)
  return parts.join('')
}

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

const formatISODate = (isoString) => {
  if (!isoString) return '-'
  const date = new Date(isoString)
  return date.toLocaleDateString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit'
  })
}

// 订阅到期剩余天数
const atExpiryTooltip = computed(() => {
  if (!props.account.token?.expires_at) return ''
  return `${$t('platform.openai.tokenExpiresAt')}\uFF1A${formatDate(props.account.token.expires_at)}`
})

const subscriptionDaysLeft = computed(() => {
  if (!authInfo.value?.chatgpt_subscription_active_until) return null
  const now = new Date()
  const expiry = new Date(authInfo.value.chatgpt_subscription_active_until)
  return Math.ceil((expiry - now) / (1000 * 60 * 60 * 24))
})

const subscriptionDaysLeftText = computed(() => {
  if (subscriptionDaysLeft.value === null) return ''
  if (subscriptionDaysLeft.value < 0) return $t('subscriptions.expired')
  if (subscriptionDaysLeft.value === 0) return $t('subscriptions.expirestoday')
  return $t('subscriptions.daysLeft', { days: subscriptionDaysLeft.value })
})

const subscriptionExpiryClass = computed(() => {
  if (subscriptionDaysLeft.value === null) return 'text-text-muted'
  if (subscriptionDaysLeft.value < 0) return 'text-danger'
  if (subscriptionDaysLeft.value < 10) return 'text-danger'
  if (subscriptionDaysLeft.value < 20) return 'text-warning'
  return 'text-success'
})

// 选择和点击
const toggleSelection = () => {
  emit('select', props.account.id)
}

const handleCardClick = () => {
  if (props.selectionMode) {
    toggleSelection()
  } else if (isApiAccount.value) {
    // 只有 API 账号可以点击编辑
    emit('edit', props.account)
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

const reverseProxyAction = computed(() => getReverseProxyAction(props.account))

const reverseProxyActionLabel = computed(() => {
  if (reverseProxyAction.value === 'disable') {
    return $t('platform.openai.disableReverseProxy')
  }
  if (reverseProxyAction.value === 'enable') {
    return $t('platform.openai.enableReverseProxy')
  }
  return ''
})

const hasThirdPartyCredentialTemplates = computed(() => {
  return getAvailableOpenAIThirdPartyCredentialTemplates(props.account).length > 0
})

const handleReverseProxyToggle = () => {
  const action = reverseProxyAction.value
  const updated = toggleReverseProxyForAccount(props.account)
  if (!updated) return

  emit('account-updated', props.account)
  window.$notify?.success(
    action === 'disable'
      ? $t('platform.openai.reverseProxyDisabledSuccess')
      : $t('platform.openai.reverseProxyEnabledSuccess')
  )
}

// 复制操作
const copyEmail = async () => {
  try {
    await navigator.clipboard.writeText(props.account.email)
    window.$notify?.success($t('messages.emailNoteCopied'))
  } catch (err) {
    window.$notify?.error($t('messages.copyEmailNoteFailed'))
  }
}

const copyRefreshToken = async () => {
  try {
    const refreshToken = props.account.token?.refresh_token
    if (!refreshToken) {
      window.$notify?.error($t('messages.noRefreshToken'))
      return
    }
    await navigator.clipboard.writeText(refreshToken)
    window.$notify?.success($t('messages.refreshTokenCopied'))
  } catch (err) {
    window.$notify?.error($t('messages.copyFailed'))
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

const handleMenuClick = async (type, close) => {
  close?.()
  switch (type) {
    case 'copyRefreshToken':
      await copyRefreshToken()
      break
    case 'copyAccessToken':
      await copyAccessToken()
      break
    case 'copyApiKey':
      await copyApiKey()
      break
    case 'copyBaseUrl':
      await copyBaseUrl()
      break
    case 'toggleReverseProxy':
      handleReverseProxyToggle()
      break
    case 'copyThirdPartyCredentials':
      emit('copy-third-party-credentials', props.account)
      break
    case 'delete':
      emit('delete', props.account.id)
      break
  }
}

const copyApiKey = async () => {
  try {
    const apiKey = props.account.api_config?.key
    if (!apiKey) {
      window.$notify?.error($t('accountCard.noApiKey'))
      return
    }
    await navigator.clipboard.writeText(apiKey)
    window.$notify?.success($t('accountCard.apiKeyCopied'))
  } catch (err) {
    window.$notify?.error($t('messages.copyFailed'))
  }
}

const copyBaseUrl = async () => {
  try {
    const baseUrl = props.account.api_config?.base_url
    if (!baseUrl) {
      window.$notify?.error($t('accountCard.noBaseUrl'))
      return
    }
    await navigator.clipboard.writeText(baseUrl)
    window.$notify?.success($t('accountCard.baseUrlCopied'))
  } catch (err) {
    window.$notify?.error($t('messages.copyFailed'))
  }
}
</script>
