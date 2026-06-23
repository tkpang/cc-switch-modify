import { invoke } from "@tauri-apps/api/core";

/** `lepro_force_sync` 返回：本次同步写入的 Lepro 凭据摘要 */
export interface LeproSyncOutcome {
  base_url: string;
  group: string;
  models: string[];
}

/**
 * Lepro 集成命令的 invoke 包装（对应 src-tauri/src/commands/lepro.rs）。
 * 飞书登录走 OIDC（loopback + PKCE），token 存 OS keychain；
 * 同步会拉取分发 API 凭据并写入/切换本地 "Lepro" Claude Code provider。
 */
export const leproApi = {
  /** 飞书登录：打开系统浏览器走 OIDC，成功后 token 存 keychain，返回 true */
  login: (): Promise<boolean> => invoke("lepro_login"),
  /** 登出：清除本地 token */
  logout: (): Promise<void> => invoke("lepro_logout"),
  /** 是否已登录（本地是否存有 token） */
  authStatus: (): Promise<boolean> => invoke("lepro_auth_status"),
  /** 强制同步：拉取凭据并写入/切换本地 "Lepro" provider（覆盖 ~/.claude/settings.json） */
  forceSync: (): Promise<LeproSyncOutcome> => invoke("lepro_force_sync"),
};
