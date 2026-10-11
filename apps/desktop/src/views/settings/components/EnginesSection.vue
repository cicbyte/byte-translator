<script setup lang="ts">
/**
 * 翻译引擎管理：分组行式列表（免 Key / 需配置 / 词典 / AI）。
 * 行内展开配置表单（Key 字段留空保存 = 清除；未传字段保持原值）+ 连通测试。
 */
import { computed, onMounted, reactive, ref } from "vue";
import { message } from "ant-design-vue";
import { ReloadOutlined, DownOutlined, RightOutlined } from "@ant-design/icons-vue";
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
const expanded = ref<string | null>(null);
const defaultEngine = ref("auto");
const testing = ref("");
const testResults = reactive<Record<string, { ok: boolean; text: string }>>({});
const forms = reactive<Record<string, Record<string, string>>>({});
const saving = ref("");

const GROUP_ORDER = ["Free", "Keyed", "Dictionary", "Llm"];
const grouped = computed(() =>
  GROUP_ORDER.map((g) => ({
    key: g,
    rows: engines.value.filter((e) => e.group === g),
  })).filter((g) => g.rows.length > 0),
);

const ENGINE_OPTIONS = [
  { value: "auto", label: "自动调度（词查词典、句走 LLM，未命中自动降级）" },
  { value: "llm", label: "始终 LLM" },
  { value: "iciba", label: "始终词典（仅中英）" },
];

async function load() {
  loading.value = true;
  try {
    const v = await invoke<{ engines: EngineRow[]; defaultEngine: string }>("engines_list");
    engines.value = v.engines;
    for (const e of v.engines) {
      if (!forms[e.name]) forms[e.name] = {};
      for (const f of e.keyFields) {
        if (forms[e.name][f.field] === undefined) forms[e.name][f.field] = "";
      }
    }
    defaultEngine.value = v.defaultEngine;
  } finally {
    loading.value = false;
  }
}

function toggle(name: string) {
  expanded.value = expanded.value === name ? null : name;
}

async function save(name: string) {
  saving.value = name;
  try {
    await invoke("engine_set_config", { name, values: forms[name] });
    message.success("已保存");
    expanded.value = null;
    for (const k in forms[name]) forms[name][k] = "";
    await load();
  } catch (e) {
    message.error(String(e));
  } finally {
    saving.value = "";
  }
}

async function test(name: string) {
  testing.value = name;
  delete testResults[name];
  try {
    const r = await invoke<{ word: string; translation?: string; note?: string }>("engine_test", { name });
    testResults[name] = {
      ok: true,
      text: r.note ?? `「${r.word}」→ ${r.translation}`,
    };
  } catch (e) {
    testResults[name] = { ok: false, text: String(e) };
  } finally {
    testing.value = "";
  }
}

async function onEngineChange() {
  await invoke("set_default_engine", { engine: defaultEngine.value });
  message.success("默认引擎已保存（各端共用）");
}

onMounted(load);
</script>

<template>
  <section class="setting-section engines">
    <header class="section-head">
      <div>
        <h3 class="section-title">翻译引擎</h3>
        <p class="section-desc">词查词典、句走 LLM、其余引擎任意语言对——显式选择时直接路由，策略各端共用</p>
      </div>
      <a-button size="small" @click="load">
        <template #icon><ReloadOutlined /></template>
        刷新
      </a-button>
    </header>

    <a-spin :spinning="loading">
      <div v-for="group in grouped" :key="group.key" class="engine-group">
        <div class="group-title">{{ group.rows[0].groupLabel }}</div>
        <div
          v-for="e in group.rows"
          :key="e.name"
          class="engine-row"
          :class="{ open: expanded === e.name }"
        >
          <div class="row-main" @click="toggle(e.name)">
            <component :is="expanded === e.name ? DownOutlined : RightOutlined" class="row-caret" />
            <span class="row-label">{{ e.label }}</span>
            <span class="row-cap" :title="e.capability">{{ e.capability }}</span>
            <span class="row-state" :class="{ ok: e.configured }">
              {{ e.name === "llm"
                ? (e.configured ? "供应商已启用" : "未启用供应商")
                : e.keyFields.length === 0 ? "免 Key" : (e.configured ? "已配置" : "未配置") }}
            </span>
          </div>
          <div class="row-actions" @click.stop>
            <button class="link-btn" :disabled="testing === e.name" @click="test(e.name)">
              {{ testing === e.name ? "测试中…" : "测试" }}
            </button>
            <button v-if="e.keyFields.length" class="link-btn" @click="toggle(e.name)">配置</button>
            <a-button
              v-else-if="e.name === 'llm'"
              size="small"
              type="link"
              style="padding: 0"
              @click="$emit('goto-providers')"
            >
              去启用
            </a-button>
          </div>

          <div v-if="testResults[e.name]" class="test-result" :class="testResults[e.name].ok ? 'ok' : 'fail'">
            {{ testResults[e.name].text }}
          </div>

          <!-- 行内展开：配置表单 -->
          <div v-if="expanded === e.name && e.keyFields.length" class="config-pane">
            <div v-for="f in e.keyFields" :key="f.field" class="config-field">
              <span class="field-label">
                {{ f.label }}
                <span v-if="e.saved[f.field]" class="field-saved">已设置</span>
              </span>
              <a-input-password
                v-model:value="forms[e.name][f.field]"
                :placeholder="e.saved[f.field] ? '留空保存 = 清除' : ''"
                size="small"
              />
            </div>
            <div class="config-actions">
              <a-button size="small" type="primary" :loading="saving === e.name" @click="save(e.name)">保存</a-button>
              <a-button size="small" @click="expanded = null">取消</a-button>
            </div>
          </div>
        </div>
      </div>
    </a-spin>

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

<script lang="ts">
export default { emits: ["goto-providers"] };
</script>

<style scoped>
.engine-group + .engine-group {
  margin-top: 4px;
}
.group-title {
  padding: 10px 6px 4px;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-tertiary);
  letter-spacing: 0.05em;
  user-select: none;
}
.engine-row {
  border-radius: 8px;
  padding: 0 6px;
}
.engine-row:hover {
  background: var(--color-nav-hover);
}
.engine-row + .engine-row {
  border-top: 1px solid var(--border-light);
}
.row-main {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 6px;
  cursor: pointer;
  min-width: 0;
}
.row-caret {
  font-size: 10px;
  color: var(--text-tertiary);
  flex-shrink: 0;
}
.row-label {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
  flex-shrink: 0;
}
.row-cap {
  flex: 1;
  min-width: 0;
  font-size: 11.5px;
  color: var(--text-tertiary);
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.row-state {
  flex-shrink: 0;
  font-size: 11px;
  padding: 1px 8px;
  border-radius: 999px;
  border: 1px solid var(--border-base);
  color: var(--text-tertiary);
  user-select: none;
}
.row-state.ok {
  color: var(--color-success);
  border-color: color-mix(in srgb, var(--color-success) 36%, transparent);
  background: color-mix(in srgb, var(--color-success) 8%, transparent);
}
.row-actions {
  position: absolute;
  right: 14px;
  margin-top: -33px;
  display: inline-flex;
  gap: 2px;
  opacity: 0;
  transition: opacity 0.15s ease;
  background: var(--bg-surface);
  border-radius: 6px;
}
.engine-row:hover .row-actions,
.engine-row.open .row-actions {
  opacity: 1;
}
.link-btn {
  border: none;
  background: transparent;
  color: var(--text-tertiary);
  font-size: 11.5px;
  padding: 4px 8px;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s ease;
}
.link-btn:hover:not(:disabled) {
  color: var(--color-primary);
  background: var(--bg-input);
}
.link-btn:disabled {
  opacity: 0.5;
  cursor: default;
}

.test-result {
  margin: 0 6px 8px 26px;
  font-size: 11.5px;
  line-height: 1.6;
  word-break: break-all;
}
.test-result.ok {
  color: var(--color-success);
}
.test-result.fail {
  color: var(--color-danger);
}

.config-pane {
  margin: 2px 6px 12px 26px;
  padding: 12px 14px;
  background: var(--bg-input);
  border: 1px solid var(--border-light);
  border-radius: 10px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.config-field {
  display: flex;
  flex-direction: column;
  gap: 5px;
}
.field-label {
  font-size: 12px;
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
.config-actions {
  display: flex;
  gap: 8px;
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
