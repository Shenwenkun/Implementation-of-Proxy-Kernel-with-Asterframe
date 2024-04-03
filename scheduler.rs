use alloc::{sync::Arc, vec::Vec};
use aster_frame::{early_println, sync::SpinLock, task::{Scheduler, Task}};

pub struct EasyScheduler{
    queue: SpinLock<Vec<Arc<Task>>>
}

impl EasyScheduler{
    pub const fn new() -> Self{
        Self { queue: SpinLock::new(Vec::new()) }
    }
}

impl Scheduler for EasyScheduler{
    fn enqueue(&self, task: alloc::sync::Arc<Task>) {
        let mut queue = self.queue.lock();
        early_println!("Enqueue");
        queue.push(task);
    }

    fn dequeue(&self) -> Option<alloc::sync::Arc<Task>> {
        let mut queue = self.queue.lock();
        early_println!("Dequeue");
        queue.pop()
    }

    fn should_preempt(&self, task: &alloc::sync::Arc<Task>) -> bool {
        false
    }
}



