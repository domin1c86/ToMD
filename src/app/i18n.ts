import { createContext, createElement, ReactNode, useContext, useMemo, useState } from "react";

export type Locale = "zh-CN" | "en-US";

const localeStorageKey = "design-md-locale";

type I18nContextValue = {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  t: (key: I18nKey) => string;
};

type Dictionary = Record<string, string>;

const dictionaries: Record<Locale, Dictionary> = {
  "zh-CN": {
    appName: "设计规范提取器",
    appNameEn: "Design Language Extractor",
    brand: "Design MD",
    localFirst: "本地优先 · 可审核运行",
    projects: "项目",
    providers: "模型配置",
    exports: "导出记录",
    settings: "设置",
    help: "帮助与反馈",
    newProject: "新建项目",
    nextAction: "下一步",
    switchToEnglish: "English",
    switchToChinese: "中文",
    lightTheme: "浅色",
    darkTheme: "暗色",
    projectStep: "项目",
    screenshotsStep: "截图",
    providerStep: "模型",
    analysisStep: "分析",
    reviewExportStep: "审核导出",
    projectStepHint: "定义项目目标",
    screenshotsStepHint: "导入界面截图",
    providerStepHint: "配置 AI 模型",
    analysisStepHint: "确认发送内容",
    reviewExportStepHint: "生成 DESIGN.md",
    privacyTitle: "隐私与数据安全",
    privacyBody: "分析前不会发送任何图片。截图和草稿默认保存在本机，只有点击发送分析后才会传给你配置的模型。",
  },
  "en-US": {
    appName: "Design Language Extractor",
    appNameEn: "Design Language Extractor",
    brand: "Design MD",
    localFirst: "Local-first · reviewed workflow",
    projects: "Projects",
    providers: "Providers",
    exports: "Exports",
    settings: "Settings",
    help: "Help & feedback",
    newProject: "New project",
    nextAction: "Next",
    switchToEnglish: "English",
    switchToChinese: "中文",
    lightTheme: "Light",
    darkTheme: "Dark",
    projectStep: "Project",
    screenshotsStep: "Screenshots",
    providerStep: "Provider",
    analysisStep: "Analyze",
    reviewExportStep: "Review & export",
    projectStepHint: "Define scope",
    screenshotsStepHint: "Import references",
    providerStepHint: "Connect AI model",
    analysisStepHint: "Confirm transmission",
    reviewExportStepHint: "Generate DESIGN.md",
    privacyTitle: "Privacy and data safety",
    privacyBody: "No screenshots leave your device before analysis. References and drafts stay local unless you explicitly send them to your configured provider.",
  },
};

export type I18nKey = keyof typeof dictionaries["zh-CN"];

const I18nContext = createContext<I18nContextValue | null>(null);

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(() => readStoredLocale());

  const value = useMemo<I18nContextValue>(
    () => ({
      locale,
      setLocale(nextLocale) {
        localStorage.setItem(localeStorageKey, nextLocale);
        setLocaleState(nextLocale);
      },
      t(key) {
        return dictionaries[locale][key];
      },
    }),
    [locale],
  );

  return createElement(I18nContext.Provider, { value }, children);
}

export function useI18n() {
  const context = useContext(I18nContext);
  if (!context) {
    throw new Error("useI18n must be used within I18nProvider");
  }
  return context;
}

function readStoredLocale(): Locale {
  const stored = localStorage.getItem(localeStorageKey);
  return stored === "en-US" ? "en-US" : "zh-CN";
}
