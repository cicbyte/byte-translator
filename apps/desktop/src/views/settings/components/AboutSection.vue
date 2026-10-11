<script setup lang="ts">
/** 关于：版本与形态说明 */
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

const version = ref("");

onMounted(async () => {
  const info = await invoke<{ version: string }>("app_info");
  version.value = info.version;
});
</script>

<template>
  <section class="setting-section">
    <header class="section-head">
      <div>
        <h3 class="section-title">关于</h3>
        <p class="section-desc">本地优先的翻译工具——CLI、桌面端、REST、MCP 同一核心</p>
      </div>
      <div class="logo-mark">译</div>
    </header>
    <div class="about-row">
      <span class="about-label">版本</span>
      <span class="about-value">v{{ version }}</span>
    </div>
    <div class="about-row">
      <span class="about-label">许可</span>
      <span class="about-value">MIT</span>
    </div>
    <div class="about-row">
      <span class="about-label">服务</span>
      <span class="about-value">REST 见「服务 · REST API」；MCP 见「服务 · MCP 服务」</span>
    </div>
  </section>
</template>

<style scoped>
.logo-mark {
  width: 34px;
  height: 34px;
  border-radius: 10px;
  background: var(--color-primary);
  color: var(--hero-cta-text);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 16px;
  font-weight: 700;
}
.about-row {
  display: flex;
  gap: 16px;
  padding: 8px 0;
}
.about-row + .about-row {
  border-top: 1px solid var(--border-light);
}
.about-label {
  width: 48px;
  flex-shrink: 0;
  font-size: 12.5px;
  color: var(--text-tertiary);
}
.about-value {
  font-size: 12.5px;
  color: var(--text-secondary);
}
</style>
