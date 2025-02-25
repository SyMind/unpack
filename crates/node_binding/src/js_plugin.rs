use async_trait::async_trait;
use napi::bindgen_prelude::{Reference, WeakReference};
use napi::tokio::sync::mpsc::unbounded_channel;
use napi::{
    bindgen_prelude::{Buffer, Promise},
    threadsafe_function::{ErrorStrategy::Fatal, ThreadsafeFunction},
    Either,
};
use napi_derive::napi;
use unpack::compilation::Compilation;
use std::{fmt::Debug, future::IntoFuture, sync::Arc};
use unpack::errors::miette::Result;
use unpack::plugin::{LoadArgs, Plugin, PluginContext, ResolveArgs};

#[napi(object, object_to_js = false)]
pub struct JsPluginAdapter {
    pub on_resolve: Option<ThreadsafeFunction<String, Fatal>>,
    pub on_load: Option<ThreadsafeFunction<String, Fatal>>,
    pub this_compilation: Option<ThreadsafeFunction<WeakReference<Compilation>, Fatal>>,
}

impl Debug for JsPluginAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsPluginAdapter").finish()
    }
}

#[async_trait]
impl Plugin for JsPluginAdapter {
    fn name(&self) -> &'static str {
        "js_plugin_adapter"
    }

    // 在 @rspack/core 中会存在 &mut Compilation 传入 js function 的问题
    // 此时需要通过 &mut Compilation 寻找到对应的 js object
    // 两种解决方案
    // 1. 通过 Reflector 寻找到对应的 js object
    // 2. 引用均通过 Binding 来传递
    async fn this_compilation(&self, _ctx: Arc<PluginContext>, compilation: &mut Reference<Compilation>) {
        let (send, mut recv) = unbounded_channel();
        let Some(callback) = &self.this_compilation else {
            return ();
        };
        callback.call_with_return_value(
            compilation.downgrade(),
            napi::threadsafe_function::ThreadsafeFunctionCallMode::Blocking,
            move |ret:()| {
                send.send(());
                Ok(())
            },
        );
        recv.recv().await.unwrap();
    }

    async fn load(&self, _ctx: Arc<PluginContext>, args: LoadArgs) -> Result<Option<Vec<u8>>> {
        let (send, mut recv) = unbounded_channel();
        let Some(callback) = &self.on_load else {
            return Ok(None);
        };
        callback.call_with_return_value(
            args.path.to_string(),
            napi::threadsafe_function::ThreadsafeFunctionCallMode::Blocking,
            move |ret: Either<Option<Buffer>, Promise<Option<Buffer>>>| {
                let _ = send.send(ret);
                Ok(())
            },
        );

        let result = recv.recv().await.unwrap();
        let result = match result {
            Either::A(s) => s,
            Either::B(s) => {
                (s.into_future()).await.unwrap()
                // use pollster::block_on;
                // block_on(s.into_future()).unwrap()
            }
        };
        Ok(result.map(|x| x.into()))
    }
    async fn resolve(&self, _ctx: Arc<PluginContext>, args: ResolveArgs) -> Result<Option<String>> {
        let (send, mut recv) = unbounded_channel();
        let Some(callback) = &self.on_resolve else {
            return Ok(None);
        };
        callback.call_with_return_value(
            args.path.to_string(),
            napi::threadsafe_function::ThreadsafeFunctionCallMode::Blocking,
            move |ret: Option<String>| {
                let _ = send.send(ret);
                Ok(())
            },
        );

        let result = recv.recv().await.unwrap();
        Ok(result)
    }
}
