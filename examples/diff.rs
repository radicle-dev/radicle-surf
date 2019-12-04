extern crate radicle_surf;

use std::env::Args;
use std::time::Instant;

use git2::Oid;
use nonempty::NonEmpty;

use radicle_surf::diff::Diff;
use radicle_surf::file_system::Directory;
use radicle_surf::vcs::git::{GitBrowser, GitRepository};
use radicle_surf::vcs::History;

fn main() {
    let options = get_options_or_exit();
    let repo = init_repository_or_exit(&options.path_to_repo);
    let mut browser = init_browser_or_exit(&repo);

    match options.head_revision {
        HeadRevision::HEAD => {
            reset_browser_to_head_or_exit(&mut browser);
        }
        HeadRevision::Commit(id) => {
            set_browser_history_or_exit(&mut browser, &id);
        }
    }
    let head_directory = get_directory_or_exit(&browser);

    set_browser_history_or_exit(&mut browser, &options.base_revision);
    let base_directory = get_directory_or_exit(&browser);

    let now = Instant::now();
    match Diff::diff(base_directory, head_directory) {
        Ok(diff) => {
            let elapsed_nanos = now.elapsed().as_nanos();
            print_diff_summary(&diff, elapsed_nanos);
        }
        Err(e) => {
            println!("Failed to build diff: {:?}", e);
            std::process::exit(1);
        }
    };
}

fn get_options_or_exit() -> Options {
    match Options::parse(std::env::args()) {
        Ok(options) => return options,
        Err(message) => {
            println!("{}", message);
            std::process::exit(1);
        }
    };
}

fn init_repository_or_exit(path_to_repo: &str) -> GitRepository {
    match GitRepository::new(path_to_repo) {
        Ok(repo) => return repo,
        Err(e) => {
            println!("Failed to create repository: {:?}", e);
            std::process::exit(1);
        }
    };
}

fn init_browser_or_exit(repo: &GitRepository) -> GitBrowser {
    match GitBrowser::new(&repo) {
        Ok(browser) => return browser,
        Err(e) => {
            println!("Failed to create browser: {:?}", e);
            std::process::exit(1);
        }
    };
}

fn reset_browser_to_head_or_exit(browser: &mut GitBrowser) {
    if let Err(e) = browser.head() {
        println!("Failed to set browser to HEAD: {:?}", e);
        std::process::exit(1);
    }
}

fn set_browser_history_or_exit(browser: &mut GitBrowser, commit_id: &str) {
    // TODO: Might consider to not require resetting to HEAD when history is not at HEAD
    reset_browser_to_head_or_exit(browser);
    if let Err(e) = set_browser_history(browser, commit_id) {
        println!("Failed to set browser history: {:?}", e);
        std::process::exit(1);
    }
}

fn set_browser_history(browser: &mut GitBrowser, commit_id: &str) -> Result<(), String> {
    let oid = match Oid::from_str(commit_id) {
        Ok(oid) => oid,
        Err(e) => return Err(format!("{}", e)),
    };
    let commit = match browser
        .get_history()
        .find_in_history(&oid, |artifact| artifact.id())
    {
        Some(commit) => commit,
        None => return Err(format!("Git commit not found: {}", commit_id)),
    };
    browser.set_history(History(NonEmpty::new(commit)));
    Ok(())
}

fn get_directory_or_exit(browser: &GitBrowser) -> Directory {
    match browser.get_directory() {
        Ok(dir) => return dir,
        Err(e) => {
            println!("Failed to get directory: {:?}", e);
            std::process::exit(1)
        }
    };
}

fn print_diff_summary(diff: &Diff, elapsed_nanos: u128) {
    diff.created.iter().for_each(|created| {
        println!("+++ {}", created.0);
    });
    diff.deleted.iter().for_each(|deleted| {
        println!("--- {}", deleted.0);
    });
    diff.modified.iter().for_each(|modified| {
        println!("mod {}", modified.path);
    });

    println!(
        "created {} / deleted {} / modified {} / total {}",
        diff.created.len(),
        diff.deleted.len(),
        diff.modified.len(),
        diff.created.len() + diff.deleted.len() + diff.modified.len()
    );
    println!("diff took {} micros ", elapsed_nanos / 1000);
}

struct Options {
    path_to_repo: String,
    base_revision: String,
    head_revision: HeadRevision,
}

enum HeadRevision {
    HEAD,
    Commit(String),
}

impl Options {
    fn parse(args: Args) -> Result<Self, String> {
        let args: Vec<String> = args.into_iter().collect();
        if args.len() != 4 {
            return Err(format!(
                "Usage: {} <path-to-repo> <base-revision> <head-revision>\n\
                \tpath-to-repo: Path to the directory containing .git subdirectory\n\
                \tbase-revision: Git commit ID of the base revision (one that will be considered less recent)\n\
                \thead-revision: Git commit ID of the head revision (one that will be considered more recent) or 'HEAD' to use current git HEAD\n",
                args[0]));
        }

        let path_to_repo = args[1].clone();
        let base_revision = args[2].clone();
        let head_revision = {
            if args[3].eq_ignore_ascii_case("HEAD") {
                HeadRevision::HEAD
            } else {
                HeadRevision::Commit(args[3].clone())
            }
        };

        Ok(Options {
            path_to_repo,
            base_revision,
            head_revision,
        })
    }
}
