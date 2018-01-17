# shellcheck disable=2148
# shipcat(1) completion

_shipcat()
{
    # shellcheck disable=2034
    local cur prev words cword
    _init_completion || return

    local -r subcommands="init help validate generate status ship
                          list-environments"

    local has_sub
    for (( i=0; i < ${#words[@]}-1; i++ )); do
        if [[ ${words[i]} == @(init|help|validate|generate|status|ship) ]]; then
            has_sub=1
        fi
    done

    #local in_shipcat_repo=""
    #if [ -f "$PWD/shipcat.yml" ] || [ -f "$PWD/shipcat.yml" ]; then
    #    in_shipcat_repo="1"
    #fi

    # global flags
    if [[ $prev = 'shipcat' && "$cur" == -* ]]; then
        COMPREPLY=( $(compgen -W '-v -h -V --version --help' -- "$cur" ) )
        return 0
    fi
    # first subcommand
    if [[ -z "$has_sub" ]]; then
        COMPREPLY=( $(compgen -W "$subcommands" -- "$cur" ) )
        return 0
    fi

    # special subcommand completions
    local special i
    for (( i=0; i < ${#words[@]}-1; i++ )); do
        if [[ ${words[i]} == @(generate|status) ]]; then
            special=${words[i]}
        fi
    done

    if [[ -n $special ]]; then
        case $special in
            generate)
                if [[ $prev = "generate" ]]; then
                    local -r envs="$(shipcat list-environments)"
                    COMPREPLY=($(compgen -W "$envs" -- "$cur"))
                fi
                ;;
        esac
    fi

    return 0
} &&
complete -F _shipcat shipcat
