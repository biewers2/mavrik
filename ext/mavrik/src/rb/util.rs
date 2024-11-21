use magnus::error::RubyUnavailableError;
use magnus::{ExceptionClass, IntoValue, Module, RHash, RModule, Ruby, Symbol, TryConvert};
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use anyhow::anyhow;

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
        Err(e) => panic!("failed to get Ruby: #{e}")
    }
}
