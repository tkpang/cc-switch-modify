import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { ArrowRight, Loader2, Settings2, Zap } from "lucide-react";
import { Switch } from "@/components/ui/switch";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { Provider } from "@/types";
import {
  useLeproAuthStatus,
  useLeproLogin,
  useLeproForceSync,
} from "@/lib/query/lepro";

const LEPRO_ID = "lepro";
const LAST_NONLEPRO_KEY = "lepro-last-nonlepro";

// 档位 → settings.json env key,ON 时据此展示「档位 → 实际模型」映射。
const TIER_ROWS: { label: string; envKey: string }[] = [
  { label: "Opus", envKey: "ANTHROPIC_DEFAULT_OPUS_MODEL" },
  { label: "Sonnet", envKey: "ANTHROPIC_DEFAULT_SONNET_MODEL" },
  { label: "Haiku", envKey: "ANTHROPIC_DEFAULT_HAIKU_MODEL" },
];

interface LeproSwitchProps {
  providers: Record<string, Provider>;
  currentProviderId: string;
  onSwitch: (provider: Provider) => void;
  settingsOpen: boolean;
  onToggleSettings: () => void;
}

/**
 * 主屏的 Lepro 总开关。开 = 用 Lepro 的 API,卡片放大并展示当前档位→模型映射;
 * 关 = 回退到上次用的非 Lepro 供应商,卡片收紧并露出「设置」按钮进入完整供应商管理。
 * app 打开后若已登录且正在用 Lepro,会自动同步一次以刷新映射。
 */
export function LeproSwitch({
  providers,
  currentProviderId,
  onSwitch,
  settingsOpen,
  onToggleSettings,
}: LeproSwitchProps) {
  const { t } = useTranslation();
  const { data: loggedIn } = useLeproAuthStatus();
  const login = useLeproLogin();
  const forceSync = useLeproForceSync();

  const isLeproActive = currentProviderId === LEPRO_ID;
  const hasLeproProvider = !!providers[LEPRO_ID];
  const pending = login.isPending || forceSync.isPending;

  // 记住最近一次使用的非 Lepro 供应商,关 Lepro 时回退到它。
  useEffect(() => {
    if (currentProviderId && currentProviderId !== LEPRO_ID) {
      localStorage.setItem(LAST_NONLEPRO_KEY, currentProviderId);
    }
  }, [currentProviderId]);

  // app 打开后:若已登录且正在用 Lepro,自动同步一次刷新模型映射(每次启动仅一次)。
  const didAutoSync = useRef(false);
  useEffect(() => {
    if (didAutoSync.current) return;
    if (loggedIn && isLeproActive) {
      didAutoSync.current = true;
      forceSync.mutate();
    }
  }, [loggedIn, isLeproActive, forceSync]);

  const fallbackNonLepro = (): string | null => {
    const last = localStorage.getItem(LAST_NONLEPRO_KEY);
    if (last && last !== LEPRO_ID && providers[last]) return last;
    return Object.keys(providers).find((id) => id !== LEPRO_ID) ?? null;
  };

  const handleToggle = (checked: boolean) => {
    if (pending) return;
    if (checked) {
      if (!loggedIn) {
        // 未登录:飞书登录成功后自动同步(会建 lepro provider 并切过去)。
        login.mutate(undefined, { onSuccess: () => forceSync.mutate() });
      } else if (hasLeproProvider) {
        onSwitch(providers[LEPRO_ID]);
      } else {
        forceSync.mutate();
      }
    } else {
      const target = fallbackNonLepro();
      if (target && providers[target]) onSwitch(providers[target]);
    }
  };

  const activeName = providers[currentProviderId]?.name ?? currentProviderId;

  // 当前 Lepro provider 写入 settings.json 的 env(含档位→模型映射)。
  const leproEnv = (providers[LEPRO_ID]?.settingsConfig?.env ?? {}) as Record<
    string,
    unknown
  >;
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
      {/* 头部:图标 + 标题/状态 + (设置) + 开关 */}
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
            <div
              className={cn(
                "truncate text-muted-foreground",
                isLeproActive ? "text-sm mt-0.5" : "text-sm",
              )}
            >
              {isLeproActive
                ? t("lepro.switch.onHint", {
                    defaultValue: "Claude Code 正在使用 Lepro API",
                  })
                : t("lepro.switch.offHint", {
                    name: activeName,
                    defaultValue: `已关闭 · 当前供应商：${activeName}`,
                  })}
            </div>
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          {!isLeproActive && (
            <Button
              variant={settingsOpen ? "secondary" : "outline"}
              size="sm"
              onClick={onToggleSettings}
              className="gap-1.5"
            >
              <Settings2 className="h-4 w-4" />
              {t("lepro.switch.settings", { defaultValue: "设置" })}
            </Button>
          )}
          <Switch
            checked={isLeproActive}
            onCheckedChange={handleToggle}
            disabled={pending}
            aria-label={t("lepro.switch.title", { defaultValue: "Lepro" })}
          />
        </div>
      </div>

      {/* ON 时展示当前档位→模型映射,方便确认 Claude Code 实际路由到哪个模型 */}
      {showMapping && (
        <div className="border-t border-emerald-200/70 px-6 pb-5 pt-4 dark:border-emerald-900/50">
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
          </div>
        </div>
      )}
    </div>
  );
}
