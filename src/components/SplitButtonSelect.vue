<script setup lang="ts">
import { computed } from 'vue'

interface Option {
  label: string
  value: any
}

const props = withDefaults(defineProps<{
  modelValue?: any
  options: Option[]
  placeholder?: string
  size?: 'normal' | 'mini' | 'small' | 'large'
  type?: 'default' | 'primary' | 'info' | 'success' | 'warning' | 'danger'
  mode?: 'normal' | 'outline' | 'text'
  disabled?: boolean
}>(), {
  modelValue: undefined,
  options: () => [],
  size: 'small',
  type: 'primary',
  mode: 'normal',
  disabled: false
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: any): void
}>()

const currentLabel = computed(() => {
  const found = props.options.find(opt => opt.value === props.modelValue)
  return found?.label || props.placeholder || ''
})

const handleSelect = (value: any) => {
  if (value !== props.modelValue) {
    emit('update:modelValue', value)
  }
}
</script>

<template>
  <var-menu placement="bottom-end" :offset-y="4" :disabled="disabled" >
    <var-button-group :type="type" :mode="mode" :size="size" :elevation="0">
      <var-button :disabled="disabled">
        {{ currentLabel }}
      </var-button>
      <var-button :disabled="disabled" style="padding: 0 6px;">
        <var-icon name="chevron-down" />
      </var-button>
    </var-button-group>

    <template #menu>
      <var-cell
        v-for="opt in options"
        :key="opt.value"
        ripple
        @click="handleSelect(opt.value)"
      >
        {{ opt.label }}
      </var-cell>
    </template>
  </var-menu>
</template>

<style scoped>
/* Optional: Adjustments if needed to match specific design requirements */
</style>
