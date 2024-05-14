use std::{borrow::Cow, path::PathBuf, sync::Arc};

use bytes::Bytes;

use crate::{Assets, BuildError, DataSource, EmbeddedEntry, EmbeddedFile, EmbeddedGlob, Modifier, ModifierContext, PathHash, SplitGlob};


/// Helper to build [`Assets`].
#[derive(Debug)]
pub struct Builder<'a> {
    pub(crate) assets: Vec<EntryBuilder<'a>>,
}

#[derive(Debug)]
pub struct EntryBuilder<'a> {
    pub(crate) kind: EntryBuilderKind<'a>,
    #[cfg_attr(not(feature = "hash"), allow(dead_code))]
    pub(crate) path_hash: PathHash<'a>,
    pub(crate) modifier: Modifier,
}

#[derive(Debug)]
pub(crate) enum EntryBuilderKind<'a> {
    Single {
        http_path: Cow<'a, str>,
        source: DataSource,
    },
    Glob {
        http_prefix: Cow<'a, str>,
        #[cfg_attr(prod_mode, allow(dead_code))]
        glob: SplitGlob,
        files: Vec<GlobFile>,
        #[cfg(dev_mode)]
        base_path: &'static str,
    }
}

#[derive(Debug)]
pub(crate) struct GlobFile {
    pub(crate) suffix: &'static str,
    pub(crate) source: DataSource,
}

impl<'a> Builder<'a> {
    pub fn add_file(
        &mut self,
        http_path: impl Into<Cow<'a, str>>,
        fs_path: impl Into<PathBuf>,
    ) -> &mut EntryBuilder<'a> {
        self.assets.push(EntryBuilder {
            kind: EntryBuilderKind::Single {
                http_path: http_path.into(),
                source: DataSource::File(fs_path.into()),
            },
            path_hash: PathHash::None,
            modifier: Modifier::None,
        });
        self.assets.last_mut().unwrap()
    }

    pub fn add_embedded_file(
        &mut self,
        http_path: impl Into<Cow<'a, str>>,
        file: &EmbeddedFile,
    ) -> &mut EntryBuilder<'a> {
        self.assets.push(EntryBuilder {
            kind: EntryBuilderKind::Single {
                http_path: http_path.into(),
                source: file.data_source(),
            },
            path_hash: PathHash::None,
            modifier: Modifier::None,
        });
        self.assets.last_mut().unwrap()
    }

    pub fn add_embedded_glob(
        &mut self,
        http_path: impl Into<Cow<'a, str>>,
        glob: &'a EmbeddedGlob,
    ) -> &mut EntryBuilder<'a> {
        let split_glob = SplitGlob::new(glob.pattern);
        self.assets.push(EntryBuilder {
            kind: EntryBuilderKind::Glob {
                http_prefix: http_path.into(),
                files: glob.files.iter().map(|f| GlobFile {
                    // This should never be `None`
                    suffix: f.path.strip_prefix(&split_glob.prefix)
                        .expect("embedded file path does not start with glob prefix"),
                    source: f.data_source(),
                }).collect(),
                glob: split_glob,
                #[cfg(dev_mode)]
                base_path: glob.base_path,
            },
            path_hash: PathHash::None,
            modifier: Modifier::None,
        });
        self.assets.last_mut().unwrap()
    }

    pub fn add_embedded(
        &mut self,
        http_path: impl Into<Cow<'a, str>>,
        entry: &'a EmbeddedEntry,
    ) -> &mut EntryBuilder<'a> {
        match entry {
            EmbeddedEntry::Single(file) => self.add_embedded_file(http_path, file),
            EmbeddedEntry::Glob(glob) => self.add_embedded_glob(http_path, glob),
        }
    }

    pub async fn build(self) -> Result<Assets, BuildError> {
        crate::imp::AssetsInner::build(self).await.map(Assets)
    }
}

impl<'a> EntryBuilder<'a> {
    #[cfg(feature = "hash")]
    pub fn with_hash(&mut self) -> &mut Self {
        self.path_hash = PathHash::Auto;
        self
    }

    // TODO: make public again once its tested.
    /// Like [`Self::with_hash`], but lets you specify where it insert the hash.
    #[cfg(feature = "hash")]
    #[allow(dead_code)]
    fn with_hash_between(&mut self, prefix: &'a str, suffix: &'a str) -> &mut Self {
        self.path_hash = PathHash::InBetween { prefix, suffix };
        self
    }

    pub fn with_path_fixup<D, T>(&mut self, paths: D) -> &mut Self
    where
        D: IntoIterator<Item = T>,
        T: Into<Cow<'static, str>>,
    {
        self.modifier = Modifier::PathFixup(paths.into_iter().map(Into::into).collect());
        self
    }

    pub fn with_modifier<F, D, T>(&mut self, dependencies: D, modifier: F) -> &mut Self
    where
        F: 'static + Send + Sync + Fn(Bytes, ModifierContext) -> Bytes,
        D: IntoIterator<Item = T>,
        T: Into<Cow<'static, str>>,
    {
        self.modifier = Modifier::Custom {
            f: Arc::new(modifier),
            deps: dependencies.into_iter().map(Into::into).collect(),
        };
        self
    }

    /// Returns all (unhashed) HTTP paths that are mounted by this entry. This
    /// is mainly useful to pass as dependencies to [`Self::with_modifier`] or
    /// [`Self::with_path_fixup`] of another entry.
    pub fn http_paths(&self) -> Vec<Cow<'a, str>> {
        match &self.kind {
            EntryBuilderKind::Single { http_path, .. } => {
                vec![http_path.clone()]
            }
            EntryBuilderKind::Glob { http_prefix, files, .. } => {
                files.iter().map(|f| f.http_path(http_prefix).into()).collect()
            }
        }
    }

    /// Like [`Self::http_paths`] but asserting that there is only one path
    /// added by this entry. If that's not the case, `None` is returned.
    pub fn single_http_path(&self) -> Option<Cow<'a, str>> {
        match &self.kind {
            EntryBuilderKind::Single { http_path, .. } => Some(http_path.clone()),
            EntryBuilderKind::Glob { http_prefix, files, .. } => {
                if files.len() == 1 {
                    Some(files[0].http_path(http_prefix).into())
                } else {
                    None
                }
            },
        }
    }
}

impl GlobFile {
    pub(crate) fn http_path(&self, http_prefix: &str) -> String {
        format!("{http_prefix}{}", self.suffix)
    }
}
