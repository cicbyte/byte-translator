<script setup lang="ts">
/**
 * AI 供应商管理：双栏布局。
 * 左栏：供应商卡片列表（名称/模型数/启停 switch）+ 底部添加按钮；
 * 右栏：详情编辑（自动保存）——名称/地址/密钥（检测）/生成参数/默认模型/模型列表
 * （拉取 /v1/models、按家族分组折叠、设为默认/移除）。
 */
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import { message, Modal } from "ant-design-vue";
import {
  PlusOutlined,
  DeleteOutlined,
  SyncOutlined,
  CheckCircleFilled,
  CloseCircleOutlined,
} from "@ant-design/icons-vue";
import { invoke } from "@tauri-apps/api/core";

interface ProviderRow {
  provider_key: string;
  name: string;
  base_url: string;
  model: string;
  is_enabled: boolean;
  api_key_set: boolean;
  max_tokens: number;
  temperature: number;
  models: string[];
}

const list = ref<ProviderRow[]>([]);
const loading = ref(false);
const selectedKey = ref("");
const checking = ref(false);
const connectResult = ref<{ ok: boolean; message: string } | null>(null);
const fetching = ref(false);

/** 添加供应商（协议决定 provider_key 与接入方式；同协议重复自动加序号） */
/** 内置供应商预设：选中即预填 Base URL（key 兼容协议分派，可改） */
const BUILTIN_PROVIDERS = [
  { value: "deepseek", label: "DeepSeek", baseUrl: "https://api.deepseek.com" },
  { value: "zhipu", label: "智谱 GLM", baseUrl: "https://open.bigmodel.cn" },
  { value: "openai", label: "OpenAI", baseUrl: "https://api.openai.com" },
  { value: "moonshot", label: "Moonshot Kimi", baseUrl: "https://api.moonshot.cn" },
  { value: "anthropic", label: "Anthropic（Claude）", baseUrl: "https://api.anthropic.com" },
  { value: "gemini", label: "Gemini", baseUrl: "https://generativelanguage.googleapis.com" },
  { value: "ollama", label: "Ollama（本地）", baseUrl: "http://127.0.0.1:11434" },
];
const addingVisible = ref(false);
const addForm = reactive({ protocol: "deepseek", name: "", base_url: "" });


/** 切换内置供应商：名称/Base URL 带默认值（可改） */
function onBuiltinChange(key: string) {
  const p = BUILTIN_PROVIDERS.find((x) => x.value === key);
  if (!p) return;
  if (!addForm.name.trim()) addForm.name = p.label.replace(/（.*）/, "");
  if (!addForm.base_url.trim()) addForm.base_url = p.baseUrl;
}

function genProviderKey(protocol: string): string {
  const keys = new Set(list.value.map((p) => p.provider_key));
  if (!keys.has(protocol)) return protocol;
  for (let i = 2; ; i++) {
    const k = `${protocol}-${i}`;
    if (!keys.has(k)) return k;
  }
}

/** 批量添加模型（逗号分隔） */
const addModelVisible = ref(false);
const addModelText = ref("");

/** 获取模型 → 勾选候选池（远端 ∪ 已有） */
const fetchVisible = ref(false);
const fetchSearch = ref("");
const fetchedModels = ref<string[]>([]);
const selectedFetch = ref<Set<string>>(new Set());

const current = computed(() => list.value.find((p) => p.provider_key === selectedKey.value) || null);

const editForm = reactive({
  name: "",
  base_url: "",
  api_key: "",
  model: "",
  max_tokens: 4096,
  temperature: 0.3,
  models: [] as string[],
});

/** 模型家族分组（按 id 首段或厂商前缀聚合） */
const modelGroups = computed(() => {
  const groups = new Map<string, string[]>();
  for (const id of editForm.models) {
    const family = id.split(/[-._]/)[0] || "其它";
    if (!groups.has(family)) groups.set(family, []);
    groups.get(family)!.push(id);
  }
  return [...groups.entries()]
    .map(([name, models]) => ({ name, models: models.sort() }))
    .sort((a, b) => b.models.length - a.models.length);
});

async function load() {
  loading.value = true;
  try {
    list.value = (await invoke<ProviderRow[]>("provider_list")).map((p) => ({
      ...p,
      models: p.models || [],
    }));
    if (!selectedKey.value && list.value.length) select(list.value[0].provider_key);
  } finally {
    loading.value = false;
  }
}

function select(key: string) {
  selectedKey.value = key;
  connectResult.value = null;
  const p = list.value.find((x) => x.provider_key === key);
  if (p) {
    editForm.name = p.name;
    editForm.base_url = p.base_url;
    editForm.api_key = "";
    editForm.model = p.model;
    editForm.max_tokens = p.max_tokens || 4096;
    editForm.temperature = p.temperature ?? 0.3;
    editForm.models = [...p.models];
  }
}

/** 自动保存（600ms 防抖；新增/删除供应商时跳过） */
const save = debounce(async () => {
  if (!current.value) return;
  await invoke("provider_set", {
    args: {
      providerKey: selectedKey.value,
      name: editForm.name || null,
      baseUrl: editForm.base_url || null,
      apiKey: editForm.api_key || null,
      model: editForm.model || null,
      maxTokens: editForm.max_tokens,
      temperature: editForm.temperature,
      models: editForm.models,
    },
  });
  await load();
  editForm.api_key = "";
}, 600);

function debounce<T extends (...args: any[]) => any>(fn: T, ms: number) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const wrapped = (...args: Parameters<T>) => {
    clearTimeout(timer);
    timer = setTimeout(() => fn(...args), ms);
  };
  wrapped.cancel = () => clearTimeout(timer);
  return wrapped;
}

watch(
  () => [
    editForm.name,
    editForm.base_url,
    editForm.api_key,
    editForm.model,
    editForm.max_tokens,
    editForm.temperature,
    editForm.models.length,
  ],
  () => {
    if (current.value) save();
  },
);

async function toggleEnabled(p: ProviderRow, val: boolean) {
  await invoke("provider_enable", { providerKey: p.provider_key, enabled: val });
  if (val) message.success(`已启用 ${p.name || p.provider_key}（其余自动停用）`);
  await load();
}

async function addProvider() {
  if (!addForm.name.trim()) {
    message.warning("请填写名称");
    return;
  }
  const key = genProviderKey(addForm.protocol);
  await invoke("provider_set", {
    args: {
      providerKey: key,
      name: addForm.name.trim(),
      baseUrl: addForm.base_url.trim() || null,
      apiKey: null,
      model: null,
    },
  });
  addingVisible.value = false;
  await load();
  select(key);
}

function confirmAddModel() {
  const ids = addModelText.value
    .split(/[,，\n]/)
    .map((x) => x.trim())
    .filter(Boolean);
  for (const id of ids) {
    if (!editForm.models.includes(id)) editForm.models.push(id);
  }
  addModelText.value = "";
  addModelVisible.value = false;
  if (ids.length) message.success(`已添加 ${ids.length} 个模型`);
}

/** 远端拉取 → 打开勾选池（远端 ∪ 已有，全选远端） */
async function fetchModels() {
  if (!current.value) return;
  fetching.value = true;
  try {
    await saveNow();
    const ids = await invoke<string[]>("provider_fetch_models", { providerKey: selectedKey.value });
    fetchedModels.value = ids;
    const pool = new Set([...ids, ...editForm.models]);
    selectedFetch.value = new Set(ids.filter((m) => editForm.models.includes(m)).concat(editForm.models));
    fetchSearch.value = "";
    fetchVisible.value = true;
    void pool;
  } catch (e) {
    message.error(`获取失败：${e}`);
  } finally {
    fetching.value = false;
  }
}

/** 候选池 = 远端 ∪ 已有（按 id 去重）；预置库不掺入——模型只走拉取与手动输入 */
const fetchCandidates = computed(() => {
  const all = [...new Set([...fetchedModels.value, ...editForm.models])].sort();
  const kw = fetchSearch.value.trim().toLowerCase();
  return kw ? all.filter((m) => m.toLowerCase().includes(kw)) : all;
});

/** 分组：按 id 首段聚合 */
const fetchGroups = computed(() => {
  const groups = new Map<string, string[]>();
  for (const id of fetchCandidates.value) {
    const family = id.split(/[-._]/)[0] || "其它";
    if (!groups.has(family)) groups.set(family, []);
    groups.get(family)!.push(id);
  }
  return [...groups.entries()].map(([name, models]) => ({ name, models })).sort((a, b) => b.models.length - a.models.length);
});

function toggleFetchModel(id: string) {
  const next = new Set(selectedFetch.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  selectedFetch.value = next;
}

async function confirmFetchModels() {
  editForm.models = [...selectedFetch.value].sort();
  fetchVisible.value = false;
  await saveNow();
  message.success(`模型列表已更新（${editForm.models.length} 个）`);
}

/** 立即保存（去抖） */
async function saveNow() {
  save.cancel?.();
  if (!current.value) return;
  await invoke("provider_set", {
    args: {
      providerKey: selectedKey.value,
      name: editForm.name || null,
      baseUrl: editForm.base_url || null,
      apiKey: editForm.api_key || null,
      model: editForm.model || null,
      maxTokens: editForm.max_tokens,
      temperature: editForm.temperature,
      models: editForm.models,
    },
  });
  editForm.api_key = "";
  await load();
}

async function onDelete() {
  if (!current.value) return;
  const key = current.value.provider_key;
  Modal.confirm({
    title: `删除供应商「${current.value.name || key}」？`,
    content: "该操作不可恢复",
    okType: "danger",
    onOk: async () => {
      await invoke("provider_delete", { providerKey: key });
      selectedKey.value = "";
      await load();
      message.success("已删除");
    },
  });
}

async function handleCheck() {
  if (!current.value) return;
  await saveNow();
  checking.value = true;
  connectResult.value = null;
  try {
    await invoke("provider_test", { providerKey: selectedKey.value });
    connectResult.value = { ok: true, message: "连接正常" };
  } catch (e) {
    connectResult.value = { ok: false, message: String(e) };
  } finally {
    checking.value = false;
  }
}

function setDefaultModel(id: string) {
  editForm.model = id;
}

function removeModel(id: string) {
  editForm.models = editForm.models.filter((m) => m !== id);
  if (editForm.model === id) editForm.model = "";
}

onMounted(load);
onBeforeUnmount(() => save.cancel?.());
</script>

<template>
  <section class="provider-section">
    <!-- 左栏：供应商列表 -->
    <div class="provider-list-panel">
      <div class="panel-header">
        <span class="panel-title">供应商</span>
        <span class="provider-count">{{ list.length }}</span>
      </div>
      <a-spin :spinning="loading">
        <div v-if="list.length" class="provider-list">
          <div
            v-for="p in list"
            :key="p.provider_key"
            class="provider-card"
            :class="{ active: selectedKey === p.provider_key }"
            @click="select(p.provider_key)"
          >
            <div class="provider-info">
              <div class="provider-name">{{ p.name || p.provider_key }}</div>
              <div class="provider-meta">
                {{ p.models.length ? `${p.models.length} 个模型` : "未配置模型" }}
              </div>
            </div>
            <div class="provider-toggle" @click.stop>
              <a-switch
                :checked="p.is_enabled"
                size="small"
                :title="p.is_enabled ? '使用中（点击停用）' : '点击启用（将停用其他供应商）'"
                @change="(val: any) => toggleEnabled(p, Boolean(val))"
              />
            </div>
          </div>
        </div>
        <div v-else class="list-empty">暂无供应商</div>
      </a-spin>
      <div class="panel-footer">
        <a-button type="dashed" block @click="addingVisible = true">
          <template #icon><PlusOutlined /></template>
          添加供应商
        </a-button>
      </div>
    </div>

    <!-- 右栏：详情 -->
    <div v-if="current" class="provider-detail-panel">
      <div class="detail-header">
        <div class="detail-title-wrap">
          <div class="detail-name">{{ editForm.name || "未命名供应商" }}</div>
          <div class="detail-desc mono">{{ current.provider_key }} · {{ editForm.base_url || "未配置地址" }}</div>
        </div>
        <a-button size="small" danger @click="onDelete">
          <template #icon><DeleteOutlined /></template>
          删除
        </a-button>
      </div>

      <div class="detail-scroll">
        <div class="setting-group">
          <div class="setting-title">名称</div>
          <a-input v-model:value="editForm.name" placeholder="如：DeepSeek 官方" />
        </div>

        <div class="setting-group">
          <div class="setting-title">Base URL</div>
          <a-input v-model:value="editForm.base_url" placeholder="https://api.deepseek.com" class="mono" />
        </div>

        <div class="setting-group">
          <div class="setting-title-row">
            <span class="setting-title">API 密钥</span>
            <a-tag :color="current.api_key_set ? 'success' : 'error'" class="key-tag">
              {{ current.api_key_set ? "已设置" : "未设置" }}
            </a-tag>
          </div>
          <div class="api-key-row">
            <a-input-password
              v-model:value="editForm.api_key"
              :placeholder="current.api_key_set ? '留空保留已设置的 Key' : 'sk-...'"
            />
            <a-button
              class="check-btn"
              :loading="checking"
              :type="connectResult?.ok ? 'primary' : 'default'"
              @click="handleCheck"
            >
              <CheckCircleFilled v-if="connectResult?.ok" style="color: #52c41a" />
              <CloseCircleOutlined v-else-if="connectResult && !connectResult.ok" style="color: #ff4d4f" />
              {{ checking ? "检测中…" : "检测" }}
            </a-button>
          </div>
          <div v-if="connectResult" class="connect-result" :class="connectResult.ok ? 'success' : 'error'">
            {{ connectResult.message }}
          </div>
        </div>

        <div class="setting-group">
          <div class="setting-title">生成参数</div>
          <div class="param-row">
            <div class="param-item">
              <span class="param-label">Max Tokens</span>
              <a-input-number v-model:value="editForm.max_tokens" :min="1" :max="200000" style="width: 100%" />
            </div>
            <div class="param-item">
              <span class="param-label">Temperature</span>
              <a-input-number
                v-model:value="editForm.temperature"
                :min="0"
                :max="2"
                :step="0.1"
                style="width: 100%"
              />
            </div>
          </div>
        </div>

        <div class="setting-group">
          <div class="setting-title">默认模型</div>
          <a-select
            v-if="editForm.models.length"
            v-model:value="editForm.model"
            :options="editForm.models.map((m) => ({ value: m, label: m }))"
            allow-clear
            show-search
            class="mono full"
            placeholder="选择或搜索模型"
          />
          <a-input v-else v-model:value="editForm.model" placeholder="deepseek-chat" class="mono" />
        </div>

        <div class="setting-group">
          <div class="setting-title-row">
            <span class="setting-title">模型列表</span>
            <a-tag class="key-tag">{{ editForm.models.length }}</a-tag>
            <div class="title-spacer" />
            <a-button size="small" :loading="fetching" @click="fetchModels">
              <template #icon><SyncOutlined /></template>
              获取模型
            </a-button>
            <a-button size="small" style="margin-left: 4px" @click="addModelVisible = true">添加</a-button>
          </div>
          <template v-if="editForm.models.length">
            <a-collapse :bordered="false" :default-active-key="modelGroups.slice(0, 3).map((g) => g.name)">
              <a-collapse-panel v-for="g in modelGroups" :key="g.name" :header="g.name">
                <template #extra>
                  <a-tag class="key-tag">{{ g.models.length }}</a-tag>
                </template>
                <div v-for="m in g.models" :key="m" class="model-row">
                  <span class="model-id mono" :title="m">{{ m }}</span>
                  <span class="model-actions">
                    <button v-if="editForm.model !== m" class="link-btn" @click="setDefaultModel(m)">设为默认</button>
                    <a-tag v-else color="blue" class="default-tag">默认</a-tag>
                    <button class="link-btn danger" @click="removeModel(m)">移除</button>
                  </span>
                </div>
              </a-collapse-panel>
            </a-collapse>
          </template>
          <div v-else class="models-empty">点「获取模型」从供应商拉取（不耗 token），或手动添加</div>
        </div>
      </div>
    </div>

    <!-- 右栏空态 -->
    <div v-else class="provider-detail-panel detail-empty">
      <div class="empty-text">左侧选择或添加一个供应商</div>
    </div>

    <!-- 添加供应商（协议/名称/URL；同协议重复自动加序号） -->
    <a-modal
      v-model:open="addingVisible"
      title="添加供应商"
      ok-text="添加"
      cancel-text="取消"
      width="440px"
      @ok="addProvider"
    >
      <a-form layout="vertical">
        <a-form-item label="供应商" required>
          <a-select
            v-model:value="addForm.protocol"
            :options="BUILTIN_PROVIDERS.map((p) => ({ value: p.value, label: `${p.label} · ${p.baseUrl}` }))"
            @change="onBuiltinChange"
          />
        </a-form-item>
        <a-form-item label="名称" required>
          <a-input v-model:value="addForm.name" placeholder="如：DeepSeek 官方" @press-enter="addProvider" />
        </a-form-item>
        <a-form-item label="Base URL">
          <a-input
            v-model:value="addForm.base_url"
            placeholder="https://api.deepseek.com（Ollama 填 http://127.0.0.1:11434）"
            @press-enter="addProvider"
          />
        </a-form-item>
      </a-form>
    </a-modal>

    <!-- 批量添加模型（逗号分隔） -->
    <a-modal
      v-model:open="addModelVisible"
      title="添加模型"
      ok-text="添加"
      cancel-text="取消"
      @ok="confirmAddModel"
    >
      <p class="modal-hint">支持逗号分隔批量添加</p>
      <a-textarea
        v-model:value="addModelText"
        placeholder="deepseek-chat, deepseek-reasoner"
        :auto-size="{ minRows: 2, maxRows: 5 }"
      />
    </a-modal>

    <!-- 获取模型 → 勾选候选池（远端 ∪ 已有） -->
    <a-modal
      v-model:open="fetchVisible"
      title="选择模型"
      ok-text="确认选择"
      cancel-text="取消"
      width="600px"
      @ok="confirmFetchModels"
    >
      <a-spin :spinning="fetching">
        <a-input-search v-model:value="fetchSearch" placeholder="搜索模型" allow-clear style="margin-bottom: 8px" />
        <div class="fetch-hint">
          已选 {{ selectedFetch.size }} / {{ fetchCandidates.length }} 个模型
          <span v-if="fetchedModels.length" class="fetch-hint-sub">（远端 {{ fetchedModels.length }} 个）</span>
        </div>
        <div class="fetch-model-list">
          <a-collapse :bordered="false" size="small">
            <a-collapse-panel v-for="g in fetchGroups" :key="g.name" :header="g.name">
              <template #extra>
                <a-tag class="key-tag">{{ g.models.length }}</a-tag>
              </template>
              <label v-for="m in g.models" :key="m" class="fetch-row" @click.prevent="toggleFetchModel(m)">
                <a-checkbox :checked="selectedFetch.has(m)" @click.prevent />
                <span class="model-id mono">{{ m }}</span>
              </label>
            </a-collapse-panel>
          </a-collapse>
        </div>
      </a-spin>
    </a-modal>
  </section>
</template>

<style scoped>
.provider-section {
  display: flex;
  height: 100%;
  min-height: 0;
  overflow: hidden;
}

/* ===== 左栏 ===== */
.provider-list-panel {
  display: flex;
  flex-direction: column;
  width: 220px;
  min-width: 220px;
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
.provider-count {
  font-size: 11.5px;
  color: var(--text-tertiary);
}
.provider-list {
  padding: 0 8px;
  overflow-y: auto;
  flex: 1;
}
.provider-card {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px;
  border-radius: 6px;
  border-left: 3px solid transparent;
  cursor: pointer;
  transition: all var(--transition-fast);
}
.provider-card:hover {
  background: var(--color-nav-hover);
}
.provider-card.active {
  border-left-color: var(--color-primary);
  background: var(--color-nav-active);
}
.provider-info {
  flex: 1;
  min-width: 0;
}
.provider-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.provider-card.active .provider-name {
  color: var(--color-primary);
}
.provider-meta {
  margin-top: 2px;
  font-size: 11px;
  color: var(--text-tertiary);
}
.list-empty {
  padding: 30px 0;
  text-align: center;
  font-size: 12px;
  color: var(--text-tertiary);
}
.panel-footer {
  padding: 10px 12px 14px;
}

/* ===== 右栏 ===== */
.provider-detail-panel {
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
.mono {
  font-family: "JetBrains Mono", Consolas, monospace;
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

.setting-group {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.setting-title {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text-secondary);
}
.setting-title-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.title-spacer {
  flex: 1;
}
.key-tag {
  margin: 0;
}
.api-key-row {
  display: flex;
  gap: 8px;
}
.check-btn {
  flex-shrink: 0;
}
.connect-result {
  margin-top: 2px;
  font-size: 12px;
}
.connect-result.success {
  color: var(--color-success);
}
.connect-result.error {
  color: var(--color-danger);
  word-break: break-all;
}
.param-row {
  display: flex;
  gap: 12px;
}
.param-item {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.param-label {
  font-size: 11.5px;
  color: var(--text-tertiary);
}
.full {
  width: 100%;
}

/* 模型列表 */
.model-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 6px 4px;
  border-radius: 6px;
}
.model-row:hover {
  background: var(--color-nav-hover);
}
.model-id {
  flex: 1;
  min-width: 0;
  font-size: 12px;
  color: var(--text-primary);
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.model-actions {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}
.default-tag {
  margin: 0;
}
.link-btn {
  border: none;
  background: transparent;
  color: var(--text-tertiary);
  font-size: 11.5px;
  padding: 2px 6px;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.15s ease;
}
.link-btn:hover {
  color: var(--color-primary);
  background: var(--bg-input);
}
.link-btn.danger:hover {
  color: var(--color-danger);
}
.models-empty {
  font-size: 12px;
  color: var(--text-tertiary);
  padding: 8px 0;
}

/* 空态 */
.detail-empty {
  align-items: center;
  justify-content: center;
}
.empty-text {
  font-size: 13px;
  color: var(--text-tertiary);
}

/* 添加弹窗 */
.add-form {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-top: 4px;
}
.modal-hint {
  margin: 0 0 8px;
  font-size: 11.5px;
  color: var(--text-tertiary);
}
.fetch-hint {
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 8px;
}
.fetch-hint-sub {
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.fetch-model-list {
  max-height: 380px;
  overflow-y: auto;
}
.fetch-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 5px 6px;
  border-radius: 6px;
  cursor: pointer;
}
.fetch-row:hover {
  background: var(--color-nav-hover);
}
</style>
