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

// 3. navigator.plugins — 5 PDF viewer entries matching real Chrome 133
const createPlugin = (name, description, filename) => {
    const plugin = Object.create(Plugin.prototype);
    Object.defineProperties(plugin, {
        name: { value: name, enumerable: true },
        description: { value: description, enumerable: true },
        filename: { value: filename, enumerable: true },
        length: { value: 1, enumerable: true }
    });
    return plugin;
};
const pluginsList = [
    createPlugin("Chrome PDF Plugin", "Portable Document Format", "internal-pdf-viewer"),
    createPlugin("Chrome PDF Viewer", "Portable Document Format", "mhjfbmdgcfjbbpaeojofohoefgiehjai"),
    createPlugin("Native Client", "", "internal-nacl-plugin"),
    createPlugin("PDF Viewer", "Portable Document Format", "mhjfbmdgcfjbbpaeojofohoefgiehjai"),
    createPlugin("Microsoft Edge PDF Viewer", "Portable Document Format", "internal-pdf-viewer")
];
Object.defineProperty(navigator, 'plugins', {
    get: () => {
        const plugins = Object.create(PluginArray.prototype);
        pluginsList.forEach((p, i) => plugins[i] = p);
        Object.defineProperty(plugins, 'length', { value: pluginsList.length });
        return plugins;
    }
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
if (window.outerWidth === undefined) {
    Object.defineProperty(window, 'outerWidth', { get: () => window.innerWidth });
    Object.defineProperty(window, 'outerHeight', { get: () => window.innerHeight });
}

// 6. navigator.connection.rtt
if (!navigator.connection) {
    navigator.connection = { rtt: 50, downlink: 10, effectiveType: '4g', saveData: false };
}

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
        { brand: "Not A(Brand", version: "24" },
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
            let seed = BigInt(__phantom_persona.canvas_noise_seed || 1n);
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
}

// 17. Delete window.__playwright, __puppeteer, __webdriver markers
['__playwright', '__puppeteer', '__webdriver'].forEach(prop => {
    delete window[prop];
});
