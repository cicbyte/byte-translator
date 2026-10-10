/**
 * 主题注入与消费：
 * - applyTheme()：把预设色板注入 CSS 变量（bg 系走 -orig 由 :root 派生半透明），
 *   同时设置 data-theme 明暗属性；
 * - buildAntdToken()：Antd ConfigProvider token 读同一份色板（双轨一致）；
 * - useTheme()：明暗 + 预设的组合状态（localStorage 持久化）。
 */
import { ref, watchEffect } from "vue";
import { theme as antdTheme } from "ant-design-vue";
import {
  PRESETS,
  hexToRgb,
  isLightPrimary,
  type Appearance,
  type PresetKey,
  type ThemeColors,
} from "./presets";

const LS_KEY = "btr-theme";

interface PersistedTheme {
  preset: PresetKey;
  appearance: Appearance;
}

function load(): PersistedTheme {
  try {
    const raw = localStorage.getItem(LS_KEY);
    if (raw) {
      const v = JSON.parse(raw);
      if (v?.preset in PRESETS && (v.appearance === "light" || v.appearance === "dark")) {
        return v;
      }
    }
  } catch {
    /* 损坏则回落默认 */
  }
  return { preset: "indigo", appearance: "dark" };
}

const preset = ref<PresetKey>(load().preset);
const appearance = ref<Appearance>(load().appearance);

export function useTheme() {
  return {
    preset,
    appearance,
    setPreset(p: PresetKey) {
      preset.value = p;
    },
    setAppearance(a: Appearance) {
      appearance.value = a;
    },
    toggleAppearance() {
      appearance.value = appearance.value === "dark" ? "light" : "dark";
    },
  };
}

export function colors(): ThemeColors {
  return PRESETS[preset.value].palette[appearance.value];
}

/** 扩展层（状态色 / 阴影 / 导航层次）按明暗固定，不随预设变 */
const EXT_LIGHT: Record<string, string> = {
  "--text-disabled": "#9aa1ab",
  "--color-success": "#16a34a",
  "--color-warning": "#d97706",
  "--color-danger": "#dc2626",
  "--color-info": "#0891b2",
  "--success-bg": "rgba(22, 163, 74, 0.12)",
  "--warning-bg": "rgba(217, 119, 6, 0.12)",
  "--danger-bg": "rgba(220, 38, 38, 0.10)",
  "--info-bg": "rgba(8, 145, 178, 0.12)",
  "--shadow-panel": "0 1px 2px rgba(28, 29, 33, 0.03), 0 8px 24px -16px rgba(28, 29, 33, 0.08)",
  "--shadow-panel-breathe": "0 2px 4px rgba(28, 29, 33, 0.04), 0 16px 38px -16px rgba(28, 29, 33, 0.16)",
  "--shadow-panel-hover": "0 1px 2px rgba(28, 29, 33, 0.04), 0 12px 32px -12px rgba(28, 29, 33, 0.14)",
  "--color-nav-hover": "rgba(28, 29, 33, 0.05)",
  "--color-nav-active": "rgba(28, 29, 33, 0.09)",
};
const EXT_DARK: Record<string, string> = {
  "--text-disabled": "#4b5563",
  "--color-success": "#22c55e",
  "--color-warning": "#f59e0b",
  "--color-danger": "#ef4444",
  "--color-info": "#06b6d4",
  "--success-bg": "rgba(34, 197, 94, 0.14)",
  "--warning-bg": "rgba(245, 158, 11, 0.14)",
  "--danger-bg": "rgba(239, 68, 68, 0.14)",
  "--info-bg": "rgba(6, 182, 212, 0.14)",
  "--shadow-panel": "0 1px 2px rgba(0, 0, 0, 0.4), 0 10px 28px -16px rgba(0, 0, 0, 0.55)",
  "--shadow-panel-breathe": "0 2px 5px rgba(0, 0, 0, 0.45), 0 18px 42px -16px rgba(0, 0, 0, 0.7)",
  "--shadow-panel-hover": "0 1px 2px rgba(0, 0, 0, 0.5), 0 14px 38px -12px rgba(0, 0, 0, 0.65)",
  "--color-nav-hover": "rgba(255, 255, 255, 0.05)",
  "--color-nav-active": "rgba(255, 255, 255, 0.09)",
};

/** 把一份色板注入 DOM（bg 走 -orig 供 :root 派生半透明层） */
export function applyThemeToDOM(c: ThemeColors, mode: Appearance): void {
  const root = document.documentElement;
  const vars: Record<string, string> = {
    "--bg-base-orig": c.bgBase,
    "--bg-surface-orig": c.bgSurface,
    "--bg-input-orig": c.bgInput,
    "--bg-elevated-orig": c.bgElevated,
    "--color-primary": c.primary,
    "--text-primary": c.textPrimary,
    "--text-secondary": c.textSecondary,
    "--text-tertiary": c.textTertiary,
    "--border-base": c.borderBase,
    "--border-light": c.borderLight,
  };
  const [pr, pg, pb] = hexToRgb(c.primary);
  vars["--color-primary-rgb"] = `${pr}, ${pg}, ${pb}`;
  vars["--accent"] = c.primary;
  vars["--hero-cta-text"] = isLightPrimary(c.primary) ? "#1b1c1f" : "#ffffff";

  for (const [k, v] of Object.entries(vars)) root.style.setProperty(k, v);
  for (const [k, v] of Object.entries(mode === "dark" ? EXT_DARK : EXT_LIGHT)) {
    root.style.setProperty(k, v);
  }
  root.setAttribute("data-theme", mode);
}

/** 派生变量（半透明 bg 系）与明暗属性挂钩的静态层——在 main.ts 引入一次 */
export const DERIVED_CSS = `
:root, [data-theme] {
  --bg-base: var(--bg-base-orig);
  --bg-surface: var(--bg-surface-orig);
  --bg-input: var(--bg-input-orig);
  --bg-elevated: var(--bg-elevated-orig);
}`;

/** antd 轨：ConfigProvider token 读同一份色板 */
export function buildAntdToken(c: ThemeColors, mode: Appearance) {
  const isDark = mode === "dark";
  return {
    algorithm: isDark ? antdTheme.darkAlgorithm : antdTheme.defaultAlgorithm,
    token: {
      colorPrimary: c.primary,
      colorBgBase: c.bgBase,
      colorBgContainer: c.bgSurface,
      colorBgElevated: c.bgElevated,
      colorBgLayout: c.bgBase,
      colorText: c.textPrimary,
      colorTextSecondary: c.textSecondary,
      colorTextTertiary: c.textTertiary,
      colorBorder: c.borderBase,
      colorBorderSecondary: c.borderLight,
      colorInfo: c.primary,
      borderRadius: 6,
      fontFamily: "'Inter', 'PingFang SC', 'Microsoft YaHei', sans-serif",
    },
  };
}

/** 响应式接线：主题状态变化即注入 DOM + 持久化（App 挂载时调用一次） */
export function bindTheme() {
  watchEffect(() => {
    const c = PRESETS[preset.value].palette[appearance.value];
    applyThemeToDOM(c, appearance.value);
    localStorage.setItem(LS_KEY, JSON.stringify({ preset: preset.value, appearance: appearance.value }));
  });
}
