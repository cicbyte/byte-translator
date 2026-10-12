<script setup lang="ts">
/**
 * REST 服务页：pane-head + 二级 tab（服务管理 / 接入 / API 文档）。
 */
import { computed, ref } from "vue";
import { message } from "ant-design-vue";
import { ApiOutlined, ExportOutlined } from "@ant-design/icons-vue";
import { invoke } from "@tauri-apps/api/core";
import ServicePanel from "@/components/servers/ServicePanel.vue";

const tab = ref<"server" | "access" | "docs">("server");
const port = ref(8321);
const restKey = ref("");
const svcRunning = ref(false);

const docsLoading = ref(false);

const curlSample = computed(
  () => `curl -X POST http://127.0.0.1:${port.value}/api/v1/translate \\
  -H "X-API-Key: ${restKey.value}" \\
  -H "Content-Type: application/json" \\
  -d '{"text": "hello", "stream": true}'`,
);

async function load() {
  const svc = await invoke<{ rest?: { running: boolean; port: number } }>("service_status");
  svcRunning.value = !!svc.rest?.running;
  port.value = svc.rest?.port || 8321;
  const keys = await invoke<{ rest: string }>("api_keys");
  restKey.value = keys.rest;
}

async function copy(text: string) {
  await navigator.clipboard.writeText(text);
  message.success("已复制");
}

function openSwagger() {
  window.open(`http://127.0.0.1:${port.value}/api/v1/docs`, "_blank");
}

load();
</script>

<template>
  <div class="page">
    <div class="pane-head">
      <div>
        <h1 class="page-title">REST API 服务</h1>
        <p class="pane-desc">标准 HTTP + JSON 端点，面向远程 CLI 与第三方脚本对接。</p>
      </div>
      <ApiOutlined class="head-icon" />
    </div>

    <div class="sub-tabs">
      <button class="sub-tab" :class="{ active: tab === 'server' }" @click="tab = 'server'">服务管理</button>
      <button class="sub-tab" :class="{ active: tab === 'access' }" @click="tab = 'access'">接入</button>
      <button
        class="sub-tab"
        :class="{ active: tab === 'docs' }"
        @click="if (tab !== 'docs') { tab = 'docs'; load(); docsLoading = true; }"
      >
        API 文档
      </button>
    </div>

    <div class="page-scroll sub-pane">
      <!-- 服务管理 -->
      <div v-if="tab === 'server'" class="sub-pane">
        <ServicePanel kind="rest" :default-port="8321" />
      </div>

      <!-- 接入：步骤卡 -->
      <div v-else-if="tab === 'access'" class="sub-pane">
        <div class="step-card">
          <div class="step-no">1</div>
          <div class="step-body">
            <div class="step-title">在主机启动服务</div>
            <p class="step-desc">本机使用：直接在「服务管理」打开开关（仅监听 127.0.0.1）。</p>
            <p class="step-desc">供局域网其它电脑访问时，用 CLI 绑定所有网卡：</p>
            <div class="cmd-row">
              <pre class="cmd">btr serve --host 0.0.0.0</pre>
              <a-button size="small" @click="copy('btr serve --host 0.0.0.0')">复制</a-button>
            </div>
            <p class="note">非回环绑定强制要求 API Key（下一步获取）；Windows 首次启动如弹出防火墙确认，请放行。</p>
          </div>
        </div>

        <div class="step-card">
          <div class="step-no">2</div>
          <div class="step-body">
            <div class="step-title">在其它电脑配置环境变量</div>
            <p class="step-desc">
              客户端机无需本地数据，只需 <code>btr</code> 可执行文件。把下面两项写入 shell 配置（或每次命令前携带）：
            </p>
            <div class="cmd-row">
              <pre class="cmd">export BTR_REMOTE=http://&lt;主机IP&gt;:{{ port }}
export BTR_API_KEY=&lt;API Key&gt;</pre>
              <a-button size="small" @click="copy(restKey)">复制 Key</a-button>
            </div>
            <p class="note">API Key 在「服务管理」tab 查看；轮换后所有客户端需同步更新。</p>
          </div>
        </div>

        <div class="step-card">
          <div class="step-no">3</div>
          <div class="step-body">
            <div class="step-title">验证连通</div>
            <div class="cmd-row">
              <pre class="cmd">btr --json hello</pre>
              <a-button size="small" @click="copy('btr --json hello')">复制</a-button>
            </div>
            <p class="step-desc">能返回翻译结果即接入成功。之后所有 <code>btr</code> 命令照常使用，数据实时读写主机。</p>
          </div>
        </div>

        <div class="block">
          <div class="block-title-row">
            <div class="block-title">快速调用（SSE 流式）</div>
            <a-button size="small" @click="copy(curlSample)">复制</a-button>
          </div>
          <pre class="cmd">{{ curlSample }}</pre>
        </div>

        <div class="block">
          <div class="block-title">远程模式能力范围</div>
          <ul class="scope-list">
            <li>支持：翻译、历史检索、生词本增删查</li>
            <li>仅主机本地：供应商管理（涉密写操作）</li>
          </ul>
        </div>
      </div>

      <!-- API 文档：运行时内嵌 Swagger UI，未运行空态 -->
      <div v-else class="sub-pane docs-pane">
        <template v-if="svcRunning">
          <div class="swagger-toolbar">
            <span class="swagger-label">交互式 API 文档（Swagger UI）</span>
            <a-button size="small" @click="openSwagger">浏览器中打开</a-button>
          </div>
          <div class="swagger-frame-wrap">
            <div v-if="docsLoading" class="swagger-loading">
              <a-spin />
              <span>正在加载 Swagger UI…</span>
            </div>
            <iframe
              :src="`http://127.0.0.1:${port}/api/v1/docs`"
              class="swagger-frame"
              @load="docsLoading = false"
              @error="docsLoading = false"
            />
          </div>
        </template>
        <div v-else class="swagger-empty">
          <p class="note">启动服务后，此处显示交互式 API 文档。</p>
          <p class="note">也可在浏览器打开 <code>http://localhost:{{ port || 8321 }}/api/v1/docs</code></p>
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

/* 步骤卡 */
.step-card {
  display: flex;
  gap: 14px;
  border: 1px solid var(--border-light);
  border-radius: 8px;
  padding: 14px 16px;
}
.step-no {
  flex-shrink: 0;
  width: 26px;
  height: 26px;
  border-radius: 50%;
  background: color-mix(in srgb, var(--color-primary) 12%, transparent);
  color: var(--color-primary);
  font-size: 13px;
  font-weight: 600;
  display: flex;
  align-items: center;
  justify-content: center;
}
.step-body {
  flex: 1;
  min-width: 0;
}
.step-title {
  font-size: 13.5px;
  font-weight: 600;
  color: var(--text-primary);
}
.step-desc {
  margin: 6px 0 0;
  font-size: 12.5px;
  color: var(--text-secondary);
  line-height: 1.7;
}
.step-desc code {
  font-family: "JetBrains Mono", Consolas, monospace;
  background: var(--bg-input);
  padding: 1px 5px;
  border-radius: 4px;
}
.note {
  margin: 8px 0 0;
  font-size: 11.5px;
  color: var(--text-tertiary);
  line-height: 1.6;
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
  margin: 0;
  white-space: pre-wrap;
}

/* 块卡 */
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
.scope-list {
  margin: 8px 0 0;
  padding-left: 18px;
  font-size: 12.5px;
  color: var(--text-secondary);
  line-height: 1.9;
}

/* 端点行 */
.endpoint-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 0;
}
.endpoint-row + .endpoint-row {
  border-top: 1px solid var(--border-light);
}
.ep-method {
  width: 76px;
  flex-shrink: 0;
  font-family: "JetBrains Mono", Consolas, monospace;
  font-size: 11px;
  color: var(--color-primary);
}
.ep-path {
  font-family: "JetBrains Mono", Consolas, monospace;
  font-size: 12.5px;
  color: var(--text-primary);
}
.ep-desc {
  flex: 1;
  min-width: 0;
  font-size: 12px;
  color: var(--text-tertiary);
  text-align: right;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
</style>
