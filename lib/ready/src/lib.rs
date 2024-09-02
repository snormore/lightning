use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub trait ReadyWaiterState: Default + Clone + Send + Sync {}

#[derive(Clone)]
pub struct ReadyWaiter<T: ReadyWaiterState> {
    notify: Arc<tokio::sync::Notify>,
    state: Arc<Mutex<Option<T>>>,
    is_ready: Arc<AtomicBool>,
}

impl<T: ReadyWaiterState> ReadyWaiter<T> {
    pub fn new() -> Self {
        Self {
            notify: Arc::new(tokio::sync::Notify::new()),
            state: Arc::new(Mutex::new(None)),
            is_ready: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn notify(&self, state: T) {
        *self.state.lock().unwrap() = Some(state);
        self.is_ready.store(true, Ordering::Relaxed);
        self.notify.notify_one();
    }

    pub async fn wait(&self) -> T {
        self.notify.notified().await;
        self.state
            .lock()
            .unwrap()
            .clone()
            .expect("state was not set")
    }

    pub fn is_ready(&self) -> bool {
        self.is_ready.load(Ordering::Relaxed)
    }

    pub fn state(&self) -> Option<T> {
        self.state.lock().unwrap().clone()
    }
}

impl<T: ReadyWaiterState> Default for ReadyWaiter<T> {
    fn default() -> Self {
        Self::new()
    }
}

mod tests {
    use super::*;

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    struct TestReadyState {
        pub ready: bool,
    }

    impl ReadyWaiterState for TestReadyState {}

    #[tokio::test]
    async fn test_ready_waiter() {
        let ready = ReadyWaiter::new();
        assert!(!ready.is_ready());
        assert!(ready.state().is_none());
        let state = TestReadyState { ready: true };
        ready.notify(state.clone());
        assert!(ready.wait().await.ready);
        assert!(ready.is_ready());
        assert_eq!(ready.state(), Some(state));
    }
}
