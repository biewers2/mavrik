use magnus::{Class, ExceptionClass, IntoValue, Module, RClass, RHash, RModule, Ruby, TryConvert};
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


pub fn module_mavrik(ruby: &Ruby) -> RModule {
    ruby
        .class_object()
        .const_get::<_, RModule>("Mavrik")
        .expect("Mavrik module not defined")
}

pub fn class_execute_task_new(ruby: &Ruby) -> Result<magnus::Value, magnus::Error> {
    let execute_task = module_mavrik(ruby)
        .const_get::<_, RClass>("ExecuteTask")?
        .new_instance(())?;

    Ok(execute_task)
}

pub fn mavrik_error<S>(error: S) -> magnus::Error
where
    S: Display
{
    with_gvl!({
        let ruby = Ruby::get().expect("Failed to get Ruby");

        let error_class = module_mavrik(&ruby)
            .const_get::<_, ExceptionClass>("Error")
            .expect("Error class not defined");
        let message = format!("{error}");

        magnus::Error::new(error_class, message)
    })
}
