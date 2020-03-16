#!/bin/bash


run_test() {
     echo "checking that a general install of a package works as expected, with tags"
     exe -d "${BASE_DIR}/test/repo" -t "${TEMP_LOCAL}/" -B desktop1 -y -T tag1 --tag tag2 install vim

     # make sure it exited ok
     local last="$?"
     [[ "$last" != "0" ]] && return $last

     # check the linked files exist
     assert_link "${TEMP_LOCAL}/.vimrc" "${BASE_DIR}/test/repo/vim/hosts/desktop1/files/.vimrc" || return 1
     assert_link "${TEMP_LOCAL}/.vim/filetype.vim" "${BASE_DIR}/test/repo/vim/files/.vim/filetype.vim" || return 1
     assert_link "${TEMP_LOCAL}/.config/i3/config" "${BASE_DIR}/test/repo/vim/hosts/desktop1/files/.config/i3/config" || return 1

     assert_link "${TEMP_LOCAL}/tag1_test_file" "${BASE_DIR}/test/repo/vim/tags/tag1/files/tag1_test_file" || return 1
     assert_link "${TEMP_LOCAL}/tag2_test_file" "${BASE_DIR}/test/repo/vim/tags/tag2/files/tag2_test_file" || return 1

     return 0
}
