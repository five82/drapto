#!/usr/bin/env bash

# Force color if environment variables are set appropriately
if [[ "$FORCE_COLOR" == "1" ]] || [[ "$CLICOLOR_FORCE" == "1" ]] || [[ "$PTY" == "1" ]]; then
    FORCE_COLORS=true
else
    FORCE_COLORS=false
fi

# Check if terminal supports colors (both Linux and macOS)
if { [[ -t 1 ]] || [[ "$FORCE_COLORS" == true ]]; } && command -v tput >/dev/null 2>&1; then
    # Number of colors
    if [[ $(tput colors 2>/dev/null || echo 0) -ge 8 ]]; then
        # Basic formatting
        BOLD="$(tput bold)"
        RESET="$(tput sgr0)"
        
        # Basic colors
        GREEN="$(tput setaf 2)"
        YELLOW="$(tput setaf 3)"
        BLUE="$(tput setaf 4)"
        MAGENTA="$(tput setaf 5)"
        CYAN="$(tput setaf 6)"
        WHITE="$(tput setaf 7)"
        RED="$(tput setaf 1)"
        
        # Bold + color combinations
        BOLD_GREEN="${BOLD}${GREEN}"
        BOLD_YELLOW="${BOLD}${YELLOW}"
        BOLD_BLUE="${BOLD}${BLUE}"
        BOLD_MAGENTA="${BOLD}${MAGENTA}"
        BOLD_CYAN="${BOLD}${CYAN}"
        BOLD_WHITE="${BOLD}${WHITE}"
        BOLD_RED="${BOLD}${RED}"
    fi
fi

# Print a checkmark message in green
print_check() {
    echo -e "${BOLD_GREEN}✓${RESET} ${BOLD}$*${RESET}" >&2
}

# Print a warning message in yellow
print_warning() {
    echo -e "${BOLD_YELLOW}⚠${RESET} ${BOLD}$*${RESET}" >&2
}

# Print an error message in red
print_error() {
    echo -e "${BOLD_RED}✗${RESET} ${BOLD}$*${RESET}" >&2
}

# Print a success message
print_success() {
    echo -e "${GREEN}✓${RESET} ${GREEN}$*${RESET}" >&2
}

# Print a section header
print_header() {
    local title="$1"
    local width=80
    local padding=$(( (width - ${#title}) / 2 ))
    
    echo
    echo -e "${BOLD_BLUE}$(printf '%*s' "$width" | tr ' ' '=')${RESET}"
    echo -e "${BOLD_BLUE}$(printf "%*s%s%*s" "$padding" "" "$title" "$padding" "")${RESET}"
    echo -e "${BOLD_BLUE}$(printf '%*s' "$width" | tr ' ' '=')${RESET}"
    echo
}

# Print a separator line
print_separator() {
    echo -e "${BLUE}----------------------------------------${RESET}"
}

# Print a file path with highlighting
print_path() {
    echo -e "${BOLD_CYAN}$*${RESET}"
}

# Print a statistic or measurement
print_stat() {
    echo -e "${BOLD_MAGENTA}$*${RESET}"
}
