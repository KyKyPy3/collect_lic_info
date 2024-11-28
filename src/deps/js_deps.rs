use crate::types::{DepsEntry, PackageJson};
use anyhow::{Context, Result};
use regex::Regex;
use std::{
  collections::HashMap,
  fs,
  path::{Path, PathBuf},
};
use walkdir::{DirEntry, WalkDir};

static PACKAGE_JSON_FILE: &str = "package.json";

pub struct JsParser {
  root_path: PathBuf,
  exclude_patterns: Vec<Regex>,
  skip_patterns: Vec<Regex>,
}

impl JsParser {
  pub fn new(directory: &str, exclude: &Option<Vec<String>>, skip: &Option<Vec<String>>) -> Result<Self> {
    let root_path = std::fs::canonicalize(directory).context("Failed to canonicalize directory path")?;

    Ok(Self {
      root_path,
      exclude_patterns: Self::compile_patterns(exclude)?,
      skip_patterns: Self::compile_patterns(skip)?,
    })
  }

  pub async fn parse(&self) -> Result<HashMap<String, DepsEntry>> {
    let mut dependencies = HashMap::new();

    let package_json_files = WalkDir::new(&self.root_path)
      .follow_links(true)
      .into_iter()
      .filter_map(|entry| entry.ok())
      .filter(|entry| self.is_valid_package_json(entry));

    for entry in package_json_files {
      let path = entry.path();
      println!("Processing file: {}", path.display());

      let package_json = self
        .parse_package_json(path)
        .with_context(|| format!("Failed to parse {}", path.display()))?;

      self.process_dependencies(&package_json, &mut dependencies)?;
    }

    Ok(dependencies)
  }

  fn is_valid_package_json(&self, entry: &DirEntry) -> bool {
    // Skip directories and hidden files
    if entry.file_type().is_dir() || entry.file_name().to_str().map_or(false, |s| s.starts_with('.')) {
      return false;
    }

    // Skip excluded paths
    if let Some(path_str) = entry.path().to_str() {
      if self.exclude_patterns.iter().any(|pattern| pattern.is_match(path_str)) {
        return false;
      }
    }

    // Check if it's a package.json file
    entry.file_name().to_str().map_or(false, |s| s == PACKAGE_JSON_FILE)
  }

  fn parse_package_json(&self, path: &Path) -> Result<PackageJson> {
    let file = fs::File::open(path).with_context(|| format!("Failed to open file: {}", path.display()))?;

    serde_json::from_reader(file).with_context(|| format!("Failed to parse JSON from: {}", path.display()))
  }

  fn process_dependencies(
    &self,
    package_json: &PackageJson,
    dependencies: &mut HashMap<String, DepsEntry>,
  ) -> Result<()> {
    let Some(deps) = &package_json.dependencies else {
      return Ok(());
    };

    for (name, version) in deps {
      if self.should_skip_dependency(name) {
        println!("Skipping dependency: {}", name);
        continue;
      }

      let version = match version.strip_prefix("^") {
        Some(version) => version,
        None => version,
      };

      dependencies.insert(
        name.clone(),
        DepsEntry {
          name: name.clone(),
          version: version.to_string(),
        },
      );
    }

    let Some(deps) = &package_json.peer_dependencies else {
      return Ok(());
    };

    for (name, version) in deps {
      if self.should_skip_dependency(name) {
        println!("Skipping dependency: {}", name);
        continue;
      }

      dependencies.insert(
        name.clone(),
        DepsEntry {
          name: name.clone(),
          version: version.clone(),
        },
      );
    }

    Ok(())
  }

  fn should_skip_dependency(&self, name: &str) -> bool {
    self.skip_patterns.iter().any(|pattern| pattern.is_match(name))
  }

  fn compile_patterns(patterns: &Option<Vec<String>>) -> Result<Vec<Regex>> {
    match patterns {
      Some(patterns) => patterns
        .iter()
        .map(|p| Regex::new(p).context("Failed to compile regex pattern"))
        .collect(),
      None => Ok(vec![]),
    }
  }
}
