import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { ArrowRight, Loader2, Settings2, Zap } from "lucide-react";
import { Switch } from "@/components/ui/switch";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { useProvidersQuery } from "@/lib/query";
import {
  useLeproAuthStatus,
  useLeproLogin,
  useLeproMaster,
} from "@/lib/query/lepro";

const LEPRO_ID = "lepro";

// 模块级守卫：父组件 motion.div 以 activeApp 为 key，切 tab 会重挂本组件；用模块级
// 标志（而非 useRef）保证「开机刷新」每个会话只跑一次，避免每次切 tab 都触发同步。
let didSessionRefresh = false;

// 档位 → settings.json env key,ON 时据此展示「档位 → 实际模型」映射（取 Claude 应用的）。
const TIER_ROWS: { label: string; envKey: string }[] = [
  { label: "Opus", envKey: "ANTHROPIC_DEFAULT_OPUS_MODEL" },
  { label: "Sonnet", envKey: "ANTHROPIC_DEFAULT_SONNET_MODEL" },
  { label: "Haiku", envKey: "ANTHROPIC_DEFAULT_HAIKU_MODEL" },
];

interface LeproSwitchProps {
  /** 是否展开「第三方供应商设置」（由父组件控制 ProviderList 的显示）。 */
  settingsOpen: boolean;
  onToggleSettings: () => void;
}

/**
 * 主屏的 Lepro 全局总开关。开 = 一键把 4 个应用（Claude CLI / Claude App / Codex /
 * Gemini）全切到 Lepro 并展示模型映射；关 = 各应用回退到上次的供应商。卡片自行查询
 * Claude 应用的状态判断开/关，状态由 cc-switch 当前供应商持久化（重启沿用；首装默认关）。
 * 底部「第三方供应商设置」展开完整的逐应用供应商管理。
 */
export function LeproSwitch({
  settingsOpen,
  onToggleSettings,
}: LeproSwitchProps) {
  const { t } = useTranslation();
  const { data: claudeData } = useProvidersQuery("claude");
  const { data: loggedIn } = useLeproAuthStatus();
  const login = useLeproLogin();
  const { enable, disable, refresh } = useLeproMaster();

  const isLeproActive = claudeData?.currentProviderId === LEPRO_ID;
  const pending =
    login.isPending ||
    enable.isPending ||
    disable.isPending ||
    refresh.isPending;

  // 开机/打开 app：若当前已在用 Lepro，静默刷新一次（拉最新映射并对已开应用重切）。
  // 每次启动仅一次；不会把「关」的状态改成「开」（仅当已是开才刷新）。
  useEffect(() => {
    if (didSessionRefresh) return;
    if (isLeproActive) {
      didSessionRefresh = true;
      refresh.mutate();
    }
  }, [isLeproActive, refresh]);

  const handleEnable = () => {
    if (pending) return;
    if (!loggedIn) {
      // 未登录：飞书登录成功后再一键开启（这本身就是用户的手动操作）。
      login.mutate(undefined, { onSuccess: () => enable.mutate() });
    } else {
      enable.mutate();
    }
  };

  const handleToggle = (checked: boolean) => {
    if (pending) return;
    if (checked) handleEnable();
    else disable.mutate();
  };

  // 当前 Lepro provider（Claude 应用）写入的 env：档位 → 实际模型映射。
  const leproEnv = (claudeData?.providers?.[LEPRO_ID]?.settingsConfig?.env ??
    {}) as Record<string, unknown>;
  const asStr = (v: unknown) => (typeof v === "string" ? v : "");
  const mappingRows = TIER_ROWS.map((r) => ({
    label: r.label,
    model: asStr(leproEnv[r.envKey]),
  })).filter((r) => r.model);
  const defaultModel = asStr(leproEnv["ANTHROPIC_MODEL"]);
  const showMapping = isLeproActive && mappingRows.length > 0;

  return (
    <div
      className={cn(
        "rounded-2xl border transition-colors",
        isLeproActive
          ? "border-emerald-300 bg-emerald-50/60 dark:border-emerald-800 dark:bg-emerald-950/30"
          : "border-border bg-muted/30",
      )}
    >
      {/* 头部：图标 + 标题/状态 + 总开关 */}
      <div
        className={cn(
          "flex items-center justify-between gap-4",
          isLeproActive ? "p-6" : "p-5",
        )}
      >
        <div className="flex min-w-0 items-center gap-3.5">
          <span
            className={cn(
              "flex shrink-0 items-center justify-center rounded-xl transition-all",
              isLeproActive
                ? "h-14 w-14 bg-emerald-500 text-white shadow-sm shadow-emerald-500/30"
                : "h-11 w-11 bg-muted-foreground/10 text-muted-foreground",
            )}
          >
            {pending ? (
              <Loader2
                className={cn(
                  "animate-spin",
                  isLeproActive ? "h-7 w-7" : "h-5 w-5",
                )}
              />
            ) : (
              <Zap className={cn(isLeproActive ? "h-7 w-7" : "h-5 w-5")} />
            )}
          </span>
          <div className="min-w-0">
            <div
              className={cn(
                "font-semibold",
                isLeproActive ? "text-xl" : "text-base",
              )}
            >
              {t("lepro.switch.title", { defaultValue: "Lepro" })}
            </div>
            <div className="mt-0.5 truncate text-sm text-muted-foreground">
              {isLeproActive
                ? t("lepro.switch.onHintAll", {
                    defaultValue:
                      "Claude Code / Claude App / Codex / Gemini 都在使用 Lepro",
                  })
                : t("lepro.switch.offHintAll", {
                    defaultValue: "已关闭 · 各应用使用原供应商",
                  })}
            </div>
          </div>
        </div>
        <Switch
          checked={!!isLeproActive}
          onCheckedChange={handleToggle}
          disabled={pending}
          aria-label={t("lepro.switch.title", { defaultValue: "Lepro" })}
        />
      </div>

      {/* 关闭时：一键开启（把所有应用切到 Lepro）的醒目入口 */}
      {!isLeproActive && (
        <div className="px-5 pb-5">
          <Button
            onClick={handleEnable}
            disabled={pending}
            className="w-full gap-2 bg-emerald-600 text-white hover:bg-emerald-700"
          >
            {pending ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Zap className="h-4 w-4" />
            )}
            {loggedIn
              ? t("lepro.switch.enableAll", {
                  defaultValue: "一键开启 · 全部应用切到 Lepro",
                })
              : t("lepro.switch.loginEnable", {
                  defaultValue: "飞书登录并开启 Lepro",
                })}
          </Button>
        </div>
      )}

      {/* 开启时：展示当前档位 → 模型映射 */}
      {showMapping && (
        <div className="border-t border-emerald-200/70 px-6 pb-4 pt-4 dark:border-emerald-900/50">
          <div className="mb-3 text-xs font-medium uppercase tracking-wide text-emerald-700/80 dark:text-emerald-400/80">
            {t("lepro.switch.mappingTitle", { defaultValue: "当前模型映射" })}
          </div>
          <div className="grid gap-2.5">
            {mappingRows.map((r) => (
              <div key={r.label} className="flex items-center gap-3 text-sm">
                <span className="w-16 shrink-0 font-semibold text-foreground">
                  {r.label}
                </span>
                <ArrowRight className="h-4 w-4 shrink-0 text-emerald-600/70 dark:text-emerald-400/70" />
                <code className="min-w-0 truncate rounded-md bg-emerald-500/10 px-2.5 py-1 font-mono text-[13px] text-emerald-800 dark:text-emerald-300">
                  {r.model}
                </code>
              </div>
            ))}
            {defaultModel && (
              <div className="flex items-center gap-3 text-sm">
                <span className="w-16 shrink-0 font-medium text-muted-foreground">
                  {t("lepro.switch.tierDefault", { defaultValue: "默认" })}
                </span>
                <ArrowRight className="h-4 w-4 shrink-0 text-muted-foreground/60" />
                <code className="min-w-0 truncate rounded-md bg-muted px-2.5 py-1 font-mono text-[13px] text-muted-foreground">
                  {defaultModel}
                </code>
              </div>
            )}
            <div className="mt-1 text-xs text-muted-foreground">
              {t("lepro.switch.codexGeminiNote", {
                defaultValue:
                  "Codex / Gemini 默认用 gpt-5.5（工具内可切中转站其它模型）",
              })}
            </div>
          </div>
        </div>
      )}

      {/* 底部：第三方供应商设置（展开逐应用供应商管理） */}
      <div
        className={cn(
          "flex justify-end border-t px-5 py-3",
          isLeproActive
            ? "border-emerald-200/70 dark:border-emerald-900/50"
            : "border-border/60",
        )}
      >
        <Button
          variant={settingsOpen ? "secondary" : "ghost"}
          size="sm"
          onClick={onToggleSettings}
          className="gap-1.5 text-muted-foreground"
        >
          <Settings2 className="h-4 w-4" />
          {t("lepro.switch.thirdParty", { defaultValue: "第三方供应商设置" })}
        </Button>
      </div>
    </div>
  );
}
