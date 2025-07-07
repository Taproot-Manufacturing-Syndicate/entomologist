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
    List,

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
}

fn handle_command(args: &Args, issues_dir: &std::path::Path) -> anyhow::Result<()> {
    match &args.command {
        Commands::List => {
            let issues =
                entomologist::issues::Issues::new_from_dir(std::path::Path::new(issues_dir))?;
            for (uuid, issue) in issues.issues.iter() {
                println!("{} {} ({:?})", uuid, issue.title(), issue.state);
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
                    println!("state: {:?}", issue.state);
                    if let Some(dependencies) = &issue.dependencies {
                        println!("dependencies: {:?}", dependencies);
                    }
                    println!("");
                    println!("{}", issue.description);
                    for (uuid, comment) in issue.comments.iter() {
                        println!("");
                        println!("comment: {}", uuid);
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
            println!("found issue {}", issue.title());
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
