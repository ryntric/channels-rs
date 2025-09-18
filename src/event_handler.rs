use std::error::Error;

pub(crate) trait EvenHandler<T: Default> {
    fn on_event(&self, event: &T) -> Result<(), Box<dyn Error>>;

    fn on_error(&self, error: Box<dyn Error>);
}
