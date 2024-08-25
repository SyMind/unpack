use std::fmt::Debug;

use super::Dependency;


pub trait ModuleDependency: Dependency + Debug {}
pub trait AsModuleDependency {
    fn as_module_dependency(&self) -> Option<&dyn ModuleDependency> {
        None
    }
}

pub type BoxModuleDependency = Box<dyn ModuleDependency>;