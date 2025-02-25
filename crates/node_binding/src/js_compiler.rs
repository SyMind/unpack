use crate::js_plugin::JsPluginAdapter;
use camino::Utf8PathBuf;
use napi::bindgen_prelude::Reference;
use napi::threadsafe_function::{ErrorStrategy, ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi::Env;
use napi_derive::napi;
use std::any::{Any, TypeId};
use std::ffi::c_void;
use std::sync::Arc;
use unpack::compiler::EntryItem;
use unpack::resolver::ResolveOptions;
use unpack::{
    compiler::{Compiler, CompilerOptions},
    plugin::BoxPlugin,
};
use napi_ext::*;
use crate::napi_allocator::NapiAllocator;

fn is_send<T: Any + ?Sized>() -> bool {
    TypeId::of::<T>() == TypeId::of::<dyn Send>()
}

fn is_sync<T: Any + ?Sized>() -> bool {
    TypeId::of::<T>() == TypeId::of::<dyn Sync>()
}

#[napi]
pub struct JsCompiler(pub(crate) Compiler);

#[napi]
impl JsCompiler {
    #[napi(constructor)]
    pub fn new(
        env: Env,
        context: String,
        entry: String,
        mut plugins: Vec<JsPluginAdapter>,
    ) -> napi::Result<Self> {
        let options = CompilerOptions {
            context: Utf8PathBuf::from(context),
            entry: vec![EntryItem {
                name: "main".to_string(),
                import: entry,
            }],
            resolve: ResolveOptions {
                extensions: vec![".js", ".ts", ".mjs", ".jsx"]
                    .into_iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>(),
                ..Default::default()
            },
        };
        // unref napi handles to avoid hang problem
        for plugin in plugins.iter_mut() {
            if let Some(resolve) = &mut plugin.on_resolve {
                resolve.unref(&env).unwrap();
            }
            if let Some(load) = &mut plugin.on_load {
                load.unref(&env).unwrap();
            }
            if let Some(this_compilation) = &mut plugin.this_compilation {
                this_compilation.unref(&env).unwrap();
            }
        }

        let plugins = plugins
            .into_iter()
            .map(|x| Arc::new(x) as BoxPlugin)
            .collect();

        let allocator = NapiAllocator::new(env);
        let compiler = Compiler::new(Arc::new(allocator), Arc::new(options), plugins);
        Ok(Self(compiler))
    }

    #[napi]
    pub fn build(&mut self, env: Env, callback: ThreadsafeFunction<(), ErrorStrategy::CalleeHandled>) -> napi::Result<()> {
        let reference: Reference<Compiler> = unsafe { Reference::from_value_ptr(self as *mut _ as *mut c_void, env.raw())? };
        let mut shared_reference = reference.share_with(env, |compiler| {
            Ok(compiler)
        })?;

        env.spawn_local(move |_env| async move {
            shared_reference.build().await;
            Ok(())
        });

        callback.call(Ok(()), ThreadsafeFunctionCallMode::NonBlocking);

        Ok(())
    }
}
