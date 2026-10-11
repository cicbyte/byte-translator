<script setup lang="ts">
/**
 * 设置页：左垂直分组菜单 + 右分区内容（分栏布局）。
 * 左栏语言与应用侧边栏一致（细线分隔、胶囊激活态）。
 */
import { ref } from "vue";
import {
  TranslationOutlined,
  CloudOutlined,
  DatabaseOutlined,
  SettingOutlined,
  BgColorsOutlined,
  InfoCircleOutlined,
} from "@ant-design/icons-vue";
import EnginesSection from "./components/EnginesSection.vue";
import ProviderSection from "./components/ProviderSection.vue";
import StorageSection from "./components/StorageSection.vue";
import PreferencesSection from "./components/PreferencesSection.vue";
import AppearanceSection from "./components/AppearanceSection.vue";
import AboutSection from "./components/AboutSection.vue";

const tabs = [
  { key: "engines", icon: TranslationOutlined, label: "翻译引擎" },
  { key: "providers", icon: CloudOutlined, label: "AI 供应商" },
  { key: "storage", icon: DatabaseOutlined, label: "数据" },
  { key: "preferences", icon: SettingOutlined, label: "偏好" },
  { key: "appearance", icon: BgColorsOutlined, label: "外观" },
  { key: "about", icon: InfoCircleOutlined, label: "关于" },
];
const activeTab = ref("engines");
</script>

<template>
  <div class="settings-view">
    <!-- 左：垂直分组菜单 -->
    <aside class="settings-sider">
      <div class="sider-title">设置</div>
      <ul class="group-menu">
        <li v-for="tab in tabs" :key="tab.key">
          <a class="menu-item" :class="{ active: activeTab === tab.key }" @click="activeTab = tab.key">
            <component :is="tab.icon" />
            <span>{{ tab.label }}</span>
          </a>
        </li>
      </ul>
    </aside>

    <!-- 右：分区内容 -->
    <div class="settings-content">
      <EnginesSection v-if="activeTab === 'engines'" class="engines-full" @goto-providers="activeTab = 'providers'" />
      <ProviderSection v-else-if="activeTab === 'providers'" />
      <StorageSection v-else-if="activeTab === 'storage'" />
      <PreferencesSection v-else-if="activeTab === 'preferences'" />
      <AppearanceSection v-else-if="activeTab === 'appearance'" />
      <AboutSection v-else-if="activeTab === 'about'" />
    </div>
  </div>
</template>

<style scoped>
.settings-view {
  display: flex;
  height: 100%;
  overflow: hidden;
  background: var(--bg-base);
}

/* 左侧分组菜单：无底跟随纸面，仅细线分隔（与应用主侧边栏同语言） */
.settings-sider {
  display: flex;
  flex-direction: column;
  min-width: 180px;
  width: 180px;
  border-right: 1px solid var(--border-light);
  padding: var(--spacing-sm);
  user-select: none;
  flex-shrink: 0;
}
.sider-title {
  padding: 8px 12px 14px;
  font-size: 12px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--text-tertiary);
}
.group-menu {
  display: flex;
  flex-direction: column;
  list-style: none;
  margin: 0;
  padding: 0;
}
.group-menu li {
  margin-bottom: 2px;
}
.menu-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 12px;
  width: 100%;
  text-decoration: none;
  color: var(--text-secondary);
  cursor: pointer;
  border-radius: var(--radius-md);
  font-weight: 500;
  font-size: 13px;
  transition: all var(--transition-fast);
}
.menu-item :deep(.anticon) {
  font-size: 15px;
  opacity: 0.85;
}
.menu-item:hover {
  background: var(--color-nav-hover);
  color: var(--text-primary);
}
.menu-item.active {
  background: var(--color-nav-active);
  color: var(--text-primary);
}
.menu-item.active :deep(.anticon) {
  color: var(--color-primary);
  opacity: 1;
}

/* 右侧内容区 */
.settings-content {
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow-y: auto;
  padding: var(--spacing-lg) var(--spacing-xl);
}
.settings-content::-webkit-scrollbar {
  width: 8px;
}
.settings-content::-webkit-scrollbar-thumb {
  background: var(--border-base);
  border-radius: 4px;
}
</style>

<style>
/* Section 公共骨架（全局：各子 Section 复用） */
.setting-section {
  background: var(--bg-surface);
  border: 1px solid var(--border-light);
  border-radius: 12px;
  padding: 18px 20px;
  flex-shrink: 0;
  max-width: 780px;
}
.section-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 8px;
}
.section-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0;
}
.section-desc {
  margin: 3px 0 0;
  font-size: 12px;
  color: var(--text-tertiary);
}
</style>
