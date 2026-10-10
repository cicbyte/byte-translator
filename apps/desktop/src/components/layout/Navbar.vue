<script setup lang="ts">
/**
 * Navbar 自绘标题栏：面包屑 + 明暗切换 + 主题预设 + 窗口控制。
 * 无边框窗口（decorations: false）——关闭/最大化/最小化由自绘按钮承担。
 */
import { computed } from "vue";
import { useRoute } from "vue-router";
import {
  MinusOutlined,
  BorderOutlined,
  CloseOutlined,
  BgColorsOutlined,
} from "@ant-design/icons-vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useTheme } from "@/themes";
import { PRESET_LIST, type PresetKey } from "@/themes/presets";

const route = useRoute();
const theme = useTheme();

// 纯浏览器调试（无 Tauri 外壳）时降级为 null，窗口按钮判空
const appWindow = (() => {
  try {
    return getCurrentWindow();
  } catch {
    return null;
  }
})();

const breadcrumb = computed(() => {
  const group = (route.meta.group as string) || "";
  const title = (route.meta.title as string) || "byte-translator";
  return { group, title };
});

function onPresetClick({ key }: { key: string }) {
  theme.setPreset(key as PresetKey);
}
</script>

<template>
  <div class="navbar" data-tauri-drag-region>
    <div class="navbar-left" data-tauri-drag-region>
      <span v-if="breadcrumb.group" class="crumb-group">{{ breadcrumb.group }}</span>
      <span v-if="breadcrumb.group" class="crumb-sep">/</span>
      <span class="crumb">{{ breadcrumb.title }}</span>
    </div>

    <div class="navbar-center" data-tauri-drag-region></div>

    <div class="navbar-right" data-tauri-drag-region>
      <a-dropdown :trigger="['click']">
        <button class="icon-btn" title="主题配色">
          <BgColorsOutlined />
        </button>
        <template #overlay>
          <a-menu :selected-keys="[theme.preset.value]" @click="onPresetClick">
            <a-menu-item v-for="p in PRESET_LIST" :key="p.key">
              <span class="preset-dot" :style="{ background: `var(--color-primary)` }" />
              {{ p.label }}
            </a-menu-item>
          </a-menu>
        </template>
      </a-dropdown>

      <button
        class="icon-btn"
        :title="theme.appearance.value === 'dark' ? '切换到浅色' : '切换到深色'"
        @click="theme.toggleAppearance()"
      >
        <!-- 暗色时显示太阳（点击切亮色）；亮色时显示月亮 -->
        <svg
          v-if="theme.appearance.value === 'dark'"
          class="theme-icon" viewBox="0 0 24 24" width="15" height="15"
          fill="none" stroke="currentColor" stroke-width="2"
          stroke-linecap="round" stroke-linejoin="round"
        >
          <circle cx="12" cy="12" r="5" />
          <line x1="12" y1="1" x2="12" y2="3" /><line x1="12" y1="21" x2="12" y2="23" />
          <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" /><line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
          <line x1="1" y1="12" x2="3" y2="12" /><line x1="21" y1="12" x2="23" y2="12" />
          <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" /><line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
        </svg>
        <svg
          v-else
          class="theme-icon" viewBox="0 0 24 24" width="15" height="15"
          fill="none" stroke="currentColor" stroke-width="2"
          stroke-linecap="round" stroke-linejoin="round"
        >
          <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
        </svg>
      </button>

      <div class="win-controls">
        <button class="win-btn" title="最小化" @click="appWindow?.minimize()">
          <MinusOutlined />
        </button>
        <button class="win-btn" title="最大化 / 还原" @click="appWindow?.toggleMaximize()">
          <BorderOutlined />
        </button>
        <button class="win-btn win-close" title="关闭" @click="appWindow?.close()">
          <CloseOutlined />
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.navbar {
  height: var(--navbar-height);
  display: flex;
  align-items: center;
  padding: 0 var(--spacing-sm) 0 var(--spacing-md);
  user-select: none;
  flex-shrink: 0;
}
.navbar-left {
  display: flex;
  align-items: center;
  min-width: 120px;
}
.crumb-group {
  font-size: 12px;
  color: var(--text-tertiary);
}
.crumb-sep {
  font-size: 12px;
  color: var(--text-tertiary);
  margin: 0 4px;
}
.crumb {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
}
.navbar-center {
  flex: 1;
}
.navbar-right {
  display: flex;
  align-items: center;
  gap: 2px;
  justify-content: flex-end;
}
.icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--transition-fast);
  font-size: 14px;
}
.icon-btn:hover {
  background: var(--bg-elevated);
  color: var(--color-primary);
}
.preset-dot {
  display: inline-block;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  margin-right: 8px;
}

/* 窗口控制（macOS 式右侧重叠区） */
.win-controls {
  display: flex;
  align-items: stretch;
  height: 100%;
  margin-left: 6px;
}
.win-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 44px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  transition: background-color var(--transition-fast), color var(--transition-fast);
  font-size: 12px;
}
.win-btn:hover {
  background: var(--color-nav-hover);
  color: var(--text-primary);
}
.win-close:hover {
  background: #e81123;
  color: #fff;
}
/* 顶栏右侧圆角内收（贴边按钮不破圆角） */
.navbar :deep(.win-controls .win-btn:first-child) {
  border-top-left-radius: 0;
}
</style>
