use lib/log.nu

def check-clone [repo: string, checkout: path]: nothing -> nothing {
    cd $checkout
    let root = (^git rev-parse --show-toplevel | str trim)
    let origin = (^git remote get-url origin | str trim)
    let expected = $"git@github.com:($repo).git"
    if $root != $checkout or $origin != $expected {
        log error 'Expected the demo SSH clone under tmp/'
        exit 1
    }
    let status = (^git status --porcelain --ignored=matching | lines)
    let artifacts = ['!! target/' '!! node_modules/']
    let changes = ($status | where {|line| $line not-in $artifacts })
    let local = (^git rev-parse HEAD | str trim)
    let remote = (^git ls-remote --exit-code origin HEAD | split row "\t").0
    if ($changes | is-not-empty) or $local != $remote {
        log error 'Checkout is dirty or differs from remote HEAD'
        exit 1
    }
}

def purge-ci [repo: string]: nothing -> nothing {
    let github = (^gh api $"repos/($repo)" --jq .id | str trim)
    let headers = {Authorization: $"Bearer ($env.WOODPECKER_TOKEN)"}
    let server = ($env.WOODPECKER_SERVER | str trim --right --char '/')
    let api = $"($server)/api/repos"
    let ci = (http get --headers $headers $"($api)/lookup/($repo)")
    let forge_id = ($ci.forge_remote_id | into string)
    if $ci.full_name != $repo or $forge_id != $github {
        log error 'Woodpecker repository does not match GitHub'
        exit 1
    }
    http delete --headers $headers $"($api)/($ci.id)?remove=true" | ignore
}

def main [confirm: string]: nothing -> nothing {
    let demos = ['jruz/template-sync-rust-demo' 'jruz/template-sync-ts-demo']
    if $confirm not-in $demos {
        log error 'Confirm deletion with the exact Rust or TypeScript demo repository name'
        exit 1
    }
    let repo = $confirm
    let name = ($repo | split row '/').1
    let checkout = ([(pwd) tmp $name] | path join)
    if ($checkout | path expand) != $checkout {
        log error 'Refusing to use a symlinked demo checkout'
        exit 1
    }
    check-clone $repo $checkout
    purge-ci $repo
    ^gh repo delete $repo --yes
    rm --recursive --permanent $checkout
}
