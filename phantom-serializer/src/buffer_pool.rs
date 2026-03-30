use parking_lot::Mutex;

pub struct BufferPool {
    pool: Mutex<Vec<String>>,
    default_capacity: usize,
}

impl BufferPool {
    pub fn new(default_capacity: usize) -> Self {
        let mut initial = Vec::with_capacity(4);
        for _ in 0..4 {
            initial.push(String::with_capacity(default_capacity));
        }
        Self {
            pool: Mutex::new(initial),
            default_capacity,
        }
    }

    pub fn acquire(&self) -> String {
        let mut p = self.pool.lock();
        if let Some(buf) = p.pop() {
            buf
        } else {
            String::with_capacity(self.default_capacity)
        }
    }

    pub fn release(&self, mut buf: String) {
        buf.clear();
        let mut p = self.pool.lock();
        if p.len() < 8 {
            p.push(buf);
        }
    }
}

pub static BUFFER_POOL: once_cell::sync::Lazy<BufferPool> =
    once_cell::sync::Lazy::new(|| BufferPool::new(80_000));
