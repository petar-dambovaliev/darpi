mod app;
mod expand;
mod route;

pub use route::Query;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
