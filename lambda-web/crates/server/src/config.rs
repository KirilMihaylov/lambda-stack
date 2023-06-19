use std::{
    num::NonZeroU16,
    ops::{Deref, DerefMut},
    path::{Path as StdPath, PathBuf},
};

use serde::{
    de::{Deserializer, Error},
    Deserialize,
};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database: Database,
    pub global: Global,
    #[serde(rename = "bind")]
    pub binds: Vec<Bind>,
    #[serde(rename = "module")]
    pub modules: Vec<Module>,
    #[serde(rename = "route")]
    pub routes: Vec<Route>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Database {
    pub host: String,
    pub port: NonZeroU16,
    pub database: String,
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct Global {
    pub requests: GlobalRequests,
    pub instances: GlobalInstances,
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct GlobalRequests {
    pub max_concurrent: NonZeroU16,
}

#[derive(Debug, Copy, Clone)]
pub struct GlobalInstances {
    pub init_pool_size: u16,
    pub max_idle_pool_size: u16,
}

impl<'de> Deserialize<'de> for GlobalInstances {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename = "GlobalInstancesConfig")]
        pub struct Unchecked {
            pub min_pool_size: u16,
            pub max_idle_pool_size: u16,
        }

        let Unchecked {
            min_pool_size,
            max_idle_pool_size,
        }: Unchecked = Unchecked::deserialize(deserializer)?;

        if min_pool_size <= max_idle_pool_size {
            Ok(Self {
                init_pool_size: min_pool_size,
                max_idle_pool_size,
            })
        } else {
            Err(Error::custom(
                "Minimum pool size can only be lower or equal to the maximum idle pool size!",
            ))
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Bind {
    pub host: String,
    pub port: NonZeroU16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Module {
    pub id: Id,
    pub path: Path,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Route {
    pub path: RoutePath,
    pub module: Id,
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Default, Hash)]
#[repr(transparent)]
pub struct Id(pub String);

impl AsRef<str> for Id {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl AsMut<str> for Id {
    fn as_mut(&mut self) -> &mut str {
        self.0.as_mut_str()
    }
}

impl AsRef<String> for Id {
    fn as_ref(&self) -> &String {
        &self.0
    }
}

impl AsMut<String> for Id {
    fn as_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

impl Deref for Id {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Id {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'r> Deserialize<'r> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'r>,
    {
        let id = String::deserialize(deserializer)?;

        if id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ['-', '_'].contains(&c))
        {
            Ok(Self(id))
        } else {
            Err(Error::custom(
                "Module ID can only contain ASCII alphanumeric characters, dashes and underscores!",
            ))
        }
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct Path(pub PathBuf);

impl Path {
    pub fn into_inner(self) -> PathBuf {
        self.0
    }
}

impl AsRef<StdPath> for Path {
    fn as_ref(&self) -> &StdPath {
        self.0.as_path()
    }
}

impl AsRef<PathBuf> for Path {
    fn as_ref(&self) -> &PathBuf {
        &self.0
    }
}

impl AsMut<PathBuf> for Path {
    fn as_mut(&mut self) -> &mut PathBuf {
        &mut self.0
    }
}

impl Deref for Path {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Path {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'r> Deserialize<'r> for Path {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'r>,
    {
        let path = PathBuf::deserialize(deserializer)?;

        if path.is_file() {
            Ok(Self(path))
        } else {
            Err(Error::custom(
                "Path doesn't point to a file, or no such exists!",
            ))
        }
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct RoutePath(pub String);

impl AsRef<str> for RoutePath {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl AsMut<str> for RoutePath {
    fn as_mut(&mut self) -> &mut str {
        self.0.as_mut_str()
    }
}

impl AsRef<String> for RoutePath {
    fn as_ref(&self) -> &String {
        &self.0
    }
}

impl AsMut<String> for RoutePath {
    fn as_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

impl Deref for RoutePath {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RoutePath {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'r> Deserialize<'r> for RoutePath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'r>,
    {
        let mut route: String = String::deserialize(deserializer)?;

        if !route
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ['/', '-', '_', '+', '%'].contains(&c))
        {
            Err(Error::custom("Routes can only contain ASCII alphanumeric characters, slashes, dashes, underscores, pluses and percent symbols!"))
        } else if route.contains("//") {
            Err(Error::custom("Routes can't contain two adjacent slashes!"))
        } else {
            route.insert(0, '/');

            Ok(Self(route))
        }
    }
}
