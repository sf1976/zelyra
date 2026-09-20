#compdef zelyra

_zelyra() {
    local -a commands
    commands=(
        'run:Run a Zelyra file directly'
        'check:Validate syntax, types and invariants without running'
        'serve:Start the built-in HTTP application server'
        'new:Scaffold a new Zelyra project'
        'setup:Run the automated environment and database setup'
        'db:Database schema migration and inspection tools'
        'verify:Run formal loop invariant and contract verification'
        'doctor:Diagnose local toolchain, Docker and runtime environment'
        'fmt:Format Zelyra source files according to style standards'
        'test:Run test suites and invariant assertions'
    )

    _arguments -C \
        '1: :->command' \
        '*:: :->args'

    case $state in
        command)
            _describe 'zelyra command' commands
            ;;
        args)
            case $words[1] in
                run|check|fmt|verify)
                    _files -g '*.zyl'
                    ;;
                serve)
                    _files -g '*.zyl'
                    ;;
                new)
                    _arguments \
                        '--mariadb[Use MariaDB template]' \
                        '--postgres[Use PostgreSQL template]' \
                        '--sqlite[Use SQLite template]'
                    ;;
                setup)
                    _arguments \
                        '--all[Run non-interactive automated setup]' \
                        '--web[Launch browser-based setup wizard]'
                    ;;
                db)
                    local -a db_subcommands
                    db_subcommands=(
                        'plan:Inspect planned schema changes'
                        'migrate:Apply pending schema migrations'
                        'setup:Initialize database and apply initial schema'
                        'status:Show migration status'
                    )
                    _describe 'db command' db_subcommands
                    ;;
            esac
            ;;
    esac
}

_zelyra "$@"
