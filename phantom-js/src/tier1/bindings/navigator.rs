use rquickjs::{class::Trace, Ctx, Result};

/// JS-facing Navigator class.
/// Fallback for properties, should shims not run.
#[derive(Trace, Clone, rquickjs::JsLifetime)]
#[rquickjs::class(rename = "Navigator")]
pub struct JsNavigator {}

impl Default for JsNavigator {
    fn default() -> Self {
        Self::new()
    }
}

#[rquickjs::methods]
impl JsNavigator {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {}
    }

    #[qjs(get, rename = "userAgent")]
    pub fn user_agent<'js>(&self, ctx: Ctx<'js>) -> Result<rquickjs::String<'js>> {
        rquickjs::String::from_str(ctx, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36")
    }

    #[qjs(get, rename = "platform")]
    pub fn platform<'js>(&self, ctx: Ctx<'js>) -> Result<rquickjs::String<'js>> {
        rquickjs::String::from_str(ctx, "Win32")
    }

    #[qjs(get, rename = "language")]
    pub fn language<'js>(&self, ctx: Ctx<'js>) -> Result<rquickjs::String<'js>> {
        rquickjs::String::from_str(ctx, "en-US")
    }

    #[qjs(get, rename = "hardwareConcurrency")]
    pub fn hardware_concurrency<'js>(&self, _ctx: Ctx<'js>) -> Result<u32> {
        Ok(8)
    }

    #[qjs(get, rename = "deviceMemory")]
    pub fn device_memory<'js>(&self, _ctx: Ctx<'js>) -> Result<u32> {
        Ok(8)
    }
}
