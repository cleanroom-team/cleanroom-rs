#!/usr/bin/sh -e

# The entire script is run in `busybox sh -e`

command_prefix="${1}"
shift

status() {
	message="${1}"
	shift

	echo "${command_prefix}: STATUS \"${message}\""
}

status "Setup phase script"

export_constant() {
	key="${1}"
	shift
	value="${1}"
	shift

	echo "${command_prefix}: SET_RO \"${key}\"=\"${value}\""
}

export_var() {
	key="${1}"
	shift
	value="${1}"
	shift

	echo "${command_prefix}: SET \"${key}\"=\"${value}\""
}
