use std::{ffi::c_void, thread::{self, ThreadId}};

use napi::{bindgen_prelude::Reference, Env};
use unpack::{compilation::Compilation, compiler::Compiler, binding_allocator::BindingAllocator};

use crate::{js_compilation::JsCompilation, js_compiler::JsCompiler};

pub struct NapiAllocator {
    env: Env,
    thread_id: ThreadId,
}

impl NapiAllocator {
    pub fn new(env: Env) -> Self {
        Self {
            env,
            thread_id: thread::current().id(),
        }
    }
}

impl BindingAllocator for NapiAllocator {
    fn alloc_compiler(&self, val: Compiler) -> Reference<Compiler> {
        if thread::current().id() == self.thread_id {
            let template = JsCompiler(val);
            let mut instance = template.into_instance(self.env).unwrap();
            unsafe { Reference::from_value_ptr(&mut *instance as *mut _ as *mut c_void, self.env.raw()).unwrap() }
        } else {
            unimplemented!()
        }
    }

    fn alloc_compilation(&self, val: Compilation) -> Reference<Compilation> {
        if thread::current().id() == self.thread_id {
            let template = JsCompilation(val);
            let mut instance = template.into_instance(self.env).unwrap();
            unsafe { Reference::from_value_ptr(&mut *instance as *mut _ as *mut c_void, self.env.raw()).unwrap() }
        } else {
            unimplemented!()
        }
    }
}
