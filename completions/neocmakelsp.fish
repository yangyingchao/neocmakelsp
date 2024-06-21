complete -c neocmakelsp -n "__fish_use_subcommand" -s h -l help -d 'Print help'
complete -c neocmakelsp -n "__fish_use_subcommand" -f -a "stdio" -d 'run with stdio'
complete -c neocmakelsp -n "__fish_use_subcommand" -f -a "tcp" -d 'run with tcp'
complete -c neocmakelsp -n "__fish_use_subcommand" -f -a "search" -d 'Search packages'
complete -c neocmakelsp -n "__fish_use_subcommand" -f -a "format" -d 'format the file'
complete -c neocmakelsp -n "__fish_use_subcommand" -f -a "tree" -d 'Tree the file'
complete -c neocmakelsp -n "__fish_use_subcommand" -f -a "complete" -d 'Register shell completions for this program'
complete -c neocmakelsp -n "__fish_use_subcommand" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c neocmakelsp -n "__fish_seen_subcommand_from stdio" -s h -l help -d 'Print help'
complete -c neocmakelsp -n "__fish_seen_subcommand_from tcp" -s P -l port -d 'listen to port' -r
complete -c neocmakelsp -n "__fish_seen_subcommand_from tcp" -s h -l help -d 'Print help'
complete -c neocmakelsp -n "__fish_seen_subcommand_from search" -s j -l tojson -d 'tojson'
complete -c neocmakelsp -n "__fish_seen_subcommand_from search" -s h -l help -d 'Print help'
complete -c neocmakelsp -n "__fish_seen_subcommand_from format" -s o -l override -d 'override'
complete -c neocmakelsp -n "__fish_seen_subcommand_from format" -s h -l help -d 'Print help'
complete -c neocmakelsp -n "__fish_seen_subcommand_from tree" -s j -l tojson -d 'tojson'
complete -c neocmakelsp -n "__fish_seen_subcommand_from tree" -s h -l help -d 'Print help'
complete -c neocmakelsp -n "__fish_seen_subcommand_from complete" -l shell -d 'Specify shell to complete for' -r -f -a "{bash	'',fish	''}"
complete -c neocmakelsp -n "__fish_seen_subcommand_from complete" -l register -d 'Path to write completion-registration to' -r -F
complete -c neocmakelsp -n "__fish_seen_subcommand_from complete" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c neocmakelsp -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from stdio; and not __fish_seen_subcommand_from tcp; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from format; and not __fish_seen_subcommand_from tree; and not __fish_seen_subcommand_from complete; and not __fish_seen_subcommand_from help" -f -a "stdio" -d 'run with stdio'
complete -c neocmakelsp -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from stdio; and not __fish_seen_subcommand_from tcp; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from format; and not __fish_seen_subcommand_from tree; and not __fish_seen_subcommand_from complete; and not __fish_seen_subcommand_from help" -f -a "tcp" -d 'run with tcp'
complete -c neocmakelsp -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from stdio; and not __fish_seen_subcommand_from tcp; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from format; and not __fish_seen_subcommand_from tree; and not __fish_seen_subcommand_from complete; and not __fish_seen_subcommand_from help" -f -a "search" -d 'Search packages'
complete -c neocmakelsp -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from stdio; and not __fish_seen_subcommand_from tcp; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from format; and not __fish_seen_subcommand_from tree; and not __fish_seen_subcommand_from complete; and not __fish_seen_subcommand_from help" -f -a "format" -d 'format the file'
complete -c neocmakelsp -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from stdio; and not __fish_seen_subcommand_from tcp; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from format; and not __fish_seen_subcommand_from tree; and not __fish_seen_subcommand_from complete; and not __fish_seen_subcommand_from help" -f -a "tree" -d 'Tree the file'
complete -c neocmakelsp -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from stdio; and not __fish_seen_subcommand_from tcp; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from format; and not __fish_seen_subcommand_from tree; and not __fish_seen_subcommand_from complete; and not __fish_seen_subcommand_from help" -f -a "complete" -d 'Register shell completions for this program'
complete -c neocmakelsp -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from stdio; and not __fish_seen_subcommand_from tcp; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from format; and not __fish_seen_subcommand_from tree; and not __fish_seen_subcommand_from complete; and not __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'