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
        /// The filter string is composed of chunks separated by ":".
        /// Each chunk is of the form "name=condition".  The supported
        /// names and their matching conditions are:
        ///
        /// "state": Comma-separated list of states to list.
        ///
        /// "assignee": Comma-separated list of assignees to list.
        /// Defaults to all assignees if not set.
        ///
        #[arg(default_value_t = String::from("state=New,Backlog,Blocked,InProgress"))]
        filter: String,
    },

    /// Create a new issue.
    New { description: Option<String> },

    /// Edit the description of an Issue or a Comment.
    Edit { uuid: String },

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

    /// Get or set the Assignee field of an Issue.
    Assign {
        issue_id: String,
        new_assignee: Option<String>,
    },

    /// Add or remove a Tag to/from an Issue, or list the Tags on an Issue.
    Tag {
        issue_id: String,
        #[arg(allow_hyphen_values = true)]
        tag: Option<String>,
    },
}

/// The main function looks at the command-line arguments and determines
/// from there where to get the Issues Database to operate on.
///
/// * If the user specified `--issues-dir` we use that.
///
/// * If the user specified `--issues-branch` we make sure the branch
///   exists, then use that.
///
/// * If the user specified neither, we use the default branch
///   `entomologist-data` (after ensuring that it exists).
///
/// * If the user specified both, it's an operator error and we abort.
///
/// The result of that code populates an IssuesDatabaseSource object,
/// that gets used later to access the database.
enum IssuesDatabaseSource<'a> {
    Dir(&'a std::path::Path),
    Branch(&'a str),
}

/// The IssuesDatabase type is a "fat path".  It holds a PathBuf pointing
/// at the issues database directory, and optionally a Worktree object
/// corresponding to that path.
///
/// The worktree field itself is never read: we put its path in `dir`
/// and that's all that the calling code cares about.
///
/// The Worktree object is included here *when* the IssuesDatabaseSource
/// is a branch.  In this case a git worktree is created to hold the
/// checkout of the branch.  When the IssueDatabase object is dropped,
/// the contained/owned Worktree object is dropped, which deletes the
/// worktree directory from the filesystem and prunes the worktree from
/// git's worktree list.
struct IssuesDatabase {
    dir: std::path::PathBuf,

    #[allow(dead_code)]
    worktree: Option<entomologist::git::Worktree>,
}

enum IssuesDatabaseAccess {
    ReadOnly,
    ReadWrite,
}

fn make_issues_database(
    issues_database_source: &IssuesDatabaseSource,
    access_type: IssuesDatabaseAccess,
) -> anyhow::Result<IssuesDatabase> {
    match issues_database_source {
        IssuesDatabaseSource::Dir(dir) => Ok(IssuesDatabase {
            dir: std::path::PathBuf::from(dir),
            worktree: None,
        }),
        IssuesDatabaseSource::Branch(branch) => {
            let worktree = match access_type {
                IssuesDatabaseAccess::ReadOnly => {
                    entomologist::git::Worktree::new_detached(branch)?
                }
                IssuesDatabaseAccess::ReadWrite => entomologist::git::Worktree::new(branch)?,
            };
            Ok(IssuesDatabase {
                dir: std::path::PathBuf::from(worktree.path()),
                worktree: Some(worktree),
            })
        }
    }
}

fn read_issues_database(
    issues_database_source: &IssuesDatabaseSource,
) -> anyhow::Result<entomologist::issues::Issues> {
    let issues_database =
        make_issues_database(issues_database_source, IssuesDatabaseAccess::ReadOnly)?;
    Ok(entomologist::issues::Issues::new_from_dir(
        &issues_database.dir,
    )?)
}

fn handle_command(
    args: &Args,
    issues_database_source: &IssuesDatabaseSource,
) -> anyhow::Result<()> {
    match &args.command {
        Commands::List { filter } => {
            let issues = read_issues_database(issues_database_source)?;
            let filter = entomologist::Filter::new_from_str(filter)?;
            let mut uuids_by_state = std::collections::HashMap::<
                entomologist::issue::State,
                Vec<&entomologist::issue::IssueHandle>,
            >::new();
            for (uuid, issue) in issues.issues.iter() {
                if !filter.include_states.contains(&issue.state) {
                    continue;
                }
                if filter.include_assignees.len() > 0 {
                    let assignee = match &issue.assignee {
                        Some(assignee) => assignee,
                        None => "",
                    };
                    if !filter.include_assignees.contains(assignee) {
                        continue;
                    }
                }

                // This issue passed all the filters, include it in list.
                uuids_by_state
                    .entry(issue.state.clone())
                    .or_default()
                    .push(uuid);
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
                    let comments = match issue.comments.len() {
                        0 => String::from("   "),
                        n => format!("🗨️ {}", n),
                    };
                    let assignee = match &issue.assignee {
                        Some(assignee) => format!(" (👉 {})", assignee),
                        None => String::from(""),
                    };
                    let tags = match &issue.tags.len() {
                        0 => String::from(""),
                        _ => {
                            // Could use `format!(" {:?}", issue.tags)`
                            // here, but that results in `["tag1", "TAG2",
                            // "i-am-also-a-tag"]` and i don't want the
                            // double-quotes around each tag.
                            let mut tags = String::from(" [");
                            let mut separator = "";
                            for tag in &issue.tags {
                                tags.push_str(separator);
                                tags.push_str(tag);
                                separator = ", ";
                            }
                            tags.push_str("]");
                            tags
                        }
                    };
                    println!(
                        "{}  {}  {}{}{}",
                        uuid,
                        comments,
                        issue.title(),
                        assignee,
                        tags
                    );
                }
                println!("");
            }
        }

        Commands::New { description } => {
            let issues_database =
                make_issues_database(issues_database_source, IssuesDatabaseAccess::ReadWrite)?;
            match entomologist::issue::Issue::new(&issues_database.dir, description) {
                Err(entomologist::issue::IssueError::EmptyDescription) => {
                    println!("no new issue created");
                    return Ok(());
                }
                Err(e) => {
                    return Err(e.into());
                }
                Ok(issue) => {
                    println!("created new issue '{}'", issue.title());
                    return Ok(());
                }
            }
        }

        Commands::Edit { uuid } => {
            let issues_database =
                make_issues_database(issues_database_source, IssuesDatabaseAccess::ReadWrite)?;
            let mut issues = entomologist::issues::Issues::new_from_dir(&issues_database.dir)?;
            if let Some(issue) = issues.get_mut_issue(uuid) {
                match issue.edit_description() {
                    Err(entomologist::issue::IssueError::EmptyDescription) => {
                        println!("aborted issue edit");
                        return Ok(());
                    }
                    Err(e) => return Err(e.into()),
                    Ok(()) => return Ok(()),
                }
            }
            // No issue by that ID, check all the comments.
            for (_, issue) in issues.issues.iter_mut() {
                for comment in issue.comments.iter_mut() {
                    if comment.uuid == *uuid {
                        match comment.edit_description() {
                            Err(entomologist::comment::CommentError::EmptyDescription) => {
                                println!("aborted comment edit");
                                return Ok(());
                            }
                            Err(e) => return Err(e.into()),
                            Ok(()) => return Ok(()),
                        }
                    }
                }
            }
            return Err(anyhow::anyhow!(
                "no issue or comment with uuid {} found",
                uuid
            ));
        }

        Commands::Show { issue_id } => {
            let issues = read_issues_database(issues_database_source)?;
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
                        println!("");
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
        } => match new_state {
            Some(new_state) => {
                let issues_database =
                    make_issues_database(issues_database_source, IssuesDatabaseAccess::ReadWrite)?;
                let mut issues = entomologist::issues::Issues::new_from_dir(&issues_database.dir)?;
                match issues.issues.get_mut(issue_id) {
                    Some(issue) => {
                        let current_state = issue.state.clone();
                        issue.set_state(new_state.clone())?;
                        println!("issue: {}", issue_id);
                        println!("state: {} -> {}", current_state, new_state);
                    }
                    None => {
                        return Err(anyhow::anyhow!("issue {} not found", issue_id));
                    }
                }
            }
            None => {
                let issues = read_issues_database(issues_database_source)?;
                match issues.issues.get(issue_id) {
                    Some(issue) => {
                        println!("issue: {}", issue_id);
                        println!("state: {}", issue.state);
                    }
                    None => {
                        return Err(anyhow::anyhow!("issue {} not found", issue_id));
                    }
                }
            }
        },

        Commands::Comment {
            issue_id,
            description,
        } => {
            let issues_database =
                make_issues_database(issues_database_source, IssuesDatabaseAccess::ReadWrite)?;
            let mut issues = entomologist::issues::Issues::new_from_dir(&issues_database.dir)?;
            let Some(issue) = issues.get_mut_issue(issue_id) else {
                return Err(anyhow::anyhow!("issue {} not found", issue_id));
            };
            match issue.add_comment(description) {
                Err(entomologist::issue::IssueError::CommentError(
                    entomologist::comment::CommentError::EmptyDescription,
                )) => {
                    println!("aborted new comment");
                    return Ok(());
                }
                Err(e) => {
                    return Err(e.into());
                }
                Ok(comment) => {
                    println!(
                        "created new comment {} on issue {}",
                        &comment.uuid, &issue_id
                    );
                }
            }
        }

        Commands::Sync { remote } => {
            if let IssuesDatabaseSource::Branch(branch) = issues_database_source {
                let issues_database =
                    make_issues_database(issues_database_source, IssuesDatabaseAccess::ReadWrite)?;
                entomologist::git::sync(&issues_database.dir, remote, branch)?;
                println!("synced {:?} with {:?}", branch, remote);
            } else {
                return Err(anyhow::anyhow!(
                    "`sync` operates on a branch, don't specify `issues_dir`"
                ));
            }
        }

        Commands::Assign {
            issue_id,
            new_assignee,
        } => {
            let issues = read_issues_database(issues_database_source)?;
            let Some(original_issue) = issues.issues.get(issue_id) else {
                return Err(anyhow::anyhow!("issue {} not found", issue_id));
            };
            let old_assignee: String = match &original_issue.assignee {
                Some(assignee) => assignee.clone(),
                None => String::from("None"),
            };
            println!("issue: {}", issue_id);
            match new_assignee {
                Some(new_assignee) => {
                    let issues_database = make_issues_database(
                        issues_database_source,
                        IssuesDatabaseAccess::ReadWrite,
                    )?;
                    let mut issues =
                        entomologist::issues::Issues::new_from_dir(&issues_database.dir)?;
                    let Some(issue) = issues.get_mut_issue(issue_id) else {
                        return Err(anyhow::anyhow!("issue {} not found", issue_id));
                    };
                    println!("assignee: {} -> {}", old_assignee, new_assignee);
                    issue.set_assignee(new_assignee)?;
                }
                None => {
                    println!("assignee: {}", old_assignee);
                }
            }
        }

        Commands::Tag { issue_id, tag } => {
            let issues = read_issues_database(issues_database_source)?;
            let Some(issue) = issues.issues.get(issue_id) else {
                return Err(anyhow::anyhow!("issue {} not found", issue_id));
            };
            match tag {
                Some(tag) => {
                    // Add or remove tag.
                    let issues_database = make_issues_database(
                        issues_database_source,
                        IssuesDatabaseAccess::ReadWrite,
                    )?;
                    let mut issues =
                        entomologist::issues::Issues::new_from_dir(&issues_database.dir)?;
                    let Some(issue) = issues.get_mut_issue(issue_id) else {
                        return Err(anyhow::anyhow!("issue {} not found", issue_id));
                    };
                    if tag.len() == 0 {
                        return Err(anyhow::anyhow!("invalid zero-length tag"));
                    }
                    if tag.chars().nth(0).unwrap() == '-' {
                        let tag = &tag[1..];
                        issue.remove_tag(tag)?;
                    } else {
                        issue.add_tag(tag)?;
                    }
                }
                None => {
                    // Just list the tags.
                    match &issue.tags.len() {
                        0 => println!("no tags"),
                        _ => {
                            // Could use `format!(" {:?}", issue.tags)`
                            // here, but that results in `["tag1", "TAG2",
                            // "i-am-also-a-tag"]` and i don't want the
                            // double-quotes around each tag.
                            for tag in &issue.tags {
                                println!("{}", tag);
                            }
                        }
                    }
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

    let issues_database_source = match (&args.issues_dir, &args.issues_branch) {
        (Some(dir), None) => IssuesDatabaseSource::Dir(std::path::Path::new(dir)),
        (None, Some(branch)) => IssuesDatabaseSource::Branch(branch),
        (None, None) => IssuesDatabaseSource::Branch("entomologist-data"),
        (Some(_), Some(_)) => {
            return Err(anyhow::anyhow!(
                "don't specify both `--issues-dir` and `--issues-branch`"
            ))
        }
    };

    if let IssuesDatabaseSource::Branch(branch) = &issues_database_source {
        if !entomologist::git::git_branch_exists(branch)? {
            entomologist::git::create_orphan_branch(branch)?;
        }
    }

    handle_command(&args, &issues_database_source)?;

    Ok(())
}
