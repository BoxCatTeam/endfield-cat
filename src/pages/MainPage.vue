
<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, computed } from "vue";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

// 常量配置
const SIDEBAR_EXPANDED_WIDTH = 230;
const CONTENT_MIN_WIDTH = 720; 

const COLLAPSE_THRESHOLD = CONTENT_MIN_WIDTH + SIDEBAR_EXPANDED_WIDTH; // 约 1110px

const windowWidth = ref(window.innerWidth);
const isCollapsed = computed(() => windowWidth.value < COLLAPSE_THRESHOLD);

function onResize() {
  windowWidth.value = window.innerWidth;
}

onMounted(async () => {
  window.addEventListener("resize", onResize);
  
  // 限制窗口最小尺寸
  try {
    const appWindow = getCurrentWindow();
    // 使用 LogicalSize 与 CSS 像素保持一致
    await appWindow.setMinSize(new LogicalSize(675, 475));
  } catch (e) {
    console.error("Failed to set min window size", e);
  }
});

onBeforeUnmount(() => {
  window.removeEventListener("resize", onResize);
});
</script>

<template>
  <div class="layout">
    <div class="shell">
      <aside class="sidebar" :class="{ collapsed: isCollapsed }">
        <nav class="nav">
          <router-link to="/" class="nav-link" exact-active-class="active">
            <var-cell ripple class="nav-item">
              <template #icon>
                <var-icon name="home" size="24" />
              </template>
              <template #default>
                <div class="nav-title" v-show="!isCollapsed">{{ t('nav.home') }}</div>
              </template>
            </var-cell>
          </router-link>
          <router-link to="/launcher" class="nav-link" active-class="active">
             <var-cell ripple class="nav-item">
              <template #icon>
                <var-icon name="play-circle-outline" size="24" />
              </template>
              <template #default>
                <div class="nav-title" v-show="!isCollapsed">{{ t('nav.launcher') }}</div>
              </template>
            </var-cell>
          </router-link>
        <router-link to="/gacha" class="nav-link" active-class="active">
           <var-cell ripple class="nav-item">
              <template #icon>
                <var-icon name="star-outline" size="24" />
              </template>
              <template #default>
                <div class="nav-title" v-show="!isCollapsed">{{ t('nav.gacha') }}</div>
              </template>
            </var-cell>
        </router-link>
        </nav>

        <div class="spacer" />

        <div class="footer">
          <router-link to="/settings" class="nav-link" active-class="active">
            <var-cell ripple class="nav-item">
              <template #icon>
                <var-icon name="cog-outline" size="24" />
              </template>
              <template #default>
                <div class="nav-title" v-show="!isCollapsed">{{ t('nav.settings') }}</div>
              </template>
            </var-cell>
          </router-link>
        </div>
      </aside>

      <section class="main">
        <main class="content">
          <router-view />
        </main>
      </section>
    </div>
  </div>
</template>

<style scoped>
.layout {
  height: 100%;
  padding: 0;
  box-sizing: border-box;
}

.shell {
  height: 100%;
  display: flex;
  background: transparent;
  border: none;
}

.sidebar {
  width: 230px;
  padding: 10px;
  display: flex;
  flex-direction: column;
  box-sizing: border-box;
  transition: width 0.25s cubic-bezier(0.4, 0, 0.2, 1);

}

.sidebar.collapsed {
  width: 80px;
}

.nav-title {
  margin-left: 12px;
  white-space: nowrap;
  font-weight: 500;
  opacity: 1;
  transition: opacity 0.2s;
}

/* 折叠状态下让图标水平居中 */
.sidebar.collapsed :deep(.var-cell__content) {
  justify-content: center;
  flex: 0 0 auto; /* 防止内容被拉伸 */
}

.sidebar.collapsed :deep(.var-cell) {
  padding-left: 0 !important;
  padding-right: 0 !important;
  justify-content: center;
}

/* 槽位下移除图标左右边距 */
.sidebar.collapsed :deep(.var-cell__icon) {
  margin-right: 0 !important;
  width: 100%;
  display: flex;
  justify-content: center;
}

.nav {
  display: grid;
  gap: 6px;
  padding-top: 4px;
}

.nav-link {
  text-decoration: none;
  color: inherit;
  border-radius: 10px;
  overflow: hidden;
}

.nav-link :deep(.var-cell) {
  border-radius: 10px;
  background: transparent;
  transition: background-color 0.15s ease;
}

.nav-link:hover :deep(.var-cell) {
  background: rgba(0, 0, 0, 0.03);
}

.nav-link.active :deep(.var-cell) {
  background: var(--color-primary-container, rgba(227, 61, 111, 0.12));
}

.nav-link.active :deep(.var-cell__title),
.nav-link.active :deep(.var-cell__description),
.nav-link.active :deep(.var-cell__icon) {
  color: var(--color-on-primary-container, var(--color-primary, #e33d6f));
}

.spacer {
  flex: 1;
}

.footer {
  padding-top: 8px;
  display: grid;
  gap: 6px;
}

.main {
  flex: 1;
  display: flex;
  min-width: 0;
}

.content {
  flex: 1;
  background: var(--color-content-background);
  border-top: 1px solid var(--color-content-border);
  border-left: 1px solid var(--color-content-border);
  border-radius: 16px 0 0 0;
  margin: 0;
  /* 内容区域不滚动，交由子页面自行处理，保持圆角裁切 */
  overflow: hidden;
}




</style>
