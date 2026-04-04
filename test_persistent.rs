use rquickjs::{Context, Runtime, Function, Persistent, Ctx};

fn main() {
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();
    ctx.with(|ctx: Ctx<'_>| {
        let func: Function<'_> = ctx.eval("(function() {})").unwrap();
        let p: Persistent<Function> = Persistent::save(&ctx, func);
    });
}
