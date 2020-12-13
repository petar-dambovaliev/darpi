use std::sync::Mutex;

pub struct Sonic<T> {
    inner: Mutex<T>,
}

impl<T> Sonic<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner: Mutex::new(inner),
        }
    }

    // pub fn get_mut(&self) -> Option<&mut T> {}
}

#[test]
fn test_blah() {
    let ptr = &mut 5;
    let at = AtomicPtr::new(ptr);
    std::thread::spawn(move || {
        let asd = at.load(Ordering::Relaxed);
    });
    let asd = at.load(Ordering::Relaxed);
}
