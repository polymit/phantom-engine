use super::element::JsHTMLElement;
use crate::tier1::session::PhantomDomHandle;
use rquickjs::{Ctx, Result, class::Trace};

#[derive(Trace, Clone, rquickjs::JsLifetime)]
#[rquickjs::class(rename = "Document")]
pub struct JsDocument {}

impl Default for JsDocument {
    fn default() -> Self {
        Self::new()
    }
}

#[rquickjs::methods]
impl JsDocument {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {}
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
        match dom.query_selector(&selector) {
            Some(node_id) => {
                let el = JsHTMLElement { arena_id: node_id };
                Ok(rquickjs::Class::instance(ctx.clone(), el)?.into_value())
            }
            None => Ok(rquickjs::Value::new_null(ctx.clone())),
        }
    }

    #[qjs(rename = "querySelectorAll")]
    pub fn query_selector_all<'js>(
        &self,
        ctx: Ctx<'js>,
        selector: String,
    ) -> Result<rquickjs::Array<'js>> {
        let dom = ctx
            .userdata::<PhantomDomHandle>()
            .ok_or(rquickjs::Error::Unknown)?
            .clone();
        let ids = dom.query_selector_all(&selector);
        let array = rquickjs::Array::new(ctx.clone())?;
        for (i, node_id) in ids.into_iter().enumerate() {
            let el = JsHTMLElement { arena_id: node_id };
            let instance = rquickjs::Class::instance(ctx.clone(), el)?;
            array.set(i, instance)?;
        }
        Ok(array)
    }

    #[qjs(rename = "getElementById")]
    pub fn get_element_by_id<'js>(
        &self,
        ctx: Ctx<'js>,
        id: String,
    ) -> Result<rquickjs::Value<'js>> {
        let dom = ctx
            .userdata::<PhantomDomHandle>()
            .ok_or(rquickjs::Error::Unknown)?
            .clone();
        match dom.get_element_by_id(&id) {
            Some(node_id) => {
                let el = JsHTMLElement { arena_id: node_id };
                Ok(rquickjs::Class::instance(ctx.clone(), el)?.into_value())
            }
            None => Ok(rquickjs::Value::new_null(ctx.clone())),
        }
    }

    #[qjs(rename = "createElement")]
    pub fn create_element<'js>(&self, ctx: Ctx<'js>, tag: String) -> Result<rquickjs::Value<'js>> {
        let dom = ctx
            .userdata::<PhantomDomHandle>()
            .ok_or(rquickjs::Error::Unknown)?
            .clone();
        let node_id = dom.create_element(&tag);
        let el = JsHTMLElement { arena_id: node_id };
        Ok(rquickjs::Class::instance(ctx.clone(), el)?.into_value())
    }

    #[qjs(get, rename = "title")]
    pub fn title<'js>(&self, ctx: Ctx<'js>) -> Result<rquickjs::String<'js>> {
        let dom = ctx
            .userdata::<PhantomDomHandle>()
            .ok_or(rquickjs::Error::Unknown)?
            .clone();
        let title = dom.get_title();
        rquickjs::String::from_str(ctx.clone(), &title)
    }
}
