<script setup lang="ts">
/**
 * 翻译引擎管理：双栏布局（与 AI 供应商同构）。
 * 左栏：分组引擎卡片列表（状态圆点）；右栏：详情（能力说明 + Key 配置 + 测试）。
 * 默认引擎策略固定在右栏底部。
 */
import { onMounted, reactive, ref } from "vue";
import { message } from "ant-design-vue";
import { ReloadOutlined } from "@ant-design/icons-vue";
import { invoke } from "@tauri-apps/api/core";

interface EngineRow {
  name: string;
  label: string;
  capability: string;
  group: string;
  groupLabel: string;
  keyFields: { field: string; label: string }[];
  configured: boolean;
  saved: Record<string, boolean>;
}

const engines = ref<EngineRow[]>([]);
const loading = ref(false);
const selected = ref("google");
const defaultEngine = ref("auto");
const testing = ref(false);
const saving = ref(false);
const testResult = ref<{ ok: boolean; text: string } | null>(null);
const form = reactive<Record<string, string>>({});

const GROUP_ORDER = ["Free", "Keyed", "Dictionary", "Llm"];

const current = ref<EngineRow | null>(null);

function select(e: EngineRow) {
  selected.value = e.name;
  testResult.value = null;
  current.value = e;
  for (const f of e.keyFields) {
    if (form[f.field] === undefined) form[f.field] = "";
  }
}

const ENGINE_OPTIONS = [
  { value: "auto", label: "自动调度（词查词典、句走 LLM，未命中自动降级）" },
  { value: "llm", label: "始终 LLM" },
  { value: "iciba", label: "始终词典（仅中英）" },
];

async function load(keep = true) {
  loading.value = true;
  try {
    const v = await invoke<{ engines: EngineRow[]; defaultEngine: string }>("engines_list");
    engines.value = v.engines;
    defaultEngine.value = v.defaultEngine;
    const cur = engines.value.find((e) => e.name === selected.value);
    if (cur && keep) {
      select(cur);
    } else if (engines.value.length) {
      select(engines.value[0]);
    }
  } finally {
    loading.value = false;
  }
}

async function save() {
  if (!current.value) return;
  saving.value = true;
  try {
    await invoke("engine_set_config", { name: current.value.name, values: form });
    message.success("已保存");
    for (const k in form) form[k] = "";
    await load();
  } catch (e) {
    message.error(String(e));
  } finally {
    saving.value = false;
  }
}

async function test() {
  if (!current.value) return;
  testing.value = true;
  testResult.value = null;
  try {
    const r = await invoke<{ word: string; translation?: string; note?: string }>("engine_test", {
      name: current.value.name,
    });
    testResult.value = { ok: true, text: r.note ?? `「${r.word}」→ ${r.translation}` };
  } catch (e) {
    testResult.value = { ok: false, text: String(e) };
  } finally {
    testing.value = false;
  }
}

async function onEngineChange() {
  await invoke("set_default_engine", { engine: defaultEngine.value });
  message.success("默认引擎已保存（各端共用）");
}

onMounted(() => load(false));
</script>

<template>
  <section class="engines-section">
    <!-- 左栏：分组引擎列表 -->
    <div class="engine-list-panel">
      <div class="panel-header">
        <span class="panel-title">引擎</span>
        <span class="engine-count">{{ engines.length }}</span>
      </div>
      <a-spin :spinning="loading">
        <div class="engine-list">
          <template v-for="g in GROUP_ORDER" :key="g">
            <template v-if="engines.filter((e) => e.group === g).length">
              <div class="group-title">
                {{ engines.filter((e) => e.group === g)[0].groupLabel }}
              </div>
              <div
                v-for="e in engines.filter((x) => x.group === g)"
                :key="e.name"
                class="engine-card"
                :class="{ active: selected === e.name }"
                @click="select(e)"
              >
                <div class="engine-info">
                  <div class="engine-name">{{ e.label }}</div>
                  <div class="engine-meta">
                    {{ e.keyFields.length === 0
                      ? (e.name === "llm" ? "AI 供应商" : "免 Key 即用")
                      : (e.configured ? "已配置" : "未配置") }}
                  </div>
                </div>
                <span class="state-dot" :class="{ on: e.configured }" :title="e.configured ? '可用' : '未配置'"></span>
              </div>
            </template>
          </template>
        </div>
      </a-spin>
      <div class="panel-footer">
        <a-button size="small" block @click="load()">
          <template #icon><ReloadOutlined /></template>
          刷新
        </a-button>
      </div>
    </div>

    <!-- 右栏：详情 -->
    <div v-if="current" class="engine-detail-panel">
      <div class="detail-header">
        <div class="detail-title-wrap">
          <div class="detail-name">{{ current.label }}</div>
          <div class="detail-desc">{{ current.capability }}</div>
        </div>
        <a-button size="small" :loading="testing" @click="test">测试</a-button>
      </div>

      <div class="detail-scroll">
        <!-- Key 配置 -->
        <div v-if="current.keyFields.length" class="setting-group">
          <div class="group-title-inline">服务凭据</div>
          <div v-for="f in current.keyFields" :key="f.field" class="field-row">
            <div class="field-label">
              {{ f.label }}
              <span v-if="current.saved[f.field]" class="field-saved">已设置</span>
            </div>
            <a-textarea
              v-model:value="form[f.field]"
              :placeholder="current.saved[f.field] ? '留空保存 = 清除；输入则整体替换' : '输入后保存；多组凭据用逗号分隔（成对字段需按顺序对应）'"
              :auto-size="{ minRows: 1, maxRows: 4 }"
              class="mono-input"
            />
          </div>
          <div class="field-actions">
            <a-button type="primary" size="small" :loading="saving" @click="save">保存</a-button>
          </div>
        </div>

        <!-- 免 Key / LLM 提示 -->
        <div v-else-if="current.name === 'llm'" class="hint-block">
          <div class="group-title-inline">补全来源</div>
          <p class="hint-text">
            {{ current.configured ? "已有启用中的 AI 供应商。" : "尚未启用供应商。" }}
            配置与管理见「AI 供应商」分区。
          </p>
          <a-button size="small" @click="$emit('goto-providers')">前往 AI 供应商</a-button>
        </div>
        <div v-else class="hint-block">
          <p class="hint-text">免 Key 引擎，开箱即用；可直接在翻译页引擎下拉选择。</p>
        </div>

        <!-- 测试结果 -->
        <div v-if="testResult" class="test-result" :class="testResult.ok ? 'ok' : 'fail'">
          {{ testResult.text }}
        </div>

        <!-- 默认策略（固定） -->
        <div class="policy-block">
          <div class="group-title-inline">默认引擎策略</div>
          <p class="hint-text">翻译页进入时的初始值，翻译时可临时覆盖；策略各端共用。</p>
          <a-select v-model:value="defaultEngine" style="width: 100%; max-width: 360px" @change="onEngineChange">
            <a-select-option v-for="o in ENGINE_OPTIONS" :key="o.value" :value="o.value">
              {{ o.label }}
            </a-select-option>
          </a-select>
        </div>
      </div>
    </div>
  </section>
</template>

<script lang="ts">
export default { emits: ["goto-providers"] };
</script>

<style scoped>
.engines-section {
  display: flex;
  height: 100%;
  min-height: 0;
  overflow: hidden;
}

/* ===== 左栏 ===== */
.engine-list-panel {
  display: flex;
  flex-direction: column;
  width: 200px;
  min-width: 200px;
  flex-shrink: 0;
  border-right: 1px solid var(--border-light);
}
.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 16px 8px;
}
.panel-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}
.engine-count {
  font-size: 11.5px;
  color: var(--text-tertiary);
}
.engine-list {
  padding: 0 8px;
  overflow-y: auto;
  flex: 1;
}
.group-title {
  padding: 12px 8px 4px;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-tertiary);
  letter-spacing: 0.05em;
  user-select: none;
}
.engine-card {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 10px;
  border-radius: 6px;
  border-left: 3px solid transparent;
  cursor: pointer;
  transition: all var(--transition-fast);
}
.engine-card:hover {
  background: var(--color-nav-hover);
}
.engine-card.active {
  border-left-color: var(--color-primary);
  background: var(--color-nav-active);
}
.engine-info {
  flex: 1;
  min-width: 0;
}
.engine-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.engine-card.active .engine-name {
  color: var(--color-primary);
}
.engine-meta {
  margin-top: 2px;
  font-size: 11px;
  color: var(--text-tertiary);
}
.state-dot {
  flex-shrink: 0;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--border-base);
}
.state-dot.on {
  background: var(--color-success);
}
.panel-footer {
  padding: 10px 12px 14px;
}

/* ===== 右栏 ===== */
.engine-detail-panel {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.detail-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 16px 20px 10px;
  flex-shrink: 0;
}
.detail-name {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
}
.detail-desc {
  margin-top: 2px;
  font-size: 11.5px;
  color: var(--text-tertiary);
}
.detail-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 4px 20px 20px;
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.group-title-inline {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text-secondary);
}
.setting-group {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.field-row {
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-width: 440px;
}
.field-label {
  font-size: 12.5px;
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  gap: 8px;
}
.field-saved {
  font-size: 10px;
  color: var(--color-success);
  border: 1px solid color-mix(in srgb, var(--color-success) 36%, transparent);
  border-radius: 999px;
  padding: 0 6px;
  line-height: 16px;
}
.field-actions {
  display: flex;
  gap: 8px;
}
.mono-input :deep(textarea) {
  font-family: "JetBrains Mono", Consolas, monospace;
  font-size: 12px;
}

.hint-block {
  border: 1px solid var(--border-light);
  border-radius: 10px;
  padding: 14px 16px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  align-items: flex-start;
}
.hint-text {
  margin: 0;
  font-size: 12.5px;
  color: var(--text-secondary);
  line-height: 1.7;
}

.test-result {
  font-size: 12px;
  line-height: 1.6;
  word-break: break-all;
  padding: 10px 14px;
  border-radius: 10px;
  border: 1px solid var(--border-light);
}
.test-result.ok {
  color: var(--color-success);
  border-color: color-mix(in srgb, var(--color-success) 30%, transparent);
  background: color-mix(in srgb, var(--color-success) 6%, transparent);
}
.test-result.fail {
  color: var(--color-danger);
  border-color: color-mix(in srgb, var(--color-danger) 30%, transparent);
  background: color-mix(in srgb, var(--color-danger) 5%, transparent);
}

.policy-block {
  border-top: 1px solid var(--border-light);
  padding-top: 14px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
</style>
