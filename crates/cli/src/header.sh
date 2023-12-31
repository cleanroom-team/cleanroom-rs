#!/usr/bin/sh -e
# The entire script is run in `busybox sh -e`
__command_prefix="${1}"
readonly __command_prefix
shift
CURRENT_PHASE="${1}"
readonly CURRENT_PHASE
shift

CLRM_DIR="/tmp/clrm"
readonly CLRM_DIR
BUSYBOX="${CLRM_DIR}/busybox"
readonly BUSYBOX

status() {
	message="${1}"
	shift

	echo "${__command_prefix}: STATUS \"${message}\""
}

status "Setup: ${CURRENT_PHASE} (${CLRM_CONTAINER})"

push_status() {
	message="${1}"
	shift

	echo "${__command_prefix}: PUSH \"${message}\""
}

pop_status() {
	echo "${__command_prefix}: POP"
}

export_constant() {
	key="${1}"
	shift

	eval "${key}=\"${*}\""
	readonly "${key}"
	echo "${__command_prefix}: SET_RO \"${key}\"=\"${*}\""
}

export_var() {
	key="${1}"
	shift

	echo "${__command_prefix}: SET \"${key}\"=\"${*}\""
}

add_dependency() {
	key="${1}"
	shift

	eval "${key}=\"${*}\""
	echo "${__command_prefix}: ADD_DEPENDENCY \"${key}\"=\"${*}\""
}

error() {
	echo "Error in Agent script: ${*}"
	exit 1
}

assert_distribution_initialized() {
	if [ -z "${CLRM_BASE_DISTRIBUTION}" ]; then
		error "Distribution not yet initialized. Call \"_distribution <id>\" first!"
	fi
}

bb_chmod() {
	"${BUSYBOX}" chmod "${@}"
}

bb_mkdir() {
	"${BUSYBOX}" mkdir "${@}"
}

bb_mknod() {
	"${BUSYBOX}" mknod "${@}"
}

cd "${ROOT_FS}" || error "Failed to cd into ${ROOT_FS}"
