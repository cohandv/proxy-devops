use clap::{Command, ArgMatches};

pub trait Plugin {
    fn name(&self) -> &'static str;
    fn subcommand(&self) -> Command;
    fn run(&self, matches: &ArgMatches);
}
