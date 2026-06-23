import { LogIn, Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  useLeproAuthStatus,
  useLeproLogin,
  useLeproForceSync,
} from "@/lib/query/lepro";

/**
 * 未登录时显示「飞书登录」按钮；登录成功后自动触发一次同步，
 * 省去用户再手动点一次「立即同步」。加载中或已登录则不渲染。
 */
export function LeproLoginButton() {
  const { t } = useTranslation();
  const { data: loggedIn, isLoading } = useLeproAuthStatus();
  const login = useLeproLogin();
  const forceSync = useLeproForceSync();

  if (isLoading || loggedIn) return null;

  const pending = login.isPending || forceSync.isPending;

  const handleLogin = () => {
    login.mutate(undefined, {
      onSuccess: () => forceSync.mutate(),
    });
  };

  return (
    <Button
      variant="default"
      size="sm"
      onClick={handleLogin}
      disabled={pending}
      className="gap-2"
      title={t("lepro.loginTooltip", {
        defaultValue: "用飞书登录并自动同步 Lepro API key 到 Claude Code",
      })}
    >
      {pending ? (
        <Loader2 className="h-4 w-4 animate-spin" />
      ) : (
        <LogIn className="h-4 w-4" />
      )}
      {t("lepro.login", { defaultValue: "飞书登录" })}
    </Button>
  );
}
