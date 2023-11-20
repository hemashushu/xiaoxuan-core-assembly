# Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
#
# This Source Code Form is subject to the terms of
# the Mozilla Public License version 2.0 and additional exceptions,
# more details in file LICENSE and CONTRIBUTING.

#!/bin/bash
gcc -Wall -g -fpic -shared -Wl,-soname,lib-test-0.so.1 -o lib-test-0.so.1.0.0 lib-test-0.c
gcc -Wall -g -o app.elf app.c

if [ ! -f lib-test-0.so.1 ]
then
    ln -s lib-test-0.so.1.0.0 lib-test-0.so.1
fi