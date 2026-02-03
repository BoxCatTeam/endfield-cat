<script setup lang="ts">
import { computed } from "vue";
import DonutChart from "../charts/DonutChart.vue";
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

export type BannerStats = {
  s6: number;
  s5: number;
  s4: number;
  s3: number;
};

export type BannerItem = {
  id: string;
  title: string;
  range: string;
  topLabel?: string;
  stats: BannerStats;
  guarantee: number;
  avg6: number;
  min6: number;
  max6: number;
  top: Array<{ name: string; count: number; rarity: 6 | 5 | 4 | 3; icon?: string; featured?: boolean }>;
  total?: number; // 总数若未传入则在组件内计算
};

const props = defineProps<{
  banner: BannerItem;
  donutSize?: number;
}>();

const actualDonutSize = computed(() => props.donutSize ?? 200);

const total = computed(() => {
  if (typeof props.banner.total === "number") return props.banner.total;
  const s = props.banner.stats;
  return s.s6 + s.s5 + s.s4 + (s.s3 || 0);
});
</script>

<template>
  <div class="banner" :style="{ '--donut-size': actualDonutSize + 'px' }">
    <div class="banner-body">
      <var-space align="center" justify="center" direction="column" class="chart">
        <DonutChart class="donut" :stats="banner.stats" :size="actualDonutSize" />
        <div class="chart-range">{{ banner.range }}</div>
      </var-space>

      <var-space direction="column" :size="12" class="stats" :wrap="false">
        <div class="chips">
          <var-chip plain color="var(--rarity-6)" text-color="var(--rarity-6)">
            {{ banner.stats.s6 }} {{ t('gacha.stats.s6') }}
          </var-chip>
          <var-chip plain color="var(--rarity-5)" text-color="var(--rarity-5)">
            {{ banner.stats.s5 }} {{ t('gacha.stats.s5') }}
          </var-chip>
          <var-chip plain color="var(--rarity-4)" text-color="var(--rarity-4)">
            {{ banner.stats.s4 }} {{ t('gacha.stats.s4') }}
          </var-chip>
        </div>

        <var-space direction="column" :size="6" class="kv" :wrap="false">
          <var-space class="row" justify="space-between" align="center" :wrap="false" :size="12">
            <div class="k">{{ t('gacha.stats.total') }}</div>
            <div class="v">{{ total }} {{ t('gacha.stats.pulls') }}</div>
          </var-space>
          <var-space class="row" justify="space-between" align="center" :wrap="false" :size="12">
            <div class="k">{{ t('gacha.stats.pity') }}</div>
            <div class="v">{{ banner.guarantee }} {{ t('gacha.stats.pulls') }}</div>
          </var-space>
          <var-space class="row" justify="space-between" align="center" :wrap="false" :size="12">
            <div class="k">{{ t('gacha.stats.avg6') }}</div>
            <div class="v">{{ banner.avg6 }} {{ t('gacha.stats.pulls') }}</div>
          </var-space>
          <var-space class="row" justify="space-between" align="center" :wrap="false" :size="12">
            <div class="k">{{ t('gacha.stats.min6') }}</div>
            <div class="v">{{ banner.min6 }} {{ t('gacha.stats.pulls') }}</div>
          </var-space>
          <var-space class="row" justify="space-between" align="center" :wrap="false" :size="12">
            <div class="k">{{ t('gacha.stats.max6') }}</div>
            <div class="v">{{ banner.max6 }} {{ t('gacha.stats.pulls') }}</div>
          </var-space>
        </var-space>
      </var-space>

      <div class="top">
        <var-space direction="column" class="top-list" :wrap="false">
          <var-cell
            v-for="item in banner.top"
            :key="item.name"
            class="top-item"
          >
            <template #icon>
              <div class="top-icon">
                <var-avatar size="32" hoverable :src="item.icon" :alt="item.name">
                  {{ item.name.slice(0, 1) }}
                </var-avatar>
              </div>
            </template>
            <template #default>
              <var-space justify="space-between" class="top-name">
                <div class="op-name" :data-rarity="item.rarity">{{ item.name }}</div>
                <var-space justify="space-between" align="center" direction="row">
                  <var-icon
                      v-if="item.featured"
                      name="star"
                      size="20"
                      class="featured-icon"
                  />
                  <div class="count">{{ item.count }}</div>
                </var-space>
              </var-space>
            </template>
          </var-cell>
        </var-space>
      </div>
    </div>
  </div>
</template>

<style scoped>
.banner {
  padding: 14px;
  --rarity-6: #F99446;
  --rarity-5: #FAC946;
  --rarity-4: #B182F2;
  --rarity-3: #61caf7;
  --banner-row-height: calc(var(--donut-size, 200px) + 62px);
  background: var(--color-record-item-bg);
  margin-top: 2px;
}
.top-name{
  padding-left: 10px;
  padding-right: 5px;
}
.banner-body {
  padding-top: 0;
  display: grid;
  grid-template-columns: 250px 1fr minmax(150px, 340px);
  gap: 14px;
  align-items: stretch;
  height: var(--banner-row-height);
}

.chart {
  row-gap: 10px;
  padding: 10px;
  border-right: 1px solid var(--color-border-subtle);
  height: 100%;
}

.donut {
  border-radius: 50%;
  overflow: hidden;
}

.chart-range {
  font-size: 12px;
  color: var(--color-on-surface-variant);
}

.stats {
  padding: 4px 8px;
  height: 100%;
  overflow: hidden;
  min-width: 220px;
}

.chips {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}

.row {
  width: 100%;
}

.k {
  color: var(--color-on-surface-variant);
}

.v {
  font-weight: 600;
}

.top {
  border-left: 1px solid var(--color-border-subtle);
  padding-left: 14px;
  height: 100%;
  overflow: hidden;
  display: grid;
  grid-template-rows: auto 1fr;
}

.top-list {
  /* 使用 var-space 代替 display:flex */
  overflow-x: hidden;
  height: 100%;
  /* var-space 已处理布局，这里无需 align-content */
  scrollbar-width: thin;
  scrollbar-color: var(--color-scrollbar-thumb) transparent;
  padding: 5px;
}

.top-list::-webkit-scrollbar {
  width: 8px;
}

.top-list::-webkit-scrollbar-track {
  background: transparent;
}

.top-list::-webkit-scrollbar-thumb {
  background-color: var(--color-scrollbar-thumb);
  border-radius: 999px;
  border: 2px solid transparent;
  background-clip: content-box;
}

.top-list::-webkit-scrollbar-thumb:hover {
  background-color: var(--color-scrollbar-thumb-hover);
}

.top-item {
  padding: 8px 5px;
  border-radius: 5px;
  min-height: 40px;
  background: linear-gradient(var(--color-card-bg), var(--color-card-bg)), var(--color-body);
}

.top-icon {
  display: flex;
  align-items: center;
  gap: 6px;
}

.featured-icon {
  color: var(--rarity-6);
  font-size: 18px;
}

.op-name {
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 16px;
}

.op-name[data-rarity="6"] {
  color: var(--rarity-6);
}
.op-name[data-rarity="5"] {
  color: var(--rarity-5);
}
.op-name[data-rarity="4"] {
  color: var(--rarity-4);
}

.count {
  text-align: right;
  font-size: 16px;
  width: 20px;
  flex-shrink: 0;
  margin-left: 8px;
}

@media (max-width: 800px) {
  .banner-body {
    grid-template-columns: 220px 1fr;
    height: auto;
  }
  .top {
    grid-column: 1 / -1;
    border-left: none;
    padding-left: 0;
    border-top: 1px solid var(--color-border-subtle);
    padding-top: 12px;
  }
}
</style>
