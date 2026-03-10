# >>> recall hook start >>>
function __recall_post_exec --on-event fish_postexec
    set -l exit_code $status
    set -l last_cmd $argv[1]
    if test -n "$last_cmd"
        recall log "$last_cmd" --exit-code $exit_code --cwd (pwd) --session %self --shell fish 2>/dev/null &
    end
end
# <<< recall hook end <<<
