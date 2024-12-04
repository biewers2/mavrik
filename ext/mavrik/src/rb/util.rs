use anyhow::anyhow;
use magnus::error::RubyUnavailableError;
use magnus::{ExceptionClass, IntoValue, Module, RHash, RModule, Ruby, Symbol, TryConvert};
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

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

#[derive(Debug)]
pub struct MRHash(pub RHash);

impl MRHash {
    pub fn new() -> Self {
        Self(RHash::new())
    }
    
    pub fn fetch_sym<T: TryConvert>(&self, key: impl AsRef<str>) -> Result<Option<T>, magnus::Error> {
        self.fetch(Symbol::new(key))
    }
    
    pub fn fetch_sym_or<T: TryConvert>(&self, key: impl AsRef<str>, default: T) -> Result<T, magnus::Error> {
        self.fetch_or(Symbol::new(key), default)
    }
    
    pub fn try_fetch_sym<T: TryConvert>(&self, key: impl AsRef<str>) -> Result<T, magnus::Error> {
        self.try_fetch(Symbol::new(key))
    }
    
    pub fn fetch_str<T: TryConvert>(&self, key: impl AsRef<str>) -> Result<Option<T>, magnus::Error> {
        self.fetch(key.as_ref())
    }
    
    pub fn fetch_str_or<T: TryConvert>(&self, key: impl AsRef<str>, default: T) -> Result<T, magnus::Error> {
        self.fetch_or(key.as_ref(), default)
    }
    
    pub fn try_fetch_str<T: TryConvert>(&self, key: impl AsRef<str>) -> Result<T, magnus::Error> {
        self.try_fetch(key.as_ref())
    }
    
    pub fn fetch<T: TryConvert>(&self, key: impl IntoValue) -> Result<Option<T>, magnus::Error> {
        let value = self.0
            .get(key)
            .map(|v| T::try_convert(v))
            .transpose()?;

        Ok(value)
    }
    
    pub fn fetch_or<T: TryConvert>(&self, key: impl IntoValue, default: T) -> Result<T, magnus::Error> {
        Ok(self.fetch(key)?.unwrap_or(default))
    }


    pub fn try_fetch<T: TryConvert>(&self, key: impl IntoValue) -> Result<T, magnus::Error> {
        let key = key.into_value();
        self.fetch(key)?.ok_or(mavrik_error(anyhow!("{} missing", key)))
    }
    
    pub fn set_sym(&self, key: impl AsRef<str>, value: impl IntoValue) -> Result<(), magnus::Error> {
        self.set(Symbol::new(key), value)
    }
    
    pub fn set_str(&self, key: impl AsRef<str>, value: impl IntoValue) -> Result<(), magnus::Error> {
        self.set(key.as_ref(), value)
    }
    
    pub fn set(&self, key: impl IntoValue, value: impl IntoValue) -> Result<(), magnus::Error> {
        self.0.aset(key, value)
    }
}

impl Deref for MRHash {
    type Target = RHash;
    
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MRHash {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<MRHash> for RHash {
    fn from(hash: MRHash) -> Self {
        hash.0
    }
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
    use crate::rb::util::{class_mavrik_error, in_ruby, mavrik_error, module_mavrik, MRHash};
    use anyhow::anyhow;
    use magnus::error::ErrorType;
    use magnus::value::{Qnil, ReprValue};
    use magnus::{eval, Ruby, Symbol, TryConvert};

    pub fn test_mrhash_fetch_sym(_r: &Ruby) -> Result<(), magnus::Error> {
        let hash = MRHash::new();
        hash.aset(Symbol::new("foo"), 42)?;

        let value: Option<i64> = hash.fetch_sym("foo")?;
        assert_eq!(value, Some(42));

        let value: Option<i64> = hash.fetch_sym("bar")?;
        assert_eq!(value, None);

        Ok(())
    }
    
    pub fn test_mrhash_fetch_sym_or(_r: &Ruby) -> Result<(), magnus::Error> {
        let hash = MRHash::new();
        hash.aset(Symbol::new("foo"), 42)?;

        let value: i64 = hash.fetch_sym_or("foo", 0)?;
        assert_eq!(value, 42);

        let value: i64 = hash.fetch_sym_or("bar", 0)?;
        assert_eq!(value, 0);

        Ok(())
    }
    
    pub fn test_mrhash_try_fetch_sym(_r: &Ruby) -> Result<(), magnus::Error> {
        let hash = MRHash::new();
        hash.aset(Symbol::new("foo"), 42)?;

        let value: i64 = hash.try_fetch_sym("foo")?;
        assert_eq!(value, 42);

        let value: Result<i64, magnus::Error> = hash.try_fetch_sym("bar");
        assert!(value.is_err());

        Ok(())
    }
    
    pub fn test_mrhash_fetch_str(_r: &Ruby) -> Result<(), magnus::Error> {
        let hash = MRHash::new();
        hash.aset("foo", 42)?;

        let value: Option<i64> = hash.fetch_str("foo")?;
        assert_eq!(value, Some(42));

        let value: Option<i64> = hash.fetch_str("bar")?;
        assert_eq!(value, None);

        Ok(())
    }
    
    pub fn test_mrhash_fetch_str_or(_r: &Ruby) -> Result<(), magnus::Error> {
        let hash = MRHash::new();
        hash.aset("foo", 42)?;

        let value: i64 = hash.fetch_str_or("foo", 0)?;
        assert_eq!(value, 42);

        let value: i64 = hash.fetch_str_or("bar", 0)?;
        assert_eq!(value, 0);

        Ok(())
    }
    
    pub fn test_mrhash_try_fetch_str(_r: &Ruby) -> Result<(), magnus::Error> {
        let hash = MRHash::new();
        hash.aset("foo", 42)?;

        let value: i64 = hash.try_fetch_str("foo")?;
        assert_eq!(value, 42);

        let value: Result<i64, magnus::Error> = hash.try_fetch_str("bar");
        assert!(value.is_err());

        Ok(())
    }
    
    pub fn test_mrhash_fetch(_r: &Ruby) -> Result<(), magnus::Error> {
        let hash = MRHash::new();
        hash.aset(Symbol::new("foo"), 42)?;
        hash.aset(eval::<Qnil>("nil")?, true)?;

        let value: Option<i64> = hash.fetch(Symbol::new("foo"))?;
        assert_eq!(value, Some(42));
        
        let value: Option<bool> = hash.fetch(eval::<Qnil>("nil")?)?;
        assert_eq!(value, Some(true));

        let value: Option<i64> = hash.fetch(Symbol::new("bar"))?;
        assert_eq!(value, None);

        Ok(())
    }
    
    pub fn test_mrhash_fetch_or(_r: &Ruby) -> Result<(), magnus::Error> {
        let hash = MRHash::new();
        hash.aset(Symbol::new("foo"), 42)?;
        hash.aset(eval::<Qnil>("nil")?, true)?;

        let value: i64 = hash.fetch_or(Symbol::new("foo"), 0)?;
        assert_eq!(value, 42);
        
        let value: bool = hash.fetch_or(eval::<Qnil>("nil")?, false)?;
        assert_eq!(value, true);

        let value: i64 = hash.fetch_or(Symbol::new("bar"), 0)?;
        assert_eq!(value, 0);

        Ok(())
    }
    
    pub fn test_mrhash_try_fetch(_r: &Ruby) -> Result<(), magnus::Error> {
        let hash = MRHash::new();
        hash.aset(Symbol::new("foo"), 42)?;
        hash.aset(eval::<Qnil>("nil")?, true)?;

        let value: i64 = hash.try_fetch(Symbol::new("foo"))?;
        assert_eq!(value, 42);
        
        let value: bool = hash.try_fetch(eval::<Qnil>("nil")?)?;
        assert_eq!(value, true);

        let value: Result<i64, magnus::Error> = hash.try_fetch(Symbol::new("bar"));
        assert!(value.is_err());

        Ok(())
    }
    
    pub fn test_mrhash_set_sym(_r: &Ruby) -> Result<(), magnus::Error> {
        let hash = MRHash::new();
        hash.set_sym("foo", 42)?;
        hash.set_sym("bar", "baz")?;

        let value: Option<i64> = hash
            .get(Symbol::new("foo"))
            .map(TryConvert::try_convert)
            .transpose()?;
        assert_eq!(value, Some(42));

        let value: Option<String> = hash
            .get(Symbol::new("bar"))
            .map(TryConvert::try_convert)
            .transpose()?;
        assert_eq!(value.as_deref(), Some("baz"));

        Ok(())
    }
    
    pub fn test_mrhash_set_str(_r: &Ruby) -> Result<(), magnus::Error> {
        let hash = MRHash::new();
        hash.set_str("foo", 42)?;
        hash.set_str("bar", "baz")?;

        let value: Option<i64> = hash
            .get("foo")
            .map(TryConvert::try_convert)
            .transpose()?;
        assert_eq!(value, Some(42));

        let value: Option<String> = hash
            .get("bar")
            .map(TryConvert::try_convert)
            .transpose()?;
        assert_eq!(value.as_deref(), Some("baz"));

        Ok(())
    }
    
    pub fn test_mrhash_set(_r: &Ruby) -> Result<(), magnus::Error> {
        let hash = MRHash::new();
        hash.set(Symbol::new("foo"), 42)?;
        hash.set(eval::<Qnil>("nil")?, true)?;

        let value: Option<i64> = hash
            .get(Symbol::new("foo"))
            .map(TryConvert::try_convert)
            .transpose()?;
        assert_eq!(value, Some(42));
        
        let value: Option<bool> = hash
            .get(eval::<Qnil>("nil")?)
            .map(TryConvert::try_convert)
            .transpose()?;
        assert_eq!(value, Some(true));

        Ok(())
    }
    
    pub fn test_mavrik_module_is_defined(_r: &Ruby) -> Result<(), magnus::Error> {
        let module = module_mavrik();
        assert!(!module.is_nil());
        Ok(())
    }
    
    pub fn test_mavrik_error_class_is_defined(_r: &Ruby) -> Result<(), magnus::Error> {
        let error = class_mavrik_error();
        assert!(!error.is_nil());
        Ok(())
    }

    pub fn test_mavrik_error_with_custom_message(_r: &Ruby) -> Result<(), magnus::Error> {
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

    pub fn test_in_ruby_calls_fn_in_gvl(_r: &Ruby) -> Result<(), magnus::Error> {
        let mut called = false;
        let called_ref = &mut called;

        in_ruby(|_| {
            *called_ref = true;
        });

        assert!(called);
        Ok(())
    }

    pub fn test_in_ruby_locks_gvl_then_calls_fn(_r: &Ruby) -> Result<(), magnus::Error> {
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
