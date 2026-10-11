<script setup lang="ts">
/** 偏好设置：默认引擎策略 + 词典 Key（保存后即时生效） */
import { onMounted, ref } from "vue";
import { message } from "ant-design-vue";
import { invoke } from "@tauri-apps/api/core";

const PREFS_KEY = "btr-prefs";

const defaultEngine = ref("auto");
const icibaKey = ref("");
const keySaving = ref(false);

const ENGINE_OPTIONS = [
  { value: "auto", label: "自动调度（词查词典、句走 LLM，未命中降级）" },
  { value: "llm", label: "始终 LLM" },
  { value: "iciba", label: "始终词典（仅中英）" },
];

function loadPrefs() {
  try {
    const v = JSON.parse(localStorage.getItem(PREFS_KEY) || "{}");
    if (v.defaultEngine) defaultEngine.value = v.defaultEngine;
    if (typeof v.icibaKey === "string") icibaKey.value = v.icibaKey;
  } catch {
    /* ignore */
  }
}

function savePrefs() {
  localStorage.setItem(
    PREFS_KEY,
    JSON.stringify({ defaultEngine: defaultEngine.value, icibaKey: icibaKey.value }),
  );
}

async function applyIcibaKey() {
  keySaving.value = true;
  try {
    await invoke("set_iciba_key", { key: icibaKey.value });
    savePrefs();
    message.success("词典 Key 已生效");
  } finally {
    keySaving.value = false;
  }
}

function onEngineChange() {
  savePrefs();
  message.success("默认引擎已保存（翻译页下次进入生效）");
}

onMounted(loadPrefs);
</script>

<template>
  <section class="setting-section">
    <header class="section-head">
      <div>
        <h3 class="section-title">偏好</h3>
        <p class="section-desc">翻译默认策略；翻译页可临时覆盖</p>
      </div>
    </header>

    <div class="pref-row">
      <div class="pref-label">默认引擎</div>
      <a-select v-model:value="defaultEngine" style="width: 320px" @change="onEngineChange">
        <a-select-option v-for="o in ENGINE_OPTIONS" :key="o.value" :value="o.value">
          {{ o.label }}
        </a-select-option>
      </a-select>
    </div>

    <div class="pref-row">
      <div>
        <div class="pref-label">词典引擎 Key（iciba）</div>
        <div class="pref-hint">
          到 open.iciba.com 免费申请；不填时用内置公共 Key（可能失效，未命中自动走 LLM）
        </div>
      </div>
      <div class="key-apply">
        <a-input-password
          v-model:value="icibaKey"
          placeholder="留空使用公共 Key"
          style="width: 280px"
        />
        <a-button :loading="keySaving" @click="applyIcibaKey">应用</a-button>
      </div>
    </div>
  </section>
</template>

<style scoped>
.pref-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 10px 0;
}
.pref-row + .pref-row {
  border-top: 1px solid var(--border-light);
}
.pref-label {
  font-size: 13px;
  color: var(--text-primary);
}
.pref-hint {
  margin-top: 3px;
  font-size: 11.5px;
  color: var(--text-tertiary);
}
.key-apply {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}
</style>
