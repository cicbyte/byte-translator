<script setup lang="ts">
/** 偏好设置：全局快捷键等（引擎配置在「翻译引擎」分区） */

const HOTKEYS = [
  { combo: "Ctrl + Alt + D", desc: "划词翻译——任意应用选中文字后按下，光标处弹出快速翻译小窗（Esc 或点击别处关闭）" },
  { combo: "Alt + T", desc: "呼出 / 隐藏主窗口" },
];
</script>

<template>
  <section class="setting-section">
    <header class="section-head">
      <div>
        <h3 class="section-title">偏好</h3>
        <p class="section-desc">全局快捷键（系统级，应用在后台也生效；占用冲突时启动日志会有提示）</p>
      </div>
    </header>
    <div v-for="k in HOTKEYS" :key="k.combo" class="hotkey-row">
      <kbd class="combo">{{ k.combo }}</kbd>
      <span class="desc">{{ k.desc }}</span>
    </div>
  </section>
</template>

<style scoped>
.hotkey-row {
  display: flex;
  align-items: baseline;
  gap: 14px;
  padding: 10px 0;
}
.hotkey-row + .hotkey-row {
  border-top: 1px solid var(--border-light);
}
.combo {
  flex-shrink: 0;
  font-family: "JetBrains Mono", Consolas, monospace;
  font-size: 12px;
  color: var(--color-primary);
  background: var(--bg-input);
  border: 1px solid var(--border-base);
  border-radius: 6px;
  padding: 3px 10px;
}
.desc {
  font-size: 12.5px;
  color: var(--text-secondary);
  line-height: 1.6;
}
</style>
