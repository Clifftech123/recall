# >>> recall hook start >>>
__recall_precmd() {
    local exit_code=$?
    local last_cmd=$(fc -ln -1)
    if [ -n "$last_cmd" ]; then
        recall log "$last_cmd" --exit-code $exit_code --cwd "$(pwd)" --session $$ --shell zsh 2>/dev/null &
    fi
}
autoload -Uz add-zsh-hook
add-zsh-hook precmd __recall_precmd
# <<< recall hook end <<<
