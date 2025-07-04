# To do

* migrate this todo list into entomologist

* teach it to work with a git branch
    - unpack the branch to a directory with `git worktree ${TMPDIR} ${BRANCH}`
    - operate on the issues in that worktree
    - git commit the result back to ${BRANCH}
    - delete and prune the worktree

* implement `ent new`

* implement user control over state transitions

* implement `ent comment ${ISSUE} [-m ${MESSAGE}]`
    - each issue dir has a `comments` subdir
    - each comment is identified by a sha1-style uid
    - each comment is a file or directory under the `${ISSUE}/comments`
    - comments are ordered by ctime?

* implement `ent edit ${ISSUE} [-t ${TITLE}] [-d ${DESCRIPTION}]`
    - or would it be better to put the title and description together into a new `message`, like git commits?

* implement `ent edit ${COMMENT}`

* implement `ent attach ${ISSUE} ${FILE}`
    - each issue has its own independent namespace for attached files
    - issue description & comments can reference attached files via standard md links

* write a manpage
