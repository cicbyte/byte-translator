<script setup lang="ts">
/**
 * 历史页：工具栏（引擎筛选/关键词/刷新）+ 自绘行式列表。
 *
 * 行结构：序号（等宽）+ 原文（主行）+ 译文摘要（次行截断）；
 * 右侧引擎胶囊、语言对、时间、hover 浮现「去翻译」。
 * 点击行展开完整词典详情（释义/例句）。
 */
import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { ReloadOutlined, ArrowRightOutlined } from "@ant-design/icons-vue";
import { invoke } from "@tauri-apps/api/core";
import EmptyState from "@/components/common/EmptyState.vue";

interface HistoryRow {
  id: number;
  text: string;
  engine: string;
  from_lang: string;
  to_lang: string;
  created_at: string;
  result_json: string;
}

interface TranslationResult {
  translation: string;
  phonetic: string[];
  parts: { pos: string; mean: string }[];
  examples: { orig: string; trans: string }[];
}

const router = useRouter();
const keyword = ref("");
const engine = ref("");
const rows = ref<HistoryRow[]>([]);
const total = ref(0);
const page = ref(1);
const size = 20;
const loading = ref(false);
const expandedId = ref<number | null>(null);

const pagination = computed(() => ({
  current: page.value,
  pageSize: size,
  total: total.value,
  size: "small",
  showSizeChanger: false,
  onChange: (p: number) => load(p),
}));

function detailOf(row: HistoryRow): TranslationResult | null {
  try {
    return JSON.parse(row.result_json) as TranslationResult;
  } catch {
    return null;
  }
}

function summaryOf(row: HistoryRow): string {
  const d = detailOf(row);
  if (!d) return "";
  if (d.translation) return d.translation;
  return d.parts.map((p) => p.mean).join("；");
}

async function load(p = 1) {
  loading.value = true;
  try {
    const v = await invoke<{ total: number; list: HistoryRow[] }>("history_list", {
      keyword: keyword.value || null,
      engine: engine.value || null,
      page: p,
      size,
    });
    rows.value = v.list;
    total.value = v.total;
    page.value = p;
  } finally {
    loading.value = false;
  }
}

function toggleExpand(id: number) {
  expandedId.value = expandedId.value === id ? null : id;
}

onMounted(() => load(1));
</script>

<template>
  <div class="page">
    <!-- 工具栏：左筛选右刷新 -->
    <div class="page-toolbar">
      <a-space>
        <a-select v-model:value="engine" style="width: 120px" @change="load(1)">
          <a-select-option value="">全部引擎</a-select-option>
          <a-select-option value="iciba">词典</a-select-option>
          <a-select-option value="llm">LLM</a-select-option>
        </a-select>
        <a-input-search
          v-model:value="keyword"
          placeholder="搜索原文"
          style="width: 240px"
          allow-clear
          @search="load(1)"
        />
      </a-space>
      <a-button @click="load(page)">
        <template #icon><ReloadOutlined /></template>
        刷新
      </a-button>
    </div>

    <!-- 自绘行式列表 -->
    <div class="page-scroll list-scroll">
      <a-spin :spinning="loading">
        <EmptyState
          v-if="!loading && rows.length === 0"
          title="暂无翻译历史"
          description="去翻译页翻点什么，结果会自动留在这里"
        />
        <template v-else>
          <div
            v-for="row in rows"
            :key="row.id"
            class="hist-row"
            @click="toggleExpand(row.id)"
          >
            <span class="row-index">{{ row.id }}</span>
            <div class="row-main">
              <div class="row-title-line">
                <span class="row-title">{{ row.text }}</span>
                <span class="engine-chip" :class="row.engine">{{ row.engine }}</span>
              </div>
              <div class="row-sub">{{ summaryOf(row) || "（无译文）" }}</div>
              <!-- 展开态：词典详情 -->
              <div v-if="expandedId === row.id && detailOf(row)" class="row-detail">
                <div v-for="(p, i) in detailOf(row)!.parts" :key="i" class="detail-part">
                  <span class="part-pos">{{ p.pos }}</span>{{ p.mean }}
                </div>
                <div v-for="(ex, i) in detailOf(row)!.examples" :key="'e' + i" class="detail-ex">
                  {{ ex.orig }}<br /><span class="ex-trans">{{ ex.trans }}</span>
                </div>
              </div>
            </div>
            <div class="row-meta">
              <span class="meta-langs">{{ row.from_lang }} → {{ row.to_lang }}</span>
              <span class="meta-time">{{ row.created_at }}</span>
              <button class="row-action" title="去翻译" @click.stop="router.push({ path: '/', query: { text: row.text } })">
                <ArrowRightOutlined />
              </button>
            </div>
          </div>
        </template>
      </a-spin>
    </div>

    <div class="page-footer">
      <a-pagination v-bind="pagination" />
      <span class="total-hint">共 {{ total }} 条</span>
    </div>
  </div>
</template>

<style scoped>
.list-scroll {
  padding-top: 6px;
}

/* ===== 行：细分隔线 + hover 浮现浅色背景 ===== */
.hist-row {
  display: flex;
  align-items: flex-start;
  gap: 14px;
  padding: 13px 12px;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.15s ease;
}
.hist-row + .hist-row {
  border-top: 1px solid var(--border-light);
}
.hist-row:hover {
  background: var(--color-nav-hover);
}

.row-index {
  width: 40px;
  flex-shrink: 0;
  text-align: right;
  font-family: "JetBrains Mono", Consolas, monospace;
  font-size: 12.5px;
  color: var(--text-tertiary);
  font-variant-numeric: tabular-nums;
  user-select: none;
  padding-top: 2px;
}
.row-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.row-title-line {
  display: flex;
  align-items: center;
  gap: 10px;
}
.row-title {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.engine-chip {
  flex-shrink: 0;
  font-size: 10.5px;
  padding: 1px 8px;
  border-radius: 999px;
  border: 1px solid var(--border-base);
  color: var(--text-tertiary);
  user-select: none;
}
.engine-chip.llm {
  color: var(--color-primary);
  border-color: color-mix(in srgb, var(--color-primary) 32%, transparent);
  background: color-mix(in srgb, var(--color-primary) 8%, transparent);
}
.row-sub {
  font-size: 12.5px;
  color: var(--text-tertiary);
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

/* 展开详情 */
.row-detail {
  margin-top: 6px;
  padding: 10px 12px;
  border-left: 2px solid color-mix(in srgb, var(--color-primary) 40%, transparent);
  background: var(--bg-input);
  border-radius: 0 8px 8px 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.detail-part {
  font-size: 13px;
  color: var(--text-secondary);
}
.part-pos {
  color: var(--color-primary);
  margin-right: 8px;
  font-weight: 500;
}
.detail-ex {
  font-size: 12.5px;
  color: var(--text-secondary);
}
.ex-trans {
  color: var(--text-tertiary);
  font-size: 12px;
}

/* 右侧元信息 */
.row-meta {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 14px;
  padding-top: 2px;
}
.meta-langs {
  font-size: 12px;
  color: var(--text-tertiary);
  font-variant-numeric: tabular-nums;
}
.meta-time {
  font-size: 12px;
  color: var(--text-tertiary);
  font-variant-numeric: tabular-nums;
}
.row-action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-tertiary);
  cursor: pointer;
  opacity: 0;
  transition: all 0.15s ease;
}
.hist-row:hover .row-action {
  opacity: 1;
}
.row-action:hover {
  background: var(--bg-input);
  color: var(--color-primary);
}

/* 底栏 */
.page-footer {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-top: 4px;
}
.total-hint {
  font-size: 12px;
  color: var(--text-tertiary);
}
</style>
