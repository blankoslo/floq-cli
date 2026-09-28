pub fn start<R, E>(f: impl AsyncFnOnce() -> Result<R, E>) -> Result<R, E> {
    tokio::runtime::Runtime::new().unwrap().block_on(f())
}
