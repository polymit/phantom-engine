(function() {
    let __phantom_href = 'about:blank';

    // Called by Rust to update the URL after navigation
    globalThis.__phantom_set_location = function(url) {
        __phantom_href = url;
    };

    const locationObj = {
        get href() { return __phantom_href; },
        set href(url) {
            // Signal to Rust that navigation was requested
            // In v0.1 this is a no-op — navigation is Rust-controlled
            __phantom_href = url;
        },
        assign(url) { this.href = url; },
        replace(url) { this.href = url; },
        reload() {},
        get origin() {
            try { return new URL(__phantom_href).origin; } catch(e) { return ''; }
        },
        get pathname() {
            try { return new URL(__phantom_href).pathname; } catch(e) { return '/'; }
        },
        get hostname() {
            try { return new URL(__phantom_href).hostname; } catch(e) { return ''; }
        },
        get protocol() {
            try { return new URL(__phantom_href).protocol; } catch(e) { return 'https:'; }
        },
        get search() {
            try { return new URL(__phantom_href).search; } catch(e) { return ''; }
        },
        get hash() {
            try { return new URL(__phantom_href).hash; } catch(e) { return ''; }
        },
        toString() { return __phantom_href; },
    };

    Object.defineProperty(window, 'location', {
        get: () => locationObj,
        configurable: false,
    });
})();
