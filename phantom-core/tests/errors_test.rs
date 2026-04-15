use phantom_core::errors::*;
use std::io;

#[test]
fn test_network_error_dns_display() {
    let err = NetworkError::Dns {
        host: "example.com".to_string(),
        source: io::Error::new(io::ErrorKind::NotFound, "not found"),
    };
    assert!(err.to_string().contains("example.com"));
}

#[test]
fn test_dom_error_element_not_found_display() {
    let err = DomError::ElementNotFound { selector: ".btn".to_string() };
    assert_eq!(err.to_string(), "element not found: '.btn'");
}

#[test]
fn test_js_error_timeout_display() {
    let err = JsError::Timeout { timeout_ms: 10_000 };
    assert_eq!(err.to_string(), "script timeout after 10000ms");
}

#[test]
fn test_js_error_oom_display() {
    let err = JsError::OutOfMemory;
    assert_eq!(err.to_string(), "JavaScript heap OOM");
}

#[test]
fn test_browser_session_error_budget_display() {
    let err = BrowserSessionError::BudgetExceeded {
        resource: "memory".to_string(),
        used: 100,
        limit: 50,
    };
    assert_eq!(err.to_string(), "budget exceeded: memory 100/50");
}

#[test]
fn test_internal_error_runtime_pool_display() {
    let err = InternalError::RuntimePoolExhausted { max: 10 };
    assert_eq!(err.to_string(), "runtime pool exhausted (max 10)");
}



#[test]
fn test_string_converts_to_js_error() {
    let e: JsError = String::from("msg").into();
    if let JsError::UncaughtException { message, stack } = e {
        assert_eq!(message, "msg");
        assert_eq!(stack, "");
    } else {
        panic!("Wrong variant");
    }
}

#[test]
fn test_js_eval_timeout_ms() {
    assert_eq!(JS_EVAL_TIMEOUT_MS, 10_000);
}

#[test]
fn test_quickjs_heap_limit() {
    assert_eq!(QUICKJS_HEAP_LIMIT_BYTES, 52428800);
}

#[test]
fn test_v8_heap_limit() {
    assert_eq!(V8_HEAP_LIMIT_BYTES, 536870912);
}

#[test]
fn test_navigation_error_too_many_redirects() {
    let err = NavigationError::TooManyRedirects {
        url: "http://a.com".to_string(),
        location: "http://b.com".to_string(),
        count: 5,
    };
    assert_eq!(err.to_string(), "too many redirects for http://a.com: 5 redirects (last: http://b.com)");
}

#[test]
fn test_dom_error_not_interactable() {
    let err = DomError::NotInteractable {
        reason: "hidden".to_string(),
        selector: "#x".to_string(),
    };
    assert_eq!(err.to_string(), "not interactable: hidden");
}
