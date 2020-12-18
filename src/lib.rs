#![forbid(unsafe_code)]

pub use hyper::{Body, Request, Response};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
