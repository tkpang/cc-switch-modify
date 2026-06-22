//! 飞书 OIDC RFC 8252 回调监听器。
//!
//! 绑定 `127.0.0.1:0`（系统分配端口），暴露 `redirect_uri`
//! (`http://127.0.0.1:{port}/cb`)，等待单次 `GET /cb?code=...&state=...`，
//! 返回 [`crate::oidc::Callback`]（含 code + state）后立即关停服务器。

use axum::{
    Router,
    extract::Query,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::{
    net::TcpListener,
    sync::oneshot,
};

use crate::error::LeproError;
use crate::oidc::Callback;

/// 回调查询参数。
#[derive(Debug, Deserialize)]
struct CallbackParams {
    code: String,
    #[serde(default)]
    state: String,
}

/// 持有端口监听状态的句柄，调用 `wait()` 可等待浏览器回调完成。
pub struct LoopbackHandle {
    /// 完整的重定向 URI，例如 `http://127.0.0.1:54321/cb`。
    pub redirect_uri: String,
    /// 收到 callback 后触发的接收端（消费即关停）。
    cb_rx: oneshot::Receiver<Callback>,
    /// 服务器任务句柄，`wait()` 完成后自动 abort。
    server_task: tokio::task::JoinHandle<()>,
}

impl LoopbackHandle {
    /// 等待浏览器回调并返回 [`Callback`]（含 `code` 与 `state`）。
    /// 此方法消费 `self`，调用后服务器停止。
    pub async fn wait(self) -> Result<Callback, LeproError> {
        let cb = self.cb_rx.await.map_err(|_| {
            LeproError::Oidc("loopback: 服务器在收到 code 前已关闭".to_string())
        })?;
        // 服务器发送 callback 后会通过 graceful shutdown 自行退出，这里 abort 是安全的双保险。
        self.server_task.abort();
        Ok(cb)
    }
}

/// 绑定 `127.0.0.1:0`，启动临时 HTTP 服务器，返回 [`LoopbackHandle`]。
///
/// 服务器在收到第一个合法的 `GET /cb?code=...` 请求后：
/// 1. 向浏览器返回 200「登录成功，可关闭本页」。
/// 2. 通过 channel 将 `code` 传回调用方。
/// 3. 触发 graceful shutdown，服务器退出。
pub async fn listen_once() -> Result<LoopbackHandle, LeproError> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| LeproError::Io(format!("loopback bind 失败: {e}")))?;

    let port = listener
        .local_addr()
        .map_err(|e| LeproError::Io(format!("loopback local_addr 失败: {e}")))?
        .port();

    let redirect_uri = format!("http://127.0.0.1:{port}/cb");

    // callback 传出 channel
    let (cb_tx, cb_rx) = oneshot::channel::<Callback>();
    // graceful shutdown 信号 channel
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    // 将发送端包在 Arc<Mutex<Option<...>>> 里，以便在 axum handler（可能被多次调用）中安全取用。
    let cb_tx = Arc::new(tokio::sync::Mutex::new(Some(cb_tx)));
    let shutdown_tx = Arc::new(tokio::sync::Mutex::new(Some(shutdown_tx)));

    let router = Router::new().route(
        "/cb",
        get({
            let cb_tx = Arc::clone(&cb_tx);
            let shutdown_tx = Arc::clone(&shutdown_tx);
            move |Query(params): Query<CallbackParams>| {
                let cb_tx = Arc::clone(&cb_tx);
                let shutdown_tx = Arc::clone(&shutdown_tx);
                async move {
                    // 仅第一次请求生效（take 保证幂等）。
                    let mut cb_guard = cb_tx.lock().await;
                    if let Some(tx) = cb_guard.take() {
                        let _ = tx.send(Callback { code: params.code, state: params.state });
                    }
                    let mut shutdown_guard = shutdown_tx.lock().await;
                    if let Some(tx) = shutdown_guard.take() {
                        let _ = tx.send(());
                    }
                    (StatusCode::OK, "登录成功，可关闭本页").into_response()
                }
            }
        }),
    );

    let server_task = tokio::spawn(async move {
        let serve = axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            });
        // 服务器退出时忽略错误（调用方已通过 callback channel 拿到结果）。
        let _ = serve.await;
    });

    Ok(LoopbackHandle {
        redirect_uri,
        cb_rx,
        server_task,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn loopback_captures_code() {
        let h = listen_once().await.unwrap();
        let uri = h.redirect_uri.clone();
        let waiter = tokio::spawn(h.wait());
        let _ = reqwest::get(format!("{uri}?code=THECODE&state=st"))
            .await
            .unwrap();
        let cb = waiter.await.unwrap().unwrap();
        assert_eq!(cb.code, "THECODE");
        assert_eq!(cb.state, "st");
    }
}
