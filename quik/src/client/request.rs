use crate::profile::ChromeProfile;
use http::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, USER_AGENT};

/// Injects Chrome 134-identical headers into the request map.
/// Enforces strict ordering and case-sensitivity where possible.
pub fn inject_chrome_headers(
    headers: &mut HeaderMap,
    profile: &ChromeProfile,
    sec_fetch_site: &str,
    is_initial_navigation: bool,
) {
    // 1. Client Hints (Sec-CH-UA)
    if let Ok(val) = HeaderValue::from_str(&profile.headers.sec_ch_ua) {
        headers.insert("sec-ch-ua", val);
    }
    headers.insert("sec-ch-ua-mobile", HeaderValue::from_static("?0"));
    if let Ok(val) = HeaderValue::from_str(&profile.headers.sec_ch_ua_platform) {
        headers.insert("sec-ch-ua-platform", val);
    }

    // 2. Navigation / Fetch metadata
    headers.insert("upgrade-insecure-requests", HeaderValue::from_static("1"));
    if let Ok(val) = HeaderValue::from_str(&profile.headers.user_agent) {
        headers.insert(USER_AGENT, val);
    }
    headers.insert(ACCEPT, HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"));

    // Inject dynamic sec-fetch-* state
    if let Ok(val) = HeaderValue::from_str(sec_fetch_site) {
        headers.insert("sec-fetch-site", val);
    }
    headers.insert("sec-fetch-mode", HeaderValue::from_static("navigate"));

    if is_initial_navigation {
        headers.insert("sec-fetch-user", HeaderValue::from_static("?1"));
    }

    headers.insert("sec-fetch-dest", HeaderValue::from_static("document"));

    // 3. Compression & Language
    let encoding = if profile.headers.zstd_encoding {
        "gzip, deflate, br, zstd"
    } else {
        "gzip, deflate, br"
    };
    headers.insert(ACCEPT_ENCODING, HeaderValue::from_static(encoding));
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));

    // 4. Chrome Priority Header (u=0, i for navigations)
    if profile.headers.include_priority_header {
        headers.insert("priority", HeaderValue::from_static("u=0, i"));
    }

    // 5. HPACK "Never Index" (Sensitive) markers
    // Mark cookies and auth as sensitive so they are never indexed in the HPACK dynamic table.
    for (name, value) in headers.iter_mut() {
        if name == "cookie" || name == "authorization" {
            value.set_sensitive(true);
        }
    }
}
