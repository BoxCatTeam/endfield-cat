<script setup lang="ts">
import "../../charts/echarts";

import { computed, onMounted, ref } from "vue";
import VChart from "vue-echarts";
import { useI18n } from 'vue-i18n';

type BannerStats = {
  s6: number;
  s5: number;
  s4: number;
  s3: number;
};

const props = defineProps<{
  stats: BannerStats;
  range?: [string, string];
  size?: number;
}>();

const root = ref<HTMLElement | null>(null);

function readCssVar(el: HTMLElement, name: string) {
  return getComputedStyle(el).getPropertyValue(name).trim();
}

const colors = ref({
  s6: "#ff6a3d",
  s5: "#b534d2",
  s4: "#2aa84a",
  s3: "#61caf7",
});

onMounted(() => {
  if (!root.value) return;
  const s6 = readCssVar(root.value, "--rarity-6");
  const s5 = readCssVar(root.value, "--rarity-5");
  const s4 = readCssVar(root.value, "--rarity-4");
  const s3 = readCssVar(root.value, "--rarity-3");

  colors.value = {
    s6: s6 || colors.value.s6,
    s5: s5 || colors.value.s5,
    s4: s4 || colors.value.s4,
    s3: s3 || colors.value.s3,
  };
});

const { t } = useI18n();

const option = computed(() => ({
  animation: false,
  tooltip: {
    trigger: "item",
    borderWidth: 0,
    backgroundColor: "rgba(0, 0, 0, 0.72)",
    textStyle: { color: "#fff" },
    formatter: "{b}<br/>{c} ({d}%)",
  },
  series: [
    {
      type: "pie",
      radius: ["46%", "82%"],
      center: ["50%", "50%"],
      silent: true,
      label: { show: false },
      labelLine: { show: false },
      data: [
        { value: props.stats.s6, name: t('gacha.stats.s6'), itemStyle: { color: colors.value.s6 } },
        { value: props.stats.s5, name: t('gacha.stats.s5'), itemStyle: { color: colors.value.s5 } },
        { value: props.stats.s4, name: t('gacha.stats.s4'), itemStyle: { color: colors.value.s4 } },

      ],
    },
  ],
}));

const sizeStyle = computed(() => {
  const size = props.size ?? 180;
  return { width: `${size}px`, height: `${size}px` };
});
</script>

<template>
  <div ref="root" class="root" :style="sizeStyle">
    <VChart class="chart" :option="option" autoresize :style="sizeStyle" />
  </div>
</template>

<style scoped>
.root {
  display: grid;
  place-items: center;
}
</style>
