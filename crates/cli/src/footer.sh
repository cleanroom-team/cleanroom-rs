#!/usr/bin/sh -e

cmd_test_pre param1_pre param2_pre
cmd_test param1 param2
cmd_test_post param1_post param2_post

echo "Done with ${VERSION}"

status "Script reached the end"
