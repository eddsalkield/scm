use serde::Deserialize;
use std::path::{PathBuf, Path};

#[derive(Deserialize, Debug)]
pub struct Target {
    pub path: PathBuf,
}

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub target: Vec<Target>,

}
