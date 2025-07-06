# To do

* migrate this todo list into entomologist

* implement user control over state transitions

* implement `ent comment ${ISSUE} [-m ${MESSAGE}]`
    - each issue dir has a `comments` subdir
    - each comment is identified by a sha1-style uid
    - each comment is a file or directory under the `${ISSUE}/comments`
    - comments are ordered by ctime?

* implement `ent edit ${COMMENT}`

* implement `ent attach ${ISSUE} ${FILE}`
    - each issue has its own independent namespace for attached files
    - issue description & comments can reference attached files via standard md links

* write a manpage
