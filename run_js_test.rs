use rquickjs::{Context, Runtime, catch};

fn main() {
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();
    ctx.with(|ctx| {
        let setup = r#"
        globalThis.__phantom_persona = {};
        globalThis.window = globalThis;
        globalThis.navigator = {};
        globalThis.PluginArray = function() {};
        globalThis.Plugin = function() {};
        "#;
        ctx.eval::<(), _>(setup).unwrap();
        let shims = include_str!("phantom-js/js/browser_shims.js");
        if let Err(e) = ctx.eval::<(), _>(shims) {
            println!("Error: {:?}", catch(&ctx));
        } else {
            println!("No error.");
        }
    });
}
