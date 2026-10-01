mod cli;
mod clone;
mod git;

use clap::Parser;
use cli::Cli;
use std::env;

fn main() -> anyhow::Result<()> {
    let args = env::args().collect::<Vec<_>>();

    match Cli::try_parse_from(args.clone()) {
        Ok(cmd) => run(cmd),
        Err(_) => git::exec(args.iter().skip(1)),
    }?;

    Ok(())
}

fn run(cmd: Cli) -> anyhow::Result<()> {
    match cmd.command {
        cli::Command::Clone(cmd) => {
            let git_bin = unsafe {
                // figure out if `which git` points to the shim
                //
                //   See if git on PATH links libxcselect?
                //     ❯ otool -L $(which git)
                //     /usr/bin/git:
                //     /usr/lib/libxcselect.dylib (compatibility version 1.0.0, current version 1.0.0)
                //     /usr/lib/libSystem.B.dylib (compatibility version 1.0.0, current version 1356.0.0)
                //
                //   Look at symbols?
                //     ❯ nm /usr/bin/git | rg '(xcselect|_setup_git_env)'
                //                      U _xcselect_invoke_xcrun
                //                      U _xcselect_invoke_xcrun
                //
                // use `xcrun --find git` to find real git if so
                //
                //     ❯ nm $(xcrun --find git) | rg '(xcselect|_setup_git_env)'
                //     00000001001d92d8 T _setup_git_env
                git::Git::link("/Library/Developer/CommandLineTools/usr/bin/git")
                    .map_err(anyhow::Error::msg)?
            };
            clone::run(&cmd, git_bin)
        }
    }
}
