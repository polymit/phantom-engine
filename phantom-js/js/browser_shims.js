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
        var result = {
            architecture: __phantom_persona.ua_architecture || "x86",
            bitness: __phantom_persona.ua_bitness || "64",
            brands: this.brands,
            mobile: this.mobile,
            model: "",
            platform: this.platform,
            platformVersion: __phantom_persona.platform_version || "15.0.0",
            uaFullVersion: __phantom_persona.ua_full_version || "133.0.6943.98"
        };
        if (hints && hints.includes('wow64')) {
            result.wow64 = !!(__phantom_persona.ua_wow64);
        }
        return Promise.resolve(result);
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

// 14. Font measureText interception — D-55
// Blueprint: "150+ Windows fonts, 100+ macOS fonts"
// RISK-25: Also shim document.fonts.check() and document.fonts.load()
// "A Linux server with near-empty font set looks nothing like Windows"
(function() {
    var IS_WIN32  = (__phantom_persona.platform === 'Win32');
    var IS_MACOS  = (__phantom_persona.platform === 'MacIntel');

    var WINDOWS_FONT_WIDTHS = {
        'Arial':56.30, 'Arial Black':63.20, 'Arial Narrow':49.80,
        'Arial Rounded MT Bold':59.40, 'Bahnschrift':55.10, 'Calibri':54.10,
        'Calibri Light':52.30, 'Cambria':56.90, 'Cambria Math':56.90,
        'Candara':57.20, 'Comic Sans MS':59.40, 'Consolas':54.80,
        'Constantia':55.60, 'Corbel':55.80, 'Courier New':54.00,
        'Ebrima':57.10, 'Franklin Gothic Medium':57.90, 'Gabriola':54.20,
        'Gadugi':55.40, 'Georgia':57.80, 'Impact':48.20, 'Ink Free':58.30,
        'Leelawadee UI':55.20, 'Lucida Console':51.60,
        'Lucida Sans Unicode':58.10, 'Malgun Gothic':56.00,
        'Microsoft Sans Serif':56.80, 'Palatino Linotype':57.50,
        'Segoe Print':60.10, 'Segoe Script':61.30, 'Segoe UI':55.70,
        'Segoe UI Black':59.80, 'Segoe UI Light':52.10,
        'Segoe UI Semibold':57.20, 'Segoe UI Semilight':53.40,
        'Sylfaen':56.20, 'Symbol':52.00, 'Tahoma':57.30,
        'Times New Roman':53.20, 'Trebuchet MS':58.10, 'Verdana':61.40,
        'Webdings':40.00, 'Wingdings':40.00, 'Wingdings 2':40.00,
        'Wingdings 3':40.00,
        // Office fonts (common on Windows with Office installed)
        'Agency FB':50.20, 'Baskerville Old Face':55.40,
        'Bell MT':55.80, 'Berlin Sans FB':57.20, 'Book Antiqua':57.20,
        'Bookman Old Style':58.60, 'Bradley Hand ITC':59.00,
        'Britannic Bold':53.00, 'Broadway':55.00, 'Brush Script MT':60.00,
        'Californian FB':56.00, 'Calisto MT':57.00, 'Centaur':54.00,
        'Century':57.80, 'Century Gothic':59.20, 'Century Schoolbook':58.40,
        'Colonna MT':51.00, 'Cooper Black':62.00,
        'Copperplate Gothic Bold':52.00, 'Copperplate Gothic Light':50.00,
        'Curlz MT':59.00, 'Dubai':56.50, 'Dubai Light':53.00,
        'Dubai Medium':57.50, 'Edwardian Script ITC':56.00,
        'Elephant':58.00, 'Engravers MT':52.00, 'Eras Bold ITC':57.00,
        'Eras Demi ITC':56.00, 'Eras Light ITC':54.00,
        'Eras Medium ITC':55.50, 'Felix Titling':51.00,
        'Footlight MT Light':56.00, 'Forte':62.00,
        'Franklin Gothic Book':56.00, 'Franklin Gothic Demi':57.00,
        'Franklin Gothic Heavy':58.00, 'Freestyle Script':62.00,
        'French Script MT':60.00, 'Gill Sans MT':56.00,
        'Gill Sans Ultra Bold':59.00, 'Goudy Old Style':57.00,
        'Haettenschweiler':43.00, 'Harrington':54.00,
        'High Tower Text':55.00, 'Imprint MT Shadow':54.00,
        'Informal Roman':61.00, 'Jokerman':62.00, 'Juice ITC':60.00,
        'Kristen ITC':62.00, 'Kunstler Script':60.00,
        'Lucida Calligraphy':61.00, 'Lucida Handwriting':62.00,
        'Lucida Sans':58.00, 'Magneto':57.00, 'Maiandra GD':57.00,
        'Matura MT Script Capitals':55.00, 'Mistral':62.00,
        'Modern No. 20':54.00, 'Monotype Corsiva':57.00,
        'Niagara Engraved':51.00, 'Niagara Solid':52.00,
        'OCR A Extended':50.00, 'Old English Text MT':53.00,
        'Onyx':48.00, 'Palace Script MT':58.00, 'Papyrus':61.00,
        'Perpetua':55.00, 'Playbill':48.00, 'Poor Richard':56.00,
        'Pristina':62.00, 'Rage Italic':65.00, 'Ravie':59.00,
        'Rockwell':57.00, 'Rockwell Condensed':48.00,
        'Rockwell Extra Bold':59.00, 'Script MT Bold':60.00,
        'Showcard Gothic':56.00, 'Snap ITC':58.00, 'Stencil':52.00,
        'Tempus Sans ITC':60.00, 'Tw Cen MT':56.00,
        'Tw Cen MT Condensed':48.00, 'Vivaldi':63.00,
        'Vladimir Script':65.00, 'Wide Latin':68.00
    };

    var MACOS_FONT_WIDTHS = {
        '-apple-system':55.70, 'BlinkMacSystemFont':55.70,
        'SF Pro':55.50, 'SF Pro Display':55.50, 'SF Pro Text':55.20,
        'SF Mono':54.80, 'Helvetica':56.30, 'Helvetica Neue':55.90,
        'Helvetica Neue Light':53.20, 'Helvetica Neue UltraLight':51.00,
        'Arial':56.30, 'Arial Black':63.20, 'Arial Narrow':49.80,
        'American Typewriter':58.20, 'Andale Mono':52.40,
        'Apple Chancery':59.00, 'Apple Color Emoji':40.00,
        'Avenir':56.00, 'Avenir Black':59.00, 'Avenir Book':54.00,
        'Avenir Next':55.70, 'Avenir Next Condensed':46.00,
        'Baskerville':56.80, 'Big Caslon':56.00, 'Bradley Hand':61.00,
        'Chalkboard':60.00, 'Charter':56.50, 'Cochin':56.00,
        'Comic Sans MS':59.40, 'Copperplate':52.00, 'Courier':54.00,
        'Courier New':54.00, 'Didot':55.00, 'Futura':56.00,
        'Geneva':58.00, 'Georgia':57.80, 'Gill Sans':56.00,
        'Heiti SC':56.00, 'Heiti TC':56.00, 'Herculanum':56.00,
        'Hoefler Text':56.00, 'Impact':48.20, 'Iowan Old Style':56.00,
        'Kefa':55.00, 'Lucida Grande':58.00, 'Luminari':56.00,
        'Marker Felt':61.00, 'Menlo':53.20, 'Monaco':52.80,
        'New York':57.00, 'Optima':56.00, 'Palatino':57.50,
        'Papyrus':61.00, 'PT Mono':52.80, 'PT Sans':55.80,
        'PT Sans Caption':55.50, 'PT Sans Narrow':49.00,
        'PT Serif':57.00, 'Rockwell':57.00, 'Sathu':56.00,
        'Savoye LET':62.00, 'Seravek':55.00, 'SignPainter':60.00,
        'Skia':56.00, 'Snell Roundhand':62.00, 'STHeiti':56.00,
        'Symbol':52.00, 'Tahoma':57.30, 'Thonburi':56.00,
        'Times':53.20, 'Times New Roman':53.20, 'Trebuchet MS':58.10,
        'Verdana':61.40, 'Zapf Dingbats':42.00, 'Zapfino':68.00
    };

    var FONT_TABLE = IS_WIN32 ? WINDOWS_FONT_WIDTHS
                  : IS_MACOS ? MACOS_FONT_WIDTHS
                  : null;

    if (FONT_TABLE && globalThis.CanvasRenderingContext2D) {
        var origMeasure = CanvasRenderingContext2D.prototype.measureText;
        CanvasRenderingContext2D.prototype.measureText = function(text) {
            var m = origMeasure.call(this, text);
            // Robust font-family extraction using new RegExp to avoid ESC issues in eval
            var fontRegex = new RegExp("(?:^|\\s)(?:\\d+px|[\\d.]+px|[\\d.]+rem|[\\d.]+em)(?:/\\S+)?\\s+([^,;]+)");
            var match = this.font && this.font.match(fontRegex);
            var family = match ? match[1].trim() : null;
            
            // Remove potential quotes from the family name for correct lookup
            if (family) {
                family = family.replace(/^['"]|['"]$/g, '');
            }

            if (family && FONT_TABLE[family] !== undefined) {
                var base = FONT_TABLE[family];
                // Deterministic sub-pixel noise +-0.01 max — blueprint Shim 14
                var seed = BigInt(__phantom_persona.canvas_noise_seed || 1n);
                var noise = Number((seed % 100n)) / 100.0 * 0.01;
                var spoofed = base + noise;
                Object.defineProperty(m, 'width', {
                    get: function() { return spoofed; },
                    configurable: false
                });
            }
            return m;
        };
    }

    // RISK-25: document.fonts.check() — returns true for known system fonts
    if (typeof document !== 'undefined' && document.fonts && FONT_TABLE) {
        var _origCheck = document.fonts.check.bind(document.fonts);
        document.fonts.check = function(font, text) {
            var fontRegex = new RegExp("(?:^|\\s)(?:\\d+px|[\\d.]+px|[\\d.]+rem|[\\d.]+em)(?:/\\S+)?\\s+([^,;]+)");
            var match = font && font.match(fontRegex);
            var family = match ? match[1].trim().replace(/^['"]|['"]$/g, '') : null;
            if (family && FONT_TABLE[family] !== undefined) {
                return true;
            }
            return _origCheck(font, text);
        };

        // RISK-25: document.fonts.load() — resolves for known fonts
        if (document.fonts.load) {
            var _origLoad = document.fonts.load.bind(document.fonts);
            document.fonts.load = function(font, text) {
                var fontRegex = new RegExp("(?:^|\\s)(?:\\d+px|[\\d.]+px|[\\d.]+rem|[\\d.]+em)(?:/\\S+)?\\s+([^,;]+)");
                var match = font && font.match(fontRegex);
                var family = match ? match[1].trim().replace(/^['"]|['"]$/g, '') : null;
                if (family && FONT_TABLE[family] !== undefined) {
                    return Promise.resolve([{ family: family, status: 'loaded' }]);
                }
                return _origLoad(font, text);
            };
        }
    }
})();

// 15. WebRTC IP leak prevention — reject offers with DOMException NetworkError.
// Detection systems check the rejection reason. Resolving with {} is an instant flag.
var _OriginalRTCPeerConnection = globalThis.RTCPeerConnection;
if (_OriginalRTCPeerConnection) {
    var _blockedOffer = function() {
        return Promise.reject(new DOMException('Network error', 'NetworkError'));
    };
    window.RTCPeerConnection = function PhantomRTC(config) {
        var pc = new _OriginalRTCPeerConnection(config || {});
        pc.createOffer  = _blockedOffer;
        pc.createAnswer = _blockedOffer;
        return pc;
    };
    // Preserve prototype chain for instanceof checks
    window.RTCPeerConnection.prototype = _OriginalRTCPeerConnection.prototype;
    Object.defineProperty(window.RTCPeerConnection, 'name', { value: 'RTCPeerConnection' });
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

// 17. Delete automation markers and prevent re-injection after deletion.
// Three exact markers: __playwright, __puppeteer_evaluation_script__, __webdriver_script_fn
[
    '__playwright',
    '__puppeteer_evaluation_script__',
    '__webdriver_script_fn',
].forEach(function(key) {
    try { delete window[key]; } catch (_) {}
    try { delete globalThis[key]; } catch (_) {}
    try {
        Object.defineProperty(window, key, {
            get: function() { return undefined; },
            set: function() {},
            configurable: false,
            enumerable: false,
        });
    } catch (_) {}
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
    
    // Stealth toString() overrides
    [
        'Event', 'UIEvent', 'MouseEvent', 'PointerEvent', 'FocusEvent'
    ].forEach(name => {
        const ctor = globalThis[name];
        if (ctor) {
            Object.defineProperty(ctor, 'toString', {
                value: function() { return `function ${name}() { [native code] }`; },
                configurable: true,
                writable: true
            });
        }
    });
}

// 19. HTMLElement inheritance from EventTarget
// This enables addEventListener/dispatchEvent on nodes returned by querySelector
if (globalThis.HTMLElement && globalThis.EventTarget) {
    Object.setPrototypeOf(HTMLElement.prototype, EventTarget.prototype);
}
