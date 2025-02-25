use napi::bindgen_prelude::Reference;

use crate::{compilation::Compilation, compiler::Compiler};

pub trait BindingAllocator {
    fn alloc_compiler(&self, val: Compiler) -> Reference<Compiler>;

    fn alloc_compilation(&self, val: Compilation) -> Reference<Compilation>;
}
