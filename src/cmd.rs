use std::{error::Error, io::Write};

use async_trait::async_trait;
use clap::ArgMatches;

#[async_trait(?Send)]
pub trait Subcommand<T: Write + Send> {
    fn matches(&self, matches: &ArgMatches) -> bool;

    async fn execute(&self, matches: &ArgMatches, out: &mut T) -> Result<(), Box<dyn Error>>;
}
