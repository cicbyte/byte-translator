<script setup lang="ts">
/**
 * 应用布局：画布 + 悬浮圆角三面板
 *
 * app-layout（画布，四周留缝）
 *   ├── Sidebar     独立悬浮面板（14px 圆角 + 呼吸阴影）
 *   └── .app-right（8px 缝隙）
 *       ├── Navbar  顶栏面板（45px，自绘标题栏 + 窗控）
 *       └── 内容面板（RouterView）
 */
import { RouterView } from "vue-router";
import Sidebar from "./Sidebar.vue";
import Navbar from "./Navbar.vue";
</script>

<template>
  <div class="app-layout">
    <Sidebar class="shell-panel" />
    <div class="app-right">
      <Navbar class="shell-panel" />
      <div class="app-content shell-panel">
        <RouterView />
      </div>
    </div>
  </div>
</template>

<style scoped>
.app-layout {
  position: relative;
  display: flex;
  height: 100vh;
  width: 100vw;
  overflow: hidden;
  background: var(--bg-base);
  color: var(--text-primary);
  gap: 8px;
  padding: 8px 10px 10px;
}

/* 悬浮面板三件套：圆角 + 细边框 + 呼吸阴影（hover 即停换档） */
.shell-panel {
  background: var(--bg-surface);
  border: 1px solid var(--border-light);
  border-radius: 14px;
  box-shadow: var(--shadow-panel);
  animation: shell-breathe 8s ease-in-out infinite;
  overflow: hidden;
  transition: box-shadow 0.35s ease;
}
.shell-panel:hover {
  animation: none;
  box-shadow: var(--shadow-panel-hover);
}
@keyframes shell-breathe {
  0%, 100% { box-shadow: var(--shadow-panel); }
  50% { box-shadow: var(--shadow-panel-breathe); }
}
@media (prefers-reduced-motion: reduce) {
  .shell-panel { animation: none; }
}
.app-right {
  position: relative;
  z-index: 2;
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  gap: 8px;
}
.app-right > .shell-panel:first-child {
  flex: none;
}
.app-content {
  flex: 1;
  min-height: 0;
  overflow: auto;
}
</style>
