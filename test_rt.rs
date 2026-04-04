use rquickjs::{Context, Runtime, Function, Persistent, Ctx, AsyncRuntime, AsyncContext, async_with};

#[tokio::main]
async fn main() {
    let rt = AsyncRuntime::new().unwrap();
    let context = AsyncContext::full(&rt).await.unwrap();
    
    async_with!(context => |ctx| {
        let _ = rt.execute_pending_job().await.unwrap();
    }).await;
}
