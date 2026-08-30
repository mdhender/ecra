#!/bin/sh

set -eu

usage() {
    cat >&2 <<'EOF'
Usage:
  scripts/beta.sh create STORE [--overwrite]
  scripts/beta.sh players available
  scripts/beta.sh players add EMAIL...
  scripts/beta.sh players assigned
  scripts/beta.sh report stellia [--json]
EOF
    exit 2
}

validate_store_name() {
    case "$1" in
        "" | "." | ".." | */*)
            echo "error: STORE must be a file name within games/beta" >&2
            exit 2
            ;;
    esac
}

require_ecra() {
    if [ ! -x "$ecra" ]; then
        echo "error: release binary not found; run cargo build --release" >&2
        exit 1
    fi
}

load_game_config() {
    if [ ! -r "$env_file" ]; then
        echo "error: $env_file is missing or unreadable" >&2
        exit 1
    fi
    . "$env_file"
    if [ -z "${ECRA_GAME_CODE:-}" ]; then
        echo "error: $env_file must define ECRA_GAME_CODE" >&2
        exit 1
    fi
}

prepare_beta_game() {
    store="$beta_dir/store"
    require_ecra
    load_game_config
}

list_available_players() {
    assigned_players=$("$ecra" report players "$store" "$ECRA_GAME_CODE")

    echo "EMAIL"
    account_number=2
    while [ "$account_number" -le 13 ]; do
        email=$(printf 'account.%04d@example.com' "$account_number")
        if ! printf '%s\n' "$assigned_players" | grep -Fqx -- "$email"; then
            echo "$email"
        fi
        account_number=$((account_number + 1))
    done
}

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
root_dir=$(dirname -- "$script_dir")
beta_dir="$root_dir/games/beta"
ecra="$root_dir/target/release/ecra"
env_file="$root_dir/.env.development.local"

[ "$#" -ge 1 ] || usage

command=$1
shift

case "$command" in
    create)
        [ "$#" -ge 1 ] || usage
        store_name=$1
        shift
        validate_store_name "$store_name"

        overwrite=false
        case "$#" in
            0) ;;
            1)
                [ "$1" = "--overwrite" ] || usage
                overwrite=true
                ;;
            *) usage ;;
        esac

        store="$beta_dir/$store_name"
        require_ecra
        load_game_config
        if [ -z "${ECRA_GAME_SEED:-}" ]; then
            echo "error: $env_file must define ECRA_GAME_SEED" >&2
            exit 1
        fi

        create_store=true
        if [ -e "$store" ] || [ -L "$store" ]; then
            if [ "$overwrite" = false ]; then
                echo "ECRA store already exists at $store"
                create_store=false
            fi
        fi

        if [ "$create_store" = true ]; then
            rm -f -- "$store"
            mkdir -p -- "$beta_dir"
            "$ecra" new "$store"
        fi
        "$ecra" seed-accounts "$store"
        if [ "$create_store" = true ] ||
            ! "$ecra" report stellia "$store" "$ECRA_GAME_CODE" >/dev/null 2>&1
        then
            "$ecra" generate-game "$store" "$ECRA_GAME_CODE" --seed "$ECRA_GAME_SEED"
        fi
        ;;
    players)
        [ "$#" -ge 1 ] || usage
        player_command=$1
        shift

        prepare_beta_game
        case "$player_command" in
            available)
                [ "$#" -eq 0 ] || usage
                list_available_players
                ;;
            add)
                [ "$#" -ge 1 ] || usage
                "$ecra" add-players "$store" "$ECRA_GAME_CODE" "$@"
                ;;
            assigned)
                [ "$#" -eq 0 ] || usage
                "$ecra" report players "$store" "$ECRA_GAME_CODE"
                ;;
            *) usage ;;
        esac
        ;;
    report)
        [ "$#" -ge 1 ] || usage
        report_name=$1
        shift
        [ "$report_name" = "stellia" ] || usage

        json=false
        case "$#" in
            0) ;;
            1)
                [ "$1" = "--json" ] || usage
                json=true
                ;;
            *) usage ;;
        esac

        store="$beta_dir/store"
        require_ecra
        load_game_config
        if [ "$json" = true ]; then
            report_dir="$beta_dir/reports"
            mkdir -p -- "$report_dir"
            "$ecra" report stellia "$store" "$ECRA_GAME_CODE" \
                --json "$report_dir/t0-stellia.json"
        else
            "$ecra" report stellia "$store" "$ECRA_GAME_CODE"
        fi
        ;;
    *) usage ;;
esac
