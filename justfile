mod rust '.just/rust.justfile'

mod just '.just/just.justfile'

mod git '.just/git.justfile'

check:
    just just::fmt-check
    just oxfmt::fmt-check
    just rust::check

fix:
    just just::fmt-fix
    just oxfmt::fmt-fix
    just rust::fix

setup:
    direnv allow
    just git::hooks-setup

install:
    just rust::install

test:
    just rust::test

run *ARGS='':
    just rust::run {{ ARGS }}

skill-sync:
    rsync --archive --delete \
        skills/template/ \
        "$HOME/dev/projects/agent-config/skills/template/"

mod tools '.just/tools.justfile'

mod oxfmt '.just/oxfmt.justfile'

[positional-arguments]
release +ARGS:
    nu scripts/release.nu "$@"

new-demo $DEMO_TEMPLATE:
    #!/usr/bin/env nu
    use std/log
    let demos = {rust: rust, typescript: ts}
    let suffix = ($demos | get $env.DEMO_TEMPLATE)
    if ('tmp' | path expand) != ([(pwd) tmp] | path join) {
        log error 'Refusing to use a symlinked tmp directory'; exit 1
    }
    cd tmp
    let repo = [$"template-sync-($suffix)-demo" --owner jruz --yes]
    let flags = [--template $env.DEMO_TEMPLATE --visibility private
        --topics demo --description 'Template sync demo']
    ^template-sync new ...$repo ...$flags --non-interactive

[positional-arguments]
new-delete CONFIRM:
    nu scripts/new-delete.nu "$@"
