<script setup lang="ts">
/**
 * 全局空状态：纸感插画（错位叠纸 + 文字行 + 主题色墨点笔迹）+ 主副文案 + 可选操作。
 * 替代 antd a-empty 的后台管理默认样式；颜色全部走主题 token
 * （纸面/线条/墨点随预设与明暗自动跟随）。
 */
defineProps<{
  title: string;
  description?: string;
}>();
</script>

<template>
  <div class="btr-empty">
    <svg viewBox="0 0 120 106" class="empty-art" aria-hidden="true">
      <!-- 后纸：错位微旋，弱化 -->
      <g class="paper-back" transform="rotate(7 62 48)">
        <rect x="34" y="8" width="58" height="76" rx="6" />
      </g>
      <!-- 前纸：带文字行 -->
      <g class="paper-front">
        <rect x="20" y="18" width="60" height="78" rx="6" />
        <line x1="30" y1="36" x2="66" y2="36" class="tline" />
        <line x1="30" y1="48" x2="70" y2="48" class="tline" />
        <line x1="30" y1="60" x2="58" y2="60" class="tline" />
        <line x1="30" y1="72" x2="64" y2="72" class="tline" />
      </g>
      <!-- 墨点笔迹：一抹主题色，落在纸面之外 -->
      <path class="ink" d="M76 46c4 8 10 13 18 15" />
      <circle class="ink-dot" cx="96" cy="63" r="3.2" />
    </svg>
    <div class="empty-title">{{ title }}</div>
    <div v-if="description" class="empty-desc">{{ description }}</div>
    <div v-if="$slots.action" class="empty-action">
      <slot name="action" />
    </div>
  </div>
</template>

<script lang="ts">
export default { name: "EmptyState" };
</script>

<style scoped>
.btr-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 2px;
  padding: 36px 0;
  user-select: none;
}
.empty-art {
  width: 116px;
  height: 102px;
  margin-bottom: 10px;
  animation: empty-float 5s ease-in-out infinite alternate;
}
@keyframes empty-float {
  from { transform: translateY(0); }
  to { transform: translateY(-6px); }
}

/* 纸面：主题底色 + 细边 */
.paper-front rect {
  fill: var(--bg-surface);
  stroke: var(--border-base);
  stroke-width: 1;
}
.paper-back rect {
  fill: var(--bg-elevated);
  stroke: var(--border-light);
  stroke-width: 1;
}
/* 文字行：弱色线 */
.tline {
  stroke: var(--border-base);
  stroke-width: 2.6;
  stroke-linecap: round;
}
/* 墨点笔迹：主题色 */
.ink {
  stroke: var(--color-primary);
  stroke-width: 2.4;
  stroke-linecap: round;
  fill: none;
  opacity: 0.85;
}
.ink-dot {
  fill: var(--color-primary);
  opacity: 0.85;
}

.empty-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-secondary);
}
.empty-desc {
  margin-top: 4px;
  font-size: 12.5px;
  color: var(--text-tertiary);
}
.empty-action {
  margin-top: 14px;
}
</style>
