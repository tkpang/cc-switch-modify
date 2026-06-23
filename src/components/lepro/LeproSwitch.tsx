import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Loader2, Settings2, Zap } from "lucide-react";
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

interface LeproSwitchProps {
  providers: Record<string, Provider>;
  currentProviderId: string;
  onSwitch: (provider: Provider) => void;
  settingsOpen: boolean;
  onToggleSettings: () => void;
}

/**
 * 主屏的 Lepro 总开关。开 = 用 Lepro 的 API;关 = 回退到上次用的非 Lepro
 * 供应商,并露出「设置」按钮进入完整供应商管理(Claude 官方/第三方)。
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

  return (
    <div
      className={cn(
        "rounded-2xl border p-5 transition-colors",
        isLeproActive
          ? "border-emerald-300 bg-emerald-50/60 dark:border-emerald-800 dark:bg-emerald-950/30"
          : "border-border bg-muted/30",
      )}
    >
      <div className="flex items-center justify-between gap-4">
        <div className="flex min-w-0 items-center gap-3">
          <span
            className={cn(
              "flex h-11 w-11 shrink-0 items-center justify-center rounded-xl transition-colors",
              isLeproActive
                ? "bg-emerald-500 text-white"
                : "bg-muted-foreground/10 text-muted-foreground",
            )}
          >
            {pending ? (
              <Loader2 className="h-5 w-5 animate-spin" />
            ) : (
              <Zap className="h-5 w-5" />
            )}
          </span>
          <div className="min-w-0">
            <div className="text-base font-semibold">
              {t("lepro.switch.title", { defaultValue: "Lepro" })}
            </div>
            <div className="truncate text-sm text-muted-foreground">
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
    </div>
  );
}
