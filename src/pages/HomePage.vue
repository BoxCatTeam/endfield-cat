<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import logo from '../assets/icon.webp'

const { t } = useI18n()
</script>

<template>
  <var-space align="center" justify="center" class="home">
    <var-space direction="column" align="center">
      <var-image :src="logo" width="128"></var-image>
        <div class="logo">
          <span class="logo-end">End</span><span class="logo-cat">Cat</span>
        </div>
        <div class="subtitle">{{ t('home.subtitle') }}</div>
    </var-space>
  </var-space>
</template>

<style scoped>
.home{
  height: 85%;
}
.logo {
  font-size: 88px;
  line-height: 1;
}

.logo-end {
  color: var(--color-logo-text);
}

.logo-cat {
  color: var(--color-logo-accent);
}

.subtitle {
  margin-top: 10px;
  color: var(--color-on-surface-variant, #666);
  font-size: 18px;
}
</style>
