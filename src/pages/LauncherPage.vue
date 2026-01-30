<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
</script>

<template>
  <section class="page">
    <var-card :elevation="false" class="card" :title="t('launcher.title')" :description="t('launcher.desc')" />
  </section>
</template>

<style scoped>
.page {
  padding: 24px;
}

.card {
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.85);
  border: 1px solid rgba(0, 0, 0, 0.06);
}
</style>
