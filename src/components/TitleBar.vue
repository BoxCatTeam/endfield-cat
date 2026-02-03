<script setup lang="ts">
import { computed } from "vue";
import { useRoute } from "vue-router";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { quitApp } from "../api/tauriCommands";
import logo from "../assets/icon.webp"

const route = useRoute();

const title = computed(() => (route.meta?.title as string | undefined) ?? "");

const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in (window as any);

async function startDragging() {
  if (!isTauri) return;
  await getCurrentWindow().startDragging();
}

async function tryStartDragging(ev: PointerEvent) {
  if (!isTauri) return;
  if (ev.button !== 0) return;

  const target = ev.target as HTMLElement | null;
  if (target?.closest?.('[data-tauri-drag-region="false"]')) return;
  if (target?.closest?.("button, a, input, textarea, select, [role='button']")) return;

  await startDragging();
}

async function windowAction(action: "minimize" | "toggleMaximize" | "close") {
  if (!isTauri) return;
  const win = getCurrentWindow();

  if (action === "minimize") await win.minimize();
  if (action === "toggleMaximize") await win.toggleMaximize();
  if (action === "close") {
    try {
      await win.close();
    } finally {
      await quitApp();
    }
  }
}
</script>

<template>
  <header class="titlebar" data-tauri-drag-region @pointerdown="tryStartDragging">
    <var-space align="center" justify="center">
      <var-avatar :src="logo" :size="32"/>
      <div class="brand">EndCat</div>
      <div class="title">{{ title }}</div>
    </var-space>

    <div class="spacer" />

    <div class="actions" data-tauri-drag-region="false">
      <var-button v-if="isTauri" text class="win-btn" @click="windowAction('minimize')">
        <var-icon name="minus" :size="18" />
      </var-button>
      <var-button v-if="isTauri" text class="win-btn" @click="windowAction('toggleMaximize')">
        <var-icon name="checkbox-blank-outline" :size="18" />
      </var-button>
      <var-button v-if="isTauri" text class="win-btn danger" @click="windowAction('close')">
        <var-icon name="window-close" :size="18" />
      </var-button>
    </div>
  </header>
</template>

<style scoped>
.titlebar {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  height: var(--titlebar-height, 44px);
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 0 0 12px;
  box-sizing: border-box;
  z-index: 1000;
  user-select: none;
  background: var(--color-body, #f6f6f6);
}

.left {
  gap: 10px;
  min-width: 0;
}

.brand {
  font-weight: 750;
  letter-spacing: 0.2px;
}

.title {
  opacity: 0.65;
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 40vw;
}

.spacer {
  flex: 1;
}

.actions {
  display: flex;
  gap: 4px;
  height: 100%;
}

.win-btn {
  width: 46px;
  height: 32px;
  padding: 0;
  min-width: 0;
  border-radius: 0;
  box-sizing: border-box;
}

.win-btn:hover {
  background: rgba(0, 0, 0, 0.04);
}

.win-btn :deep(.var-button__content) {
  width: 100%;
  justify-content: center;
}

.win-btn :deep(.var-icon) {
  line-height: 1;
}

.danger:hover {
  background: rgba(255, 0, 0, 0.08);
}


</style>
