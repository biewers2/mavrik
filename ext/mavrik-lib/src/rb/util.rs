use magnus::error::RubyUnavailableError;
use magnus::{ExceptionClass, Module, RModule, Ruby};
use std::fmt::Debug;

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
macro_rules! ruby_or_mavrik_error {
    () => {
        magnus::Ruby::get().map_err(crate::rb::util::mavrik_error)
    };
}

pub fn module_mavrik() -> RModule {
    in_ruby(|ruby|
        ruby
            .class_object()
            .const_get::<_, RModule>("Mavrik")
            .expect("Mavrik module not defined")
    )
}

pub fn class_mavrik_error() -> ExceptionClass {
    module_mavrik()
        .const_get::<_, ExceptionClass>("Error")
        .expect("Error class not defined")
}

pub fn mavrik_error<S>(error: S) -> magnus::Error
where
    S: Debug
{
    let message = format!("{error:?}");
    magnus::Error::new(class_mavrik_error(), message)
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
        Err(RubyUnavailableError::NonRubyThread) => {
            panic!("failed to get Ruby: not in a Ruby thread");
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::rb::util::{class_mavrik_error, in_ruby, mavrik_error, module_mavrik};
    use anyhow::anyhow;
    use magnus::error::ErrorType;
    use magnus::value::ReprValue;
    use magnus::Ruby;

    pub fn mavrik_module_is_defined(_r: &Ruby) -> Result<(), magnus::Error> {
        let module = module_mavrik();
        assert!(!module.is_nil());
        Ok(())
    }
    
    pub fn mavrik_error_class_is_defined(_r: &Ruby) -> Result<(), magnus::Error> {
        let error = class_mavrik_error();
        assert!(!error.is_nil());
        Ok(())
    }

    pub fn mavrik_error_uses_custom_message(_r: &Ruby) -> Result<(), magnus::Error> {
        let error = mavrik_error(anyhow!("this is an error"));

        match error.error_type() {
            ErrorType::Error(e, m) => {
                assert_eq!(e.to_string(), class_mavrik_error().to_string());
                assert!(m.contains("this is an error"));
                Ok(())
            },

            e => panic!("Expected ErrorType::Error, got {e:?}")
        }
    }

    pub fn in_ruby_calls_fn_in_gvl(_r: &Ruby) -> Result<(), magnus::Error> {
        let mut called = false;
        let called_ref = &mut called;

        in_ruby(|_| {
            *called_ref = true;
        });

        assert!(called);
        Ok(())
    }

    pub fn in_ruby_locks_gvl_then_calls_fn(_r: &Ruby) -> Result<(), magnus::Error> {
        let mut called = false;
        let called_ref = &mut called;

        without_gvl!({
            in_ruby(|_| {
                *called_ref = true;
            });
        });

        assert!(called);
        Ok(())
    }
}
