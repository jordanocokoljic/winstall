# winstall

`winstall` is an attempt at a substitute or shim for the GNU coreutils install
program on Windows. It is intended for 'drop in' use in makefiles or other such
applications. Any Linux specifics from install such as SE Linux handling or UNIX
style file permissions are ignored.

More specifically, it does support:
  - GNU style file backups, both numbered (file.ext.~1~) and simple (file.ext~)
  - Timestamp preservation
  - Bulk directory creation

`winstall` does not make any commitment to maintaining identical output, so
scripts that check for particular messages may not work.

It also doesn't currently support condensed UNIX style arguments
(e.g. `app -v -D -T -> app -vDT`) but this may change in the future.
