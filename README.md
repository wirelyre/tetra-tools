# perfect-clear

A tool for researching 5- and 10-piece perfect clears in tetromino stackers.

When you run the program, it will do a large precomputation of all possible
boards which result in 10-piece perfect clears.  These will be saved to a file.

Currently there is no interface at all, but the `gameplay` module implements
SRS with no library dependencies, which might be useful.