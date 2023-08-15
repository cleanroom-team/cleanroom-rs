#!/usr/bin/sh -e
# The entire script is run in `busybox sh -e`
__command_prefix="${1}"
readonly __command_prefix
shift
CURRENT_PHASE="${1}"
readonly CURRENT_PHASE
shift

status() {
	message="${1}"
	shift

	echo "${__command_prefix}: STATUS \"${message}\""
}

status "Setup: ${CURRENT_PHASE}"

export_constant() {
	key="${1}"
	shift
	value="${1}"
	shift

	eval "${key}=\"${value}\""
	readonly "${key}"
	echo "${__command_prefix}: SET_RO \"${key}\"=\"${value}\""
}

export_var() {
	key="${1}"
	shift
	value="${1}"
	shift

	eval "${key}=\"${value}\""
	echo "${__command_prefix}: SET \"${key}\"=\"${value}\""
}

alias_command() {
	key="${1}"
	shift
	value="${1}"
	shift

	echo "${__command_prefix}: ALIAS \"${key}\"=\"${value}\""
}

error() {
	echo "Error in Agent script: ${*}"
	exit 1
}

assert_distribution_initialized() {
	if [ -z "${CLRM_BASE_DISTRIBUTION}" ]; then
		error "Distribution not yet initialized. Call \"distribution <id>\" first!"
	fi
}
