# ~/.zshrc — sample zsh configuration for recall hook tests

# ── Environment ───────────────────────────────────────────────────────────────
export EDITOR="nvim"
export VISUAL="nvim"
export PAGER="less"
export LANG="en_US.UTF-8"
export LC_ALL="en_US.UTF-8"

export PATH="$HOME/.local/bin:$HOME/bin:/usr/local/bin:$PATH"

# ── History ───────────────────────────────────────────────────────────────────
HISTFILE="$HOME/.zsh_history"
HISTSIZE=10000
SAVEHIST=10000
setopt HIST_IGNORE_DUPS
setopt HIST_IGNORE_SPACE
setopt SHARE_HISTORY
setopt APPEND_HISTORY

# ── Completion ────────────────────────────────────────────────────────────────
autoload -Uz compinit
compinit

zstyle ':completion:*' menu select
zstyle ':completion:*' matcher-list 'm:{a-zA-Z}={A-Za-z}'

# ── Prompt ────────────────────────────────────────────────────────────────────
autoload -Uz vcs_info
precmd_vcs_info() { vcs_info }
precmd_functions+=( precmd_vcs_info )

setopt PROMPT_SUBST
PROMPT='%F{cyan}%~%f %F{yellow}${vcs_info_msg_0_}%f %F{green}❯%f '

# ── Aliases ───────────────────────────────────────────────────────────────────
alias ll='ls -lAh --color=auto'
alias la='ls -A --color=auto'
alias ls='ls --color=auto'
alias grep='grep --color=auto'
alias diff='diff --color=auto'

alias g='git'
alias gs='git status'
alias ga='git add'
alias gc='git commit'
alias gp='git push'
alias gl='git log --oneline --graph --decorate'

alias ..='cd ..'
alias ...='cd ../..'
alias ....='cd ../../..'

alias vi='nvim'
alias vim='nvim'

# ── Functions ─────────────────────────────────────────────────────────────────
mkcd() {
    mkdir -p "$1" && cd "$1"
}

# Quick find
f() {
    find . -name "*$1*" 2>/dev/null
}

# ── Key bindings ──────────────────────────────────────────────────────────────
bindkey -e
bindkey '^[[A' history-search-backward
bindkey '^[[B' history-search-forward
bindkey '^[[H' beginning-of-line
bindkey '^[[F' end-of-line

# ── Tools ─────────────────────────────────────────────────────────────────────
# nvm
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"

# cargo / rust
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"

# fzf
[ -f "$HOME/.fzf.zsh" ] && source "$HOME/.fzf.zsh"
