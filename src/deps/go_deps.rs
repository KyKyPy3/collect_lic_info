use crate::types::DepsEntry;
use anyhow::anyhow;
use anyhow::{Context as AnyhowContext, Result};
use gomod_rs::{parse_gomod, Context, Directive};
use regex::Regex;
use std::{collections::HashMap, fs, path::PathBuf};
use walkdir::{DirEntry, WalkDir};

static GO_MOD_FILE: &str = "go.mod";

pub struct GoParser {
  root_path: PathBuf,
  exclude_patterns: Vec<Regex>,
}

impl GoParser {
  pub fn new(directory: &str, exclude: &Option<Vec<String>>) -> Result<Self> {
    let root_path =
      std::fs::canonicalize(directory).with_context(|| format!("Failed to canonicalize directory: {}", directory))?;

    Ok(Self {
      root_path,
      exclude_patterns: Self::compile_patterns(exclude)?,
    })
  }

  pub async fn parse(&self) -> Result<HashMap<String, DepsEntry>> {
    let mut dependencies = HashMap::new();

    let go_mod_files = WalkDir::new(&self.root_path)
      .follow_links(true)
      .into_iter()
      .filter_map(Result::ok)
      .filter(|entry| self.is_valid_go_mod(entry));

    for entry in go_mod_files {
      let path = entry.path();
      println!("Processing file: {}", path.display());

      let go_mod_content =
        fs::read_to_string(path).with_context(|| format!("Failed to read go.mod file: {}", path.display()))?;
      let parsed_mod = parse_gomod(&go_mod_content).context("Failed to parse go.mod file")?;
      self.extract_dependencies(parsed_mod, &mut dependencies);
    }

    Ok(dependencies)
  }

  fn is_valid_go_mod(&self, entry: &DirEntry) -> bool {
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

    // Check if it's a go.mod file
    entry.file_name().to_str().map_or(false, |s| s == GO_MOD_FILE)
  }

  fn extract_dependencies(&self, go_mod: Vec<Context<Directive>>, dependencies: &mut HashMap<String, DepsEntry>) {
    for context in go_mod {
      if let Context {
        value: Directive::Require { specs },
        ..
      } = context
      {
        for spec in specs {
          let version: &str = &spec.value.1;
          let name = spec.value.0;

          dependencies.insert(
            name.to_string(),
            DepsEntry {
              name: name.to_string(),
              version: version.to_string(),
            },
          );
        }
      }
    }
  }

  fn compile_patterns(patterns: &Option<Vec<String>>) -> Result<Vec<Regex>> {
    match patterns {
      Some(patterns) => patterns
        .iter()
        .map(|p| Regex::new(p).map_err(|_| anyhow!("Failed to compile regex pattern")))
        .collect(),
      None => Ok(vec![]),
    }
  }
}
