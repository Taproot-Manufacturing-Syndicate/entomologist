Entomologist is a distributed, collaborative, offline-first issue tracker,
backed by git.


# Quick start

Entomologist provides a single executable called `ent` which performs
all interaction with the issues database.  `ent --help` provides terse
usage info.

No initialization is needed, just start using `ent` inside your git repo:

```
$ git clone git@server:my-repo.git
$ cd my-repo
$ ent list
# no issues shown, unless my-repo contained some already
```

Create an issue:
```
$ ent new
# Starts your $EDITOR.  Type in the issue description, "git-commit
# style" with a title line, optionally followed by an empty line and
# free form text.
```

List issues with `ent list`.  Optionally takes a filter argument that
controls which issues are shown, see `ent list --help` for details.
For example, to show only new and backlog issues assigned to me or
unassigned, run `ent list state=new,backlog:assignee=$(whoami),`.

Show all details of an issue with `ent show`.

Modify the state of an issue using `ent state`.  Supported states are New,
Backlog, InProgress, Done, and WontDo.

Assign an issue to a person using `ent assign`.  The person is just
a free-form text field for now.  Make it a name, or an email address,
or whatever you want.

Add a comment on an issue with `ent comment`.

Edit an issue or a comment with `ent edit`.

Add or remove tags on an issue using `ent tag`.


# Synchronization

Synchronize your local issue database with the server using `ent sync`.
This will:

1. Fetch the remote issue database branch into your local repo.

2. Show the list of local changes not yet on the remote.

3. Show the list of remote changes not yet incorporated into the local
   branch.

4. Merge the branches.

5. Push the result back to the remote.

Step 4 might fail if (for example) both sides edited the same issue in
a way that git can't merge automatically.  In this case, check out the
`entomologist-data` branch, merge by hand and resolve the conflicts,
and run `ent sync` again.


# Git storage

Issues are stored in a normal orphan branch in a git repo, next to but
independent of whatever else is stored in the repo.  The default branch
name is `entomologist-data`.

Anyone who has a clone of the repo has the complete issue database.

Anyone who has write-access to the repo can modify the issue database.
The issue database branch can be modified by pull request, same as any
other branch.
