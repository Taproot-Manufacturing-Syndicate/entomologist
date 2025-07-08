use clap::Parser;

use entomologist::issue::State;
#[cfg(feature = "log")]
use simple_logger;

#[derive(Debug, clap::Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Directory containing issues.
    #[arg(short = 'd', long)]
    issues_dir: Option<String>,

    /// Branch containing issues.
    #[arg(short = 'b', long)]
    issues_branch: Option<String>,

    /// Type of behavior/output.
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// List issues.
    List {
        /// Filter string, describes issues to include in the list.
        #[arg(default_value_t = String::from("state=New,Backlog,Blocked,InProgress"))]
        filter: String,
    },

    /// Create a new issue.
    New { description: Option<String> },

    /// Edit the description of an issue.
    Edit { issue_id: String },

    /// Show the full description of an issue.
    Show { issue_id: String },

    /// Modify the state of an issue
    State {
        issue_id: String,
        new_state: Option<State>,
    },

    /// Create a new comment on an issue.
    Comment {
        issue_id: String,
        description: Option<String>,
    },

    /// Sync entomologist data with remote.  This fetches from the remote,
    /// merges the remote entomologist data branch with the local one,
    /// and pushes the result back to the remote.
    Sync {
        /// Name of the git remote to sync with.
        #[arg(default_value_t = String::from("origin"))]
        remote: String,
    },
}

fn handle_command(args: &Args, issues_dir: &std::path::Path) -> anyhow::Result<()> {
    match &args.command {
        Commands::List { filter } => {
            let issues =
                entomologist::issues::Issues::new_from_dir(std::path::Path::new(issues_dir))?;
            let filter = entomologist::parse_filter(filter)?;
            let mut uuids_by_state = std::collections::HashMap::<
                entomologist::issue::State,
                Vec<&entomologist::issue::IssueHandle>,
            >::new();
            for (uuid, issue) in issues.issues.iter() {
                if filter.include_states.contains(&issue.state) {
                    uuids_by_state
                        .entry(issue.state.clone())
                        .or_default()
                        .push(uuid);
                }
            }

            use entomologist::issue::State;
            for state in [
                State::InProgress,
                State::Blocked,
                State::Backlog,
                State::New,
                State::Done,
                State::WontDo,
            ] {
                let these_uuids = uuids_by_state.entry(state.clone()).or_default();
                if these_uuids.len() == 0 {
                    continue;
                }
                these_uuids.sort_by(|a_id, b_id| {
                    let a = issues.issues.get(*a_id).unwrap();
                    let b = issues.issues.get(*b_id).unwrap();
                    a.timestamp.cmp(&b.timestamp)
                });
                println!("{:?}:", state);
                for uuid in these_uuids {
                    let issue = issues.issues.get(*uuid).unwrap();
                    let num_comments = issue.comments.len();
                    if num_comments == 0 {
                        println!("{}       {}", uuid, issue.title());
                    } else {
                        println!("{}  🗩 {}  {}", uuid, num_comments, issue.title());
                    }
                }
                println!("");
            }
        }

        Commands::New {
            description: Some(description),
        } => {
            let mut issue = entomologist::issue::Issue::new(issues_dir)?;
            issue.set_description(description)?;
            println!("created new issue '{}'", issue.title());
        }

        Commands::New { description: None } => {
            let mut issue = entomologist::issue::Issue::new(issues_dir)?;
            issue.edit_description()?;
            println!("created new issue '{}'", issue.title());
        }

        Commands::Edit { issue_id } => {
            let mut issues =
                entomologist::issues::Issues::new_from_dir(std::path::Path::new(issues_dir))?;
            match issues.get_mut_issue(issue_id) {
                Some(issue) => {
                    issue.edit_description()?;
                }
                None => {
                    return Err(anyhow::anyhow!("issue {} not found", issue_id));
                }
            }
        }

        Commands::Show { issue_id } => {
            let issues =
                entomologist::issues::Issues::new_from_dir(std::path::Path::new(issues_dir))?;
            match issues.get_issue(issue_id) {
                Some(issue) => {
                    println!("issue {}", issue_id);
                    println!("author: {}", issue.author);
                    println!("timestamp: {}", issue.timestamp);
                    println!("state: {:?}", issue.state);
                    if let Some(dependencies) = &issue.dependencies {
                        println!("dependencies: {:?}", dependencies);
                    }
                    if let Some(assignee) = &issue.assignee {
                        println!("assignee: {}", assignee);
                    }
                    println!("");
                    println!("{}", issue.description);
                    for comment in &issue.comments {
                        println!("");
                        println!("comment: {}", comment.uuid);
                        println!("author: {}", comment.author);
                        println!("timestamp: {}", comment.timestamp);
                        println!("{}", comment.description);
                    }
                }
                None => {
                    return Err(anyhow::anyhow!("issue {} not found", issue_id));
                }
            }
        }

        Commands::State {
            issue_id,
            new_state,
        } => {
            let mut issues =
                entomologist::issues::Issues::new_from_dir(std::path::Path::new(issues_dir))?;
            match issues.issues.get_mut(issue_id) {
                Some(issue) => {
                    let current_state = issue.state.clone();
                    match new_state {
                        Some(s) => {
                            issue.set_state(s.clone())?;
                            println!("issue: {}", issue_id);
                            println!("state: {} -> {}", current_state, s);
                        }
                        None => {
                            println!("issue: {}", issue_id);
                            println!("state: {}", current_state);
                        }
                    }
                }
                None => {
                    return Err(anyhow::anyhow!("issue {} not found", issue_id));
                }
            }
        }

        Commands::Comment {
            issue_id,
            description,
        } => {
            let mut issues =
                entomologist::issues::Issues::new_from_dir(std::path::Path::new(issues_dir))?;
            let Some(issue) = issues.get_mut_issue(issue_id) else {
                return Err(anyhow::anyhow!("issue {} not found", issue_id));
            };
            let mut comment = issue.new_comment()?;
            match description {
                Some(description) => {
                    comment.set_description(description)?;
                }
                None => {
                    comment.edit_description()?;
                }
            }
        }

        Commands::Sync { remote } => {
            if args.issues_dir.is_some() {
                return Err(anyhow::anyhow!(
                    "`sync` operates on a branch, don't specify `issues_dir`"
                ));
            }
            // FIXME: Kinda bogus to re-do this thing we just did in
            // `main()`.  Maybe `main()` shouldn't create the worktree,
            // maybe we should do it here in `handle_command()`?
            // That way also each command could decide if it wants a
            // read-only worktree or a read/write one.
            let branch = match &args.issues_branch {
                Some(branch) => branch,
                None => "entomologist-data",
            };
            entomologist::git::sync(issues_dir, remote, branch)?;
            println!("synced {:?} with {:?}", branch, remote);
        }
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    #[cfg(feature = "log")]
    simple_logger::SimpleLogger::new().env().init().unwrap();

    let args: Args = Args::parse();
    // println!("{:?}", args);

    if let (Some(_), Some(_)) = (&args.issues_dir, &args.issues_branch) {
        return Err(anyhow::anyhow!(
            "don't specify both `--issues-dir` and `--issues-branch`"
        ));
    }

    if let Some(dir) = &args.issues_dir {
        let dir = std::path::Path::new(dir);
        handle_command(&args, dir)?;
    } else {
        let branch = match &args.issues_branch {
            Some(branch) => branch,
            None => "entomologist-data",
        };
        if !entomologist::git::git_branch_exists(branch)? {
            entomologist::git::create_orphan_branch(branch)?;
        }
        let worktree = entomologist::git::Worktree::new(branch)?;
        handle_command(&args, worktree.path())?;
    }

    Ok(())
}
