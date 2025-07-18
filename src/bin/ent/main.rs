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
        /// Filter strings, describes issues to include in the list.
        /// Each filter string is of the form "name=condition".
        /// The supported names and their matching conditions are:
        ///
        /// "state": Comma-separated list of states to list.
        /// Example: "state=new,backlog".  Defaults to
        /// "new,backlog,blocked,inprogress".
        ///
        /// "assignee": Comma-separated list of assignees to include in
        /// the list.  The empty string includes issues with no assignee.
        /// Example: "assignee=seb," lists issues assigned to "seb" and
        /// issues without an assignee.  Defaults to include all issues.
        ///
        /// "tag": Comma-separated list of tags to include, or exclude
        /// if prefixed with "-".  Example: "tag=bug,-docs" shows issues
        /// that are tagged "bug" and not tagged "docs".  Defaults to
        /// including all tags and excluding none.
        ///
        /// "done-time": Time range of issue completion, in the form
        /// "[START]..[END]".  Includes issues that were marked Done
        /// between START and END.  START and END are both in RFC 3339
        /// format, e.g. "YYYY-MM-DDTHH:MM:SS[+-]HH:MM".  If START
        /// is omitted, defaults to the beginning of time.  If END is
        /// omitted, defaults to the end of time.
        filter: Vec<String>,
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

    /// Get or set the `done_time` of the Issue.
    DoneTime {
        issue_id: String,
        done_time: Option<String>,
    },

    /// get or add a dependency to the issue
    Depend {
        issue_id: String,
        dependency_id: Option<String>,
    },
}

fn handle_command(
    args: &Args,
    issues_database_source: &entomologist::database::IssuesDatabaseSource,
) -> anyhow::Result<()> {
    match &args.command {
        Commands::List { filter } => {
            let issues = entomologist::database::read_issues_database(issues_database_source)?;
            let filter = {
                let mut f = entomologist::Filter::new();
                for filter_str in filter {
                    f.parse(filter_str)?;
                }
                f
            };

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

                if filter.include_tags.len() > 0 {
                    if !issue.has_any_tag(&filter.include_tags) {
                        continue;
                    }
                }
                if filter.exclude_tags.len() > 0 {
                    if issue.has_any_tag(&filter.exclude_tags) {
                        continue;
                    }
                }

                if let Some(issue_done_time) = issue.done_time {
                    if let Some(start_done_time) = filter.start_done_time {
                        if start_done_time > issue_done_time {
                            continue;
                        }
                    }
                    if let Some(end_done_time) = filter.end_done_time {
                        if end_done_time < issue_done_time {
                            continue;
                        }
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
                    a.creation_time.cmp(&b.creation_time)
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
            let issues_database = entomologist::database::make_issues_database(
                issues_database_source,
                entomologist::database::IssuesDatabaseAccess::ReadWrite,
            )?;
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
                    println!("ID: {}", issue.id);
                    return Ok(());
                }
            }
        }

        Commands::Edit { uuid } => {
            let issues_database = entomologist::database::make_issues_database(
                issues_database_source,
                entomologist::database::IssuesDatabaseAccess::ReadWrite,
            )?;
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
            let issues = entomologist::database::read_issues_database(issues_database_source)?;
            match issues.get_issue(issue_id) {
                Some(issue) => {
                    println!("issue {}", issue_id);
                    println!("author: {}", issue.author);
                    println!("creation_time: {}", issue.creation_time);
                    if let Some(done_time) = &issue.done_time {
                        println!("done_time: {}", done_time);
                    }
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
                        println!("creation_time: {}", comment.creation_time);
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
                let issues_database = entomologist::database::make_issues_database(
                    issues_database_source,
                    entomologist::database::IssuesDatabaseAccess::ReadWrite,
                )?;
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
                let issues = entomologist::database::read_issues_database(issues_database_source)?;
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
            let issues_database = entomologist::database::make_issues_database(
                issues_database_source,
                entomologist::database::IssuesDatabaseAccess::ReadWrite,
            )?;
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
            if let entomologist::database::IssuesDatabaseSource::Branch(branch) =
                issues_database_source
            {
                let issues_database = entomologist::database::make_issues_database(
                    issues_database_source,
                    entomologist::database::IssuesDatabaseAccess::ReadWrite,
                )?;
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
            let issues = entomologist::database::read_issues_database(issues_database_source)?;
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
                    let issues_database = entomologist::database::make_issues_database(
                        issues_database_source,
                        entomologist::database::IssuesDatabaseAccess::ReadWrite,
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
            let issues = entomologist::database::read_issues_database(issues_database_source)?;
            let Some(issue) = issues.issues.get(issue_id) else {
                return Err(anyhow::anyhow!("issue {} not found", issue_id));
            };
            match tag {
                Some(tag) => {
                    // Add or remove tag.
                    let issues_database = entomologist::database::make_issues_database(
                        issues_database_source,
                        entomologist::database::IssuesDatabaseAccess::ReadWrite,
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

        Commands::DoneTime {
            issue_id,
            done_time,
        } => {
            let issues = entomologist::database::read_issues_database(issues_database_source)?;
            let Some(issue) = issues.issues.get(issue_id) else {
                return Err(anyhow::anyhow!("issue {} not found", issue_id));
            };
            match done_time {
                Some(done_time) => {
                    // Add or remove tag.
                    let issues_database = entomologist::database::make_issues_database(
                        issues_database_source,
                        entomologist::database::IssuesDatabaseAccess::ReadWrite,
                    )?;
                    let mut issues =
                        entomologist::issues::Issues::new_from_dir(&issues_database.dir)?;
                    let Some(issue) = issues.get_mut_issue(issue_id) else {
                        return Err(anyhow::anyhow!("issue {} not found", issue_id));
                    };
                    let done_time = chrono::DateTime::parse_from_rfc3339(done_time)
                        .unwrap()
                        .with_timezone(&chrono::Local);
                    issue.set_done_time(done_time)?;
                }
                None => match &issue.done_time {
                    Some(done_time) => println!("done_time: {}", done_time),
                    None => println!("None"),
                },
            };
        }

        Commands::Depend {
            issue_id,
            dependency_id,
        } => match dependency_id {
            Some(dep_id) => {
                let ent_db = entomologist::database::make_issues_database(
                    issues_database_source,
                    entomologist::database::IssuesDatabaseAccess::ReadWrite,
                )?;
                let mut issues = entomologist::issues::Issues::new_from_dir(&ent_db.dir)?;
                if issues.issues.contains_key(dep_id) {
                    if let Some(issue) = issues.issues.get_mut(issue_id) {
                        issue.add_dependency(dep_id.clone())?;
                    } else {
                        Err(anyhow::anyhow!("issue {} not found", issue_id))?;
                    };
                } else {
                    Err(anyhow::anyhow!("dependency {} not found", dep_id))?;
                };
            }
            None => {
                let ent_db = entomologist::database::read_issues_database(issues_database_source)?;

                let Some(issue) = ent_db.issues.get(issue_id) else {
                    Err(anyhow::anyhow!("issue {} not found", issue_id))?
                };
                println!("DEPENDENCIES:");
                if let Some(list) = &issue.dependencies {
                    for dependency in list {
                        println!("{}", dependency);
                    }
                } else {
                    println!("NONE");
                }
            }
        },
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    #[cfg(feature = "log")]
    simple_logger::SimpleLogger::new().env().init().unwrap();

    let args: Args = Args::parse();
    // println!("{:?}", args);

    let issues_database_source = match (&args.issues_dir, &args.issues_branch) {
        (Some(dir), None) => {
            entomologist::database::IssuesDatabaseSource::Dir(std::path::Path::new(dir))
        }
        (None, Some(branch)) => entomologist::database::IssuesDatabaseSource::Branch(branch),
        (None, None) => entomologist::database::IssuesDatabaseSource::Branch("entomologist-data"),
        (Some(_), Some(_)) => {
            return Err(anyhow::anyhow!(
                "don't specify both `--issues-dir` and `--issues-branch`"
            ));
        }
    };

    if let entomologist::database::IssuesDatabaseSource::Branch(branch) = &issues_database_source {
        if !entomologist::git::git_branch_exists(branch)? {
            entomologist::git::create_orphan_branch(branch)?;
        }
    }

    handle_command(&args, &issues_database_source)?;

    Ok(())
}
