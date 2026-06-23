//! 追踪式后台任务调度器 — application/proxy/
//!
//! 封装 fire-and-forget 的 `tokio::spawn` 调用，
//! 统一处理"飞行中写入计数 +1 → spawn → 任务尾 -1"模板，
//! 并通过 `Handle::try_current` 守卫避免运行时关闭后 spawn 触发 panic。
//!
//! 由 `ProxyPipeline` 在初始化时构造、`Clone` 传给 `ProxyCallRecord` 等下游组件。
//! 主进程在优雅关闭时轮询计数器归零，确保所有后台写入落库再退出。

use std::future::Future;
use std::sync::atomic::AtomicI64;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// 飞行中写入计数器的轻量封装
///
/// 内部持有 `Arc<AtomicI64>`，可廉价 `Clone`。
/// 主进程持有同一个计数器以观察写入归零。
#[derive(Clone)]
pub struct TrackedSpawner {
    counter: Arc<AtomicI64>,
}

impl TrackedSpawner {
    /// 由计数器构造调度器
    ///
    /// 主进程在 `main.rs` 初始化时持有同一个 `Arc<AtomicI64>`，
    /// 用于优雅关闭时轮询归零。
    pub fn new(counter: Arc<AtomicI64>) -> Self {
        TrackedSpawner { counter }
    }

    /// fire-and-forget spawn 后台写入任务
    ///
    /// `operation` 为静态标签（如 "proxy_log" / "session_affinity"），
    /// 用于运行时关闭场景下的降级日志和失败日志的字段标识。
    ///
    /// 内部行为：
    /// 1. `Handle::try_current` 守卫 —— 运行时已关闭则记录 warn 并放弃
    /// 2. 计数 +1 → spawn future → 任务尾计数 -1
    /// 3. 若 future 返回 `Err`，以 `operation` 标签记录 warn
    pub fn spawn<F, E>(&self, operation: &'static str, fut: F)
    where
        F: Future<Output = Result<(), E>> + Send + 'static,
        E: std::fmt::Display + Send + 'static,
    {
        let handle = match tokio::runtime::Handle::try_current() {
            Ok(h) => h,
            Err(_) => {
                tracing::warn!(operation, "运行时已关闭，后台写入丢弃");
                return;
            }
        };

        self.counter.fetch_add(1, Ordering::Release);
        let counter = self.counter.clone();
        handle.spawn(async move {
            if let Err(e) = fut.await {
                tracing::warn!(operation, error = %e, "后台写入失败");
            }
            counter.fetch_sub(1, Ordering::Release);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn counter_increments_and_decrements_on_success() {
        let counter = Arc::new(AtomicI64::new(0));
        let spawner = TrackedSpawner::new(counter.clone());

        spawner.spawn::<_, std::io::Error>("test_op", async { Ok(()) });

        // 等待任务完成
        for _ in 0..50 {
            if counter.load(Ordering::Acquire) == 0 {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("计数器未归零");
    }

    #[tokio::test]
    async fn counter_decrements_even_on_error() {
        let counter = Arc::new(AtomicI64::new(0));
        let spawner = TrackedSpawner::new(counter.clone());

        spawner.spawn::<_, std::io::Error>("test_op", async { Err(std::io::Error::other("boom")) });

        for _ in 0..50 {
            if counter.load(Ordering::Acquire) == 0 {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("计数器未归零");
    }
}
