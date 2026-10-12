<script setup lang="ts">
/**
 * 服务管理面板：行式布局（状态 / 监听端口+启停 / API Key 编辑+轮换）。
 * REST 与 MCP 两通道共用；停止态可编辑端口与 Key，运行态锁定。
 */
import { ref, watch } from "vue";
import { message } from "ant-design-vue";
import { invoke } from "@tauri-apps/api/core";

const props = defineProps<{
  kind: "rest" | "mcp";
  defaultPort: number;
}>();

const running = ref(false);
const host = ref<"127.0.0.1" | "0.0.0.0">("127.0.0.1");
const port = ref(props.defaultPort);
const portBeforeEdit = ref(props.defaultPort);
const toggling = ref(false);
const apiKey = ref("");
const keyInput = ref("");
const keySaving = ref(false);
const revealed = ref(false);

async function load() {
  const svc = await invoke<{
    rest?: { running: boolean; host: string; port: number };
    mcp?: { running: boolean; host: string; port: number };
  }>("service_status");
  const s = svc[props.kind];
  running.value = !!s?.running;
  if (s?.port) port.value = s.port;
  if (s?.host === "0.0.0.0") host.value = "0.0.0.0";
  const keys = await invoke<{ rest: string; mcp: string }>("api_keys");
  apiKey.value = keys[props.kind];
  keyInput.value = "";
  revealed.value = false;
}

async function toggle() {
  toggling.value = true;
  try {
    await invoke("service_toggle", { kind: props.kind, on: !running.value, port: port.value, host: host.value });
    await load();
    message.success(running.value ? `已启动（${host.value}:${port.value}）` : "已停止");
  } catch (e) {
    await load();
    message.error(String(e));
  } finally {
    toggling.value = false;
  }
}

async function saveKey() {
  const k = keyInput.value.trim();
  if (!k) {
    message.warning("Key 不能为空");
    return;
  }
  keySaving.value = true;
  try {
    await invoke("api_key_set", { kind: props.kind, key: k });
    message.success("已保存自定义 Key");
    await load();
  } catch (e) {
    message.error(String(e));
  } finally {
    keySaving.value = false;
  }
}

async function rotate() {
  try {
    await invoke<string>("api_key_rotate", { kind: props.kind });
    message.success("已轮换（旧 Key 立即失效）");
    await load();
  } catch (e) {
    message.error(String(e));
  }
}

async function copy(text: string) {
  await navigator.clipboard.writeText(text);
  message.success("已复制");
}

function onPortInput(e: Event) {
  const el = e.target as HTMLInputElement;
  const digits = el.value.replace(/\D/g, "").slice(0, 5);
  if (el.value !== digits) el.value = digits;
  port.value = digits ? Number(digits) : 0;
}

function onPortBlur() {
  if (!port.value || port.value < 1024 || port.value > 65535) {
    port.value = portBeforeEdit.value || props.defaultPort;
  }
}

watch(() => props.kind, load, { immediate: true });
</script>

<template>
  <div class="server-panel">
    <div class="row">
      <span class="row-label">状态</span>
      <div class="row-value">
        <span class="dot" :class="{ on: running }"></span>
        <span :class="running ? 'state-on' : 'state-off'">{{ running ? "运行中" : "已停止" }}</span>
      </div>
    </div>

    <div class="row">
      <span class="row-label">监听</span>
      <div class="row-value">
        <a-select
          v-model:value="host"
          size="small"
          :disabled="running"
          style="width: 108px"
          :options="[
            { value: '127.0.0.1', label: '127.0.0.1' },
            { value: '0.0.0.0', label: '0.0.0.0' },
          ]"
        />
        <span class="colon">:</span>
        <input
          :value="port"
          type="text"
          inputmode="numeric"
          class="port-field"
          :disabled="running"
          @focus="portBeforeEdit = port"
          @input="onPortInput"
          @blur="onPortBlur"
        />
        <button class="toggle-btn" :class="running ? 'stop' : 'start'" :disabled="toggling" @click="toggle">
          {{ toggling ? "…" : running ? "停止" : "启动" }}
        </button>
      </div>
    </div>

    <div class="row">
      <span class="row-label">API Key</span>
      <div class="row-value">
        <template v-if="!running">
          <input
            v-model="keyInput"
            class="key-field"
            :placeholder="revealed ? apiKey : '••••••••••••••••（留空不改，点右侧查看）'"
          />
          <button class="mini-btn" @click="revealed = !revealed">{{ revealed ? "隐藏" : "查看" }}</button>
          <button class="mini-btn" :disabled="keySaving" @click="saveKey">保存</button>
          <button class="mini-btn danger" @click="rotate">轮换</button>
          <button class="mini-btn" @click="copy(apiKey)">复制</button>
        </template>
        <template v-else>
          <code class="key-mask">{{ revealed ? apiKey : "•".repeat(32) }}</code>
          <button class="mini-btn" @click="revealed = !revealed">{{ revealed ? "隐藏" : "查看" }}</button>
          <button class="mini-btn" @click="copy(apiKey)">复制</button>
        </template>
      </div>
    </div>

    <a-alert
      v-if="host === '0.0.0.0'"
      type="warning"
      show-icon
      style="margin-top: 2px"
      message="绑定所有网卡：局域网内其它机器可访问（防火墙放行后生效），请求必须携带 API Key"
    />
    <p class="panel-note">
      鉴权恒开：回环免 Key（本机调试通道），非回环强制 Key。停止态才能改监听、端口与 Key。
    </p>
  </div>
</template>

<style scoped>
.server-panel {
  border: 1px solid var(--border-light);
  border-radius: 10px;
  padding: 14px 16px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.row {
  display: flex;
  align-items: center;
  gap: 14px;
}
.row-label {
  width: 64px;
  flex-shrink: 0;
  font-size: 12.5px;
  color: var(--text-secondary);
}
.row-value {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  min-width: 0;
}
.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--border-base);
}
.dot.on {
  background: var(--color-success);
}
.state-on {
  font-size: 13px;
  color: var(--color-success);
}
.state-off {
  font-size: 13px;
  color: var(--text-tertiary);
}
.host-static {
  font-family: "JetBrains Mono", Consolas, monospace;
  font-size: 12.5px;
  color: var(--text-secondary);
}
.colon {
  color: var(--text-tertiary);
}
.port-field {
  width: 72px;
  border: 1px solid var(--border-base);
  border-radius: 6px;
  background: var(--bg-input);
  color: var(--text-primary);
  padding: 4px 8px;
  font-size: 12.5px;
  font-family: "JetBrains Mono", Consolas, monospace;
  outline: none;
}
.port-field:focus {
  border-color: var(--color-primary);
}
.port-field:disabled {
  opacity: 0.7;
}
.toggle-btn {
  border: none;
  border-radius: 6px;
  padding: 5px 16px;
  font-size: 12.5px;
  cursor: pointer;
  transition: all 0.15s ease;
  margin-left: 6px;
}
.toggle-btn.start {
  background: var(--color-primary);
  color: var(--hero-cta-text);
}
.toggle-btn.start:hover {
  filter: brightness(1.1);
}
.toggle-btn.stop {
  background: var(--bg-input);
  border: 1px solid var(--border-base);
  color: var(--text-secondary);
}
.toggle-btn.stop:hover {
  border-color: var(--color-danger);
  color: var(--color-danger);
}
.toggle-btn:disabled {
  opacity: 0.6;
  cursor: default;
}
.key-field {
  flex: 1;
  min-width: 0;
  border: 1px solid var(--border-base);
  border-radius: 6px;
  background: var(--bg-input);
  color: var(--text-primary);
  padding: 4px 10px;
  font-size: 12px;
  font-family: "JetBrains Mono", Consolas, monospace;
  outline: none;
}
.key-field:focus {
  border-color: var(--color-primary);
}
.key-mask {
  font-family: "JetBrains Mono", Consolas, monospace;
  font-size: 12px;
  color: var(--text-secondary);
  letter-spacing: 0.06em;
  user-select: all;
}
.mini-btn {
  flex-shrink: 0;
  border: 1px solid var(--border-base);
  border-radius: 6px;
  background: var(--bg-input);
  color: var(--text-secondary);
  font-size: 11.5px;
  padding: 3px 10px;
  cursor: pointer;
  transition: all 0.15s ease;
}
.mini-btn:hover {
  border-color: var(--color-primary);
  color: var(--color-primary);
}
.mini-btn.danger:hover {
  border-color: var(--color-warning);
  color: var(--color-warning);
}
.panel-note {
  margin: 2px 0 0;
  font-size: 11.5px;
  color: var(--text-secondary);
  line-height: 1.6;
}
</style>
