<script setup lang="ts">
import { computed } from "vue";
import { ConfigProvider } from "ant-design-vue";
import zhCN from "ant-design-vue/es/locale/zh_CN";
import AppLayout from "@/components/layout/AppLayout.vue";
import { bindTheme, buildAntdToken, colors, useTheme } from "@/themes";

const theme = useTheme();
bindTheme();

const antdTheme = computed(() => buildAntdToken(colors(), theme.appearance.value));
</script>

<template>
  <ConfigProvider :theme="antdTheme" :locale="zhCN">
    <AppLayout />
  </ConfigProvider>
</template>
