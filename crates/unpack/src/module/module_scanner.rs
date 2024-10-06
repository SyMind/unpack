use crate::dependency::Dependency;
use crate::errors::Diagnostics;
use crate::module::{BuildContext, ModuleId};
use crate::normal_module_factory::{ModuleFactoryCreateData, NormalModuleFactory};
use crate::task::{AddTask, BuildTask, FactorizeTask, ProcessDepsTask};
use crate::{resolver_factory::ResolverFactory, task::Task};
use camino::Utf8PathBuf;
use indexmap::IndexMap;
use std::collections::VecDeque;
use std::mem::take;
use std::sync::Arc;

use crate::{
    compiler::CompilerOptions,
    dependency::{DependencyId, EntryDependency},
};

use super::module_graph::ModuleGraph;
#[derive(Debug)]
pub struct EntryData {
    pub dependencies: Vec<DependencyId>,
    name: Option<String>,
}
pub struct ModuleScanner {
    options: Arc<CompilerOptions>,
    context: Utf8PathBuf,
    resolver_factory: Arc<ResolverFactory>,
    module_factory: Arc<NormalModuleFactory>,
    scanner_state: ScannerState,
}
struct FactorizeParams {}
impl ModuleScanner {
    pub fn new(options: Arc<CompilerOptions>, context: Utf8PathBuf) -> Self {
        let resolver_factory = Arc::new(ResolverFactory::new_with_base_option(
            options.resolve.clone(),
        ));
        let module_factory = Arc::new(NormalModuleFactory {
            options: options.clone(),
            resolver_factory: resolver_factory.clone(),
            context: context.clone(),
        });
        Self {
            options: options.clone(),
            context,
            resolver_factory: resolver_factory.clone(),
            module_factory,
            scanner_state: ScannerState::default(), // make_artifact: Default::default(),
        }
    }
    // add entries
    pub fn add_entries(&self, state: &mut ScannerState) {
        let entry_ids = self
            .options
            .entry
            .iter()
            .map(|entry| {
                let entry_dep =
                    EntryDependency::new(entry.import.clone(), self.options.context.clone());
                let entry_dep_id = state.module_graph.add_dependency(Box::new(entry_dep));
                state.entries.insert(
                    entry.name.clone(),
                    EntryData {
                        name: Some(entry.name.clone()),
                        dependencies: vec![entry_dep_id],
                    },
                );
                return entry_dep_id;
            })
            .collect::<Vec<_>>();

        self.build_loop(state, entry_ids);
    }
    pub fn handle_module_creation(
        &self,
        state: &mut ScannerState,
        dependencies: Vec<DependencyId>,
        origin_module_id: Option<ModuleId>,
        context: Option<Utf8PathBuf>,
    ) {
        dependencies
            .iter()
            .filter(|id| {
                let dep = id.get_dependency(&state.module_graph).clone();
                dep.as_module_dependency().is_some()
            })
            .for_each(|id| {
                state.task_queue.push_back(Task::Factorize(FactorizeTask {
                    module_dependency_id: *id,
                    origin_module_id,
                    origin_module_context: context.clone(),
                }));
            });
    }
    pub fn resolve_module() {}
}

#[derive(Debug, Default)]
pub struct ScannerState {
    pub module_graph: ModuleGraph,
    task_queue: VecDeque<Task>,
    pub diagnostics: Diagnostics,
    pub entries: IndexMap<String, EntryData>,
}
/// main loop task
impl ModuleScanner {
    pub fn build_loop(&self, state: &mut ScannerState, dependencies: Vec<DependencyId>) {
        // kick off entry dependencies to task_queue
        self.handle_module_creation(state, dependencies, None, Some(self.context.clone()));
        while let Some(task) = state.task_queue.pop_front() {
            self.handle_task(task, state);
        }
    }
    fn handle_task(&self, task: Task, state: &mut ScannerState) {
        match task {
            Task::Factorize(factorize_task) => {
                self.handle_factorize(state, factorize_task);
            }
            Task::Add(add_task) => {
                self.handle_add(state, add_task);
            }
            Task::Build(task) => {
                self.handle_build(state, task);
            }
            Task::ProcessDeps(task) => {
                self.handle_process_deps(state, task);
            }
        }
    }
    fn handle_factorize(&self, state: &mut ScannerState, task: FactorizeTask) {
        let original_module = task
            .origin_module_id
            .map(|id| state.module_graph.module_by_id(id));
        let module_dependency_id = task.module_dependency_id;
        let module_dependency = <Box<dyn Dependency> as Clone>::clone(&state.module_graph.dependency_by_id(module_dependency_id)).to_module_dependency().expect("expect module dependency");
        let original_module_context = original_module.and_then(|x| x.get_context());
        let context = if let Some(context) = module_dependency.get_context() {
            context.to_owned()
        } else if let Some(context) = original_module_context {
            context.to_owned()
        } else {
            self.options.context.clone()
        };
        let module_dependency = module_dependency.clone();
        match self.module_factory.create(ModuleFactoryCreateData {
            module_dependency: module_dependency,
            context,
            options: self.options.clone(),
        }) {
            Ok(factory_result) => {
                let module_id = state
                    .module_graph
                    .add_module(Box::new(factory_result.module));
                state.task_queue.push_back(Task::Add(AddTask { 
                    
                    module_id,
                    module_dependency_id: task.module_dependency_id,
                    origin_module_id: task.origin_module_id,
                 }));
            }
            Err(err) => {
                state.diagnostics.push(err);
            }
        }
    }
    fn handle_process_deps(&self, state: &mut ScannerState, task: ProcessDepsTask) {
        let original_module_id = task.original_module_id;
        let original_module = original_module_id.map(|id| state.module_graph.module_by_id(id));
        let original_module_context =
            original_module.and_then(|x| x.get_context().map(|x| x.to_owned()));

        self.handle_module_creation(
            state,
            task.dependencies,
            original_module_id,
            original_module_context,
        );
    }
    fn handle_add(&self, state: &mut ScannerState, task: AddTask) {
        let mgm = state.module_graph.module_graph_module_by_module_id(task.module_id);
        state.module_graph.set_resolved_module(task.origin_module_id, task.module_dependency_id, task.module_id);
        state.task_queue.push_back(Task::Build(BuildTask {
            module_id: task.module_id,
        }));
    }
    fn handle_build(&self, state: &mut ScannerState, task: BuildTask) {
        let module = state.module_graph.module_by_id_mut(task.module_id);
        match module.build(BuildContext {
            options: self.options.clone(),
        }) {
            Ok(result) => {
                let dependency_ids = result
                    .dependencies
                    .into_iter()
                    .map(|dep| state.module_graph.add_dependency(dep))
                    .collect::<Vec<_>>();
                state
                    .task_queue
                    .push_back(Task::ProcessDeps(ProcessDepsTask {
                        dependencies: dependency_ids,
                        original_module_id: Some(task.module_id),
                    }));
            }
            Err(err) => {
                state.diagnostics.push(err);
            }
        };
    }
}
