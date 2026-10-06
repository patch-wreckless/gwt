use color_eyre::Result;
use color_eyre::eyre::{Context, Error, anyhow};
use gix::remote::fetch::Status::{Change, NoPackReceived};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use url::Url;

struct GixRepo {
    repo: gix::Repository,
}

struct CloneArgs<'a> {
    remote_url: Url,
    target_folder: Option<&'a Path>,
}

impl GixRepo {
    pub fn discover_from(path: &Path) -> Result<GixRepo, Error> {
        let gix_repo =
            gix::discover(path).with_context(|| "failed to find a git repo at path: {path}")?;

        Ok(GixRepo { repo: gix_repo })
    }

    pub fn clone_bare(remote_url: Url, target_folder: Option<&Path>) -> Result<GixRepo> {
        let target = match target_folder {
            Some(folder) => folder.to_owned(),
            None => Self::compute_path_from_url(&remote_url)?
        };

        let mut fetch_task = gix::prepare_clone_bare(remote_url.as_str(), &target)?;
        let cancellation = AtomicBool::new(false);
        let (repo, outcome) = fetch_task.fetch_only(gix::progress::Discard, &cancellation)?;
        
        match outcome.status {
            NoPackReceived { .. } => Err(anyhow!("Failed to get packs from a real people repo")),
            Change { .. } => Ok(GixRepo { repo }),
        }
    }

    // I mean obviously I'm just stubbing this with some stuff rn but since it's a real URL we should be able to
    // do reasonable things
    fn compute_path_from_url(url: &Url) -> Result<PathBuf> {
        let segment: &str = url.path_segments()
            .ok_or_else(|| anyhow!("failed to compute a url for the repo from: {url}"))?
            .last()
            .ok_or_else(|| anyhow!("failed to compute a url for the repo from: {url}"))?;
            
        PathBuf::from_str(segment)
            .with_context(|| "Failed to parse segment")
    }
}

impl Repo for GixRepo {
    fn root(&self) -> &Path {
        // Path is the .git dir itself (but in bare repos it's the repo root)
        // workdir is the repo root in non-bare repos but undefined in bare ones
        // Not sure why this isn't a built in concept but here we are
        self.repo.workdir().unwrap_or_else(|| self.repo.path())
    }
}
trait Repo {
    fn root(&self) -> &Path;
}
