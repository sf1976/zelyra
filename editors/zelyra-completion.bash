# bash completion for zelyra
_zelyra() {
    local cur prev words cword
    _init_completion || return

    local commands="run check serve new setup db verify doctor fmt test help"
    local db_commands="plan migrate setup status reset"
    local setup_flags="--all --web"
    local new_flags="--mariadb --postgres --sqlite"

    if [[ $cword -eq 1 ]]; then
        COMPREPLY=( $(compgen -W "${commands}" -- "${cur}") )
        return 0
    fi

    case "${words[1]}" in
        run|check|fmt|verify)
            COMPREPLY=( $(compgen -f -X '!*.zyl' -- "${cur}") $(compgen -d -- "${cur}") )
            ;;
        serve)
            if [[ $cword -eq 2 ]]; then
                COMPREPLY=( $(compgen -f -X '!*.zyl' -- "${cur}") $(compgen -d -- "${cur}") )
            fi
            ;;
        new)
            if [[ "${cur}" == -* ]]; then
                COMPREPLY=( $(compgen -W "${new_flags}" -- "${cur}") )
            fi
            ;;
        setup)
            COMPREPLY=( $(compgen -W "${setup_flags}" -- "${cur}") )
            ;;
        db)
            if [[ $cword -eq 2 ]]; then
                COMPREPLY=( $(compgen -W "${db_commands}" -- "${cur}") )
            else
                COMPREPLY=( $(compgen -f -X '!*.zyl' -- "${cur}") $(compgen -d -- "${cur}") )
            fi
            ;;
    esac
}
complete -F _zelyra zelyra
