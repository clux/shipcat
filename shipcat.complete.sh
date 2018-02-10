# shellcheck disable=2148
# shipcat(1) completion

_shipcat()
{
    # shellcheck disable=2034
    local cur prev words cword
    _init_completion || return

    local -r subcommands="init help validate generate status ship shell
                          list-environments"

    local has_sub
    for (( i=0; i < ${#words[@]}-1; i++ )); do
        if [[ ${words[i]} == @(init|help|validate|generate|status|ship|shell) ]]; then
            has_sub=1
        fi
    done

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
        if [[ ${words[i]} == @(generate|validate|ship|shell) ]]; then
            special=${words[i]}
        fi
    done

    if [[ -n $special ]]; then
        case $special in
            validate)
                if [[ $prev = "validate" ]]; then
                    svcs=$(find "./services" -maxdepth 1 -mindepth 1 -type d -printf "%f " 2> /dev/null)
                    COMPREPLY=($(compgen -W "$svcs" -- "$cur"))
                fi
                ;;
            generate|ship)
                if [[ $prev = @(generate|ship) ]]; then
                    COMPREPLY=($(compgen -W "-r --region" -- "$cur"))
                elif [[ $prev == @(-r|--region) ]]; then
                    local -r regions="dev-uk"
                    COMPREPLY=($(compgen -W "$regions" -- "$cur"))
                else
                    svcs=$(find "./services" -maxdepth 1 -mindepth 1 -type d -printf "%f " 2> /dev/null)
                    COMPREPLY=($(compgen -W "$svcs" -- "$cur"))
                fi
                ;;
            shell)
                if [[ $prev = @(shell) ]]; then
                    COMPREPLY=($(compgen -W "-r --region -p --pod" -- "$cur"))
                elif [[ $prev == @(-r|--region) ]]; then
                    local -r regions="dev-uk"
                    COMPREPLY=($(compgen -W "$regions" -- "$cur"))
                elif [[ $prev == @(-p|--pod) ]]; then
                    local -r pods="1 2 3 4 5 6"
                    COMPREPLY=($(compgen -W "$pods" -- "$cur"))
                else
                    svcs=$(find "./services" -maxdepth 1 -mindepth 1 -type d -printf "%f " 2> /dev/null)
                    COMPREPLY=($(compgen -W "$svcs" -- "$cur"))
                fi
                ;;
        esac
    fi

    return 0
} &&
complete -F _shipcat shipcat
