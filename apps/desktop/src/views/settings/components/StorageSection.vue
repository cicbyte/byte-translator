<script setup lang="ts">
/** 数据目录：路径展示 + 复制；说明与 CLI 共用 */
import { onMounted, ref } from "vue";
import { message } from "ant-design-vue";
import { CopyOutlined } from "@ant-design/icons-vue";
import { invoke } from "@tauri-apps/api/core";

const dataRoot = ref("");

onMounted(async () => {
  const info = await invoke<{ dataRoot: string }>("app_info");
  dataRoot.value = info.dataRoot;
});

async function copy() {
  await navigator.clipboard.writeText(dataRoot.value);
  message.success("已复制");
}
</script>

<template>
  <section class="setting-section">
    <header class="section-head">
      <div>
        <h3 class="section-title">数据</h3>
        <p class="section-desc">历史 / 生词本 / 供应商 / 缓存都在本地 SQLite；与 CLI、REST、MCP 共用同一目录</p>
      </div>
      <a-button size="small" @click="copy">
        <template #icon><CopyOutlined /></template>
        复制路径
      </a-button>
    </header>
    <code class="root-path">{{ dataRoot }}</code>
    <p class="note">
      覆盖位置：<code>CICBYTE_HOME</code> 环境变量或 <code>btr --app-root</code>；备份整个目录即可迁移。
    </p>
  </section>
</template>

<style scoped>
.root-path {
  display: block;
  font-family: "JetBrains Mono", Consolas, monospace;
  font-size: 12.5px;
  color: var(--text-secondary);
  background: var(--bg-input);
  border: 1px solid var(--border-light);
  border-radius: 8px;
  padding: 8px 12px;
  user-select: all;
  margin-top: 4px;
}
.note {
  margin: 8px 0 0;
  font-size: 11.5px;
  color: var(--text-tertiary);
  line-height: 1.7;
}
.note code {
  font-family: "JetBrains Mono", Consolas, monospace;
  background: var(--bg-input);
  padding: 1px 5px;
  border-radius: 4px;
}
</style>
