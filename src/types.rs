use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug)]
pub struct DepsEntry {
  pub name: String,
  pub version: String,
}

#[derive(Serialize, Deserialize)]
pub struct PackageJson {
  pub dependencies: Option<HashMap<String, String>>,
  #[serde(rename = "peerDependencies")]
  pub peer_dependencies: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize)]
pub struct PackageInfo {
  pub name: String,
  pub version: String,
  pub license: String,
  pub homepage: String,
  pub bugs: PakageBugs,
  pub repository: PackageRepo,
}

#[derive(Serialize, Deserialize)]
pub struct PackageRepo {
  pub url: String,
}

#[derive(Serialize, Deserialize)]
pub struct PakageBugs {
  pub url: String,
}
