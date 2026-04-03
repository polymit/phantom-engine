use crate::tier1::session::PhantomDomHandle;
use super::element::JsHTMLElement;
use super::document::JsDocument;
use super::navigator::JsNavigator;

pub async fn setup_dom_environment(
    context: &rquickjs::AsyncContext,
    dom_handle: PhantomDomHandle,
) -> Result<(), crate::error::PhantomJsError> {
    use rquickjs::async_with;
    use rquickjs::Class;

    async_with!(context => |ctx| {
        // Inject DOM handle — all class methods access DOM via this
        ctx.store_userdata(dom_handle)
            .map_err(|_| rquickjs::Error::Unknown)?;

        let globals = ctx.globals();

        // Register HTMLElement class on the global object
        Class::<JsHTMLElement>::define(&globals)?;

        // Register Document class and set window.document
        Class::<JsDocument>::define(&globals)?;
        let doc = JsDocument::new();
        let doc_instance = Class::instance(ctx.clone(), doc)?;
        globals.set("document", doc_instance)?;

        // Register Navigator
        Class::<JsNavigator>::define(&globals)?;
        let nav = JsNavigator::new();
        let nav_instance = Class::instance(ctx.clone(), nav)?;
        globals.set("navigator", nav_instance)?;

        // Register Web APIs
        crate::tier1::apis::timers::register_timers(&ctx, &globals, context.clone())?;
        crate::tier1::apis::fetch::register_fetch(&ctx, &globals)?;

        Ok::<(), rquickjs::Error>(())
    })
    .await
    .map_err(|e| crate::error::PhantomJsError::DomBinding(e.to_string()))
}
