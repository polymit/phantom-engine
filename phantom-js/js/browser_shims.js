// 1. navigator.webdriver override
Object.defineProperty(navigator, 'webdriver', {
    value: undefined,
    writable: false,
    configurable: false,
    enumerable: false,
});

// 2. window.chrome — full object with runtime, loadTimes, csi, app
const createRuntimeEvent = () => {
    const listeners = new Set();
    return {
        addListener: function(listener) {
            if (typeof listener === 'function') listeners.add(listener);
        },
        removeListener: function(listener) {
            listeners.delete(listener);
        },
        hasListener: function(listener) {
            return listeners.has(listener);
        },
        hasListeners: function() {
            return listeners.size > 0;
        }
    };
};
const createRuntimePort = () => ({
    name: '',
    sender: undefined,
    disconnect: function() {},
    postMessage: function() {},
    onDisconnect: createRuntimeEvent(),
    onMessage: createRuntimeEvent()
});
const runtimeId = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
window.chrome = {
    runtime: {
        id: undefined,
        connect: function() {
            return createRuntimePort();
        },
        sendMessage: function() {
            const cb = arguments[arguments.length - 1];
            if (typeof cb === 'function') cb();
            return Promise.resolve();
        },
        onMessage: createRuntimeEvent(),
        onConnect: createRuntimeEvent(),
        getManifest: function() {
            return {};
        },
        getURL: function(path) {
            let suffix = String(path || '');
            while (suffix.startsWith('/')) suffix = suffix.slice(1);
            return `chrome-extension://${runtimeId}/${suffix}`;
        },
        PlatformOs: {
            MAC: 'mac',
            WIN: 'win',
            ANDROID: 'android',
            CROS: 'cros',
            LINUX: 'linux',
            OPENBSD: 'openbsd'
        },
        PlatformArch: {
            ARM: 'arm',
            X86_32: 'x86-32',
            X86_64: 'x86-64',
            MIPS: 'mips',
            MIPS64: 'mips64'
        }
    },
    loadTimes: function() { return {}; },
    csi: function() { return {}; },
    app: {}
};

// 3. navigator.plugins + navigator.mimeTypes
const pluginProto = globalThis.Plugin?.prototype || Object.prototype;
const pluginArrayProto = globalThis.PluginArray?.prototype || Object.prototype;
const mimeTypeProto = globalThis.MimeType?.prototype || Object.prototype;
const mimeTypeArrayProto = globalThis.MimeTypeArray?.prototype || Object.prototype;
const buildMimeTypeArray = (items) => {
    const arr = Object.create(mimeTypeArrayProto);
    items.forEach((mt, idx) => {
        arr[idx] = mt;
        arr[mt.type] = mt;
    });
    Object.defineProperties(arr, {
        length: { value: items.length, enumerable: true },
        item: {
            value: function(idx) {
                return this[idx] || null;
            }
        },
        namedItem: {
            value: function(name) {
                return this[name] || null;
            }
        }
    });
    return arr;
};
const createPlugin = (name, description, filename, mimeSpecs) => {
    const plugin = Object.create(pluginProto);
    const mimeTypes = mimeSpecs.map(spec => {
        const mt = Object.create(mimeTypeProto);
        Object.defineProperties(mt, {
            type: { value: spec.type, enumerable: true },
            suffixes: { value: spec.suffixes, enumerable: true },
            description: { value: spec.description, enumerable: true },
            enabledPlugin: { get: () => plugin, enumerable: true }
        });
        return mt;
    });
    const mimeTypeArray = buildMimeTypeArray(mimeTypes);
    mimeTypes.forEach((mt, idx) => {
        plugin[idx] = mt;
        plugin[mt.type] = mt;
    });
    Object.defineProperties(plugin, {
        name: { value: name, enumerable: true },
        description: { value: description, enumerable: true },
        filename: { value: filename, enumerable: true },
        length: { value: mimeTypes.length, enumerable: true },
        item: {
            value: function(idx) {
                return this[idx] || null;
            }
        },
        namedItem: {
            value: function(name) {
                return this[name] || null;
            }
        },
        mimeTypes: { value: mimeTypeArray, enumerable: true }
    });
    return { plugin, mimeTypes };
};
const pdfMimeSpecs = [
    { type: "application/pdf", suffixes: "pdf", description: "Portable Document Format" },
    { type: "text/pdf", suffixes: "pdf", description: "Portable Document Format" }
];
const pluginEntries = [
    createPlugin("PDF Viewer", "Portable Document Format", "internal-pdf-viewer", pdfMimeSpecs),
    createPlugin("Chrome PDF Viewer", "Portable Document Format", "internal-pdf-viewer", pdfMimeSpecs),
    createPlugin("Chromium PDF Viewer", "Portable Document Format", "internal-pdf-viewer", pdfMimeSpecs),
    createPlugin("Microsoft Edge PDF Viewer", "Portable Document Format", "internal-pdf-viewer", pdfMimeSpecs),
    createPlugin("WebKit built-in PDF", "Portable Document Format", "internal-pdf-viewer", pdfMimeSpecs)
];
const pluginsList = pluginEntries.map(entry => entry.plugin);
const mimeTypeByName = new Map();
pluginEntries.forEach(entry => {
    entry.mimeTypes.forEach(mt => {
        if (!mimeTypeByName.has(mt.type)) {
            mimeTypeByName.set(mt.type, mt);
        }
    });
});
const mimeTypesList = Array.from(mimeTypeByName.values());
const buildPluginArray = () => {
    const arr = Object.create(pluginArrayProto);
    pluginsList.forEach((plugin, idx) => {
        arr[idx] = plugin;
        arr[plugin.name] = plugin;
    });
    Object.defineProperties(arr, {
        length: { value: pluginsList.length, enumerable: true },
        item: {
            value: function(idx) {
                return this[idx] || null;
            }
        },
        namedItem: {
            value: function(name) {
                return this[name] || null;
            }
        }
    });
    return arr;
};
Object.defineProperty(navigator, 'plugins', {
    get: () => buildPluginArray(),
    configurable: false,
    enumerable: true
});
Object.defineProperty(navigator, 'mimeTypes', {
    get: () => buildMimeTypeArray(mimeTypesList),
    configurable: false,
    enumerable: true
});

// 4. Permissions API consistency fix
if (navigator.permissions) {
    const originalQuery = navigator.permissions.query;
    navigator.permissions.query = parameters => (
        parameters.name === 'notifications' ?
            Promise.resolve({ state: Notification.permission }) :
            originalQuery.call(navigator.permissions, parameters)
    );
} else {
    navigator.permissions = {
        query: parameters => Promise.resolve({ state: 'prompt', onchange: null }),
    };
}

// 5. window.outerWidth / window.outerHeight
Object.defineProperty(window, 'outerWidth', {
    get: () => __phantom_persona.screen_width || 1920
});
Object.defineProperty(window, 'outerHeight', {
    get: () => (__phantom_persona.screen_height || 1080) - 40
});

// 6. navigator.connection.rtt
const connectionSeed = BigInt(__phantom_persona.canvas_noise_seed || 1n) ^ 0xA5A5A5A5n;
const connectionInfo = Object.freeze({
    rtt: 100 + Number(connectionSeed % 50n),
    effectiveType: '4g',
    downlink: 10.0,
    saveData: false,
    type: 'wifi'
});
Object.defineProperty(navigator, 'connection', {
    get: () => connectionInfo,
    configurable: false,
    enumerable: true
});

// 7. navigator.hardwareConcurrency (from __phantom_persona)
Object.defineProperty(navigator, 'hardwareConcurrency', {
    get: () => __phantom_persona.hardware_concurrency || 8
});

// 8. navigator.deviceMemory (from __phantom_persona)
Object.defineProperty(navigator, 'deviceMemory', {
    get: () => __phantom_persona.device_memory || 8
});

// 9. navigator.language + navigator.languages (from __phantom_persona)
Object.defineProperty(navigator, 'language', {
    get: () => __phantom_persona.language || 'en-US'
});
Object.defineProperty(navigator, 'languages', {
    get: () => __phantom_persona.languages || ['en-US', 'en']
});

// 10. navigator.userAgentData with full getHighEntropyValues()
const uaMajor = String(__phantom_persona.chrome_major || '133');
const uaData = {
    brands: [
        { brand: "Not_A Brand", version: "24" },
        { brand: "Chromium", version: uaMajor },
        { brand: "Google Chrome", version: uaMajor }
    ],
    mobile: false,
    platform: __phantom_persona.ua_platform || "Windows",
    getHighEntropyValues: function(hints) {
        return Promise.resolve({
            architecture: __phantom_persona.ua_architecture || "x86",
            bitness: __phantom_persona.ua_bitness || "64",
            brands: this.brands,
            mobile: this.mobile,
            model: "",
            platform: this.platform,
            platformVersion: __phantom_persona.platform_version || "15.0.0",
            uaFullVersion: __phantom_persona.ua_full_version || "133.0.6943.98"
        });
    }
};
Object.defineProperty(navigator, 'userAgentData', {
    value: uaData,
    writable: false,
    configurable: false,
    enumerable: true
});

// 11. Canvas getImageData noise (seeded, 1.5% pixels, ±1 value)
const originalGetImageData = globalThis.CanvasRenderingContext2D?.prototype.getImageData;
if (originalGetImageData) {
    globalThis.CanvasRenderingContext2D.prototype.getImageData = function(x, y, width, height) {
        const imageData = originalGetImageData.call(this, x, y, width, height);
        const data = imageData.data;
        let seed = BigInt(__phantom_persona.canvas_noise_seed || 1n);
        const random = () => {
            seed = (seed * 1103515245n + 12345n) % 2147483648n;
            return Number(seed) / 2147483648;
        };
        for (let i = 0; i < data.length; i += 4) {
            if (random() < 0.015) {
                const noise = random() > 0.5 ? 1 : -1;
                data[i] = Math.max(0, Math.min(255, data[i] + noise));
                data[i+1] = Math.max(0, Math.min(255, data[i+1] + noise));
                data[i+2] = Math.max(0, Math.min(255, data[i+2] + noise));
            }
        }
        return imageData;
    };
}

// 12. WebGL VENDOR/RENDERER override (from __phantom_persona)
const getParameter = globalThis.WebGLRenderingContext?.prototype.getParameter;
if (getParameter) {
    globalThis.WebGLRenderingContext.prototype.getParameter = function(parameter) {
        if (parameter === 37445) return __phantom_persona.webgl_vendor || 'Google Inc. (Apple)';
        if (parameter === 37446) return __phantom_persona.webgl_renderer || 'ANGLE (Apple, Apple M1 Pro, OpenGL 4.1)';
        return getParameter.call(this, parameter);
    };
}
const getParameter2 = globalThis.WebGL2RenderingContext?.prototype.getParameter;
if (getParameter2) {
    globalThis.WebGL2RenderingContext.prototype.getParameter = function(parameter) {
        if (parameter === 37445) return __phantom_persona.webgl_vendor || 'Google Inc. (Apple)';
        if (parameter === 37446) return __phantom_persona.webgl_renderer || 'ANGLE (Apple, Apple M1 Pro, OpenGL 4.1)';
        return getParameter2.call(this, parameter);
    };
}

// 13. AudioContext createAnalyser noise injection
const originalCreateAnalyser = globalThis.AudioContext?.prototype.createAnalyser;
if (originalCreateAnalyser) {
    globalThis.AudioContext.prototype.createAnalyser = function() {
        const analyser = originalCreateAnalyser.call(this);
        const originalGetFloatFrequencyData = analyser.getFloatFrequencyData;
        analyser.getFloatFrequencyData = function(array) {
            originalGetFloatFrequencyData.call(this, array);
            let seed = BigInt(__phantom_persona.canvas_noise_seed || 1n) ^ 0xDEADBEEFn;
            const random = () => {
                seed = (seed * 1103515245n + 12345n) % 2147483648n;
                return Number(seed) / 2147483648;
            };
            for (let i = 0; i < array.length; i++) {
                array[i] += (random() * 0.1 - 0.05);
            }
        };
        return analyser;
    };
}

// 14. Font measureText interception (Windows fonts table)
const originalMeasureText = globalThis.CanvasRenderingContext2D?.prototype.measureText;
if (originalMeasureText) {
    globalThis.CanvasRenderingContext2D.prototype.measureText = function(text) {
        const metrics = originalMeasureText.call(this, text);
        // Intercept based on static mapping for windows fonts
        if (this.font && this.font.includes('Segoe UI')) {
            Object.defineProperty(metrics, 'width', { value: metrics.width * 1.01 });
        }
        return metrics;
    };
}

// 15. WebRTC RTCPeerConnection override (prevent IP leak)
if (globalThis.RTCPeerConnection) {
    globalThis.RTCPeerConnection = class RTCPeerConnection {
        constructor() {}
        createOffer() { return Promise.resolve({}); }
        createAnswer() { return Promise.resolve({}); }
        setLocalDescription() { return Promise.resolve(); }
        setRemoteDescription() { return Promise.resolve(); }
        addIceCandidate() { return Promise.resolve(); }
        close() {}
    };
}

// 16. Intl.DateTimeFormat timezone fix
if (globalThis.Intl && Intl.DateTimeFormat) {
    const originalDateTimeFormat = Intl.DateTimeFormat;
    Intl.DateTimeFormat = function(locales, options) {
        const opts = options || {};
        if (!opts.timeZone && __phantom_persona.timezone) {
            opts.timeZone = __phantom_persona.timezone;
        }
        return new originalDateTimeFormat(locales, opts);
    };
    Intl.DateTimeFormat.prototype = originalDateTimeFormat.prototype;
    Intl.DateTimeFormat.supportedLocalesOf =
        originalDateTimeFormat.supportedLocalesOf.bind(originalDateTimeFormat);
}

// 17. Delete window.__playwright, __puppeteer, __webdriver markers
['__playwright', '__puppeteer', '__webdriver'].forEach(prop => {
    delete window[prop];
});

// 18. Event polyfills for Tier 1 (QuickJS)
if (typeof MouseEvent === 'undefined') {
    globalThis.Event = function Event(type, options) {
        this.type = type;
        this.bubbles = !!(options && options.bubbles);
        this.cancelable = !!(options && options.cancelable);
        this.timestamp = Date.now();
    };
    globalThis.UIEvent = function UIEvent(type, options) {
        globalThis.Event.call(this, type, options);
        this.detail = (options && options.detail) || 0;
        this.view = (options && options.view) || globalThis.window;
    };
    globalThis.MouseEvent = function MouseEvent(type, options) {
        globalThis.UIEvent.call(this, type, options);
        this.clientX = (options && options.clientX) || 0;
        this.clientY = (options && options.clientY) || 0;
        this.button = (options && options.button) || 0;
        this.buttons = (options && options.buttons) || 0;
        this.ctrlKey = !!(options && options.ctrlKey);
        this.shiftKey = !!(options && options.shiftKey);
        this.altKey = !!(options && options.altKey);
        this.metaKey = !!(options && options.metaKey);
    };
    globalThis.PointerEvent = function PointerEvent(type, options) {
        globalThis.MouseEvent.call(this, type, options);
        this.pointerId = (options && options.pointerId) || 0;
        this.width = (options && options.width) || 1;
        this.height = (options && options.height) || 1;
        this.pressure = (options && options.pressure) || 0;
        this.pointerType = (options && options.pointerType) || 'mouse';
        this.isPrimary = !!(options && options.isPrimary);
    };
    globalThis.FocusEvent = function FocusEvent(type, options) {
        globalThis.UIEvent.call(this, type, options);
        this.relatedTarget = (options && options.relatedTarget) || null;
    };
}

// 19. HTMLElement inheritance from EventTarget
// This enables addEventListener/dispatchEvent on nodes returned by querySelector
if (globalThis.HTMLElement && globalThis.EventTarget) {
    Object.setPrototypeOf(HTMLElement.prototype, EventTarget.prototype);
}
