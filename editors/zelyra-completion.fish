# Fish completion for zelyra
complete -c zelyra -f

# Main commands
complete -c zelyra -n "__fish_use_subcommand" -a run -d "Run a Zelyra file directly"
complete -c zelyra -n "__fish_use_subcommand" -a check -d "Validate syntax and invariants"
complete -c zelyra -n "__fish_use_subcommand" -a serve -d "Start built-in HTTP application server"
complete -c zelyra -n "__fish_use_subcommand" -a new -d "Scaffold a new Zelyra project"
complete -c zelyra -n "__fish_use_subcommand" -a setup -d "Run automated setup wizard"
complete -c zelyra -n "__fish_use_subcommand" -a db -d "Database migration tools"
complete -c zelyra -n "__fish_use_subcommand" -a verify -d "Run invariant verification engine"
complete -c zelyra -n "__fish_use_subcommand" -a doctor -d "Diagnose environment"
complete -c zelyra -n "__fish_use_subcommand" -a fmt -d "Format Zelyra source files"
complete -c zelyra -n "__fish_use_subcommand" -a test -d "Run tests and assertions"

# File completions for commands taking .zyl files
complete -c zelyra -n "__fish_seen_subcommand_from run check serve fmt verify" -F -k -a "(__fish_complete_suffix .zyl)"

# Flags
complete -c zelyra -n "__fish_seen_subcommand_from new" -l mariadb -d "Use MariaDB template"
complete -c zelyra -n "__fish_seen_subcommand_from new" -l postgres -d "Use PostgreSQL template"
complete -c zelyra -n "__fish_seen_subcommand_from new" -l sqlite -d "Use SQLite template"
complete -c zelyra -n "__fish_seen_subcommand_from setup" -l all -d "Non-interactive setup"
complete -c zelyra -n "__fish_seen_subcommand_from setup" -l web -d "Browser guided setup"

# db subcommands
complete -c zelyra -n "__fish_seen_subcommand_from db" -a "plan migrate setup status"
