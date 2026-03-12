pub trait Interface: Send + Sync {}
impl<T: Send + Sync + ?Sized> Interface for T {}
