use rquickjs::{Context, Runtime, Function, Persistent, Ctx, AsyncRuntime, AsyncContext, async_with};
use std::time::Duration;

#[tokio::main]
async fn main() {
    let rt = AsyncRuntime::new().unwrap();
    let context = AsyncContext::full(&rt).await.unwrap();
    
    async_with!(context => |ctx| {
        let f = Function::new(ctx.clone(), || ()).unwrap();
        // Since we spawn into ctx, we don't need Persistent!
        // Wait, tokio::time::sleep is async!
        let cb = f.clone();
        ctx.spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            let _ = cb.call::<(),()>(());
        });
    }).await;
}
