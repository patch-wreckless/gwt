use anyhow::anyhow;
use std::{fs::File, io::Write, path::Path};

use crate::git;

#[derive(clap::Args)]
pub struct Cmd {
    #[arg(required = true)]
    repository: String,

    directory: Option<String>,
}

pub fn run(cmd: &Cmd, git_bin: git::Git) -> anyhow::Result<()> {
    let wt_directory = match cmd.directory {
        Some(ref directory) => Ok(directory.to_string()),
        None => unsafe {
            git_bin
                .url_basename(&cmd.repository, false)
                .map_err(anyhow::Error::msg)
        },
    }?;
    let wt_directory_path = Path::new(&wt_directory);

    let repo_directory_path = wt_directory_path.join(".repo");
    let repo_directory = repo_directory_path
        .to_str()
        .ok_or_else(|| anyhow!("directory contained invalid Unicode"))?;

    let args = vec!["clone", "--bare", &cmd.repository, repo_directory];
    git::exec(args)?;

    let git_file_path = wt_directory_path.join(".git");
    let mut git_file = File::create(git_file_path)?;
    git_file.write_all(b"gitdir: ./.repo")?;

    git::exec(vec![
        "config",
        "remote.origin.fetch",
        "+refs/heads/*:refs/remotes/origin/*",
    ])?;

    let head = git::exec_capture(vec![
        "-C",
        &wt_directory,
        "rev-parse",
        "--abbrev-ref",
        "HEAD",
    ])?;

    git::exec(vec!["-C", &wt_directory, "worktree", "add", &head.trim()])?;

    Ok(())
}
