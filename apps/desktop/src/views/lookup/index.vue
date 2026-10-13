<script setup lang="ts">
/**
 * 划词小窗：接收 lookup-text 事件自动翻译（默认引擎策略），紧凑呈现。
 * Esc / 失焦自动隐藏（不销毁，复用窗口）。
 */
import { onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

interface TranslationResult {
  engine: string;
  text: string;
  translation: string;
  phonetic: string[];
  parts: { pos: string; mean: string }[];
  examples: { orig: string; trans: string }[];
}

const source = ref("");
const result = ref<TranslationResult | null>(null);
const fallbackFrom = ref<string | null>(null);
const loading = ref(false);
const error = ref("");

const win = getCurrentWindow();
let unlisten: UnlistenFn | null = null;
let unblur: UnlistenFn | null = null;

async function translateText(text: string) {
  source.value = text;
  loading.value = true;
  result.value = null;
  fallbackFrom.value = null;
  error.value = "";
  try {
    const outcome = await invoke<{ result: TranslationResult; fallback_from: string | null }>("translate", {
      text,
      engine: null,
      from: null,
      to: null,
    });
    result.value = outcome.result;
    fallbackFrom.value = outcome.fallback_from;
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function hide() {
  await win.hide();
}

function onKey(e: KeyboardEvent) {
  if (e.key === "Escape") hide();
}

onMounted(async () => {
  unlisten = await listen<string>("lookup-text", (ev) => {
    if (ev.payload.trim()) translateText(ev.payload);
    else {
      source.value = "";
      result.value = null;
      error.value = "";
    }
  });
  unblur = await win.onFocusChanged((ev) => {
    if (!ev.payload) hide();
  });
  window.addEventListener("keydown", onKey);
});
onUnmounted(() => {
  unlisten?.();
  unblur?.();
  window.removeEventListener("keydown", onKey);
});
</script>

<template>
  <div class="lookup" data-tauri-drag-region>
    <div class="head">
      <span class="src" :title="source">{{ source }}</span>
      <span v-if="result" class="chip">{{ result.engine }}</span>
    </div>
    <div class="body">
      <div v-if="loading" class="loading"><a-spin size="small" /></div>
      <div v-else-if="error" class="error">{{ error }}</div>
      <template v-else-if="result">
        <div v-if="fallbackFrom" class="fallback">⚠️ {{ fallbackFrom }} 未命中，已自动降级</div>
        <template v-if="result.parts.length || result.examples.length">
          <div v-if="result.phonetic.length" class="phonetic">[{{ result.phonetic.join("] [") }}]</div>
          <div v-for="(p, i) in result.parts" :key="i" class="part">
            <span class="pos">{{ p.pos }}</span>{{ p.mean }}
          </div>
          <div v-for="(ex, i) in result.examples.slice(0, 2)" :key="'e' + i" class="ex">
            {{ ex.orig }}
          </div>
        </template>
        <div v-else class="plain">{{ result.translation }}</div>
      </template>
      <div v-else class="placeholder">未取到选中文本——选中文字后再按，或在主窗口粘贴翻译</div>
    </div>
  </div>
</template>

<style scoped>
.lookup {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--bg-elevated);
  border: 1px solid var(--border-base);
  border-radius: 12px;
  overflow: hidden;
}
.head {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-bottom: 1px solid var(--border-light);
  cursor: default;
}
.src {
  flex: 1;
  min-width: 0;
  font-size: 12px;
  color: var(--text-tertiary);
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.chip {
  flex-shrink: 0;
  font-size: 10.5px;
  padding: 1px 8px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-primary) 10%, transparent);
  color: var(--color-primary);
}
.body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 12px 14px;
}
.loading {
  padding: 24px 0;
  text-align: center;
}
.error {
  font-size: 12px;
  color: var(--color-danger);
  line-height: 1.6;
  word-break: break-all;
}
.fallback {
  font-size: 11px;
  color: var(--color-warning);
  margin-bottom: 8px;
}
.phonetic {
  font-size: 12px;
  color: var(--text-tertiary);
  margin-bottom: 6px;
}
.part {
  font-size: 13px;
  line-height: 1.7;
  color: var(--text-primary);
}
.pos {
  color: var(--color-primary);
  margin-right: 6px;
  font-weight: 500;
}
.ex {
  margin-top: 8px;
  font-size: 11.5px;
  color: var(--text-tertiary);
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.plain {
  font-size: 14px;
  line-height: 1.7;
  color: var(--text-primary);
  white-space: pre-wrap;
}
.placeholder {
  padding: 24px 0;
  text-align: center;
  font-size: 12px;
  color: var(--text-tertiary);
}
</style>
