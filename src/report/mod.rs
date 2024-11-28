mod constants;
mod error;
mod formatter;

use self::{
  constants::{HEADERS, LICENSE_FILES},
  error::ReportError,
  formatter::WorkbookFormatter,
};
use crate::types::{DepsEntry, PackageInfo};
use anyhow::{anyhow, Context, Result};
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;
use xlsxwriter::{Workbook, Worksheet};

lazy_static! {
  static ref REPO_REGEX: Regex = Regex::new(r"^.*:(.*)\.[a-z#\.]*$").expect("Failed to compile repository regex");
  static ref LICENSE_REGEX: Regex =
    Regex::new(r###"<div id="#lic-0">(.*)</div>"###).expect("Failed to compile license regex");
}

pub struct ReportGenerator {
  workbook: Workbook,
  formatter: WorkbookFormatter,
}

impl ReportGenerator {
  pub fn new(filename: &str) -> Result<Self> {
    let workbook = Workbook::new(filename).context("Failed to create workbook")?;

    let formatter = WorkbookFormatter::new();

    Ok(Self { workbook, formatter })
  }

  pub async fn generate_js_report(&self, sheet_name: &str, deps: HashMap<String, DepsEntry>) -> Result<()> {
    let mut worksheet = self.create_worksheet(sheet_name)?;
    self.write_headers(&mut worksheet)?;

    for (row, (_, dep)) in deps.into_iter().enumerate() {
      self
        .process_js_dependency(&mut worksheet, (row + 1) as u32, &dep)
        .await
        .with_context(|| format!("Failed to process JS dependency: {}", dep.name))?;
    }

    Ok(())
  }

  pub async fn generate_go_report(&self, sheet_name: &str, deps: HashMap<String, DepsEntry>) -> Result<()> {
    let mut worksheet = self.create_worksheet(sheet_name)?;
    self.write_headers(&mut worksheet)?;

    for (row, (_, dep)) in deps.into_iter().enumerate() {
      self
        .process_go_dependency(&mut worksheet, (row + 1) as u32, &dep)
        .await
        .with_context(|| format!("Failed to process Go dependency: {}", dep.name))?;
    }

    Ok(())
  }

  pub fn save(self) -> Result<()> {
    self.workbook.close().context("Failed to save workbook")
  }

  fn create_worksheet(&self, name: &str) -> Result<Worksheet> {
    self
      .workbook
      .add_worksheet(Some(name))
      .context("Failed to create worksheet")
  }

  fn write_headers(&self, worksheet: &mut Worksheet) -> Result<()> {
    for (col, header) in HEADERS.iter().enumerate() {
      worksheet
        .write_string(0, col as u16, header, None)
        .context("Failed to write header")?;
    }
    Ok(())
  }

  async fn process_js_dependency(&self, worksheet: &mut Worksheet<'_>, row: u32, dep: &DepsEntry) -> Result<()> {
    let package_info = self.fetch_npm_package_info(dep).await;
    if let Err(err) = package_info {
      println!(
        "Can't parse response for {}@{}. Skip this package. Error: {}",
        dep.name, dep.version, err
      );

      return Ok(());
    }

    let package_info = package_info.unwrap();
    let repo_url = self.validate_repository_url(&package_info).await?;

    self.write_js_dependency_info(worksheet, row, &package_info)?;
    self.find_and_write_license_url(worksheet, row, &repo_url).await?;

    Ok(())
  }

  async fn process_go_dependency(&self, worksheet: &mut Worksheet<'_>, row: u32, dep: &DepsEntry) -> Result<()> {
    self.write_go_dependency_info(worksheet, row, dep)?;
    self.fetch_and_write_go_license(worksheet, row, dep).await?;

    Ok(())
  }

  async fn fetch_npm_package_info(&self, dep: &DepsEntry) -> Result<PackageInfo> {
    let url = format!("https://registry.npmjs.org/{}/{}", dep.name, dep.version);

    println!("Fetch {}", url);

    reqwest::get(&url)
      .await
      .with_context(|| anyhow!(ReportError::PackageFetchError(format!("Can't fetch package {}", url))))?
      .json::<PackageInfo>()
      .await
      .context("Failed to parse NPM package info")
  }

  async fn validate_repository_url(&self, package_info: &PackageInfo) -> Result<String> {
    let captures = REPO_REGEX
      .captures(&package_info.repository.url)
      .ok_or(ReportError::InvalidRepoUrl)?;

    let repo_url = format!("https:{}", &captures[1]);
    let response = reqwest::get(&repo_url)
      .await
      .context("Failed to validate repository URL")?;

    Ok(match response.status() {
      reqwest::StatusCode::OK => response.url().to_string(),
      _ => repo_url,
    })
  }

  fn write_js_dependency_info(&self, worksheet: &mut Worksheet, row: u32, package_info: &PackageInfo) -> Result<()> {
    worksheet.write_string(row, 0, &package_info.name, None)?;
    worksheet.write_string(row, 1, &package_info.version, None)?;
    worksheet.write_string(row, 2, &package_info.homepage, self.formatter.url_format())?;
    worksheet.write_string(row, 3, &package_info.license, None)?;
    Ok(())
  }

  async fn find_and_write_license_url(&self, worksheet: &mut Worksheet<'_>, row: u32, repo_url: &str) -> Result<()> {
    for license_file in LICENSE_FILES {
      let license_url = format!("{}/blob/master/{}", repo_url, license_file);
      let response = reqwest::get(&license_url).await?;

      if response.status() == reqwest::StatusCode::OK {
        worksheet.write_string(row, 4, &license_url, self.formatter.url_format())?;
        break;
      }
    }
    Ok(())
  }

  fn write_go_dependency_info(&self, worksheet: &mut Worksheet<'_>, row: u32, dep: &DepsEntry) -> Result<()> {
    worksheet.write_string(row, 0, &dep.name, None)?;
    worksheet.write_string(row, 1, &dep.version, None)?;
    worksheet.write_string(
      row,
      2,
      &format!("https://pkg.go.dev/{}", dep.name),
      self.formatter.url_format(),
    )?;
    Ok(())
  }

  async fn fetch_and_write_go_license(&self, worksheet: &mut Worksheet<'_>, row: u32, dep: &DepsEntry) -> Result<()> {
    let lic_url = format!("https://pkg.go.dev/{}?tab=licenses", dep.name);

    println!("Fetch license for {}", dep.name);

    let resp = reqwest::get(&lic_url).await?;
    if resp.status() == reqwest::StatusCode::OK {
      let response = resp.text().await?;

      if let Some(lic) = LICENSE_REGEX.captures(&response) {
        worksheet
          .write_string(row, 3, lic.get(1).unwrap().as_str(), None)
          .map_err(|_| {
            anyhow!(ReportError::WorksheetError(
              "Can't write license type information".to_owned(),
            ))
          })?;
        worksheet
          .write_string(row, 4, &lic_url, self.formatter.url_format())
          .map_err(|_| {
            anyhow!(ReportError::WorksheetError(
              "Can't write license link information".to_owned(),
            ))
          })?;
      } else {
        println!("Can't found license for {}", dep.name);
      }
    }

    Ok(())
  }
}
