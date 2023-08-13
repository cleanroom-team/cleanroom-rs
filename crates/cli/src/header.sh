#!/usr/bin/sh -e

# The entire script is run in `busybox sh -e`

command_prefix="${1}"

export_var() {
	key="${1}"
	value="${2}"

	echo "${command_prefix}: SET \"${key}\"=\"${value}\""
}
