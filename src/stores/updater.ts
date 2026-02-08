import { defineStore } from "pinia";
import { ref } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Snackbar } from "@varlet/ui";
import * as semver from "semver";
import i18n from "../i18n";
import { downloadAndApplyUpdate, fetchLatestPrerelease, fetchLatestRelease, getAppVersion } from "../api/tauriCommands";

export type LatestRelease = {
  tag_name: string;
  name?: string;
  html_url?: string;
  download_url?: string;
  body?: string;
};

type UpdateTarget = "primary" | "alt";

const normalizeVersion = (v: string) => v.replace(/^v/i, "").trim();

const parseSemver = (v: string) => {
  const normalized = normalizeVersion(v);
  const valid = semver.valid(normalized, { loose: true }) ?? semver.clean(normalized, { loose: true });
  if (!valid) return null;
  return semver.parse(valid, { loose: true });
};

const isHexLowerLike = (s: unknown) => typeof s === "string" && /^[0-9a-f]+$/i.test(s);

const parsePreHexTimestamp = (v: semver.SemVer) => {
  const pre = v.prerelease;
  if (pre.length < 2) return null;
  if (pre[0] !== "pre") return null;
  const ts = pre[1];
  if (!isHexLowerLike(ts)) return null;
  try {
    return BigInt(`0x${String(ts)}`);
  } catch {
    return null;
  }
};

export const useUpdaterStore = defineStore("updater", () => {
  const localVersion = ref<string>("");
  const updateInfo = ref<LatestRelease | null>(null);
  const altUpdateInfo = ref<LatestRelease | null>(null);
  const showUpdateDialog = ref(false);
  const isUpdating = ref(false);
  const isChecking = ref(false);

  const isRemoteNewer = (local: string, remote: string) => {
    const localParsed = parseSemver(local);
    const remoteParsed = parseSemver(remote);
    if (!localParsed || !remoteParsed) return false;

    if (
      localParsed.major === remoteParsed.major &&
      localParsed.minor === remoteParsed.minor &&
      localParsed.patch === remoteParsed.patch
    ) {
      const localTs = parsePreHexTimestamp(localParsed);
      const remoteTs = parsePreHexTimestamp(remoteParsed);
      if (localTs !== null && remoteTs !== null) {
        if (remoteTs > localTs) return true;
        if (remoteTs < localTs) return false;
      }
    }

    return semver.compare(remoteParsed, localParsed) > 0;
  };

  const checkForUpdate = async (silent = false) => {
    if (isChecking.value) return;
    isChecking.value = true;

    showUpdateDialog.value = false;
    updateInfo.value = null;
    altUpdateInfo.value = null;

    try {
      localVersion.value = await getAppVersion();
      const isPreviewBuild = normalizeVersion(localVersion.value).toLowerCase().includes("-pre");

      if (isPreviewBuild) {
        const [stableRes, preRes] = await Promise.allSettled([
          fetchLatestRelease<LatestRelease>(),
          fetchLatestPrerelease<LatestRelease>(),
        ]);

        const stable = stableRes.status === "fulfilled" ? stableRes.value : null;
        const prerelease = preRes.status === "fulfilled" ? preRes.value : null;

        const canUpdateStable = !!stable && isRemoteNewer(localVersion.value, stable.tag_name);
        const canUpdatePre = !!prerelease && isRemoteNewer(localVersion.value, prerelease.tag_name);

        if (canUpdateStable && canUpdatePre && stable && prerelease) {
          updateInfo.value = stable;
          altUpdateInfo.value = prerelease;
          showUpdateDialog.value = true;
          return;
        }

        const target = (canUpdateStable ? stable : null) ?? (canUpdatePre ? prerelease : null);
        if (target) {
          updateInfo.value = target;
          showUpdateDialog.value = true;
          return;
        }

        if (!silent) {
          Snackbar.success(i18n.global.t("settings.update.alreadyLatest") || "Already latest version");
        }
        return;
      }

      const release = await fetchLatestRelease<LatestRelease>();
      if (release && isRemoteNewer(localVersion.value, release.tag_name)) {
        updateInfo.value = release;
        showUpdateDialog.value = true;
      } else if (!silent) {
        Snackbar.success(i18n.global.t("settings.update.alreadyLatest") || "Already latest version");
      }
    } catch (error) {
      console.error("Failed to check for updates:", error);
      if (!silent) {
        Snackbar.error(i18n.global.t("settings.update.checkFailed") || "Check failed");
      }
    } finally {
      isChecking.value = false;
    }
  };

  const installUpdate = async (target: UpdateTarget = "primary") => {
    const info = target === "alt" ? altUpdateInfo.value : updateInfo.value;
    if (!info?.download_url) {
      Snackbar.error(i18n.global.t("settings.update.installFailed") || "Install failed");
      return;
    }

    isUpdating.value = true;
    try {
      await downloadAndApplyUpdate(info.download_url);
    } catch (error) {
      console.error("Update failed:", error);
      Snackbar.error(i18n.global.t("settings.update.installFailed") || "Install failed");
      isUpdating.value = false;
    }
  };

  const manualDownload = async (target: UpdateTarget = "primary") => {
    const info = target === "alt" ? altUpdateInfo.value : updateInfo.value;
    if (info?.html_url) {
      await openUrl(info.html_url);
    }
    showUpdateDialog.value = false;
  };

  const dismissDialog = () => {
    showUpdateDialog.value = false;
  };

  return {
    localVersion,
    updateInfo,
    altUpdateInfo,
    showUpdateDialog,
    isUpdating,
    isChecking,
    checkForUpdate,
    installUpdate,
    manualDownload,
    dismissDialog,
  };
});
