/**
 * Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
 *
 * This Source Code Form is subject to the terms of
 * the Mozilla Public License version 2.0 and additional exceptions,
 * more details in file LICENSE and CONTRIBUTING.
 */

int add(int a, int b)
{
    return a + b;
}

int mul_add(int a, int b, int c)
{
    return a * b + c;
}

int do_something(int (*callback_func)(int), int a, int b)
{
    int s = (callback_func)(a);
    return s + b;
}

// compile this file with the command:
// `$ gcc -Wall -g -fpic -shared -Wl,-soname,libtest0.so.1 -o libtest0.so.1.0.0 libtest0.c`
//
// it is recommended to create a symbolic link to this shared library:
// `ln -s libtest0.so.1.0.0 libtest0.so.1`
// `ln -s libtest0.so.1.0.0 libtest0.so`