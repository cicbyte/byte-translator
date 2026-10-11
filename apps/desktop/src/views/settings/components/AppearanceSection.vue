<script setup lang="ts">
/** 外观设置：5 套预设色卡 + 明暗切换（即时生效，localStorage 持久化） */
import { useTheme } from "@/themes";
import { PRESETS, PRESET_LIST } from "@/themes/presets";

const theme = useTheme();

function swatches(key: string) {
  const p = PRESETS[key as keyof typeof PRESETS].palette.light;
  return { primary: p.primary, bg: p.bgBase, text: p.textPrimary };
}
</script>

<template>
  <section class="setting-section">
    <header class="section-head">
      <div>
        <h3 class="section-title">外观</h3>
        <p class="section-desc">预设配色与明暗（顶栏也有快捷切换）</p>
      </div>
      <a-segmented
        :value="theme.appearance.value"
        :options="[
          { value: 'light', label: '浅色' },
          { value: 'dark', label: '深色' },
        ]"
        @change="(v: any) => theme.setAppearance(v)"
      />
    </header>

    <div class="preset-grid">
      <button
        v-for="p in PRESET_LIST"
        :key="p.key"
        class="preset-card"
        :class="{ active: theme.preset.value === p.key }"
        @click="theme.setPreset(p.key)"
      >
        <span class="preset-swatches">
          <i class="swatch" :style="{ background: swatches(p.key).primary }" />
          <i class="swatch" :style="{ background: swatches(p.key).bg }" />
          <i class="swatch" :style="{ background: swatches(p.key).text }" />
        </span>
        <span class="preset-name">{{ p.label }}</span>
      </button>
    </div>
  </section>
</template>

<style scoped>
.preset-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(96px, 1fr));
  gap: 8px;
  margin-top: 4px;
}
.preset-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 12px 8px 10px;
  border: 1px solid var(--border-light);
  border-radius: 10px;
  background: var(--bg-input);
  cursor: pointer;
  transition: all 0.15s ease;
}
.preset-card:hover {
  border-color: var(--border-base);
}
.preset-card.active {
  border-color: var(--color-primary);
  background: color-mix(in srgb, var(--color-primary) 8%, transparent);
}
.preset-swatches {
  display: flex;
  gap: 3px;
}
.swatch {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  border: 1px solid rgba(0, 0, 0, 0.15);
}
.preset-name {
  font-size: 12px;
  color: var(--text-secondary);
}
.preset-card.active .preset-name {
  color: var(--color-primary);
  font-weight: 600;
}
</style>
