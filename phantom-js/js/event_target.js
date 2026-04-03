// Simple event listener registry
// Used by page JS to attach event handlers
// We store them but dispatch is handled by the ActionEngine on the Rust side

const __phantom_listeners = new WeakMap();

class PhantomEventTarget {
    addEventListener(type, listener, options) {
        if (!__phantom_listeners.has(this)) {
            __phantom_listeners.set(this, {});
        }
        const map = __phantom_listeners.get(this);
        if (!map[type]) map[type] = [];
        map[type].push({ listener, options });
    }

    removeEventListener(type, listener) {
        if (!__phantom_listeners.has(this)) return;
        const map = __phantom_listeners.get(this);
        if (!map[type]) return;
        map[type] = map[type].filter(e => e.listener !== listener);
    }

    dispatchEvent(event) {
        if (!__phantom_listeners.has(this)) return true;
        const map = __phantom_listeners.get(this);
        const listeners = map[event.type] || [];
        listeners.forEach(({ listener }) => {
            try { listener(event); } catch(e) {}
        });
        return true;
    }
}

// Make EventTarget globally available
globalThis.EventTarget = PhantomEventTarget;
