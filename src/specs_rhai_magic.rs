use std::collections::HashMap;
use rhai::Engine;
use specs::prelude::*;
use specs::AccessorCow;
use specs::shred::{CastFrom, DynamicSystemData, MetaTable};
use specs::shred::cell::{Ref, RefMut};
use crate::DynamicSystem;

pub struct Dependencies {
    reads: Vec<ResourceId>,
    writes: Vec<ResourceId>,
}

impl Accessor for Dependencies {
    fn try_new() -> Option<Self> {
        // there's no default for this
        None
    }

    fn reads(&self) -> Vec<ResourceId> {
        let mut reads = self.reads.clone();
        reads.push(ResourceId::new::<ReflectionTable>());

        reads
    }

    fn writes(&self) -> Vec<ResourceId> {
        self.writes.clone()
    }
}

/// Some trait that all of your dynamic resources should implement.
/// This trait should be able to register / transfer it to the scripting
/// framework.
pub trait Reflection {
    fn call_method(&self, s: &str);
    fn mut_call_method(&mut self, s: &str){
        self.call_method(s)
    }
}

// necessary for `MetaTable`
unsafe impl<T> CastFrom<T> for dyn Reflection
    where
        T: Reflection + 'static,
{
    fn cast(t: &T) -> &Self {
        t
    }

    fn cast_mut(t: &mut T) -> &mut Self {
        t
    }
}

pub type ReflectionTable = MetaTable<dyn Reflection>;

/// Maps resource names to resource ids.
pub struct ResourceTable {
    map: HashMap<String, ResourceId>,
}

impl ResourceTable {
    pub(crate) fn new() -> Self {
        ResourceTable {
            map: HashMap::default(),
        }
    }

    pub(crate) fn register<T: Resource>(&mut self, name: &str) {
        self.map.insert(name.to_owned(), ResourceId::new_with_dynamic_id::<T>(0));
    }

    fn get(&self, name: &str) -> ResourceId {
        self.map.get(name).cloned().unwrap()
    }
}

pub struct ScriptInput<'a> {
    pub(crate) reads: Vec<&'a dyn Reflection>,
    pub(crate) writes: Vec<&'a mut dyn Reflection>,
}

pub struct ScriptSystemData<'a> {
    pub(crate) meta_table: Read<'a, ReflectionTable>,
    pub(crate) reads: Vec<Ref<'a, Box<dyn Resource + 'static>>>,
    pub(crate) writes: Vec<RefMut<'a, Box<dyn Resource + 'static>>>,
}

impl<'a> DynamicSystemData<'a> for ScriptSystemData<'a> {
    type Accessor = Dependencies;

    fn setup(_accessor: &Dependencies, _res: &mut World) {}

    fn fetch(access: &Dependencies, res: &'a World) -> Self {
        let reads = access
            .reads
            .iter()
            .map(|id| {
                res.try_fetch_internal(id.clone())
                    .expect("bug: the requested resource does not exist")
                    .borrow()
            })
            .collect();
        let writes = access
            .writes
            .iter()
            .map(|id| {
                res.try_fetch_internal(id.clone())
                    .expect("bug: the requested resource does not exist")
                    .borrow_mut()
            })
            .collect();

        ScriptSystemData {
            meta_table: SystemData::fetch(res),
            reads,
            writes,
        }
    }
}

pub fn create_script_sys(res: &World) -> DynamicSystem {
    // -- what we get from the script --
    fn script(mut input: ScriptInput) {
        for read in input.reads{
            read.call_method("bar")
        }
        for write in input.writes{
            write.mut_call_method("foo");
        }
    }

    let reads = vec!["Bar"];
    let writes = vec!["Foo"];

    // -- how we create the system --
    let table = res.fetch::<ResourceTable>();

    DynamicSystem {
        dependencies: Dependencies {
            reads: reads.iter().map(|r| table.get(r)).collect(),
            writes: writes.iter().map(|r| table.get(r)).collect(),
        },
        // just pass the function pointer
        script,
    }
}