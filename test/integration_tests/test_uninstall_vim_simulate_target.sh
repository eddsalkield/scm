#!/bin/bash


run_test() {
     echo "checking a general removal of a package in simulate mode, under modified target directories"
     mkdir -p "${TEMP_LOCAL}/new-target"
     exe_sans -d "${BASE_DIR}/test/repo-target" -t "${TEMP_LOCAL}/" -B desktop1 -y install vim

     exe -d "${BASE_DIR}/test/repo-target" -t "${TEMP_LOCAL}/" --hostname desktop1 --no uninstall vim

     # make sure it exited ok
     local last="$?"
     [[ "$last" != "0" ]] && return $last

     # check the linked files exist
     assert_link "${TEMP_LOCAL}/new-target/.vimrc" "${BASE_DIR}/test/repo-target/vim/hosts/desktop1/files/.vimrc" || return 1
     assert_link "${TEMP_LOCAL}/new-target/.vim/filetype.vim" "${BASE_DIR}/test/repo-target/vim/files/.vim/filetype.vim" || return 1
     assert_link "${TEMP_LOCAL}/new-target/.config/i3/config" "${BASE_DIR}/test/repo-target/vim/hosts/desktop1/files/.config/i3/config" || return 1

     return 0
}
