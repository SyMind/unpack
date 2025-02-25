mod options;
use std::mem;
use std::ops::DerefMut;
use std::sync::Arc;

use napi::bindgen_prelude::Reference;
pub use options::CompilerOptions;
pub use options::EntryItem;
use crate::binding_allocator::BindingAllocator;
use crate::compilation::ChunkAssetState;
use crate::compilation::Compilation;
use crate::plugin::BoxPlugin;
use crate::plugin::PluginContext;
use crate::plugin::PluginDriver;

pub struct Compiler {
    // allocator 用于分配需要 Binding 的内存
    allocator: Arc<dyn BindingAllocator>,
    #[allow(dead_code)]
    options: Arc<CompilerOptions>,
    plugins: Vec<BoxPlugin>,
    plugin_driver: Arc<PluginDriver>,
    // compiler 通过 Reference 关联 compilation
    last_compilation: Option<Reference<Compilation>>,
}

impl Compiler {
    pub fn new(allocator: Arc<dyn BindingAllocator>, options: Arc<CompilerOptions>, plugins: Vec<BoxPlugin>) -> Self {
        let plugin_driver = Arc::new(PluginDriver {
            plugins: plugins.clone(),
            plugin_context: Arc::new(PluginContext {
                options: options.clone()
            })
        });

        Self {
            allocator,
            options,
            plugins,
            last_compilation: None,
            plugin_driver: plugin_driver.clone()
        }
    }

    // 我们可以给在 main 线程中执行的方法使用宏标注出来，此方法可以注入 Env？
    pub async fn new_compilation(&mut self) -> &mut Compilation {
        let compilation = Compilation::new(self.options.clone(), self.plugin_driver.clone());
        self.last_compilation = Some(self.allocator.alloc_compilation(compilation));
        self.plugin_driver.run_compilation_hook(self.last_compilation.as_mut().unwrap()).await;
        self.last_compilation.as_mut().unwrap().deref_mut()
	}

    pub async fn build(&mut self) {
        let compilation = self.new_compilation().await;

        let scanner_state = compilation.scan().await;
        let linker_state = compilation.link(scanner_state);
        let mut code_generation_state = compilation.code_generation(linker_state);
        compilation.diagnostics.extend(mem::take(&mut code_generation_state.diagnostics));
        let asset_state = compilation.create_chunk_asset(&mut code_generation_state);
        
        self.emit_assets(asset_state);

        let compilation = self.last_compilation.as_mut().unwrap().deref_mut();
        if !compilation.diagnostics.is_empty() {
            for diag in &self.last_compilation.as_mut().unwrap().deref_mut().diagnostics {
                println!("{:?}", diag);
            }
        }
        println!("Compilation finished");
    }

    pub fn emit_assets(&self, asset_state: ChunkAssetState) {
        for (_name, _source) in asset_state.assets {
            // std::fs::write(name, source.buffer().as_ref());
        }
    }
}
