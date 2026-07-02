<template>
  <!-- Spotlight 搜索窗口 -->
  <SpotlightSearch v-if="isSpotlightWindow" />

  <!-- 主应用窗口 -->
  <div v-else class="flex h-screen flex-col overflow-hidden bg-page text-text">
    <!-- App Body: Sidebar + Main Content -->
    <div class="flex flex-1 overflow-hidden">
      <!-- Sidebar -->
      <aside
        :class="[
          'flex shrink-0 flex-col border-r border-border bg-surface transition-[width] duration-150 ease-out',
          isSidebarCollapsed ? 'w-16' : 'w-38',
        ]">
        <div class="flex h-full flex-col">
          <!-- Sidebar Navigation -->
          <nav class="flex-1">
            <!-- Expand/Collapse Sidebar Button (最顶部) -->
            <button
              type="button"
              :class="[
                'btn btn--ghost btn--lg min-w-0 w-full',
                isSidebarCollapsed ? 'justify-center px-2 py-2.5' : 'justify-start px-3 py-2.5',
              ]"
              @click="toggleSidebar"
              :aria-label="isSidebarCollapsed ? $t('app.expandSidebar') : $t('app.collapseSidebar')"
              v-tooltip="isSidebarCollapsed ? $t('app.expandSidebar') : $t('app.collapseSidebar')"
            >
              <svg v-if="isSidebarCollapsed" class="h-5 w-5 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="9 18 15 12 9 6" />
              </svg>
              <svg v-else class="h-5 w-5 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="15 18 9 12 15 6" />
              </svg>
              <span v-if="!isSidebarCollapsed" class="flex-1 min-w-0 truncate">{{ $t('app.collapseSidebar') }}</span>
            </button>
            <!-- Platform Selection Button -->
            <button
              :class="[
                'btn btn--ghost btn--lg min-w-0 w-full',
                isSidebarCollapsed ? 'justify-center px-2 py-2.5' : 'justify-start px-3 py-2.5',
                activeView === 'platforms'
                  ? 'text-accent'
                  : '',
              ]"
              @click="navigateToView('platforms')"
              v-tooltip="$t('platforms.title')"
            >
              <svg class="h-5 w-5 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round">
                <path d="M12 3l2.5 6.5L21 12l-6.5 2.5L12 21l-2.5-6.5L3 12l6.5-2.5L12 3z"/>
              </svg>
              <span v-if="!isSidebarCollapsed" class="flex-1 min-w-0 truncate">{{ $t('platforms.title') }}</span>
            </button>
            
            <!-- Subscriptions Button -->
            <button
              :class="[
                'btn btn--ghost btn--lg min-w-0 w-full',
                isSidebarCollapsed ? 'justify-center px-2 py-2.5' : 'justify-start px-3 py-2.5',
                activeView === 'subscriptions'
                  ? 'text-accent'
                  : '',
              ]"
              @click="navigateToView('subscriptions')"
              v-tooltip="$t('subscriptions.title')"
            >
              <svg class="h-5 w-5 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round">
                <path d="M12 3l7 7-7 11-7-11 7-7z"/>
              </svg>
              <span v-if="!isSidebarCollapsed" class="flex-1 min-w-0 truncate">{{ $t('subscriptions.title') }}</span>
            </button>

            <button
              :class="[
                'btn btn--ghost btn--lg min-w-0 w-full',
                isSidebarCollapsed ? 'justify-center px-2 py-2.5' : 'justify-start px-3 py-2.5',
                activeView === 'bookmarks'
                  ? 'text-accent'
                  : '',
              ]"
              @click="navigateToView('bookmarks')"
              v-tooltip="$t('bookmarks.title')"
            >
              <svg class="h-5 w-5 shrink-0" viewBox="0 0 24 24" fill="currentColor">
                <path d="M17 3H7c-1.1 0-1.99.9-1.99 2L5 21l7-3 7 3V5c0-1.1-.9-2-2-2z" />
              </svg>
              <span v-if="!isSidebarCollapsed" class="flex-1 min-w-0 truncate">{{ $t('bookmarks.title') }}</span>
            </button>

            <button
              :class="[
                'btn btn--ghost btn--lg min-w-0 w-full',
                isSidebarCollapsed ? 'justify-center px-2 py-2.5' : 'justify-start px-3 py-2.5',
                activeView === 'emails'
                  ? 'text-accent'
                  : '',
              ]"
              @click="navigateToView('emails')"
              v-tooltip="$t('emails.title')"
            >
              <svg class="h-5 w-5 shrink-0" viewBox="0 0 24 24" fill="currentColor">
                <path d="M20 4H4c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V6c0-1.1-.9-2-2-2zm0 4l-8 5-8-5V6l8 5 8-5v2z" />
              </svg>
              <span v-if="!isSidebarCollapsed" class="flex-1 min-w-0 truncate">{{ $t('emails.title') }}</span>
            </button>

            <!-- Gateway Button -->
            <button
              :class="[
                'btn btn--ghost btn--lg min-w-0 w-full',
                isSidebarCollapsed ? 'justify-center px-2 py-2.5' : 'justify-start px-3 py-2.5',
                activeView === 'gateway'
                  ? 'text-accent'
                  : '',
              ]"
              @click="navigateToView('gateway')"
              v-tooltip="$t('gateway.title')"
            >
              <svg class="h-5 w-5 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <rect x="2" y="2" width="20" height="8" rx="2" />
                <rect x="2" y="14" width="20" height="8" rx="2" />
                <path d="M6 6h.01" />
                <path d="M6 18h.01" />
              </svg>
              <span v-if="!isSidebarCollapsed" class="flex-1 min-w-0 truncate">{{ $t('gateway.title') }}</span>
            </button>

          </nav>

          <!-- Sidebar Bottom: Controls -->
          <div class="mt-auto border-t border-border mb-3">
            <!-- Open Data Folder Button -->
            <button
              type="button"
              :class="[
                'btn btn--ghost btn--lg min-w-0 w-full',
                isSidebarCollapsed ? 'justify-center px-2 py-2.5' : 'justify-start px-3 py-2.5',
              ]"
              @click="openDataFolder"
              v-tooltip="$t('common.openDataFolder')"
            >
              <svg class="h-5 w-5 shrink-0" viewBox="0 0 24 24" fill="currentColor">
                <path d="M10 4H4c-1.11 0-2 .89-2 2v12c0 1.11.89 2 2 2h16c1.11 0 2-.89 2-2V8c0-1.11-.89-2-2-2h-8l-2-2z" />
              </svg>
              <span v-if="!isSidebarCollapsed" class="flex-1 min-w-0 truncate">{{ $t('common.openDataFolder') }}</span>
            </button>
            <!-- Theme Toggle -->
            <button
              type="button"
              :class="[
                'btn btn--ghost btn--lg min-w-0 w-full',
                isSidebarCollapsed ? 'justify-center px-2 py-2.5' : 'justify-start px-3 py-2.5',
              ]"
              @click="toggleTheme"
              :aria-label="themeToggleLabel" v-tooltip="themeToggleLabel">
              <svg v-if="themePreference === 'system'" class="h-5 w-5 shrink-0" viewBox="0 0 24 24" fill="none"
                stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <rect x="2" y="3" width="20" height="14" rx="2" />
                <path d="M8 21h8" />
                <path d="M12 17v4" />
              </svg>
              <svg v-else-if="themePreference === 'dark'" class="h-5 w-5 shrink-0" viewBox="0 0 24 24" fill="none"
                stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
              </svg>
              <svg v-else class="h-5 w-5 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                stroke-linecap="round" stroke-linejoin="round">
                <circle cx="12" cy="12" r="5" />
                <path d="m12 1 0 2" />
                <path d="m12 21 0 2" />
                <path d="m4.22 4.22 1.42 1.42" />
                <path d="m18.36 18.36 1.42 1.42" />
                <path d="m1 12 2 0" />
                <path d="m21 12 2 0" />
                <path d="m4.22 19.78 1.42-1.42" />
                <path d="m18.36 5.64 1.42-1.42" />
              </svg>
              <span v-if="!isSidebarCollapsed" class="flex-1 min-w-0 truncate">{{ themeModeLabel }}</span>
            </button>

            <!-- Language Toggle -->
            <button
              type="button"
              :class="[
                'btn btn--ghost btn--lg min-w-0 w-full',
                isSidebarCollapsed ? 'justify-center px-2 py-2.5' : 'justify-start px-3 py-2.5',
              ]"
              @click="toggleLanguage"
              :aria-label="languageToggleLabel" v-tooltip="languageToggleLabel">
              <svg class="h-5 w-5 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"/>
                <path d="M2 12h20"/>
                <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>
              </svg>
              <span v-if="!isSidebarCollapsed" class="flex-1 min-w-0 truncate">{{ currentLocale === 'zh-CN' ? 'English' : '中文' }}</span>
            </button>

            <!-- Settings Button -->
            <button
              type="button"
              :class="[
                'btn btn--ghost btn--lg min-w-0 w-full',
                isSidebarCollapsed ? 'justify-center px-2 py-2.5' : 'justify-start px-3 py-2.5',
              ]"
              @click="navigateToView('settings')"
              v-tooltip="$t('app.settings')"
            >
              <svg class="h-5 w-5 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="3" />
                <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
              </svg>
              <span v-if="!isSidebarCollapsed" class="flex-1 min-w-0 truncate">{{ $t('app.settings') }}</span>
            </button>
          </div>
        </div>
      </aside>

      <!-- Main Content Area -->
      <main class="flex-1 overflow-hidden bg-page">
        <!-- Platform Selector View -->
        <PlatformSelector
          v-show="activeView === 'platforms'"
          ref="platformSelectorRef"
          :is-dark-theme="isDarkTheme"
          :is-sidebar-collapsed="isSidebarCollapsed"
        />

        <!-- Bookmarks View -->
        <BookmarkPage
          v-if="activeView === 'bookmarks'"
          :key="'bookmarks-' + viewRefreshKey"
          :is-sidebar-collapsed="isSidebarCollapsed"
        />

        <!-- Emails View (v-show to preserve sub-view state when switching sidebar) -->
        <EmailPage v-show="activeView === 'emails'" ref="emailPageRef" />

        <!-- Subscriptions View -->
        <SubscriptionPage
          v-if="activeView === 'subscriptions'"
          :key="'subscriptions-' + viewRefreshKey"
          :is-sidebar-collapsed="isSidebarCollapsed"
        />

        <!-- Gateway View (v-show to preserve tab state when switching sidebar) -->
        <GatewayPage v-show="activeView === 'gateway'" />

        <!-- Settings View -->
        <SettingsPage v-if="activeView === 'settings'" :key="'settings-' + viewRefreshKey" />
      </main>
    </div><!-- End of app-body -->

    <!-- Outlook Manager Modal -->
    <OutlookManager v-if="showOutlookManager" @close="showOutlookManager = false" />

    <!-- GPTMail Manager Modal -->
    <GPTMailManager v-if="showGPTMailManager" @close="showGPTMailManager = false" />

    <!-- Proxy Config Modal -->
    <ProxyConfig v-if="showProxyConfig" @close="showProxyConfig = false"/>

    <!-- API Server Status Modal -->
    <ApiServerStatus v-if="showApiServerStatus" @close="showApiServerStatus = false" />

    <!-- Notification Manager -->
    <NotificationManager ref="notificationManager" />

    <!-- Confirm Manager -->
    <ConfirmManager />

    <!-- 更新横幅 -->
    <UpdateBanner v-if="updateInfo && updateInfo.has_update" :update-info="updateInfo" @close="closeUpdateBanner" />
  </div>
</template>

<script setup>
import { ref, onMounted, computed, inject, onBeforeUnmount } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useI18n } from 'vue-i18n'
import { useSettingsStore } from './stores/settings'
import OutlookManager from './components/email/OutlookManager.vue'
import GPTMailManager from './components/email/GPTMailManager.vue'
import ProxyConfig from './components/settings/ProxyConfig.vue'
import ExternalLinkDialog from './components/common/ExternalLinkDialog.vue'
import NotificationManager from './components/common/NotificationManager.vue'
import ConfirmManager from './components/common/ConfirmManager.vue'
import UpdateBanner from './components/common/UpdateBanner.vue'
import ApiServerStatus from './components/settings/ApiServerStatus.vue'
import PlatformSelector from './components/platforms/PlatformSelector.vue'
import BookmarkPage from './components/pages/BookmarkPage.vue'
import EmailPage from './components/pages/EmailPage.vue'
import SettingsPage from './components/pages/SettingsPage.vue'
import SubscriptionPage from './components/pages/SubscriptionPage.vue'
import GatewayPage from './components/pages/GatewayPage.vue'
import SpotlightSearch from './components/spotlight/SpotlightSearch.vue'

const { t, locale } = useI18n()

// 检测是否为 Spotlight 搜索窗口
const isSpotlightWindow = new URLSearchParams(window.location.search).get('window') === 'spotlight'

// Settings store for tray management
const settingsStore = useSettingsStore()

// ========== 新增：主视图状态管理 ==========
// 主视图类型定义：'platforms' | 'bookmarks' | 'emails' | 'settings'
const activeView = ref('platforms')
const platformSelectorRef = ref(null)
const emailPageRef = ref(null)
// 侧边栏折叠状态
const isSidebarCollapsed = ref(false)

// 视图刷新计数器,用于强制重新渲染
const viewRefreshKey = ref(0)

// 导航函数
const navigateToView = (view) => {
  if (view === 'platforms' && activeView.value === 'platforms') {
    const selector = platformSelectorRef.value
    if (selector && typeof selector.clearSelection === 'function') {
      selector.clearSelection()
    }
    return
  }

  if (view === 'emails' && activeView.value === 'emails') {
    const page = emailPageRef.value
    if (page && typeof page.clearSelection === 'function') {
      page.clearSelection()
    }
    return
  }

  // 如果点击的是当前已激活的视图,增加刷新计数器强制重新渲染
  if (activeView.value === view) {
    viewRefreshKey.value++
  } else {
    activeView.value = view
  }
}

// 侧边栏收缩/展开
const toggleSidebar = () => {
  isSidebarCollapsed.value = !isSidebarCollapsed.value
  // 保存状态到localStorage
  try {
    localStorage.setItem('sidebar-collapsed', isSidebarCollapsed.value.toString())
  } catch (error) {
    console.warn('Failed to save sidebar state', error)
  }
}

// 打开数据文件夹
const openDataFolder = async () => {
  try {
    await invoke('open_data_folder')
  } catch (error) {
    console.error('Failed to open data folder:', error)
    window.$notify?.error(`Failed to open data folder: ${error}`)
  }
}

const selectPlatformById = (platformId) => {
  activeView.value = 'platforms'
  const selector = platformSelectorRef.value
  if (!selector || typeof selector.selectPlatformById !== 'function') {
    return null
  }
  return selector.selectPlatformById(platformId)
}
// ========== 主视图状态管理结束 ==========

// 当前语言
const currentLocale = ref(locale.value)

// 切换语言
const changeLanguage = () => {
  locale.value = currentLocale.value
  // 可以在这里添加保存语言偏好到本地存储的逻辑
  localStorage.setItem('preferred-language', currentLocale.value)
}

// 语言切换按钮
const toggleLanguage = () => {
  currentLocale.value = currentLocale.value === 'zh-CN' ? 'en-US' : 'zh-CN'
  changeLanguage()
}

const languageToggleLabel = computed(() => (currentLocale.value === 'zh-CN' ? t('app.switchToEnglish') : t('app.switchToChinese')))

const showOutlookManager = ref(false)
const showGPTMailManager = ref(false)
const showProxyConfig = ref(false)
const showApiServerStatus = ref(false)

// 组件引用
const notificationManager = ref(null)


// Session import data (kept for backward compatibility with modals)
const sessionInput = ref('')
const sessionTokenResult = ref(null)
const isImportingSession = ref(false)
const sessionImportProgress = ref('')
const isOpeningBrowser = ref(false)


const themeManager = inject('themeManager', null)
const themeStorageKey = themeManager?.storageKey ?? 'atm-theme'

let storedThemePreference = null
try {
  storedThemePreference = themeManager?.getStoredTheme?.() ?? localStorage.getItem(themeStorageKey) ?? null
} catch (error) {
  console.warn('Failed to read stored theme preference inside App.vue', error)
}

const resolveSystemTheme = () => {
  if (themeManager?.resolveSystemTheme) {
    return themeManager.resolveSystemTheme()
  }
  return themeManager?.mediaQuery?.matches ? 'dark' : 'light'
}

const THEME_PREFERENCES = ['light', 'dark', 'system']
const themePreference = ref(
  THEME_PREFERENCES.includes(storedThemePreference) ? storedThemePreference : 'system'
)
const currentTheme = ref(themeManager?.initialTheme ?? document.documentElement.dataset.theme ?? 'light')
const isDarkTheme = computed(() => currentTheme.value === 'dark')

const fallbackApplyTheme = (theme) => {
  const normalized = theme === 'dark' ? 'dark' : 'light'
  const root = document.documentElement
  root.dataset.theme = normalized
  root.style.colorScheme = normalized
}

const applyResolvedTheme = () => {
  const resolved = themePreference.value === 'system' ? resolveSystemTheme() : themePreference.value
  const normalized = resolved === 'dark' ? 'dark' : 'light'
  currentTheme.value = normalized

  if (themeManager?.applyTheme) {
    themeManager.applyTheme(normalized)
  } else {
    fallbackApplyTheme(normalized)
  }
}

const setThemePreference = (preference) => {
  themePreference.value = THEME_PREFERENCES.includes(preference) ? preference : 'system'
  applyResolvedTheme()

  try {
    localStorage.setItem(themeStorageKey, themePreference.value)
  } catch (error) {
    console.warn('Failed to persist theme preference', error)
  }
}

const toggleTheme = () => {
  const nextIndex = (THEME_PREFERENCES.indexOf(themePreference.value) + 1) % THEME_PREFERENCES.length
  setThemePreference(THEME_PREFERENCES[nextIndex])
}

const themeModeLabel = computed(() => {
  if (themePreference.value === 'system') return t('app.systemMode')
  return themePreference.value === 'dark' ? t('app.darkMode') : t('app.lightMode')
})
const themeToggleLabel = computed(() => t('app.switchThemeHint', { mode: themeModeLabel.value }))

let cleanupSystemThemeListener

if (themeManager?.mediaQuery) {
  const mediaQuery = themeManager.mediaQuery
  const handleSystemThemeChange = () => {
    if (themePreference.value !== 'system') {
      return
    }
    applyResolvedTheme()
  }

  if (typeof mediaQuery.addEventListener === 'function') {
    mediaQuery.addEventListener('change', handleSystemThemeChange)
    cleanupSystemThemeListener = () => mediaQuery.removeEventListener('change', handleSystemThemeChange)
  } else if (typeof mediaQuery.addListener === 'function') {
    mediaQuery.addListener(handleSystemThemeChange)
    cleanupSystemThemeListener = () => mediaQuery.removeListener(handleSystemThemeChange)
  }
}

onMounted(() => {
  applyResolvedTheme()
})

// Update check
const updateInfo = ref(null)

const checkForUpdates = async (silent = false) => {
  try {
    const result = await invoke('check_for_updates')
    if (result && result.has_update) {
      updateInfo.value = result
      if (!silent) {
        showStatus(t('update.newVersionAvailable'), 'info')
      }
    } else if (!silent) {
      showStatus(t('update.upToDate'), 'success')
    }
  } catch (error) {
    console.error('Failed to check for updates:', error)
    if (!silent) {
      showStatus(t('update.checkFailed'), 'error')
    }
  }
}

const closeUpdateBanner = () => {
  updateInfo.value = null
}


// Methods
const showStatus = (message, type = 'info') => {
  // 优先使用全局$notify
  window.$notify[type](message)
}

// Session import methods
const importFromSession = async () => {
  if (!sessionInput.value.trim()) {
    showStatus(t('messages.sessionRequired'), 'warning')
    return
  }

  isImportingSession.value = true
  sessionImportProgress.value = t('messages.sessionImportStarting')
  showStatus(t('messages.importingSession'), 'info')

  try {
    const authSession = sessionInput.value.trim()
    const result = await invoke('add_token_from_session', { session: authSession })

    // Session导入成功，跳转到Augment平台页
    sessionImportProgress.value = t('messages.sessionImportSuccess')
    showStatus(t('messages.sessionImportSuccess'), 'success')

    // 清空输入
    sessionInput.value = ''
    sessionTokenResult.value = null

    // 跳转到Augment平台页
    selectPlatformById('augment')

  } catch (error) {
    sessionImportProgress.value = t('messages.sessionImportFailed')
    // 映射后端错误标识符到 i18n key
    let errorMessage = error
    if (error.includes('SESSION_ERROR_OR_ACCOUNT_BANNED')) {
      errorMessage = t('messages.sessionErrorOrAccountBanned')
    }
    showStatus(`${t('messages.error')}: ${errorMessage}`, 'error')
  } finally {
    isImportingSession.value = false
  }
}

onMounted(async () => {
  // 读取保存的语言偏好（所有窗口共用）
  const savedLanguage = localStorage.getItem('preferred-language')
  if (savedLanguage && (savedLanguage === 'zh-CN' || savedLanguage === 'en-US')) {
    currentLocale.value = savedLanguage
    locale.value = savedLanguage
  }

  // Spotlight 窗口不需要初始化主窗口逻辑
  if (isSpotlightWindow) return

  // 读取侧边栏折叠状态
  const sidebarCollapsed = localStorage.getItem('sidebar-collapsed')
  if (sidebarCollapsed === 'true') {
    isSidebarCollapsed.value = true
  }

  // 读取并应用自定义字体
  const savedFont = localStorage.getItem('user-font-sans')
  if (savedFont) {
    const fontValue = `"${savedFont}", system-ui, sans-serif`
    document.documentElement.style.setProperty('--font-sans', fontValue)
  }

  // 启动时检查更新（静默模式）
  checkForUpdates(true)

  // 初始化系统托盘（根据用户设置）
  settingsStore.initializeTray()

  // 初始化 Dock 图标状态（macOS，根据用户设置）
  settingsStore.initializeDock()

  // 自动注册 Spotlight 快捷键（如果之前配置过）
  const savedSpotlightShortcut = localStorage.getItem('atm-spotlight-shortcut')
  if (savedSpotlightShortcut) {
    try {
      await invoke('register_spotlight_shortcut', { shortcut: savedSpotlightShortcut })
    } catch (e) {
      console.warn('Failed to auto-register spotlight shortcut:', e)
    }
  }

  // 监听托盘菜单点击事件
  await listen('tray-menu-clicked', (event) => {
    const action = event.payload?.action
    if (action === 'platforms' || action === 'subscriptions' || action === 'bookmarks' || action === 'emails') {
      navigateToView(action)
    }
  })

  // 监听 Deep Link Session 接收事件（由前端处理导入）
  await listen('deep-link-session-received', async (event) => {
    // 调用前端的导入方法（会显示进度和结果提示，并跳转到Augment平台页）
    if (event.payload.session) {
      sessionInput.value = event.payload.session
      await importFromSession()
    }
  })

  // 监听 API 服务器状态变化事件
  await listen('api-server-status-changed', () => {
    settingsStore.loadServerStatus(true)
  })

})

onBeforeUnmount(() => {
  if (typeof cleanupSystemThemeListener === 'function') {
    cleanupSystemThemeListener()
  }
})


</script>
