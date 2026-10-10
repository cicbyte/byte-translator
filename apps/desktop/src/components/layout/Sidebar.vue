<script setup lang="ts">
/** 侧边栏：品牌区 + 分组导航（路由联动高亮） */
import { useRoute, useRouter } from "vue-router";
import {
  TranslationOutlined,
  HistoryOutlined,
  BookOutlined,
  ApiOutlined,
  ClusterOutlined,
  SettingOutlined,
} from "@ant-design/icons-vue";

const route = useRoute();
const router = useRouter();

const GROUPS = [
  {
    label: "工作台",
    items: [
      { path: "/", title: "翻译", icon: TranslationOutlined },
      { path: "/history", title: "历史", icon: HistoryOutlined },
      { path: "/wordbook", title: "生词本", icon: BookOutlined },
    ],
  },
  {
    label: "服务",
    items: [
      { path: "/service/rest", title: "REST API", icon: ApiOutlined },
      { path: "/service/mcp", title: "MCP 服务", icon: ClusterOutlined },
    ],
  },
  {
    label: "配置",
    items: [{ path: "/settings", title: "设置", icon: SettingOutlined }],
  },
];

function isActive(path: string): boolean {
  if (path === "/") return route.path === "/";
  return route.path.startsWith(path);
}
</script>

<template>
  <div class="sidebar">
    <div class="sidebar-logo" @click="router.push('/')">
      <svg class="logo-mark" viewBox="0 0 96 96" xmlns="http://www.w3.org/2000/svg">
        <defs>
          <linearGradient id="sideLogoG" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0" stop-color="#4f7fd9"/><stop offset="1" stop-color="#2f56a8"/>
          </linearGradient>
        </defs>
        <rect x="6" y="6" width="84" height="84" rx="42" fill="url(#sideLogoG)"/>
        <circle cx="48" cy="48" r="34" fill="none" stroke="rgba(255,255,255,.5)" stroke-width="1.6" stroke-dasharray="3 5"/>
        <text x="48" y="49" text-anchor="middle" dominant-baseline="central" font-family="'PingFang SC','Microsoft YaHei',sans-serif" font-size="34" font-weight="800" fill="#f4f7fd">译</text>
        <text x="48" y="21" text-anchor="middle" font-family="Georgia,serif" font-size="10" font-weight="700" fill="#f4f7fd">A</text>
        <text x="75" y="52" text-anchor="middle" font-family="sans-serif" font-size="10" fill="#f4f7fd">あ</text>
        <text x="25" y="77" text-anchor="middle" font-family="sans-serif" font-size="10" fill="#f4f7fd">Ö</text>
      </svg>
      <div class="logo-text">
        <div class="logo-name">btr</div>
        <div class="logo-sub">byte-translator</div>
      </div>
    </div>
    <nav class="sidebar-nav">
      <div v-for="group in GROUPS" :key="group.label" class="nav-group">
        <div class="nav-label-title">{{ group.label }}</div>
        <button
          v-for="item in group.items"
          :key="item.path"
          class="nav-item"
          :class="{ active: isActive(item.path) }"
          @click="router.push(item.path)"
        >
          <component :is="item.icon" class="nav-icon" />
          <span class="nav-title">{{ item.title }}</span>
        </button>
      </div>
    </nav>
    <div class="sidebar-foot">btr v0.1</div>
  </div>
</template>

<style scoped>
.sidebar {
  display: flex;
  flex-direction: column;
  height: 100%;
  flex-shrink: 0;
  width: var(--sidebar-width-expanded);
  overflow: hidden;
}

/* 品牌区 */
.sidebar-logo {
  display: flex;
  align-items: center;
  gap: 10px;
  height: var(--navbar-height);
  padding: 0 var(--spacing-md);
  border-bottom: 1px solid var(--border-light);
  flex-shrink: 0;
  cursor: pointer;
  user-select: none;
}
.logo-mark {
  width: 28px;
  height: 28px;
  flex-shrink: 0;
}
.logo-name {
  font-size: 14px;
  font-weight: 700;
  line-height: 1.1;
  color: var(--text-primary);
}
.logo-sub {
  font-size: 10px;
  color: var(--text-tertiary);
  line-height: 1.2;
}

/* 导航 */
.sidebar-nav {
  flex: 1;
  overflow-y: auto;
  padding: var(--spacing-sm);
}
.nav-group {
  margin-bottom: 2px;
}
.nav-label-title {
  padding: 14px 10px 4px;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-tertiary);
  letter-spacing: 0.05em;
  user-select: none;
  white-space: nowrap;
}
.nav-group:first-of-type .nav-label-title {
  padding-top: 6px;
}
.nav-item {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  height: 30px;
  padding: 0 8px;
  margin-bottom: 2px;
  border: none;
  border-radius: 8px;
  cursor: pointer;
  transition: background-color var(--transition-fast), color var(--transition-fast);
  color: var(--text-secondary);
  font-size: 13px;
  background: transparent;
  white-space: nowrap;
  user-select: none;
  width: 100%;
  text-align: left;
}
.nav-item:hover {
  background: var(--color-nav-hover);
  color: var(--text-primary);
}
.nav-item.active {
  background: var(--color-nav-active);
  color: var(--text-primary);
  font-weight: 500;
}
.nav-icon {
  font-size: 15px;
}
.nav-item.active .nav-icon {
  color: var(--color-primary);
}

/* 底部点缀 */
.sidebar-foot {
  padding: 10px var(--spacing-md);
  font-size: 10px;
  color: var(--text-tertiary);
  border-top: 1px solid var(--border-light);
  white-space: nowrap;
  user-select: none;
}
</style>
