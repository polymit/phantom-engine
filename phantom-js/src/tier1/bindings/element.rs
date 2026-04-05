use crate::tier1::session::PhantomDomHandle;
use rquickjs::{class::Trace, Ctx, Result};

/// JS-facing HTMLElement class.
/// Stores only arena_id: u64 — per D-09, NEVER a Rust reference.
/// Methods access the DOM via ctx.userdata::<PhantomDomHandle>().
#[derive(Trace, Clone, rquickjs::JsLifetime)]
#[rquickjs::class(rename = "HTMLElement")]
pub struct JsHTMLElement {
    /// Index into the phantom-core DomTree arena.
    /// This is the ONLY link between JS and Rust DOM — by design.
    pub arena_id: u64,
}

#[rquickjs::methods]
impl JsHTMLElement {
    #[qjs(constructor)]
    pub fn new(arena_id: u64) -> Self {
        Self { arena_id }
    }

    #[qjs(get, rename = "tagName")]
    pub fn tag_name<'js>(&self, ctx: Ctx<'js>) -> Result<rquickjs::String<'js>> {
        let dom = ctx
            .userdata::<PhantomDomHandle>()
            .ok_or(rquickjs::Error::Unknown)?
            .clone();
        let name = dom.get_tag_name(self.arena_id);
        rquickjs::String::from_str(ctx, &name)
    }

    #[qjs(get, rename = "textContent")]
    pub fn text_content<'js>(&self, ctx: Ctx<'js>) -> Result<rquickjs::String<'js>> {
        let dom = ctx
            .userdata::<PhantomDomHandle>()
            .ok_or(rquickjs::Error::Unknown)?
            .clone();
        let text = dom.get_text_content(self.arena_id);
        rquickjs::String::from_str(ctx, &text)
    }

    #[qjs(get, rename = "innerText")]
    pub fn inner_text<'js>(&self, ctx: Ctx<'js>) -> Result<rquickjs::String<'js>> {
        // For v0.1: same as textContent
        self.text_content(ctx)
    }

    #[qjs(rename = "getAttribute")]
    pub fn get_attribute<'js>(&self, ctx: Ctx<'js>, name: String) -> Result<rquickjs::Value<'js>> {
        let dom = ctx
            .userdata::<PhantomDomHandle>()
            .ok_or(rquickjs::Error::Unknown)?
            .clone();
        match dom.get_attribute(self.arena_id, &name) {
            Some(val) => Ok(rquickjs::String::from_str(ctx, &val)?.into_value()),
            None => Ok(rquickjs::Value::new_null(ctx.clone())),
        }
    }

    #[qjs(rename = "querySelector")]
    pub fn query_selector<'js>(
        &self,
        ctx: Ctx<'js>,
        selector: String,
    ) -> Result<rquickjs::Value<'js>> {
        let dom = ctx
            .userdata::<PhantomDomHandle>()
            .ok_or(rquickjs::Error::Unknown)?
            .clone();
        match dom.query_selector_from(&selector, self.arena_id) {
            Some(node_id) => {
                let el = JsHTMLElement { arena_id: node_id };
                Ok(rquickjs::Class::instance(ctx.clone(), el)?.into_value())
            }
            None => Ok(rquickjs::Value::new_null(ctx.clone())),
        }
    }
}
