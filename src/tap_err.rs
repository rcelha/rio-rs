pub trait Tap {
    fn tap<F>(self, f: F) -> Self
    where
        F: FnOnce(&Self);
}

impl<T, E> Tap for Result<T, E> {
    fn tap<F>(self, f: F) -> Self
    where
        F: FnOnce(&Self),
    {
        f(&self);
        self
    }
}

pub trait TapErr<E> {
    fn tap_err<F>(self, f: F) -> Self
    where
        F: FnOnce(&E);
}

impl<T, E> TapErr<E> for Result<T, E> {
    fn tap_err<F>(self, f: F) -> Self
    where
        F: FnOnce(&E),
    {
        self.map_err(|e| {
            f(&e);
            e
        })
    }
}

pub trait TapOk<T> {
    fn tap_ok<F>(self, f: F) -> Self
    where
        F: FnOnce(&T);
}

impl<T, E> TapOk<T> for Result<T, E> {
    fn tap_ok<F>(self, f: F) -> Self
    where
        F: FnOnce(&T),
    {
        self.map(|v| {
            f(&v);
            v
        })
    }
}
