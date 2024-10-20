use magnus::error::RubyUnavailableError;
use magnus::{ExceptionClass, IntoValue, Module, RHash, RModule, Ruby, TryConvert};
use std::fmt::Display;

#[macro_export]
macro_rules! without_gvl {
    ( $blk:block ) => {
        rutie::Thread::call_without_gvl(move || $blk, Some(|| {}))
    };
}

#[macro_export]
macro_rules! with_gvl {
    ( $blk:block ) => {
        rutie::Thread::call_with_gvl(move || $blk )
    };
}

#[macro_export]
macro_rules! fetch {
    ($hash:ident, :$key:expr, $default:expr) => {
        fetch!($hash, magnus::Symbol::new($key), $default)
    };
    ($hash:ident, $key:expr, $default:expr) => {
        crate::rb::util::fetch(&$hash, $key, $default)
    };
}

pub fn fetch<V, T>(hash: &RHash, key: V, default: T) -> Result<T, magnus::Error>
where
    V: IntoValue,
    T: TryConvert
{
    let value = hash
        .get(key)
        .map(|v| T::try_convert(v))
        .transpose()?
        .unwrap_or(default);

    Ok(value)
}


pub fn module_mavrik() -> RModule {
    in_ruby(|ruby|
        ruby
            .class_object()
            .const_get::<_, RModule>("Mavrik")
            .expect("Mavrik module not defined")
    )
}

pub fn mavrik_error<S>(error: S) -> magnus::Error
where
    S: Display
{
    let error_class = module_mavrik()
        .const_get::<_, ExceptionClass>("Error")
        .expect("Error class not defined");
    let message = format!("{error}");

    magnus::Error::new(error_class, message)
}

pub fn in_ruby<T>(mut func: impl FnMut(Ruby) -> T) -> T {
    match Ruby::get() {
        Ok(r) => func(r),
        Err(RubyUnavailableError::GvlUnlocked) => {
            with_gvl!({
                let r = Ruby::get().expect("failed to get Ruby");
                func(r)
            })
        },
        Err(e) => panic!("failed to get Ruby: #{e}")
    }
}
