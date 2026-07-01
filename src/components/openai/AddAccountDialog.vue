<template>
  <BaseModal
    :visible="true"
    :title="$t('platform.openai.addAccountDialog.title')"
    :show-close="true"
    :close-on-overlay="false"
    :close-on-esc="false"
    modal-class="max-w-[500px]"
    @close="handleClose"
  >
    <!-- 添加方式选择 -->
    <div class="mb-6 flex gap-2 rounded-lg bg-muted p-1">
      <button
        :class="[
          'flex flex-1 items-center justify-center gap-1.5 rounded-md px-4 py-2.5 text-sm font-medium transition-all',
          addMethod === 'oauth'
            ? 'bg-surface text-accent shadow-sm'
            : 'text-text-secondary hover:bg-hover hover:text-text'
        ]"
        @click="switchToOAuth"
      >
        <svg class="h-4 w-4" viewBox="0 0 24 24" fill="currentColor">
          <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z"/>
        </svg>
        {{ $t('platform.openai.addAccountDialog.oauthMethod') }}
      </button>
      <button
        :class="[
          'flex flex-1 items-center justify-center gap-1.5 rounded-md px-4 py-2.5 text-sm font-medium transition-all',
          addMethod === 'api'
            ? 'bg-surface text-accent shadow-sm'
            : 'text-text-secondary hover:bg-hover hover:text-text'
        ]"
        @click="addMethod = 'api'"
      >
        <svg class="h-4 w-4" viewBox="0 0 24 24" fill="currentColor">
          <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z"/>
        </svg>
        API
      </button>
      <button
        :class="[
          'flex flex-1 items-center justify-center gap-1.5 rounded-md px-4 py-2.5 text-sm font-medium transition-all',
          addMethod === 'manual'
            ? 'bg-surface text-accent shadow-sm'
            : 'text-text-secondary hover:bg-hover hover:text-text'
        ]"
        @click="addMethod = 'manual'"
      >
        <svg class="h-4 w-4" viewBox="0 0 24 24" fill="currentColor">
          <path d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"/>
        </svg>
        {{ $t('platform.openai.addAccountDialog.manualMethod') }}
      </button>
      <button
        :class="[
          'flex flex-1 items-center justify-center gap-1.5 rounded-md px-4 py-2.5 text-sm font-medium transition-all',
          addMethod === 'import'
            ? 'bg-surface text-accent shadow-sm'
            : 'text-text-secondary hover:bg-hover hover:text-text'
        ]"
        @click="addMethod = 'import'"
      >
        <svg class="h-4 w-4" viewBox="0 0 24 24" fill="currentColor">
          <path d="M19.35 10.04C18.67 6.59 15.64 4 12 4 9.11 4 6.6 5.64 5.35 8.04 2.34 8.36 0 10.91 0 14c0 3.31 2.69 6 6 6h13c2.76 0 5-2.24 5-5 0-2.64-2.05-4.78-4.65-4.96zM14 13v4h-4v-4H7l5-5 5 5h-3z"/>
        </svg>
        {{ $t('platform.openai.addAccountDialog.importMethod') }}
      </button>
    </div>

    <!-- OAuth 授权方式 -->
    <div v-if="addMethod === 'oauth'" class="animate-fade-in">
      <!-- OpenAI OAuth Button -->
      <button
        @click="handleOAuthLogin"
        class="flex w-full items-center justify-center gap-2.5 rounded-lg border border-border bg-white px-5 py-3.5 text-[15px] font-medium text-neutral-800 transition-all hover:border-border-strong hover:bg-neutral-50 hover:shadow-sm disabled:cursor-not-allowed disabled:opacity-60"
        :disabled="isLoading"
      >
        <span class="relative inline-flex h-5 w-5 items-center justify-center">
          <svg :style="{ visibility: isLoading ? 'hidden' : 'visible' }" class="h-5 w-5" viewBox="0 0 24 24" fill="currentColor">
            <path d="M22.2819 9.8211a5.9847 5.9847 0 0 0-.5157-4.9108 6.0462 6.0462 0 0 0-6.5098-2.9A6.0651 6.0651 0 0 0 4.9807 4.1818a5.9847 5.9847 0 0 0-3.9977 2.9 6.0462 6.0462 0 0 0 .7427 7.0966 5.98 5.98 0 0 0 .511 4.9107 6.051 6.051 0 0 0 6.5146 2.9001A5.9847 5.9847 0 0 0 13.2599 24a6.0557 6.0557 0 0 0 5.7718-4.2058 5.9894 5.9894 0 0 0 3.9977-2.9001 6.0557 6.0557 0 0 0-.7475-7.0729zm-9.022 12.6081a4.4755 4.4755 0 0 1-2.8764-1.0408l.1419-.0804 4.7783-2.7582a.7948.7948 0 0 0 .3927-.6813v-6.7369l2.02 1.1686a.071.071 0 0 1 .038.052v5.5826a4.504 4.504 0 0 1-4.4945 4.4944zm-9.6607-4.1254a4.4708 4.4708 0 0 1-.5346-3.0137l.142.0852 4.783 2.7582a.7712.7712 0 0 0 .7806 0l5.8428-3.3685v2.3324a.0804.0804 0 0 1-.0332.0615L9.74 19.9502a4.4992 4.4992 0 0 1-6.1408-1.6464zM2.3408 7.8956a4.485 4.485 0 0 1 2.3655-1.9728V11.6a.7664.7664 0 0 0 .3879.6765l5.8144 3.3543-2.0201 1.1685a.0757.0757 0 0 1-.071 0l-4.8303-2.7865A4.504 4.504 0 0 1 2.3408 7.872zm16.5963 3.8558L13.1038 8.364 15.1192 7.2a.0757.0757 0 0 1 .071 0l4.8303 2.7913a4.4944 4.4944 0 0 1-.6765 8.1042v-5.6772a.79.79 0 0 0-.407-.667zm2.0107-3.0231l-.142-.0852-4.7735-2.7818a.7759.7759 0 0 0-.7854 0L9.409 9.2297V6.8974a.0662.0662 0 0 1 .0284-.0615l4.8303-2.7866a4.4992 4.4992 0 0 1 6.6802 4.66zM8.3065 12.863l-2.02-1.1638a.0804.0804 0 0 1-.038-.0567V6.0742a4.4992 4.4992 0 0 1 7.3757-3.4537l-.142.0805L8.704 5.459a.7948.7948 0 0 0-.3927.6813zm1.0976-2.3654l2.602-1.4998 2.6069 1.4998v2.9994l-2.5974 1.4997-2.6067-1.4997z"/>
          </svg>
          <span v-if="isLoading" class="btn-spinner absolute inset-0 m-auto" aria-hidden="true"></span>
        </span>
        {{ isLoading ? $t('platform.openai.addAccountDialog.adding') : $t('platform.openai.addAccountDialog.openaiLogin') }}
      </button>

      <!-- Outlook 验证码助手（默认折叠，按需展开） -->
      <div class="mt-5 rounded-lg border border-border bg-muted/40">
        <button
          type="button"
          class="flex w-full items-center justify-between px-4 py-3 text-left text-[13px] font-semibold text-text transition-colors hover:bg-muted"
          @click="mailHelperOpen = !mailHelperOpen"
        >
          <span class="inline-flex items-center gap-2">
            <svg class="h-4 w-4 text-accent" viewBox="0 0 24 24" fill="currentColor">
              <path d="M20 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V6c0-1.1-.9-2-2-2zm0 4l-8 5-8-5V6l8 5 8-5v2z"/>
            </svg>
            {{ $t('platform.openai.addAccountDialog.mailHelper.title') }}
          </span>
          <svg
            class="h-4 w-4 text-text-muted transition-transform"
            :class="{ 'rotate-180': mailHelperOpen }"
            viewBox="0 0 24 24" fill="currentColor"
          >
            <path d="M7 10l5 5 5-5z"/>
          </svg>
        </button>

        <div v-if="mailHelperOpen" class="flex flex-col gap-3 border-t border-border p-4 max-h-[min(48vh,420px)]">
          <!-- 无 Outlook 账号 -->
          <div
            v-if="!isLoadingOutlookAccounts && outlookAccounts.length === 0"
            class="shrink-0 rounded-md border border-warning/30 bg-warning/10 p-3 text-[13px] text-text-secondary"
          >
            {{ $t('platform.openai.addAccountDialog.mailHelper.noAccounts') }}
          </div>

          <template v-else>
            <!-- 邮箱选择 + 获取验证码 -->
            <div class="flex shrink-0 items-center gap-2">
              <FloatingDropdown
                placement="bottom-start"
                :close-on-select="false"
                :disabled="isLoadingOutlookAccounts || isLoadingMail"
                class="!flex flex-1 min-w-0"
                @open="handleMailDropdownOpen"
                @close="mailSearchQuery = ''"
              >
                <template #trigger="{ isOpen }">
                  <button
                    type="button"
                    class="input flex w-full min-w-0 items-center justify-between text-left"
                    :disabled="isLoadingOutlookAccounts || isLoadingMail"
                  >
                    <span class="truncate">{{ selectedOutlookEmail || $t('platform.openai.addAccountDialog.mailHelper.selectAccount') }}</span>
                    <svg class="ml-2 h-4 w-4 shrink-0 transition-transform" :class="{ 'rotate-180': isOpen }" viewBox="0 0 24 24" fill="currentColor">
                      <path d="M7 10l5 5 5-5z"/>
                    </svg>
                  </button>
                </template>
                <template #default="{ close }">
                  <div class="flex max-h-[min(70vh,420px)] w-[280px] flex-col">
                    <div class="sticky top-0 z-10 border-b border-border bg-surface p-2">
                      <input
                        ref="mailSearchInputRef"
                        v-model="mailSearchQuery"
                        type="text"
                        :placeholder="$t('platform.openai.addAccountDialog.mailHelper.searchPlaceholder')"
                        class="input w-full text-sm"
                        @keydown.enter.prevent="onSearchEnter(close)"
                        @keydown.escape.prevent="close"
                      />
                      <div v-if="mailSearchQuery" class="mt-1 px-1 text-[11px] text-text-muted">
                        {{ $t('platform.openai.addAccountDialog.mailHelper.matchCount', { count: filteredOutlookAccounts.length, total: outlookAccounts.length }) }}
                      </div>
                    </div>
                    <div class="flex-1 overflow-y-auto py-1">
                      <button
                        v-for="acc in filteredOutlookAccounts"
                        :key="acc.email"
                        type="button"
                        class="dropdown-item"
                        :class="{ 'dropdown-item--active': acc.email === selectedOutlookEmail }"
                        @click="selectOutlookEmail(acc.email, close)"
                      >
                        <svg
                          class="h-3.5 w-3.5 shrink-0"
                          :class="acc.email === selectedOutlookEmail ? 'text-accent' : 'text-transparent'"
                          viewBox="0 0 24 24" fill="currentColor"
                        >
                          <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"/>
                        </svg>
                        <span class="truncate">{{ acc.email }}</span>
                      </button>
                      <div v-if="filteredOutlookAccounts.length === 0" class="px-3 py-3 text-center text-[12px] text-text-muted">
                        {{ $t('platform.openai.addAccountDialog.mailHelper.noMatch') }}
                      </div>
                    </div>
                  </div>
                </template>
              </FloatingDropdown>
              <button
                type="button"
                class="btn btn--secondary btn--icon shrink-0"
                @click="copySelectedOutlookEmail"
                :disabled="!selectedOutlookEmail || isLoadingMail"
                v-tooltip="$t('platform.openai.addAccountDialog.mailHelper.copyEmail')"
              >
                <svg class="h-4 w-4" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/>
                </svg>
              </button>
              <button
                type="button"
                class="btn btn--primary btn--sm whitespace-nowrap"
                @click="fetchVerificationCode"
                :disabled="!selectedOutlookEmail || isLoadingMail"
              >
                <span v-if="isLoadingMail" class="btn-spinner btn-spinner--xs" aria-hidden="true"></span>
                {{ isLoadingMail ? $t('platform.openai.addAccountDialog.mailHelper.fetching') : $t('platform.openai.addAccountDialog.mailHelper.getCode') }}
              </button>
            </div>

            <p v-if="hasFetchedMail && !isLoadingMail && !extractedCode" class="shrink-0 text-[12px] text-text-muted">
              {{ $t('platform.openai.addAccountDialog.mailHelper.noCodeFound') }}
            </p>
          </template>
        </div>
      </div>

      <!-- 折叠的手动 OAuth Section（默认收起，给自动登录失败时使用） -->
      <div class="mt-5 rounded-lg border border-border bg-muted/40">
        <button
          type="button"
          class="flex w-full items-center justify-between px-4 py-3 text-left text-[13px] font-semibold text-text transition-colors hover:bg-muted"
          @click="manualOAuthOpen = !manualOAuthOpen"
        >
          <span class="inline-flex items-center gap-2">
            <svg class="h-4 w-4 text-text-muted" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 6V4l-4 4 4 4V10c3.31 0 6 2.69 6 6 0 1.01-.25 1.97-.7 2.8l1.46 1.46C19.54 19.03 20 17.57 20 16c0-4.42-3.58-8-8-8zm0 14c-3.31 0-6-2.69-6-6 0-1.01.25-1.97.7-2.8L5.24 9.74C4.46 10.97 4 12.43 4 14c0 4.42 3.58 8 8 8v2l4-4-4-4v2z"/>
            </svg>
            {{ $t('platform.openai.addAccountDialog.oauthManualTitle') }}
          </span>
          <svg
            class="h-4 w-4 text-text-muted transition-transform"
            :class="{ 'rotate-180': manualOAuthOpen }"
            viewBox="0 0 24 24" fill="currentColor"
          >
            <path d="M7 10l5 5 5-5z"/>
          </svg>
        </button>

        <div v-if="manualOAuthOpen" class="border-t border-border p-4">
          <div class="mb-3 flex gap-2.5">
            <button class="btn btn--primary" @click="generateAuthUrl" :disabled="isLoading || isManualLoading">
              {{ $t('platform.openai.addAccountDialog.generateAuthLink') }}
            </button>
          </div>

          <div v-if="oauthAuthUrl" class="mb-3 flex items-center gap-2">
            <input type="text" :value="oauthAuthUrl" readonly class="input flex-1" />
            <button
              class="btn btn--secondary btn--icon shrink-0"
              @click="copyAuthUrl"
              :disabled="isLoading || isManualLoading"
              v-tooltip="$t('platform.openai.addAccountDialog.copyAuthLink')"
            >
              <svg class="h-4 w-4" viewBox="0 0 24 24" fill="currentColor">
                <path d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/>
              </svg>
            </button>
          </div>

          <div class="form-group mb-3">
            <label class="label">{{ $t('platform.openai.addAccountDialog.callbackLabel') }}</label>
            <div class="relative flex items-center">
              <input
                v-model="oauthCallbackInput"
                type="text"
                :placeholder="$t('platform.openai.addAccountDialog.callbackPlaceholder')"
                class="input w-full pr-9"
                :disabled="isLoading || isManualLoading"
              />
              <button
                v-if="oauthCallbackInput"
                class="absolute right-1.5 flex h-7 w-7 items-center justify-center rounded text-text-muted transition-colors hover:bg-hover hover:text-text"
                type="button"
                @click="oauthCallbackInput = ''"
              >
                <svg class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z" />
                </svg>
              </button>
            </div>
            <p class="mt-1.5 text-xs text-text-muted">
              {{ $t('platform.openai.addAccountDialog.callbackHint') }}
            </p>
          </div>

          <button class="btn btn--primary" @click="handleOAuthExchange" :disabled="!canExchange || isLoading || isManualLoading">
            {{ $t('platform.openai.addAccountDialog.exchangeCode') }}
          </button>
        </div>
      </div>
    </div>

    <!-- 手动添加方式 -->
    <div v-else-if="addMethod === 'manual'" class="animate-fade-in">
      <div class="mb-4 flex gap-2 rounded-lg bg-muted p-1">
        <button
          type="button"
          :class="[
            'flex flex-1 items-center justify-center rounded-md px-3 py-2 text-sm font-medium transition-all',
            manualTokenType === 'refresh_token'
              ? 'bg-surface text-accent shadow-sm'
              : 'text-text-secondary hover:bg-hover hover:text-text'
          ]"
          @click="manualTokenType = 'refresh_token'"
        >
          Refresh Token
        </button>
        <button
          type="button"
          :class="[
            'flex flex-1 items-center justify-center rounded-md px-3 py-2 text-sm font-medium transition-all',
            manualTokenType === 'access_token'
              ? 'bg-surface text-accent shadow-sm'
              : 'text-text-secondary hover:bg-hover hover:text-text'
          ]"
          @click="manualTokenType = 'access_token'"
        >
          Access Token / Session JSON
        </button>
      </div>

      <div v-if="manualTokenType === 'refresh_token'" class="form-group mb-0">
        <label class="label">{{ $t('platform.openai.addAccountDialog.refreshToken') }}</label>
        <textarea
          v-model="refreshToken"
          :placeholder="$t('platform.openai.addAccountDialog.refreshTokenPlaceholder')"
          class="input resize-none"
          rows="4"
          :disabled="isLoading"
        ></textarea>
        <p class="mt-1.5 text-xs text-text-muted">
          {{ $t('platform.openai.addAccountDialog.refreshTokenHint') }}
        </p>
      </div>

      <div v-else class="form-group mb-0">
        <label class="label">{{ $t('platform.openai.addAccountDialog.accessToken') }}</label>
        <textarea
          v-model="accessToken"
          :placeholder="$t('platform.openai.addAccountDialog.accessTokenPlaceholder')"
          class="input resize-none"
          rows="8"
          :disabled="isLoading"
        ></textarea>
        <p v-if="$t('platform.openai.addAccountDialog.accessTokenHint')" class="mt-1.5 text-xs text-text-muted">
          {{ $t('platform.openai.addAccountDialog.accessTokenHint') }}
        </p>
        <div class="mt-3 rounded-lg border border-warning/30 bg-warning/10 p-3 text-xs text-text-secondary">
          {{ $t('platform.openai.addAccountDialog.accessTokenWarning') }}
        </div>
      </div>
    </div>

    <!-- API 添加方式 -->
    <div v-else-if="addMethod === 'api'" class="animate-fade-in">
      <div class="form-group">
        <label class="label">{{ $t('platform.openai.addAccountDialog.modelProvider') }} <span class="text-text-muted">({{ $t('platform.openai.addAccountDialog.modelProviderHint') }})</span></label>
        <input
          v-model="apiForm.model_provider"
          type="text"
          :placeholder="$t('platform.openai.addAccountDialog.modelProviderPlaceholder')"
          class="input"
          :disabled="isLoading"
        />
      </div>

      <div class="form-group">
        <label class="label">{{ $t('platform.openai.addAccountDialog.model') }} <span class="text-text-muted">({{ $t('platform.openai.addAccountDialog.modelHint') }})</span></label>
        <input
          v-model="apiForm.model"
          type="text"
          :placeholder="$t('platform.openai.addAccountDialog.modelPlaceholder')"
          class="input"
          :disabled="isLoading"
        />
      </div>

      <div class="flex gap-3">
        <div class="form-group flex-1">
          <label class="label">{{ $t('platform.openai.addAccountDialog.reasoningEffort') }} <span class="text-text-muted">({{ $t('platform.openai.addAccountDialog.reasoningEffortHint') }})</span></label>
          <FloatingDropdown v-model="selectedReasoningEffort" placement="bottom-start" :close-on-select="true">
            <template #trigger="{ isOpen }">
              <button type="button" class="input flex items-center justify-between text-left" :disabled="isLoading">
                <span>{{ selectedReasoningEffort || 'medium' }}</span>
                <svg class="w-4 h-4 transition-transform" :class="{ 'rotate-180': isOpen }" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M7 10l5 5 5-5z"/>
                </svg>
              </button>
            </template>
            <template #default="{ close }">
              <button @click="selectReasoningEffort('low', close)" class="dropdown-item">low</button>
              <button @click="selectReasoningEffort('medium', close)" class="dropdown-item">medium</button>
              <button @click="selectReasoningEffort('high', close)" class="dropdown-item">high</button>
              <button @click="selectReasoningEffort('xhigh', close)" class="dropdown-item">xhigh</button>
            </template>
          </FloatingDropdown>
        </div>

        <div class="form-group flex-1">
          <label class="label">{{ $t('platform.openai.addAccountDialog.wireApi') }} <span class="text-text-muted">({{ $t('platform.openai.addAccountDialog.wireApiHint') }})</span></label>
          <FloatingDropdown v-model="selectedWireApi" placement="bottom-start" :close-on-select="true">
            <template #trigger="{ isOpen }">
              <button type="button" class="input flex items-center justify-between text-left" :disabled="isLoading">
                <span>{{ selectedWireApi || 'responses' }}</span>
                <svg class="w-4 h-4 transition-transform" :class="{ 'rotate-180': isOpen }" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M7 10l5 5 5-5z"/>
                </svg>
              </button>
            </template>
            <template #default="{ close }">
              <button @click="selectWireApi('responses', close)" class="dropdown-item">responses</button>
              <button @click="selectWireApi('chat', close)" class="dropdown-item">chat</button>
            </template>
          </FloatingDropdown>
        </div>
      </div>

      <div class="form-group">
        <label class="label">{{ $t('platform.openai.addAccountDialog.baseUrl') }}</label>
        <input
          v-model="apiForm.base_url"
          type="text"
          :placeholder="$t('platform.openai.addAccountDialog.baseUrlPlaceholder')"
          class="input"
          :disabled="isLoading"
        />
      </div>

      <div class="form-group mb-0">
        <label class="label">{{ $t('platform.openai.addAccountDialog.apiKey') }}</label>
        <textarea
          v-model="apiForm.key"
          :placeholder="$t('platform.openai.addAccountDialog.apiKeyPlaceholder')"
          class="input resize-none"
          rows="3"
          :disabled="isLoading"
        ></textarea>
      </div>
    </div>

    <!-- 导入方式 -->
    <div v-else-if="addMethod === 'import'" class="animate-fade-in">
      <!-- Info -->
      <div class="mb-5 flex items-start gap-3 rounded-lg border border-accent/20 bg-accent/10 p-4">
        <svg class="mt-0.5 h-5 w-5 shrink-0 text-accent" viewBox="0 0 24 24" fill="currentColor">
          <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-6h2v6zm0-8h-2V7h2v2z"/>
        </svg>
        <p class="text-[13px] leading-relaxed text-text-secondary">{{ $t('platform.openai.importDialog.info') }}</p>
        <button
          @click="showFormatModal = true"
          class="shrink-0 rounded p-1 text-text-muted hover:bg-accent/20 hover:text-accent transition-colors"
          v-tooltip="$t('platform.openai.importDialog.formatExample')"
        >
          <svg class="h-4 w-4" viewBox="0 0 24 24" fill="currentColor">
            <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 17h-2v-2h2v2zm2.07-7.75l-.9.92C13.45 12.9 13 13.5 13 15h-2v-.5c0-1.1.45-2.1 1.17-2.83l1.24-1.26c.37-.36.59-.86.59-1.41 0-1.1-.9-2-2-2s-2 .9-2 2H8c0-2.21 1.79-4 4-4s4 1.79 4 4c0 .88-.36 1.68-.93 2.25z"/>
          </svg>
        </button>
      </div>

      <!-- 隐藏的文件输入 -->
      <input
        ref="fileInputRef"
        type="file"
        accept=".json"
        class="hidden"
        @change="handleFileChange"
      />

      <!-- 文件选择 -->
      <div v-if="!previewItems.length" class="form-group mb-0">
        <label class="label">{{ $t('platform.openai.importDialog.selectFile') }}</label>
        <div
          class="flex flex-col items-center justify-center gap-3 rounded-lg border-2 border-dashed border-border p-8 cursor-pointer hover:border-accent hover:bg-accent/5 transition-colors"
          @click="selectFile"
        >
          <svg class="h-10 w-10 text-text-muted" viewBox="0 0 24 24" fill="currentColor">
            <path d="M19.35 10.04C18.67 6.59 15.64 4 12 4 9.11 4 6.6 5.64 5.35 8.04 2.34 8.36 0 10.91 0 14c0 3.31 2.69 6 6 6h13c2.76 0 5-2.24 5-5 0-2.64-2.05-4.78-4.65-4.96zM14 13v4h-4v-4H7l5-5 5 5h-3z"/>
          </svg>
          <span class="text-sm text-text-secondary">{{ $t('platform.openai.importDialog.clickToSelect') }}</span>
          <span class="text-xs text-text-muted">{{ $t('platform.openai.importDialog.supportFormat') }}</span>
        </div>
      </div>

      <!-- 预览数据 -->
      <div v-else class="space-y-4">
        <!-- 文件信息 -->
        <div class="flex items-center justify-between rounded-lg bg-surface-alt p-3">
          <div class="flex items-center gap-2">
            <svg class="h-5 w-5 text-accent" viewBox="0 0 24 24" fill="currentColor">
              <path d="M14 2H6c-1.1 0-1.99.9-1.99 2L4 20c0 1.1.89 2 1.99 2H18c1.1 0 2-.9 2-2V8l-6-6zm2 16H8v-2h8v2zm0-4H8v-2h8v2zm-3-5V3.5L18.5 9H13z"/>
            </svg>
            <span class="text-sm text-text">{{ fileName }}</span>
          </div>
          <button
            @click="clearFile"
            class="text-xs text-text-muted hover:text-danger transition-colors"
          >
            {{ $t('platform.openai.importDialog.reselect') }}
          </button>
        </div>

        <!-- 预览列表 -->
        <div class="rounded-lg border border-border">
          <div class="flex items-center justify-between border-b border-border px-4 py-2 bg-surface-alt">
            <span class="text-sm font-medium text-text">
              {{ $t('platform.openai.importDialog.previewTitle', { count: previewItems.length }) }}
            </span>
          </div>
          <div class="max-h-[300px] overflow-y-auto">
            <div
              v-for="(item, index) in previewItems"
              :key="index"
              class="flex items-center gap-2 px-4 py-2.5 border-b border-border last:border-b-0"
            >
              <span class="text-xs text-text-muted shrink-0">#{{ index + 1 }}</span>
              <span v-if="item.type === 'cpa'" class="flex items-center gap-2 text-sm text-text truncate">
                <span class="badge badge--sm bg-accent/15 text-accent">CPA</span>
                <span class="font-mono truncate">{{ item.email }}</span>
              </span>
              <span v-else class="text-sm text-text truncate font-mono">{{ maskToken(item.value) }}</span>
            </div>
          </div>
        </div>

        <!-- 导入结果 -->
        <div v-if="importResult" class="rounded-lg border border-border p-4">
          <div class="flex items-center justify-between mb-3">
            <span class="text-sm font-medium text-text">{{ $t('platform.openai.importDialog.resultTitle') }}</span>
            <span class="text-xs text-text-muted">
              {{ $t('platform.openai.importDialog.resultSummary', {
                success: importResult.success_count,
                failed: importResult.failed_count,
                total: importResult.total
              }) }}
            </span>
          </div>
          <div class="max-h-[200px] overflow-y-auto space-y-1">
            <div
              v-for="(result, index) in importResult.results"
              :key="index"
              class="flex items-center gap-2 text-xs py-1"
            >
              <svg
                v-if="result.success"
                class="h-4 w-4 shrink-0 text-success"
                viewBox="0 0 24 24"
                fill="currentColor"
              >
                <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"/>
              </svg>
              <svg
                v-else
                class="h-4 w-4 shrink-0 text-danger"
                viewBox="0 0 24 24"
                fill="currentColor"
              >
                <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/>
              </svg>
              <span class="text-text truncate">{{ result.email || `#${index + 1}` }}</span>
              <span v-if="result.error" class="text-danger truncate">{{ result.error }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Error Message -->
    <div v-if="error" class="mt-4 flex items-center gap-2 rounded-lg border border-danger/30 bg-danger/10 p-3 text-[13px] text-danger">
      <svg class="h-4 w-4 shrink-0" viewBox="0 0 24 24" fill="currentColor">
        <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"/>
      </svg>
      {{ error }}
    </div>

    <template #footer>
      <button @click="handleClose" class="btn btn--secondary" :disabled="(isLoading && !isOAuthLoginActive) || isImporting">
        {{ (addMethod === 'import' && importResult) ? $t('common.close') : $t('common.cancel') }}
      </button>
      <button
        v-if="addMethod === 'manual'"
        @click="handleAdd"
        class="btn btn--primary"
        :disabled="!canSubmit || isLoading"
      >
        <span class="relative inline-flex items-center justify-center">
          <span :style="{ visibility: isLoading ? 'hidden' : 'visible' }">
            {{ $t('platform.openai.addAccountDialog.add') }}
          </span>
          <span v-if="isLoading" class="btn-spinner absolute inset-0 m-auto" aria-hidden="true"></span>
        </span>
      </button>
      <button
        v-if="addMethod === 'api'"
        @click="handleAddApi"
        class="btn btn--primary"
        :disabled="!canSubmitApi || isLoading"
      >
        <span class="relative inline-flex items-center justify-center">
          <span :style="{ visibility: isLoading ? 'hidden' : 'visible' }">
            {{ $t('platform.openai.addAccountDialog.addApiAccount') }}
          </span>
          <span v-if="isLoading" class="btn-spinner absolute inset-0 m-auto" aria-hidden="true"></span>
        </span>
      </button>
      <button
        v-if="addMethod === 'import' && previewItems.length && !importResult"
        @click="handleImport"
        class="btn btn--primary"
        :disabled="isImporting"
      >
        <span class="relative inline-flex items-center justify-center">
          <span :style="{ visibility: isImporting ? 'hidden' : 'visible' }">
            {{ $t('platform.openai.importDialog.import', { count: previewItems.length }) }}
          </span>
          <span v-if="isImporting" class="btn-spinner absolute inset-0 m-auto" aria-hidden="true"></span>
        </span>
      </button>
    </template>
  </BaseModal>

  <!-- 格式说明 Modal -->
  <BaseModal
    :visible="showFormatModal"
    :title="$t('platform.openai.importDialog.formatExample')"
    :show-close="true"
    close-on-overlay
    close-on-esc
    modal-class="max-w-[500px]"
    @close="showFormatModal = false"
  >
    <div class="space-y-4">
      <pre class="rounded-lg bg-surface-alt p-4 text-xs text-text overflow-x-auto">{{ formatExampleJson }}</pre>
    </div>
    <template #footer>
      <button @click="showFormatModal = false" class="btn btn--primary">{{ $t('common.close') }}</button>
    </template>
  </BaseModal>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import BaseModal from '@/components/common/BaseModal.vue'
import FloatingDropdown from '@/components/common/FloatingDropdown.vue'

const { t: $t } = useI18n()
const emit = defineEmits(['close', 'add', 'added', 'imported'])

const handleClose = async () => {
  if (isOAuthLoginActive.value) {
    await cancelOAuthLogin()
    emit('close')
    return
  }
  if (isLoading.value || isImporting.value) return
  emit('close')
}

const addMethod = ref('oauth') // 'oauth', 'manual', 'api', or 'import'
const manualTokenType = ref('refresh_token') // 'refresh_token' or 'access_token'
const refreshToken = ref('')
const accessToken = ref('')
const isLoading = ref(false)
const isOAuthLoginActive = ref(false)
const isManualLoading = ref(false)
const error = ref('')
const oauthAuthUrl = ref('')
const oauthRedirectUri = ref('')
const oauthSessionId = ref('')
const oauthCallbackInput = ref('')

// 导入方式状态
const showFormatModal = ref(false)
const fileName = ref('')
const previewItems = ref([])
const importResult = ref(null)
const isImporting = ref(false)
const fileInputRef = ref(null)
const formatExampleJson = `// 格式 1：Refresh Token 数组
[
  "rt_refresh_token_1",
  "rt_refresh_token_2"
]

// 格式 2：CPA 单个对象
{
  "type": "codex",
  "email": "user@example.com",
  "account_id": "861bf615-...",
  "access_token": "eyJ...",
  "refresh_token": "rt_...",
  "id_token": "eyJ..."
}

// 格式 3：CPA 对象数组
[
  {
    "type": "codex",
    "email": "user1@example.com",
    "access_token": "eyJ...",
    "refresh_token": "rt_...",
    ...
  }
]

// 格式 4：ATM 导出格式
[
  {
    "email": "user@example.com",
    "token": {
      "access_token": "eyJ...",
      "refresh_token": "rt_...",
      "id_token": "eyJ..."
    },
    ...
  }
]`

// Outlook 验证码助手（默认折叠）+ 折叠的手动 OAuth
const mailHelperOpen = ref(false)
const manualOAuthOpen = ref(false)
const outlookAccounts = ref([])
const isLoadingOutlookAccounts = ref(false)
const selectedOutlookEmail = ref('')
const hasFetchedMail = ref(false)
const isLoadingMail = ref(false)
const extractedCode = ref('')
const mailSearchQuery = ref('')
const mailSearchInputRef = ref(null)

const filteredOutlookAccounts = computed(() => {
  const q = mailSearchQuery.value.trim().toLowerCase()
  if (!q) return outlookAccounts.value
  return outlookAccounts.value.filter(a => a.email.toLowerCase().includes(q))
})

// OpenAI 验证码为 6 位数字；避免把年份等 4 位数字误识别为验证码
const CODE_RE_STRICT = /\b(\d{6})\b/

const extractCode = (text) => {
  if (!text) return ''
  const cleaned = String(text).replace(/[\u200B-\u200D\uFEFF]/g, '')
  const m6 = cleaned.match(CODE_RE_STRICT)
  return m6 ? m6[1] : ''
}

const stripHtml = (html) => {
  if (!html) return ''
  return String(html)
    .replace(/<style[^>]*>[\s\S]*?<\/style>/gi, ' ')
    .replace(/<script[^>]*>[\s\S]*?<\/script>/gi, ' ')
    .replace(/<[^>]+>/g, ' ')
    .replace(/&nbsp;/gi, ' ')
    .replace(/&amp;/gi, '&')
    .replace(/&lt;/gi, '<')
    .replace(/&gt;/gi, '>')
    .replace(/\s+/g, ' ')
    .trim()
}

const loadOutlookAccounts = async () => {
  isLoadingOutlookAccounts.value = true
  try {
    const list = await invoke('outlook_get_all_accounts_info')
    outlookAccounts.value = Array.isArray(list) ? list : []
    if (!selectedOutlookEmail.value && outlookAccounts.value.length > 0) {
      selectedOutlookEmail.value = outlookAccounts.value[0].email
    }
  } catch (err) {
    console.error('Load outlook accounts failed:', err)
    outlookAccounts.value = []
  } finally {
    isLoadingOutlookAccounts.value = false
  }
}

const copyCodeToClipboard = async (code) => {
  try {
    await invoke('copy_to_clipboard', { text: code })
  } catch (nativeErr) {
    console.warn('Native clipboard failed, falling back to browser clipboard:', nativeErr)
    await navigator.clipboard.writeText(code)
  }
}

const fetchVerificationCode = async () => {
  if (!selectedOutlookEmail.value) return
  isLoadingMail.value = true
  hasFetchedMail.value = true
  extractedCode.value = ''
  try {
    const resp = await invoke('outlook_get_emails', {
      email: selectedOutlookEmail.value,
      folder: 'inbox',
      page: 1,
      pageSize: 1
    })
    const emails = Array.isArray(resp?.emails) ? resp.emails : []

    // 先用主题做轻量匹配
    let foundCode = ''
    for (const item of emails) {
      const code = extractCode(item.subject || '')
      if (code) {
        foundCode = code
        break
      }
    }

    // 主题里没有就按最新顺序扫描正文，找到即停止
    if (!foundCode) {
      for (const item of emails) {
        try {
          const detail = await invoke('outlook_get_email_details', {
            email: selectedOutlookEmail.value,
            messageId: item.message_id,
            method: null
          })
          const bodyText = detail?.body_plain || stripHtml(detail?.body_html || '')
          const code = extractCode(bodyText)
          if (code) {
            foundCode = code
            break
          }
        } catch (err) {
          console.warn('Fetch email body failed:', err)
        }
      }
    }

    if (foundCode) {
      try {
        await copyCodeToClipboard(foundCode)
        extractedCode.value = foundCode
        window.$notify?.success($t('platform.openai.addAccountDialog.mailHelper.codeCopied', { code: foundCode }))
      } catch (err) {
        console.error('Copy code failed:', err)
        window.$notify?.error($t('platform.openai.addAccountDialog.mailHelper.copyFailed'))
      }
    } else {
      window.$notify?.warning($t('platform.openai.addAccountDialog.mailHelper.noCodeFound'))
    }
  } catch (err) {
    console.error('Fetch outlook emails failed:', err)
    window.$notify?.error(err?.message || err || $t('platform.openai.addAccountDialog.mailHelper.fetchFailed'))
    hasFetchedMail.value = false
    extractedCode.value = ''
  } finally {
    isLoadingMail.value = false
  }
}

const selectOutlookEmail = (email, close) => {
  selectedOutlookEmail.value = email
  close?.()
}

const copySelectedOutlookEmail = async () => {
  if (!selectedOutlookEmail.value) return
  try {
    await navigator.clipboard.writeText(selectedOutlookEmail.value)
    window.$notify?.success($t('platform.openai.addAccountDialog.mailHelper.emailCopied'))
  } catch (err) {
    console.error('Copy outlook email failed:', err)
    window.$notify?.error($t('platform.openai.addAccountDialog.mailHelper.copyEmailFailed'))
  }
}

const handleMailDropdownOpen = () => {
  mailSearchQuery.value = ''
  // 等待 DOM 渲染，自动聚焦搜索框
  setTimeout(() => {
    mailSearchInputRef.value?.focus()
  }, 50)
}

const onSearchEnter = (close) => {
  const first = filteredOutlookAccounts.value[0]
  if (first) {
    selectOutlookEmail(first.email, close)
  }
}

watch(mailHelperOpen, (open) => {
  if (open && outlookAccounts.value.length === 0 && !isLoadingOutlookAccounts.value) {
    loadOutlookAccounts()
  }
})


// API 表单数据
const apiForm = ref({
  model_provider: '',
  model: '',
  base_url: '',
  key: ''
})
const selectedReasoningEffort = ref('medium')
const selectedWireApi = ref('responses')

const canSubmit = computed(() => {
  if (addMethod.value !== 'manual') return false
  if (manualTokenType.value === 'access_token') return accessToken.value.trim()
  return refreshToken.value.trim()
})

const canSubmitApi = computed(() => {
  return apiForm.value.model_provider.trim() &&
         apiForm.value.base_url.trim() &&
         apiForm.value.key.trim()
})

const canExchange = computed(() => {
  const raw = oauthCallbackInput.value.trim()
  if (!raw) return false
  if (/^https?:\/\//i.test(raw)) return true
  return !!oauthSessionId.value
})

const resetOAuthState = () => {
  oauthAuthUrl.value = ''
  oauthRedirectUri.value = ''
  oauthSessionId.value = ''
  oauthCallbackInput.value = ''
}

const switchToOAuth = () => {
  addMethod.value = 'oauth'
  resetOAuthState()
}

let unlistenOAuthUrl = null
let isOAuthCancelRequested = false

onMounted(async () => {
  unlistenOAuthUrl = await listen('oauth-url-generated', event => {
    const url = typeof event.payload === 'string' ? event.payload : ''
    if (!url) return
    oauthAuthUrl.value = url
    try {
      const parsed = new URL(url)
      const redirect = parsed.searchParams.get('redirect_uri')
      if (redirect) {
        oauthRedirectUri.value = redirect
      }
    } catch (err) {
      console.error('Parse oauth url error:', err)
    }
  })
})

onUnmounted(() => {
  if (unlistenOAuthUrl) {
    unlistenOAuthUrl()
    unlistenOAuthUrl = null
  }
  if (isOAuthLoginActive.value) {
    cancelOAuthLogin()
  }
})

const cancelOAuthLogin = async () => {
  if (!isOAuthLoginActive.value) return
  isOAuthCancelRequested = true
  isOAuthLoginActive.value = false
  isLoading.value = false
  try {
    await invoke('openai_cancel_oauth_login')
  } catch (err) {
    console.error('Cancel OAuth login error:', err)
  }
}

const handleOAuthLogin = async () => {
  error.value = ''
  isLoading.value = true
  isOAuthLoginActive.value = true
  isOAuthCancelRequested = false
  resetOAuthState()

  try {
    // 使用自动 OAuth 登录流程（会启动本地服务器监听回调）
    const account = await invoke('openai_start_oauth_login')
    emit('added', account)
  } catch (err) {
    console.error('OAuth login error:', err)
    if (!isOAuthCancelRequested) {
      error.value = formatOAuthError(err)
    }
  } finally {
    isOAuthLoginActive.value = false
    isOAuthCancelRequested = false
    isLoading.value = false
  }
}

const generateAuthUrl = async () => {
  error.value = ''
  isManualLoading.value = true

  try {
    const result = await invoke('openai_generate_auth_url')
    oauthAuthUrl.value = result.auth_url
    oauthSessionId.value = result.session_id
  } catch (err) {
    console.error('Generate auth url error:', err)
    error.value = err?.message || err || '生成授权链接失败'
  } finally {
    isManualLoading.value = false
  }
}

const copyAuthUrl = async () => {
  if (!oauthAuthUrl.value) return

  try {
    await navigator.clipboard.writeText(oauthAuthUrl.value)
    window.$notify?.success($t('platform.openai.addAccountDialog.authLinkCopied'))
  } catch (err) {
    console.error('Copy auth url error:', err)
    error.value = err?.message || err || '复制授权链接失败'
  }
}

const formatOAuthError = (err) => {
  const message = err?.message || err || ''
  return message || $t('platform.openai.addAccountDialog.oauthExchangeFailed')
}

const handleOAuthExchange = async () => {
  const rawInput = oauthCallbackInput.value.trim()
  if (!rawInput) return

  error.value = ''
  isManualLoading.value = true

  try {
    let code = rawInput
    let redirectUri = oauthRedirectUri.value

    if (/^https?:\/\//i.test(rawInput)) {
      const url = new URL(rawInput)
      code = url.searchParams.get('code') || ''
      redirectUri = `${url.origin}${url.pathname}`
    }

    if (!code) {
      throw new Error($t('platform.openai.addAccountDialog.invalidCallback'))
    }

    const account = await invoke('openai_exchange_code', {
      sessionId: oauthSessionId.value,
      code,
      redirectUri
    })
    emit('added', account)
  } catch (err) {
    console.error('Exchange code error:', err)
    error.value = formatOAuthError(err)
  } finally {
    isManualLoading.value = false
  }
}

const handleAdd = async () => {
  if (!canSubmit.value) return

  error.value = ''
  isLoading.value = true

  try {
    const account = manualTokenType.value === 'access_token'
      ? await invoke('openai_add_account_with_access_token', {
          accessToken: accessToken.value.trim()
        })
      : await invoke('openai_add_account', {
          refreshToken: refreshToken.value.trim()
        })
    emit('added', account)
  } catch (err) {
    console.error('Add account error:', err)
    error.value = err?.message || err || '添加账号失败'
    isLoading.value = false
  }
}

const handleAddApi = async () => {
  if (!canSubmitApi.value) return

  error.value = ''
  isLoading.value = true

  try {
    const account = await invoke('openai_add_api_account', {
      modelProvider: apiForm.value.model_provider.trim(),
      model: apiForm.value.model.trim(),
      reasoningEffort: selectedReasoningEffort.value,
      wireApi: selectedWireApi.value,
      baseUrl: apiForm.value.base_url.trim(),
      key: apiForm.value.key.trim()
    })
    emit('added', account)
  } catch (err) {
    console.error('Add API account error:', err)
    error.value = err?.message || err || '添加 API 账号失败'
    isLoading.value = false
  }
}

const selectReasoningEffort = (value, close) => {
  selectedReasoningEffort.value = value
  close?.()
}

const selectWireApi = (value, close) => {
  selectedWireApi.value = value
  close?.()
}

// ===== 导入方式 =====
const isCpaObject = (item) => {
  return item && typeof item === 'object' && typeof item.access_token === 'string'
}

const isAtmInternalObject = (item) => {
  return item && typeof item === 'object' && item.token && typeof item.token.access_token === 'string'
}

const normalizeToCpa = (item) => {
  if (isCpaObject(item)) return item
  if (isAtmInternalObject(item)) {
    if (item.account_type === 'api') return null
    return {
      email: item.email || '',
      account_id: item.chatgpt_account_id || item.account_id || null,
      access_token: item.token.access_token,
      refresh_token: item.token.refresh_token || null,
      id_token: item.token.id_token || null
    }
  }
  return null
}

const maskToken = (token) => {
  if (!token || token.length < 12) return '***'
  return token.slice(0, 6) + '...' + token.slice(-4)
}

const selectFile = () => {
  fileInputRef.value?.click()
}

const handleFileChange = async (event) => {
  const file = event.target.files?.[0]
  if (!file) return

  try {
    const content = await file.text()
    const data = JSON.parse(content)

    const items = []

    if (Array.isArray(data)) {
      for (const entry of data) {
        if (typeof entry === 'string' && entry.trim().length > 0) {
          items.push({ type: 'rt', value: entry.trim() })
        } else {
          const normalized = normalizeToCpa(entry)
          if (normalized) {
            items.push({ type: 'cpa', email: normalized.email || '', data: normalized })
          }
        }
      }
    } else {
      const normalized = normalizeToCpa(data)
      if (normalized) {
        items.push({ type: 'cpa', email: normalized.email || '', data: normalized })
      } else if (typeof data === 'string' && data.trim().length > 0) {
        items.push({ type: 'rt', value: data.trim() })
      }
    }

    if (items.length === 0) {
      error.value = $t('platform.openai.importDialog.emptyFile')
      return
    }

    fileName.value = file.name
    previewItems.value = items
    importResult.value = null
    error.value = ''
  } catch (err) {
    console.error('Failed to read file:', err)
    error.value = err?.message || $t('platform.openai.importDialog.readError')
  } finally {
    event.target.value = ''
  }
}

const clearFile = () => {
  fileName.value = ''
  previewItems.value = []
  importResult.value = null
  error.value = ''
}

const handleImport = async () => {
  if (!previewItems.value.length || isImporting.value) return

  error.value = ''
  isImporting.value = true
  const results = []
  const accountIds = []
  let success_count = 0
  let failed_count = 0

  try {
    for (const item of previewItems.value) {
      try {
        let account
        if (item.type === 'cpa') {
          const d = item.data
          account = await invoke('openai_import_account_direct', {
            email: d.email,
            accountId: d.account_id || null,
            accessToken: d.access_token,
            refreshToken: d.refresh_token || null,
            idToken: d.id_token || null
          })
        } else {
          account = await invoke('openai_add_account', { refreshToken: item.value })
        }
        results.push({ success: true, email: account?.email || item.email || '' })
        if (account?.id) accountIds.push(account.id)
        success_count++
      } catch (err) {
        const msg = err?.message || err || String(err)
        results.push({ success: false, email: item.email || '', error: msg })
        failed_count++
      }
    }

    importResult.value = {
      success_count,
      failed_count,
      total: previewItems.value.length,
      results
    }

    if (success_count > 0) {
      emit('imported', { ...importResult.value, accountIds })
    }
  } catch (err) {
    console.error('Import error:', err)
    error.value = err?.message || err || $t('platform.openai.importDialog.importFailed')
  } finally {
    isImporting.value = false
  }
}
</script>

<style scoped>
/* Fade-in animation for tab content */
.animate-fade-in {
  animation: fadeIn 0.3s ease-out;
}

@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(-10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
