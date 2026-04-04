use rquickjs::{Context, Runtime, Function, Persistent, Ctx, AsyncRuntime, AsyncContext, async_with};

#[tokio::main]
async fn main() {
    let rt = AsyncRuntime::new().unwrap();
    let context = AsyncContext::full(&rt).await.unwrap();
    let c = context.clone();
    
    async_with!(context => |ctx| {
        c.spawn(async move {
            println!("Spawned!");
        });
    }).await;
}
