/**
 * Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
 *
 * This Source Code Form is subject to the terms of
 * the Mozilla Public License version 2.0 and additional exceptions,
 * more details in file LICENSE and CONTRIBUTING.
 */

#include <stdio.h>
#include <unistd.h>
#include <stdlib.h>
#include <dlfcn.h>

int main(void)
{
    // note that when the file path contains '/', function 'dlopen' loads from
    // the specified path, otherwise it loads from the system (see `man ldconfig`)
    void *libhandle = dlopen("./lib-test-0.so.1", RTLD_LAZY);
    if (libhandle == NULL)
    {
        const char *err = dlerror();
        printf("dlopen failed: %s\n", err);
        exit(EXIT_FAILURE);
    }

    void *fp = dlsym(libhandle, "add");
    if (fp == NULL)
    {
        const char *err = dlerror();
        printf("dlsym failed: %s\n", err);
        exit(EXIT_FAILURE);
    }

    int (*fn)(int, int) = (int (*)(int, int))fp;
    printf("1+2=%d\n", (*fn)(1, 2));

    dlclose(libhandle);

    exit(EXIT_SUCCESS);
}