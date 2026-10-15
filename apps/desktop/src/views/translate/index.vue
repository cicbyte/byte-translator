<template>
  <div class="page translate-page">
    <!-- 输入卡片：textarea 融入卡面，底部工具条 -->
    <div class="input-card">
      <textarea
        v-model="text"
        class="input-area"
        placeholder="输入要翻译的词或句子…（Enter 翻译，Shift+Enter 换行）"
        @keydown.enter.exact.prevent="translate"
      />
      <div class="input-toolbar">
        <div class="toolbar-left">
          <a-select v-model:value="fromLang" size="small" :bordered="false" class="lang-select">
            <a-select-option value="">自动检测</a-select-option>
            <a-select-option v-for="l in LANGS" :key="l.code" :value="l.code">{{ l.name }}</a-select-option>
          </a-select>
          <span class="lang-arrow">→</span>
          <a-select v-model:value="toLang" size="small" :bordered="false" class="lang-select">
            <a-select-option value="">自动</a-select-option>
            <a-select-option v-for="l in LANGS" :key="l.code" :value="l.code">{{ l.name }}</a-select-option>
          </a-select>
          <a-divider type="vertical" />
          <a-select v-model:value="engine" size="small" :bordered="false" class="engine-select">
            <a-select-option value="auto">自动调度</a-select-option>
            <a-select-option
              v-for="e in engineOptions"
              :key="e.name"
              :value="e.name"
              :disabled="!e.configured"
            >
              {{ e.label }}{{ e.configured ? "" : "（未配置）" }}
            </a-select-option>
          </a-select>
        </div>
        <a-button v-if="streaming" danger size="small" @click="stopStream">
          <template #icon><StopOutlined /></template>
          停止
        </a-button>
        <a-button v-else type="primary" size="small" :loading="loading" @click="translate">
          <template #icon><SendOutlined /></template>
          翻译
        </a-button>
      </div>
    </div>

    <!-- 结果区 -->
    <div class="page-scroll result-scroll">
      <a-alert
        v-if="fallbackFrom"
        type="warning"
        show-icon
        :message="`${fallbackFrom} 未找到结果，已自动切换到 ${result?.engine} 翻译`"
        style="margin-bottom: 12px"
      />
      <a-alert v-if="error" type="error" show-icon :message="error" style="margin-bottom: 12px" />

      <div v-if="loading && !streaming" class="pending"><a-spin /></div>

      <div v-if="result || streaming" class="result-card">
        <template v-if="isDictionary">
          <div class="word-row">
            <span class="word">{{ result?.text }}</span>
            <span v-for="p in result?.phonetic ?? []" :key="p" class="phonetic">[{{ p }}]</span>
            <span class="engine-chip">词典 · {{ result?.engine }}</span>
          </div>
          <div class="parts">
            <div v-for="(p, i) in result?.parts ?? []" :key="i" class="part-row">
              <span class="part-pos">{{ p.pos }}</span>
              <span class="part-mean">{{ p.mean }}</span>
            </div>
          </div>
          <div v-if="result?.examples.length" class="examples">
            <div v-for="(ex, i) in result?.examples ?? []" :key="i" class="ex-row">
              <div class="ex-orig">{{ ex.orig }}</div>
              <div class="ex-trans">{{ ex.trans }}</div>
            </div>
          </div>
        </template>
        <template v-else>
          <div class="word-row">
            <span class="word">{{ result?.text || text }}</span>
            <span class="engine-chip" :class="{ streaming }">
              {{ streaming ? "流式翻译中…" : result?.engine }}
            </span>
          </div>
          <p class="plain">{{ streamText || result?.translation }}</p>
        </template>
        <div v-if="result" class="result-actions">
          <button class="chip-btn" :disabled="collected || collecting" @click="collect">
            <StarOutlined />
            {{ collected ? "已收藏" : "收藏生词" }}
          </button>
          <button class="chip-btn" @click="copyResult">
            <CopyOutlined />
            复制
          </button>
        </div>
      </div>

      <div v-else-if="!loading" class="empty-hint">
        <div class="hint-icon">译</div>
        <div class="hint-line">单词自动查词典，句子自动走 LLM</div>
        <div class="hint-sub">Ctrl+Enter 也行 · 结果自动存入历史</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { useRoute } from "vue-router";
import { message } from "ant-design-vue";
import { SendOutlined, StopOutlined, StarOutlined, CopyOutlined } from "@ant-design/icons-vue";
import { Channel, invoke } from "@tauri-apps/api/core";

interface TranslationResult {
  engine: string;
  from: string;
  to: string;
  text: string;
  translation: string;
  phonetic: string[];
  parts: { pos: string; mean: string }[];
  examples: { orig: string; trans: string }[];
}

const LANGS = [
  { code: "zh", name: "中文" },
  { code: "en", name: "英语" },
  { code: "ja", name: "日语" },
  { code: "ko", name: "韩语" },
  { code: "fr", name: "法语" },
  { code: "de", name: "德语" },
  { code: "es", name: "西语" },
  { code: "ru", name: "俄语" },
];

const text = ref("");
const fromLang = ref("");
const toLang = ref("");
const engine = ref("auto");
const engineOptions = ref<{ name: string; label: string; configured: boolean }[]>([]);

onMounted(async () => {
  try {
    const v = await invoke<{ engines: { name: string; label: string; configured: boolean }[]; defaultEngine: string }>(
      "engines_list",
    );
    engineOptions.value = v.engines;
    engine.value = v.defaultEngine;
  } catch {
    /* 回落 auto */
  }
});
const loading = ref(false);
const streaming = ref(false);
const result = ref<TranslationResult | null>(null);
const streamText = ref("");
const fallbackFrom = ref<string | null>(null);
const error = ref("");
const collected = ref(false);
const route = useRoute();

const isDictionary = computed(
  () => (result.value?.parts.length || 0) + (result.value?.examples.length || 0) > 0,
);

onMounted(() => {
  const t = route.query.text;
  if (typeof t === "string" && t.trim()) text.value = t;
  window.addEventListener("keydown", onGlobalKey);
});
onUnmounted(() => {
  window.removeEventListener("keydown", onGlobalKey);
  // 离开页面即终止流式，避免 LLM 白跑
  if (streaming.value) stopStream();
});

function onGlobalKey(e: KeyboardEvent) {
  if (e.ctrlKey && e.key === "Enter") translate();
}

function resetOutput() {
  result.value = null;
  streamText.value = "";
  fallbackFrom.value = null;
  error.value = "";
  collected.value = false;
}

async function translate() {
  const t = text.value.trim();
  if (!t || loading.value) return;
  loading.value = true;
  resetOutput();

  const wordLike = !t.includes(" ") && [...t].length <= 20;
  const useDictPath =
    engine.value === "iciba" || (engine.value === "auto" && wordLike && !fromLang.value && !toLang.value);

  try {
    if (useDictPath) {
      const outcome = await invoke<{ result: TranslationResult; fallback_from: string | null }>(
        "translate",
        { text: t, engine: engine.value === "auto" ? null : engine.value, from: null, to: null },
      );
      result.value = outcome.result;
      fallbackFrom.value = outcome.fallback_from;
    } else {
      await translateStreaming(t);
    }
  } catch {
    /* 错误已写入 error 展示 */
  } finally {
    loading.value = false;
    streaming.value = false;
  }
}

function stopStream() {
  invoke("translate_llm_stop").catch(() => {});
}

function translateStreaming(t: string): Promise<void> {
  return new Promise((resolve, reject) => {
    streaming.value = true;
    const channel = new Channel<{ type: string; text?: string; result?: TranslationResult; message?: string }>();
    channel.onmessage = (ev) => {
      if (ev.type === "delta" && ev.text) {
        streamText.value += ev.text;
      } else if (ev.type === "done" && ev.result) {
        result.value = ev.result;
        resolve();
      } else if (ev.type === "error") {
        // 主动停止不是错误：保留已流出的部分文本即可
        if ((ev.message || "").includes("已取消")) {
          resolve();
          return;
        }
        error.value = ev.message || "流式翻译失败";
        reject(new Error(ev.message));
      }
    };
    invoke("translate_llm_stream", {
      text: t,
      from: fromLang.value || null,
      to: toLang.value || null,
      onEvent: channel,
    }).catch((e) => {
      if (!String(e).includes("已取消")) {
        error.value = error.value || String(e);
      }
      reject(e);
    });
  });
}

const collecting = ref(false);
async function collect() {
  if (!result.value || collecting.value) return;
  collecting.value = true;
  try {
    await invoke("wordbook_add", { text: result.value.text, note: result.value.translation });
    collected.value = true;
    message.success("已收藏到生词本");
  } catch (e) {
    message.error(`收藏失败：${String(e)}`);
  } finally {
    collecting.value = false;
  }
}

async function copyResult() {
  const r = result.value;
  if (!r) return;
  const content = r.translation || r.parts.map((p) => `${p.pos} ${p.mean}`).join("\n");
  await navigator.clipboard.writeText(content);
  message.success("已复制");
}
</script>

<style scoped>
/* ===== 输入卡片 ===== */
.input-card {
  flex-shrink: 0;
  background: var(--bg-input);
  border: 1px solid var(--border-base);
  border-radius: 12px;
  overflow: hidden;
  transition: border-color var(--transition-fast);
}
.input-card:focus-within {
  border-color: var(--color-primary);
}
.input-area {
  width: 100%;
  border: none;
  outline: none;
  resize: none;
  background: transparent;
  color: var(--text-primary);
  font-size: 15px;
  line-height: 1.7;
  padding: 14px 16px 6px;
  min-height: 88px;
  max-height: 220px;
  font-family: inherit;
}
.input-area::placeholder {
  color: var(--text-tertiary);
}
.input-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px 8px 12px;
  gap: 8px;
}
.toolbar-left {
  display: flex;
  align-items: center;
  gap: 6px;
}
.lang-arrow {
  color: var(--text-tertiary);
  font-size: 12px;
}
.lang-select {
  min-width: 100px;
}
.engine-select {
  min-width: 96px;
}

/* ===== 结果区 ===== */
.pending {
  padding: 48px 0;
  text-align: center;
}
.result-card {
  background: var(--bg-surface);
  border: 1px solid var(--border-light);
  border-radius: 12px;
  padding: 20px 24px;
}
.word-row {
  display: flex;
  align-items: baseline;
  gap: 10px;
  flex-wrap: wrap;
}
.word {
  font-size: 24px;
  font-weight: 700;
  letter-spacing: 0.01em;
}
.phonetic {
  color: var(--text-tertiary);
  font-size: 14px;
}
.engine-chip {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-primary) 10%, transparent);
  color: var(--color-primary);
  user-select: none;
}
.engine-chip.streaming {
  animation: pulse 1.4s ease-in-out infinite;
}
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
.parts {
  margin-top: 14px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.part-row {
  display: flex;
  gap: 10px;
  font-size: 14px;
  line-height: 1.6;
}
.part-pos {
  color: var(--color-primary);
  font-weight: 500;
  flex-shrink: 0;
  min-width: 34px;
}
.part-mean {
  color: var(--text-primary);
}
.examples {
  margin-top: 16px;
  border-top: 1px solid var(--border-light);
  padding-top: 14px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.ex-orig {
  font-size: 13.5px;
  color: var(--text-secondary);
}
.ex-trans {
  font-size: 12.5px;
  color: var(--text-tertiary);
  margin-top: 2px;
}
.plain {
  font-size: 16px;
  line-height: 1.8;
  white-space: pre-wrap;
  margin-top: 10px;
}
.result-actions {
  margin-top: 16px;
  padding-top: 12px;
  border-top: 1px solid var(--border-light);
  display: flex;
  gap: 8px;
}
.chip-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 26px;
  padding: 0 12px;
  border-radius: 999px;
  border: 1px solid var(--border-base);
  background: transparent;
  color: var(--text-secondary);
  font-size: 12px;
  cursor: pointer;
  transition: all var(--transition-fast);
}
.chip-btn:hover:not(:disabled) {
  border-color: var(--color-primary);
  color: var(--color-primary);
}
.chip-btn:disabled {
  opacity: 0.55;
  cursor: default;
}

/* ===== 空态 ===== */
.empty-hint {
  margin: auto;
  text-align: center;
  padding: 40px 0;
  user-select: none;
}
.hint-icon {
  width: 56px;
  height: 56px;
  margin: 0 auto 16px;
  border-radius: 16px;
  background: color-mix(in srgb, var(--color-primary) 12%, transparent);
  color: var(--color-primary);
  font-size: 24px;
  font-weight: 700;
  display: flex;
  align-items: center;
  justify-content: center;
}
.hint-line {
  font-size: 14px;
  color: var(--text-secondary);
}
.hint-sub {
  margin-top: 6px;
  font-size: 12px;
  color: var(--text-tertiary);
}
</style>
