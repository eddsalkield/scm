#!/bin/bash


run_test() {
     echo "checking that a general install of a different package works as expected, under modified target directories"
     mkdir -p "${TEMP_LOCAL}/new-target"
     mkdir -p "${TEMP_LOCAL}/new-target-2"
     exe -d "${BASE_DIR}/test/repo-target" -t "${TEMP_LOCAL}/" -B desktop1 -y install zsh

     # make sure it exited ok
     local last="$?"
     [[ "$last" != "0" ]] && return $last

     # check the linked files exist
     echo "next six assertations should fail - we don't want to install the vim package"
     assert_link "${TEMP_LOCAL}/.vimrc" "${BASE_DIR}/test/repo-target/vim/hosts/desktop1/files/.vimrc" && return 1
     assert_link "${TEMP_LOCAL}/new-target/.vimrc" "${BASE_DIR}/test/repo-target/vim/hosts/desktop1/files/.vimrc" && return 1
     assert_link "${TEMP_LOCAL}/new-target2/.vimrc" "${BASE_DIR}/test/repo-target/vim/hosts/desktop1/files/.vimrc" && return 1
     assert_link "${TEMP_LOCAL}/.vim/filetype.vim" "${BASE_DIR}/test/repo-target/vim/files/.vim/filetype.vim" && return 1
     assert_link "${TEMP_LOCAL}/new-target/.vim/filetype.vim" "${BASE_DIR}/test/repo-target/vim/files/.vim/filetype.vim" && return 1
     assert_link "${TEMP_LOCAL}/new-target-2/.vim/filetype.vim" "${BASE_DIR}/test/repo-target/vim/files/.vim/filetype.vim" && return 1

     assert_link "${TEMP_LOCAL}/new-target-2/.zshrc" "${BASE_DIR}/test/repo-target/zsh/files/.zshrc" || return 1

     return 0
}
