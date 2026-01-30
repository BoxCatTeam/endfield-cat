import { defineStore } from 'pinia'
import { ref } from 'vue'
import { openUrl } from '@tauri-apps/plugin-opener'
import { Snackbar } from '@varlet/ui'
import i18n from '../i18n' // 直接使用全局 i18n 实例
import { downloadAndApplyUpdate, fetchLatestRelease, getAppVersion } from '../api/tauriCommands'

export type LatestRelease = {
    tag_name: string
    name?: string
    html_url?: string
    download_url?: string
    body?: string
}

export const useUpdaterStore = defineStore('updater', () => {
    const updateInfo = ref<LatestRelease | null>(null)
    const showUpdateDialog = ref(false)
    const isUpdating = ref(false)
    const isChecking = ref(false)

    // 版本比较：返回远端是否更高
    const isNewerVersion = (local: string, remote: string): boolean => {
        const parseVersion = (v: string) => v.replace(/^v/, '').split('.').map(Number)
        const localParts = parseVersion(local)
        const remoteParts = parseVersion(remote)

        for (let i = 0; i < Math.max(localParts.length, remoteParts.length); i++) {
            const l = localParts[i] || 0
            const r = remoteParts[i] || 0
            if (r > l) return true
            if (r < l) return false
        }
        return false
    }

    const checkForUpdate = async (silent = false) => {
        if (isChecking.value) return
        isChecking.value = true
        try {
            const [localVersion, release] = await Promise.all([
                getAppVersion(),
                fetchLatestRelease<LatestRelease>()
            ])

            if (release && isNewerVersion(localVersion, release.tag_name)) {
                updateInfo.value = release
                showUpdateDialog.value = true
            } else if (!silent) {
                // 主动检查时提示“已是最新”
                Snackbar.success(i18n.global.t('settings.update.alreadyLatest') || 'Already latest version')
            }
        } catch (error) {
            console.error("Failed to check for updates:", error)
            if (!silent) {
                Snackbar.error(i18n.global.t('settings.update.checkFailed') || 'Check failed')
            }
        } finally {
            isChecking.value = false
        }
    }

    const installUpdate = async () => {
        if (!updateInfo.value?.download_url) {
            Snackbar.error(i18n.global.t('settings.update.installFailed') || 'Install failed')
            return
        }

        isUpdating.value = true
        try {
            await downloadAndApplyUpdate(updateInfo.value.download_url)
        } catch (error) {
            console.error("Update failed:", error)
            Snackbar.error(i18n.global.t('settings.update.installFailed') || 'Install failed')
            isUpdating.value = false
        }
    }

    const manualDownload = async () => {
        if (updateInfo.value?.html_url) {
            await openUrl(updateInfo.value.html_url)
        }
        showUpdateDialog.value = false
    }

    const dismissDialog = () => {
        showUpdateDialog.value = false
    }

    return {
        updateInfo,
        showUpdateDialog,
        isUpdating,
        isChecking,
        checkForUpdate,
        installUpdate,
        manualDownload,
        dismissDialog
    }
})
