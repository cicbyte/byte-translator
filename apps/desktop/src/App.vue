<script setup lang="ts">
import { computed } from "vue";
import { ConfigProvider } from "ant-design-vue";
import zhCN from "ant-design-vue/es/locale/zh_CN";
import { useRoute } from "vue-router";
import AppLayout from "@/components/layout/AppLayout.vue";
import { bindTheme, buildAntdToken, colors, useTheme } from "@/themes";
import LookupView from "@/views/lookup/index.vue";

const theme = useTheme();
bindTheme();
const route = useRoute();

const antdTheme = computed(() => buildAntdToken(colors(), theme.appearance.value));
/** 划词小窗等沉浸路由（meta.layout=bare）不走主框架 */
const isBare = computed(() => route.meta.layout === "bare");
</script>

<template>
  <ConfigProvider :theme="antdTheme" :locale="zhCN">
    <LookupView v-if="isBare" />
    <AppLayout v-else />
  </ConfigProvider>
</template>
