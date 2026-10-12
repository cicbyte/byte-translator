<script setup lang="ts">
/**
 * MCP 服务页：pane-head + 二级 tab（服务管理 / 工具列表 / 客户端接入）。
 */
import { computed, ref } from "vue";
import { message } from "ant-design-vue";
import { ClusterOutlined } from "@ant-design/icons-vue";
import { invoke } from "@tauri-apps/api/core";
import ServicePanel from "@/components/servers/ServicePanel.vue";
import ToolListPane from "./components/ToolListPane.vue";

const tab = ref<"server" | "tools" | "config">("server");
const port = ref(8324);
const mcpKey = ref("");

const httpEndpoint = computed(() => `http://127.0.0.1:${port.value}/mcp`);

const stdioConfig = JSON.stringify(
  {
    mcpServers: {
      btr: {
        command: "C:/path/to/btr.exe",
        args: ["mcp"],
      },
    },
  },
  null,
  2,
);

async function load() {
  const svc = await invoke<{ mcp?: { running: boolean; port: number } }>("service_status");
  port.value = svc.mcp?.port || 8324;
  const keys = await invoke<{ mcp: string }>("api_keys");
  mcpKey.value = keys.mcp;
}

async function copy(text: string) {
  await navigator.clipboard.writeText(text);
  message.success("已复制");
}

load();
</script>

<template>
  <div class="page">
    <div class="pane-head">
      <div>
        <h1 class="page-title">MCP 服务</h1>
        <p class="pane-desc">把翻译能力暴露为 AI 客户端（Claude Desktop / Cursor 等）的工具源。</p>
      </div>
      <ClusterOutlined class="head-icon" />
    </div>

    <div class="sub-tabs">
      <button class="sub-tab" :class="{ active: tab === 'server' }" @click="tab = 'server'">服务管理</button>
      <button class="sub-tab" :class="{ active: tab === 'tools' }" @click="tab = 'tools'">工具列表（5）</button>
      <button class="sub-tab" :class="{ active: tab === 'config' }" @click="tab = 'config'; load()">客户端接入</button>
    </div>

    <div class="page-scroll">
      <!-- 服务管理（HTTP 通道；stdio 随客户端拉起无需开关） -->
      <div v-if="tab === 'server'" class="sub-pane">
        <ServicePanel kind="mcp" :default-port="8324" />
        <p class="note">
          此开关只管 HTTP 通道（多客户端共享）；stdio 模式由 AI 客户端直接拉起（见「客户端接入」），不受影响。
        </p>
      </div>

      <!-- 工具列表 -->
      <ToolListPane v-else-if="tab === 'tools'" />

      <!-- 客户端接入 -->
      <div v-else class="sub-pane">
        <div class="block">
          <div class="block-title-row">
            <div class="block-title">方式一：stdio（推荐——客户端拉起命令，无需本页开关）</div>
            <a-button size="small" @click="copy(stdioConfig)">复制</a-button>
          </div>
          <p class="note">适用于 AI 客户端与本机同机。把下面的配置加进客户端的 mcpServers：</p>
          <pre class="cmd">{{ stdioConfig }}</pre>
          <p class="note">
            Claude Desktop 配置文件（Windows）：%APPDATA%\Claude\claude_desktop_config.json；
            Cursor：.cursor/mcp.json。command 换成 btr 实际路径。
          </p>
        </div>

        <div class="block">
          <div class="block-title">方式二：HTTP（多客户端共享一个实例）</div>
          <p class="note">
            先在「服务管理」启动 HTTP 通道，客户端以 Streamable HTTP 接入
            <code>{{ httpEndpoint }}</code>，请求头带 Key：
          </p>
          <div class="cmd-row">
            <pre class="cmd">X-API-Key: {{ mcpKey }}</pre>
            <a-button size="small" @click="copy(mcpKey)">复制 Key</a-button>
          </div>
          <p class="note">此 Key 与 REST 通道独立（暴露面不同，轮换互不影响）。</p>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.pane-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-shrink: 0;
}
.page-title {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
}
.pane-desc {
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--text-tertiary);
  line-height: 1.6;
}
.head-icon {
  font-size: 28px;
  color: var(--color-primary);
  opacity: 0.7;
}

.sub-tabs {
  display: flex;
  gap: 4px;
  border-bottom: 1px solid var(--border-light);
  flex-shrink: 0;
}
.sub-tab {
  padding: 8px 16px;
  border: none;
  border-bottom: 2px solid transparent;
  background: transparent;
  color: var(--text-tertiary);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  margin-bottom: -1px;
}
.sub-tab:hover {
  color: var(--text-primary);
}
.sub-tab.active {
  color: var(--color-primary);
  border-bottom-color: var(--color-primary);
  font-weight: 600;
}
.sub-pane {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.block {
  border: 1px solid var(--border-light);
  border-radius: 8px;
  padding: 12px 16px;
}
.block-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}
.block-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
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
.cmd-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 8px;
}
.cmd {
  flex: 1;
  min-width: 0;
  background: var(--bg-input);
  border: 1px solid var(--border-light);
  border-radius: 8px;
  padding: 8px 12px;
  font-family: "JetBrains Mono", Consolas, monospace;
  font-size: 12px;
  line-height: 1.7;
  color: var(--text-secondary);
  overflow-x: auto;
  user-select: all;
  margin: 8px 0 0;
  white-space: pre-wrap;
}
.cmd-row .cmd {
  margin: 0;
}

</style>
