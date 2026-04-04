use rquickjs::{AsyncRuntime, AsyncContext, Function, Persistent};

#[tokio::main]
async fn main() {
    let rt = AsyncRuntime::new().unwrap();
    let ctx = AsyncContext::full(&rt).await.unwrap();
    
    rquickjs::async_with!(ctx.clone() => |qctx| {
        let f = Function::new(qctx.clone(), || ()).unwrap();
        let p = Persistent::save(&qctx, f);
        
        let c = ctx.clone();
        tokio::spawn(async move {
            rquickjs::async_with!(c => |qctx2| {
                if let Ok(cb) = p.restore(&qctx2) {
                    let _ = cb.call::<(),()>(());
                }
            }).await;
        });
    }).await;
}
