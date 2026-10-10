/**
 * 预设色板：每套预设含浅/暗两态完整基础层（主色 + 9 个中性 token）。
 * 扩展层（状态色 / 阴影 / 导航层次）按明暗固定补充，不参与预设色调。
 */

export type Appearance = "light" | "dark";
export type PresetKey = "paper" | "indigo" | "celadon" | "plum" | "tea";

export interface ThemeColors {
  primary: string;
  bgBase: string;
  bgSurface: string;
  bgInput: string;
  bgElevated: string;
  textPrimary: string;
  textSecondary: string;
  textTertiary: string;
  borderBase: string;
  borderLight: string;
}

export const PRESETS: Record<PresetKey, { label: string; palette: Record<Appearance, ThemeColors> }> = {
  paper: {
    label: "纸墨",
    palette: {
      light: {
        primary: "#1b1c1f", bgBase: "#f1f1ee", bgSurface: "#ffffff",
        bgInput: "#f3f3f1", bgElevated: "#f8f8f7", textPrimary: "#1c1d21",
        textSecondary: "#5c6167", textTertiary: "#97999d",
        borderBase: "#dcdcd9", borderLight: "#e9e9e7",
      },
      dark: {
        primary: "#f2f2f0", bgBase: "#191a1c", bgSurface: "#232527",
        bgInput: "#2d2f31", bgElevated: "#282a2c", textPrimary: "#ececee",
        textSecondary: "#a3a6aa", textTertiary: "#74777c",
        borderBase: "#3a3c3f", borderLight: "#2d2f31",
      },
    },
  },
  indigo: {
    label: "青黛",
    palette: {
      light: {
        primary: "#46618f", bgBase: "#f1f2f5", bgSurface: "#ffffff",
        bgInput: "#f2f3f6", bgElevated: "#f7f8fa", textPrimary: "#1d2027",
        textSecondary: "#565d6b", textTertiary: "#949aa5",
        borderBase: "#d8dae0", borderLight: "#e7e8ed",
      },
      dark: {
        primary: "#8ca3cc", bgBase: "#15171d", bgSurface: "#1e2129",
        bgInput: "#282c36", bgElevated: "#232730", textPrimary: "#e9ebf0",
        textSecondary: "#a0a6b3", textTertiary: "#707583",
        borderBase: "#343945", borderLight: "#282c36",
      },
    },
  },
  celadon: {
    label: "青瓷",
    palette: {
      light: {
        primary: "#47796b", bgBase: "#eef2f0", bgSurface: "#ffffff",
        bgInput: "#f0f3f1", bgElevated: "#f5f8f6", textPrimary: "#1b211f",
        textSecondary: "#55605b", textTertiary: "#939c97",
        borderBase: "#d7ded9", borderLight: "#e6ebe8",
      },
      dark: {
        primary: "#82b0a0", bgBase: "#131a17", bgSurface: "#1b2420",
        bgInput: "#242e29", bgElevated: "#202925", textPrimary: "#e8ece9",
        textSecondary: "#9da8a1", textTertiary: "#6e7973",
        borderBase: "#303b35", borderLight: "#242e29",
      },
    },
  },
  plum: {
    label: "藕荷",
    palette: {
      light: {
        primary: "#6f6494", bgBase: "#f3f2f6", bgSurface: "#ffffff",
        bgInput: "#f4f3f7", bgElevated: "#f8f8fa", textPrimary: "#201e28",
        textSecondary: "#5a5768", textTertiary: "#96939f",
        borderBase: "#dad8e0", borderLight: "#e9e8ee",
      },
      dark: {
        primary: "#a79bd0", bgBase: "#17151d", bgSurface: "#201e29",
        bgInput: "#2a2735", bgElevated: "#252230", textPrimary: "#eae8ef",
        textSecondary: "#a5a1b4", textTertiary: "#746f82",
        borderBase: "#363244", borderLight: "#2a2735",
      },
    },
  },
  tea: {
    label: "茶褐",
    palette: {
      light: {
        primary: "#8a6242", bgBase: "#f6f3ed", bgSurface: "#fffdf9",
        bgInput: "#f6f4ee", bgElevated: "#faf8f3", textPrimary: "#262019",
        textSecondary: "#5f574c", textTertiary: "#9b9184",
        borderBase: "#e0d9cc", borderLight: "#ece6da",
      },
      dark: {
        primary: "#c49a72", bgBase: "#191612", bgSurface: "#221e19",
        bgInput: "#2c2721", bgElevated: "#27221d", textPrimary: "#ede8df",
        textSecondary: "#a89e8f", textTertiary: "#786e60",
        borderBase: "#393229", borderLight: "#2c2721",
      },
    },
  },
};

export const PRESET_LIST: { key: PresetKey; label: string }[] = [
  { key: "paper", label: "纸墨" },
  { key: "indigo", label: "青黛" },
  { key: "celadon", label: "青瓷" },
  { key: "plum", label: "藕荷" },
  { key: "tea", label: "茶褐" },
];

export function hexToRgb(hex: string): [number, number, number] {
  const h = hex.replace("#", "");
  return [
    parseInt(h.slice(0, 2), 16),
    parseInt(h.slice(2, 4), 16),
    parseInt(h.slice(4, 6), 16),
  ];
}

/** 主色是否偏亮（相对亮度 > 0.7）——亮主色需深色反白文字 */
export function isLightPrimary(hex: string): boolean {
  const [r, g, b] = hexToRgb(hex);
  return (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255 > 0.7;
}
