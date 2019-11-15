use git2::{BranchType, Branches, Commit, Error, Repository};

pub fn list_branches(repo: Repository) -> Result<Vec<String>, Error> {
    let mut names = Vec::new();
    let branches: Branches = repo.branches(None)?;
    for branch_result in branches {
        let (branch, branch_type) = branch_result?;
        let name = branch.name()?;
        if let Some(n) = name {
            let result = match branch_type {
                BranchType::Local => format!("local: {}", n),
                BranchType::Remote => format!("remote: {}", n),
            };
            names.push(result);
        }
    }
    Ok(names)
}

pub fn get_commit_walk<'repo>(
    repo: &'repo Repository,
    head: Commit<'repo>,
) -> Result<Vec<Commit<'repo>>, Error> {
    let mut commits = vec![head.clone()];
    let mut revwalk = repo.revwalk()?;

    revwalk.push(head.id())?;

    for try_commit in revwalk {
        let commit_id = try_commit?;
        let commit = repo.find_commit(commit_id)?;
        commits.push(commit.clone());
    }

    Ok(commits)
}

pub fn branch_commit<'repo>(
    repo: &'repo Repository,
    branch: &str,
    branch_type: BranchType,
) -> Result<Commit<'repo>, Error> {
    let branch = repo.find_branch(branch, branch_type)?;
    branch.get().peel_to_commit()
}

pub fn get_branch_commits<'repo>(
    repo: &'repo Repository,
    branch: &str,
    branch_type: BranchType,
) -> Result<Vec<Commit<'repo>>, Error> {
    let commit = branch_commit(repo, branch, branch_type)?;
    get_commit_walk(repo, commit)
}

#[cfg(test)]
mod tests {
    use crate::vcs::git::*;
    use git2::{BranchType, Commit, Error, Repository};

    /*
    fn test_branch() -> Result<(), Error> {
        let repo = Repository::open("/home/haptop/Developer/radicle-surf")?;
        let branch = repo.find_branch("fintan/revisit-design-implementation", BranchType::Local)?;
        let commit = branch.get().peel_to_commit()?;
        let parents = commit.parents();
        for c in parents {
            println!("{:#?}", c.message());
        }
        Ok(())
    }
    */

    /*
    fn test_revwalk() -> Result<(), Error> {
        let repo = Repository::open("/home/haptop/Developer/radicle-surf")?;
        let spec = repo.revspec("fintan/revisit-design-implementation")?;
    }
    */

    #[test]
    fn test_print() {
        let repo = Repository::open("/home/haptop/Developer/radicle-surf");
        match repo {
            Ok(repo) => match get_branch_commits(
                &repo,
                "fintan/revisit-design-implementation",
                BranchType::Local,
            ) {
                Ok(commits) => commits.iter().for_each(|c| println!("{:#?}", c.message())),
                Err(err) => println!("{}", err),
            },
            Err(err) => println!("{}", err),
        }
        assert!(false);
        // let implementation = repo
        //            .and_then(|r| r.find_branch("fintan/revisit-design-implementation", BranchType::Local));
        // println!("{:#?}", implementation.map(|b| b.clone().name()));
    }
}
