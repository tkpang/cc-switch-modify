import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { leproApi, type LeproSyncOutcome } from "@/lib/api/lepro";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import { extractErrorMessage } from "@/utils/errorUtils";

const AUTH_STATUS_KEY = ["lepro", "authStatus"] as const;
const SYNC_SNAPSHOT_KEY = ["lepro", "syncSnapshot"] as const;

/** 最近一次成功同步的快照（纯前端记录：后端不持久化同步时间戳） */
export interface LeproSyncSnapshot {
  outcome: LeproSyncOutcome;
  syncedAt: number; // epoch ms
}

/** 是否已登录（本地是否存有 token） */
export function useLeproAuthStatus() {
  return useQuery({
    queryKey: AUTH_STATUS_KEY,
    queryFn: () => leproApi.authStatus(),
    placeholderData: false,
    retry: false,
  });
}

/**
 * 最近一次同步快照。无后端数据源——初始为 null，由 forceSync mutation
 * 通过 setQueryData 写入。staleTime + gcTime 均为 Infinity：组件卸载后不回收、
 * 也不会因失效而被 queryFn 清空（queryFn 返回已有缓存）。
 */
export function useLeproSyncSnapshot() {
  const queryClient = useQueryClient();
  return useQuery<LeproSyncSnapshot | null>({
    queryKey: SYNC_SNAPSHOT_KEY,
    // 无后端来源：刷新时返回已有快照，避免 invalidate 把它清成 null。
    queryFn: () =>
      queryClient.getQueryData<LeproSyncSnapshot | null>(SYNC_SNAPSHOT_KEY) ??
      null,
    staleTime: Infinity,
    // 离开 providers 视图（组件卸载）后不回收，否则再回来「上次同步时间」会丢失。
    gcTime: Infinity,
  });
}

/** 强制同步：拉取凭据 → 写入并切换 "Lepro" provider */
export function useLeproForceSync() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: () => leproApi.forceSync(),
    onSuccess: (outcome) => {
      queryClient.setQueryData<LeproSyncSnapshot>(SYNC_SNAPSHOT_KEY, {
        outcome,
        syncedAt: Date.now(),
      });
      // 同步会新建/切换 "Lepro" provider 并写入 ~/.claude/settings.json
      // → 刷新供应商列表（不限当前 app tab），使界面立即反映。
      queryClient.invalidateQueries({ queryKey: ["providers"] });
      toast.success(
        t("lepro.syncSuccess", {
          num: outcome.models.length,
          defaultValue: `同步成功，已可用 ${outcome.models.length} 个模型`,
        }),
        { closeButton: true },
      );
    },
    onError: (error: unknown) => {
      const detail =
        extractErrorMessage(error) ||
        t("common.unknown", { defaultValue: "未知错误" });
      toast.error(
        t("lepro.syncFailed", { detail, defaultValue: `同步失败：${detail}` }),
      );
    },
  });
}

/** 飞书登录（阻塞至浏览器完成 OIDC 回调） */
export function useLeproLogin() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: () => leproApi.login(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: AUTH_STATUS_KEY });
      toast.success(t("lepro.loginSuccess", { defaultValue: "飞书登录成功" }), {
        closeButton: true,
      });
    },
    onError: (error: unknown) => {
      const detail =
        extractErrorMessage(error) ||
        t("common.unknown", { defaultValue: "未知错误" });
      toast.error(
        t("lepro.loginFailed", { detail, defaultValue: `登录失败：${detail}` }),
      );
    },
  });
}

/** 登出：清除本地 token + 清空同步快照 */
export function useLeproLogout() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: () => leproApi.logout(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: AUTH_STATUS_KEY });
      queryClient.setQueryData<LeproSyncSnapshot | null>(
        SYNC_SNAPSHOT_KEY,
        null,
      );
      toast.success(t("lepro.logoutSuccess", { defaultValue: "已退出登录" }));
    },
    onError: (error: unknown) => {
      const detail =
        extractErrorMessage(error) ||
        t("common.unknown", { defaultValue: "未知错误" });
      toast.error(
        t("lepro.logoutFailed", {
          detail,
          defaultValue: `退出失败：${detail}`,
        }),
      );
    },
  });
}
