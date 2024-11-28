use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
  pub directory: String,

  #[clap(short, long, value_parser, num_args = 1.., value_delimiter = ' ')]
  pub exclude: Option<Vec<String>>,

  #[clap(short, long, value_parser, num_args = 1.., value_delimiter = ' ')]
  pub skip: Option<Vec<String>>,
}

impl Args {
  pub fn parse_args() -> Result<Self> {
    Ok(Self::parse())
  }
}
