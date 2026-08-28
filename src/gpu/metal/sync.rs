use dispatch2::{DispatchRetained, DispatchSemaphore, DispatchTime};

pub struct Frames {
    semaphore: DispatchRetained<DispatchSemaphore>,
}

pub struct Guards {
    semaphore: DispatchRetained<DispatchSemaphore>,
    released: bool,
}

impl Frames {
    pub fn new(value: isize) -> Self {
        Self {
            semaphore: DispatchSemaphore::new(value),
        }
    }

    pub fn wait(&self) {
        let _ = self.semaphore.wait(DispatchTime::FOREVER);
    }

    pub fn handle(&self) -> DispatchRetained<DispatchSemaphore> {
        self.semaphore.clone()
    }
}

impl Guards {
    pub fn new(frames: &Frames) -> Self {
        frames.wait();

        Self {
            semaphore: frames.handle(),
            released: false,
        }
    }

    pub fn release(&mut self) {
        self.released = true;
    }
}

impl Drop for Guards {
    fn drop(&mut self) {
        if !self.released {
            self.semaphore.signal();
        }
    }
}
