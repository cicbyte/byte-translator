<script setup lang="ts">
import { computed, onMounted } from "vue";
import { ConfigProvider, message } from "ant-design-vue";
import zhCN from "ant-design-vue/es/locale/zh_CN";
import { useRoute } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import AppLayout from "@/components/layout/AppLayout.vue";
import { bindTheme, buildAntdToken, colors, useTheme } from "@/themes";
import LookupView from "@/views/lookup/index.vue";

const theme = useTheme();
bindTheme();
const route = useRoute();

const antdTheme = computed(() => buildAntdToken(colors(), theme.appearance.value));
/** 划词小窗等沉浸路由（meta.layout=bare）不走主框架 */
const isBare = computed(() => route.meta.layout === "bare");

onMounted(async () => {
  if (isBare.value) return;
  // 热键注册失败提示（可能被其他应用占用）
  try {
    const failed = await invoke<string>("hotkey_status");
    if (failed) message.warning(`全局热键未注册成功：${failed}（可能被其他应用占用）`);
  } catch {
    /* 提示失败不影响主流程 */
  }
});
</script>

<template>
  <ConfigProvider :theme="antdTheme" :locale="zhCN">
    <LookupView v-if="isBare" />
    <AppLayout v-else />
  </ConfigProvider>
</template>
