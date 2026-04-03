const __phantom_observers = new Map();
let __phantom_observer_id = 0;

class PhantomMutationObserver {
    constructor(callback) {
        this._callback = callback;
        this._id = ++__phantom_observer_id;
        this._targets = new Map();
    }

    observe(target, options = {}) {
        this._targets.set(target, options);
        __phantom_observers.set(this._id, this);
    }

    disconnect() {
        __phantom_observers.delete(this._id);
        this._targets.clear();
    }

    takeRecords() {
        return [];
    }
}

// Called by Rust after every DOM mutation
// execute_pending_job() drains this as a microtask
globalThis.__phantom_dispatch_mutation = function(mutationRecord) {
    __phantom_observers.forEach((observer) => {
        try {
            observer._callback([mutationRecord], observer);
        } catch(e) {}
    });
};

globalThis.MutationObserver = PhantomMutationObserver;
