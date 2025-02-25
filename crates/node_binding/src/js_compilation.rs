use std::{cell::UnsafeCell, ptr::NonNull, sync::Arc};

use napi::bindgen_prelude::{External, Reference};
use napi_derive::napi;
use unpack::{compilation::{self, Compilation}, plugin::CompilationCell};

#[napi]
pub struct JsCompilation(pub(crate) Compilation);
