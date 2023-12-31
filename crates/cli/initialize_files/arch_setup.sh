#!/usr/bin/sh -e

# cSpell: ignore pacman

dir="${1}"
readonly dir
shift

busybox="${1}"
readonly busybox
shift

cleanroom="${1}"
readonly cleanroom
shift

## Fix up .envrc file

# There is no global replace in busybox sed...
"${busybox}" sed -i "s!%%%DIR%%%!${dir}!" README.md
"${busybox}" sed -i "s!%%%DIR%%%!${dir}!" README.md
"${busybox}" sed -i "s!%%%DIR%%%!${dir}!" README.md
"${busybox}" sed -i "s!%%%DIR%%%!${dir}!" README.md
"${busybox}" sed -i "s!%%%DIR%%%!${dir}!" README.md

"${busybox}" sed -i "s!%%%DIR%%%!${dir}!" .envrc
"${busybox}" sed -i "s!%%%DIR%%%!${dir}!" .envrc
"${busybox}" sed -i "s!%%%DIR%%%!${dir}!" .envrc
"${busybox}" sed -i "s!%%%DIR%%%!${dir}!" .envrc
"${busybox}" sed -i "s!%%%DIR%%%!${dir}!" .envrc
"${busybox}" sed -i "s!%%%BUSYBOX%%%!${busybox}!" .envrc
"${busybox}" sed -i "s!%%%BUSYBOX%%%!${busybox}!" .envrc
"${busybox}" sed -i "s!%%%BUSYBOX%%%!${busybox}!" .envrc
"${busybox}" sed -i "s!%%%BUSYBOX%%%!${busybox}!" .envrc
"${busybox}" sed -i "s!%%%BUSYBOX%%%!${busybox}!" .envrc

## Arch:

pch="hook_write_pacman_conf"
"${cleanroom}" dump-command "${pch}" >"commands/${pch}.dump"
mv "commands/${pch}.dump" "commands/${pch}.toml"

## Set up Git:
git="$("${busybox}" which git || true)"
test -z "${git}" && echo "Warn: Git not found, not initializing version control."

test -n "${git}" && git init -b main . >/dev/null
test -n "${git}" && git add . >/dev/null

## Finish up:

cat <<EOF
Next steps:

 * Check out the README.md for instructions
EOF
test -n "${git}" && echo " * Git was initialized already"

rm setup.sh
