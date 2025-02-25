use indexmap::IndexSet;
use miette::Result;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rspack_sources::{BoxSource, ConcatSource, SourceExt};
use rustc_hash::FxHashMap;
use tokio::sync::mpsc::unbounded_channel;

use crate::{
    chunk::{ChunkGraph, ChunkId, ChunkLinker, LinkerState}, compiler::CompilerOptions, errors::Diagnostics, module::{
        CodeGenerationContext, CodeGenerationResult, ModuleGraph, ModuleId, ModuleScanner,
        ScannerState,
    }, plugin::PluginDriver, task::Task
};
use std::{future::IntoFuture, sync::Arc};

#[derive(Debug, Default)]
struct CodeGenerationResults {
    module_id_to_generation_result: FxHashMap<ModuleId, CodeGenerationResult>,
}
pub struct CodeGenerationState {
    chunk_graph: ChunkGraph,
    code_generation_results: CodeGenerationResults,
    pub diagnostics: Diagnostics
}
#[derive(Debug, Clone)]
pub struct ChunkAssetState {
    pub assets: FxHashMap<String, BoxSource>,
}

#[derive(Debug)]
pub struct Compilation {
    #[allow(dead_code)]
    pub options: Arc<CompilerOptions>,
    module_graph: ModuleGraph,
    pub diagnostics: Diagnostics,
    pub plugin_driver: Arc<PluginDriver>
}

impl Compilation {
    pub fn new(options: Arc<CompilerOptions>, plugin_driver: Arc<PluginDriver>) -> Self {
        Self {
            options,
            module_graph: Default::default(),
            diagnostics: Default::default(),
            plugin_driver,
        }
    }

    /// similar with webpack's make phase, which will make module graph
    pub async fn scan(&mut self) -> ScannerState {
        let (sender, receiver) = tokio::sync::oneshot::channel();

        let options = self.options.clone();
        let context = self.options.context.clone();
        let plugin_driver = self.plugin_driver.clone();
        
        std::thread::spawn(move || {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let (send, mut recv) = unbounded_channel::<Result<Task>>();
                    let module_scanner =
                        ModuleScanner::new(options, context, plugin_driver);
                    let mut scanner_state = ScannerState::new(send);
                    module_scanner.add_entries(&mut scanner_state,&mut recv).await;
                    sender.send(scanner_state).unwrap();
                })
        });
        receiver.into_future().await.unwrap()
    }

    /// similar with webpack's seal phase
    /// this will make chunk(consists of connected modules)
    pub fn link(&mut self, scanner_state: ScannerState) -> LinkerState {
        let mut linker_state = LinkerState::new(scanner_state.module_graph, scanner_state.diagnostics);
        let linker = ChunkLinker::new(self.options.clone(), scanner_state.entries);
        linker.build_chunk_graph(&mut linker_state);
        linker_state
    }

    /// code generation
    pub fn code_generation(&self, linker_state: LinkerState) -> CodeGenerationState {
        let mut code_generation_results = CodeGenerationResults::default();
        let results = linker_state
            .module_graph
            .modules
            .indices()
            .collect::<Vec<_>>()
            .into_par_iter()
            .map(|module_id| {
                let module = linker_state.module_graph.module_by_id(module_id);
                let codegen_result = module.code_generation(CodeGenerationContext {
                    module_graph: &linker_state.module_graph,
                });
                (module_id, codegen_result)
            })
            .collect::<Vec<_>>();
        for (id, result) in results {
            // FIXME: fixed codegeneration diagnostics later
            code_generation_results
                .module_id_to_generation_result
                .insert(id, result.expect("codegeneration failed"));
        }
        CodeGenerationState {
            chunk_graph: linker_state.chunk_graph,
            code_generation_results,
            diagnostics: linker_state.diagnostics
        }
    }

    // chunk asset
    pub fn create_chunk_asset(
        &self,
        code_generation_state: &mut CodeGenerationState,
    ) -> ChunkAssetState {
        // let manifest = vec![];
        let mut assets = FxHashMap::default();
        for chunk_id in code_generation_state.chunk_graph.chunks.indices() {
            let chunk_name = code_generation_state
                .chunk_graph
                .chunk_by_id(chunk_id)
                .name
                .to_owned()
                .expect("should have name");
            let chunk_modules = code_generation_state
                .chunk_graph
                .get_chunk_modules(chunk_id);
            let chunk_source =
                self.render_chunk_modules(code_generation_state, chunk_id, chunk_modules);
            assets.insert(chunk_name, chunk_source);
        }
        ChunkAssetState { assets }
    }

    pub fn render_chunk_modules(
        &self,
        state: &mut CodeGenerationState,
        chunk_id: ChunkId,
        chunk_modules: IndexSet<ModuleId>,
    ) -> BoxSource {
        let module_sources = chunk_modules
            .iter()
            .map(|module_id| self.render_module(state, chunk_id, *module_id));
        let concat_source = ConcatSource::new(module_sources);
        concat_source.boxed()
    }

    pub fn render_module(
        &self,
        state: &mut CodeGenerationState,
        _chunk_id: ChunkId,
        module_id: ModuleId,
    ) -> BoxSource {
        state.code_generation_results.module_id_to_generation_result[&module_id]
            .source
            .clone()
    }
}
