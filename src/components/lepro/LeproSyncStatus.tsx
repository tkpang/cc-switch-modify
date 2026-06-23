import { CheckCircle2, RefreshCw, LogOut, Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuItem,
  DropdownMenuSeparator,
} from "@/components/ui/dropdown-menu";
import {
  useLeproAuthStatus,
  useLeproSyncSnapshot,
  useLeproForceSync,
  useLeproLogout,
} from "@/lib/query/lepro";

/**
 * 已登录时显示同步状态下拉：分组 / 上次同步时间 / 可用模型，
 * 并提供「立即同步」「退出登录」操作。未登录则不渲染。
 */
export function LeproSyncStatus() {
  const { t } = useTranslation();
  const { data: loggedIn } = useLeproAuthStatus();
  const { data: snapshot } = useLeproSyncSnapshot();
  const forceSync = useLeproForceSync();
  const logout = useLeproLogout();

  if (!loggedIn) return null;

  const models = snapshot?.outcome.models ?? [];
  const group = snapshot?.outcome.group;
  const syncedAt = snapshot?.syncedAt;

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          size="sm"
          className="gap-1.5 text-emerald-600 hover:text-emerald-700 dark:text-emerald-400 dark:hover:text-emerald-300"
          title={t("lepro.statusTitle", { defaultValue: "Lepro 已登录" })}
        >
          {forceSync.isPending ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <CheckCircle2 className="h-4 w-4" />
          )}
          Lepro
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-64">
        <DropdownMenuLabel className="flex flex-col gap-0.5">
          <span>
            {t("lepro.statusTitle", { defaultValue: "Lepro 已登录" })}
          </span>
          {group && (
            <span className="text-xs font-normal text-muted-foreground">
              {t("lepro.group", { defaultValue: "分组" })}: {group}
            </span>
          )}
          <span className="text-xs font-normal text-muted-foreground">
            {syncedAt
              ? t("lepro.lastSync", {
                  time: new Date(syncedAt).toLocaleTimeString(),
                  defaultValue: `上次同步 ${new Date(
                    syncedAt,
                  ).toLocaleTimeString()}`,
                })
              : t("lepro.neverSynced", { defaultValue: "尚未同步" })}
          </span>
        </DropdownMenuLabel>

        {models.length > 0 && (
          <>
            <DropdownMenuSeparator />
            <DropdownMenuLabel className="text-xs font-normal text-muted-foreground">
              {t("lepro.models", {
                num: models.length,
                defaultValue: `可用模型（${models.length}）`,
              })}
            </DropdownMenuLabel>
            <div className="max-h-40 overflow-y-auto px-2 pb-1">
              {models.map((m) => (
                <div key={m} className="truncate py-0.5 text-xs">
                  {m}
                </div>
              ))}
            </div>
          </>
        )}

        <DropdownMenuSeparator />
        <DropdownMenuItem
          onSelect={(e) => {
            // 阻止下拉菜单在点击「立即同步」时关闭，便于看到同步进度
            e.preventDefault();
            forceSync.mutate();
          }}
          disabled={forceSync.isPending}
        >
          <RefreshCw className="mr-2 h-4 w-4" />
          {t("lepro.syncNow", { defaultValue: "立即同步" })}
        </DropdownMenuItem>
        <DropdownMenuItem
          onSelect={() => logout.mutate()}
          disabled={logout.isPending}
          className="text-destructive focus:text-destructive"
        >
          <LogOut className="mr-2 h-4 w-4" />
          {t("lepro.logout", { defaultValue: "退出登录" })}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
