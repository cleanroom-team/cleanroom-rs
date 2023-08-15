#!/usr/bin/sh -e
# The entire script is run in `busybox sh -e`

__command_prefix="${1}"
readonly __command_prefix
shift
CURRENT_PHASE="${1}"
readonly CURRENT_PHASE
shift
CURRENT_SUB_PHASE="${1}"
readonly CURRENT_SUB_PHASE
shift

status() {
	message="${1}"
	shift

	echo "${__command_prefix}: STATUS \"${message}\""
}

status "Setup: ${CURRENT_PHASE}::${CURRENT_SUB_PHASE}"

export_constant() {
	key="${1}"
	shift
	value="${1}"
	shift

	echo "${__command_prefix}: SET_RO \"${key}\"=\"${value}\""
}

export_var() {
	key="${1}"
	shift
	value="${1}"
	shift

	echo "${__command_prefix}: SET \"${key}\"=\"${value}\""
}
