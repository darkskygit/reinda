use std::fmt;

pub mod template;


/// Simple ID to refer to one asset in a `Setup` or `Assets` struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetId(pub u32);

// See documentation in the main crate.
#[derive(Debug, Clone, Copy)]
pub struct Setup {
    #[doc(hidden)]
    pub assets: &'static [AssetDef],
    #[doc(hidden)]
    pub path_to_id: PathToIdMap,
    #[doc(hidden)]
    pub base_path: &'static str,
}

impl Setup {
    #[doc(hidden)]
    pub fn asset_by_path(&self, path: &str) -> Option<&AssetDef> {
        self.path_to_id(path).map(|id| self.def(id))
    }

    #[doc(hidden)]
    pub fn path_to_id(&self, path: &str) -> Option<AssetId> {
        (self.path_to_id.0)(path)
    }

    #[doc(hidden)]
    pub fn def(&self, id: AssetId) -> &AssetDef {
        &self.assets[id.0 as usize]
    }
}

#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct PathToIdMap(pub fn(&str) -> Option<AssetId>);

impl fmt::Debug for PathToIdMap {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("<function>")
    }
}

#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub struct AssetDef {
    pub path: &'static str,

    pub serve: bool,
    pub dynamic: bool,
    pub hash: bool, // TODO
    pub template: bool,
    pub append: Option<&'static str>,
    pub prepend: Option<&'static str>,

    #[cfg(not(debug_assertions))]
    pub content: &'static [u8],
}
