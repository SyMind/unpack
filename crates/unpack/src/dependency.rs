mod dependency_id;
mod entry_dependency;
mod harmony_import_side_effect_dependency;
mod module_dependency;
mod dependency_block;
mod dependency_template;
use std::fmt::Debug;

use camino::Utf8Path;
pub use dependency_id::*;
use dyn_clone::{clone_trait_object, DynClone};
pub use entry_dependency::*;
pub use harmony_import_side_effect_dependency::*;
pub use module_dependency::*;
pub use dependency_block::*;
pub use dependency_template::*;

pub trait Dependency: AsModuleDependency + AsDependencyTemplate + Debug + DynClone + Send + Sync {
    fn get_context(&self) -> Option<&Utf8Path> {
        None
    }
    
}

clone_trait_object!(Dependency);

pub type BoxDependency = Box<dyn Dependency>;
