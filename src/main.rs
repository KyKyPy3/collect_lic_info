mod cli;
mod deps;
mod report;
mod types;

use anyhow::Result;
use cli::Args;
use deps::{go_deps::GoParser, js_deps::JsParser};
use report::ReportGenerator;

#[tokio::main]
async fn main() -> Result<()> {
  let args = Args::parse_args()?;
  let report_generator = ReportGenerator::new("deps_report.xlsx")?;

  // Process JavaScript dependencies
  let js_parser = JsParser::new(&args.directory, &args.exclude, &args.skip)?;
  let web_deps = js_parser.parse().await?;
  report_generator.generate_js_report("Web", web_deps).await?;

  // Process Go dependencies
  let go_parser = GoParser::new(&args.directory, &args.exclude)?;
  let go_deps = go_parser.parse().await?;
  report_generator.generate_go_report("Backend", go_deps).await?;

  report_generator.save()?;
  Ok(())
}
