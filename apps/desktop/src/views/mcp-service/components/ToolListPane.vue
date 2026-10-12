<script setup lang="ts">
/** 工具清单：读取 / 修改 分组，行点击展开参数表（参数/类型/必填/说明） */
import { ref } from "vue";
import { SearchOutlined, EditOutlined, CaretRightOutlined } from "@ant-design/icons-vue";
import { readTools, writeTools } from "../lib/tools";

const expanded = ref(new Set<string>());

function toggle(name: string) {
  if (expanded.value.has(name)) expanded.value.delete(name);
  else expanded.value.add(name);
}
</script>

<template>
  <div class="pane">
    <div class="group">
      <div class="group-label"><SearchOutlined /> 读取（{{ readTools.length }}）</div>
      <template v-for="t in readTools" :key="t.name">
        <div class="row" @click="toggle(t.name)">
          <span class="arrow"><CaretRightOutlined class="icon" :class="{ open: expanded.has(t.name) }" /></span>
          <code class="name">{{ t.name }}</code>
          <span class="desc">{{ t.desc }}</span>
        </div>
        <div v-if="expanded.has(t.name)" class="detail">
          <div v-if="!t.params.length" class="no-param">无参数</div>
          <div v-else class="param-table">
            <div class="p-row p-head"><span>参数</span><span>类型</span><span>必填</span><span>说明</span></div>
            <div v-for="p in t.params" :key="p.name" class="p-row">
              <span class="p-name">{{ p.name }}</span>
              <span class="p-type" :class="'type-' + p.type">{{ p.type }}</span>
              <span class="param-required" :class="p.required ? 'req-yes' : 'req-no'">{{ p.required ? "是" : "否" }}</span>
              <span class="p-desc">{{ p.desc }}</span>
            </div>
          </div>
        </div>
      </template>
    </div>

    <div class="group">
      <div class="group-label write"><EditOutlined /> 修改（{{ writeTools.length }}）</div>
      <template v-for="t in writeTools" :key="t.name">
        <div class="row" @click="toggle(t.name)">
          <span class="arrow"><CaretRightOutlined class="icon" :class="{ open: expanded.has(t.name) }" /></span>
          <code class="name">{{ t.name }}</code>
          <span class="desc">{{ t.desc }}</span>
        </div>
        <div v-if="expanded.has(t.name)" class="detail">
          <div class="param-table">
            <div class="p-row p-head"><span>参数</span><span>类型</span><span>必填</span><span>说明</span></div>
            <div v-for="p in t.params" :key="p.name" class="p-row">
              <span class="p-name">{{ p.name }}</span>
              <span class="p-type" :class="'type-' + p.type">{{ p.type }}</span>
              <span class="param-required" :class="p.required ? 'req-yes' : 'req-no'">{{ p.required ? "是" : "否" }}</span>
              <span class="p-desc">{{ p.desc }}</span>
            </div>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.pane {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.group {
  margin-bottom: 12px;
}
.group-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  font-weight: 600;
  color: var(--color-primary);
  padding: 8px 6px 4px;
  user-select: none;
}
.group-label.write {
  color: var(--color-warning);
}
.row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 9px 8px;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.15s ease;
}
.row:hover {
  background: var(--color-nav-hover);
}
.arrow {
  flex-shrink: 0;
  font-size: 11px;
  color: var(--text-tertiary);
  width: 14px;
}
.arrow .icon {
  transition: transform 0.15s ease;
}
.arrow .icon.open {
  transform: rotate(90deg);
}
.name {
  flex-shrink: 0;
  font-family: "JetBrains Mono", Consolas, monospace;
  font-size: 12.5px;
  color: var(--color-primary);
}
.desc {
  flex: 1;
  min-width: 0;
  font-size: 12px;
  color: var(--text-secondary);
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.detail {
  margin: 0 6px 6px 28px;
}
.no-param {
  font-size: 12px;
  color: var(--text-tertiary);
  padding: 4px 0;
}
.param-table {
  border: 1px solid var(--border-light);
  border-radius: 10px;
  overflow: hidden;
}
.p-row {
  display: grid;
  grid-template-columns: 130px 72px 52px 1fr;
  gap: 10px;
  padding: 7px 12px;
  font-size: 12px;
  align-items: baseline;
}
.p-row + .p-row {
  border-top: 1px solid var(--border-light);
}
.p-head {
  background: var(--bg-input);
  color: var(--text-tertiary);
  font-weight: 500;
  font-size: 11.5px;
}
.p-name {
  font-family: "JetBrains Mono", Consolas, monospace;
  color: var(--text-primary);
}
.p-type {
  font-family: "JetBrains Mono", Consolas, monospace;
  font-size: 11px;
}
.p-type.type-string {
  color: var(--color-success);
}
.p-type.type-integer {
  color: var(--color-warning);
}
.p-type.type-boolean {
  color: var(--color-info, #06b6d4);
}
.req-yes {
  color: var(--color-danger);
  font-weight: 600;
}
.req-no {
  color: var(--text-tertiary);
}
.p-desc {
  color: var(--text-secondary);
  line-height: 1.5;
}
</style>
