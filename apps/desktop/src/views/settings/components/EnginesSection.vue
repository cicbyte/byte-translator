<script setup lang="ts">
/**
 * 翻译引擎管理：引擎卡（能力/语言对/就绪状态）+ 词典 Key 配置与连通测试
 * + 默认引擎策略。引擎是调度层概念（词查词典/句走 LLM），AI 供应商是 LLM
 * 引擎的补全来源（见「AI 供应商」分区）。
 */
import { onMounted, reactive, ref } from "vue";
import { message } from "ant-design-vue";
import { ReloadOutlined, BookOutlined, CloudOutlined } from "@ant-design/icons-vue";
import { invoke } from "@tauri-apps/api/core";

interface EngineInfo {
  iciba: { keySource: "env" | "db" | "builtin"; keySet: boolean };
  llm: { provider: string | null };
  defaultEngine: string;
}

const info = ref<EngineInfo | null>(null);
const icibaKey = ref("");
const keySaving = ref(false);
const testing = ref(false);
const testResult = ref<{ ok: boolean; text: string } | null>(null);
const defaultEngine = ref("auto");

const ENGINE_OPTIONS = [
  { value: "auto", label: "自动调度（词查词典、句走 LLM，未命中自动降级）" },
  { value: "llm", label: "始终 LLM" },
  { value: "iciba", label: "始终词典（仅中英）" },
];

async function load() {
  info.value = await invoke<EngineInfo>("engine_info");
  defaultEngine.value = info.value.defaultEngine;
  testResult.value = null;
}

async function saveKey() {
  keySaving.value = true;
  try {
    await invoke("set_iciba_key", { key: icibaKey.value });
    icibaKey.value = "";
    message.success("词典 Key 已保存并生效");
    await load();
  } catch (e) {
    message.error(String(e));
  } finally {
    keySaving.value = false;
  }
}

async function testIciba() {
  testing.value = true;
  testResult.value = null;
  try {
    const r = await invoke<{ word: string; meanings: number }>("engine_test_iciba");
    testResult.value = { ok: true, text: `查询「${r.word}」成功，返回 ${r.meanings} 条释义` };
  } catch (e) {
    testResult.value = { ok: false, text: String(e) };
  } finally {
    testing.value = false;
  }
}

async function onEngineChange() {
  await invoke("set_default_engine", { engine: defaultEngine.value });
  message.success("默认引擎已保存（各端共用）");
  await load();
}

const keySourceLabel: Record<string, string> = {
  env: "环境变量",
  db: "已配置",
  builtin: "内置公共 Key（已停用，未命中自动走 LLM）",
};

onMounted(load);
</script>

<template>
  <section class="setting-section engines">
    <header class="section-head">
      <div>
        <h3 class="section-title">翻译引擎</h3>
        <p class="section-desc">调度层概念：单词走词典、句子走 LLM；策略对 CLI / 桌面 / 服务端共用</p>
      </div>
      <a-button size="small" @click="load">
        <template #icon><ReloadOutlined /></template>
        刷新
      </a-button>
    </header>

    <!-- 引擎卡 × 2 -->
    <div class="engine-grid">
      <div class="engine-card">
        <div class="engine-head">
          <span class="engine-icon"><BookOutlined /></span>
          <div class="engine-title-wrap">
            <div class="engine-name">词典 · iciba</div>
            <div class="engine-meta">音标 / 词性 / 例句 · 仅中英 · 结果本地缓存</div>
          </div>
        </div>
        <div class="key-row">
          <span class="key-label">Key</span>
          <span class="key-state" :class="{ ok: info?.iciba.keySet && info?.iciba.keySource !== 'builtin' }">
            {{ info ? keySourceLabel[info.iciba.keySource] : "…" }}
          </span>
        </div>
        <div class="key-config">
          <a-input-password
            v-model:value="icibaKey"
            placeholder="到 open.iciba.com 免费申请；留空保存 = 清除自定义"
            size="small"
          />
          <a-button size="small" :loading="keySaving" @click="saveKey">保存</a-button>
          <a-button size="small" :loading="testing" @click="testIciba">测试</a-button>
        </div>
        <div v-if="testResult" class="test-result" :class="testResult.ok ? 'ok' : 'fail'">
          {{ testResult.text }}
        </div>
      </div>

      <div class="engine-card">
        <div class="engine-head">
          <span class="engine-icon"><CloudOutlined /></span>
          <div class="engine-title-wrap">
            <div class="engine-name">文本 · LLM</div>
            <div class="engine-meta">任意语言对 · 句子与长文 · 流式输出</div>
          </div>
        </div>
        <div class="key-row">
          <span class="key-label">来源</span>
          <span class="key-state" :class="{ ok: !!info?.llm.provider }">
            {{ info?.llm.provider ? `启用中：${info.llm.provider}` : "未启用——在「AI 供应商」配置并启用" }}
          </span>
        </div>
      </div>
    </div>

    <!-- 默认策略 -->
    <div class="policy-row">
      <div>
        <div class="policy-label">默认引擎策略</div>
        <div class="policy-hint">翻译页进入时的初始值，翻译时可临时覆盖</div>
      </div>
      <a-select v-model:value="defaultEngine" style="width: 320px" @change="onEngineChange">
        <a-select-option v-for="o in ENGINE_OPTIONS" :key="o.value" :value="o.value">
          {{ o.label }}
        </a-select-option>
      </a-select>
    </div>
  </section>
</template>

<style scoped>
.engine-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 10px;
}
.engine-card {
  border: 1px solid var(--border-light);
  border-radius: 10px;
  padding: 14px 16px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.engine-head {
  display: flex;
  align-items: center;
  gap: 12px;
}
.engine-icon {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  background: color-mix(in srgb, var(--color-primary) 12%, transparent);
  color: var(--color-primary);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 17px;
  flex-shrink: 0;
}
.engine-name {
  font-size: 13.5px;
  font-weight: 600;
  color: var(--text-primary);
}
.engine-meta {
  margin-top: 2px;
  font-size: 11.5px;
  color: var(--text-tertiary);
}
.key-row {
  display: flex;
  align-items: center;
  gap: 10px;
}
.key-label {
  width: 36px;
  flex-shrink: 0;
  font-size: 12px;
  color: var(--text-tertiary);
}
.key-state {
  font-size: 12px;
  color: var(--text-secondary);
}
.key-state.ok {
  color: var(--color-success);
}
.key-config {
  display: flex;
  gap: 8px;
}
.key-config .ant-input-password {
  flex: 1;
}
.test-result {
  font-size: 12px;
  line-height: 1.6;
  word-break: break-all;
}
.test-result.ok {
  color: var(--color-success);
}
.test-result.fail {
  color: var(--color-danger);
}
.policy-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding-top: 12px;
  border-top: 1px solid var(--border-light);
}
.policy-label {
  font-size: 13px;
  color: var(--text-primary);
}
.policy-hint {
  margin-top: 3px;
  font-size: 11.5px;
  color: var(--text-tertiary);
}
</style>
