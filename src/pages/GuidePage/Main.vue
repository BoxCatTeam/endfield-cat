<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'

const route = useRoute()
const { t } = useI18n()

// 路由名称与步骤编号映射
const stepMap: Record<string, number> = {
  'guide-welcome': 0,
  'guide-disclaimer': 1,
  'guide-resource': 2,
  'guide-ready': 3
}

const step = ref(0)

// 根据路由同步步骤高亮
watch(
  () => route.name,
  (newRouteName) => {
    const s = stepMap[newRouteName as string]
    if (s !== undefined) {
      step.value = s
    }
  },
  { immediate: true }
)

onMounted(() => {
  // 额外跳转逻辑由路由重定向处理，此处无需动作
})
</script>

<template>
  <div class="guide-page-bg">
    <div class="guide-container">
      <div class="guide-content-wrapper">
          
        <var-steps :active="step" class="steps-header" active-color="var(--color-primary)">
          <var-step>{{ t('guide.stepWelcome') }}</var-step>
          <var-step>{{ t('guide.stepDisclaimer') }}</var-step>
          <var-step>{{ t('guide.stepResource') }}</var-step>
          <var-step>{{ t('guide.stepFinish') }}</var-step>
        </var-steps>

        <!-- 带过渡的子路由区域 -->
        <router-view v-slot="{ Component }">
          <transition name="fade-slide" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>

      </div>
    </div>
  </div>
</template>

<style scoped>
.guide-content-wrapper{
  margin-top: -25px;
  margin-bottom: 50px;
}

.guide-page-bg {
  height: 100vh;
  width: 100vw;
  background-image: linear-gradient(135deg, #fdfbfb 0%, #ebedee 100%);
  background-size: cover;
  position: relative;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
}

/* 深色模式的背景覆盖 */
@media (prefers-color-scheme: dark) {
  .guide-page-bg {
    background-image: linear-gradient(135deg, #1a1a1a 0%, #0d0d0d 100%);
  }
}

.guide-container {
  width: 90%;
  max-width: 600px;
  z-index: 2;
  margin: auto;
  padding: 40px 0;
}

.steps-header {
  margin-bottom: 24px;
}

/* 过渡动画 */
.fade-slide-enter-active,
.fade-slide-leave-active {
  transition: all 0.3s ease;
}

.fade-slide-enter-from {
  opacity: 0;
  transform: translateY(10px);
}

.fade-slide-leave-to {
  opacity: 0;
  transform: translateY(-10px);
}
</style>
