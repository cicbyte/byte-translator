<script setup lang="ts">
/**
 * 生词本页：工具栏（关键词/刷新）+ 自绘行式列表。
 *
 * 行结构：序号 + 词条（主行）+ 备注（次行，行内编辑：点击备注进入编辑态）；
 * 右侧收藏时间、hover 浮现删除。
 */
import { computed, onMounted, nextTick, ref } from "vue";
import { useRouter } from "vue-router";
import { message } from "ant-design-vue";
import { ReloadOutlined, DeleteOutlined, CheckOutlined } from "@ant-design/icons-vue";
import { invoke } from "@tauri-apps/api/core";
import EmptyState from "@/components/common/EmptyState.vue";

interface WordRow {
  id: number;
  text: string;
  note: string;
  created_at: string;
}

const router = useRouter();
const keyword = ref("");
const rows = ref<WordRow[]>([]);
const total = ref(0);
const page = ref(1);
const size = 20;
const loading = ref(false);
const editingId = ref<number | null>(null);
const editingNote = ref("");
const editInput = ref<HTMLInputElement | null>(null);

const pagination = computed(() => ({
  current: page.value,
  pageSize: size,
  total: total.value,
  size: "small",
  showSizeChanger: false,
  onChange: (p: number) => load(p),
}));

async function load(p = 1) {
  loading.value = true;
  try {
    const v = await invoke<{ total: number; list: WordRow[] }>("wordbook_list", {
      keyword: keyword.value || null,
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

async function startEdit(row: WordRow) {
  editingId.value = row.id;
  editingNote.value = row.note;
  await nextTick();
  editInput.value?.focus();
}

async function saveNote(row: WordRow) {
  if (editingId.value !== row.id) return;
  await invoke("wordbook_add", { text: row.text, note: editingNote.value });
  editingId.value = null;
  message.success("已更新");
  await load(page.value);
}

async function remove(row: WordRow) {
  await invoke("wordbook_remove", { id: row.id });
  message.success("已删除");
  await load(page.value);
}

function onNoteKeydown(e: KeyboardEvent, row: WordRow) {
  if (e.key === "Enter") saveNote(row);
  if (e.key === "Escape") editingId.value = null;
}

onMounted(() => load(1));
</script>

<template>
  <div class="page">
    <div class="page-toolbar">
      <a-input-search
        v-model:value="keyword"
        placeholder="过滤词条"
        style="width: 240px"
        allow-clear
        @search="load(1)"
      />
      <a-button @click="load(page)">
        <template #icon><ReloadOutlined /></template>
        刷新
      </a-button>
    </div>

    <div class="page-scroll list-scroll">
      <a-spin :spinning="loading">
        <EmptyState
          v-if="!loading && rows.length === 0"
          title="生词本还是空的"
          description="翻译结果页点「收藏生词」，或者直接双击下方添加"
        >
          <template #action>
            <a-button type="primary" size="small" @click="router.push('/')">去翻译</a-button>
          </template>
        </EmptyState>
        <template v-else>
          <div v-for="row in rows" :key="row.id" class="wb-row">
            <span class="row-index">{{ row.id }}</span>
            <div class="row-main">
              <div class="row-title">{{ row.text }}</div>
              <div v-if="editingId !== row.id" class="row-note" @click.stop="startEdit(row)">
                {{ row.note || "点击添加备注…" }}
              </div>
              <div v-else class="note-edit">
                <input
                  ref="editInput"
                  v-model="editingNote"
                  class="note-input"
                  @keydown="onNoteKeydown($event, row)"
                  @blur="saveNote(row)"
                />
                <CheckOutlined class="note-ok" @click.stop="saveNote(row)" />
              </div>
            </div>
            <div class="row-meta">
              <span class="meta-time">{{ row.created_at }}</span>
              <button class="row-action danger" title="删除" @click="remove(row)">
                <DeleteOutlined />
              </button>
            </div>
          </div>
        </template>
      </a-spin>
    </div>

    <div class="page-footer">
      <a-pagination v-bind="pagination" />
      <span class="total-hint">共 {{ total }} 词</span>
    </div>
  </div>
</template>

<style scoped>
.list-scroll {
  padding-top: 6px;
}

.wb-row {
  display: flex;
  align-items: flex-start;
  gap: 14px;
  padding: 13px 12px;
  border-radius: 8px;
  transition: background 0.15s ease;
}
.wb-row + .wb-row {
  border-top: 1px solid var(--border-light);
}
.wb-row:hover {
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
.row-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.row-note {
  font-size: 12.5px;
  color: var(--text-tertiary);
  cursor: text;
  border-radius: 4px;
  padding: 1px 4px;
  margin-left: -4px;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.row-note:hover {
  color: var(--text-secondary);
  background: var(--bg-input);
}
.note-edit {
  display: flex;
  align-items: center;
  gap: 6px;
}
.note-input {
  flex: 1;
  min-width: 0;
  border: none;
  outline: none;
  background: var(--bg-input);
  border-radius: 6px;
  padding: 3px 8px;
  font-size: 12.5px;
  color: var(--text-primary);
  font-family: inherit;
}
.note-ok {
  font-size: 12px;
  color: var(--color-primary);
  cursor: pointer;
}

.row-meta {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 14px;
  padding-top: 2px;
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
.wb-row:hover .row-action {
  opacity: 1;
}
.row-action:hover {
  background: var(--bg-input);
  color: var(--color-primary);
}
.row-action.danger:hover {
  color: var(--color-danger);
}

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
